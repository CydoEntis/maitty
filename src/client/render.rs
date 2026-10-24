//! Drawing: sidebar, tab bar, panes, status bar and overlays.

use super::copy::Copy;
use super::design::{Seg, keycap, put, seg};
use super::{App, Mode};
use crate::protocol::Status;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The smallest window the app draws into (below it, a note to make it bigger).
const MIN_WIDTH: u16 = 40;
const MIN_HEIGHT: u16 = 10;

pub fn draw(app: &mut App, f: &mut Frame) {
    app.hits.clear();
    app.panes.clear();
    app.pane_frames.clear();
    let area = f.area();
    let t = app.theme.clone();
    set_palette(t.ansi, t.ansi.map(|_| t.card2));
    super::screen::set_round(app.cfg.ui.rounded.caps());
    // The terminal's own background (its window padding) matches ours while we run.
    let terminal_bg = app.cfg.ui.background != "solid";
    if let Color::Rgb(r, g, b) = t.bg
        && !terminal_bg
        && app.osc_bg != Some(t.bg)
        && !cfg!(test)
    {
        use std::io::Write;
        let _ = write!(std::io::stdout(), "]11;#{r:02x}{g:02x}{b:02x}");
        app.osc_bg = Some(t.bg);
    }

    // Smaller than this, nothing fits: say so instead of drawing a broken screen.
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let buf = f.buffer_mut();
        super::design::fill(buf, area, t.bg);
        let msg = "make the window bigger";
        let x = area.x + area.width.saturating_sub(msg.width() as u16) / 2;
        super::design::put(buf, x, area.y + area.height / 2, &[super::design::seg(msg, Style::default().fg(t.muted).bg(t.bg))], area.right());
        return;
    }
    if app.splash {
        super::screen::draw_splash(app, f, area, &t);
        return;
    }
    super::screen::draw(app, f, area, &t);
    draw_overlays(app, f, area, &t);
    // Held under a popup, shortcut mode stays readable above the dimming behind it.
    if super::screen::shortcut_paused(app) {
        let room = super::screen::framed(app, area);
        super::screen::draw_corner_pill(app, f.buffer_mut(), room, &t);
    }
    tint_hovered(app, f.buffer_mut(), &t);
    if terminal_bg {
        use_terminal_background(f.buffer_mut(), t.bg);
    }
}

/// How far the pointer's tint takes a clickable region's ground toward the text.
const HOVER_TINT: f32 = 0.10;

/// Regions that take a click but are not a control to tint: a pane's terminal, the ground
/// inside or around an overlay, the sidebar's or settings' body, and the edges you drag
/// (they light their own line).
fn tints(h: &super::Hit) -> bool {
    use super::screen::ScreenHit as S;
    !matches!(h, super::Hit::Pane(_) | super::Hit::Screen(S::Noop | S::SideFocus | S::SettingsBody | S::Divider(_) | S::SideEdge | S::SheetEdge | S::ScrollBar(_)))
}

/// Tint the topmost clickable region under the pointer, the one a click there would hit, once:
/// every row, chip, button and ✕ shows it the same way, so no widget tints itself.
fn tint_hovered(app: &App, buf: &mut Buffer, t: &crate::theme::Theme) {
    let Some(at) = app.hover else { return };
    let Some((r, h)) = app.hits.iter().rev().find(|(r, _)| r.contains(at)) else { return };
    if !tints(h) || *r == buf.area {
        return;
    }
    tint(buf, *r, t);
}

/// Lay the pointer's tint over `r`, its ends rounded like every other shape when that is on.
pub(super) fn tint(buf: &mut Buffer, r: Rect, t: &crate::theme::Theme) {
    let r = r.intersection(buf.area);
    let lit = |c: Color| crate::theme::mix(if c == Color::Reset { t.bg } else { c }, t.text, HOVER_TINT);
    let behind = super::screen::ends_behind(buf, r);
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            let cell = &mut buf[(x, y)];
            cell.bg = lit(cell.bg);
        }
    }
    super::screen::round_tinted(buf, r, &behind, lit);
}

/// Hand every cell on the theme's ground back to the terminal's own background (SGR 49), so
/// transparency or a background image looks the same under all of the app's chrome. Done last,
/// after everything that blends colours has drawn. Raised tones (sheets, modals, the status
/// line) and the colours programs paint for themselves are not `bg`, so they stay.
fn use_terminal_background(buf: &mut ratatui::buffer::Buffer, bg: Color) {
    for cell in buf.content.iter_mut() {
        if cell.bg == bg {
            cell.bg = Color::Reset;
        }
    }
}

