//! Messages from the server, and results of work done in the background.

use super::*;

/// A whole list asked for is asked for again when it has not come in this long, so one lost
/// answer cannot leave a machine's deltas dropped for as long as its line stays up.
pub(super) const LIST_ASKED_AGAIN_AFTER: Duration = Duration::from_secs(5);
/// The revision of a daemon built before revisions, or of a surface sent nothing yet.
const UNNUMBERED: u64 = 0;

/// Whether a delta numbered `revision` is the next after the snapshot numbered `held`. An
/// unnumbered delta cannot be checked and is taken as it comes. Another machine's numbers are
/// whatever it sends, so the sum is checked.
fn follows(held: u64, revision: u64) -> bool {
    revision == UNNUMBERED || held.checked_add(1) == Some(revision)
}

/// Whether what a window was told about a pane still holds in its latest state. A pane's own
/// status says so, except for a check: it fails or waits for approval after the turn ended, so
/// the pane reads done or idle while it needs you all the same.
fn still_asks(t: &TermInfo, status: Status) -> bool {
    let check_asks = || {
        matches!(t.status, Status::Done | Status::Idle)
            && t.checks.iter().any(|check| {
                matches!(
                    check.state,
                    CheckState::Failed | CheckState::NeedsApproval
                )
            })
    };
    t.status == status || (status == Status::Blocked && check_asks())
}

impl App {
    /// A message from the server can close a popup with no key pressed: shortcut mode held
    /// under it comes back then, not after a key has gone to the pane.
    pub(super) fn on_machine_server(&mut self, machine: MachineId, msg: ServerMsg) {
        if !self.admits(machine, &msg) {
            return;
        }
        // Another machine's whole list is taken only as the answer to this window's own asking
        // for it, once: a delta that machine sends can make the window ask, so that asking opens
        // nothing for an error or any other reply.
        let listed = matches!(msg, ServerMsg::Reply(Reply::List(_)));
        if listed && machine != MachineId::LOCAL && self.list_asked.remove(&machine).is_none() {
            tracing::debug!("ignored a list this window did not ask for");
            return;
        }
        // What a refused copy would have said, to tell the person it was refused.
        let copy_size = match &msg {
            ServerMsg::Clipboard { text, .. } => Some(text.chars().count()),
            _ => None,
        };
        let Some(msg) = self.vetted(machine, msg, listed) else {
            if let Some(n) = copy_size {
                self.say_copy_refused(machine, n);
            }
            return;
        };
        if machine != MachineId::LOCAL
            && !listed
            && matches!(msg, ServerMsg::Reply(_) | ServerMsg::Error(_))
        {
            self.route.answered(machine);
        }
        self.apply_server(machine, msg);
        self.keep_shortcut_mode();
    }

    /// A program on another machine copied text and that machine may not write this clipboard:
    /// nothing was copied, and the toast says how to allow it (once a minute, so a selection
    /// dragged over many cells does not repeat it).
    fn say_copy_refused(&mut self, machine: MachineId, chars: usize) {
        let name = self.machine_name(machine).to_string();
        if self
            .copy_refusal_told
            .is_some_and(|at| at.elapsed() < Duration::from_secs(60))
        {
            return;
        }
        self.copy_refusal_told = Some(Instant::now());
        self.notify_card(
            format!("{name} tried to copy"),
            format!("{chars} characters, not copied: {name} may not write your clipboard"),
            Some(format!(
                "allow it: clipboard = true under [[machines]] {name} in config.local.toml"
            )),
            true,
        );
    }

    /// Whether to act on a message from `machine`. Another machine's state is held back when a
    /// different daemon answers at its name: its saved layout must not be reused for it. The
    /// reason is told once, and everything else it says is dropped. A machine that was removed
    /// is not heard at all.
    fn admits(&mut self, machine: MachineId, msg: &ServerMsg) -> bool {
        if machine == MachineId::LOCAL {
            return true;
        }
        if self.refused.contains_key(&machine) || self.machine_is_forgotten(machine) {
            return false;
        }
        let ServerMsg::State(snap) = msg else {
            return true;
        };
        let name = self.machine_name(machine).to_owned();
        match machine_ids::check_daemon_id(&mut self.machine_ids, &name, snap.daemon_id.as_deref())
        {
            Ok(machine_ids::IdCheck::Recorded) => {
                machine_ids::save(&self.machine_ids);
                true
            }
            Ok(_) => true,
            Err(mismatch) => {
                tracing::warn!("{mismatch}");
                let reason = mismatch.to_string();
                self.notify(reason.clone(), true);
                self.links.refuse(machine, reason.clone());
                self.refused.insert(machine, reason);
                false
            }
        }
    }

