use maitty_shared::conversation::{ApprovalPolicy, LaunchSettings, Provider, Sandbox, StartTarget};
mod host_env;
mod ipc;
mod shared_daemon;
use anyhow::{Context, Result, bail};
pub use host_env::host_command;
pub use ipc::data_dir;
pub use shared_daemon::{ensure_shared_daemon, shared_daemon_path};
use maitty_shared::protocol::{ClientMsg, Command, Query, Reply, ServerMsg};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// The most one keystroke message may carry.
pub const MAX_TERMINAL_INPUT_BYTES: usize = 16 * 1024;
/// The longest message the daemon is asked to hold for a starting agent.
pub const MAX_HELD_MESSAGE_BYTES: usize = 256 * 1024;
const APP_COMMAND: &str = env!("MAITTY_APP_COMMAND");
const APP_NAME: &str = env!("MAITTY_APP_NAME");
const APP_ENV: &str = env!("MAITTY_APP_ENV");

/// What the window may ask a pane's daemon to do, whichever machine the pane is on. One place
/// makes each message, so this computer's panes and another machine's are asked the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneCommand {
    Input(Vec<u8>),
    WantReplay,
    Resize { cols: u16, rows: u16 },
    Rename(String),
    /// A message for the agent starting in the pane, held by its daemon until the agent takes input.
    SendWhenReady(String),
    /// A message for the agent working in the pane, held by its daemon until the turn is over;
    /// `first` puts it ahead of the ones already waiting.
    Queue { text: String, first: bool },
    /// Takes back the queued message at `index`, while it still reads `text`.
    DropQueued { index: u32, text: String },
    Pin(bool),
    Archive(bool),
    Seen,
    /// Restart an agent in place so it loads hooks it missed.
    Reload,
    Close,
    /// End the session after an undo window; `UndoClose` inside it brings the session back.
    CloseSession,
    UndoClose,
    /// The person's answer to a session that asked to start helpers.
    AnswerHelpers(maitty_shared::protocol::HelperChoice),
    /// Run the pane's check `name` again (None: all of them).
    RerunCheck { name: Option<String> },
    /// Accept a failed check as it is (None: the headline one), for the reason given (it may be empty).
    OverrideCheck { name: Option<String>, reason: String },
    /// Take an override back: the check reads as failed again.
    UndoOverride { name: Option<String> },
    /// The person's answer to a repo-supplied check command, as shown (`command` is hashed here, so the
    /// daemon refuses the answer if a different command waits by then).
    ApproveCheck {
        name: String,
        choice: maitty_shared::protocol::ApprovalChoice,
        command: String,
    },
}

impl PaneCommand {
    /// The message that asks `term`'s daemon.
    pub fn message(self, term: u32) -> ClientMsg {
        match self {
            PaneCommand::Input(data) => ClientMsg::Input { term, data },
            PaneCommand::WantReplay => ClientMsg::WantReplay { term },
            PaneCommand::Resize { cols, rows } => {
                ClientMsg::Command(Command::TerminalResize { term, cols, rows })
            }
            PaneCommand::Rename(name) => ClientMsg::Command(Command::RenamePane { term, name }),
            PaneCommand::SendWhenReady(text) => {
                ClientMsg::Command(Command::SendWhenReady { term, text })
            }
            PaneCommand::Queue { text, first } => {
                ClientMsg::Command(Command::QueueFollowUp { term, text, first })
            }
            PaneCommand::DropQueued { index, text } => {
                ClientMsg::Command(Command::DropQueued { term, index, text })
            }
            PaneCommand::Pin(pinned) => ClientMsg::Command(Command::PinPane { term, pinned }),
            PaneCommand::Archive(archived) => {
                ClientMsg::Command(Command::ArchivePane { term, archived })
            }
            PaneCommand::Seen => ClientMsg::Command(Command::MarkSeen { term }),
            PaneCommand::Reload => ClientMsg::Command(Command::ReloadAgent { term }),
            PaneCommand::Close => ClientMsg::Command(Command::ClosePane { term }),
            PaneCommand::CloseSession => ClientMsg::Command(Command::CloseSession { term }),
            PaneCommand::UndoClose => ClientMsg::Command(Command::UndoClose { term }),
            PaneCommand::AnswerHelpers(choice) => {
                ClientMsg::Command(Command::AnswerHelpers { term, choice })
            }
            PaneCommand::RerunCheck { name } => ClientMsg::Command(Command::RerunCheck { term, name }),
            PaneCommand::OverrideCheck { name, reason } => {
                ClientMsg::Command(Command::OverrideCheck { term, name, reason })
            }
            PaneCommand::UndoOverride { name } => {
                ClientMsg::Command(Command::UndoOverride { term, name })
            }
            PaneCommand::ApproveCheck { name, choice, command } => {
                ClientMsg::Command(Command::ApproveCheck {
                    term,
                    name,
                    choice,
                    command_sha256: maitty_shared::protocol::command_hash(&command),
                })
            }
        }
    }
}

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
        "canRestart": can_restart_mismatched_daemon(
            daemon_protocol,
            activity.as_ref(),
            session_file().is_file(),
        ),
        "sessionCount": session_count,
        "activeTurnCount": activity.as_ref().map(Activity::busy),
    }))
}

/// An older daemon may be replaced when its sessions are saved on disk (the next daemon
/// restores them) or when it provably has none. One that owns sessions it isn't saving
/// would lose them, so it is left running.
fn can_restart_mismatched_daemon(
    daemon_protocol: u32,
    activity: Option<&Activity>,
    sessions_saved: bool,
) -> bool {
    daemon_protocol < maitty_shared::protocol::PROTOCOL_VERSION
        && sessions_survive_restart(activity, sessions_saved)
}

/// Whether stopping the daemon loses nothing: its sessions are saved on disk, or it has none.
fn sessions_survive_restart(activity: Option<&Activity>, sessions_saved: bool) -> bool {
    sessions_saved || activity.is_some_and(|activity| activity.sessions == 0)
}

/// Whether the local daemon runs an older release than this app though the two still talk to each
/// other. Such a daemon is never replaced unasked (it may hold running sessions), so the window
/// says so and offers the restart.
pub fn daemon_update_status(app_version: &str) -> Value {
    update_status(
        read_activity().as_ref(),
        app_version,
        session_file().is_file(),
    )
}

fn update_status(activity: Option<&Activity>, app_version: &str, sessions_saved: bool) -> Value {
    let behind = activity.is_some_and(|a| shared_daemon::is_older(&a.version, app_version));
    json!({
        "daemonVersion": activity.map(|a| a.version.as_str()).filter(|_| behind),
        "behind": behind,
        "canRestart": behind && sessions_survive_restart(activity, sessions_saved),
        "sessionCount": activity.map_or(0, |a| a.sessions),
        "activeTurnCount": activity.map(Activity::busy),
    })
}

fn validate_restart_activity(activity: Option<&Activity>, sessions_saved: bool) -> Result<()> {
    if sessions_survive_restart(activity, sessions_saved) {
        return Ok(());
    }
    match activity {
        Some(activity) => bail!(
            "the older Maitty service owns {} sessions but isn't saving them, so a restart would lose them; it was left running",
            activity.sessions
        ),
        None => bail!(
            "couldn't verify that the older Maitty service's sessions are saved; it was left running"
        ),
    }
}

