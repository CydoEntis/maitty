//! The Menu (the common commands, each run by the letter lit in its name) and tabs (new,
//! renamed in their pill). The key map, search and All keys are in `keyoverlay`.

use super::*;

/// How long a tab's typed name may get.
const NAME_MAX: usize = 40;

/// A Menu row: its name, the letter that runs it (lit in the name, so it is one of the
/// name's own letters and no two rows share one) and what it does.
pub(in crate::client) type MenuItem = (&'static str, char, Action);

/// The Menu's groups, a blank row apart, in the handoff's order. Labels are lowercase so the one
/// capital you might read as a key never competes with the lit letter. "shell here" starts a plain
/// shell (what `n` does); "session" starts an agent straight away (`s`).
pub(in crate::client) fn menu_groups() -> Vec<Vec<MenuItem>> {
    vec![
        vec![
            ("shell here", 'h', Action::ShellHere),
            ("session", 's', Action::StartSession),
            ("session with options…", 'w', Action::NewPane),
            ("pane beside this", 'p', Action::SplitRight),
            ("tab", 't', Action::NewTab),
        ],
        vec![("inbox", 'i', Action::Jump), ("search commands", 'c', Action::Palette), ("all keys", 'k', Action::Help)],
        vec![("collapse sidebar", 'a', Action::ToggleSidebar), ("hide sidebar", 'd', Action::HideSidebar)],
        vec![("settings", 'e', Action::Settings)],
    ]
}

/// The Menu's rows in the order they are numbered.
pub(in crate::client) fn action_items() -> Vec<MenuItem> {
    menu_groups().into_iter().flatten().collect()
}

/// The label a Menu row shows now: the sidebar's rows say what the next press does.
fn menu_label(app: &App, a: &Action, label: &'static str) -> &'static str {
    match a {
        Action::ToggleSidebar if !app.sidebar => "expand sidebar",
        Action::HideSidebar if app.side_hidden => "show sidebar",
        _ => label,
    }
}

/// The Menu: a card at the bottom left, over the sidebar's foot. Groups a blank row apart;
/// each row is run by the letter lit in its name.
pub(in crate::client) fn draw_actions(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, sel: usize) {
    let look = Look::of(&app.cfg.ui);
    let buf = f.buffer_mut();
    dim_all(buf, area, t);
    hit(app, area, ScreenHit::Close);
    let groups = menu_groups();
    let lines: u16 = groups.iter().map(|items| items.len() as u16 + 1).sum();
    let w = 46.min(area.width.saturating_sub(4));
    let h = (lines + 5).min(area.height.saturating_sub(2));
    let side = grid(app, area).side;
    let x = (if side.width > 0 { side.x + 1 } else { area.x + 3 }).min(area.right().saturating_sub(w));
    let y = area.bottom().saturating_sub(4 + h).max(area.y);
    let r = Rect { x, y, width: w, height: h };
    let mut c = Card::new(t, "Menu").lit(t.accent);
    c.bg = t.card;
    let inside = card(app, buf, r, &c, t);
    hit(app, r, ScreenHit::Noop);
    let needs = app.screen_model().iter().flat_map(|p| p.sessions()).filter(|s| s.status == Status::Blocked).count();
    let mut yy = r.y + 2;
    let mut i = 0usize;
    for (g, items) in groups.iter().enumerate() {
        if yy >= inside.bottom().saturating_sub(2) {
            break;
        }
        if g > 0 {
            yy += 1;
        }
        for (label, letter, a) in items {
            if yy >= inside.bottom().saturating_sub(2) {
                i += 1;
                continue;
            }
            let rr = Rect { x: r.x + 2, y: yy, width: w.saturating_sub(4), height: 1 };
            let on = i == sel || hovered(app, rr);
            if on {
                row_pill(look, buf, rr.x, yy, rr.width, t.sel_on(t.card), t.card);
            }
            let bg = if on { t.sel_on(t.card) } else { t.card };
            let st = Style::default().bg(bg);
            let mut ls = st.fg(if on { t.strong } else { t.text });
            if on {
                ls = ls.add_modifier(Modifier::BOLD);
            }
            let label = menu_label(app, a, label);
            // The leader key that does the same, at the right: the lit letter only works in here.
            let bound = crate::client::design::key_text(app, std::slice::from_ref(a));
            let keys = bound.split("  ").next().filter(|k| !k.is_empty()).map(|k| crate::client::design::keycap(t, &k.replace(' ', ""))).unwrap_or_default();
            let kw = segs_width(&keys);
            if kw > 0 {
                put(buf, rr.right().saturating_sub(kw + 1), yy, &keys, rr.right());
            }
            let room = r.right().saturating_sub(6 + if kw > 0 { kw + 1 } else { 0 });
            let end = put(buf, r.x + 4, yy, &lit_letter(label, Some(*letter), ls, st.fg(t.accent).add_modifier(Modifier::BOLD)), room);
            if *a == Action::Jump && needs > 0 {
                put(buf, end + 1, yy, &[seg(format!("● {needs}"), st.fg(t.blocked).add_modifier(Modifier::BOLD))], r.right().saturating_sub(6));
            } else if *a == Action::Settings && app.restart_pending() {
                put(buf, end + 1, yy, &[seg("●", st.fg(t.accent).add_modifier(Modifier::BOLD))], r.right().saturating_sub(6));
            }
            hit(app, rr, ScreenHit::ActionRow(i));
            yy += 1;
            i += 1;
        }
    }
    let lead = app.keymap.prefix.to_string().replace("C-", "Ctrl+");
    let cs = Style::default().bg(t.card);
    let mut foot = crate::client::design::keycap(t, &lead);
    foot.push(seg(" then a key works anywhere", cs.fg(t.muted)));
    put(buf, r.x + 3, inside.bottom().saturating_sub(1), &foot, r.right().saturating_sub(2));
}

/// `label` with its first `letter` (any case) in `lit`, the rest in `plain`; all of it
/// in `plain` when it has none.
pub(in crate::client) fn lit_letter(label: &str, letter: Option<char>, plain: Style, lit: Style) -> Vec<Seg> {
    let Some((at, c)) = letter.and_then(|l| label.char_indices().find(|(_, c)| c.eq_ignore_ascii_case(&l))) else {
        return vec![seg(label, plain)];
    };
    let end = at + c.len_utf8();
    [(&label[..at], plain), (&label[at..end], lit), (&label[end..], plain)].into_iter().filter(|(s, _)| !s.is_empty()).map(|(s, st)| seg(s, st)).collect()
}

impl App {
    pub(in crate::client) fn on_actions_key(&mut self, sel: usize, k: &KeyEvent) {
        let items = action_items();
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Up => self.mode = Mode::Actions { sel: sel.saturating_sub(1) },
            KeyCode::Down => self.mode = Mode::Actions { sel: (sel + 1).min(items.len() - 1) },
            KeyCode::Enter => self.actions_run(sel),
            KeyCode::Char(c) if !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                if let Some(i) = items.iter().position(|(_, letter, _)| *letter == c.to_ascii_lowercase()) {
                    self.actions_run(i);
                }
            }
            _ => {}
        }
    }

    pub(in crate::client) fn actions_run(&mut self, i: usize) {
        if let Some((_, _, a)) = action_items().into_iter().nth(i) {
            self.mode = Mode::Normal;
            self.act(a);
        }
    }

    /// A new tab of the session you're on: a shell where you are (cd and run what you like;
    /// the sidebar groups it by where it works).
    pub(in crate::client) fn new_tab_start(&mut self) {
        let Some(owner) = self.screen.tabs.get(self.screen.tab).map(|t| t.owner).or(self.focused()) else {
            self.notify("open a session first".into(), true);
            return;
        };
        self.screen.new_tab = Some((Instant::now(), owner));
        self.screen_act(&Action::ShellHere);
    }

    /// Rename the tab you're on, in its pill.
    pub(in crate::client) fn rename_tab_start(&mut self) {
        let i = self.screen.tab;
        if let Some(tab) = self.screen.tabs.get(i) {
            let name = tab.name.clone();
            self.mode = Mode::RenameTab(Box::new(TabName { tab: i, name }));
        }
    }

    /// Typing a tab's name: Enter keeps it (empty: named after what runs in it), Esc doesn't.
    pub(in crate::client) fn on_rename_tab_key(&mut self, mut nt: TabName, k: &KeyEvent) {
        match k.code {
            KeyCode::Enter => {
                let name = nt.name.trim().to_string();
                if let Some(tab) = self.screen.tabs.get_mut(nt.tab) {
                    let focus = tab.focus;
                    // The daemon owns the name: it outlives this view's tab being rebuilt, and
                    // `screen_sync` reads it back from the snapshot.
                    match self.snap.workspaces.iter().find_map(|w| w.tabs.iter().find(|t| t.layout.contains(focus)).map(|t| (w.id, t.id))) {
                        Some((ws, tab)) => self.cmd(Command::RenameTab { ws, tab, name }),
                        None => tab.name = name,
                    }
                }
                self.last_click = None;
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Esc => {
                self.last_click = None;
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Backspace => {
                nt.name.pop();
            }
            KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) && nt.name.chars().count() < NAME_MAX => nt.name.push(c),
            _ => {}
        }
        self.mode = Mode::RenameTab(Box::new(nt));
    }
}
