//! The main screen: the sidebar, the panes,
//! the sheet, the mode bar along the bottom and the toast.

use super::*;
use crate::brand::APP_COMMAND;
use crate::client::ids::ClientTerm;

// ---- main screen ---------------------------------------------------------------------------

/// The sidebar's width before you drag it: the design's 34 columns, less on a screen too
/// narrow to leave the workspace half of it.
pub(in crate::client) fn side_w(width: u16) -> u16 {
    SIDE_DEFAULT.min(width / 2)
}

/// The sidebar's default width in columns (resizable between `SIDE_MIN` and `SIDE_MAX`).
pub(in crate::client) const SIDE_DEFAULT: u16 = 34;

/// The sidebar's width: the default, or what you dragged it to.
pub(in crate::client) fn full_side_w(app: &App, area: Rect) -> u16 {
    app.screen
        .saved
        .side_w
        .unwrap_or_else(|| side_w(area.width))
        .clamp(SIDE_MIN, SIDE_MAX)
        .min(area.width / 2)
}

/// Below this many columns a docked sheet (a check's evidence, both diffs) takes the sidebar's
/// room as well as the panes'.
const SHEET_OVER_SIDE_BELOW: u16 = 100;

/// The narrowest the sidebar (sliding) or the sheet is drawn; below it, it's only space.
const SIDE_DRAWN_FROM: u16 = 12;
pub(in crate::client) const SHEET_DRAWN_FROM: u16 = 24;
/// The narrowest a pane is drawn while it grows in.
pub(super) const PANE_DRAWN_FROM: u16 = 8;

/// Below this many columns the layout tightens: a slimmer sidebar, and a sheet takes the
/// whole pane column instead of docking beside the panes.
pub(in crate::client) const NARROW: u16 = 140;

/// Blank rows round the screen for each step of `ui.padding` (small 0, medium 1, large 2); a
/// column is half the width of a row, so each gets twice as many.
const PADDING_COLS_PER_ROW: u16 = 2;

/// The window less the padding setting: `small` none, `medium` 1 row and 2 columns, `large`
/// 2 rows and 4 columns, all round.
fn padded(app: &App, area: Rect) -> Rect {
    let rows = crate::config::size_cells(&app.cfg.ui.padding).saturating_sub(1);
    let cols = rows * PADDING_COLS_PER_ROW;
    Rect {
        x: area.x + cols,
        y: area.y + rows,
        width: area.width.saturating_sub(2 * cols),
        height: area.height.saturating_sub(2 * rows),
    }
}

/// Blank rows between the frames and the mode bar, so the bar does not touch their bottom lines.
const BAR_GAP: u16 = 1;

/// Whether the bar's rows are kept. Always: opening a menu or key mode fills the bar but must not move the frames.
pub(in crate::client) fn bar_shown(app: &App) -> bool {
    // The bar's rows are always kept, so opening a menu or key mode never moves the panes.
    let _ = app;
    true
}

/// Whether the bar has anything to say. While keys go to the panes it has nothing: the pane and the sidebar already
/// name the session, and key mode shows on the frame. It speaks for key mode, for anything that holds the keys, for a
/// line it flashes, and while Update or Restart waits; otherwise its rows stay blank.
pub(in crate::client) fn bar_has_words(app: &App) -> bool {
    armed(app)
        || shortcut_waits(app)
        || app.mode != Mode::Normal
        || app.view_has_keys()
        || app.bar_flash.as_ref().is_some_and(|(_, at)| at.elapsed() < crate::client::BAR_FLASH_FOR)
        || app.update_available.is_some()
        || app.restart_needed()
}

/// The room the screen is laid out in: inside the padding, above the mode bar and its gap.
pub(in crate::client) fn framed(app: &App, area: Rect) -> Rect {
    let r = padded(app, area);
    let taken = if bar_shown(app) { 1 + BAR_GAP } else { 0 };
    Rect {
        height: r.height.saturating_sub(taken),
        ..r
    }
}

/// The mode bar's row: the bottom row inside the padding.
pub(in crate::client) fn mode_bar_row(app: &App, area: Rect) -> Rect {
    let r = padded(app, area);
    if !bar_shown(app) {
        return Rect { y: r.bottom(), height: 0, ..r };
    }
    Rect {
        y: r.bottom().saturating_sub(1),
        height: r.height.min(1),
        ..r
    }
}

/// Where popups and their dimming go: the window above the mode bar, so the mode stays in
/// view and readable under any of them.
pub(in crate::client) fn above_bar(app: &App, area: Rect) -> Rect {
    let top = mode_bar_row(app, area).y;
    Rect {
        height: top.saturating_sub(area.y),
        ..area
    }
}

/// Where everything goes: the sidebar, the pane column and the sheet.
pub(in crate::client) struct Grid {
    pub side: Rect,
    pub panes: Rect,
    /// Between the sidebar and the panes (drag it to resize).
    pub edge: Rect,
    /// The sheet docked right of the panes, when one is open (else empty).
    pub sheet: Rect,
}