#[derive(serde::Deserialize)]
struct Activity {
    pid: u32,
    /// Empty for a daemon from before it published its version.
    #[serde(default)]
    version: String,
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

/// Where the daemon keeps this socket's saved sessions, which the next daemon restores.
fn session_file() -> PathBuf {
    let label = std::env::var(format!("{APP_ENV}_SOCKET")).unwrap_or_else(|_| "default".into());
    ipc::data_dir().join(format!("session-{label}.json"))
}

fn saved_agent_count() -> usize {
    let Ok(saved) = std::fs::read(session_file()) else {
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
        std::fs::read(format!("/proc/{pid}/cmdline"))
            .is_ok_and(|command_line| is_daemon_command_line(&command_line))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        true
    }
}

/// Whether a process's NUL-separated command line is this product's daemon: the terminal app's
/// program, or the copy the desktop app bundles (named with `-gui-daemon`, and with the target
/// triple when it runs from a development build), started with `daemon`.
#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
fn is_daemon_command_line(command_line: &[u8]) -> bool {
    let mut args = command_line.split(|byte| *byte == 0);
    let program = args.next().unwrap_or_default();
    is_daemon_invocation(program, args.next())
}

const EXE_SUFFIX: &[u8] = b".exe";

/// Whether a program (a path or a bare name) started with this first argument is this product's
/// daemon. One rule for stopping a daemon and for listing them.
pub fn is_daemon_invocation(program: &[u8], first_argument: Option<&[u8]>) -> bool {
    let file = program
        .rsplit(|byte| matches!(byte, b'/' | b'\\'))
        .next()
        .unwrap_or_default();
    // Windows names the program with its extension.
    let name = match file.len().checked_sub(EXE_SUFFIX.len()) {
        Some(stem) if file[stem..].eq_ignore_ascii_case(EXE_SUFFIX) => &file[..stem],
        _ => file,
    };
    let bundled = format!("{APP_COMMAND}-gui-daemon");
    (name == APP_COMMAND.as_bytes() || name.starts_with(bundled.as_bytes()))
        && first_argument == Some(b"daemon".as_slice())
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
    stop_local_daemon().await
}

/// Stop the exact older release the user confirmed replacing. It must still be the version the
/// prompt named and still older than this app, so a daemon another window already replaced is
/// never stopped by a stale prompt.
pub async fn restart_stale_daemon(app_version: &str, expected_version: &str) -> Result<()> {
    let activity = read_activity();
    let running = activity.as_ref().map_or("", |a| a.version.as_str());
    anyhow::ensure!(
        running == expected_version && shared_daemon::is_older(running, app_version),
        "the local {APP_NAME} service changed since the restart prompt opened"
    );
    stop_local_daemon().await
}

/// Stop the local daemon, after checking that its sessions are saved or that it has none, and
/// wait until it lets go of the socket.
async fn stop_local_daemon() -> Result<()> {
    validate_restart_activity(read_activity().as_ref(), session_file().is_file())?;
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
    daemon: impl Into<Daemon>,
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
        daemon,
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

/// What the checkout has changed and not committed, as the handoff brief lists it. Git is run as a program
/// with fixed arguments, never through a shell. The branch's committed changes are not listed here, and the
/// brief says so.
fn uncommitted_changes(checkout: &std::path::Path) -> std::result::Result<maitty_shared::handoff::Changed, String> {
    let status = run_git(checkout, &["status", "--porcelain", "-z", "--untracked-files=all"])?;
    let mut files = maitty_shared::handoff::uncommitted(&status);
    files.sort();
    files.dedup();
    Ok(maitty_shared::handoff::Changed { files, committed_unknown: Some("only uncommitted files are listed here".into()) })
}

/// Runs git with fixed arguments in `dir`; its output, or the first line it complained with.
fn run_git(dir: &std::path::Path, args: &[&str]) -> std::result::Result<String, String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("git failed").trim().to_string())
    }
}

/// The files that more than one of these checkouts (name, folder) changed since their branches parted. Git runs
/// off the caller's thread; a checkout git cannot read is left out.
pub async fn checkout_overlaps(checkouts: Vec<(String, PathBuf)>) -> Result<Vec<maitty_shared::overlap::Overlap>> {
    tokio::task::spawn_blocking(move || maitty_shared::overlap::between(&checkouts, &run_git))
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))
}

/// The text a new agent starts with when the work of the agent in pane `term` is handed to it: the task, the
/// branch, the uncommitted files and the end of the screen. This computer's panes only.
pub async fn handoff_brief(
    term: u32,
    agent: String,
    task: String,
    last_prompt: String,
    branch: Option<String>,
    checkout: PathBuf,
    daemon_executable: PathBuf,
) -> Result<String> {
    let screen = pane_terminal_screen(term, daemon_executable).await?;
    let folder = checkout.clone();
    let changed = tokio::task::spawn_blocking(move || uncommitted_changes(&folder))
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(maitty_shared::handoff::brief(
        &maitty_shared::handoff::Facts { pane: term, agent, task, last_prompt, branch, checkout, changed, screen },
        env!("MAITTY_APP_COMMAND"),
    ))
}

