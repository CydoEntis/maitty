//! A stand-in for `ssh` in tests: point `MAITTY_SSH` at it. It ignores the host and every
//! option, and runs `maitty proxy` against whatever socket the test chose, so connection,
//! framing, handshake and failure paths run with no real server and no `sh`.
//!
//! `MAITTY_FAKE_SSH_MAITTY` names the `maitty` to run. `MAITTY_FAKE_SSH_MODE` picks the
//! behaviour: `proxy` (default), `auth-fail`, `noise`, `noise-only`, `noise-flood`, `old`, `hang`,
//! `mismatch` (an older daemon), `mismatch-newer`, `exit`, `unreachable` (ssh cannot connect), `not-installed` (the remote has no
//! app), `capture-hello`, `require-pipes`, `capture-argv`.
//! The shim also stands in for a machine's custom transport (`via`): it is then the program,
//! the settings are its own arguments, and the remote command is its last argument.
//! `require-pipes` runs the proxy only when its stdin and stdout are not terminals (it exits 3
//! otherwise), proving the transport was started on plain pipes. `capture-argv` appends every
//! argument it was called with, one per line then a line of `--`, to the file `--argv-log=<path>`
//! names, and runs the proxy.
//! A call whose remote command is `... --version` (the probe and the first login) answers with
//! a version line and exits 0 in the `proxy`, `noise` and `old` modes; the other modes fail or
//! hang for it as they do for a proxy. A call asking `command -v maitty` (`doctor` looking for
//! the app on the login PATH) answers a path the same way.
//! `remote-sh` stands in for a real machine's shell: it runs the remote command (the last
//! argument) through the host's POSIX `sh -c`, with `HOME` set to the folder `--home=<dir>`
//! names (a fake remote home, whose `bin` comes first on `PATH`) and stdin passed through, so
//! the install and paste scripts run against a temporary filesystem. The host's `XDG_*` folders
//! are removed from its environment, so a fake remote never writes to the real ones. `--argv-log=<path>` appends each
//! remote command, newlines written as a backslash and `n`, so a test can see what was (not) sent.
//! `capture-hello` writes the client's first frame (its Hello) to the file `MAITTY_FAKE_SSH_HELLO`
//! names, answers it with a Welcome, and exits.
//! `MAITTY_FAKE_SSH_LOG`, when set, receives the arguments it was called with, one per line.
//!
//! A test that holds several machines in one process puts each machine's settings in the
//! arguments instead, where they win over the environment: `--mode=<mode>`,
//! `--maitty=<path>`, and `--env=<KEY>=<value>` (repeatable) for the proxy's environment.
//! `--calls=<path>` appends a line `<shim pid> <proxy pid>` per call (the proxy's is 0 when none
//! was started), so a test can count the dials and kill the connection the way a dead link does.
//! `--argv-log=<path>` also records each `proxy` call's arguments (see `capture-argv`).
//! Like ssh, it passes `--attach-only` on to `maitty proxy` when the remote command asks for it.

use std::io::{IsTerminal, Read, Write};
use std::process::{Command, Stdio};

const VERSION: &str = "maitty 0.0.0-fake\n";
const FOUND_AT: &str = "/fake/bin/maitty\n";
const NOISE: &str =
    "Last login: Mon Nov  2 09:14:07 2026 from 10.0.0.2\nWelcome to the build box\n";

/// What the arguments say, over the environment.
struct Own {
    mode: String,
    maitty: Option<std::ffi::OsString>,
    env: Vec<(String, String)>,
    calls: Option<std::path::PathBuf>,
    /// `remote-sh`: the fake remote home.
    home: Option<std::path::PathBuf>,
    /// Where each remote command is appended.
    argv_log: Option<std::path::PathBuf>,
    /// The remote command (the last argument) is `... proxy --attach-only`.
    attach_only: bool,
    /// The remote command (the last argument) is `... --version`.
    asks_version: bool,
    /// The remote command (the last argument) is the `command -v maitty` script.
    asks_where: bool,
}