    /// The snapshot `machine` holds with a delta applied; removing a pane it does not have is
    /// harmless. A pane dropped with no reason (a daemon from before reasons) is kept: only a
    /// whole list may take it away.
    fn merged_terms(
        &self,
        machine: MachineId,
        changed: Vec<TermInfo>,
        removed: Vec<TermId>,
        reasons: std::collections::BTreeMap<TermId, RemovalReason>,
        revision: u64,
    ) -> Snapshot {
        let mut merged = self.snap_of(machine).cloned().unwrap_or_default();
        merged.removed = reasons;
        if revision != UNNUMBERED {
            merged.revision = revision;
        }
        for id in removed {
            if merged.removed.contains_key(&id) {
                merged.terms.remove(&id);
            }
        }
        for term in changed {
            merged.terms.insert(term.id, term);
        }
        merged
    }

    /// Ask `machine` for its whole list, unless one asked for lately is still to come: a run of
    /// deltas after one missed update asks once.
    fn want_list(&mut self, machine: MachineId) {
        let now = Instant::now();
        let waiting = self
            .list_asked
            .get(&machine)
            .is_some_and(|at| now.saturating_duration_since(*at) < LIST_ASKED_AGAIN_AFTER);
        if !waiting {
            self.list_asked.insert(machine, now);
            // Not through `send`: its routing bookkeeping would let any reply or error in.
            let _ = self.out.send(Outgoing {
                machine,
                msg: ClientMsg::Query(Query::List),
            });
        }
    }

    /// A list asked for that never came (refused, or lost) is asked for again, so a pane kept
    /// for want of a reason cannot stay while the machine sends nothing more.
    pub(super) fn ask_lists_again(&mut self) {
        let now = Instant::now();
        let due: Vec<MachineId> = self
            .list_asked
            .iter()
            .filter(|(_, at)| now.saturating_duration_since(**at) >= LIST_ASKED_AGAIN_AFTER)
            .map(|(machine, _)| *machine)
            .collect();
        for machine in due {
            if self.refused.contains_key(&machine) || self.machine_is_forgotten(machine) {
                self.list_asked.remove(&machine);
            } else {
                self.want_list(machine);
            }
        }
    }

    /// What another machine's daemon says, cleaned and held to what its entry allows. This
    /// computer's own daemon is trusted as it was. `listed`: a whole list this window asked for.
    fn vetted(&self, machine: MachineId, msg: ServerMsg, listed: bool) -> Option<ServerMsg> {
        if machine == MachineId::LOCAL {
            return Some(msg);
        }
        let name = self.machine_name(machine);
        let asked = listed || self.route.awaiting(machine);
        remote_policy::vet(&self.machine_opts(machine), asked, name, msg)
    }

    /// What `machine` may do on this computer: everything for this one, only its opt-ins for
    /// any other.
    pub(super) fn machine_opts(&self, machine: MachineId) -> remote_policy::MachineOpts {
        if machine == MachineId::LOCAL {
            return remote_policy::MachineOpts::LOCAL;
        }
        remote_policy::MachineOpts::named(&self.cfg.machines, self.machine_name(machine))
    }

    #[cfg(test)]
    pub(super) fn on_server(&mut self, msg: ServerMsg) {
        self.on_machine_server(MachineId::LOCAL, msg);
    }