/// The grid in `area` (already `framed`): the sidebar the full height at one side, then the
/// pane column from its top row. Cards floating leave the gap setting between them. A width
/// you dragged the sidebar to wins over the default.
pub(in crate::client) fn grid(app: &App, area: Rect) -> Grid {
    let look = Look::of(&app.cfg.ui);
    let full = full_side_w(app, area);
    // Collapsed, the sidebar is the narrow column; it slides between that and its full width.
    let rail = side::RAIL_W.min(area.width / 2);
    let sw = rail + (full.saturating_sub(rail) as f32 * app.side_frac).round() as u16;
    let gap = look.gap;
    let (side, col) = if app.cfg.ui.sidebar_position == "right" {
        let side = Rect {
            x: area.right().saturating_sub(sw),
            width: sw,
            ..area
        };
        (
            side,
            Rect {
                width: area.width.saturating_sub(sw + gap),
                ..area
            },
        )
    } else {
        let x = area.x + sw + gap;
        (
            Rect { width: sw, ..area },
            Rect {
                x,
                width: area.right().saturating_sub(x),
                ..area
            },
        )
    };
    let edge = if side.x > col.x {
        Rect {
            x: col.right(),
            width: gap.max(1),
            ..area
        }
    } else {
        Rect {
            x: col.x.saturating_sub(gap),
            width: gap.max(1),
            ..area
        }
    };
    let (sheet, panes) = match open_sheet(app) {
        // The Inbox floats over the workspace from the sidebar's side; the panes keep the column.
        Some(SheetKind::Inbox) => {
            let w = ((INBOX_W.min(col.width) as f32 * app.sheet_frac.clamp(0.0, 1.0)).round()
                as u16)
                .min(col.width);
            let x = if side.x > col.x { col.right() - w } else { col.x };
            (Rect { x, width: w, ..col }, col)
        }
        // A tool slides in from the panes' right edge, inside their box (its top and bottom
        // lines and its chips stay in view): the panes keep the whole column.
        Some(kind) if overlaid(app, kind) => {
            let pane = focused_box(app, col);
            let full = wanted_w(app, kind, area.width)
                .min(drawer_cap(kind, pane))
                .clamp(SHEET_MIN, overlay_widest(pane).max(SHEET_MIN));
            let inner = pane.width.saturating_sub(2);
            let w = ((full as f32 * app.sheet_frac.clamp(0.0, 1.0)).round() as u16).min(inner);
            (
                Rect {
                    x: pane.right().saturating_sub(1 + w),
                    y: pane.y + 1,
                    width: w,
                    height: pane.height.saturating_sub(2),
                },
                col,
            )
        }
        // Too narrow to read beside the sidebar: a docked sheet covers it too.
        Some(_) if area.width < SHEET_OVER_SIDE_BELOW => (area, col),
        Some(kind) => split_for_sheet(
            area.width,
            col,
            look,
            app.sheet_frac,
            wanted_w(app, kind, area.width),
        ),
        None => (Rect::default(), col),
    };
    Grid {
        side,
        panes,
        edge,
        sheet,
    }
}

/// Where the side sheet is this frame, the widest it could be, and the screen's width, for its
/// edge and its `[` `]` keys. A sheet that takes the whole pane column, or none, can't be
/// sized: its widest is 0.
fn note_sheet(app: &mut App, g: &Grid, screen_w: u16) {
    let look = Look::of(&app.cfg.ui);
    let col = Rect {
        width: g.sheet.right().saturating_sub(g.panes.x),
        ..g.panes
    };
    // A floating drawer is sized against the terminal under it, which it does not take room from.
    let room = match open_sheet(app) {
        Some(kind) if overlaid(app, kind) => overlay_widest(g.panes),
        _ => widest(col, look),
    };
    let sizable = open_sheet(app).is_some_and(|k| k != SheetKind::Inbox)
        && screen_w >= NARROW
        && room >= SHEET_MIN;
    app.screen.sheet_rect = g.sheet;
    app.screen.sheet_widest = if sizable { room } else { 0 };
    app.screen.sheet_screen_w = screen_w;
}

