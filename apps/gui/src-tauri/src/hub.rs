//! The host's connections to other machines (ADR 0007). One supervisor per configured machine
//! keeps its line up through `maitty-machines`; this takes what the supervisors and the lines
//! report and decides what the host may act on. The web view never connects.
//!
//! Everything a remote daemon says is vetted here, by the code the terminal app uses
//! (`remote_policy`), and a different daemon answering at a machine's name is turned away
//! (`machine_ids`). Nothing here knows about Tauri, so the whole of it is tested without a
//! window, against the fake `ssh` shim.

use crate::routing::{ReplyAllowance, expects_answer};
use maitty_ipc_transport::{Reader, Writer};
use maitty_machines::connections::{Burst, Connections, NotConnected, SERVER_BURST};
use maitty_machines::link::{
    ConnectionState, LinkControl, LinkEvent, Links, Timing, configured_machines,
};
use maitty_machines::machine_ids::{IdCheck, MachineIds, check_daemon_id};
use maitty_machines::machine_config::Grant;
use maitty_machines::remote_policy::{MachineOpts, vet};
use maitty_machines::{Dial, Machine, MachineDef, MachineId, Outgoing};
use maitty_shared::protocol::{ClientMsg, ServerMsg, Surface};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// How this computer's own line is made: a connection to its daemon, starting the daemon when
/// none answers.
pub type LocalDial = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = anyhow::Result<(Reader, Writer)>> + Send>>
        + Send
        + Sync,
>;

/// How long to wait before this computer's line is made again.
const LOCAL_REDIAL: Duration = Duration::from_secs(1);

/// What a connection says about the host in its Hello: it attaches to the panes this app owns
/// (ADR 0004) and asks for a pane's replay when a view opens on it. A desktop has no pane of
/// its own, so there is no claim to make. A machine on an older version is not used as it is:
/// it shows as needing its update, which the window offers.
pub(crate) fn dial() -> anyhow::Result<Dial> {
    Ok(Dial {
        attach: true,
        surface: Surface::Desktop,
        replays: false,
        term: None,
        token: None,
        structured: None,
        newest_only: true,
    })
}

/// What the host asks of the hub about a machine, by name.
#[derive(Debug)]
enum Command {
    Add(Box<MachineDef>),
    Retry(String),
    Remove(String),
}

/// What the hub has to tell the host.
#[derive(Debug)]
pub enum Update {
    /// A machine was taken out of the list: the window drops everything it holds for it.
    Removed { machine: MachineId },
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
    /// A machine's line came up, before anything it says.
    LineUp { machine: MachineId },
    /// A machine's line ended; its messages came first.
    LineEnded { machine: MachineId },
}

/// Where the host sends what it has for a machine. Cheap to clone; it reaches the hub's lines
/// without waiting for the hub's loop.
#[derive(Clone)]
pub struct HubHandle {
    outbox: mpsc::UnboundedSender<Outgoing>,
    commands: mpsc::UnboundedSender<Command>,
    shared: Arc<Shared>,
}

/// What the hub and the host's commands both look at.
#[derive(Default)]
struct Shared {
    /// Machines with a line.
    up: Mutex<BTreeSet<MachineId>>,
    /// Machines a different daemon answered for: told to the window as `refused`, not `unreachable`.
    refused: Mutex<BTreeSet<MachineId>>,
    /// Each machine's name, for saying which one a refusal is about.
    names: Mutex<BTreeMap<MachineId, String>>,
    /// Requests out to each other machine that are waiting for an answer.
    allowance: Mutex<BTreeMap<MachineId, ReplyAllowance>>,
    /// Keystrokes a machine without a line could not take, counted until it is back.
    unsent: Mutex<BTreeMap<MachineId, u32>>,
    /// What each machine may do to this computer (`notices`, `sounds`, `clipboard`), as configured.
    opts: Mutex<BTreeMap<MachineId, MachineOpts>>,
    /// How to reach each machine, so a request line can be made beside its long one.
    routes: Mutex<BTreeMap<MachineId, Machine>>,
    /// When each machine that is not connected stopped being, in milliseconds since the epoch.
    down_since: Mutex<BTreeMap<MachineId, u64>>,
}

impl HubHandle {
    /// Hand a message to its machine's line. Never waits. Refused when the machine has no line;
    /// typing a machine could not take is counted, so the person is told when it is back.
    pub fn send(&self, outgoing: Outgoing) -> Result<(), NotConnected> {
        let machine = outgoing.machine;
        if !self.is_up(machine) {
            if matches!(outgoing.msg, ClientMsg::Input { .. })
                && machine != MachineId::LOCAL
                && lock(&self.shared.names).contains_key(&machine)
            {
                *lock(&self.shared.unsent).entry(machine).or_default() += 1;
            }
            return Err(NotConnected(machine));
        }
        // What the machine says to this request may be taken as its answer.
        if machine != MachineId::LOCAL && expects_answer(&outgoing.msg) {
            lock(&self.shared.allowance)
                .entry(machine)
                .or_default()
                .asked(Instant::now());
        }
        self.outbox
            .send(outgoing)
            .map_err(|_| NotConnected(machine))
    }

