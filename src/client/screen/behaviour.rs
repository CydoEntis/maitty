//! What keys, clicks and actions do in this layout.

use super::*;

// ---- behaviour -----------------------------------------------------------------------------


pub(in crate::client) use crate::gitfs::WT_NAMES;

impl App {

    /// Where a session that just got the focus shows: beside the one it was opened from, in
    /// a new tab, in the tab that already shows it, or in place of the one you were on.
    pub(in crate::client) fn screen_place(&mut self, f: TermId, prev: Option<TermId>) {
        use crate::layout::{Dir, Node};
        let beside = match self.screen.pending_split {
            Some((p, _)) if p == f => None,
            Some((p, at)) => {
                self.screen.pending_split = None;
                if prev == Some(p) && at.elapsed().as_secs() < 20 && self.snap.terms.contains_key(&p) {
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
        let new_tab = self.screen.new_tab.take().filter(|(at, _)| at.elapsed().as_secs() < 60).map(|(_, owner)| owner);
        let take_out = |tabs: &mut Vec<ScreenTab>, f: TermId| {
            for tab in tabs.iter_mut() {
                if tab.layout.contains(f)
                    && tab.layout.leaves().len() > 1
                    && let Some(l) = tab.layout.clone().remove(f)
                {
                    tab.layout = l;
                    tab.focus = tab.layout.first_leaf();
                }
            }
            tabs.retain(|t| !(t.layout == Node::Leaf(f)));
        };
        if let Some(owner) = new_tab.filter(|o| *o != f) {
            take_out(&mut self.screen.tabs, f);
            self.screen.tabs.push(ScreenTab { layout: Node::Leaf(f), focus: f, owner, used: 0, name: String::new() });
            self.screen.tab = self.screen.tabs.len() - 1;
            return;
        }
        if let Some(p) = beside {
            take_out(&mut self.screen.tabs, f);
            let i = match self.screen.tabs.iter().position(|t| t.layout.contains(p)) {
                Some(i) => i,
                None => {
                    self.screen.tabs.push(ScreenTab { layout: Node::Leaf(p), focus: p, owner: p, used: 0, name: String::new() });
                    self.screen.tabs.len() - 1
                }
            };
            // Beside a wide one, under a tall one.
            let wide = self.screen.leaf_rects.iter().find(|(id, _)| *id == p).is_none_or(|(_, r)| r.width >= r.height * 3);
            let dir = self.screen.split_dir.take().unwrap_or(if wide { Dir::Right } else { Dir::Down });
            let tab = &mut self.screen.tabs[i];
            tab.layout.split(p, dir, f);
            tab.focus = f;
            self.screen.tab = i;
            return;
        }
        // A session with tabs: the one you were last on (unless you're already in one of them).
        let here = self.screen.tabs.get(self.screen.tab).is_some_and(|t| t.owner == f && t.layout.contains(f));
        if !here && let Some(i) = (0..self.screen.tabs.len()).filter(|i| self.screen.tabs[*i].owner == f).max_by_key(|i| self.screen.tabs[*i].used) {
            self.screen.tab = i;
            let to = self.screen.tabs[i].focus;
            if to != f {
                self.cmd(Command::FocusPane { term: to });
            }
            return;
        }
        if let Some(i) = self.screen.tabs.iter().position(|t| t.layout.contains(f)) {
            self.screen.tab = i;
            self.screen.tabs[i].focus = f;
            return;
        }
        if self.screen.tabs.is_empty() {
            self.screen.tabs.push(ScreenTab { layout: Node::Leaf(f), focus: f, owner: f, used: 0, name: String::new() });
            self.screen.tab = 0;
            return;
        }
        // The one you were on was closed: the rest of its split stays as it is, and the focus
        // goes to what's left there (not to whatever the server picked).
        let tab = &mut self.screen.tabs[self.screen.tab];
        if let Some(gone) = prev.filter(|o| tab.layout.contains(*o) && !self.snap.terms.contains_key(o))
            && tab.layout.leaves().len() > 1
            && let Some(rest) = tab.layout.clone().remove(gone)
        {
            tab.layout = rest;
            tab.focus = tab.layout.first_leaf();
            let to = tab.focus;
            self.cmd(Command::FocusPane { term: to });
            return;
        }
        // Not shown anywhere: it shows on its own, full size. A view that's one session's only
        // tab with nothing beside it is reused (this one first); a split or another session's
        // tabs stay as they are (pick its row to get them back).
        let lone = |tabs: &[ScreenTab], i: usize| tabs[i].layout.leaves().len() == 1 && tabs.iter().filter(|u| u.owner == tabs[i].owner).count() == 1;
        let fresh = ScreenTab { layout: Node::Leaf(f), focus: f, owner: f, used: 0, name: String::new() };
        let reuse = if lone(&self.screen.tabs, self.screen.tab) { Some(self.screen.tab) } else { (0..self.screen.tabs.len()).find(|i| lone(&self.screen.tabs, *i)) };
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

    /// The tabs of the session you're on, in order.
    pub(in crate::client) fn session_tabs(&self) -> Vec<usize> {
        let Some(owner) = self.screen.tabs.get(self.screen.tab).map(|t| t.owner) else { return Vec::new() };
        (0..self.screen.tabs.len()).filter(|i| self.screen.tabs[*i].owner == owner).collect()
    }

    /// Close a session's tab and what's in it (one of several: its last tab is the session,
    /// closed from the sidebar). With an agent in it, a second click within a few seconds.
    pub(in crate::client) fn close_tab(&mut self, i: usize) {
        let Some(tab) = self.screen.tabs.get(i) else { return };
        if self.screen.tabs.iter().filter(|t| t.owner == tab.owner).count() < 2 {
            return;
        }
        let leaves = tab.layout.leaves();
        let agents: Vec<String> = leaves.iter().filter_map(|id| self.snap.terms.get(id)).filter_map(|t| t.agent.clone()).collect();
        let armed = self.screen.close_armed.take().is_some_and(|(at, when)| at == i && when.elapsed().as_secs() < 4);
        if !agents.is_empty() && !armed {
            self.screen.close_armed = Some((i, Instant::now()));
            self.notify(format!("{} is running in this tab: close it again to stop it", agents.join(", ")), false);
            return;
        }
        for term in leaves {
            self.cmd(Command::ClosePane { term });
        }
    }

    /// Move tab `from` to where tab `to` is (both of the same session); returns where it is
    /// now. The tab you're on stays the one you're on.
    pub(in crate::client) fn move_tab(&mut self, from: usize, to: usize) -> usize {
        let n = self.screen.tabs.len();
        if from >= n || to >= n || from == to || self.screen.tabs[from].owner != self.screen.tabs[to].owner {
            return from;
        }
        let current = self.screen.tab;
        let tab = self.screen.tabs.remove(from);
        self.screen.tabs.insert(to, tab);
        // Every index between the two shifted by one; the one you're on follows its tab.
        self.screen.tab = if current == from {
            to
        } else if from < to && current > from && current <= to {
            current - 1
        } else if to < from && current >= to && current < from {
            current + 1
        } else {
            current
        };
        self.screen.close_armed = None;
        to
    }

    /// Stop showing `t` beside the others (it keeps running). False when it's on its own.
    pub(in crate::client) fn screen_unshow(&mut self, t: TermId) -> bool {
        let Some(i) = self.screen.tabs.iter().position(|tab| tab.layout.contains(t)) else { return false };
        let tab = &mut self.screen.tabs[i];
        if tab.layout.leaves().len() < 2 {
            return false;
        }
        if let Some(l) = tab.layout.clone().remove(t) {
            tab.layout = l;
            tab.focus = tab.layout.first_leaf();
            let to = tab.focus;
            if self.focused() == Some(t) {
                self.cmd(Command::FocusPane { term: to });
            }
        }
        true
    }


    /// Show a session: it becomes the focused one.
    pub(in crate::client) fn screen_focus(&mut self, term: TermId) {
        self.screen.cursor = None;
        self.mode = Mode::Normal;
        self.cmd(Command::FocusPane { term });
    }

    /// Start a session in `cwd` running `cmd` (None: a shell), beside the focused one or
    /// on its own.
    pub(in crate::client) fn screen_new_session(&mut self, cwd: PathBuf, cmd: Option<String>, beside: bool) {
        if beside && let Some(f) = self.focused() {
            self.screen.pending_split = Some((f, Instant::now()));
        }
        self.screen.cursor = None;
        self.mode = Mode::Normal;
        self.cmd(Command::NewWorkspace { cwd: Some(cwd), name: None, cmd });
    }

    /// A new worktree of a project running `cmd`: on `branch` if given (an existing one),
    /// else on a new branch with a generated name.
    pub(in crate::client) fn screen_new_worktree(&mut self, proj: &Proj, cmd: Option<String>, beside: bool, branch: Option<String>) {
        let taken: HashSet<String> = proj.wts.iter().flat_map(|w| [w.name.clone(), w.branch.clone()]).chain(proj.branches.iter().cloned()).collect();
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as usize).unwrap_or(0);
        let branch = branch.unwrap_or_else(|| {
            (0..WT_NAMES.len())
                .map(|i| WT_NAMES[(n + i) % WT_NAMES.len()].to_string())
                .find(|b| !taken.contains(b))
                .unwrap_or_else(|| format!("{}-{}", WT_NAMES[n % WT_NAMES.len()], n % 1000))
        });
        let Some(ws) = self.active_ws().map(|w| w.id).or_else(|| self.snap.workspaces.first().map(|w| w.id)) else {
            self.notify("start something first".into(), true);
            return;
        };
        if beside && let Some(f) = self.focused() {
            self.screen.pending_split = Some((f, Instant::now()));
        }
        self.mode = Mode::Normal;
        self.notify(format!("new worktree {branch} in {}", proj.name), false);
        self.cmd(Command::NewWorktree { ws, branch, base: None, cmd, split: None, from: Some(proj.path.clone()) });
    }

    /// A new worktree running `cmd` (on `branch` if given), full screen.
    pub(in crate::client) fn screen_start_worktree(&mut self, proj: &Proj, cmd: Option<String>, branch: Option<String>) {
        self.screen_new_worktree(proj, cmd, false, branch);
    }

    /// Open a folder as a project: switch to it, or start a session there if nothing runs.
    pub(in crate::client) fn screen_open_project(&mut self, path: PathBuf) {
        self.mode = Mode::Normal;
        self.splash = false;
        let root = crate::gitfs::head(&path).map(|h| h.main_root).unwrap_or_else(|| path.clone());
        let key = path_key(&root);
        let model = self.screen_model();
        let existing = model.iter().find(|p| p.key == key);
        let found = existing.and_then(|p| p.sessions().min_by_key(|s| (rank(s.status), s.term)).map(|s| (p.name.clone(), s.term)));
        let is_new = existing.is_none();
        self.screen.proj = Some(key.clone());
        match found {
            Some((name, term)) => {
                self.screen_focus(term);
                self.notify(format!("Switched to {name}"), false);
            }
            None => {
                if is_new {
                    self.screen.fresh.insert(key);
                }
                self.screen_new_session(root.clone(), None, false);
                self.notify(format!("Opened {} with a shell; + New starts an agent", folder_name(&root)), false);
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
        let key = self.screen.cursor.and_then(|c| model.iter().find(|p| p.sessions().any(|s| s.term == c)).map(|p| p.key.clone())).or(self.screen.proj.clone());
        let p = model.iter().position(|p| Some(&p.key) == key.as_ref()).unwrap_or(0);
        self.screen_new(p, beside);
    }

    pub(in crate::client) fn screen_settings(&mut self) {
        self.refresh_installed_version();
        self.screen.check_page = None;
        self.mode = Mode::ScreenSettings(Box::new(SView { cat: 0, sel: 0, editing: None, capturing: false, scroll: 0 }));
    }

    /// Move the sidebar cursor; bare keys then work on the sidebar until Esc or Enter.
    pub(in crate::client) fn screen_side_move(&mut self, d: i8) {
        let items = self.screen.side_items.clone();
        if items.is_empty() {
            return;
        }
        let cur = match (&self.screen.cursor_proj, self.screen.cursor.or(self.focused())) {
            (Some(k), _) => items.iter().position(|i| *i == SideItem::Proj(k.clone())),
            (None, Some(t)) => items.iter().position(|i| *i == SideItem::Sess(t)),
            _ => None,
        };
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
        let cur = match (&self.screen.cursor_proj, self.screen.cursor) {
            (Some(k), _) => items.iter().position(|i| *i == SideItem::Proj(k.clone())),
            (None, Some(t)) => items.iter().position(|i| *i == SideItem::Sess(t)),
            _ => None,
        }?;
        items.get(cur + 1).or_else(|| cur.checked_sub(1).and_then(|i| items.get(i))).cloned()
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
        match item {
            SideItem::Proj(k) => {
                self.screen.cursor_proj = Some(k);
                self.screen.cursor = None;
            }
            SideItem::Sess(t) => {
                self.screen.cursor_proj = None;
                self.screen.cursor = Some(t);
                // Resting on a finished agent counts as seeing it.
                if self.snap.terms.get(&t).is_some_and(|i| i.status == Status::Done) {
                    self.cmd(Command::MarkSeen { term: t });
                }
            }
        }
    }

    pub(in crate::client) fn screen_side_leave(&mut self) {
        self.screen.cursor = None;
        self.screen.cursor_proj = None;
        self.mode = Mode::Normal;
    }

    /// Fold or unfold a project (true: open it).
    /// A new colour for a project's +/− button, never the one it had.
    pub(in crate::client) fn pick_fold_color(&mut self, key: &str) {
        let mut palette: Vec<Color> = self.cfg.ui.workspace_colors.iter().filter_map(|c| crate::theme::parse_color(c)).collect();
        let had = self.screen.fold_colors.get(key).copied();
        palette.retain(|c| Some(*c) != had);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos() as usize);
        if let Some(c) = palette.get(nanos % palette.len().max(1)) {
            self.screen.fold_colors.insert(key.to_string(), *c);
        }
    }

    pub(in crate::client) fn screen_fold(&mut self, key: &str, open: Option<bool>) {
        let k = format!("p:{key}");
        let is_open = !self.screen.saved.closed.contains(&k);
        let want = open.unwrap_or(!is_open);
        if want != is_open {
            if want {
                self.screen.saved.closed.retain(|c| *c != k);
                self.pick_fold_color(key);
            } else {
                self.screen.saved.closed.push(k);
            }
            self.screen.save();
        }
    }

    /// Actions that work differently in this layout. Returns true if handled.
    pub(in crate::client) fn screen_act(&mut self, a: &Action) -> bool {
        let focused = self.focused();
        match a {
            Action::Settings => self.screen_settings(),
            Action::NewPane => self.screen_open_new_session(None),
            Action::StartSession => self.screen_start_session(None),
            // A plain shell in the project's own folder (the repo itself, not a worktree).
            Action::ShellHere => {
                let dir = self
                    .screen
                    .cursor
                    .or(self.focused())
                    .and_then(|t| self.snap.terms.get(&t))
                    .map(|t| t.root.clone().unwrap_or_else(|| t.cwd.clone()))
                    .or_else(|| self.screen_model().iter().find(|p| Some(&p.key) == self.screen.proj.as_ref()).map(|p| p.path.clone()))
                    .unwrap_or_else(|| self.here_dir());
                self.screen.cursor = None;
                self.screen_new_session(dir, None, false);
            }
            // One place: what needs you on top, and type to go anywhere.
            Action::Message | Action::Why => {
                let side = self.mode == Mode::Side || self.screen.cursor.is_some();
                match self.screen.cursor.or(self.focused()) {
                    Some(term) if *a == Action::Message => self.start_compose(term, side, None),
                    Some(term) => self.mode = Mode::Why { term, side },
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
            Action::Focus(d) => match focused.and_then(|f| crate::layout::neighbor(&self.screen.leaf_rects, f, *d)) {
                Some(to) => self.cmd(Command::FocusPane { term: to }),
                // Past the left edge: the sidebar.
                None if *d == crate::layout::Dir::Left && !self.side_hidden => self.screen_side_move(0),
                None => {}
            },
            Action::FocusNext | Action::FocusPrev => {
                let shown: Vec<TermId> = self.screen.leaf_rects.iter().map(|(id, _)| *id).collect();
                if let Some(f) = focused
                    && shown.len() > 1
                {
                    let i = shown.iter().position(|x| *x == f).unwrap_or(0);
                    let n = shown.len();
                    let to = if *a == Action::FocusNext { shown[(i + 1) % n] } else { shown[(i + n - 1) % n] };
                    self.cmd(Command::FocusPane { term: to });
                }
            }
            // A new tab in this session: a shell where you are, straight away.
            Action::NewTab => self.new_tab_start(),
            Action::RenameTab => self.rename_tab_start(),
            Action::Menu => self.mode = Mode::Actions { sel: 0 },
            Action::Help => self.open_key_view(KeyView::All),
            Action::Palette => self.open_key_view(KeyView::Search),
            Action::Worktrees => self.open_keymap(Some(Step::Worktrees)),
            Action::NextTab | Action::PrevTab => {
                // This session's tabs, in order.
                let seen = self.session_tabs();
                let n = seen.len();
                if n > 1 {
                    let at = seen.iter().position(|i| *i == self.screen.tab);
                    let next = match (at, *a == Action::NextTab) {
                        (Some(k), true) => seen[(k + 1) % n],
                        (Some(k), false) => seen[(k + n - 1) % n],
                        (None, _) => seen[0],
                    };
                    self.screen.tab = next;
                    let to = self.screen.tabs[next].focus;
                    self.cmd(Command::FocusPane { term: to });
                }
            }
            Action::SelectTab(n) => {
                let seen = self.session_tabs();
                if let Some(i) = n.checked_sub(1).and_then(|k| seen.get(k).copied())
                    && let Some(tab) = self.screen.tabs.get(i)
                {
                    let to = tab.focus;
                    self.screen.tab = i;
                    self.cmd(Command::FocusPane { term: to });
                }
            }
            Action::CloseTab => self.close_tab(self.screen.tab),
            // z: the sessions move into the top strip, or back into the sidebar.
            Action::ToggleSidebar => {
                self.sidebar = !self.sidebar;
                self.side_hidden = false;
                self.screen.follow = true;
            }
            // Z: no sessions at all; the same key brings the sidebar back.
            Action::HideSidebar => {
                self.side_hidden = self.sidebar || !self.side_hidden;
                self.sidebar = !self.side_hidden;
                self.screen.follow = true;
                if self.side_hidden && self.mode == Mode::Side {
                    self.screen_side_leave();
                }
            }
            Action::RenameWorkspace => {
                if let Some(t) = self.screen.cursor.or(focused) {
                    self.menu_do(crate::client::menu::Act::RenamePane(t));
                }
            }
            Action::SplitRight | Action::SplitDown | Action::SplitLeft | Action::SplitUp | Action::Spawn(..) => {
                self.screen.split_dir = Some(match a {
                    Action::SplitDown | Action::SplitUp => crate::layout::Dir::Down,
                    Action::Spawn(d, _) => *d,
                    _ => crate::layout::Dir::Right,
                });
                let cwd = focused.and_then(|f| self.snap.terms.get(&f)).map(|t| t.cwd.clone()).unwrap_or_else(|| self.here_dir());
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
                let many = self.screen.tabs.get(self.screen.tab).is_some_and(|tab| tab.layout.leaves().len() > 1);
                if let (true, Some(f)) = (many, focused) {
                    let tab = self.screen.tab;
                    self.screen.tabs[tab].layout.resize(f, *d, 0.05);
                } else {
                    let w = self.screen.saved.side_w.unwrap_or(self.screen.side_rect.width.max(SIDE_MIN));
                    let w = if grow { w + 2 } else { w.saturating_sub(2) };
                    self.screen.saved.side_w = Some(w.clamp(SIDE_MIN, SIDE_MAX));
                }
                self.screen.save();
            }
            Action::Files | Action::Changes => {
                let dir = self.screen_target_dir();
                self.screen.cursor = None;
                self.mode = Mode::Normal;
                if *a == Action::Files {
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

    /// Keys while the sidebar cursor is up: arrows move, Enter opens, other keys act as
    /// if the leader had been pressed.
    /// Keys while the sidebar has them: ↑↓ move, →← open or fold (← from a session goes to
    /// its project), Enter opens, Space messages, Esc goes back to the pane. Anything you
    /// type goes to the pane you're on.
    pub(in crate::client) fn on_side_key(&mut self, k: &KeyEvent) {
        let spec = KeySpec::from_event(k);
        let proj = self.screen.cursor_proj.clone();
        let model = self.screen_model();
        if spec == self.keymap.prefix {
            self.mode = Mode::Prefix { since: Instant::now(), acted: false };
            return;
        }
        let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if k.code == KeyCode::Enter && proj.is_none() && let Some(c) = self.screen.cursor && self.helpers_asked(c) {
            self.open_helpers(c, true);
            return;
        }
        if k.code == KeyCode::Enter && proj.is_none() && let Some(c) = self.screen.cursor && self.approval_waits(c) {
            self.screen_side_leave();
            self.open_approval(c);
            return;
        }
        match spec.code {
            KeyCode::Up | KeyCode::Char('k') if !k.modifiers.contains(KeyModifiers::CONTROL) => self.screen_side_move(-1),
            KeyCode::Down | KeyCode::Char('j') if !k.modifiers.contains(KeyModifiers::CONTROL) => self.screen_side_move(1),
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
            // The collapsed strip: ← → and 1–9 (only while it has the keys) switch sessions.
            KeyCode::Left | KeyCode::Right | KeyCode::Char('1'..='9') if super::draw::strip_on(self) && plain => self.strip_switch(spec.code),
            KeyCode::Right => match &proj {
                Some(key) => self.screen_fold(key, Some(true)),
                None => {
                    // Into the pane.
                    if let Some(c) = self.screen.cursor {
                        self.screen_focus(c);
                    }
                    self.screen_side_leave();
                }
            },
            KeyCode::Left => match (&proj, self.screen.cursor) {
                (Some(key), _) => self.screen_fold(key, Some(false)),
                (None, Some(t)) => {
                    if let Some(p) = model.iter().find(|p| p.sessions().any(|s| s.term == t)) {
                        self.screen_side_set(SideItem::Proj(p.key.clone()));
                    }
                }
                _ => {}
            },
            KeyCode::Enter => match (&proj, self.screen.cursor) {
                (Some(key), _) => self.screen_fold(key, None),
                (None, Some(c)) if self.check_wants_a_look(c) => {
                    self.screen_side_leave();
                    self.open_check_sheet(c);
                }
                (None, c) => {
                    if let Some(c) = c {
                        self.screen_focus(c);
                    }
                    self.screen_side_leave();
                }
            },
            KeyCode::Esc => self.screen_side_leave(),
            _ if spec == self.keymap.prefix => self.mode = Mode::Prefix { since: Instant::now(), acted: false },
            // Nothing else acts from the sidebar: an action is the leader and its key (ADR 0014).
            // Anything else is typing: leave the sidebar and hand the key over.
            _ => {
                self.screen_side_leave();
                if let Some(term) = self.focused() {
                    let data = crate::keys::encode(k, self.parsers.get(&term).is_some_and(|p| p.screen().application_cursor()));
                    if !data.is_empty() {
                        self.send(crate::protocol::ClientMsg::Input { term, data });
                    }
                }
            }
        }
    }

    /// A key on the collapsed strip: ← → step to the neighbouring session, 1–9 jump to that one.
    /// The session opens in the pane, and the keys stay on the strip.
    fn strip_switch(&mut self, code: KeyCode) {
        let sessions: Vec<TermId> = self.screen.side_items.iter().filter_map(|i| if let SideItem::Sess(t) = i { Some(*t) } else { None }).collect();
        let here = sessions.iter().position(|t| Some(*t) == self.screen.cursor.or(self.focused()));
        let to = match code {
            KeyCode::Left => here.map_or(0, |i| i.saturating_sub(1)),
            KeyCode::Right => here.map_or(0, |i| (i + 1).min(sessions.len().saturating_sub(1))),
            KeyCode::Char(c) => c.to_digit(10).map_or(0, |d| d as usize - 1),
            _ => return,
        };
        if let Some(&term) = sessions.get(to) {
            self.screen_side_set(SideItem::Sess(term));
            self.cmd(Command::FocusPane { term });
        }
    }

    /// Go to, opened on its first row you can pick (what needs you, when anything does).
    pub(in crate::client) fn open_goto(&mut self) {
        let rows = goto_rows(&self.screen_model(), "", heads_up(self).len());
        let sel = rows.iter().position(GoRow::pickable).unwrap_or(0);
        self.mode = Mode::GoTo { query: String::new(), sel };
    }

    /// Answer an agent's numbered question from the Inbox: its option `key`, as if typed there.
    pub(in crate::client) fn inbox_answer(&mut self, term: TermId, key: char, name: &str) {
        let label = self.answer_options(term).get((key as u8 - b'1') as usize).cloned().unwrap_or_else(|| key.to_string());
        self.answer(term, key);
        self.notify(format!("answered {name}: {label}"), false);
    }

    /// A question an agent asked you with `seshi ask-human`, waiting in this pane.
    pub(in crate::client) fn pending_question(&self, term: TermId) -> Option<&crate::protocol::HumanQuestion> {
        self.snap.questions.iter().find(|q| q.term == term)
    }

    /// The answers a waiting agent offers: its ask-human question's, else the numbered
    /// choices on its screen.
    pub(in crate::client) fn answer_options(&self, term: TermId) -> Vec<String> {
        match self.pending_question(term) {
            Some(q) => q.options.clone(),
            None => options(self.parsers.get(&term)),
        }
    }

    /// Answer with choice `key` ('1' is the first): to its ask-human question, or typed into
    /// the agent as its own numbered prompt expects.
    pub(in crate::client) fn answer(&mut self, term: TermId, key: char) {
        match self.pending_question(term).map(|q| q.id) {
            Some(id) => self.cmd(Command::AnswerHuman { id, choice: (key as u8).saturating_sub(b'1') as usize }),
            None => self.send(crate::protocol::ClientMsg::Input { term, data: key.to_string().into_bytes() }),
        }
    }

    /// The folder Files / Changes / PR act on: the sidebar cursor's, else the focused one's.
    pub(in crate::client) fn screen_target_dir(&self) -> PathBuf {
        self.screen
            .cursor
            .or(self.focused())
            .and_then(|t| self.snap.terms.get(&t))
            .map(|t| t.top.clone().unwrap_or_else(|| t.cwd.clone()))
            .unwrap_or_else(|| self.here_dir())
    }

    /// Picking for a setting: save the folder and go back to Settings, on that row.
    fn finder_chose(&mut self, fd: &Finder, p: PathBuf) -> bool {
        let Some(path) = fd.for_setting else { return false };
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
                    .position(|r| matches!(r, crate::client::design::SRow::Setting(s) if s.path == path))
                    .map(|i| (ci, i))
            })
            .unwrap_or((0, 0));
        self.mode = Mode::ScreenSettings(Box::new(SView { cat, sel, editing: None, capturing: false, scroll: 0 }));
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
        let main = p.wts.iter().find(|w| w.main).map(|w| w.path.clone()).unwrap_or_else(|| p.path.clone());
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
            return self.screen_new_session(main, cmd, np.beside);
        }
        match np.place_for(&agent, self.cfg.worktree.per_agent) {
            0 => self.screen_new_worktree(&p, cmd, np.beside, None),
            1 => {
                // A new branch right in the repo folder (no worktree).
                let taken: HashSet<String> = p.wts.iter().map(|w| w.branch.clone()).collect();
                let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as usize).unwrap_or(0);
                let name = (0..WT_NAMES.len()).map(|i| WT_NAMES[(n + i) % WT_NAMES.len()].to_string()).find(|b| !taken.contains(b)).unwrap_or_else(|| format!("branch-{}", n % 10_000));
                let (pname, beside) = (p.name.clone(), np.beside);
                self.spawn_bg(move || {
                    let mut git = std::process::Command::new("git");
                    git.arg("-C").arg(&main).args(["switch", "-c", &name]);
                    crate::proc::quiet(&mut git);
                    let out = git.output();
                    crate::client::Bg::Then(Box::new(move |app: &mut App| match out {
                        Ok(o) if o.status.success() => {
                            app.notify(format!("{pname} is on a new branch, {name}"), false);
                            app.screen_new_session(main, cmd, beside);
                        }
                        Ok(o) => app.notify(format!("couldn't make a branch: {}", String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("git failed")), true),
                        Err(e) => app.notify(format!("couldn't run git: {e}"), true),
                    }))
                });
            }
            n @ 3.. => match p.wts.iter().filter(|w| !w.main).nth(n as usize - 3) {
                Some(w) => self.screen_new_session(w.path.clone(), cmd, np.beside),
                None => {
                    self.mode = Mode::ScreenPane(np.clone());
                    self.notify("that worktree is no longer available; choose where to start again".into(), true);
                }
            },
            _ => self.screen_new_session(main, cmd, np.beside),
        }
    }

    pub(in crate::client) fn on_goto_key(&mut self, mut query: String, mut sel: usize, k: &KeyEvent) {
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
        let sel_item = sel_row.as_ref().filter(|r| !matches!(r, GoRow::Sess(..))).and_then(GoRow::term);
        let sel_check = sel_item.and_then(|t| find(&model, t)).and_then(|(.., s)| super::check::shown(s).cloned());
        // A merge or throw-away asked in a finished row: Enter does it, Esc doesn't, the rest wait.
        if let Some((term, merge, false)) = self.screen.inbox_confirm {
            match k.code {
                KeyCode::Esc => self.screen.inbox_confirm = None,
                KeyCode::Enter => {
                    if let Some((_, wt, _)) = find(&model, term) {
                        let dir = wt.path.clone();
                        if merge {
                            if let Some((task, ..)) = self.task_for(&dir) {
                                self.spawn_bg(move || crate::client::Bg::Merged(crate::client::tasks::merge(&task), dir));
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
            KeyCode::Char('1'..='9') if query.is_empty() && matches!(sel_row, Some(GoRow::Ask(t)) if self.helpers_asked(t)) => {
                if let Some(GoRow::Ask(term)) = sel_row {
                    return self.open_helpers_from_inbox(term, query, sel);
                }
            }
            KeyCode::Char(c @ '1'..='9') if query.is_empty() && matches!(sel_row, Some(GoRow::Ask(_))) => {
                if let Some(GoRow::Ask(term)) = sel_row {
                    let name = self.session_name(term);
                    self.inbox_answer(term, c, &name);
                }
            }
            // Seen: a finished one leaves the list.
            // A heads-up: both diffs, a note to the first agent, or dismiss it.
            KeyCode::Char(c @ ('d' | 'm' | 'k')) if query.is_empty() && matches!(sel_row, Some(GoRow::Heads(_))) => {
                let Some(GoRow::Heads(i)) = sel_row else { return };
                let Some((repo, o)) = heads_up(self).get(i).cloned() else { return };
                match c {
                    'd' => return self.open_both(repo, o),
                    'm' => return self.tell_about(&repo, &o, 0, Some((query, sel))),
                    _ => self.dismiss_heads_up(&repo, &o),
                }
                self.mode = Mode::GoTo { query, sel: sel.saturating_sub(1) };
                return;
            }
            // A finished one: its diff (in the sheet), or merge it or throw it away (asked first).
            KeyCode::Char(c @ ('d' | 'M' | 'x')) if query.is_empty() && matches!(sel_row, Some(GoRow::Done(_))) => {
                let Some(GoRow::Done(term)) = sel_row else { return };
                let Some((_, wt, _)) = find(&model, term) else { return };
                match c {
                    'd' => {
                        let dir = wt.path.clone();
                        self.mode = Mode::Normal;
                        self.open_changes(dir);
                        return;
                    }
                    _ if !wt.main => self.screen.inbox_confirm = Some((term, c == 'M', false)),
                    _ => self.notify("the main folder isn't a worktree: open Changes (d) to commit".into(), false),
                }
                self.mode = Mode::GoTo { query, sel };
                return;
            }
            // m: a follow-up, inside the row.
            KeyCode::Char('m') if query.is_empty() && matches!(sel_row, Some(GoRow::Ask(_) | GoRow::Done(_))) => {
                if let Some(GoRow::Ask(term) | GoRow::Done(term)) = sel_row {
                    self.start_compose(term, false, Some((query, sel)));
                }
                return;
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
            KeyCode::Char(c @ ('r' | 'O' | 'u' | ',')) if query.is_empty() && sel_check.as_ref().is_some_and(|i| super::check_inbox::check_key_acts(c, i)) => {
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
                    self.cmd(Command::MarkSeen { term });
                }
            }
            KeyCode::Delete if let Some(GoRow::Quiet(term)) = sel_row => {
                if let Some(at) = self.snap.terms.get(&term).and_then(|i| i.quiet_since) {
                    self.screen.quiet_seen.insert(term, at);
                    self.screen_fresh();
                }
            }
            KeyCode::Enter if let Some(t) = sel_row.as_ref().and_then(GoRow::term) && self.approval_waits(t) => {
                self.open_approval(t);
                self.screen.inbox_return = Some((query, sel));
                return;
            }
            // The project's check settings, for a row about a session.
            KeyCode::Char(',') if query.is_empty() && let Some(t) = sel_row.as_ref().and_then(GoRow::term) => return self.open_project_check(t),
            KeyCode::Enter => {
                let stale = sel_check.as_ref().is_some_and(|i| i.state == crate::protocol::CheckState::Stale);
                match sel_row {
                    Some(GoRow::Ask(t)) if query.is_empty() && self.helpers_asked(t) => return self.open_helpers_from_inbox(t, query, sel),
                    Some(GoRow::Ask(t)) if query.is_empty() => return self.start_compose(t, false, Some((query, sel))),
                    Some(GoRow::Approve(t) | GoRow::Failed(t) | GoRow::NotRun(t)) => self.open_check_sheet(t),
                    Some(GoRow::Done(t)) if stale && query.is_empty() => {
                        self.do_check_key(t, None, 'r');
                        self.mode = Mode::GoTo { query, sel };
                    }
                    Some(GoRow::Ask(t) | GoRow::Done(t) | GoRow::Quiet(t) | GoRow::Sess(_, t)) => {
                        self.mode = Mode::Normal;
                        self.screen_focus(t);
                    }
                    Some(GoRow::Head(_) | GoRow::Heads(_)) | None => self.mode = Mode::Normal,
                }
                return;
            }
            KeyCode::Backspace => {
                query.pop();
                sel = 0;
            }
            KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                query.push(c);
                // Land on the first session that matches, when there is one.
                let rows = goto_rows(&model, &query, heads_up(self).len());
                sel = rows.iter().position(GoRow::pickable).unwrap_or(0);
            }
            _ => {}
        }
        if query.is_empty() && !matches!(k.code, KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown) {
            sel = land(sel, true);
        }
        self.mode = Mode::GoTo { query, sel };
    }

    pub(in crate::client) fn on_history_key(&mut self, sel: usize, k: &KeyEvent) {
        let n = self.history.len();
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Down => self.mode = Mode::History { sel: (sel + 1).min(n.saturating_sub(1)) },
            KeyCode::Up => self.mode = Mode::History { sel: sel.saturating_sub(1) },
            KeyCode::Char('c') => {
                self.history.clear();
                self.mode = Mode::History { sel: 0 };
            }
            KeyCode::Enter => {
                let term = self.history.iter().rev().nth(sel).and_then(|h| h.1);
                match term.filter(|t| self.snap.terms.contains_key(t)) {
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
        let nmodels = if agent.starts_with('★') || agent.starts_with('◇') { 0 } else { np_models(self, &agent).len() };
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
                    if agent.starts_with('◇') { np.a = 0; np.model = 0; }
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

    /// Keys on the splash: move between its buttons, Enter picks one, or a button's own key.
    /// Nothing else leaves it.
    pub(in crate::client) fn on_hy_splash_key(&mut self, k: &KeyEvent) {
        let opts = splash_options(self);
        let n = opts.len().max(1);
        match k.code {
            KeyCode::Up | KeyCode::Left | KeyCode::BackTab | KeyCode::Char('k') => self.screen.splash_sel = (self.screen.splash_sel + n - 1) % n,
            KeyCode::Down | KeyCode::Right | KeyCode::Tab | KeyCode::Char('j') => self.screen.splash_sel = (self.screen.splash_sel + 1) % n,
            KeyCode::Enter => {
                if let Some((_, _, c)) = opts.get(self.screen.splash_sel.min(n - 1)) {
                    self.screen_splash_action(*c);
                }
            }
            KeyCode::Char(c) if opts.iter().any(|(_, _, k)| *k == c) || c == '?' || c == ',' => self.screen_splash_action(c),
            _ => {}
        }
    }

    pub(in crate::client) fn screen_splash_action(&mut self, c: char) {
        self.splash = false;
        self.screen.splash_sel = 0;
        match c {
            'n' => self.act(Action::StartSession),
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
            ScreenHit::ToggleProj(i) => {
                if let Some(key) = self.screen.proj_keys.get(i).cloned() {
                    self.screen_fold(&key, None);
                    // The keys follow you into the sidebar.
                    self.screen_side_set(SideItem::Proj(key));
                }
            }
            // Opens it, and the arrows keep walking the sidebar until you type or press Enter.
            ScreenHit::Session(t) => {
                self.screen_focus(t);
                self.screen_side_set(SideItem::Sess(t));
            }
            ScreenHit::Update => self.act(Action::Update),
            ScreenHit::CheckUpdate => self.act(Action::CheckUpdate),
            ScreenHit::Restart => self.ask_restart(),
            ScreenHit::Why(term) => {
                self.mode = Mode::Why { term, side: matches!(self.mode, Mode::Side) };
            }
            ScreenHit::CheckEvidence(term) => self.open_check_sheet(term),
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
            ScreenHit::ShellIn(pi) => {
                if let Some(p) = self.screen_model().get(pi) {
                    let dir = p.wts.iter().find(|w| w.main).map(|w| w.path.clone()).unwrap_or_else(|| p.path.clone());
                    self.screen_new_session(dir, None, false);
                }
            }
            ScreenHit::RowMenuSess(t) => {
                let at = self.hover.map(|p| (p.x, p.y)).unwrap_or((0, 0));
                self.menu_for_session(t, at);
            }
            ScreenHit::RowMenuProj(i) => {
                let at = self.hover.map(|p| (p.x, p.y)).unwrap_or((0, 0));
                self.menu_for_project(i, at);
            }
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
            ScreenHit::HelperAsk(term) => self.open_helpers(term, matches!(self.mode, Mode::Side)),
            ScreenHit::HelperAnswer(term, choice) => self.answer_helpers(term, choice),
            ScreenHit::CheckDraft(h) => self.on_check_draft_hit(h),
            ScreenHit::TabPick(i) => {
                if let Some(tab) = self.screen.tabs.get(i) {
                    let to = tab.focus;
                    self.screen.tab = i;
                    self.cmd(Command::FocusPane { term: to });
                    // Double-click: rename it in its pill.
                    if double {
                        self.rename_tab_start();
                    }
                }
            }
            ScreenHit::CloseTab(i) => self.close_tab(i),
            ScreenHit::NewTab => self.act(Action::NewTab),
            ScreenHit::SideFocus => self.act(Action::BrowseTree),
            ScreenHit::ExpandSidebar => self.act(Action::ToggleSidebar),
            ScreenHit::NewShell => self.act(Action::ShellHere),
            ScreenHit::NsOpt(row, i) => self.ns_pick(row, i),
            ScreenHit::NsStart => self.ns_start(),
            ScreenHit::Actions => self.act(Action::Menu),
            ScreenHit::ActionRow(i) => self.actions_run(i),
            ScreenHit::Leader => self.open_keymap(None),
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
                if matches!(self.mode, Mode::Spinners(_)) {
                    if let Mode::Spinners(view) = &self.mode && view.saving { return; }
                    self.back_to_setting("ui.spinner");
                } else if let Mode::Themes { query, sel, saving } = self.mode.clone() {
                    self.on_theme_key(query, sel, saving, &KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                } else {
                    self.mode = Mode::Normal;
                }
            }
            ScreenHit::ViewClose => self.view = None,
            ScreenHit::Noop => {}
            ScreenHit::ToastClose => self.dismiss_notice(),
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
            ScreenHit::NpProj(i) | ScreenHit::NpRun(i) | ScreenHit::NpBeside(i) | ScreenHit::NpWhere(i) | ScreenHit::NpModel(i) => {
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
            ScreenHit::ThemePick(i) => {
                if let Mode::Themes { query, sel, saving } = self.mode.clone() {
                    if saving { return; }
                    self.mode = Mode::Themes { query: query.clone(), sel: i, saving: false };
                    if sel == i || double {
                        self.on_theme_key(query, i, false, &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
            }
            ScreenHit::SetRow(i) | ScreenHit::SetVal(i, _) => {
                let Mode::ScreenSettings(v) = &mut self.mode else { return };
                let again = v.sel == i;
                v.sel = i;
                let cat = crate::client::modal::Cat::ALL[v.cat.min(crate::client::modal::Cat::ALL.len() - 1)];
                let row = crate::client::design::settings_rows(cat).get(i).cloned();
                let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
                match (h, row) {
                    (ScreenHit::SetVal(_, vi), Some(row @ SRow::Setting(_))) if vi != usize::MAX && chips_for(self, &row).is_some() => set_chip(self, &row, vi),
                    (ScreenHit::SetVal(..), Some(_)) => self.screen_settings_key(&enter),
                    (ScreenHit::SetRow(_), Some(_)) if again || double => self.screen_settings_key(&enter),
                    _ => {}
                }
            }
            ScreenHit::SplashKey(c) => {
                self.splash = false;
                self.screen_splash_action(c);
            }
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
            ScreenHit::ToBottom(term) => self.back_to_bottom(term),
            ScreenHit::ScrollBar(term) => {
                self.screen.drag = Some(Drag::Scroll(term));
                if let (Some((t, r, total)), Some(pos)) = (self.screen.bar, self.hover)
                    && t == term
                {
                    let from_bottom = r.bottom().saturating_sub(pos.y + 1) as usize;
                    self.scroll_to(term, (from_bottom * total / r.height.max(1) as usize).min(total));
                }
            }
        }
    }

    /// A key for the settings sheet (shared with clicks).
    pub(in crate::client) fn screen_settings_key(&mut self, k: &KeyEvent) {
        if self.on_check_page_key(k) {
            return;
        }
        let Mode::ScreenSettings(v) = self.mode.clone() else { return };
        let mut v = *v;
        let open = self.on_settings_view_key(&mut v, k);
        if !open {
            self.mode = Mode::Normal;
        } else if matches!(self.mode, Mode::ScreenSettings(_)) {
            self.mode = Mode::ScreenSettings(Box::new(v));
        }
    }
}
