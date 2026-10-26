//! Saving and rebuilding sessions; recovering agents left asleep by older versions.

use super::*;

/// What makes a pane the same session to you when its process is replaced (resumed after
/// older-version sleep, moved to another folder): who it is and what you called it.
struct Kept {
    agent: Option<String>,
    session: Option<String>,
    cmd: Option<String>,
    label: String,
    name: String,
    first_prompt: String,
    model: String,
    grants: Option<Vec<String>>,
    started_by: Option<TermId>,
    queue: std::collections::VecDeque<String>,
}

impl Kept {
    fn of(t: &Term) -> Kept {
        Kept {
            agent: t.agent.clone(),
            session: t.session.clone(),
            cmd: t.cmd.clone(),
            label: t.label.clone(),
            name: t.name.clone(),
            first_prompt: t.first_prompt.clone(),
            model: t.model.clone(),
            grants: t.grants.clone(),
            started_by: t.started_by,
            queue: t.queue.clone(),
        }
    }

    fn give(self, t: &mut Term) {
        (t.agent, t.session, t.cmd) = (self.agent, self.session, self.cmd);
        (t.label, t.name, t.first_prompt) = (self.label, self.name, self.first_prompt);
        (t.model, t.grants, t.started_by) = (self.model, self.grants, self.started_by);
        t.queue = self.queue;
    }
}

impl Daemon {
    /// `new` took `old`'s place: the helpers `old` started are still its.
    fn takes_over_helpers(&mut self, old: TermId, new: TermId) {
        for t in self.terms.values_mut().filter(|t| t.started_by == Some(old)) {
            t.started_by = Some(new);
        }
    }

    /// A pane or agent ended on its own within `LOSS_HOLD`: it may be a shutdown under way.
    pub(super) fn losing(&self) -> bool {
        self.last_loss.is_some_and(|t| t.elapsed() < LOSS_HOLD)
    }

