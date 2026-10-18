//! Right-click menus for the seshi layout: a pane, an agent row, a project, a branch or a
//! worktree. Short, hoverable, and every item does something.

use super::design::{BtnKind, button, put, seg, segs_width};
use super::hydra::{HyHit, find, hit, hovered};
use super::render::truncate;
use super::{App, Mode};
use crate::protocol::{Command, TermId};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use std::path::PathBuf;
use unicode_width::UnicodeWidthStr;

/// How long a snooze can be (seconds; none: until it has something for you).
const SNOOZES: [(&str, Option<u64>); 4] = [("for an hour", Some(3600)), ("for four hours", Some(4 * 3600)), ("for a day", Some(86_400)), ("until it has something for me", None)];

/// How many snapshots the list offers (each has its number as its key).
const SNAPSHOTS_LISTED: usize = 9;

#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    Beside(TermId),
    /// The program in this pane is an agent: seshi learns it.
    TeachAgent(TermId),
    /// End these (stops what runs in them).
    End(Vec<TermId>),
    Changes(PathBuf),
    RenamePane(TermId),
    /// Tell this agent to run a fleet from now on (`seshi brief`).
    Commander(TermId),
    /// Drop the follow-ups waiting for this agent's turn to end.
    ClearQueue(TermId),
    /// Fold / unfold a session's subagents in the sidebar.
    ToggleSubs(TermId),
    /// The choices for how long to snooze a session.
    SnoozeMenu(TermId),
    /// Set a session aside until then (`Some(0)`: settled), or bring it back (`None`).
    Aside(TermId, Option<u64>),
    /// Fold / unfold a project's settled group (by the project's key).
    ToggleAside(String),
    /// List a checkout's snapshots (its folder, and whose row asked).
    Snapshots(PathBuf, TermId),
    /// Put a checkout's files back as a snapshot has them: (folder, commit, when, whose row).
    Restore(PathBuf, String, String, TermId),
    Split(TermId, crate::layout::Dir),
    RightClicks(TermId),
    RenameProject(String, String),
    /// End everything in a project and forget it.
    CloseProject(usize),
    NewWorktree(usize),
    /// git init and a first commit, so agents there can get worktrees.
    GitInit(PathBuf),
    OpenWorktree(usize),
    /// The clean-up list for a project's worktrees.
    TidyOpen(usize),
    /// Remove these worktrees (each: its folder, and whether to lose uncommitted changes).
    Tidy(Vec<(PathBuf, bool)>),
    /// Install the newest seshi without interrupting the running sessions.
    Update,
    InstallStatus(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Confirm {
    /// The title bar ("Close pane", "changes").
    pub title: String,
    /// Dim next to the title ("dsa-tool").
    pub sub: String,
    /// What it's about (bold) and a dim detail after it.
    pub what: String,
    pub detail: String,
    /// A plain line of explanation.
    pub note: String,
    /// A short list under the note ("What's new"), or none.
    pub list: Vec<String>,
    /// The button that does it, and its key ("Close", "Enter").
    pub yes: String,
    pub key: char,
    /// Red (destructive) or the accent.
    pub danger: bool,
    pub act: Act,
}

/// Items a confirm's list shows before "…and N more".
const CONFIRM_LIST_MAX: usize = 8;

/// A modal like everything else: what it's about, a line of explanation, the action
/// (red when it destroys something) and Cancel.
pub(super) fn draw_confirm(app: &mut App, f: &mut ratatui::Frame, area: ratatui::layout::Rect, t: &crate::theme::Theme, c: &Confirm) {
    use super::design::{put, seg};
    use super::hydra::{HyHit, dim_all, hit, hovered, panel};
    use ratatui::layout::Rect;
    use ratatui::style::{Modifier, Style};
    let buf = f.buffer_mut();
    dim_all(buf, area, t);
    let widest_item = c.list.iter().map(|l| l.chars().count() + 10).max().unwrap_or(0);
    let w = (c.what.chars().count() + c.detail.chars().count() + 12).max(c.note.chars().count() + 8).max(widest_item).clamp(58, 90) as u16;
    // The list (its heading, at most CONFIRM_LIST_MAX items and a "more" line), then a gap.
    let shown = c.list.len().min(CONFIRM_LIST_MAX);
    let more = c.list.len() - shown;
    let list_h = if c.list.is_empty() { 0 } else { 2 + shown as u16 + u16::from(more > 0) };
    let r = panel(app, buf, area, w, 9 + list_h, &c.title, &[], t);
    if !c.sub.is_empty() {
        put(buf, r.x + 3 + c.title.chars().count() as u16, r.y, &[seg(format!("  {}", c.sub), Style::default().bg(t.accent).fg(t.acc_ink))], r.right().saturating_sub(12));
    }
    let card = Style::default().bg(t.card);
    put(buf, r.x + 3, r.y + 2, &[seg(c.what.clone(), card.fg(t.strong).add_modifier(Modifier::BOLD)), seg(format!("   {}", c.detail), card.fg(t.muted))], r.right() - 2);
    put(buf, r.x + 3, r.y + 3, &[seg(c.note.clone(), card.fg(t.text))], r.right() - 2);
    if !c.list.is_empty() {
        put(buf, r.x + 3, r.y + 5, &[seg("What's new".to_string(), card.fg(t.muted).add_modifier(Modifier::BOLD))], r.right() - 2);
        for (i, item) in c.list.iter().take(shown).enumerate() {
            put(buf, r.x + 3, r.y + 6 + i as u16, &[seg("• ".to_string(), card.fg(t.accent)), seg(item.clone(), card.fg(t.text))], r.right() - 2);
        }
        if more > 0 {
            put(buf, r.x + 5, r.y + 6 + shown as u16, &[seg(format!("…and {more} more"), card.fg(t.muted))], r.right() - 2);
        }
    }
    let buttons_y = r.y + 6 + list_h;
    let key = if c.key == '\n' { "Enter".to_string() } else { c.key.to_string() };
    let yes = button(t, &c.yes, &key, if c.danger { BtnKind::Danger } else { BtnKind::Primary }, false);
    let yw = segs_width(&yes);
    let yr = Rect { x: r.x + 3, y: buttons_y, width: yw, height: 1 };
    put(buf, yr.x, yr.y, &yes, r.right());
    let nr = Rect { x: yr.right() + 2, y: buttons_y, width: 12, height: 1 };
    put(buf, nr.x, nr.y, &button(t, "Cancel", "Esc", BtnKind::Normal, hovered(app, nr)), r.right());
    hit(app, area, HyHit::ConfirmNo);
    hit(app, r, HyHit::Noop);
    hit(app, yr, HyHit::ConfirmYes);
    hit(app, nr, HyHit::ConfirmNo);
}

#[derive(Debug, Clone, PartialEq)]
pub struct HyMenu {
    pub title: String,
    pub items: Vec<(String, Act)>,
    pub sel: usize,
    pub at: (u16, u16),
}

impl App {
    pub(super) fn menu(&mut self, title: String, mut items: Vec<(String, Act)>, at: (u16, u16)) {
        // The destructive item last.
        items.sort_by_key(|(_, a)| matches!(a, Act::End(_) | Act::CloseProject(_)));
        self.mode = Mode::HyMenu(Box::new(HyMenu { title, items, sel: 0, at }));
    }

    /// Right-click on a pane's content.
    pub(super) fn menu_for_pane(&mut self, term: TermId, at: (u16, u16)) {
        if Some(term) != self.focused() {
            self.cmd(Command::FocusPane { term });
        }
        let model = self.hy_model();
        let agent = find(&model, term).map(|(_, _, s)| s.agent.clone()).unwrap_or_default();
        let mut items = vec![
            ("Rename pane".to_string(), Act::RenamePane(term)),
            ("Split right".to_string(), Act::Split(term, crate::layout::Dir::Right)),
            ("Split down".to_string(), Act::Split(term, crate::layout::Dir::Down)),
        ];
        items.push((
            if self.hy.right_clicks.contains(&term) { "Stop sending right-clicks to pane" } else { "Send right-clicks to pane" }.to_string(),
            Act::RightClicks(term),
        ));
        // A program seshi doesn't know (an agent started by an alias, say): teach it.
        if let Some(t) = self.snap.terms.get(&term).filter(|t| t.agent.is_none() && !t.is_shell() && !t.process.is_empty() && t.remote.is_none()) {
            items.push((format!("{} is an agent…", t.process), Act::TeachAgent(term)));
        }
        items.push(("Close pane".to_string(), Act::End(vec![term])));
        self.menu(if agent.is_empty() { "pane".into() } else { agent }, items, at);
    }

    /// Right-click on an agent (or shell) row.
    pub(super) fn menu_for_session(&mut self, term: TermId, at: (u16, u16)) {
        if let Some((title, items)) = self.session_items(term) {
            self.menu(title, items, at);
        }
    }

    /// A session's menu: its title and items, for right-click and for the sidebar's
    /// letter keys.
    pub(super) fn session_items(&self, term: TermId) -> Option<(String, Vec<(String, Act)>)> {
        let model = self.hy_model();
        let (_, _, s) = find(&model, term)?;
        let agent = s.agent.clone();
        let dir = find(&model, term).map(|(_, w, _)| w.path.clone());
        let info = self.snap.terms.get(&term);
        // A program seshi doesn't know can be taught.
        let mut items = Vec::new();
        if let Some(t) = info.filter(|t| t.agent.is_none() && !t.is_shell() && !t.process.is_empty() && t.remote.is_none()) {
            items.push((format!("{} is an agent…", t.process), Act::TeachAgent(term)));
        }
        if self.focused().is_some_and(|f| f != term) {
            items.push(("Open beside".into(), Act::Beside(term)));
        }
        items.push(("Rename".to_string(), Act::RenamePane(term)));
        if let Some(d) = dir.filter(|d| crate::gitfs::head(d).is_some()) {
            items.push(("Changes".to_string(), Act::Changes(d.clone())));
            if s.is_agent && self.cfg.snapshots.enabled {
                items.push(("Snapshots…".to_string(), Act::Snapshots(d, term)));
            }
        }
        // Its key comes from its label: f, not a (the foot teaches a for actions) or m
        // (a row's follow-up key).
        if s.is_agent && !s.asleep {
            items.push(("Fleet commander".to_string(), Act::Commander(term)));
        }
        if !s.subagents.is_empty() {
            let label = if self.hy.subs_closed.contains(&term) { "Show subagents" } else { "Hide subagents" };
            items.push((label.to_string(), Act::ToggleSubs(term)));
        }
        // Out of the way: until later, or for good until it has something for you.
        if s.aside.is_some() {
            items.push(("Bring back".to_string(), Act::Aside(term, None)));
        } else if s.is_agent && s.status != crate::protocol::Status::Blocked {
            items.push(("Snooze…".to_string(), Act::SnoozeMenu(term)));
        }
        if !s.queued.is_empty() {
            items.push((format!("Drop queued follow-ups ({})", s.queued.len()), Act::ClearQueue(term)));
        }
        items.push(("Close".to_string(), Act::End(vec![term])));
        // Titled by what it is (an agent, or the program's name), never its window title.
        let what = if s.is_agent { agent } else { info.map(|t| t.process.clone()).filter(|p| !p.is_empty()).unwrap_or_else(|| "shell".into()) };
        let title = if what == s.name { what } else { format!("{} · {}", s.name, truncate(&what, 30)) };
        Some((title, items))
    }

    /// A checkout's snapshots as a numbered list, newest first; picking one asks, then
    /// restores it.
    pub(super) fn snapshots_menu(&mut self, dir: PathBuf, term: TermId, kept: Vec<crate::snapshot::Snapshot>, at: (u16, u16)) {
        if kept.is_empty() {
            return self.notify("no snapshots here yet: one is kept each time an agent's turn begins".into(), false);
        }
        let items = kept
            .iter()
            .take(SNAPSHOTS_LISTED)
            .enumerate()
            .map(|(i, s)| {
                let when = format!("{} ago · {}", super::hydra::age(s.at), s.why);
                (format!("{}  {when}", i + 1), Act::Restore(dir.clone(), s.commit.clone(), when, term))
            })
            .collect();
        self.menu(format!("snapshots · {}", super::hydra::folder_name(&dir)), items, at);
    }

    /// Fold / unfold a session's subagents.
    pub(super) fn toggle_subs(&mut self, term: TermId) {
        if !self.hy.subs_closed.remove(&term) {
            self.hy.subs_closed.insert(term);
        }
    }

    /// Fold / unfold a project's settled group.
    pub(super) fn toggle_aside(&mut self, key: String) {
        if !self.hy.aside_open.remove(&key) {
            self.hy.aside_open.insert(key);
        }
    }

    /// Right-click on a project row.
    pub(super) fn menu_for_project(&mut self, pi: usize, at: (u16, u16)) {
        if let Some((title, items)) = self.project_items(pi) {
            self.menu(title, items, at);
        }
    }

    pub(super) fn project_items(&self, pi: usize) -> Option<(String, Vec<(String, Act)>)> {
        let model = self.hy_model();
        let p = model.get(pi)?;
        let mut items = vec![("Rename".to_string(), Act::RenameProject(p.key.clone(), p.name.clone())), ("Close".to_string(), Act::CloseProject(pi))];
        if p.git {
            items.push(("New worktree".to_string(), Act::NewWorktree(pi)));
            items.push(("Open worktree…".to_string(), Act::OpenWorktree(pi)));
            items.push(("Clean up worktrees…".to_string(), Act::TidyOpen(pi)));
        } else {
            items.push(("Make it a git repo…".to_string(), Act::GitInit(p.path.clone())));
        }
        let aside = p.sessions().filter(|s| s.aside.is_some()).count();
        if aside > 0 {
            let label = if self.hy.aside_open.contains(&p.key) { "Hide settled".to_string() } else { format!("Show settled ({aside})") };
            items.push((label, Act::ToggleAside(p.key.clone())));
        }
        Some((p.name.clone(), items))
    }

    /// The menu items for the sidebar cursor's row (what its letter keys do), in the
    /// menu's order.
    pub(super) fn cursor_items(&self) -> Vec<(String, Act)> {
        let mut items = match (&self.hy.cursor_proj, self.hy.cursor) {
            (Some(key), _) => self.hy_model().iter().position(|p| p.key == *key).and_then(|pi| self.project_items(pi)),
            (None, Some(t)) => self.session_items(t),
            _ => None,
        }
        .map(|(_, i)| i)
        .unwrap_or_default();
        items.sort_by_key(|(_, a)| matches!(a, Act::End(_) | Act::CloseProject(_)));
        items
    }

    pub(super) fn on_hy_menu_key(&mut self, mut m: HyMenu, k: &KeyEvent) {
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Down | KeyCode::Char('j') => {
                m.sel = (m.sel + 1) % m.items.len().max(1);
                self.mode = Mode::HyMenu(Box::new(m));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                m.sel = (m.sel + m.items.len().max(1) - 1) % m.items.len().max(1);
                self.mode = Mode::HyMenu(Box::new(m));
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                if let Some((_, a)) = m.items.get(m.sel).cloned() {
                    self.menu_act(a);
                }
            }
            // Delete: its close item, like x.
            KeyCode::Delete if m.items.iter().any(|(_, a)| matches!(a, Act::End(_) | Act::CloseProject(_))) => {
                self.mode = Mode::Normal;
                if let Some((_, a)) = m.items.iter().find(|(_, a)| matches!(a, Act::End(_) | Act::CloseProject(_))).cloned() {
                    self.menu_act(a);
                }
            }
            // An item's key picks it.
            KeyCode::Char(c) if menu_keys(&m.items).contains(&Some(c.to_ascii_lowercase())) => {
                let i = menu_keys(&m.items).iter().position(|k| *k == Some(c.to_ascii_lowercase())).unwrap_or(0);
                self.mode = Mode::Normal;
                if let Some((_, a)) = m.items.get(i).cloned() {
                    self.menu_act(a);
                }
            }
            // A numbered list (presets): its number picks it.
            KeyCode::Char(c @ '1'..='9') if m.items.iter().any(|(l, _)| l.starts_with("1 ")) => {
                let i = c as usize - '1' as usize;
                if let Some((_, a)) = m.items.get(i).cloned() {
                    self.mode = Mode::Normal;
                    self.menu_act(a);
                } else {
                    self.mode = Mode::HyMenu(Box::new(m));
                }
            }
            _ => self.mode = Mode::HyMenu(Box::new(m)),
        }
    }

    pub(super) fn menu_pick(&mut self, i: usize) {
        let Mode::HyMenu(m) = std::mem::replace(&mut self.mode, Mode::Normal) else { return };
        if let Some((_, a)) = m.items.get(i).cloned() {
            self.menu_act(a);
        }
    }

    /// What a menu item does; closing things asks first.
    pub(super) fn menu_act(&mut self, a: Act) {
        let model = self.hy_model();
        let ask: Option<Confirm> = match &a {
            Act::End(ts) if ts.len() == 1 => {
                let name = find(&model, ts[0]).map(|(_, _, s)| s.name.clone()).unwrap_or_else(|| "this pane".into());
                let path = self.snap.terms.get(&ts[0]).map(|t| t.cwd.display().to_string()).unwrap_or_default();
                let running = self.snap.terms.get(&ts[0]).is_some_and(|t| t.status == crate::protocol::Status::Working);
                Some(Confirm {
                    title: "Close pane".into(),
                    sub: String::new(),
                    what: name,
                    detail: path,
                    note: if running { "It's still working; the program running in it will stop.".into() } else { "The program running in it will stop.".into() },
                    list: Vec::new(),
                    yes: "Close".into(),
                    key: '\n',
                    danger: true,
                    act: a.clone(),
                })
            }
            Act::End(ts) => Some(Confirm {
                title: "Close panes".into(),
                sub: String::new(),
                what: format!("{} panes", ts.len()),
                detail: String::new(),
                note: "The programs running in them will stop.".into(),
                list: Vec::new(),
                yes: "Close".into(),
                key: '\n',
                danger: true,
                act: a.clone(),
            }),
            Act::Restore(dir, _, when, term) => {
                let working = self.snap.terms.get(term).is_some_and(|x| x.status == crate::protocol::Status::Working);
                Some(Confirm {
                    title: "Restore snapshot".into(),
                    sub: String::new(),
                    what: super::hydra::folder_name(dir),
                    detail: when.clone(),
                    note: if working {
                        "Its agent is working in these files right now. They go back to how they were then; what's there now is kept as a snapshot first.".into()
                    } else {
                        "The files go back to how they were then. What's there now is kept as a snapshot first, so this can be taken back.".into()
                    },
                    list: Vec::new(),
                    yes: "Restore".into(),
                    key: '\n',
                    danger: working,
                    act: a.clone(),
                })
            }
            Act::GitInit(p) => Some(Confirm {
                title: "Make it a git repo".into(),
                sub: String::new(),
                what: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                detail: p.display().to_string(),
                note: "git init, then what's there as the first commit. Agents then get worktrees.".into(),
                list: Vec::new(),
                yes: "Make it a git repo".into(),
                key: 'g',
                danger: false,
                act: a.clone(),
            }),
            // It types into the agent, so never on a slip of a key. (One that's on a prompt is
            // told to answer first, with nothing to confirm.)
            Act::Commander(t) if self.snap.terms.get(t).is_some_and(|x| x.status != crate::protocol::Status::Blocked) => find(&model, *t).map(|(_, _, s)| Confirm {
                title: "Fleet commander".into(),
                sub: String::new(),
                what: s.name.clone(),
                detail: s.agent.clone(),
                note: "It's told to run seshi brief and ask you for the goal; from then on it starts helpers.".into(),
                list: Vec::new(),
                yes: "Make it commander".into(),
                key: '\n',
                danger: false,
                act: a.clone(),
            }),
            Act::CloseProject(pi) => model.get(*pi).map(|p| {
                let n = p.sessions().count();
                Confirm {
                    title: "Close project".into(),
                    sub: String::new(),
                    what: p.name.clone(),
                    detail: format!("{n} pane{}", if n == 1 { "" } else { "s" }),
                    note: "Everything running in it stops; the folder stays where it is.".into(),
                    list: Vec::new(),
                    yes: "Close".into(),
                    key: '\n',
                    danger: true,
                    act: a.clone(),
                }
            }),
            _ => None,
        };
        match ask {
            Some(c) => {
                // Asked from the sidebar: come back to it afterwards, on the next row (the
                // one asked about is going), or the same row when it stays.
                // The cursor is still on a row when the leader key or the actions list was
                // opened from the sidebar: that's the sidebar too.
                let from_side = self.mode == Mode::Side || self.hy.cursor.is_some() || self.hy.cursor_proj.is_some();
                self.hy.side_return = match &c.act {
                    _ if !from_side => None,
                    Act::Commander(t) | Act::Restore(_, _, _, t) => Some(super::hydra::SideItem::Sess(*t)),
                    _ => self.side_after_close(),
                };
                self.mode = Mode::Confirm(Box::new(c));
            }
            None => self.menu_do(a),
        }
    }

    pub(super) fn menu_do(&mut self, a: Act) {
        match a {
            Act::RenamePane(t) => {
                let cur = self.snap.terms.get(&t).map(|x| x.label.clone()).unwrap_or_default();
                self.mode = Mode::Prompt { kind: super::PromptKind::RenamePane(t), input: cur };
            }
            Act::Split(t, dir) => {
                let cwd = self.snap.terms.get(&t).map(|x| x.top.clone().unwrap_or_else(|| x.cwd.clone())).unwrap_or_else(|| self.here_dir());
                self.hy.split_dir = Some(dir);
                if self.focused() != Some(t) {
                    self.cmd(Command::FocusPane { term: t });
                }
                self.hy.pending_split = Some((t, std::time::Instant::now()));
                self.cmd(Command::NewWorkspace { cwd: Some(cwd), name: None, cmd: None });
            }
            Act::Update => self.start_update(),
            Act::InstallStatus(agent) => {
                self.notify(format!("setting up {agent} status…"), false);
                self.spawn_bg(move || {
                    let result = crate::cli::integrations::install_status(&agent);
                    super::Bg::Then(Box::new(move |app: &mut App| match result {
                        Ok(_) => app.notify(format!("{agent} status installed; restart {agent} to load it (Pi also supports /reload)"), false),
                        Err(e) => app.notify(format!("couldn't set up {agent} status: {e:#}"), true),
                    }))
                });
            }
            Act::GitInit(dir) => {
                self.notify("making it a git repo…".into(), false);
                self.spawn_bg(move || {
                    let result = git_init(&dir);
                    super::Bg::Then(Box::new(move |app: &mut App| {
                        match result {
                            Ok(()) => app.notify("it's a git repo now; new agents get their own worktree".into(), false),
                            Err(e) => app.notify(e, true),
                        }
                        app.hy_fresh();
                    }))
                });
            }
            Act::TeachAgent(term) => self.cmd(Command::TeachAgent { term }),
            Act::TidyOpen(pi) => self.tidy_open(pi),
            Act::ClearQueue(term) => self.cmd(Command::ClearQueue { term }),
            Act::SnoozeMenu(term) => {
                let at = self.hy.row_y.get(&term).map(|y| (self.hy.saved.side_w.unwrap_or(0), *y)).unwrap_or((0, 0));
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let items = SNOOZES.iter().enumerate().map(|(i, (label, secs))| (format!("{}  {label}", i + 1), Act::Aside(term, Some(secs.map_or(0, |s| now + s))))).collect();
                self.menu("snooze".into(), items, at);
            }
            Act::Aside(term, until) => {
                self.cmd(Command::SetAside { term, until });
                let who = self.snap.terms.get(&term).map(|t| t.display_name()).unwrap_or_default();
                self.notify(
                    match until {
                        None => format!("{who} is back"),
                        Some(0) => format!("{who} is settled: it comes back when it has something for you"),
                        Some(_) => format!("{who} is snoozed: it comes back then, or sooner if it needs you"),
                    },
                    false,
                );
            }
            Act::ToggleAside(key) => self.toggle_aside(key),
            Act::ToggleSubs(term) => self.toggle_subs(term),
            Act::Snapshots(dir, term) => {
                let at = self.hy.row_y.get(&term).map(|y| (self.hy.saved.side_w.unwrap_or(0), *y)).unwrap_or((0, 0));
                self.spawn_bg(move || {
                    let kept = crate::snapshot::list(&dir);
                    super::Bg::Then(Box::new(move |app: &mut App| app.snapshots_menu(dir, term, kept, at)))
                });
            }
            Act::Restore(dir, commit, _, _) => {
                self.notify("restoring…".into(), false);
                self.spawn_bg(move || {
                    let result = crate::snapshot::restore(&dir, &commit);
                    super::Bg::Then(Box::new(move |app: &mut App| {
                        match result {
                            Ok(0) => app.notify("the files are already as they were then".into(), false),
                            Ok(n) => app.notify(format!("Restored {n} file{} · the newest snapshot takes it back", if n == 1 { "" } else { "s" }), false),
                            Err(e) => app.notify(format!("couldn't restore: {e}"), true),
                        }
                        app.hy_fresh();
                    }))
                });
            }
            Act::Tidy(items) => {
                self.notify("Cleaning up worktrees…".into(), false);
                self.cmd(Command::TidyWorktrees { items });
            }
            // Typed into a prompt it would be taken as that prompt's answer.
            Act::Commander(term) if self.snap.terms.get(&term).is_some_and(|t| t.status == crate::protocol::Status::Blocked) => {
                self.notify("it's waiting on you: answer it first".into(), true);
            }
            Act::Commander(term) => self.send_follow_up(term, &crate::brief::commander_prompt("")),
            Act::RightClicks(t) => {
                if !self.hy.right_clicks.remove(&t) {
                    self.hy.right_clicks.insert(t);
                    self.notify("right-clicks go to this pane now (Shift+right-click for seshi's menu)".into(), false);
                }
            }
            Act::RenameProject(key, name) => self.mode = Mode::Prompt { kind: super::PromptKind::RenameProject(key), input: name },
            Act::CloseProject(pi) => {
                let model = self.hy_model();
                if let Some(p) = model.get(pi) {
                    for s in p.sessions() {
                        self.cmd(Command::ClosePane { term: s.term });
                    }
                }
            }
            Act::NewWorktree(pi) => {
                self.hy_new(pi, false);
                if let Mode::HyPane(np) = &mut self.mode {
                    np.place = Some(0);
                }
            }
            Act::OpenWorktree(pi) => {
                let model = self.hy_model();
                let Some(p) = model.get(pi) else { return };
                let key = super::design::path_key(&p.path);
                let ws = self.snap.workspaces.iter().find(|w| {
                    w.tabs.iter().flat_map(|t| t.layout.leaves()).any(|id| self.snap.terms.get(&id).and_then(|t| t.root.as_ref()).is_some_and(|r| super::design::path_key(r) == key))
                });
                match ws.map(|w| w.id) {
                    Some(ws) => {
                        self.send(crate::protocol::ClientMsg::Query(crate::protocol::Query::Worktrees { ws }));
                        self.mode = Mode::Worktrees { ws, cmd: None, items: None, query: String::new(), sel: 0 };
                    }
                    None => self.notify("start something in this project first".into(), true),
                }
            }
            Act::Beside(t) => {
                if let Some(f) = self.focused() {
                    self.hy_unshow(t);
                    self.hy.pending_split = Some((f, std::time::Instant::now()));
                    self.cmd(Command::FocusPane { term: t });
                }
            }
            Act::End(ts) => {
                for t in ts {
                    // Out of its split first: the pane beside it takes the room (and the
                    // focus), so nothing else slides into its place.
                    self.hy_unshow(t);
                    self.cmd(Command::ClosePane { term: t });
                }
            }
            Act::Changes(p) => self.open_changes(p),
        }
    }
}

