use maitty_shared::conversation::{ApprovalPolicy, LaunchSettings, Provider, Sandbox, StartTarget};
mod ipc;
use anyhow::{Context, Result, bail};
use maitty_shared::protocol::{ClientMsg, Command, Query, Reply, ServerMsg};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;

const MAX_TERMINAL_INPUT_BYTES: usize = 16 * 1024;
const APP_COMMAND: &str = env!("MAITTY_APP_COMMAND");
const APP_NAME: &str = env!("MAITTY_APP_NAME");
const APP_ENV: &str = env!("MAITTY_APP_ENV");

pub async fn list_sessions(daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(ClientMsg::Query(Query::List), daemon_executable).await?;
    match reply {
        Reply::List(snapshot) => Ok(serde_json::to_value(snapshot)?),
        _ => bail!("daemon returned an unexpected session list reply"),
    }
}

/// Report whether a local daemon speaks an older protocol and can be replaced by this build.
pub async fn daemon_restart_status() -> Result<Value> {
    let daemon_protocol = match ipc::open_for_user().await {
        Err(error) => ipc::other_version(&error)
            .ok_or_else(|| error.context("checking the local daemon version"))?,
        Ok((reader, writer)) => {
            drop((reader, writer));
            bail!("the local {APP_NAME} service is already compatible with this GUI")
        }
    };
    let activity = read_activity();
    let session_count = activity
        .as_ref()
        .map_or_else(saved_agent_count, |activity| activity.sessions);
    Ok(json!({
        "daemonProtocol": daemon_protocol,
        "guiProtocol": maitty_shared::protocol::PROTOCOL_VERSION,
        "canRestart": can_restart_mismatched_daemon(daemon_protocol, activity.as_ref()),
        "sessionCount": session_count,
        "activeTurnCount": activity.as_ref().map(Activity::busy),
    }))
}

fn can_restart_mismatched_daemon(daemon_protocol: u32, activity: Option<&Activity>) -> bool {
    daemon_protocol < maitty_shared::protocol::PROTOCOL_VERSION
        && activity.is_some_and(|activity| activity.sessions == 0 && activity.busy() == 0)
}

fn validate_restart_activity(activity: Option<&Activity>) -> Result<()> {
    let Some(activity) = activity else {
        bail!("couldn't verify that the older Maitty service has no sessions; it was left running")
    };
    anyhow::ensure!(
        activity.sessions == 0 && activity.busy() == 0,
        "the older Maitty service still owns {} sessions and was left running",
        activity.sessions
    );
    Ok(())
}

#[derive(serde::Deserialize)]
struct Activity {
    pid: u32,
    sessions: usize,
    working: usize,
    waiting: usize,
}

impl Activity {
    fn busy(&self) -> usize {
        self.working + self.waiting
    }
}

fn read_activity() -> Option<Activity> {
    let pid_path = ipc::pid_file();
    let activity_path = pid_path.with_extension("activity");
    let activity: Activity = serde_json::from_slice(&std::fs::read(activity_path).ok()?).ok()?;
    let pid = std::fs::read_to_string(pid_path)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()?;
    (activity.pid == pid).then_some(activity)
}

