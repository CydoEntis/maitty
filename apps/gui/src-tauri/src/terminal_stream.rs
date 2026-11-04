//! Live terminal output for the window, from every machine. The hub holds a line to this
//! computer's daemon (machine 0) and one to each configured machine; this forwards what they
//! push to the window as events and frames, and keeps each pane's history so a view that opens
//! later can catch up and then continue from the next chunk without a gap or a repeat.
//!
//! A PTY read is often a few bytes and every message is a trip through the web view, so output
//! waits briefly and one frame carries what gathered (`streams.rs` holds the rules, including
//! how little another machine may take). Frames go over the raw channel the window opens with
//! `watch_terminal_output`, as bytes rather than JSON. This computer's state goes out as
//! `sessions-changed` when it changes, so the window need not poll; a change to some panes
//! arrives as a delta and is merged, so the window always gets a whole snapshot. Another
//! machine's goes out as `machine-sessions-changed`, and each machine's connection state as
//! `machine-state`.
//!
//! The stream attaches without replays: a pane's replay is asked for over the stream when a view
//! opens on it (`terminal_history`), so panes nobody looks at cost nothing to carry.

use crate::hub::{Hub, HubHandle, LocalDial, MACHINE_STATE_EVENT, MachineState, Update, describe};
use crate::routing::{self, machine_of};
use crate::streams::{Out, Streams};
use maitty_desktop_client::PaneCommand;
use maitty_machines::link::Timing;
use maitty_machines::machine_ids::MachineIds;
use maitty_machines::{MachineDef, MachineId, Outgoing};
use maitty_shared::protocol::{ClientMsg, Snapshot};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Emitter, Manager};

/// Carries this computer's `Snapshot` (what `list_sessions` returns) as it changes, or `null`
/// when the line dropped and the window should ask for the list itself.
const SESSIONS_EVENT: &str = "sessions-changed";
/// Carries another machine's whole snapshot as it changes: `{machine, snapshot}`.
const MACHINE_SESSIONS_EVENT: &str = "machine-sessions-changed";
/// Carries what another machine said went wrong with a request: `{machine, text}`.
/// Carries the number of a machine that was removed from the list.
const MACHINE_REMOVED_EVENT: &str = "machine-removed";
const MACHINE_ERROR_EVENT: &str = "machine-error";

const NOT_CONNECTED: &str = "The terminal is not connected to the daemon";

#[derive(Default)]
pub struct StreamState(Mutex<Streams>);

/// Each machine's last state, for a window that was not there to hear it change.
#[derive(Default)]
pub struct MachineStates(Mutex<BTreeMap<u16, MachineState>>);

/// A machine as the window finds it when it starts: its state, and the snapshot it last gave.
#[derive(Clone, Serialize, Debug)]
pub struct MachineOverview {
    #[serde(flatten)]
    pub state: MachineState,
    pub snapshot: Option<Box<Snapshot>>,
}

/// The overview of every machine but this computer, which the window asks for itself.
pub fn overview(states: &MachineStates, streams: &Streams) -> Vec<MachineOverview> {
    let Ok(states) = states.0.lock() else {
        return Vec::new();
    };
    states
        .values()
        .filter(|state| state.machine != MachineId::LOCAL.0)
        .map(|state| MachineOverview {
            snapshot: streams
                .last_snapshot(MachineId(state.machine))
                .map(|snapshot| Box::new(snapshot.clone())),
            state: state.clone(),
        })
        .collect()
}

/// Dial a machine now, and let a daemon that was refused be looked at again.
#[tauri::command]
pub fn retry_machine(wire: tauri::State<'_, Wire>, name: String) -> Result<(), String> {
    wire.0
        .get()
        .ok_or_else(|| NOT_CONNECTED.to_string())?
        .retry(&name);
    Ok(())
}

/// Take a machine out of the list: its entry leaves `config.local.toml` first, and only when
/// that worked is its connection stopped and its record forgotten. The machine, its sessions and
/// the layout saved under its name are not touched.
#[tauri::command]
pub async fn remove_machine(wire: tauri::State<'_, Wire>, name: String) -> Result<(), String> {
    let handle = wire.0.get().ok_or_else(|| NOT_CONNECTED.to_string())?;
    let file = maitty_machines::local_machines::config_path().with_file_name("config.local.toml");
    let entry = name.clone();
    tauri::async_runtime::spawn_blocking(move || remove_entry(&file, &entry))
        .await
        .map_err(|error| format!("Could not remove {name}: {error}"))??;
    handle.remove(&name);
    Ok(())
}

