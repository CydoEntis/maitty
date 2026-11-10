//! What each action does, and the helpers they share.

use super::*;

impl App {
    pub(super) fn act(&mut self, a: Action) {
        // A chat with no terminal can't be ended or renamed as a pane, but it can be put away
        // and named: the daemon keeps both for every client.
        if let Some(id) = self
            .screen
            .cursor
            .and_then(|key| self.bare_conversation(key))
        {
            match a {
                Action::ClosePane => return self.menu_act(menu::Act::ArchiveConversation(id)),
                Action::RenameWorkspace => {
                    return self.menu_do(menu::Act::RenameConversation(id));
                }
                _ => {}
            }
        }
        if let Some(key) = self.screen.cursor
            && self.session_term(key).is_none()
            && matches!(
                a,
                Action::ClosePane
                    | Action::CloseTab
                    | Action::CloseSplit
                    | Action::StopAgent
                    | Action::RenameTab
                    | Action::RenameWorkspace
                    | Action::NextTab
                    | Action::PrevTab
                    | Action::SelectTab(_)
                    | Action::SplitRight
                    | Action::SplitDown
                    | Action::SplitLeft
                    | Action::SplitUp
                    | Action::Spawn(..)
                    | Action::SpawnTab(_)
                    | Action::NewTab
                    | Action::NewSession
                    | Action::Resize(_)
                    | Action::Swap(_)
                    | Action::Message
                    | Action::PrevPrompt
                    | Action::NextPrompt
                    | Action::CopyMode
                    | Action::Search
                    | Action::ScrollUp
                    | Action::ScrollDown
                    | Action::SendPrefix
            )
        {
            self.require_session_term(key);
            return;
        }
        // These read or change the files of the pane they apply to, which on another machine
        // are not on this computer.
        if let Some(feature) = routing::Feature::of(&a)
            && !self.guard_local(
                self.selected_term()
                    .map_or(self.target_machine(), |t| t.machine),
                feature,
            )
        {
            return;
        }
        if self.screen_act(&a) {
            return;
        }
        let ws = self.active_ws().map(|w| w.id);
        let focused = self.selected_term();
        match a {
            Action::PasteImage => self.paste_image(),
            Action::SendFile => self.send_file_action(),
            Action::Find(tab) => self.open_find(tab),
            Action::AgentStatusSetup => self.menu(
                "Connect an agent · it then reports working, needs you and done itself".into(),
                maitty_agent_integrations::STATUS_AGENTS
                    .iter()
                    .map(|a| {
                        (
                            format!("Connect {}", a.name),
                            menu::Act::InstallStatus(a.id.into()),
                        )
                    })
                    .collect(),
                (3, 3),
            ),
            Action::ReloadAgents => self.reload_agents_from_menu(),
            Action::Branches => self.toggle_branch_sheet(),
            Action::References => self.toggle_references_sheet(),
            Action::History => self.mode = Mode::History { sel: 0 },
            Action::Machines => self.open_machines(),
            Action::SpawnTab(c) => {
                if let Some(ws) = self.target_ws() {
                    self.leave_shortcut_mode();
                    self.cmd_on(
                        self.target_machine(),
                        Command::NewTab {
                            ws,
                            name: None,
                            cmd: Some(c),
                        },
                    );
                }
            }
            // x closes what has the keys: a side panel (Files, Changes, Branch, a check) that
            // has them, not the pane behind it, whose program would stop. Set aside by a click
            // into a pane, the pane has the keys and x closes it.
            Action::ClosePane if self.view_has_keys() => {
                let unsaved = match &self.view {
                    Some(View::Files(v)) => v.edit.as_ref().filter(|e| e.dirty).map(|e| e.warned),
                    _ => None,
                };
                if let Some(warned) = unsaved {
                    // After the editor's own warning one Esc throws them away, before it two.
                    let esc = if warned { "Esc again" } else { "Esc twice" };
                    self.notify(
                        format!("unsaved changes: Ctrl+S saves, {esc} throws them away"),
                        true,
                    );
                } else {
                    self.view = None;
                }
            }
            Action::ClosePane => {
                // From the sidebar it's the row the cursor is on, as its x is; anywhere else
                // it's the focused pane, whatever the cursor last rested on.
                let side = matches!(self.mode, Mode::Side | Mode::Prefix { .. });
                let term = if side {
                    self.screen
                        .cursor
                        .and_then(|key| self.session_term(key))
                        .or_else(|| self.focused())
                } else {
                    self.focused()
                };
                if let Some(term) = term {
                    self.menu_act(menu::Act::End(vec![term]));
                }
            }
            Action::RenameTab => {
                if let (Some(ws), Some(tab)) = (ws, self.active_tab()) {
                    self.mode = Mode::Prompt {
                        kind: PromptKind::RenameTab(ws, tab.id),
                        input: tab.name.clone(),
                    };
                }
            }
            Action::NewWorkspace => {
                self.mode = Mode::Prompt {
                    kind: PromptKind::NewWorkspace,
                    input: String::new(),
                };
            }
            Action::CloseWorkspace => {
                if let Some(ws) = ws {
                    self.mode = Mode::Prompt {
                        kind: PromptKind::ConfirmCloseWorkspace(ws),
                        input: String::new(),
                    };
                }
            }
            Action::StopAgent => {
                if let Some(term) = self.target_term() {
                    self.send_input(term, vec![3]);
                }
            }
            Action::ScrollUp | Action::ScrollDown => {
                if let Some(term) = focused {
                    let half = self.sizes.get(&term).map(|s| s.1 / 2).unwrap_or(10).max(1) as i32;
                    self.scroll_by(term, if a == Action::ScrollUp { half } else { -half });
                }
            }
            Action::CopyMode | Action::Search => {
                if let Some(term) = focused
                    && self.enter_copy(term)
                    && a == Action::Search
                    && let Mode::Copy(c) = &mut self.mode
                {
                    c.input = Some(String::new());
                    c.backward = true;
                }
            }
            Action::NewWorktree(cmd) => {
                let Some(w) = self.active_ws() else { return };
                if w.git.is_none() {
                    let here = focused
                        .and_then(|id| self.term_info(id))
                        .map(|t| t.cwd.clone());
                    let hint = match here {
                        Some(h) if h != w.cwd => format!(
                            "workspace {} isn't a git repo — {} M makes this pane's folder a workspace",
                            w.name, self.keymap.prefix
                        ),
                        _ => format!("{} is not in a git repository", w.cwd.display()),
                    };
                    self.notify(hint, true);
                    return;
                }
                let ws = w.id;
                self.send(
                    self.focus_machine(),
                    ClientMsg::Query(Query::Worktrees { ws }),
                );
                self.mode = Mode::Worktrees {
                    ws,
                    cmd,
                    items: None,
                    query: String::new(),
                    sel: 0,
                };
            }
            Action::RemoveWorktree => {
                let Some(w) = self.active_ws() else { return };
                if !w.worktree && !w.git.as_ref().is_some_and(|g| g.linked) {
                    self.notify("this workspace is not a linked worktree".into(), true);
                    return;
                }
                self.mode = Mode::Prompt {
                    kind: PromptKind::ConfirmRemoveWorktree(w.id),
                    input: String::new(),
                };
            }
            Action::Detach => self.quit = Some("detached".into()),
            Action::ReloadConfig => self.reload_config(),
            Action::PaneInfo => {
                if let Some(msg) = self.focused().and_then(|t| self.pane_info(t)) {
                    super::copy::to_clipboard(&msg);
                    self.notify("Pane details copied to clipboard".into(), false);
                }
            }
            Action::SendPrefix => {
                let p = self.keymap.prefix;
                let ev = KeyEvent::new(p.code, p.mods);
                self.forward_key(&ev);
            }
            Action::KillServer => {
                self.mode = Mode::Prompt {
                    kind: PromptKind::ConfirmKillServer,
                    input: String::new(),
                };
            }
            Action::Update => self.ask_update(),
            Action::CheckUpdate => self.check_for_update(true),
            Action::Restart => self.restart_from_menu(),
            Action::ToggleAppearance => self.toggle_appearance(),
            Action::TidyWorktrees => self.tidy_open_here(),
            Action::None => {}
            // The rest are handled by screen_act above.
            _ => {}
        }
    }

