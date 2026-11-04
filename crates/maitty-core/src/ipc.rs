//! Local-socket transport: named pipes on Windows, Unix domain sockets elsewhere. With
//! `--remote host` the same messages go through `ssh host <app> proxy` instead, to the
//! server on that machine.

use crate::brand::{APP_COMMAND, APP_ENV};
use crate::link_failure::SshEnd;
use crate::protocol::{self, ClientMsg, ServerMsg};
use crate::ssh_command::{self, ProxyMode};
use anyhow::{Context, Result};
use interprocess::local_socket::tokio::{RecvHalf, SendHalf, Stream, prelude::*};
use interprocess::local_socket::{ListenerOptions, Name};
use maitty_ipc_transport::{LIMITS, NoMarker, framed_parts, reader, skip_login_noise, writer};
pub use maitty_ipc_transport::{Reader, STREAM_MARKER, Writer, is_decode_error, send};
use maitty_settings::config::MachineDef;

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

/// The running server is another the app version: the two can't talk (see PROTOCOL_VERSION).
#[derive(Debug)]
pub struct OtherVersion {
    pub daemon: u32,
}

impl std::fmt::Display for OtherVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            concat!(
                "the running server is another ",
                env!("APP_COMMAND"),
                " version (protocol v{}, this one v{}); run ",
                env!("APP_COMMAND"),
                " in a terminal to restart it, or `",
                env!("APP_COMMAND"),
                " kill-server` and run ",
                env!("APP_COMMAND"),
                " again"
            ),
            self.daemon,
            protocol::PROTOCOL_VERSION
        )
    }
}

impl std::error::Error for OtherVersion {}

/// The protocol of the running server when an error says it is another the app version.
pub fn other_version(e: &anyhow::Error) -> Option<u32> {
    e.chain()
        .find_map(|c| c.downcast_ref::<OtherVersion>())
        .map(|o| o.daemon)
}

/// Where the server notes its process id, so the app of another version can still stop it.
pub fn pid_file() -> std::path::PathBuf {
    crate::config::data_dir().join(format!("{}.pid", socket_id()))
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

/// The program that reaches a machine unless `<APP>_SSH` names another.
pub const SSH_PROGRAM: &str = "ssh";

/// How long one connection attempt may take: a little over ssh's own connect timeout, so ssh
/// reports its failure before this gives up on it.
pub const CONNECT_BUDGET: std::time::Duration =
    std::time::Duration::from_secs(ssh_command::CONNECT_TIMEOUT_SECS as u64 + 5);

/// The exit status of `proxy --attach-only` when no daemon is running on that machine. ssh
/// passes the remote command's status through, so this is how the client tells that apart
/// from a machine it could not reach.
pub const NO_DAEMON_EXIT: i32 = 74;

/// The flag `proxy` takes to attach without starting a daemon.
pub const PROXY_ATTACH_ONLY_FLAG: &str = ssh_command::ATTACH_ONLY_FLAG;

/// A machine whose daemon a connection reaches: this one, or another over ssh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    name: String,
    route: Route,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Route {
    Local,
    Ssh(SshRoute),
    Command(CommandRoute),
}

/// How to run `<app> proxy` on another machine through the user's own program (`via`) in
/// place of ssh: it carries stdin and stdout unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandRoute {
    /// The program and its arguments, `{command}` still in place.
    via: Vec<String>,
    remote_cmd: String,
    proxy: ProxyMode,
}

/// How to run `<app> proxy` on another machine.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SshRoute {
    target: String,
    /// The program and its leading arguments (`["ssh", "-p", "2222"]`).
    ssh: Vec<String>,
    /// The app's command on the far side (`~/.cargo/bin/<app>`).
    remote_cmd: String,
    proxy: ProxyMode,
}

impl Machine {
    /// This machine: the daemon on its own socket.
    pub fn local() -> Machine {
        Machine {
            name: "local".into(),
            route: Route::Local,
        }
    }

    /// Another machine, reached by running `<remote_cmd> proxy` on `target` through `ssh`
    /// (the program, then its leading arguments).
    pub fn ssh(
        target: impl Into<String>,
        ssh: Vec<String>,
        remote_cmd: impl Into<String>,
    ) -> Machine {
        let target = target.into();
        Machine {
            name: target.clone(),
            route: Route::Ssh(SshRoute {
                target,
                ssh,
                remote_cmd: remote_cmd.into(),
                proxy: ProxyMode::StartsDaemon,
            }),
        }
    }