/// Delete the entry named `name` from the machine file.
fn remove_entry(file: &std::path::Path, name: &str) -> Result<(), String> {
    match maitty_machines::machine_config::remove_machine(file, name) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "{name} is not in config.local.toml, so it cannot be removed here."
        )),
        Err(error) => Err(format!("Could not remove {name}: {error:#}")),
    }
}

#[tauri::command]
pub fn list_machines_overview(
    states: tauri::State<'_, MachineStates>,
    streams: tauri::State<'_, StreamState>,
) -> Vec<MachineOverview> {
    match streams.0.lock() {
        Ok(streams) => overview(&states, &streams),
        Err(_) => Vec::new(),
    }
}

/// The window's channel for output frames. None until the window asks; a window that reloads
/// asks again and takes the place of the old one.
#[derive(Default)]
pub struct TerminalOutput(Mutex<Option<Channel<Response>>>);

/// Where typed keys and requests for a replay go: to the machine's line, through the hub.
/// Empty until the hub has started.
#[derive(Default)]
pub struct Wire(OnceLock<HubHandle>);

impl Wire {
    #[cfg(test)]
    pub fn reaching(handle: HubHandle) -> Wire {
        let wire = Wire::default();
        let _ = wire.0.set(handle);
        wire
    }

    fn send(&self, machine: MachineId, msg: ClientMsg) -> Result<(), String> {
        let handle = self.0.get().ok_or_else(|| NOT_CONNECTED.to_string())?;
        handle
            .send(Outgoing { machine, msg })
            .map_err(|_| refusal(handle, machine))
    }

    /// Ask the daemon of `term`'s machine (this computer when none is named) to do `command`.
    /// The machine's line takes it without waiting; its answer is taken, if it comes, by the hub.
    /// Refused, naming the machine, when the machine has no line.
    pub fn command(
        &self,
        machine: Option<u16>,
        term: u32,
        command: PaneCommand,
    ) -> Result<(), String> {
        self.send(machine_of(machine), routing::message(term, command))
    }
}

/// Why a machine took nothing: it is down, or it is not one of ours.
fn refusal(handle: &HubHandle, machine: MachineId) -> String {
    if machine == MachineId::LOCAL {
        return NOT_CONNECTED.to_string();
    }
    match handle.name_of(machine) {
        Some(name) => format!("{name} is not connected"),
        None => format!(
            "Machine {} is not one of this computer's machines",
            machine.0
        ),
    }
}

/// Each frame: the machine as 2 big-endian bytes, the pane as 4, the sequence number as 8, one
/// byte that is 1 for a replay, then the output. The view decodes it in
/// `apps/gui/src/api/terminalStream.ts`.
#[tauri::command]
pub fn watch_terminal_output(
    output: tauri::State<'_, TerminalOutput>,
    on_chunk: Channel<Response>,
) {
    if let Ok(mut channel) = output.0.lock() {
        *channel = Some(on_chunk);
    }
}

/// `machine` is left out for this computer's own panes.
#[tauri::command]
pub async fn terminal_input(
    wire: tauri::State<'_, Wire>,
    machine: Option<u16>,
    term: u32,
    data: Vec<u8>,
) -> Result<(), String> {
    let largest = maitty_desktop_client::MAX_TERMINAL_INPUT_BYTES;
    if data.is_empty() || data.len() > largest {
        return Err(format!(
            "Terminal input must contain between 1 and {largest} bytes"
        ));
    }
    wire.command(machine, term, PaneCommand::Input(data))
}

/// What a view needs to open on a pane: the sequence number it ends at (`SEQ_BYTES`, big-endian),
/// then everything shown so far. Empty when the pane has no history yet: its daemon has then been
/// asked for its replay over the stream, and it arrives as the next frame for that pane. An error
/// when the stream is down, so the view asks the daemon itself. Raw bytes, because as JSON a
/// megabyte of output is several of text to parse.
#[tauri::command]
pub async fn terminal_history(
    streams: tauri::State<'_, StreamState>,
    wire: tauri::State<'_, Wire>,
    machine: Option<u16>,
    term: u32,
) -> Result<Response, String> {
    let machine = machine_of(machine);
    let body = streams
        .0
        .lock()
        .map_err(|_| "The terminal history is unavailable".to_string())?
        .history(machine, term);
    if let Some(body) = body {
        return Ok(Response::new(body));
    }
    wire.command(Some(machine.0), term, PaneCommand::WantReplay)?;
    Ok(Response::new(Vec::new()))
}

