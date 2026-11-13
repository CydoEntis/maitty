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
use maitty_desktop_client::{FleetCommand, PaneCommand};
use maitty_machines::link::Timing;
use maitty_machines::machine_config::Grant;
use maitty_machines::machine_ids::MachineIds;
use maitty_machines::{MachineDef, MachineId, Outgoing};
use maitty_shared::protocol::{ClientMsg, Command, FolderListing, Query, Snapshot, Status, TermId};
use maitty_shared::settings_view::{Appearance, NewSessionView, SettingKey, SettingValue, SharedSettings, SidebarSide, WindowStyle};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
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
/// Carries what an opted-in machine wants shown: `{machine, text}`.
const MACHINE_NOTICE_EVENT: &str = "machine-notice";
/// Carries text an opted-in machine asks to be copied: `{machine, text}`.
const MACHINE_CLIPBOARD_EVENT: &str = "machine-clipboard";
/// Carries a daemon's decision to alert for a pane: `{machine, term, status}` (`blocked` or `done`).
const SESSION_ATTENTION_EVENT: &str = "session-attention";
/// Says a conversation of this computer's daemon changed: `{conversation, generation}`.
const CONVERSATION_CHANGED_EVENT: &str = "conversation-changed";
const MACHINE_ERROR_EVENT: &str = "machine-error";
const FOLDER_LISTING_EVENT: &str = "folder-listing";
/// The shared settings, whenever the daemon says them: the answer to a question, or a change.
const SHARED_SETTINGS_EVENT: &str = "shared-settings";

/// The longest folder path sent to another machine.
const MOST_FOLDER_BYTES: usize = 4096;

const NOT_CONNECTED: &str = "The terminal is not connected to the daemon";

#[derive(Default)]
pub struct StreamState(Mutex<Streams>);

/// Each machine's last state, for a window that was not there to hear it change.
#[derive(Default)]
pub struct MachineStates(Mutex<BTreeMap<u16, MachineState>>);

impl MachineStates {
    /// Whether the machine called `name` (any case) is `connected` now.
    pub fn is_connected(&self, name: &str) -> bool {
        self.0.lock().is_ok_and(|states| {
            states
                .values()
                .any(|state| state.name.eq_ignore_ascii_case(name) && state.state == "connected")
        })
    }
}

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

/// Start a terminal session in a folder of another machine.
#[tauri::command]
pub fn start_session_on_machine(
    wire: tauri::State<'_, Wire>,
    machine: u16,
    folder: String,
) -> Result<(), String> {
    wire.new_session(machine, &folder)
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

/// Allow or stop one of a machine's powers over this computer. The entry in `config.local.toml`
/// is written first; only when that worked does the host start or stop acting on what the machine
/// sends, and the window is told the machine's new state.
#[tauri::command]
pub async fn set_machine_grant(
    app: AppHandle,
    wire: tauri::State<'_, Wire>,
    name: String,
    grant: String,
    on: bool,
) -> Result<(), String> {
    let handle = wire.handle()?.clone();
    let grant = Grant::parse(&grant).ok_or_else(|| format!("{grant} is not something a machine can be allowed"))?;
    if handle.id_of(&name).is_none() {
        return Err(format!("{name} is not one of this computer's machines"));
    }
    let file = maitty_machines::local_machines::config_path().with_file_name("config.local.toml");
    let entry = name.clone();
    tauri::async_runtime::spawn_blocking(move || write_grant(&file, &entry, grant, on))
        .await
        .map_err(|error| format!("Could not change {name}: {error}"))??;
    if let Some(machine) = handle.set_grant(&name, grant, on)
        && let Ok(mut states) = app.state::<MachineStates>().0.lock()
        && let Some(state) = states.get_mut(&machine.0)
    {
        let opts = handle.opts_of(machine);
        (state.clipboard, state.notices, state.sounds) = (opts.clipboard, opts.notices, opts.sounds);
        let _ = app.emit(MACHINE_STATE_EVENT, state.clone());
    }
    Ok(())
}

/// Write one grant of the entry named `name` in the machine file.
fn write_grant(file: &std::path::Path, name: &str, grant: Grant, on: bool) -> Result<(), String> {
    match maitty_machines::machine_config::set_grant(file, name, grant, on) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "{name} is not in config.local.toml, so it cannot be changed here."
        )),
        Err(error) => Err(format!("Could not change {name}: {error:#}")),
    }
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

/// The least time between two asks of one machine for its list made through `ask_machine_list`,
/// so a window that asks in a loop cannot flood the machine with `Query::List`.
pub const WINDOW_LIST_ASK_EVERY: Duration = Duration::from_secs(5);