    /// This machine's proxy only attaches: it never starts a daemon the user did not.
    pub fn attach_only(mut self) -> Machine {
        if let Route::Ssh(route) = &mut self.route {
            route.proxy = ProxyMode::AttachOnly;
        }
        self
    }

    /// This machine with a proxy that starts the daemon when none runs there. The daemon
    /// exits when nothing is attached, so a first connection or an asked-for retry has to be
    /// able to start it; a redial after a lost line must not (`attach_only`).
    pub fn starting(&self) -> Machine {
        let mut machine = self.clone();
        match &mut machine.route {
            Route::Ssh(route) => route.proxy = ProxyMode::StartsDaemon,
            Route::Command(route) => route.proxy = ProxyMode::StartsDaemon,
            Route::Local => {}
        }
        machine
    }

    /// A configured machine: `program` (ssh), the def's `ssh_args`, then its target. The
    /// def must have passed `validate_machines`. It only attaches; the link supervisor asks
    /// for `starting` on a first connection and on a retry.
    pub fn from_def(def: &MachineDef, program: &str) -> Machine {
        Machine::from_def_via(def, &[program.to_string()])
    }

    /// `from_def` with `ssh` as the program and its leading arguments: a window that adds a
    /// machine while it runs keeps the program it started with, and tests put a shim there.
    pub fn from_def_via(def: &MachineDef, ssh: &[String]) -> Machine {
        match &def.via {
            Some(via) => Machine::through_command(def, via),
            None => Machine::named_by(def, ssh).attach_only(),
        }
    }

    /// A configured machine reached by its own `via` program; it only attaches by default.
    fn through_command(def: &MachineDef, via: &[String]) -> Machine {
        Machine {
            name: def.name.clone(),
            route: Route::Command(CommandRoute {
                via: via.to_vec(),
                remote_cmd: def.remote_cmd.as_deref().unwrap_or(APP_COMMAND).to_string(),
                proxy: ProxyMode::AttachOnly,
            }),
        }
    }

    fn named_by(def: &MachineDef, ssh: &[String]) -> Machine {
        let argv = ssh.iter().chain(def.ssh_args.iter()).cloned().collect();
        let remote_cmd = def.remote_cmd.as_deref().unwrap_or(APP_COMMAND);
        Machine {
            name: def.name.clone(),
            ..Machine::ssh(def.target.as_str(), argv, remote_cmd)
        }
    }

    /// `--remote host` as a machine named after the host. `ssh` is `<APP>_SSH` (a program and
    /// its leading arguments), `remote_cmd` is `<APP>_REMOTE_CMD`.
    fn adhoc(host: &str, ssh: Option<&str>, remote_cmd: Option<String>) -> Machine {
        let mut words = ssh.unwrap_or(SSH_PROGRAM).split_whitespace();
        let program = words.next().unwrap_or(SSH_PROGRAM);
        let def = MachineDef {
            name: host.into(),
            target: host.into(),
            ssh_args: words.map(String::from).collect(),
            remote_cmd,
            ..MachineDef::default()
        };
        Machine::named_by(&def, &[program.to_string()])
    }

    /// The machine `--remote host` (or `<APP>_REMOTE`) names, else this one. The flag's
    /// only reader: everything else is handed a `Machine`.
    pub fn current() -> Machine {
        let Some(host) = std::env::var(concat!(env!("APP_ENV"), "_REMOTE"))
            .ok()
            .filter(|s| !s.trim().is_empty())
        else {
            return Machine::local();
        };
        let ssh = std::env::var(concat!(env!("APP_ENV"), "_SSH")).ok();
        let remote_cmd = std::env::var(concat!(env!("APP_ENV"), "_REMOTE_CMD")).ok();
        Machine::adhoc(&host, ssh.as_deref(), remote_cmd)
    }

    pub fn is_remote(&self) -> bool {
        !matches!(self.route, Route::Local)
    }

    /// The program of this machine's custom transport, safe to show (its arguments can hold
    /// tokens, so they are never shown); `None` when it is reached over ssh or is this computer.
    pub fn transport_program(&self) -> Option<String> {
        match &self.route {
            Route::Command(route) => route.via.first().map(|p| crate::link_failure::clean(p)),
            Route::Local | Route::Ssh(_) => None,
        }
    }

