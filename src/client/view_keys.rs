//! Keys inside the full-pane views and popups that aren't the app's own.

use super::*;

impl App {
    /// Keys while a view replaces the pane area (single letters act directly).
    pub(super) fn on_view_key(&mut self, k: &KeyEvent) {
        // `[` `]` `=` and Tab are text while Files' filter or editor takes it, and answer
        // Changes' y/n question (anything but y says no) rather than leave it unanswered.
        let typing = match &self.view {
            Some(View::Files(v)) => v.filtering || v.edit.is_some(),
            Some(View::Changes(v)) => v.confirm.is_some(),
            _ => false,
        };
        if !typing && (self.sheet_size_key(k) || self.drawer_tab_key(k)) {
            return;
        }
        let Some(view) = self.view.take() else { return };
        match view {
            View::Changes(mut v) => {
                if self.on_changes_key(&mut v, k) {
                    self.view = Some(View::Changes(v));
                }
            }
            View::Files(mut v) => {
                if self.on_files_tree_key(&mut v, k) {
                    self.view = Some(View::Files(v));
                }
            }
            View::Both(mut v) => {
                if self.on_both_key(&mut v, k) {
                    self.view = Some(View::Both(v));
                }
            }
            View::Check(mut v) => {
                if self.on_check_key(&mut v, k) {
                    self.view = Some(View::Check(v));
                }
            }
            View::Branch(mut v) => {
                if self.on_branch_sheet_key(&mut v, k) {
                    self.view = Some(View::Branch(v));
                }
            }
            View::References(mut v) => {
                if self.on_references_key(&mut v, k) {
                    self.view = Some(View::References(v));
                }
            }
        }
    }


    /// Returns false when the view closes.
    pub(super) fn on_changes_key(&mut self, v: &mut views::ChangesView, k: &KeyEvent) -> bool {
        let c = match k.code {
            KeyCode::Enter => '\n',
            KeyCode::Char(c) => c,
            KeyCode::Esc => '\x1b',
            _ => '\0',
        };
        if let Some((_, action)) = v.confirm.take() {
            if (c == 'y' || c == '\n')
                && let Some(r) = &v.review {
                    let task = r.task.clone();
                    match action {
                        'm' => {
                            let dir = task.dir.clone();
                            self.spawn_bg(move || Bg::Merged(tasks::merge(&task), dir));
                        }
                        'c' => self.spawn_bg(move || Bg::Done(tasks::commit(&task), true)),
                        'p' => self.spawn_bg(move || Bg::Done(tasks::pull_request(&task), true)),
                        'd' => {
                            let d = v.dir.clone();
                            self.throw_away(&d);
                            return false;
                        }
                        _ => {}
                    }
                    self.notify("working on it…".into(), false);
                }
            return true;
        }
        let order = v.order();
        let dir_key = design::path_key(&v.dir);
        let Some(r) = v.review.as_mut() else { return c != '\x1b' };
        let pos = order.iter().position(|i| *i == r.sel).unwrap_or(0);
        let go = |r: &mut tasks::Review, to: usize| {
            if let Some(i) = order.get(to) {
                r.sel = *i;
                r.diff_sel = None;
                r.scroll = 0;
            }
        };
        match k.code {
            KeyCode::Esc => return false,
            KeyCode::Down | KeyCode::Char('j') => go(r, (pos + 1).min(order.len().saturating_sub(1))),
            KeyCode::Up | KeyCode::Char('k') => go(r, pos.saturating_sub(1)),
            // Mark reviewed (or not): it sinks, and the next one to look at comes up.
            KeyCode::Char('x') => {
                let Some(file) = r.files.get(r.sel).map(|f| f.path.clone()) else { return true };
                let marks = self.screen.saved.reviewed.entry(dir_key).or_default();
                let now = !v.reviewed.contains(&file);
                if now {
                    marks.insert(file.clone(), views::fingerprint(&v.dir, &file));
                    v.reviewed.insert(file.clone());
                } else {
                    marks.remove(&file);
                    v.reviewed.remove(&file);
                }
                self.screen.save();
                let order = v.order();
                if let Some(r) = v.review.as_mut() {
                    let next = if now { order.iter().copied().find(|i| !v.reviewed.contains(&r.files[*i].path)) } else { None };
                    if let Some(i) = next.or_else(|| order.iter().copied().find(|i| r.files[*i].path == file)) {
                        r.sel = i;
                        r.diff_sel = None;
                        r.scroll = 0;
                    }
                    if now && v.reviewed.len() == r.files.len() {
                        self.notify("all reviewed".into(), false);
                    }
                }
            }
            KeyCode::PageDown | KeyCode::Char(' ') => r.scroll = r.scroll.saturating_add(15),
            KeyCode::PageUp => r.scroll = r.scroll.saturating_sub(15),
            // Asked first: `c` is also Changes' leader key, shown in the tab strip.
            KeyCode::Char('c') => {
                let n = r.files.len();
                v.confirm = Some((format!("Commit {n} file{} on {}?", if n == 1 { "" } else { "s" }, r.task.branch), 'c'));
            }
            KeyCode::Char('e') => {
                if let Some(f) = r.files.get(r.sel) {
                    let p = v.dir.join(&f.path);
                    self.open_in_editor(&p);
                }
            }
            KeyCode::Char('m') if v.linked => v.confirm = Some((format!("Merge {} into {}, then close it and remove its worktree and branch?", r.task.branch, r.task.base), 'm')),
            KeyCode::Char('p') => v.confirm = Some((format!("Push {} and open a pull request?", r.task.branch), 'p')),
            KeyCode::Char('d') if v.linked => v.confirm = Some((format!("Throw away {} (folder and branch)?", r.task.branch), 'd')),
            KeyCode::Char('r') => {
                if let Some(term) = v.term {
                    self.cmd(Command::FocusPane { term });
                    self.mode = Mode::Normal;
                    return false;
                }
                self.notify("no agent runs in this worktree".into(), true);
            }
            _ => {}
        }
        true
    }

