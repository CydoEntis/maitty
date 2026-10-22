//! A pane's check result as the daemon holds and saves it: the summary every client sees,
//! and the output tail only `Query::CheckOutput` returns. Also what produces one: the
//! project's check command, run in the session's worktree when an agent's turn ends.

use super::persist::{self, CheckSlot, SavedPane};
use super::term::{self, Term};
use super::{Daemon, Ev};
use crate::output_line::{classify, clean, LineKind};
use crate::protocol::{CheckInfo, CheckState, ServerMsg, Status, TermId};
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The most output kept per result. A check's tail is what explains a failure; the rest
/// would only grow the saved files.
pub const CHECK_OUTPUT_MAX_BYTES: usize = 1024 * 1024;

/// The longest reason an override may give; it is shown beside the result, not an essay.
pub use crate::protocol::OVERRIDE_REASON_MAX_CHARS;

/// What the desktop alert says when a check fails.
const CHECK_FAILED_WHAT: &str = "check failed";

/// The shell's exit code for a command it could not find (cmd.exe's is 9009). PowerShell
/// reports a missing command as 1, which cannot be told from a failing one.
const EXIT_COMMAND_NOT_FOUND: i32 = 127;
const EXIT_COMMAND_NOT_FOUND_CMD: i32 = 9009;

/// How often a running check is looked at for its end or its deadline.
const CHECK_POLL: Duration = Duration::from_millis(25);
/// Once the shell has exited, how long to wait for its output to finish arriving. A
/// background process that kept the pipes open must not hold the result back.
const OUTPUT_DRAIN_GRACE: Duration = Duration::from_secs(2);
/// Bytes read from a pipe at a time.
const READ_CHUNK_BYTES: usize = 8 * 1024;
/// Variables the check's process must not see: the pane's secret lets whoever holds it
/// report that pane's status.
const SCRUBBED_ENV: &[&str] = &["SESHI_PANE_TOKEN"];

/// How a check's process ended, before it is read as a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    Exited(i32),
    TimedOut,
    SpawnFailed,
}

/// A command that never ran, or never finished, says nothing about the work, so it is not
/// a failure.
pub fn state_for(outcome: CheckOutcome) -> CheckState {
    match outcome {
        CheckOutcome::Exited(0) => CheckState::Passed,
        CheckOutcome::Exited(EXIT_COMMAND_NOT_FOUND | EXIT_COMMAND_NOT_FOUND_CMD) => CheckState::CouldNotRun,
        CheckOutcome::Exited(_) => CheckState::Failed,
        CheckOutcome::TimedOut | CheckOutcome::SpawnFailed => CheckState::CouldNotRun,
    }
}

/// The last `CHECK_OUTPUT_MAX_BYTES` of `text`, starting on a character boundary.
pub fn cap_output(text: &str) -> String {
    let Some(excess) = text.len().checked_sub(CHECK_OUTPUT_MAX_BYTES) else { return text.to_string() };
    let start = (excess..=text.len()).find(|&i| text.is_char_boundary(i)).unwrap_or(text.len());
    text[start..].to_string()
}

/// The longest first failing line kept: the Inbox row shows it on one line, and a minified
/// bundle's single line must not bloat every snapshot.
pub const FIRST_FAILING_LINE_MAX_CHARS: usize = 200;

/// The line that sums a failed run up: the first that reads as a failure, else the last
/// that printed anything. Safe to draw and cut short.
pub fn first_failing_line(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|l| clean(l).trim().to_string()).filter(|l| !l.is_empty()).collect();
    let picked = lines.iter().find(|l| classify(l) == LineKind::Failure).or(lines.last());
    picked.map_or_else(String::new, |l| l.chars().take(FIRST_FAILING_LINE_MAX_CHARS).collect())
}

/// One check result: the summary clients see, and its output tail.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckRecord {
    pub info: CheckInfo,
    pub output: String,
}

