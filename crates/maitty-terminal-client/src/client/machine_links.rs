//! What the window does with its supervisors' news: register a new line, remember and tell the
//! state of each machine, and count the typing a down machine could not take.

use super::link::{ConnectionState, LinkEvent};
use super::*;
use maitty_settings::config::MachineDef;

impl App {
    pub(super) fn on_link_event(&mut self, event: LinkEvent, connections: &mut Connections) {
        match event {
            // A stopped supervisor's last events can still be in the queue. A line dropped
            // here ends its ssh.
            LinkEvent::Line(machine, _) | LinkEvent::State { machine, .. }
                if self.machine_is_forgotten(machine) => {}
            LinkEvent::Line(machine, halves) => {
                // A fresh line is a fresh look at who answers: the same wrong daemon is
                // refused again on its first State, a fixed one is let in.
                self.refused.remove(&machine);
                connections.add(machine, halves);
            }
            LinkEvent::State {
                machine,
                name,
                state,
            } => self.on_link_state(machine, name, state),
        }
    }

    /// Typing for a machine that is down is dropped (see `unsent_input`); it is counted so the
    /// person is told when the machine is back.
    pub(super) fn note_unsent_input(&mut self, machine: MachineId) {
        *self.unsent_input.entry(machine).or_default() += 1;
    }

    pub(super) fn on_link_state(
        &mut self,
        machine: MachineId,
        name: String,
        state: ConnectionState,
    ) {
        let remote = self.remotes.entry(machine).or_default();
        if remote.name.is_empty() {
            remote.name = name.clone();
        }
        let before = std::mem::replace(&mut remote.state, state.clone());
        remote.down_since = state.is_down().then(|| {
            remote
                .down_since
                .or(state.since())
                .unwrap_or_else(Instant::now)
        });
        self.screen_fresh();
        self.dirty = true;
        // The refusal's own words are already up; saying "unreachable" over them hides why.
        if before.kind() == state.kind() || self.refused.contains_key(&machine) {
            return;
        }
        match state {
            ConnectionState::Connecting => {}
            ConnectionState::Connected => self.tell_connected(machine, &name),
            ConnectionState::Reconnecting { .. } => self.notify(
                format!("Lost the connection to {name}; reconnecting"),
                false,
            ),
            ConnectionState::NeedsLogin { why } | ConnectionState::NeedsUpdate { why } => {
                self.notify(format!("{name}: {why}"), true)
            }
            ConnectionState::Unreachable { why, .. } => {
                self.notify(format!("{name} is unreachable: {why}"), true)
            }
        }
    }

    fn tell_connected(&mut self, machine: MachineId, name: &str) {
        match self.unsent_input.remove(&machine) {
            Some(n) if n > 0 => {
                let (what, were) = if n == 1 {
                    ("keystroke", "was")
                } else {
                    ("keystrokes", "were")
                };
                self.notify(
                    format!("{n} {what} to {name} {were} not sent while it was disconnected"),
                    true,
                );
            }
            _ => self.notify(format!("Connected to {name}"), false),
        }
    }

    /// A machine's entry in the config changed or went: what was refused about it is looked at
    /// again, with a dial now. (A supervisor keeps the entry it started with until the app
    /// restarts; this only lets a fixed daemon in.)
    pub(super) fn machines_edited(&mut self, new: &[MachineDef]) {
        let edited: Vec<MachineId> = self
            .remotes
            .iter()
            .filter(|(id, remote)| {
                self.refused.contains_key(id) && !same_entry(&self.cfg.machines, new, &remote.name)
            })
            .map(|(id, _)| *id)
            .collect();
        for machine in edited {
            self.refused.remove(&machine);
            self.links.retry(machine);
        }
    }
}

fn same_entry(old: &[MachineDef], new: &[MachineDef], name: &str) -> bool {
    let find = |defs: &[MachineDef]| defs.iter().find(|d| d.name == name).cloned();
    find(old) == find(new)
}
