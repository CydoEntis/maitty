//! What keys, clicks and actions do in this layout.

use super::*;
use crate::client::ids::ClientSession;
use crate::client::ids::ClientTerm;

// ---- behaviour -----------------------------------------------------------------------------

pub(in crate::client) use crate::gitfs::WT_NAMES;

impl App {
    /// Where a session that just got the focus shows: beside the one it was opened from, in
    /// a new tab, in the tab that already shows it, or in place of the one you were on.
    /// A session is about to open beside `f`: pair them when it appears, and keep it off the sidebar until then.
    pub(in crate::client) fn begin_split(&mut self, f: ClientTerm) {
        self.screen.pending_split = Some((f, Instant::now()));
        self.screen.split_known = self
            .machines()
            .iter()
            .flat_map(|m| m.snap.terms.keys().map(move |id| ClientTerm::on(m.id, *id)))
            .collect();
        *self.screen.model_cache.borrow_mut() = None;
    }

    pub(in crate::client) fn screen_place(&mut self, f: ClientTerm, prev: Option<ClientTerm>) {
        use crate::layout::{Dir, Node};
        // Whatever was held off the sidebar for a split is placed now, or never will be.
        *self.screen.model_cache.borrow_mut() = None;
        let beside = match self.screen.pending_split {
            Some((p, _)) if p == f => None,
            Some((p, at)) => {
                self.screen.pending_split = None;
                if prev == Some(p) && at.elapsed().as_secs() < 20 && self.term_info(p).is_some() {
                    Some(p)
                } else {
                    self.screen.split_dir = None;
                    None
                }
            }
            None => {
                self.screen.split_dir = None;
                None
            }
        };
        let take_out = |tabs: &mut Vec<ScreenTab>, f: ClientTerm| {
            for tab in tabs.iter_mut() {
                if tab.contains(f)
                    && tab.layout.leaves().len() > 1
                    && let Some(l) = tab.layout.clone().remove(f.term)
                {
                    tab.layout = l;
                    tab.focus = ClientTerm::on(f.machine, tab.layout.first_leaf());
                }
            }
            tabs.retain(|t| !(t.owner.machine == f.machine && t.layout == Node::Leaf(f.term)));
        };
        if let Some(p) = beside {
            take_out(&mut self.screen.tabs, f);
            let i = match self.screen.tabs.iter().position(|t| t.contains(p)) {
                Some(i) => i,
                None => {
                    self.screen.tabs.push(ScreenTab::single(p, p));
                    self.screen.tabs.len() - 1
                }
            };
            // Beside a wide one, under a tall one.
            let wide = self
                .screen
                .leaf_rects
                .iter()
                .find(|(id, _)| *id == p)
                .is_none_or(|(_, r)| r.width >= r.height * 3);
            let dir =
                self.screen
                    .split_dir
                    .take()
                    .unwrap_or(if wide { Dir::Right } else { Dir::Down });
            let tab = &mut self.screen.tabs[i];
            tab.layout.split(p.term, dir, f.term);
            tab.focus = f;
            let owner = tab.owner;
            self.screen.tab = i;
            if owner != f {
                self.cmd_on(
                    f.machine,
                    Command::SetTabOf {
                        term: f.term,
                        owner: Some(owner.term),
                    },
                );
            }
            return;
        }
        // You picked this session: its own tab, not the shell tab you were last on in it.
        if self.screen.show_own.take() == Some(f)
            && let Some(i) = self.screen.tabs.iter().position(|t| t.contains(f))
        {
            self.screen.tab = i;
            self.screen.tabs[i].focus = f;
            return;
        }
        // A session with tabs: the one you were last on (unless you're already in one of them).
        let here = self
            .screen
            .tabs
            .get(self.screen.tab)
            .is_some_and(|t| t.owner == f && t.contains(f));
        if !here
            && let Some(i) = (0..self.screen.tabs.len())
                .filter(|i| self.screen.tabs[*i].owner == f)
                .max_by_key(|i| self.screen.tabs[*i].used)
        {
            self.screen.tab = i;
            let to = self.screen.tabs[i].focus;
            if to != f {
                self.cmd_on(to.machine, Command::FocusPane { term: to.term });
            }
            return;
        }
        if let Some(i) = self.screen.tabs.iter().position(|t| t.contains(f)) {
            self.screen.tab = i;
            self.screen.tabs[i].focus = f;
            return;
        }
        if self.screen.tabs.is_empty() {
            self.screen.tabs.push(ScreenTab::single(f, f));
            self.screen.tab = 0;
            return;
        }
        // The one you were on was closed: the rest of its split stays as it is, and the focus
        // goes to what's left there (not to whatever the server picked).
        let ended = prev.filter(|o| self.term_info(*o).is_none());
        let tab = &mut self.screen.tabs[self.screen.tab];
        if let Some(gone) = ended.filter(|o| tab.contains(*o))
            && tab.layout.leaves().len() > 1
            && let Some(rest) = tab.layout.clone().remove(gone.term)
        {
            tab.layout = rest;
            tab.focus = ClientTerm::on(gone.machine, tab.layout.first_leaf());
            let to = tab.focus;
            self.cmd_on(to.machine, Command::FocusPane { term: to.term });
            return;
        }
        // Not shown anywhere: it shows on its own, full size. A view that's one session's only
        // tab with nothing beside it is reused (this one first); a split or another session's
        // tabs stay as they are (pick its row to get them back).
        let lone = |tabs: &[ScreenTab], i: usize| {
            tabs[i].layout.leaves().len() == 1
                && tabs.iter().filter(|u| u.owner == tabs[i].owner).count() == 1
        };
        let fresh = ScreenTab::single(f, f);
        let reuse = if lone(&self.screen.tabs, self.screen.tab) {
            Some(self.screen.tab)
        } else {
            (0..self.screen.tabs.len()).find(|i| lone(&self.screen.tabs, *i))
        };
        match reuse {
            Some(i) => {
                self.screen.tabs[i] = fresh;
                self.screen.tab = i;
            }
            None => {
                self.screen.tabs.push(fresh);
                self.screen.tab = self.screen.tabs.len() - 1;
            }
        }
    }

    /// Close a session's tab and what's in it (one of several: its last tab is the session,
    /// closed from the sidebar). With an agent in it, a second click within a few seconds.
    pub(in crate::client) fn close_tab(&mut self, i: usize) {
        let Some(tab) = self.screen.tabs.get(i) else {
            return;
        };
        // Asked first, as closing a pane is.
        self.menu_act(crate::client::menu::Act::End(tab.leaves()));
    }

    /// Trade the places of two panes shown in one tab; each takes the other's size. The
    /// server's tab follows when it holds both, so a restart keeps the new order. False when
    /// no tab here shows both.
    pub(in crate::client) fn swap_panes(&mut self, a: ClientTerm, b: ClientTerm) -> bool {
        let Some(tab) = self
            .screen
            .tabs
            .iter_mut()
            .find(|t| t.contains(a) && t.contains(b))
        else {
            return false;
        };
        if !tab.layout.swap(a.term, b.term) {
            return false;
        }
        if self
            .snap_of(a.machine)
            .into_iter()
            .flat_map(|snap| &snap.workspaces)
            .flat_map(|w| &w.tabs)
            .any(|t| t.layout.contains(a.term) && t.layout.contains(b.term))
        {
            self.cmd_on(
                a.machine,
                Command::SwapPanes {
                    a: a.term,
                    b: b.term,
                },
            );
        }
        self.dirty = true;
        true
    }

    /// Stop showing `t` beside the others (it keeps running). False when it's on its own.
    pub(in crate::client) fn screen_unshow(&mut self, t: ClientTerm) -> bool {
        let Some(i) = self.screen.tabs.iter().position(|tab| tab.contains(t)) else {
            return false;
        };
        let tab = &mut self.screen.tabs[i];
        if tab.layout.leaves().len() < 2 {
            return false;
        }
        if let Some(l) = tab.layout.clone().remove(t.term) {
            tab.layout = l;
            tab.focus = ClientTerm::on(t.machine, tab.layout.first_leaf());
            let to = tab.focus;
            if self.focused() == Some(t) {
                self.cmd_on(to.machine, Command::FocusPane { term: to.term });
            }
        }
        true
    }

    /// Show a session: it becomes the focused one.
    pub(in crate::client) fn screen_focus(&mut self, term: ClientTerm) {
        self.screen.show_own = Some(term);
        self.screen.cursor = None;
        self.mode = Mode::Normal;
        self.cmd_on(term.machine, Command::FocusPane { term: term.term });
    }

    /// Start a session in `cwd` running `cmd` (None: a shell), beside the focused one or
    /// on its own.
    pub(in crate::client) fn screen_new_session(
        &mut self,
        cwd: PathBuf,
        cmd: Option<String>,
        beside: bool,
    ) {
        self.screen_new_session_on(self.target_machine(), cwd, cmd, beside);
    }

    /// The same on a named machine, for a folder that belongs to it.
    pub(in crate::client) fn screen_new_session_on(
        &mut self,
        machine: MachineId,
        cwd: PathBuf,
        cmd: Option<String>,
        beside: bool,
    ) {
        self.leave_shortcut_mode();
        self.screen_start_pane_on(machine, cwd, cmd, beside);
    }

