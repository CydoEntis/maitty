//! The workspace: one pane in a frame whose top border carries its status, title and chips,
//! or up to four panes in one frame cut by single lines with proper joints, each with a
//! two-row header. Exactly one frame, or one pane's lines, is in the accent. The program's
//! own output is drawn as it is.

use super::chips::{Chip, chip, right_chips};
use super::side::display_name;
use super::status_mark::State;
use super::*;
use crate::layout::{Dir, Node};

/// Most panes one view holds.
pub(in crate::client) const MAX_PANES: usize = 4;
/// A pane at least this wide gets the split, zoom and menu buttons as well as `⋯ ×`.
const WIDE_HEADER: u16 = 60;
/// Rows a pane's header takes.
const HEADER_ROWS: u16 = 2;
/// Cells each header button takes (the glyph and a cell either side).
const BUTTON_W: u16 = 3;

/// A button on a pane's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum PaneBtn {
    SplitRight,
    SplitDown,
    Zoom,
    Menu,
}

impl PaneBtn {
    fn glyph(self) -> &'static str {
        match self {
            PaneBtn::SplitRight => "⊞",
            PaneBtn::SplitDown => "⊟",
            PaneBtn::Zoom => "⤢",
            PaneBtn::Menu => "⋯",
        }
    }
}

// ---- the lines between panes ----------------------------------------------------------------

const UP: u8 = 1;
const DOWN: u8 = 2;
const LEFT: u8 = 4;
const RIGHT: u8 = 8;

/// The cells of the lines a view is cut by, each a set of the four directions a line leaves
/// it in, so the glyph (a corner, a tee or a cross) follows from what meets there.
struct LineGrid {
    area: Rect,
    bits: Vec<u8>,
}

impl LineGrid {
    fn new(area: Rect) -> LineGrid {
        LineGrid {
            area,
            bits: vec![0; area.width as usize * area.height as usize],
        }
    }

    fn at(&mut self, x: u16, y: u16) -> Option<&mut u8> {
        let (dx, dy) = (x.checked_sub(self.area.x)?, y.checked_sub(self.area.y)?);
        if dx >= self.area.width || dy >= self.area.height {
            return None;
        }
        let w = self.area.width as usize;
        self.bits.get_mut(dy as usize * w + dx as usize)
    }

    fn get(&self, x: u16, y: u16) -> u8 {
        let (Some(dx), Some(dy)) = (x.checked_sub(self.area.x), y.checked_sub(self.area.y)) else {
            return 0;
        };
        if dx >= self.area.width || dy >= self.area.height {
            return 0;
        }
        self.bits[dy as usize * self.area.width as usize + dx as usize]
    }

    fn add(&mut self, x: u16, y: u16, b: u8) {
        if let Some(cell) = self.at(x, y) {
            *cell |= b;
        }
    }

    /// The frame round the whole view.
    fn outline(&mut self) {
        let a = self.area;
        if a.width < 2 || a.height < 2 {
            return;
        }
        let (right, bottom) = (a.right() - 1, a.bottom() - 1);
        for x in a.x..=right {
            let sides = (if x > a.x { LEFT } else { 0 }) | (if x < right { RIGHT } else { 0 });
            self.add(x, a.y, sides);
            self.add(x, bottom, sides);
        }
        for y in a.y..=bottom {
            let sides = (if y > a.y { UP } else { 0 }) | (if y < bottom { DOWN } else { 0 });
            self.add(a.x, y, sides);
            self.add(right, y, sides);
        }
    }

    /// The line of a split laid out in `sa`: down the middle of a side-by-side one, across a
    /// stacked one. It runs one cell past `sa`'s start when that cell is another line.
    fn divider(&mut self, sa: Rect, side_by_side: bool, ratio: f32) {
        let div = Node::divider(sa, side_by_side, ratio);
        if side_by_side {
            let from = sa.y.saturating_sub(u16::from(sa.y > self.area.y));
            let to = sa.bottom().saturating_sub(1);
            for y in from..=to {
                let sides = (if y > from { UP } else { 0 }) | (if y < to { DOWN } else { 0 });
                self.add(div.x, y, sides);
            }
        } else {
            let from = sa.x.saturating_sub(u16::from(sa.x > self.area.x));
            let to = sa.right().saturating_sub(1);
            for x in from..=to {
                let sides = (if x > from { LEFT } else { 0 }) | (if x < to { RIGHT } else { 0 });
                self.add(x, div.y, sides);
            }
        }
    }

