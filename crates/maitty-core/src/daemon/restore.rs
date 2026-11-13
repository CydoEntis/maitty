//! Saving and rebuilding sessions; recovering agents left asleep by older versions.

use super::*;
use std::collections::HashSet;
use std::path::Path;

/// What makes a pane the same session to you when its process is replaced (resumed after
/// older-version sleep, moved to another folder): who it is and what you called it.
struct Kept {
    agent: Option<String>,
    session: Option<String>,
    cmd: Option<String>,
    label: String,
    pinned: bool,
    archived: bool,
    name: String,
    first_prompt: String,
    model: String,
    grants: Option<Vec<String>>,
    started_by: Option<TermId>,
    tab_of: Option<TermId>,
    queue: std::collections::VecDeque<String>,
    held_message: Option<String>,
}

impl Kept {
    fn of(t: &Term) -> Kept {
        Kept {
            agent: t.agent.clone(),
            session: t.session.clone(),
            cmd: t.cmd.clone(),
            label: t.label.clone(),
            pinned: t.pinned,
            archived: t.archived,
            name: t.name.clone(),
            first_prompt: t.first_prompt.clone(),
            model: t.model.clone(),
            grants: t.grants.clone(),
            started_by: t.started_by,
            tab_of: t.tab_of,
            queue: t.queue.clone(),
            held_message: t.held_message.as_ref().map(|held| held.text.clone()),
        }
    }

    fn give(self, t: &mut Term) {
        (t.agent, t.session, t.cmd) = (self.agent, self.session, self.cmd);
        (t.label, t.name, t.first_prompt) = (self.label, self.name, self.first_prompt);
        (t.pinned, t.archived) = (self.pinned, self.archived);
        (t.model, t.grants, t.started_by, t.tab_of) =
            (self.model, self.grants, self.started_by, self.tab_of);
        t.queue = self.queue;
        t.held_message = self.held_message.as_deref().map(term::HeldMessage::new);
    }
}

impl Daemon {
    /// `new` took `old`'s place: the helpers `old` started, and the panes in its tabs, are
    /// still its.
    fn takes_over_helpers(&mut self, old: TermId, new: TermId) {
        for t in self.terms.values_mut() {
            if t.started_by == Some(old) {
                t.started_by = Some(new);
            }
            if t.tab_of == Some(old) {
                t.tab_of = Some(new);
            }
        }
    }

    /// A pane or agent ended on its own within `LOSS_HOLD`: it may be a shutdown under way.
    pub(super) fn losing(&self) -> bool {
        self.last_loss.is_some_and(|t| t.elapsed() < LOSS_HOLD)
    }

    /// Write the session file if it changed. `force` saves even while `losing`.
    pub(super) fn persist(&mut self, force: bool) {
        drop(self.schedule_session_save(force, None));
    }

    pub(super) fn persist_shutdown(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        self.schedule_session_save(true, None)
    }

    fn schedule_session_save(
        &mut self,
        force: bool,
        barrier: Option<u64>,
    ) -> Option<tokio::task::JoinHandle<()>> {
        if (cfg!(test) && self.session_path.as_os_str().is_empty())
            || !self.cfg.restore.enabled
            || (!force && self.losing())
        {
            return None;
        }
        match self.dispatch_session_save(barrier) {
            Ok(task) => task,
            Err(e) => {
                tracing::warn!("saving session: {e:#}");
                None
            }
        }
    }

