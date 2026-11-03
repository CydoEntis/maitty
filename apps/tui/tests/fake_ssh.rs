//! The fake-ssh harness (`examples/fake-ssh.rs`): `MAITTY_SSH` points at it, so the whole
//! remote path (spawn, framing, handshake, failure) runs against a real daemon on a private
//! socket with no ssh and no server. Each test gets its own socket, data folder and daemon.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const MAITTY: &str = env!("CARGO_BIN_EXE_maitty");
const WAIT: Duration = Duration::from_secs(60);

/// A private daemon's worth of environment: its own socket, data folder and config.
struct Sandbox {
    dir: PathBuf,
    socket: String,
}

impl Sandbox {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let socket = format!("fakessh-{}-{n}", std::process::id());
        let dir = std::env::temp_dir().join(&socket);
        std::fs::create_dir_all(dir.join("data")).expect("create the sandbox");
        Sandbox { dir, socket }
    }

    fn data(&self) -> PathBuf {
        self.dir.join("data")
    }

    fn log(&self) -> PathBuf {
        self.dir.join("ssh-args.txt")
    }

    /// Where the shim appends a line per call (`--calls=`).
    fn calls(&self) -> PathBuf {
        self.dir.join("ssh-calls.txt")
    }

    /// `program` with this sandbox's socket, data folder and config, and no remote.
    fn local(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(program);
        command
            .env("MAITTY_SOCKET", &self.socket)
            .env("MAITTY_DATA_DIR", self.data())
            .env("MAITTY_CONFIG", self.dir.join("config.toml"))
            .env_remove("MAITTY_REMOTE")
            .env_remove("MAITTY_REMOTE_CMD")
            // Run inside a maitty pane, the suite would speak as that pane and be refused.
            .env_remove("MAITTY_TERM_ID")
            .env_remove("MAITTY_PANE_TOKEN");
        command
    }

    /// `program` as above, with the fake ssh in `mode` as `MAITTY_SSH`.
    fn command(&self, program: impl AsRef<std::ffi::OsStr>, mode: &str) -> Command {
        let mut command = self.local(program);
        command
            .env("MAITTY_SSH", fake_ssh())
            .env("MAITTY_FAKE_SSH_MAITTY", MAITTY)
            .env("MAITTY_FAKE_SSH_MODE", mode)
            .env("MAITTY_FAKE_SSH_LOG", self.log());
        command
    }

    /// `maitty --remote box <args>` through the fake ssh in `mode`.
    fn remote(&self, mode: &str, args: &[&str]) -> Output {
        let mut command = self.command(MAITTY, mode);
        command.args(["--remote", "box"]).args(args);
        output_within(command, WAIT)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // Must not panic: a test that failed is already unwinding, and a second panic aborts.
        let _ = self
            .local(MAITTY)
            .arg("kill-server")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// `target/<profile>/examples/fake-ssh`, built here: `cargo test --test <name>` does not
/// build examples.
fn fake_ssh() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let exe = std::env::current_exe().expect("the test's own path");
            let profile = exe
                .parent()
                .and_then(Path::parent)
                .expect("target/<profile>");
            let path = profile
                .join("examples")
                .join(format!("fake-ssh{}", std::env::consts::EXE_SUFFIX));
            // Always ask cargo: it is a no-op when current, and `cargo test --test` would
            // otherwise leave a stale shim from an earlier build.
            let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let built = Command::new(cargo)
                .args(["build", "--package", "maitty", "--example", "fake-ssh"])
                .status()
                .expect("run cargo");
            assert!(built.success(), "building the fake-ssh example failed");
            path
        })
        .clone()
}

