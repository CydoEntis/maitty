//! The main screen: the sidebar, the panes (the top-left one carrying the tabs in its title),
//! the sheet, the pill at the top right and the toast.

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

/// The room the screen is laid out in: the window less the padding setting all round.
pub(in crate::client) fn framed(app: &App, area: Rect) -> Rect {
    let m = app.cfg.ui.padding.parse().unwrap_or(1).min(PADDING_MAX);
    Rect { x: area.x + m, y: area.y + m, width: area.width.saturating_sub(2 * m), height: area.height.saturating_sub(2 * m) }
}

/// The grid in `area` (already `framed`): the sidebar the full height at one side, then the
/// pane column from its top row. Cards floating leave the gap setting between them, tiled none (the two
/// borders sit side by side); Flat starts the panes on the sidebar's last column, so one `│`
/// parts them. A width you dragged the sidebar to wins over the default.
pub(in crate::client) fn grid(app: &App, area: Rect) -> Grid {
    let look = Look::of(&app.cfg.ui);
    let full = app.screen.saved.side_w.unwrap_or_else(|| side_w(area.width)).clamp(SIDE_MIN, SIDE_MAX).min(area.width / 2);
    let sw = (full as f32 * app.side_frac).round() as u16;
    let gap = if sw == 0 { 0 } else { look.gap };
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
        draw_side(app, f.buffer_mut(), g.side, &model, t);
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
    draw_mode_frame(app, f.buffer_mut(), area, t);
    draw_corner_pill(app, f.buffer_mut(), room, t);
    app.screen.crumb_x = g.panes.x + 1;
    // A popup (`<app> popup`) floats over everything, the rest dimmed.
    match app.mode.clone() {
        Mode::Compose(c) if c.inbox.is_none() => draw_compose_pop(app, f.buffer_mut(), area, g.side, &c, t),
        Mode::Why { term, .. } => draw_why_pop(app, f.buffer_mut(), area, g.side, term, t),
        Mode::Helpers { term, .. } => draw_helpers_pop(app, f.buffer_mut(), area, g.side, term, t),
        _ => {}
    }
    if let Some(term) = app.popup() {
        let whole = f.area();
        dim_all(f.buffer_mut(), whole, t);
        let w = (whole.width * 4 / 5).max(40).min(whole.width);
        let h = (whole.height * 3 / 4).max(12).min(whole.height);
        let r = Rect { x: whole.x + (whole.width - w) / 2, y: whole.y + (whole.height - h) / 2, width: w, height: h };
        draw_session(app, f, r, term, true, &model, t);
    }
    draw_toast(app, f.buffer_mut(), room, t);
    g.panes
}

/// The corner pill while shortcut mode is held under a popup, the wider of its two words.
const SHORTCUT_PAUSED: &str = " SHORTCUT · paused ";

/// The pill at the top right of row 0, capped like every chip when rounded: the caps' ground is
/// filled in where it is drawn (`draw_corner_pill`).
pub(super) fn corner_pill(app: &App, t: &Theme) -> Option<(Vec<Seg>, ScreenHit)> {
    let (segs, h) = corner_words(app, t)?;
    Some((if Look::of(&app.cfg.ui).round_tabs { round_ends(segs, Color::Reset) } else { segs }, h))
}

/// What the corner pill says: the leader while it's armed, else Update when a newer release is
/// out, else Restart once a newer one is installed but not running.
fn corner_words(app: &App, t: &Theme) -> Option<(Vec<Seg>, ScreenHit)> {
    // Shortcut mode is on (ADR 0016): say so, and how to leave it.
    if armed(app) {
        return Some((vec![seg(" SHORTCUT · Esc ", Style::default().fg(t.bg).bg(t.sky()).add_modifier(Modifier::BOLD))], ScreenHit::Leader));
    }
    // Held under a popup or view it opened: dim, so you still know it comes back.
    if shortcut_paused(app) {
        return Some((vec![seg(SHORTCUT_PAUSED, Style::default().fg(t.muted).bg(t.btn))], ScreenHit::Leader));
    }
    if let Some(version) = app.update_available.as_ref() {
        return Some((vec![seg(format!(" ↑ Update {version} "), Style::default().fg(t.accent).bg(t.btn).add_modifier(Modifier::BOLD))], ScreenHit::Update));
    }
    // Installed (here or from outside the app), not running yet: one click finishes it.
    app.restart_needed().then(|| (vec![seg(" ↻ Restart ", Style::default().fg(t.accent).bg(t.btn).add_modifier(Modifier::BOLD))], ScreenHit::Restart))
}