    /// Turn one of a machine's grants on or off, for what it says from now on. Returns the
    /// machine's number, or `None` when no machine has that name.
    pub fn set_grant(&self, name: &str, grant: Grant, on: bool) -> Option<MachineId> {
        let machine = self.id_of(name)?;
        let mut all = lock(&self.shared.opts);
        let opts = all.entry(machine).or_default();
        match grant {
            Grant::Clipboard => opts.clipboard = on,
            Grant::Notices => opts.notices = on,
            Grant::Sounds => opts.sounds = on,
        }
        Some(machine)
    }

    /// The number of the machine called `name`, in any case.
    pub fn id_of(&self, name: &str) -> Option<MachineId> {
        lock(&self.shared.names)
            .iter()
            .find(|(id, known)| **id != MachineId::LOCAL && known.eq_ignore_ascii_case(name))
            .map(|(id, _)| *id)
    }

    /// What the machine may do to this computer; nothing for one that is not known.
    pub fn opts_of(&self, machine: MachineId) -> MachineOpts {
        lock(&self.shared.opts)
            .get(&machine)
            .copied()
            .unwrap_or_default()
    }

    /// Whether a different daemon answered at the machine's name, so its line was turned away.
    /// How to reach `machine`, when it is one of the list's.
    pub fn route(&self, machine: MachineId) -> Option<Machine> {
        lock(&self.shared.routes).get(&machine).cloned()
    }

    pub fn is_refused(&self, machine: MachineId) -> bool {
        lock(&self.shared.refused).contains(&machine)
    }

    /// Ask the hub to start dialling a machine added while the app runs. It shows as connecting
    /// at once; its entry is already in the config file.
    pub fn add(&self, def: MachineDef) {
        let _ = self.commands.send(Command::Add(Box::new(def)));
    }

    /// Ask the hub to dial the machine named `name` now, letting a refused daemon be looked at again.
    pub fn retry(&self, name: &str) {
        let _ = self.commands.send(Command::Retry(name.to_owned()));
    }

    /// Ask the hub to stop the machine named `name` and forget it. Its sessions, its daemon and
    /// the layout saved under its name are not touched.
    pub fn remove(&self, name: &str) {
        let _ = self.commands.send(Command::Remove(name.to_owned()));
    }

    pub fn is_up(&self, machine: MachineId) -> bool {
        lock(&self.shared.up).contains(&machine)
    }

    /// The machine's name, as it is configured.
    pub fn name_of(&self, machine: MachineId) -> Option<String> {
        lock(&self.shared.names).get(&machine).cloned()
    }

    /// When the machine's present outage began: the first state that was not connected since it
    /// last was. `None` while it is connected.
    pub fn down_since(&self, machine: MachineId, connected: bool, now_ms: u64) -> Option<u64> {
        let mut down = lock(&self.shared.down_since);
        if connected {
            down.remove(&machine);
            None
        } else {
            Some(*down.entry(machine).or_insert(now_ms))
        }
    }