/// Runs to completion, or is killed once `limit` passes (a hung connection must fail the
/// test, not stall the suite). Output goes to files, not pipes: a daemon started by the
/// command inherits its pipes on Windows, and reading them would wait for the daemon to exit.
fn output_within(mut command: Command, limit: Duration) -> Output {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!("fakessh-out-{}-{n}", std::process::id()));
    let (out_path, err_path) = (base.with_extension("out"), base.with_extension("err"));
    let mut child = command
        .stdin(Stdio::null())
        .stdout(std::fs::File::create(&out_path).expect("create the stdout file"))
        .stderr(std::fs::File::create(&err_path).expect("create the stderr file"))
        .spawn()
        .expect("start the process");
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll the process") {
            break Some(status);
        }
        if start.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let read = |path: &Path| {
        let bytes = std::fs::read(path).unwrap_or_default();
        let _ = std::fs::remove_file(path);
        bytes
    };
    let (stdout, stderr) = (read(&out_path), read(&err_path));
    let Some(status) = status else {
        panic!(
            "still running after {limit:?}:
{}",
            text(&stderr)
        );
    };
    Output {
        status,
        stdout,
        stderr,
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_client_completes_the_handshake_through_the_shim() {
    let sandbox = Sandbox::new();
    let out = sandbox.remote("proxy", &["ls", "--json"]);
    assert!(
        out.status.success(),
        "ls through the shim failed:\n{}",
        text(&out.stderr)
    );
    let reply = text(&out.stdout);
    assert!(
        reply.trim_start().starts_with(['[', '{']),
        "ls --json printed {reply:?}"
    );
}

#[test]
fn the_shim_is_called_the_way_the_client_calls_ssh() {
    let sandbox = Sandbox::new();
    let out = sandbox.remote("proxy", &["ls", "--json"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let log = std::fs::read_to_string(sandbox.log()).expect("the shim logs its arguments");
    let args: Vec<&str> = log.lines().collect();
    let dashes = args
        .iter()
        .position(|a| *a == "--")
        .expect("`--` ends ssh's options");
    assert_eq!(args[dashes + 1], "box", "the host follows `--`: {args:?}");
    assert_eq!(
        args[dashes + 2],
        "'maitty' proxy",
        "one quoted remote command: {args:?}"
    );
    assert_eq!(
        args.len(),
        dashes + 3,
        "nothing after the remote command: {args:?}"
    );
    let options = args[..dashes].join(" ");
    for want in [
        "-o BatchMode=yes",
        "-o ForwardAgent=no",
        "-o ConnectTimeout=",
        "-o ServerAliveInterval=",
        "-o ServerAliveCountMax=",
        "-a",
        "-T",
    ] {
        assert!(
            options.contains(want),
            "{want} is passed before `--`: {args:?}"
        );
    }
}

#[test]
fn a_host_that_is_an_ssh_option_never_reaches_ssh() {
    let sandbox = Sandbox::new();
    let mut command = sandbox.command(MAITTY, "proxy");
    command
        .env("MAITTY_REMOTE", "-oProxyCommand=touch pwned")
        .args(["ls", "--json"]);
    let out = output_within(command, WAIT);
    assert!(
        !out.status.success(),
        "a host that is an option must be refused"
    );
    assert!(
        text(&out.stderr).contains("starts with `-`"),
        "stderr was:
{}",
        text(&out.stderr)
    );
    assert!(!sandbox.log().exists(), "ssh must not have been run");
}

/// The Hello the remote daemon is sent, as the client (run inside a pane of this computer)
/// makes it. The pane's id and secret are on this one child, not on the test's environment.
fn hello_sent_to_a_remote_from_a_pane(sandbox: &Sandbox) -> runtime::protocol::ClientMsg {
    let hello = sandbox.dir.join("hello.bin");
    let mut command = sandbox.command(MAITTY, "capture-hello");
    command
        .env("MAITTY_FAKE_SSH_HELLO", &hello)
        .env("MAITTY_TERM_ID", "7")
        .env("MAITTY_PANE_TOKEN", "local-pane-secret")
        .args(["--remote", "box", "ls", "--json"]);
    let _ = output_within(command, WAIT);
    let bytes = std::fs::read(&hello).expect("the shim captured the Hello");
    runtime::protocol::decode(&bytes).expect("a Hello")
}

#[test]
fn a_local_pane_token_never_reaches_another_machine_s_daemon() {
    let sandbox = Sandbox::new();
    let hello = hello_sent_to_a_remote_from_a_pane(&sandbox);
    let runtime::protocol::ClientMsg::Hello { from, attach, .. } = hello else {
        panic!("the first frame is a Hello, not {hello:?}");
    };
    assert!(!attach, "the CLI is not an attached client");
    assert_eq!(from, None, "the remote daemon is told no pane id or token");
}

/// `maitty <args>` in this sandbox, which configures no machine, with the fake ssh in `proxy`.
fn usage(sandbox: &Sandbox, args: &[&str]) -> Output {
    let mut command = sandbox.command(MAITTY, "proxy");
    command.args(args);
    output_within(command, WAIT)
}

#[test]
fn an_unknown_machine_is_a_usage_error_that_says_how_to_add_one() {
    let sandbox = Sandbox::new();
    for args in [&["--on", "bild", "ls"][..], &["read", "bild/4"]] {
        let out = usage(&sandbox, args);
        let said = text(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {said}");
        assert!(
            said.contains(r#"unknown machine "bild"; no machines are configured"#)
                && said.contains("[[machines]] in config.local.toml"),
            "{args:?}: {said}"
        );
    }
    assert!(!sandbox.log().exists(), "ssh must not have been run");
}

#[test]
fn machine_update_needs_a_configured_machine_and_never_runs_ssh_for_anything_else() {
    let sandbox = Sandbox::new();
    for (name, want) in [
        (
            "bild",
            r#"unknown machine "bild"; no machines are configured"#,
        ),
        ("local", "this computer is not reached over ssh"),
    ] {
        let out = usage(&sandbox, &["machine", "update", name, "--install"]);
        let said = text(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{name}: {said}");
        assert!(said.contains(want), "{name}: {said}");
    }
    assert!(!sandbox.log().exists(), "ssh must not have been run");
}

#[test]
fn restart_if_idle_with_no_server_says_so_and_starts_nothing() {
    let sandbox = Sandbox::new();
    let mut command = sandbox.local(MAITTY);
    command.arg("restart-if-idle");
    let out = output_within(command, WAIT);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout).trim(), "no server is running");
}

#[test]
fn on_with_remote_or_with_a_command_that_ignores_it_is_a_usage_error() {
    let sandbox = Sandbox::new();
    let both = usage(&sandbox, &["--on", "build", "--remote", "box", "ls"]);
    assert_eq!(both.status.code(), Some(2));
    assert!(
        text(&both.stderr).contains("--on and --remote"),
        "{}",
        text(&both.stderr)
    );
    let ignored = usage(&sandbox, &["--on", "build", "worktree", "x"]);
    assert_eq!(ignored.status.code(), Some(2));
    assert!(
        text(&ignored.stderr).contains("--on works with"),
        "{}",
        text(&ignored.stderr)
    );
    let all = usage(&sandbox, &["ls", "--all", "--remote", "box"]);
    assert_eq!(all.status.code(), Some(2));
    assert!(!sandbox.log().exists(), "ssh must not have been run");
}

#[test]
fn remote_keeps_listing_panes_by_number_with_no_machine_tag() {
    let sandbox = Sandbox::new();
    let made = sandbox.remote("proxy", &["new", "--name", "adhoc"]);
    assert!(made.status.success(), "{}", text(&made.stderr));
    let listed = sandbox.remote("proxy", &["ls"]);
    let out = text(&listed.stdout);
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    assert!(!out.contains("machine "), "ls --remote is as it was: {out}");
    let pane = out.split("pane ").nth(1).expect("a pane line");
    assert!(pane.starts_with(|c: char| c.is_ascii_digit()), "{out}");
    let json = text(&sandbox.remote("proxy", &["ls", "--json"]).stdout);
    assert!(!json.contains("\"machine\""), "{json}");
}

#[test]
fn the_auth_failure_shim_exits_255_with_ssh_s_words() {
    let sandbox = Sandbox::new();
    let out = output_within(sandbox.command(fake_ssh(), "auth-fail"), WAIT);
    assert_eq!(
        out.status.code(),
        Some(255),
        "ssh exits 255 when it cannot connect"
    );
    assert!(
        out.stdout.is_empty(),
        "nothing on stdout: the stream is not framed"
    );
    assert!(
        text(&out.stderr).contains("Permission denied"),
        "stderr was:
{}",
        text(&out.stderr)
    );
}

#[test]
fn a_daemon_of_another_protocol_version_is_refused() {
    let sandbox = Sandbox::new();
    let out = sandbox.remote("mismatch", &["ls"]);
    assert!(!out.status.success());
    assert!(
        text(&out.stderr).contains("version"),
        "stderr was:\n{}",
        text(&out.stderr)
    );
}

#[test]
fn login_noise_comes_out_of_the_shim_ahead_of_the_first_frame() {
    let sandbox = Sandbox::new();
    let out = output_within(
        sandbox.command(fake_ssh(), "noise"),
        Duration::from_secs(30),
    );
    assert!(
        text(&out.stdout).starts_with("Last login:"),
        "stdout started {:?}",
        text(&out.stdout)
    );
}

#[test]
fn a_silent_shim_keeps_the_line_open_without_a_word() {
    let sandbox = Sandbox::new();
    let mut child = sandbox
        .command(fake_ssh(), "hang")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start the shim");
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        child.try_wait().expect("poll the shim").is_none(),
        "a hanging shim must stay running"
    );
    child.kill().expect("stop the shim");
    let _ = child.wait();
}

#[test]
fn a_failed_remote_command_says_what_ssh_said() {
    let sandbox = Sandbox::new();
    let out = sandbox.remote("auth-fail", &["ls"]);
    assert!(!out.status.success());
    assert!(
        text(&out.stderr).contains("Permission denied"),
        "stderr was:
{}",
        text(&out.stderr)
    );
}

#[test]
fn login_noise_before_the_marker_is_skipped_by_the_cli() {
    let sandbox = Sandbox::new();
    let out = sandbox.remote("noise", &["ls", "--json"]);
    assert!(
        out.status.success(),
        "ls behind a noisy login failed:
{}",
        text(&out.stderr)
    );
    assert!(
        text(&out.stdout).trim_start().starts_with(['[', '{']),
        "ls --json printed {:?}",
        text(&out.stdout)
    );
}

#[test]
fn login_noise_before_the_marker_is_skipped_in_process() {
    let sandbox = Sandbox::new();
    let runtime = runtime();
    let opened = runtime.block_on(runtime::ipc::open_to(
        &machine_in_mode("noisy", &sandbox, "noise"),
        true,
    ));
    assert!(
        opened.is_ok(),
        "a noisy login still connects: {:?}",
        opened.err()
    );
    drop(opened);
    runtime.shutdown_background();
}

#[test]
fn text_with_no_marker_fails_with_the_login_error_instead_of_hanging() {
    for mode in ["noise-only", "noise-flood"] {
        let sandbox = Sandbox::new();
        let out = sandbox.remote(mode, &["ls"]);
        assert!(
            !out.status.success(),
            "{mode}: a stream with no marker must fail"
        );
        let said = text(&out.stderr);
        assert!(
            said.contains("the remote login printed text before maitty started"),
            "{mode}: stderr was:
{said}"
        );
        assert!(
            said.contains("update it"),
            "{mode}: the older-maitty hint is given:
{said}"
        );
    }
}

#[test]
fn a_remote_from_before_the_marker_still_connects() {
    let sandbox = Sandbox::new();
    let out = sandbox.remote("old", &["ls", "--json"]);
    assert!(
        out.status.success(),
        "an old remote failed:
{}",
        text(&out.stderr)
    );
    assert!(
        text(&out.stdout).trim_start().starts_with(['[', '{']),
        "ls --json printed {:?}",
        text(&out.stdout)
    );
}

/// One machine failing must say its own words, and must not touch another's connection.
#[test]
fn ssh_errors_belong_to_their_own_connection() {
    use runtime::ipc::{Machine, open_to};
    let sandbox = Sandbox::new();
    let shim = fake_ssh().to_string_lossy().into_owned();
    let good = Machine::ssh(
        "good",
        vec![
            shim.clone(),
            "--mode=proxy".into(),
            format!("--maitty={MAITTY}"),
            format!("--env=MAITTY_SOCKET={}", sandbox.socket),
            format!("--env=MAITTY_DATA_DIR={}", sandbox.data().display()),
            format!(
                "--env=MAITTY_CONFIG={}",
                sandbox.dir.join("config.toml").display()
            ),
        ],
        "maitty",
    );
    let bad = Machine::ssh("bad", vec![shim, "--mode=auth-fail".into()], "maitty");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    let (good_result, bad_result) =
        runtime.block_on(async { tokio::join!(open_to(&good, true), open_to(&bad, true)) });
    assert!(
        good_result.is_ok(),
        "the working machine connects: {:?}",
        good_result.err()
    );
    let said = format!("{:#}", bad_result.err().expect("the failing machine fails"));
    assert!(
        said.contains("ssh bad:") && said.contains("Permission denied"),
        "its own words: {said}"
    );
    assert!(
        !said.contains("good"),
        "nothing of the other connection: {said}"
    );
    // On Windows tokio reads a child's pipes on blocking threads, and the daemon the shim
    // started still holds them: do not wait for those reads (the CLI does the same).
    runtime.shutdown_background();
}

/// A machine whose daemon is `sandbox`'s, reached through the shim: several in one process.
fn machine(name: &str, sandbox: &Sandbox) -> runtime::ipc::Machine {
    machine_in_mode(name, sandbox, "proxy")
}

fn machine_in_mode(name: &str, sandbox: &Sandbox, mode: &str) -> runtime::ipc::Machine {
    runtime::ipc::Machine::ssh(
        name,
        vec![
            fake_ssh().to_string_lossy().into_owned(),
            format!("--mode={mode}"),
            format!("--maitty={MAITTY}"),
            format!("--env=MAITTY_SOCKET={}", sandbox.socket),
            format!("--env=MAITTY_DATA_DIR={}", sandbox.data().display()),
            format!(
                "--env=MAITTY_CONFIG={}",
                sandbox.dir.join("config.toml").display()
            ),
        ],
        "maitty",
    )
}

const THERE: runtime::connections::MachineId = runtime::connections::MachineId(1);

/// Two daemons in one registry, a pane opened on each; returns once each machine has
/// delivered a state with its pane and some output from it.
async fn two_machines_talking(
    here: &Sandbox,
    there: &Sandbox,
) -> runtime::connections::Connections {
    use runtime::connections::{Connections, MachineId, Outgoing};
    use runtime::ipc::open_to;
    use runtime::protocol::{ClientMsg, Command, ServerMsg};
    use std::collections::BTreeSet;

    let mut connections = Connections::default();
    for (id, sandbox, name) in [(MachineId::LOCAL, here, "here"), (THERE, there, "there")] {
        connections.add(
            id,
            open_to(&machine(name, sandbox), true)
                .await
                .expect("connect"),
        );
        let open = ClientMsg::Command(Command::NewWorkspace {
            cwd: None,
            name: None,
            cmd: None,
        });
        connections
            .send(Outgoing {
                machine: id,
                msg: open,
            })
            .expect("ask for a pane");
    }
    let (mut with_pane, mut with_output) = (BTreeSet::new(), BTreeSet::new());
    let heard = async {
        while with_pane.len() < 2 || with_output.len() < 2 {
            let burst = connections.recv_burst(512).await;
            assert!(!burst.lost, "machine {} was lost", burst.machine.0);
            for msg in burst.msgs {
                match msg {
                    ServerMsg::State(state) if !state.terms.is_empty() => {
                        with_pane.insert(burst.machine);
                    }
                    ServerMsg::Output { .. } | ServerMsg::Replay { .. } => {
                        with_output.insert(burst.machine);
                    }
                    _ => {}
                }
            }
        }
    };
    tokio::time::timeout(WAIT, heard)
        .await
        .expect("both machines deliver a pane and its output");
    connections
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
}

#[test]
fn two_daemons_deliver_through_one_registry() {
    let (here, there) = (Sandbox::new(), Sandbox::new());
    let runtime = runtime();
    let connections = runtime.block_on(two_machines_talking(&here, &there));
    assert!(
        connections.is_connected(runtime::connections::MachineId::LOCAL)
            && connections.is_connected(THERE)
    );
    drop(connections);
    runtime.shutdown_background();
}

#[test]
fn a_daemon_going_away_marks_only_its_machine_lost() {
    use runtime::connections::{Connections, MachineId, NotConnected, Outgoing};
    use runtime::protocol::{ClientMsg, Command, ServerMsg};

    let (here, there) = (Sandbox::new(), Sandbox::new());
    let runtime = runtime();
    let mut connections: Connections = runtime.block_on(two_machines_talking(&here, &there));
    let mut kill = there.local(MAITTY);
    kill.arg("kill-server");
    let stopped = output_within(kill, WAIT);
    assert!(stopped.status.success(), "{}", text(&stopped.stderr));

    runtime.block_on(async {
        let lost = async {
            loop {
                let burst = connections.recv_burst(512).await;
                if burst.lost {
                    break burst.machine;
                }
            }
        };
        let gone = tokio::time::timeout(WAIT, lost)
            .await
            .expect("the stopped daemon's machine is lost");
        assert_eq!(gone, THERE);
        assert!(!connections.is_connected(THERE) && connections.is_connected(MachineId::LOCAL));
        let msg = ClientMsg::Command(Command::NewWorkspace {
            cwd: None,
            name: None,
            cmd: None,
        });
        assert_eq!(
            connections.send(Outgoing {
                machine: THERE,
                msg: msg.clone()
            }),
            Err(NotConnected(THERE))
        );

        // The surviving machine still answers.
        connections
            .send(Outgoing {
                machine: MachineId::LOCAL,
                msg,
            })
            .expect("send to the live machine");
        let answered = async {
            loop {
                let burst = connections.recv_burst(512).await;
                assert_eq!(burst.machine, MachineId::LOCAL);
                assert!(!burst.lost, "the local machine must stay up");
                if burst
                    .msgs
                    .iter()
                    .any(|m| matches!(m, ServerMsg::State(s) if s.workspaces.len() >= 2))
                {
                    break;
                }
            }
        };
        tokio::time::timeout(WAIT, answered)
            .await
            .expect("the local machine keeps delivering");
    });
    drop(connections);
    runtime.shutdown_background();
}

/// What each machine has printed, as text: the bytes of its panes' output, in order.
#[derive(Default)]
struct Heard {
    here: String,
    there: String,
}

impl Heard {
    fn of(&mut self, machine: runtime::connections::MachineId) -> &mut String {
        if machine == THERE {
            &mut self.there
        } else {
            &mut self.here
        }
    }
}

/// Read both machines until `done` holds of what they have printed and the sizes they report.
async fn listen_until(
    connections: &mut runtime::connections::Connections,
    heard: &mut Heard,
    sizes: &mut [(u16, u16); 2],
    done: impl Fn(&Heard, &[(u16, u16); 2]) -> bool,
) {
    use runtime::protocol::ServerMsg;
    let listening = async {
        while !done(heard, sizes) {
            let burst = connections.recv_burst(512).await;
            assert!(!burst.lost, "machine {} was lost", burst.machine.0);
            let at = usize::from(burst.machine == THERE);
            for msg in burst.msgs {
                match msg {
                    ServerMsg::Output { data, .. } | ServerMsg::Replay { data, .. } => {
                        heard
                            .of(burst.machine)
                            .push_str(&String::from_utf8_lossy(&data));
                    }
                    ServerMsg::State(state) => {
                        if let Some(pane) = state.terms.get(&1) {
                            sizes[at] = (pane.cols, pane.rows);
                        }
                    }
                    ServerMsg::TermsChanged { changed, .. } => {
                        if let Some(pane) = changed.iter().find(|t| t.id == 1) {
                            sizes[at] = (pane.cols, pane.rows);
                        }
                    }
                    _ => {}
                }
            }
        }
    };
    tokio::time::timeout(WAIT, listening)
        .await
        .expect("each machine reports what was sent to it");
}

#[test]
fn typing_and_resizing_reach_only_the_machine_that_owns_the_pane() {
    use runtime::connections::{MachineId, Outgoing};
    use runtime::protocol::ClientMsg;

    const FOR_THERE: &str = "typed-for-the-build-box";
    const FOR_HERE: &str = "typed-for-this-computer";
    let (here, there) = (Sandbox::new(), Sandbox::new());
    let runtime = runtime();
    runtime.block_on(async {
        let mut connections = two_machines_talking(&here, &there).await;
        let (mut heard, mut sizes) = (Heard::default(), [(0, 0); 2]);
        let input = |machine, text: &str| Outgoing {
            machine,
            msg: ClientMsg::Input {
                term: 1,
                data: format!("echo {text}\r").into_bytes(),
            },
        };
        let resize = |machine, cols, rows| Outgoing {
            machine,
            msg: ClientMsg::Resize {
                term: 1,
                cols,
                rows,
            },
        };
        connections.send(input(THERE, FOR_THERE)).unwrap();
        connections.send(input(MachineId::LOCAL, FOR_HERE)).unwrap();
        connections.send(resize(THERE, 101, 31)).unwrap();
        connections.send(resize(MachineId::LOCAL, 91, 25)).unwrap();
        listen_until(&mut connections, &mut heard, &mut sizes, |heard, sizes| {
            heard.there.contains(FOR_THERE)
                && heard.here.contains(FOR_HERE)
                && sizes == &[(91, 25), (101, 31)]
        })
        .await;
        assert!(
            !heard.there.contains(FOR_HERE),
            "this computer's typing reached the other machine"
        );
        assert!(
            !heard.here.contains(FOR_THERE),
            "the other machine's typing reached this computer"
        );
    });
    runtime.shutdown_background();
}

// ---- the connection supervisor, against real daemons through the shim ----------------------

mod supervised {
    use super::*;
    use runtime::connections::{Connections, MachineId, Outgoing};
    use runtime::ipc::Machine;
    use runtime::link::{Backoff, ConnectionState, LinkControl, LinkEvent, Links, Timing};
    use runtime::link_failure::{FIX_LOGIN, FIX_NO_DAEMON, FIX_UPDATE};
    use runtime::protocol::{ClientMsg, Command as Cmd, ServerMsg};

    const BOX: MachineId = MachineId(1);

    fn quick(connect: Duration) -> Timing {
        Timing {
            connect,
            backoff: Backoff {
                first: Duration::from_millis(10),
                most: Duration::from_millis(40),
            },
        }
    }

    /// A configured machine (attach only) behind the shim in `mode`, recording its calls.
    fn configured(sandbox: &Sandbox, mode: &str) -> Machine {
        let calls = format!("--calls={}", sandbox.calls().display());
        let ssh = vec![
            fake_ssh().to_string_lossy().into_owned(),
            format!("--mode={mode}"),
            format!("--maitty={MAITTY}"),
            calls,
            format!("--env=MAITTY_SOCKET={}", sandbox.socket),
            format!("--env=MAITTY_DATA_DIR={}", sandbox.data().display()),
            format!(
                "--env=MAITTY_CONFIG={}",
                sandbox.dir.join("config.toml").display()
            ),
        ];
        Machine::ssh("box", ssh, "maitty").attach_only()
    }

    /// The sandbox's daemon, running: a configured machine never starts one, so the test does,
    /// the way `maitty --remote` does.
    fn start_daemon(sandbox: &Sandbox) {
        let runtime = runtime();
        let opened = runtime.block_on(runtime::ipc::open_to(&machine("starter", sandbox), true));
        assert!(opened.is_ok(), "starting the daemon: {:?}", opened.err());
        drop(opened);
        runtime.shutdown_background();
    }

    /// What the app's event loop does with supervisors and connections, driven by a test.
    struct Window {
        links: Links,
        control: LinkControl,
        connections: Connections,
        msgs: Vec<(MachineId, ServerMsg)>,
        states: Vec<(MachineId, ConnectionState)>,
    }

    impl Window {
        fn new(machines: Vec<(MachineId, Machine)>, timing: Timing) -> Window {
            let links = Links::start(machines, timing);
            Window {
                control: links.control(),
                links,
                connections: Connections::default(),
                msgs: Vec::new(),
                states: Vec::new(),
            }
        }

        async fn step(&mut self) {
            tokio::select! {
                event = self.links.next() => match event {
                    LinkEvent::Line(machine, halves) => self.connections.add(machine, halves),
                    LinkEvent::State { machine, state, .. } => self.states.push((machine, state)),
                },
                // Lets `until` look again when nothing is said, e.g. while dials fail quietly.
                () = tokio::time::sleep(Duration::from_millis(25)) => {}
                burst = self.connections.recv_burst(64) => {
                    self.msgs.extend(burst.msgs.into_iter().map(|m| (burst.machine, m)));
                    if burst.lost {
                        self.control.line_ended(burst.machine, burst.stalled);
                    }
                }
            }
        }

        async fn until(&mut self, what: &str, done: impl Fn(&Window) -> bool) {
            let run = async {
                while !done(self) {
                    self.step().await;
                }
            };
            tokio::time::timeout(WAIT, run).await.unwrap_or_else(|_| {
                panic!(
                    "timed out waiting for {what}; states: {:?}",
                    self.kinds(BOX)
                )
            });
        }

        fn kinds(&self, machine: MachineId) -> Vec<&'static str> {
            self.states
                .iter()
                .filter(|(m, _)| *m == machine)
                .map(|(_, s)| s.kind())
                .collect()
        }

        fn state_of(&self, machine: MachineId) -> Option<&ConnectionState> {
            self.states
                .iter()
                .rev()
                .find(|(m, _)| *m == machine)
                .map(|(_, s)| s)
        }

        fn is(&self, machine: MachineId, kind: &str) -> bool {
            self.state_of(machine).is_some_and(|s| s.kind() == kind)
        }

        fn send(&self, machine: MachineId, msg: ClientMsg) {
            self.connections
                .send(Outgoing { machine, msg })
                .expect("the machine is connected");
        }

        /// The first pane the machine reported.
        fn pane_of(&self, machine: MachineId) -> Option<u32> {
            self.msgs.iter().rev().find_map(|(m, msg)| match msg {
                ServerMsg::State(s) if *m == machine => s.terms.keys().next().copied(),
                _ => None,
            })
        }

        /// Everything the machine's panes printed (output and replays) from message `from` on.
        fn printed(&self, machine: MachineId, from: usize) -> String {
            let mut text = String::new();
            for (m, msg) in &self.msgs[from..] {
                if let (true, ServerMsg::Output { data, .. } | ServerMsg::Replay { data, .. }) =
                    (*m == machine, msg)
                {
                    text.push_str(&String::from_utf8_lossy(data));
                }
            }
            text
        }
    }

    fn kill_connection(calls: &Path) {
        let text = std::fs::read_to_string(calls).expect("the shim recorded its call");
        let last = text.lines().last().expect("at least one call");
        for pid in last.split_whitespace().filter(|pid| *pid != "0") {
            #[cfg(windows)]
            let mut kill = Command::new("taskkill");
            #[cfg(windows)]
            kill.args(["/F", "/PID", pid]);
            #[cfg(unix)]
            let mut kill = Command::new("kill");
            #[cfg(unix)]
            kill.args(["-9", pid]);
            let _ = kill.stdout(Stdio::null()).stderr(Stdio::null()).status();
        }
    }

    fn is_running(pid: &str) -> bool {
        #[cfg(windows)]
        let probe = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).contains(pid));
        #[cfg(unix)]
        let probe = Command::new("kill")
            .args(["-0", pid])
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success());
        probe.expect("probe the process")
    }

    fn calls_made(sandbox: &Sandbox) -> usize {
        std::fs::read_to_string(sandbox.calls()).map_or(0, |text| text.lines().count())
    }

    #[test]
    fn killing_the_ssh_process_shows_reconnecting_then_connected_and_the_pane_repaints() {
        let sandbox = Sandbox::new();
        start_daemon(&sandbox);
        let local = Sandbox::new();
        start_daemon(&local);
        let runtime = runtime();
        let _enter = runtime.enter();
        let mut window = Window::new(vec![(BOX, configured(&sandbox, "proxy"))], quick(WAIT));
        // The machine of this computer, in the same registry, must not notice any of it.
        window.connections.add(
            MachineId::LOCAL,
            runtime
                .block_on(runtime::ipc::open_to(&machine("here", &local), true))
                .expect("local"),
        );

        let token = "PANE_KEEPS_RUNNING_4242";
        runtime.block_on(window.until("connected", |w| w.is(BOX, "connected")));
        window.send(
            BOX,
            ClientMsg::Command(Cmd::NewWorkspace {
                cwd: None,
                name: None,
                cmd: None,
            }),
        );
        runtime.block_on(window.until("a pane", |w| w.pane_of(BOX).is_some()));
        let term = window.pane_of(BOX).unwrap();
        window.send(
            BOX,
            ClientMsg::Input {
                term,
                data: format!("echo {token}\r").into_bytes(),
            },
        );
        runtime.block_on(window.until("the token", |w| w.printed(BOX, 0).contains(token)));

        let before = window.msgs.len();
        kill_connection(&sandbox.calls());
        runtime.block_on(window.until("reconnecting and connected again", |w| {
            w.kinds(BOX).ends_with(&["reconnecting", "connected"])
        }));
        assert_eq!(
            window.kinds(BOX),
            ["connecting", "connected", "reconnecting", "connected"]
        );

        // Output from the old line can still arrive after `before` and carry the token, so only
        // the new line's Replay of the pane proves it repainted.
        let replay_with_token = |w: &Window| {
            w.msgs[before..].iter().any(|(m, msg)| {
                *m == BOX
                    && matches!(msg, ServerMsg::Replay { term: t, data, .. }
                        if *t == term && String::from_utf8_lossy(data).contains(token))
            })
        };
        runtime.block_on(window.until("the pane repainted", replay_with_token));
        assert_eq!(
            window.pane_of(BOX),
            Some(term),
            "the same pane, still running: not restarted"
        );
        assert!(
            window.connections.is_connected(MachineId::LOCAL),
            "this computer's machine was never lost"
        );
        assert_eq!(
            calls_made(&sandbox),
            2,
            "one dial for the line, one to redial at once"
        );

        drop(window);
        runtime.shutdown_background();
    }

    #[test]
    fn one_machine_dying_leaves_this_computers_machine_answering() {
        let (there, here) = (Sandbox::new(), Sandbox::new());
        start_daemon(&there);
        let runtime = runtime();
        let _enter = runtime.enter();
        let mut window = Window::new(vec![(BOX, configured(&there, "proxy"))], quick(WAIT));
        start_daemon(&here);
        window.connections.add(
            MachineId::LOCAL,
            runtime
                .block_on(runtime::ipc::open_to(&machine("here", &here), true))
                .expect("local"),
        );
        runtime.block_on(window.until("connected", |w| w.is(BOX, "connected")));

        kill_connection(&there.calls());
        runtime.block_on(window.until("the lost line", |w| w.kinds(BOX).contains(&"reconnecting")));
        window.send(
            MachineId::LOCAL,
            ClientMsg::Command(Cmd::NewWorkspace {
                cwd: None,
                name: None,
                cmd: None,
            }),
        );
        runtime.block_on(window.until("this machine to answer", |w| {
            w.msgs.iter().any(|(m, msg)| {
                *m == MachineId::LOCAL
                    && matches!(msg, ServerMsg::State(s) if !s.workspaces.is_empty())
            })
        }));
        assert!(window.connections.is_connected(MachineId::LOCAL));
        drop(window);
        runtime.shutdown_background();
    }

    #[test]
    fn a_failed_login_needs_login_with_the_fix_and_keeps_trying() {
        let sandbox = Sandbox::new();
        let runtime = runtime();
        let _enter = runtime.enter();
        let mut window = Window::new(vec![(BOX, configured(&sandbox, "auth-fail"))], quick(WAIT));
        runtime.block_on(window.until("needs-login", |w| w.is(BOX, "needs-login")));
        assert_eq!(
            window.state_of(BOX),
            Some(&ConnectionState::NeedsLogin {
                why: FIX_LOGIN.into()
            })
        );
        runtime.block_on(window.until("another dial", |_| calls_made(&sandbox) >= 3));
        assert_eq!(
            window.kinds(BOX),
            ["connecting", "needs-login"],
            "retries are quiet"
        );
        drop(window);
        runtime.shutdown_background();
    }

    #[test]
    fn a_daemon_of_another_version_needs_an_update_with_the_fix() {
        let sandbox = Sandbox::new();
        let runtime = runtime();
        let _enter = runtime.enter();
        let mut window = Window::new(vec![(BOX, configured(&sandbox, "mismatch"))], quick(WAIT));
        runtime.block_on(window.until("needs-update", |w| w.is(BOX, "needs-update")));
        assert_eq!(
            window.state_of(BOX),
            Some(&ConnectionState::NeedsUpdate {
                why: FIX_UPDATE.into()
            })
        );
        drop(window);
        runtime.shutdown_background();
    }

    #[test]
    fn a_connection_that_hangs_is_dropped_and_dialled_again() {
        let sandbox = Sandbox::new();
        let runtime = runtime();
        let _enter = runtime.enter();
        let mut window = Window::new(
            vec![(BOX, configured(&sandbox, "hang"))],
            quick(Duration::from_millis(300)),
        );
        runtime.block_on(window.until("unreachable", |w| w.is(BOX, "unreachable")));
        let Some(ConnectionState::Unreachable { why, .. }) = window.state_of(BOX) else {
            panic!("{:?}", window.state_of(BOX))
        };
        assert!(why.starts_with("no answer within"), "{why}");
        runtime.block_on(window.until("a second and third dial", |_| calls_made(&sandbox) >= 3));
        let calls = std::fs::read_to_string(sandbox.calls()).unwrap();
        let first = calls
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap();
        runtime.block_on(
            window.until("the hung ssh to be killed, not waited for", |_| {
                !is_running(first)
            }),
        );
        drop(window);
        runtime.shutdown_background();
    }

    #[test]
    fn a_machine_with_no_daemon_is_unreachable_and_no_daemon_is_started() {
        let sandbox = Sandbox::new();
        let runtime = runtime();
        let _enter = runtime.enter();
        let mut window = Window::new(vec![(BOX, configured(&sandbox, "proxy"))], quick(WAIT));
        runtime.block_on(window.until("unreachable", |w| w.is(BOX, "unreachable")));
        let Some(ConnectionState::Unreachable { why, .. }) = window.state_of(BOX) else {
            panic!("{:?}", window.state_of(BOX))
        };
        assert_eq!(why, FIX_NO_DAEMON);
        runtime.block_on(window.until("another dial", |_| calls_made(&sandbox) >= 3));
        let started = std::fs::read_dir(sandbox.data())
            .expect("the data folder")
            .flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "pid"));
        assert!(!started, "attaching never starts the daemon");
        assert!(!window.connections.is_connected(BOX));
        drop(window);
        runtime.shutdown_background();
    }

    #[test]
    fn a_refused_machine_stays_parked_until_a_person_asks_again() {
        let sandbox = Sandbox::new();
        start_daemon(&sandbox);
        let runtime = runtime();
        let _enter = runtime.enter();
        let mut window = Window::new(vec![(BOX, configured(&sandbox, "proxy"))], quick(WAIT));
        runtime.block_on(window.until("connected", |w| w.is(BOX, "connected")));

        window
            .control
            .refuse(BOX, "a different machine answers".into());
        window.connections.mark_lost(BOX);
        runtime.block_on(window.until("unreachable", |w| w.is(BOX, "unreachable")));
        let parked = calls_made(&sandbox);
        runtime.block_on(async {
            let watching = async {
                loop {
                    window.step().await;
                }
            };
            let quiet = tokio::time::timeout(Duration::from_millis(500), watching).await;
            assert!(quiet.is_err(), "the watch only ends by timing out");
        });
        assert_eq!(calls_made(&sandbox), parked, "no dial while parked");

        window.control.retry(BOX);
        runtime.block_on(window.until("connected again", |w| {
            w.kinds(BOX).ends_with(&["reconnecting", "connected"])
        }));
        assert_eq!(calls_made(&sandbox), parked + 1);
        drop(window);
        runtime.shutdown_background();
    }
}