/// The side sheet's left edge, beside the panes: drag it to resize the sheet. It lights up
/// while you point at it or drag it. A sheet that takes the whole pane column has none.
fn draw_sheet_edge(app: &mut App, buf: &mut Buffer, g: &Grid, t: &Theme) {
    if app.screen.sheet_widest < SHEET_MIN || g.panes.width == 0 {
        return;
    }
    // The gap between them (Floating); the sheet's own line where they touch (Tiled, Compact).
    let x = g.panes.right().min(g.sheet.x);
    let edge = Rect {
        x,
        width: g.sheet.x.saturating_sub(g.panes.right()).max(1),
        ..g.sheet
    };
    let ex = edge.x + edge.width / 2;
    if app.screen.drag == Some(Drag::Sheet) || hovered(app, edge) {
        for yy in g.sheet.top() + 1..g.sheet.bottom().saturating_sub(1) {
            if let Some(px) = buf.cell_mut((ex, yy)) {
                px.set_symbol("│")
                    .set_style(Style::default().fg(t.accent).bg(t.bg));
            }
        }
        draw_grip(buf, ex, (g.sheet.top(), g.sheet.bottom()), true, t);
    } else {
        draw_grip(buf, ex, (g.sheet.top(), g.sheet.bottom()), false, t);
    }
    hit(app, edge, ScreenHit::SheetEdge);
}

/// Rows the grip on a draggable edge is tall.
const GRIP_ROWS: u16 = 3;

/// The mark that an edge can be dragged: a short heavy bar at the middle of the line (`lit` when
/// it is pointed at or dragged, else quiet), drawn in the column `x` between `top` and `bottom`.
fn draw_grip(buf: &mut Buffer, x: u16, (top, bottom): (u16, u16), lit: bool, t: &Theme) {
    let mid = top + bottom.saturating_sub(top) / 2;
    let colour = if lit { t.accent } else { t.muted };
    for y in mid.saturating_sub(GRIP_ROWS / 2)..mid.saturating_sub(GRIP_ROWS / 2) + GRIP_ROWS {
        if y < bottom.saturating_sub(1)
            && y > top
            && let Some(px) = buf.cell_mut((x, y))
        {
            px.set_symbol("┃").set_style(Style::default().fg(colour).bg(t.bg));
        }
    }
}

/// Draw the main screen; returns the pane area.
pub(in crate::client) fn draw(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) -> Rect {
    // How far the sidebar and the sheet are in this frame (they slide when they change).
    let on = app.motion_on();
    app.side_frac = app.motion.side(app.sidebar, on);
    app.sheet_frac = app.motion.sheet(open_sheet(app).is_some(), on);
    let model = app.screen_model();
    app.screen.wt_keys.clear();
    app.screen.release_in_side = false;
    app.screen.branch_keys.clear();
    fill(f.buffer_mut(), area, t.bg);
    let room = framed(app, area);
    // A view set aside by a click into a pane is forgotten once it closes.
    if app.view.is_none() {
        app.view_set_aside = None;
    }
    let g = grid(app, room);
    note_sheet(app, &g, room.width);
    // Too narrow mid-slide to draw anything sensible: just the room it takes.
    if g.side.width >= SIDE_DRAWN_FROM {
        draw_side(app, f.buffer_mut(), g.side, &model, t);
        // The edge beside the sidebar: drag it. It lights up while you point at it.
        if app.screen.drag == Some(Drag::Side) || hovered(app, g.edge) {
            let ex = g.edge.x + g.edge.width / 2;
            for yy in g.side.top() + 1..g.side.bottom().saturating_sub(1) {
                if let Some(px) = f.buffer_mut().cell_mut((ex, yy)) {
                    px.set_symbol("│")
                        .set_style(Style::default().fg(t.accent).bg(t.bg));
                }
            }
            draw_grip(f.buffer_mut(), ex, (g.side.top(), g.side.bottom()), true, t);
        }
        hit(app, g.edge, ScreenHit::SideEdge);
    } else {
        // The narrow column: nothing to move a cursor over, and `»` to open the sidebar again.
        app.screen.side_rect = Rect::default();
        app.screen.visible.clear();
        app.screen.side_items.clear();
        app.screen.row_y.clear();
        app.screen.sub_rows.clear();
        side::draw_rail(app, f.buffer_mut(), g.side, &model, t);
    }
    if g.panes.width > 0 {
        draw_main(app, f, g.panes, &model, t);
    }
    if let Some(kind) = open_sheet(app).filter(|_| g.sheet.width >= SHEET_DRAWN_FROM) {
        // A floating drawer clears what it covers first.
        if overlaid(app, kind) || kind == SheetKind::Inbox {
            fill(f.buffer_mut(), g.sheet, t.bg);
        }
        draw_sheet(app, f.buffer_mut(), g.sheet, kind, t);
        draw_sheet_edge(app, f.buffer_mut(), &g, t);
    }
    let bar = mode_bar_row(app, area);
    if bar_has_words(app) {
        draw_mode_bar(app, f.buffer_mut(), bar, t);
    } else {
        // Nothing to say: the rows stay where they are, blank, so the frames above never move.
        app.screen.bar_keys.clear();
        fill(f.buffer_mut(), bar, t.bg);
    }
    app.screen.crumb_x = g.panes.x + 1;
    // A popup (`<app> popup`) floats over everything, the rest dimmed.
    let above = above_bar(app, area);
    match app.mode.clone() {
        Mode::Compose(c) if c.inbox.is_none() => {
            draw_compose_pop(app, f.buffer_mut(), above, g.side, &c, t)
        }
        Mode::Helpers { term, .. } => draw_helpers_pop(app, f.buffer_mut(), above, g.side, term, t),
        Mode::Secret { term, .. } => draw_secret_pop(app, f.buffer_mut(), above, g.side, term, t),
        _ => {}
    }
    if let Some(term) = app.popup() {
        let whole = above;
        dim_all(app, f.buffer_mut(), whole, t);
        let w = (whole.width * 4 / 5).max(40).min(whole.width);
        let h = (whole.height * 3 / 4).max(12).min(whole.height);
        let r = Rect {
            x: whole.x + (whole.width - w) / 2,
            y: whole.y + (whole.height - h) / 2,
            width: w,
            height: h,
        };
        draw_pane(app, f, r, term, (true, true), &model, t);
    }
    draw_toast(app, f.buffer_mut(), room, t);
    g.panes
}

