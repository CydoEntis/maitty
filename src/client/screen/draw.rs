//! The main screen: the sidebar, the panes (the top-left one carrying the tabs in its title),
//! the sheet, the pill at the top right and the toast.

use super::*;
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
pub(super) const RAIL_WIDTH: u16 = 7;
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

/// The grid, with no outer margin: the sidebar the full height at one side, then the pane
/// column from row 0. Cards floating leave the gap setting between them, tiled none (the two
/// borders sit side by side); Flat starts the panes on the sidebar's last column, so one `│`
/// parts them. A width you dragged the sidebar to wins over the default.
pub(in crate::client) fn grid(app: &App, area: Rect) -> Grid {
    let look = Look::of(&app.cfg.ui);
    let full = app.screen.saved.side_w.unwrap_or_else(|| side_w(area.width)).clamp(SIDE_MIN, SIDE_MAX).min(area.width / 2);
    let rail = RAIL_WIDTH.min(full);
    let sw = rail + ((full - rail) as f32 * app.side_frac).round() as u16;
    let gap = if sw == 0 { 0 } else { look.gap };
    let (side, col) = if app.cfg.ui.sidebar_position == "right" {
        let side = Rect { x: area.right().saturating_sub(sw), width: sw, ..area };
        (side, Rect { width: area.width.saturating_sub(sw + gap), ..area })
    } else {
        let overlap = u16::from(look.flat && sw > 0);
        let x = area.x + sw + gap - overlap;
        (Rect { width: sw, ..area }, Rect { x, width: area.right().saturating_sub(x), ..area })
    };
    let edge = if side.x > col.x { Rect { x: col.right(), width: gap.max(1), ..area } } else { Rect { x: col.x.saturating_sub(gap), width: gap.max(1), ..area } };
    let (sheet, panes) = if open_sheet(app).is_some() { split_for_sheet(area.width, col, look, app.sheet_frac) } else { (Rect::default(), col) };
    Grid { side, panes, edge, sheet }
}