/// A machine's custom transport (`via`) with the shim as the program. A transport is unix-only
/// in v1, so these run on Linux and macOS and are only compiled on Windows.
#[cfg(unix)]
mod custom_transport {
    use super::*;
    use runtime::config::MachineDef;
    use runtime::ipc::{Machine, open_to};
    use runtime::link_failure::{FailureClass, classify};

    const LOGIN_TEXT: &str = "the remote login printed text before maitty started";

    /// The shim as `fly`'s `via`, in `mode`, with `extra` arguments and, when asked, the
    /// `{command}` placeholder last.
    fn via(sandbox: &Sandbox, mode: &str, extra: &[String], placeholder: bool) -> Machine {
        let mut words = vec![
            fake_ssh().to_string_lossy().into_owned(),
            format!("--mode={mode}"),
            format!("--maitty={MAITTY}"),
            format!("--env=MAITTY_SOCKET={}", sandbox.socket),
            format!("--env=MAITTY_DATA_DIR={}", sandbox.data().display()),
            format!(
                "--env=MAITTY_CONFIG={}",
                sandbox.dir.join("config.toml").display()
            ),
        ];
        words.extend(extra.iter().cloned());
        if placeholder {
            words.push("{command}".into());
        }
        let def = MachineDef {
            name: "fly".into(),
            target: "fly".into(),
            via: Some(words),
            ..MachineDef::default()
        };
        Machine::from_def(&def, "ssh")
    }