fn saved_agent_count() -> usize {
    let label = std::env::var(format!("{APP_ENV}_SOCKET")).unwrap_or_else(|_| "default".into());
    let path = ipc::data_dir().join(format!("session-{label}.json"));
    let Ok(saved) = std::fs::read(path) else {
        return 0;
    };
    let Ok(saved) = serde_json::from_slice::<Value>(&saved) else {
        return 0;
    };
    saved
        .get("workspaces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|workspace| workspace.get("tabs").and_then(Value::as_array))
        .flatten()
        .filter_map(|tab| tab.get("panes").and_then(Value::as_object))
        .flat_map(|panes| panes.values())
        .filter(|pane| pane.get("agent").is_some_and(|agent| !agent.is_null()))
        .count()
}

fn stop_mismatched_daemon() -> Result<()> {
    let pid = std::fs::read_to_string(ipc::pid_file())
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|pid| is_app_daemon(*pid))
        .ok_or_else(|| {
            anyhow::anyhow!("couldn't identify the local {APP_NAME} ({APP_COMMAND}) daemon process")
        })?;
    let mut command = if cfg!(windows) {
        let mut command = std::process::Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/F"]);
        command
    } else {
        let mut command = std::process::Command::new("kill");
        command.arg(pid.to_string());
        command
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let status = command
        .status()
        .context("stopping the old daemon process")?;
    anyhow::ensure!(
        status.success(),
        "couldn't stop the old daemon process {pid}"
    );
    Ok(())
}

fn is_app_daemon(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(command_line) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
            return false;
        };
        let args = command_line.split(|byte| *byte == 0).collect::<Vec<_>>();
        args.first()
            .is_some_and(|program| program.ends_with(APP_COMMAND.as_bytes()))
            && args.get(1).is_some_and(|arg| *arg == b"daemon")
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        true
    }
}