/// Where typed keys and requests for a replay go: to the machine's line, through the hub.
/// Empty until the hub has started. Also holds when the window last asked each machine for its
/// list.
#[derive(Default)]
pub struct Wire(OnceLock<HubHandle>, Mutex<BTreeMap<MachineId, Instant>>);

impl Wire {
    #[cfg(test)]
    pub fn reaching(handle: HubHandle) -> Wire {
        let wire = Wire::default();
        let _ = wire.0.set(handle);
        wire
    }

    /// Where a conversation request goes: this computer's daemon (`local` says where it is), or
    /// another machine's, over request lines of its own beside its long one (ADR 0007). A
    /// machine that is down or was refused takes nothing, and what it answers is vetted before
    /// anything reads it.
    pub fn daemon(
        &self,
        machine: Option<u16>,
        local: impl FnOnce() -> Result<std::path::PathBuf, String>,
    ) -> Result<maitty_desktop_client::Daemon, String> {
        use maitty_desktop_client::{Daemon, RemoteDaemon};
        if !routing::is_remote(machine) {
            return local().map(Daemon::Local);
        }
        let id = machine_of(machine);
        let handle = self.handle()?;
        if !handle.is_up(id) || handle.is_refused(id) {
            return Err(refusal(handle, id));
        }
        let route = handle.route(id).ok_or_else(|| refusal(handle, id))?;
        let name = handle.name_of(id).ok_or_else(|| refusal(handle, id))?;
        Ok(Daemon::Remote(RemoteDaemon::new(
            id.0,
            name,
            move || {
                let route = route.clone();
                Box::pin(async move { maitty_machines::open_remote(&route, crate::hub::dial()?).await })
            },
            maitty_machines::remote_chat::vet_chat_reply,
        )))
    }

    /// Refuse a request about a pane's files when the pane is on another machine.
    pub fn require_local(&self, machine: Option<u16>, label: &str) -> Result<(), String> {
        routing::require_local(machine, label, |id| {
            self.0.get().and_then(|handle| handle.name_of(id))
        })
    }

    /// The hub's handle, or why there is none yet.
    pub fn handle(&self) -> Result<&HubHandle, String> {
        self.0.get().ok_or_else(|| NOT_CONNECTED.to_string())
    }

    fn send(&self, machine: MachineId, msg: ClientMsg) -> Result<(), String> {
        let handle = self.0.get().ok_or_else(|| NOT_CONNECTED.to_string())?;
        handle
            .send(Outgoing { machine, msg })
            .map_err(|_| refusal(handle, machine))
    }

    /// A fleet request for another machine's daemon: the machine's line takes it without waiting.
    /// Refused, naming the machine, when the machine has no line or the request is not valid.
    pub fn fleet(&self, machine: Option<u16>, command: FleetCommand) -> Result<(), String> {
        self.send(machine_of(machine), command.message()?)
    }

    /// Start a terminal session in `folder` on another machine. The folder is a path there: one
    /// that does not exist opens in that machine's home folder, so `~` is a fair default.
    /// Refused, naming the machine, when it has no line.
    pub fn new_session(&self, machine: u16, folder: &str) -> Result<(), String> {
        if !routing::is_remote(Some(machine)) {
            return Err("This computer starts its own sessions".to_string());
        }
        let folder = folder.trim();
        if folder.is_empty() || folder.len() > MOST_FOLDER_BYTES || folder.chars().any(char::is_control) {
            return Err("Type a folder on that machine, such as ~ or /srv/app".to_string());
        }
        self.send(
            MachineId(machine),
            ClientMsg::Command(Command::NewWorkspace {
                cwd: Some(std::path::PathBuf::from(folder)),
                name: None,
                cmd: None,
            }),
        )
    }

    /// Ask another machine's daemon for its whole list; it comes back as `machine-sessions-changed`.
    /// The window asks while it holds a row of that machine that a list left out with no reason,
    /// so the answer can confirm the row is gone. Refused, naming the machine, when it has no line.
    /// An ask within `WINDOW_LIST_ASK_EVERY` of the last one that went out is dropped: the list
    /// asked for is on its way.
    pub fn ask_machine_list(&self, machine: u16) -> Result<(), String> {
        self.ask_machine_list_at(machine, Instant::now())
    }