    fn apply_server(&mut self, machine: MachineId, msg: ServerMsg) {
        // Output doesn't change the sidebar; everything else might. Output for a pane you
        // can't see changes nothing on screen at all: no redraw for it.
        match &msg {
            ServerMsg::Output { term, .. } => {
                let term = ClientTerm::on(machine, *term);
                if self.panes.iter().any(|(t, _)| *t == term) || self.panes.is_empty() {
                    self.dirty = true;
                }
            }
            _ => {
                self.screen_fresh();
                self.dirty = true;
            }
        }
        if matches!(msg, ServerMsg::State(_)) {
            self.list_asked.remove(&machine);
        }
        match msg {
            ServerMsg::State(s) if machine != MachineId::LOCAL => {
                self.say_alerts_are_off(machine);
                self.sync_terms(machine, &s);
                self.remember_changes(machine, &s);
                self.remotes.entry(machine).or_default().snap = s;
                self.screen_sync();
                self.mark_looked_pane_seen();
            }
            ServerMsg::State(s) => {
                self.sync_terms(machine, &s);
                let first = !self.got_state;
                let last_session_closed = !self.snap.terms.is_empty() && s.terms.is_empty();
                self.got_state = true;
                if !first {
                    self.remember_changes(MachineId::LOCAL, &s);
                }
                self.snap = s;
                self.on_login_snapshot();
                if self.daemon_restart_reconnect {
                    let daemon_matches_install = self
                        .installed_version
                        .as_ref()
                        .and_then(|v| v.as_ref().ok())
                        .is_some_and(|installed| installed == &self.snap.daemon_version);
                    self.daemon_restart_requested = false;
                    self.daemon_restart_reconnect = false;
                    if daemon_matches_install {
                        self.notify(
                            concat!(
                                "The ",
                                env!("APP_NAME"),
                                " daemon restarted with the update."
                            )
                            .into(),
                            false,
                        );
                    }
                }
                if first {
                    self.on_first_state();
                }
                if !self.snap.conversations.is_empty() {
                    self.splash = false;
                }
                self.screen_sync();
                self.mark_looked_pane_seen();
                self.auto_open_secret();
                self.check_view_sync();
                self.maybe_restart_daemon(last_session_closed);
            }
            ServerMsg::TermsChanged { changed, removed, reasons, revision } => {
                // A delta fits only the snapshot it was worked out from: after a missed update,
                // merging it would leave the list wrong until the next whole one.
                let held = self.snap_of(machine).map_or(UNNUMBERED, |s| s.revision);
                if !follows(held, revision) {
                    self.want_list(machine);
                    return;
                }
                let unexplained = removed.iter().any(|id| !reasons.contains_key(id));
                // Re-entering as a whole snapshot runs every rule a state update runs for this
                // machine: the daemon-id check, the cleaning and caps, and the side effects. A
                // delta arriving before any state merges into an empty snapshot, which has no
                // daemon id; that is accepted only while no id is recorded for the name.
                let merged = self.merged_terms(machine, changed, removed, reasons, revision);
                self.on_machine_server(machine, ServerMsg::State(merged));
                if unexplained {
                    self.want_list(machine);
                }
            }
            // Only the list asked for after a missed update; nothing else asks on this line.
            ServerMsg::Reply(Reply::List(snap)) => {
                self.on_machine_server(machine, ServerMsg::State(snap));
            }
            ServerMsg::Replay {
                term,
                cols,
                rows,
                data,
            } => {
                let term = ClientTerm::on(machine, term);
                let p = self.new_parser(rows, cols);
                self.parsers.insert(term, p);
                self.marks.remove(&term);
                self.feed(term, &data);
                self.keep_raw(term, &data, true);
                if let Some(c) = keys::last_cursor_style(&data) {
                    self.cursor_style.insert(term, c);
                }
                self.sizes.insert(term, (cols, rows));
            }
            ServerMsg::Output { term, data } => {
                let term = ClientTerm::on(machine, term);
                self.keep_raw(term, &data, false);
                if let Some(c) = keys::last_cursor_style(&data) {
                    self.cursor_style.insert(term, c);
                }
                let last = |pat: &[u8]| data.windows(pat.len()).rposition(|w| w == pat);
                match (last(b"\x1b[?2026h"), last(b"\x1b[?2026l")) {
                    (Some(h), l) if l.is_none_or(|l| h > l) => {
                        self.sync_hold.insert(term, Instant::now());
                    }
                    (_, Some(_)) => {
                        self.sync_hold.remove(&term);
                    }
                    _ => {}
                }
                match (last(b"\x1b[?1004h"), last(b"\x1b[?1004l")) {
                    (Some(h), l) if l.is_none_or(|l| h > l) => {
                        self.focus_report.insert(term);
                    }
                    (_, Some(_)) => {
                        self.focus_report.remove(&term);
                    }
                    _ => {}
                }
                if !self.parsers.contains_key(&term)
                    && let Some(t) = self.term_info(term)
                {
                    let p = self.new_parser(t.rows, t.cols);
                    self.parsers.insert(term, p);
                }
                self.feed(term, &data);
            }
            ServerMsg::Attention { term, status } => {
                let term = ClientTerm::on(machine, term);
                self.settling.remove(&term);
                if (status != Status::Done || !self.is_helper(term))
                    && let Some(delay) = crate::alert::settle_delay(status)
                {
                    self.settling.insert(term, (status, Instant::now() + delay));
                }
            }
            ServerMsg::Clipboard { term, text } => {
                let term = ClientTerm::on(machine, term);
                let native = copy::to_clipboard(&text);
                // The same card as a copy made here, naming the session and, for another
                // machine, which one.
                let who = self.session_name(term);
                let from = if machine == MachineId::LOCAL {
                    who
                } else {
                    format!("{who} on {}", self.machine_name(machine))
                };
                self.announce_copy(&text, native, Some(from));
            }
            ServerMsg::Error(e) => {
                // The output a failure waited for may be what failed; a later reply is not its.
                self.screen.failure_ask = None;
                if machine == MachineId::LOCAL {
                    self.resume_failed();
                }
                self.notify(e, true);
            }
            ServerMsg::Notice(n) if machine != MachineId::LOCAL => self.notify(n, false),
            ServerMsg::Notice(n) => {
                self.undo_hint = false;
                self.notify(n, false)
            }
            ServerMsg::AskRemoveWorktree { path, name, why } => {
                self.ask_remove_worktree(path, name, why)
            }
            ServerMsg::AutoWorkspace(n) => {
                self.notify(n, false);
                self.undo_hint = true;
                if let Some((_, at, _)) = &mut self.notice {
                    // Leave time to undo.
                    *at = Instant::now() + Duration::from_secs(10);
                }
            }
            // The desktop app changed a shared setting: load the file, unless this window
            // already shows these values (its own change comes back as the same message).
            ServerMsg::SettingsChanged(settings)
                if machine == MachineId::LOCAL
                    && settings != crate::shared_settings::read(&self.cfg) =>
            {
                self.reload_config()
            }
            ServerMsg::SettingsChanged(_) => {}
            ServerMsg::Raise => crate::reveal::raise_window(),
            // The desktop app's chats; the daemon sends this app none.
            ServerMsg::ConversationChanged { .. } => {}
            // The server stopped for the update: an older window can't speak to the new one, so
            // the installed the app takes its place; a current window just connects again.
            // Another machine's daemon stopping ends its connection, not the window.
            ServerMsg::Bye if machine != MachineId::LOCAL => {}
            ServerMsg::Bye if self.daemon_restart_requested && self.window_stale() => {
                self.relaunch_window()
            }
            ServerMsg::Bye if self.daemon_restart_requested => self.daemon_restart_reconnect = true,
            ServerMsg::Bye => self.quit = Some("server exited".into()),
            // Worktrees are listed for this computer's own folders only.
            ServerMsg::Reply(Reply::Worktrees(list)) if machine == MachineId::LOCAL => {
                if let Mode::Worktrees { items, .. } = &mut self.mode {
                    *items = Some(list);
                }
            }
            // Saved conversations are listed for this computer's own folders only.
            ServerMsg::Reply(Reply::SavedConversations { tag, found })
                if machine == MachineId::LOCAL =>
            {
                self.saved_conversations_arrived(tag, found)
            }
            ServerMsg::Reply(Reply::Text(text)) if self.check_asked_of(machine) => {
                self.check_output_arrived(text)
            }
            ServerMsg::Welcome { .. } | ServerMsg::Reply(_) => {}
        }
    }

