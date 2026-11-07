//! A machine's connection as the rest of the screen words it: its label, its state mark, and
//! whether its line is down.
//!
//! Everything is derived from the machine's `Remote` each frame, so a machine that comes back
//! is drawn normally again with nothing to reset.

use super::*;
use crate::client::link::ConnectionState;
use crate::hidden::sanitize;

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

pub(super) fn state_mark(state: &ConnectionState, ascii: bool, t: &Theme) -> StateMark {
    let pick = |unicode, plain| if ascii { plain } else { unicode };
    let (glyph, word, color) = match state {
        ConnectionState::Connecting => (pick("◌", "."), "connecting".to_string(), t.muted),
        ConnectionState::Connected => (pick("●", "+"), "connected".to_string(), t.done),
        ConnectionState::Reconnecting { attempt, .. } => {
            (pick("↻", "~"), format!("reconnecting #{attempt}"), t.muted)
        }
        ConnectionState::NeedsLogin { .. } => (pick("✕", "x"), "needs login".to_string(), t.err),
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
}

