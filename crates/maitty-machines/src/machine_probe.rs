//! A look at a machine over ssh before it is added: log in with what ssh already has (keys,
//! an agent, a known host), never asking anyone anything, and run the app's `--version`
//! there. A result tells the dialog whether to add the machine straight away, to open a
//! terminal for a login that needs a person, or to show why the machine cannot be reached.
//!
//! It blocks, so a window runs it on a background thread; setting `cancel` ends it.

use crate::machine::Machine;
use crate::link_failure::{FailureClass, SshEnd, classify};
use maitty_shared::hidden::sanitize;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How often a running ssh is looked at for cancel and the deadline.
const POLL: Duration = Duration::from_millis(25);
/// How long output is waited for once ssh has ended.
const OUTPUT_GRACE: Duration = Duration::from_secs(1);
const MOST_OUTPUT_BYTES: u64 = 64 * 1024;
const MOST_VERSION_CHARS: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// ssh logged in and the app answered; `version` is its first line.
    Ready { version: String },
    /// ssh did not get as far, classified as a connection that failed.
    Failed(FailureClass),
    /// The caller gave up; ssh has been ended.
    Cancelled,
}

/// Where the app is on a machine's login PATH.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Located {
    /// The path `command -v` printed, cleaned and cut.
    At(String),
    NotFound,
    Failed(FailureClass),
    Cancelled,
}

/// Probe `machine`, ending ssh when `limit` passes or `cancel` is set.
pub fn probe(machine: &Machine, limit: Duration, cancel: &AtomicBool) -> Probe {
    match machine.probe_argv() {
        Ok(argv) => run(&argv, limit, cancel),
        Err(e) => Probe::Failed(FailureClass::Unreachable(sanitize(
            &format!("{e:#}"),
            false,
        ))),
    }
}

/// Ask `machine` where its login shell finds the app, under the same rules as `probe`.
pub fn locate(machine: &Machine, limit: Duration, cancel: &AtomicBool) -> Located {
    let argv = match machine.where_argv() {
        Ok(argv) => argv,
        Err(e) => {
            return Located::Failed(FailureClass::Unreachable(sanitize(
                &format!("{e:#}"),
                false,
            )));
        }
    };
    match collect(&argv, limit, cancel) {
        Ok(ended) => located(ended.code, &ended.out, &ended.err),
        Err(Stopped::Cancelled) => Located::Cancelled,
        Err(Stopped::Failed(class)) => Located::Failed(class),
    }
}

/// Why a line did not run to its end.
enum Stopped {
    Cancelled,
    Failed(FailureClass),
}

/// A line that ran to its end: its status and what it printed.
struct Ended {
    code: Option<i32>,
    out: mpsc::Receiver<String>,
    err: mpsc::Receiver<String>,
}

fn run(argv: &[String], limit: Duration, cancel: &AtomicBool) -> Probe {
    match collect(argv, limit, cancel) {
        Ok(ended) => finish(ended.code, &ended.out, &ended.err),
        Err(Stopped::Cancelled) => Probe::Cancelled,
        Err(Stopped::Failed(class)) => Probe::Failed(class),
    }
}

fn collect(argv: &[String], limit: Duration, cancel: &AtomicBool) -> Result<Ended, Stopped> {
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
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            return Err(Stopped::Failed(FailureClass::Unreachable(sanitize(
                &format!("running {} (is OpenSSH installed?): {e}", argv[0]),
                false,
            ))));
        }
    };
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let deadline = Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(Ended {
                    code: status.code(),
                    out,
                    err,
                });
            }
            Ok(None) => {}
            Err(e) => {
                end(&mut child);
                return Err(Stopped::Failed(FailureClass::Unreachable(sanitize(
                    &e.to_string(),
                    false,
                ))));
            }
        }
        if cancel.load(Ordering::Relaxed) {
            end(&mut child);
            return Err(Stopped::Cancelled);
        }
        if Instant::now() >= deadline {
            end(&mut child);
            return Err(Stopped::Failed(FailureClass::Unreachable(format!(
                "no answer within {} seconds",
                limit.as_secs()
            ))));
        }
        std::thread::sleep(POLL);
    }
}

/// Kill ssh and wait for it, so no process outlives the probe.
fn end(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// What a pipe holds until it closes, bounded. The thread ends when the pipe does.
fn drain(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    if let Some(pipe) = pipe {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.take(MOST_OUTPUT_BYTES).read_to_end(&mut bytes);
            let _ = tx.send(String::from_utf8_lossy(&bytes).into_owned());
        });
    }
    rx
}