    /// Per-pane state of `machine` follows what its daemon reports: no parser, size or pending
    /// alert for a pane it no longer lists, and a parser and size for each it does.
    pub(super) fn sync_terms(&mut self, machine: MachineId, s: &Snapshot) {
        self.settling.retain(|id, (status, _)| {
            id.machine != machine
                || s.terms
                    .get(&id.term)
                    .is_some_and(|t| still_asks(t, *status))
        });
        self.parsers
            .retain(|id, _| id.machine != machine || s.terms.contains_key(&id.term));
        for (id, t) in &s.terms {
            let id = ClientTerm::on(machine, *id);
            if !self.parsers.contains_key(&id) {
                let p = self.new_parser(t.rows, t.cols);
                self.parsers.insert(id, p);
            }
            // Trust the daemon's size for panes we aren't showing.
            if !self.panes.iter().any(|(p, _)| *p == id) {
                self.sizes.insert(id, (t.cols, t.rows));
            }
        }
    }

    pub(super) fn on_first_state(&mut self) {
        let open = self.open.take();
        if let Some(dir) = &open {
            let dir = dir.canonicalize().unwrap_or_else(|_| dir.clone());
            if let Some(w) = self.snap.workspaces.iter().find(|w| same_dir(&w.cwd, &dir)) {
                self.cmd(Command::SelectWorkspace { ws: w.id });
                return;
            }
        }
        if (self.snap.workspaces.is_empty() && self.snap.conversations.is_empty()) || open.is_some()
        {
            let cwd = open
                .or_else(|| self.cfg.start_dir())
                .or_else(|| std::env::current_dir().ok());
            self.cmd(Command::NewWorkspace {
                cwd,
                name: None,
                cmd: None,
            });
        }
    }