impl CheckRecord {
    pub fn new(info: CheckInfo, output: &str) -> Self {
        Self { info, output: cap_output(output) }
    }
}

/// What a run left behind.
#[derive(Debug)]
pub struct CheckRun {
    pub outcome: CheckOutcome,
    /// The tail of what the command printed (both streams), capped.
    pub output: String,
    pub duration: Duration,
}

/// Run `line` through `shell` in `dir` until it ends or `timeout` passes, and stop its whole
/// process tree if it does not. Its output is returned and never logged.
pub fn run_check(shell: &[String], dir: &Path, line: &str, timeout: Duration) -> CheckRun {
    run_scrubbed(shell, dir, line, timeout, SCRUBBED_ENV)
}

fn run_scrubbed(shell: &[String], dir: &Path, line: &str, timeout: Duration, scrub: &[&str]) -> CheckRun {
    let started = Instant::now();
    let done = |outcome, output: &str| CheckRun { outcome, output: cap_output(output), duration: started.elapsed() };
    let mut child = match check_command(shell, dir, line, scrub).spawn() {
        Ok(child) => child,
        Err(e) => {
            tracing::warn!("couldn't start the check: {e}");
            return done(CheckOutcome::SpawnFailed, "");
        }
    };
    let tree = Tree::of(&child);
    let output = Output::default();
    let readers = [child.stdout.take().map(|r| output.fill_from(r)), child.stderr.take().map(|r| output.fill_from(r))];
    let outcome = wait_for(&mut child, &tree, timeout, started);
    let grace_ends = Instant::now() + OUTPUT_DRAIN_GRACE;
    for reader in readers.into_iter().flatten() {
        while !reader.is_finished() && Instant::now() < grace_ends {
            std::thread::sleep(CHECK_POLL);
        }
    }
    done(outcome, &output.text())
}

fn check_command(shell: &[String], dir: &Path, line: &str, scrub: &[&str]) -> std::process::Command {
    let mut c = crate::proc::shell(shell, line, false);
    c.current_dir(dir).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for var in scrub {
        c.env_remove(var);
    }
    crate::proc::quiet(&mut c);
    #[cfg(unix)]
    {
        std::os::unix::process::CommandExt::process_group(&mut c, 0);
        die_with_daemon(&mut c);
    }
    c
}

/// A check leads its own process group, so the panes' hang-up on a closing PTY never reaches
/// it. Where the kernel can say so, make it die with the daemon: Linux's parent-death signal
/// kills the shell. The signal is tied to the thread that spawned the child, which stays in
/// `run_check` for as long as the child lives. macOS has no such call, so there a check
/// outlives a crashed daemon until it ends or writes to its closed pipes (SIGPIPE); see
/// docs/SECURITY.md.
#[cfg(target_os = "linux")]
fn die_with_daemon(c: &mut std::process::Command) {
    let daemon = std::process::id() as libc::pid_t;
    // SAFETY: runs between fork and exec and calls only async-signal-safe functions.
    unsafe {
        std::os::unix::process::CommandExt::pre_exec(c, move || {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong);
            // The daemon may have died before the signal was armed, and the signal never fires then.
            if libc::getppid() != daemon {
                libc::_exit(1);
            }
            Ok(())
        });
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
fn die_with_daemon(_: &mut std::process::Command) {}

fn wait_for(child: &mut Child, tree: &Tree, timeout: Duration, started: Instant) -> CheckOutcome {
    loop {
        match child.try_wait() {
            // A process a signal ended has no code; it did not pass.
            Ok(Some(status)) => return CheckOutcome::Exited(status.code().unwrap_or(-1)),
            Ok(None) if started.elapsed() >= timeout => {
                tree.kill();
                let _ = child.kill();
                let _ = child.wait();
                return CheckOutcome::TimedOut;
            }
            Ok(None) => std::thread::sleep(CHECK_POLL),
            Err(e) => {
                tracing::warn!("couldn't wait for the check: {e}");
                tree.kill();
                let _ = child.kill();
                let _ = child.wait();
                return CheckOutcome::SpawnFailed;
            }
        }
    }
}

/// What both of a check's streams printed, kept to its last `CHECK_OUTPUT_MAX_BYTES * 2`
/// so a chatty command cannot fill memory.
#[derive(Clone, Default)]
struct Output(Arc<Mutex<Vec<u8>>>);

impl Output {
    fn fill_from<R: std::io::Read + Send + 'static>(&self, mut pipe: R) -> std::thread::JoinHandle<()> {
        let sink = self.0.clone();
        std::thread::spawn(move || {
            let mut chunk = [0u8; READ_CHUNK_BYTES];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                let mut kept = sink.lock().unwrap_or_else(|e| e.into_inner());
                kept.extend_from_slice(&chunk[..n]);
                if let Some(excess) = kept.len().checked_sub(CHECK_OUTPUT_MAX_BYTES * 2) {
                    kept.drain(..excess + CHECK_OUTPUT_MAX_BYTES);
                }
            }
        })
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap_or_else(|e| e.into_inner())).into_owned()
    }
}

