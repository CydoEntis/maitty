//! Live terminal output for the window. The daemon pushes each pane's replay and then its output;
//! this forwards them to the window as events and keeps a short history per pane, so a view that
//! opens later can catch up and then continue from the next chunk without a gap or a repeat.

use base64::{Engine, engine::general_purpose::STANDARD};
use maitty_desktop_client::TerminalInput;
use maitty_shared::protocol::ServerMsg;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const CHUNK_EVENT: &str = "terminal-chunk";
/// More history than this for one pane is dropped; a view then asks the daemon for a fresh replay.
const HISTORY_LIMIT_BYTES: usize = 8 * 1024 * 1024;
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalChunk {
    term: u32,
    seq: u64,
    /// A replay replaces what the view shows; output continues it.
    replay: bool,
    data: String,
}

#[derive(Serialize)]
pub struct HistoryReply {
    seq: u64,
    data: String,
}

/// What a view needs to open on a pane: everything so far, and the sequence number it ends at.
#[tauri::command]
pub fn terminal_history(
    history: tauri::State<'_, TerminalHistory>,
    term: u32,
) -> Option<HistoryReply> {
    let panes = history.0.lock().ok()?;
    let pane = panes.get(&term).filter(|pane| pane.complete)?;
    Some(HistoryReply {
        seq: pane.seq,
        data: STANDARD.encode(&pane.history),
    })
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
    let Ok((mut stream, input)) = maitty_desktop_client::open_terminal_stream(executable).await
    else {
        return;
    };
    *app.state::<TerminalKeys>().0.lock().await = Some(input);
    // The daemon replays every pane to a new connection, so start from nothing.
    if let Ok(mut panes) = app.state::<TerminalHistory>().0.lock() {
        panes.clear();
    }
    while let Ok(Some(message)) = stream.next().await {
        handle(app, message);
    }
    *app.state::<TerminalKeys>().0.lock().await = None;
}

fn handle(app: &AppHandle, message: ServerMsg) {
    let history = app.state::<TerminalHistory>();
    let chunk = match message {
        ServerMsg::Replay { term, data, .. } => record(&history, term, data, true),
        ServerMsg::Output { term, data } => record(&history, term, data, false),
        ServerMsg::State(snapshot) => {
            if let Ok(mut panes) = history.0.lock() {
                panes.retain(|term, _| snapshot.terms.contains_key(term));
            }
            None
        }
        _ => None,
    };
    if let Some(chunk) = chunk {
        let _ = app.emit(CHUNK_EVENT, chunk);
    }
}

fn record(
    history: &TerminalHistory,
    term: u32,
    data: Vec<u8>,
    replay: bool,
) -> Option<TerminalChunk> {
    let mut panes = history.0.lock().ok()?;
    let pane = panes.entry(term).or_default();
    pane.seq += 1;
    if replay {
        pane.history.clear();
        pane.complete = true;
    }
    pane.history.extend_from_slice(&data);
    if pane.history.len() > HISTORY_LIMIT_BYTES {
        pane.history.clear();
        pane.complete = false;
    }
    Some(TerminalChunk {
        term,
        seq: pane.seq,
        replay,
        data: STANDARD.encode(&data),
    })
}
