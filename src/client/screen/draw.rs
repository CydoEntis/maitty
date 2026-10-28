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
    /// The tab row: over the panes and the sheet in Hairline, over the panes in Floating.
    pub tabs: Rect,
}

/// Most the padding round the screen may be, in cells.
const PADDING_MAX: u16 = 2;

/// The window less the padding setting all round.
fn padded(app: &App, area: Rect) -> Rect {
    let m = app.cfg.ui.padding.parse().unwrap_or(1).min(PADDING_MAX);
    Rect { x: area.x + m, y: area.y + m, width: area.width.saturating_sub(2 * m), height: area.height.saturating_sub(2 * m) }
}

/// The room the screen is laid out in: inside the padding, above the mode bar and, in
/// Hairline, above the rule over it.
pub(in crate::client) fn framed(app: &App, area: Rect) -> Rect {
    let r = padded(app, area);
    let bar = if Look::of(&app.cfg.ui).hairline { 2 } else { 1 };
    Rect { height: r.height.saturating_sub(bar), ..r }
}

/// The mode bar's row (ADR 0024): the bottom row inside the padding.
pub(in crate::client) fn mode_bar_row(app: &App, area: Rect) -> Rect {
    let r = padded(app, area);
    Rect { y: r.bottom().saturating_sub(1), height: r.height.min(1), ..r }
}