fn own(args: &[String]) -> Own {
    let mut own = Own {
        mode: std::env::var("MAITTY_FAKE_SSH_MODE").unwrap_or_else(|_| "proxy".into()),
        maitty: std::env::var_os("MAITTY_FAKE_SSH_MAITTY"),
        env: Vec::new(),
        calls: None,
        home: None,
        argv_log: None,
        attach_only: args
            .last()
            .is_some_and(|last| last.ends_with(" proxy --attach-only")),
        asks_version: args.last().is_some_and(|last| last.ends_with(" --version")),
        asks_where: args
            .last()
            .is_some_and(|last| last.contains("command -v maitty")),
    };
    for arg in args {
        if let Some(mode) = arg.strip_prefix("--mode=") {
            own.mode = mode.into();
        } else if let Some(path) = arg.strip_prefix("--maitty=") {
            own.maitty = Some(path.into());
        } else if let Some(path) = arg.strip_prefix("--calls=") {
            own.calls = Some(path.into());
        } else if let Some(path) = arg.strip_prefix("--home=") {
            own.home = Some(path.into());
        } else if let Some(path) = arg.strip_prefix("--argv-log=") {
            own.argv_log = Some(path.into());
        } else if let Some((key, value)) = arg
            .strip_prefix("--env=")
            .and_then(|pair| pair.split_once('='))
        {
            own.env.push((key.into(), value.into()));
        }
    }
    own
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let own = own(&args);
    if let Some(log) = std::env::var_os("MAITTY_FAKE_SSH_LOG") {
        let mut text = args.join("\n");
        text.push('\n');
        std::fs::write(log, text).expect("write the argument log");
    }
    if !matches!(
        own.mode.as_str(),
        "proxy" | "noise" | "old" | "require-pipes" | "capture-argv"
    ) {
        record_call(&own, 0);
    }
    if own.asks_version && matches!(own.mode.as_str(), "proxy" | "noise" | "old") {
        print_stdout(VERSION.as_bytes());
        return;
    }
    if own.asks_where && matches!(own.mode.as_str(), "proxy" | "noise" | "old") {
        print_stdout(FOUND_AT.as_bytes());
        return;
    }
    match own.mode.as_str() {
        "proxy" => {
            if own.argv_log.is_some() {
                log_argv(&own, &args);
            }
            std::process::exit(run_proxy(&own));
        }
        "noise" => {
            print_stdout(NOISE.as_bytes());
            std::process::exit(run_proxy(&own));
        }
        "require-pipes" => {
            if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                eprintln!("fake-ssh: stdin or stdout is a terminal");
                std::process::exit(3);
            }
            std::process::exit(run_proxy(&own));
        }
        "capture-argv" => {
            log_argv(&own, &args);
            std::process::exit(run_proxy(&own));
        }
        "noise-only" => print_stdout(NOISE.as_bytes()),
        "noise-flood" => {
            print_stdout(&NOISE.as_bytes().repeat(2000));
            let mut sink = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut sink);
        }
        "old" => std::process::exit(run_old_proxy(&own)),
        "remote-sh" => std::process::exit(run_remote_sh(&own, &args)),
        "auth-fail" => {
            eprintln!("host@box: Permission denied (publickey).");
            std::process::exit(255);
        }
        "unreachable" => {
            eprintln!("ssh: connect to host box port 22: Connection refused");
            std::process::exit(255);
        }
        "not-installed" => {
            eprintln!("bash: maitty: command not found");
            std::process::exit(127);
        }
        "exit" => std::process::exit(0),
        "hang" => loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        },
        // An older daemon (`mismatch`) or a newer one than this build (`mismatch-newer`).
        "mismatch" | "mismatch-newer" => {
            let theirs = if own.mode == "mismatch" {
                maitty_shared::protocol::PROTOCOL_VERSION - 1
            } else {
                maitty_shared::protocol::PROTOCOL_VERSION + 1
            };
            print_stdout(&welcome_frame(theirs));
            let mut sink = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut sink);
        }
        "capture-hello" => capture_hello(),
        other => {
            eprintln!("fake-ssh: unknown MAITTY_FAKE_SSH_MODE {other}");
            std::process::exit(2);
        }
    }
}

fn log_argv(own: &Own, args: &[String]) {
    let path = own.argv_log.as_ref().expect("--argv-log= names the file");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open the argument log");
    for arg in args {
        writeln!(file, "{}", arg.replace('\n', "\\n")).expect("write the argument log");
    }
    writeln!(file, "--").expect("write the argument log");
}

fn record_call(own: &Own, proxy_pid: u32) {
    let Some(path) = &own.calls else { return };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open the calls file");
    writeln!(file, "{} {proxy_pid}", std::process::id()).expect("write the calls file");
}

/// `maitty proxy`, with `--attach-only` when ssh was asked to pass it.
fn proxy_command(own: &Own, maitty: &std::ffi::OsStr) -> Command {
    let mut command = Command::new(maitty);
    command.arg("proxy");
    if own.attach_only {
        command.arg("--attach-only");
    }
    command.envs(own.env.iter().map(|(key, value)| (key, value)));
    command
}

