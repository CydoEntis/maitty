//! The sheet: one card docked right of the panes for what you look at beside your work (the
//! Inbox, Changes). The panes reflow into the rest; below `NARROW` it takes the whole pane
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
}

/// Its width beside the panes.
pub(super) const SHEET_W: u16 = 72;

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
    } else {
        None
    }
}

/// Where the sheet goes in the pane column `col`, and what's left for the panes, with `frac`
/// of its width in (it slides in). It docks right, `SHEET_W` wide, the layout's gap from the
/// panes (Flat: the panes run under its `│` rule, as beside the sidebar); a screen narrower
/// than `NARROW` gives it the whole column.
pub(in crate::client) fn split_for_sheet(screen_w: u16, col: Rect, look: Look, frac: f32) -> (Rect, Rect) {
    let full = if screen_w < NARROW || col.width <= SHEET_W + look.gap + 20 { col.width } else { SHEET_W };
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

/// Columns of a sheet's title row that are not the text after its name: the corners, the
/// name, the ✕ and the gaps between.
const TITLE_CHROME: u16 = 24;

pub(in crate::client) fn draw_sheet(app: &mut App, buf: &mut Buffer, r: Rect, kind: SheetKind, t: &Theme) {
    let (title, sub, close) = match kind {
        SheetKind::Inbox => ("INBOX", inbox_count(app).to_string(), ScreenHit::Close),
        SheetKind::Changes | SheetKind::Both => ("Changes", String::new(), ScreenHit::ViewClose),
        SheetKind::Check => ("CHECK", check_title(app), ScreenHit::ViewClose),
    };
    let mut c = Card::new(t, title).lit(t.accent);
    // Cut here, so a long name still shows its start rather than being left out.
    let sub = truncate(&sub, r.width.saturating_sub(TITLE_CHROME) as usize);
    c.sub = &sub;
    // The read-while-working sheets and Changes sit on the desk's ground, like the panes.
    c.bg = if kind == SheetKind::Both { t.card } else { t.bg };
    c.close = Some(close);
    let inside = card(app, buf, r, &c, t);
    // Clicks inside it stay in it.
    hit(app, inside, ScreenHit::Noop);
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
    super::dim_all(buf, area, t);
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

/// The sheet's last row: a pill of key hints, and a note at the right when there's room.
pub(in crate::client) fn status_bar(buf: &mut Buffer, inside: Rect, t: &Theme, keys: &[(&str, &str)], note: &[Seg]) {
    let y = inside.bottom().saturating_sub(1);
    let bar = Rect { x: inside.x + 1, y, width: inside.width.saturating_sub(2), height: 1 };
    strip(buf, bar, t.card2);
    let segs = cap_hints(t, t.card2, keys);
    let end = put(buf, bar.x + 2, y, &segs, bar.right().saturating_sub(1));
    let nw = segs_width(note);
    if nw > 0 && end + nw + 3 <= bar.right().saturating_sub(2) {
        let note: Vec<Seg> = note.iter().map(|(s, st)| (s.clone(), st.bg(t.card2))).collect();
        put(buf, bar.right().saturating_sub(nw + 2), y, &note, bar.right());
    }
}