fn finish(code: Option<i32>, out: &mpsc::Receiver<String>, err: &mpsc::Receiver<String>) -> Probe {
    let said = err.recv_timeout(OUTPUT_GRACE).unwrap_or_default();
    if code == Some(0) {
        let printed = out.recv_timeout(OUTPUT_GRACE).unwrap_or_default();
        return Probe::Ready {
            version: first_line(&printed),
        };
    }
    let end = SshEnd {
        code,
        message: format!("ssh: {}", said.trim()),
        said,
        streamed: false,
        custom: false,
    };
    Probe::Failed(classify(&anyhow::Error::from(end)))
}

/// `command -v` printed a path (status 0), or nothing (1); any other end is the line failing.
fn located(code: Option<i32>, out: &mpsc::Receiver<String>, err: &mpsc::Receiver<String>) -> Located {
    match code {
        Some(0) => {
            let printed = out.recv_timeout(OUTPUT_GRACE).unwrap_or_default();
            match first_line(&printed) {
                path if path.is_empty() => Located::NotFound,
                path => Located::At(path),
            }
        }
        Some(1) => Located::NotFound,
        _ => match finish(code, out, err) {
            Probe::Failed(class) => Located::Failed(class),
            Probe::Ready { .. } | Probe::Cancelled => Located::NotFound,
        },
    }
}

fn first_line(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty());
    sanitize(line.unwrap_or_default(), false)
        .chars()
        .take(MOST_VERSION_CHARS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reader(text: &str) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel();
        tx.send(text.into()).unwrap();
        rx
    }

    fn finished(code: Option<i32>, out: &str, said: &str) -> Probe {
        finish(code, &reader(out), &reader(said))
    }

    #[test]
    fn exit_zero_is_ready_with_the_first_line_of_the_version() {
        assert_eq!(
            finished(Some(0), "\n maitty 1.2.3 \nmore\n", ""),
            Probe::Ready {
                version: "maitty 1.2.3".into()
            }
        );
    }

    #[test]
    fn the_version_text_is_cleaned_and_cut_short() {
        let Probe::Ready { version } =
            finished(Some(0), &format!("\u{1b}[31m{}", "v".repeat(500)), "")
        else {
            panic!("ready");
        };
        assert!(version.chars().count() == MOST_VERSION_CHARS && !version.contains('\u{1b}'));
    }

    #[test]
    fn a_login_ssh_could_not_do_is_needs_login() {
        assert_eq!(
            finished(Some(255), "", "box: Permission denied (publickey).\n"),
            Probe::Failed(FailureClass::Login)
        );
        assert_eq!(
            finished(Some(255), "", "Host key verification failed.\n"),
            Probe::Failed(FailureClass::Login)
        );
    }

    fn located_from(code: Option<i32>, out: &str, said: &str) -> Located {
        located(code, &reader(out), &reader(said))
    }

    #[test]
    fn the_path_command_v_prints_is_where_the_app_is_found() {
        assert_eq!(
            located_from(Some(0), "\n/home/me/.local/bin/maitty\n", ""),
            Located::At("/home/me/.local/bin/maitty".into())
        );
        assert_eq!(located_from(Some(0), "", ""), Located::NotFound);
        assert_eq!(located_from(Some(1), "", ""), Located::NotFound);
    }

    #[test]
    fn a_path_from_the_machine_is_cleaned_and_a_dead_line_is_a_failure() {
        let Located::At(path) = located_from(Some(0), "/bin/\u{1b}[31mmaitty\u{7}", "") else {
            panic!("found");
        };
        assert_eq!(path, "/bin/[31mmaitty");
        assert_eq!(
            located_from(Some(255), "", "box: Permission denied (publickey).\n"),
            Located::Failed(FailureClass::Login)
        );
    }

    #[test]
    fn a_missing_app_is_install_and_a_dead_host_says_why() {
        assert_eq!(
            finished(Some(127), "", "bash: maitty: command not found"),
            Probe::Failed(FailureClass::Install)
        );
        assert_eq!(
            finished(
                Some(255),
                "",
                "ssh: connect to host box port 22: Connection timed out\n"
            ),
            Probe::Failed(FailureClass::Unreachable(
                "ssh: connect to host box port 22: Connection timed out".into()
            ))
        );
    }

    #[test]
    fn what_ssh_said_is_cleaned_before_it_is_kept() {
        let Probe::Failed(FailureClass::Unreachable(why)) =
            finished(Some(255), "", "\u{1b}[2Jconnect failed\u{7}\n")
        else {
            panic!("unreachable");
        };
        assert_eq!(why, "[2Jconnect failed");
    }

    #[test]
    fn a_program_that_cannot_run_is_unreachable_not_a_hang() {
        let cancel = AtomicBool::new(false);
        let argv = vec!["definitely-not-an-ssh-program".to_string()];
        assert!(matches!(
            run(&argv, Duration::from_secs(5), &cancel),
            Probe::Failed(FailureClass::Unreachable(_))
        ));
    }
}
