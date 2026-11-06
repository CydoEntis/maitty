//! Local-socket transport: named pipes on Windows, Unix domain sockets elsewhere. With
//! `--remote host` the same messages go through `ssh host <app> proxy` instead, to the
//! server on that machine.

use crate::brand::APP_COMMAND;
use crate::protocol::{self, ClientMsg, ServerMsg};
use anyhow::{Context, Result};
use interprocess::local_socket::tokio::{RecvHalf, SendHalf, Stream, prelude::*};
use interprocess::local_socket::{ListenerOptions, Name};
use maitty_ipc_transport::framed_parts;
pub use maitty_ipc_transport::{Reader, STREAM_MARKER, Writer, is_decode_error, send};
pub use maitty_machines::{
    CONNECT_BUDGET, Machine, NO_DAEMON_EXIT, OtherVersion, PROXY_ATTACH_ONLY_FLAG, SSH_PROGRAM,
    other_version,
};

#[allow(dead_code)]
type Halves = (RecvHalf, SendHalf);

pub(crate) const STRUCTURED_ENV: [&str; 4] = [
    concat!(env!("APP_ENV"), "_CONVERSATION_ID"),
    concat!(env!("APP_ENV"), "_CONVERSATION_GENERATION"),
    concat!(env!("APP_ENV"), "_CONVERSATION_CONNECTION"),
    concat!(env!("APP_ENV"), "_CONVERSATION_AUTH"),
];
/// The variables that send this client's commands to another machine.
pub(crate) const REMOTE_ROUTE_ENV: [&str; 3] = [
    concat!(env!("APP_ENV"), "_REMOTE"),
    concat!(env!("APP_ENV"), "_SSH"),
    concat!(env!("APP_ENV"), "_REMOTE_CMD"),
];
pub(crate) const RUNTIME_ROUTE_ENV: [&str; 6] = [
    concat!(env!("APP_ENV"), "_SOCKET"),
    concat!(env!("APP_ENV"), "_CONFIG"),
    concat!(env!("APP_ENV"), "_DATA_DIR"),
    concat!(env!("APP_ENV"), "_REMOTE"),
    concat!(env!("APP_ENV"), "_SSH"),
    concat!(env!("APP_ENV"), "_REMOTE_CMD"),
];

pub fn has_structured_identity() -> bool {
    STRUCTURED_ENV
        .iter()
        .any(|key| std::env::var_os(key).is_some())
}

pub fn framed(stream: Stream) -> (Reader, Writer) {
    let (read, write) = stream.split();
    framed_parts(read, write)
}

fn structured_identity(
    values: [Option<std::ffi::OsString>; 4],
) -> Result<Option<protocol::StructuredIdentity>> {
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    let parsed = || -> Option<_> {
        let number = |index: usize| {
            values[index]
                .as_ref()?
                .to_str()?
                .parse::<u64>()
                .ok()
                .filter(|number| *number != 0)
        };
        let token = values[3].as_ref()?.to_str()?.to_owned();
        if token.is_empty() || token.chars().any(char::is_control) {
            return None;
        }
        Some(protocol::StructuredIdentity {
            conversation: crate::conversation::ConversationId(number(0)?),
            generation: number(1)?,
            connection: number(2)?,
            token,
        })
    };
    parsed()
        .map(Some)
        .ok_or_else(|| anyhow::anyhow!("Structured runtime identity is incomplete or malformed"))
}

/// Where the server notes its process id, so the app of another version can still stop it.
pub fn pid_file() -> std::path::PathBuf {
    crate::config::data_dir().join(format!("{}.pid", socket_id()))
}

/// The process groups of the providers this daemon started (`daemon/provider_groups.rs`).
#[cfg(unix)]
pub fn providers_file() -> std::path::PathBuf {
    crate::config::data_dir().join(format!("{}.providers", socket_id()))
}

/// Server identity. `<APP>_SOCKET` picks a separate server (like `tmux -L`).
pub fn socket_id() -> String {
    let label = std::env::var(concat!(env!("APP_ENV"), "_SOCKET")).ok();
    let username = std::env::var("USERNAME").ok();
    let user = std::env::var("USER").ok();
    maitty_ipc_transport::socket_id(
        APP_COMMAND,
        label.as_deref(),
        username.as_deref(),
        user.as_deref(),
    )
}

/// Windows: a named pipe (its default security lets only this user and admins write).
/// Unix: a socket file in a folder only this user can open, so no other local user can
/// reach the daemon (an abstract socket would have no permissions at all).
fn name() -> Result<Name<'static>> {
    let id = socket_id();
    #[cfg(unix)]
    let directory = Some(socket_dir()?);
    #[cfg(windows)]
    let directory: Option<std::path::PathBuf> = None;
    maitty_ipc_transport::socket_name(&id, directory.as_deref())
}

