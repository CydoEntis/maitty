//! The main screen: the sidebar, the panes (the top-left one carrying the tabs in its title),
//! the sheet, the mode bar along the bottom and the toast.

use super::*;
use crate::brand::{APP_COMMAND};
use super::glyphs::{self, Kind};

// ---- main screen ---------------------------------------------------------------------------

/// The sidebar's width on a screen `width` wide: 40 from `WIDE` columns, 34 from `NARROW`,
/// else 26.
pub(in crate::client) fn side_w(width: u16) -> u16 {
    if width >= WIDE {
        40
    } else if width >= NARROW {
        34
    } else {
        26
    }
}

/// The narrowest the sidebar (sliding) or the sheet is drawn; below it, it's only space.
const SIDE_DRAWN_FROM: u16 = 12;
pub(in crate::client) const SHEET_DRAWN_FROM: u16 = 24;
/// The narrowest a pane is drawn while it grows in.
const PANE_DRAWN_FROM: u16 = 8;

/// Below this many columns the layout tightens: a slimmer sidebar, and a sheet takes the
/// whole pane column instead of docking beside the panes.
pub(in crate::client) const NARROW: u16 = 140;
/// From this many columns the sidebar is at its widest.
const WIDE: u16 = 200;

/// Where everything goes: the sidebar, the pane column and the sheet.
pub(in crate::client) struct Grid {
    pub side: Rect,
    pub panes: Rect,
    /// Between the sidebar and the panes (drag it to resize).
    pub edge: Rect,
    /// The sheet docked right of the panes, when one is open (else empty).
    pub sheet: Rect,
}

/// Most the padding round the screen may be, in cells.
const PADDING_MAX: u16 = 2;

/// The window less the padding setting all round.
fn padded(app: &App, area: Rect) -> Rect {
    let m = app.cfg.ui.padding.parse().unwrap_or(1).min(PADDING_MAX);
    Rect { x: area.x + m, y: area.y + m, width: area.width.saturating_sub(2 * m), height: area.height.saturating_sub(2 * m) }
}

/// Rows along the bottom under the layout: the mode bar, and in Hairline the status strip over it.
fn bottom_rows(app: &App) -> u16 {
    if Look::of(&app.cfg.ui).hairline { 2 } else { 1 }
}

/// The room the screen is laid out in: inside the padding, above the mode bar (and the strip).
pub(in crate::client) fn framed(app: &App, area: Rect) -> Rect {
    let r = padded(app, area);
    Rect { height: r.height.saturating_sub(bottom_rows(app)), ..r }
}

/// Hairline's status strip: the row over the mode bar.
fn strip_row(app: &App, area: Rect) -> Option<Rect> {
    let bar = mode_bar_row(app, area);
    (Look::of(&app.cfg.ui).hairline && bar.y > padded(app, area).y).then(|| Rect { y: bar.y - 1, ..bar })
}

/// The mode bar's row (ADR 0024): the bottom row inside the padding.
pub(in crate::client) fn mode_bar_row(app: &App, area: Rect) -> Rect {
    let r = padded(app, area);
    Rect { y: r.bottom().saturating_sub(1), height: r.height.min(1), ..r }
}

/// Where popups and their dimming go: the window above the mode bar (and the strip), so the
/// mode stays in view and readable under any of them.
pub(in crate::client) fn above_bar(app: &App, area: Rect) -> Rect {
    let top = strip_row(app, area).unwrap_or_else(|| mode_bar_row(app, area)).y;
    Rect { height: top.saturating_sub(area.y), ..area }
}

/// The grid in `area` (already `framed`): the sidebar the full height at one side, then the
/// pane column from its top row. Cards floating leave the gap setting between them, tiled none (the two
/// borders sit side by side); Flat starts the panes on the sidebar's last column, so one `│`
/// parts them. A width you dragged the sidebar to wins over the default.
pub(in crate::client) fn grid(app: &App, area: Rect) -> Grid {
    let look = Look::of(&app.cfg.ui);
    let default_w = if look.hairline { hairline::SIDE_W } else { side_w(area.width) };
    let full = app.screen.saved.side_w.unwrap_or(default_w).clamp(SIDE_MIN, SIDE_MAX).min(area.width / 2);
    let sw = (full as f32 * app.side_frac).round() as u16;
    // Hairline keeps a column for the `│` between the sidebar and the panes.
    let gap = if sw == 0 { 0 } else if look.hairline { 1 } else { look.gap };
    let (side, col) = if app.cfg.ui.sidebar_position == "right" {
        let side = Rect { x: area.right().saturating_sub(sw), width: sw, ..area };
        (side, Rect { width: area.width.saturating_sub(sw + gap), ..area })
    } else {
        let overlap = u16::from(look.flat && sw > 0);
        let x = area.x + sw + gap - overlap;
        (Rect { width: sw, ..area }, Rect { x, width: area.right().saturating_sub(x), ..area })
    };
    // Collapsed, the sessions are chips on row 0 and the panes (with their tabs) start below it.
    let col = if strip_on(app) { Rect { y: col.y + 1, height: col.height.saturating_sub(1), ..col } } else { col };
    let edge = if side.x > col.x { Rect { x: col.right(), width: gap.max(1), ..area } } else { Rect { x: col.x.saturating_sub(gap), width: gap.max(1), ..area } };
    let (sheet, panes) = match open_sheet(app) {
        Some(kind) => split_for_sheet(area.width, col, look, app.sheet_frac, wanted_w(app, kind, area.width)),
        None => (Rect::default(), col),
    };
    Grid { side, panes, edge, sheet }
}

/// Where the side sheet is this frame, the widest it could be, and the screen's width, for its
/// edge and its `[` `]` keys. A sheet that takes the whole pane column, or none, can't be
/// sized: its widest is 0.
fn note_sheet(app: &mut App, g: &Grid, screen_w: u16) {
    let look = Look::of(&app.cfg.ui);
    let col = Rect { width: g.sheet.right().saturating_sub(g.panes.x), ..g.panes };
    let sizable = open_sheet(app).is_some() && screen_w >= NARROW && widest(col, look) >= SHEET_MIN;
    app.screen.sheet_rect = g.sheet;
    app.screen.sheet_widest = if sizable { widest(col, look) } else { 0 };
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
    let edge = Rect { x, width: g.sheet.x.saturating_sub(g.panes.right()).max(1), ..g.sheet };
    if app.screen.drag == Some(Drag::Sheet) || hovered(app, edge) {
        let ex = edge.x + edge.width / 2;
        for yy in g.sheet.top() + 1..g.sheet.bottom().saturating_sub(1) {
            if let Some(px) = buf.cell_mut((ex, yy)) {
                px.set_symbol("│").set_style(Style::default().fg(t.accent).bg(t.bg));
            }
        }
    }
    hit(app, edge, ScreenHit::SheetEdge);
}