fn draw_overlays(app: &mut App, f: &mut Frame, area: Rect, t: &crate::theme::Theme) {
    let t = t.clone();
    // Copy mode holds the whole history; match by reference and clone only small fields.
    match &app.mode {
        Mode::Prefix { since, acted } => {
            if !*acted && app.keymap_shown(*since) {
                let km = super::screen::KeyMap::new(super::screen::KeyView::Map, None);
                super::screen::draw_keymap(app, f, area, &t, &km);
            }
        }
        Mode::KeyMap(km) => {
            let km = (**km).clone();
            super::screen::draw_keymap(app, f, area, &t, &km);
        }
        Mode::Actions { sel } => {
            let sel = *sel;
            super::screen::draw_actions(app, f, area, &t, sel);
        }
        Mode::Confirm(c) => {
            let c = (**c).clone();
            super::menu::draw_confirm(app, f, area, &t, &c);
        }
        // The Inbox is a sheet beside the panes (drawn with the screen).
        Mode::GoTo { .. } => {}
        // Popovers beside a sidebar row (drawn with the screen).
        Mode::Compose(_) | Mode::Why { .. } | Mode::Helpers { .. } => {}
        Mode::History { sel } => {
            let sel = *sel;
            super::screen::draw_history(app, f, area, &t, sel);
        }
        Mode::Tidy(v) => {
            let v = (**v).clone();
            super::screen::draw_tidy(app, f, area, &t, &v);
        }
        Mode::Branch(v) => {
            let v = (**v).clone();
            super::branch::draw_branches(app, f, area, &t, &v);
        }
        Mode::Find(v) => {
            let v = (**v).clone();
            super::find::draw_find(app, f, area, &t, &v);
        }
        Mode::ScreenMenu(m) => {
            let m = (**m).clone();
            super::menu::draw_menu(app, f, area, &t, &m);
        }
        Mode::Finder(fd) => {
            let fd = (**fd).clone();
            super::screen::draw_finder(app, f, area, &t, &fd);
        }
        Mode::ScreenPane(np) => {
            let np = np.clone();
            super::screen::draw_new_pane(app, f, area, &t, &np);
        }
        Mode::NewSession(ns) => {
            let ns = (**ns).clone();
            super::screen::draw_new_session(app, f, area, &t, &ns);
        }
        Mode::ScreenSettings(_) => super::screen::draw_settings_modal(app, f.buffer_mut(), area, &t),
        Mode::Approve(a) => {
            let a = (**a).clone();
            super::screen::draw_approve(app, f.buffer_mut(), area, &t, &a);
        }
        Mode::ProjectCheck(d) => {
            let d = (**d).clone();
            super::screen::check_settings::draw_project_check(app, f.buffer_mut(), area, &t, &d);
        }
        Mode::Themes { query, sel, saving } => {
            let (query, sel, saving) = (query.clone(), *sel, *saving);
            super::themes::draw_themes(app, f, area, &t, &query, sel, saving);
        }
        Mode::Spinners(view) => {
            let view = (**view).clone();
            super::spinners::draw(app, f, area, &t, &view);
        }
        Mode::Prompt { kind, input } => {
            let (label, input, confirm) = (kind.label().to_string(), input.clone(), kind.is_confirm());
            draw_prompt(app, f, area, &t, &label, &input, confirm)
        }
        Mode::RenameTab(tab) => {
            let name = tab.name.clone();
            draw_prompt(app, f, area, &t, "Rename session (empty: automatic)", &name, false);
        }
        Mode::Worktrees { items, query, sel, .. } => {
            let rows = app.worktree_rows(items.as_deref(), query);
            let (loading, query, sel) = (items.is_none(), query.clone(), *sel);
            draw_worktrees(app, f, area, &t, &rows, &query, sel, loading);
        }
        _ => {}
    }
}

/// Mix `b` into `a` by `t` (0..1). Non-RGB colours can't be mixed and fall back.
pub fn blend(a: Color, b: Color, t: f32) -> Color {
    let rgb = |c: Color| match c {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Black | Color::Reset => Some((12, 12, 16)),
        Color::White => Some((240, 240, 240)),
        _ => None,
    };
    match (rgb(a), rgb(b)) {
        (Some(x), Some(y)) => {
            let m = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * t.clamp(0.0, 1.0)).round() as u8;
            Color::Rgb(m(x.0, y.0), m(x.1, y.1), m(x.2, y.2))
        }
        _ if t >= 0.5 => b,
        _ => a,
    }
}

