//! Helpers shared by the tests that run the app through the fake `ssh` shim
//! (`apps/tui/examples/fake-ssh.rs`), so a test in any crate can use them.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// `target/<profile>/examples/fake-ssh`, built here: `cargo test --test <name>` does not
/// build examples.
pub fn fake_ssh() -> PathBuf {
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
pub fn output_within(mut command: Command, limit: Duration) -> Output {
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

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `target/<profile>/maitty`, built here: a test of another crate does not get the terminal
/// app's binary built for it.
pub fn maitty_binary() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let exe = std::env::current_exe().expect("the test's own path");
            let profile = exe
                .parent()
                .and_then(Path::parent)
                .expect("target/<profile>");
            let path = profile.join(format!("maitty{}", std::env::consts::EXE_SUFFIX));
            let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let built = Command::new(cargo)
                .args(["build", "--package", "maitty", "--bin", "maitty"])
                .status()
                .expect("run cargo");
            assert!(built.success(), "building the maitty binary failed");
            path
        })
        .clone()
}

/// A private daemon's worth of environment for a test: its own socket, data folder and config,
/// and the shim's calls log. The daemon is stopped and the folder removed when it drops.
pub struct Sandbox {
    pub dir: PathBuf,
    pub socket: String,
    maitty: PathBuf,
    daemon: Option<std::process::Child>,
}

impl Sandbox {
    pub fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let socket = format!("sandbox-{}-{n}", std::process::id());
        let dir = std::env::temp_dir().join(&socket);
        std::fs::create_dir_all(dir.join("data")).expect("create the sandbox");
        Sandbox {
            dir,
            socket,
            maitty: maitty_binary(),
            daemon: None,
        }
    }

    pub fn data(&self) -> PathBuf {
        self.dir.join("data")
    }

    /// Where the shim appends a line per call (`--calls=`).
    pub fn calls(&self) -> PathBuf {
        self.dir.join("ssh-calls.txt")
    }

    /// The sandbox's environment: what makes a `maitty` its own daemon's.
    pub fn env(&self) -> [(&'static str, String); 3] {
        [
            ("MAITTY_SOCKET", self.socket.clone()),
            ("MAITTY_DATA_DIR", self.data().display().to_string()),
            (
                "MAITTY_CONFIG",
                self.dir.join("config.toml").display().to_string(),
            ),
        ]
    }

    /// `maitty` as this sandbox's own, with no remote and no pane identity.
    pub fn maitty(&self) -> Command {
        let mut command = Command::new(&self.maitty);
        command
            .envs(self.env())
            .env_remove("MAITTY_REMOTE")
            .env_remove("MAITTY_REMOTE_CMD")
            .env_remove("MAITTY_TERM_ID")
            .env_remove("MAITTY_PANE_TOKEN");
        command
    }

    /// The arguments that make the shim, standing in for ssh, reach this sandbox's daemon in
    /// `mode`. They go in a machine's `ssh_args`, where they win over the environment, so
    /// several machines can live in one process.
    pub fn shim_args(&self, mode: &str) -> Vec<String> {
        let mut args = vec![
            format!("--mode={mode}"),
            format!("--maitty={}", self.maitty.display()),
            format!("--calls={}", self.calls().display()),
        ];
        args.extend(
            self.env()
                .map(|(key, value)| format!("--env={key}={value}")),
        );
        args
    }

    /// Start this sandbox's daemon and wait until it answers.
    pub fn start_daemon(&mut self) {
        let child = self
            .maitty()
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start the daemon");
        self.daemon = Some(child);
        let start = Instant::now();
        loop {
            let up = self
                .maitty()
                .args(["ls", "--json"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if up {
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(60),
                "the daemon never answered"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Kill the connection the shim made last (the shim and its proxy), the way a dead link does.
    pub fn kill_last_connection(&self) {
        let text = std::fs::read_to_string(self.calls()).expect("the shim recorded its call");
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

    /// How many times the shim has been called.
    pub fn calls_made(&self) -> usize {
        std::fs::read_to_string(self.calls()).map_or(0, |text| text.lines().count())
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        Sandbox::new()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // Must not panic: a test that failed is already unwinding, and a second panic aborts.
        let _ = self
            .maitty()
            .arg("kill-server")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Some(mut child) = self.daemon.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
