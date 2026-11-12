//! Helpers shared by the tests that run the app through the fake `ssh` shim
//! (`apps/tui/examples/fake-ssh.rs`), so a test in any crate can use them.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// The built `fake-ssh` example: `cargo test --test <name>` does not build examples.
pub fn fake_ssh() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| build_with_cargo(&["--example", "fake-ssh"], "fake-ssh"))
        .clone()
}

/// Builds with cargo and returns the file it reports. Always asks cargo: a no-op when
/// current, and it would otherwise leave a stale build from earlier. The path comes from
/// cargo's own output, because guessing it from the test binary's folder broke when a newer
/// Rust moved intermediate build files out of `target/<profile>/`.
fn build_with_cargo(what: &[&str], target_name: &str) -> PathBuf {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let built = Command::new(cargo)
        .args(["build", "--package", "maitty"])
        .args(what)
        .arg("--message-format=json-render-diagnostics")
        .stderr(Stdio::inherit())
        .output()
        .expect("run cargo");
    assert!(built.status.success(), "building {target_name} failed");
    String::from_utf8_lossy(&built.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|msg| msg["reason"] == "compiler-artifact" && msg["target"]["name"] == target_name)
        .find_map(|msg| msg["executable"].as_str().map(PathBuf::from))
        .unwrap_or_else(|| panic!("cargo did not say where it built {target_name}"))
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

/// The terminal app's binary, built here: a test of another crate does not get it built
/// for it.
pub fn maitty_binary() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| build_with_cargo(&["--bin", "maitty"], "maitty"))
        .clone()
}

/// `pwsh` 7 keeps one PSReadLine history file per user, whatever `USERPROFILE` says, so shells
/// in different sandboxes recall each other's typed lines. Saving nothing and predicting
/// nothing keeps each sandbox's shells to what they were typed.
const PWSH_PRIVATE_HISTORY: &str =
    "Set-PSReadLineOption -HistorySaveStyle SaveNothing -PredictionSource None";

/// The sandbox's `config.toml`, if it needs one. Only Windows with `pwsh` does: the options are
/// set by the shell's own start-up script, which is why `shell_integration` is off (it would
/// add a second `-Command`).
fn config_text(windows: bool, pwsh: bool) -> Option<String> {
    (windows && pwsh).then(|| {
        format!(
            "shell = \"pwsh.exe\"\nshell_integration = false\n\
             shell_args = [\"-NoExit\", \"-Command\", '{PWSH_PRIVATE_HISTORY}']\n"
        )
    })
}

fn pwsh_on_path() -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join("pwsh.exe").is_file()))
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
        if let Some(text) = config_text(cfg!(windows), pwsh_on_path()) {
            std::fs::write(dir.join("config.toml"), text).expect("write the sandbox config");
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_with_pwsh_gets_a_config_that_turns_history_off() {
        let text = config_text(true, true).expect("a config");
        let parsed: toml::Table = text.parse().expect("valid toml");
        assert_eq!(parsed["shell"].as_str(), Some("pwsh.exe"));
        assert_eq!(parsed["shell_integration"].as_bool(), Some(false));
        let args: Vec<_> = parsed["shell_args"]
            .as_array()
            .expect("an array")
            .iter()
            .filter_map(|a| a.as_str())
            .collect();
        assert_eq!(args, ["-NoExit", "-Command", PWSH_PRIVATE_HISTORY]);
    }

    #[test]
    fn other_setups_keep_the_default_config() {
        assert_eq!(config_text(false, true), None);
        assert_eq!(config_text(true, false), None);
    }
}