fn welcome_frame(version: u32) -> Vec<u8> {
    let body =
        maitty_shared::protocol::encode(&maitty_shared::protocol::ServerMsg::Welcome { version })
            .expect("encode a Welcome");
    let mut frame = (body.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(&body);
    frame
}

/// Keeps what a remote daemon would be told first, so a test can read the Hello.
fn capture_hello() {
    let path = std::env::var_os("MAITTY_FAKE_SSH_HELLO")
        .expect("MAITTY_FAKE_SSH_HELLO names the file for the Hello");
    let mut stdin = std::io::stdin().lock();
    let mut length = [0u8; 4];
    stdin
        .read_exact(&mut length)
        .expect("read the first frame's length");
    let mut body = vec![0u8; u32::from_be_bytes(length) as usize];
    stdin.read_exact(&mut body).expect("read the first frame");
    std::fs::write(path, body).expect("write the Hello");
    print_stdout(&welcome_frame(maitty_shared::protocol::PROTOCOL_VERSION));
}

fn print_stdout(bytes: &[u8]) {
    let mut out = std::io::stdout();
    out.write_all(bytes).expect("write stdout");
    out.flush().expect("flush stdout");
}

/// A folder as the shell reads it: Git's `sh` on Windows wants `/c/dir`, not `C:\dir`.
fn shell_path(path: &std::path::Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    match text.as_bytes() {
        [drive, b':', rest @ ..] if cfg!(windows) => format!(
            "/{}{}",
            (*drive as char).to_ascii_lowercase(),
            String::from_utf8_lossy(rest)
        ),
        _ => text,
    }
}

/// The remote command through `sh -c` in the fake remote home, with this process's stdio.
fn run_remote_sh(own: &Own, args: &[String]) -> i32 {
    let remote = args.last().expect("ssh is given the remote command last");
    let home = own
        .home
        .as_ref()
        .expect("--home= names the fake remote home");
    if let Some(path) = &own.argv_log {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open the argument log");
        writeln!(file, "{}", remote.replace('\n', "\\n")).expect("write the argument log");
    }
    let old = std::env::var_os("PATH").unwrap_or_default();
    let path =
        std::env::join_paths(std::iter::once(home.join("bin")).chain(std::env::split_paths(&old)))
            .expect("a PATH");
    // The fake `bin` goes first from inside the shell: Git's `sh` on Windows puts its own
    // /usr/bin ahead of the PATH it was started with when that PATH lacks it (a CI runner's
    // does), and the real `uname` would win over the fake one.
    let bin = shell_path(&home.join("bin")).replace('\'', "'\\''");
    let script = format!("PATH='{bin}':\"$PATH\"; export PATH; {remote}");
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .env("HOME", home)
        .env("PATH", path)
        // A fake remote has no folders but its own: the host's cache and data folders would
        // otherwise be written to by whatever a test runs there.
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .env_remove("XDG_CONFIG_HOME")
        .envs(own.env.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map(|status| status.code().unwrap_or(1))
        .unwrap_or_else(|e| {
            eprintln!("fake-ssh: running sh: {e}");
            127
        })
}

/// Runs `maitty proxy` with this process's stdio and returns its exit code.
fn run_proxy(own: &Own) -> i32 {
    let maitty = own
        .maitty
        .as_ref()
        .expect("MAITTY_FAKE_SSH_MAITTY or --maitty names the maitty to run");
    let mut child = proxy_command(own, maitty)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("run maitty proxy");
    record_call(own, child.id());
    child
        .wait()
        .ok()
        .and_then(|status| status.code())
        .unwrap_or(1)
}

/// Runs `maitty proxy` as one from before the marker would: its stdout without the marker.
fn run_old_proxy(own: &Own) -> i32 {
    let maitty = own
        .maitty
        .as_ref()
        .expect("MAITTY_FAKE_SSH_MAITTY or --maitty names the maitty to run");
    let mut child = proxy_command(own, maitty)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("run maitty proxy");
    record_call(own, child.id());
    let mut from_proxy = child.stdout.take().expect("the proxy's stdout");
    let mut marker = vec![0u8; runtime::ipc::STREAM_MARKER.len()];
    from_proxy.read_exact(&mut marker).expect("read the marker");
    assert_eq!(
        marker,
        runtime::ipc::STREAM_MARKER,
        "the proxy opens with the marker"
    );
    let mut buf = [0u8; 8192];
    loop {
        match from_proxy.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => print_stdout(&buf[..n]),
        }
    }
    child
        .wait()
        .ok()
        .and_then(|status| status.code())
        .unwrap_or(1)
}
