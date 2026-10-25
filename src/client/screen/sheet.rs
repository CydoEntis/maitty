//! The sheet: one card docked right of the panes for what you look at beside your work (the
//! Inbox, Changes, Files). The panes reflow into the rest; below `NARROW` it takes the whole pane
//! column. One at a time; Esc closes it. Its last row is a pill of key hints.

use super::*;
use super::inbox::{draw_inbox, inbox_count, inbox_place};
use crate::client::overlap::Overlap;

/// What the sheet shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum SheetKind {
    Inbox,
    Changes,
    /// Both diffs of a heads-up's file.
    Both,
    /// A check's output and where it came from.
    Check,
    /// The project's files beside the one you picked.
    Files,
    /// Where a checkout stands: its branch, commits, check and snapshots.
    Branch,
    /// The project's reference images and design files.
    References,
}

/// Its width beside the panes, before you size it (Changes takes half the screen).
pub(super) const SHEET_W: u16 = 72;
/// The narrowest it gets.
pub(in crate::client) const SHEET_MIN: u16 = 32;
/// The columns the panes keep beside it; with less, it takes the whole pane column.
pub(in crate::client) const PANES_KEEP: u16 = 52;
/// How far `[` and `]` move its edge.
const SHEET_STEP: u16 = 4;
/// The percent of the screen Files takes before you size it: the tree and a readable file.
const FILES_SHARE: u32 = 55;
/// Narrower than this, Files shows the tree or the open file, not both side by side.
const FILES_BOTH_FROM: u16 = 60;

impl SheetKind {
    /// Its name in the saved UI state, where its width is kept.
    fn key(self) -> &'static str {
        match self {
            SheetKind::Inbox => "inbox",
            SheetKind::Changes => "changes",
            SheetKind::Both => "both",
            SheetKind::Check => "check",
            SheetKind::Files => "files",
            SheetKind::Branch => "branch",
            SheetKind::References => "references",
        }
    }

    /// Its width before you size it, on a screen `screen_w` wide.
    fn default_w(self, screen_w: u16) -> u16 {
        match self {
            SheetKind::Changes => screen_w / 2,
            SheetKind::Files => (u32::from(screen_w) * FILES_SHARE / 100) as u16,
            SheetKind::Branch => screen_w * 3 / 10,
            SheetKind::References => screen_w * 2 / 5,
            _ => SHEET_W,
        }
    }
}

/// The width the sheet showing `kind` wants: what you sized it to, else its default.
pub(in crate::client) fn wanted_w(app: &App, kind: SheetKind, screen_w: u16) -> u16 {
    app.screen.saved.sheet_w.get(kind.key()).copied().unwrap_or_else(|| kind.default_w(screen_w))
}

/// The widest the sheet can be beside the panes in `col`: they keep `PANES_KEEP` and the gap.
pub(in crate::client) fn widest(col: Rect, look: Look) -> u16 {
    col.width.saturating_sub(look.gap + PANES_KEEP)
}

/// The side panel's tabs, in Tab's order: the view, its name, its name where room is short and
/// the action that opens it.
/// The Inbox and the check sheets are not among them.
const DRAWER_TABS: [(SheetKind, &str, &str, Action); 4] = [
    (SheetKind::Files, "Files", "Files", Action::Files),
    (SheetKind::Changes, "Changes", "Chg", Action::Changes),
    (SheetKind::Branch, "Branch", "Br", Action::Branches),
    (SheetKind::References, "References", "Refs", Action::References),
];

/// Where `kind` sits in the tab strip, if it has a tab.
fn drawer_tab(kind: SheetKind) -> Option<usize> {
    DRAWER_TABS.iter().position(|(k, ..)| *k == kind)
}

impl App {
    /// Tab / Shift+Tab move along the tab strip while Files, Changes or Branch has the keys.
    /// Returns whether the key was one of them.
    pub(in crate::client) fn drawer_tab_key(&mut self, k: &KeyEvent) -> bool {
        let n = DRAWER_TABS.len();
        let step = match k.code {
            KeyCode::Tab => 1,
            KeyCode::BackTab => n - 1,
            _ => return false,
        };
        let Some(cur) = open_sheet(self).and_then(drawer_tab) else { return false };
        self.switch_drawer((cur + step) % n);
        true
    }

    /// Open the strip's tab `to` for the same checkout (`screen_target_dir`), as its leader key
    /// does; the open tab's own key would close it, so that one does nothing.
    pub(in crate::client) fn switch_drawer(&mut self, to: usize) {
        let Some((kind, _, _, act)) = DRAWER_TABS.get(to) else { return };
        if open_sheet(self) != Some(*kind) {
            self.act(act.clone());
        }
    }