/// Which working-spinner frame to show: it turns with motion on and rests on the handoff's
/// third frame with motion off.
fn working_step(app: &App, len: usize) -> usize {
    if app.motion_on() { (app.spinner_frame() % len as u64) as usize } else { REST_FRAME.min(len - 1) }
}

/// The frame the working spinner holds without motion (`checks7.js` `glyph`: `G.work[2]`).
const REST_FRAME: usize = 2;

pub(super) fn status_icon(app: &App, s: Status) -> String {
    let i = &app.cfg.icons;
    match s {
        Status::Working if !app.cfg.ui.spinner.is_empty() => {
            let ascii = super::screen::glyphs::chosen(&app.cfg.ui.glyphs, i) == &super::screen::glyphs::ASCII;
            if ascii && app.cfg.ui.spinner == crate::config::BARS_FRAMES {
                crate::config::BARS_ASCII[working_step(app, crate::config::BARS_ASCII.len())].into()
            } else {
                app.cfg.ui.spinner[working_step(app, app.cfg.ui.spinner.len())].clone()
            }
        }
        Status::Working => i.working.clone(),
        Status::Blocked => i.blocked.clone(),
        Status::Done => i.done.clone(),
        Status::Idle => i.idle.clone(),
        Status::None => i.shell.clone(),
    }
}

pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.width() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for c in s.chars() {
        if out.width() + 1 >= max {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}

/// How far toward the sky colour a selection's ground is.
const SELECT_TINT: f32 = 0.38;

/// `y copy · Esc cancel` as keycaps, for the end of a selection.
fn copy_hints(t: &crate::theme::Theme) -> Vec<Seg> {
    let label = |s: &str| seg(s.to_string(), Style::default().fg(t.text).bg(t.card));
    let mut out = keycap(t, "y");
    out.push(label(" copy · "));
    out.extend(keycap(t, "Esc"));
    out.push(label(" cancel"));
    out
}

pub(super) fn render_copy(c: &Copy, area: Rect, buf: &mut Buffer, t: &crate::theme::Theme) {
    let select = Style::default().bg(blend(t.bg, t.sky(), SELECT_TINT)).fg(t.strong);
    let hit = Style::default().bg(t.working).fg(Color::Black);
    for row in 0..area.height {
        let li = c.top + row as usize;
        let Some(line) = c.lines.get(li) else { break };
        let y = area.y + row;
        let matches = c.matches(li);
        let mut x = area.x;
        let mut n = 0;
        for (ci, ch) in line.chars().enumerate() {
            n = ci + 1;
            let w = ch.width().unwrap_or(0) as u16;
            if w == 0 {
                continue;
            }
            if x + w > area.right() {
                break;
            }
            let mut style = Style::default().fg(t.fg);
            if matches.iter().any(|(s, e)| ci >= *s && ci < *e) {
                style = hit;
            }
            if c.selected(li, ci) {
                style = select;
            }
            if (li, ci) == c.cur {
                style = style.add_modifier(Modifier::REVERSED);
            }
            let mut tmp = [0u8; 4];
            if let Some(px) = buf.cell_mut((x, y)) {
                px.set_symbol(ch.encode_utf8(&mut tmp)).set_style(style);
            }
            x += w;
        }
        // Show the cursor (and line selections) past the end of the text.
        if x < area.right() {
            if c.cur.0 == li && c.cur.1 >= n {
                if let Some(px) = buf.cell_mut((x, y)) {
                    px.set_symbol(" ").set_style(Style::default().add_modifier(Modifier::REVERSED));
                }
            } else if c.selected(li, n)
                && let Some(px) = buf.cell_mut((x, y)) {
                    px.set_symbol(" ").set_style(select);
                }
        }
    }
    // A mouse selection is copied when the button is released, so it needs no keys.
    if let Some(&(row, _, end)) = c.selected_spans().last().filter(|_| !c.mouse) {
        // The keys that finish the selection sit just after its end, on the card ground.
        let mut hints = copy_hints(t);
        hints.insert(0, seg("  ", Style::default().bg(t.card)));
        put(buf, area.x + end, area.y + row, &hints, area.right());
    }
    let bar = if let Some(input) = &c.input {
        Some((format!("{}{input}", if c.backward { "?" } else { "/" }), t.accent))
    } else {
        c.message.clone().map(|m| (m, t.blocked))
    };
    if let Some((text, color)) = bar
        && area.height > 0
    {
        let r = Rect { y: area.bottom() - 1, height: 1, ..area };
        Paragraph::new(Span::styled(text, Style::default().fg(t.bg).bg(color)))
            .style(Style::default().bg(color))
            .render(r, buf);
    }
}

/// The theme's terminal colours, if it has its own (set each frame from the theme).
static PALETTE: std::sync::RwLock<Option<[Color; 7]>> = std::sync::RwLock::new(None);

/// What programs' "bright black" backgrounds become: a subtle panel, not a grey slab.
static PANEL: std::sync::RwLock<Option<Color>> = std::sync::RwLock::new(None);

pub(super) fn set_palette(p: Option<[Color; 7]>, panel: Option<Color>) {
    if let Ok(mut w) = PALETTE.write() {
        *w = p;
    }
    if let Ok(mut w) = PANEL.write() {
        *w = panel;
    }
}

/// Map the basic ANSI colours (red … cyan, and gray) to the theme's.
fn themed(c: Color, pal: &Option<[Color; 7]>) -> Color {
    match (c, pal) {
        (Color::Indexed(i @ 1..=6), Some(p)) => p[i as usize - 1],
        (Color::Indexed(i @ 9..=14), Some(p)) => p[i as usize - 9],
        (Color::Indexed(8), Some(p)) => p[6],
        _ => c,
    }
}

pub(super) fn vt_color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Use a terminal app's background for the card only when it fills nearly the whole screen.
pub(super) fn terminal_surface_bg(screen: &vt100::Screen, default_bg: Color) -> Color {
    const MIN_MATCHING_CORNERS: usize = 3;

    let (rows, cols) = screen.size();
    if rows == 0 || cols == 0 {
        return default_bg;
    }
    let palette = PALETTE.read().map(|palette| *palette).unwrap_or(None);
    let panel = PANEL.read().map(|panel| *panel).unwrap_or(None);
    let corners = [
        (0, 0),
        (0, cols - 1),
        (rows - 1, 0),
        (rows - 1, cols - 1),
    ];
    let backgrounds: [Option<Color>; 4] = corners.map(|(row, col)| {
        let cell = screen.cell(row, col)?;
        let mut bg = if cell.inverse() {
            themed(vt_color(cell.fgcolor()), &palette)
        } else {
            themed(vt_color(cell.bgcolor()), &palette)
        };
        if bg == Color::Reset {
            return None;
        }
        if bg == Color::Indexed(8) {
            bg = panel.unwrap_or(bg);
        }
        Some(bg)
    });

    for bg in backgrounds.iter().flatten() {
        if backgrounds.iter().filter(|sample| sample.as_ref() == Some(bg)).count() >= MIN_MATCHING_CORNERS {
            return *bg;
        }
    }
    default_bg
}

/// Draw a terminal screen. Cells with the default background get `default_bg` (the
/// workspace tint); programs' own background colours are left alone.
pub(super) fn render_screen(screen: &vt100::Screen, area: Rect, buf: &mut Buffer, default_bg: Color) {
    let (rows, cols) = screen.size();
    let pal = PALETTE.read().map(|p| *p).unwrap_or(None);
    let panel = PANEL.read().map(|p| *p).unwrap_or(None);
    for row in 0..area.height.min(rows) {
        for col in 0..area.width.min(cols) {
            let Some(cell) = screen.cell(row, col) else { continue };
            if cell.is_wide_continuation() {
                continue;
            }
            let mut fg = themed(vt_color(cell.fgcolor()), &pal);
            let mut bg = match vt_color(cell.bgcolor()) {
                Color::Reset => default_bg,
                Color::Indexed(8) if panel.is_some() => panel.unwrap_or(default_bg),
                c => c,
            };
            if cell.inverse() {
                std::mem::swap(&mut fg, &mut bg);
                if fg == Color::Reset {
                    fg = Color::Black;
                }
                if bg == Color::Reset {
                    bg = Color::White;
                }
            }
            let mut style = Style::default().fg(fg).bg(bg);
            if cell.bold() {
                style = style.add_modifier(Modifier::BOLD);
            }
            if cell.dim() {
                style = style.add_modifier(Modifier::DIM);
            }
            if cell.italic() {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if cell.underline() {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            let contents = cell.contents();
            let c = &mut buf[(area.x + col, area.y + row)];
            c.set_symbol(if contents.is_empty() { " " } else { contents });
            c.set_style(style);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_worktrees(
    app: &mut App,
    f: &mut Frame,
    area: Rect,
    t: &crate::theme::Theme,
    rows: &[super::WtRow],
    query: &str,
    sel: usize,
    loading: bool,
) {
    use super::design::{put, seg};
    let buf = f.buffer_mut();
    let (r, list, start) = super::screen::query_list(app, buf, area, t, "Worktrees", 84, rows.len().max(12), query, "pick one, or type a new branch", sel);
    let c = Style::default().bg(t.card);
    if loading {
        put(buf, list.x + 2, list.y, &[seg("reading worktrees…", c.fg(t.muted).add_modifier(Modifier::ITALIC))], list.right());
    } else if rows.is_empty() {
        put(buf, list.x + 2, list.y, &[seg("type a branch name to make a worktree", c.fg(t.muted).add_modifier(Modifier::ITALIC))], list.right());
    }
    for (i, row) in rows.iter().enumerate().skip(start).take(list.height as usize) {
        let y = list.y + (i - start) as u16;
        let rr = Rect { y, height: 1, ..list };
        let on = i == sel;
        let st = super::screen::list_row(app, f.buffer_mut(), rr, on, t);
        let line = match row {
            super::WtRow::Existing(w) => {
                let open = app.snap.workspaces.iter().any(|x| super::same_dir(&x.cwd, &w.path));
                let mut l = vec![
                    seg(if open { "● " } else { "○ " }, st.fg(if open { t.accent } else { t.muted })),
                    seg(truncate(&w.branch, 28), st.fg(if on { t.strong } else { t.text }).add_modifier(Modifier::BOLD)),
                ];
                if w.main {
                    l.push(seg("  main", st.fg(t.accent)));
                }
                l.push(seg(format!("   {}", truncate(&w.path.display().to_string(), rr.width.saturating_sub(42) as usize)), st.fg(t.muted)));
                l
            }
            super::WtRow::Create(text) => vec![seg("+ ", st.fg(t.done)), seg("new worktree ", st.fg(t.text)), seg(text.clone(), st.fg(t.done).add_modifier(Modifier::BOLD))],
        };
        put(f.buffer_mut(), rr.x + 3, y, &line, rr.right().saturating_sub(1));
    }
    super::screen::panel_bar(f.buffer_mut(), r, t, &[("↑↓", "move"), ("Enter", "open"), ("Esc", "close")]);
}

fn draw_prompt(app: &mut App, f: &mut Frame, area: Rect, t: &crate::theme::Theme, label: &str, input: &str, confirm: bool) {
    use super::design::{put, seg};
    let buf = f.buffer_mut();
    super::screen::dim_all(buf, area, t);
    let title = label.trim().trim_end_matches(':');
    let mut chars = title.chars();
    let title: String = chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default();
    let r = super::screen::panel(app, buf, area, 64, 7, &title, &[], t);
    // A one-line prompt needs no frame: the title bar and the field carry it.
    for y in r.y + 1..r.bottom() {
        for x in [r.x, r.right() - 1] {
            buf[(x, y)].set_symbol(" ").set_style(Style::default().bg(t.card));
        }
    }
    for x in r.x..r.right() {
        buf[(x, r.bottom() - 1)].set_symbol(" ").set_style(Style::default().bg(t.card));
    }
    if confirm {
        super::screen::panel_bar(buf, r, t, &[("y", "yes"), ("n", "no")]);
        return;
    }
    let field = Rect { x: r.x + 2, y: r.y + 2, width: r.width.saturating_sub(4), height: 1 };
    super::screen::strip(buf, field, t.card2);
    let s2 = Style::default().bg(t.card2);
    put(buf, field.x + 1, field.y, &[seg("› ", s2.fg(t.accent).add_modifier(Modifier::BOLD)), seg(input.to_string(), s2.fg(t.strong)), seg("█", s2.fg(t.accent))], field.right());
    super::screen::panel_bar(buf, r, t, &[("Enter", "save"), ("Esc", "cancel")]);
}
