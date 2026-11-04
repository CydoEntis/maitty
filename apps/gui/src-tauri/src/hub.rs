//! The host's connections to other machines (ADR 0048). One supervisor per configured machine
//! keeps its line up through `maitty-machines`; this takes what the supervisors and the lines
//! report and decides what the host may act on. The web view never connects.
//!
//! Everything a remote daemon says is vetted here, by the code the terminal app uses
//! (`remote_policy`), and a different daemon answering at a machine's name is turned away
//! (`machine_ids`). Nothing here knows about Tauri, so the whole of it is tested without a
//! window, against the fake `ssh` shim.

use maitty_machines::connections::{Burst, Connections, NotConnected, SERVER_BURST};
use maitty_machines::link::{
    ConnectionState, LinkControl, LinkEvent, Links, Timing, configured_machines,
};
use maitty_machines::machine_ids::{IdCheck, MachineIds, check_daemon_id};
use maitty_machines::remote_policy::{MachineOpts, vet};
use maitty_machines::{Dial, MachineDef, MachineId, Outgoing};
use maitty_shared::protocol::{ServerMsg, Surface};
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

/// What a connection says about the host in its Hello: it attaches to the panes this app owns
/// (ADR 0045) and asks for a pane's replay when a view opens on it. A desktop has no pane of
/// its own, so there is no claim to make.
fn dial() -> anyhow::Result<Dial> {
    Ok(Dial {
        attach: true,
        surface: Surface::Desktop,
        replays: false,
        term: None,
        token: None,
        structured: None,
    })
}

/// What the hub has to tell the host.
#[derive(Debug)]
pub enum Update {
    /// A machine's connection changed state.
    State {
        machine: MachineId,
        name: String,
        state: ConnectionState,
    },
    /// What a machine said, admitted and vetted, in the order it was said.
    Messages {
        machine: MachineId,
        msgs: Vec<ServerMsg>,
    },
    /// A machine's line ended; its messages came first.
    LineEnded { machine: MachineId },
}

pub struct Hub {
    links: Links,
    control: LinkControl,
    connections: Connections,
    ids: MachineIds,
    ids_file: PathBuf,
    names: BTreeMap<MachineId, String>,
    opts: BTreeMap<MachineId, MachineOpts>,
    /// Machines a different daemon answered for, with the reason. Cleared by a fresh line.
    refused: BTreeMap<MachineId, String>,
    queue: VecDeque<Update>,
}

impl Hub {
    /// Start a supervisor for each of `defs`, reached through `ssh`. Must run inside the
    /// runtime. With no machine nothing starts.
    pub fn start(defs: &[MachineDef], ssh: &str, ids_file: PathBuf, timing: Timing) -> Hub {
        let machines = configured_machines(defs, ssh);
        let names = machines
            .iter()
            .map(|(id, machine)| (*id, machine.name().to_owned()))
            .collect::<BTreeMap<_, _>>();
        let opts = names
            .iter()
            .map(|(id, name)| (*id, MachineOpts::named(defs, name)))
            .collect();
        let links = Links::start(machines, timing, dial);
        Hub {
            control: links.control(),
            links,
            connections: Connections::default(),
            ids: MachineIds::load(&ids_file),
            ids_file,
            names,
            opts,
            refused: BTreeMap::new(),
            queue: VecDeque::new(),
        }
    }

    /// Whether any machine is supervised: with none there is nothing to wait for.
    pub fn any(&self) -> bool {
        self.links.any()
    }

    pub fn name_of(&self, machine: MachineId) -> Option<&str> {
        self.names.get(&machine).map(String::as_str)
    }

    /// Dial the machine now, whatever the wait.
    pub fn retry(&self, machine: MachineId) -> bool {
        self.control.retry(machine)
    }

    /// Hand a message to its machine's line. Never waits.
    pub fn send(&self, outgoing: Outgoing) -> Result<(), NotConnected> {
        self.connections.send(outgoing)
    }

    /// The next thing to tell the host. Waits for ever with no machine.
    pub async fn next(&mut self) -> Update {
        loop {
            if let Some(update) = self.queue.pop_front() {
                return update;
            }
            tokio::select! {
                event = self.links.next() => self.on_link_event(event),
                burst = self.connections.recv_burst(SERVER_BURST) => self.on_burst(burst),
            }
        }
    }