    /// The glyph for what meets in a cell; the view's four outer corners round with `rounded`.
    fn glyph(&self, x: u16, y: u16, rounded: bool) -> Option<&'static str> {
        let a = self.area;
        let corner = |left: bool, top: bool| {
            (x == if left { a.x } else { a.right() - 1 })
                && (y == if top { a.y } else { a.bottom() - 1 })
        };
        Some(match self.get(x, y) {
            0 => return None,
            b if b == DOWN | RIGHT => {
                if rounded && corner(true, true) {
                    "╭"
                } else {
                    "┌"
                }
            }
            b if b == DOWN | LEFT => {
                if rounded && corner(false, true) {
                    "╮"
                } else {
                    "┐"
                }
            }
            b if b == UP | RIGHT => {
                if rounded && corner(true, false) {
                    "╰"
                } else {
                    "└"
                }
            }
            b if b == UP | LEFT => {
                if rounded && corner(false, false) {
                    "╯"
                } else {
                    "┘"
                }
            }
            b if b == UP | DOWN | RIGHT => "├",
            b if b == UP | DOWN | LEFT => "┤",
            b if b == LEFT | RIGHT | DOWN => "┬",
            b if b == LEFT | RIGHT | UP => "┴",
            b if b == UP | DOWN | LEFT | RIGHT => "┼",
            b if b & (UP | DOWN) != 0 && b & (LEFT | RIGHT) == 0 => "│",
            _ => "─",
        })
    }
}

/// The lines round pane `r` of a view laid out in `area`, as cells: the top and bottom runs
/// and the left and right ones, corners and joints included.
fn perimeter(r: Rect, area: Rect) -> Vec<(u16, u16)> {
    let left = r.x.saturating_sub(u16::from(r.x > area.x));
    let top = r.y.saturating_sub(u16::from(r.y > area.y));
    let (right, bottom) = (r.right().saturating_sub(1), r.bottom().saturating_sub(1));
    let mut cells = Vec::new();
    for x in left..=right {
        cells.push((x, top));
        cells.push((x, bottom));
    }
    for y in top..=bottom {
        cells.push((left, y));
        cells.push((right, y));
    }
    cells
}

/// What a pane has of a view tiled in `area` once the lines are taken out: its place between
/// them, for its header and terminal.
fn region(r: Rect, area: Rect) -> Rect {
    let x = r.x + u16::from(r.x == area.x);
    let y = r.y + u16::from(r.y == area.y);
    Rect {
        x,
        y,
        width: r.right().saturating_sub(1).saturating_sub(x),
        height: r.bottom().saturating_sub(1).saturating_sub(y),
    }
}

// ---- the workspace -------------------------------------------------------------------------