    /// How many keystrokes the machine could not take while it was down, and start counting again.
    pub fn take_unsent(&self, machine: MachineId) -> u32 {
        lock(&self.shared.unsent).remove(&machine).unwrap_or(0)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Keep this computer's line up: dial, hand the line over, and dial again when it ends.
async fn supervise_local(
    dial: LocalDial,
    events: mpsc::UnboundedSender<LinkEvent>,
    mut ended: mpsc::UnboundedReceiver<()>,
) {
    let name = maitty_machines::LOCAL_MACHINE.to_owned();
    let state = |state| LinkEvent::State {
        machine: MachineId::LOCAL,
        name: name.clone(),
        state,
    };
    if events.send(state(ConnectionState::Connecting)).is_err() {
        return;
    }
    loop {
        if let Ok(halves) = dial().await {
            if events
                .send(LinkEvent::Line(MachineId::LOCAL, halves))
                .is_err()
                || events.send(state(ConnectionState::Connected)).is_err()
            {
                return;
            }
            if ended.recv().await.is_none() {
                return;
            }
            let lost = ConnectionState::Reconnecting {
                attempt: 1,
                since: std::time::Instant::now(),
            };
            if events.send(state(lost)).is_err() {
                return;
            }
        }
        tokio::time::sleep(LOCAL_REDIAL).await;
    }
}

pub struct Hub {
    links: Links,
    control: LinkControl,
    connections: Connections,
    ids: MachineIds,
    ids_file: PathBuf,
    /// The ssh program (and shim, in tests) machines added later are reached through.
    ssh: String,
    names: BTreeMap<MachineId, String>,
    /// Machines a different daemon answered for, with the reason. Cleared by a fresh line.
    refused: BTreeMap<MachineId, String>,
    queue: VecDeque<Update>,
    handle: HubHandle,
    outbox: mpsc::UnboundedReceiver<Outgoing>,
    commands: mpsc::UnboundedReceiver<Command>,
    /// This computer's line, when the hub keeps it: what it says, and telling it a line ended.
    local_events: mpsc::UnboundedReceiver<LinkEvent>,
    local_ended: Option<mpsc::UnboundedSender<()>>,
    local_task: Option<tokio::task::JoinHandle<()>>,
}

impl Hub {
    /// Start a supervisor for each of `defs`, reached through `ssh`, and for this computer's own
    /// line when `local` says how to make it. Must run inside the runtime. With no machine and
    /// no local line nothing starts.
    pub fn start(
        defs: &[MachineDef],
        ssh: &str,
        ids_file: PathBuf,
        timing: Timing,
        local: Option<LocalDial>,
    ) -> Hub {
        let machines = configured_machines(defs, ssh);
        let mut names = machines
            .iter()
            .map(|(id, machine)| (*id, machine.name().to_owned()))
            .collect::<BTreeMap<_, _>>();
        let mut opts: BTreeMap<MachineId, MachineOpts> = names
            .iter()
            .map(|(id, name)| (*id, MachineOpts::named(defs, name)))
            .collect();
        names.insert(MachineId::LOCAL, maitty_machines::LOCAL_MACHINE.to_owned());
        opts.insert(MachineId::LOCAL, MachineOpts::LOCAL);
        let routes: BTreeMap<MachineId, Machine> = machines.iter().cloned().collect();
        let links = Links::start(machines, timing, dial);
        let names_for_hub = names.clone();
        let (outbox_tx, outbox) = mpsc::unbounded_channel();
        let (commands_tx, commands) = mpsc::unbounded_channel();
        let (local_tx, local_events) = mpsc::unbounded_channel();
        let (ended_tx, ended_rx) = mpsc::unbounded_channel();
        let (local_ended, local_task) = match local {
            Some(dial) => (
                Some(ended_tx),
                Some(tokio::spawn(supervise_local(dial, local_tx, ended_rx))),
            ),
            None => (None, None),
        };
        Hub {
            control: links.control(),
            links,
            connections: Connections::default(),
            ids: MachineIds::load(&ids_file),
            ids_file,
            ssh: ssh.to_owned(),
            names: names_for_hub,
            refused: BTreeMap::new(),
            queue: VecDeque::new(),
            handle: HubHandle {
                outbox: outbox_tx,
                commands: commands_tx,
                shared: Arc::new(Shared {
                    names: Mutex::new(names),
                    opts: Mutex::new(opts),
                    routes: Mutex::new(routes),
                    ..Shared::default()
                }),
            },
            outbox,
            commands,
            local_events,
            local_ended,
            local_task,
        }
    }

    /// Whether anything is supervised: with nothing there is nothing to wait for.
    pub fn any(&self) -> bool {
        self.links.any() || self.local_task.is_some()
    }

    pub fn handle(&self) -> HubHandle {
        self.handle.clone()
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

    /// The next thing to tell the host. Waits for ever with nothing to wait for. Safe to cut
    /// short: whatever it had taken is kept for the next call.
    pub async fn next(&mut self) -> Update {
        loop {
            if let Some(update) = self.queue.pop_front() {
                return update;
            }
            tokio::select! {
                event = self.links.next() => self.on_link_event(event),
                Some(event) = self.local_events.recv() => self.on_link_event(event),
                Some(command) = self.commands.recv() => self.on_command(command),
                Some(outgoing) = self.outbox.recv() => {
                    // A line that went since the handle looked has nowhere to take it.
                    let _ = self.connections.send(outgoing);
                }
                burst = self.connections.recv_burst(SERVER_BURST) => self.on_burst(burst),
            }
        }
    }

    fn on_command(&mut self, command: Command) {
        match command {
            Command::Add(def) => {
                if self.control.id_of(&def.name).is_some() {
                    return;
                }
                let machine = Machine::from_def(&def, &self.ssh);
                let route = machine.clone();
                // Named before the supervisor starts, so nothing it says is dropped as unnamed.
                let Some(id) = self.control.start(machine) else {
                    return;
                };
                lock(&self.handle.shared.routes).insert(id, route);
                self.names.insert(id, def.name.clone());
                lock(&self.handle.shared.names).insert(id, def.name.clone());
                lock(&self.handle.shared.opts).insert(id, MachineOpts::named(std::slice::from_ref(&def), &def.name));
            }
            Command::Retry(name) => {
                let Some(machine) = self.control.id_of(&name) else {
                    return;
                };
                if self.connections.is_connected(machine) {
                    return;
                }
                self.refused.remove(&machine);
                lock(&self.handle.shared.refused).remove(&machine);
                self.control.retry(machine);
            }
            Command::Remove(name) => {
                let Some(machine) = self.control.id_of(&name) else {
                    return;
                };
                // The id store is keyed by the name as configured, not as it was typed.
                let configured = self.names.remove(&machine).unwrap_or(name);
                self.control.stop(machine);
                self.connections.remove(machine);
                self.refused.remove(&machine);
                let shared = &self.handle.shared;
                lock(&shared.names).remove(&machine);
                lock(&shared.opts).remove(&machine);
                lock(&shared.routes).remove(&machine);
                lock(&shared.up).remove(&machine);
                lock(&shared.refused).remove(&machine);
                lock(&shared.unsent).remove(&machine);
                lock(&shared.allowance).remove(&machine);
                lock(&shared.down_since).remove(&machine);
                if self.ids.forget(&configured) {
                    self.ids.save(&self.ids_file);
                }
                self.queue.push_back(Update::Removed { machine });
            }
        }
    }

    fn on_link_event(&mut self, event: LinkEvent) {
        match event {
            LinkEvent::Line(machine, halves) => {
                // A fresh line is a fresh look at who answers: the same wrong daemon is
                // refused again on its first State, a fixed one is let in.
                self.refused.remove(&machine);
                lock(&self.handle.shared.refused).remove(&machine);
                self.connections.add(machine, halves);
                lock(&self.handle.shared.up).insert(machine);
                self.queue.push_back(Update::LineUp { machine });
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
            lock(&self.handle.shared.up).remove(&machine);
        }
        if lost {
            lock(&self.handle.shared.up).remove(&machine);
            // A supervisor dials its machine again once told the line ended.
            if machine == MachineId::LOCAL {
                if let Some(ended) = &self.local_ended {
                    let _ = ended.send(());
                }
            } else {
                self.control.line_ended(machine, stalled);
            }
            self.queue.push_back(Update::LineEnded { machine });
        }
    }

    /// The message to act on, or `None` when `machine` may not send it. A machine a different
    /// daemon answers for says nothing that counts, and what the others say is vetted.
    fn admit(&mut self, machine: MachineId, msg: ServerMsg) -> Option<ServerMsg> {
        // This computer's own daemon is trusted as it always was.
        if machine == MachineId::LOCAL {
            return Some(msg);
        }
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
                    lock(&self.handle.shared.refused).insert(machine);
                    return None;
                }
            }
        }
        let opts = self.handle.opts_of(machine);
        // A reply or an error counts only while a request of the host's is out to the machine.
        let asked = lock(&self.handle.shared.allowance)
            .entry(machine)
            .or_default()
            .awaiting(Instant::now());
        let vetted = vet(&opts, asked, &name, msg)?;
        if matches!(vetted, ServerMsg::Reply(_) | ServerMsg::Error(_)) {
            lock(&self.handle.shared.allowance)
                .entry(machine)
                .or_default()
                .answered();
        }
        Some(vetted)
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        if let Some(task) = &self.local_task {
            task.abort();
        }
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
    /// On connecting: how many keystrokes the machine could not take while it was down.
    pub unsent: Option<u32>,
    /// Since when the machine has not been connected, in milliseconds since the epoch.
    pub since: Option<u64>,
    /// What the machine may do to this computer, from its entry in `config.local.toml`.
    pub clipboard: bool,
    pub notices: bool,
    pub sounds: bool,
}

/// A fix text is one constant for every machine, with `<name>` where the machine goes: the window
/// is given the command it can run.
fn fix_for(why: &str, name: &str) -> String {
    why.replace("<name>", name)
}

pub fn describe(machine: MachineId, name: &str, state: &ConnectionState) -> MachineState {
    let (why, attempt) = match state {
        ConnectionState::NeedsLogin { why }
        | ConnectionState::NeedsUpdate { why }
        | ConnectionState::Unreachable { why, .. } => (Some(fix_for(why, name)), None),
        ConnectionState::Reconnecting { attempt, .. } => (None, Some(*attempt)),
        ConnectionState::Connecting | ConnectionState::Connected => (None, None),
    };
    MachineState {
        machine: machine.0,
        name: name.to_owned(),
        state: state.kind(),
        why,
        attempt,
        unsent: None,
        since: None,
        clipboard: false,
        notices: false,
        sounds: false,
    }
}

/// The event the window listens to for each machine's state.
pub const MACHINE_STATE_EVENT: &str = "machine-state";

#[cfg(test)]
mod tests;
