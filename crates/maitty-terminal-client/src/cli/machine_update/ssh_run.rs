//! Running one ssh line to its end: stdin from a file when there is one, stdout and stderr
//! collected (bounded), the exit status kept apart from what was said. The line comes from
//! `ssh_command`; nothing here builds or reads ssh arguments.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How often a running ssh is looked at for its end and the deadline.
const POLL: Duration = Duration::from_millis(25);
const MOST_OUTPUT_BYTES: u64 = 64 * 1024;

/// How long a short question (a platform, a version, a restart) may take, connecting included.
pub(super) const ASK_LIMIT: Duration = Duration::from_secs(30);
/// How long streaming a binary may take.
pub(super) const UPLOAD_LIMIT: Duration = Duration::from_secs(15 * 60);

pub(super) struct Ran {
    /// `None` when a signal ended it, or the limit did.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Run `argv`; its stdin is `stdin`'s bytes, or nothing. Ended (killed) when `limit` passes.
pub(super) fn run(argv: &[String], stdin: Option<&Path>, limit: Duration) -> Result<Ran, String> {
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("running {} (is OpenSSH installed?): {e}", argv[0]))?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let feeding = stdin.map(|path| feed(path, child.stdin.take()));
    let deadline = Instant::now() + limit;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("waiting for {}: {e}", argv[0]));
            }
        }
    };
    let mut stderr = err.join().unwrap_or_default();
    if let Some(Err(e)) = feeding.map(|writer| writer.join()).and_then(Result::ok) {
        stderr.push_str(&format!("\nsending the file failed: {e}"));
    }
    Ok(Ran {
        code,
        stdout: out.join().unwrap_or_default(),
        stderr,
    })
}

/// Copy `path` into the child's stdin, then close it so the far side sees the end. A pipe
/// that closes early is the install script ending before it read everything (it says why
/// in its exit status), so that is not an error here.
fn feed(
    path: &Path,
    stdin: Option<std::process::ChildStdin>,
) -> std::thread::JoinHandle<std::io::Result<()>> {
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let Some(mut stdin) = stdin else {
            return Ok(());
        };
        let mut file = std::fs::File::open(path)?;
        match std::io::copy(&mut file, &mut stdin) {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
            other => other.map(drop),
        }
    })
}

/// What a pipe holds until it closes, bounded.
fn drain(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = pipe {
            let _ = pipe.take(MOST_OUTPUT_BYTES).read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}