    /// Whether another session's agent started this one (and is still there to hear of it).
    fn is_helper(&self, term: ClientTerm) -> bool {
        let Some(snap) = self.snap_of(term.machine) else {
            return false;
        };
        snap.terms
            .get(&term.term)
            .and_then(|t| t.started_by)
            .is_some_and(|by| snap.terms.contains_key(&by))
    }

    /// Recheck queued alerts after short-lived prompts and automatic continuations settle.
    pub(super) fn settle_alerts(&mut self) {
        let now = Instant::now();
        let due: Vec<_> = self
            .settling
            .iter()
            .filter(|(_, (_, at))| *at <= now)
            .map(|(t, (s, _))| (*t, *s))
            .collect();
        for (term, status) in due {
            self.settling.remove(&term);
            if self.term_info(term).is_some_and(|t| still_asks(t, status)) {
                self.alert_now(term, status);
                self.dirty = true;
            }
        }
    }

    /// Needs-you alerts use two rising notes and background completions one soft note; the
    /// pane you are looking at rings neither.
    /// With no window open the server shows a desktop notification instead.
    pub(super) fn alert_now(&mut self, term: ClientTerm, status: Status) {
        if !self.term_info(term).is_some_and(|t| still_asks(t, status)) {
            return;
        }
        // You are looking at it: a sound adds nothing.
        if self.focused() == Some(term) && self.window_focused {
            return;
        }
        let opts = self.machine_opts(term.machine);
        // The same news again soon, or another agent just rang: the note says it, quietly.
        let now = Instant::now();
        if opts.sounds
            && let Some(kind) = crate::alert::Kind::for_status(status)
            && crate::alert::should_alert(
                &status,
                self.alerted.get(&term).map(|(s, at)| (s, *at)),
                // Any sound starts the quiet spell, whichever kind it was: a "needs you" and a
                // "finished" a moment apart would be three notes, which reads as noise.
                self.last_alert.map(|(_, at)| at),
                now,
            )
        {
            self.alerted.insert(term, (status, now));
            self.last_alert = Some((kind, now));
            crate::alert::ring(&self.cfg.notify, kind);
        }
        if self.focused() == Some(term) || !opts.notices {
            return;
        }
        let who = self.describe_term(term);
        let what = if status == Status::Blocked {
            "needs you"
        } else {
            "finished"
        };
        let told = format!("{who} {what}");
        let told = if term.machine == MachineId::LOCAL {
            told
        } else {
            remote_policy::tagged(self.machine_name(term.machine), &told)
        };
        self.notify(told, false);
        self.notice_term = Some(term);
    }

