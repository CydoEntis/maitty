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

use crate::hub::{Hub, HubHandle, LocalDial, MACHINE_STATE_EVENT, Update, describe};
use crate::streams::{Out, Streams};
use maitty_machines::link::Timing;
use maitty_machines::machine_ids::MachineIds;
use maitty_machines::{MachineDef, MachineId, Outgoing};
use maitty_shared::protocol::{ClientMsg, Snapshot};
use serde::Serialize;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Emitter, Manager};

/// Carries this computer's `Snapshot` (what `list_sessions` returns) as it changes, or `null`
/// when the line dropped and the window should ask for the list itself.
const SESSIONS_EVENT: &str = "sessions-changed";
/// Carries another machine's whole snapshot as it changes: `{machine, snapshot}`.
const MACHINE_SESSIONS_EVENT: &str = "machine-sessions-changed";

const NOT_CONNECTED: &str = "The terminal is not connected to the daemon";

#[derive(Default)]
pub struct StreamState(Mutex<Streams>);

/// The window's channel for output frames. None until the window asks; a window that reloads
/// asks again and takes the place of the old one.
#[derive(Default)]
pub struct TerminalOutput(Mutex<Option<Channel<Response>>>);

/// Where typed keys and requests for a replay go: to the machine's line, through the hub.
/// Empty until the hub has started.
#[derive(Default)]
pub struct Wire(OnceLock<HubHandle>);

impl Wire {
    fn send(&self, machine: MachineId, msg: ClientMsg) -> Result<(), String> {
        self.0
            .get()
            .ok_or_else(|| NOT_CONNECTED.to_string())?
            .send(Outgoing { machine, msg })
            .map_err(|_| NOT_CONNECTED.to_string())
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
    wire.send(
        MachineId(machine.unwrap_or(0)),
        ClientMsg::Input { term, data },
    )
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
    let machine = MachineId(machine.unwrap_or(0));
    let body = streams
        .0
        .lock()
        .map_err(|_| "The terminal history is unavailable".to_string())?
        .history(machine, term);
    if let Some(body) = body {
        return Ok(Response::new(body));
    }
    wire.send(machine, ClientMsg::WantReplay { term })?;
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
                            let out = streams(&app, |streams| streams.flush(Instant::now()));
                            perform(&app, &handle, out);
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

fn apply(app: &AppHandle, handle: &HubHandle, update: Update) {
    match update {
        Update::State {
            machine,
            name,
            state,
        } => {
            let _ = app.emit(MACHINE_STATE_EVENT, describe(machine, &name, &state));
        }
        Update::LineUp { machine } => {
            // Panes a view was watching are asked for again; the replay that comes puts each
            // view back in step.
            let watched = streams(app, |streams| streams.line_up(machine)).unwrap_or_default();
            for term in watched {
                // A write that fails here shows up as the line ending.
                let _ = handle.send(Outgoing {
                    machine,
                    msg: ClientMsg::WantReplay { term },
                });
            }
        }
        Update::Messages { machine, msgs } => {
            for msg in msgs {
                let now = Instant::now();
                let out = streams(app, |streams| streams.on_message(machine, msg, now));
                perform(app, handle, out);
            }
        }
        Update::LineEnded { machine } => {
            let out = streams(app, |streams| streams.line_ended(machine, Instant::now()));
            perform(app, handle, out);
        }
    }
}

#[derive(Clone, Serialize)]
struct MachineSessions {
    machine: u16,
    snapshot: Option<Box<Snapshot>>,
}

fn perform(app: &AppHandle, handle: &HubHandle, out: Option<Vec<Out>>) {
    for out in out.unwrap_or_default() {
        match out {
            Out::Frame(frame) => {
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
            Out::Sessions { machine, snapshot } if machine == MachineId::LOCAL => {
                let _ = app.emit(SESSIONS_EVENT, snapshot);
            }
            Out::Sessions { machine, snapshot } => {
                let _ = app.emit(
                    MACHINE_SESSIONS_EVENT,
                    MachineSessions {
                        machine: machine.0,
                        snapshot,
                    },
                );
            }
            Out::WantReplay { machine, term } => {
                let _ = handle.send(Outgoing {
                    machine,
                    msg: ClientMsg::WantReplay { term },
                });
            }
        }
    }
}