/// Everything a check started, so a timeout can stop all of it and not only the shell. On
/// unix that is the process group the check leads; on Windows, a job object.
struct Tree {
    #[cfg(unix)]
    group: libc::pid_t,
    #[cfg(windows)]
    job: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(unix)]
impl Tree {
    fn of(child: &Child) -> Self {
        // `process_group(0)` made the child its own group leader, so its pid is the group.
        Self { group: child.id() as libc::pid_t }
    }

    fn kill(&self) {
        // SAFETY: signals a process group this check created; the child is not yet reaped, so
        // the number cannot belong to anything else.
        unsafe { libc::kill(-self.group, libc::SIGKILL) };
    }
}

#[cfg(windows)]
impl Tree {
    fn of(child: &Child) -> Self {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectExtendedLimitInformation, SetInformationJobObject,
        };
        // SAFETY: plain Win32 calls on a job this function creates and a handle `child` owns.
        // A process that already left (or an error) leaves a job nothing is in; killing it is a no-op.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Self { job };
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            AssignProcessToJobObject(job, child.as_raw_handle());
            Self { job }
        }
    }

    fn kill(&self) {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if !self.job.is_null() {
            // SAFETY: `job` is the handle `of` created and `drop` has not closed yet.
            unsafe { TerminateJobObject(self.job, 1) };
        }
    }
}

#[cfg(windows)]
impl Drop for Tree {
    fn drop(&mut self) {
        if !self.job.is_null() {
            // SAFETY: closes the handle `of` created, once. Closing the job also ends anything left in it.
            unsafe { windows_sys::Win32::Foundation::CloseHandle(self.job) };
        }
    }
}

impl Daemon {
    /// An agent's turn ended: run the project's check in its worktree, off the loop. Nothing
    /// happens for a shell, for a project with no check, or while a check is already running
    /// (the new turn's code is checked by the next turn end). A re-run queued during the turn
    /// is this run.
    pub(super) fn turn_ended(&mut self, term: TermId) {
        let Some(t) = self.terms.get_mut(&term) else { return };
        if std::mem::replace(&mut t.turn_end_handled, true) {
            return;
        }
        let running = t.check.as_ref().is_some_and(|c| c.info.state == CheckState::Checking);
        if t.agent.is_none() || t.spare || running {
            return;
        }
        self.start_check(term, term::unix_now());
    }