    /// When a machine connects with toasts off, as every machine is until you turn them on: say so once per run, so
    /// it is never a mystery why its agents raise nothing. Said at the connection, which you caused, and not when an
    /// agent there needs you, which would let a machine put a toast on the screen unasked. The words are this app's
    /// own and the only name in them is the one you gave the machine.
    fn say_alerts_are_off(&mut self, machine: MachineId) {
        if machine == MachineId::LOCAL || self.alerts_off_told.contains(&machine) {
            return;
        }
        self.alerts_off_told.insert(machine);
        if self.machine_opts(machine).notices {
            return;
        }
        let name = self.machine_name(machine);
        self.notify(
            format!("Toasts from {name} are off. Open Machines, pick {name} and press n to turn them on."),
            false,
        );
    }

    /// Note what changed on `machine` between two states: agents that now need you or finished.
    /// Another machine's lines name it. A pane it had not listed before is not news, so the first
    /// state after a connect says nothing.
    pub(super) fn remember_changes(&mut self, machine: MachineId, new: &crate::protocol::Snapshot) {
        let mut events = Vec::new();
        for (id, t) in &new.terms {
            let old = self.snap_of(machine).and_then(|snap| snap.terms.get(id));
            let id = &ClientTerm::on(machine, *id);
            let name = if t.label.trim().is_empty() {
                t.display_name()
            } else {
                t.label.trim().to_string()
            };
            let place = t
                .top
                .as_ref()
                .or(t.root.as_ref())
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let who = if place.is_empty() {
                name
            } else {
                format!("{name} in {place}")
            };
            if old.is_some_and(|o| o.status != t.status) {
                match t.status {
                    // A new turn's finish is news again; a question asked again soon is not.
                    Status::Working => {
                        if self
                            .alerted
                            .get(id)
                            .is_some_and(|(s, _)| *s == Status::Done)
                        {
                            self.alerted.remove(id);
                        }
                    }
                    Status::Blocked => events.push((*id, '!', format!("{who} needs you"))),
                    Status::Done => {
                        let said = t
                            .said
                            .lines()
                            .find(|l| !l.trim().is_empty())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        events.push((
                            *id,
                            '✓',
                            if said.is_empty() {
                                format!("{who} finished")
                            } else {
                                format!("{who} finished: {said}")
                            },
                        ));
                    }
                    _ => {}
                }
            }
        }
        for (id, k, text) in events {
            let text = if machine == MachineId::LOCAL {
                text
            } else {
                remote_policy::tagged(self.machine_name(machine), &text)
            };
            self.remember(Some(id), k, text);
        }
    }

    /// The Changes view's selected file has no diff yet: work it out in the background.
    pub(super) fn request_diff(&mut self) {
        let Some(View::Changes(v)) = &mut self.view else {
            return;
        };
        let Some(r) = v.review.as_mut() else { return };
        if r.diff_sel == Some(r.sel) {
            return;
        }
        r.diff_sel = Some(r.sel);
        r.diff = vec!["loading…".into()];
        let (dir, sel, job) = (v.dir.clone(), r.sel, r.diff_job());
        self.dirty = true;
        self.spawn_bg(move || Bg::Diff(dir, sel, tasks::diff_of(&job)));
    }

    /// Reads the file the Files view now points at, off the UI thread.
    pub(super) fn request_files_preview(&mut self) {
        let Some(View::Files(v)) = &mut self.view else {
            return;
        };
        let Some(path) = v.preview_wanted.take() else {
            return;
        };
        let root = v.root.clone();
        self.spawn_bg(move || {
            let lines = files::preview(&path);
            Bg::FilesPreview(root, path, lines)
        });
    }

    /// Lists the folder the folder picker now points at, off the UI thread.
    pub(super) fn request_folder_list(&mut self) {
        let Mode::Finder(fd) = &mut self.mode else {
            return;
        };
        let Some(dir) = fd.wanted.take() else {
            return;
        };
        self.spawn_bg(move || {
            let (list, repo) = screen::Finder::list_folders(&dir);
            Bg::FolderList(dir, list, repo)
        });
    }