    /// The line that checks this machine without asking anyone anything: it logs in with
    /// what ssh already has and prints the app's version there.
    pub fn probe_argv(&self) -> Result<Vec<String>> {
        let route = self.ssh_route()?;
        Ok(ssh_command::ssh_argv_probe(
            &route.ssh,
            &route.target,
            &route.remote_cmd,
        )?)
    }

    /// The line that asks this machine for its platform (`uname -sm`), without asking a person.
    pub fn platform_argv(&self) -> Result<Vec<String>> {
        let route = self.ssh_route()?;
        Ok(ssh_command::ssh_argv_platform(&route.ssh, &route.target)?)
    }

    /// The line that asks this machine where its login shell finds the app.
    pub fn where_argv(&self) -> Result<Vec<String>> {
        let route = self.ssh_route()?;
        Ok(ssh_command::ssh_argv_where(&route.ssh, &route.target)?)
    }

    /// The options every connection to a machine carries, in words (see `doctor`).
    pub fn hardening() -> String {
        ssh_command::hardening_summary()
    }

    /// The line that runs the install script on this machine, the binary on its stdin.
    pub fn install_argv(&self, destination: &str, bytes: u64) -> Result<Vec<String>> {
        let route = self.ssh_route()?;
        Ok(ssh_command::ssh_argv_install(
            &route.ssh,
            &route.target,
            destination,
            bytes,
        )?)
    }

    /// The line that runs the paste script on this machine, the image on its stdin: the far
    /// side keeps it as `name` and prints where.
    pub fn paste_argv(&self, name: &str, bytes: u64) -> Result<Vec<String>> {
        let route = self.ssh_route()?;
        Ok(ssh_command::ssh_argv_paste(
            &route.ssh,
            &route.target,
            name,
            bytes,
        )?)
    }

    /// The line that has the app installed at `program` there restart its daemon if idle.
    pub fn restart_if_idle_argv(&self, program: &str) -> Result<Vec<String>> {
        let route = self.ssh_route()?;
        Ok(ssh_command::ssh_argv_restart_if_idle(
            &route.ssh,
            &route.target,
            program,
        )?)
    }

    /// The command that runs the app there, as configured (`remote_cmd`); `None` for this computer.
    pub fn remote_cmd(&self) -> Option<&str> {
        match &self.route {
            Route::Ssh(route) => Some(route.remote_cmd.as_str()),
            Route::Command(route) => Some(route.remote_cmd.as_str()),
            Route::Local => None,
        }
    }

    /// The first login as one line for a pane's shell: ssh may ask for a password, a code or
    /// a host key, and prints the app's version there.
    pub fn first_login_command(&self) -> Result<String> {
        let route = self.ssh_route()?;
        let argv = ssh_command::ssh_argv_interactive(&route.ssh, &route.target, &route.remote_cmd)?;
        Ok(ssh_command::popup_command(&argv)?)
    }