pub(in crate::client) fn find(model: &[Proj], term: ClientTerm) -> Option<(&Proj, &Wt, &Session)> {
    model.iter().find_map(|p| {
        p.wts.iter().find_map(|w| {
            w.sessions
                .iter()
                .find(|s| s.native_term == Some(term))
                .map(|s| (p, w, s))
        })
    })
}

pub(in crate::client) fn hline(buf: &mut Buffer, x: u16, y: u16, w: u16, t: &Theme, bg: Color) {
    for i in 0..w {
        if let Some(px) = buf.cell_mut((x + i, y)) {
            px.set_symbol("─")
                .set_style(Style::default().fg(t.line).bg(bg));
        }
    }
}

/// The workspace with no pane open: a dashed box, "No pane open", an `n new session` chip
/// (the New session dialog) and where else to go.
pub(in crate::client) fn draw_no_sessions(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) {
    let buf = f.buffer_mut();
    dashed_box(buf, area, Style::default().fg(t.line).bg(t.bg));
    let quiet = Style::default().fg(t.muted).bg(t.bg);
    let mid = area.y + area.height.saturating_sub(5) / 2;
    let centred = |w: usize| area.x + area.width.saturating_sub(w as u16) / 2;
    let title = "No pane open";
    put(buf, centred(title.width()), mid, &[seg(title, quiet)], area.right());
    let chip = pill(
        Look::of(&app.cfg.ui),
        vec![
            seg("n", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            seg(" new session", Style::default().fg(t.strong).add_modifier(Modifier::BOLD)),
        ],
        t.btn,
        t.bg,
    );
    let w = segs_width(&chip);
    let x = centred(w as usize);
    put(buf, x, mid + 2, &chip, area.right());
    hit(app, Rect { x, y: mid + 2, width: w, height: 1 }, ScreenHit::NoPaneNew);
    let hint = "or pick a session on the left";
    put(f.buffer_mut(), centred(hint.width()), mid + 4, &[seg(hint, quiet)], area.right());
}

/// A box of dashed lines round `r`, with round corners.
fn dashed_box(buf: &mut Buffer, r: Rect, st: Style) {
    if r.width < 2 || r.height < 2 {
        return;
    }
    let (right, bottom) = (r.right() - 1, r.bottom() - 1);
    for x in r.x + 1..right {
        buf[(x, r.y)].set_symbol("┄").set_style(st);
        buf[(x, bottom)].set_symbol("┄").set_style(st);
    }
    for y in r.y + 1..bottom {
        buf[(r.x, y)].set_symbol("┆").set_style(st);
        buf[(right, y)].set_symbol("┆").set_style(st);
    }
    for (x, y, g) in [(r.x, r.y, "╭"), (right, r.y, "╮"), (r.x, bottom, "╰"), (right, bottom, "╯")] {
        buf[(x, y)].set_symbol(g).set_style(st);
    }
}

/// A pane dragged by its title: the pane it would swap with gets lit lines and a "swap here"
/// pill, and the dragged pane's name follows the pointer.
pub(super) fn draw_pane_drag(app: &App, buf: &mut Buffer, model: &[Proj], t: &Theme) {
    let Some(Drag::Pane { from, over, at }) = app.screen.drag else {
        return;
    };
    let look = Look::of(&app.cfg.ui);
    if let Some((_, r)) = over.and_then(|o| app.pane_frames.iter().find(|(id, _)| *id == o)) {
        light_lines(buf, *r, t.accent);
        let pill = pill(
            look,
            vec![seg(
                "⇄ swap here",
                Style::default().fg(t.acc_ink).add_modifier(Modifier::BOLD),
            )],
            t.accent,
            t.bg,
        );
        let w = segs_width(&pill);
        // Clear of the dragged name, which sits on the pointer's row.
        let y = r.y + r.height / 2;
        let y = if y == at.y { y + 2 } else { y };
        put(
            buf,
            r.x + r.width.saturating_sub(w) / 2,
            y.min(r.bottom().saturating_sub(2)),
            &pill,
            r.right(),
        );
    }
    let ghost = pill(
        look,
        vec![seg(
            format!("⇄ {}", session_name(app, model, from)),
            Style::default().fg(t.strong).add_modifier(Modifier::BOLD),
        )],
        t.btn,
        t.bg,
    );
    put(buf, at.x.saturating_add(1), at.y, &ghost, buf.area.right());
}

/// Light the frame lines of the pane at `r` (its border).
fn light_lines(buf: &mut Buffer, r: Rect, colour: Color) {
    let r = r.intersection(buf.area);
    let edge = |x: u16, y: u16| {
        y == r.y || y + 1 == r.bottom() || x == r.x || x + 1 == r.right()
    };
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            if let Some(px) = buf.cell_mut((x, y))
                && edge(x, y)
                && px
                    .symbol()
                    .chars()
                    .next()
                    .is_some_and(|c| ('\u{2500}'..='\u{257F}').contains(&c))
            {
                px.set_fg(colour);
            }
        }
    }
}

