//! The workspace: one pane, or up to four side by side and stacked, each in a box of its own a
//! column or row apart. A pane's title chip sits on its top border at the left (`mark name
//! ~/folder branch`) and its `files`, `changes` and `×` chips at the right. Exactly one box, the
//! focused pane's, is in the accent. The program's own output is drawn as it is: an agent's pane
//! is its own terminal.

use super::chips::{Chip, chip};
use super::side::display_name;
use super::status_mark::State;
use super::*;
use crate::layout::{Dir, Node};

/// Most panes one view holds.
pub(in crate::client) const MAX_PANES: usize = 4;
/// Narrower than this a box keeps its title and `×` and leaves out `files` and `changes`.
const TOOL_CHIPS_FROM: u16 = 40;

// ---- the workspace -------------------------------------------------------------------------

pub(in crate::client) fn draw_main(
    app: &mut App,
    f: &mut Frame,
    area: Rect,
    model: &[Proj],
    t: &Theme,
) {
    let Some(focus) = app.focused() else {
        super::draw::draw_no_sessions(app, f, area, t);
        return;
    };
    // Which view was on screen last, per session.
    app.screen.tick += 1;
    let (tick, cur) = (app.screen.tick, app.screen.tab);
    if let Some(tab) = app.screen.tabs.get_mut(cur) {
        tab.used = tick;
    }
    app.screen.dividers.clear();
    let layout = app
        .screen
        .tabs
        .get(app.screen.tab)
        .map(|tab| tab.layout.clone())
        .filter(|l| {
            l.contains(focus.term)
                && l.leaves().len() > 1
                && app.screen.zoom != Some(app.screen.tab)
        });
    let lit = !side_focused(app) && !sheet_has_keys(app);
    let Some(mut layout) = layout else {
        app.screen.leaf_rects = vec![(focus, area)];
        draw_pane(app, f, area, focus, (lit, false), model, t);
        return;
    };
    // A pane that has just opened grows out of its edge (the split's share eases to where it
    // goes); one too thin to draw yet is only space.
    let leaves = layout.leaves();
    let ratio = layout.ratio_at(&[]).unwrap_or(0.5);
    let ids: Vec<u64> = leaves.iter().map(|l| u64::from(*l)).collect();
    let on = app.motion_on();
    let shown = app.motion.split(app.screen.tab as u64, &ids, ratio, on);
    if (shown - ratio).abs() > f32::EPSILON {
        layout.set_ratio(&[], shown);
    }
    let rects = layout.rects(area);
    app.screen.leaf_rects = rects
        .iter()
        .map(|(id, r)| (ClientTerm::on(focus.machine, *id), *r))
        .collect();
    for (id, r) in &rects {
        let id = ClientTerm::on(focus.machine, *id);
        let r = apart(*r, area);
        if r.width >= PANE_DRAWN_FROM && r.height >= 4 {
            draw_pane(app, f, r, id, (lit && id == focus, false), model, t);
        }
    }
    for (i, (sa, horizontal, path)) in layout.splits(area).into_iter().enumerate() {
        let ratio = layout.ratio_at(&path).unwrap_or(0.5);
        let div = Node::divider(sa, horizontal, ratio);
        // The gap between two boxes: drag it to resize them.
        let grab = if horizontal {
            Rect {
                x: div.x.saturating_sub(1),
                width: 3,
                ..div
            }
        } else {
            div
        };
        if app.screen.drag == Some(Drag::Divider(i)) || hovered(app, grab) {
            let cells: Vec<(u16, u16, &str)> = if horizontal {
                (sa.y + 1..sa.bottom().saturating_sub(1)).map(|y| (div.x, y, "│")).collect()
            } else {
                (sa.x + 1..sa.right().saturating_sub(1)).map(|x| (x, div.y, "─")).collect()
            };
            let buf = f.buffer_mut();
            for (x, y, glyph) in cells {
                buf[(x, y)]
                    .set_symbol(glyph)
                    .set_style(Style::default().fg(t.accent).bg(t.bg));
            }
        }
        hit(app, grab, ScreenHit::Divider(i));
        app.screen.dividers.push((sa, horizontal, path));
    }
    draw_pane_drag(app, f.buffer_mut(), model, t);
}

/// The box of the focused pane in the pane column `area`: its share of a split, or the whole
/// column. The side panel slides in inside it.
pub(in crate::client) fn focused_box(app: &App, area: Rect) -> Rect {
    let Some(focus) = app.focused() else {
        return area;
    };
    app.screen
        .tabs
        .get(app.screen.tab)
        .filter(|tab| {
            tab.layout.contains(focus.term)
                && tab.layout.leaves().len() > 1
                && app.screen.zoom != Some(app.screen.tab)
        })
        .and_then(|tab| {
            tab.layout
                .rects(area)
                .into_iter()
                .find(|(id, _)| *id == focus.term)
                .map(|(_, r)| apart(r, area))
        })
        .unwrap_or(area)
}

