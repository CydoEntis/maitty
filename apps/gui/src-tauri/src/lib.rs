mod add_machine;
mod browse;
mod file_get;
mod file_send;
mod attachment;
mod image_paste;
pub mod hub;
mod agent_auth;
mod daemon_log;
mod export;
mod highlight;
#[cfg(target_os = "linux")]
mod linux_gpu;
mod machine_update;
mod machines;
mod changes;
mod claude_models;
mod codex_skills;
mod new_chat;
mod open_link;
mod output_socket;
mod performance;
mod project_files;
pub mod routing;
pub mod streams;

use std::path::PathBuf;
mod terminal_stream;
mod tray;
mod worktrees;

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
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    id: u64,
    cursor: Option<serde_json::Value>,
    limit: usize,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::read_conversation(id, cursor, limit, wire.daemon(machine, || daemon_executable(&app))?)
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
#[derive(Debug, Default, PartialEq, serde::Serialize)]
struct AgentModel {
    id: String,
    label: String,
    blurb: String,
    /// The model the agent runs when none is picked, where the agent says which that is.
    #[serde(rename = "isDefault")]
    is_default: bool,
    /// The effort levels the model takes, where the agent lists them; `None` when it does not say.
    efforts: Option<Vec<String>>,
    /// An earlier model, kept under "More models" so the newest ones lead the list.
    more: bool,
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
                    efforts: codex_efforts(model),
                    ..Default::default()
                },
            ))
        })
        .collect();
    listed.sort_by_key(|(priority, _)| *priority);
    Ok(listed.into_iter().map(|(_, model)| model).collect())
}

/// The model Codex runs when none is picked: the top-level `model` in its `config.toml`. Only that
/// one line is read, so a table's own `model` key (a profile's) is never taken for it.
fn codex_default_model(config: &str) -> Option<String> {
    config
        .lines()
        .map(str::trim)
        .take_while(|line| !line.starts_with('['))
        .find_map(|line| {
            let value = line.strip_prefix("model")?.trim_start().strip_prefix('=')?.trim();
            let name = value.strip_prefix('"')?.split('"').next()?;
            is_model_name(name).then(|| name.to_string())
        })
}

/// Marks the model Codex's settings name as the default, where the list has it.
fn mark_codex_default(mut models: Vec<AgentModel>, config: &str) -> Vec<AgentModel> {
    if let Some(name) = codex_default_model(config) {
        for model in &mut models {
            model.is_default = model.id == name;
        }
    }
    models
}

/// The effort levels a model in Codex's cache says it takes (`supported_reasoning_levels`).
fn codex_efforts(model: &serde_json::Value) -> Option<Vec<String>> {
    let levels = model.get("supported_reasoning_levels")?.as_array()?;
    Some(levels.iter().filter_map(|level| level.get("effort")?.as_str().map(str::to_string)).collect())
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
            Some(AgentModel { id: id.to_string(), label: label.to_string(), blurb: blurb.to_string(), ..Default::default() })
        })
        .collect()
}

/// The longest model name taken from a settings file.
const MAX_MODEL_NAME_CHARS: usize = 100;

/// A name `--model` could take: plain model-id characters, so nothing read from a file reaches a
/// launch line as anything else.
fn is_model_name(name: &str) -> bool {
    let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '/' | '[' | ']' | '-');
    !name.is_empty() && name.chars().count() <= MAX_MODEL_NAME_CHARS && name.chars().all(plain)
}

/// The model Claude Code runs when none is picked: the `model` in its `settings.json`. Without
/// one, Claude Code picks by account, which cannot be read.
fn claude_default_model(settings: &serde_json::Value) -> Option<String> {
    let name = settings.get("model")?.as_str()?.trim();
    is_model_name(name).then(|| name.to_string())
}

/// The models this account has settings of its own for (`modelSettings` in `settings.json`, keyed
/// by model id): Claude Code writes one when a model is used, so these are models the account runs.
fn claude_used_models(settings: &serde_json::Value) -> Vec<AgentModel> {
    settings
        .get("modelSettings")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|models| models.keys())
        .filter(|name| is_model_name(name))
        .map(|name| AgentModel { id: name.clone(), label: name.clone(), blurb: String::new(), ..Default::default() })
        .collect()
}