    /// Start a pane without touching shortcut mode: for one nobody asked for (the shell that
    /// replaces the last session closed), which must not end a mode the user is in.
    pub(in crate::client) fn screen_start_pane_on(
        &mut self,
        machine: MachineId,
        cwd: PathBuf,
        cmd: Option<String>,
        beside: bool,
    ) {
        self.screen.cursor = None;
        self.mode = Mode::Normal;
        // Beside the focused pane: a split of its tab, so the pair stays in one fleet and the
        // daemon remembers the layout.
        if beside && let Some(f) = self.focused().filter(|f| f.machine == machine) {
            self.begin_split(f);
            let dir = self.screen.split_dir.unwrap_or(crate::layout::Dir::Right);
            self.cmd_on(
                machine,
                Command::Split {
                    term: f.term,
                    dir,
                    cmd,
                    cwd: Some(cwd),
                },
            );
            return;
        }
        // Otherwise a ship in the fleet that works in this folder, if the machine has one;
        // else the folder starts a fleet of its own.
        let fleet = self.snap_of(machine).and_then(|snap| {
            snap.workspaces
                .iter()
                .find(|w| super::super::same_dir(&w.cwd, &cwd))
                .map(|w| w.id)
        });
        match fleet {
            Some(ws) => self.cmd_on(machine, Command::NewTab { ws, name: None, cmd }),
            None => self.cmd_on(
                machine,
                Command::NewWorkspace {
                    cwd: Some(cwd),
                    name: None,
                    cmd,
                },
            ),
        }
    }