    fn ssh_route(&self) -> Result<&SshRoute> {
        match &self.route {
            Route::Ssh(route) => Ok(route),
            Route::Command(_) => {
                anyhow::bail!("{} is reached through a custom transport, not ssh", self.name)
            }
            Route::Local => anyhow::bail!("this computer is not reached over ssh"),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name to show for a remote machine; nothing for this one.
    pub fn host(&self) -> Option<&str> {
        self.is_remote().then_some(self.name.as_str())
    }
}

/// What one ssh connection's stderr said and how it exited, collected as it ends (shown
/// instead of a bare "connection closed", and how a failure is told apart). Each connection
/// has its own.
struct SshSaid {
    stderr: tokio::task::JoinHandle<String>,
    exit: tokio::sync::oneshot::Receiver<Option<i32>>,
}

impl SshSaid {
    /// How long to wait for ssh to finish once the connection has failed.
    const PATIENCE: std::time::Duration = std::time::Duration::from_secs(1);

    /// Waits for ssh to exit on its own, so a failure it ended is seen as its own status and not
    /// as the kill the caller gives it next.
    async fn collect(self) -> (Option<i32>, String) {
        let code = tokio::time::timeout(Self::PATIENCE, self.exit)
            .await
            .ok()
            .and_then(Result::ok)
            .flatten();
        let said = tokio::time::timeout(Self::PATIENCE, self.stderr)
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or_default();
        (code, said)
    }
}

type Spawned = (tokio::process::ChildStdout, Writer, SshSaid);

/// `ssh [hardening] host -- <app> proxy`: its stdout still unread, its stdin as the writer.
fn spawn_ssh(route: &SshRoute) -> Result<Spawned> {
    let argv = ssh_command::ssh_argv(&route.ssh, &route.target, &route.remote_cmd, route.proxy)?;
    let failed = format!("running {} (is OpenSSH installed?)", argv[0]);
    spawn_piped(&argv, false, &failed)
}

/// The user's `via` program with the remote command, on plain pipes.
fn spawn_command(route: &CommandRoute, name: &str) -> Result<Spawned> {
    refuse_custom_transport_on(cfg!(windows))?;
    let argv = ssh_command::command_argv(&route.via, &route.remote_cmd, route.proxy)?;
    let failed = format!(
        "running the custom transport {} for {name}",
        crate::link_failure::clean(&argv[0])
    );
    spawn_piped(&argv, true, &failed)
}

/// A custom transport runs a program this app does not control, so it is a unix feature in v1.
fn refuse_custom_transport_on(windows: bool) -> Result<()> {
    anyhow::ensure!(!windows, "{}", maitty_settings::config::VIA_NOT_ON_WINDOWS);
    Ok(())
}

/// `argv` with stdin, stdout and stderr piped (never a terminal). `own_session` detaches it
/// from the controlling terminal, so a program that would prompt cannot read the user's.
fn spawn_piped(argv: &[String], own_session: bool, failed: &str) -> Result<Spawned> {
    let mut cmd = tokio::process::Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    if own_session {
        // SAFETY: setsid is async-signal-safe and touches no memory the child shares. It
        // leaves the controlling terminal behind, so the child cannot prompt on it.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    #[cfg(not(unix))]
    let _ = own_session;
    let mut child = cmd.spawn().with_context(|| failed.to_string())?;
    let stdin = child.stdin.take().context("ssh stdin")?;
    let stdout = child.stdout.take().context("ssh stdout")?;
    let stderr = match child.stderr.take() {
        Some(mut err) => tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            let _ = err.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).trim().to_string()
        }),
        None => tokio::spawn(async { String::new() }),
    };
    let (kill, exit) = reap_in_background(child);
    // ssh lives as long as the connection's writing half.
    Ok((
        stdout,
        writer(SshIn { stdin, _kill: kill }),
        SshSaid { stderr, exit },
    ))
}

/// A task that owns ssh until it exits, and kills it the moment the returned sender is dropped
/// or sent to. Nobody awaits the child's end: ending a connection never waits on ssh.
fn reap_in_background(
    mut child: tokio::process::Child,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::sync::oneshot::Receiver<Option<i32>>,
) {
    let (kill, killed) = tokio::sync::oneshot::channel::<()>();
    let (exited, exit) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let status = tokio::select! {
            status = child.wait() => status,
            _ = killed => {
                let _ = child.start_kill();
                child.wait().await
            }
        };
        let _ = exited.send(status.ok().and_then(|s| s.code()));
    });
    (kill, exit)
}

/// The remote's login printed text and `proxy` never announced itself, so the stream can't be
/// framed. The hint is there because a remote from before the marker looks the same.
#[derive(Debug)]
struct LoginText;

impl std::fmt::Display for LoginText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the remote login printed text before {APP_COMMAND} started (or it runs an older {APP_COMMAND}: update it)"
        )
    }
}

impl std::error::Error for LoginText {}

/// Sends the Hello, then skips the login text ahead of the remote's first frame. The Hello
/// goes first because a remote from before the marker only speaks once it has one.
///
/// A remote that has already ended (its login printed text and its shell exited) closes its end
/// before the Hello is written, and writing to it fails with a broken pipe. That failure is not
/// what went wrong: what the remote said is. So the text is read even when the Hello could not
/// be sent, and only a stream that says nothing useful is reported as the failed write.
async fn greet_ssh<R: tokio::io::AsyncRead + Send + Unpin + 'static>(
    stdout: R,
    w: &mut Writer,
    hello: &ClientMsg,
) -> Result<Reader> {
    let sent = send(w, hello).await;
    match skip_login_noise(stdout, LIMITS).await {
        Ok(resynced) => {
            sent?;
            Ok(reader(resynced))
        }
        Err(NoMarker::Text) => Err(LoginText.into()),
        Err(e) => {
            sent?;
            Err(e.into())
        }
    }
}

/// ssh's stdin, holding ssh's kill switch: dropping the connection ends ssh, so nothing is
/// left waiting on its pipes.
struct SshIn {
    stdin: tokio::process::ChildStdin,
    _kill: tokio::sync::oneshot::Sender<()>,
}

