//! Agent status: hook reports (checked on their own thread) and screen detection.

use super::*;

/// A status report waiting for its process chain to be checked.
pub(super) struct HookJob {
    pub(super) msg: ClientMsg,
    pub(super) pid: u32,
    /// The pane's own process, when there's a chain to check.
    pub(super) root: Option<u32>,
    pub(super) known: std::collections::HashSet<u32>,
}

/// One thread checks status reports' process chains, in the order they came, and hands
/// them back to the daemon loop.
pub(super) fn hook_thread(tx: mpsc::Sender<Ev>) -> std::sync::mpsc::Sender<HookJob> {
    let (jobs, rx) = std::sync::mpsc::channel::<HookJob>();
    let spawned = std::thread::Builder::new().name("hook-check".into()).spawn(move || {
        for job in rx {
            let (verdict, chain) = match job.root {
                Some(root) => scan::descends_from(job.pid, root, &job.known),
                None => (None, Vec::new()),
            };
            if tx.blocking_send(Ev::HookChecked { msg: job.msg, verdict, chain }).is_err() {
                break;
            }
        }
    });
    if let Err(e) = spawned {
        tracing::error!("couldn't start the hook thread: {e}");
    }
    jobs
}

/// How long after your last key, with the pane quiet, a question that's no longer on screen
/// counts as dismissed.
pub(super) const QUESTION_GONE_AFTER: Duration = Duration::from_secs(2);
/// How long a "needs you" with no question anywhere on screen lasts (you typed nothing).
pub(super) const UNASKED_QUESTION_GONE_AFTER: Duration = Duration::from_secs(8);
/// A numbered choice an agent offers ("❯ 1. Yes"): a question, whatever its wording.
static CHOICES: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"(?m)^[\s│┃]*[❯>]?\s*1\.\s+\S").unwrap());
/// Codex can keep working while an asynchronous question waits in its composer.
static CODEX_QUEUED_QUESTION: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(crate::config::CODEX_QUEUED_QUESTION_PATTERN).unwrap());
const CODEX_QUESTION_WHY: &str = "screen: Codex has an unanswered question";
/// A "done" held while subagents run is let through once none has been heard from for this
/// long (one died without saying so). Background agents can run, and wait on CI, for a while.
/// After a queued follow-up goes in, the agent has this long to say it has started on it
/// before the next one may go (an agent with no hooks never says).
const QUEUED_TURN_STARTS_WITHIN: Duration = Duration::from_secs(60);
pub(super) const SUBAGENTS_QUIET_AT_MOST: Duration = Duration::from_secs(60 * 60);
/// A permission ping this soon after a "working" is Claude repeating one already answered.
const REPEATED_PROMPT_WITHIN: Duration = Duration::from_secs(5);
/// Agents whose hooks report only the end of a turn.
const HOOKS_END_ONLY: &[&str] = &["codex"];
/// These adapters report every reply/dismissal, including custom extension dialogs.
const NATIVE_PROMPT_LIFECYCLE: &[&str] = &["pi", "omp", "mastracode", "opencode", "kilo", "kimi"];
const FULL_LIFECYCLE_HOOKS: &[&str] = &["pi", "omp", "mastracode", "opencode", "kilo", "kimi"];
const VISIBLE_BLOCKER_WHY: &str = "screen: a live agent dialog needs you";
/// The progress indicator gone this long with no turn-end: the turn was cancelled (Esc).
const CANCELLED_AFTER: Duration = Duration::from_secs(2);
/// An agent read from its screen (no hooks) is done only once its screen has been still this
/// long: working ones redraw a timer every second.
const SCREEN_DONE_QUIET: Duration = Duration::from_secs(5);

