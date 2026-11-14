//! Running one ssh line to its end: stdin from a file when there is one, stdout and stderr
//! collected (bounded), the exit status kept apart from what was said. The line comes from
//! `ssh_command`; nothing here builds or reads ssh arguments.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How often a running ssh is looked at for its end and the deadline.
const POLL: Duration = Duration::from_millis(25);
const MOST_OUTPUT_BYTES: u64 = 64 * 1024;

/// How long a short question (a platform, a version, a restart) may take, connecting included.
pub const ASK_LIMIT: Duration = Duration::from_secs(30);
/// How long streaming a binary may take.
pub const UPLOAD_LIMIT: Duration = Duration::from_secs(15 * 60);

/// What the caller can see and do while a long send runs: how many bytes have gone into the
/// line, and a flag that ends it (ADR 0007).
#[derive(Clone, Default)]
pub struct Watch {
    /// The size being sent, set once the sender knows it.
    pub total: Arc<AtomicU64>,
    pub sent: Arc<AtomicU64>,
    pub cancel: Arc<AtomicBool>,
}

/// How much of the file is copied between looks at the cancel flag.
const CHUNK: usize = 64 * 1024;

pub struct Ran {
    /// `None` when a signal ended it, or the limit did, or `Watch::cancel` did.
    pub code: Option<i32>,
    /// Ended because `Watch::cancel` was set.
    pub cancelled: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Run `argv`; its stdin is `stdin`'s bytes, or nothing. Ended (killed) when `limit` passes.
pub fn run(argv: &[String], stdin: Option<&Path>, limit: Duration) -> Result<Ran, String> {
    run_watched(argv, stdin, limit, &Watch::default())
}

/// `run`, counting the bytes written to the line and ending it early when `watch.cancel` is set.
pub fn run_watched(
    argv: &[String],
    stdin: Option<&Path>,
    limit: Duration,
    watch: &Watch,
) -> Result<Ran, String> {
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
        command.creation_flags(crate::CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("running {} (is OpenSSH installed?): {e}", argv[0]))?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let feeding = stdin.map(|path| feed(path, child.stdin.take(), watch.clone()));
    let deadline = Instant::now() + limit;
    let mut cancelled = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if watch.cancel.load(Ordering::Relaxed) => {
                cancelled = true;
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
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
        cancelled,
        stdout: out.join().unwrap_or_default(),
        stderr,
    })
}

/// What came of a line whose stdout was read as it arrived.
pub struct Streamed<T> {
    /// `None` when a signal ended it, or the limit did, or `Watch::cancel` did.
    pub code: Option<i32>,
    /// Ended because `Watch::cancel` was set.
    pub cancelled: bool,
    pub stderr: String,
    /// What `consume` made of the stream.
    pub result: Result<T, String>,
}

/// Run `argv` and hand its stdout to `consume` as it arrives, instead of collecting it. The line
/// is ended (killed) when `limit` passes, when `watch.cancel` is set, or when `consume` fails; a
/// read in progress then ends with the pipe. `consume` should stop at the end of what it wants:
/// the exit status is still waited for.
pub fn run_reading<T>(
    argv: &[String],
    limit: Duration,
    watch: &Watch,
    consume: impl FnOnce(&mut dyn Read) -> Result<T, String>,
) -> Result<Streamed<T>, String> {
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("running {} (is OpenSSH installed?): {e}", argv[0]))?;
    let mut stdout = child.stdout.take();
    let err = drain(child.stderr.take());
    let child = Arc::new(Mutex::new(child));
    let finished = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let watcher = {
        let (child, finished, cancelled, cancel) = (
            Arc::clone(&child),
            Arc::clone(&finished),
            Arc::clone(&cancelled),
            Arc::clone(&watch.cancel),
        );
        let deadline = Instant::now() + limit;
        std::thread::spawn(move || {
            while !finished.load(Ordering::Relaxed) {
                let cancel_now = cancel.load(Ordering::Relaxed);
                if cancel_now || Instant::now() >= deadline {
                    cancelled.store(cancel_now, Ordering::Relaxed);
                    if let Ok(mut child) = child.lock() {
                        let _ = child.kill();
                    }
                    return;
                }
                std::thread::sleep(POLL);
            }
        })
    };

    let result = match stdout.as_mut() {
        Some(pipe) => consume(pipe),
        None => Err("the line gave no output to read".to_owned()),
    };
    if result.is_err()
        && let Ok(mut child) = child.lock()
    {
        let _ = child.kill();
    }
    drop(stdout);
    let code = loop {
        let status = child.lock().map_err(|_| "waiting for ssh failed".to_owned())?.try_wait();
        match status {
            Ok(Some(status)) => break status.code(),
            Ok(None) => std::thread::sleep(POLL),
            Err(e) => {
                finished.store(true, Ordering::Relaxed);
                return Err(format!("waiting for {}: {e}", argv[0]));
            }
        }
    };
    finished.store(true, Ordering::Relaxed);
    let _ = watcher.join();
    Ok(Streamed {
        code,
        cancelled: cancelled.load(Ordering::Relaxed),
        stderr: err.join().unwrap_or_default(),
        result,
    })
}

