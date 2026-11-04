mod machines;
mod project_files;

use std::path::PathBuf;
mod terminal_stream;

use tauri::Manager;

fn daemon_executable(app: &tauri::AppHandle) -> Result<PathBuf, String> {
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
async fn pane_terminal_screen(app: tauri::AppHandle, term: u32) -> Result<String, String> {
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
    term: u32,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    maitty_desktop_client::pane_terminal_resize(term, cols, rows, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn repo_query(
    app: tauri::AppHandle,
    term: u32,
    question: serde_json::Value,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::repo_query(term, question, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn new_worktree_session(
    app: tauri::AppHandle,
    term: u32,
    branch: String,
    base: Option<String>,
) -> Result<serde_json::Value, String> {
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

#[tauri::command]
async fn restore_snapshot(app: tauri::AppHandle, term: u32, commit: String) -> Result<u64, String> {
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
async fn rename_terminal_session(
    app: tauri::AppHandle,
    term: u32,
    name: String,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::rename_terminal_session(term, name, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn pin_terminal_session(
    app: tauri::AppHandle,
    term: u32,
    pinned: bool,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::pin_terminal_session(term, pinned, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn archive_terminal_session(
    app: tauri::AppHandle,
    term: u32,
    archived: bool,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::archive_terminal_session(term, archived, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn mark_terminal_seen(
    app: tauri::AppHandle,
    term: u32,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::mark_terminal_seen(term, daemon_executable(&app)?)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn close_terminal_session(
    app: tauri::AppHandle,
    term: u32,
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::close_terminal_session(term, daemon_executable(&app)?)
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
) -> Result<serde_json::Value, String> {
    maitty_desktop_client::send_conversation(
        id,
        generation,
        operation,
        text,
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
        daemon_executable(&app)?,
    )
    .await
    .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
fn codex_executable_path() -> Result<String, String> {
    maitty_desktop_client::codex_app_server_executable()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
fn claude_executable_path() -> Result<String, String> {
    maitty_desktop_client::claude_executable()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("{error:#}"))
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
        .manage(machines::Machines(maitty_machines::local_machines::local_machines()))
        .manage(terminal_stream::TerminalHistory::default())
        .manage(terminal_stream::TerminalKeys::default())
        .manage(terminal_stream::TerminalOutput::default())
        .manage(terminal_stream::LastSnapshot::default())
        .setup(|app| {
            terminal_stream::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            machines::list_machines,
            read_conversation,
            conversation_diff,
            conversation_files,
            conversation_file,
            suggest_project_folders,
            default_session_folder,
            terminal_stream::terminal_history,
            terminal_stream::watch_terminal_output,
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
            new_worktree_session,
            restore_snapshot,
            start_terminal_session,
            start_shell_session,
            rename_terminal_session,
            mark_terminal_seen,
            pin_terminal_session,
            archive_terminal_session,
            close_terminal_session,
            send_conversation,
            approve_conversation,
            answer_conversation_input,
            cancel_conversation,
            rename_conversation,
            archive_conversation,
            resume_codex_conversation,
            create_conversation,
            end_conversation,
            codex_executable_path,
            claude_executable_path,
            daemon_restart_status,
            restart_mismatched_daemon
        ])
        .run(tauri::generate_context!())
        .expect("Maitty desktop host failed");
}

#[cfg(test)]
mod tests {
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