/// Leader mode is on: waiting for the key, or showing the key map.
pub(in crate::client) fn armed(app: &App) -> bool {
    matches!(app.mode, Mode::Prefix { .. } | Mode::KeyMap(_))
}

/// Shortcut mode is held while a popup or view it opened has the keys. Normal with
/// no view is the moment before it comes back, when keys still reach the pane.
pub(in crate::client) fn shortcut_waits(app: &App) -> bool {
    app.shortcut_held && !armed(app) && (app.mode != Mode::Normal || app.view_has_keys())
}

/// What a session is called on its pane: its sidebar name, else (a pane in another's tab or
/// split has no row) your name for it or what runs in it.
pub(in crate::client) fn session_name(app: &App, model: &[Proj], term: ClientTerm) -> String {
    if let Some((_, _, s)) = find(model, term) {
        return s.name.clone();
    }
    let Some(info) = app.term_info(term) else {
        return String::new();
    };
    if !info.label.trim().is_empty() {
        return info.label.trim().to_string();
    }
    match &info.agent {
        Some(a) => a.clone(),
        None if info.is_shell() => "shell".into(),
        None => info.display_name(),
    }
}

/// A close confirm asks about this pane: its lines are lit in `err` until you answer, so you
/// see which one goes.
pub(in crate::client) fn closing(app: &App, term: ClientTerm) -> bool {
    matches!(&app.mode, Mode::Confirm(c) if matches!(&c.act, crate::client::menu::Act::End(ts) if ts.contains(&term)))
}

/// A scrollbar in the margin right of the terminal when there's history: where you are, click
/// or drag it.
pub(super) fn draw_scrollbar(
    app: &mut App,
    buf: &mut Buffer,
    term: ClientTerm,
    inner: Rect,
    terminal_bg: Color,
    t: &Theme,
) {
    let (cur, total) = app.history(term);
    if total > 0 && inner.height > 2 {
        let track = Rect {
            x: inner.right() + 1,
            y: inner.y,
            width: 1,
            height: inner.height,
        };
        let h = track.height as usize;
        let thumb = (h * h / (h + total)).clamp(1, h);
        let top = track.y + ((h - thumb) * (total - cur) / total) as u16;
        let hot = app.screen.drag == Some(Drag::Scroll(term)) || hovered(app, track);
        for yy in track.top()..track.bottom() {
            let on = yy >= top && yy < top + thumb as u16;
            let (sym, col) = if on {
                ("┃", if hot { t.accent } else { t.muted })
            } else {
                ("│", t.line)
            };
            if let Some(px) = buf.cell_mut((track.x, yy)) {
                px.set_symbol(sym)
                    .set_style(Style::default().fg(col).bg(terminal_bg));
            }
        }
        if app.screen.drag.is_none() || app.screen.drag == Some(Drag::Scroll(term)) {
            app.screen.bar = Some((term, track, total));
        }
        hit(app, track, ScreenHit::ScrollBar(term));
    }
}

/// Scrolled up (in the app's history, or an agent that scrolls itself paged up by the wheel):
/// a button back to the newest output, where a chat's is, at the bottom right.
pub(super) fn draw_to_bottom(
    app: &mut App,
    buf: &mut Buffer,
    term: ClientTerm,
    inner: Rect,
    scrolled: Option<usize>,
    terminal_bg: Color,
    t: &Theme,
) {
    let label = match (scrolled, app.paged_up.get(&term)) {
        (Some(n), _) => Some(format!(" ↓ Back to bottom · {n} lines up ")),
        (None, Some(_)) => Some(" ↓ Back to bottom ".to_string()),
        (None, None) => None,
    };
    if let Some(label) = label
        && inner.height > 0
    {
        let look = Look::of(&app.cfg.ui);
        let note = pill(
            look,
            vec![seg(
                label,
                Style::default().fg(t.acc_ink).add_modifier(Modifier::BOLD),
            )],
            t.accent,
            terminal_bg,
        );
        let w = segs_width(&note).min(inner.width);
        let (x, y) = (inner.right().saturating_sub(w), inner.bottom() - 1);
        put(buf, x, y, &note, inner.right());
        hit(
            app,
            Rect {
                x,
                y,
                width: w,
                height: 1,
            },
            ScreenHit::ToBottom(term),
        );
    }
}