    /// A new worktree of a project running `cmd`: on `branch` if given (an existing one),
    /// else on a new branch with a generated name.
    pub(in crate::client) fn screen_new_worktree(
        &mut self,
        proj: &Proj,
        cmd: Option<String>,
        beside: bool,
        branch: Option<String>,
    ) {
        let taken: HashSet<String> = proj
            .wts
            .iter()
            .flat_map(|w| [w.name.clone(), w.branch.clone()])
            .chain(proj.branches.iter().cloned())
            .collect();
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as usize)
            .unwrap_or(0);
        // A name made up here starts a new branch from this checkout. Giving the daemon a base stops it
        // checking out an old `origin/<name>` branch that happens to share the name.
        let base = branch.is_none().then(|| "HEAD".to_string());
        let branch = branch.unwrap_or_else(|| {
            (0..WT_NAMES.len())
                .map(|i| WT_NAMES[(n + i) % WT_NAMES.len()].to_string())
                .find(|b| !taken.contains(b))
                .unwrap_or_else(|| format!("{}-{}", WT_NAMES[n % WT_NAMES.len()], n % 1000))
        });
        if !self.guard_local(proj.machine, super::super::routing::Feature::Worktrees) {
            return;
        }
        let Some(ws) = self
            .active_ws()
            .map(|w| w.id)
            .or_else(|| self.snap.workspaces.first().map(|w| w.id))
        else {
            self.notify("start something first".into(), true);
            return;
        };
        if beside && let Some(f) = self.focused() {
            self.begin_split(f);
        }
        self.leave_shortcut_mode();
        self.notify(format!("new worktree {branch} in {}", proj.name), false);
        self.cmd(Command::NewWorktree {
            ws,
            branch,
            base,
            cmd,
            split: None,
            from: Some(proj.path.clone()),
            agent: None,
            prompt: None,
        });
    }

    /// A new worktree running `cmd` (on `branch` if given), full screen.
    pub(in crate::client) fn screen_start_worktree(
        &mut self,
        proj: &Proj,
        cmd: Option<String>,
        branch: Option<String>,
    ) {
        self.screen_new_worktree(proj, cmd, false, branch);
    }

    /// Open a folder as a project: switch to it, or start a session there if nothing runs.
    pub(in crate::client) fn screen_open_project(&mut self, path: PathBuf) {
        // Into a session either way, new or already running: you go there to type.
        self.leave_shortcut_mode();
        self.splash = false;
        let root = crate::gitfs::head(&path)
            .map(|h| h.main_root)
            .unwrap_or_else(|| path.clone());
        let key = path_key(&root);
        let model = self.screen_model();
        let existing = model.iter().find(|p| p.key == key);
        let found = existing.and_then(|p| {
            p.sessions()
                .min_by_key(|s| (rank(s.status), s.key))
                .map(|s| (p.name.clone(), s.key))
        });
        let is_new = existing.is_none();
        self.screen.proj = Some(key.clone());
        match found {
            Some((name, term)) => {
                if let Some(id) = self.session_term(term) {
                    self.screen_focus(id);
                } else {
                    self.screen_side_set(SideItem::Sess(term));
                }
                self.notify(format!("Switched to {name}"), false);
            }
            None => {
                if is_new {
                    self.screen.fresh.insert(key);
                }
                self.screen_new_session_on(MachineId::LOCAL, root.clone(), None, false);
                self.notify(
                    format!(
                        "Opened {} with a shell; + New starts an agent",
                        folder_name(&root)
                    ),
                    false,
                );
            }
        }
    }

    pub(in crate::client) fn screen_open_finder(&mut self) {
        let model = self.screen_model();
        let start = model
            .iter()
            .find(|p| self.screen.proj.as_deref() == Some(p.key.as_str()))
            .and_then(|p| p.path.parent().map(Path::to_path_buf))
            .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()))
            .unwrap_or_default();
        self.mode = Mode::Finder(Box::new(Finder::new(&start)));
    }

    /// The + New chooser, for the current project (or the sidebar cursor's).
    pub(in crate::client) fn screen_open_new_pane(&mut self, beside: bool) {
        let model = self.screen_model();
        let key = self
            .screen
            .cursor
            .and_then(|c| {
                model
                    .iter()
                    .find(|p| p.sessions().any(|s| s.key == c))
                    .map(|p| p.key.clone())
            })
            .or(self.screen.proj.clone());
        let p = model
            .iter()
            .position(|p| Some(&p.key) == key.as_ref())
            .unwrap_or(0);
        self.screen_new(p, beside);
    }

    pub(in crate::client) fn screen_settings(&mut self) {
        self.refresh_installed_version();
        self.screen.check_page = None;
        self.mode = Mode::ScreenSettings(Box::new(SView {
            cat: 0,
            sel: 0,
            editing: None,
            capturing: false,
            scroll: 0,
        }));
    }

    /// Move the sidebar cursor; bare keys then work on the sidebar until Esc or Enter.
    pub(in crate::client) fn screen_side_move(&mut self, d: i8) {
        let items = self.screen.side_items.clone();
        if items.is_empty() {
            return;
        }
        let cur = self
            .screen
            .cursor
            .or(self.focused().map(|id| self.session_key(id)))
            .and_then(|t| items.iter().position(|i| *i == SideItem::Sess(t)));
        let next = match (cur, d) {
            (None, _) => 0,
            (Some(i), 0) => i,
            (Some(i), d) if d < 0 => i.saturating_sub(1),
            (Some(i), _) => (i + 1).min(items.len() - 1),
        };
        self.screen_side_set(items[next].clone());
    }

    /// The sidebar row to land on after the cursor's row goes: the next one, else the one
    /// before.
    pub(in crate::client) fn side_after_close(&self) -> Option<SideItem> {
        let items = &self.screen.side_items;
        let t = self.screen.cursor?;
        let cur = items.iter().position(|i| *i == SideItem::Sess(t))?;
        items
            .get(cur + 1)
            .or_else(|| cur.checked_sub(1).and_then(|i| items.get(i)))
            .cloned()
    }


    /// A confirm answered (`act` when yes): do it, and go back to the sidebar if it was
    /// asked from there.
    pub(in crate::client) fn confirm_done(&mut self, act: Option<crate::client::menu::Act>) {
        let back = self.screen.side_return.take();
        let inbox = self.screen.inbox_return.take();
        let yes = act.is_some();
        if !yes && let Some((query, sel)) = inbox {
            self.mode = Mode::GoTo { query, sel };
            return;
        }
        if let Some(a) = act {
            self.menu_do(a);
        }
        if let Some(v) = self.screen.settings_return.take() {
            self.mode = Mode::ScreenSettings(v);
        }
        if let Some(item) = back {
            if yes {
                self.screen_side_set(item);
            } else {
                self.screen_side_move(0);
            }
        }
    }

    /// Put the sidebar cursor on a row (and the keys in the sidebar).
    pub(in crate::client) fn screen_side_set(&mut self, item: SideItem) {
        self.screen.follow = true;
        self.mode = Mode::Side;
        // The sidebar has the keys now, not an open side panel: set it aside, as a click
        // into a pane does, so the leader's x closes the cursor's row and not the panel.
        self.view_set_aside = self.view_kind();
        match item {
            SideItem::Sess(t) => {
                self.screen.cursor = Some(t);
                // Resting on a finished agent counts as seeing it.
                if let Some(term) = self.session_term(t)
                    && self
                        .term_info(term)
                        .is_some_and(|i| i.status == Status::Done)
                {
                    self.cmd_on(term.machine, Command::MarkSeen { term: term.term });
                }
            }
        }
    }

    pub(in crate::client) fn screen_side_leave(&mut self) {
        self.screen.cursor = None;
        self.mode = Mode::Normal;
    }

    /// The focused pane fills its view, or goes back to its split.
    fn toggle_zoom(&mut self) {
        let tab = self.screen.tab;
        self.screen.zoom = if self.screen.zoom == Some(tab) {
            None
        } else {
            Some(tab)
        };
    }

    /// Put the keys in the sidebar's filter: what is typed narrows the list.
    pub(in crate::client) fn start_side_filter(&mut self) {
        self.screen.filtering = true;
        self.screen.follow = true;
        self.mode = Mode::Side;
        if self.screen.cursor.is_none() {
            let first = self.screen.visible.first().copied();
            self.screen.cursor = first.or(self.focused().map(|id| self.session_key(id)));
        }
    }

    /// Keys while the filter has them: letters narrow the list, Backspace edits, Enter opens
    /// the session on the cursor, Esc clears it all. Returns whether the key was the filter's.
    fn on_filter_key(&mut self, k: &KeyEvent) -> bool {
        if !self.screen.filtering {
            return false;
        }
        let plain = !k
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match k.code {
            KeyCode::Esc => {
                self.screen.filter.clear();
                self.screen.filtering = false;
            }
            // The session on the cursor opens next; the filter goes so the list is whole again.
            KeyCode::Enter => {
                self.screen.filtering = false;
                self.screen.filter.clear();
                return false;
            }
            KeyCode::Backspace => {
                self.screen.filter.pop();
            }
            KeyCode::Char(c) if plain => self.screen.filter.push(c),
            _ => return false,
        }
        self.screen.side_scroll = 0;
        self.screen.follow = true;
        // The cursor stays on a session the filter still shows.
        let model = self.screen_model();
        let shown = super::side::side_keys(self, &model);
        if !self.screen.cursor.is_some_and(|c| shown.contains(&c)) {
            self.screen.cursor = shown.first().copied();
        }
        true
    }

    /// Actions that work differently in this layout. Returns true if handled.
    pub(in crate::client) fn screen_act(&mut self, a: &Action) -> bool {
        let focused = self.selected_term();
        match a {
            Action::Settings => self.screen_settings(),
            // New session and New shell are the same thing: a shell at once where you are, as Alt+N does. There is no dialog.
            Action::PickSession | Action::ShellHere => self.new_ship(None),
            // One place: what needs you on top, and type to go anywhere.
            Action::Message => {
                let side = self.mode == Mode::Side || self.screen.cursor.is_some();
                match self.selected_term() {
                    Some(term) => self.start_compose(term, side, None),
                    None => self.notify("no session to ask about".into(), true),
                }
            }
            // It opens the Inbox, and closes it again.
            Action::Jump if matches!(self.mode, Mode::GoTo { .. }) => self.mode = Mode::Normal,
            Action::Jump => self.open_goto(),
            Action::PrevPrompt | Action::NextPrompt => {
                if let Some(t) = self.focused() {
                    self.jump_prompt(t, *a == Action::PrevPrompt);
                }
            }
            Action::Focus(d) => match focused
                .and_then(|f| crate::layout::neighbor(&self.screen.leaf_rects, f, *d))
            {
                Some(to) => self.cmd_on(to.machine, Command::FocusPane { term: to.term }),
                // Past the left edge: the sidebar.
                None if *d == crate::layout::Dir::Left && self.sidebar => {
                    self.screen_side_move(0)
                }
                None => {}
            },
            Action::Swap(d) => {
                if let Some(f) = focused
                    && let Some(to) = crate::layout::neighbor(&self.screen.leaf_rects, f, *d)
                {
                    self.swap_panes(f, to);
                }
            }
            Action::FocusNext | Action::FocusPrev => {
                let shown: Vec<ClientTerm> =
                    self.screen.leaf_rects.iter().map(|(id, _)| *id).collect();
                if let Some(f) = focused
                    && shown.len() > 1
                {
                    let i = shown.iter().position(|x| *x == f).unwrap_or(0);
                    let n = shown.len();
                    let to = if *a == Action::FocusNext {
                        shown[(i + 1) % n]
                    } else {
                        shown[(i + n - 1) % n]
                    };
                    self.cmd_on(to.machine, Command::FocusPane { term: to.term });
                }
            }
            // A new tab is a new shell in the fleet you are in (a binding of your own may name it).
            Action::NewTab => self.new_ship(None),
            Action::RenameTab => self.rename_tab_start(),
            // From the sidebar, on a ship: that row's menu, as a right-click on it opens.
            Action::Menu if self.screen.cursor.is_some_and(|c| self.session_term(c).is_some()) => {
                if let Some(term) = self.screen.cursor.and_then(|c| self.session_term(c)) {
                    let y = self.screen.row_y.get(&term).copied().unwrap_or(self.screen.side_rect.y);
                    self.screen_side_leave();
                    self.screen.cursor = None;
                    self.menu_for_session(term, (self.screen.side_rect.x + 8, y + 1));
                }
            }
            Action::Menu => self.mode = Mode::Actions { sel: 0 },
            Action::Help => self.open_key_view(KeyView::All),
            Action::Palette => self.open_key_view(KeyView::Search),
            Action::Worktrees => self.open_keymap(Some(Step::Worktrees)),
            // The tabs of the fleet you are in, one per ship, in creation order.
            Action::ZoomPane => self.toggle_zoom(),
            Action::SwapNext => {
                let shown: Vec<ClientTerm> =
                    self.screen.leaf_rects.iter().map(|(id, _)| *id).collect();
                if let Some(f) = focused
                    && let Some(i) = shown.iter().position(|x| *x == f)
                    && shown.len() > 1
                {
                    self.swap_panes(f, shown[(i + 1) % shown.len()]);
                }
            }
            Action::FilterSessions => {
                self.sidebar = true;
                self.start_side_filter();
            }
            Action::Usage => self.mode = Mode::Usage,
            // There is no tab row: a tab key steps through the sidebar's sessions.
            Action::NextTab | Action::PrevTab => self.step_session(*a == Action::NextTab),
            Action::SelectTab(n) => self.jump_to_session(n.saturating_sub(1)),
            Action::CloseTab => self.close_tab(self.screen.tab),
            // b: hide the sidebar, or bring it back.
            Action::ToggleSidebar | Action::HideSidebar => {
                self.sidebar = !self.sidebar;
                self.screen.follow = true;
                if !self.sidebar && self.mode == Mode::Side {
                    self.screen_side_leave();
                }
            }
            Action::RenameWorkspace => {
                if let Some(ClientSession {
                    key: SessionKey::Conversation(id),
                    ..
                }) = self.screen.cursor
                {
                    self.menu_do(crate::client::menu::Act::RenameConversation(id));
                } else if let Some(t) = self.selected_term() {
                    self.menu_do(crate::client::menu::Act::RenamePane(t));
                }
            }
            Action::SplitRight
            | Action::SplitDown
            | Action::SplitLeft
            | Action::SplitUp
            | Action::Spawn(..) => {
                // A view holds up to four panes.
                let panes = self
                    .screen
                    .tabs
                    .get(self.screen.tab)
                    .map_or(1, |tab| tab.layout.leaves().len());
                if !room_for_pane(panes) {
                    self.notify(format!("{MAX_PANES} panes is the most one view holds"), true);
                    return true;
                }
                // Up and left put the new pane before this one, down and right after it.
                self.screen.split_dir = Some(match a {
                    Action::Spawn(d, _) => *d,
                    other => split_dir(other).unwrap_or(crate::layout::Dir::Right),
                });
                let cwd = focused
                    .and_then(|f| self.term_info(f))
                    .map(|t| t.cwd.clone())
                    .unwrap_or_else(|| self.here_dir());
                let cmd = match a {
                    Action::Spawn(_, c) => Some(c.clone()),
                    _ => None,
                };
                self.screen_new_session(cwd, cmd, true);
            }
            Action::NewSession => self.screen_open_new_pane(true),
            Action::CloseSplit => {
                if let Some(f) = focused
                    && !self.screen_unshow(f)
                {
                    self.notify("nothing split here".into(), false);
                }
            }
            Action::SideMove(d) => self.screen_side_move(*d),
            Action::Resize(d) => {
                use crate::layout::Dir;
                let grow = matches!(d, Dir::Right | Dir::Down);
                let many = self
                    .screen
                    .tabs
                    .get(self.screen.tab)
                    .is_some_and(|tab| tab.layout.leaves().len() > 1);
                if let (true, Some(f)) = (many, focused) {
                    let tab = self.screen.tab;
                    self.screen.tabs[tab].layout.resize(f.term, *d, 0.05);
                } else {
                    self.resize_sidebar(grow);
                }
                self.screen.save();
            }
            Action::SidebarWider | Action::SidebarNarrower => {
                self.resize_sidebar(*a == Action::SidebarWider);
                self.screen.save();
            }
            Action::NextSession | Action::PrevSession => self.step_session(*a == Action::NextSession),
            Action::UndoClose => self.undo_close(),
            Action::Updates => self.open_updates(),
            Action::Files | Action::Changes => {
                let dir = self.screen_target_dir();
                self.screen.cursor = None;
                self.mode = Mode::Normal;
                // Open with the keys: f closes it; set aside by a click into a pane: f hands it
                // the keys back.
                if *a == Action::Files
                    && matches!(self.view, Some(crate::client::View::Files(_)))
                    && !self.view_has_keys()
                {
                    self.view_set_aside = None;
                } else if *a == Action::Files
                    && matches!(self.view, Some(crate::client::View::Files(_)))
                {
                    self.view = None;
                } else if *a == Action::Files {
                    self.open_files(dir);
                } else {
                    self.open_changes(dir);
                }
            }
            Action::BrowseTree => self.screen_side_move(0),
            _ => return false,
        }
        true
    }

    /// Two columns wider or narrower, within the sidebar's limits.
    fn resize_sidebar(&mut self, grow: bool) {
        let w = self
            .screen
            .saved
            .side_w
            .unwrap_or(self.screen.side_rect.width.max(SIDE_MIN));
        let w = if grow { w + 2 } else { w.saturating_sub(2) };
        self.screen.saved.side_w = Some(w.clamp(SIDE_MIN, SIDE_MAX));
    }

    /// The next or previous session in the sidebar's order, wrapping.
    fn step_session(&mut self, next: bool) {
        let model = self.screen_model();
        let terms: Vec<ClientTerm> = super::side::side_keys(self, &model)
            .into_iter()
            .filter_map(|k| self.session_term(k))
            .collect();
        let at = self.focused().and_then(|f| terms.iter().position(|t| *t == f));
        let n = terms.len();
        let to = match at {
            Some(i) if n > 1 => terms[if next { (i + 1) % n } else { (i + n - 1) % n }],
            Some(_) => return,
            None => match terms.first() {
                Some(t) => *t,
                None => return,
            },
        };
        self.focus_keeping_keys(to);
    }

    /// Keys while the sidebar has them: ↑↓ move, → or Enter open (a subagent's row its chat),
    /// `/` filters, Esc goes back to the pane. Anything you type goes to the pane you're on.
    pub(in crate::client) fn on_side_key(&mut self, k: &KeyEvent) {
        let spec = KeySpec::from_event(k);
        if self.on_filter_key(k) {
            return;
        }
        if self.keymap.is_prefix(&spec) {
            self.mode = Mode::Prefix {
                since: Instant::now(),
                acted: false,
            };
            return;
        }
        // A key that needs no leader acts here as it does over a pane, on the sidebar's row
        // where it has one (Alt+x closes the selected session), instead of being typed.
        if let Some(a) = self.keymap.global.get(&spec).cloned() {
            self.act(a);
            return;
        }
        let plain = !k
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if k.code == KeyCode::Enter
            && let Some(c) = self.screen.cursor.and_then(|key| self.session_term(key))
            && self.asks_you(c)
        {
            self.open_ask(c, true);
            return;
        }
        if k.code == KeyCode::Enter
            && let Some(c) = self.screen.cursor.and_then(|key| self.session_term(key))
            && self.approval_waits(c)
        {
            self.screen_side_leave();
            self.open_approval(c);
            return;
        }
        match spec.code {
            KeyCode::Char('/') if plain => self.start_side_filter(),
            KeyCode::Up | KeyCode::Char('k') if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                self.screen_side_move(-1)
            }
            KeyCode::Down | KeyCode::Char('j') if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                self.screen_side_move(1)
            }
            KeyCode::Home => {
                if let Some(first) = self.screen.side_items.first().cloned() {
                    self.screen_side_set(first);
                }
            }
            KeyCode::End => {
                if let Some(last) = self.screen.side_items.last().cloned() {
                    self.screen_side_set(last);
                }
            }
            KeyCode::Char(c @ '1'..='9') if plain => self.jump_to_session(c as usize - '1' as usize),
            KeyCode::Right => {
                // Into the pane.
                if let Some(c) = self.screen.cursor
                    && let Some(term) = self.open_session_term(c)
                {
                    self.screen_focus(term);
                }
                self.screen_side_leave();
            }
            KeyCode::Enter => match self.screen.cursor {
                Some(c)
                    if self
                        .session_term(c)
                        .is_some_and(|id| self.check_wants_a_look(id)) =>
                {
                    self.screen_side_leave();
                    if let Some(term) = self.session_term(c) {
                        self.open_check_sheet(term);
                    }
                }
                c => {
                    if let Some(c) = c {
                        if let Some(term) = self.open_session_term(c) {
                            // An agent the restart stopped: Enter brings its conversation back.
                            self.resume_stopped_agent(term);
                            self.screen_focus(term);
                            self.screen_side_leave();
                        }
                    } else {
                        self.screen_side_leave();
                    }
                }
            },
            KeyCode::Esc if !self.screen.filter.is_empty() => {
                self.screen.filter.clear();
                self.screen.side_scroll = 0;
            }
            KeyCode::Esc => self.screen_side_leave(),
            _ if self.keymap.is_prefix(&spec) => {
                self.mode = Mode::Prefix {
                    since: Instant::now(),
                    acted: false,
                }
            }
            // Nothing else acts from the sidebar: an action is the leader and its key, or a key
            // with no leader.
            // Anything else is typing: leave the sidebar and hand the key over.
            _ => {
                if let Some(key) = self.screen.cursor
                    && self.require_session_term(key).is_none()
                {
                    return;
                }
                self.screen_side_leave();
                if let Some(term) = self.focused() {
                    let data = crate::keys::encode(
                        k,
                        self.parsers
                            .get(&term)
                            .is_some_and(|p| p.screen().application_cursor()),
                    );
                    if !data.is_empty() {
                        self.send_input(term, data);
                    }
                }
            }
        }
    }

    /// Show a session and stay in key mode when it was on, so `j` `j` `k` walks the sessions.
    fn focus_keeping_keys(&mut self, term: ClientTerm) {
        let keys = matches!(self.mode, Mode::Prefix { .. }).then(|| self.mode.clone());
        self.screen_focus(term);
        if let Some(mode) = keys {
            self.mode = mode;
        }
    }

    /// Open the `n`th session of the sidebar (counting from 0, top to bottom).
    pub(in crate::client) fn jump_to_session(&mut self, n: usize) {
        let model = self.screen_model();
        let Some(key) = super::side::side_keys(self, &model).get(n).copied() else {
            return;
        };
        if let Some(term) = self.open_session_term(key) {
            self.focus_keeping_keys(term);
        }
    }

    /// Go to, opened on its first row you can pick (what needs you, when anything does).
    pub(in crate::client) fn open_goto(&mut self) {
        let rows = goto_rows(&self.screen_model(), "", heads_up(self).len());
        let sel = rows.iter().position(GoRow::pickable).unwrap_or(0);
        self.mode = Mode::GoTo {
            query: String::new(),
            sel,
        };
    }

    /// Answer an agent's numbered question from the Inbox: its option `key`, as if typed there.
    pub(in crate::client) fn inbox_answer(&mut self, term: ClientTerm, key: char, name: &str) {
        let label = self
            .answer_options(term)
            .get((key as u8 - b'1') as usize)
            .cloned()
            .unwrap_or_else(|| key.to_string());
        self.answer(term, key);
        self.notify(format!("answered {name}: {label}"), false);
    }

    /// A question an agent asked you with `<app> ask-human`, waiting in this pane.
    pub(in crate::client) fn pending_question(
        &self,
        term: ClientTerm,
    ) -> Option<&crate::protocol::HumanQuestion> {
        self.snap_of(term.machine)?
            .questions
            .iter()
            .find(|q| q.term == term.term)
    }

    /// The answers a waiting agent offers: its ask-human question's, else the numbered
    /// choices on its screen.
    pub(in crate::client) fn answer_options(&self, term: ClientTerm) -> Vec<String> {
        match self.pending_question(term) {
            Some(q) => q.options.clone(),
            None => options(self.parsers.get(&term)),
        }
    }

    /// Answer with choice `key` ('1' is the first): to its ask-human question, or typed into
    /// the agent as its own numbered prompt expects.
    pub(in crate::client) fn answer(&mut self, term: ClientTerm, key: char) {
        match self.pending_question(term).map(|q| q.id) {
            Some(id) => self.cmd_on(
                term.machine,
                Command::AnswerHuman {
                    id,
                    choice: (key as u8).saturating_sub(b'1') as usize,
                },
            ),
            None => self.send_input(term, key.to_string().into_bytes()),
        }
    }

    /// The folder Files / Changes / PR act on: the sidebar cursor's, else the focused one's.
    pub(in crate::client) fn screen_target_dir(&self) -> PathBuf {
        self.selected_term()
            .and_then(|t| self.term_info(t))
            .map(|t| t.top.clone().unwrap_or_else(|| t.cwd.clone()))
            .or_else(|| match self.screen.cursor {
                Some(ClientSession {
                    machine,
                    key: SessionKey::Conversation(id),
                }) => self
                    .snap_of(machine)?
                    .conversations
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.worktree.clone().unwrap_or_else(|| c.cwd.clone())),
                _ => None,
            })
            .unwrap_or_else(|| self.here_dir())
    }

    /// Picking for a setting: save the folder and go back to Settings, on that row.
    fn finder_chose(&mut self, fd: &Finder, p: PathBuf) -> bool {
        let Some(path) = fd.for_setting else {
            return false;
        };
        self.save_setting(path, p.display().to_string().into());
        self.back_to_setting(path);
        true
    }

    /// Settings, on the row of `path`.
    pub(in crate::client) fn back_to_setting(&mut self, path: &str) {
        use crate::client::modal::Cat;
        let (cat, sel) = Cat::ALL
            .iter()
            .enumerate()
            .find_map(|(ci, c)| {
                crate::client::design::settings_rows(*c)
                    .iter()
                    .position(
                        |r| matches!(r, crate::client::design::SRow::Setting(s) if s.path == path),
                    )
                    .map(|i| (ci, i))
            })
            .unwrap_or((0, 0));
        self.mode = Mode::ScreenSettings(Box::new(SView {
            cat,
            sel,
            editing: None,
            capturing: false,
            scroll: 0,
        }));
    }

    pub(in crate::client) fn finder_enter(&mut self, mut fd: Finder) {
        let sep = std::path::MAIN_SEPARATOR;
        let rows = fd.rows();
        match rows.get(fd.sel).cloned() {
            None => {
                let (dir, part) = fd.split();
                let p = dir.join(part);
                if p.is_dir() {
                    if self.finder_chose(&fd, p.clone()) {
                        return;
                    }
                    self.screen_open_project(p);
                } else {
                    self.notify(format!("no folder {}", tilde(&p)), true);
                    self.mode = Mode::Finder(Box::new(fd));
                }
            }
            Some((n, p, repo)) if n == "." || repo => {
                if !self.finder_chose(&fd, p.clone()) {
                    self.screen_open_project(p);
                }
            }
            Some((_, p, _)) => {
                fd.q = format!("{}{sep}", tilde(&p).trim_end_matches(sep));
                fd.sel = 0;
                fd.refresh();
                self.mode = Mode::Finder(Box::new(fd));
            }
        }
    }

    pub(in crate::client) fn on_finder_key(&mut self, mut fd: Finder, k: &KeyEvent) {
        let sep = std::path::MAIN_SEPARATOR;
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc => {
                match fd.for_setting {
                    Some(path) => self.back_to_setting(path),
                    None => self.mode = Mode::Normal,
                }
                return;
            }
            KeyCode::Enter => return self.finder_enter(fd),
            KeyCode::Tab => {
                if let Some((n, p, _)) = fd.rows().get(fd.sel).cloned()
                    && n != "."
                {
                    fd.q = format!("{}{sep}", tilde(&p).trim_end_matches(sep));
                    fd.sel = 0;
                }
            }
            KeyCode::Down => fd.sel = (fd.sel + 1).min(fd.rows().len().saturating_sub(1)),
            KeyCode::Up => fd.sel = fd.sel.saturating_sub(1),
            KeyCode::Backspace => {
                fd.q.pop();
                fd.sel = 0;
            }
            KeyCode::Char(c) if !ctrl => {
                fd.q.push(if c == '/' || c == '\\' { sep } else { c });
                fd.sel = 0;
            }
            _ => {}
        }
        fd.refresh();
        self.mode = Mode::Finder(Box::new(fd));
    }

    /// Start what the chooser says: an agent gets its own worktree in a git project, a
    /// shell (or an agent outside git) runs in the project's folder.
    pub(in crate::client) fn screen_pane_go(&mut self, np: &NewPaneHy) {
        let model = self.screen_model();
        let Some(p) = model.get(np.p).cloned() else {
            self.screen_open_finder();
            return;
        };
        let agents = np_agents(self, np.p);
        let agent = agents.get(np.a).cloned().unwrap_or_else(|| "shell".into());
        let main = p
            .wts
            .iter()
            .find(|w| w.main)
            .map(|w| w.path.clone())
            .unwrap_or_else(|| p.path.clone());
        let cmd = np_command(self, &agent, np.model, &np.task);
        if !self.screen.saved.draft.is_empty() {
            self.screen.saved.draft.clear();
            self.screen.save();
        }
        if let Some(recipe) = agent.strip_prefix("⚙ ") {
            self.mode = Mode::Normal;
            self.run_recipe(&p, recipe);
            return;
        }
        if !p.git {
            return self.screen_new_session_on(p.machine, main, cmd, np.beside);
        }
        let place = np.place_for(&agent, self.cfg.worktree.per_agent);
        if place == 1 && !self.guard_local(p.machine, super::super::routing::Feature::Worktrees) {
            return;
        }
        match place {
            // Another machine's worktrees wait for a later version: its folder it is.
            0 if p.machine != MachineId::LOCAL => {
                self.screen_new_session_on(p.machine, main, cmd, np.beside)
            }
            0 => self.screen_new_worktree(&p, cmd, np.beside, None),
            1 => {
                // A new branch right in the repo folder (no worktree).
                let taken: HashSet<String> = p.wts.iter().map(|w| w.branch.clone()).collect();
                let n = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as usize)
                    .unwrap_or(0);
                let name = (0..WT_NAMES.len())
                    .map(|i| WT_NAMES[(n + i) % WT_NAMES.len()].to_string())
                    .find(|b| !taken.contains(b))
                    .unwrap_or_else(|| format!("branch-{}", n % 10_000));
                let (pname, beside, machine) = (p.name.clone(), np.beside, p.machine);
                self.spawn_bg(move || {
                    let mut git = std::process::Command::new("git");
                    git.arg("-C").arg(&main).args(["switch", "-c", &name]);
                    crate::proc::quiet(&mut git);
                    let out = git.output();
                    crate::client::Bg::Then(Box::new(move |app: &mut App| match out {
                        Ok(o) if o.status.success() => {
                            app.notify(format!("{pname} is on a new branch, {name}"), false);
                            app.screen_new_session_on(machine, main, cmd, beside);
                        }
                        Ok(o) => app.notify(
                            format!(
                                "couldn't make a branch: {}",
                                String::from_utf8_lossy(&o.stderr)
                                    .lines()
                                    .next()
                                    .unwrap_or("git failed")
                            ),
                            true,
                        ),
                        Err(e) => app.notify(format!("couldn't run git: {e}"), true),
                    }))
                });
            }
            n @ 3.. => match p.wts.iter().filter(|w| !w.main).nth(n as usize - 3) {
                Some(w) => self.screen_new_session_on(p.machine, w.path.clone(), cmd, np.beside),
                None => {
                    self.mode = Mode::ScreenPane(np.clone());
                    self.notify(
                        "that worktree is no longer available; choose where to start again".into(),
                        true,
                    );
                }
            },
            _ => self.screen_new_session_on(p.machine, main, cmd, np.beside),
        }
    }

    pub(in crate::client) fn on_goto_key(
        &mut self,
        mut query: String,
        mut sel: usize,
        k: &KeyEvent,
    ) {
        // With nothing typed, `[` `]` `=` size the sheet; once you search they are text.
        if query.is_empty() && self.sheet_size_key(k) {
            self.mode = Mode::GoTo { query, sel };
            return;
        }
        let model = self.screen_model();
        let rows = goto_rows(&model, &query, heads_up(self).len());
        // Headings aren't rows you land on.
        let land = |sel: usize, down: bool| -> usize {
            let n = rows.len();
            if n == 0 {
                return 0;
            }
            let mut i = sel.min(n - 1);
            while !rows[i].pickable() {
                match (down, i) {
                    (true, i2) if i2 + 1 < n => i = i2 + 1,
                    (false, i2) if i2 > 0 => i = i2 - 1,
                    _ => return rows.iter().position(GoRow::pickable).unwrap_or(0),
                }
            }
            i
        };
        let sel_row = rows.get(land(sel, true)).cloned();
        let sel_item = sel_row
            .as_ref()
            .filter(|r| !matches!(r, GoRow::Sess(..)))
            .and_then(GoRow::term);
        let sel_check = sel_item
            .and_then(|t| find(&model, t))
            .and_then(|(.., s)| super::check::shown(s).cloned());
        // A merge or throw-away asked in a finished row: Enter does it, Esc doesn't, the rest wait.
        if let Some((term, merge, false)) = self.screen.inbox_confirm {
            match k.code {
                KeyCode::Esc => self.screen.inbox_confirm = None,
                KeyCode::Enter => {
                    if let Some((_, wt, _)) = find(&model, term) {
                        let dir = wt.path.clone();
                        if merge {
                            if let Some((task, ..)) = self.task_for(&dir) {
                                self.spawn_bg(move || {
                                    crate::client::Bg::Merged(
                                        crate::client::tasks::merge(&task),
                                        dir,
                                    )
                                });
                            }
                        } else {
                            self.throw_away(&dir);
                        }
                        self.screen.inbox_confirm = Some((term, merge, true));
                    }
                }
                _ => {}
            }
            self.mode = Mode::GoTo { query, sel };
            return;
        }
        match k.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Down => sel = land((sel + 1).min(rows.len().saturating_sub(1)), true),
            KeyCode::Up => sel = land(sel.saturating_sub(1), false),
            KeyCode::PageDown => sel = land((sel + 10).min(rows.len().saturating_sub(1)), true),
            KeyCode::PageUp => sel = land(sel.saturating_sub(10), false),
            // With nothing typed, a number answers the agent you're on.
            // A session asking to start helpers has no numbered answer to type: its question
            // opens (a number there would be typed into the agent).
            KeyCode::Char('1'..='9')
                if query.is_empty()
                    && matches!(sel_row, Some(GoRow::Ask(t)) if self.asks_you(t)) =>
            {
                if let Some(GoRow::Ask(term)) = sel_row {
                    return self.open_ask_from_inbox(term, query, sel);
                }
            }
            KeyCode::Char(c @ '1'..='9')
                if query.is_empty() && matches!(sel_row, Some(GoRow::Ask(_))) =>
            {
                if let Some(GoRow::Ask(term)) = sel_row {
                    let name = self.session_name(term);
                    self.inbox_answer(term, c, &name);
                }
            }
            // Seen: a finished one leaves the list.
            // A heads-up: both diffs, a note to the first agent, or dismiss it.
            KeyCode::Char(c @ ('d' | 'm' | 'k'))
                if query.is_empty() && matches!(sel_row, Some(GoRow::Heads(_))) =>
            {
                let Some(GoRow::Heads(i)) = sel_row else {
                    return;
                };
                let Some((repo, o)) = heads_up(self).get(i).cloned() else {
                    return;
                };
                match c {
                    'd' => return self.open_both(repo, o),
                    'm' => return self.tell_about(&repo, &o, 0, Some((query, sel))),
                    _ => self.dismiss_heads_up(&repo, &o),
                }
                self.mode = Mode::GoTo {
                    query,
                    sel: sel.saturating_sub(1),
                };
                return;
            }
            // A finished one: its diff (in the sheet), or merge it or throw it away (asked first).
            KeyCode::Char(c @ ('d' | 'M' | 'x'))
                if query.is_empty() && matches!(sel_row, Some(GoRow::Done(_))) =>
            {
                let Some(GoRow::Done(term)) = sel_row else {
                    return;
                };
                if !self.guard_local(term.machine, super::super::routing::Feature::Changes) {
                    return;
                }
                let Some((_, wt, _)) = find(&model, term) else {
                    return;
                };
                match c {
                    'd' => {
                        let dir = wt.path.clone();
                        self.mode = Mode::Normal;
                        self.open_changes(dir);
                        return;
                    }
                    _ if !wt.main => self.screen.inbox_confirm = Some((term, c == 'M', false)),
                    _ => self.notify(
                        "the main folder isn't a worktree: open Changes (d) to commit".into(),
                        false,
                    ),
                }
                self.mode = Mode::GoTo { query, sel };
                return;
            }
            // m on a failure: a message to its agent with the failure written in.
            KeyCode::Char('m')
                if query.is_empty()
                    && let Some(GoRow::Failed(term)) = sel_row =>
            {
                let name = self
                    .term_info(term)
                    .and_then(|t| t.check())
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                self.send_failure(term, name, None, Some((query, sel)));
                return;
            }
            // m: a follow-up, inside the row.
            KeyCode::Char('m')
                if query.is_empty() && matches!(sel_row, Some(GoRow::Ask(_) | GoRow::Done(_))) =>
            {
                if let Some(GoRow::Ask(term) | GoRow::Done(term)) = sel_row {
                    self.start_compose(term, false, Some((query, sel)));
                }
                return;
            }
            // y, a and n answer a permission prompt in the selected card's session, as if typed
            // there: yes once, always this session, no. They go to its PTY; the card leaves when
            // the agent stops asking.
            KeyCode::Char(c @ ('y' | 'a' | 'n'))
                if query.is_empty() && matches!(sel_row, Some(GoRow::Ask(_))) =>
            {
                if let Some(GoRow::Ask(term)) = sel_row {
                    self.send_input(term, vec![c as u8]);
                    let word = match c {
                        'y' => "yes once",
                        'a' => "yes for the session",
                        _ => "no",
                    };
                    let name = self.session_name(term);
                    self.notify(format!("{name}: {word}"), false);
                }
                self.mode = Mode::GoTo { query, sel };
                return;
            }
            // c clears what finished and what exited: each card leaves, its session stays.
            KeyCode::Char('c') if query.is_empty() => {
                for row in &rows {
                    match row {
                        GoRow::Done(term) => {
                            self.cmd_on(term.machine, Command::MarkSeen { term: term.term });
                        }
                        GoRow::Exited(term) => self.clear_exit(*term),
                        _ => {}
                    }
                }
                self.screen_fresh();
            }
            // Any item: its session.
            KeyCode::Char('o') if query.is_empty() && sel_item.is_some() => {
                self.mode = Mode::Normal;
                if let Some(term) = sel_item {
                    self.screen_focus(term);
                }
                return;
            }
            // An item's check: run it again, accept a failure (asked first), take that back, or
            // its settings. Where it doesn't act, the key is typed.
            KeyCode::Char(c @ ('r' | 'O' | 'u' | ','))
                if query.is_empty()
                    && sel_check
                        .as_ref()
                        .is_some_and(|i| super::check_inbox::check_key_acts(c, i)) =>
            {
                let Some(term) = sel_item else { return };
                self.do_check_key(term, None, c);
                if c == 'O' && matches!(self.mode, Mode::Confirm(_)) {
                    self.screen.inbox_return = Some((query, sel));
                } else if c != ',' {
                    self.mode = Mode::GoTo { query, sel };
                }
                return;
            }
            KeyCode::Delete if matches!(sel_row, Some(GoRow::Done(_) | GoRow::NotRun(_))) => {
                if let Some(GoRow::Done(term) | GoRow::NotRun(term)) = sel_row {
                    self.cmd_on(term.machine, Command::MarkSeen { term: term.term });
                }
            }
            KeyCode::Delete if let Some(GoRow::Exited(term)) = sel_row => self.clear_exit(term),
            KeyCode::Delete if let Some(GoRow::Quiet(term)) = sel_row => {
                if let Some(at) = self.term_info(term).and_then(|i| i.quiet_since) {
                    self.screen.quiet_seen.insert(term, at);
                    self.screen_fresh();
                }
            }
            KeyCode::Enter
                if let Some(t) = sel_row.as_ref().and_then(GoRow::term)
                    && self.approval_waits(t) =>
            {
                self.open_approval(t);
                self.screen.inbox_return = Some((query, sel));
                return;
            }
            // The project's check settings, for a row about a session.
            KeyCode::Char(',')
                if query.is_empty()
                    && let Some(t) = sel_row.as_ref().and_then(GoRow::term) =>
            {
                return self.open_project_check(t);
            }
            KeyCode::Enter => {
                let stale = sel_check
                    .as_ref()
                    .is_some_and(|i| i.state == crate::protocol::CheckState::Stale);
                match sel_row {
                    Some(GoRow::Ask(t)) if query.is_empty() && self.asks_you(t) => {
                        return self.open_ask_from_inbox(t, query, sel);
                    }
                    Some(GoRow::Ask(t)) if query.is_empty() => {
                        return self.start_compose(t, false, Some((query, sel)));
                    }
                    Some(GoRow::Approve(t) | GoRow::Failed(t) | GoRow::NotRun(t)) => {
                        self.open_check_sheet(t)
                    }
                    Some(GoRow::Done(t)) if stale && query.is_empty() => {
                        self.do_check_key(t, None, 'r');
                        self.mode = Mode::GoTo { query, sel };
                    }
                    Some(
                        GoRow::Ask(t)
                        | GoRow::Done(t)
                        | GoRow::Quiet(t)
                        | GoRow::Exited(t)
                        | GoRow::Sess(_, t),
                    ) => {
                        self.mode = Mode::Normal;
                        self.screen_focus(t);
                    }
                    Some(GoRow::Head(_) | GoRow::Heads(_)) | None => self.mode = Mode::Normal,
                }
                return;
            }
            // The list changes with the search: land on its first row you can pick, worked out
            // on the new list (the rows above are the old search's).
            KeyCode::Backspace => {
                query.pop();
                let rows = goto_rows(&model, &query, heads_up(self).len());
                let sel = rows.iter().position(GoRow::pickable).unwrap_or(0);
                self.mode = Mode::GoTo { query, sel };
                return;
            }
            KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                query.push(c);
                // Land on the first session that matches, when there is one.
                let rows = goto_rows(&model, &query, heads_up(self).len());
                sel = rows.iter().position(GoRow::pickable).unwrap_or(0);
            }
            _ => {}
        }
        if query.is_empty()
            && !matches!(
                k.code,
                KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
            )
        {
            sel = land(sel, true);
        }
        self.mode = Mode::GoTo { query, sel };
    }

    /// Take an exit off the Inbox; it comes back only once another command fails.
    fn clear_exit(&mut self, term: ClientTerm) {
        if let Some(exit) = self.term_info(term).and_then(|i| i.exit) {
            self.screen.exit_seen.insert(term, exit.at);
            self.screen_fresh();
        }
    }

    pub(in crate::client) fn on_history_key(&mut self, sel: usize, k: &KeyEvent) {
        let n = self.history.len();
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Down => {
                self.mode = Mode::History {
                    sel: (sel + 1).min(n.saturating_sub(1)),
                }
            }
            KeyCode::Up => {
                self.mode = Mode::History {
                    sel: sel.saturating_sub(1),
                }
            }
            KeyCode::Char('c') => {
                self.history.clear();
                self.mode = Mode::History { sel: 0 };
            }
            KeyCode::Enter => {
                let term = self.history.iter().rev().nth(sel).and_then(|h| h.1);
                match term.filter(|t| self.term_info(*t).is_some()) {
                    Some(t) => {
                        self.mode = Mode::Normal;
                        self.screen_focus(t);
                    }
                    None => self.mode = Mode::History { sel },
                }
            }
            _ => self.mode = Mode::History { sel },
        }
    }

    /// + New for project `p`, with the task you didn't start last time.
    pub(in crate::client) fn screen_new(&mut self, p: usize, beside: bool) {
        let mut np = NewPaneHy::new(p, beside);
        np.task = self.screen.saved.draft.clone();
        self.mode = Mode::ScreenPane(np);
    }

    pub(in crate::client) fn on_hy_pane_key(&mut self, mut np: NewPaneHy, k: &KeyEvent) {
        let model = self.screen_model();
        let nproj = model.len() + 1;
        let agents = np_agents(self, np.p);
        let nag = agents.len();
        let agent = agents.get(np.a).cloned().unwrap_or_default();
        let nmodels = if agent.starts_with('★') || agent.starts_with('◇') {
            0
        } else {
            np_models(self, &agent).len()
        };
        let nplaces = model.get(np.p).map(|p| np_places(p).len()).unwrap_or(3);
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let d: i32 = match k.code {
            KeyCode::Left => -1,
            KeyCode::Right => 1,
            _ => 0,
        };
        let cyc = |v: usize, d: i32, n: usize| (v as i32 + d).rem_euclid(n.max(1) as i32) as usize;
        // Rows with nothing to choose are skipped.
        let skip = |r: u8| r == 2 && nmodels == 0;
        let step = |np: &mut NewPaneHy, by: u8| {
            np.row = (np.row + by) % NP_ROWS;
            if skip(np.row) {
                np.row = (np.row + by) % NP_ROWS;
            }
        };
        match k.code {
            KeyCode::Esc => {
                // The task stays for next time.
                if self.screen.saved.draft != np.task {
                    self.screen.saved.draft = np.task.clone();
                    self.screen.save();
                }
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Enter => {
                np.beside |= k.modifiers.contains(KeyModifiers::SHIFT);
                return self.screen_pane_go(&np);
            }
            KeyCode::Tab | KeyCode::Down => step(&mut np, 1),
            KeyCode::BackTab | KeyCode::Up => step(&mut np, NP_ROWS - 1),
            // Typing on the task row.
            KeyCode::Backspace if np.row == 0 => {
                if ctrl {
                    let keep = np.task.trim_end().rfind(' ').map(|i| i + 1).unwrap_or(0);
                    np.task.truncate(keep);
                } else {
                    np.task.pop();
                }
            }
            KeyCode::Char('u') if ctrl && np.row == 0 => np.task.clear(),
            KeyCode::Char(c) if np.row == 0 && !ctrl => np.task.push(c),
            KeyCode::Char('b') if !ctrl => np.beside = !np.beside,
            _ if d != 0 => match np.row {
                0 => {}
                1 => {
                    np.a = cyc(np.a, d, nag);
                    np.model = 0;
                }
                2 => np.model = cyc(np.model, d, nmodels),
                3 => {
                    np.p = cyc(np.p, d, nproj);
                    np.place = np.place.filter(|p| *p < 3);
                    if agent.starts_with('◇') {
                        np.a = 0;
                        np.model = 0;
                    }
                }
                4 => {
                    let cur = np.place_for(&agent, self.cfg.worktree.per_agent) as usize;
                    np.place = Some(cyc(cur, d, nplaces) as u8);
                }
                _ => np.beside = !np.beside,
            },
            _ => {}
        }
        self.mode = Mode::ScreenPane(np);
    }

    /// Keys on the splash: arrows (or j, k) and a card's own key choose, Enter opens the chosen
    /// one. Nothing else leaves it.
    pub(in crate::client) fn on_hy_splash_key(&mut self, k: &KeyEvent) {
        let opts = splash_options(self);
        let last = opts.len().saturating_sub(1);
        if self.screen.splash_wide && self.on_wide_splash_key(k, opts.len()) {
            return;
        }
        let sel = self.screen.splash_sel.min(last);
        match k.code {
            KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
                self.screen.splash_sel = sel.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
                self.screen.splash_sel = (sel + 1).min(last)
            }
            KeyCode::Enter => self.screen_splash_open(sel),
            KeyCode::Char('?' | ',') => {
                if let KeyCode::Char(c) = k.code {
                    self.screen_splash_action(c);
                }
            }
            // The release banner's key: install the newer one, or restart into it.
            KeyCode::Char('u') if self.update_available.is_some() || self.restart_needed() => {
                self.splash = false;
                if self.update_available.is_some() {
                    self.act(Action::Update);
                } else {
                    self.ask_restart();
                }
            }
            KeyCode::Char(c) => {
                if let Some(i) = opts.iter().position(|(_, _, key)| *key == c) {
                    self.screen.splash_sel = i;
                }
            }
            _ => {}
        }
    }

    /// The wide splash's own keys: Tab changes box, the arrows walk the box that has the keys,
    /// Enter opens its chosen row. True when the key was one of those.
    fn on_wide_splash_key(&mut self, k: &KeyEvent, menu: usize) -> bool {
        let model = self.screen_model();
        let (projects, away) = (
            splash_wide::recent_projects(&model).len(),
            splash_wide::away_sessions(&model).len(),
        );
        let going = match k.code {
            KeyCode::Up | KeyCode::Char('k') => -1i32,
            KeyCode::Down | KeyCode::Char('j') => 1,
            KeyCode::Tab | KeyCode::BackTab => {
                self.screen.splash_away = !self.screen.splash_away;
                return true;
            }
            KeyCode::Enter => {
                if self.screen.splash_away {
                    self.splash_open_away(self.screen.splash_away_sel);
                } else {
                    self.splash_open_start(self.screen.splash_sel, menu);
                }
                return true;
            }
            KeyCode::Char('r' | 'n' | 'o') => {
                self.screen.splash_away = false;
                return false;
            }
            _ => return false,
        };
        let step = |at: usize, len: usize| {
            (at as i32 + going).clamp(0, len.saturating_sub(1) as i32) as usize
        };
        if self.screen.splash_away {
            self.screen.splash_away_sel = step(self.screen.splash_away_sel, away);
        } else {
            self.screen.splash_sel = step(self.screen.splash_sel, menu + projects);
        }
        true
    }

    /// Open the wide splash's START row: a choice, or a recent project (the splash goes, the
    /// project opens in the sidebar and the keys are there).
    pub(in crate::client) fn splash_open_start(&mut self, i: usize, menu: usize) {
        if i < menu {
            self.screen_splash_open(i);
            return;
        }
        let model = self.screen_model();
        let Some(path) = splash_wide::recent_projects(&model)
            .get(i - menu)
            .map(|p| p.path.clone())
        else {
            return;
        };
        self.splash = false;
        self.screen.splash_sel = 0;
        self.screen_open_project(path);
    }

    /// Open the wide splash's away-list session.
    pub(in crate::client) fn splash_open_away(&mut self, i: usize) {
        let model = self.screen_model();
        let Some(key) = splash_wide::away_sessions(&model).get(i).map(|(s, _)| s.key) else {
            return;
        };
        self.splash = false;
        if let Some(term) = self.session_term(key) {
            self.screen_focus(term);
        }
        self.screen_side_set(SideItem::Sess(key));
    }

    /// Open the splash's `i`th choice.
    pub(in crate::client) fn screen_splash_open(&mut self, i: usize) {
        if let Some((_, _, c)) = splash_options(self).get(i) {
            self.screen_splash_action(*c);
        }
    }

    pub(in crate::client) fn screen_splash_action(&mut self, c: char) {
        self.splash = false;
        self.screen.splash_sel = 0;
        match c {
            'n' => self.new_ship(None),
            'o' => self.screen_open_finder(),
            ',' => self.screen_settings(),
            '?' => self.open_keymap(None),
            // r: resume, just the app as you left it.
            _ => {}
        }
    }

    /// Clicks on this layout's chips, rows and buttons.
    pub(in crate::client) fn on_hy_hit(&mut self, h: ScreenHit, double: bool) {
        match h {
            // Opens it, as a click on one of its tabs does: the keys go to its pane. A session
            // with no pane has nothing to type into, so the sidebar keeps them on its row.
            ScreenHit::Session(t) => match self.session_term(t) {
                Some(term) => self.screen_focus(term),
                None => self.screen_side_set(SideItem::Sess(t)),
            },
            // From the splash's banner too, which then goes.
            ScreenHit::Update => {
                self.splash = false;
                self.act(Action::Update);
            }
            ScreenHit::Restart => {
                self.splash = false;
                self.ask_restart();
            }
            ScreenHit::Subagent(i) => {
                if let Some(term) = self.screen.sub_rows.get(i).copied() {
                    self.screen_focus(term);
                }
            }
            ScreenHit::ReleaseMenu => self.release_menu(),
            ScreenHit::NoPaneNew => self.new_ship(None),
            // A shell on that machine at once.
            ScreenHit::MachinePlus(machine) => self.new_ship_on(machine),
            ScreenHit::NewMenu => self.new_menu(),
            ScreenHit::Fold(i) | ScreenHit::FoldMachine(i, _) => {
                if let Some(key) = self.screen.fold_keys.get(i).cloned() {
                    let closed = &mut self.screen.saved.closed;
                    if let Some(at) = closed.iter().position(|k| *k == key) {
                        closed.remove(at);
                    } else {
                        closed.push(key);
                    }
                    self.screen.save();
                    self.dirty = true;
                }
            }
            ScreenHit::MachineHead(MachineId::LOCAL) => self.act(Action::BrowseTree),
            ScreenHit::MachineHead(machine) => self.open_machines_on(machine),
            ScreenHit::MachineFix(machine) => self.machine_update_from_side(machine),
            ScreenHit::PaneProject(term) => {
                let at = self.hover.map_or((0, 0), |p| (p.x, p.y.saturating_sub(1)));
                self.menu_for_project_tools(term, at);
            }
            ScreenHit::TidyRow(i) => self.tidy_click(i),
            ScreenHit::TidyGo => {
                if let Mode::Tidy(v) = std::mem::replace(&mut self.mode, Mode::Normal) {
                    self.tidy_ask(*v);
                }
            }
            // The sidebar's "new": a shell where you are.
            // The ✕ on a pane: close it (after asking).
            ScreenHit::CloseSplit(t) => self.menu_act(crate::client::menu::Act::End(vec![t])),
            ScreenHit::Divider(i) => self.screen.drag = Some(Drag::Divider(i)),
            ScreenHit::ConfirmYes => {
                if let Mode::Confirm(c) = std::mem::replace(&mut self.mode, Mode::Normal) {
                    self.confirm_done(Some(c.act));
                }
            }
            ScreenHit::ConfirmNo => {
                self.mode = Mode::Normal;
                self.confirm_done(None);
            }
            ScreenHit::Approve(h) => self.on_approve_hit(h),
            ScreenHit::HelperAnswer(term, choice) => self.answer_helpers(term, choice),
            ScreenHit::SecretGive(term) => self.answer_secret(term, true),
            ScreenHit::SecretDecline(term) => self.answer_secret(term, false),
            ScreenHit::CheckDraft(h) => self.on_check_draft_hit(h),
            ScreenHit::SideFocus => self.act(Action::BrowseTree),
            ScreenHit::ExpandSidebar | ScreenHit::CollapseSidebar => {
                self.act(Action::ToggleSidebar)
            }
            ScreenHit::Actions => self.sidebar_menu(),
            ScreenHit::GroupFolders => self.group_menu(),
            ScreenHit::MachineRow(i) => self.machines_pick(i, None),
            ScreenHit::MachineButton(i, key) => self.machines_pick(i, Some(key)),
            ScreenHit::ActionRow(i) => self.actions_run(i),
            ScreenHit::Leader => self.open_keymap(None),
            ScreenHit::ShortcutMode => {
                self.mode = Mode::Prefix {
                    since: Instant::now(),
                    acted: false,
                }
            }
            ScreenHit::BarKey(i) => self.press_from_bar(i),
            ScreenHit::KeyRow(i) => {
                if let Mode::KeyMap(km) = &self.mode {
                    let km = (**km).clone();
                    self.keymap_run(&km, i);
                } else if matches!(self.mode, Mode::Prefix { .. }) {
                    let km = KeyMap::new(KeyView::Map, None);
                    self.keymap_run(&km, i);
                }
            }
            ScreenHit::Close => {
                // A confirm's Esc chip, or a click outside it, answers no, as Esc does.
                if matches!(&self.mode, Mode::Confirm(c) if !matches!(c.act, crate::client::menu::Act::OverrideCheck(..)))
                {
                    self.mode = Mode::Normal;
                    self.confirm_done(None);
                } else if matches!(self.mode, Mode::Spinners(_)) {
                    if let Mode::Spinners(view) = &self.mode
                        && view.saving
                    {
                        return;
                    }
                    self.back_to_setting("ui.spinner");
                } else if let Mode::Themes { query, sel, saving } = self.mode.clone() {
                    self.on_theme_key(
                        query,
                        sel,
                        saving,
                        &KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                    );
                } else {
                    self.mode = Mode::Normal;
                }
            }
            ScreenHit::ViewClose => self.view = None,
            ScreenHit::Noop => {}
            ScreenHit::ToastClose => {
                // The close mark on a progress toast stops the send it shows.
                self.cancel_send();
                self.dismiss_notice();
            }
            ScreenHit::SettingsBody => {}
            ScreenHit::FinderPick(i) => {
                if let Mode::Finder(fd) = &self.mode {
                    let mut fd = (**fd).clone();
                    fd.sel = i;
                    self.finder_enter(fd);
                }
            }
            ScreenHit::InboxKey(key) => {
                if let Mode::GoTo { query, sel } = self.mode.clone() {
                    let code = match key {
                        '\n' => KeyCode::Enter,
                        '\x1b' => KeyCode::Esc,
                        c => KeyCode::Char(c),
                    };
                    self.on_goto_key(query, sel, &KeyEvent::new(code, KeyModifiers::NONE));
                }
            }
            ScreenHit::NpTask => {
                if let Mode::ScreenPane(np) = &mut self.mode {
                    np.row = 0;
                }
            }
            ScreenHit::NpProj(i)
            | ScreenHit::NpRun(i)
            | ScreenHit::NpBeside(i)
            | ScreenHit::NpWhere(i)
            | ScreenHit::NpModel(i) => {
                if let Mode::ScreenPane(np) = &mut self.mode {
                    match h {
                        ScreenHit::NpWhere(_) => {
                            np.place = Some(i as u8);
                            np.row = 4;
                        }
                        ScreenHit::NpRun(_) => {
                            if np.a != i {
                                np.model = 0;
                            }
                            np.a = i;
                            np.row = 1;
                        }
                        ScreenHit::NpModel(_) => {
                            np.model = i;
                            np.row = 2;
                        }
                        ScreenHit::NpProj(_) => {
                            np.p = i;
                            np.place = np.place.filter(|p| *p < 3);
                            np.a = 0;
                            np.model = 0;
                            np.row = 3;
                        }
                        _ => {
                            np.beside = i == 1;
                            np.row = 5;
                        }
                    }
                }
            }
            ScreenHit::NpGo => {
                if let Mode::ScreenPane(np) = &self.mode {
                    let np = np.clone();
                    self.screen_pane_go(&np);
                }
            }
            ScreenHit::SetTab(i) => {
                if let Mode::ScreenSettings(v) = &mut self.mode {
                    v.cat = i;
                    v.sel = 0;
                    v.editing = None;
                    v.capturing = false;
                }
            }
            ScreenHit::SpinnerPick(i) => self.spinner_pick(i),
            ScreenHit::ThemeTab(light) => self.theme_tab(light),
            ScreenHit::ThemePick(i) => {
                if let Mode::Themes { query, sel, saving } = self.mode.clone() {
                    if saving {
                        return;
                    }
                    self.mode = Mode::Themes {
                        query: query.clone(),
                        sel: i,
                        saving: false,
                    };
                    if sel == i || double {
                        self.on_theme_key(
                            query,
                            i,
                            false,
                            &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                        );
                    }
                }
            }
            ScreenHit::SetRow(i) | ScreenHit::SetVal(i, _) => {
                let Mode::ScreenSettings(v) = &mut self.mode else {
                    return;
                };
                let again = v.sel == i;
                v.sel = i;
                let cat = crate::client::modal::Cat::ALL
                    [v.cat.min(crate::client::modal::Cat::ALL.len() - 1)];
                let row = crate::client::design::settings_rows(cat).get(i).cloned();
                let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
                match (h, row) {
                    (ScreenHit::SetVal(_, vi), Some(row @ SRow::Setting(_)))
                        if vi != usize::MAX && chips_for(self, &row).is_some() =>
                    {
                        set_chip(self, &row, vi)
                    }
                    (ScreenHit::SetVal(..), Some(_)) => self.screen_settings_key(&enter),
                    (ScreenHit::SetRow(_), Some(_)) if again || double => {
                        self.screen_settings_key(&enter)
                    }
                    _ => {}
                }
            }
            ScreenHit::SplashKey(c) => {
                self.splash = false;
                self.screen_splash_action(c);
            }
            ScreenHit::SplashCard(i) => {
                self.screen.splash_away = false;
                if self.screen.splash_sel == i || double {
                    self.screen_splash_open(i);
                } else {
                    self.screen.splash_sel = i;
                }
            }
            ScreenHit::SplashProject(j) => {
                let at = splash_options(self).len() + j;
                self.screen.splash_away = false;
                if self.screen.splash_sel == at || double {
                    self.splash_open_start(at, splash_options(self).len());
                } else {
                    self.screen.splash_sel = at;
                }
            }
            ScreenHit::SplashAway(i) => {
                if self.screen.splash_away && self.screen.splash_away_sel == i || double {
                    self.splash_open_away(i);
                } else {
                    self.screen.splash_away = true;
                    self.screen.splash_away_sel = i;
                }
            }
            ScreenHit::SplashFocus(away) => self.screen.splash_away = away,
            ScreenHit::MenuPick(i) => self.menu_pick(i),
            ScreenHit::HistRow(i) => {
                if let Mode::History { sel } = &mut self.mode {
                    let again = *sel == i;
                    *sel = i;
                    if again || double {
                        self.on_history_key(i, &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
            }
            ScreenHit::GoPick(i) => {
                if let Mode::GoTo { query, sel } = &mut self.mode {
                    let again = *sel == i;
                    *sel = i;
                    let q = query.clone();
                    if again || double {
                        self.on_goto_key(q, i, &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
            }
            ScreenHit::BranchRow(i) => {
                if let Mode::Branch(v) = &mut self.mode {
                    let again = v.sel == i;
                    v.sel = i;
                    if again || double {
                        let v = (**v).clone();
                        self.on_branch_key(v, &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
            }
            ScreenHit::BranchChoice(i) => {
                if let Mode::Branch(v) = &self.mode {
                    let v = (**v).clone();
                    self.branch_choose(v, i);
                }
            }
            ScreenHit::FindTab(i) => {
                if let Mode::Find(v) = &self.mode
                    && v.tab != i
                {
                    let v = (**v).clone();
                    self.on_find_key(v, &KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
                }
            }
            ScreenHit::FindRow(i) => {
                if let Mode::Find(v) = &mut self.mode {
                    let again = v.sel == i;
                    v.sel = i;
                    v.refresh_preview();
                    if again || double {
                        let v = (**v).clone();
                        self.on_find_key(v, &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
            }
            ScreenHit::SideEdge => self.screen.drag = Some(Drag::Side),
            ScreenHit::SheetEdge => self.screen.drag = Some(Drag::Sheet),
            ScreenHit::ToBottom(term) => self.back_to_bottom(term),
            ScreenHit::ScrollBar(term) => {
                self.screen.drag = Some(Drag::Scroll(term));
                if let (Some((t, r, total)), Some(pos)) = (self.screen.bar, self.hover)
                    && t == term
                {
                    let from_bottom = r.bottom().saturating_sub(pos.y + 1) as usize;
                    self.scroll_to(
                        term,
                        (from_bottom * total / r.height.max(1) as usize).min(total),
                    );
                }
            }
        }
    }

    /// A click on a mode bar suggestion presses its key in shortcut mode, so it does exactly
    /// what the key does there, the mode staying for the next one. The click itself took the
    /// mode down (`on_mouse`), so it is put back first.
    fn press_from_bar(&mut self, i: usize) {
        let Some(key) = self.screen.bar_keys.get(i).copied() else {
            return;
        };
        self.mode = Mode::Prefix {
            since: Instant::now(),
            acted: true,
        };
        self.shortcut_held = true;
        self.on_key(KeyEvent::new(key.code, key.mods));
    }

    /// A key for the settings sheet (shared with clicks).
    pub(in crate::client) fn screen_settings_key(&mut self, k: &KeyEvent) {
        if self.on_check_page_key(k) {
            return;
        }
        let Mode::ScreenSettings(v) = self.mode.clone() else {
            return;
        };
        let mut v = *v;
        let open = self.on_settings_view_key(&mut v, k);
        if !open {
            self.mode = Mode::Normal;
        } else if matches!(self.mode, Mode::ScreenSettings(_)) {
            self.mode = Mode::ScreenSettings(Box::new(v));
        }
    }
}
