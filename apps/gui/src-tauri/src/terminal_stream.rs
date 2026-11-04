//! Live terminal output for the window. The daemon pushes each pane's replay and then its output;
//! this forwards them to the window as events and keeps a short history per pane, so a view that
//! opens later can catch up and then continue from the next chunk without a gap or a repeat.
//!
//! A PTY read is often a few bytes and every message is a trip through the web view, so output
//! waits briefly (`FLUSH_EVERY`, or `FLUSH_AT_BYTES` of one pane) and one frame carries what
//! gathered. Frames go over the raw channel the window opens with `watch_terminal_output`, as
//! bytes rather than JSON. The daemon's state goes out as `sessions-changed` when it changes, so
//! the window need not poll; a change to some panes arrives as a delta and is merged here, so the
//! window always gets a whole snapshot.
//!
//! The stream attaches without replays: a pane's replay is asked for over the stream when a view
//! opens on it (`terminal_history`), so panes nobody looks at cost nothing to carry.

use maitty_desktop_client::TerminalInput;
use maitty_shared::protocol::{ServerMsg, Snapshot};
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::Duration;
use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::Instant;

/// Carries the daemon's `Snapshot` (what `list_sessions` returns) as it changes, or `null` when
/// the stream dropped and the window should ask for the list itself.
const SESSIONS_EVENT: &str = "sessions-changed";
/// More history than this for one pane is dropped; a view then asks the daemon for a fresh replay.
const HISTORY_LIMIT_BYTES: usize = 8 * 1024 * 1024;
const RECONNECT_DELAY: Duration = Duration::from_secs(1);
/// Output waits this long for more before it is sent: under a frame, so typing still echoes at once.
const FLUSH_EVERY: Duration = Duration::from_millis(12);
/// A pane with this much waiting is sent without waiting for the timer.
const FLUSH_AT_BYTES: usize = 64 * 1024;
/// The history reply starts with the sequence number it ends at, as big-endian bytes.
const SEQ_BYTES: usize = std::mem::size_of::<u64>();
/// A frame's header: the pane, the sequence number and the replay flag.
const FRAME_HEADER_BYTES: usize = std::mem::size_of::<u32>() + SEQ_BYTES + 1;

#[derive(Default)]
struct Pane {
    /// The sequence number of the last chunk, counting from 1.
    seq: u64,
    /// The pane's last replay followed by every chunk of output since.
    history: Vec<u8>,
    /// False once the history overflowed: it no longer starts at a replay.
    complete: bool,
}

#[derive(Default)]
pub struct TerminalHistory(Mutex<HashMap<u32, Pane>>);

/// Where typed keys go: back over the stream's own connection. None while it is not connected.
#[derive(Default)]
pub struct TerminalKeys(tokio::sync::Mutex<Option<TerminalInput>>);

/// The window's channel for output frames. None until the window asks; a window that reloads
/// asks again and takes the place of the old one.
#[derive(Default)]
pub struct TerminalOutput(Mutex<Option<Channel<Response>>>);

/// The daemon's last whole snapshot for this window, so a delta can be applied to it.
#[derive(Default)]
pub struct LastSnapshot(Mutex<Option<Snapshot>>);

/// Each frame: the pane as 4 big-endian bytes, the sequence number as 8, one byte that is 1 for a
/// replay, then the output. The view decodes it in `apps/gui/src/api/terminalStream.ts`.
#[tauri::command]
pub fn watch_terminal_output(output: tauri::State<'_, TerminalOutput>, on_chunk: Channel<Response>) {
    if let Ok(mut channel) = output.0.lock() {
        *channel = Some(on_chunk);
    }
}

/// Output the daemon sent that the window has not been given yet, by pane.
#[derive(Default)]
struct Pending {
    panes: BTreeMap<u32, Vec<u8>>,
    /// When the oldest waiting output arrived; None while nothing waits.
    since: Option<Instant>,
}

#[tauri::command]
pub async fn terminal_input(
    keys: tauri::State<'_, TerminalKeys>,
    term: u32,
    data: Vec<u8>,
) -> Result<(), String> {
    let mut keys = keys.0.lock().await;
    let input = keys
        .as_mut()
        .ok_or_else(|| "The terminal is not connected to the daemon".to_string())?;
    input
        .send(term, data)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// What a view needs to open on a pane: the sequence number it ends at (`SEQ_BYTES`, big-endian),
/// then everything shown so far. Empty when the pane has no history yet: the daemon has then been
/// asked for its replay over the stream, and it arrives as the next frame for that pane. An error
/// when the stream is down, so the view asks the daemon itself. Raw bytes, because as JSON a
/// megabyte of output is several of text to parse.
#[tauri::command]
pub async fn terminal_history(
    history: tauri::State<'_, TerminalHistory>,
    keys: tauri::State<'_, TerminalKeys>,
    term: u32,
) -> Result<Response, String> {
    let body = {
        let panes = history
            .0
            .lock()
            .map_err(|_| "The terminal history is unavailable".to_string())?;
        panes.get(&term).filter(|pane| pane.complete).map(|pane| {
            // One copy under the lock; live output waits for a memcpy, not for an encoding.
            let mut body = Vec::with_capacity(SEQ_BYTES + pane.history.len());
            body.extend_from_slice(&pane.seq.to_be_bytes());
            body.extend_from_slice(&pane.history);
            body
        })
    };
    if let Some(body) = body {
        return Ok(Response::new(body));
    }
    let mut keys = keys.0.lock().await;
    let input = keys
        .as_mut()
        .ok_or_else(|| "The terminal is not connected to the daemon".to_string())?;
    input
        .want_replay(term)
        .await
        .map_err(|error| format!("{error:#}"))?;
    Ok(Response::new(Vec::new()))
}

/// Read the daemon's stream for as long as the app runs, reconnecting when it drops.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            read_stream(&app).await;
            tokio::time::sleep(RECONNECT_DELAY).await;
        }
    });
}