/// A full model id, not an alias: an alias follows the newest model of its family, so the ids are
/// the earlier models.
const FULL_MODEL_ID_PREFIX: &str = "claude-";
/// The entry Claude Code lists for "whatever I would pick": not a model, but it says which one that is.
const CLAUDE_DEFAULT_ENTRY: &str = "default";

/// Claude's rows from the list Claude Code itself gave: every model with the efforts it takes, the
/// newest first and the earlier ones marked `more`. The default is the model the settings name, else
/// the one Claude Code's own "default" entry stands for; it leads the list.
fn claude_rows_from_list(listed: Vec<claude_models::Listed>, settings: &str) -> Vec<AgentModel> {
    let settings = serde_json::from_str::<serde_json::Value>(settings).unwrap_or_default();
    let named = claude_default_model(&settings);
    let own_pick = listed.iter().find(|model| model.id == CLAUDE_DEFAULT_ENTRY).map(|model| model.resolved.clone());
    let default = named.clone().or(own_pick);
    let blurb = match &default {
        Some(name) => format!("Claude Code's own choice: {name}"),
        None => "Whatever Claude Code picks for your account".to_string(),
    };
    let models: Vec<&claude_models::Listed> =
        listed.iter().filter(|model| model.id != CLAUDE_DEFAULT_ENTRY && is_model_name(&model.id)).collect();
    // An alias is preferred to the full id it stands for, so the default follows new releases.
    let default_id = default.as_ref().and_then(|name| {
        models
            .iter()
            .find(|model| &model.id == name)
            .or_else(|| models.iter().find(|model| &model.resolved == name))
            .map(|model| model.id.clone())
    });
    let mut rows: Vec<AgentModel> = models
        .into_iter()
        .map(|model| AgentModel {
            id: model.id.clone(),
            label: model.label.clone(),
            blurb: model.blurb.clone(),
            is_default: Some(&model.id) == default_id.as_ref(),
            efforts: Some(model.efforts.clone()),
            more: model.id.starts_with(FULL_MODEL_ID_PREFIX),
        })
        .collect();
    // The settings name a model the list does not have: it still runs, so it is still offered.
    if let (None, Some(name)) = (&default_id, named) {
        rows.push(AgentModel { id: name.clone(), label: name, is_default: true, ..Default::default() });
    }
    rows.sort_by_key(|row| (row.more, !row.is_default));
    rows.insert(0, AgentModel { id: CLAUDE_DEFAULT_ENTRY.into(), label: "Default".into(), blurb, ..Default::default() });
    rows
}

/// Claude's rows when Claude Code could not be asked: a "Default" that says what it stands for, the
/// account's extra models, the models it has used, and the one its settings name, marked as the
/// default. This is only what its own files show; the window adds its fixed aliases.
fn claude_rows(config: &str, settings: &str) -> Vec<AgentModel> {
    let settings = serde_json::from_str::<serde_json::Value>(settings).unwrap_or_default();
    let named = claude_default_model(&settings);
    let blurb = match &named {
        Some(name) => format!("Claude Code's own setting: {name}"),
        None => "Whatever Claude Code picks for your account".to_string(),
    };
    let mut rows = vec![AgentModel { id: "default".into(), label: "Default".into(), blurb, ..Default::default() }];
    for model in claude_models_from(config).into_iter().chain(claude_used_models(&settings)) {
        if !rows.iter().any(|row| row.id == model.id) {
            rows.push(model);
        }
    }
    if let Some(name) = named {
        match rows.iter_mut().find(|row| row.id == name) {
            Some(row) => row.is_default = true,
            None => rows.push(AgentModel { id: name.clone(), label: name, blurb: String::new(), is_default: true, ..Default::default() }),
        }
    }
    rows
}

/// Claude Code's own list of its models (`claude_models::ask`), started in the home folder so no
/// project's settings shape it.
async fn ask_claude_for_models(home: &std::path::Path) -> Result<Vec<claude_models::Listed>, String> {
    claude_models::ask(&find_claude().await?, home).await
}

/// Where Claude Code is on this computer. The search reads the disk, so it runs off the async threads.
async fn find_claude() -> Result<PathBuf, String> {
    tauri::async_runtime::spawn_blocking(maitty_desktop_client::claude_executable)
        .await
        .map_err(|error| format!("the search for Claude Code stopped: {error}"))?
        .map_err(|error| format!("{error:#}"))
}