/// `$XDG_RUNTIME_DIR/<app>-<uid>` (or the temp folder's), created 0700 and checked to be
/// ours and private before use.
#[cfg(unix)]
fn socket_dir() -> Result<std::path::PathBuf> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    maitty_ipc_transport::private_socket_dir(
        APP_COMMAND,
        &format!("start {APP_COMMAND} again"),
        runtime_dir.as_deref(),
        &std::env::temp_dir(),
    )
}

/// No daemon runs here and `proxy --attach-only` was asked not to start one.
#[derive(Debug)]
pub struct NoDaemonHere;

impl std::fmt::Display for NoDaemonHere {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no {APP_COMMAND} server is running here")
    }
}

impl std::error::Error for NoDaemonHere {}

pub fn is_no_daemon_here(e: &anyhow::Error) -> bool {
    e.is::<NoDaemonHere>()
}

/// `<app> proxy`, run by ssh on the far side: this machine's server over stdin and stdout.
/// Started if needed, unless `attach_only`: then a missing server is `NoDaemonHere`, before
/// anything is written to stdout.
pub async fn proxy(attach_only: bool) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = connect().await;
    if stream.is_err() {
        if attach_only {
            return Err(NoDaemonHere.into());
        }
        spawn_daemon(None)?;
        for _ in 0..60 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            stream = connect().await;
            if stream.is_ok() {
                break;
            }
        }
    }
    let (mut sr, mut sw) = stream
        .context(concat!(
            "couldn't reach or start the ",
            env!("APP_COMMAND"),
            " server here"
        ))?
        .split();
    // Before any frame, so the client can tell the stream from login text printed ahead of it.
    let mut out = tokio::io::stdout();
    out.write_all(STREAM_MARKER).await?;
    out.flush().await?;
    let up = async {
        let mut stdin = tokio::io::stdin();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = stdin.read(&mut buf).await?;
            if n == 0 {
                return anyhow::Ok(());
            }
            sw.write_all(&buf[..n]).await?;
        }
    };
    let down = async {
        let mut stdout = tokio::io::stdout();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = sr.read(&mut buf).await?;
            if n == 0 {
                return anyhow::Ok(());
            }
            stdout.write_all(&buf[..n]).await?;
            stdout.flush().await?;
        }
    };
    tokio::select! {
        r = up => r,
        r = down => r,
    }
}

pub async fn connect() -> Result<Stream> {
    Ok(Stream::connect(name()?).await?)
}

/// Proof that this process is the one daemon for its socket. The OS releases it when the process
/// ends, so a crashed daemon never leaves a stale lock.
#[derive(Debug)]
pub struct DaemonLock {
    _file: std::fs::File,
}

fn lock_path() -> std::path::PathBuf {
    crate::config::data_dir().join(format!("{}.lock", socket_id()))
}

/// Take the right to be the daemon. Two clients starting at once both pass the "does a daemon
/// answer?" check, and `listen` then deletes the other's socket; each restores the saved session
/// and spawns every pane. Only one process can hold this lock, so only one gets that far.
pub fn lock_daemon() -> Result<DaemonLock> {
    lock_daemon_at(&lock_path())
}

fn lock_daemon_at(path: &std::path::Path) -> Result<DaemonLock> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).context("creating the daemon data folder")?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .context("opening the daemon lock file")?;
    match file.try_lock() {
        Ok(()) => Ok(DaemonLock { _file: file }),
        Err(std::fs::TryLockError::WouldBlock) => anyhow::bail!(
            "a {APP_COMMAND} daemon is already running on {}",
            socket_id()
        ),
        Err(std::fs::TryLockError::Error(e)) => Err(e).context("locking the daemon lock file"),
    }
}

pub fn listen() -> Result<interprocess::local_socket::tokio::Listener> {
    // A stale socket file from a crashed daemon would make bind fail. (Callers check
    // that no daemon answers first.)
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(socket_dir()?.join(socket_id()));
    }
    ListenerOptions::new()
        .name(name()?)
        .create_tokio()
        .context("creating the daemon socket")
}

pub async fn recv_server(r: &mut Reader) -> Result<Option<ServerMsg>> {
    maitty_ipc_transport::receive(r).await
}

pub async fn recv_client(r: &mut Reader) -> Result<Option<ClientMsg>> {
    maitty_ipc_transport::receive(r).await
}

/// Connect to the machine `--remote` names (else this one) and complete the handshake.
pub async fn open(attach: bool) -> Result<(Reader, Writer)> {
    open_to(&Machine::current(), attach).await
}