    /// A configured machine only attaches, so its daemon is started the way its user would.
    fn sandbox_with_daemon() -> Sandbox {
        let sandbox = Sandbox::new();
        let out = sandbox.remote("proxy", &["ls", "--json"]);
        assert!(out.status.success(), "start the daemon: {}", text(&out.stderr));
        sandbox
    }

    struct Failed {
        class: FailureClass,
        said: String,
    }

    fn failure(machine: &Machine) -> Failed {
        let runtime = runtime();
        let result = runtime.block_on(async {
            tokio::time::timeout(WAIT, open_to(machine, true))
                .await
                .expect("the connection fails instead of hanging")
        });
        runtime.shutdown_background();
        match result {
            Ok(_) => panic!("the connection should have failed"),
            Err(e) => Failed {
                class: classify(&e),
                said: format!("{e:#}"),
            },
        }
    }

    #[test]
    fn a_via_machine_connects_and_lists_through_a_local_pipe() {
        use runtime::protocol::{ClientMsg, Query, Reply, ServerMsg};
        let sandbox = sandbox_with_daemon();
        let runtime = runtime();
        let listed = runtime.block_on(async {
            let (mut r, mut w) = open_to(&via(&sandbox, "proxy", &[], false), true)
                .await
                .expect("connect through the transport");
            runtime::ipc::send(&mut w, &ClientMsg::Query(Query::List))
                .await
                .expect("ask for the list");
            tokio::time::timeout(WAIT, async {
                loop {
                    match runtime::ipc::recv_server(&mut r).await.expect("a message") {
                        Some(ServerMsg::Reply(Reply::List(_))) => return true,
                        Some(_) => continue,
                        None => return false,
                    }
                }
            })
            .await
            .expect("the list arrives")
        });
        assert!(listed, "the daemon answered the list");
        runtime.shutdown_background();
    }

