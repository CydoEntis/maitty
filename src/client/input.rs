//! Keys, pastes and the mouse.

use super::*;

/// One line this long, arriving in a single burst, was pasted: nobody types it that fast.
const PASTE_MIN_CHARS: usize = 16;

/// How many keys went down in `events`.
pub(super) fn presses(events: &[Event]) -> usize {
    events.iter().filter(|ev| matches!(ev, Event::Key(k) if k.kind != KeyEventKind::Release)).count()
}

/// A burst of keys read at once that is really a paste: plain text with a line break inside
/// it (Enter followed by more text), or one long line. Windows hands a terminal paste over
/// this way, each new line an Enter; sent as keys, the first one would send the message and
/// the rest would crawl in a letter at a time. Typing ahead that ends in Enter isn't a
/// paste, and stays keys.
pub(super) fn paste_from_burst(events: &[Event]) -> Option<String> {
    let mut text = String::new();
    let mut broken_line = false;
    for ev in events {
        let Event::Key(k) = ev else { return None };
        if k.kind == KeyEventKind::Release {
            continue;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        match k.code {
            // AltGr arrives as Ctrl+Alt; it types a character.
            KeyCode::Char(c) if !(ctrl || alt) || (ctrl && alt && !c.is_ascii_alphabetic()) => {
                broken_line |= text.ends_with('\n');
                text.push(c);
            }
            KeyCode::Enter if !(ctrl || alt) => text.push('\n'),
            KeyCode::Tab if k.modifiers.is_empty() => text.push('\t'),
            _ => return None,
        }
    }
    let long_line = text.chars().count() >= PASTE_MIN_CHARS && !text.ends_with('\n');
    (broken_line || long_line).then_some(text)
}

impl App {
    pub(super) fn on_event(&mut self, ev: Event) {
        self.dirty = true;
        match ev {
            Event::Key(k) if k.kind != KeyEventKind::Release => self.on_key(k),
            Event::Paste(s) => self.on_paste(s),
            Event::FocusGained => self.window_focused = true,
            Event::FocusLost => self.window_focused = false,
            Event::Mouse(m) => self.on_mouse(m),
            _ => {}
        }
    }

    pub(super) fn on_paste(&mut self, s: String) {
        match &mut self.mode {
            Mode::Copy(c) => {
                if let Some(input) = &mut c.input {
                    input.push_str(s.lines().next().unwrap_or(""));
                }
            }
            Mode::ScreenPane(np) => np.task.push_str(&s.replace("\r\n", "\n")),
            Mode::Prompt { input, .. } | Mode::Worktrees { query: input, .. } => {
                input.push_str(s.lines().next().unwrap_or(""));
            }
            // The app's own text boxes take the paste, not the pane behind them.
            Mode::Find(v) => v.query.push_str(s.lines().next().unwrap_or("")),
            Mode::GoTo { query, .. } => query.push_str(s.lines().next().unwrap_or("").trim()),
            Mode::KeyMap(km) if km.view == screen::KeyView::Search && km.step.is_none() => {
                themes::append_query(&mut km.query, s.lines().next().unwrap_or(""));
                km.sel = 0;
            }
            Mode::Compose(c) => c.text.push_str(&s.replace("\r\n", "\n")),
            Mode::Confirm(c) if matches!(c.act, menu::Act::OverrideCheck(..)) => {
                if let menu::Act::OverrideCheck(_, reason, _) = &mut c.act {
                    screen::append_reason(reason, &s);
                }
            }
            // A pasted "2" must not count as a choice: the approval takes no paste.
            Mode::Approve(_) => {}
            Mode::ProjectCheck(d) => screen::check_settings::paste_into_draft(d, &s),
            Mode::ScreenSettings(_) if self.screen.check_page.as_ref().is_some_and(|d| d.editing) => {
                if let Some(d) = &mut self.screen.check_page {
                    screen::check_settings::paste_into_draft(d, &s);
                }
            }
            Mode::Themes { query, sel, saving: false } => {
                themes::append_query(query, s.lines().next().unwrap_or(""));
                *sel = 0;
            }
            Mode::Themes { saving: true, .. } => {}
            Mode::Branch(v) => v.query.push_str(s.lines().next().unwrap_or("").trim()),
            Mode::Finder(fd) => {
                fd.q.push_str(s.lines().next().unwrap_or("").trim());
                fd.sel = 0;
                fd.refresh();
            }
            Mode::ScreenSettings(v) if v.editing.is_some() => {
                if let Some(e) = &mut v.editing {
                    e.push_str(s.lines().next().unwrap_or(""));
                }
            }
            _ if s.is_empty() => self.paste_image(),
            _ => {
                let Some(term) = self.typing_to() else { return };
                let bracketed = self.parsers.get(&term).is_some_and(|p| p.screen().bracketed_paste());
                let body = s.replace("\r\n", "\r").replace('\n', "\r");
                let data = if bracketed { format!("\x1b[200~{body}\x1b[201~") } else { body };
                if let Some(p) = self.parsers.get_mut(&term) {
                    p.screen_mut().set_scrollback(0);
                }
                self.send(ClientMsg::Input { term, data: data.into_bytes() });
            }
        }
    }

    pub(super) fn on_key(&mut self, k: KeyEvent) {
        self.handle_key(k);
        self.keep_shortcut_mode();
    }

    /// Shortcut mode stays until you leave it (ADR 0016, ADR 0021): an action that opened a
    /// popup hands it the keys, and when the popup closes the mode is back.
    fn keep_shortcut_mode(&mut self) {
        match self.mode {
            Mode::Prefix { .. } => self.shortcut_held = true,
            // A view still open has the keys; the mode comes back once it closes.
            Mode::Normal if self.shortcut_held && self.view.is_none() => self.mode = Mode::Prefix { since: Instant::now(), acted: true },
            _ => {}
        }
    }

    /// Which kind of view is open, to tell whether an action opened one.
    fn view_kind(&self) -> Option<std::mem::Discriminant<View>> {
        self.view.as_ref().map(std::mem::discriminant)
    }

    fn handle_key(&mut self, k: KeyEvent) {
        self.screen_fresh();
        let spec = KeySpec::from_event(&k);
        if k.code == KeyCode::Esc && !matches!(self.mode, Mode::Copy(_)) {
            self.dismiss_notice();
        }
        if self.splash {
            self.on_hy_splash_key(&k);
            return;
        }
        if let Mode::Copy(c) = &mut self.mode {
            // Tab / Shift+Tab: keep this pane's selection and go on to the next pane of the
            // split, the same search there; y copies them all.
            if matches!(k.code, KeyCode::Tab | KeyCode::BackTab) && c.input.is_none() {
                let (term, query) = (c.term, c.query.clone());
                let picked = c.anchor.is_some().then(|| c.selected_text()).filter(|s| !s.trim().is_empty());
                let panes: Vec<TermId> = self.screen.tabs.get(self.screen.tab).map(|t| t.layout.leaves()).unwrap_or_default();
                if panes.len() > 1 && let Some(i) = panes.iter().position(|p| *p == term) {
                    if let Some(text) = picked {
                        self.copy_set.retain(|(p, _)| *p != term);
                        self.copy_set.push((term, text));
                    }
                    let n = panes.len();
                    let next = panes[if k.code == KeyCode::Tab { (i + 1) % n } else { (i + n - 1) % n }];
                    if self.enter_copy(next)
                        && let Mode::Copy(c) = &mut self.mode
                    {
                        c.query = query;
                        c.search_next();
                        c.message = Some(format!("{} kept · Tab next pane · y copy all", self.copy_set.len()));
                    }
                }
                return;
            }
            let term = c.term;
            match c.key(&k) {
                copy::Outcome::Stay => {}
                copy::Outcome::Exit => {
                    self.copy_set.clear();
                    self.mode = Mode::Normal;
                }
                copy::Outcome::Yank(text) => {
                    self.copy_term = Some(term);
                    self.yank(text);
                }
            }
            return;
        }
        match self.mode.clone() {
            Mode::Normal => {
                if spec == self.keymap.prefix {
                    self.mode = Mode::Prefix { since: Instant::now(), acted: false };
                } else if let Some(a) = self.keymap.global.get(&spec).cloned() {
                    self.act(a);
                } else if self.view.is_some() {
                    self.on_view_key(&k);
                } else if self.scroll_key(&k) {
                } else {
                    self.forward_key(&k);
                }
            }
            Mode::Finder(fd) => self.on_finder_key(*fd, &k),
            Mode::ScreenPane(np) => self.on_hy_pane_key(np, &k),
            Mode::NewSession(ns) => self.on_new_session_key(*ns, &k),
            Mode::ScreenSettings(_) => self.screen_settings_key(&k),
            Mode::Themes { query, sel, saving } => self.on_theme_key(query, sel, saving, &k),
            Mode::Spinners(_) => self.on_spinner_key(&k),
            Mode::Side => self.on_side_key(&k),
            Mode::ScreenMenu(m) => self.on_hy_menu_key(*m, &k),
            Mode::Find(v) => self.on_find_key(*v, &k),
            Mode::Branch(v) => self.on_branch_key(*v, &k),
            Mode::History { sel } => self.on_history_key(sel, &k),
            Mode::Tidy(v) => self.on_tidy_key(*v, &k),
            Mode::GoTo { query, sel } => self.on_goto_key(query, sel, &k),
            Mode::Compose(c) => self.on_compose_key(*c, &k),
            Mode::Why { term, side } => self.on_why_key(term, side, &k),
            Mode::Helpers { term, side } => self.on_helpers_key(term, side, &k),
            Mode::Confirm(c) if matches!(c.act, crate::client::menu::Act::OverrideCheck(..)) => self.on_override_key(*c, &k),
            Mode::Approve(a) => self.on_approve_key(*a, &k),
            Mode::ProjectCheck(d) => self.on_project_check_key(*d, &k),
            Mode::Confirm(c) => match k.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.mode = Mode::Normal;
                    self.confirm_done(Some(c.act));
                }
                KeyCode::Char(ch) if ch == c.key => {
                    self.mode = Mode::Normal;
                    self.confirm_done(Some(c.act));
                }
                // Closing: x (as you asked to close) or Delete says yes too.
                KeyCode::Char('x') | KeyCode::Delete if matches!(c.act, crate::client::menu::Act::End(_) | crate::client::menu::Act::CloseProject(_)) => {
                    self.mode = Mode::Normal;
                    self.confirm_done(Some(c.act));
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                    self.mode = Mode::Normal;
                    self.confirm_done(None);
                }
                _ => self.mode = Mode::Confirm(c),
            },
            Mode::Prefix { since, acted } => {
                // Esc or the leader again: back to typing.
                if k.code == KeyCode::Esc || spec == self.keymap.prefix {
                    self.mode = Mode::Normal;
                    self.shortcut_held = false;
                    return;
                }
                if let Some(a) = self.keymap.prefixed.get(&spec).cloned() {
                    let before = self.view_kind();
                    self.mode = Mode::Normal;
                    self.act(a);
                    // A view it opened (Changes, Files, a check) takes the keys, as a popup
                    // does: its own k, arrows and Esc must not run leader actions. Closing it
                    // returns to shortcut mode (`keep_shortcut_mode`).
                    let opened = self.view.is_some() && self.view_kind() != before;
                    // Nothing opened: the mode holds, the key map stays away.
                    if self.mode == Mode::Normal && !opened {
                        self.mode = Mode::Prefix { since, acted: true };
                    }
                } else if !acted
                    && self.keymap_shown(since)
                    && let KeyCode::Char(c) = k.code
                {
                    // The key map is up and this key does nothing: it starts a search.
                    self.mode = Mode::KeyMap(Box::new(screen::KeyMap { query: c.to_string(), ..screen::KeyMap::new(screen::KeyView::Search, None) }));
                }
            }
            Mode::KeyMap(km) => self.on_keymap_key(*km, &k),
            Mode::Actions { sel } => self.on_actions_key(sel, &k),
            Mode::RenameTab(nt) => self.on_rename_tab_key(*nt, &k),
            Mode::Worktrees { ws, cmd, items, mut query, mut sel } => {
                let rows = self.worktree_rows(items.as_deref(), &query);
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                match k.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        return;
                    }
                    KeyCode::Enter => {
                        self.mode = Mode::Normal;
                        match rows.get(sel) {
                            Some(WtRow::Existing(w)) => self.open_worktree(w, cmd),
                            Some(WtRow::Create(text)) => {
                                let mut words = text.split_whitespace();
                                if let Some(branch) = words.next().map(str::to_string) {
                                    let base = words.next().map(str::to_string);
                                    self.notify(format!("creating worktree {branch}…"), false);
                                    self.cmd(Command::NewWorktree { ws, branch, base, cmd, split: None, from: None });
                                }
                            }
                            None => {}
                        }
                        return;
                    }
                    KeyCode::Down | KeyCode::Tab => sel = (sel + 1).min(rows.len().saturating_sub(1)),
                    KeyCode::Char('n') if ctrl => sel = (sel + 1).min(rows.len().saturating_sub(1)),
                    KeyCode::Up | KeyCode::BackTab => sel = sel.saturating_sub(1),
                    KeyCode::Char('p') if ctrl => sel = sel.saturating_sub(1),
                    KeyCode::Backspace => {
                        query.pop();
                        sel = 0;
                    }
                    KeyCode::Char(c) if !ctrl => {
                        query.push(c);
                        sel = 0;
                    }
                    _ => {}
                }
                self.mode = Mode::Worktrees { ws, cmd, items, query, sel };
            }
            Mode::Prompt { kind, mut input } => {
                if kind.is_confirm() {
                    self.mode = Mode::Normal;
                    let yes = matches!(k.code, KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter);
                    let force = matches!(k.code, KeyCode::Char('f') | KeyCode::Char('F'));
                    match kind {
                        PromptKind::ConfirmCloseWorkspace(ws) if yes => self.cmd(Command::CloseWorkspace { ws }),
                        PromptKind::ConfirmKillServer if yes => self.cmd(Command::KillServer { forget: false }),
                        PromptKind::ConfirmRemoveWorktree(ws) if yes || force => {
                            self.notify("removing worktree…".into(), false);
                            self.cmd(Command::RemoveWorktree { ws, force, delete_branch: false });
                        }
                        _ => {}
                    }
                    return;
                }
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Enter => {
                        self.mode = Mode::Normal;
                        self.submit_prompt(kind, input);
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        self.mode = Mode::Prompt { kind, input };
                    }
                    KeyCode::Char('u') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.mode = Mode::Prompt { kind, input: String::new() };
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        self.mode = Mode::Prompt { kind, input };
                    }
                    _ => self.mode = Mode::Prompt { kind, input },
                }
            }
            Mode::Copy(_) => unreachable!("handled above"),
        }
    }

    pub(super) fn forward_key(&mut self, k: &KeyEvent) {
        let Some(term) = self.typing_to() else { return };
        let app_cursor = self.parsers.get(&term).is_some_and(|p| p.screen().application_cursor());
        let win32 = cfg!(windows) && self.snap.terms.get(&term).is_some_and(|t| t.win32_input) && keys::wants_win32(k);
        let data = match win32.then(|| keys::encode_win32(k)).flatten() {
            Some(d) => d,
            None => keys::encode(k, app_cursor),
        };
        if data.is_empty() {
            return;
        }
        if let Some(p) = self.parsers.get_mut(&term) {
            p.screen_mut().set_scrollback(0);
        }
        // Typing goes to the prompt at the bottom; the agent scrolls there itself.
        self.paged_up.remove(&term);
        self.send(ClientMsg::Input { term, data });
    }

    /// Shift+PageUp / PageDown (and Shift+Up / Down a line) scroll the history, like any
    /// terminal; full-screen programs get the keys themselves.
    pub(super) fn scroll_key(&mut self, k: &KeyEvent) -> bool {
        let Some(term) = self.focused() else { return false };
        let Some(p) = self.parsers.get(&term) else { return false };
        if p.screen().alternate_screen() {
            return false;
        }
        let agent = self.snap.terms.get(&term).is_some_and(|t| t.agent.is_some());
        let has_history = self.history(term).1 > 0;
        if agent && !has_history && matches!(k.code, KeyCode::PageUp | KeyCode::PageDown) {
            return false;
        }
        let Some(p) = self.parsers.get(&term) else { return false };
        let s = p.screen();
        let at_prompt = s.mouse_protocol_mode() == vt100::MouseProtocolMode::None && (!s.application_cursor() || s.bracketed_paste());
        if k.modifiers.is_empty() && at_prompt && matches!(k.code, KeyCode::PageUp | KeyCode::PageDown) {
            let page = s.size().0.saturating_sub(1).max(1) as i32;
            self.scroll_by(term, if k.code == KeyCode::PageUp { page } else { -page });
            return true;
        }
        if !k.modifiers.contains(KeyModifiers::SHIFT) {
            return false;
        }
        let page = (p.screen().size().0 / 2).max(1) as i32;
        let d = match k.code {
            KeyCode::PageUp => page,
            KeyCode::PageDown => -page,
            KeyCode::Up if k.modifiers.contains(KeyModifiers::CONTROL) => 1,
            KeyCode::Down if k.modifiers.contains(KeyModifiers::CONTROL) => -1,
            _ => return false,
        };
        self.scroll_by(term, d);
        true
    }

    /// The pane under `pos` and the cell inside it.
    pub(super) fn pane_cell(&self, pos: Position) -> Option<(TermId, u16, u16)> {
        let (term, r) = self.panes.iter().find(|(_, r)| r.contains(pos))?;
        Some((*term, pos.y - r.y, pos.x - r.x))
    }

    pub(super) fn on_mouse(&mut self, m: MouseEvent) {
        let pos = Position::new(m.column, m.row);
        // A click leaves shortcut mode for good, not just the popup it lands in.
        if matches!(m.kind, MouseEventKind::Down(_)) {
            self.shortcut_held = false;
        }
        self.screen_fresh();
        // Ctrl+click opens a link; double-click copies a word (or a path, or a link).
        if m.kind == MouseEventKind::Down(MouseButton::Left)
            && !self.splash
            && matches!(self.mode, Mode::Normal)
            && self.view.is_none()
            && let Some((term, row, col)) = self.pane_cell(pos)
        {
            let screen_has_mouse = self.parsers.get(&term).is_some_and(|p| p.screen().mouse_protocol_mode() != vt100::MouseProtocolMode::None);
            if m.modifiers.contains(KeyModifiers::CONTROL)
                && let Some(url) = self.parsers.get(&term).and_then(|p| pick::url_at(p.screen(), row, col))
            {
                self.open_link(url);
                return;
            }
            if m.modifiers.contains(KeyModifiers::CONTROL)
                && let Some((p, line)) = self.parsers.get(&term).and_then(|p| pick::path_at(p.screen(), row, col))
            {
                self.open_path_from(term, &p, line);
                return;
            }
            let double = self.pane_click.is_some_and(|(p, at)| at.elapsed() < DOUBLE_CLICK && p.y == pos.y && p.x.abs_diff(pos.x) <= 1);
            self.pane_click = Some((pos, Instant::now()));
            if double
                && !screen_has_mouse
                && let Some(word) = self.parsers.get(&term).and_then(|p| pick::word_at(p.screen(), row, col))
            {
                self.pane_click = None;
                self.drag = None;
                copy::to_clipboard(&word);
                self.notify(format!("copied {}", render::truncate(&word, 60)), false);
                return;
            }
        }
        // Programs that ask for the mouse (Claude Code's full-screen view, vim, lazygit,
        // htop, …) get it, like in any terminal: clicks, wheel, drags. Shift keeps it for
        // the app (select text); right-click stays the app's menu.
        if self.screen.drag.is_none()
            && !self.splash
            && matches!(self.mode, Mode::Normal)
            && self.view.is_none()
            && !m.modifiers.contains(KeyModifiers::SHIFT)
            && let Some((term, inner)) = self.panes.iter().find(|(_, r)| r.contains(pos)).copied()
            && (self.screen.right_clicks.contains(&term)
                || !matches!(m.kind, MouseEventKind::Down(MouseButton::Right) | MouseEventKind::Up(MouseButton::Right) | MouseEventKind::Drag(MouseButton::Right)))
            // Full-screen programs own their scrolling. On a normal screen, use the app history
            // while it exists; pass the wheel to a mouse-aware program when that history is empty.
            && (!matches!(m.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown)
                || self.parsers.get(&term).is_some_and(|p| p.screen().alternate_screen())
                || self.history(term).1 == 0)
            && let Some(bytes) = self.parsers.get(&term).and_then(|p| keys::mouse_bytes(p.screen(), &m, pos.x - inner.x, pos.y - inner.y))
        {
            if m.kind == MouseEventKind::Moved && self.hover != Some(pos) {
                self.hover = Some(pos);
                self.dirty = true;
            }
            if matches!(m.kind, MouseEventKind::Down(_)) {
                if Some(term) != self.focused() {
                    self.cmd(Command::FocusPane { term });
                }
                // Clicking for the program brings its view back to the bottom.
                if let Some(p) = self.parsers.get_mut(&term) {
                    p.screen_mut().set_scrollback(0);
                }
            }
            if !bytes.is_empty() {
                self.send(ClientMsg::Input { term, data: bytes });
            }
            return;
        }
        // Dragging the sidebar edge or the split divider.
        if let Some(d) = self.screen.drag {
            match m.kind {
                MouseEventKind::Drag(MouseButton::Left) => {
                    match d {
                        screen::Drag::Side => {
                            let side = self.screen.side_rect;
                            let w = if self.cfg.ui.sidebar_position == "right" { side.right().saturating_sub(m.column) } else { m.column.saturating_sub(side.x) };
                            self.screen.saved.side_w = Some(w.clamp(screen::SIDE_MIN, screen::SIDE_MAX));
                        }
                        screen::Drag::Sheet => {
                            let right = self.screen.sheet_rect.right();
                            self.set_sheet_w(right.saturating_sub(m.column));
                        }
                        screen::Drag::Scroll(term) => {
                            if let Some((t, r, total)) = self.screen.bar
                                && t == term
                            {
                                let from_bottom = r.bottom().saturating_sub(m.row + 1) as usize;
                                let v = (from_bottom * total) / r.height.max(1) as usize;
                                self.scroll_to(term, v.min(total));
                            }
                        }
                        screen::Drag::Session(t, _, press) => {
                            let over = self
                                .hits
                                .iter()
                                .rev()
                                .find(|(r, h)| r.contains(pos) && matches!(h, Hit::Screen(screen::ScreenHit::Session(_) | screen::ScreenHit::ToggleProj(_))))
                                .map(|(_, h)| *h);
                            let moved = match over {
                                // Onto another session: into its group, at its place.
                                Some(Hit::Screen(screen::ScreenHit::Session(u))) if u != t => {
                                    let model = self.screen_model();
                                    let into = model.iter().position(|p| p.sessions().any(|s| s.term == u));
                                    (into.is_some_and(|pi| self.place_session(t, pi))) | self.move_session(t, u)
                                }
                                // Onto a group's name: into that group.
                                Some(Hit::Screen(screen::ScreenHit::ToggleProj(pi))) => self.place_session(t, pi),
                                _ => false,
                            };
                            if moved {
                                self.screen.drag = Some(screen::Drag::Session(t, true, press));
                            }
                        }
                        screen::Drag::Group(pi, _) => {
                            let over = self.hits.iter().rev().find(|(r, h)| r.contains(pos) && matches!(h, Hit::Screen(screen::ScreenHit::ToggleProj(_)))).map(|(_, h)| *h);
                            if let Some(Hit::Screen(screen::ScreenHit::ToggleProj(to))) = over
                                && let Some(from) = self.screen.drag_key.clone()
                                && let Some(target) = self.screen.proj_keys.get(to).cloned()
                                && target != from
                            {
                                // Every group in today's order, then this one moved to the
                                // other's place.
                                let mut order: Vec<String> = self.screen.proj_keys.clone();
                                order.retain(|k| *k != from);
                                let at = order.iter().position(|k| *k == target).unwrap_or(order.len());
                                let at = if to > pi { at + 1 } else { at };
                                order.insert(at.min(order.len()), from);
                                // Groups not on screen keep their place after these.
                                let rest: Vec<String> = self.screen.saved.order.iter().filter(|k| !order.contains(k)).cloned().collect();
                                order.extend(rest);
                                self.screen.saved.order = order;
                                self.screen.drag = Some(screen::Drag::Group(to, true));
                                self.screen_fresh();
                            }
                        }
                        screen::Drag::Tab(i) => {
                            let over = self.hits.iter().rev().find(|(r, h)| r.contains(pos) && matches!(h, Hit::Screen(screen::ScreenHit::TabPick(_)))).map(|(_, h)| *h);
                            if let Some(Hit::Screen(screen::ScreenHit::TabPick(j))) = over
                                && j != i
                            {
                                let now = self.move_tab(i, j);
                                self.screen.drag = Some(screen::Drag::Tab(now));
                            }
                        }
                        screen::Drag::Divider(i) => {
                            if let Some((r, horizontal, path)) = self.screen.dividers.get(i).cloned() {
                                // The line sits in the gap after the left card (as wide as the
                                // gap setting): put it under the pointer.
                                let gap = screen::Look::of(&self.cfg.ui).gap.max(1);
                                let f = if horizontal {
                                    (m.column + gap).saturating_sub(r.x) as f32 / r.width.max(1) as f32
                                } else {
                                    (m.row + 1).saturating_sub(r.y) as f32 / r.height.max(1) as f32
                                };
                                let tab = self.screen.tab;
                                if let Some(tab) = self.screen.tabs.get_mut(tab) {
                                    tab.layout.set_ratio(&path, f);
                                }
                            }
                        }
                    }
                    self.dirty = true;
                    return;
                }
                MouseEventKind::Up(_) => {
                    // A session pressed and let go where it was: open it, or its Changes when
                    // the press was on its `+N −N`.
                    if let screen::Drag::Session(_, false, press) = d {
                        self.screen.drag = None;
                        self.on_hy_hit(press, false);
                        self.dirty = true;
                        return;
                    }
                    // A group header pressed and let go where it was: fold or open it.
                    if let screen::Drag::Group(pi, false) = d {
                        self.screen.drag = None;
                        self.screen.drag_key = None;
                        self.on_hy_hit(screen::ScreenHit::ToggleProj(pi), false);
                        self.dirty = true;
                        return;
                    }
                    self.screen.drag = None;
                    self.screen.drag_key = None;
                    self.screen.save();
                    self.dirty = true;
                    return;
                }
                _ => {}
            }
        }
        // A middle click on a tab closes it, as in a browser.
        if m.kind == MouseEventKind::Down(MouseButton::Middle)
            && let Some(Hit::Screen(screen::ScreenHit::TabPick(i))) = self.hits.iter().rev().find(|(r, _)| r.contains(pos)).map(|(_, h)| *h)
        {
            self.close_tab(i);
            self.dirty = true;
            return;
        }
        // Some terminals only report the release of a right click: open on whichever comes
        // first.
        let right_click = match m.kind {
            MouseEventKind::Down(MouseButton::Right) => {
                self.right_down = Some(Instant::now());
                true
            }
            MouseEventKind::Up(MouseButton::Right) => !self.right_down.take().is_some_and(|at| at.elapsed() < Duration::from_millis(600)),
            _ => false,
        };
        if right_click && !self.splash && !matches!(self.mode, Mode::ScreenMenu(_) | Mode::Confirm(_)) {
            let hit = self.hits.iter().rev().find(|(r, _)| r.contains(pos)).map(|(_, h)| *h);
            let at = (m.column, m.row);
            match hit {
                Some(Hit::Screen(screen::ScreenHit::Session(t) | screen::ScreenHit::RowChanges(t))) => self.menu_for_session(t, at),
                Some(Hit::Screen(screen::ScreenHit::Why(t))) if self.screen.side_rect.contains(pos) => self.menu_for_session(t, at),
                Some(Hit::Screen(screen::ScreenHit::ToggleProj(pi))) => self.menu_for_project(pi, at),
                _ => {
                    if let Some((term, _)) = self.pane_frames.iter().find(|(_, r)| r.contains(pos)).copied() {
                        self.menu_for_pane(term, at);
                    }
                }
            }
            self.dirty = true;
            return;
        }
        if m.kind == MouseEventKind::Moved {
            // Moving with no button held: a drag whose release never came (let go outside the
            // window) is over. Left in place it would keep the mouse from the panes.
            if self.screen.drag.is_some() {
                self.screen.drag = None;
                self.screen.drag_key = None;
                self.screen.save();
            }
            if self.hover != Some(pos) {
                self.hover = Some(pos);
                self.dirty = true;
            }
            self.pointer_for(pos);
            return;
        }
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            let hit = self.hits.iter().rev().find(|(r, _)| r.contains(pos)).map(|(_, h)| *h);
            // A session's row: it may be dragged within its group; a plain click opens it
            // (on release).
            if let Some(Hit::Screen(press @ (screen::ScreenHit::Session(t) | screen::ScreenHit::RowChanges(t)))) = hit {
                self.screen.drag = Some(screen::Drag::Session(t, false, press));
                self.dirty = true;
                return;
            }
            // A group's header: it may be dragged to a new place; a plain click folds it (on
            // release).
            if let Some(Hit::Screen(screen::ScreenHit::ToggleProj(pi))) = hit {
                self.screen.drag = Some(screen::Drag::Group(pi, false));
                self.screen.drag_key = self.screen.proj_keys.get(pi).cloned();
                self.dirty = true;
                return;
            }
            if let Some(h @ Hit::Screen(hh)) = hit {
                // A tab's pill may be dragged to another tab's place.
                if let screen::ScreenHit::TabPick(i) = hh {
                    self.screen.drag = Some(screen::Drag::Tab(i));
                }
                let double = self.last_click.is_some_and(|(prev, at)| prev == h && at.elapsed() < DOUBLE_CLICK);
                self.last_click = Some((h, Instant::now()));
                if matches!(self.mode, Mode::Prefix { .. } | Mode::Side) && !matches!(hh, screen::ScreenHit::Why(_)) {
                    self.mode = Mode::Normal;
                }
                self.on_hy_hit(hh, double);
                self.dirty = true;
                return;
            }
            // The splash waits for one of its buttons.
            if self.splash {
                return;
            }
            if let Some(h @ Hit::Button(_)) = hit {
                let double = self.last_click.is_some_and(|(prev, at)| prev == h && at.elapsed() < DOUBLE_CLICK);
                self.last_click = Some((h, Instant::now()));
                if matches!(self.mode, Mode::Prefix { .. } | Mode::KeyMap(_) | Mode::Actions { .. }) {
                    self.mode = Mode::Normal;
                }
                if self.on_button(h, double) {
                    self.dirty = true;
                    return;
                }
            }
        }
        if self.on_mouse_select(&m, pos) {
            return;
        }
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if !matches!(self.mode, Mode::Normal) {
                    self.mode = Mode::Normal;
                    return;
                }
                if let Some((term, _, _)) = self.pane_cell(pos) {
                    self.drag = Some((term, pos));
                }
                let hit = self.hits.iter().find(|(r, _)| r.contains(pos)).map(|(_, h)| *h);
                match hit {
                    Some(Hit::Pane(term)) => self.cmd(Command::FocusPane { term }),
                    Some(Hit::Button(_) | Hit::Screen(_)) => {}
                    None => {
                        if let Some((term, _)) = self.pane_frames.iter().find(|(_, r)| r.contains(pos))
                            && Some(*term) != self.focused() {
                                self.cmd(Command::FocusPane { term: *term });
                            }
                    }
                }
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let up = m.kind == MouseEventKind::ScrollUp;
                if matches!(self.mode, Mode::KeyMap(_) | Mode::Actions { .. }) {
                    self.on_key(KeyEvent::new(if up { KeyCode::Up } else { KeyCode::Down }, KeyModifiers::NONE));
                    self.dirty = true;
                    return;
                }
                let sheet_hit = self.hits.iter().rev().find(|(r, _)| r.contains(pos)).map(|(_, h)| *h);
                if matches!(self.mode, Mode::ScreenSettings(_))
                    && matches!(sheet_hit, Some(Hit::Screen(screen::ScreenHit::SettingsBody | screen::ScreenHit::SetRow(_) | screen::ScreenHit::SetVal(_, _) | screen::ScreenHit::SetTab(_))))
                {
                    self.screen_settings_key(&KeyEvent::new(if up { KeyCode::Up } else { KeyCode::Down }, KeyModifiers::NONE));
                    self.dirty = true;
                    return;
                }
                if let Mode::Themes { query, sel, saving } = self.mode.clone() {
                    self.on_theme_key(query, sel, saving, &KeyEvent::new(if up { KeyCode::Up } else { KeyCode::Down }, KeyModifiers::NONE));
                    return;
                }
                if matches!(self.mode, Mode::Spinners(_)) {
                    self.on_spinner_key(&KeyEvent::new(if up { KeyCode::Up } else { KeyCode::Down }, KeyModifiers::NONE));
                    return;
                }
                if self.screen.preview_rect.contains(pos)
                    && let Some(View::Files(v)) = &mut self.view
                {
                    let n = v.edit.as_ref().map(|e| e.lines.len()).or_else(|| v.preview.as_ref().map(|(_, l)| l.len())).unwrap_or(0);
                    v.scroll = if up { v.scroll.saturating_sub(3) } else { (v.scroll + 3).min(n.saturating_sub(1)) };
                    self.dirty = true;
                    return;
                }
                if self.screen.side_rect.contains(pos) {
                    self.screen.side_scroll = if up { self.screen.side_scroll.saturating_sub(3) } else { self.screen.side_scroll + 3 };
                    self.dirty = true;
                    return;
                }
                let Some((term, _)) = self.pane_frames.iter().find(|(_, r)| r.contains(pos)).copied() else {
                    return;
                };
                let alt = self.parsers.get(&term).is_some_and(|p| p.screen().alternate_screen());
                let agent = self.snap.terms.get(&term).is_some_and(|t| t.agent.is_some());
                let no_history = self.history(term).1 == 0;
                if (alt || no_history) && agent && !m.modifiers.contains(KeyModifiers::SHIFT) {
                    // A full-screen view, or a normal-screen agent with no the app history:
                    // arrow keys would walk its prompt history, so page it at most every 150 ms.
                    if self.wheel_page.is_none_or(|at| at.elapsed() >= Duration::from_millis(150)) {
                        self.wheel_page = Some(Instant::now());
                        let paged = self.paged_up.entry(term).or_default();
                        *paged = if up { *paged + 1 } else { paged.saturating_sub(1) };
                        if *paged == 0 {
                            self.paged_up.remove(&term);
                        }
                        let key: &[u8] = if up { b"\x1b[5~" } else { b"\x1b[6~" };
                        self.send(ClientMsg::Input { term, data: key.to_vec() });
                    }
                } else if alt {
                    // Full-screen programs scroll themselves: one arrow key per notch.
                    let app_cursor = self.parsers.get(&term).is_some_and(|p| p.screen().application_cursor());
                    let key: &[u8] = match (up, app_cursor) {
                        (true, true) => b"\x1bOA",
                        (true, false) => b"\x1b[A",
                        (false, true) => b"\x1bOB",
                        (false, false) => b"\x1b[B",
                    };
                    self.send(ClientMsg::Input { term, data: key.to_vec() });
                } else {
                    self.scroll_by(term, if up { 3 } else { -3 });
                }
            }
            _ => {}
        }
    }

    /// Put session `t` where `u` is, within their group (the order you dragged them to).
    /// False when they aren't in the same group.
    /// Put session `t` in group `pi` (by the sidebar's groups). Not onto another machine's group.
    pub(super) fn place_session(&mut self, t: TermId, pi: usize) -> bool {
        let model = self.screen_model();
        let Some(to) = model.get(pi) else { return false };
        let Some(from) = model.iter().find(|p| p.sessions().any(|s| s.term == t)) else { return false };
        if from.key == to.key || from.kind != to.kind || to.kind == screen::Kind::Ssh {
            return false;
        }
        let placed = &mut self.screen.saved.placed;
        placed.retain(|(x, ..)| *x != t);
        placed.push((t, to.path.clone(), to.git));
        let alive: Vec<TermId> = self.snap.terms.keys().copied().collect();
        placed.retain(|(x, ..)| alive.contains(x));
        self.screen_fresh();
        self.dirty = true;
        true
    }

    pub(super) fn move_session(&mut self, t: TermId, u: TermId) -> bool {
        let model = self.screen_model();
        let Some(group) = model.iter().find(|p| p.sessions().any(|s| s.term == t)) else { return false };
        if !group.sessions().any(|s| s.term == u) {
            return false;
        }
        // The group's sessions as shown, then t moved to u's place.
        let lines = screen::side_lines(self, &model, &self.theme);
        let mut ids: Vec<TermId> = lines.iter().filter_map(|l| screen::line_term(&model, l)).filter(|id| group.sessions().any(|s| s.term == *id)).collect();
        let (from, to) = (ids.iter().position(|x| *x == t), ids.iter().position(|x| *x == u));
        let (Some(from), Some(to)) = (from, to) else { return false };
        let id = ids.remove(from);
        ids.insert(to, id);
        let order = &mut self.screen.saved.session_order;
        order.retain(|x| !ids.contains(x));
        order.extend(ids);
        // Only sessions that still exist.
        let alive: Vec<TermId> = self.snap.terms.keys().copied().collect();
        order.retain(|x| alive.contains(x));
        self.screen_fresh();
        self.dirty = true;
        true
    }

    /// The mouse pointer for what's under it: ↔ / ↕ over a line you can drag (in terminals
    /// that take OSC 22: Ghostty, kitty, foot, WezTerm; others keep their arrow).
    pub(super) fn pointer_for(&mut self, pos: Position) {
        let want = match self.hits.iter().rev().find(|(r, _)| r.contains(pos)).map(|(_, h)| *h) {
            Some(Hit::Screen(screen::ScreenHit::Divider(i))) => match self.screen.dividers.get(i) {
                Some((_, true, _)) => "col-resize",
                Some(_) => "row-resize",
                None => "default",
            },
            Some(Hit::Screen(screen::ScreenHit::SideEdge | screen::ScreenHit::SheetEdge)) => "col-resize",
            _ => "default",
        };
        if self.pointer != want {
            self.pointer = want;
            use std::io::Write;
            let _ = write!(std::io::stdout(), "\x1b]22;{want}\x1b\\");
            let _ = std::io::stdout().flush();
        }
    }

    /// Drag to select (enters copy mode), release to copy; clicks and wheel inside copy mode.
    /// Returns true when the event was consumed.
    pub(super) fn on_mouse_select(&mut self, m: &MouseEvent, pos: Position) -> bool {
        match m.kind {
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Mode::Copy(c) = &mut self.mode {
                    if let Some((r_term, r)) = self.panes.iter().find(|(t, _)| *t == c.term) {
                        let _ = r_term;
                        // Dragging past the edge scrolls.
                        if pos.y < r.y {
                            c.scroll(-1);
                        } else if pos.y >= r.bottom() {
                            c.scroll(1);
                        }
                        let row = pos.y.min(r.bottom().saturating_sub(1)).max(r.y) - r.y;
                        let col = pos.x.min(r.right().saturating_sub(1)).max(r.x) - r.x;
                        let at = c.at_cell(row, col);
                        if c.anchor.is_none() {
                            c.anchor = Some(c.cur);
                        }
                        c.move_to(at);
                    }
                    return true;
                }
                let Some((term, start)) = self.drag else { return false };
                if start == pos || !matches!(self.mode, Mode::Normal) || !self.enter_copy(term) {
                    return false;
                }
                let Some((_, r)) = self.panes.iter().find(|(t, _)| *t == term).copied() else { return true };
                if let Mode::Copy(c) = &mut self.mode {
                    c.mouse = true;
                    let s = c.at_cell(start.y.saturating_sub(r.y), start.x.saturating_sub(r.x));
                    c.cur = s;
                    c.anchor = Some(s);
                    let row = pos.y.min(r.bottom().saturating_sub(1)).max(r.y) - r.y;
                    let col = pos.x.min(r.right().saturating_sub(1)).max(r.x) - r.x;
                    let at = c.at_cell(row, col);
                    c.move_to(at);
                }
                true
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag = None;
                if let Mode::Copy(c) = &self.mode
                    && c.mouse
                {
                    let text = c.selected_text();
                    self.yank(text);
                    return true;
                }
                false
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let Mode::Copy(c) = &mut self.mode else { return false };
                let Some((_, r)) = self.panes.iter().find(|(t, _)| *t == c.term).copied() else { return false };
                if !r.contains(pos) {
                    self.mode = Mode::Normal;
                    return true;
                }
                let at = c.at_cell(pos.y - r.y, pos.x - r.x);
                c.anchor = None;
                c.move_to(at);
                true
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let Mode::Copy(c) = &mut self.mode else { return false };
                c.scroll(if m.kind == MouseEventKind::ScrollUp { -3 } else { 3 });
                true
            }
            _ => false,
        }
    }

    pub(super) fn scroll_by(&mut self, term: TermId, delta: i32) {
        let Some(p) = self.parsers.get_mut(&term) else { return };
        // The parser's own offset: vt100 raises it as output arrives, to keep the view still.
        let cur = p.screen().scrollback() as i32;
        p.screen_mut().set_scrollback((cur + delta).max(0) as usize);
    }

    /// Clicks on chips and buttons. Returns true if handled.
    pub(super) fn on_button(&mut self, hit: Hit, double: bool) -> bool {
        match hit {
            Hit::Button(b) => match b {
                Btn::Answer(term, c) => self.answer(term, c),
                Btn::ViewKey(c) => {
                    let code = match c {
                        '\n' => KeyCode::Enter,
                        c => KeyCode::Char(c),
                    };
                    self.on_view_key(&KeyEvent::new(code, KeyModifiers::NONE));
                }
                Btn::Row(i) => match &mut self.view {
                    Some(View::Changes(v)) => {
                        let rows = v.rows();
                        if let (Some(views::ChangesRow::File(fi, _)), Some(r)) = (rows.get(i), v.review.as_mut()) {
                            r.sel = *fi;
                            r.diff_sel = None;
                            r.scroll = 0;
                        }
                    }
                    Some(View::Files(v)) => {
                        v.sel = i;
                        v.refresh_preview();
                        if double {
                            self.on_view_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                        }
                    }
                    Some(View::Both(_) | View::Check(_)) | None => {}
                },
            },
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::paste_from_burst;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn keys(s: &str) -> Vec<Event> {
        s.chars()
            .flat_map(|c| {
                let code = match c {
                    '\n' => KeyCode::Enter,
                    c => KeyCode::Char(c),
                };
                let press = KeyEvent::new(code, KeyModifiers::NONE);
                let release = KeyEvent { kind: KeyEventKind::Release, ..press };
                [Event::Key(press), Event::Key(release)]
            })
            .collect()
    }

    #[test]
    fn a_pasted_line_break_is_text_not_send() {
        assert_eq!(paste_from_burst(&keys("first\nsecond")).as_deref(), Some("first\nsecond"));
        assert_eq!(paste_from_burst(&keys("one\ntwo\n")).as_deref(), Some("one\ntwo\n"), "a trailing line break stays in the paste");
        assert_eq!(paste_from_burst(&keys("yes\n")), None, "typed ahead and sent: still keys");
        assert_eq!(paste_from_burst(&keys("abc")), None, "a few letters: keys do the same");
        assert_eq!(paste_from_burst(&keys("https://example.com/a/long/link")).as_deref(), Some("https://example.com/a/long/link"), "one long line at once: pasted whole");
        assert_eq!(paste_from_burst(&keys("cargo test --workspace\n")), None, "a long line typed ahead and sent: still keys");
        assert_eq!(super::presses(&keys("ab")), 2, "releases don't count");
        let mut with_ctrl = keys("a\nb");
        with_ctrl.push(Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
        assert_eq!(paste_from_burst(&with_ctrl), None, "a shortcut in it: keys");
    }
}