impl Daemon {
    /// A status report whose sender checked out (`verdict`: did its process chain lead to
    /// the pane; None when it couldn't be traced and the secret decides).
    pub(super) fn apply_hook(&mut self, msg: ClientMsg, verdict: Option<bool>, chain: Vec<u32>) {
        let ClientMsg::Hook { term, agent, status, session, cwd, prompt, said, subagent, event, pid, transcript, model, name, .. } = msg else { return };
        match verdict {
            Some(false) => {
                tracing::info!("ignoring a status report for pane {term} from pid {pid} outside it");
                return;
            }
            Some(true) => {
                if let Some(t) = self.terms.get_mut(&term) {
                    t.trusted.extend(chain);
                }
            }
            None => {}
        }
                if let Some(t) = self.terms.get_mut(&term) {
                    // A helper's first event can be the first reliable source of the parent ID.
                    // Set the thread before recording its helper, or the later ID check clears it.
                    if let Some(id) = session.as_ref().filter(|s| {
                        !s.is_empty() && s.len() < 128 && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
                    }) {
                        if t.session.as_deref() != Some(id) {
                            t.subagents.clear();
                            t.subagent_hooks_seen = false;
                        }
                        t.session = Some(id.clone());
                    }
                    if let Some(sa) = subagent {
                        if agent == "codex" && matches!(event.as_str(), "SubagentStart" | "SubagentStop") {
                            t.subagent_hooks_seen = true;
                        }
                        t.subagent_seen = Some(Instant::now());
                        if sa.start {
                            // Its start, or news from one already running (listed once; one
                            // missed while seshi was away joins the list here).
                            if sa.id.is_empty() || !t.subagents.iter().any(|(id, _)| *id == sa.id) {
                                t.subagents.push((sa.id, sa.kind));
                            }
                        } else if let Some(i) = t.subagents.iter().position(|(id, _)| !sa.id.is_empty() && *id == sa.id) {
                            t.subagents.remove(i);
                        } else if sa.id.is_empty() {
                            // A stop with no id can only be for the last one started.
                            t.subagents.pop();
                        }
                        // A stop for an id we never saw start (it began before seshi was
                        // listening) must not remove a different helper that is still running.
                        tracing::debug!("pane {term}: {} helper(s) running after {event}", t.subagents.len());
                        self.dirty = true;
                    }
                    // A turn over (or a new session) has no subagents left; a "done" while some
                    // still run is held instead (see `hook`).
                    if matches!(status, HookStatus::Gone) || (matches!(status, HookStatus::Idle) && event != "SessionStart") {
                        t.subagents.clear();
                    }
                    if let Some(s) = said {
                        t.said = s;
                    }
                    if let Some(tr) = transcript.filter(|p| p.is_file()) {
                        t.transcript = Some(tr);
                    }
                    if let Some(p) = prompt.filter(|p| !p.trim().is_empty()) {
                        if t.first_prompt.is_empty() {
                            t.first_prompt = p.clone();
                        }
                        t.summary = p;
                    }
                    if let Some(m) = model {
                        t.model = m;
                    }
                    if let Some(n) = name {
                        t.name = n;
                    }
                    // Ids end up in a shell command on restore: keep them boring.
                    if let Some(id) = session.filter(|s| {
                        !s.is_empty() && s.len() < 128 && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
                    }) {
                        if t.session.as_deref() != Some(&id) {
                            t.subagents.clear();
                        }
                        t.session = Some(id);
                    }
                    // An agent stays in the group of the folder it started in: only its first
                    // report places it, so its row doesn't jump as it works in other folders.
                    if let Some(c) = cwd.filter(|c| c.is_dir() && !(t.hooked && t.agent.is_some())) {
                        t.cwd = c;
                        t.cwd_reported = true;
                        t.refresh_head();
                    }
                }
                self.hook(term, agent, status, &event);
    }