pub(in crate::client) fn draw_main(
    app: &mut App,
    f: &mut Frame,
    area: Rect,
    model: &[Proj],
    t: &Theme,
) {
    let Some(focus) = app.focused() else {
        let nk = k(app, &Action::ShellHere);
        put(
            f.buffer_mut(),
            area.x + 4,
            area.y + 3,
            &[seg(
                format!(
                    "Nothing open. Press {} {nk} for a new shell.",
                    app.keymap.prefix.to_string().replace("C-", "Ctrl+")
                ),
                Style::default().fg(t.muted),
            )],
            area.right(),
        );
        return;
    };
    // Which tab was on screen last, per session.
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
    app.screen.leaf_rects = rects
        .iter()
        .map(|(id, r)| (ClientTerm::on(focus.machine, *id), *r))
        .collect();
    let look = Look::of(&app.cfg.ui);
    let splits = layout.splits(area);

    // The lines, then the one pane (or none) that has the accent.
    let mut grid = LineGrid::new(area);
    grid.outline();
    for (sa, side_by_side, path) in &splits {
        let ratio = layout.ratio_at(path).unwrap_or(0.5);
        grid.divider(*sa, *side_by_side, ratio);
    }
    let lit = !side_focused(app) && !sheet_has_keys(app);
    let accent = if armed(app) { t.sky() } else { t.accent };
    let mut colours: HashMap<(u16, u16), Color> = HashMap::new();
    let colour_of = |id: &ClientTerm| {
        if closing(app, *id) {
            Some(t.err)
        } else if lit && *id == focus {
            Some(accent)
        } else {
            None
        }
    };
    let marked: Vec<(Rect, Color)> = rects
        .iter()
        .filter_map(|(id, r)| colour_of(&ClientTerm::on(focus.machine, *id)).map(|c| (*r, c)))
        .collect();
    for (r, c) in &marked {
        for cell in perimeter(*r, area) {
            colours.insert(cell, *c);
        }
    }
    {
        let buf = f.buffer_mut();
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                if let Some(glyph) = grid.glyph(x, y, look.rounded) {
                    let fg = colours.get(&(x, y)).copied().unwrap_or(t.line);
                    buf[(x, y)]
                        .set_symbol(glyph)
                        .set_style(Style::default().fg(fg).bg(t.bg));
                }
            }
        }
    }

    for (id, r) in &rects {
        let id = ClientTerm::on(focus.machine, *id);
        let inside = region(*r, area);
        if inside.width >= PANE_DRAWN_FROM && inside.height > HEADER_ROWS {
            draw_split_pane(app, f, inside, id, id == focus && lit, model, t);
        }
    }
    for (i, (sa, horizontal, path)) in splits.into_iter().enumerate() {
        let ratio = layout.ratio_at(&path).unwrap_or(0.5);
        let div = Node::divider(sa, horizontal, ratio);
        // A cell either side of a vertical line is easier to hit than the one column.
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
            let buf = f.buffer_mut();
            let cells: Vec<(u16, u16)> = if horizontal {
                (sa.y..sa.bottom()).map(|y| (div.x, y)).collect()
            } else {
                (sa.x..sa.right()).map(|x| (x, div.y)).collect()
            };
            for (x, y) in cells {
                if grid.get(x, y) != 0 {
                    buf[(x, y)].set_fg(t.accent);
                }
            }
        }
        hit(app, grab, ScreenHit::Divider(i));
        app.screen.dividers.push((sa, horizontal, path));
    }
    draw_pane_drag(app, f.buffer_mut(), model, t);
}

/// A pane's name for its title: yours, else what runs in it.
fn pane_title(app: &App, model: &[Proj], term: ClientTerm) -> String {
    let fallback = find(model, term).map_or_else(String::new, |(_, _, s)| s.name.clone());
    display_name(app.term_info(term), &fallback)
}

/// The state glyph of a pane, from the daemon's report.
fn pane_state(app: &App, info: &TermInfo, term: ClientTerm) -> State {
    let failed = super::status_mark::failed(info.exit.is_some(), info.status, info.check());
    State::of(info.status, failed, app.machine_down(term.machine))
}

/// Where a pane runs, as its machine chip says it.
fn machine_text(app: &App, machine: MachineId) -> String {
    if machine == MachineId::LOCAL {
        let this = if cfg!(target_os = "macos") {
            "This Mac"
        } else if cfg!(windows) {
            "This PC"
        } else {
            "This computer"
        };
        format!("{} {this}", super::machine_group::LOCAL_ICON)
    } else {
        format!(
            "{} {}",
            super::machine_group::REMOTE_ICON,
            app.machine_label_of(machine)
        )
    }
}