    /// Run the check again. While the agent works, the run waits for its turn to end; the
    /// result on show stands until then.
    pub(super) fn rerun_check(&mut self, term: TermId) -> anyhow::Result<()> {
        let t = self.terms.get(&term).ok_or_else(|| anyhow::anyhow!("no pane {term}"))?;
        if t.agent.is_none() || t.spare {
            anyhow::bail!("only an agent's work is checked");
        }
        if self.check_def(term).is_none() {
            anyhow::bail!("this project has no check to run");
        }
        let latest = t.check.as_ref().map(|c| c.info.clone());
        if latest.as_ref().is_some_and(|c| c.state == CheckState::Checking) {
            anyhow::bail!("its check is already running");
        }
        if t.status == Status::Working {
            let Some(record) = self.terms.get_mut(&term).and_then(|t| t.check.as_mut()) else {
                anyhow::bail!("its turn hasn't ended yet; it is checked when it does");
            };
            record.info.rerun_queued = true;
            self.mark_check_changed(term);
            return Ok(());
        }
        let turn_ended_at = latest.map_or_else(term::unix_now, |c| c.after_turn_ended_at);
        self.start_check(term, turn_ended_at);
        Ok(())
    }

    /// Accept a failed result as it is. The result keeps what failed and its output.
    pub(super) fn override_check(&mut self, term: TermId, reason: &str) -> anyhow::Result<()> {
        let reason = reason.trim();
        if reason.chars().count() > OVERRIDE_REASON_MAX_CHARS {
            anyhow::bail!("give a reason of at most {OVERRIDE_REASON_MAX_CHARS} characters");
        }
        let t = self.terms.get_mut(&term).ok_or_else(|| anyhow::anyhow!("no pane {term}"))?;
        let Some(record) = t.check.as_mut().filter(|c| c.info.state == CheckState::Failed) else {
            anyhow::bail!("only a failed check can be overridden");
        };
        record.info.state = CheckState::Overridden;
        record.info.overridden_at = Some(term::unix_now());
        record.info.override_reason = reason.to_string();
        t.check_overridden = Some(record.clone());
        self.mark_check_changed(term);
        Ok(())
    }

    /// Take an override back: the result reads as failed again.
    pub(super) fn undo_override(&mut self, term: TermId) -> anyhow::Result<()> {
        let t = self.terms.get_mut(&term).ok_or_else(|| anyhow::anyhow!("no pane {term}"))?;
        let Some(record) = t.check.as_mut().filter(|c| c.info.state == CheckState::Overridden) else {
            anyhow::bail!("this check isn't overridden");
        };
        record.info.state = CheckState::Failed;
        record.info.overridden_at = None;
        record.info.override_reason.clear();
        t.check_overridden = None;
        self.mark_check_changed(term);
        Ok(())
    }

    fn mark_check_changed(&mut self, term: TermId) {
        if let Some(t) = self.terms.get_mut(&term) {
            t.check_dirty = true;
        }
        self.dirty = true;
    }

    /// The check configured for the pane's project, if any.
    fn check_def(&self, term: TermId) -> Option<crate::config::CheckDef> {
        let t = self.terms.get(&term)?;
        let project = t.head.as_ref().map_or(&t.cwd, |h| &h.main_root);
        self.cfg.check.iter().find(|c| super::same_path(&c.project, project)).cloned()
    }

    /// Start the pane's check, off the loop, replacing its result. A project that no longer
    /// has a check leaves the result alone and drops any queued re-run, which has nothing to run.
    fn start_check(&mut self, term: TermId, after_turn_ended_at: u64) {
        let Some(def) = self.check_def(term) else {
            if let Some(info) = self.terms.get_mut(&term).and_then(|t| t.check.as_mut()).map(|c| &mut c.info) {
                info.rerun_queued = false;
            }
            return;
        };
        let Some(t) = self.terms.get_mut(&term) else { return };
        let dir = t.head.as_ref().map_or_else(|| t.cwd.clone(), |h| h.top.clone());
        let info = CheckInfo {
            command: def.command.clone(),
            state: CheckState::Checking,
            started_at: term::unix_now(),
            after_turn_ended_at,
            ..Default::default()
        };
        t.check = Some(CheckRecord::new(info, ""));
        self.mark_check_changed(term);
        let (shell, tx, timeout) = (self.cfg.shell_command(), self.tx.clone(), def.timeout());
        tokio::task::spawn_blocking(move || {
            let run = run_check(&shell, &dir, &def.command, timeout);
            let _ = tx.blocking_send(Ev::CheckRan { term, run });
        });
    }