/// Draw the main screen; returns the pane area.
pub(in crate::client) fn draw(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) -> Rect {
    // How far the sidebar and the sheet are in this frame (they slide when they change).
    let on = app.motion_on();
    app.side_frac = app.motion.side(app.sidebar, on);
    app.sheet_frac = app.motion.sheet(open_sheet(app).is_some(), on);
    let model = app.screen_model();
    app.screen.wt_keys.clear();
    app.screen.branch_keys.clear();
    fill(f.buffer_mut(), area, t.bg);
    let g = grid(app, area);
    // Too narrow mid-slide to draw anything sensible: just the room it takes.
    if !app.sidebar {
        rail::draw(app, f.buffer_mut(), g.side, &model, t);
    } else if g.side.width >= SIDE_DRAWN_FROM {
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
    }
    if g.panes.width > 0 {
        draw_main(app, f, g.panes, &model, t);
    }
    rail::hint(app, f.buffer_mut(), g.panes, &model, t);
    if let Some(kind) = open_sheet(app).filter(|_| g.sheet.width >= SHEET_DRAWN_FROM) {
        draw_sheet(app, f.buffer_mut(), g.sheet, kind, t);
    }
    draw_corner_pill(app, f.buffer_mut(), area, t);
    app.screen.crumb_x = g.panes.x + 1;
    // A popup (`seshi popup`) floats over everything, the rest dimmed.
    match app.mode.clone() {
        Mode::Compose(c) if c.inbox.is_none() => draw_compose_pop(app, f.buffer_mut(), area, g.side, &c, t),
        Mode::Why { term, .. } => draw_why_pop(app, f.buffer_mut(), area, g.side, term, t),
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
    draw_toast(app, f.buffer_mut(), area, t);
    // Files open over everything in one tool-window size; Esc closes. (Changes is a sheet.)
    if let Some(view) = app.view.take_if(|v| matches!(v, crate::client::View::Files(_))) {
        let buf = f.buffer_mut();
        dim_all(buf, area, t);
        hit(app, area, ScreenHit::Noop);
        let r = panel_at(app, buf, tool_rect(area), "Files", &[], t);
        // The tool draws inside with a cell of air.
        let inner = Rect { x: r.x + 2, y: r.y + 1, width: r.width.saturating_sub(4), height: r.height.saturating_sub(2) };
        match view {
            crate::client::View::Files(v) => {
                crate::client::design::draw_files(app, buf, inner, t, &v);
                app.view = Some(crate::client::View::Files(v));
            }
            // In the sheet (above).
            v @ (crate::client::View::Changes(_) | crate::client::View::Both(_) | crate::client::View::Check(_)) => app.view = Some(v),
        }
    }
    g.panes
}

/// The pill at the top right of row 0: the leader while it's armed, else Update when a newer
/// release is out.
fn corner_pill(app: &App, t: &Theme) -> Option<(Vec<Seg>, ScreenHit)> {
    if armed(app) {
        let lead = app.keymap.prefix.to_string().replace("C-", "Ctrl+");
        return Some((vec![seg(format!(" {lead} "), Style::default().fg(t.bg).bg(t.sky()).add_modifier(Modifier::BOLD))], ScreenHit::Leader));
    }
    let version = app.update_available.as_ref()?;
    Some((vec![seg(format!(" Update {version} "), Style::default().fg(t.accent).bg(t.btn).add_modifier(Modifier::BOLD))], ScreenHit::Update))
}

fn draw_corner_pill(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme) {
    let Some((segs, h)) = corner_pill(app, t) else { return };
    let w = segs_width(&segs);
    let x = area.right().saturating_sub(w + 1);
    put(buf, x, area.y, &segs, area.right());
    hit(app, Rect { x, y: area.y, width: w, height: 1 }, h);
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
        put(f.buffer_mut(), area.x + 4, area.y + 3, &[seg(format!("Nothing open. Press {} {nk} for a new session.", app.keymap.prefix.to_string().replace("C-", "Ctrl+")), Style::default().fg(t.muted))], area.right());
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
/// current one in the accent. Tabs that don't fit drop from the right; a click on one picks it.
fn tab_strip(app: &mut App, model: &[Proj], x: u16, y: u16, max_x: u16, lit: bool, t: &Theme) -> Vec<Seg> {
    let mut out = Vec::new();
    let mut at = x;
    for (n, i) in app.session_tabs().into_iter().enumerate() {
        let tab = &app.screen.tabs[i];
        let name = tab_name(tab).unwrap_or_else(|| session_name(app, model, tab.focus));
        let on = i == app.screen.tab;
        let number = if on { Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(t.text).bg(t.btn) };
        let label = if on { name_chip(&name, lit, t) } else { seg(format!(" {name} "), Style::default().fg(t.text).bg(t.card)) };
        let pair = [seg(format!(" {} ", n + 1), number), label];
        let w = segs_width(&pair);
        if at + w > max_x {
            break;
        }
        hit(app, Rect { x: at, y, width: w, height: 1 }, ScreenHit::TabPick(i));
        out.extend(pair);
        at += w;
    }
    out
}

/// A session's mark on its pane: glyph and word, bold in its colour (working is the
/// Appearance spinner). A shell has none.
fn pane_mark(app: &App, info: &TermInfo, t: &Theme) -> Option<(Seg, u16)> {
    info.agent.as_ref()?;
    let set = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let spinner = glyph(app, Status::Working);
    let step = if app.motion_on() { check::ring_step(app.spinner_frame(), set.ring.len()) } else { 0 };
    let m = Kind::of(info.status, info.check(), info.asleep).mark(set, t, step, &spinner);
    let text = format!("{} {}", m.glyph, m.word);
    let w = text.width() as u16;
    Some((seg(text, Style::default().fg(m.color).add_modifier(Modifier::BOLD)), w))
}

/// Room the top-right pane leaves at the end of its title for the Update pill.
fn corner_room(app: &App, r: Rect, screen: Rect, t: &Theme) -> u16 {
    if armed(app) || r.y != screen.y || r.right() != screen.right() {
        return 0;
    }
    corner_pill(app, t).map_or(0, |(segs, _)| segs_width(&segs))
}

/// A pane as a card. Its title: the tabs of the session on screen when it's the top-left pane
/// of the tab, else one chip with its session's name; on the right, the session's mark (it
/// opens why) and a ✕ that closes the pane. The terminal sits inside, 3 columns and 2 rows in.
/// Unfocused cards fade; one that needs you keeps its amber.
pub(in crate::client) fn draw_session(app: &mut App, f: &mut Frame, r: Rect, term: TermId, focused: bool, model: &[Proj], t: &Theme) {
    let Some(info) = app.snap.terms.get(&term).cloned() else { return };
    let look = Look::of(&app.cfg.ui);
    let st = info.status;
    // While the sidebar has the keys, no pane shows as focused.
    // Exactly one lit edge: an open sheet has the keys, so the pane gives up its lit border.
    let focused = focused && app.mode != Mode::Side && open_sheet(app).is_none();
    let scrolled = app.parsers.get(&term).map(|p| p.screen().scrollback()).filter(|n| *n > 0);
    let border = if !focused {
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
    if let Some((note, colour)) = note {
        c.right.push(seg(format!("{note}  "), Style::default().fg(colour)));
    }
    if let Some((m, _)) = &mark {
        c.right.push(m.clone());
    }
    c.close = Some(ScreenHit::CloseSplit(term));
    let room = corner_room(app, r, f.area(), t);
    if room > 0 {
        c.right.push(seg(" ".repeat(room as usize), Style::default()));
    }
    if let Some(parser) = app.parsers.get(&term) {
        c.bg = crate::client::render::terminal_surface_bg(parser.screen(), c.bg);
    }
    if look.flat {
        c.bg = t.card;
    }
    let terminal_bg = if look.flat { flat_pane_bg(t) } else { c.bg };
    if focused {
        c = c.lit(border);
    }
    // The title runs up to two cells before the right-hand segments ("  ✕ " is the close).
    let right_w = segs_width(&c.right) + if c.right.is_empty() { 2 } else { 4 };
    let title_end = right_end(look, r).saturating_sub(right_w + 2);
    let top_left = app.screen.leaf_rects.iter().min_by_key(|(_, lr)| (lr.y, lr.x)).is_some_and(|(id, _)| *id == term);
    c.chips = if top_left {
        tab_strip(app, model, title_x(look, r), r.y, title_end, focused, t)
    } else {
        let name = truncate(&session_name(app, model, term), title_end.saturating_sub(title_x(look, r) + 2) as usize);
        vec![name_chip(&name, focused, t)]
    };
    let buf = f.buffer_mut();
    let inside = card(app, buf, r, &c, t);
    if let Some((_, w)) = mark {
        let mx = right_end(look, r).saturating_sub(room + 4 + w);
        hit(app, Rect { x: mx, y: r.y, width: w, height: 1 }, ScreenHit::Why(term));
    }
    app.pane_frames.push((term, r));

    // The terminal: 3 columns in, a row below the title, down to the last row inside.
    // Answer buttons only for a question asked through seshi (ask-human), which has nowhere
    // else to be answered: an agent's own question is answered in its own prompt.
    let ask = st == Status::Blocked && info.agent.is_some() && app.pending_question(term).is_some();
    let below = 1 + if ask { 2 } else { 0 };
    let inner = Rect { x: r.x + 3, y: r.y + 2, width: r.width.saturating_sub(6), height: r.height.saturating_sub(2 + below) };
    app.panes.push((term, inner));
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
            && app.view.is_none()
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

    // Scrolled up: say so, and how to get back.
    if let Some(n) = scrolled {
        let note = pill(
            look,
            vec![seg(format!(" ↑ {n} lines up "), Style::default().fg(t.acc_ink).add_modifier(Modifier::BOLD))],
            t.accent,
            c.bg,
        );
        let w = segs_width(&note);
        put(f.buffer_mut(), inner.right().saturating_sub(w), inner.y, &note, inner.right());
    }

    // Not the one you're in: its inside fades (the border and its amber tag stay as they are).
    if !focused {
        dim_inside(f.buffer_mut(), inside, look.dim, t);
    }

    // Answer bar: the choices of the question asked through seshi, a full-width pill that
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
            let segs = button_pill(look, t, &label, &key, i == 0, false, t.card2);
            let br = Rect { x, y: ay, width: segs_width(&segs), height: 1 };
            x = put(f.buffer_mut(), x, ay, &segs, rx) + 1;
            app.hits.push((br, Hit::Button(crate::client::Btn::Answer(term, key.chars().next().unwrap_or('1')))));
        }
    }
}

/// How long a toast stays: an error longer, one about a session (a click goes there) longer
/// still, one with a button longest.
pub(in crate::client) fn toast_hold(err: bool, about: bool, action: bool) -> std::time::Duration {
    std::time::Duration::from_millis(if action {
        12000
    } else if err {
        4500
    } else if about {
        6000
    } else {
        2500
    })
}

/// How far a toast's hint ("click to open") sinks into its fill: readable, second to the news.
const TOAST_HINT_FADE: f32 = 0.3;
/// How far toward its mark's colour a toast is when it arrives.
const TOAST_FLASH_BRIGHT: f32 = 0.35;

/// A note (copied, saved, couldn't …) at the bottom right, on the second row from the bottom
/// and three columns from the edge, raised (`btn`) in `strong`; its mark says what kind. It
/// goes after a few seconds (errors stay a little longer).
pub(in crate::client) fn draw_toast(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme) {
    let Some((msg, at, err)) = app.notice.clone() else { return };
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
    let hint = if about.is_some() { "   click to open" } else { "" };
    // The update's own sentence is long; with the button the toast keeps to the news.
    let msg = match (restart, &app.installed_version) {
        (true, Some(Ok(v))) => format!("seshi {v} installed"),
        _ => msg,
    };
    let text = truncate(&msg, (area.width.saturating_sub(16) as usize).saturating_sub(hint.width()));
    let needs = about.and_then(|term| app.snap.terms.get(&term)).is_some_and(|x| x.status == Status::Blocked);
    let (colour, mark) = match (err, needs, about) {
        (true, ..) => (t.err, "✕ "),
        (_, true, _) => (t.blocked, "● "),
        (_, _, Some(_)) => (t.done, "✓ "),
        _ => (t.accent, "✓ "),
    };
    // It arrives tinted with its mark's colour and settles into the raised grey.
    let flash = if on { (1.0 - shown.as_secs_f32() / crate::client::motion::TOAST_FLASH.as_secs_f32()).max(0.0) } else { 0.0 };
    let fill_c = blend(t.btn, colour, flash * TOAST_FLASH_BRIGHT);
    let look = Look::of(&app.cfg.ui);
    let mut body = vec![seg(mark, Style::default().fg(colour).add_modifier(Modifier::BOLD)), seg(text, Style::default().fg(t.strong)), seg(hint, Style::default().fg(blend(t.strong, fill_c, TOAST_HINT_FADE)))];
    if restart {
        body.push(seg("  ", Style::default()));
        body.extend(button_pill(look, t, "Restart now", "", false, false, fill_c));
    }
    let segs = pill(look, body, fill_c, t.bg);
    let w = segs_width(&segs);
    if area.height < 4 || area.width < w + 6 {
        return;
    }
    let fx = area.right().saturating_sub(w + 2);
    let x = fx + ((area.right().saturating_sub(fx)) as f32 * slide).round() as u16;
    let segs: Vec<Seg> = if fade > 0.0 {
        segs.into_iter()
            .map(|(s, st)| {
                let st = match (st.fg, st.bg) {
                    (Some(fg), Some(bg)) => st.fg(blend(fg, bg, fade)).bg(blend(bg, t.bg, fade)),
                    (Some(fg), None) => st.fg(blend(fg, t.bg, fade)),
                    _ => st,
                };
                (s, st)
            })
            .collect()
    } else {
        segs
    };
    let r = Rect { x, y: area.bottom().saturating_sub(2), width: area.right().saturating_sub(x).min(w), height: 1 };
    put(buf, r.x, r.y, &segs, area.right());
    if let Some(term) = about {
        hit(app, r, ScreenHit::Session(term));
    } else if restart {
        hit(app, r, ScreenHit::Restart);
    }
}