    /// The open sheet's width set to `w`, held between `SHEET_MIN` and what the panes leave.
    pub(in crate::client) fn set_sheet_w(&mut self, w: u16) {
        let Some(kind) = open_sheet(self) else { return };
        let w = w.clamp(SHEET_MIN, self.screen.sheet_widest.max(SHEET_MIN));
        self.screen.saved.sheet_w.insert(kind.key().into(), w);
    }

    /// `[` narrows the open sheet, `]` widens it and `=` puts its default back, while it has
    /// the keys. Returns whether the key was one of them.
    pub(in crate::client) fn sheet_size_key(&mut self, k: &KeyEvent) -> bool {
        let Some(kind) = open_sheet(self) else { return false };
        if k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) || !matches!(k.code, KeyCode::Char('[' | ']' | '=')) {
            return false;
        }
        // Taking the whole pane column, it has no width of its own to change: the keys do
        // nothing, and the width you chose for wider screens stays.
        let widest = self.screen.sheet_widest;
        if widest < SHEET_MIN {
            return true;
        }
        // From the width it wants as held to this screen, never the drawn one: that may be
        // mid-slide, or another view's, when keys come before the next frame.
        let now = wanted_w(self, kind, self.screen.sheet_screen_w).clamp(SHEET_MIN, widest);
        match k.code {
            KeyCode::Char('[') => self.set_sheet_w(now.saturating_sub(SHEET_STEP)),
            KeyCode::Char(']') => self.set_sheet_w(now + SHEET_STEP),
            KeyCode::Char('=') => {
                self.screen.saved.sheet_w.remove(kind.key());
            }
            _ => return false,
        }
        self.screen.save();
        true
    }
}

/// The sheet that's open, if any: the Inbox while you're in it (or writing a follow-up in
/// one of its rows), else Changes.
pub(in crate::client) fn open_sheet(app: &App) -> Option<SheetKind> {
    if inbox_place(app).is_some() {
        Some(SheetKind::Inbox)
    } else if matches!(app.view, Some(crate::client::View::Changes(_))) {
        Some(SheetKind::Changes)
    } else if matches!(app.view, Some(crate::client::View::Both(_))) {
        Some(SheetKind::Both)
    } else if matches!(app.view, Some(crate::client::View::Check(_))) {
        Some(SheetKind::Check)
    } else if matches!(app.view, Some(crate::client::View::Files(_))) {
        Some(SheetKind::Files)
    } else if matches!(app.view, Some(crate::client::View::Branch(_))) {
        Some(SheetKind::Branch)
    } else if matches!(app.view, Some(crate::client::View::References(_))) {
        Some(SheetKind::References)
    } else {
        None
    }
}

/// The open sheet has the keys: the Inbox always, a view unless a click into a pane set it
/// aside.
pub(in crate::client) fn sheet_has_keys(app: &App) -> bool {
    match open_sheet(app) {
        None => false,
        Some(SheetKind::Inbox) => true,
        Some(_) => app.view_has_keys(),
    }
}

/// Where the sheet goes in the pane column `col`, and what's left for the panes, with `frac`
/// of its width in (it slides in). It docks right, `want` wide (held between `SHEET_MIN` and
/// what leaves the panes `PANES_KEEP`), the layout's gap from the panes (Flat: the panes run
/// under its `│` rule, as beside the sidebar). A screen narrower than `NARROW`, or a column
/// too narrow for both, gives it the whole column.
pub(in crate::client) fn split_for_sheet(screen_w: u16, col: Rect, look: Look, frac: f32, want: u16) -> (Rect, Rect) {
    let room = widest(col, look);
    let full = if screen_w < NARROW || room < SHEET_MIN { col.width } else { want.clamp(SHEET_MIN, room) };
    let w = (full as f32 * frac.clamp(0.0, 1.0)).round() as u16;
    let sheet = Rect { x: col.right() - w, width: w, ..col };
    let beside = if w == 0 || w >= col.width { w } else if look.flat { w - 1 } else { w + look.gap };
    (sheet, Rect { width: col.width.saturating_sub(beside), ..col })
}

/// The same-file overlaps the Inbox lists, in a stable order: (repo, overlap).
pub(in crate::client) fn heads_up(app: &App) -> Vec<(PathBuf, Overlap)> {
    let mut repos: Vec<&PathBuf> = app.screen.overlaps.keys().collect();
    repos.sort();
    repos
        .into_iter()
        .flat_map(|r| app.screen.overlaps[r].iter().map(move |o| (r.clone(), o.clone())))
        .filter(|(r, o)| !app.screen.dismissed.contains(&(r.clone(), o.file.clone())))
        .collect()
}