impl tokio::io::AsyncWrite for SshIn {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.stdin).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stdin).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stdin).poll_shutdown(cx)
    }
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

/// Connect to `machine` and complete the handshake.
pub async fn open_to(machine: &Machine, attach: bool) -> Result<(Reader, Writer)> {
    let structured = structured_identity(STRUCTURED_ENV.map(std::env::var_os))?;
    let from = (!attach && structured.is_none())
        .then(|| {
            pane_claim(
                machine,
                std::env::var(concat!(env!("APP_ENV"), "_TERM_ID")).ok(),
                std::env::var(concat!(env!("APP_ENV"), "_PANE_TOKEN")).ok(),
            )
        })
        .flatten();
    let hello = ClientMsg::Hello {
        version: protocol::PROTOCOL_VERSION,
        attach,
        from,
        structured,
        surface: protocol::Surface::Terminal,
        replays: true,
    };
    let (mut r, w, said) = match &machine.route {
        Route::Local => {
            let (r, mut w) = framed(connect().await?);
            send(&mut w, &hello).await?;
            (r, w, None)
        }
        Route::Ssh(route) => {
            let (stdout, mut w, said) = spawn_ssh(route)?;
            match greet_ssh(stdout, &mut w, &hello).await {
                Ok(r) => (r, w, Some(said)),
                Err(e) => return Err(ssh_failed(machine, said, e, false).await),
            }
        }
        Route::Command(route) => {
            let (stdout, mut w, said) = spawn_command(route, machine.name())?;
            match greet_ssh(stdout, &mut w, &hello).await {
                Ok(r) => (r, w, Some(said)),
                Err(e) => return Err(ssh_failed(machine, said, e, false).await),
            }
        }
    };
    let version = match maitty_ipc_transport::welcome_version(&mut r).await {
        Ok(version) => version,
        Err(e) => {
            return Err(match said {
                Some(said) => ssh_failed(machine, said, e, true).await,
                None => e,
            });
        }
    };
    if version == protocol::PROTOCOL_VERSION {
        Ok((r, w))
    } else {
        Err(OtherVersion { daemon: version }.into())
    }
}

/// A command run inside a pane says which (with the pane's secret): what it may do is that
/// pane's to say. The app's own window is you. The pane belongs to this machine's daemon,
/// so another machine, which is untrusted, never gets its id or secret.
fn pane_claim(
    machine: &Machine,
    term: Option<String>,
    token: Option<String>,
) -> Option<(protocol::TermId, String)> {
    if machine.is_remote() {
        return None;
    }
    Some((term?.parse().ok()?, token.unwrap_or_default()))
}

/// `e` with what ssh said and how it exited, unless `e` already says why. `streamed` is
/// whether the far side had spoken before the failure. The connection is still held while ssh
/// is given a moment to exit; it is dropped (and ssh killed) when the caller returns.
async fn ssh_failed(
    machine: &Machine,
    said: SshSaid,
    e: anyhow::Error,
    streamed: bool,
) -> anyhow::Error {
    if e.is::<LoginText>() {
        return e;
    }
    let (code, said) = said.collect().await;
    let program = machine.transport_program();
    let message = match &program {
        Some(program) => command_failure(machine.name(), program, &said),
        None => ssh_failure(machine.name(), &said),
    };
    e.context(SshEnd {
        code,
        message,
        said,
        streamed,
        custom: program.is_some(),
    })
}

/// Why a custom transport to `host` failed, from what it said (which is not ssh's to read).
fn command_failure(host: &str, program: &str, said: &str) -> String {
    match crate::link_failure::clean(said).as_str() {
        "" => format!("couldn't reach {APP_COMMAND} on {host} through {program}"),
        said => format!("{host}: {program}: {said}"),
    }
}