    /// A check ended: record how, unless the pane went away or was given a different result
    /// meanwhile.
    pub(super) fn check_ran(&mut self, term: TermId, run: CheckRun) {
        let Some(t) = self.terms.get_mut(&term) else { return };
        let Some(record) = t.check.as_mut().filter(|c| c.info.state == CheckState::Checking) else { return };
        let info = &mut record.info;
        info.state = state_for(run.outcome);
        info.exit_code = match run.outcome {
            CheckOutcome::Exited(code) => Some(code),
            CheckOutcome::TimedOut | CheckOutcome::SpawnFailed => None,
        };
        info.duration_ms = u64::try_from(run.duration.as_millis()).unwrap_or(u64::MAX);
        record.output = cap_output(&run.output);
        info.first_failing_line = if info.state == CheckState::Failed { first_failing_line(&record.output) } else { String::new() };
        t.check_dirty = true;
        self.dirty = true;
        self.check_settled(term);
    }

    /// Say what the turn's end held back, now the check has an answer. A pass (or a check
    /// that never ran) is the finish; a failure is news of its own, never a finish.
    fn check_settled(&mut self, term: TermId) {
        let Some(t) = self.terms.get(&term) else { return };
        let Some(state) = t.check.as_ref().map(|c| c.info.state) else { return };
        let (status, place) = (t.status, t.place());
        match state {
            CheckState::Failed if matches!(status, Status::Done | Status::Idle) => {
                self.broadcast(|c| c.attach, ServerMsg::Attention { term, status: Status::Blocked });
                self.broadcast(|c| c.attach, ServerMsg::Notice(format!("{place}: check failed")));
                self.alert(term, Status::Blocked, CHECK_FAILED_WHAT);
            }
            CheckState::CouldNotRun | CheckState::Passed if status == Status::Done => {
                if state == CheckState::CouldNotRun {
                    self.broadcast(|c| c.attach, ServerMsg::Notice(format!("{place}: check could not run")));
                }
                self.announce(term, Status::Done);
            }
            _ => {}
        }
    }

    /// Output tails that could not be written are written again at the next save.
    pub(super) fn check_outputs_unsaved(&mut self, terms: &[TermId]) {
        for id in terms {
            if let Some(t) = self.terms.get_mut(id) {
                t.check_dirty = true;
            }
        }
    }
}

impl Term {
    /// A turn ended and its check has not answered yet.
    pub fn check_running(&self) -> bool {
        self.check.as_ref().is_some_and(|c| c.info.state == CheckState::Checking)
    }

    /// The work is not known to be good: a check is running or has failed.
    pub fn check_holds_done(&self) -> bool {
        self.check.as_ref().is_some_and(|c| matches!(c.info.state, CheckState::Checking | CheckState::Failed))
    }