/// Answer bar: the choices of the question asked through the app, a full-width pill that
/// keeps its amber even when the card is faded.
pub(super) fn draw_answer_bar(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    term: ClientTerm,
    terminal_bg: Color,
    t: &Theme,
) {
    let look = Look::of(&app.cfg.ui);
    let ay = r.bottom() - 3;
    let (lx, rx) = (r.x + 3, r.right().saturating_sub(3));
    row_pill(look, buf, lx - 1, ay, rx - lx + 2, t.card2, terminal_bg);
    let opts = app.answer_options(term);
    let full: u16 = opts.iter().take(4).map(|o| o.width() as u16 + 6).sum();
    // In a narrow pane the label shrinks to its dot, then the buttons to their keys.
    let roomy = rx.saturating_sub(lx) >= full + 11;
    let label = if roomy { "● answer   " } else { "● " };
    let mut x = put(
        buf,
        lx,
        ay,
        &[seg(
            label,
            Style::default()
                .fg(t.blocked)
                .bg(t.card2)
                .add_modifier(Modifier::BOLD),
        )],
        rx,
    );
    let keys_only = rx.saturating_sub(x) < full;
    for (i, o) in opts.iter().enumerate().take(4) {
        let key = char::from_digit(i as u32 + 1, 10)
            .unwrap_or('1')
            .to_string();
        let label = if keys_only { String::new() } else { o.clone() };
        let segs = button_pill(look, t, &label, &key, i == 0, t.card2);
        let br = Rect {
            x,
            y: ay,
            width: segs_width(&segs),
            height: 1,
        };
        x = put(buf, x, ay, &segs, rx) + 1;
        app.hits.push((
            br,
            Hit::Button(crate::client::Btn::Answer(
                term,
                key.chars().next().unwrap_or('1'),
            )),
        ));
    }
}

/// How long a plain toast stays.
const TOAST_CLEAR: std::time::Duration = std::time::Duration::from_millis(2400);

/// How long a toast stays: a plain one `TOAST_CLEAR`; an error longer, one about a session
/// (a click goes there) longer still, one with a button longest.
pub(in crate::client) fn toast_hold(err: bool, about: bool, action: bool) -> std::time::Duration {
    if action {
        std::time::Duration::from_millis(12000)
    } else if err {
        std::time::Duration::from_millis(4500)
    } else if about {
        std::time::Duration::from_millis(6000)
    } else {
        TOAST_CLEAR
    }
}

/// The toast's narrowest card, in columns, and the room its text keeps beside the bar and ✕.
const TOAST_MIN_W: usize = 40;
const TOAST_TEXT_PAD: usize = 8;
/// Rows between the card's bottom edge and the screen's: the agent's input area sits there.
const TOAST_ABOVE_INPUT: u16 = 5;
/// The most lines a toast's title and detail take, and its preview (an excerpt), before the end
/// is cut with an ellipsis.
const TOAST_TITLE_LINES: usize = 3;
const TOAST_DETAIL_LINES: usize = 3;
const TOAST_PREVIEW_LINES: usize = 2;
/// Where a toast's rows after the title start, from its left edge; the close leaves two cells
/// on the right.
const TOAST_TEXT_X: usize = 2;

/// `text` wrapped on at most `max` lines of `width` cells, none when there is no text.
fn toast_lines(text: Option<&str>, width: usize, max: usize) -> Vec<String> {
    match text {
        Some(t) if max > 0 => popover::wrap_capped(t, width, max),
        _ => Vec::new(),
    }
}