    pub(super) fn reload_config(&mut self) {
        match Config::load() {
            Ok(cfg) => {
                self.apply_config(cfg);
            }
            Err(e) => self.notify(format!("config error: {e:#}"), true),
        }
    }

    pub(super) fn apply_config(&mut self, cfg: Config) {
        self.keymap = cfg.keymap();
        self.theme = cfg.theme();
        // Only what the file changed: a sidebar hidden with a key stays hidden.
        if cfg.ui.sidebar != self.cfg.ui.sidebar {
            self.sidebar = cfg.ui.sidebar;
        }
        let mouse_changed = cfg.ui.mouse != self.cfg.ui.mouse;
        self.machines_edited(&cfg.machines);
        self.cfg = cfg;
        if mouse_changed {
            let _ = if self.cfg.ui.mouse {
                execute!(std::io::stdout(), event::EnableMouseCapture)
            } else {
                execute!(std::io::stdout(), event::DisableMouseCapture)
            };
        }
        self.cmd(Command::ReloadConfig);
        if self.keymap.warnings.is_empty() {
            self.notify("config reloaded".into(), false);
        } else {
            self.notify(format!("config: {}", self.keymap.warnings.join("; ")), true);
        }
    }

    /// Every command worth offering in the palette, plus your own spawn bindings.
    pub(super) fn palette_commands(&self) -> Vec<Action> {
        use crate::layout::Dir;
        let mut actions = vec![
            Action::Jump,
            Action::BrowseTree,
            Action::PickSession,
            Action::ShellHere,
            Action::SplitRight,
            Action::SplitDown,
            Action::SplitLeft,
            Action::SplitUp,
            Action::FocusNext,
            Action::FocusPrev,
            Action::Resize(Dir::Left),
            Action::Resize(Dir::Right),
            Action::Resize(Dir::Up),
            Action::Resize(Dir::Down),
            Action::Focus(Dir::Left),
            Action::Focus(Dir::Right),
            Action::Focus(Dir::Up),
            Action::Focus(Dir::Down),
            Action::Swap(Dir::Left),
            Action::Swap(Dir::Right),
            Action::Swap(Dir::Up),
            Action::Swap(Dir::Down),
            Action::ToggleSidebar,
            Action::HideSidebar,
            Action::ClosePane,
            Action::NewTab,
            Action::NextTab,
            Action::PrevTab,
            Action::CloseTab,
            Action::RenameTab,
            Action::NewWorkspace,
            Action::CloseWorkspace,
            Action::RenameWorkspace,
            Action::NewWorktree(None),
            Action::RemoveWorktree,
            Action::Worktrees,
            Action::NewSession,
            Action::CloseSplit,
            Action::StopAgent,
            Action::Message,
            Action::AgentStatusSetup,
            Action::ReloadAgents,
            Action::Files,
            Action::Find(0),
            Action::Find(1),
            Action::Changes,
            Action::Branches,
            Action::References,
            Action::CopyMode,
            Action::Search,
            Action::ScrollUp,
            Action::ScrollDown,
            Action::PrevPrompt,
            Action::NextPrompt,
            Action::PasteImage,
            Action::SendFile,
            Action::History,
            Action::Machines,
            Action::Settings,
            Action::Help,
            Action::ReloadConfig,
            Action::PaneInfo,
            Action::Update,
            Action::CheckUpdate,
            Action::Restart,
            Action::ToggleAppearance,
            Action::TidyWorktrees,
            Action::Detach,
            Action::SendPrefix,
        ];
        let mut bound: Vec<_> = self
            .keymap
            .prefixed
            .iter()
            .chain(self.keymap.global.iter())
            .collect();
        bound.sort_by_key(|(key, _)| key.to_config());
        for (_, action) in bound {
            if !matches!(
                action,
                Action::None
                    | Action::Palette
                    | Action::Menu
                    | Action::SendPrefix
                    | Action::SideMove(_)
            ) && !actions.contains(action)
            {
                actions.push(action.clone());
            }
        }
        actions
    }