/// A single pane: a frame with its status, title and chips on the top border and the
/// program below. Also a popup's pane.
pub(in crate::client) fn draw_session(
    app: &mut App,
    f: &mut Frame,
    r: Rect,
    term: ClientTerm,
    focused: bool,
    model: &[Proj],
    t: &Theme,
) {
    let Some(info) = app.term_info(term).cloned() else {
        return;
    };
    // While the sidebar has the keys, no pane shows as focused.
    let focused = focused && !side_focused(app) && !sheet_has_keys(app);
    let ask = info.status == Status::Blocked
        && info.agent.is_some()
        && app.pending_question(term).is_some();
    let (inside, terminal_bg) = lone_frame(app, f.buffer_mut(), r, term, &info, focused, model, t);
    let below = 1 + if ask { 2 } else { 0 };
    let inner = Rect {
        x: r.x + 2,
        y: r.y + 1,
        width: r.width.saturating_sub(5),
        height: r.height.saturating_sub(1 + below),
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

/// The frame of a lone pane and the chips on its top border. Returns the inside and the
/// terminal's ground.
#[allow(clippy::too_many_arguments)]
fn lone_frame(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    term: ClientTerm,
    info: &TermInfo,
    focused: bool,
    model: &[Proj],
    t: &Theme,
) -> (Rect, Color) {
    let closing = closing(app, term);
    let border = if closing {
        t.err
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
    let border = if flash > 0.0 {
        blend(border, flare, flash)
    } else {
        border
    };
    let mut c = Card::new(t, "");
    if let Some(parser) = app.parsers.get(&term) {
        c.bg = crate::client::render::terminal_surface_bg(parser.screen(), c.bg);
    }
    let terminal_bg = c.bg;
    if focused || closing {
        c = c.lit(border);
    }
    let inside = card(app, buf, r, &c, t);
    if r.width < 12 {
        return (inside, terminal_bg);
    }

    // On the border at the left: the status glyph, then the title chip.
    let mark = app.state_mark(pane_state(app, info, term), t);
    let glyph_x = r.x + 2;
    put(
        buf,
        glyph_x - 1,
        r.y,
        &[
            seg(" ", Style::default().bg(t.bg)),
            seg(mark.glyph, Style::default().fg(mark.color).bg(t.bg)),
        ],
        glyph_x + 1,
    );
    let right_edge = r.right().saturating_sub(1);
    let title_x = glyph_x + 1;
    // What the right of the border offers, most wanted last: the path chips go first.
    let mut chips: Vec<(&str, Vec<Seg>, Option<ScreenHit>)> = vec![(
        "project",
        chip(app, "▾ Project", Chip::Action, t.bg, t),
        Some(ScreenHit::ProjectMenu(term)),
    )];
    let folder = tilde(&info.cwd);
    if !folder.is_empty() {
        chips.push((
            "folder",
            chip(app, &format!("▢ {folder}"), Chip::Quiet, t.bg, t),
            None,
        ));
    }
    if let Some(branch) = info.branch.as_deref().filter(|b| !b.is_empty()) {
        chips.push((
            "branch",
            chip(app, &format!("⎇ {branch}"), Chip::Branch, t.bg, t),
            None,
        ));
    }
    chips.push((
        "machine",
        chip(app, &machine_text(app, term.machine), Chip::Quiet, t.bg, t),
        None,
    ));
    let close = chip(app, "×", Chip::Close, t.bg, t);
    let close_w = segs_width(&close) + 1;
    let title_min = 12u16.min(right_edge.saturating_sub(title_x));
    let room = right_edge.saturating_sub(title_x + title_min + 1 + close_w);
    let width_of = |chips: &[(&str, Vec<Seg>, Option<ScreenHit>)]| -> u16 {
        chips.iter().map(|(_, s, _)| segs_width(s) + 1).sum()
    };
    // The path chips drop first, then the branch, the machine and the menu.
    for name in ["folder", "branch", "machine", "project"] {
        if width_of(&chips) <= room {
            break;
        }
        chips.retain(|(n, _, _)| *n != name);
    }
    let mut row: Vec<(Vec<Seg>, Option<ScreenHit>)> =
        chips.into_iter().map(|(_, s, h)| (s, h)).collect();
    row.push((close, Some(ScreenHit::CloseSplit(term))));
    let start = right_chips(app, buf, r.y, (title_x, right_edge), row, border);
    let title_room = start.saturating_sub(title_x + 1) as usize;
    let name = truncate(
        &pane_title(app, model, term),
        title_room.saturating_sub(2).max(1),
    );
    let title = chip(app, &name, Chip::Title { focused }, t.bg, t);
    put(buf, title_x, r.y, &title, start.saturating_sub(1));
    (inside, terminal_bg)
}

/// A pane of a split: its two-row header, and the program below.
fn draw_split_pane(
    app: &mut App,
    f: &mut Frame,
    r: Rect,
    term: ClientTerm,
    focused: bool,
    model: &[Proj],
    t: &Theme,
) {
    let Some(info) = app.term_info(term).cloned() else {
        return;
    };
    let ask = info.status == Status::Blocked
        && info.agent.is_some()
        && app.pending_question(term).is_some();
    let mut c = Card::new(t, "");
    let terminal_bg = app.parsers.get(&term).map_or(c.bg, |p| {
        crate::client::render::terminal_surface_bg(p.screen(), c.bg)
    });
    c.bg = terminal_bg;
    fill(f.buffer_mut(), r, terminal_bg);
    let buf = f.buffer_mut();

    // The buttons are clickable above the pane's own hit, which covers the header too.
    let mut button_hits: Vec<(Rect, ScreenHit)> = Vec::new();
    // Row 1: the title centred, the buttons at the right.
    let mut buttons = vec![PaneBtn::Menu];
    if r.width >= WIDE_HEADER {
        buttons = vec![
            PaneBtn::SplitRight,
            PaneBtn::SplitDown,
            PaneBtn::Zoom,
            PaneBtn::Menu,
        ];
    }
    let bw = (buttons.len() as u16 + 1) * BUTTON_W;
    let mut x = (r.right()).saturating_sub(bw);
    for b in &buttons {
        let cell = Rect {
            x,
            y: r.y,
            width: BUTTON_W,
            height: 1,
        };
        let colour = if hovered(app, cell) {
            t.strong
        } else {
            t.muted
        };
        put(
            buf,
            x + 1,
            r.y,
            &[seg(b.glyph(), Style::default().fg(colour).bg(terminal_bg))],
            x + 2,
        );
        button_hits.push((cell, ScreenHit::PaneBtn(term, *b)));
        x += BUTTON_W;
    }
    let close = Rect {
        x,
        y: r.y,
        width: BUTTON_W,
        height: 1,
    };
    let close_colour = if hovered(app, close) { t.err } else { t.muted };
    put(
        buf,
        x + 1,
        r.y,
        &[seg("×", Style::default().fg(close_colour).bg(terminal_bg))],
        x + 2,
    );
    button_hits.push((close, ScreenHit::CloseSplit(term)));
    let mark = app.state_mark(pane_state(app, &info, term), t);
    let name_room = bw as usize + 2;
    let title_room = (r.width as usize).saturating_sub(name_room * 2).max(4);
    let name = truncate(&pane_title(app, model, term), title_room);
    let name_style = Style::default()
        .fg(if focused { t.strong } else { t.text })
        .bg(terminal_bg)
        .add_modifier(Modifier::BOLD);
    let label = vec![
        seg(
            format!("{} ", mark.glyph),
            Style::default().fg(mark.color).bg(terminal_bg),
        ),
        seg(name, name_style),
    ];
    let lw = segs_width(&label);
    let lx = r.x + r.width.saturating_sub(lw) / 2;
    put(buf, lx, r.y, &label, r.right().saturating_sub(bw));

    // Row 2: the path, and the branch at the right.
    let y2 = r.y + 1;
    let branch = info
        .branch
        .as_deref()
        .filter(|b| !b.is_empty())
        .map(|b| format!("⎇ {b}"));
    let bwid = branch.as_ref().map_or(0, |b| b.width() as u16 + 1);
    let path_room = (r.width.saturating_sub(2 + bwid)) as usize;
    let path = truncate(&tilde(&info.cwd), path_room.max(1));
    put(
        buf,
        r.x + 1,
        y2,
        &[seg(path, Style::default().fg(t.muted).bg(terminal_bg))],
        r.right(),
    );
    if let Some(branch) = branch {
        let bx = r.right().saturating_sub(branch.width() as u16 + 1);
        put(
            buf,
            bx,
            y2,
            &[seg(branch, Style::default().fg(t.violet()).bg(terminal_bg))],
            r.right(),
        );
    }

    let below = if ask { 2 } else { 0 };
    let inner = Rect {
        x: r.x + 1,
        y: r.y + HEADER_ROWS,
        width: r.width.saturating_sub(3),
        height: r.height.saturating_sub(HEADER_ROWS + below),
    };
    // The whole pane focuses it, the header included.
    let outer = r;
    render_pane(
        app,
        f,
        (outer, outer, inner),
        term,
        focused,
        ask,
        terminal_bg,
        t,
    );
    for (rect, h) in button_hits {
        hit(app, rect, h);
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