    #[test]
    fn the_transport_gets_one_attach_only_remote_command_and_no_ssh_options() {
        let sandbox = sandbox_with_daemon();
        for placeholder in [false, true] {
            let log = sandbox.dir.join(format!("argv-{placeholder}.txt"));
            let machine = via(
                &sandbox,
                "capture-argv",
                &[format!("--argv-log={}", log.display())],
                placeholder,
            );
            let runtime = runtime();
            let opened = runtime.block_on(open_to(&machine, true));
            assert!(opened.is_ok(), "{:?}", opened.err());
            drop(opened);
            runtime.shutdown_background();
            let logged = std::fs::read_to_string(&log).expect("the transport logs its arguments");
            let args: Vec<&str> = logged.lines().filter(|l| *l != "--").collect();
            assert_eq!(
                args.last(),
                Some(&"'maitty' proxy --attach-only"),
                "{args:?}"
            );
            for ssh_only in ["-o", "-a", "-T", "BatchMode=yes", "ForwardAgent=no"] {
                assert!(!args.contains(&ssh_only), "{ssh_only} in {args:?}");
            }
            assert!(!args.contains(&"{command}"), "{args:?}");
        }
    }

    #[test]
    fn the_transport_runs_on_plain_pipes_not_a_terminal() {
        let sandbox = sandbox_with_daemon();
        let runtime = runtime();
        let opened = runtime.block_on(open_to(&via(&sandbox, "require-pipes", &[], false), true));
        assert!(
            opened.is_ok(),
            "stdin and stdout are pipes: {:?}",
            opened.err()
        );
        drop(opened);
        runtime.shutdown_background();
    }

