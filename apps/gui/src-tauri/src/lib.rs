mod add_machine;
mod browse;
mod file_get;
mod file_send;
mod attachment;
mod image_paste;
pub mod hub;
mod daemon_log;
mod highlight;
mod machine_update;
mod machines;
mod new_chat;
mod output_socket;
mod performance;
mod project_files;
pub mod routing;
pub mod streams;

use std::path::PathBuf;
mod terminal_stream;
mod tray;

use maitty_desktop_client::{FleetCommand, PaneCommand};
use tauri::Manager;

/// The daemon to start. A packaged app runs the shared per-user `maitty`, which it first brings
/// up to its own release (ADR 0004); a development build runs the one it just built.
fn daemon_executable(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        return bundled_daemon(app);
    }
    static SHARED: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
    let mut shared = SHARED
        .lock()
        .map_err(|_| "the daemon install lock was poisoned".to_string())?;
    if let Some(path) = shared.as_ref() {
        return Ok(path.clone());
    }
    let version = app.package_info().version.to_string();
    let path = maitty_desktop_client::ensure_shared_daemon(&bundled_daemon(app)?, &version)
        .map_err(|error| format!("{error:#}"))?;
    *shared = Some(path.clone());
    Ok(path)
}

/// The daemon file shipped inside the app: what gets installed, never what runs.
fn bundled_daemon(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let (file_name, development_path) = if cfg!(debug_assertions) {
        let file_name = format!(
            "binaries/maitty-gui-daemon-{}{suffix}",
            env!("MAITTY_TARGET_TRIPLE")
        );
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&file_name);
        (file_name, Some(path))
    } else {
        (format!("maitty-gui-daemon{suffix}"), None)
    };
    if let Some(path) = development_path {
        return path
            .is_file()
            .then_some(path)
            .ok_or_else(|| "the bundled daemon executable is missing".into());
    }

    let mut candidates = Vec::new();
    if let Ok(path) = app
        .path()
        .resolve(file_name, tauri::path::BaseDirectory::Resource)
    {
        candidates.push(path);
    }
    if let Ok(gui_executable) = std::env::current_exe()
        && let Some(directory) = gui_executable.parent()
    {
        candidates.push(directory.join(format!(
            "maitty-gui-daemon-{}{suffix}",
            env!("MAITTY_TARGET_TRIPLE")
        )));
        candidates.push(directory.join(format!("maitty-gui-daemon{suffix}")));
    }
    first_existing_daemon(&candidates)
        .ok_or_else(|| "the bundled daemon executable is missing".into())
}

fn first_existing_daemon(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.is_file()).cloned()
}

/// The agents whose status hooks the window may connect. Only these names ever reach
/// `maitty integrate`; the webview cannot choose what the host runs.
const CONNECTABLE_AGENTS: [&str; 3] = ["claude", "codex", "gemini"];

