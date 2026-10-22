//! A pane's check result as the daemon holds and saves it: the summary every client sees,
//! and the output tail only `Query::CheckOutput` returns. Also what produces one: the
//! project's check command, run in the session's worktree when an agent's turn ends.

use super::persist::{self, CheckSlot, SavedPane};
use super::term::{self, Term};
use super::check_source::{Gate, Resolved};
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
/// What it says when a repo's check command waits for your approval.
const NEEDS_APPROVAL_WHAT: &str = "needs you";

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
    /// Lines it printed in all, before the tail was capped.
    pub lines: u64,
    pub duration: Duration,
}

/// Run `line` through `shell` in `dir` until it ends or `timeout` passes, and stop its whole
/// process tree if it does not. Its output is returned and never logged.
pub fn run_check(shell: &[String], dir: &Path, line: &str, timeout: Duration) -> CheckRun {
    run_scrubbed(shell, dir, line, timeout, SCRUBBED_ENV)
}

fn run_scrubbed(shell: &[String], dir: &Path, line: &str, timeout: Duration, scrub: &[&str]) -> CheckRun {
    let started = Instant::now();
    let done = |outcome, output: &Output| {
        let text = output.text();
        let unended = u64::from(!text.is_empty() && !text.ends_with('\n'));
        CheckRun { outcome, output: cap_output(&text), lines: output.newlines() + unended, duration: started.elapsed() }
    };
    let mut child = match check_command(shell, dir, line, scrub).spawn() {
        Ok(child) => child,
        Err(e) => {
            tracing::warn!("couldn't start the check: {e}");
            return done(CheckOutcome::SpawnFailed, &Output::default());
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
    done(outcome, &output)
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
/// Its line breaks are counted in full, so a capped tail can say how much it leaves out.
#[derive(Clone, Default)]
struct Output(Arc<Mutex<Vec<u8>>>, Arc<std::sync::atomic::AtomicU64>);

impl Output {
    fn fill_from<R: std::io::Read + Send + 'static>(&self, mut pipe: R) -> std::thread::JoinHandle<()> {
        let (sink, newlines) = (self.0.clone(), self.1.clone());
        std::thread::spawn(move || {
            let mut chunk = [0u8; READ_CHUNK_BYTES];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                let breaks = chunk[..n].iter().filter(|&&b| b == b'\n').count() as u64;
                newlines.fetch_add(breaks, std::sync::atomic::Ordering::Relaxed);
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

    fn newlines(&self) -> u64 {
        self.1.load(std::sync::atomic::Ordering::Relaxed)
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

/// One checkpoint of a session: its latest result, and the failed one you overrode (kept
/// until you undo it).
#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {
    pub latest: CheckRecord,
    pub overridden: Option<CheckRecord>,
}

impl From<CheckRecord> for Checkpoint {
    fn from(latest: CheckRecord) -> Self {
        Self { latest, overridden: None }
    }
}

/// A session's checkpoints, in project order, and the run that is filling them.
#[derive(Debug, Default)]
pub struct Checks {
    pub points: Vec<Checkpoint>,
    /// How many of its turns have ended: the `turn` its results are about.
    pub turn: u32,
    /// The latest run started; results of an older one are dropped.
    pub(super) run: u64,
    /// The checkpoint the latest run is for, or all of them.
    run_only: Option<String>,
    /// A result changed since its output was last written to disk.
    pub dirty: bool,
    /// The turn whose waiting-for-approval was already said, so it is said once.
    approval_told: Option<u32>,
}

impl Checks {
    pub fn infos(&self) -> Vec<CheckInfo> {
        self.points.iter().map(|p| p.latest.info.clone()).collect()
    }

    /// The checkpoint the session is reported by (see `protocol::headline`).
    pub fn headline(&self) -> Option<&CheckRecord> {
        self.headline_index().map(|i| &self.points[i].latest)
    }

    fn headline_index(&self) -> Option<usize> {
        let infos = self.infos();
        let top = crate::protocol::headline(&infos)?;
        infos.iter().position(|i| std::ptr::eq(i, top))
    }

    pub fn named(&self, name: &str) -> Option<&Checkpoint> {
        self.points.iter().find(|p| p.latest.info.name == name)
    }

    fn named_mut(&mut self, name: &str) -> Option<&mut Checkpoint> {
        self.points.iter_mut().find(|p| p.latest.info.name == name)
    }

    fn any(&self, states: &[CheckState]) -> bool {
        self.points.iter().any(|p| states.contains(&p.latest.info.state))
    }

    /// A run has not answered for every checkpoint yet.
    pub fn running(&self) -> bool {
        self.any(&[CheckState::Checking, CheckState::Pending])
    }

    /// The work is not known to be good: a checkpoint is running, waiting, failed or waiting
    /// for your approval.
    pub fn holds_done(&self) -> bool {
        self.any(&[CheckState::Checking, CheckState::Pending, CheckState::Failed, CheckState::NeedsApproval])
    }

    /// The first checkpoint waiting its turn starts now.
    fn start_next(&mut self) {
        if self.any(&[CheckState::Checking]) {
            return;
        }
        if let Some(next) = self.points.iter_mut().find(|p| p.latest.info.state == CheckState::Pending) {
            next.latest.info.state = CheckState::Checking;
            next.latest.info.started_at = term::unix_now();
        }
    }
}

/// What a run of a session's checkpoints knows before it starts.
struct RunPlan {
    dir: std::path::PathBuf,
    turn: u32,
    after_turn_ended_at: u64,
}

impl RunPlan {
    fn info(&self, def: &crate::config::CheckDef, state: CheckState) -> CheckInfo {
        CheckInfo {
            name: def.name().to_string(),
            command: def.command.clone(),
            state,
            started_at: if state == CheckState::Checking { term::unix_now() } else { 0 },
            after_turn_ended_at: self.after_turn_ended_at,
            turn: self.turn,
            cwd: self.dir.clone(),
            ..Default::default()
        }
    }
}

impl Daemon {
    /// An agent's turn ended: run the project's checkpoints in its worktree, off the loop.
    /// Nothing happens for a shell, for a project with no check, or while a run is still
    /// going (the new turn's code is checked by the next turn end). A re-run queued during
    /// the turn is this run.
    pub(super) fn turn_ended(&mut self, term: TermId) {
        let Some(t) = self.terms.get_mut(&term) else { return };
        if std::mem::replace(&mut t.turn_end_handled, true) {
            return;
        }
        if t.agent.is_none() || t.spare {
            return;
        }
        t.checks.turn += 1;
        if t.checks.running() {
            return;
        }
        self.start_checks(term, None, term::unix_now());
    }

    /// Run checkpoint `name` again, or all of them. While the agent works, the run waits for
    /// its turn to end; the results on show stand until then.
    pub(super) fn rerun_check(&mut self, term: TermId, name: Option<&str>) -> anyhow::Result<()> {
        let defs = self.check_defs(term);
        let t = self.terms.get_mut(&term).ok_or_else(|| anyhow::anyhow!("no pane {term}"))?;
        if t.agent.is_none() || t.spare {
            anyhow::bail!("only an agent's work is checked");
        }
        if defs.is_empty() {
            anyhow::bail!("this project has no check to run");
        }
        if let Some(name) = name
            && !defs.iter().any(|d| d.name() == name)
            && t.checks.named(name).is_none()
        {
            anyhow::bail!("this project has no checkpoint {name}");
        }
        if t.checks.running() {
            anyhow::bail!("its check is already running");
        }
        if t.status == Status::Working {
            let mut queued = false;
            for p in t.checks.points.iter_mut().filter(|p| name.is_none_or(|n| p.latest.info.name == n)) {
                p.latest.info.rerun_queued = true;
                queued = true;
            }
            if !queued {
                anyhow::bail!("its turn hasn't ended yet; it is checked when it does");
            }
            self.mark_check_changed(term);
            return Ok(());
        }
        let turn_ended_at = t.checks.headline().map_or_else(term::unix_now, |c| c.info.after_turn_ended_at);
        self.start_checks(term, name.map(str::to_string), turn_ended_at);
        Ok(())
    }

    /// Accept a failed result as it is (checkpoint `name`, else the headline one). The result
    /// keeps what failed and its output.
    pub(super) fn override_check(&mut self, term: TermId, name: Option<&str>, reason: &str, by: String) -> anyhow::Result<()> {
        let reason = reason.trim();
        if reason.chars().count() > OVERRIDE_REASON_MAX_CHARS {
            anyhow::bail!("give a reason of at most {OVERRIDE_REASON_MAX_CHARS} characters");
        }
        let t = self.terms.get_mut(&term).ok_or_else(|| anyhow::anyhow!("no pane {term}"))?;
        let at = match name {
            Some(name) => t.checks.points.iter().position(|p| p.latest.info.name == name),
            None => t.checks.headline_index(),
        };
        let Some(point) = at.map(|i| &mut t.checks.points[i]).filter(|p| p.latest.info.state == CheckState::Failed) else {
            anyhow::bail!("only a failed check can be overridden");
        };
        let info = &mut point.latest.info;
        info.state = CheckState::Overridden;
        info.overridden_at = Some(term::unix_now());
        info.override_reason = reason.to_string();
        info.overridden_by = by;
        point.overridden = Some(point.latest.clone());
        self.mark_check_changed(term);
        Ok(())
    }

    /// Take an override back (checkpoint `name`, else the first overridden one): the result
    /// reads as failed again.
    pub(super) fn undo_override(&mut self, term: TermId, name: Option<&str>) -> anyhow::Result<()> {
        let t = self.terms.get_mut(&term).ok_or_else(|| anyhow::anyhow!("no pane {term}"))?;
        let Some(point) = t
            .checks
            .points
            .iter_mut()
            .filter(|p| name.is_none_or(|n| p.latest.info.name == n))
            .find(|p| p.latest.info.state == CheckState::Overridden)
        else {
            anyhow::bail!("this check isn't overridden");
        };
        let info = &mut point.latest.info;
        info.state = CheckState::Failed;
        info.overridden_at = None;
        info.override_reason.clear();
        info.overridden_by.clear();
        point.overridden = None;
        self.mark_check_changed(term);
        Ok(())
    }

    /// Who an override is from: you, or the pane a client acts from, by its name.
    pub(super) fn overrider(&self, client: super::ClientId) -> String {
        let from = self.clients.get(&client).and_then(|c| c.from).and_then(|p| self.terms.get(&p));
        from.map_or_else(|| "you".to_string(), Term::who)
    }

    /// The output tail of checkpoint `name`, else of the headline one.
    pub(super) fn check_output(&self, term: TermId, name: Option<&str>) -> Option<&str> {
        let checks = &self.terms.get(&term)?.checks;
        let record = match name {
            Some(name) => checks.named(name).map(|p| &p.latest),
            None => checks.headline(),
        };
        record.map(|r| r.output.as_str())
    }

    pub(super) fn mark_check_changed(&mut self, term: TermId) {
        if let Some(t) = self.terms.get_mut(&term) {
            t.checks.dirty = true;
        }
        self.dirty = true;
    }

    /// The checks you configured for the pane's project, in order.
    fn check_defs(&self, term: TermId) -> Vec<crate::config::CheckDef> {
        let Some(t) = self.terms.get(&term) else { return Vec::new() };
        let project = t.head.as_ref().map_or(&t.cwd, |h| &h.main_root);
        self.cfg.check.iter().filter(|c| super::same_path(&c.project, project)).cloned().collect()
    }

    /// Start a run of checkpoint `only` (or all of them), replacing their results. They read
    /// as checking at once; which ones really run is settled off the loop, where the
    /// project's checks are resolved and gated. A project that no longer has a check leaves
    /// the results alone and drops any queued re-run, which has nothing to run.
    fn start_checks(&mut self, term: TermId, only: Option<String>, after_turn_ended_at: u64) {
        let defs: Vec<_> = self.check_defs(term).into_iter().filter(|d| only.as_deref().is_none_or(|n| d.name() == n)).collect();
        let (user, resolver, shell, tx) = (self.cfg.check.clone(), self.resolver, self.cfg.shell_command(), self.tx.clone());
        let Some(t) = self.terms.get_mut(&term) else { return };
        if defs.is_empty() && only.as_deref().is_none_or(|n| t.checks.named(n).is_none()) {
            for p in &mut t.checks.points {
                p.latest.info.rerun_queued = false;
            }
            return;
        }
        let (dir, project) = match &t.head {
            Some(h) => (h.top.clone(), h.main_root.clone()),
            None => (t.cwd.clone(), t.cwd.clone()),
        };
        let plan = RunPlan { dir: dir.clone(), turn: t.checks.turn, after_turn_ended_at };
        let checks = &mut t.checks;
        checks.run += 1;
        checks.run_only = only.clone();
        let run = checks.run;
        match &only {
            None => {
                let mut old = std::mem::take(&mut checks.points);
                checks.points = defs
                    .iter()
                    .enumerate()
                    .map(|(i, def)| {
                        let state = if i == 0 { CheckState::Checking } else { CheckState::Pending };
                        let overridden = old.iter_mut().find(|p| p.latest.info.name == def.name()).and_then(|p| p.overridden.take());
                        Checkpoint { latest: CheckRecord::new(plan.info(def, state), ""), overridden }
                    })
                    .collect();
            }
            Some(name) => {
                let fresh = defs.first().map(|def| plan.info(def, CheckState::Checking));
                if let Some(point) = checks.named_mut(name) {
                    let info = fresh.unwrap_or_else(|| CheckInfo { state: CheckState::Checking, started_at: term::unix_now(), cwd: plan.dir.clone(), ..point.latest.info.clone() });
                    point.latest = CheckRecord::new(CheckInfo { after_turn_ended_at, turn: plan.turn, ..info }, "");
                } else if let Some(info) = fresh {
                    checks.points.push(CheckRecord::new(info, "").into());
                }
            }
        }
        self.mark_check_changed(term);
        tokio::task::spawn_blocking(move || {
            let approvals = super::approvals::Approvals::load(&super::approvals::path()).unwrap_or_else(|e| {
                tracing::warn!("reading check approvals: {e:#}");
                super::approvals::Approvals::default()
            });
            let resolved: Vec<_> = resolver(&user, &project, &dir, &approvals).into_iter().filter(|r| only.as_deref().is_none_or(|n| r.def.name() == n)).collect();
            let to_run: Vec<_> = resolved.iter().filter(|r| r.gate == Gate::Run).map(|r| r.def.clone()).collect();
            let _ = tx.blocking_send(Ev::ChecksResolved { term, run, resolved });
            for def in to_run {
                let result = run_check(&shell, &dir, &def.command, def.timeout());
                let _ = tx.blocking_send(Ev::CheckRan { term, run, name: def.name().to_string(), result });
            }
        });
    }

    /// The run's checkpoints are known and gated: put them in place, in order. The ones that
    /// may run wait their turn; the rest are settled already.
    pub(super) fn checks_resolved(&mut self, term: TermId, run: u64, resolved: Vec<Resolved>) {
        let Some(t) = self.terms.get_mut(&term).filter(|t| t.checks.run == run) else { return };
        let checks = &mut t.checks;
        match checks.run_only.clone() {
            // A single checkpoint's run leaves the others where they were.
            Some(only) => {
                if let Some(point) = checks.named_mut(&only) {
                    match resolved.first() {
                        Some(r) => point.gate(r),
                        None => point.latest.info.state = CheckState::CouldNotRun,
                    }
                }
            }
            None => {
                let mut old = std::mem::take(&mut checks.points);
                checks.points = resolved
                    .iter()
                    .map(|r| {
                        let name = r.def.name();
                        let mut point = match old.iter().position(|p| p.latest.info.name == name) {
                            Some(i) => old.remove(i),
                            None => CheckRecord::new(CheckInfo { name: name.to_string(), state: CheckState::Pending, ..Default::default() }, "").into(),
                        };
                        point.gate(r);
                        point
                    })
                    .collect();
            }
        }
        checks.start_next();
        self.mark_check_changed(term);
        self.check_settled(term);
    }

    /// A checkpoint's run ended: record how, unless the pane went away or a newer run
    /// replaced it meanwhile, and start the next one.
    pub(super) fn check_ran(&mut self, term: TermId, run: u64, name: &str, result: CheckRun) {
        let Some(t) = self.terms.get_mut(&term).filter(|t| t.checks.run == run) else { return };
        let Some(point) = t.checks.named_mut(name).filter(|p| p.latest.info.state == CheckState::Checking) else { return };
        let record = &mut point.latest;
        let info = &mut record.info;
        info.state = state_for(result.outcome);
        info.exit_code = match result.outcome {
            CheckOutcome::Exited(code) => Some(code),
            CheckOutcome::TimedOut | CheckOutcome::SpawnFailed => None,
        };
        info.duration_ms = u64::try_from(result.duration.as_millis()).unwrap_or(u64::MAX);
        info.line_count = result.lines;
        record.output = cap_output(&result.output);
        info.first_failing_line = if info.state == CheckState::Failed { first_failing_line(&record.output) } else { String::new() };
        t.checks.start_next();
        self.mark_check_changed(term);
        self.check_settled(term);
    }

    /// Say what the turn's end held back, once every checkpoint has an answer. A failure is
    /// news of its own, never a finish; so is a command waiting for your approval, said once
    /// a turn. Otherwise the turn is finished.
    fn check_settled(&mut self, term: TermId) {
        let Some(t) = self.terms.get_mut(&term).filter(|t| !t.checks.running()) else { return };
        let (status, place, turn) = (t.status, t.place(), t.checks.turn);
        let first = |state| t.checks.points.iter().map(|p| &p.latest.info).find(|i| i.state == state).map(|i| i.command.clone());
        let (failed, approve) = (first(CheckState::Failed), first(CheckState::NeedsApproval));
        let (could_not_run, never) = (first(CheckState::CouldNotRun), first(CheckState::Never));
        let ended = matches!(status, Status::Done | Status::Idle);
        if failed.is_some() {
            if ended {
                self.broadcast(|c| c.attach, ServerMsg::Attention { term, status: Status::Blocked });
                self.broadcast(|c| c.attach, ServerMsg::Notice(format!("{place}: check failed")));
                self.alert(term, Status::Blocked, CHECK_FAILED_WHAT);
            }
        } else if let Some(command) = approve {
            if ended && t.checks.approval_told.replace(turn) != Some(turn) {
                self.broadcast(|c| c.attach, ServerMsg::Attention { term, status: Status::Blocked });
                self.broadcast(|c| c.attach, ServerMsg::Notice(format!("{place}: approve check: {command}")));
                self.alert(term, Status::Blocked, NEEDS_APPROVAL_WHAT);
            }
        } else if status == Status::Done {
            if could_not_run.is_some() {
                self.broadcast(|c| c.attach, ServerMsg::Notice(format!("{place}: check could not run")));
            } else if never.is_some() {
                self.broadcast(|c| c.attach, ServerMsg::Notice(format!("{place}: not checked: never for this repo")));
            }
            self.announce(term, Status::Done);
        }
    }

    /// Output tails that could not be written are written again at the next save.
    pub(super) fn check_outputs_unsaved(&mut self, terms: &[TermId]) {
        for id in terms {
            if let Some(t) = self.terms.get_mut(id) {
                t.checks.dirty = true;
            }
        }
    }
}

impl Checkpoint {
    /// Take on what resolving said of it: its command and source, and whether it runs.
    fn gate(&mut self, r: &Resolved) {
        let info = &mut self.latest.info;
        info.command = r.def.command.clone();
        info.source = r.source.clone();
        info.previous_command = None;
        info.state = match &r.gate {
            Gate::Run if info.state == CheckState::Checking => CheckState::Checking,
            Gate::Run => CheckState::Pending,
            Gate::NeedsApproval { previous } => {
                info.previous_command = previous.clone();
                CheckState::NeedsApproval
            }
            Gate::Never => CheckState::Never,
        };
        if r.gate != Gate::Run {
            info.started_at = 0;
        }
    }
}

impl Term {
    /// A turn ended and its checkpoints have not all answered yet.
    pub fn check_running(&self) -> bool {
        self.checks.running()
    }

    /// The work is not known to be good (see `Checks::holds_done`).
    pub fn check_holds_done(&self) -> bool {
        self.checks.holds_done()
    }

    /// What an override from this pane is signed with: its name, else its folder's.
    fn who(&self) -> String {
        [&self.label, &self.name, &self.first_prompt].into_iter().find(|n| !n.trim().is_empty()).cloned().unwrap_or_else(|| self.place())
    }

    /// Take back the checkpoints a pane had before the server restarted, tails read from
    /// `dir`. A run that was going died with the server, so its checkpoints read as not run
    /// rather than checking forever.
    pub fn restore_checks(&mut self, dir: &Path, old: TermId, saved: &SavedPane) {
        let record = |info: &CheckInfo, slot| {
            let mut info = info.clone();
            if matches!(info.state, CheckState::Checking | CheckState::Pending) {
                info.state = CheckState::CouldNotRun;
            }
            // Nothing is queued after a restart.
            info.rerun_queued = false;
            let output = persist::read_check_output(dir, old, &info.name, slot).unwrap_or_default();
            CheckRecord::new(info, &output)
        };
        self.checks.points = saved
            .checks
            .iter()
            .map(|info| Checkpoint {
                latest: record(info, CheckSlot::Latest),
                overridden: saved.checks_overridden.iter().find(|o| o.name == info.name).map(|o| record(o, CheckSlot::Overridden)),
            })
            .collect();
        self.checks.turn = saved.checks.iter().map(|c| c.turn).max().unwrap_or(0);
        self.checks.dirty = !self.checks.points.is_empty();
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