    /// The workspace a command applies to: the one on screen.
    pub(super) fn target_ws(&self) -> Option<WsId> {
        self.snap_of(self.target_machine())?.active_ws
    }

    /// The terminal a command applies to: the target pane's focused one.
    pub(super) fn target_term(&self) -> Option<ClientTerm> {
        let machine = self.target_machine();
        self.target_ws()
            .and_then(|ws| self.snap_of(machine)?.workspace(ws))
            .and_then(|w| w.tab())
            .map(|t| ClientTerm::on(machine, t.focus))
            .or(self.focused())
    }

    pub(super) fn open_changes(&mut self, dir: PathBuf) {
        let head = crate::gitfs::head(&dir);
        // Not a git repo: say so, and offer the next step.
        if head.is_none() {
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.mode = Mode::Confirm(Box::new(menu::Confirm {
                title: "changes".into(),
                sub: name,
                what: format!("{} isn't a git repository yet.", dir.display()),
                detail: String::new(),
                note: "Git lets you review and undo what agents change here.".into(),
                list: Vec::new(),
                yes: "Make it a git repo".into(),
                key: 'g',
                danger: false,
                act: menu::Act::GitInit(dir),
            }));
            return;
        }
        let Some((task, info, linked)) = self.task_for(&dir) else {
            return;
        };
        let (top, term, ws) = (
            task.dir.clone(),
            task.agent,
            Some(task.ws).filter(|w| *w != 0),
        );
        self.show_view(View::Changes(Box::new(views::ChangesView {
            dir: top.clone(),
            review: None,
            error: None,
            agent: info
                .as_ref()
                .and_then(|i| i.agent.clone())
                .unwrap_or_default(),
            said: info.as_ref().map(|i| i.said.clone()).unwrap_or_default(),
            term,
            ws,
            reviewed: Default::default(),
            linked,
            confirm: None,
        })));
        self.load_changes(top, task);
    }