    pub(crate) fn ask_machine_list_at(&self, machine: u16, now: Instant) -> Result<(), String> {
        if !routing::is_remote(Some(machine)) {
            return Err("This computer's list is read with list_sessions".to_string());
        }
        let handle = self.handle()?;
        let id = MachineId(machine);
        let mut asked = self.1.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if asked.get(&id).is_some_and(|at| now.saturating_duration_since(*at) < WINDOW_LIST_ASK_EVERY) {
            return Ok(());
        }
        handle.ask_list(id).map_err(|_| refusal(handle, id))?;
        asked.insert(id, now);
        Ok(())
    }

    /// Ask a machine's daemon (this computer's when none is named) for the folders inside `path`.
    /// The answer comes back as a `folder-listing` event carrying `tag`.
    pub fn list_folders(&self, machine: Option<u16>, path: &str, tag: u64) -> Result<(), String> {
        let path = path.trim();
        if path.is_empty() || path.len() > MOST_FOLDER_BYTES || path.chars().any(char::is_control) {
            return Err("Type a folder, such as ~ or /srv/app".to_string());
        }
        self.send(
            machine_of(machine),
            ClientMsg::Query(Query::ListFolders {
                path: path.to_string(),
                tag,
            }),
        )
    }

    /// Ask this computer's daemon for the shared settings. The answer comes back as a
    /// `shared-settings` event, the same one a change anywhere raises.
    pub fn shared_settings(&self) -> Result<(), String> {
        self.send(MachineId::LOCAL, ClientMsg::Query(Query::Settings))
    }