/// What a new repo ignores when it has no .gitignore of its own, so its first commit
/// doesn't take in dependencies, build output or secrets.
const DEFAULT_GITIGNORE: &str = "node_modules/\ntarget/\ndist/\nbuild/\n.venv/\n__pycache__/\n.env\n.env.*\n*.log\n.DS_Store\n";

/// `git init`, a .gitignore when there isn't one, and a first commit (worktrees need one).
fn git_init(dir: &std::path::Path) -> Result<(), String> {
    let run = |args: &[&str]| {
        let mut c = std::process::Command::new("git");
        c.arg("-C").arg(dir).args(args);
        crate::proc::quiet(&mut c);
        match c.output() {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => Err(String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("git failed").to_string()),
            Err(e) => Err(format!("couldn't run git: {e}")),
        }
    };
    run(&["init", "-q"])?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, DEFAULT_GITIGNORE).map_err(|e| format!("couldn't write .gitignore: {e}"))?;
    }
    run(&["add", "-A"])?;
    run(&["commit", "-q", "-m", "Initial commit"]).map_err(|e| format!("the first commit failed ({e}); is git set up with your name and email?"))
}

/// A key for each item: the first letter of its label not taken yet (j and k move).
/// Numbered lists (presets) use their numbers.
pub(super) fn menu_keys(items: &[(String, Act)]) -> Vec<Option<char>> {
    // Taken already where a row's letters work (the sidebar): moving, actions, follow-up, why.
    let mut used: Vec<char> = vec!['j', 'k', 'a', 'm', 'i'];
    // Closing is always x.
    let closes = |a: &Act| matches!(a, Act::End(_) | Act::CloseProject(_));
    if items.iter().any(|(_, a)| closes(a)) {
        used.push('x');
    }
    items
        .iter()
        .map(|(l, a)| {
            if closes(a) {
                return Some('x');
            }
            if l.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return None;
            }
            let c = l.chars().filter(|c| c.is_ascii_alphabetic()).map(|c| c.to_ascii_lowercase()).find(|c| !used.contains(c))?;
            used.push(c);
            Some(c)
        })
        .collect()
}