    /// Reads a checkout's review, and its marked files, off the UI thread.
    pub(super) fn load_changes(&self, dir: PathBuf, task: tasks::TaskRow) {
        let marked: HashSet<String> = self
            .screen
            .saved
            .reviewed
            .get(&design::path_key(&dir))
            .map(|marks| marks.keys().cloned().collect())
            .unwrap_or_default();
        self.spawn_bg(move || {
            let review = tasks::load_review(task);
            let now = review
                .as_ref()
                .map(|r| views::marked_fingerprints(&dir, &r.files, &marked))
                .unwrap_or_default();
            Bg::Changes(dir, review.map(Box::new), now)
        });
    }

    /// The checkout `dir` is in, as a task (what Changes, merge and throw-away work on), the
    /// pane working there (an agent first), and whether it's a linked worktree. None outside git.
    pub(super) fn task_for(
        &self,
        dir: &std::path::Path,
    ) -> Option<(tasks::TaskRow, Option<TermInfo>, bool)> {
        let head = crate::gitfs::head(dir)?;
        let top = head.top.clone();
        let base = if head.linked {
            crate::gitfs::main_branch(&head.main_root).unwrap_or_else(|| "main".into())
        } else {
            head.branch.clone()
        };
        let mut here: Vec<&TermInfo> = self
            .snap
            .terms
            .values()
            .filter(|t| {
                crate::gitfs::head(&t.cwd)
                    .is_some_and(|h| design::path_key(&h.top) == design::path_key(&top))
            })
            .collect();
        here.sort_by_key(|t| (t.agent.is_none(), t.status.urgency()));
        let info = here.first().map(|t| (*t).clone());
        let term = info.as_ref().map(|i| i.id);
        let ws = term
            .and_then(|t| self.snap.locate(t).map(|(w, _)| w.id))
            .or(self.snap.active_ws);
        let task = tasks::TaskRow {
            ws: ws.unwrap_or(0),
            name: head.branch.clone(),
            branch: head.branch.clone(),
            base,
            stage: tasks::Stage::Ready,
            summary: info.as_ref().map(|i| i.summary.clone()).unwrap_or_default(),
            dirty: 0,
            ahead: 0,
            agent: term.map(ClientTerm::local),
            dir: top,
            root: head.main_root.clone(),
        };
        Some((task, info, head.linked))
    }

