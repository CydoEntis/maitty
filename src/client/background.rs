//! Messages from the server, and results of work done in the background.

use super::*;

impl App {
    /// A message from the server can close a popup with no key pressed: shortcut mode held
    /// under it comes back then, not after a key has gone to the pane.
    pub(super) fn on_server(&mut self, msg: ServerMsg) {
        self.apply_server(msg);
        self.keep_shortcut_mode();
    }

    fn apply_server(&mut self, msg: ServerMsg) {
        // Output doesn't change the sidebar; everything else might. Output for a pane you
        // can't see changes nothing on screen at all: no redraw for it.
        match &msg {
            ServerMsg::Output { term, .. } => {
                if self.panes.iter().any(|(t, _)| t == term) || self.panes.is_empty() {
                    self.dirty = true;
                }
            }
            _ => {
                self.screen_fresh();
                self.dirty = true;
            }
        }
        match msg {
            ServerMsg::State(s) => {
                self.settling.retain(|id, (status, _)| s.terms.get(id).is_some_and(|t| t.status == *status));
                self.parsers.retain(|id, _| s.terms.contains_key(id));
                for (id, t) in &s.terms {
                    if !self.parsers.contains_key(id) {
                        let p = self.new_parser(t.rows, t.cols);
                        self.parsers.insert(*id, p);
                    }
                    // Trust the daemon's size for panes we aren't showing.
                    if !self.panes.iter().any(|(p, _)| p == id) {
                        self.sizes.insert(*id, (t.cols, t.rows));
                    }
                }
                let first = !self.got_state;
                let last_session_closed = !self.snap.terms.is_empty() && s.terms.is_empty();
                self.got_state = true;
                if !first {
                    self.remember_changes(&s);
                }
                self.snap = s;
                if self.daemon_restart_reconnect {
                    let daemon_matches_install = self.installed_version.as_ref().and_then(|v| v.as_ref().ok())
                        .is_some_and(|installed| installed == &self.snap.daemon_version);
                    self.daemon_restart_requested = false;
                    self.daemon_restart_reconnect = false;
                    if daemon_matches_install {
                        self.notify(concat!("The ", env!("APP_NAME"), " daemon restarted with the update.").into(), false);
                    }
                }
                if first {
                    self.on_first_state();
                }
                self.screen_sync();
                self.check_view_sync();
                self.maybe_restart_daemon(last_session_closed);
            }
            ServerMsg::Replay { term, cols, rows, data } => {
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
                    && let Some(t) = self.snap.terms.get(&term)
                {
                    let p = self.new_parser(t.rows, t.cols);
                    self.parsers.insert(term, p);
                }
                self.feed(term, &data);
            }
            ServerMsg::Attention { term, status } => {
                self.settling.remove(&term);
                if (status != Status::Done || !self.is_helper(term))
                    && let Some(delay) = crate::alert::settle_delay(status)
                {
                    self.settling.insert(term, (status, Instant::now() + delay));
                }
            }
            ServerMsg::Clipboard { term, text } => {
                let n = text.chars().count();
                copy::to_clipboard(&text);
                let who = self.session_name(term);
                self.notify(format!("{who} copied {n} character{} to your clipboard", if n == 1 { "" } else { "s" }), false);
            }
            ServerMsg::Error(e) => {
                // The output a failure waited for may be what failed; a later reply is not its.
                self.screen.failure_ask = None;
                self.notify(e, true);
            }
            ServerMsg::Notice(n) => {
                self.undo_hint = false;
                self.notify(n, false)
            }
            ServerMsg::AskRemoveWorktree { path, name, why } => self.ask_remove_worktree(path, name, why),
            ServerMsg::AutoWorkspace(n) => {
                self.notify(n, false);
                self.undo_hint = true;
                if let Some((_, at, _)) = &mut self.notice {
                    // Leave time to undo.
                    *at = Instant::now() + Duration::from_secs(10);
                }
            }
            ServerMsg::Raise => crate::reveal::raise_window(),
            // The server stopped for the update: an older window can't speak to the new one, so
            // the installed the app takes its place; a current window just connects again.
            ServerMsg::Bye if self.daemon_restart_requested && self.window_stale() => self.relaunch_window(),
            ServerMsg::Bye if self.daemon_restart_requested => self.daemon_restart_reconnect = true,
            ServerMsg::Bye => self.quit = Some("server exited".into()),
            ServerMsg::Reply(Reply::Worktrees(list)) => {
                if let Mode::Worktrees { items, .. } = &mut self.mode {
                    *items = Some(list);
                }
            }
            ServerMsg::Reply(Reply::Text(text)) => self.check_output_arrived(text),
            ServerMsg::Welcome { .. } | ServerMsg::Reply(_) => {}
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
        if self.snap.workspaces.is_empty() || open.is_some() {
            let cwd = open.or_else(|| self.cfg.start_dir()).or_else(|| std::env::current_dir().ok());
            self.cmd(Command::NewWorkspace { cwd, name: None, cmd: None });
        }
    }

    /// Whether another session's agent started this one (and is still there to hear of it).
    fn is_helper(&self, term: TermId) -> bool {
        self.snap.terms.get(&term).and_then(|t| t.started_by).is_some_and(|by| self.snap.terms.contains_key(&by))
    }

    /// Recheck queued alerts after short-lived prompts and automatic continuations settle.
    pub(super) fn settle_alerts(&mut self) {
        let now = Instant::now();
        let due: Vec<_> = self.settling.iter().filter(|(_, (_, at))| *at <= now).map(|(t, (s, _))| (*t, *s)).collect();
        for (term, status) in due {
            self.settling.remove(&term);
            if self.snap.terms.get(&term).is_some_and(|t| t.status == status) {
                self.alert_now(term, status);
                self.dirty = true;
            }
        }
    }

    /// Needs-you alerts ring even in the focused pane; background completions use a soft note.
    /// With no window open the server shows a desktop notification instead.
    fn alert_now(&mut self, term: TermId, status: Status) {
        if !self.snap.terms.get(&term).is_some_and(|t| t.status == status) {
            return;
        }
        if status == Status::Done && self.focused() == Some(term) && self.window_focused {
            return;
        }
        // The same news again soon, or another agent just rang: the note says it, quietly.
        let now = Instant::now();
        if let Some(kind) = crate::alert::Kind::for_status(status)
            && crate::alert::should_alert(
                &status,
                self.alerted.get(&term).map(|(s, at)| (s, *at)),
                self.last_alert.filter(|(previous, _)| *previous == kind).map(|(_, at)| at),
                now,
            )
        {
            self.alerted.insert(term, (status, now));
            self.last_alert = Some((kind, now));
            crate::alert::ring(&self.cfg.notify, kind);
        }
        if self.focused() == Some(term) {
            return;
        }
        let who = self.describe_term(term);
        let what = if status == Status::Blocked { "needs you" } else { "finished" };
        self.notify(format!("{who} {what}"), false);
        self.notice_term = Some(term);
    }

    /// Note what changed between two states: agents that now need you or finished.
    pub(super) fn remember_changes(&mut self, new: &crate::protocol::Snapshot) {
        let mut events = Vec::new();
        for (id, t) in &new.terms {
            let old = self.snap.terms.get(id);
            let name = if t.label.trim().is_empty() { t.display_name() } else { t.label.trim().to_string() };
            let place = t.top.as_ref().or(t.root.as_ref()).and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let who = if place.is_empty() { name } else { format!("{name} in {place}") };
            if old.is_some_and(|o| o.status != t.status) {
                match t.status {
                    Status::Working => {
                        self.alerted.remove(id);
                    }
                    Status::Blocked => events.push((*id, '!', format!("{who} needs you"))),
                    Status::Done => {
                        let said = t.said.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
                        events.push((*id, '✓', if said.is_empty() { format!("{who} finished") } else { format!("{who} finished: {said}") }));
                    }
                    _ => {}
                }
            }
        }
        for (id, k, text) in events {
            self.remember(Some(id), k, text);
        }
    }

    /// The Changes view's selected file has no diff yet: work it out in the background.
    pub(super) fn request_diff(&mut self) {
        let Some(View::Changes(v)) = &mut self.view else { return };
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

    pub(super) fn spawn_bg(&self, f: impl FnOnce() -> Bg + Send + 'static) {
        let tx = self.bg.clone();
        let job = move || {
            let _ = tx.send(f());
        };
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
                if let Mode::Find(v) = &mut self.mode
                    && v.dir == dir
                {
                    v.files = Some(list);
                    v.refresh_preview();
                }
                self.dirty = true;
                return;
            }
            Bg::Grep(dir, seq, hits) => {
                if let Mode::Find(v) = &mut self.mode
                    && v.dir == dir
                    && v.seq == seq
                {
                    v.hits = Some(hits);
                    v.sel = 0;
                    v.refresh_preview();
                }
                self.dirty = true;
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
            (Bg::Changes(dir, r), _) => {
                if let Some(View::Changes(v)) = &mut self.view
                    && v.dir == dir
                {
                    match r {
                        Ok(r) => {
                            let keep = v.review.as_ref().map(|o| o.sel);
                            let mut r = *r;
                            v.reviewed = views::still_reviewed(&v.dir, &r.files, self.screen.saved.reviewed.get(&design::path_key(&v.dir)));
                            if let Some(sel) = keep {
                                r.sel = sel.min(r.files.len().saturating_sub(1));
                                r.diff_sel = None;
                            }
                            let first = keep.is_none();
                            v.review = Some(r);
                            // Start on the first file not reviewed yet.
                            if first && let Some(i) = v.order().first().copied() && let Some(r) = v.review.as_mut() {
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
                    .find(|t| crate::gitfs::head(&t.cwd).is_some_and(|h| design::path_key(&h.top) == design::path_key(&root)))
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
                let new: Vec<_> = found.iter().filter(|o| !seen.contains(o)).cloned().collect();
                if let Some(msg) = super::overlap::say(&new) {
                    self.notify(msg, false);
                }
                // A dismissed one that has cleared may say so again next time.
                self.screen.dismissed.retain(|(r, f)| r != &repo || found.iter().any(|o| &o.file == f));
                self.screen.overlaps.insert(repo, found);
            }
            (Bg::Merged(result, dir), _) => {
                // A merge asked from the Inbox is over either way.
                self.screen.inbox_confirm = None;
                match result {
                    Ok(msg) => {
                        self.notify(format!("{msg}; closing its sessions and removing the worktree and branch"), false);
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
                    self.spawn_bg(move || Bg::Changes(d, tasks::load_review(task).map(Box::new)));
                }
            }
            _ => {} // the panel it was for has closed
        }
    }
}