    /// Write the session file if it changed. `force` saves even while `losing`.
    pub(super) fn persist(&mut self, force: bool) {
        if !self.cfg.restore.enabled || (!force && self.losing()) {
            return;
        }
        let saved = self.saved();
        let Ok(text) = serde_json::to_string(&saved) else { return };
        if text == self.last_saved {
            return;
        }
        match persist::save(&saved) {
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
        let live: Vec<TermId> = self.terms.values().filter(|t| !t.spare).map(|t| t.id).collect();
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
                let slots = [(persist::CheckSlot::Latest, Some(&p.latest)), (persist::CheckSlot::Overridden, p.overridden.as_ref())];
                out.extend(slots.into_iter().filter_map(|(slot, r)| {
                    r.map(|r| persist::CheckTail { term: t.id, name: r.info.name.clone(), slot, text: r.output.clone() })
                }));
            }
        }
        out
    }

    pub(super) fn saved(&self) -> persist::Saved {
        let pane = |term: &Term| persist::SavedPane {
            cwd: Some(term.cwd.clone()),
            cmd: term.cmd.clone(),
            agent: term.agent.clone(),
            session: term.agent.as_ref().and(term.session.clone()),
            unseen: term.status == Status::Done,
            name: if term.name.is_empty() { term.first_prompt.clone() } else { term.name.clone() },
            model: term.model.clone(),
            label: term.label.clone(),
            started_by: term.started_by,
            queue: term.queue.iter().cloned().collect(),
            account: term.account.clone(),
            subagent_activity: term.subagent_activity.clone(),
            checks: term.checks.infos(),
            checks_overridden: term.checks.points.iter().filter_map(|p| p.overridden.as_ref().map(|o| o.info.clone())).collect(),
            check_trees: term.checks.points.iter().filter_map(|p| Some((p.latest.info.name.clone(), p.tree.clone()?))).collect(),
        };
        let workspaces = self
            .workspaces
            .iter()
            .map(|w| persist::SavedWs {
                name: w.name.clone(),
                cwd: w.cwd.clone(),
                worktree: w.worktree,
                color: Some(w.color),
                group: w.group.clone(),
                active_tab: w.tabs.iter().position(|t| t.id == w.active_tab).unwrap_or(0),
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
        let active = self.workspaces.iter().position(|w| Some(w.id) == self.active_ws).unwrap_or(0);
        persist::Saved { workspaces, active, made_worktrees: self.made_worktrees.clone() }
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
        let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem).to_lowercase();
        self.agents.iter().find(|a| a.process.iter().any(|p| p.eq_ignore_ascii_case(&stem))).map(|a| a.name.clone())
    }

    /// The command that brings a saved pane back: an agent's resume command, the pane's
    /// original command, or nothing (a shell). An agent's own start command is never run
    /// again: it carries the task, and the agent would start that task over.
    pub(super) fn restore_cmd(&self, p: &persist::SavedPane) -> Option<String> {
        if self.cfg.restore.agents
            && let Some(def) = p.agent.as_ref().and_then(|n| self.agents.iter().find(|a| &a.name == n))
        {
            let as_account = |cmd: String| p.account.as_deref().map(|a| self.cfg.as_account(a, &cmd)).unwrap_or(cmd);
            if let (Some(tpl), Some(id)) = (&def.resume, &p.session) {
                return Some(as_account(tpl.replace("{session}", id)));
            }
            if let Some(tpl) = &def.resume_last {
                return Some(as_account(tpl.clone()));
            }
        }
        let starts_an_agent = p.agent.is_some() || p.cmd.as_deref().is_some_and(|c| self.agent_in_cmd(c).is_some());
        if self.cfg.restore.commands && !starts_an_agent { p.cmd.clone() } else { None }
    }

    pub(super) fn restore(&mut self, mut saved: persist::Saved) {
        self.made_worktrees = std::mem::take(&mut saved.made_worktrees);
        let check_dir = self.check_dir.clone();
        let mut restored = 0;
        // Every pane's new id by its saved one, and who started it by the saved id: panes come
        // back in any order, so the links are made once all of them are up.
        let mut ids: HashMap<TermId, TermId> = HashMap::new();
        let mut started: Vec<(TermId, TermId)> = Vec::new();
        for sw in saved.workspaces {
            let mut tabs = Vec::new();
            for st in sw.tabs {
                let mut map = HashMap::new();
                for old in st.layout.leaves() {
                    let mut pane = st.panes.get(&old).cloned().unwrap_or_default();
                    // Saved after it lost which agent it was (asleep, before 0.26.2): its
                    // command still says, so it comes back as that agent, resumed.
                    if pane.agent.is_none() {
                        pane.agent = pane.cmd.as_deref().and_then(|c| self.agent_in_cmd(c));
                    }
                    let cwd = pane.cwd.clone().filter(|p| p.is_dir()).unwrap_or_else(|| sw.cwd.clone());
                    let cmd = self.restore_cmd(&pane);
                    match self.spawn(cmd.as_deref(), &cwd, 120, 32) {
                        Ok(new) => {
                            if let Some(t) = self.terms.get_mut(&new) {
                                if self.cfg.restore.output
                                    && let Some(out) = persist::take_output(old)
                                {
                                    t.seed_output(&out);
                                }
                                // Keep the original identity so the next save matches this one.
                                t.cmd = pane.cmd.clone();
                                t.session = pane.session.clone();
                                t.agent = pane.agent.clone();
                                t.restore_unseen = pane.unseen;
                                t.first_prompt = pane.name.clone();
                                t.model = pane.model.clone();
                                t.label = pane.label.clone();
                                t.queue = pane.queue.iter().cloned().collect();
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
                            restored += 1;
                        }
                        Err(e) => tracing::warn!("restoring pane in {}: {e:#}", cwd.display()),
                    }
                }
                let Some(layout) = st.layout.map_leaves(&mut |old| map.get(&old).copied()) else { continue };
                let focus = map.get(&st.focus).copied().unwrap_or_else(|| layout.first_leaf());
                let id = self.next();
                tabs.push(TabInfo { id, name: st.name, layout, focus });
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
                name: if sw.cwd.file_name().is_some_and(|n| n.to_string_lossy() == sw.name) { String::new() } else { sw.name },
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
        self.active_ws = self.workspaces.get(saved.active).or(self.workspaces.first()).map(|w| w.id);
        self.dirty = true;
        tracing::info!("restored {} workspaces, {restored} panes", self.workspaces.len());
    }

    /// The command that resumes an agent's conversation, if its kind supports it.
    pub(super) fn resume_cmd(&self, t: &Term) -> Option<String> {
        let def = t.agent.as_ref().and_then(|n| self.agents.iter().find(|a| &a.name == n))?;
        let resume = match (&def.resume, &t.session) {
            (Some(tpl), Some(id)) => Some(tpl.replace("{session}", id)),
            _ => def.resume_last.clone(),
        };
        // Under the account it was started as: its conversation is in that account's folder.
        resume.map(|cmd| t.account.as_deref().map(|a| self.cfg.as_account(a, &cmd)).unwrap_or(cmd))
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
                    if let Some(l) = tab.layout.map_leaves(&mut |id| Some(if id == old { new } else { id })) {
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
        let Some(t) = self.terms.get(&old) else { return };
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
            n.pending_input = Some((format!("{note}\r").into_bytes(), Instant::now() + Duration::from_secs(45)));
        }
        for w in &mut self.workspaces {
            for tab in &mut w.tabs {
                if tab.layout.contains(old) {
                    if let Some(l) = tab.layout.map_leaves(&mut |id| Some(if id == old { new } else { id })) {
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