    #[test]
    fn login_text_before_the_marker_is_skipped() {
        let sandbox = sandbox_with_daemon();
        let runtime = runtime();
        let opened = runtime.block_on(open_to(&via(&sandbox, "noise", &[], false), true));
        assert!(opened.is_ok(), "{:?}", opened.err());
        drop(opened);
        runtime.shutdown_background();
    }

    #[test]
    fn a_transport_that_prints_text_and_never_starts_maitty_fails_with_the_login_error() {
        let sandbox = Sandbox::new();
        let failed = failure(&via(&sandbox, "noise-only", &[], false));
        assert!(failed.said.contains(LOGIN_TEXT), "{}", failed.said);
        assert!(failed.said.contains("update it"), "{}", failed.said);
    }

    #[test]
    fn permission_denied_from_a_transport_is_unreachable_not_needs_login() {
        let sandbox = Sandbox::new();
        let failed = failure(&via(&sandbox, "auth-fail", &[], false));
        let FailureClass::Unreachable(why) = failed.class else {
            panic!("not unreachable: {:?}", failed.class);
        };
        assert!(why.contains("Permission denied"), "{why}");
        assert!(failed.said.starts_with("fly: "), "{}", failed.said);
        assert!(!failed.said.contains("ssh fly"), "{}", failed.said);
    }