pub(in crate::client) fn draw_sheet(app: &mut App, buf: &mut Buffer, r: Rect, kind: SheetKind, t: &Theme) {
    let (title, sub, close) = match kind {
        SheetKind::Inbox => ("INBOX", inbox_count(app).to_string(), ScreenHit::Close),
        SheetKind::Changes | SheetKind::Both => ("Changes", String::new(), ScreenHit::ViewClose),
        SheetKind::Files => ("Files", String::new(), ScreenHit::ViewClose),
        SheetKind::Check => ("CHECK", check_title(app), ScreenHit::ViewClose),
        SheetKind::Branch => ("BRANCH", branch_title(app), ScreenHit::ViewClose),
        SheetKind::References => ("REFERENCES", references_title(app), ScreenHit::ViewClose),
    };
    // Lit while it has the keys; a click into a pane sets it aside, unlit, still open.
    let mut c = Card::new(t, title);
    if sheet_has_keys(app) {
        c = c.lit(t.accent);
    }
    c.sub = &sub;
    // The read-while-working sheets and Changes sit on the desk's ground, like the panes.
    c.bg = if kind == SheetKind::Both { t.card } else { t.bg };
    c.close = Some(close);
    let inside = card(app, buf, r, &c, t);
    // Clicks inside it stay in it.
    hit(app, inside, ScreenHit::Noop);
    let inside = match drawer_tab(kind) {
        Some(cur) if inside.height > 1 => {
            draw_drawer_tabs(app, buf, Rect { height: 1, ..inside }, cur, c.bg, t);
            Rect { y: inside.y + 1, height: inside.height - 1, ..inside }
        }
        _ => inside,
    };
    // The Inbox and the evidence sheet keep the Cards box in Flat too: only the frame changes.
    let body = if Look::of(&app.cfg.ui).flat { Rect { width: inside.width.saturating_sub(1), height: inside.height.saturating_sub(1), ..inside } } else { inside };
    match kind {
        SheetKind::Inbox => draw_inbox(app, buf, body, c.bg, t),
        SheetKind::Changes => {
            let inner = Rect { x: inside.x + 1, width: inside.width.saturating_sub(2), ..inside };
            if let Some(crate::client::View::Changes(v)) = app.view.take() {
                crate::client::design::draw_changes(app, buf, inner, t, &v);
                app.view = Some(crate::client::View::Changes(v));
            }
        }
        SheetKind::Both => {
            if let Some(crate::client::View::Both(v)) = &app.view {
                draw_both(buf, inside, t, v);
            }
        }
        SheetKind::Check => {
            if let Some(crate::client::View::Check(mut v)) = app.view.take() {
                draw_check(app, buf, body, c.bg, t, &mut v);
                app.view = Some(crate::client::View::Check(v));
            }
        }
        SheetKind::Files => {
            let inner = Rect { x: inside.x + 1, width: inside.width.saturating_sub(2), ..inside };
            if let Some(crate::client::View::Files(mut v)) = app.view.take() {
                crate::client::design::draw_files(app, buf, inner, t, &mut v, r.width < FILES_BOTH_FROM);
                app.view = Some(crate::client::View::Files(v));
            }
        }
        SheetKind::Branch => {
            if let Some(crate::client::View::Branch(mut v)) = app.view.take() {
                draw_branch(app, buf, body, c.bg, t, &mut v);
                app.view = Some(crate::client::View::Branch(v));
            }
        }
        SheetKind::References => {
            if let Some(crate::client::View::References(mut v)) = app.view.take() {
                draw_references(app, buf, body, c.bg, t, &mut v);
                app.view = Some(crate::client::View::References(v));
            }
        }
    }
}

/// A tab's width: its name and key (with its trailing space) between the pill's ends and pads.
fn tab_w(name: &str, key: &str) -> u16 {
    (name.width() + key.width()) as u16 + 4
}