/// The sessions are collapsed into the strip on row 0 (`z`), not hidden (`Z`).
pub(in crate::client) fn strip_on(app: &App) -> bool {
    !app.sidebar && !app.side_hidden
}

/// Draw the main screen; returns the pane area.
pub(in crate::client) fn draw(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) -> Rect {
    // How far the sidebar and the sheet are in this frame (they slide when they change).
    popover::refresh_receipts(app);
    let on = app.motion_on();
    app.side_frac = app.motion.side(app.sidebar, on);
    app.sheet_frac = app.motion.sheet(open_sheet(app).is_some(), on);
    let model = app.screen_model();
    app.screen.wt_keys.clear();
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
        if Look::of(&app.cfg.ui).hairline {
            hairline::draw_side(app, f.buffer_mut(), g.side, &model, t);
            hairline::draw_side_rule(f.buffer_mut(), g.edge, t);
        } else {
            draw_side(app, f.buffer_mut(), g.side, &model, t);
        }
        // The edge beside the sidebar: drag it. It lights up while you point at it.
        if app.screen.drag == Some(Drag::Side) || hovered(app, g.edge) {
            let ex = g.edge.x + g.edge.width / 2;
            for yy in g.side.top() + 1..g.side.bottom().saturating_sub(1) {
                if let Some(px) = f.buffer_mut().cell_mut((ex, yy)) {
                    px.set_symbol("│").set_style(Style::default().fg(t.accent).bg(t.bg));
                }
            }
        }
        hit(app, g.edge, ScreenHit::SideEdge);
    } else if strip_on(app) {
        rail::draw(app, f.buffer_mut(), room, &model, t);
    } else {
        // Hidden: nothing to point at, nothing to move a cursor over.
        app.screen.side_rect = Rect::default();
        app.screen.visible.clear();
        app.screen.side_items.clear();
        app.screen.row_y.clear();
    }
    if g.panes.width > 0 {
        draw_main(app, f, g.panes, &model, t);
    }
    if let Some(kind) = open_sheet(app).filter(|_| g.sheet.width >= SHEET_DRAWN_FROM) {
        draw_sheet(app, f.buffer_mut(), g.sheet, kind, t);
        draw_sheet_edge(app, f.buffer_mut(), &g, t);
    }
    if let Some(row) = strip_row(app, area) {
        hairline::draw_strip(app, f.buffer_mut(), row, &model, t);
    }
    draw_mode_bar(app, f.buffer_mut(), mode_bar_row(app, area), t);
    app.screen.crumb_x = g.panes.x + 1;
    // A popup (`<app> popup`) floats over everything, the rest dimmed.
    let above = above_bar(app, area);
    match app.mode.clone() {
        Mode::Compose(c) if c.inbox.is_none() => draw_compose_pop(app, f.buffer_mut(), above, g.side, &c, t),
        Mode::Why { term, .. } => draw_why_pop(app, f.buffer_mut(), above, g.side, term, t),
        Mode::Helpers { term, .. } => draw_helpers_pop(app, f.buffer_mut(), above, g.side, term, t),
        _ => {}
    }
    if let Some(term) = app.popup() {
        let whole = above;
        dim_all(app, f.buffer_mut(), whole, t);
        let w = (whole.width * 4 / 5).max(40).min(whole.width);
        let h = (whole.height * 3 / 4).max(12).min(whole.height);
        let r = Rect { x: whole.x + (whole.width - w) / 2, y: whole.y + (whole.height - h) / 2, width: w, height: h };
        draw_session(app, f, r, term, true, &model, t);
    }
    draw_toast(app, f.buffer_mut(), room, t);
    g.panes
}

pub(in crate::client) fn find(model: &[Proj], term: TermId) -> Option<(&Proj, &Wt, &Session)> {
    model.iter().find_map(|p| p.wts.iter().find_map(|w| w.sessions.iter().find(|s| s.term == term).map(|s| (p, w, s))))
}


pub(in crate::client) fn hline(buf: &mut Buffer, x: u16, y: u16, w: u16, t: &Theme, bg: Color) {
    for i in 0..w {
        if let Some(px) = buf.cell_mut((x + i, y)) {
            px.set_symbol("─").set_style(Style::default().fg(t.line).bg(bg));
        }
    }
}

pub(in crate::client) fn draw_main(app: &mut App, f: &mut Frame, area: Rect, model: &[Proj], t: &Theme) {
    let flat_splits_allowed = !Look::of(&app.cfg.ui).flat || area.width >= 150;
    let Some(focus) = app.focused() else {
        let nk = k(app, &Action::ShellHere);
        put(f.buffer_mut(), area.x + 4, area.y + 3, &[seg(format!("Nothing open. Press {} {nk} for a new shell.", app.keymap.prefix.to_string().replace("C-", "Ctrl+")), Style::default().fg(t.muted))], area.right());
        return;
    };
    // Which tab was on screen last, per session.
    app.screen.tick += 1;
    let (tick, cur) = (app.screen.tick, app.screen.tab);
    if let Some(tab) = app.screen.tabs.get_mut(cur) {
        tab.used = tick;
    }
    let look = Look::of(&app.cfg.ui);
    // Between cards: `gap` rows stacked, as many columns side by side.
    let (gap_x, gap_y) = (look.gap, look.gap);
    let inset = |r: Rect| -> Rect {
        let mut r = r;
        if r.right() < area.right() {
            r.width = r.width.saturating_sub(gap_x);
        }
        if r.bottom() < area.bottom() {
            r.height = r.height.saturating_sub(gap_y);
        }
        r
    };
    app.screen.dividers.clear();
    let layout = app
        .screen
        .tabs
        .get(app.screen.tab)
        .map(|tab| tab.layout.clone())
        .filter(|l| l.contains(focus) && l.leaves().len() > 1)
        .filter(|_| flat_splits_allowed);
    let Some(layout) = layout else {
        app.screen.leaf_rects = vec![(focus, area)];
        draw_session(app, f, area, focus, true, model, t);
        return;
    };
    let leaves = layout.leaves();
    // A pane that has just opened grows out of its edge (the split's share eases to where it
    // goes); one too thin to draw yet is only space.
    let mut layout = layout;
    let ratio = layout.ratio_at(&[]).unwrap_or(0.5);
    let ids: Vec<u64> = leaves.iter().map(|l| u64::from(*l)).collect();
    let on = app.motion_on();
    let shown = app.motion.split(app.screen.tab as u64, &ids, ratio, on);
    if (shown - ratio).abs() > f32::EPSILON {
        layout.set_ratio(&[], shown);
    }
    let rects = layout.rects(area);
    app.screen.leaf_rects = rects.clone();
    for (id, r) in rects {
        if r.width >= PANE_DRAWN_FROM && r.height >= 3 {
            draw_session(app, f, inset(r), id, id == focus, model, t);
        }
    }
    for (i, (sa, horizontal, path)) in layout.splits(area).into_iter().enumerate() {
        let ratio = layout.ratio_at(&path).unwrap_or(0.5);
        let div = crate::layout::Node::divider(sa, horizontal, ratio);
        // The gap between the two grabs it, and a cell either side of its middle line (one
        // column is hard to hit); it lights up while you point at it or drag.
        let grab = if horizontal {
            let g = gap_x.max(1);
            Rect { x: div.x.saturating_sub(g), width: g + 2, ..div }
        } else if look.hairline {
            // Hairline draws the `─` on the upper pane's last row: that row is the grab.
            div
        } else {
            let h = gap_y.max(1);
            Rect { y: div.y.saturating_sub(h), height: h, ..div }
        };
        if app.screen.drag == Some(Drag::Divider(i)) || hovered(app, grab) {
            let buf = f.buffer_mut();
            let (lx, ly) = (grab.x + 1, grab.y + grab.height / 2);
            for yy in grab.top()..grab.bottom() {
                for xx in grab.left()..grab.right() {
                    let on = if horizontal { xx == lx } else { yy == ly };
                    if on && let Some(px) = buf.cell_mut((xx, yy)) {
                        px.set_symbol(if horizontal { "│" } else { "─" }).set_style(Style::default().fg(t.accent).bg(t.bg));
                    }
                }
            }
        }
        hit(app, grab, ScreenHit::Divider(i));
        app.screen.dividers.push((sa, horizontal, path));
    }
    draw_pane_drag(app, f.buffer_mut(), model, t);
}