    /// Take back the check results a pane had before the server restarted, tails read from `dir`. A check that was
    /// running died with the server, so it reads as not run rather than checking forever.
    /// One check runs per pane for now: the first saved checkpoint.
    pub fn restore_checks(&mut self, dir: &Path, old: TermId, saved: &SavedPane) {
        let record = |infos: &[CheckInfo], slot| {
            let mut info = infos.first()?.clone();
            if info.state == CheckState::Checking {
                info.state = CheckState::CouldNotRun;
            }
            // Nothing is queued after a restart.
            info.rerun_queued = false;
            let output = persist::read_check_output(dir, old, &info.name, slot).unwrap_or_default();
            Some(CheckRecord::new(info, &output))
        };
        self.check = record(&saved.checks, CheckSlot::Latest);
        self.check_overridden = record(&saved.checks_overridden, CheckSlot::Overridden);
        self.check_dirty = self.check.is_some() || self.check_overridden.is_some();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exit_code_a_timeout_and_a_spawn_failure_each_read_as_one_state() {
        let table = [
            (CheckOutcome::Exited(0), CheckState::Passed),
            (CheckOutcome::Exited(1), CheckState::Failed),
            (CheckOutcome::Exited(2), CheckState::Failed),
            (CheckOutcome::Exited(126), CheckState::Failed),
            (CheckOutcome::Exited(-1), CheckState::Failed),
            (CheckOutcome::Exited(127), CheckState::CouldNotRun),
            (CheckOutcome::TimedOut, CheckState::CouldNotRun),
            (CheckOutcome::SpawnFailed, CheckState::CouldNotRun),
        ];
        for (outcome, state) in table {
            assert_eq!(state_for(outcome), state, "{outcome:?}");
        }
    }

    /// A shell whose syntax the tests know: `sh` where there is one, else the user's default.
    fn test_shell() -> Vec<String> {
        if cfg!(unix) { vec!["sh".into()] } else { crate::config::Config::default().shell_command() }
    }

    fn run(line: &str, timeout: Duration) -> CheckRun {
        run_check(&test_shell(), &std::env::temp_dir(), line, timeout)
    }

    const LONG: Duration = Duration::from_secs(60);

    #[test]
    fn a_command_that_exits_reads_by_its_code_and_keeps_what_it_printed() {
        let pass = run("echo \"all good\"; exit 0", LONG);
        assert_eq!((pass.outcome, pass.output.trim()), (CheckOutcome::Exited(0), "all good"));
        // PowerShell does not take `1>&2` the way sh does (the exit code came back 1), so
        // it writes to stderr through the console.
        let to_stderr = if cfg!(windows) { "[Console]::Error.WriteLine('err')" } else { "echo err 1>&2" };
        let fail = run(&format!("echo out; {to_stderr}; exit 3"), LONG);
        assert_eq!(fail.outcome, CheckOutcome::Exited(3));
        assert!(fail.output.contains("out") && fail.output.contains("err"), "both streams: {:?}", fail.output);
    }

    #[test]
    fn the_first_failing_line_is_the_first_failure_else_the_last_line_printed() {
        let long = "x".repeat(FIRST_FAILING_LINE_MAX_CHARS + 50);
        let cut_multibyte = "é".repeat(FIRST_FAILING_LINE_MAX_CHARS + 1);
        let cases: [(&str, &str, String); 9] = [
            ("a marker mid-output", "ok a\nFAIL tests/foo: boom\nFAIL tests/bar\n", "FAIL tests/foo: boom".into()),
            ("an error word anywhere in the line", "running\nthread panicked at src/x.rs\n", "thread panicked at src/x.rs".into()),
            ("no marker takes the last line", "one\ntwo\n\n  \n", "two".into()),
            ("nothing printed", "", String::new()),
            ("only blank lines", "\n  \n\t\n", String::new()),
            ("trimmed", "   FAIL padded   \n", "FAIL padded".into()),
            ("ansi and control codes removed", "\u{1b}[31mFAIL \u{1b}[1mred\u{1b}[0m\u{7}\n", "FAIL red".into()),
            ("a long line is cut to the maximum", &format!("FAIL {long}"), format!("FAIL {long}").chars().take(FIRST_FAILING_LINE_MAX_CHARS).collect()),
            ("a multibyte line is cut on a character", &format!("FAIL {cut_multibyte}"), format!("FAIL {cut_multibyte}").chars().take(FIRST_FAILING_LINE_MAX_CHARS).collect()),
        ];
        for (what, output, want) in cases {
            assert_eq!(first_failing_line(output), want, "{what}");
        }
    }

    #[test]
    fn a_shell_that_cannot_start_reads_as_not_run() {
        let run = run_check(&["/no/such/shell-for-seshi".into()], &std::env::temp_dir(), "exit 0", LONG);
        assert_eq!(run.outcome, CheckOutcome::SpawnFailed);
        assert_eq!(state_for(run.outcome), CheckState::CouldNotRun);
    }

    #[cfg(unix)]
    #[test]
    fn a_missing_command_reads_as_not_run_not_failed() {
        let run = run("seshi-no-such-command-xyz", LONG);
        assert_eq!(run.outcome, CheckOutcome::Exited(EXIT_COMMAND_NOT_FOUND));
        assert_eq!(state_for(run.outcome), CheckState::CouldNotRun);
    }

    #[test]
    fn a_command_over_its_time_is_stopped_and_reads_as_timed_out() {
        let slow = if cfg!(windows) { "Start-Sleep 60" } else { "sleep 60" };
        let started = Instant::now();
        let run = run(slow, Duration::from_millis(200));
        assert_eq!(run.outcome, CheckOutcome::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(30), "it did not wait out the command");
    }

    /// Whether process `pid` is still running (a zombie is not).
    #[cfg(unix)]
    fn alive(pid: &str) -> bool {
        let out = std::process::Command::new("ps").args(["-o", "stat=", "-p", pid]).output().expect("ps");
        let stat = String::from_utf8_lossy(&out.stdout);
        !stat.trim().is_empty() && !stat.trim().starts_with('Z')
    }

    #[cfg(unix)]
    #[test]
    fn a_timed_out_check_leaves_no_child_running() {
        let dir = persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        let pidfile = dir.path().join("grandchild.pid");
        // The shell starts a child of its own and waits on it: killing only the shell would
        // leave `sleep` behind.
        let line = format!("sleep 60 & echo $! > '{}'; wait", pidfile.display());
        let run = run_check(&test_shell(), dir.path(), &line, Duration::from_millis(500));
        assert_eq!(run.outcome, CheckOutcome::TimedOut);
        let pid = std::fs::read_to_string(&pidfile).expect("the shell started its child").trim().to_string();
        let gone_by = Instant::now() + Duration::from_secs(10);
        while alive(&pid) && Instant::now() < gone_by {
            std::thread::sleep(CHECK_POLL);
        }
        assert!(!alive(&pid), "the child {pid} outlived the check");
    }

    #[cfg(windows)]
    #[test]
    fn a_timed_out_check_leaves_no_child_running() {
        let dir = persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        let pidfile = dir.path().join("grandchild.pid");
        let line = format!(
            "$p = Start-Process -PassThru -WindowStyle Hidden powershell -ArgumentList '-NoProfile','-Command','Start-Sleep 60'; $p.Id | Set-Content '{}'; Start-Sleep 60",
            pidfile.display()
        );
        let run = run_check(&crate::config::Config::default().shell_command(), dir.path(), &line, Duration::from_secs(5));
        assert_eq!(run.outcome, CheckOutcome::TimedOut);
        let pid = std::fs::read_to_string(&pidfile).expect("the shell started its child").trim().to_string();
        let gone_by = Instant::now() + Duration::from_secs(10);
        let alive = || std::process::Command::new("tasklist").args(["/FI", &format!("PID eq {pid}"), "/NH"]).output().is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(&pid));
        while alive() && Instant::now() < gone_by {
            std::thread::sleep(CHECK_POLL);
        }
        assert!(!alive(), "the child {pid} outlived the check");
    }

