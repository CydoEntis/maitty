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
/// column of gap, then the tab row and the pane cards directly under it, a blank row and the
/// status line at the foot. Tiled packs it all edge to edge.
pub(in crate::client) fn grid(app: &App, area: Rect) -> Grid {
    let look = Look::of(&app.cfg.ui);
    if look.flat {
        let height = area.height.saturating_sub(1);
        let default_side_w = if area.width >= 200 { 40 } else { side_w(area.width) };
        let full = app.hy.saved.side_w.unwrap_or(default_side_w).clamp(SIDE_MIN, SIDE_MAX).min(area.width / 2);
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
    // The bottom row is the status line; floating cards leave a blank row above it.
    let body = area.height.saturating_sub(1);
    let inner = Rect { x: area.x + mx, y: area.y + my, width: area.width.saturating_sub(2 * mx), height: body.saturating_sub(2 * my) };
    let full = app.hy.saved.side_w.unwrap_or_else(|| side_w(area.width)).clamp(SIDE_MIN, SIDE_MAX).min(inner.width / 2);
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
    let model = app.hy_model();
    app.hy.wt_keys.clear();
    app.hy.branch_keys.clear();
    fill(f.buffer_mut(), area, t.bg);
    let g = grid(app, area);
    // Too narrow mid-slide to draw anything sensible: just the room it takes.
    if !app.sidebar {
        rail::draw(app, f.buffer_mut(), g.side, &model, t);
    } else if g.side.width >= SIDE_DRAWN_FROM {
        draw_side(app, f.buffer_mut(), g.side, &model, t);
        // The gap beside the sidebar: drag it. It lights up while you point at it.
        if app.hy.drag == Some(Drag::Side) || hovered(app, g.edge) {
            let ex = g.edge.x + g.edge.width / 2;
            for yy in g.side.top() + 1..g.side.bottom().saturating_sub(1) {
                if let Some(px) = f.buffer_mut().cell_mut((ex, yy)) {
                    px.set_symbol("│").set_style(Style::default().fg(t.accent).bg(t.bg));
                }
            }
        }
        hit(app, g.edge, HyHit::SideEdge);
    }
    let col = Rect { y: g.tabs.y, height: g.panes.bottom().saturating_sub(g.tabs.y), ..g.tabs };
    if g.panes.width > 0 {
        draw_main(app, f, Rect { width: g.panes.width, ..col }, &model, t);
    }
    rail::hint(app, f.buffer_mut(), g.panes, &model, t);
    if let Some(kind) = open_sheet(app).filter(|_| g.sheet.width >= SHEET_DRAWN_FROM) {
        draw_sheet(app, f.buffer_mut(), g.sheet, kind, t);
    }
    app.hy.crumb_x = g.panes.x + 1;
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
    draw_toast(app, f.buffer_mut(), g.panes, t);
    draw_status_line(app, f.buffer_mut(), area, &model, t);
    // Files open over everything in one tool-window size; Esc closes. (Changes is a sheet.)
    if let Some(view) = app.view.take_if(|v| matches!(v, crate::client::View::Files(_))) {
        let buf = f.buffer_mut();
        dim_all(buf, area, t);
        let frame = tool_rect(area);
        hit(app, area, HyHit::Noop);
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
            v @ (crate::client::View::Changes(_) | crate::client::View::Both(_)) => app.view = Some(v),
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

/// A line of the sidebar tree.
#[derive(Debug, Clone)]
pub(in crate::client) enum Line {
    /// A section's heading: AGENTS, TERMINALS, SSH (with how many sessions).
    Section(Kind, usize),
    Proj(usize),
    /// A heading: BRANCHES or WORKTREES.
    /// An agent or shell (pi, wi, si), and how deep it sits in a fleet (0: not under anyone).
    Sess(usize, usize, usize, u8),
    /// A dim line under an agent: what it's on, its question, a subagent. Shares the
    /// agent's highlight.
    Note(String, Color, TermId),
    /// A live helper, with a working marker and a colour keyed to its role.
    Subagent(String, Color, TermId),
    /// The heading over a session's subagents: how many, and whether they're listed.
    Subs(TermId, usize, bool),
    /// Nothing running in a folder project.
    Empty(usize),
    /// A project's settled group: how many sessions are set aside, and whether it's unfolded.
    Aside(usize, usize, bool),
    Gap,
}

/// What a sidebar row shows of a session: the agent and its state on top, what it's working
/// on (or asking) underneath.
/// A session's load is said under its row from here: a fifth of the machine's CPU, or two
/// gigabytes. And in the working colour from here, where it's the reason the PC is slow.
const LOAD_SAID_CPU: f32 = 20.0;
const LOAD_SAID_MEM: u64 = 2 << 30;
const LOAD_HEAVY_CPU: f32 = 50.0;
const LOAD_HEAVY_MEM: u64 = 6 << 30;

/// "▲ 38% CPU · 2.1 GB" for a session that's taking a real share of the machine, and whether
/// it's heavy. Nothing for one that isn't.
pub(in crate::client) fn load_note(cpu: f32, mem: u64) -> Option<(String, bool)> {
    if cpu < LOAD_SAID_CPU && mem < LOAD_SAID_MEM {
        return None;
    }
    let gb = mem as f64 / f64::from(1u32 << 30);
    Some((format!("▲ {cpu:.0}% CPU · {gb:.1} GB"), cpu >= LOAD_HEAVY_CPU || mem >= LOAD_HEAVY_MEM))
}

pub(in crate::client) fn session_lines(s: &Session, subs_closed: bool, t: &Theme, out: &mut Vec<Line>) {
    let notes_c = t.muted;
    if let Some(q) = &s.question {
        out.push(Line::Note(q.clone(), blend(t.blocked, t.sidebar_bg, 0.25), s.term));
    }
    // What it's been told to do next, once this turn is over.
    if let Some(next) = s.queued.first() {
        let more = if s.queued.len() > 1 { format!(" (+{})", s.queued.len() - 1) } else { String::new() };
        out.push(Line::Note(format!("⏵ next: {next}{more}"), notes_c, s.term));
    }
    // Its subagents, each on a line, under a heading that folds them away.
    if !s.subagents.is_empty() {
        out.push(Line::Subs(s.term, s.subagents.len(), !subs_closed));
        if !subs_closed {
            out.extend(s.subagents.iter().take(SUBS_LISTED).map(|sub| Line::Subagent(sub.clone(), subagent_color(sub), s.term)));
            if s.subagents.len() > SUBS_LISTED {
                out.push(Line::Note(format!("  +{} more", s.subagents.len() - SUBS_LISTED), notes_c, s.term));
            }
        }
    }
}

fn subagent_color(kind: &str) -> Color {
    let palette = [Color::Rgb(0, 205, 210), Color::Rgb(160, 120, 255), Color::Rgb(240, 90, 190)];
    let normalized = kind.to_ascii_lowercase();
    let role = normalized.split(|c: char| !c.is_ascii_alphanumeric()).find(|part| !part.is_empty()).unwrap_or("");
    let index = match role {
        "planner" => 0,
        "builder" => 1,
        "reviewer" => 2,
        _ => normalized.bytes().fold(0usize, |sum, byte| sum.wrapping_add(usize::from(byte))) % palette.len(),
    };
    palette[index]
}

/// This many of a session's native helper agents are named before they are counted.
const SUBS_LISTED: usize = 8;

/// A helper's row steps in this far under the session that started it, per level.
const FLEET_INDENT: u16 = 2;
/// Deeper helpers stay at this level: the names need the width more than the tree does.
const FLEET_DEPTH: u8 = 2;

/// Sessions of a project, each by (worktree, session) index.
type Rows = Vec<(usize, usize)>;

pub(in crate::client) fn side_lines(app: &App, model: &[Proj], t: &Theme) -> Vec<Line> {
    let mut out = Vec::new();
    for (pi, p) in model.iter().enumerate() {
        if pi == 0 || model[pi - 1].kind != p.kind {
            let n = model.iter().filter(|q| q.kind == p.kind).map(|q| q.sessions().count()).sum();
            out.push(Line::Section(p.kind, n));
        }
        out.push(Line::Proj(pi));
        if app.hy.saved.closed.contains(&format!("p:{}", p.key)) {
            out.push(Line::Gap);
            continue;
        }
        // Just the sessions: the ones that need you first, the rest where they were (the repo
        // folder's before the worktrees' when equal).
        let every: Rows = (0..p.wts.len()).flat_map(|wi| (0..p.wts[wi].sessions.len()).map(move |si| (wi, si))).collect();
        // The ones set aside (settled, snoozed) go under one line at the foot of the project.
        let (aside, mut rows): (Rows, Rows) = every.into_iter().partition(|&(wi, si)| p.wts[wi].sessions[si].aside.is_some());
        // Your order (dragged) otherwise.
        let order = &app.hy.saved.session_order;
        let at = |t: TermId| order.iter().position(|x| *x == t).unwrap_or(usize::MAX);
        let sort = app.cfg.ui.attention_sort;
        rows.sort_by_key(|&(wi, si)| {
            let s = &p.wts[wi].sessions[si];
            let mine = at(s.term);
            (sort && settled(s.status), if mine == usize::MAX { !p.wts[wi].main } else { false }, mine, s.term)
        });
        // A fleet: the sessions a session started sit under it, in the order above. One that
        // needs you has sorted to the top and stays there, on its own.
        let term_of = |r: &(usize, usize)| p.wts[r.0].sessions[r.1].term;
        let commander = |r: &(usize, usize)| {
            let s = &p.wts[r.0].sessions[r.1];
            let floats = sort && !settled(s.status);
            s.started_by.filter(|by| !floats && *by != s.term && rows.iter().any(|x| term_of(x) == *by))
        };
        let mut placed: Vec<((usize, usize), u8)> = Vec::with_capacity(rows.len());
        let mut next: Vec<((usize, usize), u8)> = rows.iter().rev().filter(|r| commander(r).is_none()).map(|r| (*r, 0)).collect();
        while let Some((r, depth)) = next.pop() {
            placed.push((r, depth));
            let below = (depth + 1).min(FLEET_DEPTH);
            next.extend(rows.iter().rev().filter(|k| commander(k) == Some(term_of(&r)) && !placed.iter().any(|(x, _)| x == *k)).map(|k| (*k, below)));
        }
        // Sessions that started each other can't come from the daemon; if they did, they're
        // still listed.
        let lost: Vec<(usize, usize)> = rows.iter().filter(|r| !placed.iter().any(|(x, _)| x == *r)).copied().collect();
        placed.extend(lost.into_iter().map(|r| (r, 0)));
        let mut any = false;
        for ((wi, si), depth) in placed {
            any = true;
            out.push(Line::Sess(pi, wi, si, depth));
            let s = &p.wts[wi].sessions[si];
            session_lines(s, app.hy.subs_closed.contains(&s.term), t, &mut out);
        }
        if !any && aside.is_empty() {
            out.push(Line::Empty(pi));
        }
        if !aside.is_empty() {
            let open = app.hy.aside_open.contains(&p.key);
            out.push(Line::Aside(pi, aside.len(), open));
            for (wi, si) in aside.into_iter().filter(|_| open) {
                out.push(Line::Sess(pi, wi, si, 1));
                let s = &p.wts[wi].sessions[si];
                if let Some(until) = s.aside.filter(|u| *u > 0) {
                    out.push(Line::Note(format!("snoozed · back in {}", until_note(until)), t.muted, s.term));
                }
            }
        }
        out.push(Line::Gap);
    }
    out
}

/// How long until `at` (unix seconds): "40m", "3h", "2d".
fn until_note(at: u64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    match at.saturating_sub(now) {
        s @ 0..=3599 => format!("{}m", s.div_ceil(60)),
        s @ 3600..=86_399 => format!("{}h", s.div_ceil(3600)),
        s => format!("{}d", s.div_ceil(86_400)),
    }
}

/// The session a sidebar line stands for, if any.
pub(in crate::client) fn line_term(model: &[Proj], l: &Line) -> Option<TermId> {
    match l {
        Line::Sess(pi, wi, si, _) => Some(model[*pi].wts[*wi].sessions[*si].term),
        _ => None,
    }
}

/// The sidebar card: projects (● in their colour, bold), their sessions under them most urgent
/// first, a question under a session that needs you; new and actions at the foot.
pub(in crate::client) fn draw_side(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    let look = Look::of(&app.cfg.ui);
    let pb = if look.flat { t.card } else { pane_bg(t) };
    let focused_side = app.mode == Mode::Side;
    let host = crate::ipc::remote().map(|h| format!("⇄ {h}")).unwrap_or_default();
    let mut c = Card::new(t, &host);
    if look.flat { c.bg = t.card; }
    if focused_side {
        c = c.lit(t.accent);
    }
    card(app, buf, r, &c, t);
    app.hy.side_rect = r;
    hit(app, r, HyHit::SideFocus);
    let small = buf.area.width < NARROW;
    let lines = side_lines(app, model, t);
    let shown: Vec<TermId> = app.hy.tabs.get(app.hy.tab).map(|t| t.layout.leaves()).unwrap_or_default();
    // In a split, the split's row (its first pane's) is the open one, whichever side you're on.
    let focus = app.focused().map(|f| if shown.len() > 1 && shown.contains(&f) { shown[0] } else { f });
    // Rows: from 1 below the top border, inset 3; a blank row above the bottom border.
    let (x0, w) = (r.x, r.width);
    let top = r.y + if look.flat { 1 } else { 2 };
    let list_h = if look.flat { r.bottom().saturating_sub(top) as usize } else { r.height.saturating_sub(4) as usize };
    let right = r.right().saturating_sub(if look.flat { 1 } else { 3 });
    // Keep the focused (or cursor) row in view when it changes; otherwise the wheel rules.
    let mut scroll = app.hy.side_scroll as usize;
    if app.hy.follow {
        let want = app.hy.cursor.or(focus).and_then(|term| lines.iter().position(|l| line_term(model, l) == Some(term)));
        if let Some(i) = want {
            if i < scroll {
                scroll = i.saturating_sub(1);
            } else if i + 2 >= scroll + list_h {
                scroll = i + 3 - list_h.min(i + 3);
            }
        }
        app.hy.follow = false;
    }
    scroll = scroll.min(lines.len().saturating_sub(list_h));
    app.hy.side_scroll = scroll as u16;

    app.hy.visible = lines.iter().filter_map(|l| line_term(model, l)).collect();
    app.hy.side_items = lines
        .iter()
        .filter_map(|l| match l {
            Line::Proj(pi) => Some(SideItem::Proj(model[*pi].key.clone())),
            Line::Sess(..) => line_term(model, l).map(SideItem::Sess),
            _ => None,
        })
        .collect();
    app.hy.row_y.clear();
    app.hy.proj_keys = model.iter().map(|p| p.key.clone()).collect();

    // Hover and keyboard browsing must never erase the session that's actually open.
    let hover_bg = if look.flat { t.hov } else { blend(pb, t.hov, 0.4) };
    let on = app.motion_on();
    let mut gliding: Option<(u16, Rect)> = None;
    let shade = |app: &App, term: TermId, row: Rect, glide: bool| -> Option<Color> {
        if Some(term) == focus { Some(t.hov) }
        else if focused_side && app.hy.cursor == Some(term) && !glide { Some(if look.flat { t.hov } else { t.card2 }) }
        else { hovered(app, row).then_some(hover_bg) }
    };
    // The lines under a session (what it asks, its subagents) step in as far as it does.
    let mut indents: Vec<u16> = Vec::with_capacity(lines.len());
    for l in &lines {
        let last = indents.last().copied().unwrap_or(0);
        indents.push(match l {
            Line::Sess(.., depth) => u16::from(*depth) * FLEET_INDENT,
            Line::Note(..) | Line::Subagent(..) | Line::Subs(..) => last,
            _ => 0,
        });
    }
    for (i, line) in lines.iter().enumerate().skip(scroll).take(list_h) {
        let y = top + (i - scroll) as u16;
        let indent = indents[i];
        let row = Rect { x: x0 + if look.flat { 1 } else { 2 }, y, width: w.saturating_sub(if look.flat { 2 } else { 4 }), height: 1 };
        match line {
            Line::Proj(pi) => {
                let p = &model[*pi];
                let open = !app.hy.saved.closed.contains(&format!("p:{}", p.key));
                let on = focused_side && app.hy.cursor_proj.as_ref() == Some(&p.key);
                let hov = hovered(app, row) || on;
                let bg = if on { t.card2 } else if hov { hover_bg } else { pb };
                if bg != pb {
                    row_pill(look, buf, row.x, y, row.width, bg, pb);
                }
                let s = Style::default().bg(bg);
                // Its fold arrow, in the project's colour.
                let mut left = vec![seg(if open { "▾ " } else { "▸ " }, s.fg(p.color).add_modifier(Modifier::BOLD)), seg(p.name.clone(), s.fg(t.strong).add_modifier(Modifier::BOLD))];
                if p.fresh {
                    left.push(seg(" ", s));
                    left.extend(chip("NEW", Style::default().bg(t.accent).fg(t.acc_ink).add_modifier(Modifier::BOLD)));
                }
                // Right: ● n needing you, or "no git"; folded, what's inside.
                let needs = p.sessions().filter(|x| x.status == Status::Blocked).count();
                let mut c: Vec<Seg> = Vec::new();
                if needs > 0 {
                    c.push(seg(format!("● {needs}"), s.fg(t.blocked).add_modifier(Modifier::BOLD)));
                } else if !p.git && p.kind != Kind::Ssh {
                    c.push(seg("no git", s.fg(t.muted)));
                }
                if !open {
                    let n = p.sessions().count();
                    let what = if n == 0 { "empty".to_string() } else { format!("▸ {n}") };
                    c.push(seg(format!("{}{what}", if c.is_empty() { "" } else { "  " }), s.fg(t.muted)));
                }
                let cw = segs_width(&c);
                put(buf, x0 + 3, y, &left, right.saturating_sub(cw + 1));
                hit(app, row, HyHit::ToggleProj(*pi));
                if hov && !small {
                    row_menu_button(app, buf, Rect { x: right.saturating_sub(1), y, width: 1, height: 1 }, bg, t, HyHit::RowMenuProj(*pi));
                    row_glyph(app, buf, Rect { x: right.saturating_sub(3), y, width: 1, height: 1 }, "+", bg, t.accent, t, HyHit::ShellIn(*pi));
                } else {
                    put(buf, right.saturating_sub(cw), y, &c, right);
                }
            }
            Line::Sess(pi, wi, si, _) => {
                let s = &model[*pi].wts[*wi].sessions[*si];
                app.hy.row_y.insert(s.term, y);
                let active = Some(s.term) == focus;
                let keyboard = focused_side && app.hy.cursor == Some(s.term);
                let glide = if keyboard && !active { app.motion.glide("side", u64::from(s.term), y, on) } else { None };
                if let Some(gy) = glide {
                    gliding = Some((gy, row));
                }
                let lit = shade(app, s.term, row, glide.is_some());
                let bg = lit.unwrap_or(pb);
                if lit.is_some() {
                    row_pill(look, buf, row.x, y, row.width, bg, pb);
                }
                if look.flat && lit.is_some() {
                    let bar = if s.status == Status::Blocked { t.blocked } else if focused_side && app.hy.cursor == Some(s.term) { t.accent } else { t.muted };
                    if let Some(cell) = buf.cell_mut((row.x, y)) {
                        cell.set_symbol("▎").set_style(Style::default().fg(bar).bg(bg));
                    }
                }
                if active || keyboard {
                    put(buf, x0 + 3 + indent, y, &[seg(if active { "▸" } else { "›" }, Style::default().bg(bg).fg(t.strong))], x0 + 5 + indent);
                }
                let sel = focused_side && app.hy.cursor == Some(s.term);
                let st = Style::default().bg(bg);
                // A state's colour on this row: the theme's own, kept readable when the
                // row is lit.
                let state = |c: Color| if lit.is_some() { t.on_lit(c) } else { c };
                let (gl, gc) = if s.is_agent || s.status != Status::None {
                    (glyph(app, s.status), state(state_color(t, s.status)))
                } else {
                    ("›".to_string(), t.muted)
                };
                let mut gs = st.fg(gc);
                if s.status == Status::Blocked {
                    gs = gs.add_modifier(Modifier::BOLD);
                }
                let open_row = Some(s.term) == focus;
                // Finished and not looked at yet: green, like its dot.
                // An agent's name wears its state, as its glyph does: amber shimmering while it
                // works, red when it needs you, green until you look at what it finished, dim
                // asleep. The lit row says its state too: only a row with no state to show
                // (idle, asleep) goes bright there. Every colour is the theme's own.
                let unseen = s.is_agent && s.status == Status::Done && !s.asleep;
                let needs = s.is_agent && s.status == Status::Blocked && !s.asleep;
                let working = s.is_agent && s.status == Status::Working && !s.asleep;
                let name_c = if needs {
                    state(t.blocked)
                } else if unseen {
                    state(t.done)
                } else if sel {
                    t.strong
                } else if s.asleep {
                    t.muted
                } else if open_row {
                    t.strong
                } else {
                    t.text
                };
                let mut ns = st.fg(name_c);
                if open_row || sel || unseen || needs {
                    ns = ns.add_modifier(Modifier::BOLD);
                }
                let mut left = vec![];
                if working {
                    let working = state(t.working);
                    left.extend(shimmer(&s.name, app.spinner_frame(), working, blend(working, t.strong, 0.6), ns));
                } else {
                    left.push(seg(s.name.clone(), ns));
                }
                // Right: agent · age (amber when it needs you); a limit, asleep.
                let tail: Vec<Seg> = if small {
                    vec![]
                } else if s.asleep {
                    vec![seg("asleep", st.fg(t.muted))]
                } else if let Some(at) = s.resume_at {
                    vec![seg(format!("⏸ resumes in {}", until(at)), st.fg(t.working))]
                } else if s.is_agent {
                    let col = if s.status == Status::Blocked { t.blocked } else { t.muted };
                    let first = if s.agent.is_empty() || s.agent == s.name { String::new() } else { format!("{} · ", truncate(&s.agent, 16)) };
                    vec![seg(format!("{first}{}", age(s.since)), st.fg(col))]
                } else {
                    vec![]
                };
                // The name comes first: when it doesn't fit, the agent gives way (the age stays).
                let name_x = x0 + 7 + indent + u16::from(s.asleep);
                let room = right.saturating_sub(name_x) as usize;
                let tail = if s.is_agent && !sel && !s.asleep && s.resume_at.is_none() && !small && segs_width(&left) as usize + segs_width(&tail) as usize + 2 > room {
                    let col = if s.status == Status::Blocked { t.blocked } else { t.muted };
                    vec![seg(age(s.since), st.fg(col))]
                } else {
                    tail
                };
                let tw = segs_width(&tail);
                // Pointed at: its ⋯ takes the row's last column, the right side steps in.
                let menu = hovered(app, row) && !sel;
                let end = if menu { right.saturating_sub(2) } else { right };
                if s.asleep {
                    // Snoring, the way a comic draws it: z's growing as they rise. They hang
                    // left of the icon column so the name stays in line with the others.
                    let zs: Vec<Seg> = SLEEP_ZS.iter().map(|(z, f)| seg(*z, st.fg(blend(bg, t.text, *f)).add_modifier(Modifier::BOLD))).collect();
                    put(buf, x0 + 4 + indent, y, &zs, name_x);
                } else {
                    put(buf, x0 + 5 + indent, y, &[seg(gl, gs)], name_x);
                }
                put(buf, name_x, y, &left, end.saturating_sub(tw + 1));
                put(buf, end.saturating_sub(tw), y, &tail, end);
                hit(app, row, HyHit::Session(s.term));
                hit(app, Rect { x: x0 + 5 + indent, y, width: 2, height: 1 }, HyHit::Why(s.term));
                if menu {
                    row_menu_button(app, buf, Rect { x: right.saturating_sub(1), y, width: 1, height: 1 }, bg, t, HyHit::RowMenuSess(s.term));
                }
            }
            Line::Note(text, c, term) => {
                if small {
                    continue;
                }
                put(buf, x0 + 7 + indent, y, &[seg(truncate(text, w.saturating_sub(11 + indent) as usize), Style::default().bg(pb).fg(*c).add_modifier(Modifier::ITALIC))], right);
                hit(app, row, HyHit::Session(*term));
            }
            Line::Subagent(name, color, term) => {
                if small {
                    continue;
                }
                let base = Style::default().bg(pb);
                let available = w.saturating_sub(11 + indent) as usize;
                let label = truncate(name, available.saturating_sub(6));
                put(
                    buf,
                    x0 + 7 + indent,
                    y,
                    &[
                        seg("  ↳ ", base.fg(t.muted)),
                        seg("● ", base.fg(t.working).add_modifier(Modifier::BOLD)),
                        seg(label, base.fg(*color).add_modifier(Modifier::BOLD)),
                    ],
                    right,
                );
                hit(app, row, HyHit::Session(*term));
            }
            // Nothing running: one click starts a shell there.
            Line::Empty(pi) => {
                let bg = if hovered(app, row) { t.hov } else { pb };
                if bg != pb {
                    row_pill(look, buf, row.x, y, row.width, bg, pb);
                }
                let st = Style::default().bg(bg);
                let nk = k(app, &Action::ShellHere);
                put(
                    buf,
                    x0 + 5,
                    y,
                    &[seg("empty  ", st.fg(t.muted).add_modifier(Modifier::ITALIC)), seg(nk, st.fg(t.accent).add_modifier(Modifier::BOLD)), seg(" new pane", st.fg(t.muted))],
                    right,
                );
                hit(app, row, HyHit::ShellIn(*pi));
            }
            // A session's subagents: the heading folds and unfolds them.
            Line::Subs(term, n, open) => {
                if small {
                    continue;
                }
                let st = Style::default().bg(pb).fg(if hovered(app, row) { t.text } else { t.muted });
                let label = format!("{} {n} subagent{}", if *open { "▾" } else { "▸" }, if *n == 1 { "" } else { "s" });
                put(buf, x0 + 7 + indent, y, &[seg(label, st)], right);
                hit(app, row, HyHit::ToggleSubs(*term));
            }
            // The settled group: one quiet line that unfolds.
            Line::Aside(pi, n, open) => {
                let bg = if hovered(app, row) { t.hov } else { pb };
                if bg != pb {
                    row_pill(look, buf, row.x, y, row.width, bg, pb);
                }
                let st = Style::default().bg(bg).fg(t.muted);
                put(buf, x0 + 5, y, &[seg(if *open { "▾ " } else { "▸ " }, st), seg(format!("settled · {n}"), st.add_modifier(Modifier::ITALIC))], right);
                hit(app, row, HyHit::ToggleAside(*pi));
            }
            // A quiet heading: "AGENTS 2 ────".
            Line::Section(kind, n) => {
                let st = Style::default().bg(pb);
                let label = format!("{} {n} ", kind.heading().to_uppercase());
                let rest = right.saturating_sub(x0 + 3 + label.width() as u16);
                put(buf, x0 + 3, y, &[seg(label, st.fg(t.muted).add_modifier(Modifier::BOLD)), seg("─".repeat(rest as usize), st.fg(t.line))], right);
            }
            Line::Gap => {}
        }
    }
    let plain = Style::default().bg(pb);
    if scroll > 0 {
        put(buf, r.right() - 2, top, &[seg("▲", plain.fg(t.muted))], r.right() - 1);
    }
    if scroll + list_h < lines.len() {
        put(buf, r.right() - 2, top + list_h as u16 - 1, &[seg("▼", plain.fg(t.muted))], r.right() - 1);
    }
    if let Some((gy, row)) = gliding.filter(|(gy, _)| (top..top + list_h as u16).contains(gy))
        && app.hy.row_y.get(&focus.unwrap_or_default()) != Some(&gy)
    {
        tint_row(look, buf, row.x, gy, row.width, t.card2, pb);
    }
}

/// The sleeping agent whose last screen is shown in place of the panes: the one the
/// sidebar's highlight is on, while the sidebar has the keys.
pub(in crate::client) fn peeked(app: &App, focus: TermId) -> Option<TermId> {
    let on = app.hy.cursor.filter(|c| *c != focus)?;
    (app.mode == Mode::Side && app.snap.terms.get(&on).is_some_and(|t| t.asleep)).then_some(on)
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
    app.hy.tick += 1;
    let (tick, cur) = (app.hy.tick, app.hy.tab);
    if let Some(tab) = app.hy.tabs.get_mut(cur) {
        tab.used = tick;
    }
    let look = Look::of(&app.cfg.ui);
    let skip = 1;
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
    app.hy.dividers.clear();
    // A peek: with the keys in the sidebar and the highlight on a sleeping agent, its last
    // screen shows here. Looking doesn't wake it; opening it (Enter, a click) does.
    if let Some(peek) = peeked(app, focus) {
        app.hy.leaf_rects = vec![(peek, area)];
        draw_session(app, f, area, peek, false, model, t);
        // It isn't in this place to stay: its size is left as it was.
        app.panes.retain(|(term, _)| *term != peek);
        return;
    }
    let layout = app
        .hy
        .tabs
        .get(app.hy.tab)
        .map(|tab| tab.layout.clone())
        .filter(|l| l.contains(focus) && l.leaves().len() > 1)
        .filter(|_| flat_splits_allowed);
    let Some(layout) = layout else {
        app.hy.leaf_rects = vec![(focus, area)];
        draw_session(app, f, area, focus, true, model, t);
        return;
    };
    let leaves = layout.leaves();
    // Two: where you drag the gap (below). More: tiled.
    if leaves.len() >= 3 {
        let rects = arranged(&leaves, area);
        app.hy.leaf_rects = rects.clone();
        for (id, r) in rects {
            draw_session(app, f, inset(r), id, id == focus, model, t);
        }
        return;
    }
    // A pane that has just opened grows out of its edge (the split's share eases to where it
    // goes); one too thin to draw yet is only space.
    let mut layout = layout;
    let ratio = layout.ratio_at(&[]).unwrap_or(0.5);
    let ids: Vec<u64> = leaves.iter().map(|l| u64::from(*l)).collect();
    let on = app.motion_on();
    let shown = app.motion.split(app.hy.tab as u64, &ids, ratio, on);
    if (shown - ratio).abs() > f32::EPSILON {
        layout.set_ratio(&[], shown);
    }
    let rects = layout.rects(area);
    app.hy.leaf_rects = rects.clone();
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
        if app.hy.drag == Some(Drag::Divider(i)) || hovered(app, grab) {
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
        hit(app, grab, HyHit::Divider(i));
        app.hy.dividers.push((sa, horizontal, path));
    }
}

fn draw_status_line(app: &mut App, buf: &mut Buffer, area: Rect, model: &[Proj], t: &Theme) {
    if area.height == 0 || area.width < 20 { return; }
    let y = area.bottom() - 1;
    fill(buf, Rect { x: area.x, y, width: area.width, height: 1 }, t.card);
    let ink = Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD);
    let plain = Style::default().fg(t.text).bg(t.card);
    let dim = Style::default().fg(t.muted).bg(t.card);
    let mut x = put(buf, area.x, y, &[seg(if app.mode == Mode::Side { " SIDEBAR " } else { " PANE " }, ink)], area.right());
    x = put(buf, x, y, &[seg("  ", plain)], area.right());
    if let Some(id) = app.focused() {
        if let Some(info) = app.snap.terms.get(&id) {
            let mut left = vec![seg(tilde(&info.cwd), plain)];
            let branch = find(model, id).and_then(|(p, w, _)| (p.git && !w.branch.is_empty()).then(|| w.branch.clone())).or_else(|| info.branch.clone());
            if let Some(branch) = branch {
                left.push(seg("   ", dim));
                left.push(seg(format!("⎇ {branch}"), Style::default().fg(t.done).bg(t.card)));
            }
            x = put(buf, x, y, &left, area.right());
        }
    } else {
        x = put(buf, x, y, &[seg("~", plain)], area.right());
    }
    let mut right: Vec<(String, HyHit, Color, bool)> = Vec::new();
    let needs = app.snap.terms.values().filter(|term| term.status == Status::Blocked).count();
    if needs > 0 { right.push((format!("● {needs} needs you"), HyHit::StatusInbox, t.blocked, false)); }
    right.push((format!("{} inbox", k(app, &Action::Jump)), HyHit::StatusInbox, t.text, false));
    // The accent dot marks where an update is waiting.
    let update = app.update_available.is_some() || app.restart_pending();
    right.push((format!("{} actions", k(app, &Action::Actions)), HyHit::Actions, t.text, update));
    right.push((format!("{} settings", k(app, &Action::Settings)), HyHit::StatusSettings, t.text, false));
    right.push((format!("{} keys", k(app, &Action::Help)), HyHit::StatusKeys, t.text, false));
    let width_of = |label: &str, dot: bool| label.width() as u16 + if dot { 2 } else { 0 };
    let total: u16 = right.iter().map(|(s, _, _, dot)| width_of(s, *dot) + 3).sum();
    let mut rx = area.right().saturating_sub(total);
    if rx < x.saturating_add(1) { rx = x.min(area.right()); }
    for (label, target, colour, dot) in right {
        let width = width_of(&label, dot);
        if rx.saturating_add(width + 2) > area.right() { break; }
        let end = put(buf, rx + 1, y, &[seg(label, Style::default().fg(colour).bg(t.card).add_modifier(Modifier::BOLD))], area.right());
        if dot {
            put(buf, end, y, &[seg(" ●", Style::default().fg(t.accent).bg(t.card).add_modifier(Modifier::BOLD))], area.right());
        }
        hit(app, Rect { x: rx, y, width: width + 2, height: 1 }, target);
        rx = rx.saturating_add(width + 3);
    }
}

/// The ⋯ that opens a sidebar row's menu (for terminals that keep right-clicks).
pub(in crate::client) fn row_menu_button(app: &mut App, buf: &mut Buffer, r: Rect, bg: Color, t: &Theme, h: HyHit) {
    row_glyph(app, buf, r, "⋯", bg, t.muted, t, h);
}

/// A sleeping row's three z's and how strongly each shows: small and faint to big and clear.
const SLEEP_ZS: [(&str, f32); 3] = [("z", 0.5), ("z", 0.75), ("Z", 1.0)];

/// A small glyph button on a sidebar row (⋯, +): just the glyph on the row's own ground,
/// brightening when it's the one under the mouse.
#[allow(clippy::too_many_arguments)]
fn row_glyph(app: &mut App, buf: &mut Buffer, r: Rect, glyph: &str, bg: Color, fg: Color, t: &Theme, h: HyHit) {
    let mut st = Style::default().bg(bg).fg(if hovered(app, r) { t.strong } else { fg });
    if hovered(app, r) {
        st = st.add_modifier(Modifier::BOLD);
    }
    put(buf, r.x, r.y, &[seg(glyph, st)], r.right());
    hit(app, r, h);
}

/// The most urgent state among the panes of a tab, for its pill.
fn tab_state(app: &App, tab: &HyTab) -> Option<Status> {
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
pub(in crate::client) fn tab_name(tab: &HyTab) -> Option<String> {
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
    } else if look.flat {
        Vec::new()
    } else {
        let label = if app.update_available.is_some() || app.restart_pending() { " Actions ● " } else { " Actions " };
        pill(look, vec![seg(label, Style::default().fg(t.accent).add_modifier(Modifier::BOLD))], t.card, t.bg)
    };
    let rw = segs_width(&right);
    let rx = r.right().saturating_sub(rw);
    if rw > 0 {
        put(buf, rx, r.y, &right, r.right());
        hit(app, Rect { x: rx, y: r.y, width: rw, height: 1 }, if armed(app) { HyHit::Leader } else { HyHit::Actions });
    }
    let mut x = r.x;
    let tabs = app.session_tabs();
    for (n, i) in tabs.iter().copied().enumerate() {
        let tab = &app.hy.tabs[i];
        let on = i == app.hy.tab;
        let state = tab_state(app, tab);
        let editing = matches!(&app.mode, Mode::RenameTab(nt) if nt.tab == i);
        let name = if editing {
            match &app.mode {
                Mode::RenameTab(nt) => nt.name.clone(),
                _ => String::new(),
            }
        } else if small && !on {
            String::new()
        } else {
            tab_name(tab).unwrap_or_default()
        };
        // Its name if you gave it one (or are typing one), else its number.
        let fg = if on { t.acc_ink } else { t.text };
        let label = if name.is_empty() && !editing { format!("{}", n + 1) } else { truncate(&name, 24) };
        let mut segs = vec![seg(" ", Style::default()), seg(label, Style::default().fg(fg).add_modifier(Modifier::BOLD))];
        if editing {
            segs.push(seg("█", Style::default().fg(fg)));
        }
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
        hit(app, cr, HyHit::TabPick(i));
        if closable {
            // The ✕ sits before the pill's pad and end cap.
            hit(app, Rect { x: x + w - 3, y: r.y, width: 1, height: 1 }, HyHit::TabClose(i));
        }
        x += w + if look.flat { 0 } else { 1 };
    }
    let plus = pill(look, vec![seg(" + ", Style::default().fg(t.accent).add_modifier(Modifier::BOLD))], if hovered(app, Rect { x, y: r.y, width: 5, height: 1 }) { t.hov } else { pb }, t.bg);
    let pw = segs_width(&plus);
    if x + pw <= rx {
        put(buf, x, r.y, &plus, rx);
        hit(app, Rect { x, y: r.y, width: pw, height: 1 }, HyHit::TabNew);
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
    let focused = focused && app.mode != Mode::Side;
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
    c.state_hit = c.state.map(|_| HyHit::Why(term));
    // What it's taking of the machine, beside its state: where you look when the PC is slow.
    c.note = match info.resume_at {
        Some(at) => Some((format!("⏸ limit · continues in {}", until(at)), t.working)),
        None => load_note(info.cpu, info.mem).map(|(load, heavy)| (load, if heavy { t.working } else { t.muted })),
    };
    c.close = Some(HyHit::CloseSplit(term));
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
    let below = 1 + if ask { 2 } else { 0 };
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
        let hot = app.hy.drag == Some(Drag::Scroll(term)) || hovered(app, track);
        for yy in track.top()..track.bottom() {
            let on = yy >= top && yy < top + thumb as u16;
            let (sym, col) = if on { ("┃", if hot { t.accent } else { t.muted }) } else { ("│", t.line) };
            if let Some(px) = f.buffer_mut().cell_mut((track.x, yy)) {
                px.set_symbol(sym).set_style(Style::default().fg(col).bg(terminal_bg));
            }
        }
        if app.hy.drag.is_none() || app.hy.drag == Some(Drag::Scroll(term)) {
            app.hy.bar = Some((term, track, total));
        }
        hit(app, track, HyHit::ScrollBar(term));
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

    // A session left asleep by an older version shows its saved screen until focused to resume.
    if info.asleep {
        dim_all(f.buffer_mut(), inner, t);
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
/// How long a toast stays: an error longer, one about a session (a click goes there) longest.
pub(in crate::client) fn toast_hold(err: bool, about: bool) -> std::time::Duration {
    std::time::Duration::from_millis(if err {
        4500
    } else if about {
        6000
    } else {
        2500
    })
}

/// How far a toast's hint ("click to open") sinks into its fill: readable, second to the news.
const TOAST_HINT_FADE: f32 = 0.3;
/// How far toward white a toast is when it arrives.
const TOAST_FLASH_BRIGHT: f32 = 0.55;

pub(in crate::client) fn draw_toast(app: &mut App, buf: &mut Buffer, panes: Rect, t: &Theme) {
    let Some((msg, at, err)) = app.notice.clone() else { return };
    let about = app.notice_term.filter(|t| app.snap.terms.contains_key(t));
    let hold = toast_hold(err, about.is_some());
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
    let segs = pill(
        Look::of(&app.cfg.ui),
        vec![seg("  ", Style::default()), seg(mark, bold), seg(text, bold), seg(hint, Style::default().fg(blend(ink, fill, TOAST_HINT_FADE))), seg("  ", Style::default())],
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
        hit(app, r, HyHit::Session(term));
    }
}

