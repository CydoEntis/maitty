//! Resume a conversation Claude or Codex saved for the focused pane's folder: the daemon lists
//! each agent's (`Query::SavedConversations`), the answers come back as messages, and the
//! newest are offered as a menu whose pick starts that agent there, resumed.

use super::*;
use crate::protocol::{SavedConversation, SavedConversations};

/// The agents whose saved conversations the daemon lists, in the order they are asked, each
/// with the name a row shows.
const AGENTS: [(&str, &str); 2] = [("claude", "Claude"), ("codex", "Codex")];

/// How many conversations the menu offers.
const LISTED: usize = 9;

/// The widest a row's label is drawn; the menu cuts anything wider.
const LABEL_MAX: usize = 44;

/// The longest id passed to an agent's resume option. Claude's and Codex's are 36-character UUIDs.
const ID_MAX: usize = 128;

#[derive(Default)]
pub(super) struct Resume {
    next_tag: u64,
    asked: Option<Asked>,
}

/// The lists asked for, one per agent of `AGENTS`, until every one is answered.
struct Asked {
    dir: PathBuf,
    first_tag: u64,
    lists: [Option<Vec<SavedConversation>>; AGENTS.len()],
    at: (u16, u16),
}

impl App {
    /// Ask this computer's daemon for each agent's saved conversations in the selected pane's folder.
    pub(super) fn ask_saved_conversations(&mut self) {
        let Some(term) = self.selected_term() else {
            return self.notify("no pane to resume a conversation in".into(), true);
        };
        let Some(dir) = self.local_cwd(term) else {
            return;
        };
        let first_tag = self.resume.next_tag;
        self.resume.next_tag += AGENTS.len() as u64;
        for (i, (agent, _)) in AGENTS.iter().enumerate() {
            self.send(
                term.machine,
                ClientMsg::Query(Query::SavedConversations {
                    agent: (*agent).into(),
                    folder: dir.display().to_string(),
                    tag: first_tag + i as u64,
                }),
            );
        }
        let at = self
            .pane_frames
            .iter()
            .find(|(t, _)| *t == term)
            .map_or((0, 0), |(_, r)| (r.x + 2, r.y));
        self.resume.asked = Some(Asked {
            dir,
            first_tag,
            lists: Default::default(),
            at,
        });
    }

    /// One agent's answer; the last one opens the menu, or says there is nothing to resume.
    pub(super) fn saved_conversations_arrived(&mut self, tag: u64, found: SavedConversations) {
        let Some(asked) = self.resume.asked.as_mut() else {
            return;
        };
        let Some(slot) = tag
            .checked_sub(asked.first_tag)
            .and_then(|i| asked.lists.get_mut(usize::try_from(i).ok()?))
        else {
            return;
        };
        *slot = Some(match found {
            SavedConversations::Listed(list) => list,
            SavedConversations::NotSupported => Vec::new(),
        });
        if asked.lists.iter().any(Option::is_none) {
            return;
        }
        let Some(asked) = self.resume.asked.take() else {
            return;
        };
        self.resume_menu(asked);
    }

    fn resume_menu(&mut self, asked: Asked) {
        let mut all: Vec<(&str, SavedConversation)> = AGENTS
            .into_iter()
            .map(|(agent, _)| agent)
            .zip(asked.lists)
            .flat_map(|(agent, list)| {
                list.unwrap_or_default()
                    .into_iter()
                    .map(move |c| (agent, c))
            })
            .collect();
        if all.is_empty() {
            return self.notify("no saved Claude or Codex chats".into(), false);
        }
        all.sort_by_key(|(_, c)| std::cmp::Reverse(c.last_active));
        let items = all
            .into_iter()
            .take(LISTED)
            .map(|(agent, c)| {
                (
                    row_label(agent, &c),
                    menu::Act::Resume(asked.dir.clone(), agent.into(), c.id),
                )
            })
            .collect();
        self.menu(
            format!("resume · {}", super::screen::folder_name(&asked.dir)),
            items,
            asked.at,
        );
    }

    /// Start `agent` in a new pane in `dir`, resuming conversation `id`.
    pub(super) fn resume_saved(&mut self, dir: PathBuf, agent: &str, id: &str) {
        match resume_command(&self.cfg, agent, id) {
            Ok(cmd) => self.screen_new_session_on(MachineId::LOCAL, dir, Some(cmd), false),
            Err(why) => self.notify(why, true),
        }
    }
}

/// `Claude · 3h ago · the first message…`, cut to fit the menu.
fn row_label(agent: &str, c: &SavedConversation) -> String {
    let said = crate::hidden::sanitize(&c.first_message, false);
    let label = format!(
        "{} · {} ago · {said}",
        agent_name(agent),
        super::screen::age(c.last_active)
    );
    super::render::truncate(&label, LABEL_MAX)
}

fn agent_name(agent: &str) -> &str {
    AGENTS
        .iter()
        .find(|(id, _)| *id == agent)
        .map_or(agent, |(_, name)| name)
}

/// The agent's own resume command (`claude --resume <id>`, `codex resume <id>`), from its
/// definition in the config. The id came from files on disk, so only plain ids are put in a
/// command line.
fn resume_command(cfg: &Config, agent: &str, id: &str) -> Result<String, String> {
    let plain = !id.is_empty()
        && id.len() <= ID_MAX
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !plain {
        return Err(format!(
            "{} gave a conversation id that can't be resumed",
            agent_name(agent)
        ));
    }
    cfg.agent_defs()
        .into_iter()
        .find(|d| d.name == agent)
        .and_then(|d| d.resume)
        .map(|tpl| tpl.replace("{session}", id))
        .ok_or_else(|| format!("{} has no resume command in your config", agent_name(agent)))
}