    #[cfg(target_os = "linux")]
    const KILLED_DAEMON_ENV: &str = "SESHI_TEST_CHECK_AS_DAEMON";

    /// Run as the stand-in daemon for `a_check_dies_with_the_daemon_that_started_it`: start a
    /// check that writes its pid and waits, and never return.
    #[cfg(target_os = "linux")]
    fn be_the_killed_daemon(pidfile: &str) {
        let line = format!("echo $$ > '{pidfile}'; exec sleep 60");
        run_check(&test_shell(), &std::env::temp_dir(), &line, LONG);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_check_dies_with_the_daemon_that_started_it() {
        if let Ok(pidfile) = std::env::var(KILLED_DAEMON_ENV) {
            return be_the_killed_daemon(&pidfile);
        }
        let dir = persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        let pidfile = dir.path().join("check.pid");
        let mut daemon = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "daemon::check::tests::a_check_dies_with_the_daemon_that_started_it", "--nocapture"])
            .env(KILLED_DAEMON_ENV, &pidfile)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start the stand-in daemon");
        let started_by = Instant::now() + Duration::from_secs(30);
        while !pidfile.exists() && Instant::now() < started_by {
            std::thread::sleep(CHECK_POLL);
        }
        let pid = std::fs::read_to_string(&pidfile).expect("the check started").trim().to_string();
        assert!(alive(&pid), "the check runs while its daemon does");
        daemon.kill().unwrap();
        let _ = daemon.wait();
        let gone_by = Instant::now() + Duration::from_secs(10);
        while alive(&pid) && Instant::now() < gone_by {
            std::thread::sleep(CHECK_POLL);
        }
        let outlived = alive(&pid);
        if outlived {
            // SAFETY: stops the process this test started so a failure leaves nothing behind.
            unsafe { libc::kill(pid.parse().unwrap(), libc::SIGKILL) };
        }
        assert!(!outlived, "the check {pid} outlived its daemon");
    }

    #[test]
    fn the_check_does_not_inherit_the_panes_token_but_keeps_the_rest_of_the_environment() {
        assert!(SCRUBBED_ENV.contains(&"SESHI_PANE_TOKEN"), "the secret is scrubbed");
        // Names nothing else reads, so setting them cannot disturb a test running beside this one.
        let (secret, plain) = ("SESHI_TEST_CHECK_SECRET", "SESHI_TEST_CHECK_PLAIN");
        // SAFETY: only this test touches these two names.
        unsafe {
            std::env::set_var(secret, "hunter2");
            std::env::set_var(plain, "visible");
        }
        let print = if cfg!(windows) { "Get-ChildItem Env: | ForEach-Object { \"$($_.Name)=$($_.Value)\" }" } else { "env" };
        let run = run_scrubbed(&test_shell(), &std::env::temp_dir(), print, LONG, &[secret]);
        assert_eq!(run.outcome, CheckOutcome::Exited(0));
        assert!(run.output.contains("SESHI_TEST_CHECK_PLAIN=visible"), "the rest passes through");
        assert!(!run.output.contains(secret) && !run.output.contains("hunter2"), "the scrubbed name is gone: {}", run.output);
    }

    #[test]
    fn a_chatty_command_keeps_only_the_tail_of_its_output() {
        let line = if cfg!(windows) { "1..200000 | ForEach-Object { 'line ' + $_ }" } else { "seq 1 200000 | sed 's/^/line /'" };
        let run = run(line, LONG);
        assert_eq!(run.outcome, CheckOutcome::Exited(0));
        assert!(run.output.len() <= CHECK_OUTPUT_MAX_BYTES, "capped: {}", run.output.len());
        assert!(run.output.trim_end().ends_with("line 200000"), "the end is what is kept");
    }

    #[test]
    fn output_over_the_cap_keeps_its_tail_whole_characters_only() {
        assert_eq!(cap_output("short"), "short");
        let exact = "x".repeat(CHECK_OUTPUT_MAX_BYTES);
        assert_eq!(cap_output(&exact), exact, "exactly the cap is kept whole");
        // The cut would fall inside the second 3-byte character, so that one is dropped whole.
        let text = format!("{}{}", "\u{2713}".repeat(2), "y".repeat(CHECK_OUTPUT_MAX_BYTES - 1));
        assert_eq!(cap_output(&text), "y".repeat(CHECK_OUTPUT_MAX_BYTES - 1));
    }
}