/// What the terminal app says about itself in a Hello: the pane and the conversation it runs in,
/// from the environment.
pub fn terminal_dial(attach: bool) -> Result<maitty_machines::Dial> {
    let structured = structured_identity(STRUCTURED_ENV.map(std::env::var_os))?;
    Ok(maitty_machines::Dial {
        attach,
        surface: protocol::Surface::Terminal,
        replays: true,
        term: std::env::var(concat!(env!("APP_ENV"), "_TERM_ID")).ok(),
        token: std::env::var(concat!(env!("APP_ENV"), "_PANE_TOKEN")).ok(),
        structured,
    })
}

/// Connect to `machine` and complete the handshake.
pub async fn open_to(machine: &Machine, attach: bool) -> Result<(Reader, Writer)> {
    let dial = terminal_dial(attach)?;
    if machine.is_remote() {
        return maitty_machines::open_remote(machine, dial).await;
    }
    let hello = maitty_machines::hello(machine, dial);
    let (mut r, mut w) = framed(connect().await?);
    send(&mut w, &hello).await?;
    let version = maitty_ipc_transport::welcome_version(&mut r).await?;
    if version == protocol::PROTOCOL_VERSION {
        Ok((r, w))
    } else {
        Err(OtherVersion { daemon: version }.into())
    }
}

/// Connect, starting the daemon in the background if none is running.
pub async fn open_or_spawn(attach: bool) -> Result<(Reader, Writer)> {
    open_or_spawn_with(&Machine::current(), attach, None).await
}

/// Reconnect after replacing the executable, spawning the newly installed image if needed.
pub async fn open_or_spawn_installed(attach: bool) -> Result<(Reader, Writer)> {
    let exe = crate::installed_executable::installed_executable()?;
    open_or_spawn_with(&Machine::current(), attach, Some(exe)).await
}

async fn open_or_spawn_with(
    machine: &Machine,
    attach: bool,
    exe: Option<std::path::PathBuf>,
) -> Result<(Reader, Writer)> {
    if has_structured_identity() {
        return open_to(machine, attach).await;
    }
    // Over ssh the far side starts its own server.
    if machine.is_remote() {
        return open_to(machine, attach).await;
    }
    match open_to(machine, attach).await {
        Ok(c) => return Ok(c),
        // A server is running: starting another would only fail to take its socket.
        Err(e) if other_version(&e).is_some() => return Err(e),
        Err(_) => {}
    }
    spawn_daemon(exe.as_deref())?;
    let mut last = None;
    for _ in 0..60 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        match open_to(machine, attach).await {
            Ok(c) => return Ok(c),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("daemon did not start")))
}

fn spawn_daemon(exe: Option<&std::path::Path>) -> Result<()> {
    daemon_command(exe)?
        .spawn()
        .context("starting the daemon")?;
    Ok(())
}

fn daemon_command(exe: Option<&std::path::Path>) -> Result<std::process::Command> {
    let current = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe.unwrap_or(&current));
    cmd.arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        // A hidden console of its own, so ConPTY works and closing this window doesn't kill it.
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | crate::proc::CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Own session: survives the client's terminal closing and never sees its Ctrl+C.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_second_daemon_is_refused_while_the_first_holds_the_lock() {
        let path =
            std::env::temp_dir().join(format!("daemon-lock-test-{}.lock", std::process::id()));
        let first = super::lock_daemon_at(&path).unwrap();
        let second = super::lock_daemon_at(&path).unwrap_err();
        assert!(
            format!("{second:#}").contains("already running"),
            "a second daemon must be refused, got: {second:#}"
        );
        drop(first);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn daemon_command_starts_daemon() {
        let command = super::daemon_command(None).unwrap();
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [std::ffi::OsStr::new("daemon")]
        );
    }

    #[test]
    fn structured_identity_is_explicit_and_partial_claims_never_become_human() {
        assert!(
            super::structured_identity([None, None, None, None])
                .unwrap()
                .is_none()
        );
        for values in [
            [Some("1".into()), None, None, None],
            [None, None, None, Some("".into())],
            [
                Some("1".into()),
                Some("0".into()),
                Some("1".into()),
                Some("secret".into()),
            ],
        ] {
            assert!(super::structured_identity(values).is_err());
        }
        let claim = super::structured_identity([
            Some("1".into()),
            Some("2".into()),
            Some("3".into()),
            Some("synthetic-token".into()),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(
            (claim.conversation.0, claim.generation, claim.connection),
            (1, 2, 3)
        );
        assert!(!format!("{claim:?}").contains("synthetic-token"));
    }

    #[test]
    fn a_bad_message_is_a_decode_error() {
        let e = crate::protocol::decode::<crate::protocol::ServerMsg>(&[0xc1, 0xff]).unwrap_err();
        assert!(super::is_decode_error(&e));
        assert!(!super::is_decode_error(&anyhow::anyhow!(
            "connection reset"
        )));
    }
}