/// A pane's share of a view, less the column or row that keeps it apart from the next box.
fn apart(r: Rect, area: Rect) -> Rect {
    Rect {
        width: r.width - u16::from(r.right() < area.right()),
        height: r.height - u16::from(r.bottom() < area.bottom()),
        ..r
    }
}

/// A pane's name for its title: yours, else what runs in it.
pub(super) fn pane_title(app: &App, model: &[Proj], term: ClientTerm) -> String {
    let fallback = find(model, term).map_or_else(String::new, |(_, _, s)| s.name.clone());
    display_name(app.term_info(term), &fallback)
}

/// The state glyph of a pane, from the daemon's report.
pub(super) fn pane_state(app: &App, info: &TermInfo, term: ClientTerm) -> State {
    let failed = super::status_mark::failed(info.exit.is_some(), info.status, info.check());
    State::of(info.status, failed, app.machine_down(term.machine))
}

/// The border of a pane's box: red while a close asks about it, the accent (or the setting's
/// colour) when it has the keys, sky in shortcut mode, else the quiet line; just landed on, it
/// flares and settles.
fn border_of(app: &mut App, term: ClientTerm, focused: bool, t: &Theme) -> Color {
    let border = if closing(app, term) {
        t.err
    } else if !focused {
        return t.line;
    } else if armed(app) {
        t.sky()
    } else {
        match app.cfg.ui.focus_border.as_str() {
            "bright" => t.strong,
            "none" => t.line,
            _ => t.accent,
        }
    };
    let on = app.motion_on();
    let flash = if focused {
        app.motion.land(term.motion_key(), on)
    } else {
        0.0
    };
    let flare = if app.cfg.ui.focus_border == "bright" {
        t.accent
    } else {
        t.strong
    };
    if flash > 0.0 {
        blend(border, flare, flash)
    } else {
        border
    }
}

/// One pane: its box, the title chip and the chips at the right on the top border, then the
/// program inside. A popup (`<app> popup`) has only the `×` at the right.
pub(in crate::client) fn draw_pane(
    app: &mut App,
    f: &mut Frame,
    r: Rect,
    term: ClientTerm,
    (focused, popup): (bool, bool),
    model: &[Proj],
    t: &Theme,
) {
    let Some(info) = app.term_info(term).cloned() else {
        return;
    };
    let ask = info.status == Status::Blocked
        && info.agent.is_some()
        && app.pending_question(term).is_some();
    let border = border_of(app, term, focused, t);
    let mut c = Card::new(t, "");
    if let Some(parser) = app.parsers.get(&term) {
        c.bg = crate::client::render::terminal_surface_bg(parser.screen(), c.bg);
    }
    let terminal_bg = c.bg;
    if border != t.line {
        c = c.lit(border);
    }
    let inside = card(app, f.buffer_mut(), r, &c, t);
    if r.width >= 12 {
        let chips_x = draw_right_chips(app, f.buffer_mut(), r, term, popup, t);
        let lit = focused && border != t.line;
        let title = title_segs(app, model, term, &info, lit, t);
        let start = r.x + 2;
        let room = chips_x.saturating_sub(start + 1);
        put(f.buffer_mut(), start, r.y, &fit(title, room), chips_x.saturating_sub(1));
        app.hits.push((Rect { x: start, y: r.y, width: room, height: 1 }, Hit::Pane(term)));
    }
    let below = if ask { 2 } else { 0 };
    let inner = Rect {
        x: r.x + 2,
        y: r.y + 1,
        width: r.width.saturating_sub(5),
        height: r.height.saturating_sub(2 + below),
    };
    render_pane(
        app,
        f,
        (r, inside, inner),
        term,
        focused,
        ask,
        terminal_bg,
        t,
    );
}

/// The title chip: the status mark, the name in bold, then the folder and the branch quieter.
/// Lit (the accent) while the pane has the keys.
fn title_segs(app: &App, model: &[Proj], term: ClientTerm, info: &TermInfo, lit: bool, t: &Theme) -> Vec<Seg> {
    let (ground, ink, quiet) = if lit {
        (t.accent, t.acc_ink, t.acc_ink)
    } else {
        (t.btn, t.strong, t.muted)
    };
    let base = Style::default().bg(ground);
    let mark = app.state_mark(pane_state(app, info, term), t);
    let mut where_ = tilde(&info.cwd);
    if let Some(b) = info.branch.as_deref().filter(|b| !b.is_empty()) {
        where_.push(' ');
        where_.push_str(b);
    }
    vec![
        seg(" ", base),
        seg(mark.glyph, base.fg(if lit { t.acc_ink } else { mark.color }).add_modifier(Modifier::BOLD)),
        seg(" ", base),
        seg(pane_title(app, model, term), base.fg(ink).add_modifier(Modifier::BOLD)),
        seg(format!(" {where_} "), base.fg(quiet)),
    ]
}

