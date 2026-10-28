//! New sessions: leader `s` starts one straight away with the defaults in Settings ›
//! Sessions (the task is typed in the session). There is no form.

use super::*;

/// The two places a session can open; the same words as `sessions.where`.
pub(in crate::client) const WHERE_WORKTREE: usize = 0;
const WHERE_FOLDER: usize = 1;

const AGENT_LAST_USED: &str = "last used";
const AGENT_FALLBACK: &str = "claude";

/// The agents `s` can start: the configured quick agents and the built-in three, with no
/// shell, recipe or preset (those start from Menu › Shell here and the search).
pub(in crate::client) fn ns_agents(app: &App) -> Vec<String> {
    np_agents(app, 0).into_iter().filter(|a| a != "shell" && !a.starts_with('⚙') && !a.starts_with('★')).collect()
}

impl App {
    /// The agent `s` starts: `sessions.agent`, where "last used" is the last one started
    /// from the app (kept in `<app>-ui.json`) and claude before there is one.
    pub(in crate::client) fn ns_default_agent(&self) -> String {
        let agents = ns_agents(self);
        let pick = match self.cfg.sessions.agent.as_str() {
            AGENT_LAST_USED => self.screen.saved.last_agent.as_str(),
            named => named,
        };
        if agents.iter().any(|a| a == pick) { pick.to_string() } else { AGENT_FALLBACK.to_string() }
    }

    /// Where `s` opens: the project the sidebar cursor is on, else the one you are in.
    pub(in crate::client) fn ns_default_project(&self) -> usize {
        let model = self.screen_model();
        let key = self.screen.cursor.and_then(|c| model.iter().find(|p| p.sessions().any(|s| s.term == c)).map(|p| p.key.clone())).or(self.screen.proj.clone());
        model.iter().position(|p| Some(&p.key) == key.as_ref()).unwrap_or(0)
    }

    pub(in crate::client) fn ns_default_place(&self) -> usize {
        if self.cfg.sessions.place == crate::config::NEW_WHERE[WHERE_FOLDER] { WHERE_FOLDER } else { WHERE_WORKTREE }
    }

    /// `s`: a session now, with the defaults.
    pub(in crate::client) fn screen_start_session(&mut self, proj: Option<usize>) {
        let proj = proj.unwrap_or_else(|| self.ns_default_project());
        let agent = self.ns_default_agent();
        self.screen_start_agent(proj, &agent, self.ns_default_place() == WHERE_WORKTREE);
    }

    /// Start `agent` in project number `proj`: in its own new worktree when asked for one and
    /// the project is a git repo, else in the project's folder. Remembers the agent as the
    /// last used.
    pub(in crate::client) fn screen_start_agent(&mut self, proj: usize, agent: &str, worktree: bool) {
        let Some(p) = self.screen_model().get(proj).cloned() else {
            self.screen_open_finder();
            return;
        };
        let main = p.wts.iter().find(|w| w.main).map(|w| w.path.clone()).unwrap_or_else(|| p.path.clone());
        let cmd = np_command(self, agent, 0, "");
        if self.screen.saved.last_agent != agent {
            self.screen.saved.last_agent = agent.to_string();
            self.screen.save();
        }
        self.screen.cursor = None;
        if worktree && p.git {
            self.screen_new_worktree(&p, cmd, false, None);
        } else {
            self.screen_new_session(main, cmd, false);
        }
    }
}