    pub(super) fn spawn_bg(&self, f: impl FnOnce() -> Bg + Send + 'static) {
        let tx = self.bg.clone();
        let job = move || {
            let _ = tx.send(f());
        };
        #[cfg(test)]
        if self.run_background_jobs_inline {
            job();
            return;
        }
        // Tests (and anything else outside the runtime) get a plain thread.
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::spawn_blocking(job);
        } else {
            std::thread::spawn(job);
        }
    }

    /// Background work can close a popup too; see `on_server`.
    pub(super) fn on_bg(&mut self, b: Bg) {
        self.apply_bg(b);
        self.keep_shortcut_mode();
    }

    fn apply_bg(&mut self, b: Bg) {
        self.screen_fresh();
        let b = match b {
            Bg::Branches(dir, current, list, dirty) => {
                if let Mode::Branch(v) = &mut self.mode
                    && v.dir == dir
                {
                    v.current = current;
                    v.list = Some(list);
                    v.dirty = dirty;
                }
                self.dirty = true;
                return;
            }
            Bg::Switched(r) => {
                match r {
                    Ok(m) => self.notify(m, false),
                    Err(e) => self.notify(format!("couldn't switch: {e}"), true),
                }
                self.branch_sheet_reload();
                self.screen_fresh();
                self.dirty = true;
                return;
            }
            Bg::FindFiles(dir, list) => {
                let mut wanted = None;
                if let Mode::Find(v) = &mut self.mode
                    && v.dir == dir
                {
                    v.files = Some(list);
                    wanted = v.preview_wanted();
                }
                self.load_find_preview(wanted);
                self.dirty = true;
                return;
            }
            Bg::Grep(dir, seq, hits) => {
                let mut wanted = None;
                if let Mode::Find(v) = &mut self.mode
                    && v.dir == dir
                    && v.seq == seq
                {
                    v.hits = Some(hits);
                    v.sel = 0;
                    wanted = v.preview_wanted();
                }
                self.load_find_preview(wanted);
                self.dirty = true;
                return;
            }
            Bg::FindPreview(dir, target, lines) => {
                // The selection may have moved on while this was read.
                if let Mode::Find(v) = &mut self.mode
                    && v.dir == dir
                    && v.preview_of.as_ref() == Some(&target)
                {
                    v.preview = lines;
                    self.dirty = true;
                }
                return;
            }
            Bg::Synced(changed) => {
                if changed {
                    self.reload_config();
                    self.notify("pulled your setup from another machine".into(), false);
                }
                return;
            }
            b => b,
        };
        match (b, &mut self.mode) {
            (Bg::Changes(dir, r, now), _) => {
                if let Some(View::Changes(v)) = &mut self.view
                    && v.dir == dir
                {
                    match r {
                        Ok(r) => {
                            let keep = v.review.as_ref().map(|o| o.sel);
                            let mut r = *r;
                            v.reviewed = views::still_reviewed(
                                &r.files,
                                self.screen.saved.reviewed.get(&design::path_key(&v.dir)),
                                &now,
                                &v.reviewed,
                            );
                            if let Some(sel) = keep {
                                r.sel = sel.min(r.files.len().saturating_sub(1));
                                r.diff_sel = None;
                            }
                            let first = keep.is_none();
                            v.review = Some(r);
                            // Start on the first file not reviewed yet.
                            if first
                                && let Some(i) = v.order().first().copied()
                                && let Some(r) = v.review.as_mut()
                            {
                                r.sel = i;
                                r.diff_sel = None;
                            }
                        }
                        Err(e) => v.error = Some(e),
                    }
                }
            }
            (Bg::Diff(dir, sel, lines), _) => {
                if let Some(View::Changes(v)) = &mut self.view
                    && v.dir == dir
                    && let Some(r) = v.review.as_mut()
                    && r.sel == sel
                {
                    r.diff = lines;
                }
            }
            (Bg::Then(finish), _) => finish(self),
            (Bg::Tree(root, nodes, git), _) => {
                let working = self
                    .snap
                    .terms
                    .values()
                    .filter(|t| matches!(t.status, Status::Working | Status::Blocked))
                    .find(|t| {
                        crate::gitfs::head(&t.cwd)
                            .is_some_and(|h| design::path_key(&h.top) == design::path_key(&root))
                    })
                    .and_then(|t| t.agent.clone());
                if let Some(View::Files(v)) = &mut self.view
                    && v.root == root
                {
                    // Open the folders that hold changes; an agent working here is editing them.
                    for rel in git.keys() {
                        let mut p = root.join(rel);
                        while let Some(parent) = p.parent().map(|x| x.to_path_buf()) {
                            if parent == root {
                                break;
                            }
                            v.expanded.insert(parent.clone());
                            p = parent;
                        }
                    }
                    if let Some(a) = working {
                        v.editing = git.keys().map(|k| (k.clone(), a.clone())).collect();
                    }
                    v.all = nodes;
                    v.git = git;
                    v.loading = false;
                    if let Some((target, line)) = v.reveal.take() {
                        let mut p = target.clone();
                        while let Some(parent) = p.parent().map(|x| x.to_path_buf()) {
                            if !parent.starts_with(&v.root) || parent == v.root {
                                break;
                            }
                            v.expanded.insert(parent.clone());
                            p = parent;
                        }
                        if let Some(i) = v.visible().iter().position(|(_, n)| n.path == target) {
                            v.sel = i;
                        }
                        v.refresh_preview();
                        if line.is_some() && v.markdown() {
                            v.raw = Some(target);
                        }
                        v.scroll = line.map(|l| (l as usize).saturating_sub(4)).unwrap_or(0);
                        v.in_preview = true;
                    }
                    v.refresh_preview();
                }
            }
            (Bg::FolderList(dir, list, repo), _) => {
                if let Mode::Finder(fd) = &mut self.mode
                    && fd.listed(&dir, list, repo)
                {
                    self.dirty = true;
                }
            }
            (Bg::FilesPreview(root, path, lines), _) => {
                if let Some(View::Files(v)) = &mut self.view
                    && v.root == root
                    && v.preview_read(&path, lines)
                {
                    self.dirty = true;
                }
            }
            (Bg::TreeRecent(root, list), _) => {
                if let Some(View::Files(v)) = &mut self.view
                    && v.root == root
                {
                    v.recent_list = Some(list);
                    v.refresh_preview();
                }
            }
            (Bg::Tidy(repo, rows), _) => self.tidy_loaded(repo, rows),
            (Bg::BothDiff(repo, parts), _) => {
                if let Some(View::Both(v)) = &mut self.view
                    && v.repo == repo
                {
                    v.parts = Some(parts);
                }
            }
            (Bg::LinesChanged(dir, Some(counts)), _) => {
                self.screen.lines_changed.insert(dir, counts);
            }
            (Bg::LinesChanged(dir, None), _) => {
                self.screen.lines_changed.remove(&dir);
            }
            (Bg::Overlaps(repo, found), _) => {
                // Say each overlap once, when it first shows up.
                let seen = self.screen.overlaps.get(&repo).cloned().unwrap_or_default();
                let new: Vec<_> = found
                    .iter()
                    .filter(|o| !seen.contains(o))
                    .cloned()
                    .collect();
                if let Some(msg) = super::overlap::say(&new) {
                    self.notify(msg, false);
                }
                // A dismissed one that has cleared may say so again next time.
                self.screen
                    .dismissed
                    .retain(|(r, f)| r != &repo || found.iter().any(|o| &o.file == f));
                self.screen.overlaps.insert(repo, found);
            }
            (Bg::Merged(result, dir), _) => {
                // A merge asked from the Inbox is over either way.
                self.screen.inbox_confirm = None;
                match result {
                    Ok(msg) => {
                        self.notify(
                            format!(
                                "{msg}; closing its sessions and removing the worktree and branch"
                            ),
                            false,
                        );
                        self.cmd(Command::CloseWorktree { path: dir });
                        if matches!(self.view, Some(View::Changes(_))) {
                            self.view = None;
                        }
                    }
                    Err(e) => self.notify(e, true),
                }
            }
            (Bg::Done(result, reload), _) => {
                self.screen.inbox_confirm = None;
                match result {
                    Ok(msg) => self.notify(msg, false),
                    Err(e) => self.notify(e, true),
                }
                if let (true, Some(View::Changes(v))) = (reload, &self.view)
                    && let Some(r) = &v.review
                {
                    let (task, d) = (r.task.clone(), v.dir.clone());
                    self.load_changes(d, task);
                }
            }
            _ => {} // the panel it was for has closed
        }
    }
}