/// Copy `path` into the child's stdin, then close it so the far side sees the end. A pipe
/// that closes early is the install script ending before it read everything (it says why
/// in its exit status), so that is not an error here.
fn feed(
    path: &Path,
    stdin: Option<std::process::ChildStdin>,
    watch: Watch,
) -> std::thread::JoinHandle<std::io::Result<()>> {
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let Some(mut stdin) = stdin else {
            return Ok(());
        };
        let mut file = std::fs::File::open(path)?;
        let mut chunk = vec![0u8; CHUNK];
        loop {
            if watch.cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            let n = file.read(&mut chunk)?;
            if n == 0 {
                return Ok(());
            }
            match stdin.write_all(&chunk[..n]) {
                Ok(()) => watch.sent.fetch_add(n as u64, Ordering::Relaxed),
                Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => return Ok(()),
                Err(e) => return Err(e),
            };
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> Vec<String> {
        vec!["sh".into(), "-c".into(), script.into()]
    }

    fn file(label: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ssh-run-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn the_file_reaches_stdin_whole_and_every_byte_is_counted() {
        let body = vec![7u8; CHUNK * 3 + 11];
        let path = file("whole", &body);
        let watch = Watch::default();
        let ran = run_watched(&sh("wc -c; echo said >&2"), Some(&path), ASK_LIMIT, &watch).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(ran.code, Some(0));
        assert_eq!(ran.stdout.trim(), body.len().to_string());
        assert_eq!(ran.stderr.trim(), "said");
        assert_eq!(watch.sent.load(Ordering::Relaxed), body.len() as u64);
        assert!(!ran.cancelled);
    }

    #[test]
    fn an_exit_status_is_kept_apart_from_what_was_said() {
        let ran = run(&sh("echo out; echo err >&2; exit 70"), None, ASK_LIMIT).unwrap();
        assert_eq!(ran.code, Some(70));
        assert_eq!(ran.stdout.trim(), "out");
        assert_eq!(ran.stderr.trim(), "err");
    }

    // `exec`, because dash forks a `-c` command instead of becoming it: killing the shell would
    // leave `sleep` holding the output pipes, and collecting the output would wait the 30 s out.
    #[test]
    fn a_cancel_ends_a_line_that_would_run_on() {
        let watch = Watch::default();
        watch.cancel.store(true, Ordering::Relaxed);
        let started = Instant::now();
        let ran = run_watched(&sh("exec sleep 30"), None, ASK_LIMIT, &watch).unwrap();
        assert!(ran.cancelled);
        assert_eq!(ran.code, None);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn the_time_limit_ends_a_line_that_never_finishes() {
        let ran = run(&sh("exec sleep 30"), None, Duration::from_millis(200)).unwrap();
        assert_eq!(ran.code, None);
        assert!(!ran.cancelled, "the limit is not a cancel");
    }

    #[test]
    fn a_program_that_cannot_start_says_so() {
        let argv = vec!["definitely-not-a-program-ssh".to_owned()];
        let e = run(&argv, None, ASK_LIMIT).err().expect("it cannot run");
        assert!(e.contains("is OpenSSH installed"), "{e}");
    }

    #[test]
    fn what_a_line_prints_is_bounded() {
        let ran = run(
            &sh("head -c 200000 /dev/zero | tr '\\0' x"),
            None,
            ASK_LIMIT,
        )
        .unwrap();
        assert!(ran.stdout.len() as u64 <= MOST_OUTPUT_BYTES);
    }
}

#[cfg(test)]
mod reading_tests {
    use super::*;

    fn sh(script: &str) -> Option<Vec<String>> {
        let has_sh = Command::new("sh")
            .arg("-c")
            .arg("true")
            .status()
            .is_ok_and(|status| status.success());
        has_sh.then(|| vec!["sh".into(), "-c".into(), script.into()])
    }

    #[test]
    fn the_output_is_read_as_it_arrives_and_the_status_is_kept() {
        let Some(argv) = sh("printf 'one\ntwo\n'; echo said >&2; exit 3") else {
            return;
        };
        let got = run_reading(&argv, ASK_LIMIT, &Watch::default(), |out| {
            let mut text = String::new();
            out.read_to_string(&mut text).map_err(|e| e.to_string())?;
            Ok(text.replace("\r\n", "\n"))
        })
        .unwrap();
        assert_eq!(got.result, Ok("one\ntwo\n".to_owned()));
        assert_eq!(got.code, Some(3));
        assert_eq!(got.stderr.trim(), "said");
        assert!(!got.cancelled);
    }

    #[test]
    fn a_cancel_ends_a_line_that_is_still_sending() {
        let Some(argv) = sh("echo started; exec sleep 3") else {
            return;
        };
        let watch = Watch::default();
        let cancel = Arc::clone(&watch.cancel);
        let got = run_reading(&argv, ASK_LIMIT, &watch, |out| {
            let mut first = [0u8; 1];
            out.read_exact(&mut first).map_err(|e| e.to_string())?;
            cancel.store(true, Ordering::Relaxed);
            let mut rest = Vec::new();
            out.read_to_end(&mut rest).map_err(|e| e.to_string())?;
            Ok(rest.len())
        })
        .unwrap();
        assert!(got.cancelled);
        assert_ne!(got.code, Some(0), "it was ended, not finished");
    }

    #[test]
    fn the_time_limit_ends_a_line_that_never_finishes() {
        let Some(argv) = sh("exec sleep 3") else {
            return;
        };
        let got = run_reading(&argv, Duration::from_millis(300), &Watch::default(), |out| {
            let mut rest = Vec::new();
            out.read_to_end(&mut rest).map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();
        assert_ne!(got.code, Some(0), "it was ended, not finished");
        assert!(!got.cancelled, "the limit is not a cancel");
    }

    #[test]
    fn a_consumer_that_fails_ends_the_line_at_once() {
        let Some(argv) = sh("echo go; exec sleep 3") else {
            return;
        };
        let got = run_reading(&argv, ASK_LIMIT, &Watch::default(), |_| -> Result<(), String> {
            Err("not wanted".into())
        })
        .unwrap();
        assert_eq!(got.result, Err("not wanted".to_owned()));
        assert_ne!(got.code, Some(0), "the line was ended, not left to finish");
    }
}
