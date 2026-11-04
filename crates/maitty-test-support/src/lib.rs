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