    #[test]
    fn a_transport_that_exits_127_needs_the_app_installed() {
        let sandbox = Sandbox::new();
        let failed = failure(&via(&sandbox, "not-installed", &[], false));
        assert_eq!(failed.class, FailureClass::Install);
    }

    #[test]
    fn a_transport_that_fails_before_any_frame_is_unreachable_with_its_last_line() {
        let sandbox = Sandbox::new();
        let failed = failure(&via(&sandbox, "unreachable", &[], false));
        assert_eq!(
            failed.class,
            FailureClass::Unreachable("ssh: connect to host box port 22: Connection refused".into())
        );
    }

    fn is_running(pid: &str) -> bool {
        Command::new("kill")
            .args(["-0", pid])
            .stderr(Stdio::null())
            .status()
            .expect("probe the process")
            .success()
    }

    #[test]
    fn a_transport_that_hangs_is_dropped_by_the_connect_timeout_and_its_process_is_gone() {
        let sandbox = Sandbox::new();
        let machine = via(
            &sandbox,
            "hang",
            &[format!("--calls={}", sandbox.calls().display())],
            false,
        );
        let runtime = runtime();
        let timed_out = runtime.block_on(async {
            tokio::time::timeout(Duration::from_millis(500), open_to(&machine, true))
                .await
                .is_err()
        });
        assert!(timed_out, "a hanging transport never answers");
        let calls = std::fs::read_to_string(sandbox.calls()).expect("the transport was started");
        let pid = calls
            .split_whitespace()
            .next()
            .expect("the transport's pid")
            .to_string();
        let start = Instant::now();
        runtime.block_on(async {
            while is_running(&pid) {
                assert!(start.elapsed() < WAIT, "the transport {pid} is still running");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
        runtime.shutdown_background();
    }
}