/// A pane dragged by its title: the pane it would swap with gets lit lines and a "swap here"
/// pill, and the dragged pane's name follows the pointer.
fn draw_pane_drag(app: &App, buf: &mut Buffer, model: &[Proj], t: &Theme) {
    let Some(Drag::Pane { from, over, at }) = app.screen.drag else { return };
    let look = Look::of(&app.cfg.ui);
    if let Some((_, r)) = over.and_then(|o| app.pane_frames.iter().find(|(id, _)| *id == o)) {
        light_lines(buf, *r, look.hairline, t.accent);
        let pill = pill(look, vec![seg("⇄ swap here", Style::default().fg(t.acc_ink).add_modifier(Modifier::BOLD))], t.accent, t.bg);
        let w = segs_width(&pill);
        // Clear of the dragged name, which sits on the pointer's row.
        let y = r.y + r.height / 2;
        let y = if y == at.y { y + 2 } else { y };
        put(buf, r.x + r.width.saturating_sub(w) / 2, y.min(r.bottom().saturating_sub(2)), &pill, r.right());
    }
    let ghost = pill(look, vec![seg(format!("⇄ {}", session_name(app, model, from)), Style::default().fg(t.strong).add_modifier(Modifier::BOLD))], t.btn, t.bg);
    put(buf, at.x.saturating_add(1), at.y, &ghost, buf.area.right());
}

/// Light the frame lines of the pane at `r` (its border, or Hairline's rule under the title).
fn light_lines(buf: &mut Buffer, r: Rect, hairline: bool, colour: Color) {
    let r = r.intersection(buf.area);
    let edge = |x: u16, y: u16| y == r.y || (hairline && y == r.y + 1) || y + 1 == r.bottom() || x == r.x || x + 1 == r.right();
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            if let Some(px) = buf.cell_mut((x, y))
                && edge(x, y)
                && px.symbol().chars().next().is_some_and(|c| ('\u{2500}'..='\u{257F}').contains(&c))
            {
                px.set_fg(colour);
            }
        }
    }
}

/// The name you gave a tab, if any. "tab 2" (what an earlier version named new tabs) isn't
/// one: the chip shows the number already.
pub(in crate::client) fn tab_name(tab: &ScreenTab) -> Option<String> {
    let n = tab.name.trim();
    let generic = n.strip_prefix("tab ").is_some_and(|d| d.chars().all(|c| c.is_ascii_digit()));
    (!n.is_empty() && !generic).then(|| n.to_string())
}

/// Leader mode is on: waiting for the key, or showing the key map.
pub(in crate::client) fn armed(app: &App) -> bool {
    matches!(app.mode, Mode::Prefix { .. } | Mode::KeyMap(_))
}

/// Shortcut mode is held while a popup or view it opened has the keys (ADR 0021). Normal with
/// no view is the moment before it comes back, when keys still reach the pane.
pub(in crate::client) fn shortcut_waits(app: &App) -> bool {
    app.shortcut_held && !armed(app) && (app.mode != Mode::Normal || app.view_has_keys())
}

/// What a session is called on its pane: its sidebar name, else (a pane in another's tab or
/// split has no row) your name for it or what runs in it.
pub(in crate::client) fn session_name(app: &App, model: &[Proj], term: TermId) -> String {
    if let Some((_, _, s)) = find(model, term) {
        return s.name.clone();
    }
    let Some(info) = app.snap.terms.get(&term) else { return String::new() };
    if !info.label.trim().is_empty() {
        return info.label.trim().to_string();
    }
    match &info.agent {
        Some(a) => a.clone(),
        None if info.is_shell() => "shell".into(),
        None => info.display_name(),
    }
}

/// A name chip: ` name `, in the accent while its pane has the keys, else raised and bold.
fn name_chip(name: &str, lit: bool, t: &Theme) -> Seg {
    let (bg, fg) = if lit { (t.accent, t.acc_ink) } else { (t.btn, t.strong) };
    seg(format!(" {name} "), Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD))
}

