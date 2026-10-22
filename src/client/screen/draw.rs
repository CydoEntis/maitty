//! The main screen: the sidebar card, the tab row, the pane cards and the toast.

use super::*;

// ---- main screen ---------------------------------------------------------------------------

pub(in crate::client) fn side_w(width: u16) -> u16 {
    if width < NARROW { 26 } else { 34 }
}

/// The narrowest the sidebar (sliding) or the sheet is drawn; below it, it's only space.
const SIDE_DRAWN_FROM: u16 = 12;
pub(super) const RAIL_WIDTH: u16 = 7;
pub(in crate::client) const SHEET_DRAWN_FROM: u16 = 24;
/// The narrowest a pane is drawn while it grows in.
const PANE_DRAWN_FROM: u16 = 8;

/// Below this many columns the layout tightens: a slimmer sidebar without its right-hand
/// meta, and tabs other than the current one shrink to their number and state.
pub(in crate::client) const NARROW: u16 = 140;

/// Where everything goes: the sidebar card, the tab row and the pane area.
pub(in crate::client) struct Grid {
    pub side: Rect,
    pub tabs: Rect,
    pub panes: Rect,
    /// Between the sidebar and the right column (drag it to resize).
    pub edge: Rect,
    /// The sheet docked right of the panes, when one is open (else empty).
    pub sheet: Rect,
}