/// Stop only the exact older local daemon the user confirmed replacing.
pub async fn restart_mismatched_daemon(expected_protocol: u32) -> Result<()> {
    let daemon_protocol = match ipc::open_for_user().await {
        Err(error) => ipc::other_version(&error)
            .ok_or_else(|| error.context("checking the local daemon version"))?,
        Ok((reader, writer)) => {
            drop((reader, writer));
            bail!("the local {APP_NAME} service is already compatible with this GUI")
        }
    };
    validate_restart_confirmation(daemon_protocol, expected_protocol)?;
    validate_restart_activity(read_activity().as_ref())?;
    tokio::task::spawn_blocking(stop_mismatched_daemon)
        .await
        .context(format!("waiting for the local {APP_NAME} service to stop"))??;
    for _ in 0..50 {
        if !ipc::is_running().await {
            ipc::remove_pid_file();
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("the old daemon still holds the local service socket")
}

fn validate_restart_confirmation(daemon_protocol: u32, expected_protocol: u32) -> Result<()> {
    anyhow::ensure!(
        daemon_protocol == expected_protocol,
        "the local {APP_NAME} service changed since the restart prompt opened"
    );
    anyhow::ensure!(
        daemon_protocol < maitty_shared::protocol::PROTOCOL_VERSION,
        "the local {APP_NAME} service is newer than this GUI; update the GUI before reconnecting"
    );
    Ok(())
}

pub async fn read_conversation(
    id: u64,
    cursor: Option<Value>,
    limit: usize,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let cursor = cursor
        .map(serde_json::from_value)
        .transpose()
        .context("invalid conversation history cursor")?;
    let reply = request(
        ClientMsg::Query(Query::ConversationRead {
            id: maitty_shared::conversation::ConversationId(id),
            cursor,
            limit,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationRead(history) => Ok(serde_json::to_value(history)?),
        _ => bail!("daemon returned an unexpected conversation history reply"),
    }
}

pub async fn conversation_diff(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Query(Query::ConversationDiff {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationDiff(diff) => Ok(serde_json::to_value(diff)?),
        _ => bail!("daemon returned an unexpected conversation diff reply"),
    }
}

pub async fn conversation_files(
    id: u64,
    generation: u64,
    path: Option<String>,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Query(Query::ConversationFiles {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            path,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationFiles(entries) => Ok(serde_json::to_value(entries)?),
        _ => bail!("daemon returned an unexpected project file listing"),
    }
}

pub async fn conversation_file(
    id: u64,
    generation: u64,
    path: String,
    daemon_executable: PathBuf,
) -> Result<String> {
    let reply = request(
        ClientMsg::Query(Query::ConversationFileRead {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            path,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationFile(content) => Ok(content),
        _ => bail!("daemon returned an unexpected project file"),
    }
}

pub async fn terminal_screen(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<String> {
    let reply = request(
        ClientMsg::Query(Query::ConversationTerminalScreen {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Text(screen) => Ok(screen),
        _ => bail!("daemon returned an unexpected terminal screen reply"),
    }
}

pub async fn terminal_replay(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Vec<u8>> {
    let reply = request(
        ClientMsg::Query(Query::ConversationTerminalReplay {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::TerminalReplay(replay) => Ok(replay),
        _ => bail!("daemon returned an unexpected terminal replay reply"),
    }
}

pub async fn send_terminal_input(
    id: u64,
    generation: u64,
    data: Vec<u8>,
    daemon_executable: PathBuf,
) -> Result<String> {
    anyhow::ensure!(
        !data.is_empty() && data.len() <= MAX_TERMINAL_INPUT_BYTES,
        "Terminal input must contain between 1 and {MAX_TERMINAL_INPUT_BYTES} bytes"
    );
    let (mut reader, mut writer) = ipc::open_or_spawn_for_user(&daemon_executable)
        .await
        .context("connecting to the local daemon")?;
    ipc::send_message(
        &mut writer,
        &ClientMsg::Command(Command::ConversationTerminalInput {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            data,
        }),
    )
    .await?;
    match receive_reply(&mut reader).await? {
        Reply::Ok => {}
        _ => bail!("daemon returned an unexpected terminal input reply"),
    }
    ipc::send_message(
        &mut writer,
        &ClientMsg::Query(Query::ConversationTerminalScreen {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
    )
    .await?;
    receive_reply(&mut reader)
        .await
        .and_then(|reply| match reply {
            Reply::Text(screen) => Ok(screen),
            _ => bail!("daemon returned an unexpected terminal screen reply"),
        })
}

pub async fn resize_terminal(
    id: u64,
    generation: u64,
    cols: u16,
    rows: u16,
    daemon_executable: PathBuf,
) -> Result<()> {
    let reply = request(
        ClientMsg::Command(Command::ConversationTerminalResize {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            cols,
            rows,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(()),
        _ => bail!("daemon returned an unexpected terminal resize reply"),
    }
}

pub async fn send_conversation(
    id: u64,
    generation: u64,
    operation: u64,
    text: String,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationSend {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            operation,
            text,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationSent(receipt) => Ok(serde_json::to_value(receipt)?),
        _ => bail!("daemon returned an unexpected conversation send reply"),
    }
}

pub async fn approve_conversation(
    id: u64,
    generation: u64,
    request_id: maitty_shared::conversation::ProviderRequestId,
    decision: maitty_shared::conversation::ApprovalDecision,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationApprove {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            request_id,
            decision,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected conversation approval reply"),
    }
}

pub async fn answer_user_input(
    id: u64,
    generation: u64,
    request_id: maitty_shared::conversation::ProviderRequestId,
    answers: std::collections::BTreeMap<String, Vec<String>>,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationAnswerInput {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            request_id,
            answers,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected conversation input reply"),
    }
}

pub async fn cancel_conversation(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationCancel {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected turn cancellation reply"),
    }
}

pub async fn handoff_conversation_to_native(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationHandoffToNative {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected native conversation handoff reply"),
    }
}

pub async fn handoff_conversation_to_chat(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationHandoffToChat {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationStarted { id, generation } => {
            Ok(json!({"id": id.0, "generation": generation}))
        }
        _ => bail!("daemon returned an unexpected chat handoff reply"),
    }
}

pub async fn resume_codex_conversation(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationStart {
            target: StartTarget::Conversation {
                id: maitty_shared::conversation::ConversationId(id),
                generation,
            },
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationStarted { id, generation } => {
            Ok(json!({"id": id.0, "generation": generation}))
        }
        _ => bail!("daemon returned an unexpected conversation resume reply"),
    }
}

/// What a new conversation may do. The provider enforces it; Maitty only picks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permissions {
    ReadOnly,
    WorkspaceWrite,
    /// No sandbox and no approval prompts. Chosen only on request.
    FullAccess,
}

impl Permissions {
    pub fn parse(text: &str) -> Result<Self> {
        match text {
            "read-only" => Ok(Self::ReadOnly),
            "workspace-write" => Ok(Self::WorkspaceWrite),
            "full-access" => Ok(Self::FullAccess),
            other => bail!("unknown permissions {other:?}"),
        }
    }

    fn launch_policy(self) -> (Sandbox, ApprovalPolicy) {
        match self {
            Self::ReadOnly => (Sandbox::ReadOnly, ApprovalPolicy::OnRequest),
            Self::WorkspaceWrite => (Sandbox::WorkspaceWrite, ApprovalPolicy::OnRequest),
            Self::FullAccess => (Sandbox::FullAccess, ApprovalPolicy::Never),
        }
    }
}

pub async fn create_conversation(
    cwd: PathBuf,
    provider: Provider,
    permissions: Permissions,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let cwd = cwd
        .canonicalize()
        .context("resolving the selected project folder")?;
    if !cwd.is_dir() {
        bail!("the selected project folder is not a directory");
    }
    let (executable, config_root) = match provider {
        Provider::Codex => (codex_app_server_executable()?, codex_config_dir()),
        Provider::Claude => (claude_executable()?, claude_config_dir()?),
    };
    let (sandbox, approval_policy) = permissions.launch_policy();
    let settings = LaunchSettings {
        provider,
        executable,
        config_root,
        cwd,
        model: None,
        model_provider: None,
        approval_policy,
        sandbox,
    };
    let reply = request(
        ClientMsg::Command(Command::ConversationStart {
            target: StartTarget::Create(settings),
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationStarted { id, generation } => {
            Ok(json!({"id": id, "generation": generation}))
        }
        _ => bail!("daemon returned an unexpected conversation start reply"),
    }
}

pub fn claude_executable() -> Result<PathBuf> {
    executable_on_path("claude")
}

fn claude_config_dir() -> Result<PathBuf> {
    if let Some(config_root) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return Ok(PathBuf::from(config_root));
    }
    let home = directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .context("resolving the user home directory")?;
    Ok(home.join(".claude"))
}

pub fn codex_app_server_executable() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        windows_codex_app_server_executable()
    }
    #[cfg(not(windows))]
    {
        executable_on_path("codex")
    }
}

#[cfg(windows)]
fn windows_codex_app_server_executable() -> Result<PathBuf> {
    match executable_on_path("codex") {
        Ok(executable) => Ok(executable),
        Err(error) => {
            if let Some(executable) = windows_npm_codex_binary(windows_npm_search_roots(
                std::env::var_os("PATH")
                    .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
                    .unwrap_or_default(),
                std::env::var_os("NVM_SYMLINK").map(PathBuf::from),
                std::env::var_os("APPDATA").map(PathBuf::from),
                std::env::var_os("ProgramFiles").map(PathBuf::from),
            )) {
                return Ok(executable);
            }
            Err(error
                .context("Codex executable not found on PATH or in common Windows npm locations"))
        }
    }
}

fn codex_config_dir() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            directories::BaseDirs::new()
                .map(|dirs| dirs.home_dir().join(".codex"))
                .unwrap_or_default()
        })
}

fn executable_on_path(tool: &str) -> Result<PathBuf> {
    let path = std::env::var_os("PATH").ok_or_else(|| anyhow::anyhow!("PATH is not set"))?;
    executable_from_path(tool, &path)
}

fn executable_from_path(tool: &str, path: &std::ffi::OsStr) -> Result<PathBuf> {
    let extensions: &[&str] = if cfg!(windows) { &["exe"] } else { &[""] };
    for directory in std::env::split_paths(path) {
        for extension in extensions {
            let name = if extension.is_empty() {
                tool.to_string()
            } else {
                format!("{tool}.{extension}")
            };
            let candidate = directory.join(name);
            if !candidate.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                let file_name = candidate
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Executable path has no file name"))?;
                let parent = candidate
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or(std::path::Path::new("."));
                return Ok(std::fs::canonicalize(parent)?.join(file_name));
            }
            #[cfg(windows)]
            return candidate.canonicalize().map_err(Into::into);
        }
    }
    bail!("{tool} is not installed as a native executable on PATH")
}

#[cfg(windows)]
fn windows_npm_search_roots(
    path_entries: impl IntoIterator<Item = PathBuf>,
    nvm_symlink: Option<PathBuf>,
    app_data: Option<PathBuf>,
    program_files: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = path_entries.into_iter().collect::<Vec<_>>();
    if let Some(nvm_symlink) = nvm_symlink {
        roots.push(nvm_symlink);
    }
    if let Some(app_data) = app_data {
        roots.push(app_data.join("npm"));
    }
    if let Some(program_files) = program_files {
        roots.push(program_files.join("nodejs"));
    }
    roots
}

#[cfg(windows)]
fn windows_npm_codex_binary(paths: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    const PLATFORM_BINARY: &str = "vendor/x86_64-pc-windows-msvc/bin/codex.exe";
    const PACKAGE_LAYOUTS: &[&str] = &[
        "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64",
        "node_modules/@openai/codex-win32-x64",
        "node_modules/@openai/codex",
    ];

    for path in paths {
        for ancestor in path.ancestors() {
            for layout in PACKAGE_LAYOUTS {
                let executable = ancestor.join(layout).join(PLATFORM_BINARY);
                if executable.is_file() {
                    return executable.canonicalize().ok();
                }
            }
        }
    }
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::{windows_npm_codex_binary, windows_npm_search_roots};
    use std::path::PathBuf;

    #[test]
    fn finds_codex_in_the_nested_windows_npm_package() {
        let root = std::env::temp_dir().join(format!(
            "{}-codex-app-server-{}-{}",
            env!("MAITTY_APP_COMMAND"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let prefix = root.join("npm");
        let executable = prefix.join(
            "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        );
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"").unwrap();

        let resolved = windows_npm_codex_binary([PathBuf::from(&prefix)]).unwrap();

        assert_eq!(resolved, executable.canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_codex_under_nvm_symlink_when_node_is_not_on_path() {
        let root = std::env::temp_dir().join(format!(
            "{}-codex-nvm-path-{}-{}",
            env!("MAITTY_APP_COMMAND"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nvm_symlink = root.join("nodejs");
        let executable = nvm_symlink.join(
            "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        );
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"").unwrap();

        let roots = windows_npm_search_roots([], Some(nvm_symlink.clone()), None, None);
        let resolved = super::windows_npm_codex_binary(roots).unwrap();

        assert_eq!(resolved, executable.canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_codex_under_the_standard_npm_root_when_node_is_not_on_path() {
        let root = std::env::temp_dir().join(format!(
            "{}-codex-npm-root-{}-{}",
            env!("MAITTY_APP_COMMAND"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app_data = root.join("roaming");
        let executable = app_data.join(
            "npm/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        );
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"").unwrap();

        let roots = windows_npm_search_roots([], None, Some(app_data), None);
        let resolved = super::windows_npm_codex_binary(roots).unwrap();

        assert_eq!(resolved, executable.canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_codex_under_the_system_node_root_when_node_is_not_on_path() {
        let root = std::env::temp_dir().join(format!(
            "{}-codex-system-node-{}-{}",
            env!("MAITTY_APP_COMMAND"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let program_files = root.join("program-files");
        let executable = program_files.join(
            "nodejs/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        );
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"").unwrap();

        let roots = windows_npm_search_roots([], None, None, Some(program_files));
        let resolved = super::windows_npm_codex_binary(roots).unwrap();

        assert_eq!(resolved, executable.canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
}

async fn request(message: ClientMsg, daemon_executable: PathBuf) -> Result<Reply> {
    let (mut reader, mut writer) = ipc::open_or_spawn_for_user(&daemon_executable)
        .await
        .context("connecting to the local daemon")?;
    ipc::send_message(&mut writer, &message).await?;
    receive_reply(&mut reader).await
}

async fn receive_reply(reader: &mut ipc::Reader) -> Result<Reply> {
    loop {
        match ipc::receive_server(reader).await? {
            Some(ServerMsg::Reply(reply)) => return Ok(reply),
            Some(ServerMsg::Error(error)) => bail!(error),
            Some(ServerMsg::Bye) => bail!("local daemon closed the connection"),
            Some(_) => continue,
            None => bail!("local daemon closed the connection"),
        }
    }
}

#[cfg(test)]
mod restart_confirmation_tests {
    use super::{
        Activity, can_restart_mismatched_daemon, validate_restart_activity,
        validate_restart_confirmation,
    };

    #[test]
    fn accepts_only_the_confirmed_older_protocol() {
        let current = maitty_shared::protocol::PROTOCOL_VERSION;
        assert!(validate_restart_confirmation(current - 1, current - 1).is_ok());
        assert!(validate_restart_confirmation(current - 2, current - 1).is_err());
        assert!(validate_restart_confirmation(current + 1, current + 1).is_err());
    }

    fn activity(sessions: usize, working: usize, waiting: usize) -> Activity {
        Activity {
            pid: 1,
            sessions,
            working,
            waiting,
        }
    }

    #[test]
    fn only_restarts_an_older_service_that_proves_it_owns_no_sessions() {
        let older = maitty_shared::protocol::PROTOCOL_VERSION - 1;
        assert!(can_restart_mismatched_daemon(
            older,
            Some(&activity(0, 0, 0))
        ));
        assert!(!can_restart_mismatched_daemon(
            older,
            Some(&activity(1, 0, 0))
        ));
        assert!(!can_restart_mismatched_daemon(
            older,
            Some(&activity(0, 1, 0))
        ));
        assert!(!can_restart_mismatched_daemon(older, None));
        assert!(!can_restart_mismatched_daemon(
            maitty_shared::protocol::PROTOCOL_VERSION + 1,
            Some(&activity(0, 0, 0))
        ));
    }

    #[test]
    fn refuses_to_restart_when_session_ownership_is_active_or_unknown() {
        assert!(validate_restart_activity(Some(&activity(0, 0, 0))).is_ok());
        assert!(validate_restart_activity(Some(&activity(1, 0, 0))).is_err());
        assert!(validate_restart_activity(Some(&activity(0, 1, 0))).is_err());
        assert!(validate_restart_activity(None).is_err());
    }
}

#[cfg(all(test, unix))]
mod executable_lookup_tests {
    use super::executable_from_path;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn preserves_a_mise_shim_name_when_resolving_codex() {
        let directory = std::env::temp_dir().join(format!(
            "{}-mise-shim-{}-{}",
            env!("MAITTY_APP_COMMAND"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let mise = directory.join("mise");
        std::fs::write(&mise, "#!/bin/sh\nprintf '%s\\n' \"$0\" \"$1\"\n").unwrap();
        std::fs::set_permissions(&mise, std::fs::Permissions::from_mode(0o755)).unwrap();
        let codex = directory.join("codex");
        symlink(&mise, &codex).unwrap();
        let path = std::env::join_paths([&directory]).unwrap();

        let executable = executable_from_path("codex", &path).unwrap();
        let output = std::process::Command::new(&executable)
            .arg("app-server")
            .output()
            .unwrap();
        let expected = format!(
            "{}\napp-server\n",
            std::path::absolute(&codex).unwrap().display()
        );

        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