pub(in crate::client) fn draw_corner_pill(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme) {
    let Some((segs, h)) = corner_pill(app, t) else { return };
    let w = segs_width(&segs);
    let x = area.right().saturating_sub(w + 1);
    let r = Rect { x, y: area.y, width: w, height: 1 };
    let behind = ends_behind(buf, r);
    put(buf, x, area.y, &segs, area.right());
    // The caps sit on whatever is there: a border, a header strip, the dimming behind a popup.
    if Look::of(&app.cfg.ui).round_tabs
        && let Some(&(bl, br)) = behind.first()
    {
        let on = r.intersection(buf.area);
        buf[(on.x, on.y)].set_bg(bl);
        buf[(on.right() - 1, on.y)].set_bg(br);
    }
    // Paused, it only says so: the popup has the keys and its own way out, and the key map
    // opened over it would throw away what was typed there. It still catches the click, which
    // would otherwise land outside the popup and close it.
    hit(app, r, if shortcut_paused(app) { ScreenHit::Noop } else { h });
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
pub(in crate::client) fn shortcut_paused(app: &App) -> bool {
    app.shortcut_held && !armed(app) && (app.mode != Mode::Normal || app.view_has_keys())
}

/// What a session is called on its pane: its sidebar name, else (a pane in another's tab or
/// split has no row) your name for it or what runs in it.
fn session_name(app: &App, model: &[Proj], term: TermId) -> String {
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

/// The tabs of the session on screen, as the top-left pane's title from `x` up to `max_x`:
/// a number chip and a name chip each (the tab's name, else its focused session's), the
/// current one in the accent, then a + chip that opens a tab. Tabs that don't fit drop from
/// the right; a click on one picks it.
fn tab_strip(app: &mut App, model: &[Proj], x: u16, y: u16, max_x: u16, lit: bool, t: &Theme) -> Vec<Seg> {
    let look = Look::of(&app.cfg.ui);
    let ground = title_ground(look, t);
    let mut out = Vec::new();
    let mut at = x;
    let tabs = app.session_tabs();
    let closable = tabs.len() > 1;
    let mut all_fit = true;
    for (n, i) in tabs.into_iter().enumerate() {
        let tab = &app.screen.tabs[i];
        let name = tab_name(tab).unwrap_or_else(|| session_name(app, model, tab.focus));
        let on = i == app.screen.tab;
        let number = if on { Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(t.text).bg(t.btn) };
        let chips = |name: &str| {
            let label = if on { name_chip(name, lit, t) } else { seg(format!(" {name} "), Style::default().fg(t.text).bg(t.card)) };
            let mut pair = vec![seg(format!(" {} ", n + 1), number), label];
            if closable {
                let (fg, bg) = if on { if lit { (t.acc_ink, t.accent) } else { (t.strong, t.btn) } } else { (t.muted, t.card) };
                pair.push(seg("✕ ", Style::default().fg(fg).bg(bg)));
            }
            if look.round_tabs { round_ends(pair, ground) } else { pair }
        };
        let mut pair = chips(&name);
        // The tab you are on shortens its name before it would drop out of a narrow title.
        let rest = segs_width(&pair).saturating_sub(name.width() as u16);
        if on && at + segs_width(&pair) > max_x && max_x > at + rest + 1 {
            pair = chips(&truncate(&name, (max_x - at - rest) as usize));
        }
        let w = segs_width(&pair);
        if at + w > max_x {
            all_fit = false;
            break;
        }
        hit(app, Rect { x: at, y, width: w, height: 1 }, ScreenHit::TabPick(i));
        if closable {
            let close_w = if look.round_tabs { 3 } else { 2 };
            hit(app, Rect { x: at + w - close_w, y, width: 2, height: 1 }, ScreenHit::CloseTab(i));
        }
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
fn close_width(c: &Card) -> u16 {
    match (c.close.is_some(), c.right.is_empty()) {
        (false, _) => 0,
        (true, true) => 2,
        (true, false) => 4,
    }
}

/// The right of a pane's title in a narrow card: the load note goes first, then the mark's
/// word, then the ✕; the mark's glyph is the last thing to go, since status must stay in view.
/// Returns where the mark sits (a click opens why).
fn fit_right(look: Look, r: Rect, c: &mut Card, note: Option<Seg>, mark: Option<PaneMark>) -> Option<Rect> {
    let end = right_end(look, r).saturating_sub(c.right_room);
    // The card draws its right-hand segments only with two cells to spare after the title's start.
    let avail = end.saturating_sub(title_x(look, r) + 2);
    let full = mark.as_ref().map(|m| seg(format!("{} {}", m.glyph, m.word), m.style));
    let short = mark.as_ref().map(|m| seg(m.glyph.clone(), m.style));
    let tries = [(note.clone(), full.clone(), true), (None, full, true), (None, short.clone(), true), (None, short, false)];
    let close = c.close;
    for (note, mark, closes) in tries {
        c.right = note.into_iter().chain(mark.clone()).collect();
        c.close = if closes { close } else { None };
        if segs_width(&c.right) + close_width(c) <= avail {
            return mark.map(|m| {
                let w = m.0.width() as u16;
                Rect { x: end.saturating_sub(close_width(c) + w), y: r.y, width: w, height: 1 }
            });
        }
    }
    c.right.clear();
    None
}

/// Room a card in the top-right corner (a pane or the sheet) leaves at the end of its title for
/// whichever corner pill shows, so its mark and ✕ never sit under it or jump as it changes.
pub(in crate::client) fn corner_room(app: &App, r: Rect, screen: Rect, t: &Theme) -> u16 {
    if r.y != screen.y || r.right() != screen.right() {
        return 0;
    }
    let shown = corner_pill(app, t).map_or(0, |(segs, _)| segs_width(&segs));
    // Lit and paused differ in width: both keep the wider, so the title holds still between them.
    let caps = if Look::of(&app.cfg.ui).round_tabs { 2 } else { 0 };
    let held = if armed(app) || shortcut_paused(app) { SHORTCUT_PAUSED.width() as u16 + caps } else { 0 };
    shown.max(held)
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

/// The ✕ at the end of a title and the spaces round the right-hand segments.
const CLOSE_W: u16 = 4;

/// A pane's place among the tab's panes (0 first): top-left first, left to right and top to
/// bottom. Grid numbers and colours panes by it.
fn grid_index(app: &App, term: TermId) -> usize {
    let mut order: Vec<(u16, u16, TermId)> = app.screen.leaf_rects.iter().map(|(id, r)| (r.y, r.x, *id)).collect();
    order.sort_unstable();
    order.iter().position(|(.., id)| *id == term).unwrap_or(0)
}

/// A Grid pane's colour, by its place; they repeat from the fifth.
fn grid_color(app: &App, term: TermId, t: &Theme) -> Color {
    let colors = t.grid_colors();
    colors[grid_index(app, term) % colors.len()]
}

/// Grid's top border: the pane's number and its state on the left (the top-left pane's tabs
/// after them), its name as a tag at the front of the right-hand segments. A narrow pane drops
/// the tag, then the state's words, keeping the number. Returns where the number and state
/// sit, which open why.
fn grid_title(app: &mut App, model: &[Proj], info: &TermInfo, focused: bool, r: Rect, c: &mut Card, t: &Theme) -> Rect {
    let look = Look::of(&app.cfg.ui);
    let own = grid_color(app, info.id, t);
    let chip_style = if focused {
        Style::default().bg(own).fg(t.ink_on(own)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(crate::theme::mix(own, t.bg, GRID_TAG_FADE)).fg(t.strong)
    };
    let chip = |text: String| {
        let chip = vec![seg(text, chip_style)];
        if look.round_tabs { round_ends(chip, title_ground(look, t)) } else { chip }
    };
    let number = chip(format!(" {} ", grid_index(app, info.id) + 1));
    let state = match (info.agent.is_some(), info.status) {
        (true, Status::Blocked) => Some(("▲".to_string(), " needs you", Style::default().fg(t.blocked).add_modifier(Modifier::BOLD))),
        (true, Status::Working) => Some((glyph(app, Status::Working).to_string(), " working", Style::default().fg(own))),
        _ => None,
    };
    let tag = chip(format!(" {} ", session_name(app, model, info.id)));
    let x = title_x(look, r);
    let room = right_end(look, r).saturating_sub(x);
    let corner = c.right_room;
    let (number_w, note_w, tag_w) = (segs_width(&number), segs_width(&c.right), segs_width(&tag) + 2);
    let (words_w, glyph_w) = state.as_ref().map_or((0, 0), |(g, words, _)| ((1 + g.width() + words.width()) as u16, (1 + g.width()) as u16));
    // The top-left pane keeps room for its current tab at its shortest: the tag, the load note
    // and the state's words drop before it does.
    let top_left = app.screen.leaf_rects.iter().min_by_key(|(_, lr)| (lr.y, lr.x)).is_some_and(|(id, _)| *id == info.id);
    let caps = if look.round_tabs { 2 } else { 0 };
    let tab_close = if app.session_tabs().len() > 1 { 2 } else { 0 };
    let tabs_min = if top_left { SHORTEST_TAB + caps + tab_close + 2 } else { 0 };
    // Number, state, the gaps round them, what is on the right, the ✕ and the tabs' share.
    let fits = |number: u16, state: u16, right: u16, close: u16, tabs: u16| number + state + 2 + right + close + corner + tabs <= room;
    let with_tag = fits(number_w, words_w, note_w + tag_w, CLOSE_W, tabs_min);
    if with_tag {
        // A gap only when something (a load note) follows it; the ✕ brings its own.
        let gap = (!c.right.is_empty()).then(|| seg("  ", Style::default()));
        c.right.splice(0..0, tag.into_iter().chain(gap));
    }
    let right_w = if fits(number_w, words_w, note_w, CLOSE_W, tabs_min) {
        note_w + if with_tag { tag_w } else { 0 }
    } else {
        c.right.clear();
        0
    };
    // Then the words, then the ✕, then the number: the state's glyph is the last to go, since
    // status must stay in view. The tabs only get what is left after the ✕.
    let (words, close, keep_number) = if fits(number_w, words_w, right_w, CLOSE_W, tabs_min) {
        (true, true, true)
    } else if fits(number_w, glyph_w, right_w, CLOSE_W, 0) {
        (false, true, true)
    } else if fits(number_w, glyph_w, right_w, 0, 0) {
        (false, false, true)
    } else {
        (false, false, false)
    };
    if !close {
        c.close = None;
    }
    let mut left = if keep_number { number } else { Vec::new() };
    if let Some((g, w, st)) = state {
        let text = if words { format!(" {g}{w}") } else { format!(" {g}") };
        left.extend([seg(text, st), seg(" ", Style::default())]);
    }
    let rest = right_w + if close { CLOSE_W } else { 0 } + corner;
    let left_w = segs_width(&left);
    if top_left {
        let title_end = right_end(look, r).saturating_sub(rest + 2);
        left.push(seg(" ", Style::default()));
        left.extend(tab_strip(app, model, x + left_w + 1, r.y, title_end, focused, t));
    }
    c.chips = left;
    Rect { x, y: r.y, width: left_w, height: 1 }
}

pub(in crate::client) fn draw_session(app: &mut App, f: &mut Frame, r: Rect, term: TermId, focused: bool, model: &[Proj], t: &Theme) {
    let Some(info) = app.snap.terms.get(&term).cloned() else { return };
    let look = Look::of(&app.cfg.ui);
    let st = info.status;
    // While the sidebar has the keys, no pane shows as focused.
    // Exactly one lit edge: an open sheet has the keys, so the pane gives up its lit border.
    let focused = focused && !side_focused(app) && !sheet_has_keys(app);
    let scrolled = app.parsers.get(&term).map(|p| p.screen().scrollback()).filter(|n| *n > 0);
    let grid_color = look.grid.then(|| grid_color(app, term, t));
    let border = if let Some(own) = grid_color {
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
    let mark = pane_mark(app, &info, t);
    // What it's waiting for, beside its state: a usage limit, or what it takes of the machine.
    let note = match info.resume_at {
        Some(at) => Some((format!("⏸ limit · continues in {}", until(at)), t.muted)),
        None => load_note(info.cpu, info.mem).map(|(load, heavy)| (load, if heavy { t.err } else { t.muted })),
    };
    let note = note.map(|(note, colour)| seg(format!("{note}  "), Style::default().fg(colour)));
    c.close = Some(ScreenHit::CloseSplit(term));
    let room = corner_room(app, r, framed(app, f.area()), t);
    c.right_room = room;
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
    if focused {
        c = c.lit(border);
    } else if look.grid {
        c.border = border;
    }
    let top_left = app.screen.leaf_rects.iter().min_by_key(|(_, lr)| (lr.y, lr.x)).is_some_and(|(id, _)| *id == term);
    let why_at = if look.grid {
        Some(grid_title(app, model, &info, focused, r, &mut c, t))
    } else {
        // The title runs up to two cells before the right-hand segments and the ✕.
        let right_w = segs_width(&c.right) + close_width(&c) + room;
        let title_end = right_end(look, r).saturating_sub(right_w + 2);
        c.chips = if top_left {
            tab_strip(app, model, title_x(look, r), r.y, title_end, focused, t)
        } else {
            let caps = if look.round_tabs { 2 } else { 0 };
            let name = truncate(&session_name(app, model, term), title_end.saturating_sub(title_x(look, r) + 2 + caps) as usize);
            let chip = vec![name_chip(&name, focused, t)];
            if look.round_tabs { round_ends(chip, title_ground(look, t)) } else { chip }
        };
        mark_at
    };
    let buf = f.buffer_mut();
    let inside = card(app, buf, r, &c, t);
    if let Some(at) = why_at {
        hit(app, at, ScreenHit::Why(term));
    }
    app.pane_frames.push((term, r));

    // The terminal: 3 columns in, a row below the title, down to the last row inside.
    // Answer buttons only for a question asked through the app (ask-human), which has nowhere
    // else to be answered: an agent's own question is answered in its own prompt.
    let ask = st == Status::Blocked && info.agent.is_some() && app.pending_question(term).is_some();
    let below = 1 + if ask { 2 } else { 0 };
    let inner = Rect { x: r.x + 3, y: r.y + 2, width: r.width.saturating_sub(6), height: r.height.saturating_sub(2 + below) };
    app.panes.push((term, inner));
    // The whole inside of the card focuses the pane, as checks7.js's focus region does.
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

    // A scrollbar in the margin when there's history: where you are, click or drag it.
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
            if let Some(px) = f.buffer_mut().cell_mut((track.x, yy)) {
                px.set_symbol(sym).set_style(Style::default().fg(col).bg(terminal_bg));
            }
        }
        if app.screen.drag.is_none() || app.screen.drag == Some(Drag::Scroll(term)) {
            app.screen.bar = Some((term, track, total));
        }
        hit(app, track, ScreenHit::ScrollBar(term));
    }

    // Scrolled up (in the app's history, or an agent that scrolls itself paged up by the wheel):
    // a button back to the newest output, where a chat's is, at the bottom right.
    let label = match (scrolled, app.paged_up.get(&term)) {
        (Some(n), _) => Some(format!(" ↓ Back to bottom · {n} lines up ")),
        (None, Some(_)) => Some(" ↓ Back to bottom ".to_string()),
        (None, None) => None,
    };
    if let Some(label) = label
        && inner.height > 0
    {
        let note = pill(look, vec![seg(label, Style::default().fg(t.acc_ink).add_modifier(Modifier::BOLD))], t.accent, c.bg);
        let w = segs_width(&note).min(inner.width);
        let (x, y) = (inner.right().saturating_sub(w), inner.bottom() - 1);
        put(f.buffer_mut(), x, y, &note, inner.right());
        hit(app, Rect { x, y, width: w, height: 1 }, ScreenHit::ToBottom(term));
    }

    // Not the one you're in: its inside fades (the border and its amber tag stay as they are).
    if !focused {
        dim_inside(f.buffer_mut(), inside, look.dim, t);
    }

    // Answer bar: the choices of the question asked through the app, a full-width pill that
    // keeps its amber even when the card is faded.
    if ask && r.height >= 8 {
        let ay = r.bottom() - 3;
        let (lx, rx) = (r.x + 3, r.right().saturating_sub(3));
        row_pill(look, f.buffer_mut(), lx - 1, ay, rx - lx + 2, t.card2, c.bg);
        let opts = app.answer_options(term);
        let full: u16 = opts.iter().take(4).map(|o| o.width() as u16 + 6).sum();
        // In a narrow pane the label shrinks to its dot, then the buttons to their keys.
        let roomy = rx.saturating_sub(lx) >= full + 11;
        let label = if roomy { "● answer   " } else { "● " };
        let mut x = put(f.buffer_mut(), lx, ay, &[seg(label, Style::default().fg(t.blocked).bg(t.card2).add_modifier(Modifier::BOLD))], rx);
        let keys_only = rx.saturating_sub(x) < full;
        for (i, o) in opts.iter().enumerate().take(4) {
            let key = char::from_digit(i as u32 + 1, 10).unwrap_or('1').to_string();
            let label = if keys_only { String::new() } else { o.clone() };
            let segs = button_pill(look, t, &label, &key, i == 0, t.card2);
            let br = Rect { x, y: ay, width: segs_width(&segs), height: 1 };
            x = put(f.buffer_mut(), x, ay, &segs, rx) + 1;
            app.hits.push((br, Hit::Button(crate::client::Btn::Answer(term, key.chars().next().unwrap_or('1')))));
        }
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