    /// Throw a worktree away: its sessions end, its folder and branch go.
    pub(super) fn throw_away(&mut self, dir: &std::path::Path) {
        if let Some(ws) = self
            .snap
            .workspaces
            .iter()
            .find(|w| design::path_key(&w.cwd) == design::path_key(dir))
            .map(|w| w.id)
        {
            self.cmd(Command::RemoveWorktree {
                ws,
                force: true,
                delete_branch: true,
            });
        } else {
            let d = dir.to_path_buf();
            self.spawn_bg(move || Bg::Done(remove_worktree_dir(&d), false));
        }
        self.notify("discarding…".into(), false);
    }

    /// A path an agent printed (Ctrl+click): relative to where it runs. Text opens in the file
    /// viewer at that file (and line); anything else in its own app.
    pub(super) fn open_path_from(&mut self, term: ClientTerm, raw: &str, line: Option<u32>) {
        if !self.guard_local(term.machine, routing::Feature::Files) {
            return;
        }
        let info = self.term_info(term);
        let bases: Vec<PathBuf> = [
            info.map(|t| t.cwd.clone()),
            info.and_then(|t| t.top.clone()),
            info.and_then(|t| t.root.clone()),
        ]
        .into_iter()
        .flatten()
        .collect();
        let raw = raw
            .trim_start_matches("./")
            .replace('/', std::path::MAIN_SEPARATOR_STR);
        let p = PathBuf::from(&raw);
        let found = if p.is_absolute() {
            Some(p).filter(|p| p.exists())
        } else {
            bases.iter().map(|b| b.join(&raw)).find(|p| p.exists())
        };
        let Some(path) = found else {
            self.notify(format!("no file {raw} where this pane runs"), true);
            return;
        };
        if path.is_dir() {
            self.open_files(path);
            return;
        }
        let text = files::looks_like_text(&path);
        if text {
            let dir = path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
            self.open_files(dir);
            if let Some(View::Files(v)) = &mut self.view {
                v.reveal = Some((path, line));
            }
        } else {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.open_outside(path, name);
        }
    }

    /// Writes the editor's file off the UI thread, then says what happened. The editor may
    /// have been closed by then; the file is saved all the same.
    pub(super) fn save_edit(&mut self, job: views::SaveJob) {
        self.spawn_bg(move || {
            let path = job.path().to_path_buf();
            let outcome = job.run();
            Bg::Then(Box::new(move |app: &mut App| app.edit_saved(&path, outcome)))
        });
    }