    pub(super) fn hook(&mut self, term: TermId, agent: String, status: HookStatus, event: &str) {
        tracing::debug!("hook for pane {term}: {event} -> {status:?}");
        if let Some(t) = self.terms.get_mut(&term) {
            let name = if event.is_empty() { "status report".to_string() } else { event.to_string() };
            t.last_hook = Some((name, term::unix_now()));
            t.last_hook_at = Some(Instant::now());
            self.dirty = true;
        }
        let focused = self.focused_term() == Some(term) && self.has_viewer();
        if matches!(status, HookStatus::Done | HookStatus::Idle)
            && let Some(dest) = self.terms.get_mut(&term).and_then(|t| t.pending_move.take())
        {
            self.relocate(term, &dest);
            return;
        }
        let Some(t) = self.terms.get_mut(&term) else { return };
        t.screen_blocked_from = None;
        let now = Instant::now();
        if matches!(status, HookStatus::Done | HookStatus::Idle) {
            t.last_turn_end = now;
            if agent == "codex" {
                t.ended_activity = self.agents.iter().find(|a| a.name == "codex").and_then(|d| d.working.iter().find_map(|r| r.find(&t.tail_text(t.rows)).map(|m| m.as_str().to_owned())));
            }
        }
        let new = match status {
            HookStatus::Same => {
                // A subagent finished: if the turn was waiting on it, it's done now.
                if t.subagents.is_empty() && t.done_held.take().is_some() {
                    let s = if focused { Status::Idle } else { Status::Done };
                    self.set_status(term, s, "hook: its last subagent finished");
                }
                self.dirty = true;
                return;
            }
            // Claude sometimes repeats a permission ping after you've already answered and it's
            // moved on: ignore one that comes right after a "working".
            HookStatus::Blocked
                if event == "Notification:permission_prompt" && t.last_working_hook.is_some_and(|w| w.elapsed() < REPEATED_PROMPT_WITHIN) =>
            {
                return;
            }
            // Done while subagents still run: hold it until they finish (or 3 minutes pass).
            HookStatus::Done if !t.subagents.is_empty() => {
                t.done_held = Some(now);
                self.dirty = true;
                return;
            }
            // A session starting again (after compaction, say) while its helpers still run is
            // the same turn carrying on, not an idle agent.
            HookStatus::Idle if event == "SessionStart" && !t.subagents.is_empty() => {
                self.dirty = true;
                return;
            }
            // After a restart, a session that was done and unseen comes back done.
            HookStatus::Idle if std::mem::take(&mut t.restore_unseen) && !focused => Status::Done,
            HookStatus::Gone => {
                t.hooked = false;
                t.hooked_process_seen = false;
                t.agent = None;
                t.session = None;
                t.summary.clear();
                t.status = Status::None;
                t.status_since = term::unix_now();
                self.dirty = true;
                return;
            }
            // Claude's "still waiting for you", a minute after its turn ended, isn't a second
            // finish: one you've already looked at stays looked at (and doesn't alert again).
            HookStatus::Done if event == "Notification:idle_prompt" && t.status == Status::Idle => return,
            HookStatus::Working => Status::Working,
            HookStatus::Blocked => Status::Blocked,
            HookStatus::Idle => Status::Idle,
            HookStatus::Done if focused => Status::Idle,
            HookStatus::Done => Status::Done,
        };
        t.hooked = true;
        if !agent.is_empty() {
            if t.agent.as_deref() != Some(&agent) { t.hooked_process_seen = false; }
            t.agent = Some(agent);
        }
        if new == Status::Working {
            t.last_working_hook = Some(now);
            t.done_held = None;
            t.progress_off = None;
        }
        if new != Status::Working {
            t.progress_off = None;
        }
        self.set_status(term, new, &format!("hook: {}", if event.is_empty() { "status report" } else { event }));
        self.dirty = true;
    }

    /// Agents that have sat finished or idle for `settle_after` fold into the settled group,
    /// and snoozes whose time has come end. Never the one you're on.
    pub(super) fn settle_idle(&mut self) {
        let now = term::unix_now();
        let after = self.cfg.settle_secs();
        let focused = if self.has_viewer() { self.focused_term() } else { None };
        for (id, t) in self.terms.iter_mut() {
            let sat = matches!(t.status, Status::Idle | Status::Done) && t.agent.is_some();
            let next = match t.aside {
                Some(until) if until > 0 && until <= now => None,
                None if sat && Some(*id) != focused && after.is_some_and(|a| now.saturating_sub(t.status_since) >= a) => Some(0),
                same => same,
            };
            if next != t.aside {
                t.aside = next;
                self.dirty = true;
            }
        }
    }

    /// Look (off the loop) at where each resting agent's worktree stands, to settle the ones
    /// whose work has been merged.
    pub(super) fn sweep_merged(&mut self) {
        if self.cfg.settle_secs().is_none() {
            return;
        }
        let resting: Vec<(TermId, std::path::PathBuf)> = self
            .terms
            .iter()
            .filter(|(_, t)| t.agent.is_some() && t.aside.is_none() && matches!(t.status, Status::Idle | Status::Done))
            .filter_map(|(id, t)| t.head.as_ref().filter(|h| h.linked).map(|h| (*id, h.top.clone())))
            .collect();
        if resting.is_empty() {
            return;
        }
        let tx = self.tx.clone();
        tokio::task::spawn_blocking(move || {
            let stands = resting.into_iter().map(|(id, top)| (id, crate::tidy::standing(&top))).collect();
            let _ = tx.blocking_send(Ev::Stands(stands));
        });
    }