/// One toast for every notice, bottom right above the agent's input area: a `card` boxed in
/// the notice's colour with a kind chip on its top border. Row 1 is the mark and bold
/// title with a ✕; row 2 the detail; an optional row 3 a preview in dim italics. It clears
/// after `toast_hold` (errors a little longer), or with the ✕ / Esc.
pub(in crate::client) fn draw_toast(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme) {
    app.screen.toast_rect = Rect::default();
    if let Some(flash) = app.copy_flash.as_ref().filter(|f| f.live())
        && let Some((_, pane)) = app.panes.iter().find(|(term, _)| *term == flash.term)
    {
        flash.tint(buf, *pane, t);
    }
    let Some((title, at, err)) = app.notice.clone() else {
        return;
    };
    let about = app.notice_term.filter(|t| app.term_info(*t).is_some());
    let restart = app.toast_restart();
    let hold = toast_hold(err, about.is_some(), restart);
    let shown = at.elapsed();
    if shown > hold {
        return;
    }
    // It slides in from the right, and fades at the end of its time.
    let on = app.motion_on();
    let slide = if on {
        1.0 - crate::client::motion::ease_out(
            shown.as_secs_f32() / crate::client::motion::TOAST_IN.as_secs_f32(),
        )
    } else {
        0.0
    };
    let left = hold.saturating_sub(shown);
    let fade = if on && left < crate::client::motion::TOAST_FADE {
        1.0 - left.as_secs_f32() / crate::client::motion::TOAST_FADE.as_secs_f32()
    } else {
        0.0
    };
    // The update's own sentence is long; with the button the toast keeps to the news.
    let title = match (restart, &app.installed_version) {
        (true, Some(Ok(v))) => format!("{APP_COMMAND} {v} installed"),
        _ => title,
    };
    let card = app.notice_card.clone();
    let detail = match (&card, about.is_some()) {
        (Some(c), _) => Some(c.detail.clone()),
        (None, true) => Some("click to open".to_string()),
        _ => None,
    };
    let preview = card.and_then(|c| c.preview);
    let needs = about
        .and_then(|term| app.term_info(term))
        .is_some_and(|x| x.status == Status::Blocked);
    let (colour, mark, kind) = match (err, needs, about) {
        (true, ..) => (t.err, "✕", "Error"),
        (_, true, _) => (t.blocked, "▲", "Needs you"),
        (_, _, Some(_)) => (t.done, "✓", "Done"),
        // A hint about keys (shortcut mode's dead key) is information, not a success.
        _ if app.notice_hint => (t.sky(), "i", "Hint"),
        _ => (t.accent, "✓", "Notice"),
    };
    let look = Look::of(&app.cfg.ui);
    let button = restart.then(|| button_pill(look, t, "Restart now", "", false, t.bg));
    let body_w = |s: &str| s.width() + TOAST_TEXT_PAD;
    let want = [
        Some(body_w(&title)),
        detail.as_deref().map(body_w),
        preview.as_deref().map(body_w),
        button
            .as_ref()
            .map(|b| segs_width(b) as usize + TOAST_TEXT_PAD),
    ];
    let w = want
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(0)
        .max(TOAST_MIN_W)
        .min((area.width as usize).saturating_sub(4)) as u16;
    // Wrapped against the settled width, not the sliding one, so the text does not reflow while
    // the toast slides in. What the window cannot hold is left out, the title first kept.
    let text_x = TOAST_TEXT_X;
    let text_room = (w as usize).saturating_sub(text_x + 2);
    let mut rows_left = area.height.saturating_sub(TOAST_ABOVE_INPUT + 2) as usize;
    let title_w = (w as usize).saturating_sub(TOAST_TEXT_PAD + 1);
    let title_lines = toast_lines(
        Some(&title),
        title_w,
        TOAST_TITLE_LINES.min(rows_left.saturating_sub(1).max(1)),
    );
    rows_left = rows_left.saturating_sub(title_lines.len());
    let detail_lines = match button {
        Some(_) => Vec::new(),
        None => toast_lines(
            detail.as_deref(),
            text_room,
            TOAST_DETAIL_LINES.min(rows_left),
        ),
    };
    rows_left = rows_left.saturating_sub(detail_lines.len().max(usize::from(button.is_some())));
    let preview_lines = toast_lines(
        preview.as_deref(),
        text_room,
        TOAST_PREVIEW_LINES.min(rows_left),
    );
    let h = 2
        + title_lines.len() as u16
        + (detail_lines.len() as u16).max(u16::from(button.is_some()))
        + preview_lines.len() as u16;
    if area.height < h + TOAST_ABOVE_INPUT || w < 12 {
        return;
    }
    let Some(r) = toast_rect(area, w, h, slide) else {
        return;
    };
    app.screen.toast_rect = r;
    let ink = |c: Color, bg: Color| if fade > 0.0 { blend(c, bg, fade) } else { c };
    // Framed like a pane (a box on the plain ground, round corners whatever `ui.rounded` says,
    // no fill of its own), its border and a kind chip on the top border in the notice's colour.
    let bg = t.bg;
    // A box's outline keeps the ground behind it, as a modal's does: a border cell on the
    // toast's own ground would show a band of it outside the line.
    let behind_top = top_ground(buf, r);
    fill(buf, inside_outline(r), bg);
    panel_box(
        buf,
        r,
        Look {
            rounded: true,
            ..look
        },
        Style::default().fg(ink(colour, bg)),
    );
    // The kind chip carries the notice's mark, as a modal's title chip carries its title.
    let chip = pill(
        look,
        vec![seg(
            format!("{mark} {kind}"),
            Style::default()
                .fg(ink(t.ink_on(colour), colour))
                .add_modifier(Modifier::BOLD),
        )],
        ink(colour, bg),
        bg,
    );
    put(buf, r.x + 2, r.y, &chip, r.right().saturating_sub(1));
    // Every row starts under the chip.
    let tx = r.x + 2;
    let title_style = Style::default()
        .fg(ink(t.strong, bg))
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let title_seg = seg(title_lines[0].clone(), title_style);
    put(buf, r.x + 2, r.y + 1, &[title_seg], r.right() - 1);
    for (i, line) in title_lines.iter().enumerate().skip(1) {
        put(
            buf,
            tx,
            r.y + 1 + i as u16,
            &[seg(line.clone(), title_style)],
            r.right() - 1,
        );
    }
    // The close is the red chip modals wear on their top border.
    let close_chip = pill(
        look,
        vec![seg(
            "×",
            Style::default()
                .fg(ink(t.ink_on(t.err), t.err))
                .add_modifier(Modifier::BOLD),
        )],
        ink(t.err, bg),
        bg,
    );
    let close_w = segs_width(&close_chip);
    let close = Rect {
        x: r.right().saturating_sub(close_w + 1),
        y: r.y,
        width: close_w,
        height: 1,
    };
    put(buf, close.x, close.y, &close_chip, r.right() - 1);
    let mut row = r.y + 1 + title_lines.len() as u16;
    if let Some(b) = &button {
        put(buf, tx, row, b, r.right() - 1);
        row += 1;
    }
    let detail_style = Style::default().fg(ink(t.text, bg)).bg(bg);
    let preview_style = Style::default()
        .fg(ink(t.muted, bg))
        .bg(bg)
        .add_modifier(Modifier::ITALIC);
    let wrapped = detail_lines
        .into_iter()
        .map(|l| (l, detail_style))
        .chain(preview_lines.into_iter().map(|l| (l, preview_style)));
    for (line, style) in wrapped {
        put(buf, tx, row, &[seg(line, style)], r.right() - 1);
        row += 1;
    }
    if let Some(term) = about {
        hit(app, r, ScreenHit::Session(app.session_key(term)));
    } else if restart {
        hit(app, r, ScreenHit::Restart);
    } else {
        hit(app, r, ScreenHit::Noop);
    }
    // A question waiting on you gets its two answers as buttons on the bottom border: go and
    // answer it, or leave it for later (Esc does the same).
    if needs
        && let Some(term) = about
    {
        let on = |c: Color| pill_label(look, ink(c, bg), bg, ink(t.ink_on(c), c));
        let answer = on(t.done)("Answer");
        let later = on(t.err)("Later esc");
        let y = r.bottom().saturating_sub(1);
        let ax = r.x + 2;
        let lx = ax + segs_width(&answer) + 1;
        put(buf, ax, y, &answer, r.right() - 1);
        put(buf, lx, y, &later, r.right() - 1);
        let rect = |x: u16, w: u16| Rect {
            x,
            y,
            width: w,
            height: 1,
        };
        hit(
            app,
            rect(ax, segs_width(&answer)),
            ScreenHit::Session(app.session_key(term)),
        );
        hit(app, rect(lx, segs_width(&later)), ScreenHit::ToastClose);
    }
    hit(app, close, ScreenHit::ToastClose);
    border_on_ground(buf, r, bg, &behind_top);
}