/// Connect an agent's status hooks (`maitty integrate <agent>`), so its chat sees when it works,
/// needs you or is done.
#[tauri::command]
async fn connect_agent(app: tauri::AppHandle, agent: String) -> Result<(), String> {
    if !CONNECTABLE_AGENTS.contains(&agent.as_str()) {
        return Err(format!("{agent} cannot be connected from here"));
    }
    let executable = daemon_executable(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut command = maitty_desktop_client::host_command(executable);
        command
            .args(["integrate", agent.as_str()])
            .stdin(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let output = command
            .output()
            .map_err(|error| format!("Could not connect {agent}: {error}"))?;
        if output.status.success() {
            return Ok(());
        }
        let text = String::from_utf8_lossy(&output.stderr);
        let reason = text.lines().rev().find(|line| !line.trim().is_empty());
        Err(reason.map_or_else(|| format!("Could not connect {agent}."), str::to_string))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn list_sessions(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    maitty_desktop_client::list_sessions(daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn read_conversation(
    app: tauri::AppHandle,
    id: u64,
    cursor: Option<serde_json::Value>,
    limit: usize,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::read_conversation(id, cursor, limit, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn conversation_diff(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::conversation_diff(id, generation, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// One model an agent offers, as its own cache of them lists it.
#[derive(Debug, PartialEq, serde::Serialize)]
struct AgentModel {
    id: String,
    label: String,
    blurb: String,
}

/// The models listed in Codex's own cache (`models_cache.json`, which Codex refreshes from the
/// account), the picker's ones first. Hidden models are left out.
fn codex_models_from(cache: &str) -> Result<Vec<AgentModel>, String> {
    let value: serde_json::Value =
        serde_json::from_str(cache).map_err(|error| format!("Codex's model list is unreadable: {error}"))?;
    let mut listed: Vec<(i64, AgentModel)> = value
        .get("models")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|model| model.get("visibility").and_then(serde_json::Value::as_str) == Some("list"))
        .filter_map(|model| {
            let text = |key: &str| model.get(key).and_then(serde_json::Value::as_str);
            let id = text("slug")?;
            let priority = model.get("priority").and_then(serde_json::Value::as_i64).unwrap_or(i64::MAX);
            Some((
                priority,
                AgentModel {
                    id: id.to_string(),
                    label: text("display_name").unwrap_or(id).to_string(),
                    blurb: text("description").unwrap_or_default().to_string(),
                },
            ))
        })
        .collect();
    listed.sort_by_key(|(priority, _)| *priority);
    Ok(listed.into_iter().map(|(_, model)| model).collect())
}

/// The extra models Claude Code offers this account, from `additionalModelOptionsCache` in its own
/// `.claude.json` (Claude Code refreshes it; the file is not a documented interface, so anything
/// unexpected gives an empty list and the window keeps its fixed aliases). Each entry reads
/// `Name 5.1 · what it is for`, and its value is what `--model` takes.
fn claude_models_from(config: &str) -> Vec<AgentModel> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(config) else {
        return Vec::new();
    };
    value
        .get("additionalModelOptionsCache")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|option| {
            let id = option.get("value")?.as_str()?.trim();
            if id.is_empty() {
                return None;
            }
            let text = option.get("description").and_then(serde_json::Value::as_str).unwrap_or_default();
            let (name, blurb) = text.split_once(" · ").unwrap_or((text, ""));
            let label = if name.is_empty() { option.get("label")?.as_str()? } else { name };
            Some(AgentModel { id: id.to_string(), label: label.to_string(), blurb: blurb.to_string() })
        })
        .collect()
}

/// The "Default" row for Claude: what Claude Code will run when no model is picked, from the
/// `model` in its `settings.json`. Without one, Claude Code picks by account, which cannot be read.
fn claude_default_from(settings: &str) -> AgentModel {
    let named = serde_json::from_str::<serde_json::Value>(settings)
        .ok()
        .and_then(|value| value.get("model").and_then(serde_json::Value::as_str).map(str::trim).filter(|name| !name.is_empty()).map(str::to_string));
    let blurb = match named {
        Some(name) => format!("Claude Code's own setting: {name}"),
        None => "Whatever Claude Code picks for your account".to_string(),
    };
    AgentModel { id: "default".into(), label: "Default".into(), blurb }
}

/// The models an agent offers on this computer. Codex keeps a list under its own folder; Claude
/// keeps its account's extra models in `.claude.json`. Gemini has none.
#[tauri::command]
async fn agent_models(app: tauri::AppHandle, agent: String) -> Result<Vec<AgentModel>, String> {
    if agent == "claude" {
        let folder = match std::env::var_os("CLAUDE_CONFIG_DIR") {
            Some(path) => PathBuf::from(path),
            None => app.path().home_dir().map_err(|error| error.to_string())?,
        };
        // A missing or unreadable file is no list, not a failure: the fixed aliases stay.
        let extras = std::fs::read_to_string(folder.join(".claude.json")).map(|text| claude_models_from(&text)).unwrap_or_default();
        let settings = match std::env::var_os("CLAUDE_CONFIG_DIR") {
            Some(path) => PathBuf::from(path),
            None => folder.join(".claude"),
        };
        let default = claude_default_from(&std::fs::read_to_string(settings.join("settings.json")).unwrap_or_default());
        return Ok(std::iter::once(default).chain(extras).collect());
    }
    if agent != "codex" {
        return Ok(Vec::new());
    }
    let folder = match std::env::var_os("CODEX_HOME") {
        Some(path) => PathBuf::from(path),
        None => app.path().home_dir().map_err(|error| error.to_string())?.join(".codex"),
    };
    let cache = folder.join("models_cache.json");
    match std::fs::read_to_string(&cache) {
        Ok(text) => codex_models_from(&text),
        // Codex has not fetched its list yet: nothing to show beyond its default.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not read {}: {error}", cache.display())),
    }
}

/// Where a new session starts when no folder is chosen: your home folder, as the terminal app does.
#[tauri::command]
fn default_session_folder(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .home_dir()
        .map(|home| home.display().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn suggest_project_folders(typed: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || project_files::suggest_folders(&typed))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn conversation_files(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    path: Option<String>,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::conversation_files(id, generation, path, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn conversation_file(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    path: String,
) -> Result<String, String> {
    maitty_desktop_client::conversation_file(id, generation, path, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn terminal_screen(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
) -> Result<String, String> {
    maitty_desktop_client::terminal_screen(id, generation, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Raw bytes: as JSON, a replay would be a number per byte, several megabytes of text to parse.
#[tauri::command]
async fn terminal_replay(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
) -> Result<tauri::ipc::Response, String> {
    maitty_desktop_client::terminal_replay(id, generation, daemon_executable(&app)?)
        .await
        .map(tauri::ipc::Response::new)
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn send_terminal_input(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    data: Vec<u8>,
) -> Result<String, String> {
    maitty_desktop_client::send_terminal_input(id, generation, data, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn resize_terminal(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    maitty_desktop_client::resize_terminal(id, generation, cols, rows, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn pane_terminal_screen(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
) -> Result<String, String> {
    wire.require_local(machine, "The screen text")?;
    maitty_desktop_client::pane_terminal_screen(term, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn pane_terminal_replay(
    app: tauri::AppHandle,
    term: u32,
) -> Result<tauri::ipc::Response, String> {
    maitty_desktop_client::pane_terminal_replay(term, daemon_executable(&app)?)
        .await
        .map(tauri::ipc::Response::new)
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn pane_terminal_resize(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    // Another machine's pane is asked over that machine's line; this computer's, as ever.
    if routing::is_remote(machine) {
        return wire.command(machine, term, PaneCommand::Resize { cols, rows });
    }
    maitty_desktop_client::pane_terminal_resize(term, cols, rows, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn repo_query(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    question: serde_json::Value,
) -> Result<serde_json::Value, String> {
    wire.require_local(machine, "Inspect")?;
    maitty_desktop_client::repo_query(term, question, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn new_worktree_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    branch: String,
    base: Option<String>,
) -> Result<serde_json::Value, String> {
    wire.require_local(machine, "Worktrees")?;
    maitty_desktop_client::new_worktree_session(term, branch, base, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn search_panes(app: tauri::AppHandle, query: String) -> Result<serde_json::Value, String> {
    maitty_desktop_client::search_panes(query, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// What agents reported using over a period (`day`, `week` or `month`), from the daemon's
/// usage store on this computer.
#[tauri::command]
async fn usage_report(app: tauri::AppHandle, period: String) -> Result<serde_json::Value, String> {
    let period = match period.as_str() {
        "day" => maitty_shared::usage_report::UsagePeriod::Day,
        "week" => maitty_shared::usage_report::UsagePeriod::Week,
        "month" => maitty_shared::usage_report::UsagePeriod::Month,
        other => return Err(format!("unknown usage period: {other}")),
    };
    maitty_desktop_client::usage_report(period, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn restore_snapshot(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    commit: String,
) -> Result<u64, String> {
    wire.require_local(machine, "Restoring a snapshot")?;
    maitty_desktop_client::restore_snapshot(term, commit, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn start_shell_session(
    app: tauri::AppHandle,
    cwd: PathBuf,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::start_shell_session(cwd, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Start an agent as the first command of a new terminal session (see `maitty_desktop_client::start_agent_session`).
#[tauri::command]
async fn start_agent_session(
    app: tauri::AppHandle,
    cwd: PathBuf,
    agent: String,
    flags: Vec<String>,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::start_agent_session(cwd, agent, flags, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn start_terminal_session(
    app: tauri::AppHandle,
    cwd: PathBuf,
    provider: maitty_shared::conversation::Provider,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::start_terminal_session(cwd, provider, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn send_when_agent_ready(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    text: String,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        let command =
            maitty_desktop_client::held_message(text).map_err(|error| format!("{error:#}"))?;
        return wire
            .command(machine, term, command)
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::send_when_ready(term, text, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn rename_terminal_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    name: String,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::Rename(name))
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::rename_terminal_session(term, name, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn pin_terminal_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    pinned: bool,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::Pin(pinned))
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::pin_terminal_session(term, pinned, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn archive_terminal_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    archived: bool,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::Archive(archived))
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::archive_terminal_session(term, archived, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn reload_terminal_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::Reload)
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::reload_terminal_session(term, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn mark_terminal_seen(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::Seen)
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::mark_terminal_seen(term, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn close_terminal_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::Close)
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::close_terminal_session(term, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn close_session_with_undo(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::CloseSession)
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::close_session_with_undo(term, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn undo_close_terminal_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::UndoClose)
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::undo_close_terminal_session(term, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Start a named fleet (holding one shell) on a machine. The answer to a request for another machine arrives with the next state update.
#[tauri::command]
async fn create_fleet(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    name: String,
) -> Result<serde_json::Value, String> {
    let command = FleetCommand::Create { name };
    if routing::is_remote(machine) {
        return wire.fleet(machine, command).map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::fleet_command(command, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Rename a fleet. The answer to a request for another machine arrives with the next state update.
#[tauri::command]
async fn rename_fleet(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    fleet: u32,
    name: String,
) -> Result<serde_json::Value, String> {
    let command = FleetCommand::Rename { fleet, name };
    if routing::is_remote(machine) {
        return wire.fleet(machine, command).map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::fleet_command(command, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Recolour a fleet with one of `maitty_desktop_client::FLEET_COLORS`. The answer to a request for another machine arrives with the next state update.
#[tauri::command]
async fn recolor_fleet(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    fleet: u32,
    color: u8,
) -> Result<serde_json::Value, String> {
    let command = FleetCommand::Recolor { fleet, color };
    if routing::is_remote(machine) {
        return wire.fleet(machine, command).map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::fleet_command(command, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Move a session to another fleet of the same machine. The answer to a request for another machine arrives with the next state update.
#[tauri::command]
async fn move_session_to_fleet(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    session: u32,
    fleet: u32,
) -> Result<serde_json::Value, String> {
    let command = FleetCommand::MoveSession { term: session, fleet };
    if routing::is_remote(machine) {
        return wire.fleet(machine, command).map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::fleet_command(command, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Start a shell in a fleet, in `folder` or the fleet's own folder. The answer to a request for another machine arrives with the next state update.
#[tauri::command]
async fn start_shell_in_fleet(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    fleet: u32,
    folder: Option<String>,
) -> Result<serde_json::Value, String> {
    let command = FleetCommand::StartShell { fleet, folder };
    if routing::is_remote(machine) {
        return wire.fleet(machine, command).map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::fleet_command(command, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn send_conversation(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    operation: u64,
    text: String,
    model: Option<String>,
    effort: Option<String>,
) -> Result<serde_json::Value, String> {
    let choice = maitty_shared::conversation::TurnChoice { model, effort };
    choice.validate().map_err(str::to_owned)?;
    maitty_desktop_client::send_conversation(
        id,
        generation,
        operation,
        text,
        Some(choice),
        daemon_executable(&app)?,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn approve_conversation(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    request_id: maitty_shared::conversation::ProviderRequestId,
    decision: maitty_shared::conversation::ApprovalDecision,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::approve_conversation(
        id,
        generation,
        request_id,
        decision,
        daemon_executable(&app)?,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn answer_conversation_input(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    request_id: maitty_shared::conversation::ProviderRequestId,
    answers: std::collections::BTreeMap<String, Vec<String>>,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::answer_user_input(
        id,
        generation,
        request_id,
        answers,
        daemon_executable(&app)?,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn cancel_conversation(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::cancel_conversation(id, generation, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn dequeue_conversation(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    operation: u64,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::dequeue_conversation(id, generation, operation, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn rename_conversation(
    app: tauri::AppHandle,
    id: u64,
    title: String,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::rename_conversation(id, title, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn archive_conversation(
    app: tauri::AppHandle,
    id: u64,
    archived: bool,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::archive_conversation(id, archived, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn resume_codex_conversation(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::resume_codex_conversation(id, generation, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn end_conversation(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::end_conversation(id, generation, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// A chat session (ADR 0003): a structured Claude or Codex conversation the daemon owns, in `cwd` or,
/// with `branch`, in a new worktree of the project at `cwd`. The session asks before a tool use the
/// provider does not allow by itself, unless `full_access` is true (the provider's skip-permissions mode).
#[tauri::command]
async fn start_chat_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    cwd: PathBuf,
    provider: maitty_shared::conversation::Provider,
    branch: Option<String>,
    full_access: Option<bool>,
) -> Result<serde_json::Value, String> {
    wire.require_local(machine, "A chat session")?;
    maitty_desktop_client::start_chat_session(
        cwd,
        provider,
        branch,
        full_access.unwrap_or(false),
        daemon_executable(&app)?,
    )
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn create_conversation(
    app: tauri::AppHandle,
    cwd: PathBuf,
    provider: maitty_shared::conversation::Provider,
    permissions: String,
) -> Result<serde_json::Value, String> {
    let permissions = maitty_desktop_client::Permissions::parse(&permissions)
        .map_err(|error| format!("{error:#}"))?;
    maitty_desktop_client::create_conversation(
        cwd,
        provider,
        permissions,
        None,
        daemon_executable(&app)?,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

/// Runs a search of the disk away from the window's own thread, where a plain command runs and
/// would hold up every key and frame for as long as the search takes.
async fn found_off_the_window_thread(
    find: fn() -> anyhow::Result<PathBuf>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        find()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|error| format!("{error:#}"))
    })
    .await
    .map_err(|error| format!("the search for the program stopped: {error}"))?
}

#[tauri::command]
async fn codex_executable_path() -> Result<String, String> {
    found_off_the_window_thread(maitty_desktop_client::codex_app_server_executable).await
}

#[tauri::command]
async fn claude_executable_path() -> Result<String, String> {
    found_off_the_window_thread(maitty_desktop_client::claude_executable).await
}

#[tauri::command]
async fn daemon_restart_status() -> Result<serde_json::Value, String> {
    maitty_desktop_client::daemon_restart_status()
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn restart_mismatched_daemon(expected_protocol: u32) -> Result<(), String> {
    maitty_desktop_client::restart_mismatched_daemon(expected_protocol)
        .await
        .map_err(|error| format!("{error:#}"))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(machines::Machines(maitty_machines::local_machines::local_machines()))
        .manage(terminal_stream::StreamState::default())
        .manage(terminal_stream::MachineStates::default())
        .manage(terminal_stream::TerminalOutput::default())
        .manage(terminal_stream::Wire::default())
        .manage(performance::ProcessStats::default())
        .manage(add_machine::AddProbe::default())
        .manage(file_send::Transfers::default())
        .setup(|app| {
            // The development server's origin is let in only by a debug build.
            let dev_origin = cfg!(debug_assertions)
                .then(|| app.config().build.dev_url.as_ref().map(|url| url.origin().ascii_serialization()))
                .flatten();
            app.manage(output_socket::OutputSocket::start(dev_origin));
            terminal_stream::start(
                app.handle().clone(),
                app.state::<machines::Machines>().0.machines.clone(),
            );
            // Bringing the shared daemon up to this release copies a file and may run it. Done
            // here, on a thread of its own, the first command that needs the daemon finds it
            // ready and does not do that work on whichever thread it runs on.
            // The app works without its tray item, so one the system refuses is only said.
            if let Err(error) = tray::start(app.handle()) {
                eprintln!("adding the tray item: {error}");
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = daemon_executable(&handle) {
                    eprintln!("preparing the daemon: {error}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            tray::set_tray_sessions,
            connect_agent,
            agent_models,
            new_chat::agents_installed,
            new_chat::git_branches,
            new_chat::git_switch_branch,
            new_chat::git_add_worktree,
            new_chat::open_agent_docs,
            machines::list_machines,
            terminal_stream::list_machines_overview,
            terminal_stream::retry_machine,
            daemon_log::open_daemon_log,
            highlight::highlight_file,
            machine_update::update_machine,
            machine_update::machine_activity,
            terminal_stream::start_session_on_machine,
            terminal_stream::remove_machine,
            terminal_stream::set_machine_grant,
            add_machine::list_ssh_hosts,
            add_machine::add_machine,
            add_machine::cancel_add_machine,
            file_send::send_files_to_machine,
            browse::list_remote_folder,
            browse::index_remote_files,
            file_get::get_file_from_machine,
            file_get::open_pulled_folder,
            file_send::cancel_file_send,
            attachment::attach_local_file,
            attachment::attachments_exist,
            attachment::attachment_image,
            image_paste::save_pasted_image,
            image_paste::send_pasted_image,
            read_conversation,
            conversation_diff,
            conversation_files,
            conversation_file,
            suggest_project_folders,
            default_session_folder,
            terminal_stream::terminal_history,
            terminal_stream::watch_terminal_output,
            terminal_stream::list_folders,
            terminal_stream::get_shared_settings,
            terminal_stream::set_shared_setting,
            output_socket::open_terminal_socket,
            terminal_screen,
            terminal_replay,
            send_terminal_input,
            resize_terminal,
            pane_terminal_screen,
            pane_terminal_replay,
            terminal_stream::terminal_input,
            pane_terminal_resize,
            repo_query,
            search_panes,
            usage_report,
            new_worktree_session,
            restore_snapshot,
            start_agent_session,
            start_terminal_session,
            start_shell_session,
            rename_terminal_session,
            send_when_agent_ready,
            mark_terminal_seen,
            pin_terminal_session,
            archive_terminal_session,
            reload_terminal_session,
            close_terminal_session,
            close_session_with_undo,
            undo_close_terminal_session,
            create_fleet,
            rename_fleet,
            recolor_fleet,
            move_session_to_fleet,
            start_shell_in_fleet,
            send_conversation,
            approve_conversation,
            answer_conversation_input,
            cancel_conversation,
            dequeue_conversation,
            rename_conversation,
            archive_conversation,
            resume_codex_conversation,
            create_conversation,
            start_chat_session,
            end_conversation,
            codex_executable_path,
            claude_executable_path,
            daemon_restart_status,
            restart_mismatched_daemon,
            performance::performance_sample
        ])
        .run(tauri::generate_context!())
        .expect("Maitty desktop host failed");
}

#[cfg(test)]
mod tests {
    #[test]
    fn codex_models_come_in_priority_order_without_the_hidden_ones() {
        let cache = r#"{"models":[
            {"slug":"b","display_name":"B","description":"second","visibility":"list","priority":2},
            {"slug":"hidden","display_name":"H","visibility":"hide","priority":0},
            {"slug":"a","display_name":"A","description":"first","visibility":"list","priority":1}]}"#;
        let models = super::codex_models_from(cache).unwrap();
        let ids: Vec<&str> = models.iter().map(|model| model.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(models[0].label, "A");
        assert!(super::codex_models_from("not json").is_err());
    }

    #[test]
    fn claude_offers_the_extra_models_its_own_config_lists() {
        let config = r#"{"additionalModelOptionsCache":[{"description":"Fable 5.1 · Most capable for your hardest tasks","label":"Fable","value":"claude-fable-5-1[1m]"}]}"#;
        assert_eq!(
            super::claude_models_from(config),
            vec![super::AgentModel { id: "claude-fable-5-1[1m]".into(), label: "Fable 5.1".into(), blurb: "Most capable for your hardest tasks".into() }]
        );
        assert!(super::claude_models_from("not json").is_empty());
        assert_eq!(super::claude_default_from(r#"{"model":"sonnet"}"#).blurb, "Claude Code's own setting: sonnet");
        assert!(super::claude_default_from("{}").blurb.starts_with("Whatever Claude Code picks"));
        assert!(super::claude_models_from("{}").is_empty());
    }

    use super::first_existing_daemon;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn daemon_lookup_uses_the_first_existing_packaged_or_adjacent_binary() {
        let directory = std::env::temp_dir().join(format!(
            "maitty-gui-daemon-path-{}-{}",
            std::process::id(),
            NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();

        let missing_resource = directory.join("resources").join("maitty.exe");
        let adjacent_sidecar = directory.join("maitty-gui-daemon-x86_64-pc-windows-msvc.exe");
        let adjacent_daemon = directory.join("maitty.exe");
        std::fs::write(&adjacent_sidecar, []).unwrap();
        std::fs::write(&adjacent_daemon, []).unwrap();

        assert_eq!(
            first_existing_daemon(&[missing_resource, adjacent_sidecar.clone(), adjacent_daemon,]),
            Some(adjacent_sidecar)
        );

        let packaged_directory = directory.join("resources");
        std::fs::create_dir(&packaged_directory).unwrap();
        let packaged_daemon = packaged_directory.join("maitty.exe");
        std::fs::write(&packaged_daemon, []).unwrap();
        assert_eq!(
            first_existing_daemon(&[
                packaged_daemon.clone(),
                directory.join("maitty-gui-daemon-x86_64-pc-windows-msvc.exe"),
                directory.join("maitty.exe"),
            ]),
            Some(packaged_daemon.clone())
        );

        std::fs::remove_file(directory.join("maitty-gui-daemon-x86_64-pc-windows-msvc.exe"))
            .unwrap();
        std::fs::remove_file(directory.join("maitty.exe")).unwrap();
        std::fs::remove_file(packaged_daemon).unwrap();
        std::fs::remove_dir(packaged_directory).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