/// The tabs of the session on screen, as the top-left pane's title along `row`: a number chip
/// and a name chip each (the tab's name, else its focused session's), the current one in the
/// accent, then a + chip that opens a tab. The current tab outranks the rest: the other tabs,
/// before and after it, drop before it shortens its name. A click on one picks it. Grid passes
/// the pane's `state`, drawn at the front of the current tab; returns where its glyph sits.
fn tab_strip(app: &mut App, model: &[Proj], row: Rect, lit: bool, state: Option<&ChipState>, t: &Theme) -> (Vec<Seg>, Option<Rect>) {
    let look = Look::of(&app.cfg.ui);
    let ground = title_ground(look, t);
    let (y, max_x) = (row.y, row.right());
    let mut out = Vec::new();
    let mut at = row.x;
    let tabs = app.session_tabs();
    let top_left = app.screen.leaf_rects.iter().min_by_key(|(_, r)| (r.y, r.x)).map(|(id, _)| *id);
    let names: Vec<String> = tabs
        .iter()
        .map(|&i| {
            let name = tab_name(&app.screen.tabs[i]).unwrap_or_else(|| session_name(app, model, app.screen.tabs[i].focus));
            match top_left.filter(|_| i == app.screen.tab) {
                Some(term) => numbered(app, term, &name),
                None => name,
            }
        })
        .collect();
    let closable = tabs.len() > 1;
    let current = Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD);
    let cap = u16::from(look.round_tabs);
    let chips = |n: usize, on: bool, name: &str| {
        let number = if on { current } else { Style::default().fg(t.text).bg(t.btn) };
        let label = if on { name_chip(name, lit, t) } else { seg(format!(" {name} "), Style::default().fg(t.text).bg(t.card)) };
        let mut pair: Vec<Seg> = state.filter(|_| on).map(|s| state_seg(s, number, t)).into_iter().collect();
        pair.extend([seg(format!(" {} ", n + 1), number), label]);
        if closable {
            let (fg, bg) = if on { if lit { (t.acc_ink, t.accent) } else { (t.strong, t.btn) } } else { (t.muted, t.card) };
            pair.push(seg("✕ ", Style::default().fg(fg).bg(bg)));
        }
        if look.round_tabs { round_ends(pair, ground) } else { pair }
    };
    let alone_w = state.map_or(0, |s| segs_width(&state_alone(s, current, look, t)));
    // The room the tabs before the current one leave it: its whole pair, else its shortest,
    // else its glyph, so it never shortens or drops while another tab shows.
    let current_w = |shorten: bool| {
        let n = tabs.iter().position(|&i| i == app.screen.tab);
        n.map_or(0, |n| segs_width(&chips(n, true, &if shorten { cut_label(&names[n], 2) } else { names[n].clone() })))
    };
    let keep = [current_w(false), current_w(true), alone_w].into_iter().find(|w| row.x + w <= max_x).unwrap_or(alone_w);
    let (mut all_fit, mut placed, mut crowded) = (true, false, false);
    let mut why = None;
    for (n, &i) in tabs.iter().enumerate() {
        let on = i == app.screen.tab;
        let mut pair = chips(n, on, &names[n]);
        let rest = segs_width(&pair).saturating_sub(names[n].width() as u16);
        if on && at + segs_width(&pair) > max_x && max_x > at + rest + 1 {
            pair = chips(n, on, &cut_label(&names[n], (max_x - at - rest) as usize));
        }
        let w = segs_width(&pair);
        let reserve = if on || placed { 0 } else { keep };
        if crowded && !on || at + w + reserve > max_x {
            all_fit = false;
            if on || placed {
                break;
            }
            crowded = true;
            continue;
        }
        hit(app, Rect { x: at, y, width: w, height: 1 }, ScreenHit::TabPick(i));
        if closable {
            let close_w = if look.round_tabs { 3 } else { 2 };
            hit(app, Rect { x: at + w - close_w, y, width: 2, height: 1 }, ScreenHit::CloseTab(i));
        }
        if on {
            placed = true;
            why = state.map(|s| Rect { x: at + cap, y, width: segs_width(&[state_seg(s, current, t)]), height: 1 });
        }
        out.extend(pair);
        at += w;
    }
    // The chip that opens a tab, a cell clear of the tabs: it drops first, so never shows
    // without every tab before it.
    if !all_fit {
        // The current tab did not fit even shortened: its glyph alone keeps the state in view.
        if !placed
            && let Some(s) = state
            && at + alone_w <= max_x
        {
            why = Some(Rect { x: at + cap, y, width: alone_w - 2 * cap, height: 1 });
            out.extend(state_alone(s, current, look, t));
        }
        return (out, why);
    }
    let mut plus = vec![seg(" + ", Style::default().fg(t.strong).bg(t.btn).add_modifier(Modifier::BOLD))];
    if look.round_tabs {
        plus = round_ends(plus, ground);
    }
    let w = segs_width(&plus);
    if at + 1 + w <= max_x {
        hit(app, Rect { x: at + 1, y, width: w, height: 1 }, ScreenHit::NewTab);
        out.push(seg(" ", Style::default()));
        out.extend(plus);
    }
    (out, why)
}

/// A Grid pane's state, shown only as its glyph inside the chip that names the pane.
struct ChipState {
    glyph: String,
    blocked: bool,
}

/// What a Grid pane's chip shows of its state: an agent working or needing you. Done and
/// idle show nothing.
fn chip_state(app: &App, info: &TermInfo) -> Option<ChipState> {
    match (info.agent.is_some(), info.status) {
        (true, Status::Blocked) => Some(ChipState { glyph: "▲".into(), blocked: true }),
        (true, Status::Working) => Some(ChipState { glyph: glyph(app, Status::Working), blocked: false }),
        _ => None,
    }
}

/// The state's glyph at the front of a chip drawn in `chip`. Working takes the chip's own ink,
/// which reads on any chip colour. Needs you gets its own `blocked` ground; on a chip whose
/// colour is close to `blocked` (a pane colour or an accent near amber) that patch would melt
/// into it, so there it turns over: the glyph in `blocked` on the ink chosen to read on
/// `blocked`, which by the same token stands out from a chip close to it.
fn state_seg(s: &ChipState, chip: Style, t: &Theme) -> Seg {
    if !s.blocked {
        return seg(format!(" {}", s.glyph), chip);
    }
    let melts = chip.bg.is_some_and(|bg| crate::theme::contrast(bg, t.blocked) < PATCH_CONTRAST);
    let (fg, bg) = if melts { (t.blocked, t.ink_on(t.blocked)) } else { (t.ink_on(t.blocked), t.blocked) };
    let style = Style::default().fg(fg).bg(bg);
    seg(format!(" {} ", s.glyph), style.add_modifier(Modifier::BOLD))
}

/// The least contrast between a chip and the needs-you patch on it for the patch's edge to
/// show: the default theme's fourth Grid colour is 1.17 from `blocked`, which reads as one.
const PATCH_CONTRAST: f64 = 1.5;

/// The state's glyph as a chip of its own, for a title too narrow for the chip it belongs in.
fn state_alone(s: &ChipState, chip: Style, look: Look, t: &Theme) -> Vec<Seg> {
    let (text, st) = state_seg(s, chip, t);
    let segs = vec![seg(if s.blocked { text } else { format!("{text} ") }, st)];
    let ground = title_ground(look, t);
    if look.round_tabs { round_ends(segs, ground) } else { segs }
}

/// A session's mark on its pane: glyph and word, bold in its colour (working is the
/// Appearance spinner). A shell has none.
fn pane_mark(app: &App, info: &TermInfo, t: &Theme) -> Option<PaneMark> {
    info.agent.as_ref()?;
    let set = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let spinner = glyph(app, Status::Working);
    let step = if app.motion_on() { check::ring_step(app.spinner_frame(), set.ring.len()) } else { 0 };
    let m = Kind::of(info.status, info.check(), info.asleep).mark(set, t, step, &spinner);
    Some(PaneMark { glyph: m.glyph.to_string(), word: m.word.to_string(), style: Style::default().fg(m.color).add_modifier(Modifier::BOLD) })
}