/// The floating grid: a margin of a column and a row, the sidebar card the full height, a
/// column of gap, then the tab row and the pane cards directly under it. Tiled packs it all edge to edge.
pub(in crate::client) fn grid(app: &App, area: Rect) -> Grid {
    let look = Look::of(&app.cfg.ui);
    if look.flat {
        let height = area.height;
        let default_side_w = if area.width >= 200 { 40 } else { side_w(area.width) };
        let full = app.screen.saved.side_w.unwrap_or(default_side_w).clamp(SIDE_MIN, SIDE_MAX).min(area.width / 2);
        let rail = RAIL_WIDTH.min(full);
        let sw = rail + ((full - rail) as f32 * app.side_frac).round() as u16;
        let side = if app.cfg.ui.sidebar_position == "right" {
            Rect { x: area.right().saturating_sub(sw), y: area.y, width: sw, height }
        } else {
            Rect { x: area.x, y: area.y, width: sw, height }
        };
        let col_x = if app.cfg.ui.sidebar_position == "right" { area.x } else { side.right() };
        let col = Rect { x: col_x, y: area.y, width: area.width.saturating_sub(sw), height };
        let tabs = Rect { x: col.x, y: col.y, width: col.width, height: 1 };
        let panes = Rect { x: col.x, y: col.y.saturating_add(1), width: col.width, height: col.height.saturating_sub(1) };
        let (sheet, panes) = if open_sheet(app).is_some() {
            let full = if area.width < NARROW || panes.width <= super::sheet::SHEET_W + 20 { panes.width } else if area.width >= super::sheet::WIDE_AT { super::sheet::SHEET_W_WIDE.min(panes.width) } else { super::sheet::SHEET_W.min(panes.width) };
            let w = (full as f32 * app.sheet_frac.clamp(0.0, 1.0)).round() as u16;
            let sheet = Rect { x: panes.right().saturating_sub(w), y: area.y, width: w, height };
            (sheet, Rect { x: panes.x, y: panes.y, width: panes.width.saturating_sub(w), height: panes.height })
        } else { (Rect::default(), panes) };
        let edge_x = if app.cfg.ui.sidebar_position == "right" { side.x.saturating_sub(1) } else { side.right() };
        let edge = Rect { x: edge_x, y: area.y, width: 1, height };
        return Grid { side, tabs, panes, edge, sheet };
    }
    let (mx, my, gx) = if look.tiled { (0, 0, 0) } else { (1, 1, 1) };
    let body = area.height;
    let inner = Rect { x: area.x + mx, y: area.y + my, width: area.width.saturating_sub(2 * mx), height: body.saturating_sub(2 * my) };
    let full = app.screen.saved.side_w.unwrap_or_else(|| side_w(area.width)).clamp(SIDE_MIN, SIDE_MAX).min(inner.width / 2);
    let rail = RAIL_WIDTH.min(full);
    let sw = rail + ((full - rail) as f32 * app.side_frac).round() as u16;
    let right_side = app.cfg.ui.sidebar_position == "right";
    let gap = if sw > 0 { gx } else { 0 };
    let col_w = inner.width.saturating_sub(sw + gap);
    let (side, col, edge) = if right_side {
        let side = Rect { x: inner.right().saturating_sub(sw), width: sw, ..inner };
        (side, Rect { width: col_w, ..inner }, Rect { x: side.x.saturating_sub(gap.max(1)), width: gap.max(1), ..inner })
    } else {
        let side = Rect { width: sw, ..inner };
        (side, Rect { x: inner.x + sw + gap, width: col_w, ..inner }, Rect { x: side.right(), width: gap.max(1), ..inner })
    };
    // The tab row is the column's top row, the pane cards directly under it.
    let tabs = Rect { height: 1, ..col };
    let top = tabs.y + 1;
    let panes = Rect { y: top, height: col.bottom().saturating_sub(top), ..col };
    let (sheet, panes) = if open_sheet(app).is_some() { split_for_sheet(area.width, panes, look.gap.max(1), app.sheet_frac) } else { (Rect::default(), panes) };
    Grid { side, tabs, panes, edge, sheet }
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
        // The gap beside the sidebar: drag it. It lights up while you point at it.
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
    let col = Rect { y: g.tabs.y, height: g.panes.bottom().saturating_sub(g.tabs.y), ..g.tabs };
    if g.panes.width > 0 {
        draw_main(app, f, Rect { width: g.panes.width, ..col }, &model, t);
    }
    rail::hint(app, f.buffer_mut(), g.panes, &model, t);
    if let Some(kind) = open_sheet(app).filter(|_| g.sheet.width >= SHEET_DRAWN_FROM) {
        draw_sheet(app, f.buffer_mut(), g.sheet, kind, t);
    }
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
    // A sheet that takes the pane column would hide the toast, so it shows over the sheet.
    let toast_at = if g.panes.width < TOAST_ROOM && g.sheet.width >= SHEET_DRAWN_FROM { g.sheet } else { g.panes };
    draw_toast(app, f.buffer_mut(), toast_at, t);
    // Files open over everything in one tool-window size; Esc closes. (Changes is a sheet.)
    if let Some(view) = app.view.take_if(|v| matches!(v, crate::client::View::Files(_))) {
        let buf = f.buffer_mut();
        dim_all(buf, area, t);
        let frame = tool_rect(area);
        hit(app, area, ScreenHit::Noop);
        // A lit card around it; the tool draws inside with a cell of air.
        let mut c = Card::new(t, "").lit(t.accent);
        c.bg = t.bg;
        let inside = card(app, buf, frame, &c, t);
        let inner = Rect { x: inside.x + 1, width: inside.width.saturating_sub(2), ..inside };
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
    draw_tab_bar(app, f.buffer_mut(), Rect { height: 1, ..area }, t);
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
    let skip = if look.flat || look.tiled { 1 } else { 2 };
    let area = Rect { y: area.y + skip, height: area.height.saturating_sub(skip), ..area };
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

/// The most urgent state among the panes of a tab, for its pill.
fn tab_state(app: &App, tab: &ScreenTab) -> Option<Status> {
    tab.layout
        .leaves()
        .iter()
        .filter_map(|id| app.snap.terms.get(id))
        .filter(|i| i.agent.is_some())
        .map(|i| i.status)
        .min_by_key(|s| rank(*s))
}

/// The name you gave a tab, if any. "tab 2" (what an earlier version named new tabs) isn't
/// one: the pill shows the number already.
pub(in crate::client) fn tab_name(tab: &ScreenTab) -> Option<String> {
    let n = tab.name.trim();
    let generic = n.strip_prefix("tab ").is_some_and(|d| d.chars().all(|c| c.is_ascii_digit()));
    (!n.is_empty() && !generic).then(|| n.to_string())
}

/// Leader mode is on: waiting for the key, or showing the key map.
pub(in crate::client) fn armed(app: &App) -> bool {
    matches!(app.mode, Mode::Prefix { .. } | Mode::KeyMap(_))
}

/// The tab row: a pill per tab (number, name, its most urgent state; the current one in the
/// accent), a + pill, and the leader pill at the far right while leader mode is on.
pub(in crate::client) fn draw_tab_bar(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme) {
    let mut look = Look::of(&app.cfg.ui);
    if look.flat { look.caps = false; }
    let pb = if look.flat { t.card } else { pane_bg(t) };
    fill(buf, r, if look.flat { t.card } else { t.bg });
    let small = buf.area.width < NARROW;
    // The right end first, so the tabs know where to stop.
    let right: Vec<Seg> = if armed(app) {
        let ink = Style::default().fg(t.bg).add_modifier(Modifier::BOLD);
        let lead = app.keymap.prefix.to_string().replace("C-", "Ctrl+").to_uppercase();
        let mut segs = vec![seg(format!(" ⌨ {lead} "), ink)];
        if !small {
            segs.push(seg(" press a key · ? shortcuts · Esc ", Style::default().fg(t.bg)));
        }
        pill(look, segs, t.sky(), if look.flat { t.card } else { t.bg })
    } else if let Some(version) = &app.update_available {
        pill(look, vec![seg(format!(" Update {version} "), Style::default().fg(t.accent).add_modifier(Modifier::BOLD))], t.card, if look.flat { t.card } else { t.bg })
    } else {
        Vec::new()
    };
    let rw = segs_width(&right);
    let rx = r.right().saturating_sub(rw);
    if rw > 0 {
        put(buf, rx, r.y, &right, r.right());
        hit(app, Rect { x: rx, y: r.y, width: rw, height: 1 }, if armed(app) { ScreenHit::Leader } else { ScreenHit::Update });
    }
    let mut x = r.x;
    let tabs = app.session_tabs();
    for (n, i) in tabs.iter().copied().enumerate() {
        let tab = &app.screen.tabs[i];
        let on = i == app.screen.tab;
        let state = tab_state(app, tab);
        let name = if small && !on {
            String::new()
        } else {
            tab_name(tab).unwrap_or_default()
        };
        // Its name if you gave it one, else its number.
        let fg = if on { t.acc_ink } else { t.text };
        let label = if name.is_empty() { format!("{}", n + 1) } else { truncate(&name, 24) };
        let mut segs = vec![seg(" ", Style::default()), seg(label, Style::default().fg(fg).add_modifier(Modifier::BOLD))];
        if let Some(st) = state {
            let c = if st == Status::Blocked { t.blocked } else if on { t.acc_ink } else { t.muted };
            let mut gs = Style::default().fg(c);
            if st == Status::Blocked {
                gs = gs.add_modifier(Modifier::BOLD);
            }
            segs.push(seg(format!(" {}", glyph(app, st)), gs));
        }
        let closable = on && tabs.len() > 1;
        if closable {
            segs.push(seg("  ✕", Style::default().fg(t.acc_ink)));
        }
        segs.push(seg(" ", Style::default()));
        let bg = if on { t.accent } else { pb };
        let segs = pill(look, segs, bg, if look.flat { t.card } else { t.bg });
        let w = segs_width(&segs);
        if x + w + 6 > rx {
            break;
        }
        let cr = Rect { x, y: r.y, width: w, height: 1 };
        let segs = if !on && hovered(app, cr) { segs.into_iter().map(|(s, st)| (s, if st.bg == Some(pb) { st.bg(t.hov) } else if st.fg == Some(pb) { st.fg(t.hov) } else { st })).collect() } else { segs };
        put(buf, x, r.y, &segs, rx);
        hit(app, cr, ScreenHit::TabPick(i));
        if closable {
            // The ✕ sits before the pill's pad and end cap.
            hit(app, Rect { x: x + w - 3, y: r.y, width: 1, height: 1 }, ScreenHit::TabClose(i));
        }
        x += w + if look.flat { 0 } else { 1 };
    }
    let plus = pill(look, vec![seg(" + ", Style::default().fg(t.accent).add_modifier(Modifier::BOLD))], if hovered(app, Rect { x, y: r.y, width: 5, height: 1 }) { t.hov } else { pb }, t.bg);
    let pw = segs_width(&plus);
    if x + pw <= rx {
        put(buf, x, r.y, &plus, rx);
        hit(app, Rect { x, y: r.y, width: pw, height: 1 }, ScreenHit::TabNew);
    }
}

/// A pane as a card: its name in the top border with its repo after it, state and ✕ on the
/// right, the terminal inside (3 columns and a row in) down to the last row. Where it runs and
/// its branch are in the status line; model, context and spend are the agent's own. Unfocused
/// cards fade; one that needs you keeps its amber.
pub(in crate::client) fn draw_session(app: &mut App, f: &mut Frame, r: Rect, term: TermId, focused: bool, model: &[Proj], t: &Theme) {
    let Some(info) = app.snap.terms.get(&term).cloned() else { return };
    let look = Look::of(&app.cfg.ui);
    let found = find(model, term);
    let st = info.status;
    // While the sidebar has the keys, no pane shows as focused.
    // Exactly one lit edge: an open sheet has the keys, so the pane gives up its lit border.
    let focused = focused && app.mode != Mode::Side && open_sheet(app).is_none();
    let (name, sub) = found
        .map(|(p, _, s)| {
            // What the agent is on comes first: no other place on screen says it.
            let on = if s.is_agent && s.title != WAITING && !s.title.is_empty() && s.title != s.name { format!("{} · ", s.title) } else { String::new() };
            (s.name.clone(), format!("{on}{}", p.name))
        })
        .unwrap_or_else(|| {
            // A pane beside another in a split has no sidebar row of its own: say what it is
            // from the pane itself.
            let agent = match &info.agent {
                Some(a) => a.clone(),
                None if info.is_shell() => "shell".into(),
                None => info.display_name(),
            };
            (agent, folder_name(&info.cwd))
        });
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
    // Just moved here: the border and title flare and settle, so you see where you landed.
    let on = app.motion_on();
    let flash = if focused { app.motion.land(term as u64, on) } else { 0.0 };
    let flare = if app.cfg.ui.focus_border == "bright" { t.accent } else { t.strong };
    let border = if flash > 0.0 { blend(border, flare, flash) } else { border };
    let mut c = Card::new(t, &name);
    c.sub = &sub;
    c.state = info.agent.is_some().then_some(st).filter(|s| *s != Status::None);
    c.state_hit = c.state.map(|_| ScreenHit::Why(term));
    // What it's taking of the machine, beside its state: where you look when the PC is slow.
    c.note = match info.resume_at {
        Some(at) => Some((format!("⏸ limit · continues in {}", until(at)), t.muted)),
        None => load_note(info.cpu, info.mem).map(|(load, heavy)| (load, if heavy { t.err } else { t.muted })),
    };
    c.close = Some(ScreenHit::CloseSplit(term));
    if let Some(parser) = app.parsers.get(&term) {
        c.bg = crate::client::render::terminal_surface_bg(parser.screen(), c.bg);
    }
    if look.flat {
        c.bg = t.card;
    }
    let terminal_bg = if look.flat { flat_pane_bg(t) } else { c.bg };
    if focused {
        c = c.lit(border);
        c.title_fg = if flash > 0.0 { blend(t.accent, flare, flash) } else { t.accent };
    }
    let buf = f.buffer_mut();
    let inside = card(app, buf, r, &c, t);
    app.pane_frames.push((term, r));

    // The terminal: 3 columns in, a row below the border, down to the last row inside.
    // Answer buttons only for a question asked through seshi (ask-human), which has nowhere
    // else to be answered: an agent's own question is answered in its own prompt.
    let ask = st == Status::Blocked && info.agent.is_some() && app.pending_question(term).is_some();
    let below = 3 + if ask { 2 } else { 0 };
    let pane_top = if look.flat { 3 } else { 2 };
    let inner = Rect { x: r.x + 3, y: r.y + pane_top, width: r.width.saturating_sub(6), height: r.height.saturating_sub(pane_top + below) };
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
            let w = segs_width(&button_pill(look, t, &label, &key, i == 0, false, t.card2));
            let br = Rect { x, y: ay, width: w, height: 1 };
            let segs = button_pill(look, t, &label, &key, i == 0, hovered(app, br), t.card2);
            x = put(f.buffer_mut(), x, ay, &segs, rx) + 1;
            app.hits.push((br, Hit::Button(crate::client::Btn::Answer(term, key.chars().next().unwrap_or('1')))));
        }
    }
}

