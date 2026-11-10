//! A machine's connection as the rest of the screen words it: its label, its state mark, and
//! whether its line is down.
//!
//! Everything is derived from the machine's `Remote` each frame, so a machine that comes back
//! is drawn normally again with nothing to reset.

use super::*;
use crate::brand::APP_NAME;
use crate::client::link::ConnectionState;
use crate::hidden::sanitize;
use crate::link_failure::{FIX_INSTALL, FIX_UPDATE_HERE};

/// This computer's mark on the pane badge, and another machine's (here and on the sidebar).
pub(super) const LOCAL_ICON: &str = "⌂";
pub(super) const REMOTE_ICON: &str = "⇄";

/// What the header calls the machine the window runs on, once there is another to tell it from.
pub(super) const THIS_COMPUTER: &str = "this computer";

/// A connection state as the header says it: a glyph, a word, and the role that colours both.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct StateMark {
    pub glyph: &'static str,
    pub word: String,
    pub color: Color,
}

/// What `machine update` does for a machine that needs it: put the app there, or replace the
/// version it has. Told apart by the fix text the connection carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum AppFix {
    Install,
    Update,
    /// The app there is already current; only its daemon, kept running by open sessions, is old.
    Restart,
}

impl AppFix {
    pub fn of(state: &ConnectionState) -> Option<AppFix> {
        match state {
            ConnectionState::NeedsUpdate { why } if why == FIX_INSTALL => Some(AppFix::Install),
            // Newer there than here: nothing to do to that machine, so no button for it.
            ConnectionState::NeedsUpdate { why } if why == FIX_UPDATE_HERE => None,
            ConnectionState::NeedsUpdate { .. } => Some(AppFix::Update),
            _ => None,
        }
    }

    /// The one word on a chip or a key hint.
    pub fn verb(self) -> &'static str {
        match self {
            AppFix::Install => "install",
            AppFix::Update => "update",
            AppFix::Restart => "restart",
        }
    }

    /// The item of a menu.
    pub fn label(self) -> String {
        match self {
            AppFix::Install => format!("Install {APP_NAME}"),
            AppFix::Update => format!("Update {APP_NAME}"),
            AppFix::Restart => format!("Restart {APP_NAME}"),
        }
    }

    /// The toast when a machine is found in need: what it lacks and where the fix is.
    pub fn notice(self, name: &str) -> String {
        let (lacks, item) = match self {
            AppFix::Install => (format!("has no {APP_NAME}"), "install"),
            AppFix::Update => (format!("runs another version of {APP_NAME}"), "update"),
            AppFix::Restart => (format!("needs a restart to finish updating {APP_NAME}"), "restart"),
        };
        format!(
            "{name} {lacks}: click {item} on its sidebar heading, or press u on it in Machines"
        )
    }
}

pub(super) fn state_mark(state: &ConnectionState, ascii: bool, t: &Theme) -> StateMark {
    let pick = |unicode, plain| if ascii { plain } else { unicode };
    let (glyph, word, color) = match state {
        ConnectionState::Connecting => (pick("◌", "."), "connecting".to_string(), t.muted),
        ConnectionState::Connected => (pick("●", "+"), "connected".to_string(), t.done),
        ConnectionState::Reconnecting { attempt, .. } => {
            (pick("↻", "~"), format!("reconnecting #{attempt}"), t.muted)
        }
        ConnectionState::NeedsLogin { .. } => (pick("✕", "x"), "needs login".to_string(), t.err),
        ConnectionState::NeedsUpdate { why } if why == FIX_UPDATE_HERE => {
            (pick("✕", "x"), "newer than this app".to_string(), t.err)
        }
        ConnectionState::NeedsUpdate { .. } => (pick("✕", "x"), "needs update".to_string(), t.err),
        ConnectionState::Unreachable { .. } => (pick("✕", "x"), "unreachable".to_string(), t.err),
    };
    StateMark { glyph, word, color }
}

impl App {
    pub(super) fn machine_down(&self, machine: MachineId) -> bool {
        // This computer's sessions are as good as gone to the window while its daemon is away.
        if machine == MachineId::LOCAL && self.service != super::service::Service::Up {
            return true;
        }
        self.remotes
            .get(&machine)
            .is_some_and(|remote| remote.state.is_down())
    }

    fn machine_label(&self, machine: MachineId) -> String {
        match self.remotes.get(&machine) {
            Some(remote) => sanitize(&remote.name, false),
            None => THIS_COMPUTER.to_string(),
        }
    }

    /// The name a remote machine's items carry; empty for this computer's, which carry none.
    pub(super) fn machine_label_of(&self, machine: MachineId) -> String {
        if machine == MachineId::LOCAL {
            String::new()
        } else {
            self.machine_label(machine)
        }
    }

    /// The machine's connection as one dot: `done` when it is up, amber while it connects or
    /// reconnects, `err` when it cannot be reached (it needs a login or an update, or is down).
    pub(super) fn machine_dot(&self, machine: MachineId, t: &Theme) -> (&'static str, Color) {
        let colour = match self.remotes.get(&machine).map(|r| &r.state) {
            Some(ConnectionState::Connected) => t.done,
            Some(ConnectionState::Connecting | ConnectionState::Reconnecting { .. }) => t.blocked,
            _ => t.err,
        };
        ("●", colour)
    }

    pub(super) fn ascii_marks(&self) -> bool {
        glyphs::chosen(&self.cfg.ui.glyphs, &self.cfg.icons)
            .needs
            .is_ascii()
    }

    pub(super) fn machine_mark(&self, machine: MachineId, t: &Theme) -> Option<StateMark> {
        let remote = self.remotes.get(&machine)?;
        Some(state_mark(&remote.state, self.ascii_marks(), t))
    }

    /// The app needs installing or updating there. A refused machine is about a different daemon
    /// answering, which `machine update` does not fix.
    pub(in crate::client) fn machine_fix(&self, machine: MachineId) -> Option<AppFix> {
        if self.refused.contains_key(&machine) {
            return None;
        }
        let fix = AppFix::of(&self.remotes.get(&machine)?.state)?;
        Some(match fix {
            AppFix::Update if self.machine_restart_pending(machine) => AppFix::Restart,
            other => other,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link_failure::FIX_UPDATE;

    #[test]
    fn the_fix_text_tells_an_install_from_an_update_and_nothing_else_needs_one() {
        let needs = |why: &str| ConnectionState::NeedsUpdate { why: why.into() };
        assert_eq!(AppFix::of(&needs(FIX_INSTALL)), Some(AppFix::Install));
        assert_eq!(AppFix::of(&needs(FIX_UPDATE)), Some(AppFix::Update));
        assert_eq!(AppFix::of(&needs(FIX_UPDATE_HERE)), None);
        assert_eq!(AppFix::Restart.verb(), "restart");
        assert_eq!(AppFix::Restart.label(), format!("Restart {APP_NAME}"));
        assert_eq!(AppFix::of(&ConnectionState::Connected), None);
        assert_eq!(
            AppFix::of(&ConnectionState::NeedsLogin { why: FIX_INSTALL.into() }),
            None
        );
    }
}