/// A session's mark on its pane, whose word can go while its glyph stays.
struct PaneMark {
    glyph: String,
    word: String,
    style: Style,
}

/// How wide the card's ✕ is drawn: with its gap after other right-hand segments.
fn close_w(look: Look, c: &Card) -> u16 {
    if c.close.is_some() { close_width(look, !c.right.is_empty()) } else { 0 }
}

/// The right of a pane's title in a narrow card: the load note goes first, then the mark's
/// word, then the ✕; the mark's glyph is the last thing to go, since status must stay in view.
/// Returns where the mark sits (a click opens why).
fn fit_right(look: Look, r: Rect, c: &mut Card, note: Option<Seg>, mark: Option<PaneMark>) -> Option<Rect> {
    let end = right_end(look, r);
    // The card draws its right-hand segments only with two cells to spare after the title's start.
    let avail = end.saturating_sub(title_x(look, r) + 2);
    let full = mark.as_ref().map(|m| seg(format!("{} {}", m.glyph, m.word), m.style));
    let short = mark.as_ref().map(|m| seg(m.glyph.clone(), m.style));
    let tries = [(note.clone(), full.clone(), true), (None, full, true), (None, short.clone(), true), (None, short, false)];
    let close = c.close;
    for (note, mark, closes) in tries {
        c.right = note.into_iter().chain(mark.clone()).collect();
        c.close = if closes { close } else { None };
        if segs_width(&c.right) + close_w(look, c) <= avail {
            return mark.map(|m| {
                let w = m.0.width() as u16;
                Rect { x: end.saturating_sub(close_w(look, c) + w), y: r.y, width: w, height: 1 }
            });
        }
    }
    c.right.clear();
    None
}

/// A pane as a card. Its title: the tabs of the session on screen when it's the top-left pane
/// of the tab, else one chip with its session's name; on the right, the session's mark (it
/// opens why) and a ✕ that closes the pane. The terminal sits inside, 3 columns and 2 rows in.
/// Unfocused cards fade; one that needs you keeps its amber.
/// How far an unfocused Grid pane's colour fades toward the ground.
const GRID_UNFOCUSED_FADE: f32 = 0.42;

/// How far an unfocused Grid pane's number and name chips fade toward the ground.
const GRID_TAG_FADE: f32 = 0.28;

/// The narrowest a tab chip pair gets before it drops: ` 1 ` and ` x… `.
const SHORTEST_TAB: u16 = 7;

/// A pane's place among the tab's panes (0 first): top-left first, left to right and top to
/// bottom. Grid colours panes by it, and every look numbers them by it.
fn grid_index(app: &App, term: TermId) -> usize {
    let mut order: Vec<(u16, u16, TermId)> = app.screen.leaf_rects.iter().map(|(id, r)| (r.y, r.x, *id)).collect();
    order.sort_unstable();
    order.iter().position(|(.., id)| *id == term).unwrap_or(0)
}

/// The number a pane shows in its title: its place in the tab in reading order, from 1. Only
/// while the tab shows two panes or more, so a lone pane's title stays as it was.
pub(in crate::client) fn pane_number(app: &App, term: TermId) -> Option<usize> {
    let split = app.screen.leaf_rects.len() > 1 && app.screen.leaf_rects.iter().any(|(id, _)| *id == term);
    split.then(|| grid_index(app, term) + 1)
}

/// Between a pane's number and its name in a title chip.
const NUMBER_SEP: &str = " · ";

/// A pane's name with its number in front (`2 · claude`) while it has one.
pub(in crate::client) fn numbered(app: &App, term: TermId, name: &str) -> String {
    match pane_number(app, term) {
        Some(n) => format!("{n}{NUMBER_SEP}{name}"),
        None => name.to_string(),
    }
}

/// `label` cut to `room` columns. A numbered name keeps its number: the name is cut first
/// (`2 · warm-f…`), then the ` · ` (`2…`), leaving the number alone.
fn cut_label(label: &str, room: usize) -> String {
    let Some((number, name)) = label.split_once(NUMBER_SEP).filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) else {
        return truncate(label, room);
    };
    let front = number.width() + NUMBER_SEP.width();
    match room {
        r if r >= front + 2 => format!("{number}{NUMBER_SEP}{}", truncate(name, r - front)),
        r if r > number.width() => format!("{number}…"),
        r => truncate(number, r),
    }
}

/// A Grid pane's colour, by its place; they repeat from the fifth.
fn grid_color(app: &App, term: TermId, t: &Theme) -> Color {
    let colors = t.grid_colors();
    colors[grid_index(app, term) % colors.len()]
}