    /// Where resting agents' worktrees stand: one whose own commits are now somewhere else
    /// too, with nothing uncommitted, is done with and settles.
    pub(super) fn settle_merged(&mut self, stands: Vec<(TermId, crate::tidy::Standing)>) {
        let focused = if self.has_viewer() { self.focused_term() } else { None };
        for (id, standing) in stands {
            let Some(t) = self.terms.get_mut(&id) else { continue };
            if !standing.merged {
                t.had_own_work = true;
            } else if t.had_own_work && standing.dirty == 0 && t.aside.is_none() && Some(id) != focused && matches!(t.status, Status::Idle | Status::Done) {
                t.aside = Some(0);
                self.dirty = true;
            }
        }
    }

    /// Keep `dir`'s files as they are, off the loop. Not in tests: they'd write refs into
    /// whatever repo they run from.
    fn snapshot_before_turn(&self, agent: String, dir: std::path::PathBuf) {
        if cfg!(test) {
            return;
        }
        let tx = self.tx.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = crate::snapshot::take(&dir, &format!("before {agent}'s turn")) {
                let _ = tx.blocking_send(Ev::HookRan(format!("couldn't snapshot {}: {e}", dir.display())));
            }
        });
    }

    pub(super) fn set_status(&mut self, term: TermId, new: Status, why: &str) {
        let Some(t) = self.terms.get_mut(&term) else { return };
        if let Some(seen) = why.strip_prefix("screen: ") {
            let now = term::unix_now();
            if t.last_screen.as_ref().is_none_or(|(text, at)| text != seen || *at != now) {
                t.last_screen = Some((seen.to_string(), now));
                self.dirty = true;
            }
        }
        if t.status == new {
            return;
        }
        t.status_why = why.to_string();
        let turn = (t.status == Status::Working && t.agent.is_some())
            .then(|| (t.agent.clone().unwrap_or_default(), t.place(), term::unix_now().saturating_sub(t.status_since)));
        // A turn is starting: the files as they are now are what it can be taken back to.
        let before_turn = (new == Status::Working && self.cfg.snapshots.enabled).then(|| t.agent.clone().map(|a| (a, t.cwd.clone()))).flatten();
        t.blocked_at = (new == Status::Blocked).then(Instant::now);
        t.status = new;
        t.status_since = term::unix_now();
        self.dirty = true;
        if let Some((agent, place, secs)) = turn {
            self.note_turn(agent, place, secs);
        }
        if let Some((agent, dir)) = before_turn {
            self.snapshot_before_turn(agent, dir);
        }
        // Whatever it was about to say is out of date now.
        if let Some(t) = self.terms.get_mut(&term) {
            t.alert_due = None;
            // Set aside, it comes back when it needs you; settled (not snoozed), also when
            // it gets to work again.
            if new == Status::Blocked || (new == Status::Working && t.aside == Some(0)) {
                t.aside = None;
            }
            // A turn has started: the next queued follow-up waits for this one to end.
            if new == Status::Working {
                t.queue_sent = None;
                t.alerted = None;
            }
        }
        if matches!(new, Status::Blocked | Status::Done) {
            self.broadcast(|c| c.attach, ServerMsg::Attention { term, status: new });
            if (new != Status::Done || !self.is_helper(term))
                && let Some(t) = self.terms.get_mut(&term)
                && let Some(delay) = crate::alert::settle_delay(new)
            {
                t.alert_due = Some((new, std::time::Instant::now() + delay));
            }
        }
    }

    /// Whether another session's agent started this one (and is still there to hear of it).
    pub(super) fn is_helper(&self, term: TermId) -> bool {
        self.terms.get(&term).and_then(|t| t.started_by).is_some_and(|by| self.terms.contains_key(&by))
    }

    /// Pending alerts only ring while the matching attention status is still current.
    pub(super) fn settled_alerts(&mut self) {
        let now = std::time::Instant::now();
        let due: Vec<_> = self.terms.values().filter_map(|t| t.alert_due.filter(|(_, at)| *at <= now).map(|(status, _)| (t.id, status))).collect();
        for (term, status) in due {
            let still = self.terms.get_mut(&term).is_some_and(|t| t.alert_due.take().is_some() && t.status == status);
            if still {
                self.alert(term, status);
            }
        }
    }

    /// No window open: the server tells you itself (once for the same news), with seshi's
    /// sound and a desktop notification.
    fn alert(&mut self, term: TermId, status: Status) {
        let now = std::time::Instant::now();
        let last_any = self.terms.values().filter_map(|t| t.alerted.map(|(_, at)| at)).max();
        if !self.has_viewer()
            && let Some(t) = self.terms.get_mut(&term)
            && crate::alert::should_alert(&status, t.alerted.as_ref().map(|(s, at)| (s, *at)), last_any, now)
        {
            t.alerted = Some((status, now));
            let agent = t.agent.clone().unwrap_or_else(|| "an agent".into());
            let place = t.place();
            let summary = t.summary.trim();
            let body = if summary.is_empty() { place } else { format!("{place} · {summary}") };
            let (kind, what) = if status == Status::Blocked { (crate::alert::Kind::Needs, "needs you") } else { (crate::alert::Kind::Done, "finished") };
            crate::alert::ring(&self.cfg.notify, kind);
            crate::alert::notify_soon(&format!("{agent} {what}"), &body, Some(crate::reveal::link(term)));
        }
    }

    /// Phone alerts for agents that have waited `phone_after` with nobody answering (or,
    /// when asked for, finished with nobody looking). Each status at most once.
    pub(super) fn phone_alerts(&mut self) {
        let n = &self.cfg.notify;
        if n.phone_topic.trim().is_empty() {
            return;
        }
        let now = term::unix_now();
        for t in self.terms.values_mut() {
            let Some(agent) = t.agent.clone() else { continue };
            let wanted = t.status == Status::Blocked || (t.status == Status::Done && n.phone_done);
            if !wanted || t.phoned == t.status_since || now.saturating_sub(t.status_since) < n.phone_after {
                continue;
            }
            t.phoned = t.status_since;
            let place = t.place();
            let (title, body) = crate::alert::phone_message(&agent, &place, t.status == Status::Blocked, &t.summary, n.phone_text);
            let cfg = n.clone();
            std::thread::spawn(move || {
                if let Err(e) = crate::alert::phone(&cfg, &title, &body) {
                    tracing::warn!("phone alert didn't go: {e}");
                }
            });
        }
    }

    pub(super) fn update_statuses(&mut self) {
        // A follow-up queued for an agent goes in when its turn is over: never into a question
        // it's asking, and one at a time (the next waits for the turn this one starts).
        for t in self.terms.values_mut() {
            let turn_over = matches!(t.status, Status::Idle | Status::Done) && t.agent.is_some() && !t.asleep;
            let last_taken = t.queue_sent.is_none_or(|at| at.elapsed() > QUEUED_TURN_STARTS_WITHIN);
            if turn_over && last_taken && t.pending_input.is_none()
                && let Some(next) = t.queue.pop_front()
            {
                t.pending_input = Some((format!("{}\r", next.replace(['\r', '\n'], " ")).into_bytes(), Instant::now()));
                t.queue_sent = Some(Instant::now());
                self.dirty = true;
            }
        }
        // Messages typed to a sleeping agent: deliver once it's ready (its hooks say idle) or
        // after a few seconds.
        for t in self.terms.values_mut() {
            let ready = matches!(t.status, Status::Idle | Status::Done) && t.hooked;
            if let Some((data, by)) = t.pending_input.take() {
                if ready || Instant::now() >= by {
                    // Text, then Enter a beat later, so it lands as a message, not a paste.
                    if data.len() > 1 && data.ends_with(b"\r") {
                        t.input(&data[..data.len() - 1]);
                        t.pending_input = Some((b"\r".to_vec(), Instant::now()));
                    } else {
                        t.input(&data);
                    }
                } else {
                    t.pending_input = Some((data, by));
                }
            }
        }
        let focused = self.has_viewer().then(|| self.focused_term()).flatten();
        let window = Duration::from_millis(self.cfg.detection.working_window_ms);
        let grace = Duration::from_millis(self.cfg.detection.echo_grace_ms);
        let rows = self.cfg.detection.pattern_rows;
        let mut changes = Vec::new();
        let mut still_working = Vec::new();
        let mut screen_blocks = Vec::new();
        for t in self.terms.values() {
            let Some(name) = &t.agent else { continue };
            let def = self.agents.iter().find(|a| &a.name == name);
            let screen = t.tail_text(t.rows);
            let signal = if def.is_some_and(|d| d.builtin_screen) {
                crate::detection::detect(name, crate::detection::DetectionInput { screen: &screen, osc_title: &t.parser.callbacks().title, osc_progress: &t.parser.callbacks().progress })
            } else { crate::detection::Signal::default() };
            if signal.skip { continue; }
            if t.hooked && !FULL_LIFECYCLE_HOOKS.contains(&name.as_str()) {
                if signal.visible_blocker && signal.status == Some(Status::Blocked) && t.last_output >= t.last_turn_end {
                    screen_blocks.push((t.id, t.status));
                    changes.push((t.id, Status::Blocked, VISIBLE_BLOCKER_WHY.into()));
                    continue;
                }
                if t.status == Status::Blocked && t.status_why == VISIBLE_BLOCKER_WHY {
                    changes.push((t.id, t.screen_blocked_from.unwrap_or(Status::Idle), "screen: live dialog dismissed".into()));
                    continue;
                }
            }
            if name == "codex" && CODEX_QUEUED_QUESTION.is_match(&t.tail_text(rows)) {
                changes.push((t.id, Status::Blocked, CODEX_QUESTION_WHY.into()));
                continue;
            }
            if name == "codex" && t.status == Status::Blocked && t.status_why == CODEX_QUESTION_WHY {
                let working = self.agents.iter().find(|a| &a.name == name).is_some_and(|d| d.working.iter().any(|r| r.is_match(&t.tail_text(rows))));
                changes.push((t.id, if working { Status::Working } else if Some(t.id) == focused { Status::Idle } else { Status::Done }, "screen: queued question answered".into()));
                continue;
            }
            // Any trusted adapter may report child agents without using lifecycle
            // hooks (Codex is discovered from its local session files). Keep the
            // parent visibly working while a recently observed child is active.
            if matches!(t.status, Status::Idle | Status::Done)
                && !t.subagents.is_empty()
                && t.subagent_seen.is_some_and(|s| s.elapsed() <= SUBAGENTS_QUIET_AT_MOST)
            {
                still_working.push(t.id);
                continue;
            }
            if t.hooked {
                if t.status == Status::Working
                    && let Some(pattern) = self.agents.iter().find(|a| &a.name == name).and_then(|d| d.working.iter().find(|r| r.is_match(&t.tail_text(rows))))
                {
                    changes.push((t.id, t.status, format!("screen: matched `{pattern}`")));
                }
                if t.status == Status::Done && Some(t.id) == focused {
                    changes.push((t.id, Status::Idle, "you're looking at it".to_string()));
                }
                // Subagents went quiet for good (one died without reporting): don't hold "done"
                // forever.
                if t.done_held.is_some_and(|h| h.elapsed() > SUBAGENTS_QUIET_AT_MOST)
                    && t.subagent_seen.is_none_or(|s| s.elapsed() > SUBAGENTS_QUIET_AT_MOST)
                {
                    changes.push((t.id, if Some(t.id) == focused { Status::Idle } else { Status::Done }, "subagents went quiet".to_string()));
                }
                // A question dismissed with Esc sends no hook at all. Once you've typed since
                // it was asked, the pane has gone quiet, and no question shows on screen, it's
                // over.
                if t.status == Status::Blocked
                    && !NATIVE_PROMPT_LIFECYCLE.contains(&name.as_str())
                    && t.blocked_at.is_some_and(|b| t.last_input > b)
                    && t.last_input.elapsed() > QUESTION_GONE_AFTER
                    && t.last_output.elapsed() > QUESTION_GONE_AFTER
                {
                    let text = t.tail_text(rows);
                    let asking = self.agents.iter().find(|a| &a.name == name).is_some_and(|d| d.blocked.iter().any(|r| r.is_match(&text)));
                    if !asking {
                        changes.push((t.id, Status::Idle, "screen: you typed and the question went away".to_string()));
                    }
                }
                // A permission ask that auto mode (or the agent) settled by itself also sends
                // no hook: a while on, with no question on screen at all, it's over.
                if t.status == Status::Blocked
                    && !NATIVE_PROMPT_LIFECYCLE.contains(&name.as_str())
                    && t.blocked_at.is_some_and(|b| b.elapsed() > UNASKED_QUESTION_GONE_AFTER)
                    && !self.questions.iter().any(|(q, ..)| q.term == t.id)
                {
                    let text = t.tail_text(rows);
                    let asking = CHOICES.is_match(&text) || self.agents.iter().find(|a| &a.name == name).is_some_and(|d| d.blocked.iter().any(|r| r.is_match(&text)));
                    if !asking {
                        changes.push((t.id, if Some(t.id) == focused { Status::Idle } else { Status::Done }, "screen: the question settled itself".to_string()));
                    }
                }
                // The progress indicator went away and no turn-end came: cancelled (Esc).
                // A progress bar that is off while helpers run is the agent waiting on them.
                let helpers_running = !t.subagents.is_empty() && t.subagent_seen.is_some_and(|s| s.elapsed() <= SUBAGENTS_QUIET_AT_MOST);
                if t.status == Status::Working && t.done_held.is_none() && !helpers_running && t.progress_off.is_some_and(|p| p.elapsed() > CANCELLED_AFTER) {
                    changes.push((t.id, Status::Idle, "progress bar gone, no turn end: cancelled".to_string()));
                }
                // Agents whose hooks only say a turn ended (Codex's notify): the start of the
                // next turn shows on screen only.
                if HOOKS_END_ONLY.contains(&name.as_str())
                    && matches!(t.status, Status::Idle | Status::Done)
                    // Typed since it finished: not the last turn's line still on screen.
                    && let Some(d) = self.agents.iter().find(|a| &a.name == name)
                    && let Some(r) = d.working.iter().find(|r| r.is_match(&t.tail_text(rows)))
                    && (t.last_input > t.last_turn_end || (t.last_output > t.last_turn_end
                        && t.ended_activity.as_deref() != r.find(&t.tail_text(rows)).map(|m| m.as_str())))
                {
                    changes.push((t.id, Status::Working, format!("screen: matched `{}`", r.as_str())));
                }
                continue;
            }
            let text = t.tail_text(rows);
            let blocked = def.and_then(|d| d.blocked.iter().find(|r| r.is_match(&text)));
            let working: Option<String> = match def {
                Some(d) if !d.working.is_empty() => d.working.iter().find(|r| r.is_match(&text)).map(|r| format!("screen: matched `{}`", r.as_str())),
                _ => (t.last_output.elapsed() < window && t.last_output.saturating_duration_since(t.last_input) > grace)
                    .then(|| "screen: it's printing".to_string()),
            };
            // Its working line can leave the rows looked at while it still runs (a long block
            // printed under it): working until the screen has also gone still, so it doesn't
            // flip to done (and alert) and back on every check.
            let still_going = t.status == Status::Working && t.last_output.elapsed() < SCREEN_DONE_QUIET;
            let (new, why) = if let Some(status) = signal.status {
                let status = if status == Status::Idle && !signal.visible_idle && still_going { Status::Working }
                    else if status == Status::Idle && Some(t.id) != focused && matches!(t.status, Status::Working | Status::Done) { Status::Done } else { status };
                (status, format!("screen: {} rule `{}`", name, signal.rule))
            } else if let Some(r) = blocked {
                (Status::Blocked, format!("screen: matched `{}`", r.as_str()))
            } else if let Some(why) = working {
                (Status::Working, why)
            } else if still_going {
                (Status::Working, "screen: still printing".to_string())
            } else if Some(t.id) == focused {
                (Status::Idle, "screen: quiet, and you're on it".to_string())
            } else if matches!(t.status, Status::Working | Status::Done) {
                (Status::Done, "screen: went quiet after working".to_string())
            } else {
                (Status::Idle, "screen: quiet".to_string())
            };
            changes.push((t.id, new, why));
        }
        for id in still_working {
            if let Some(t) = self.terms.get_mut(&id) {
                t.done_held = Some(Instant::now());
            }
            self.set_status(id, Status::Working, "one of its own agents is still running");
        }
        for (id, before) in screen_blocks {
            if let Some(t) = self.terms.get_mut(&id) && before != Status::Blocked { t.screen_blocked_from = Some(before); }
        }
        for (id, s, why) in changes {
            if s != Status::Working
                && let Some(t) = self.terms.get_mut(&id)
                && t.status != s
            {
                if t.done_held.take().is_some() {
                    t.subagents.clear();
                }
                t.progress_off = None;
            }
            self.set_status(id, s, &why);
        }
    }
}