/// The visible screen of a terminal session, by its pane number.
pub async fn pane_terminal_screen(term: u32, daemon_executable: PathBuf) -> Result<String> {
    let reply = request(
        ClientMsg::Query(Query::TerminalScreen { term }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Text(screen) => Ok(screen),
        _ => bail!("daemon returned an unexpected terminal screen reply"),
    }
}

pub async fn pane_terminal_replay(term: u32, daemon_executable: PathBuf) -> Result<Vec<u8>> {
    let reply = request(
        ClientMsg::Query(Query::TerminalReplay { term }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::TerminalReplay(replay) => Ok(replay),
        _ => bail!("daemon returned an unexpected terminal replay reply"),
    }
}

pub async fn pane_terminal_resize(
    term: u32,
    cols: u16,
    rows: u16,
    daemon_executable: PathBuf,
) -> Result<()> {
    let reply = request(
        PaneCommand::Resize { cols, rows }.message(term),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(()),
        _ => bail!("daemon returned an unexpected terminal resize reply"),
    }
}

/// A live stream of the terminal panes this app owns: each pane's replay, then its output as
/// it happens. Pushed by the daemon, so the window never polls (ADR 0004).
pub struct TerminalStream {
    reader: ipc::Reader,
}

impl TerminalStream {
    /// The next message, or `None` when the daemon closed the connection.
    pub async fn next(&mut self) -> Result<Option<maitty_shared::protocol::ServerMsg>> {
        ipc::receive_server(&mut self.reader).await
    }
}

/// The other half of the stream's connection: what the user types goes back over it.
pub struct TerminalInput {
    writer: ipc::Writer,
}

impl TerminalInput {
    pub async fn send(&mut self, term: u32, data: Vec<u8>) -> Result<()> {
        anyhow::ensure!(
            !data.is_empty() && data.len() <= MAX_TERMINAL_INPUT_BYTES,
            "Terminal input must contain between 1 and {MAX_TERMINAL_INPUT_BYTES} bytes"
        );
        ipc::send_message(&mut self.writer, &PaneCommand::Input(data).message(term)).await
    }

    /// Ask for the pane's replay on this stream, in order with its output.
    pub async fn want_replay(&mut self, term: u32) -> Result<()> {
        ipc::send_message(&mut self.writer, &PaneCommand::WantReplay.message(term)).await
    }
}

/// This computer's daemon as a live line for the panes this app owns (replays, then output),
/// started when none answers. The caller reads and writes it.
pub async fn open_terminal_line(daemon_executable: PathBuf) -> Result<(ipc::Reader, ipc::Writer)> {
    ipc::open_stream_for_user(&daemon_executable)
        .await
        .context("connecting to the local daemon")
}

pub async fn open_terminal_stream(
    daemon_executable: PathBuf,
) -> Result<(TerminalStream, TerminalInput)> {
    let (reader, writer) = ipc::open_stream_for_user(&daemon_executable)
        .await
        .context("connecting to the local daemon")?;
    Ok((TerminalStream { reader }, TerminalInput { writer }))
}

/// Start a plain shell in a folder. The user types what they want to run in it.
pub async fn start_shell_session(cwd: PathBuf, daemon_executable: PathBuf) -> Result<Value> {
    let cwd = cwd.canonicalize().context("resolving the start folder")?;
    if !cwd.is_dir() {
        bail!("the start folder is not a directory");
    }
    let reply = request(
        ClientMsg::Command(Command::NewWorkspace {
            cwd: Some(cwd),
            name: None,
            cmd: None,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected shell session reply"),
    }
}

/// Run a command line in a new terminal session on this computer, named `name`. For a login that
/// needs a person (ssh asking for a password or a host key): the person answers in the pane.
pub async fn start_command_session(
    command: String,
    name: String,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::NewWorkspace {
            cwd: None,
            name: Some(name),
            cmd: Some(command),
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected reply to a command session"),
    }
}

/// Start an agent in a terminal session (a pane the daemon owns) in a project folder.
pub async fn start_terminal_session(
    cwd: PathBuf,
    provider: Provider,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let cwd = cwd
        .canonicalize()
        .context("resolving the selected project folder")?;
    if !cwd.is_dir() {
        bail!("the selected project folder is not a directory");
    }
    let program = provider.agent_name();
    let reply = request(
        ClientMsg::Command(Command::NewWorkspace {
            cwd: Some(cwd),
            name: None,
            cmd: Some(program.into()),
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected terminal session reply"),
    }
}

/// The agents a window may start by name, and the program each one runs.
const STARTABLE_AGENTS: [&str; 3] = ["claude", "codex", "gemini"];

/// The command line for an agent and its launch flags. Only a known agent runs, and a flag is one plain word
/// (letters, digits and `. _ : = @ / + -`), so nothing the window sends can add a second command.
pub fn agent_command(agent: &str, flags: &[String]) -> Result<String> {
    if !STARTABLE_AGENTS.contains(&agent) {
        bail!("{agent} cannot be started from here");
    }
    let plain = |word: &str| {
        !word.is_empty()
            && word
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._:=@/+-".contains(c))
    };
    if let Some(bad) = flags.iter().find(|word| !plain(word)) {
        bail!("{bad:?} is not a valid launch flag");
    }
    Ok(std::iter::once(agent).chain(flags.iter().map(String::as_str)).collect::<Vec<_>>().join(" "))
}

/// Start an agent as the first command of a new terminal session in a folder. The pane is the shell with the
/// agent already running in it, as if it had been typed, without waiting for the shell to be ready for typing.
pub async fn start_agent_session(
    cwd: PathBuf,
    agent: String,
    flags: Vec<String>,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let command = agent_command(&agent, &flags)?;
    let cwd = cwd.canonicalize().context("resolving the start folder")?;
    if !cwd.is_dir() {
        bail!("the start folder is not a directory");
    }
    let reply = request(
        ClientMsg::Command(Command::NewWorkspace {
            cwd: Some(cwd),
            name: None,
            cmd: Some(command),
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected agent session reply"),
    }
}

/// The command that keeps an agent warm in a folder: the same line `start_agent_session` sends, so the session started
/// next with the same agent, flags and folder is the one already running.
pub fn prewarm_message(cwd: PathBuf, agent: &str, flags: &[String]) -> Result<ClientMsg> {
    let cmd = agent_command(agent, flags)?;
    let cwd = cwd.canonicalize().context("resolving the start folder")?;
    if !cwd.is_dir() {
        bail!("the start folder is not a directory");
    }
    Ok(ClientMsg::Command(Command::Prewarm { cwd, cmd }))
}

/// Start an agent ahead of its first message, hidden until its session is started. False when the daemon running is
/// from before this and was not asked.
pub async fn prewarm_agent_session(cwd: PathBuf, agent: String, flags: Vec<String>) -> Result<Value> {
    let asked = ask_if_known(prewarm_message(cwd, &agent, &flags)?).await?;
    Ok(Value::Bool(asked))
}

/// Stop the agent kept warm, if no session took it.
pub async fn drop_prewarmed_agent() -> Result<Value> {
    ask_if_known(ClientMsg::Command(Command::DropPrewarm)).await?;
    Ok(Value::Null)
}

/// Whether a daemon at `daemon_version` reads `message`. An older one passes over what it cannot read and never answers.
fn daemon_reads(message: &ClientMsg, daemon_version: u32) -> bool {
    daemon_version >= message.introduced_in()
}

/// Ask the running daemon for something it may be too old to know: true when it was asked and said yes.
async fn ask_if_known(message: ClientMsg) -> Result<bool> {
    answered_within(REQUEST_TIMEOUT, async {
        let (mut connection, version) = ipc::open_for_user_knowing_version()
            .await
            .context("connecting to the local daemon")?;
        if !daemon_reads(&message, version) {
            return Ok(false);
        }
        match ask(&mut connection, &message).await? {
            Reply::Ok => Ok(true),
            _ => bail!("daemon returned an unexpected reply"),
        }
    })
    .await
}

/// What may be held for a starting agent: some words, and not more than the daemon is asked to keep.
pub fn held_message(text: String) -> Result<PaneCommand> {
    if text.trim().is_empty() {
        bail!("there is no message to hold");
    }
    if text.len() > MAX_HELD_MESSAGE_BYTES {
        bail!("the message is too long to hold for the agent");
    }
    Ok(PaneCommand::SendWhenReady(text))
}

pub async fn send_when_ready(term: u32, text: String, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(held_message(text)?.message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected reply to a held message"),
    }
}

/// What may be queued for a working agent: some words, and not more than the daemon is asked to keep.
pub fn queued_message(text: String, first: bool) -> Result<PaneCommand> {
    if text.trim().is_empty() {
        bail!("there is no message to queue");
    }
    if text.len() > MAX_HELD_MESSAGE_BYTES {
        bail!("the message is too long to queue for the agent");
    }
    Ok(PaneCommand::Queue { text, first })
}

pub async fn queue_follow_up(
    term: u32,
    text: String,
    first: bool,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(queued_message(text, first)?.message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected reply to a queued message"),
    }
}

/// Takes one queued message back. A daemon from before this could only drop them all, and would
/// never answer, so it is not asked.
pub async fn drop_queued(term: u32, index: u32, text: String) -> Result<Value> {
    if ask_if_known(PaneCommand::DropQueued { index, text }.message(term)).await? {
        Ok(Value::Null)
    } else {
        bail!("the Maitty service is from an older release: restart it to take back one queued message")
    }
}

pub async fn rename_terminal_session(
    term: u32,
    name: String,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(PaneCommand::Rename(name).message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected rename reply"),
    }
}

/// A read-only question about the repository a pane is in. `question` is a `RepoRequest` as
/// JSON; the daemon scopes it to the pane's own checkout.
pub async fn repo_query(term: u32, question: Value, daemon_executable: PathBuf) -> Result<Value> {
    let question: maitty_shared::repo::RepoRequest =
        serde_json::from_value(question).context("invalid repository request")?;
    let reply = request(
        ClientMsg::Query(Query::Repo {
            term,
            tag: 0,
            request: question,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Repo { reply, .. } => Ok(serde_json::to_value(reply)?),
        _ => bail!("daemon returned an unexpected repository reply"),
    }
}

/// `git worktree add` for `branch` next to the repository `term` is in (new from `base`, or the
/// repo's HEAD); the daemon opens it as a new session. `branch` names an existing branch to
/// check it out instead.
pub async fn new_worktree_session(
    term: u32,
    branch: String,
    base: Option<String>,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let snapshot = match request(ClientMsg::Query(Query::List), daemon_executable.clone()).await? {
        Reply::List(snapshot) => snapshot,
        _ => bail!("daemon returned an unexpected session list reply"),
    };
    let ws = snapshot
        .workspaces
        .iter()
        .find(|workspace| {
            workspace
                .tabs
                .iter()
                .any(|tab| tab.layout.leaves().contains(&term))
        })
        .map(|workspace| workspace.id)
        .with_context(|| format!("pane {term} is not open"))?;
    let reply = request(
        ClientMsg::Command(Command::NewWorktree {
            ws,
            branch,
            base,
            cmd: None,
            split: None,
            from: None,
            agent: None,
            prompt: None,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected worktree reply"),
    }
}

/// Remove worktrees (a folder and whether to go on with uncommitted changes in it, which are lost).
/// The daemon closes what runs in a worktree before it goes and answers at once; one something
/// still runs in is left alone.
pub async fn tidy_worktrees(items: Vec<(PathBuf, bool)>, daemon_executable: PathBuf) -> Result<Value> {
    match request(ClientMsg::Command(Command::TidyWorktrees { items }), daemon_executable).await? {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected worktree reply"),
    }
}

/// How long a worktree for a branch may take, connecting included. The repo's setup hook runs
/// before the daemon answers (it may take ten minutes), so the usual request limit is too short.
const BRANCH_WORKTREE_TIMEOUT: Duration = Duration::from_secs(11 * 60);

/// A worktree of `project` on `branch`, made by the daemon the way every other one is: trusted
/// like its repo, the repo's setup run, tracked for tidy-up. The answer is its folder, the place
/// to start an agent. A daemon built before the command is told so in words, since it would never
/// answer.
pub async fn branch_worktree(project: PathBuf, branch: String) -> Result<Value> {
    let message = ClientMsg::Command(Command::BranchWorktree { project, branch });
    answered_within(BRANCH_WORKTREE_TIMEOUT, async {
        let (mut connection, version) = ipc::open_for_user_knowing_version()
            .await
            .context("connecting to the local daemon")?;
        if !daemon_reads(&message, version) {
            bail!(
                "the Maitty service running is older than this app and cannot make a worktree yet: restart it (Settings › Daemon), then try again"
            );
        }
        match ask(&mut connection, &message).await? {
            Reply::Text(folder) => Ok(Value::String(folder)),
            _ => bail!("daemon returned an unexpected worktree reply"),
        }
    })
    .await
}

/// Where `query` appears in what the app's terminal sessions printed recently.
pub async fn search_panes(query: String, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(
        ClientMsg::Query(Query::PaneSearch { query }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Search(hits) => Ok(serde_json::to_value(hits)?),
        _ => bail!("daemon returned an unexpected search reply"),
    }
}

/// What agents reported using over `period`: totals, per session and per day.
pub async fn usage_report(
    period: maitty_shared::usage_report::UsagePeriod,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Query(Query::UsageReport { period }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::UsageReport(report) => Ok(serde_json::to_value(report)?),
        _ => bail!("daemon returned an unexpected usage reply"),
    }
}

/// Put a pane's checkout back as it was in one of the daemon's snapshots. Returns how many
/// files changed.
pub async fn restore_snapshot(
    term: u32,
    commit: String,
    daemon_executable: PathBuf,
) -> Result<u64> {
    let reply = request(
        ClientMsg::Command(Command::RestoreSnapshot {
            term,
            tag: 0,
            commit,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Repo {
            reply: maitty_shared::repo::RepoReply::Restored(changed),
            ..
        } => Ok(changed as u64),
        _ => bail!("daemon returned an unexpected restore reply"),
    }
}

/// Replace the text of one existing file of the checkout `term` is in (the daemon checks the path and refuses
/// anything but a plain file of the checkout).
pub async fn write_project_file(
    term: u32,
    path: String,
    contents: String,
) -> Result<Value> {
    let message = ClientMsg::Command(Command::WriteFile {
        term,
        tag: 0,
        path,
        contents,
    });
    answered_within(Duration::from_secs(30), async {
        let (mut connection, version) = ipc::open_for_user_knowing_version()
            .await
            .context("connecting to the local daemon")?;
        if !daemon_reads(&message, version) {
            bail!(
                "the Maitty service running is older than this app and cannot save a file yet: restart it (Settings › Daemon), then try again"
            );
        }
        match ask(&mut connection, &message).await? {
            Reply::Repo {
                reply: maitty_shared::repo::RepoReply::Written,
                ..
            } => Ok(Value::Null),
            _ => bail!("daemon returned an unexpected save reply"),
        }
    })
    .await
}

/// End a terminal session: its process stops, as when it is closed in the terminal app.
pub async fn close_terminal_session(term: u32, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(PaneCommand::Close.message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected close reply"),
    }
}

/// End a terminal session with an undo window: the daemon hides it at once and closes it after
/// the window unless `undo_close_terminal_session` comes first.
pub async fn close_session_with_undo(term: u32, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(PaneCommand::CloseSession.message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected close reply"),
    }
}

/// Bring back a session `close_session_with_undo` hid, while the undo window is open.
pub async fn undo_close_terminal_session(term: u32, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(PaneCommand::UndoClose.message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected undo reply"),
    }
}

/// Pin a pane to the top of its project, or let it go back to its place.
pub async fn pin_terminal_session(
    term: u32,
    pinned: bool,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(PaneCommand::Pin(pinned).message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected pin reply"),
    }
}

/// Put a pane away from the project lists without stopping it, or bring it back.
pub async fn archive_terminal_session(
    term: u32,
    archived: bool,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        PaneCommand::Archive(archived).message(term),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected archive reply"),
    }
}

/// Restart an agent in place, keeping its name, so it loads hooks it missed.
pub async fn reload_terminal_session(term: u32, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(PaneCommand::Reload.message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected reload reply"),
    }
}

/// The user looked at a pane: the daemon turns a finished agent's done into idle.
/// The words the window uses for each answer to a helper request.
pub fn helper_choice(word: &str) -> Result<maitty_shared::protocol::HelperChoice> {
    use maitty_shared::protocol::HelperChoice;
    match word {
        "allow" => Ok(HelperChoice::Allow),
        "always" => Ok(HelperChoice::Always),
        "no" => Ok(HelperChoice::No),
        _ => bail!("unknown answer to a helper request"),
    }
}

pub async fn answer_helpers(
    term: u32,
    choice: maitty_shared::protocol::HelperChoice,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(PaneCommand::AnswerHelpers(choice).message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected helper answer reply"),
    }
}

/// What the person chose for a repo-supplied check command: `once`, `always` or `never`.
pub fn approval_choice(word: &str) -> Result<maitty_shared::protocol::ApprovalChoice> {
    use maitty_shared::protocol::ApprovalChoice;
    match word {
        "once" => Ok(ApprovalChoice::Once),
        "always" => Ok(ApprovalChoice::Always),
        "never" => Ok(ApprovalChoice::Never),
        _ => bail!("unknown answer to a check approval"),
    }
}

/// Ask this computer's daemon to do something to a pane's checks (see `PaneCommand`).
pub async fn check_command(term: u32, command: PaneCommand, daemon_executable: PathBuf) -> Result<Value> {
    match request(command.message(term), daemon_executable).await? {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected check reply"),
    }
}

/// The tail of a pane's last output for check `name` (None: the headline one).
pub async fn check_output(term: u32, name: Option<String>, daemon_executable: PathBuf) -> Result<Value> {
    match request(ClientMsg::Query(Query::CheckOutput { term, name }), daemon_executable).await? {
        Reply::Text(output) => Ok(Value::String(output)),
        _ => bail!("daemon returned an unexpected check output reply"),
    }
}

/// The person's answer to an agent's request for a secret (`value`: the secret, or None to decline). The value
/// goes into the message as `SecretText`, which prints as `<hidden>`, and is not kept or logged here.
pub async fn answer_secret(id: u64, value: Option<String>, daemon_executable: PathBuf) -> Result<Value> {
    let message = ClientMsg::Command(Command::AnswerSecret {
        id,
        value: value.map(maitty_shared::protocol::SecretText),
    });
    match request(message, daemon_executable).await? {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected secret reply"),
    }
}

pub async fn mark_terminal_seen(term: u32, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(PaneCommand::Seen.message(term), daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected mark-seen reply"),
    }
}

pub async fn send_conversation(
    id: u64,
    generation: u64,
    operation: u64,
    text: String,
    choice: Option<maitty_shared::conversation::TurnChoice>,
    daemon: impl Into<Daemon>,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationSend {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            operation,
            text,
            choice: choice.filter(|picked| !picked.is_empty()),
        }),
        daemon,
    )
    .await?;
    match reply {
        Reply::ConversationSent(receipt) => Ok(serde_json::to_value(receipt)?),
        // The conversation was busy: the daemon holds the message and sends it when the turn ends.
        Reply::ConversationQueued(queued) => Ok(serde_json::to_value(queued)?),
        _ => bail!("daemon returned an unexpected conversation send reply"),
    }
}

pub async fn dequeue_conversation(
    id: u64,
    generation: u64,
    operation: u64,
    daemon: impl Into<Daemon>,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationDequeue {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            operation,
        }),
        daemon,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected queued message removal reply"),
    }
}

pub async fn approve_conversation(
    id: u64,
    generation: u64,
    request_id: maitty_shared::conversation::ProviderRequestId,
    decision: maitty_shared::conversation::ApprovalDecision,
    daemon: impl Into<Daemon>,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationApprove {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            request_id,
            decision,
        }),
        daemon,
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
    daemon: impl Into<Daemon>,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationAnswerInput {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            request_id,
            answers,
        }),
        daemon,
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
    daemon: impl Into<Daemon>,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationCancel {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected turn cancellation reply"),
    }
}

pub async fn rename_conversation(
    id: u64,
    title: String,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationRename {
            id: maitty_shared::conversation::ConversationId(id),
            title,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected rename reply"),
    }
}

pub async fn archive_conversation(
    id: u64,
    archived: bool,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationArchive {
            id: maitty_shared::conversation::ConversationId(id),
            archived,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected close reply"),
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
            branch: None,
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

/// Approve a command request of a chat and have the daemon remember it for the chat's project.
pub async fn allow_conversation_always(
    id: u64,
    generation: u64,
    request_id: maitty_shared::conversation::ProviderRequestId,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationAllowAlways {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
            request_id,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected remembered approval reply"),
    }
}

/// Close a chat with an undo window: the daemon hides it at once and ends it when the window passes.
pub async fn close_conversation_with_undo(id: u64, generation: u64, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationClose {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected close reply"),
    }
}

/// Bring back a chat closed with `close_conversation_with_undo` while its window is open.
pub async fn undo_close_conversation(id: u64, daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationUndoClose {
            id: maitty_shared::conversation::ConversationId(id),
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected undo reply"),
    }
}

/// The commands approved "always in this project" for chats.
pub async fn allowed_commands(daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(ClientMsg::Query(Query::AllowedCommands), daemon_executable).await?;
    match reply {
        Reply::AllowedCommands(commands) => Ok(serde_json::to_value(commands)?),
        _ => bail!("daemon returned an unexpected approved commands reply"),
    }
}

/// The conversations `agent` saved for `folder` on this computer, newest first, as the daemon's
/// `SavedConversations`. Null when the daemon running is older than the question: it would never answer.
pub async fn saved_conversations(agent: String, folder: String) -> Result<Value> {
    let message = ClientMsg::Query(Query::SavedConversations { agent, folder, tag: 0 });
    answered_within(REQUEST_TIMEOUT, async {
        let (mut connection, version) = ipc::open_for_user_knowing_version()
            .await
            .context("connecting to the local daemon")?;
        if !daemon_reads(&message, version) {
            return Ok(Value::Null);
        }
        match ask(&mut connection, &message).await? {
            Reply::SavedConversations { found, .. } => Ok(serde_json::to_value(found)?),
            _ => bail!("daemon returned an unexpected saved conversations reply"),
        }
    })
    .await
}

/// The agent sessions this computer's daemon recorded as closed, newest first, each with `folder_present`: whether its
/// folder is still there, looked at off the async workers. Null when the daemon running is older than the question.
pub async fn closed_sessions() -> Result<Value> {
    let message = ClientMsg::Query(Query::ClosedSessions);
    answered_within(REQUEST_TIMEOUT, async {
        let (mut connection, version) = ipc::open_for_user_knowing_version()
            .await
            .context("connecting to the local daemon")?;
        if !daemon_reads(&message, version) {
            return Ok(Value::Null);
        }
        match ask(&mut connection, &message).await? {
            Reply::ClosedSessions(list) => with_folder_presence(list, FOLDER_LOOK_LIMIT, |path| path.is_dir()).await,
            _ => bail!("daemon returned an unexpected closed sessions reply"),
        }
    })
    .await
}

/// Have this computer's daemon reopen a closed session it keeps, making its worktree again when the folder is gone.
/// The daemon starts the agent itself. False when the daemon running is from before this and was not asked.
pub async fn reopen_closed_session(agent: String, session: String) -> Result<Value> {
    let asked = ask_if_known(reopen_closed_message(agent, session)).await?;
    Ok(Value::Bool(asked))
}

fn reopen_closed_message(agent: String, session: String) -> ClientMsg {
    ClientMsg::Command(Command::ReopenClosedSession { agent, session })
}

/// How long the closed sessions' folders are looked for, all of them together.
const FOLDER_LOOK_LIMIT: Duration = Duration::from_secs(2);

/// Each closed session as JSON with `folder_present`. The folders are looked at side by side on blocking threads, and
/// one that has not answered by `limit` (a network share that is gone can hang for a long time) reads as missing.
async fn with_folder_presence(
    list: Vec<maitty_shared::protocol::ClosedSession>,
    limit: Duration,
    look: impl Fn(&std::path::Path) -> bool + Send + Sync + Clone + 'static,
) -> Result<Value> {
    let deadline = tokio::time::Instant::now() + limit;
    let looks: Vec<_> = list
        .iter()
        .map(|closed| {
            let (cwd, look) = (closed.cwd.clone(), look.clone());
            tokio::task::spawn_blocking(move || look(&cwd))
        })
        .collect();
    let mut rows = Vec::with_capacity(list.len());
    for (closed, folder_look) in list.into_iter().zip(looks) {
        let folder_present = match tokio::time::timeout_at(deadline, folder_look).await {
            Ok(answer) => answer.context("looking for a closed session's folder")?,
            Err(_) => false,
        };
        let mut row = serde_json::to_value(closed)?;
        row["folder_present"] = Value::Bool(folder_present);
        rows.push(row);
    }
    Ok(Value::Array(rows))
}

/// Take back a command approved "always in this project".
pub async fn forget_allowed_command(
    project: PathBuf,
    command_sha256: String,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ForgetAllowedCommand { project, command_sha256 }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected forget reply"),
    }
}

/// Stop a chat conversation's agent. Its record and history stay.
pub async fn end_conversation(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationEnd {
            id: maitty_shared::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected end conversation reply"),
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

/// The posture of a chat session: it asks before a tool use the provider does not allow by itself,
/// unless the caller asks for the provider's own skip-permissions mode (ADR 0005, ADR 0003).
fn chat_session_permissions(full_access: bool) -> Permissions {
    if full_access {
        Permissions::FullAccess
    } else {
        Permissions::WorkspaceWrite
    }
}

/// Start a chat session: a structured conversation the daemon owns (ADR 0003). With `full_access` the
/// provider runs unsandboxed and without approvals; there is no other posture to choose. With a
/// `branch` the daemon makes a worktree for it from the project at `cwd` and starts there.
pub async fn start_chat_session(
    cwd: PathBuf,
    provider: Provider,
    branch: Option<String>,
    full_access: bool,
    daemon_executable: PathBuf,
) -> Result<Value> {
    create_conversation(
        cwd,
        provider,
        chat_session_permissions(full_access),
        branch,
        daemon_executable,
    )
    .await
}

pub async fn create_conversation(
    cwd: PathBuf,
    provider: Provider,
    permissions: Permissions,
    branch: Option<String>,
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
        // Gemini runs in a terminal and its chat is read from the transcript; it has no structured runtime.
        Provider::Gemini => bail!("Gemini has no structured chat; start it in a terminal"),
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
            branch,
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

/// How long one request may take, connecting included. The slowest answers (a diff of a large
/// tree, a new checkout) take seconds; a daemon that says nothing for this long is stuck, and the
/// window must be told so instead of waiting on it for ever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Connections kept open between questions, so each one does not connect and say hello again:
/// an open chat asks several times a second. Only a `Query` uses them. It changes nothing, so
/// when a kept connection turns out to be dead (its daemon restarted) the question is simply
/// asked again on a new one; a command asked twice could act twice.
static IDLE_CONNECTIONS: IdleConnections = std::sync::Mutex::new(Vec::new());
const MAX_IDLE_CONNECTIONS: usize = 4;
/// Another machine's lines are ssh processes, so fewer are kept for each.
const MAX_IDLE_REMOTE_CONNECTIONS: usize = 2;

/// One line to a daemon: what it reads, what it writes.
pub type Connection = (ipc::Reader, ipc::Writer);
type IdleConnections = std::sync::Mutex<Vec<Connection>>;

/// Where a request goes: this computer's daemon, started when none answers, or another machine's.
#[derive(Clone)]
pub enum Daemon {
    Local(PathBuf),
    Remote(RemoteDaemon),
}

impl From<PathBuf> for Daemon {
    fn from(daemon_executable: PathBuf) -> Daemon {
        Daemon::Local(daemon_executable)
    }
}

type Opening = Pin<Box<dyn Future<Output = Result<Connection>> + Send>>;

/// Another machine's daemon, reached over lines the caller knows how to open. The host makes the
/// lines (the ssh line, attach only, ADR 0007) and says how to vet a reply, so this crate holds no
/// ssh and trusts nothing a remote says: every reply goes through `vet` before anything reads it.
#[derive(Clone)]
pub struct RemoteDaemon {
    /// The machine's number for this run, which never names another machine.
    key: u16,
    name: String,
    open: Arc<dyn Fn() -> Opening + Send + Sync>,
    vet: Arc<dyn Fn(Reply) -> Result<Reply, String> + Send + Sync>,
}

impl RemoteDaemon {
    pub fn new(
        key: u16,
        name: String,
        open: impl Fn() -> Opening + Send + Sync + 'static,
        vet: impl Fn(Reply) -> Result<Reply, String> + Send + Sync + 'static,
    ) -> RemoteDaemon {
        RemoteDaemon {
            key,
            name,
            open: Arc::new(open),
            vet: Arc::new(vet),
        }
    }

    async fn request(&self, message: ClientMsg) -> Result<Reply> {
        let idle = remote_idle(self.key);
        let asked = ask_reusing(&idle, MAX_IDLE_REMOTE_CONNECTIONS, &message, || {
            (self.open)()
        });
        let reply = match tokio::time::timeout(REQUEST_TIMEOUT, asked).await {
            Ok(answer) => answer.with_context(|| format!("asking {}", self.name))?,
            Err(_) => bail!(
                "{} did not answer within {} seconds",
                self.name,
                REQUEST_TIMEOUT.as_secs()
            ),
        };
        (self.vet)(reply).map_err(|why| anyhow::anyhow!("{} sent an answer that was refused: {why}", self.name))
    }
}

/// The lines kept for each machine, by its number for this run. A machine's number is never
/// reused (ADR 0007), so one machine's lines cannot serve another.
static REMOTE_IDLE: std::sync::Mutex<BTreeMap<u16, Arc<IdleConnections>>> =
    std::sync::Mutex::new(BTreeMap::new());

fn remote_idle(key: u16) -> Arc<IdleConnections> {
    let mut all = REMOTE_IDLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    all.entry(key).or_default().clone()
}

/// Close the lines kept for a machine that was taken out of the list.
pub fn forget_remote(key: u16) {
    REMOTE_IDLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&key);
}

async fn ask(connection: &mut Connection, message: &ClientMsg) -> Result<Reply> {
    ipc::send_message(&mut connection.1, message).await?;
    receive_reply(&mut connection.0).await
}

/// Ask `message`: on a kept connection when it is a question, else on one from `open`. At most
/// `keep_at_most` are kept.
async fn ask_reusing<Open, Opening>(
    idle: &IdleConnections,
    keep_at_most: usize,
    message: &ClientMsg,
    open: Open,
) -> Result<Reply>
where
    Open: FnOnce() -> Opening,
    Opening: std::future::Future<Output = Result<Connection>>,
{
    let reusable = matches!(message, ClientMsg::Query(_));
    let keep = |connection: Connection| {
        if let Ok(mut idle) = idle.lock()
            && idle.len() < keep_at_most
        {
            idle.push(connection);
        }
    };
    let kept = if reusable {
        idle.lock().ok().and_then(|mut idle| idle.pop())
    } else {
        None
    };
    if let Some(mut kept) = kept
        && let Ok(reply) = ask(&mut kept, message).await
    {
        keep(kept);
        return Ok(reply);
    }
    let mut fresh = open().await?;
    let reply = ask(&mut fresh, message).await?;
    if reusable {
        keep(fresh);
    }
    Ok(reply)
}

async fn request(message: ClientMsg, daemon: impl Into<Daemon>) -> Result<Reply> {
    match daemon.into() {
        Daemon::Local(daemon_executable) => {
            answered_within(
                REQUEST_TIMEOUT,
                ask_reusing(&IDLE_CONNECTIONS, MAX_IDLE_CONNECTIONS, &message, || async {
                    ipc::open_or_spawn_for_user(&daemon_executable)
                        .await
                        .context("connecting to the local daemon")
                }),
            )
            .await
        }
        Daemon::Remote(remote) => remote.request(message).await,
    }
}

#[cfg(test)]
mod kept_connection_tests {
    use super::{Connection, Daemon, IdleConnections, MAX_IDLE_CONNECTIONS, RemoteDaemon, ask_reusing, forget_remote, request};
    use maitty_shared::protocol::{ClientMsg, Command, Query, Reply, ServerMsg};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A connection to a daemon that answers everything with `Ok`, until `answers` have gone out.
    fn daemon_answering(answers: usize) -> Connection {
        let (ours, theirs) = tokio::io::duplex(64 * 1024);
        let (their_read, their_write) = tokio::io::split(theirs);
        let (mut reader, mut writer) = maitty_ipc_transport::framed_parts(their_read, their_write);
        tokio::spawn(async move {
            for _ in 0..answers {
                let Ok(Some(_)) = maitty_ipc_transport::receive::<ClientMsg>(&mut reader).await
                else {
                    return;
                };
                if maitty_ipc_transport::send(&mut writer, &ServerMsg::Reply(Reply::Ok))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });
        let (our_read, our_write) = tokio::io::split(ours);
        maitty_ipc_transport::framed_parts(our_read, our_write)
    }

    fn question() -> ClientMsg {
        ClientMsg::Query(Query::Settings)
    }

    #[tokio::test]
    async fn a_second_question_goes_out_on_the_connection_the_first_one_opened() {
        let idle = IdleConnections::default();
        let opened = Arc::new(AtomicUsize::new(0));
        for _ in 0..3 {
            let opened = opened.clone();
            let reply = ask_reusing(&idle, MAX_IDLE_CONNECTIONS, &question(), || async move {
                opened.fetch_add(1, Ordering::SeqCst);
                Ok(daemon_answering(usize::MAX))
            })
            .await;
            assert!(matches!(reply, Ok(Reply::Ok)));
        }
        assert_eq!(opened.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_question_on_a_kept_connection_whose_daemon_went_away_is_asked_again_on_a_new_one() {
        let idle = IdleConnections::default();
        let opened = Arc::new(AtomicUsize::new(0));
        for _ in 0..2 {
            let opened = opened.clone();
            let reply = ask_reusing(&idle, MAX_IDLE_CONNECTIONS, &question(), || async move {
                opened.fetch_add(1, Ordering::SeqCst);
                // Each daemon answers once and is gone.
                Ok(daemon_answering(1))
            })
            .await;
            assert!(matches!(reply, Ok(Reply::Ok)));
        }
        assert_eq!(opened.load(Ordering::SeqCst), 2);
    }

    /// A remote machine whose lines each answer everything with `Ok`; `opened` counts the lines made.
    fn remote(key: u16, opened: &Arc<AtomicUsize>, vet_ok: bool) -> RemoteDaemon {
        let opened = opened.clone();
        RemoteDaemon::new(
            key,
            "box".into(),
            move || {
                opened.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(daemon_answering(usize::MAX)) })
            },
            move |reply| if vet_ok { Ok(reply) } else { Err("not a reply it may give".into()) },
        )
    }

    #[tokio::test]
    async fn a_remote_machines_questions_reuse_its_own_lines_and_no_other_machines() {
        let (first, second) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        for _ in 0..3 {
            request(question(), Daemon::Remote(remote(60_001, &first, true))).await.unwrap();
        }
        request(question(), Daemon::Remote(remote(60_002, &second, true))).await.unwrap();
        assert_eq!(first.load(Ordering::SeqCst), 1);
        assert_eq!(second.load(Ordering::SeqCst), 1, "a line kept for one machine never serves another");

        forget_remote(60_001);
        request(question(), Daemon::Remote(remote(60_001, &first, true))).await.unwrap();
        assert_eq!(first.load(Ordering::SeqCst), 2, "a machine taken out of the list keeps no lines");
        forget_remote(60_001);
        forget_remote(60_002);
    }

    #[tokio::test]
    async fn an_answer_the_host_will_not_vet_is_an_error_not_a_reply() {
        let opened = Arc::new(AtomicUsize::new(0));
        let failure = request(question(), Daemon::Remote(remote(60_003, &opened, false)))
            .await
            .unwrap_err();
        assert!(format!("{failure:#}").contains("refused"), "{failure:#}");
        forget_remote(60_003);
    }

    #[tokio::test]
    async fn a_command_never_goes_out_on_a_kept_connection_and_leaves_none_behind() {
        let idle = IdleConnections::default();
        idle.lock().unwrap().push(daemon_answering(0));
        let command = ClientMsg::Command(Command::SelectWorkspace { ws: 1 });

        let reply = ask_reusing(&idle, MAX_IDLE_CONNECTIONS, &command, || async { Ok(daemon_answering(1)) }).await;

        assert!(matches!(reply, Ok(Reply::Ok)));
        assert_eq!(idle.lock().unwrap().len(), 1, "only the one that was there");
    }
}

async fn answered_within<T>(
    limit: Duration,
    request: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    match tokio::time::timeout(limit, request).await {
        Ok(answer) => answer,
        Err(_) => bail!(
            "the local daemon did not answer within {} seconds",
            limit.as_secs()
        ),
    }
}

#[cfg(test)]
mod request_timeout_tests {
    use super::answered_within;
    use std::time::Duration;

    #[tokio::test]
    async fn a_request_the_daemon_never_answers_fails_instead_of_waiting_for_ever() {
        let stuck = std::future::pending::<anyhow::Result<()>>();

        let error = answered_within(Duration::from_millis(20), stuck)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("did not answer"), "{error}");
    }

    #[tokio::test]
    async fn an_answer_in_time_is_passed_on_as_it_is() {
        let answer = answered_within(Duration::from_secs(5), async { Ok(7) }).await;

        assert_eq!(answer.unwrap(), 7);
    }
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
        Activity, can_restart_mismatched_daemon, is_daemon_command_line, update_status,
        validate_restart_activity, validate_restart_confirmation,
    };

    #[test]
    fn the_daemon_to_restart_may_be_the_terminal_apps_or_the_one_the_desktop_app_bundles() {
        let command = env!("MAITTY_APP_COMMAND");
        let line = |program: &str, args: &str| format!("{program}\0{args}\0").into_bytes();
        for program in [
            format!("/home/me/.local/bin/{command}"),
            command.to_string(),
            format!("/tmp/.mount_AppXyZ/usr/bin/{command}-gui-daemon"),
            format!("/repo/target/debug/{command}-gui-daemon-x86_64-unknown-linux-gnu"),
            format!(r"C:\Users\me\AppData\Local\Maitty\{command}.exe"),
            format!(r"C:\Program Files\Maitty\{command}-gui-daemon.EXE"),
        ] {
            assert!(is_daemon_command_line(&line(&program, "daemon")), "{program}");
        }
        for (program, args) in [
            (format!("/usr/bin/not{command}"), "daemon"),
            (format!("/usr/bin/{command}"), "mcp"),
            (format!("/usr/bin/{command}"), ""),
            ("/usr/bin/bash".to_string(), "daemon"),
        ] {
            assert!(!is_daemon_command_line(&line(&program, args)), "{program} {args}");
        }
        assert!(!is_daemon_command_line(b""));
    }

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
            version: "0.71.0".into(),
            sessions,
            working,
            waiting,
        }
    }

    #[test]
    fn an_older_service_with_saved_sessions_can_be_replaced_even_mid_turn() {
        let older = maitty_shared::protocol::PROTOCOL_VERSION - 1;
        assert!(can_restart_mismatched_daemon(
            older,
            Some(&activity(4, 1, 1)),
            true
        ));
        assert!(can_restart_mismatched_daemon(older, None, true));
    }

    #[test]
    fn an_older_service_with_no_sessions_can_be_replaced() {
        let older = maitty_shared::protocol::PROTOCOL_VERSION - 1;
        assert!(can_restart_mismatched_daemon(
            older,
            Some(&activity(0, 0, 0)),
            false
        ));
    }

    #[test]
    fn sessions_that_are_not_saved_are_never_thrown_away() {
        let older = maitty_shared::protocol::PROTOCOL_VERSION - 1;
        assert!(!can_restart_mismatched_daemon(
            older,
            Some(&activity(2, 0, 0)),
            false
        ));
        assert!(!can_restart_mismatched_daemon(older, None, false));
        assert!(validate_restart_activity(Some(&activity(2, 0, 0)), false).is_err());
        assert!(validate_restart_activity(None, false).is_err());
        assert!(validate_restart_activity(Some(&activity(2, 1, 0)), true).is_ok());
        assert!(validate_restart_activity(Some(&activity(0, 0, 0)), false).is_ok());
    }

    #[test]
    fn a_daemon_a_release_behind_is_reported_with_what_a_restart_costs() {
        let status = update_status(Some(&activity(11, 1, 0)), "0.72.0", true);

        assert_eq!(status["behind"], true);
        assert_eq!(status["daemonVersion"], "0.71.0");
        assert_eq!(status["canRestart"], true, "its sessions are saved");
        assert_eq!(status["sessionCount"], 11);
        assert_eq!(status["activeTurnCount"], 1);
    }

    #[test]
    fn a_behind_daemon_whose_sessions_are_not_saved_is_reported_but_not_restartable() {
        let status = update_status(Some(&activity(3, 0, 0)), "0.72.0", false);

        assert_eq!(status["behind"], true);
        assert_eq!(status["canRestart"], false);
    }

    #[test]
    fn a_daemon_on_this_release_or_newer_or_silent_is_not_behind() {
        for version in ["0.72.0", "0.73.0", ""] {
            let mut current = activity(2, 0, 0);
            current.version = version.into();
            let status = update_status(Some(&current), "0.72.0", true);
            assert_eq!(status["behind"], false, "{version:?}");
            assert_eq!(status["canRestart"], false, "{version:?}");
        }
        assert_eq!(update_status(None, "0.72.0", true)["behind"], false);
    }

    #[test]
    fn a_newer_service_is_never_replaced_by_an_older_gui() {
        assert!(!can_restart_mismatched_daemon(
            maitty_shared::protocol::PROTOCOL_VERSION + 1,
            Some(&activity(0, 0, 0)),
            true
        ));
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

#[cfg(test)]
mod agent_command_tests {
    use super::agent_command;

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    #[test]
    fn an_agent_and_its_flags_make_one_command_line() {
        assert_eq!(agent_command("claude", &[]).unwrap(), "claude");
        assert_eq!(
            agent_command("claude", &words(&["--model", "opus", "--effort", "high"])).unwrap(),
            "claude --model opus --effort high"
        );
        assert_eq!(
            agent_command("codex", &words(&["-c", "model_reasoning_effort=high"])).unwrap(),
            "codex -c model_reasoning_effort=high"
        );
    }

    #[test]
    fn an_unknown_agent_or_a_flag_that_could_add_a_command_is_refused() {
        assert!(agent_command("rm", &[]).is_err());
        for bad in ["a;b", "$(x)", "a b", "`x`", "a&&b", "a|b", "", "a\nb"] {
            assert!(agent_command("claude", &words(&[bad])).is_err(), "{bad:?} was allowed");
        }
    }

    #[test]
    fn keeping_an_agent_warm_asks_for_the_same_line_a_start_sends_and_refuses_what_a_start_refuses() {
        use super::{ClientMsg, Command, prewarm_message};
        let here = std::env::temp_dir();
        let flags = words(&["--model", "opus"]);
        match prewarm_message(here.clone(), "claude", &flags).unwrap() {
            ClientMsg::Command(Command::Prewarm { cwd, cmd }) => {
                assert_eq!(cmd, agent_command("claude", &flags).unwrap());
                assert_eq!(cwd, here.canonicalize().unwrap());
            }
            other => panic!("{other:?}"),
        }
        assert!(prewarm_message(here.clone(), "rm", &[]).is_err());
        assert!(prewarm_message(here.clone(), "claude", &words(&["a;b"])).is_err());
        assert!(prewarm_message(here.join("no-such-folder-here"), "claude", &[]).is_err());
    }

    #[test]
    fn a_message_queued_for_a_working_agent_is_checked_and_one_is_dropped_only_by_a_daemon_that_can() {
        use super::{ClientMsg, Command, PaneCommand, daemon_reads, queued_message};
        use maitty_shared::protocol::{DROP_QUEUED_SINCE, PROTOCOL_VERSION};
        assert!(queued_message("  ".into(), false).is_err());
        assert!(queued_message("x".repeat(super::MAX_HELD_MESSAGE_BYTES + 1), false).is_err());
        assert!(matches!(
            queued_message("/model opus".into(), true).unwrap().message(4),
            ClientMsg::Command(Command::QueueFollowUp { term: 4, text, first: true }) if text == "/model opus"
        ));
        let drop = PaneCommand::DropQueued {
            index: 1,
            text: "then test".into(),
        }
        .message(4);
        assert!(matches!(
            &drop,
            ClientMsg::Command(Command::DropQueued { term: 4, index: 1, text }) if text == "then test"
        ));
        assert!(daemon_reads(&drop, PROTOCOL_VERSION));
        assert!(!daemon_reads(&drop, DROP_QUEUED_SINCE - 1));
    }

    #[test]
    fn a_daemon_from_before_saved_conversations_is_not_asked_for_them() {
        use super::{ClientMsg, Query, daemon_reads};
        use maitty_shared::protocol::{PROTOCOL_VERSION, SAVED_CONVERSATIONS_SINCE};
        let ask = ClientMsg::Query(Query::SavedConversations {
            agent: "claude".into(),
            folder: "~/app".into(),
            tag: 0,
        });
        assert!(daemon_reads(&ask, PROTOCOL_VERSION));
        assert!(!daemon_reads(&ask, SAVED_CONVERSATIONS_SINCE - 1));
    }

    #[test]
    fn a_daemon_from_before_closed_sessions_is_not_asked_for_them() {
        use super::{ClientMsg, Query, daemon_reads};
        use maitty_shared::protocol::{CLOSED_SESSIONS_SINCE, PROTOCOL_VERSION};
        let ask = ClientMsg::Query(Query::ClosedSessions);
        assert!(daemon_reads(&ask, PROTOCOL_VERSION));
        assert!(!daemon_reads(&ask, CLOSED_SESSIONS_SINCE - 1));
    }

    #[test]
    fn a_daemon_from_before_reopening_closed_sessions_is_not_asked_to() {
        use super::{daemon_reads, reopen_closed_message};
        use maitty_shared::protocol::{PROTOCOL_VERSION, REOPEN_CLOSED_SINCE};
        let reopen = reopen_closed_message("claude".into(), "abc-123".into());
        assert!(daemon_reads(&reopen, PROTOCOL_VERSION));
        assert!(!daemon_reads(&reopen, REOPEN_CLOSED_SINCE - 1));
    }

    fn closed_in(cwd: std::path::PathBuf) -> maitty_shared::protocol::ClosedSession {
        maitty_shared::protocol::ClosedSession {
            agent: "claude".into(),
            session: "abc-123".into(),
            cwd,
            branch: Some("dev".into()),
            title: Some("Fix the parser".into()),
            at: 1_700_000_000,
            worktree: None,
            repo: None,
        }
    }

    #[tokio::test]
    async fn a_closed_session_says_whether_its_folder_is_still_there() {
        let here = std::env::temp_dir();
        let gone = here.join("maitty-closed-session-folder-that-was-removed");
        let list = vec![closed_in(here), closed_in(gone)];
        let rows = super::with_folder_presence(list, super::FOLDER_LOOK_LIMIT, |path| path.is_dir())
            .await
            .unwrap();
        assert_eq!(rows[0]["folder_present"], true);
        assert_eq!(rows[1]["folder_present"], false);
        assert_eq!(rows[0]["session"], "abc-123");
        assert_eq!(rows[0]["branch"], "dev");
    }

    #[tokio::test]
    async fn a_folder_that_does_not_answer_in_time_reads_as_missing_and_holds_up_nothing() {
        use std::time::Duration;
        let hung = std::path::PathBuf::from("//dead-share/app");
        let list = vec![closed_in(hung.clone()), closed_in(hung.clone()), closed_in(std::env::temp_dir())];
        let limit = Duration::from_millis(100);
        let started = std::time::Instant::now();
        let rows = super::with_folder_presence(list, limit, move |path| {
            if path == hung {
                std::thread::sleep(Duration::from_secs(2));
            }
            true
        })
        .await
        .unwrap();
        assert!(started.elapsed() < Duration::from_secs(1), "the looks run side by side under one deadline");
        assert_eq!(rows[0]["folder_present"], false);
        assert_eq!(rows[1]["folder_present"], false);
        assert_eq!(rows[2]["folder_present"], true);
    }

    #[test]
    fn a_daemon_from_before_warm_agents_is_not_asked_for_one() {
        use super::{ClientMsg, Command, daemon_reads};
        use maitty_shared::protocol::{PREWARM_SINCE, PROTOCOL_VERSION};
        let drop = ClientMsg::Command(Command::DropPrewarm);
        assert!(daemon_reads(&drop, PROTOCOL_VERSION));
        assert!(daemon_reads(&drop, PREWARM_SINCE));
        assert!(!daemon_reads(&drop, PREWARM_SINCE - 1));
    }
}

#[cfg(test)]
mod pane_command_tests {
    use super::*;

    #[test]
    fn each_pane_command_is_one_message_for_its_pane() {
        let cases = [
            (
                PaneCommand::Input(b"ls\r".to_vec()),
                ClientMsg::Input {
                    term: 4,
                    data: b"ls\r".to_vec(),
                },
            ),
            (PaneCommand::WantReplay, ClientMsg::WantReplay { term: 4 }),
            (
                PaneCommand::Resize {
                    cols: 100,
                    rows: 30,
                },
                ClientMsg::Command(Command::TerminalResize {
                    term: 4,
                    cols: 100,
                    rows: 30,
                }),
            ),
            (
                PaneCommand::Rename("build".into()),
                ClientMsg::Command(Command::RenamePane {
                    term: 4,
                    name: "build".into(),
                }),
            ),
            (
                PaneCommand::SendWhenReady("fix the parser".into()),
                ClientMsg::Command(Command::SendWhenReady {
                    term: 4,
                    text: "fix the parser".into(),
                }),
            ),
            (
                PaneCommand::Pin(true),
                ClientMsg::Command(Command::PinPane {
                    term: 4,
                    pinned: true,
                }),
            ),
            (
                PaneCommand::Archive(false),
                ClientMsg::Command(Command::ArchivePane {
                    term: 4,
                    archived: false,
                }),
            ),
            (
                PaneCommand::Seen,
                ClientMsg::Command(Command::MarkSeen { term: 4 }),
            ),
            (
                PaneCommand::Close,
                ClientMsg::Command(Command::ClosePane { term: 4 }),
            ),
            (
                PaneCommand::CloseSession,
                ClientMsg::Command(Command::CloseSession { term: 4 }),
            ),
            (
                PaneCommand::UndoClose,
                ClientMsg::Command(Command::UndoClose { term: 4 }),
            ),
        ];
        for (command, expected) in cases {
            let shown = format!("{command:?}");
            assert_eq!(
                serde_json::to_string(&command.message(4)).unwrap(),
                serde_json::to_string(&expected).unwrap(),
                "{shown}"
            );
        }
    }
}

#[cfg(test)]
mod chat_posture_tests {
    use super::{chat_session_permissions, Permissions};
    use maitty_shared::conversation::{ApprovalPolicy, Sandbox};

    #[test]
    fn a_chat_session_asks_unless_full_access_is_requested() {
        assert_eq!(
            chat_session_permissions(false).launch_policy(),
            (Sandbox::WorkspaceWrite, ApprovalPolicy::OnRequest)
        );
        assert!(matches!(
            chat_session_permissions(true),
            Permissions::FullAccess
        ));
        assert_eq!(
            chat_session_permissions(true).launch_policy(),
            (Sandbox::FullAccess, ApprovalPolicy::Never)
        );
    }
}

/// The workspace-palette indexes a fleet may be coloured with, in the order the window shows
/// them: teal, violet, blue, green, pink, orange. They are positions in the default
/// `ui.workspace_colors`, whose meaning saved colours keep; amber (4) and red are left out so a
/// fleet never looks like a status.
pub const FLEET_COLORS: [u8; 6] = [3, 0, 1, 7, 2, 5];

/// What the window may ask a machine's daemon to do with a fleet (a workspace) or with a session
/// moving between fleets. Like `PaneCommand`, one place makes each message for this computer and
/// for another machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetCommand {
    /// A named fleet with one shell; its colour is the first of `FLEET_COLORS` not in use.
    Create { name: String },
    Rename { fleet: u32, name: String },
    /// `color` is a palette index from `FLEET_COLORS`.
    Recolor { fleet: u32, color: u8 },
    /// Move a session to another fleet on the same machine.
    MoveSession { term: u32, fleet: u32 },
    /// A new shell in the fleet, in `folder` or, when that is left out, the fleet's own folder.
    StartShell { fleet: u32, folder: Option<String> },
}

impl FleetCommand {
    /// The message that asks the daemon, or why the window's request is refused.
    pub fn message(self) -> Result<ClientMsg, String> {
        let command = match self {
            FleetCommand::Create { name } => Command::NewFleet {
                name: fleet_name(&name)?,
                cwd: None,
                colors: FLEET_COLORS.to_vec(),
            },
            FleetCommand::Rename { fleet, name } => Command::RenameWorkspace {
                ws: fleet,
                name: fleet_name(&name)?,
            },
            FleetCommand::Recolor { fleet, color } => {
                if !FLEET_COLORS.contains(&color) {
                    return Err("That is not one of the fleet colours".to_string());
                }
                Command::SetWorkspaceColor { ws: fleet, color }
            }
            FleetCommand::MoveSession { term, fleet } => Command::MovePane {
                term,
                to: Some(fleet),
                name: None,
            },
            FleetCommand::StartShell { fleet, folder } => Command::NewShip {
                ws: fleet,
                cwd: folder
                    .map(|f| f.trim().to_string())
                    .filter(|f| !f.is_empty())
                    .map(PathBuf::from),
                cmd: None,
            },
        };
        Ok(ClientMsg::Command(command))
    }
}

/// The most bytes a fleet name may have; the daemon holds the same limit.
const MOST_FLEET_NAME_BYTES: usize = 80;

/// A fleet name as the daemon will take it: trimmed, not empty, short, no control characters.
fn fleet_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > MOST_FLEET_NAME_BYTES || name.chars().any(char::is_control)
    {
        return Err("Give the fleet a short name".to_string());
    }
    Ok(name.to_string())
}

/// Ask this computer's daemon to do `command` to a fleet.
pub async fn fleet_command(command: FleetCommand, daemon_executable: PathBuf) -> Result<Value> {
    let message = command.message().map_err(anyhow::Error::msg)?;
    let reply = request(message, daemon_executable).await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected fleet reply"),
    }
}

#[cfg(test)]
mod fleet_tests {
    use super::{FLEET_COLORS, FleetCommand};
    use maitty_shared::protocol::{ClientMsg, Command};

    fn command(of: FleetCommand) -> Command {
        match of.message().unwrap() {
            ClientMsg::Command(command) => command,
            other => panic!("not a command: {other:?}"),
        }
    }

    #[test]
    fn a_fleet_is_created_with_a_trimmed_name_and_only_the_fleet_colours() {
        let Command::NewFleet { name, colors, .. } = command(FleetCommand::Create {
            name: " api ".into(),
        }) else {
            panic!("expected NewFleet")
        };
        assert_eq!(name, "api");
        assert_eq!(colors, FLEET_COLORS);
        assert!(!colors.contains(&4), "amber is not a fleet colour");
    }

    #[test]
    fn a_nameless_or_oversized_fleet_name_is_refused() {
        for name in ["", "   ", "a\nb", &"x".repeat(81)] {
            assert!(
                FleetCommand::Create { name: name.into() }.message().is_err(),
                "{name:?}"
            );
            assert!(
                FleetCommand::Rename {
                    fleet: 1,
                    name: name.into()
                }
                .message()
                .is_err()
            );
        }
    }

    #[test]
    fn only_a_fleet_colour_can_be_set() {
        for color in FLEET_COLORS {
            assert!(FleetCommand::Recolor { fleet: 1, color }.message().is_ok());
        }
        for color in [4, 6, 8, 255] {
            assert!(FleetCommand::Recolor { fleet: 1, color }.message().is_err());
        }
    }

    #[test]
    fn moving_and_starting_name_the_fleet_and_a_blank_folder_means_the_fleets_own() {
        assert!(matches!(
            command(FleetCommand::MoveSession { term: 4, fleet: 2 }),
            Command::MovePane {
                term: 4,
                to: Some(2),
                name: None
            }
        ));
        let Command::NewShip { ws, cwd, cmd } = command(FleetCommand::StartShell {
            fleet: 2,
            folder: Some("  ".into()),
        }) else {
            panic!("expected NewShip")
        };
        assert_eq!((ws, cwd, cmd), (2, None, None));
        let Command::NewShip { cwd, .. } = command(FleetCommand::StartShell {
            fleet: 2,
            folder: Some("/srv/app".into()),
        }) else {
            panic!("expected NewShip")
        };
        assert_eq!(cwd, Some("/srv/app".into()));
    }
}

#[cfg(test)]
mod handoff_tests {
    use super::*;

    #[test]
    fn the_brief_lists_uncommitted_files_and_says_the_commits_are_not_listed() {
        let folder = std::env::temp_dir().join(format!("maitty-handoff-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder");
        let git = |args: &[&str]| assert!(std::process::Command::new("git").args(args).current_dir(&folder).status().expect("git").success());
        git(&["init", "-q"]);
        std::fs::write(folder.join("b.txt"), "x").expect("a file");
        std::fs::write(folder.join("a.txt"), "x").expect("a file");
        let changed = uncommitted_changes(&folder).expect("a status");
        assert_eq!(changed.files, ["a.txt", "b.txt"]);
        assert!(changed.committed_unknown.is_some());
        assert!(uncommitted_changes(&folder.join("missing")).is_err());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