    fn on_link_event(&mut self, event: LinkEvent) {
        match event {
            LinkEvent::Line(machine, halves) => {
                // A fresh line is a fresh look at who answers: the same wrong daemon is
                // refused again on its first State, a fixed one is let in.
                self.refused.remove(&machine);
                self.connections.add(machine, halves);
            }
            LinkEvent::State {
                machine,
                name,
                state,
            } => self.queue.push_back(Update::State {
                machine,
                name,
                state,
            }),
        }
    }

    fn on_burst(&mut self, burst: Burst) {
        let Burst {
            machine,
            msgs,
            lost,
            stalled,
        } = burst;
        let admitted: Vec<ServerMsg> = msgs
            .into_iter()
            .filter_map(|msg| self.admit(machine, msg))
            .collect();
        if !admitted.is_empty() {
            self.queue.push_back(Update::Messages {
                machine,
                msgs: admitted,
            });
        }
        if self.refused.contains_key(&machine) {
            self.connections.mark_lost(machine);
        }
        if lost {
            // Another machine is reported lost by its supervisor, which dials again.
            self.control.line_ended(machine, stalled);
            self.queue.push_back(Update::LineEnded { machine });
        }
    }

    /// The message to act on, or `None` when `machine` may not send it. A machine a different
    /// daemon answers for says nothing that counts, and what the others say is vetted.
    fn admit(&mut self, machine: MachineId, msg: ServerMsg) -> Option<ServerMsg> {
        if self.refused.contains_key(&machine) {
            return None;
        }
        let name = self.names.get(&machine)?.clone();
        if let ServerMsg::State(snapshot) = &msg {
            match check_daemon_id(&mut self.ids, &name, snapshot.daemon_id.as_deref()) {
                Ok(IdCheck::Recorded) => self.ids.save(&self.ids_file),
                Ok(_) => {}
                Err(mismatch) => {
                    tracing::warn!("{mismatch}");
                    let reason = mismatch.to_string();
                    self.control.refuse(machine, reason.clone());
                    self.refused.insert(machine, reason);
                    return None;
                }
            }
        }
        let opts = self.opts.get(&machine).copied().unwrap_or_default();
        // No request of the host's is waiting for an answer yet, so a reply or an error that
        // nobody asked for is ignored.
        vet(&opts, false, &name, msg)
    }
}

/// A machine's state as the window is told it (`machine-state`): the kind, and the words that
/// say what to do when a person has to act.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MachineState {
    pub machine: u16,
    pub name: String,
    pub state: &'static str,
    pub why: Option<String>,
    /// Which dial a reconnect is on.
    pub attempt: Option<u32>,
}

pub fn describe(machine: MachineId, name: &str, state: &ConnectionState) -> MachineState {
    let (why, attempt) = match state {
        ConnectionState::NeedsLogin { why }
        | ConnectionState::NeedsUpdate { why }
        | ConnectionState::Unreachable { why, .. } => (Some(why.clone()), None),
        ConnectionState::Reconnecting { attempt, .. } => (None, Some(*attempt)),
        ConnectionState::Connecting | ConnectionState::Connected => (None, None),
    };
    MachineState {
        machine: machine.0,
        name: name.to_owned(),
        state: state.kind(),
        why,
        attempt,
    }
}

/// The event the window listens to for each machine's state.
pub const MACHINE_STATE_EVENT: &str = "machine-state";

/// Keep every configured machine connected for as long as the app runs, telling the window of
/// each change of state.
pub fn start(app: tauri::AppHandle, defs: Vec<MachineDef>) {
    use tauri::Emitter;
    if defs.is_empty() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let ids_file = MachineIds::file_in(&maitty_desktop_client::data_dir());
        let mut hub = Hub::start(
            &defs,
            maitty_machines::SSH_PROGRAM,
            ids_file,
            Timing::DEFAULT,
        );
        loop {
            if let Update::State {
                machine,
                name,
                state,
            } = hub.next().await
            {
                let _ = app.emit(MACHINE_STATE_EVENT, describe(machine, &name, &state));
            }
        }
    });
}

#[cfg(test)]
mod tests;