/// Where popups and their dimming go: the window above the mode bar, so the mode stays in
/// view and readable under any of them.
pub(in crate::client) fn above_bar(app: &App, area: Rect) -> Rect {
    let top = mode_bar_row(app, area).y;
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
        let x = area.x + sw + gap;
        (Rect { width: sw, ..area }, Rect { x, width: area.right().saturating_sub(x), ..area })
    };
    // Collapsed, the sessions are chips on row 0 and the panes (with their tabs) start below it.
    let col = if strip_on(app) { Rect { y: col.y + 1, height: col.height.saturating_sub(1), ..col } } else { col };
    // The tabs have a row of their own over the panes in both looks (ADR 0028): Hairline's
    // over a rule across the whole column, Floating's chips right over the boxes only, the side
    // sheet being a card of its own.
    let tab_rows = if look.hairline { hairline::TAB_ROWS } else { FLOAT_TAB_ROW };
    let carve = |r: Rect| if r.height > tab_rows && r.width > 0 {
        (Rect { height: tab_rows, ..r }, Rect { y: r.y + tab_rows, height: r.height - tab_rows, ..r })
    } else {
        (Rect::default(), r)
    };
    let (hair_tabs, col) = if look.hairline { carve(col) } else { (Rect::default(), col) };
    let edge = if side.x > col.x { Rect { x: col.right(), width: gap.max(1), ..area } } else { Rect { x: col.x.saturating_sub(gap), width: gap.max(1), ..area } };
    let (sheet, panes) = match open_sheet(app) {
        Some(kind) => split_for_sheet(area.width, col, look, app.sheet_frac, wanted_w(app, kind, area.width)),
        None => (Rect::default(), col),
    };
    let (tabs, panes) = if look.hairline { (hair_tabs, panes) } else { carve(panes) };
    Grid { side, panes, edge, sheet, tabs }
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
    if !Look::of(&app.cfg.ui).hairline {
        draw_tab_chips(app, f.buffer_mut(), g.tabs, &model, t);
    } else {
        hairline::draw_tab_row(app, f.buffer_mut(), g.tabs, &model, t);
        let edge = (g.side.width >= SIDE_DRAWN_FROM).then_some(g.edge.x);
        hairline::draw_bar_rule(f.buffer_mut(), Rect { y: room.bottom(), height: 1, ..room }, g.tabs, edge, g.side.x < g.edge.x, t);
    }
    if let Some(kind) = open_sheet(app).filter(|_| g.sheet.width >= SHEET_DRAWN_FROM) {
        draw_sheet(app, f.buffer_mut(), g.sheet, kind, t);
        draw_sheet_edge(app, f.buffer_mut(), &g, t);
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

/// What a tab's ✕ does: close the tab, or a session's only tab's only pane. Panes have no ✕
/// of their own (ADR 0030), so every tab has one. Each asks first.
pub(in crate::client) fn tab_close(app: &App, tab: usize, tabs: usize) -> Option<ScreenHit> {
    if tabs > 1 {
        return Some(ScreenHit::CloseTab(tab));
    }
    // A session's only tab: its only pane, or (split, its panes having no ✕) all of them.
    match app.screen.tabs.get(tab)?.layout.leaves().as_slice() {
        [only] => Some(ScreenHit::CloseSplit(*only)),
        _ => Some(ScreenHit::CloseTab(tab)),
    }
}

/// A tab's most urgent agent, whose state the tab shows wherever it is drawn (its chip, its
/// sidebar line), on screen or not: one that needs you first, then by its state.
pub(in crate::client) fn tab_agent(app: &App, tab: usize) -> Option<&TermInfo> {
    let urgency = |i: &TermInfo| if check::needs(i.status, i.check()) { 0 } else { i.status.urgency() };
    let leaves = app.screen.tabs.get(tab)?.layout.leaves();
    leaves.iter().filter_map(|id| app.snap.terms.get(id)).filter(|i| i.agent.is_some()).min_by_key(|i| urgency(i))
}

/// The tab on screen shows `term` alone: its tab carries its name, so its title doesn't repeat it.
pub(in crate::client) fn lone_pane(app: &App, term: TermId) -> bool {
    matches!(app.screen.leaf_rects.as_slice(), [(id, _)] if *id == term)
}

/// `project · ⎇ branch`, from the session's sidebar row, else the pane's own branch.
pub(in crate::client) fn project_branch(app: &App, info: &TermInfo, model: &[Proj]) -> String {
    let branch_mark = check::marks(app).branch;
    let (project, branch) = match find(model, info.id) {
        Some((p, w, _)) => (p.name.clone(), w.branch.clone()),
        None => (String::new(), info.branch.clone().unwrap_or_default()),
    };
    let branch = if branch.is_empty() { String::new() } else { format!("{branch_mark} {branch}") };
    [project, branch].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
}

/// Floating's tab row: the chips' own row, right over the boxes (no gap: rows are scarce).
const FLOAT_TAB_ROW: u16 = 1;

/// Floating's tabs along the first row of `r`, lined up with the pane titles' chips.
fn draw_tab_chips(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    if r.height == 0 || r.width < 4 {
        return;
    }
    let row = Rect { x: r.x + 1, width: r.width - 2, height: 1, ..r };
    let chips = tab_strip(app, model, row, true, t);
    put(buf, row.x, row.y, &chips, row.right());
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
        .filter(|l| l.contains(focus) && l.leaves().len() > 1);
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

/// A tab's chips as text: its number when the session has several (a lone tab has none) and
/// its name, else its focused session's, as the sidebar's tab lines say it.
fn tab_label(app: &App, model: &[Proj], i: usize, n: usize, several: bool) -> (Option<String>, String) {
    let tab = &app.screen.tabs[i];
    (several.then(|| (n + 1).to_string()), tab_name(tab).unwrap_or_else(|| session_name(app, model, tab.focus)))
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
/// before and after it, drop before it shortens its name. A click on one picks it.
fn tab_strip(app: &mut App, model: &[Proj], row: Rect, lit: bool, t: &Theme) -> Vec<Seg> {
    let look = Look::of(&app.cfg.ui);
    let ground = Color::Reset;
    let (y, max_x) = (row.y, row.right());
    let mut out = Vec::new();
    let mut at = row.x;
    let tabs = app.session_tabs();
    let closes: Vec<Option<ScreenHit>> = tabs.iter().map(|&i| tab_close(app, i, tabs.len())).collect();
    let marks: Vec<Option<(String, Color)>> = tabs.iter().map(|&i| super::side_tabs::tab_mark(app, i, t).map(|(g, _, c)| (g, c))).collect();
    let labels: Vec<(Option<String>, String)> = tabs.iter().enumerate().map(|(n, &i)| tab_label(app, model, i, n, tabs.len() > 1)).collect();
    let names: Vec<String> = labels.iter().map(|(_, name)| name.clone()).collect();
    let current = Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD);
    let chips = |n: usize, on: bool, name: &str| {
        let number = if on { current } else { Style::default().fg(t.text).bg(t.btn) };
        let label = if on { name_chip(name, lit, t) } else { seg(format!(" {name} "), Style::default().fg(t.text).bg(t.card)) };
        // Its state, on screen or not, so a tab out of view still says it works or asks; on the
        // accent the glyph takes the accent's ink, its shape saying the state.
        let mut pair: Vec<Seg> = marks[n].iter().map(|(g, c)| seg(format!(" {g}"), number.fg(if on { t.acc_ink } else { *c }).add_modifier(Modifier::BOLD))).collect();
        pair.extend(labels[n].0.as_ref().map(|head| seg(format!(" {head} "), number)));
        pair.push(label);
        if closes[n].is_some() {
            let (fg, bg) = if on { if lit { (t.acc_ink, t.accent) } else { (t.strong, t.btn) } } else { (t.muted, t.card) };
            pair.push(seg("✕ ", Style::default().fg(fg).bg(bg)));
        }
        if look.round_tabs { round_ends(pair, ground) } else { pair }
    };
    // The room the tabs before the current one leave it: its whole pair, else its shortest,
    // so it never shortens or drops while another tab shows.
    let current_w = |shorten: bool| {
        let n = tabs.iter().position(|&i| i == app.screen.tab);
        n.map_or(0, |n| segs_width(&chips(n, true, &if shorten { cut_label(&names[n], 2) } else { names[n].clone() })))
    };
    let keep = [current_w(false), current_w(true)].into_iter().find(|w| row.x + w <= max_x).unwrap_or(0);
    let (mut all_fit, mut placed, mut crowded) = (true, false, false);
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
        // The tab's glyph opens why, as a split pane's does: a lone pane's state is only here.
        if marks[n].is_some() && let Some(agent) = tab_agent(app, i).map(|a| a.id) {
            hit(app, Rect { x: at + u16::from(look.round_tabs) + 1, y, width: 1, height: 1 }, ScreenHit::Why(agent));
        }
        if let Some(h) = closes[n] {
            let close_w = if look.round_tabs { 3 } else { 2 };
            hit(app, Rect { x: at + w - close_w, y, width: 2, height: 1 }, h);
        }
        placed |= on;
        out.extend(pair);
        at += w;
    }
    // The chip that opens a tab, a cell clear of the tabs: it drops first, so never shows
    // without every tab before it.
    if !all_fit {
        return out;
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
    out
}

/// A session's mark on its pane: its glyph, bold in its colour (working is the Appearance
/// spinner). A shell has none.
fn pane_mark(app: &App, info: &TermInfo, t: &Theme) -> Option<PaneMark> {
    info.agent.as_ref()?;
    let set = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let spinner = glyph(app, Status::Working);
    let step = if app.motion_on() { check::ring_step(app.spinner_frame(), set.ring.len()) } else { 0 };
    let m = Kind::of(info.status, info.check(), info.asleep).mark(set, t, step, &spinner);
    Some(PaneMark { glyph: m.glyph.to_string(), style: Style::default().fg(m.color).add_modifier(Modifier::BOLD) })
}

/// A session's mark on its pane.
struct PaneMark {
    glyph: String,
    style: Style,
}

/// How far an unfocused pane's own colour (Pane colours) fades toward the ground.
const GRID_UNFOCUSED_FADE: f32 = 0.42;

/// A pane's place among the tab's panes (0 first): top-left first, left to right and top to
/// bottom. Pane colours colour panes by it, and every look numbers them by it.
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
/// (`2 · warm-f…`), then the ` · `, leaving the bare number (`2`).
fn cut_label(label: &str, room: usize) -> String {
    let Some((number, name)) = label.split_once(NUMBER_SEP).filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) else {
        return truncate(label, room);
    };
    let front = number.width() + NUMBER_SEP.width();
    if room >= front + 2 { format!("{number}{NUMBER_SEP}{}", truncate(name, room - front)) } else { truncate(number, room) }
}

/// A pane's own colour (Pane colours), by its place; they repeat from the fifth.
fn grid_color(app: &App, term: TermId, t: &Theme) -> Color {
    let colors = t.grid_colors();
    colors[grid_index(app, term) % colors.len()]
}

/// A pane as a card, or Hairline's zone. A split pane's title is its number, state glyph (it
/// opens why) and name; a lone pane has none (ADR 0030). The terminal sits inside, 3 columns
/// and 2 rows in.
/// Unfocused cards fade; one that needs you keeps its amber.
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

/// A Floating pane's frame: the box (or zone) with its title on the top border.
/// Returns the inside and the terminal's ground.
fn card_frame(app: &mut App, buf: &mut Buffer, r: Rect, info: &TermInfo, focused: bool, model: &[Proj], t: &Theme) -> (Rect, Color) {
    let look = Look::of(&app.cfg.ui);
    let term = info.id;
    let grid_color = look.pane_colors.then(|| grid_color(app, term, t));
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
    // What it's waiting for: a usage limit, or what it takes of the machine. Its state is the
    // glyph in the title (and on its tab); it has no ✕: the tab's, Alt+x or its menu close it.
    let note = match info.resume_at {
        Some(at) => Some((format!("⏸ limit · continues in {}", until(at)), t.muted)),
        None => load_note(info.cpu, info.mem).map(|(load, heavy)| (load, if heavy { t.err } else { t.muted })),
    };
    let note = note.map(|(note, colour)| seg(note, Style::default().fg(colour)));
    c.right = note.into_iter().filter(|n| n.0.width() as u16 + 2 <= right_end(r).saturating_sub(title_x(r) + 2) / 2).collect();
    if let Some(parser) = app.parsers.get(&term) {
        c.bg = crate::client::render::terminal_surface_bg(parser.screen(), c.bg);
    }
    let terminal_bg = c.bg;
    if focused || closing {
        c = c.lit(border);
    } else if look.pane_colors {
        c.border = border;
    }
    // A tab's only pane has no title: its tab names it (as an editor's lone pane, ADR 0030).
    // A split's: number, state glyph and name, in one chip.
    let mut mark_at = None;
    if !lone_pane(app, term) {
        let right_w = segs_width(&c.right);
        let title_end = right_end(r).saturating_sub(if right_w > 0 { right_w + 2 } else { 0 });
        let caps = if look.round_tabs { 2 } else { 0 };
        let mark = pane_mark(app, info, t);
        let room = title_end.saturating_sub(title_x(r) + caps) as usize;
        let chip = title_chip(&numbered(app, term, &session_name(app, model, term)), mark.as_ref(), focused, room, t);
        mark_at = mark.as_ref().map(|_| Rect { x: title_x(r) + caps / 2 + chip[0].0.width() as u16, y: r.y, width: 1, height: 1 });
        c.chips = if look.round_tabs { round_ends(chip, Color::Reset) } else { chip };
    }
    let inside = card(app, buf, r, &c, t);
    // After the card's hits, so a click on the glyph opens why.
    if let Some(at) = mark_at {
        hit(app, at, ScreenHit::Why(term));
    }
    (inside, terminal_bg)
}

/// A split pane's title chip, at most `room` wide: ` 2 ` then the state glyph in its colour
/// (an agent's) and the name, as Hairline's title reads. The name is cut first, then the
/// glyph; the number stays.
fn title_chip(label: &str, mark: Option<&PaneMark>, lit: bool, room: usize, t: &Theme) -> Vec<Seg> {
    let (bg, fg) = if lit { (t.accent, t.acc_ink) } else { (t.btn, t.strong) };
    let st = Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD);
    let (front, name) = match label.split_once(NUMBER_SEP) {
        Some((n, name)) if n.chars().all(|c| c.is_ascii_digit()) => (format!(" {n} "), name),
        _ => (" ".to_string(), label),
    };
    let glyph = mark.filter(|_| room >= front.width() + 4).map(|m| seg(format!("{} ", m.glyph), m.style.bg(bg).fg(if lit { t.acc_ink } else { m.style.fg.unwrap_or(fg) })));
    let used = front.width() + glyph.as_ref().map_or(0, |g| g.0.width()) + 1;
    let mut out = vec![seg(front, st)];
    out.extend(glyph);
    out.push(seg(format!("{} ", cut_label(name, room.saturating_sub(used))), st));
    out
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
    // Floating: a `line` outline on the ground with round corners whatever `ui.rounded` says
    // (the box glyphs need no Nerd Font), its left side a heavy rule in the notice's colour.
    // Hairline, which has no boxes: a solid block in `card2` (its `card` is the ground), its
    // left edge that rule (ADR 0028).
    let bg = if look.hairline { t.card2 } else { t.bg };
    fill(buf, r, bg);
    let bar = Style::default().fg(ink(colour, bg)).bg(bg);
    if look.hairline {
        for yy in r.y..r.bottom() {
            buf[(r.x, yy)].set_symbol("┃").set_style(bar);
        }
    } else {
        panel_box(buf, r, Look { rounded: true, ..look }, Style::default().fg(ink(t.line, bg)).bg(bg));
        for yy in r.y + 1..r.bottom().saturating_sub(1) {
            buf[(r.x, yy)].set_symbol("┃").set_style(bar);
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