    pub(super) fn persist_barrier(&mut self, token: u64) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.cfg.restore.enabled,
            "structured operations require session restoration"
        );
        drop(self.dispatch_session_save(Some(token))?);
        Ok(())
    }

    /// Write the session file off the loop. None when the file already holds this session and
    /// no write is on its way (one in flight may land something newer than this): a barrier
    /// still needs its own write acknowledged, nothing else does, and the periodic save would
    /// otherwise fsync the same bytes every two seconds.
    fn dispatch_session_save(
        &mut self,
        barrier: Option<u64>,
    ) -> anyhow::Result<Option<tokio::task::JoinHandle<()>>> {
        anyhow::ensure!(
            self.session_path.is_absolute(),
            "session save requires an absolute file path"
        );
        let saved = self.saved();
        let text = serde_json::to_string(&saved)?;
        if barrier.is_none() && text == self.last_saved && self.saves_in_flight == 0 {
            return Ok(None);
        }
        let ticket = persist::next_session_ticket()?;
        self.saves_in_flight += 1;
        if let Some(token) = barrier {
            self.conversation_barriers.insert(token, ticket);
        }
        let path = self.session_path.clone();
        let tx = self.tx.clone();
        Ok(Some(tokio::task::spawn_blocking(move || {
            let result =
                persist::save_session_ticket(&path, ticket, &saved).map_err(|e| format!("{e:#}"));
            // The daemon may already be gone during shutdown; the committed file remains authoritative.
            let _ = tx.blocking_send(Ev::SessionSaved {
                ticket,
                text,
                result,
                barrier,
            });
        })))
    }

    #[cfg(test)]
    pub(super) fn persist_with(
        &mut self,
        force: bool,
        save: impl FnOnce(&persist::Saved) -> anyhow::Result<()>,
    ) {
        if !self.cfg.restore.enabled || (!force && self.losing()) {
            return;
        }
        let saved = self.saved();
        let Ok(text) = serde_json::to_string(&saved) else {
            return;
        };
        if text == self.last_saved {
            return;
        }
        match save(&saved) {
            Ok(()) => self.last_saved = text,
            Err(e) => tracing::warn!("saving session: {e:#}"),
        }
    }

    /// With `restore.output` off, delete the pane output already saved: it can hold secrets
    /// that were on screen.
    pub(super) fn drop_saved_outputs(&self) {
        if !self.cfg.restore.output
            && let Err(e) = persist::forget_outputs()
        {
            tracing::warn!("deleting saved pane output: {e:#}");
        }
    }

    /// Save panes' output to disk so their history survives the server restarting: what
    /// changed, off the loop (`all`: everything, now, for a server that's stopping). With
    /// `restore.output` off only check tails are saved.
    pub(super) fn save_outputs(&mut self, all: bool) {
        if !self.cfg.restore.enabled {
            return;
        }
        let ticket = persist::save_ticket();
        let live: Vec<TermId> = self
            .terms
            .values()
            .filter(|t| !t.spare)
            .map(|t| t.id)
            .collect();
        let outputs: Vec<(TermId, Vec<u8>)> = self
            .terms
            .values_mut()
            .filter(|t| !t.spare && (all || t.output_changed))
            .map(|t| {
                t.output_changed = false;
                (t.id, t.kept_output())
            })
            .collect();
        self.save_check_tails(ticket, live.clone(), all);
        if !self.cfg.restore.output || (outputs.is_empty() && !all) {
            return;
        }
        let save = move || {
            if let Err(e) = persist::save_outputs(ticket, &outputs, &live) {
                tracing::warn!("saving panes' output: {e:#}");
            }
        };
        if all {
            save();
        } else {
            tokio::task::spawn_blocking(save);
        }
    }

    /// Save the check output tails that changed, off the loop (`all`: now, and drop the tails
    /// of panes not in `live` even if none changed). Tails that could not be written are
    /// marked due again, so a failed write loses nothing.
    pub(super) fn save_check_tails(&mut self, ticket: u64, live: Vec<TermId>, all: bool) {
        let tails = self.take_dirty_check_outputs();
        if tails.is_empty() && !all {
            return;
        }
        let dir = self.check_dir.clone();
        if all {
            let unsaved = persist::save_check_outputs(&dir, ticket, &tails, &live);
            self.check_outputs_unsaved(&unsaved);
            return;
        }
        let tx = self.tx.clone();
        tokio::task::spawn_blocking(move || {
            let unsaved = persist::save_check_outputs(&dir, ticket, &tails, &live);
            if !unsaved.is_empty() {
                let _ = tx.blocking_send(Ev::CheckOutputsUnsaved(unsaved));
            }
        });
    }

    /// The check output tails that changed since they were last written, marked as written.
    pub(super) fn take_dirty_check_outputs(&mut self) -> Vec<persist::CheckTail> {
        let mut out = Vec::new();
        for t in self.terms.values_mut().filter(|t| t.checks.dirty) {
            t.checks.dirty = false;
            for p in &t.checks.points {
                let slots = [
                    (persist::CheckSlot::Latest, Some(&p.latest)),
                    (persist::CheckSlot::Overridden, p.overridden.as_ref()),
                ];
                out.extend(slots.into_iter().filter_map(|(slot, r)| {
                    r.map(|r| persist::CheckTail {
                        term: t.id,
                        name: r.info.name.clone(),
                        slot,
                        text: r.output.clone(),
                    })
                }));
            }
        }
        out
    }

    pub(super) fn saved(&self) -> persist::Saved {
        let pane = |term: &Term| persist::SavedPane {
            conversation_id: term.conversation_id,
            cwd: Some(term.cwd.clone()),
            cmd: term.cmd.clone(),
            agent: term.agent.clone(),
            session: term.agent.as_ref().and(term.session.clone()),
            unseen: term.status == Status::Done,
            was_working: term.status == Status::Working,
            name: if term.name.is_empty() {
                term.first_prompt.clone()
            } else {
                term.name.clone()
            },
            model: term.model.clone(),
            label: term.label.clone(),
            pinned: term.pinned,
            archived: term.archived,
            started_by: term.started_by,
            owner: term.owner,
            tab_of: term.tab_of,
            queue: term.queue.iter().cloned().collect(),
            held_message: term.held_message.as_ref().map(|held| held.text.clone()),
            account: term.account.clone(),
            subagent_activity: term.subagent_activity.clone(),
            checks: term.checks.infos(),
            checks_overridden: term
                .checks
                .points
                .iter()
                .filter_map(|p| p.overridden.as_ref().map(|o| o.info.clone()))
                .collect(),
            check_trees: term
                .checks
                .points
                .iter()
                .filter_map(|p| Some((p.latest.info.name.clone(), p.tree.clone()?)))
                .collect(),
        };
        // A session still inside its undo window is saved as closed: a restart ends it.
        let visible = self.visible_workspaces();
        let workspaces = visible
            .iter()
            .map(|w| persist::SavedWs {
                name: w.name.clone(),
                cwd: w.cwd.clone(),
                worktree: w.worktree,
                color: Some(w.color),
                group: w.group.clone(),
                active_tab: w
                    .tabs
                    .iter()
                    .position(|t| t.id == w.active_tab)
                    .unwrap_or(0),
                tabs: w
                    .tabs
                    .iter()
                    .map(|t| persist::SavedTab {
                        name: t.name.clone(),
                        layout: t.layout.clone(),
                        focus: t.focus,
                        panes: t
                            .layout
                            .leaves()
                            .iter()
                            .filter_map(|id| self.terms.get(id))
                            .map(|term| (term.id, pane(term)))
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        let active = visible
            .iter()
            .position(|w| Some(w.id) == self.active_ws)
            .unwrap_or(0);
        persist::Saved {
            workspaces,
            active,
            made_worktrees: self.made_worktrees.clone(),
            conversations: self.conversations.clone(),
            pending_starts: self.pending_starts.clone(),
            conversation_operations: self.conversation_operations.clone(),
            turn_continues: self.turn_continues.clone(),
            turns_in_flight: self.turns_in_flight(),
        }
    }

    /// Chat conversations in a turn: running, reconciled with their history, and working. A
    /// conversation still starting up is working too, but its history is not read yet.
    fn turns_in_flight(&self) -> Vec<crate::conversation::ConversationId> {
        self.conversations
            .iter()
            .filter(|c| c.state == crate::conversation::RecoveryState::Running)
            .filter(|c| {
                self.conversation_states
                    .get(&c.id)
                    .is_some_and(|state| state.history_ready && state.status == Status::Working)
            })
            .map(|c| c.id)
            .collect()
    }

    /// The agent a command line starts (`codex "…"`, `"C:\…\claude.exe" …`), by its program,
    /// for a pane whose record lost which agent it was.
    pub(super) fn agent_in_cmd(&self, cmd: &str) -> Option<String> {
        let (_, cmd) = self.cfg.split_account(cmd);
        let cmd = cmd.trim_start().trim_start_matches("& ");
        let program = match cmd.strip_prefix('"') {
            Some(rest) => rest.split('"').next(),
            None => cmd.split_whitespace().next(),
        }?;
        // Both separators: a saved command may be a Windows path on any machine.
        let file = program.rsplit(['/', '\\']).next()?;
        let stem = file
            .rsplit_once('.')
            .map_or(file, |(stem, _)| stem)
            .to_lowercase();
        self.agents
            .iter()
            .find(|a| a.process.iter().any(|p| p.eq_ignore_ascii_case(&stem)))
            .map(|a| a.name.clone())
    }

    /// The command that brings a saved pane back: an agent's resume command, the pane's
    /// original command, or nothing (a shell). An agent's own start command is never run
    /// again: it carries the task, and the agent would start that task over.
    #[cfg(test)]
    pub(super) fn restore_cmd(&self, p: &persist::SavedPane) -> Option<String> {
        if p.conversation_id.is_some() {
            return None;
        }
        self.agent_resume_cmd(p).or_else(|| self.saved_cmd(p))
    }

    /// `restore_cmd` for one pane of a restore that brings back several. A conversation comes
    /// back in one pane: two agents on one conversation write over each other's history. Two
    /// panes with the same session, or two of an agent with no session in one folder (each
    /// would resume "the last one"), would both resume it, so only the first does and the
    /// rest come back as plain shells that no longer claim to be that agent.
    pub(super) fn restore_cmd_once(
        &self,
        p: &mut persist::SavedPane,
        cwd: &Path,
        resuming: &mut HashSet<(PathBuf, String)>,
    ) -> Option<String> {
        if p.conversation_id.is_some() {
            return None;
        }
        let Some(resume) = self.agent_resume_cmd(p) else {
            return self.saved_cmd(p);
        };
        if resuming.insert((cwd.to_path_buf(), resume.clone())) {
            return Some(resume);
        }
        tracing::warn!(
            "restore: another pane in {} already resumes this conversation; this one is a shell",
            cwd.display()
        );
        (p.agent, p.session, p.cmd) = (None, None, None);
        None
    }

    fn agent_resume_cmd(&self, p: &persist::SavedPane) -> Option<String> {
        if self.cfg.restore.agents
            && let Some(def) = p
                .agent
                .as_ref()
                .and_then(|n| self.agents.iter().find(|a| &a.name == n))
        {
            let as_account = |cmd: String| {
                p.account
                    .as_deref()
                    .map(|a| self.cfg.as_account(a, &cmd))
                    .unwrap_or(cmd)
            };
            if let (Some(tpl), Some(id)) = (&def.resume, &p.session) {
                return Some(as_account(tpl.replace("{session}", id)));
            }
            if let Some(tpl) = &def.resume_last {
                return Some(as_account(tpl.clone()));
            }
        }
        None
    }

    fn saved_cmd(&self, p: &persist::SavedPane) -> Option<String> {
        let starts_an_agent = p.agent.is_some()
            || p.cmd
                .as_deref()
                .is_some_and(|c| self.agent_in_cmd(c).is_some());
        if self.cfg.restore.commands && !starts_an_agent {
            p.cmd.clone()
        } else {
            None
        }
    }

    /// Start again each chat conversation that was running when the last daemon went away, on
    /// its own provider session (ADR 0003). Each is tried once: the list is emptied here, so a
    /// runtime that fails again stays failed until a person resumes it.
    pub(super) fn resume_conversations(&mut self) {
        let candidates = std::mem::take(&mut self.resumable_conversations);
        let cut = std::mem::take(&mut self.cut_conversations);
        if !self.cfg.restore.conversations {
            return;
        }
        for (id, generation) in candidates {
            let target = crate::conversation::StartTarget::Conversation { id, generation };
            match self.conversation_start(conversation::commands::DAEMON_REQUESTER, target) {
                Ok(()) if cut.contains(&id) => self.continue_when_resumed(id),
                Ok(()) => {}
                Err(error) => tracing::warn!("conversation {} was not resumed: {error:#}", id.0),
            }
        }
    }

    pub(super) fn restore_startup(&mut self, saved: persist::Saved) {
        // Recovery changes must differ from the snapshot actually on disk until saved.
        self.last_saved = serde_json::to_string(&saved).unwrap_or_default();
        if !self.restore(saved) && !self.session_path.as_os_str().is_empty() {
            persist::keep_unusable_now(&self.session_path);
        }
    }

    /// Whether the saved session was accepted. A refused one leaves the daemon empty, so the
    /// caller keeps the file before the next save replaces it.
    pub(super) fn restore(&mut self, mut saved: persist::Saved) -> bool {
        if let Err(e) = crate::conversation::validate_metadata(
            &saved.conversations,
            &saved.pending_starts,
            &saved.conversation_operations,
        ) {
            tracing::warn!("refusing invalid conversation registry: {e:?}");
            return false;
        }
        let mut resumable = Vec::new();
        let mut cut = std::collections::BTreeSet::new();
        for record in &mut saved.conversations {
            let was_running = record.state == crate::conversation::RecoveryState::Running
                && saved.turns_in_flight.contains(&record.id);
            match record.recover() {
                Ok(true) if record.execution == crate::conversation::ExecutionKind::Structured => {
                    resumable.push((record.id, record.generation));
                    if was_running {
                        cut.insert(record.id);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("refusing conversation recovery: {e:?}");
                    return false;
                }
            }
        }
        self.resumable_conversations = resumable;
        self.cut_conversations = cut;
        self.turn_continues = std::mem::take(&mut saved.turn_continues);
        self.conversations = std::mem::take(&mut saved.conversations);
        self.pending_starts = std::mem::take(&mut saved.pending_starts);
        self.conversation_operations = std::mem::take(&mut saved.conversation_operations);
        self.made_worktrees = std::mem::take(&mut saved.made_worktrees);
        let check_dir = self.check_dir.clone();
        let mut restored = 0;
        // Every pane's new id by its saved one, and who started it by the saved id: panes come
        // back in any order, so the links are made once all of them are up.
        let mut ids: HashMap<TermId, TermId> = HashMap::new();
        let mut started: Vec<(TermId, TermId)> = Vec::new();
        let mut owned: Vec<(TermId, TermId)> = Vec::new();
        let mut resuming = HashSet::new();
        for sw in saved.workspaces {
            let mut tabs = Vec::new();
            for st in sw.tabs {
                let mut map = HashMap::new();
                for old in st.layout.leaves() {
                    // A slot with no saved pane is one that was lost before the save; giving it a
                    // default shell is how a restart grew extra shells.
                    let Some(mut pane) = st.panes.get(&old).cloned() else {
                        tracing::warn!("restore: layout slot {old} has no saved pane; skipped");
                        continue;
                    };
                    // Linked runtimes require explicit recovery; legacy resume may choose --last.
                    if pane.conversation_id.is_some() {
                        continue;
                    }
                    // Saved after it lost which agent it was (asleep, before 0.26.2): its
                    // command still says, so it comes back as that agent, resumed.
                    if pane.agent.is_none() {
                        pane.agent = pane.cmd.as_deref().and_then(|c| self.agent_in_cmd(c));
                    }
                    let cwd = pane
                        .cwd
                        .clone()
                        .filter(|p| p.is_dir())
                        .unwrap_or_else(|| sw.cwd.clone());
                    let cmd = self.restore_cmd_once(&mut pane, &cwd, &mut resuming);
                    // An agent that died mid-turn is not started again on its own, whichever app
                    // owns the session: resumed, it would sit at its prompt as if it had finished,
                    // and nothing would say its turn was cut. The pane comes back as a shell and
                    // offers the resume command.
                    let interrupted = pane
                        .was_working
                        .then(|| pane.agent.clone().zip(cmd.clone()))
                        .flatten()
                        .map(|(agent, resume)| Interrupted { agent, resume });
                    let cmd = cmd.filter(|_| interrupted.is_none());
                    match self.spawn(cmd.as_deref(), &cwd, 120, 32) {
                        Ok(new) => {
                            if let Some(t) = self.terms.get_mut(&new) {
                                if self.cfg.restore.output
                                    && let Some(out) = persist::take_output(old)
                                {
                                    t.seed_output(&out);
                                }
                                // Keep the original identity so the next save matches this one.
                                if interrupted.is_none() {
                                    t.cmd = pane.cmd.clone();
                                    t.session = pane.session.clone();
                                    t.agent = pane.agent.clone();
                                }
                                t.interrupted = interrupted;
                                t.restore_unseen = pane.unseen;
                                t.first_prompt = pane.name.clone();
                                t.model = pane.model.clone();
                                t.label = pane.label.clone();
                                (t.pinned, t.archived) = (pane.pinned, pane.archived);
                                t.owner = pane.owner;
                                t.queue = pane.queue.iter().cloned().collect();
                                t.held_message =
                                    pane.held_message.as_deref().map(term::HeldMessage::new);
                                t.restore_checks(&check_dir, old, &pane);
                                let now = term::unix_now();
                                t.subagent_activity = pane.subagent_activity.into_iter().map(|mut activity| {
                                    if activity.state.active() {
                                        activity.state = SubagentState::ConnectionLost;
                                        activity.updated_at = now;
                                        activity.latest_activity = Some(concat!(env!("APP_NAME"), " restarted while the child was active; completion unconfirmed").into());
                                    }
                                    activity
                                }).collect();
                            }
                            map.insert(old, new);
                            ids.insert(old, new);
                            started.extend(pane.started_by.map(|by| (new, by)));
                            owned.extend(pane.tab_of.map(|owner| (new, owner)));
                            restored += 1;
                        }
                        Err(e) => tracing::warn!("restoring pane in {}: {e:#}", cwd.display()),
                    }
                }
                let Some(layout) = st.layout.map_leaves(&mut |old| map.get(&old).copied()) else {
                    continue;
                };
                let focus = map
                    .get(&st.focus)
                    .copied()
                    .unwrap_or_else(|| layout.first_leaf());
                let id = self.next();
                tabs.push(TabInfo {
                    id,
                    name: st.name,
                    layout,
                    focus,
                });
            }
            if tabs.is_empty() {
                continue;
            }
            let active_tab = tabs[sw.active_tab.min(tabs.len() - 1)].id;
            let id = self.next();
            let color = sw.color.unwrap_or_else(|| self.free_color());
            self.workspaces.push(WorkspaceInfo {
                id,
                // Older sessions named panes after their folder; treat that as no name.
                name: if sw
                    .cwd
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy() == sw.name)
                {
                    String::new()
                } else {
                    sw.name
                },
                cwd: sw.cwd,
                tabs,
                active_tab,
                git: None,
                worktree: sw.worktree,
                color,
                is_new: false,
                group: sw.group.clone(),
            });
        }
        for (term, by) in started {
            // The session that started it didn't come back: it's on its own.
            self.started(term, ids.get(&by).copied());
        }
        for (term, owner) in owned {
            if let Some(t) = self.terms.get_mut(&term) {
                t.tab_of = ids.get(&owner).copied();
            }
        }
        self.active_ws = self
            .workspaces
            .get(saved.active)
            .or(self.workspaces.first())
            .map(|w| w.id);
        self.dirty = true;
        tracing::info!(
            "restored {} workspaces, {restored} panes",
            self.workspaces.len()
        );
        true
    }

    /// The command that resumes an agent's conversation, if its kind supports it.
    pub(super) fn resume_cmd(&self, t: &Term) -> Option<String> {
        let def = t
            .agent
            .as_ref()
            .and_then(|n| self.agents.iter().find(|a| &a.name == n))?;
        let resume = match (&def.resume, &t.session) {
            (Some(tpl), Some(id)) => Some(tpl.replace("{session}", id)),
            _ => def.resume_last.clone(),
        };
        // Under the account it was started as: its conversation is in that account's folder.
        resume.map(|cmd| {
            t.account
                .as_deref()
                .map(|a| self.cfg.as_account(a, &cmd))
                .unwrap_or(cmd)
        })
    }

    /// Restart an agent that is running, in the same spot and conversation, so it loads hooks
    /// installed since it started. Its process tree is stopped first; `wake` does the rest.
    pub(super) fn reload_agent(&mut self, old: TermId) -> Option<TermId> {
        self.terms.get_mut(&old)?.kill_tree();
        self.wake(old)
    }

    /// Bring a sleeping agent back: start its resume command in the same spot.
    pub(super) fn wake(&mut self, old: TermId) -> Option<TermId> {
        let t = self.terms.get(&old)?;
        let cmd = self.resume_cmd(t).or_else(|| t.cmd.clone());
        let (cwd, cols, rows) = (t.cwd.clone(), t.cols, t.rows);
        let kept = Kept::of(t);
        let new = match self.spawn(cmd.as_deref(), &cwd, cols, rows) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("waking {old}: {e:#}");
                return None;
            }
        };
        if let Some(n) = self.terms.get_mut(&new) {
            kept.give(n);
        }
        for w in &mut self.workspaces {
            for tab in &mut w.tabs {
                if tab.layout.contains(old) {
                    if let Some(l) = tab
                        .layout
                        .map_leaves(&mut |id| Some(if id == old { new } else { id }))
                    {
                        tab.layout = l;
                    }
                    if tab.focus == old {
                        tab.focus = new;
                    }
                }
            }
        }
        self.takes_over_helpers(old, new);
        // Closing a terminal can block on Windows until its programs let go; never here.
        if let Some(t) = self.terms.remove(&old) {
            std::thread::spawn(move || drop(t));
        }
        if let Some((ws, tab)) = self.locate(new) {
            if let Ok(w) = self.ws_mut(ws) {
                w.active_tab = tab;
            }
            self.active_ws = Some(ws);
        }
        self.dirty = true;
        Some(new)
    }

    /// Restart an agent in another folder, in the same conversation: its transcript is put
    /// where the agent looks for that folder (Claude files them by folder), then it resumes
    /// in place of the old pane and is told where it now is.
    pub(super) fn relocate(&mut self, old: TermId, dest: &std::path::Path) {
        let Some(t) = self.terms.get(&old) else {
            return;
        };
        if let (Some(src), Some(session)) = (&t.transcript, &t.session) {
            copy_claude_transcript(src, session, dest);
        }
        let cmd = self.resume_cmd(t);
        let (cols, rows) = (t.cols, t.rows);
        let kept = Kept::of(t);
        if let Some(t) = self.terms.get_mut(&old) {
            t.kill_tree();
        }
        let new = match self.spawn(cmd.as_deref(), dest, cols, rows) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("moving {old} to {}: {e:#}", dest.display());
                return;
            }
        };
        if let Some(n) = self.terms.get_mut(&new) {
            kept.give(n);
            let note = format!(
                "You've been moved into the git worktree at {}. Your conversation continues here; make every further change in this folder. Carry on with the task.",
                dest.display()
            );
            // Only once it's up (its hooks say so), never into a startup question.
            n.pending_input = Some((
                format!("{note}\r").into_bytes(),
                Instant::now() + Duration::from_secs(45),
            ));
        }
        for w in &mut self.workspaces {
            for tab in &mut w.tabs {
                if tab.layout.contains(old) {
                    if let Some(l) = tab
                        .layout
                        .map_leaves(&mut |id| Some(if id == old { new } else { id }))
                    {
                        tab.layout = l;
                    }
                    if tab.focus == old {
                        tab.focus = new;
                    }
                }
            }
            if w.tabs.iter().any(|tab| tab.layout.contains(new)) {
                w.cwd = dest.to_path_buf();
            }
        }
        self.takes_over_helpers(old, new);
        self.remove_term(old);
        self.poll_git_soon();
        self.dirty = true;
    }
}