/// The slash commands an agent takes in `folder`, for the chat's command picker. Claude Code is asked
/// for its own list (`claude_models::ask_commands`), started in the session's folder so the project's
/// commands and skills are among them. Codex is asked for its skills (`codex_skills::ask_skills`), which
/// are all it lists. The other agents have no list to ask for yet. The window names
/// the folder, so only a full path to a folder that exists is started in, and the home folder
/// otherwise. That folder is one an agent of the person's already runs in.
#[tauri::command]
async fn agent_commands(app: tauri::AppHandle, agent: String, folder: String) -> Result<Vec<claude_models::Command>, String> {
    if agent != "claude" && agent != "codex" {
        return Ok(Vec::new());
    }
    let home = app.path().home_dir().map_err(|error| error.to_string())?;
    let start = tauri::async_runtime::spawn_blocking(move || {
        let folder = PathBuf::from(folder);
        if folder.is_absolute() && folder.is_dir() { folder } else { home }
    })
    .await
    .map_err(|error| error.to_string())?;
    if agent == "codex" {
        // Codex lists only its skills (typed after a `$`); its own commands are a list the window keeps.
        let codex = tauri::async_runtime::spawn_blocking(maitty_desktop_client::codex_app_server_executable)
            .await
            .map_err(|error| format!("the search for Codex stopped: {error}"))?
            .map_err(|error| format!("{error:#}"))?;
        return codex_skills::ask_skills(&codex, &start).await;
    }
    claude_models::ask_commands(&find_claude().await?, &start).await
}