/// Grid's top border: the chip that names the pane, with the state's glyph at its front: the
/// current tab on the top-left pane (its other tabs and the + around it), else one chip with the
/// session's name, each with the pane's number before the name while the tab is split. No state
/// words, and the name is not repeated on the right. A narrow pane drops the
/// load note, then shortens the name, then keeps only the glyph, then drops the ✕. Returns where
/// the glyph sits, which opens why.
fn grid_title(app: &mut App, model: &[Proj], info: &TermInfo, focused: bool, r: Rect, c: &mut Card, t: &Theme) -> Vec<Rect> {
    let look = Look::of(&app.cfg.ui);
    let own = grid_color(app, info.id, t);
    let chip_style = if focused {
        Style::default().bg(own).fg(t.ink_on(own)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(crate::theme::mix(own, t.bg, GRID_TAG_FADE)).fg(t.strong)
    };
    let state = chip_state(app, info);
    let x = title_x(look, r);
    let room = right_end(look, r).saturating_sub(x);
    let note_w = segs_width(&c.right);
    let caps = if look.round_tabs { 2 } else { 0 };
    let glyph_w = state.as_ref().map_or(0, |s| segs_width(&[state_seg(s, chip_style, t)]));
    let alone_w = state.as_ref().map_or(0, |s| segs_width(&state_alone(s, chip_style, look, t)));
    let top_left = app.screen.leaf_rects.iter().min_by_key(|(_, lr)| (lr.y, lr.x)).is_some_and(|(id, _)| *id == info.id);
    let tab_close = if app.session_tabs().len() > 1 { 2 } else { 0 };
    // The chip that carries the state at its shortest: the current tab, else the pane's name.
    let chip_min = glyph_w + caps + if top_left { SHORTEST_TAB + tab_close } else { SHORTEST_NAME };
    // The two cells the card keeps before the right-hand segments, what is on the right, the ✕
    // and the named chip's share. A square ✕ keeps the room it always had, as if after other
    // segments, so square titles shorten where they did; a capped ✕ costs what it draws.
    let close_w = |right: u16, close: bool| if close { close_width(look, right > 0 || !look.caps) } else { 0 };
    let fits = |right: u16, close: bool, body: u16| 2 + right + close_w(right, close) + body <= room;
    if !fits(note_w, true, chip_min) {
        c.right.clear();
    }
    let right_w = segs_width(&c.right);
    // Then the ✕: the state's glyph is the last to go, since status must stay in view.
    let close = fits(right_w, true, alone_w);
    if !close {
        c.close = None;
    }
    let rest = right_w + close_w(right_w, close);
    let title_end = right_end(look, r).saturating_sub(rest + 2);
    let (named, glyph_at) = if top_left {
        tab_strip(app, model, Rect { x, y: r.y, width: title_end.saturating_sub(x), height: 1 }, focused, state.as_ref(), t)
    } else {
        let name = numbered(app, info.id, &session_name(app, model, info.id));
        let (segs, glyph_w) = name_with_state(&name, state.as_ref(), chip_style, title_end.saturating_sub(x), look, t);
        let cap = u16::from(look.round_tabs);
        (segs, (glyph_w > 0).then_some(Rect { x: x + cap, y: r.y, width: glyph_w, height: 1 }))
    };
    c.chips = named;
    glyph_at.into_iter().collect()
}

/// The narrowest a pane's name chip gets: ` x… `.
const SHORTEST_NAME: u16 = 4;

/// A Grid pane's name as one chip `room` wide at most, the state's glyph at its front; too
/// narrow for any of the name, the glyph alone. Returns the chip and how wide its glyph is
/// (0 with no state), the glyph starting after the chip's cap.
fn name_with_state(name: &str, state: Option<&ChipState>, chip: Style, room: u16, look: Look, t: &Theme) -> (Vec<Seg>, u16) {
    let caps = if look.round_tabs { 2 } else { 0 };
    let front: Vec<Seg> = state.map(|s| state_seg(s, chip, t)).into_iter().collect();
    let front_w = segs_width(&front);
    if room >= front_w + caps + SHORTEST_NAME {
        let mut segs = front;
        segs.push(seg(format!(" {} ", cut_label(name, (room - front_w - caps - 2) as usize)), chip));
        return (if look.round_tabs { round_ends(segs, title_ground(look, t)) } else { segs }, front_w);
    }
    match state {
        Some(s) => {
            let alone = state_alone(s, chip, look, t);
            let w = segs_width(&alone);
            if room >= w { (alone, w - caps) } else { (Vec::new(), 0) }
        }
        None => (Vec::new(), 0),
    }
}

pub(in crate::client) fn draw_session(app: &mut App, f: &mut Frame, r: Rect, term: TermId, focused: bool, model: &[Proj], t: &Theme) {
    let Some(info) = app.snap.terms.get(&term).cloned() else { return };
    let look = Look::of(&app.cfg.ui);
    let st = info.status;
    // While the sidebar has the keys, no pane shows as focused.
    // Exactly one lit edge: an open sheet has the keys, so the pane gives up its lit border.
    let focused = focused && !side_focused(app) && !sheet_has_keys(app);
    let scrolled = app.parsers.get(&term).map(|p| p.screen().scrollback()).filter(|n| *n > 0);
    // Answer buttons only for a question asked through the app (ask-human), which has nowhere
    // else to be answered: an agent's own question is answered in its own prompt.
    let ask = st == Status::Blocked && info.agent.is_some() && app.pending_question(term).is_some();
    let (inside, inner, terminal_bg) = if look.hairline {
        let surface = app.parsers.get(&term).map_or(pane_bg(t), |p| crate::client::render::terminal_surface_bg(p.screen(), pane_bg(t)));
        let inside = hairline::pane_frame(app, f.buffer_mut(), r, &info, focused, model, surface, t);
        // The terminal: the pane's padding in from each side, from under the rule to the bottom.
        let below = if ask { 3 } else { 0 };
        let pad = hairline::PANE_PAD;
        let inner = Rect { x: inside.x + pad, width: inside.width.saturating_sub(2 * pad), height: inside.height.saturating_sub(below), ..inside };
        (inside, inner, surface)
    } else {
        let (inside, terminal_bg) = card_frame(app, f.buffer_mut(), r, &info, focused, model, t);
        // The terminal: 3 columns in, a row below the title, down to the last row inside.
        let below = 1 + if ask { 2 } else { 0 };
        (inside, Rect { x: r.x + 3, y: r.y + 2, width: r.width.saturating_sub(6), height: r.height.saturating_sub(2 + below) }, terminal_bg)
    };
    app.pane_frames.push((term, r));
    app.panes.push((term, inner));
    // The whole inside of the card focuses the pane, as the reference's focus region does.
    app.hits.push((inside, Hit::Pane(term)));
    app.hits.push((inner, Hit::Pane(term)));
    let copying = matches!(&app.mode, Mode::Copy(c) if c.term == term);
    if copying {
        if let Mode::Copy(c) = &mut app.mode {
            c.height = inner.height as usize;
            c.width = inner.width as usize;
            crate::client::render::render_copy(c, inner, f.buffer_mut(), t);
        }
    } else if let Some(p) = app.parsers.get(&term) {
        let screen = p.screen();
        render_screen(screen, inner, f.buffer_mut(), terminal_bg);
        if focused
            && !screen.hide_cursor()
            && scrolled.is_none()
            && matches!(app.mode, Mode::Normal | Mode::Prefix { .. })
            && !sheet_has_keys(app)
        {
            let (row, col) = screen.cursor_position();
            if row < inner.height && col < inner.width {
                f.set_cursor_position(Position::new(inner.x + col, inner.y + row));
            }
        }
    }
    draw_scrollbar(app, f.buffer_mut(), term, inner, terminal_bg, t);
    draw_to_bottom(app, f.buffer_mut(), term, inner, scrolled, terminal_bg, t);

    // Not the one you're in: its inside fades (the border and its amber tag stay as they are).
    if !focused {
        dim_inside(f.buffer_mut(), inside, look.dim, t);
    }
    if ask && r.height >= 8 {
        draw_answer_bar(app, f.buffer_mut(), r, term, terminal_bg, t);
    }
}

/// A close confirm asks about this pane: its lines are lit in `err` until you answer, so you
/// see which one goes.
pub(in crate::client) fn closing(app: &App, term: TermId) -> bool {
    matches!(&app.mode, Mode::Confirm(c) if matches!(&c.act, crate::client::menu::Act::End(ts) if ts.contains(&term)))
}

/// A Cards, Compact or Grid pane's frame: the box (or zone) with its title on the top border.
/// Returns the inside and the terminal's ground.
fn card_frame(app: &mut App, buf: &mut Buffer, r: Rect, info: &TermInfo, focused: bool, model: &[Proj], t: &Theme) -> (Rect, Color) {
    let look = Look::of(&app.cfg.ui);
    let term = info.id;
    let grid_color = look.grid.then(|| grid_color(app, term, t));
    let closing = closing(app, term);
    let border = if closing {
        t.err
    } else if let Some(own) = grid_color {
        match (focused, armed(app)) {
            (true, true) => t.sky(),
            (true, false) => own,
            (false, _) => crate::theme::mix(own, t.bg, GRID_UNFOCUSED_FADE),
        }
    } else if !focused {
        t.line
    } else if armed(app) {
        t.sky()
    } else {
        match app.cfg.ui.focus_border.as_str() {
            "bright" => t.strong,
            "none" => t.line,
            _ => t.accent,
        }
    };
    // Just moved here: the border flares and settles, so you see where you landed.
    let on = app.motion_on();
    let flash = if focused { app.motion.land(term as u64, on) } else { 0.0 };
    let flare = if app.cfg.ui.focus_border == "bright" { t.accent } else { t.strong };
    let border = if flash > 0.0 { blend(border, flare, flash) } else { border };
    let mut c = Card::new(t, "");
    let mark = pane_mark(app, info, t);
    // What it's waiting for, beside its state: a usage limit, or what it takes of the machine.
    let note = match info.resume_at {
        Some(at) => Some((format!("⏸ limit · continues in {}", until(at)), t.muted)),
        None => load_note(info.cpu, info.mem).map(|(load, heavy)| (load, if heavy { t.err } else { t.muted })),
    };
    let note = note.map(|(note, colour)| seg(format!("{note}  "), Style::default().fg(colour)));
    c.close = Some(ScreenHit::CloseSplit(term));
    // Grid says the state on the left of the border instead.
    let mark_at = if look.grid {
        c.right.extend(note);
        None
    } else {
        fit_right(look, r, &mut c, note, mark)
    };
    if let Some(parser) = app.parsers.get(&term) {
        c.bg = crate::client::render::terminal_surface_bg(parser.screen(), c.bg);
    }
    let terminal_bg = c.bg;
    if focused || closing {
        c = c.lit(border);
    } else if look.grid {
        c.border = border;
    }
    let top_left = app.screen.leaf_rects.iter().min_by_key(|(_, lr)| (lr.y, lr.x)).is_some_and(|(id, _)| *id == term);
    let why_at = if look.grid {
        grid_title(app, model, info, focused, r, &mut c, t)
    } else {
        // The title runs up to two cells before the right-hand segments and the ✕.
        let right_w = segs_width(&c.right) + close_w(look, &c);
        let title_end = right_end(look, r).saturating_sub(right_w + 2);
        c.chips = if top_left {
            tab_strip(app, model, Rect { x: title_x(look, r), y: r.y, width: title_end.saturating_sub(title_x(look, r)), height: 1 }, focused, None, t).0
        } else {
            let caps = if look.round_tabs { 2 } else { 0 };
            let name = cut_label(&numbered(app, term, &session_name(app, model, term)), title_end.saturating_sub(title_x(look, r) + 2 + caps) as usize);
            let chip = vec![name_chip(&name, focused, t)];
            if look.round_tabs { round_ends(chip, title_ground(look, t)) } else { chip }
        };
        mark_at.into_iter().collect()
    };
    let inside = card(app, buf, r, &c, t);
    // After the tabs' hits, so a click on the glyph inside the current tab opens why.
    for at in why_at {
        hit(app, at, ScreenHit::Why(term));
    }
    (inside, terminal_bg)
}

/// A scrollbar in the margin right of the terminal when there's history: where you are, click
/// or drag it.
fn draw_scrollbar(app: &mut App, buf: &mut Buffer, term: TermId, inner: Rect, terminal_bg: Color, t: &Theme) {
    let (cur, total) = app.history(term);
    if total > 0 && inner.height > 2 {
        let track = Rect { x: inner.right() + 1, y: inner.y, width: 1, height: inner.height };
        let h = track.height as usize;
        let thumb = (h * h / (h + total)).clamp(1, h);
        let top = track.y + ((h - thumb) * (total - cur) / total) as u16;
        let hot = app.screen.drag == Some(Drag::Scroll(term)) || hovered(app, track);
        for yy in track.top()..track.bottom() {
            let on = yy >= top && yy < top + thumb as u16;
            let (sym, col) = if on { ("┃", if hot { t.accent } else { t.muted }) } else { ("│", t.line) };
            if let Some(px) = buf.cell_mut((track.x, yy)) {
                px.set_symbol(sym).set_style(Style::default().fg(col).bg(terminal_bg));
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
fn draw_to_bottom(app: &mut App, buf: &mut Buffer, term: TermId, inner: Rect, scrolled: Option<usize>, terminal_bg: Color, t: &Theme) {
    let label = match (scrolled, app.paged_up.get(&term)) {
        (Some(n), _) => Some(format!(" ↓ Back to bottom · {n} lines up ")),
        (None, Some(_)) => Some(" ↓ Back to bottom ".to_string()),
        (None, None) => None,
    };
    if let Some(label) = label
        && inner.height > 0
    {
        let look = Look::of(&app.cfg.ui);
        let note = pill(look, vec![seg(label, Style::default().fg(t.acc_ink).add_modifier(Modifier::BOLD))], t.accent, terminal_bg);
        let w = segs_width(&note).min(inner.width);
        let (x, y) = (inner.right().saturating_sub(w), inner.bottom() - 1);
        put(buf, x, y, &note, inner.right());
        hit(app, Rect { x, y, width: w, height: 1 }, ScreenHit::ToBottom(term));
    }
}

/// Answer bar: the choices of the question asked through the app, a full-width pill that
/// keeps its amber even when the card is faded.
fn draw_answer_bar(app: &mut App, buf: &mut Buffer, r: Rect, term: TermId, terminal_bg: Color, t: &Theme) {
    let look = Look::of(&app.cfg.ui);
    let ay = r.bottom() - 3;
    let (lx, rx) = (r.x + 3, r.right().saturating_sub(3));
    row_pill(look, buf, lx - 1, ay, rx - lx + 2, t.card2, terminal_bg);
    let opts = app.answer_options(term);
    let full: u16 = opts.iter().take(4).map(|o| o.width() as u16 + 6).sum();
    // In a narrow pane the label shrinks to its dot, then the buttons to their keys.
    let roomy = rx.saturating_sub(lx) >= full + 11;
    let label = if roomy { "● answer   " } else { "● " };
    let mut x = put(buf, lx, ay, &[seg(label, Style::default().fg(t.blocked).bg(t.card2).add_modifier(Modifier::BOLD))], rx);
    let keys_only = rx.saturating_sub(x) < full;
    for (i, o) in opts.iter().enumerate().take(4) {
        let key = char::from_digit(i as u32 + 1, 10).unwrap_or('1').to_string();
        let label = if keys_only { String::new() } else { o.clone() };
        let segs = button_pill(look, t, &label, &key, i == 0, t.card2);
        let br = Rect { x, y: ay, width: segs_width(&segs), height: 1 };
        x = put(buf, x, ay, &segs, rx) + 1;
        app.hits.push((br, Hit::Button(crate::client::Btn::Answer(term, key.chars().next().unwrap_or('1')))));
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

/// One toast for every notice, bottom right above the agent's input area: a `card` with a
/// full-height bar in the notice's colour and no frame. Row 1 is the mark and bold
/// title with a ✕; row 2 the detail; an optional row 3 a preview in dim italics. It clears
/// after `toast_hold` (errors a little longer), or with the ✕ / Esc.
pub(in crate::client) fn draw_toast(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme) {
    if let Some(flash) = app.copy_flash.as_ref().filter(|f| f.live())
        && let Some((_, pane)) = app.panes.iter().find(|(term, _)| *term == flash.term)
    {
        flash.tint(buf, *pane, t);
    }
    let Some((title, at, err)) = app.notice.clone() else { return };
    let about = app.notice_term.filter(|t| app.snap.terms.contains_key(t));
    let restart = app.toast_restart();
    let hold = toast_hold(err, about.is_some(), restart);
    let shown = at.elapsed();
    if shown > hold {
        return;
    }
    // It slides in from the right, and fades at the end of its time.
    let on = app.motion_on();
    let slide = if on { 1.0 - crate::client::motion::ease_out(shown.as_secs_f32() / crate::client::motion::TOAST_IN.as_secs_f32()) } else { 0.0 };
    let left = hold.saturating_sub(shown);
    let fade = if on && left < crate::client::motion::TOAST_FADE { 1.0 - left.as_secs_f32() / crate::client::motion::TOAST_FADE.as_secs_f32() } else { 0.0 };
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
    let needs = about.and_then(|term| app.snap.terms.get(&term)).is_some_and(|x| x.status == Status::Blocked);
    let (colour, mark) = match (err, needs, about) {
        (true, ..) => (t.err, "✕"),
        (_, true, _) => (t.blocked, "●"),
        (_, _, Some(_)) => (t.done, "✓"),
        // A hint about keys (shortcut mode's dead key) is information, not a success.
        _ if app.notice_hint => (t.sky(), "i"),
        _ => (t.accent, "✓"),
    };
    let look = Look::of(&app.cfg.ui);
    let button = restart.then(|| button_pill(look, t, "Restart now", "", false, t.card));
    let body_w = |s: &str| s.width() + TOAST_TEXT_PAD;
    let want = [Some(body_w(&title)), detail.as_deref().map(body_w), preview.as_deref().map(body_w), button.as_ref().map(|b| segs_width(b) as usize + TOAST_TEXT_PAD)];
    let w = want.into_iter().flatten().max().unwrap_or(0).max(TOAST_MIN_W).min((area.width as usize).saturating_sub(4)) as u16;
    let h = 2 + 1 + u16::from(detail.is_some() || button.is_some()) + u16::from(preview.is_some());
    if area.height < h + TOAST_ABOVE_INPUT || w < 12 {
        return;
    }
    let fx = area.right().saturating_sub(w + 2);
    let x = fx + ((area.right().saturating_sub(fx)) as f32 * slide).round() as u16;
    let y = area.bottom().saturating_sub(h + TOAST_ABOVE_INPUT);
    let r = Rect { x, y, width: area.right().saturating_sub(x).min(w), height: h };
    let ink = |c: Color, bg: Color| if fade > 0.0 { blend(c, bg, fade) } else { c };
    let bg = t.card;
    let behind = ends_behind(buf, r);
    for yy in r.y..r.bottom() {
        for xx in r.x..r.right() {
            if let Some(px) = buf.cell_mut((xx, yy)) {
                px.set_symbol(" ").set_style(Style::default().bg(bg));
            }
        }
    }
    for yy in r.y..r.bottom() {
        put(buf, r.x, yy, &[seg("▌", Style::default().fg(ink(colour, bg)).bg(bg))], r.x + 1);
    }
    // Hairline: on the ground, so a square hairline frame in `line` holds it; the bar stays its
    // left edge.
    if look.hairline && r.width >= 3 {
        let edge = Style::default().fg(ink(t.line, bg)).bg(bg);
        for xx in r.x + 1..r.right() {
            buf[(xx, r.y)].set_symbol("─").set_style(edge);
            buf[(xx, r.bottom() - 1)].set_symbol("─").set_style(edge);
        }
        for yy in r.y..r.bottom() {
            buf[(r.right() - 1, yy)].set_symbol("│").set_style(edge);
        }
        buf[(r.right() - 1, r.y)].set_symbol("┐");
        buf[(r.right() - 1, r.bottom() - 1)].set_symbol("┘");
    }
    // Rounded, the card's corners soften as a popup's do, and the bar becomes its left edge.
    if look.rounded && r.width >= 2 {
        soften_block(buf, r, &behind);
        for yy in r.y..r.bottom() {
            buf[(r.x, yy)].set_fg(ink(colour, bg));
        }
    }
    // Rows 2 and 3 start under the title, one column in from the border; the title also
    // leaves the mark's column and the ✕'s.
    let text_w = r.width.saturating_sub(6) as usize;
    let title_w = r.width.saturating_sub(TOAST_TEXT_PAD as u16 + 1) as usize;
    let head = [
        seg(format!("{mark} "), Style::default().fg(ink(colour, bg)).bg(bg).add_modifier(Modifier::BOLD)),
        seg(truncate(&title, title_w), Style::default().fg(ink(t.strong, bg)).bg(bg).add_modifier(Modifier::BOLD)),
    ];
    put(buf, r.x + 2, r.y + 1, &head, r.right() - 1);
    let close = Rect { x: r.right().saturating_sub(4), y: r.y + 1, width: 2, height: 1 };
    put(buf, close.x, close.y, &[seg("✕", Style::default().fg(ink(t.muted, bg)).bg(bg))], r.right() - 1);
    let mut row = r.y + 2;
    if let Some(b) = &button {
        put(buf, r.x + 4, row, b, r.right() - 1);
        row += 1;
    } else if let Some(d) = &detail {
        put(buf, r.x + 4, row, &[seg(truncate(d, text_w), Style::default().fg(ink(t.text, bg)).bg(bg))], r.right() - 1);
        row += 1;
    }
    if let Some(p) = &preview {
        put(buf, r.x + 4, row, &[seg(truncate(p, text_w), Style::default().fg(ink(t.muted, bg)).bg(bg).add_modifier(Modifier::ITALIC))], r.right() - 1);
    }
    if let Some(term) = about {
        hit(app, r, ScreenHit::Session(term));
    } else if restart {
        hit(app, r, ScreenHit::Restart);
    } else {
        hit(app, r, ScreenHit::Noop);
    }
    hit(app, close, ScreenHit::ToastClose);
}