pub(super) fn draw_menu(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, m: &HyMenu) {
    let buf = f.buffer_mut();
    let keys = menu_keys(&m.items);
    let danger = |a: &Act| matches!(a, Act::End(_) | Act::CloseProject(_));
    // The destructive item sits last, after a blank row.
    let gap = m.items.iter().any(|(_, a)| danger(a)) && m.items.len() > 1;
    let w = m.items.iter().map(|(l, _)| l.width() as u16).max().unwrap_or(10).max(m.title.width() as u16).clamp(16, 44) + 10;
    let h = m.items.len() as u16 + 3 + gap as u16;
    let x = m.at.0.min(area.right().saturating_sub(w + 1)).max(area.x);
    let y = if m.at.1 + h < area.bottom() { m.at.1 + 1 } else { m.at.1.saturating_sub(h) }.max(area.y);
    let r = Rect { x, y, width: w, height: h + 1 };
    // Clicking anywhere else closes it.
    hit(app, area, HyHit::Close);
    // The same card as every popup: its title lit in the border, a ✕ that closes it.
    let bgm = t.card;
    hit(app, r, HyHit::Noop);
    let title = truncate(&m.title, (w - 10) as usize);
    let mut c = super::hydra::Card::new(t, &title).lit(t.accent);
    c.bg = bgm;
    c.close = Some(HyHit::Close);
    super::hydra::card(app, buf, r, &c, t);
    let look = super::hydra::Look::of(&app.cfg.ui);
    let mut yy = r.y + 2;
    for (i, (label, act)) in m.items.iter().enumerate() {
        if gap && danger(act) && i + 1 == m.items.len() {
            yy += 1;
        }
        let row = Rect { x: r.x + 1, y: yy, width: w - 2, height: 1 };
        let on = i == m.sel || hovered(app, row);
        let bg = if on { t.hov } else { bgm };
        if on {
            super::hydra::row_pill(look, f.buffer_mut(), row.x, yy, row.width, bg, bgm);
        }
        if i == m.sel {
            put(f.buffer_mut(), row.x + 1, yy, &[seg("›", Style::default().fg(t.accent).bg(bg).add_modifier(Modifier::BOLD))], row.right());
        }
        let mut ls = Style::default().fg(if danger(act) { t.err } else { t.strong }).bg(bg);
        if i == m.sel {
            ls = ls.add_modifier(Modifier::BOLD);
        }
        put(f.buffer_mut(), row.x + 3, yy, &[seg(label.clone(), ls)], row.right().saturating_sub(4));
        if let Some(k) = keys.get(i).copied().flatten() {
            put(f.buffer_mut(), row.right().saturating_sub(3), yy, &[seg(k.to_string(), Style::default().fg(t.accent).bg(bg).add_modifier(Modifier::BOLD))], row.right());
        }
        hit(app, row, HyHit::MenuPick(i));
        yy += 1;
    }
}