/// Read every machine's line for as long as the app runs, reconnecting when one drops.
pub fn start(app: AppHandle, defs: Vec<MachineDef>) {
    tauri::async_runtime::spawn(async move {
        let local: LocalDial = {
            let app = app.clone();
            Arc::new(move || {
                let executable = super::daemon_executable(&app);
                Box::pin(async move {
                    maitty_desktop_client::open_terminal_line(
                        executable.map_err(anyhow::Error::msg)?,
                    )
                    .await
                })
            })
        };
        let mut hub = Hub::start(
            &defs,
            maitty_machines::SSH_PROGRAM,
            MachineIds::file_in(&maitty_desktop_client::data_dir()),
            Timing::DEFAULT,
            Some(local),
        );
        let handle = hub.handle();
        let _ = app.state::<Wire>().0.set(handle.clone());
        loop {
            let due = app
                .state::<StreamState>()
                .0
                .lock()
                .ok()
                .and_then(|streams| streams.due());
            let update = match due {
                None => hub.next().await,
                Some(at) => {
                    // Waiting for the next update is safe to cut short: the hub keeps what it took.
                    match tokio::time::timeout_at(tokio::time::Instant::from_std(at), hub.next())
                        .await
                    {
                        Ok(update) => update,
                        Err(_elapsed) => {
                            let now = Instant::now();
                            let done = streams(&app, |streams| flush(streams, &handle, now));
                            perform(&app, done.unwrap_or_default());
                            continue;
                        }
                    }
                }
            };
            apply(&app, &handle, update);
        }
    });
}

fn streams<T>(app: &AppHandle, f: impl FnOnce(&mut Streams) -> T) -> Option<T> {
    app.state::<StreamState>()
        .0
        .lock()
        .ok()
        .map(|mut s| f(&mut s))
}

/// What the window is to be given because of one thing the hub reported.
pub enum Reaction {
    /// A machine's connection changed state.
    State(MachineState),
    /// A frame of output for the window's channel.
    Frame(Vec<u8>),
    /// What another machine said went wrong with a request.
    Error { machine: MachineId, text: String },
    /// A machine was taken out of the list.
    Removed { machine: MachineId },
    /// A machine's whole snapshot, or none when this computer's line ended.
    Sessions {
        machine: MachineId,
        snapshot: Option<Box<Snapshot>>,
    },
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}

/// What one update from the hub does: to the history and snapshots held for the window, to the
/// machines (a replay asked for again), and what the window is to be told. No window is involved,
/// so all of it is tested without one.
pub fn react(
    streams: &mut Streams,
    handle: &HubHandle,
    update: Update,
    now: Instant,
) -> Vec<Reaction> {
    let outs = match update {
        Update::State {
            machine,
            name,
            state,
        } => {
            let mut told = describe(machine, &name, &state);
            // The state machine has no word for it: a refusal reads as unreachable there.
            if handle.is_refused(machine) {
                told.state = "refused";
            }
            told.since = handle.down_since(machine, state.is_connected(), now_ms());
            if state.is_connected() {
                // Typing the machine could not take while it was down, now it can be said.
                told.unsent = Some(handle.take_unsent(machine)).filter(|n| *n > 0);
            }
            return vec![Reaction::State(told)];
        }
        Update::Removed { machine } => {
            streams.forget(machine);
            return vec![Reaction::Removed { machine }];
        }
        Update::LineUp { machine } => {
            // Panes a view was watching are asked for again; the replay that comes puts each
            // view back in step.
            for term in streams.line_up(machine) {
                // A write that fails here shows up as the line ending.
                let _ = handle.send(Outgoing {
                    machine,
                    msg: ClientMsg::WantReplay { term },
                });
            }
            Vec::new()
        }
        Update::Messages { machine, msgs } => msgs
            .into_iter()
            .flat_map(|msg| streams.on_message(machine, msg, now))
            .collect(),
        Update::LineEnded { machine } => streams.line_ended(machine, now),
    };
    reactions(handle, outs)
}

/// Give the window what has waited long enough.
pub fn flush(streams: &mut Streams, handle: &HubHandle, now: Instant) -> Vec<Reaction> {
    reactions(handle, streams.flush(now))
}