async fn read_stream(app: &AppHandle) {
    let Ok(executable) = super::daemon_executable(app) else {
        return;
    };
    let Ok((mut stream, mut input)) = maitty_desktop_client::open_terminal_stream(executable).await
    else {
        return;
    };
    // The histories start over, but the sequence numbers run on: a view drops anything numbered
    // at or below what it has seen, so a restart from 1 would leave it deaf. Panes a view was
    // watching are asked for again, and the replay that comes puts each view back in step.
    let watched: Vec<u32> = match app.state::<TerminalHistory>().0.lock() {
        Ok(mut panes) => panes
            .iter_mut()
            .filter_map(|(term, pane)| {
                let watched = pane.complete;
                pane.history.clear();
                pane.complete = false;
                watched.then_some(*term)
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    for term in watched {
        // A write that fails here shows up as the stream ending on the first read below.
        let _ = input.want_replay(term).await;
    }
    *app.state::<TerminalKeys>().0.lock().await = Some(input);
    if let Ok(mut last) = app.state::<LastSnapshot>().0.lock() {
        *last = None;
    }
    let mut pending = Pending::default();
    loop {
        // Waiting for the next frame is safe to cut short: the framed reader keeps a partial frame.
        let next = match pending.since {
            None => stream.next().await,
            Some(since) => match tokio::time::timeout_at(since + FLUSH_EVERY, stream.next()).await
            {
                Ok(next) => next,
                Err(_elapsed) => {
                    flush(app, &mut pending);
                    continue;
                }
            },
        };
        let Ok(Some(message)) = next else {
            break;
        };
        handle(app, message, &mut pending);
    }
    flush(app, &mut pending);
    *app.state::<TerminalKeys>().0.lock().await = None;
    // The window's list is stale now: it asks again, and learns the daemon is gone if it is.
    let _ = app.emit(SESSIONS_EVENT, ());
}

fn handle(app: &AppHandle, message: ServerMsg, pending: &mut Pending) {
    match message {
        ServerMsg::Replay { term, data, .. } => {
            // A replay replaces the pane, so output held from before it lands first.
            flush(app, pending);
            emit_chunk(app, term, data, true);
        }
        ServerMsg::Output { term, data } => {
            let waiting = pending.panes.entry(term).or_default();
            waiting.extend_from_slice(&data);
            pending.since.get_or_insert_with(Instant::now);
            if waiting.len() >= FLUSH_AT_BYTES {
                let data = pending.panes.remove(&term).unwrap_or_default();
                if pending.panes.is_empty() {
                    pending.since = None;
                }
                emit_chunk(app, term, data, false);
            }
        }
        ServerMsg::State(snapshot) => publish_snapshot(app, snapshot),
        ServerMsg::TermsChanged { changed, removed } => {
            let merged = app.state::<LastSnapshot>().0.lock().ok().and_then(|last| {
                let mut merged = last.clone()?;
                for term in removed {
                    merged.terms.remove(&term);
                }
                for term in changed {
                    merged.terms.insert(term.id, term);
                }
                Some(merged)
            });
            // A delta before any whole snapshot has nothing to apply to; the next whole one comes with the next reconnect.
            if let Some(merged) = merged {
                publish_snapshot(app, merged);
            }
        }
        _ => {}
    }
}

fn publish_snapshot(app: &AppHandle, snapshot: Snapshot) {
    if let Ok(mut panes) = app.state::<TerminalHistory>().0.lock() {
        panes.retain(|term, _| snapshot.terms.contains_key(term));
    }
    let _ = app.emit(SESSIONS_EVENT, &snapshot);
    if let Ok(mut last) = app.state::<LastSnapshot>().0.lock() {
        *last = Some(snapshot);
    }
}

fn flush(app: &AppHandle, pending: &mut Pending) {
    pending.since = None;
    for (term, data) in std::mem::take(&mut pending.panes) {
        emit_chunk(app, term, data, false);
    }
}

fn emit_chunk(app: &AppHandle, term: u32, data: Vec<u8>, replay: bool) {
    let Some(frame) = record(&app.state::<TerminalHistory>(), term, data, replay) else {
        return;
    };
    let channel = app
        .state::<TerminalOutput>()
        .0
        .lock()
        .ok()
        .and_then(|channel| channel.clone());
    // A window that has not asked yet, or is gone, has nothing to show; the next one opens from the history.
    if let Some(channel) = channel {
        let _ = channel.send(Response::new(frame));
    }
}

/// Adds the output to the pane's history and returns its frame for the window.
fn record(history: &TerminalHistory, term: u32, data: Vec<u8>, replay: bool) -> Option<Vec<u8>> {
    let mut panes = history.0.lock().ok()?;
    let pane = panes.entry(term).or_default();
    pane.seq += 1;
    if replay {
        pane.history.clear();
        pane.complete = true;
    }
    // Output before a replay is not kept: without the replay it starts nowhere a view can use.
    if pane.complete {
        pane.history.extend_from_slice(&data);
    }
    if pane.history.len() > HISTORY_LIMIT_BYTES {
        pane.history.clear();
        pane.complete = false;
    }
    let mut frame = Vec::with_capacity(FRAME_HEADER_BYTES + data.len());
    frame.extend_from_slice(&term.to_be_bytes());
    frame.extend_from_slice(&pane.seq.to_be_bytes());
    frame.push(u8::from(replay));
    frame.extend_from_slice(&data);
    Some(frame)
}