/// Each tab's (name, key) in the widest form whose strip fits `room` columns: names and keys,
/// names alone, then short names.
fn drawer_tab_labels(app: &App, room: u16) -> Vec<(String, String)> {
    let key = |act: &Action| {
        let k = crate::client::design::key_text(app, std::slice::from_ref(act));
        let k = k.split("  ").next().unwrap_or_default().replace(' ', "");
        if k.is_empty() { k } else { format!("{k} ") }
    };
    let full: Vec<(String, String)> = DRAWER_TABS.iter().map(|(_, name, _, act)| (name.to_string(), key(act))).collect();
    let names: Vec<(String, String)> = DRAWER_TABS.iter().map(|(_, name, ..)| (name.to_string(), String::new())).collect();
    let short: Vec<(String, String)> = DRAWER_TABS.iter().map(|(_, _, short, _)| (short.to_string(), String::new())).collect();
    // The strip's lead column, then each tab and the ` · ` between them.
    let strip_w = |tabs: &[(String, String)]| 1 + tabs.iter().map(|(n, k)| tab_w(n, k)).sum::<u16>() + 3 * (tabs.len() as u16 - 1);
    [full, names].into_iter().find(|tabs| strip_w(tabs) <= room).unwrap_or(short)
}

/// The tab strip across the top of Files, Changes and Branch: `Files f · Changes c · Branch b`
/// with each one's leader key from the keymap, the open one a pill like Settings' tabs (lit
/// while the panel has the keys). A click switches. Where the whole strip does not fit, the
/// keys go, then the long names, so every tab stays on screen.
fn draw_drawer_tabs(app: &mut App, buf: &mut Buffer, row: Rect, cur: usize, ground: Color, t: &Theme) {
    let look = Look::of(&app.cfg.ui);
    let (pill_bg, pill_ink) = if sheet_has_keys(app) { (t.accent, t.acc_ink) } else { (t.btn, t.strong) };
    let tabs = drawer_tab_labels(app, row.width);
    let mut x = row.x + 1;
    for (i, (name, key)) in tabs.into_iter().enumerate() {
        if i > 0 {
            put(buf, x, row.y, &[seg("·", Style::default().fg(t.muted).bg(ground))], row.right());
            x += 2;
        }
        let w = tab_w(&name, &key);
        let tr = Rect { x, y: row.y, width: w.min(row.right().saturating_sub(x)), height: 1 };
        let segs = if i == cur {
            let st = Style::default().fg(pill_ink).add_modifier(Modifier::BOLD);
            pill(look, vec![seg(format!(" {name} "), st), seg(key, st)], pill_bg, ground)
        } else {
            let bg = if hovered(app, tr) { t.hov } else { ground };
            let st = Style::default().bg(bg);
            vec![seg(format!("  {name} "), st.fg(t.text)), seg(key, st.fg(t.accent).add_modifier(Modifier::BOLD)), seg(" ", st)]
        };
        put(buf, x, row.y, &segs, row.right());
        hit(app, tr, ScreenHit::DrawerTab(i));
        x += w + 1;
    }
}

/// Settings is a modal so it does not take width away from the workspace.
pub(in crate::client) fn draw_settings_modal(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme) {
    let Mode::ScreenSettings(view) = &app.mode else { return };
    let view = (**view).clone();
    let max_width = area.width.saturating_sub(4).min(128);
    let max_height = area.height.saturating_sub(2).min(48);
    let r = Rect {
        x: area.x + area.width.saturating_sub(max_width) / 2,
        y: area.y + area.height.saturating_sub(max_height) / 2,
        width: max_width,
        height: max_height,
    };
    super::dim_all(app, buf, area, t);
    hit(app, area, ScreenHit::Noop);
    let r = panel_at(app, buf, r, "Settings", &[], t);
    let inside = Rect { x: r.x + 1, y: r.y + 1, width: r.width.saturating_sub(2), height: r.height.saturating_sub(2) };
    if inside.width < 44 {
        put(buf, inside.x + 2, inside.y + 2, &[seg("Make the window wider", Style::default().fg(t.muted).bg(t.card))], inside.right());
    } else if inside.height < 18 {
        put(buf, inside.x + 2, inside.y + 2, &[seg("Make the window taller", Style::default().fg(t.muted).bg(t.card))], inside.right());
    } else {
        crate::client::screen::draw_settings(app, buf, inside, t, &view);
    }
}

/// The sheet's last row: its key hints in an action bar, and a note at the right when there's room.
pub(in crate::client) fn status_bar(buf: &mut Buffer, inside: Rect, t: &Theme, keys: &[(&str, &str)], note: &[Seg]) {
    let y = inside.bottom().saturating_sub(1);
    let bar = Rect { y, height: 1, ..inside };
    action_bar(buf, bar, t, t.card2, keys);
    let nw = segs_width(note);
    let used = segs_width(&cap_hints(t, t.card2, keys)) + 2;
    if nw > 0 && used + nw + 3 <= bar.width {
        let note: Vec<Seg> = note.iter().map(|(s, st)| (s.clone(), st.bg(t.card2))).collect();
        put(buf, bar.right().saturating_sub(nw + 2), y, &note, bar.right());
    }
}