/// `segs` cut to `room` cells: the last piece that does not fit ends in `…` and a cell of its
/// ground, so the chip still closes.
fn fit(segs: Vec<Seg>, room: u16) -> Vec<Seg> {
    let room = room as usize;
    let mut used = 0;
    let mut out = Vec::new();
    for (text, style) in segs {
        let w = text.width();
        if used + w <= room {
            used += w;
            out.push((text, style));
            continue;
        }
        let left = room.saturating_sub(used);
        if left > 2 {
            out.push((format!("{} ", truncate(&text, left - 1)), style));
        }
        break;
    }
    out
}

/// `files`, `changes` and `×` at the right of the top border, a cell of border between them;
/// returns where the leftmost starts. `files` and `changes` are lit while their panel is open.
fn draw_right_chips(app: &mut App, buf: &mut Buffer, r: Rect, term: ClientTerm, popup: bool, t: &Theme) -> u16 {
    let mut chips: Vec<(Vec<Seg>, ScreenHit)> = Vec::new();
    if !popup && r.width >= TOOL_CHIPS_FROM {
        let files_on = matches!(app.view, Some(crate::client::View::Files(_)));
        let changes_on = matches!(app.view, Some(crate::client::View::Changes(_)));
        chips.push((chip(app, "files", Chip::Title { focused: files_on }, t.bg, t), ScreenHit::FilesChip));
        let changes = match changed_files(app) {
            Some(n) => format!("changes {n}"),
            None => "changes".to_string(),
        };
        chips.push((chip(app, &changes, Chip::Title { focused: changes_on }, t.bg, t), ScreenHit::ChangesChip));
    }
    chips.push((chip(app, "×", Chip::Title { focused: false }, t.bg, t), ScreenHit::CloseSplit(term)));
    let mut x = r.right().saturating_sub(2);
    for (segs, h) in chips.into_iter().rev() {
        let w = segs_width(&segs);
        if x < r.x + w + 4 {
            break;
        }
        x -= w;
        put(buf, x, r.y, &segs, x + w);
        hit(app, Rect { x, y: r.y, width: w, height: 1 }, h);
        x -= 1;
    }
    x + 1
}

/// How many files the open Changes panel lists, once it has read them.
fn changed_files(app: &App) -> Option<usize> {
    match &app.view {
        Some(crate::client::View::Changes(v)) => v.review.as_ref().map(|r| r.files.len()),
        _ => None,
    }
}

/// The program's output in `inner`, its scrollbar and the answer bar; the pane not in focus
/// fades.
#[allow(clippy::too_many_arguments)]
fn render_pane(
    app: &mut App,
    f: &mut Frame,
    (r, inside, inner): (Rect, Rect, Rect),
    pane: ClientTerm,
    focused: bool,
    ask: bool,
    terminal_bg: Color,
    t: &Theme,
) {
    let look = Look::of(&app.cfg.ui);
    let scrolled = app
        .parsers
        .get(&pane)
        .map(|p| p.screen().scrollback())
        .filter(|n| *n > 0);
    app.pane_frames.push((pane, r));
    app.panes.push((pane, inner));
    // The whole inside of the frame focuses the pane.
    app.hits.push((inside, Hit::Pane(pane)));
    app.hits.push((inner, Hit::Pane(pane)));
    let copying = matches!(&app.mode, Mode::Copy(c) if c.term == pane);
    if copying {
        if let Mode::Copy(c) = &mut app.mode {
            c.height = inner.height as usize;
            c.width = inner.width as usize;
            crate::client::render::render_copy(c, inner, f.buffer_mut(), t);
        }
    } else if let Some(p) = app.parsers.get(&pane) {
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
    draw_scrollbar(app, f.buffer_mut(), pane, inner, terminal_bg, t);
    draw_to_bottom(app, f.buffer_mut(), pane, inner, scrolled, terminal_bg, t);
    // Not the one you're in: its inside fades (the border and its chips stay as they are).
    if !focused {
        dim_inside(f.buffer_mut(), inner, look.dim, t);
    }
    if ask && r.height >= 8 {
        draw_answer_bar(app, f.buffer_mut(), r, pane, terminal_bg, t);
    }
}

/// Whether a view with `panes` panes can take another.
pub(in crate::client) fn room_for_pane(panes: usize) -> bool {
    panes < MAX_PANES
}

/// The direction a split command inserts the new pane: up and left before the focused pane,
/// down and right after it.
pub(in crate::client) fn split_dir(action: &Action) -> Option<Dir> {
    match action {
        Action::SplitUp => Some(Dir::Up),
        Action::SplitDown => Some(Dir::Down),
        Action::SplitLeft => Some(Dir::Left),
        Action::SplitRight => Some(Dir::Right),
        _ => None,
    }
}