/// Why an ssh connection to `host` failed, from what ssh said.
fn ssh_failure(host: &str, said: &str) -> String {
    if said.contains("not found") || said.contains("not recognized") {
        format!(
            "{host} has no `{APP_COMMAND}` on its PATH for ssh; install it there, or set {APP_ENV}_REMOTE_CMD to its full path ({said})"
        )
    } else if said.is_empty() {
        format!("couldn't reach {APP_COMMAND} on {host} over ssh")
    } else {
        format!("ssh {host}: {said}")
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
    use super::{APP_COMMAND, MachineDef};

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
    fn configured(name: &str) -> MachineDef {
        MachineDef {
            name: name.into(),
            target: "me@10.0.0.2".into(),
            ssh_args: vec!["-p".into(), "2222".into()],
            ..MachineDef::default()
        }
    }

    #[test]
    fn a_configured_machine_runs_ssh_with_its_args_and_the_default_command() {
        let m = super::Machine::from_def(&configured("box"), "ssh");
        let want = super::Machine::ssh(
            "me@10.0.0.2",
            vec!["ssh".into(), "-p".into(), "2222".into()],
            APP_COMMAND,
        )
        .attach_only();
        assert_eq!((m.name(), m.host()), ("box", Some("box")));
        assert_eq!(m.route, want.route);
    }

    #[test]
    fn a_configured_machine_uses_its_own_remote_command() {
        let mut def = configured("box");
        def.remote_cmd = Some("~/bin/maitty".into());
        let m = super::Machine::from_def(&def, "ssh");
        let want = super::Machine::ssh(
            "me@10.0.0.2",
            vec!["ssh".into(), "-p".into(), "2222".into()],
            "~/bin/maitty",
        )
        .attach_only();
        assert_eq!(m.route, want.route);
    }

    #[test]
    fn remote_host_is_a_machine_named_after_the_host() {
        let m = super::Machine::adhoc("box", Some("ssh -p 2222"), Some("~/maitty".into()));
        let want = super::Machine::ssh(
            "box",
            vec!["ssh".into(), "-p".into(), "2222".into()],
            "~/maitty",
        );
        assert_eq!((m.name(), m.host()), ("box", Some("box")));
        assert_eq!(m, want);
    }

    #[test]
    fn remote_host_defaults_to_plain_ssh_and_the_app_command() {
        for ssh in [None, Some(""), Some("  ")] {
            let m = super::Machine::adhoc("box", ssh, None);
            assert_eq!(
                m,
                super::Machine::ssh("box", vec!["ssh".into()], APP_COMMAND)
            );
        }
    }

    #[test]
    fn only_a_remote_machine_has_a_host_to_show() {
        let local = super::Machine::local();
        assert!(!local.is_remote() && local.host().is_none());
        let box_ = super::Machine::ssh("box", vec!["ssh".into()], "maitty");
        assert!(box_.is_remote());
        assert_eq!((box_.host(), box_.name()), (Some("box"), "box"));
    }

    #[test]
    fn an_ssh_failure_is_told_from_what_ssh_said() {
        let missing = super::ssh_failure("box", "bash: maitty: command not found");
        assert!(missing.starts_with("box has no `"), "{missing}");
        assert!(
            missing.ends_with("(bash: maitty: command not found)"),
            "the words stay: {missing}"
        );
        assert_eq!(
            super::ssh_failure("box", "box: Permission denied (publickey)."),
            "ssh box: box: Permission denied (publickey)."
        );
        assert!(super::ssh_failure("box", "").starts_with("couldn't reach "));
    }

    #[tokio::test]
    async fn login_text_is_reported_as_it_is_and_other_failures_get_what_ssh_said() {
        let said = || {
            let (exited, exit) = tokio::sync::oneshot::channel();
            let _ = exited.send(Some(255));
            super::SshSaid {
                stderr: tokio::spawn(async { "box: Permission denied".to_string() }),
                exit,
            }
        };
        let machine = super::Machine::ssh("box", vec!["ssh".into()], APP_COMMAND);

        let text = super::ssh_failed(&machine, said(), super::LoginText.into(), false).await;
        let text = format!("{text:#}");
        assert!(
            text.contains("the remote login printed text before"),
            "{text}"
        );
        assert!(!text.contains("Permission denied"), "{text}");

        let other = super::ssh_failed(&machine, said(), anyhow::anyhow!("closed"), false).await;
        assert!(
            format!("{other:#}").contains("Permission denied"),
            "{other:#}"
        );
    }

    #[test]
    fn starting_flips_the_proxy_of_both_routes_and_nothing_else() {
        let proxy = |m: &super::Machine| match &m.route {
            super::Route::Ssh(route) => route.proxy,
            super::Route::Command(route) => route.proxy,
            super::Route::Local => panic!("a local machine has no proxy"),
        };
        for attached in [configured_machine(), through(&["relay", "{command}"])] {
            let starting = attached.starting();
            assert_eq!(proxy(&attached), super::ProxyMode::AttachOnly);
            assert_eq!(proxy(&starting), super::ProxyMode::StartsDaemon);
            assert_eq!(starting.name(), attached.name());
            assert_eq!(starting.host(), attached.host());
            assert_eq!(starting.transport_program(), attached.transport_program());
            let mut back = starting.clone();
            match &mut back.route {
                super::Route::Ssh(r) => r.proxy = super::ProxyMode::AttachOnly,
                super::Route::Command(r) => r.proxy = super::ProxyMode::AttachOnly,
                super::Route::Local => {}
            }
            assert_eq!(back, attached);
        }
        assert_eq!(super::Machine::local().starting(), super::Machine::local());
    }

    #[test]
    fn the_remote_command_asks_not_to_start_the_daemon_only_when_attaching() {
        let remote_command = |m: &super::Machine| match &m.route {
            super::Route::Ssh(r) => {
                super::ssh_command::ssh_argv(&r.ssh, &r.target, &r.remote_cmd, r.proxy)
            }
            super::Route::Command(r) => {
                super::ssh_command::command_argv(&r.via, &r.remote_cmd, r.proxy)
            }
            super::Route::Local => panic!("a local machine has no proxy"),
        }
        .unwrap()
        .pop()
        .unwrap();
        for attached in [configured_machine(), through(&["relay", "{command}"])] {
            assert!(remote_command(&attached).ends_with(" proxy --attach-only"));
            assert!(remote_command(&attached.starting()).ends_with(" proxy"));
        }
    }

    #[test]
    fn a_configured_machine_attaches_only_and_a_remote_flag_machine_starts_the_daemon() {
        let configured = super::Machine::from_def(&configured("box"), "ssh");
        let adhoc = super::Machine::adhoc("box", None, None);
        let proxy = |m: &super::Machine| match &m.route {
            super::Route::Ssh(route) => route.proxy,
            super::Route::Command(route) => route.proxy,
            super::Route::Local => panic!("a local machine has no proxy"),
        };
        assert_eq!(proxy(&configured), super::ProxyMode::AttachOnly);
        assert_eq!(proxy(&adhoc), super::ProxyMode::StartsDaemon);
    }

    fn through(via: &[&str]) -> super::Machine {
        let def = MachineDef {
            via: Some(via.iter().map(|w| w.to_string()).collect()),
            ..configured("fly")
        };
        super::Machine::from_def(&def, "ssh")
    }

    #[test]
    fn a_via_machine_is_remote_attaches_only_and_shows_only_its_program() {
        let m = through(&["/opt/relay", "--token", "s3cret", "{command}"]);
        assert!(m.is_remote());
        assert_eq!(m.transport_program().as_deref(), Some("/opt/relay"));
        assert_eq!(m.host(), Some("fly"));
        assert!(matches!(&m.route, super::Route::Command(r) if r.proxy == super::ProxyMode::AttachOnly));
        assert_eq!(super::Machine::local().transport_program(), None);
        assert_eq!(configured_machine().transport_program(), None);
    }

    fn configured_machine() -> super::Machine {
        super::Machine::from_def(&configured("box"), "ssh")
    }

    #[test]
    fn the_ssh_only_lines_are_refused_for_a_via_machine() {
        let m = through(&["relay"]);
        let words = |e: anyhow::Error| e.to_string();
        assert!(words(m.probe_argv().unwrap_err()).contains("custom transport"));
        assert!(words(m.platform_argv().unwrap_err()).contains("custom transport"));
        assert!(words(m.where_argv().unwrap_err()).contains("custom transport"));
        assert!(words(m.install_argv("/x", 1).unwrap_err()).contains("custom transport"));
        assert!(words(m.first_login_command().unwrap_err()).contains("custom transport"));
        assert_eq!(m.remote_cmd(), Some(APP_COMMAND));
    }

    #[test]
    fn a_custom_transport_is_refused_on_windows_with_the_clear_message() {
        let err = super::refuse_custom_transport_on(true).unwrap_err().to_string();
        assert!(err.contains("not supported on Windows in v1") && err.contains("use ssh"), "{err}");
        super::refuse_custom_transport_on(false).unwrap();
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn connecting_to_a_via_machine_on_windows_shows_the_message_and_spawns_nothing() {
        let err = super::open_to(&through(&["relay.exe", "{command}"]), true)
            .await
            .err()
            .expect("refused");
        assert!(format!("{err:#}").contains("not supported on Windows in v1"), "{err:#}");
        assert!(matches!(
            crate::link_failure::classify(&err),
            crate::link_failure::FailureClass::Unreachable(why) if why.contains("use ssh")
        ));
    }

    #[tokio::test]
    async fn a_via_that_says_permission_denied_and_exits_255_is_unreachable_not_needs_login() {
        let machine = through(&["relay"]);
        let (exited, exit) = tokio::sync::oneshot::channel();
        let _ = exited.send(Some(255));
        let said = super::SshSaid {
            stderr: tokio::spawn(async { "relay: Permission denied \u{1b}[31m(token)".to_string() }),
            exit,
        };
        let error = super::ssh_failed(&machine, said, anyhow::anyhow!("closed"), false).await;
        let class = crate::link_failure::classify(&error);
        let crate::link_failure::FailureClass::Unreachable(why) = class else {
            panic!("not unreachable: {class:?}");
        };
        assert!(why.contains("Permission denied") && !why.chars().any(char::is_control), "{why:?}");
        assert!(format!("{error}").starts_with("fly: relay: "), "{error}");
    }

    #[tokio::test]
    async fn a_via_that_exits_127_needs_the_app_installed() {
        let machine = through(&["relay"]);
        let (exited, exit) = tokio::sync::oneshot::channel();
        let _ = exited.send(Some(127));
        let said = super::SshSaid {
            stderr: tokio::spawn(async { String::new() }),
            exit,
        };
        let error = super::ssh_failed(&machine, said, anyhow::anyhow!("closed"), false).await;
        assert_eq!(
            crate::link_failure::classify(&error),
            crate::link_failure::FailureClass::Install
        );
    }

    #[tokio::test]
    async fn the_failure_carries_ssh_s_exit_status_for_the_supervisor() {
        let machine = super::Machine::ssh("box", vec!["ssh".into()], APP_COMMAND);
        let (exited, exit) = tokio::sync::oneshot::channel();
        let _ = exited.send(Some(255));
        let said = super::SshSaid {
            stderr: tokio::spawn(async { "box: Permission denied".to_string() }),
            exit,
        };
        let error = super::ssh_failed(&machine, said, anyhow::anyhow!("closed"), false).await;
        assert_eq!(
            crate::link_failure::classify(&error),
            crate::link_failure::FailureClass::Login
        );
    }

    #[test]
    fn only_this_machines_daemon_is_told_which_pane_is_asking() {
        let claim =
            |m: &super::Machine| super::pane_claim(m, Some("7".into()), Some("secret".into()));
        assert_eq!(claim(&super::Machine::local()), Some((7, "secret".into())));
        let remote = super::Machine::ssh("box", vec!["ssh".into()], "maitty");
        assert_eq!(claim(&remote), None);
        assert_eq!(
            super::pane_claim(&super::Machine::local(), None, None),
            None
        );
    }

    #[test]
    fn a_bad_message_is_a_decode_error() {
        let e = crate::protocol::decode::<crate::protocol::ServerMsg>(&[0xc1, 0xff]).unwrap_err();
        assert!(super::is_decode_error(&e));
        assert!(!super::is_decode_error(&anyhow::anyhow!(
            "connection reset"
        )));
    }

    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::AsyncWrite;

    /// A stand-in for ssh's stdin once ssh has ended.
    struct Ended;

    impl AsyncWrite for Ended {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    fn ended_writer() -> super::Writer {
        tokio_util::codec::FramedWrite::new(
            Box::new(Ended),
            tokio_util::codec::LengthDelimitedCodec::new(),
        )
    }

    fn hello_msg() -> super::ClientMsg {
        super::ClientMsg::Hello {
            version: super::protocol::PROTOCOL_VERSION,
            attach: true,
            from: None,
            structured: None,
            surface: super::protocol::Surface::Desktop,
            replays: false,
        }
    }

    #[tokio::test]
    async fn a_remote_that_ended_after_printing_text_is_reported_by_what_it_printed() {
        let text: &'static [u8] = b"Last login: Mon\nthis account is currently not available\n";
        let error = super::greet_ssh(text, &mut ended_writer(), &hello_msg())
            .await
            .err()
            .expect("it fails");
        assert!(error.is::<super::LoginText>(), "{error:#}");
    }

    #[tokio::test]
    async fn a_remote_that_ended_saying_nothing_is_reported_as_the_failed_write() {
        let nothing: &'static [u8] = b"";
        let error = super::greet_ssh(nothing, &mut ended_writer(), &hello_msg())
            .await
            .err()
            .expect("it fails");
        assert!(!error.is::<super::LoginText>());
        assert!(
            format!("{error:#}").to_lowercase().contains("pipe"),
            "{error:#}"
        );
    }
}