fn reactions(handle: &HubHandle, outs: Vec<Out>) -> Vec<Reaction> {
    outs.into_iter()
        .filter_map(|out| match out {
            Out::Frame(frame) => Some(Reaction::Frame(frame)),
            Out::Error { machine, text } => Some(Reaction::Error { machine, text }),
            Out::Sessions { machine, snapshot } => Some(Reaction::Sessions { machine, snapshot }),
            Out::WantReplay { machine, term } => {
                let _ = handle.send(Outgoing {
                    machine,
                    msg: ClientMsg::WantReplay { term },
                });
                None
            }
        })
        .collect()
}

fn apply(app: &AppHandle, handle: &HubHandle, update: Update) {
    let now = Instant::now();
    let done = streams(app, |streams| react(streams, handle, update, now));
    perform(app, done.unwrap_or_default());
}

#[derive(Clone, Serialize)]
struct MachineError {
    machine: u16,
    text: String,
}

#[derive(Clone, Serialize)]
struct MachineSessions {
    machine: u16,
    snapshot: Option<Box<Snapshot>>,
}

fn perform(app: &AppHandle, reactions: Vec<Reaction>) {
    for reaction in reactions {
        match reaction {
            Reaction::State(told) => {
                if let Ok(mut states) = app.state::<MachineStates>().0.lock() {
                    states.insert(told.machine, told.clone());
                }
                let _ = app.emit(MACHINE_STATE_EVENT, told);
            }
            Reaction::Removed { machine } => {
                if let Ok(mut states) = app.state::<MachineStates>().0.lock() {
                    states.remove(&machine.0);
                }
                let _ = app.emit(MACHINE_REMOVED_EVENT, machine.0);
            }
            Reaction::Error { machine, text } => {
                let _ = app.emit(
                    MACHINE_ERROR_EVENT,
                    MachineError {
                        machine: machine.0,
                        text,
                    },
                );
            }
            Reaction::Frame(frame) => {
                let channel = app
                    .state::<TerminalOutput>()
                    .0
                    .lock()
                    .ok()
                    .and_then(|channel| channel.clone());
                // A window that has not asked yet, or is gone, has nothing to show; the next one
                // opens from the history.
                if let Some(channel) = channel {
                    let _ = channel.send(Response::new(frame));
                }
            }
            Reaction::Sessions { machine, snapshot } if machine == MachineId::LOCAL => {
                let _ = app.emit(SESSIONS_EVENT, snapshot);
            }
            Reaction::Sessions { machine, snapshot } => {
                let _ = app.emit(
                    MACHINE_SESSIONS_EVENT,
                    MachineSessions {
                        machine: machine.0,
                        snapshot,
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maitty_shared::protocol::ServerMsg;

    fn state(machine: u16, name: &str, kind: &'static str) -> MachineState {
        MachineState {
            machine,
            name: name.into(),
            state: kind,
            why: None,
            attempt: None,
            unsent: None,
            since: None,
        }
    }

    #[test]
    fn removing_an_entry_takes_only_that_machine_out_of_the_file() {
        let dir = std::env::temp_dir().join(format!("maitty-gui-remove-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("config.local.toml");
        std::fs::write(
            &file,
            "[[machines]]\nname = 'box'\ntarget = 'box'\n\n# mine\n[[machines]]\nname = 'build'\ntarget = 'build'\n",
        )
        .unwrap();
        remove_entry(&file, "BOX").unwrap();
        let left = std::fs::read_to_string(&file).unwrap();
        assert!(left.contains("# mine") && left.contains("build") && !left.contains("'box'"), "{left}");
        let again = remove_entry(&file, "box").unwrap_err();
        assert!(again.contains("not in config.local.toml"), "{again}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_window_that_starts_late_is_told_each_machine_and_the_snapshot_it_last_gave() {
        let states = MachineStates::default();
        {
            let mut held = states.0.lock().unwrap();
            held.insert(0, state(0, "local", "connected"));
            held.insert(1, state(1, "box", "connected"));
            held.insert(2, state(2, "build", "needs-login"));
        }
        let mut streams = Streams::default();
        let now = Instant::now();
        streams.on_message(MachineId(1), ServerMsg::State(Snapshot::default()), now);

        let told = overview(&states, &streams);
        assert_eq!(
            told.iter()
                .map(|m| m.state.name.as_str())
                .collect::<Vec<_>>(),
            ["box", "build"],
            "this computer is not among them"
        );
        assert!(told[0].snapshot.is_some());
        assert!(
            told[1].snapshot.is_none(),
            "a machine never heard from has none"
        );
        let json = serde_json::to_value(&told[0]).unwrap();
        assert_eq!(json["machine"], 1);
        assert_eq!(json["state"], "connected");
        assert!(json.get("snapshot").is_some());
    }
}