    /// Returns false when the view closes.
    pub(super) fn on_files_tree_key(&mut self, v: &mut views::FilesTree, k: &KeyEvent) -> bool {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let page = self.screen.preview_rect.height.saturating_sub(4).max(5) as usize;
        // Editing the file in the preview.
        if let Some(ed) = v.edit.as_mut() {
            if matches!(k.code, KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End) {
                ed.select(k.modifiers.contains(KeyModifiers::SHIFT));
            }
            match k.code {
                KeyCode::Char('a') if ctrl => ed.select_all(),
                KeyCode::Char('c') if ctrl => {
                    let text = ed.copied();
                    copy::to_clipboard(&text);
                    self.notify(copied_note("copied", &text), false);
                }
                KeyCode::Char('x') if ctrl => {
                    if let Some(text) = ed.cut() {
                        copy::to_clipboard(&text);
                        self.notify(copied_note("cut", &text), false);
                    }
                }
                KeyCode::Char('v') if ctrl => self.paste_clipboard_into_editor(ed.path.clone()),
                KeyCode::Char('z') if ctrl => {
                    if !ed.undo() {
                        self.notify("nothing to undo".into(), false);
                    }
                }
                KeyCode::Char('y') if ctrl => {
                    if !ed.redo() {
                        self.notify("nothing to redo".into(), false);
                    }
                }
                KeyCode::Char('s') if ctrl => match ed.save() {
                    Ok(views::Saved::Done) => self.notify(format!("saved {}", ed.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()), false),
                    Ok(views::Saved::ChangedOnDisk) => {
                        self.notify("the file changed since you opened it (an agent?): Ctrl+S again overwrites it, Esc keeps theirs".into(), true)
                    }
                    Err(e) => self.notify(e, true),
                },
                KeyCode::Esc if ed.dirty && !ed.warned => {
                    ed.warned = true;
                    self.notify("unsaved changes: Ctrl+S saves, Esc again throws them away".into(), true);
                }
                KeyCode::Esc => {
                    v.edit = None;
                    // Show what's on disk now.
                    v.preview = None;
                    v.refresh_preview();
                    return true;
                }
                KeyCode::Up => ed.go(-1, 0),
                KeyCode::Down => ed.go(1, 0),
                KeyCode::Left => ed.go(0, -1),
                KeyCode::Right => ed.go(0, 1),
                KeyCode::PageUp => ed.go(-(page as isize), 0),
                KeyCode::PageDown => ed.go(page as isize, 0),
                KeyCode::Home => ed.home(),
                KeyCode::End => ed.end(),
                KeyCode::Enter => ed.newline(),
                KeyCode::Backspace => ed.backspace(),
                KeyCode::Delete => ed.delete(),
                KeyCode::Tab => {
                    for _ in 0..4 {
                        ed.insert(' ');
                    }
                }
                KeyCode::Char(c) if !ctrl => {
                    ed.warned = false;
                    ed.insert(c);
                }
                _ => {}
            }
            v.follow_cursor(page);
            return true;
        }
        if v.filtering {
            match k.code {
                KeyCode::Esc => {
                    v.filtering = false;
                    v.filter.clear();
                }
                KeyCode::Enter => v.filtering = false,
                KeyCode::Backspace => {
                    v.filter.pop();
                }
                KeyCode::Char(c) if !ctrl => v.filter.push(c),
                KeyCode::Down | KeyCode::Up => {
                    let n = v.visible().len();
                    Self::list_move(&mut v.sel, n, k);
                }
                _ => {}
            }
            v.sel = v.sel.min(v.visible().len().saturating_sub(1));
            v.refresh_preview();
            return true;
        }
        if ctrl && k.code == KeyCode::Char('c') {
            match v.picked_text() {
                Some(text) => {
                    copy::to_clipboard(&text);
                    self.notify(copied_note("copied", &text), false);
                }
                None => self.notify("drag over the file to select text; y copies its path".into(), false),
            }
            return true;
        }
        let node = v.selected();
        let lines = v.shown_lines(self.screen.preview_rect.width, &self.theme, &self.cfg.theme);
        // In the preview: the arrows read the file; ← (or Esc) goes back to the tree.
        if v.in_preview {
            match k.code {
                KeyCode::Left | KeyCode::Esc | KeyCode::Char('h') => v.in_preview = false,
                KeyCode::Down | KeyCode::Char('j') => v.scroll = (v.scroll + 1).min(lines.saturating_sub(1)),
                KeyCode::Up | KeyCode::Char('k') => v.scroll = v.scroll.saturating_sub(1),
                KeyCode::PageDown | KeyCode::Char(' ') => v.scroll = (v.scroll + page).min(lines.saturating_sub(1)),
                KeyCode::PageUp => v.scroll = v.scroll.saturating_sub(page),
                KeyCode::Home | KeyCode::Char('g') => v.scroll = 0,
                KeyCode::End | KeyCode::Char('G') => v.scroll = lines.saturating_sub(page),
                // Everything else (i, e, y, o, Enter) works as from the tree.
                _ => {
                    v.in_preview = false;
                    let keep = self.on_files_tree_key(v, k);
                    // Still the file once an edit opened here ends; the filter (`/`) and the
                    // recent list (`r`) show the tree, so they leave the file.
                    if !matches!(k.code, KeyCode::Enter | KeyCode::Char('r' | '/')) {
                        v.in_preview = true;
                    }
                    return keep;
                }
            }
            return true;
        }
        match k.code {
            KeyCode::Right | KeyCode::Char('l') if node.as_ref().is_some_and(|n| !n.is_dir) && lines > 0 => v.in_preview = true,
            KeyCode::Esc | KeyCode::Char('f') => return false,
            // Read the preview: page, or a line at a time with Shift.
            KeyCode::PageDown => v.scroll = (v.scroll + page).min(lines.saturating_sub(1)),
            KeyCode::PageUp => v.scroll = v.scroll.saturating_sub(page),
            KeyCode::Down if k.modifiers.contains(KeyModifiers::SHIFT) => v.scroll = (v.scroll + 1).min(lines.saturating_sub(1)),
            KeyCode::Up if k.modifiers.contains(KeyModifiers::SHIFT) => v.scroll = v.scroll.saturating_sub(1),
            // Edit it right here.
            KeyCode::Char('i') => {
                if let Some(n) = node.as_ref().filter(|n| !n.is_dir) {
                    match views::Edit::open(&n.path) {
                        Ok(mut ed) => {
                            // A drawn line is no line of the text: start at the top.
                            if v.drawn() {
                                v.scroll = 0;
                            }
                            ed.row = v.scroll.min(ed.lines.len().saturating_sub(1));
                            v.edit = Some(ed);
                        }
                        Err(e) => self.notify(e, true),
                    }
                }
            }
            KeyCode::Char('m') => {
                v.toggle_raw();
                // Keep about the same place in the file: the same share of the way down.
                let after = v.shown_lines(self.screen.preview_rect.width, &self.theme, &self.cfg.theme);
                v.scroll = (v.scroll * after).checked_div(lines).unwrap_or(0);
            }
            KeyCode::Char('/') => {
                v.filtering = true;
                v.sel = 0;
            }
            KeyCode::Char('r') => {
                v.recent = !v.recent;
                v.sel = 0;
                if v.recent && v.recent_list.is_none() {
                    let r = v.root.clone();
                    self.spawn_bg(move || Bg::TreeRecent(r.clone(), files::scan_recent(&r)));
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(n) = node.filter(|n| n.is_dir) {
                    v.expanded.insert(n.path);
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(n) = node {
                    if n.is_dir && v.expanded.remove(&n.path) {
                    } else if let Some(parent) = n.path.parent().map(|p| p.to_path_buf()) {
                        v.expanded.remove(&parent);
                        if let Some(i) = v.visible().iter().position(|(_, x)| x.path == parent) {
                            v.sel = i;
                        }
                    }
                }
            }
            KeyCode::Enter => match node {
                Some(n) if n.is_dir => {
                    if !v.expanded.remove(&n.path) {
                        v.expanded.insert(n.path);
                    }
                }
                Some(n) => {
                    if let Some(term) = self.focused() {
                        self.send(ClientMsg::Input { term, data: format!("{} ", files::quote_path(&n.path)).into_bytes() });
                        self.notify(format!("inserted {}", n.path.display()), false);
                    }
                    return false;
                }
                None => {}
            },
            KeyCode::Char('o') => {
                if let Some(n) = node {
                    let name = n.path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                    self.open_outside(n.path.clone(), name);
                }
            }
            KeyCode::Char('e') => {
                if let Some(n) = node.filter(|n| !n.is_dir) {
                    self.open_in_editor(&n.path);
                    return self.view.is_some();
                }
            }
            KeyCode::Char('y') => {
                if let Some(n) = node {
                    copy::to_clipboard(&n.path.display().to_string());
                    self.notify(format!("copied {}", n.path.display()), false);
                }
            }
            KeyCode::Char('j') => {
                let n = v.visible().len();
                v.sel = (v.sel + 1).min(n.saturating_sub(1));
            }
            KeyCode::Char('k') => v.sel = v.sel.saturating_sub(1),
            _ => {
                let n = v.visible().len();
                Self::list_move(&mut v.sel, n, k);
            }
        }
        v.refresh_preview();
        true
    }



    /// Ctrl+V in Files' editor: the clipboard is read in the background, and its text lands at
    /// the cursor if that file is still being edited.
    fn paste_clipboard_into_editor(&mut self, path: std::path::PathBuf) {
        let read = copy::clipboard_reader();
        self.spawn_bg(move || {
            let got = read();
            Bg::Then(Box::new(move |app: &mut App| match got {
                Ok(text) => app.paste_into_editor(&path, &text),
                Err(e) => app.notify(e, true),
            }))
        });
    }

    /// A paste while Files has the keys (and nothing is over it) stays in Files: the editor
    /// takes it as text, the filter its first line. Otherwise it is dropped with a toast: the
    /// pane behind is not what you were looking at, and an empty paste would start an image
    /// paste there. False when Files doesn't have the keys.
    pub(super) fn paste_into_files(&mut self, text: &str) -> bool {
        if !self.view_has_keys() || !matches!(self.mode, Mode::Normal) {
            return false;
        }
        let Some(View::Files(v)) = &mut self.view else { return false };
        let editing = v.edit.as_ref().map(|ed| ed.path.clone());
        match editing {
            Some(path) => self.paste_into_editor(&path, text),
            None if v.filtering => {
                v.filter.push_str(text.lines().next().unwrap_or(""));
                v.sel = v.sel.min(v.visible().len().saturating_sub(1));
                v.refresh_preview();
            }
            None => self.notify("Files has the keys: / filters, i edits; Esc closes it, or click a pane to paste there".into(), false),
        }
        true
    }

    /// Text pasted into Files' editor for `path`, if that file is still open there.
    pub(super) fn paste_into_editor(&mut self, path: &std::path::Path, text: &str) {
        if text.is_empty() {
            // A terminal's own paste of an image arrives empty; it is no text to insert.
            self.notify(copy::NO_TEXT.into(), true);
            return;
        }
        let page = self.screen.preview_rect.height.max(1) as usize;
        let Some(View::Files(v)) = &mut self.view else { return };
        let Some(ed) = v.edit.as_mut().filter(|ed| ed.path == path) else { return };
        ed.insert_text(text);
        v.follow_cursor(page);
        self.dirty = true;
    }

    /// Keys in the settings view. Returns false when it closes.
    pub(super) fn on_settings_view_key(&mut self, v: &mut design::SettingsView, k: &KeyEvent) -> bool {
        use modal::{Cat, Kind};
        let cat = Cat::ALL[v.cat.min(Cat::ALL.len() - 1)];
        let rows = design::settings_rows(cat);
        let row = rows.get(v.sel).cloned();
        // Waiting for a key: the new leader, or a new key for a shortcut.
        if v.capturing {
            v.capturing = false;
            if k.code == KeyCode::Esc {
                return true;
            }
            let spec = KeySpec::from_event(k);
            match row {
                Some(design::SRow::Setting(s)) => self.save_setting(s.path, spec.to_config().into()),
                Some(design::SRow::Bind { acts, .. }) if acts.len() == 1 => {
                    let act = &acts[0];
                    let Some(name) = act.to_config() else { return true };
                    let new = spec.to_config();
                    // Unbind the old keys, then bind the new one.
                    let old: Vec<String> = self
                        .keymap
                        .prefixed_order
                        .iter()
                        .filter(|key| self.keymap.prefixed.get(key) == Some(act))
                        .map(|key| key.to_config())
                        .filter(|key| *key != new)
                        .collect();
                    let mut result = Ok(());
                    for o in old {
                        result = result.and_then(|_| modal::write_at(&["keys", "prefix", &o], "none".into()));
                    }
                    result = result.and_then(|_| modal::write_at(&["keys", "prefix", &new], name.into()));
                    match result {
                        Ok(()) => {
                            self.reload_config();
                            self.notify(format!("{} is now {} {}", act.describe(), self.keymap.prefix, spec), false);
                        }
                        Err(e) => self.notify(format!("{e:#}"), true),
                    }
                }
                _ => {}
            }
            return true;
        }
        if let Some(mut text) = v.editing.take() {
            match k.code {
                KeyCode::Esc => {}
                KeyCode::Enter => {
                    if let Some(design::SRow::Setting(s)) = row {
                        self.save_setting(s.path, text.trim().into());
                    }
                }
                KeyCode::Backspace => {
                    text.pop();
                    v.editing = Some(text);
                }
                KeyCode::Char(c) => {
                    text.push(c);
                    v.editing = Some(text);
                }
                _ => v.editing = Some(text),
            }
            return true;
        }
        if matches!(&row, Some(design::SRow::Setting(s)) if s.path == "theme")
            && matches!(k.code, KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right | KeyCode::Char('h' | 'l'))
        {
            self.open_themes();
            return true;
        }
        if matches!(&row, Some(design::SRow::Setting(s)) if s.kind == Kind::Spinner)
            && matches!(k.code, KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right | KeyCode::Char('h' | 'l'))
        {
            self.open_spinners();
            return true;
        }
        let step = |app: &mut App, dir: i64| {
            if let Some(design::SRow::Setting(s)) = &row
                && let Some(val) = modal::step(&app.cfg, s, dir)
            {
                app.save_setting(s.path, val);
            }
        };
        match k.code {
            KeyCode::Esc => return false,
            KeyCode::Tab => {
                v.cat = (v.cat + 1) % Cat::ALL.len();
                v.sel = 0;
            }
            KeyCode::BackTab => {
                v.cat = (v.cat + Cat::ALL.len() - 1) % Cat::ALL.len();
                v.sel = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => v.sel = (v.sel + 1).min(rows.len().saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') => v.sel = v.sel.saturating_sub(1),
            KeyCode::Left | KeyCode::Char('h') => step(self, -1),
            KeyCode::Right | KeyCode::Char('l') => step(self, 1),
            KeyCode::Char('o') => {
                self.open_outside(crate::config::config_path(), "config.toml".into());
            }
            KeyCode::Char('u') => self.act(Action::CheckUpdate),
            KeyCode::Char('R') => self.ask_restart(),
            KeyCode::Enter | KeyCode::Char(' ') => match &row {
                Some(design::SRow::Setting(s)) => match s.kind {
                    // A folder: pick it in the folder browser (Esc comes back here).
                    Kind::Folder => {
                        let cur = self.cfg.start_dir().or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())).unwrap_or_default();
                        let mut fd = screen::Finder::new(&cur);
                        fd.for_setting = Some(s.path);
                        self.mode = Mode::Finder(Box::new(fd));
                        return true;
                    }
                    Kind::Text | Kind::Program(_) => {
                        let cur = modal::current(&self.cfg, s.path).and_then(|x| x.as_str().map(str::to_string)).unwrap_or_default();
                        v.editing = Some(cur);
                    }
                    Kind::Key => v.capturing = true,
                    _ => step(self, 1),
                },
                Some(design::SRow::Bind { acts, .. }) if acts.len() == 1 => v.capturing = true,
                Some(design::SRow::Bind { .. }) => self.notify("that row has several keys: change them in the config file (o)".into(), false),
                // Claude and Codex rows only report; the last one opens the list of agents.
                Some(design::SRow::Agent(design::AgentRow::SetUp)) => self.act(Action::AgentStatusSetup),
                // Version: install what was found, else look again.
                Some(design::SRow::Version) if self.update_available.is_some() => self.act(Action::Update),
                Some(design::SRow::Version) => self.act(Action::CheckUpdate),
                Some(design::SRow::Agent(_)) | None => {}
            },
            _ => {}
        }
        true
    }

    /// Generic list movement shared by the panels. Returns true if the key was handled.
    pub(super) fn list_move(sel: &mut usize, len: usize, k: &KeyEvent) -> bool {
        let last = len.saturating_sub(1);
        match k.code {
            KeyCode::Down => *sel = (*sel + 1).min(last),
            KeyCode::Up => *sel = sel.saturating_sub(1),
            KeyCode::PageDown => *sel = (*sel + 10).min(last),
            KeyCode::PageUp => *sel = sel.saturating_sub(10),
            KeyCode::Home => *sel = 0,
            KeyCode::End => *sel = last,
            _ => return false,
        }
        true
    }




    pub(super) fn submit_prompt(&mut self, kind: PromptKind, input: String) {
        let input = input.trim().to_string();
        match kind {
            PromptKind::RenamePane(term) => self.cmd(Command::RenamePane { term, name: input }),
            PromptKind::RenameProject(key) => {
                if input.is_empty() {
                    self.screen.saved.names.remove(&key);
                } else {
                    self.screen.saved.names.insert(key, input);
                }
                self.screen.save();
            }
            PromptKind::RenameTab(ws, tab) => self.cmd(Command::RenameTab { ws, tab, name: input }),
            PromptKind::AddReference(root, dir) => self.add_reference(root, dir, &input),
            PromptKind::NewWorkspace => {
                // A group starts with a shell where you are.
                let name = (!input.is_empty()).then_some(input);
                self.leave_shortcut_mode();
                self.cmd(Command::NewWorkspace { cwd: Some(self.here_dir()), name, cmd: None });
            }
            _ => {}
        }
    }

    /// Rows for the worktree picker: matching worktrees, then "create <query>" when the
    /// query isn't already an existing branch.
    pub(super) fn worktree_rows(&self, items: Option<&[WorktreeEntry]>, query: &str) -> Vec<WtRow> {
        let q = query.trim().to_lowercase();
        let items = items.unwrap_or_default();
        let mut rows: Vec<WtRow> = items
            .iter()
            .filter(|w| q.is_empty() || w.branch.to_lowercase().contains(&q) || w.path.to_string_lossy().to_lowercase().contains(&q))
            .cloned()
            .map(WtRow::Existing)
            .collect();
        let first = q.split_whitespace().next().unwrap_or("");
        if !first.is_empty() && !items.iter().any(|w| w.branch.to_lowercase() == first) {
            rows.push(WtRow::Create(query.trim().to_string()));
        }
        rows
    }

    /// Switch to the workspace already open on this worktree, or open one.
    pub(super) fn open_worktree(&mut self, w: &WorktreeEntry, cmd: Option<String>) {
        self.leave_shortcut_mode();
        if let Some(open) = self.snap.workspaces.iter().find(|x| same_dir(&x.cwd, &w.path)) {
            self.cmd(Command::SelectWorkspace { ws: open.id });
            if let Some(cmd) = cmd {
                self.cmd(Command::NewTab { ws: open.id, name: None, cmd: Some(cmd) });
            }
            return;
        }
        let name = if w.main { w.repo.clone() } else { format!("{}:{}", w.repo, w.branch) };
        self.cmd(Command::NewWorkspace { cwd: Some(w.path.clone()), name: Some(name), cmd });
    }
}

/// The toast for text taken from Files: how much, not the text itself.
fn copied_note(verb: &str, text: &str) -> String {
    let n = text.chars().count();
    format!("{verb} {n} character{}", if n == 1 { "" } else { "s" })
}