/// The models an agent offers on this computer. Claude Code is asked for its own list
/// (`claude_rows_from_list`), and when it cannot be, its files are read instead (`claude_rows`).
/// Codex keeps a list under its own folder. Gemini has none.
#[tauri::command]
async fn agent_models(app: tauri::AppHandle, agent: String) -> Result<Vec<AgentModel>, String> {
    if agent == "claude" {
        let folder = match std::env::var_os("CLAUDE_CONFIG_DIR") {
            Some(path) => PathBuf::from(path),
            None => app.path().home_dir().map_err(|error| error.to_string())?,
        };
        // A missing or unreadable file is no list, not a failure: the fixed aliases stay.
        let config = std::fs::read_to_string(folder.join(".claude.json")).unwrap_or_default();
        let settings = match std::env::var_os("CLAUDE_CONFIG_DIR") {
            Some(path) => PathBuf::from(path),
            None => folder.join(".claude"),
        };
        let settings = std::fs::read_to_string(settings.join("settings.json")).unwrap_or_default();
        let home = app.path().home_dir().map_err(|error| error.to_string())?;
        match ask_claude_for_models(&home).await {
            Ok(listed) if !listed.is_empty() => return Ok(claude_rows_from_list(listed, &settings)),
            Ok(_) => tracing::warn!("Claude Code listed no models; reading its files instead"),
            Err(error) => tracing::warn!("{error}; reading Claude Code's files instead"),
        }
        return Ok(claude_rows(&config, &settings));
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
        // A missing or unreadable settings file names no default: the Default row stays.
        Ok(text) => codex_models_from(&text)
            .map(|models| mark_codex_default(models, &std::fs::read_to_string(folder.join("config.toml")).unwrap_or_default())),
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

/// A worktree of the project at `folder` on `branch`, made by the daemon (trusted like its repo, the repo's
/// setup run, tracked for tidy-up). Answers its folder. This computer's daemon only: the new chat page is
/// this computer's, and the web view names no executable or machine.
#[tauri::command]
async fn add_branch_worktree(folder: PathBuf, branch: String) -> Result<serde_json::Value, String> {
    maitty_desktop_client::branch_worktree(folder, branch)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// The conversations `agent` saved for `folder`, for the new chat page's Resume list; null when the service
/// running is older. This computer's daemon only, which checks the agent and the folder.
#[tauri::command]
async fn saved_conversations(agent: String, folder: String) -> Result<serde_json::Value, String> {
    maitty_desktop_client::saved_conversations(agent, folder)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// The agent sessions that closed on this computer, for the Recent sessions list, each saying whether its folder is
/// still there; null when the service running is older. This computer's daemon only: a folder another machine names is
/// not on this disk.
#[tauri::command]
async fn closed_sessions() -> Result<serde_json::Value, String> {
    maitty_desktop_client::closed_sessions()
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Reopens a closed session from the Recent sessions list whose folder is gone: the daemon makes its worktree again
/// from the record it keeps and starts the agent there with its conversation. False when the service running is older.
/// This computer's daemon only, like `closed_sessions`.
#[tauri::command]
async fn reopen_closed_session(agent: String, session: String) -> Result<serde_json::Value, String> {
    maitty_desktop_client::reopen_closed_session(agent, session)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Removes worktrees of the project with the daemon, which closes what runs in them first: each folder with
/// whether to go on when it holds uncommitted changes (they are lost). A branch is kept unless it is already
/// merged. The daemon answers at once and does the removing in the background; a worktree something still runs
/// in is left, so the caller looks again.
#[tauri::command]
async fn remove_worktrees(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    items: Vec<(PathBuf, bool)>,
) -> Result<serde_json::Value, String> {
    wire.require_local(machine, "Worktrees")?;
    maitty_desktop_client::tidy_worktrees(items, daemon_executable(&app)?)
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

/// How many files restoring the snapshot would change now, asked before the person confirms one.
#[tauri::command]
async fn snapshot_restore_count(
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    commit: String,
) -> Result<u64, String> {
    wire.require_local(machine, "Restoring a snapshot")?;
    maitty_desktop_client::restore_count(term, commit)
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

/// Start an agent ahead of its first message (see `maitty_desktop_client::prewarm_agent_session`): the same agent
/// names and flag rules as `start_agent_session`.
#[tauri::command]
async fn prewarm_agent_session(cwd: PathBuf, agent: String, flags: Vec<String>) -> Result<serde_json::Value, String> {
    maitty_desktop_client::prewarm_agent_session(cwd, agent, flags)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn drop_prewarmed_agent() -> Result<serde_json::Value, String> {
    maitty_desktop_client::drop_prewarmed_agent()
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

/// Holds a message with the pane's daemon for its working agent: typed in when the turn is over,
/// whether or not this window is still open.
#[tauri::command]
async fn queue_follow_up(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    text: String,
    first: bool,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        let command = maitty_desktop_client::queued_message(text, first)
            .map_err(|error| format!("{error:#}"))?;
        return wire
            .command(machine, term, command)
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::queue_follow_up(term, text, first, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Takes one queued message back from the pane's daemon.
#[tauri::command]
async fn drop_queued_follow_up(
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    index: u32,
    text: String,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::DropQueued { index, text })
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::drop_queued(term, index, text)
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

/// Snooze a pane until `until` (unix seconds), or wake it (`None`).
#[tauri::command]
async fn snooze_terminal_session(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    until: Option<u64>,
) -> Result<serde_json::Value, String> {
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::Snooze(until))
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::snooze_terminal_session(term, until, daemon_executable(&app)?)
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

/// The person's answer to a session that asked to start helpers: `allow`, `always` or `no`.
#[tauri::command]
async fn answer_helpers(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    choice: String,
) -> Result<serde_json::Value, String> {
    let choice = maitty_desktop_client::helper_choice(&choice).map_err(|error| format!("{error:#}"))?;
    if routing::is_remote(machine) {
        return wire
            .command(machine, term, PaneCommand::AnswerHelpers(choice))
            .map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::answer_helpers(term, choice, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Does something to a pane's checks: `rerun`, `override` (with a reason), `undo` an override, or `approve` a
/// repo's check command (`choice`: `once`, `always` or `never`; `command` is the one the person was shown). Works for
/// a pane on another machine as well: the checks are that machine's daemon's.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn check_action(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    action: String,
    name: Option<String>,
    reason: Option<String>,
    choice: Option<String>,
    command: Option<String>,
) -> Result<serde_json::Value, String> {
    let pane = match action.as_str() {
        "rerun" => PaneCommand::RerunCheck { name },
        "override" => PaneCommand::OverrideCheck { name, reason: reason.unwrap_or_default() },
        "undo" => PaneCommand::UndoOverride { name },
        "approve" => PaneCommand::ApproveCheck {
            name: name.ok_or("a check approval names the check")?,
            choice: maitty_desktop_client::approval_choice(&choice.ok_or("a check approval needs a choice")?).map_err(|error| format!("{error:#}"))?,
            command: command.ok_or("a check approval needs the command that was shown")?,
        },
        other => return Err(format!("unknown check action: {other}")),
    };
    if routing::is_remote(machine) {
        return wire.command(machine, term, pane).map(|()| serde_json::Value::Null);
    }
    maitty_desktop_client::check_command(term, pane, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Saves the text of one existing file of the checkout `term` is in. The daemon checks the path (inside the
/// checkout, no links, not under `.git`) and makes no new file. This computer's panes only.
#[tauri::command]
async fn write_project_file(
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    path: String,
    contents: String,
) -> Result<serde_json::Value, String> {
    wire.require_local(machine, "Editing a file")?;
    maitty_desktop_client::write_project_file(term, path, contents)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// The person's answer to an agent's request for a secret: the value, or none to decline (ADR 0006). This
/// computer's daemon only; the value is passed on and not kept or logged.
#[tauri::command]
async fn answer_secret(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    id: u64,
    value: Option<String>,
) -> Result<serde_json::Value, String> {
    wire.require_local(machine, "Giving a secret")?;
    maitty_desktop_client::answer_secret(id, value, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// The files more than one of a project's checkouts changed (name, folder per checkout). This computer's only.
#[tauri::command]
async fn worktree_overlaps(
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    checkouts: Vec<(String, String)>,
) -> Result<Vec<maitty_shared::overlap::Overlap>, String> {
    wire.require_local(machine, "Worktrees")?;
    maitty_desktop_client::checkout_overlaps(checkouts.into_iter().map(|(name, folder)| (name, PathBuf::from(folder))).collect())
        .await
        .map_err(|error| format!("{error:#}"))
}

/// The brief a new agent starts with when the work of the agent in `term` is handed to it. This computer's panes only.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn handoff_brief(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    agent: String,
    task: String,
    last_prompt: String,
    branch: Option<String>,
    checkout: String,
) -> Result<String, String> {
    wire.require_local(machine, "Handing off")?;
    maitty_desktop_client::handoff_brief(term, agent, task, last_prompt, branch, PathBuf::from(checkout), daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// The tail of a pane's last output for check `name` (None: the headline one). This computer's panes only.
#[tauri::command]
async fn check_output(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    term: u32,
    name: Option<String>,
) -> Result<serde_json::Value, String> {
    wire.require_local(machine, "A check's output")?;
    maitty_desktop_client::check_output(term, name, daemon_executable(&app)?)
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
    machine: Option<u16>,
    id: u64,
    generation: u64,
    operation: u64,
    text: String,
    choice: Option<maitty_shared::conversation::TurnChoice>,
) -> Result<serde_json::Value, String> {
    let choice = choice.unwrap_or_default();
    choice.validate().map_err(str::to_owned)?;
    let wire = app.state::<terminal_stream::Wire>();
    maitty_desktop_client::send_conversation(
        id,
        generation,
        operation,
        text,
        Some(choice),
        wire.daemon(machine, || daemon_executable(&app))?,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn approve_conversation(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
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
        wire.daemon(machine, || daemon_executable(&app))?,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

/// The commands approved "always in this project", for Settings. This computer's service only.
#[tauri::command]
async fn allowed_commands(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    maitty_desktop_client::allowed_commands(daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

/// Takes back one approved command: the service forgets it at once and updates its file.
#[tauri::command]
async fn forget_allowed_command(app: tauri::AppHandle, project: String, command_sha256: String) -> Result<serde_json::Value, String> {
    maitty_desktop_client::forget_allowed_command(PathBuf::from(project), command_sha256, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn allow_conversation_always(
    app: tauri::AppHandle,
    id: u64,
    generation: u64,
    request_id: maitty_shared::conversation::ProviderRequestId,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::allow_conversation_always(id, generation, request_id, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn close_conversation_with_undo(app: tauri::AppHandle, id: u64, generation: u64) -> Result<serde_json::Value, String> {
    maitty_desktop_client::close_conversation_with_undo(id, generation, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn undo_close_conversation(app: tauri::AppHandle, id: u64) -> Result<serde_json::Value, String> {
    maitty_desktop_client::undo_close_conversation(id, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn answer_conversation_input(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
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
        wire.daemon(machine, || daemon_executable(&app))?,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn cancel_conversation(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    id: u64,
    generation: u64,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::cancel_conversation(id, generation, wire.daemon(machine, || daemon_executable(&app))?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn dequeue_conversation(
    app: tauri::AppHandle,
    wire: tauri::State<'_, terminal_stream::Wire>,
    machine: Option<u16>,
    id: u64,
    generation: u64,
    operation: u64,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::dequeue_conversation(id, generation, operation, wire.daemon(machine, || daemon_executable(&app))?)
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

#[tauri::command]
async fn daemon_update_status(app: tauri::AppHandle) -> serde_json::Value {
    maitty_desktop_client::daemon_update_status(&app.package_info().version.to_string())
}

#[tauri::command]
async fn restart_stale_daemon(
    app: tauri::AppHandle,
    expected_version: String,
) -> Result<(), String> {
    maitty_desktop_client::restart_stale_daemon(
        &app.package_info().version.to_string(),
        &expected_version,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

pub fn run() {
    #[cfg(target_os = "linux")]
    linux_gpu::prefer_gpu_drawing();
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
            agent_commands,
            agent_auth::agent_auth_status,
            agent_auth::agent_sign_out,
            new_chat::agents_installed,
            new_chat::git_branches,
            new_chat::git_switch_branch,
            add_branch_worktree,
            saved_conversations,
            closed_sessions,
            reopen_closed_session,
            new_chat::git_sync_state,
            new_chat::git_pull,
            new_chat::open_agent_docs,
            changes::git_commit_all,
            changes::git_create_pull_request,
            changes::git_sync,
            worktrees::worktree_standing,
            worktrees::worktree_recreate,
            worktrees::worktree_merge,
            remove_worktrees,
            machines::list_machines,
            terminal_stream::list_machines_overview,
            terminal_stream::retry_machine,
            open_link::open_link,
            daemon_log::open_daemon_log,
            daemon_log::export_daemon_log,
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
            export::save_export,
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
            terminal_stream::ask_machine_list,
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
            snapshot_restore_count,
            start_agent_session,
            prewarm_agent_session,
            drop_prewarmed_agent,
            start_terminal_session,
            start_shell_session,
            rename_terminal_session,
            queue_follow_up,
            drop_queued_follow_up,
            send_when_agent_ready,
            mark_terminal_seen,
            answer_helpers,
            check_action,
            answer_secret,
            handoff_brief,
            worktree_overlaps,
            write_project_file,
            check_output,
            pin_terminal_session,
            archive_terminal_session,
            snooze_terminal_session,
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
            allow_conversation_always,
            allowed_commands,
            forget_allowed_command,
            close_conversation_with_undo,
            undo_close_conversation,
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
            daemon_update_status,
            restart_stale_daemon,
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
        assert_eq!(models[0].efforts, None);
        assert!(super::codex_models_from("not json").is_err());
    }

    #[test]
    fn codex_models_say_the_efforts_they_take() {
        let cache = r#"{"models":[{"slug":"a","visibility":"list","supported_reasoning_levels":[{"effort":"low"},{"effort":"max"}]}]}"#;
        let models = super::codex_models_from(cache).unwrap();
        assert_eq!(models[0].efforts, Some(vec!["low".to_string(), "max".to_string()]));
    }

    #[test]
    fn codex_marks_the_model_its_settings_name_and_not_a_profiles() {
        let cache = r#"{"models":[{"slug":"a","visibility":"list"},{"slug":"b","visibility":"list"}]}"#;
        let models = || super::codex_models_from(cache).unwrap();
        let defaults = |config: &str| -> Vec<String> {
            super::mark_codex_default(models(), config).into_iter().filter(|model| model.is_default).map(|model| model.id).collect()
        };
        assert_eq!(defaults("approval_policy = \"never\"\nmodel = \"b\" # mine\n[profiles.x]\nmodel = \"a\"\n"), ["b"]);
        assert!(defaults("model_reasoning_effort = \"high\"\n[profiles.x]\nmodel = \"a\"\n").is_empty());
        assert!(defaults("model = \"not listed\"").is_empty());
    }

    fn listed(id: &str, resolved: &str, efforts: &[&str]) -> super::claude_models::Listed {
        super::claude_models::Listed {
            id: id.into(),
            label: id.to_uppercase(),
            blurb: String::new(),
            resolved: resolved.into(),
            efforts: efforts.iter().map(|level| level.to_string()).collect(),
        }
    }

    fn claude_list() -> Vec<super::claude_models::Listed> {
        vec![
            listed("default", "claude-fable-5-2", &["low"]),
            listed("opus", "claude-opus-5-5", &["low", "max"]),
            listed("fable", "claude-fable-5-2", &["low", "max"]),
            listed("claude-opus-4-8", "claude-opus-4-8", &[]),
            listed("bad name; rm", "x", &[]),
        ]
    }

    #[test]
    fn claude_lists_what_claude_code_listed_with_its_own_pick_first_and_earlier_models_as_more() {
        let rows = super::claude_rows_from_list(claude_list(), "{}");
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["default", "fable", "opus", "claude-opus-4-8"]);
        let fable = &rows[1];
        assert!(fable.is_default);
        assert_eq!(fable.efforts, Some(vec!["low".to_string(), "max".to_string()]));
        assert_eq!(rows.iter().filter(|row| row.more).map(|row| row.id.as_str()).collect::<Vec<_>>(), ["claude-opus-4-8"]);
        assert_eq!(rows[3].efforts, Some(Vec::new()));
    }

    #[test]
    fn claude_marks_the_model_its_settings_name_over_claude_codes_own_pick() {
        let by_alias = super::claude_rows_from_list(claude_list(), r#"{"model":"claude-opus-5-5"}"#);
        assert_eq!(by_alias.iter().filter(|row| row.is_default).map(|row| row.id.as_str()).collect::<Vec<_>>(), ["opus"]);
        let unlisted = super::claude_rows_from_list(claude_list(), r#"{"model":"opus[1m]"}"#);
        let named: Vec<(&str, Option<&Vec<String>>)> =
            unlisted.iter().filter(|row| row.is_default).map(|row| (row.id.as_str(), row.efforts.as_ref())).collect();
        assert_eq!(named, [("opus[1m]", None)]);
    }

    #[test]
    fn claude_offers_the_extra_models_its_own_config_lists() {
        let config = r#"{"additionalModelOptionsCache":[{"description":"Fable 5.1 · Most capable for your hardest tasks","label":"Fable","value":"claude-fable-5-1[1m]"}]}"#;
        assert_eq!(
            super::claude_models_from(config),
            vec![super::AgentModel { id: "claude-fable-5-1[1m]".into(), label: "Fable 5.1".into(), blurb: "Most capable for your hardest tasks".into(), ..Default::default() }]
        );
        assert!(super::claude_models_from("not json").is_empty());
        assert!(super::claude_models_from("{}").is_empty());
    }

    #[test]
    fn claude_lists_the_models_it_has_used_and_marks_the_one_its_settings_name() {
        let config = r#"{"additionalModelOptionsCache":[{"description":"Fable 5.1 · Hardest tasks","label":"Fable","value":"claude-fable-5-1[1m]"}]}"#;
        let settings = r#"{"model":"claude-fable-5-2","modelSettings":{"claude-sonnet-5-5":{},"claude-fable-5-2":{},"bad name; rm":{}}}"#;
        let rows = super::claude_rows(config, settings);
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["default", "claude-fable-5-1[1m]", "claude-fable-5-2", "claude-sonnet-5-5"]);
        let defaults: Vec<&str> = rows.iter().filter(|row| row.is_default).map(|row| row.id.as_str()).collect();
        assert_eq!(defaults, ["claude-fable-5-2"]);
        assert_eq!(rows[0].blurb, "Claude Code's own setting: claude-fable-5-2");
    }

    #[test]
    fn claude_adds_a_default_its_settings_name_that_no_list_has_and_says_when_there_is_none() {
        let rows = super::claude_rows("{}", r#"{"model":"fable"}"#);
        assert_eq!(rows.last().map(|row| (row.id.as_str(), row.is_default)), Some(("fable", true)));
        let none = super::claude_rows("not json", "not json");
        assert_eq!(none.len(), 1);
        assert!(none[0].blurb.starts_with("Whatever Claude Code picks"));
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