/// A note (copied, saved, couldn't …) as a small pop-up just above the bottom bar, centred
/// over the panes; it goes after a few seconds (errors stay a little longer).
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

/// Panes narrower than this leave no room for a toast.
const TOAST_ROOM: u16 = 40;

/// How far a toast's hint ("click to open") sinks into its fill: readable, second to the news.
const TOAST_HINT_FADE: f32 = 0.3;
/// How far toward white a toast is when it arrives.
const TOAST_FLASH_BRIGHT: f32 = 0.55;

pub(in crate::client) fn draw_toast(app: &mut App, buf: &mut Buffer, panes: Rect, t: &Theme) {
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
    let text = truncate(&msg, (panes.width.saturating_sub(16) as usize).saturating_sub(hint.width()));
    // Filled with the colour of what it says, so it's seen from the corner of your eye: an
    // agent that needs you or finished wears its state, an error the error colour, a plain
    // note the accent. On the card's own grey it was easy to miss.
    let needs = about.and_then(|term| app.snap.terms.get(&term)).is_some_and(|x| x.status == Status::Blocked);
    let (fill, mark) = match (err, needs, about) {
        (true, ..) => (t.err, "✕ "),
        (_, true, _) => (t.blocked, "● "),
        (_, _, Some(_)) => (t.done, "✓ "),
        _ => (t.accent, "✓ "),
    };
    let ink = t.ink_on(fill);
    // It arrives bright and settles into its colour.
    let flash = if on { (1.0 - shown.as_secs_f32() / crate::client::motion::TOAST_FLASH.as_secs_f32()).max(0.0) } else { 0.0 };
    let bold = Style::default().fg(ink).add_modifier(Modifier::BOLD);
    // A pill inside the top right of the panes, a row below the card's border: away from
    // where you type (an agent's prompt is at the bottom).
    let look = Look::of(&app.cfg.ui);
    let mut body = vec![seg("  ", Style::default()), seg(mark, bold), seg(text, bold), seg(hint, Style::default().fg(blend(ink, fill, TOAST_HINT_FADE)))];
    if restart {
        body.push(seg("  ", Style::default()));
        body.extend(button_pill(look, t, "Restart now", "", false, false, fill));
    }
    body.push(seg("  ", Style::default()));
    let segs = pill(
        look,
        body,
        blend(fill, t.strong, flash * TOAST_FLASH_BRIGHT),
        pane_bg(t),
    );
    let w = segs_width(&segs);
    if panes.height < 4 || panes.width < w + 6 {
        return;
    }
    let fx = panes.right().saturating_sub(w + 3);
    let x = fx + ((panes.right().saturating_sub(fx)) as f32 * slide).round() as u16;
    let segs: Vec<Seg> = if fade > 0.0 {
        let ground = pane_bg(t);
        segs.into_iter()
            .map(|(s, st)| {
                let st = match (st.fg, st.bg) {
                    (Some(fg), Some(bg)) => st.fg(blend(fg, bg, fade)).bg(blend(bg, ground, fade)),
                    (Some(fg), None) => st.fg(blend(fg, ground, fade)),
                    _ => st,
                };
                (s, st)
            })
            .collect()
    } else {
        segs
    };
    let r = Rect { x, y: panes.y + 1, width: panes.right().saturating_sub(x).min(w), height: 1 };
    put(buf, r.x, r.y, &segs, panes.right());
    if let Some(term) = about {
        hit(app, r, ScreenHit::Session(term));
    } else if restart {
        hit(app, r, ScreenHit::Restart);
    }
}