    fn edit_saved(&mut self, path: &std::path::Path, outcome: views::SaveOutcome) {
        if let Some(View::Files(v)) = &mut self.view
            && let Some(ed) = v.edit.as_mut().filter(|ed| ed.path == path)
        {
            ed.finish_save(&outcome);
        }
        match outcome {
            Ok((views::Saved::Done, _)) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.notify(format!("saved {name}"), false);
            }
            Ok((views::Saved::ChangedOnDisk, _)) => self.notify(
                "the file changed since you opened it (an agent?): Ctrl+S again overwrites it, Esc keeps theirs".into(),
                true,
            ),
            Err(e) => self.notify(e, true),
        }
    }

    /// Open a file, folder or web link in its default app, off the UI thread: say so, and say
    /// why when it didn't.
    pub(super) fn open_outside(&mut self, target: PathBuf, label: String) {
        self.notify(format!("opening {label}"), false);
        self.spawn_bg(move || {
            let result = files::open_default(&target);
            Bg::Then(Box::new(move |app: &mut App| {
                if let Err(e) = result {
                    app.notify(format!("couldn't open {label}: {e}"), true);
                }
            }))
        });
    }

    /// A web link (http or https) in your browser; anything else is refused.
    pub(super) fn open_link(&mut self, url: String) {
        if files::is_web_link(&url) {
            self.open_outside(PathBuf::from(&url), url);
        } else {
            self.notify(format!("only web links open from a pane: {url}"), true);
        }
    }

    pub(super) fn open_files(&mut self, dir: PathBuf) {
        let head = crate::gitfs::head(&dir);
        let root = head.as_ref().map(|h| h.top.clone()).unwrap_or(dir);
        let branch = head.map(|h| h.branch);
        self.show_view(View::Files(Box::new(views::FilesTree::new(
            root.clone(),
            branch,
        ))));
        self.spawn_bg(move || {
            Bg::Tree(
                root.clone(),
                views::scan_tree(&root),
                views::git_marks(&root),
            )
        });
    }

    /// Open a file in your editor: terminal editors inside the app beside what you're on,
    /// others (VS Code, …) as their own window.
    pub(super) fn open_in_editor(&mut self, path: &std::path::Path) {
        self.open_in_editor_at(path, None);
    }

    /// Open a file in your editor, at a line if given.
    pub(super) fn open_in_editor_at(&mut self, path: &std::path::Path, at: Option<u32>) {
        let ed = [
            self.cfg.editor.clone(),
            std::env::var("VISUAL").unwrap_or_default(),
            std::env::var("EDITOR").unwrap_or_default(),
        ]
        .into_iter()
        .find(|e| !e.trim().is_empty())
        .unwrap_or_else(|| "code".into());
        let exe = ed.split_whitespace().next().unwrap_or("code");
        let name = std::path::Path::new(exe)
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let q = files::quote_path(path);
        let line = match (at, name.as_str()) {
            (None, _) => format!("{ed} {q}"),
            (Some(l), "code" | "code-insiders" | "cursor" | "windsurf") => {
                format!(
                    "{ed} -g {}",
                    files::quote_path(std::path::Path::new(&format!("{}:{l}", path.display())))
                )
            }
            (Some(l), "hx" | "helix" | "zed") => format!("{ed} {}:{l}", q),
            (Some(l), _) => format!("{ed} +{l} {q}"),
        };
        let dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.here_dir());
        if [
            "nvim", "vim", "vi", "hx", "helix", "nano", "micro", "kak", "emacs", "ne",
        ]
        .contains(&name.as_str())
        {
            self.screen_new_session(dir, Some(line), true);
            self.view = None;
            return;
        }
        let mut cmd = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(["/C", &line]);
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", &line]);
            c
        };
        cmd.current_dir(&dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        crate::proc::quiet(&mut cmd);
        match cmd.spawn() {
            Ok(_) => self.notify(
                format!(
                    "opened {} in {exe}",
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                ),
                false,
            ),
            Err(e) => self.notify(format!("couldn't start {exe}: {e}"), true),
        }
    }

    pub(super) fn save_setting(&mut self, path: &str, value: toml_edit::Value) {
        match modal::write(path, value) {
            Ok(()) => self.reload_config(),
            Err(e) => self.notify(format!("{e:#}"), true),
        }
    }
}