    /// Ask this computer's daemon to change one shared setting. The daemon checks the value and
    /// tells every client; this window learns of it from the `shared-settings` event.
    pub fn set_shared_setting(&self, key: SettingKey, value: SettingValue) -> Result<(), String> {
        self.send(
            MachineId::LOCAL,
            ClientMsg::Command(Command::SetSetting { key, value }),
        )
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

/// The folders inside `path` on a machine (this computer when `machine` is left out). Nothing is
/// returned here: the answer arrives as a `folder-listing` event with the same `tag`.
#[tauri::command]
pub fn list_folders(
    wire: tauri::State<'_, Wire>,
    machine: Option<u16>,
    path: String,
    tag: u64,
) -> Result<(), String> {
    wire.list_folders(machine, &path, tag)
}

/// Asks another machine for its whole list; it arrives as `machine-sessions-changed`.
#[tauri::command]
pub fn ask_machine_list(wire: tauri::State<'_, Wire>, machine: u16) -> Result<(), String> {
    wire.ask_machine_list(machine)
}

/// Asks the daemon for the shared settings; the answer arrives as a `shared-settings` event.
#[tauri::command]
pub fn get_shared_settings(wire: tauri::State<'_, Wire>) -> Result<(), String> {
    wire.shared_settings()
}

/// Changes one shared setting. `key` is the setting's name in config.toml terms (`default_agent`,
/// `new_session_view`, `sidebar_side`, `theme_dark`, `theme_light`, `appearance`, `spinner`,
/// `start_dir`, `reduce_motion`, `window_style`) and `value` the kind it takes; anything else is
/// refused here, and the daemon checks the value again.
#[tauri::command]
pub fn set_shared_setting(
    wire: tauri::State<'_, Wire>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let (key, value) = setting_from_wire(&key, value)?;
    wire.set_shared_setting(key, value)
}

/// The typed setting a name and a JSON value stand for.
pub(crate) fn setting_from_wire(
    key: &str,
    value: serde_json::Value,
) -> Result<(SettingKey, SettingValue), String> {
    fn text(value: serde_json::Value) -> Result<String, String> {
        match value {
            serde_json::Value::String(text) => Ok(text),
            _ => Err("This setting takes text".to_string()),
        }
    }
    fn word<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T, String> {
        serde_json::from_value(value).map_err(|_| "That is not one of this setting's values".to_string())
    }
    Ok(match key {
        "default_agent" => (SettingKey::DefaultAgent, SettingValue::Text(text(value)?)),
        "theme_dark" => (SettingKey::ThemeDark, SettingValue::Text(text(value)?)),
        "theme_light" => (SettingKey::ThemeLight, SettingValue::Text(text(value)?)),
        "start_dir" => (SettingKey::StartDir, SettingValue::Text(text(value)?)),
        "new_session_view" => (
            SettingKey::NewSessionView,
            SettingValue::NewSessionView(word::<NewSessionView>(value)?),
        ),
        "sidebar_side" => (
            SettingKey::SidebarSide,
            SettingValue::SidebarSide(word::<SidebarSide>(value)?),
        ),
        "appearance" => (
            SettingKey::Appearance,
            SettingValue::Appearance(word::<Appearance>(value)?),
        ),
        "window_style" => (
            SettingKey::WindowStyle,
            SettingValue::WindowStyle(word::<WindowStyle>(value)?),
        ),
        "reduce_motion" => match value {
            serde_json::Value::Bool(on) => (SettingKey::ReduceMotion, SettingValue::Bool(on)),
            _ => return Err("This setting takes true or false".to_string()),
        },
        "spinner" => (SettingKey::Spinner, SettingValue::List(word::<Vec<String>>(value)?)),
        _ => return Err("That is not a shared setting".to_string()),
    })
}

/// `machine` is left out for this computer's own panes.
///
/// Not `async` on purpose: Tauri runs a plain command on the main thread, one request at a time in
/// the order they arrive, so keys the window sends without waiting for each reply still reach the
/// pane in the order they were typed. An `async` command runs on any worker and could swap two.
/// The body only queues the bytes (`Wire::command` does not wait), so the main thread is not held.
#[tauri::command]
pub fn terminal_input(
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
    /// What another machine, opted in to notices, wants shown.
    Notice { machine: MachineId, text: String },
    /// Text an opted-in machine asks to be copied.
    Clipboard { machine: MachineId, text: String },
    /// A daemon decided a pane needs a person, and told this window.
    Attention {
        machine: MachineId,
        term: TermId,
        status: Status,
    },
    /// A machine was taken out of the list.
    Removed { machine: MachineId },
    /// The shared settings of this computer's daemon.
    Settings(SharedSettings),
    /// A conversation of this computer's daemon changed.
    ConversationChanged { conversation: u64, generation: u64 },
    /// A machine's answer about a folder, for the window that asked.
    Folders {
        machine: MachineId,
        tag: u64,
        listing: FolderListing,
    },
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
            let opts = handle.opts_of(machine);
            (told.clipboard, told.notices, told.sounds) = (opts.clipboard, opts.notices, opts.sounds);
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
            Out::Notice { machine, text } => Some(Reaction::Notice { machine, text }),
            Out::Clipboard { machine, text } => Some(Reaction::Clipboard { machine, text }),
            Out::Attention { machine, term, status } => Some(Reaction::Attention { machine, term, status }),
            Out::Settings(settings) => Some(Reaction::Settings(settings)),
            Out::ConversationChanged { conversation, generation } => {
                Some(Reaction::ConversationChanged { conversation, generation })
            }
            Out::Folders { machine, tag, listing } => Some(Reaction::Folders { machine, tag, listing }),
            Out::Sessions { machine, snapshot } => Some(Reaction::Sessions { machine, snapshot }),
            Out::WantReplay { machine, term } => {
                let _ = handle.send(Outgoing {
                    machine,
                    msg: ClientMsg::WantReplay { term },
                });
                None
            }
            Out::WantList { machine } => {
                // A write that fails here shows up as the line ending, and the list comes whole
                // with the next line.
                let _ = handle.ask_list(machine);
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
struct AttentionPayload {
    machine: u16,
    term: TermId,
    status: &'static str,
}

#[derive(Clone, Serialize)]
struct ConversationChangedPayload {
    conversation: u64,
    generation: u64,
}

#[derive(Clone, Serialize)]
struct MachineError {
    machine: u16,
    text: String,
}

#[derive(Clone, Serialize)]
struct FolderAnswer {
    machine: u16,
    tag: u64,
    listing: FolderListing,
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
            Reaction::Notice { machine, text } => {
                let _ = app.emit(
                    MACHINE_NOTICE_EVENT,
                    MachineError {
                        machine: machine.0,
                        text,
                    },
                );
            }
            Reaction::Attention { machine, term, status } => {
                let status = match status {
                    Status::Blocked => "blocked",
                    Status::Done => "done",
                    _ => continue,
                };
                let _ = app.emit(
                    SESSION_ATTENTION_EVENT,
                    AttentionPayload {
                        machine: machine.0,
                        term,
                        status,
                    },
                );
            }
            Reaction::Clipboard { machine, text } => {
                let _ = app.emit(
                    MACHINE_CLIPBOARD_EVENT,
                    MachineError {
                        machine: machine.0,
                        text,
                    },
                );
            }
            Reaction::Settings(settings) => {
                let _ = app.emit(SHARED_SETTINGS_EVENT, settings);
            }
            Reaction::ConversationChanged { conversation, generation } => {
                let _ = app.emit(
                    CONVERSATION_CHANGED_EVENT,
                    ConversationChangedPayload {
                        conversation,
                        generation,
                    },
                );
            }
            Reaction::Folders { machine, tag, listing } => {
                let _ = app.emit(
                    FOLDER_LISTING_EVENT,
                    FolderAnswer {
                        machine: machine.0,
                        tag,
                        listing,
                    },
                );
            }
            Reaction::Removed { machine } => {
                maitty_desktop_client::forget_remote(machine.0);
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
                // The window's own socket first: the channel waits for the host's main loop and
                // the web view's next event, which is what held a pane a step behind (ticket 35).
                let frame = match app.state::<crate::output_socket::OutputSocket>().send(frame) {
                    Ok(()) => {
                        crate::performance::count_terminal_frame_sent();
                        continue;
                    }
                    Err(frame) => frame,
                };
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
                    crate::performance::count_terminal_frame_sent();
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
            clipboard: false,
            notices: false,
            sounds: false,
        }
    }

    #[test]
    fn a_grant_is_written_to_the_entry_and_a_missing_entry_says_so() {
        let dir = std::env::temp_dir().join(format!("maitty-gui-grant-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("config.local.toml");
        std::fs::write(&file, "[[machines]]\nname = 'box'\ntarget = 'box'\n").unwrap();
        write_grant(&file, "BOX", Grant::Notices, true).unwrap();
        assert!(std::fs::read_to_string(&file).unwrap().contains("notices = true"));
        let missing = write_grant(&file, "ghost", Grant::Notices, true).unwrap_err();
        assert!(missing.contains("not in config.local.toml"), "{missing}");
        let _ = std::fs::remove_dir_all(&dir);
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

    #[test]
    fn a_shared_setting_is_typed_by_its_name_and_a_wrong_kind_is_refused() {
        use serde_json::json;
        assert_eq!(
            setting_from_wire("window_style", json!("flat")).unwrap(),
            (SettingKey::WindowStyle, SettingValue::WindowStyle(WindowStyle::Flat))
        );
        assert_eq!(
            setting_from_wire("reduce_motion", json!(true)).unwrap(),
            (SettingKey::ReduceMotion, SettingValue::Bool(true))
        );
        assert_eq!(
            setting_from_wire("start_dir", json!("/srv")).unwrap(),
            (SettingKey::StartDir, SettingValue::Text("/srv".to_string()))
        );
        assert_eq!(
            setting_from_wire("spinner", json!(["a", "b"])).unwrap(),
            (SettingKey::Spinner, SettingValue::List(vec!["a".to_string(), "b".to_string()]))
        );
        assert!(setting_from_wire("window_style", json!("tiled")).is_err(), "tiled is gone");
        assert!(setting_from_wire("reduce_motion", json!("yes")).is_err());
        assert!(setting_from_wire("start_dir", json!(3)).is_err());
        assert!(setting_from_wire("machines", json!("x")).is_err(), "only the shared keys are reachable");
    }

    #[test]
    fn only_this_computers_settings_reach_the_window() {
        let settings = SharedSettings {
            default_agent: "claude".to_string(),
            new_session_view: NewSessionView::Chat,
            sidebar_side: SidebarSide::Left,
            theme_dark: "Midnight".to_string(),
            theme_light: "Paper".to_string(),
            appearance: Appearance::Dark,
            spinner: vec![],
            start_dir: String::new(),
            reduce_motion: false,
            window_style: WindowStyle::Flat,
        };
        let mut streams = Streams::default();
        let now = Instant::now();
        let local = streams.on_message(MachineId::LOCAL, ServerMsg::SettingsChanged(settings.clone()), now);
        assert!(matches!(local.as_slice(), [Out::Settings(told)] if *told == settings));
        let answer = streams.on_message(
            MachineId::LOCAL,
            ServerMsg::Reply(maitty_shared::protocol::Reply::Settings(settings.clone())),
            now,
        );
        assert!(matches!(answer.as_slice(), [Out::Settings(_)]));
        let other = streams.on_message(MachineId(1), ServerMsg::SettingsChanged(settings), now);
        assert!(other.is_empty(), "another machine's settings are not shown");
    }

    #[test]
    fn a_changed_conversation_is_told_for_this_computer_only() {
        let changed = || ServerMsg::ConversationChanged {
            id: maitty_shared::conversation::ConversationId(7),
            generation: 2,
        };
        let mut streams = Streams::default();
        let now = Instant::now();
        let local = streams.on_message(MachineId::LOCAL, changed(), now);
        assert!(matches!(
            local.as_slice(),
            [Out::ConversationChanged {
                conversation: 7,
                generation: 2
            }]
        ));
        let other = streams.on_message(MachineId(1), changed(), now);
        assert!(other.is_empty(), "another machine's chats are not read here");
    }
}