/// A bold label on a pill of `bg`, ends rounded on `outer` when the look rounds them.
fn pill_label<'a>(look: Look, bg: Color, outer: Color, fg: Color) -> impl Fn(&'a str) -> Vec<Seg> {
    move |label| {
        pill(
            look,
            vec![seg(
                label.to_string(),
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            )],
            bg,
            outer,
        )
    }
}

fn toast_rect(area: Rect, width: u16, height: u16, slide: f32) -> Option<Rect> {
    let settled_x = area.right().saturating_sub(width + 2);
    let x = settled_x + ((area.right().saturating_sub(settled_x)) as f32 * slide).round() as u16;
    let rect = Rect {
        x,
        y: area.bottom().saturating_sub(height + TOAST_ABOVE_INPUT),
        width: area.right().saturating_sub(x).min(width),
        height,
    };
    (rect.width > 0).then_some(rect)
}

#[cfg(test)]
mod tests {
    use super::toast_rect;
    use ratatui::layout::Rect;

    #[test]
    fn toast_has_no_drawable_rectangle_while_sliding_in_from_beyond_the_right_edge() {
        assert_eq!(toast_rect(Rect::new(0, 0, 80, 24), 40, 4, 1.0), None);
    }

    #[test]
    fn toast_settles_inside_the_screen() {
        assert_eq!(
            toast_rect(Rect::new(0, 0, 80, 24), 40, 4, 0.0),
            Some(Rect::new(38, 15, 40, 4))
        );
    }
}
