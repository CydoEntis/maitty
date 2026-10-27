//! Hairline's sidebar: `sessions` and its count, dim project headers with their branch, one row
//! per session (no second lines) with its tabs under it while it has several, and the new and
//! menu hints pinned to the foot.

use super::super::side_tabs::{draw_tab_line, tab_line, tab_segs};
use super::*;

/// Where a row's pieces go, from the sidebar's left edge (the handoff's columns): the `▎`
/// marker on the edge itself, words, headers and the glyph a column in, the name two after it.
/// A helper's row steps in `HELPER_INDENT` per level under the session that started it.
const TEXT_IN: u16 = 1;
const GLYPH_IN: u16 = 1;
const NAME_IN: u16 = 3;

/// A session row's look, from the handoff's state table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowState {
    /// The open session, or the keyboard's row while the sidebar has the keys.
    Focused,
    /// On screen in a pane, without the keys.
    Visible,
    Plain,
}

/// The sidebar at `r`: rows over the `│` the caller draws beside it.
pub(in crate::client) fn draw_side(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    let ink = Ink::of(t);
    fill(buf, r, ink.bg);
    app.screen.side_rect = r;
    hit(app, r, ScreenHit::SideFocus);
    let side_has_keys = side_focused(app);
    draw_head(buf, r, model, ink, side_has_keys);
    let lines = hairline_lines(app, model, t);
    let shown: Vec<TermId> = app.screen.tabs.get(app.screen.tab).map(|tab| tab.layout.leaves()).unwrap_or_default();
    let focus = open_row(app);
    let top = r.y + 2;
    let foot = r.height >= 6;
    let list_end = if foot { r.bottom().saturating_sub(2) } else { r.bottom() };
    let list_h = list_end.saturating_sub(top) as usize;
    let scroll = side_scroll(app, model, &lines, focus, list_h);
    note_rows(app, model, &lines);
    let mut indent = 0;
    for (i, line) in lines.iter().enumerate().skip(scroll).take(list_h) {
        let y = top + (i - scroll) as u16;
        match line {
            Line::Proj(pi) => draw_proj(app, buf, model, *pi, r, y, ink, t),
            Line::Detail(term) => {
                let Some((.., s)) = find(model, *term) else { continue };
                let x = r.x + NAME_IN + indent;
                let room = text_end(r).saturating_sub(x) as usize;
                put(buf, x, y, &[seg(truncate(&helpers_line_in(s.helpers_wanted, room), room), Style::default().fg(ink.amber).bg(ink.bg))], text_end(r));
                hit(app, Rect { x: r.x, y, width: r.width, height: 1 }, ScreenHit::HelperAsk(*term));
            }
            Line::Sess(.., depth) => {
                indent = u16::from(*depth) * HELPER_INDENT;
                let Some((_, _, s)) = line_term(model, line).and_then(|term| find(model, term)) else { continue };
                let keyboard = side_has_keys && app.screen.cursor == Some(s.term);
                let state = if keyboard || (!side_has_keys && Some(s.term) == focus) {
                    RowState::Focused
                } else if shown.contains(&s.term) || Some(s.term) == focus {
                    RowState::Visible
                } else {
                    RowState::Plain
                };
                app.screen.row_y.insert(s.term, y);
                draw_session(app, buf, s, Row { r, y, indent, state }, model, ink, t);
            }
            Line::Tab(owner, tab) => {
                let Some(line) = tab_line(app, model, *owner, *tab, t) else { continue };
                let x = r.x + NAME_IN + indent;
                let name_fg = if line.current { ink.teal } else { ink.text };
                let segs = tab_segs(&line, text_end(r).saturating_sub(x), Style::default().bg(ink.bg), name_fg, ink.line);
                draw_tab_line(app, buf, Rect { x: r.x, y, width: r.width, height: 1 }, x, text_end(r), segs, *tab);
            }
            Line::Section(kind, n) => {
                put(buf, r.x + TEXT_IN, y, &[seg(format!("{} {n}", kind.heading().to_lowercase()), Style::default().fg(ink.dim).bg(ink.bg))], text_end(r));
            }
            Line::Empty(pi) => {
                let st = Style::default().fg(ink.dim).bg(ink.bg);
                put(buf, r.x + NAME_IN, y, &[seg("empty", st.add_modifier(Modifier::ITALIC))], text_end(r));
                hit(app, Rect { x: r.x, y, width: r.width, height: 1 }, ScreenHit::ShellIn(*pi));
            }
            _ => {}
        }
    }
    scroll_marks(buf, text_end(r).saturating_sub(1), top, (scroll, list_h, lines.len()), Style::default().fg(ink.dim).bg(ink.bg));
    if foot {
        draw_foot(app, buf, r, ink);
    }
}

/// The end (exclusive) of right-aligned text: a column of padding before the `│`.
fn text_end(r: Rect) -> u16 {
    r.right().saturating_sub(1)
}

/// The sidebar's lines, one row per session: of the second lines only a request to start
/// helpers stays, since that line is where you answer it.
fn hairline_lines(app: &App, model: &[Proj], t: &Theme) -> Vec<Line> {
    let mut lines = side_lines(app, model, t);
    lines.retain(|l| match l {
        Line::Detail(term) => find(model, *term).is_some_and(|(.., s)| s.helpers_wanted > 0),
        _ => matches!(l, Line::Section(..) | Line::Proj(_) | Line::Sess(..) | Line::Tab(..) | Line::Empty(_) | Line::Gap),
    });
    if matches!(lines.last(), Some(Line::Gap)) {
        lines.pop();
    }
    lines
}

/// `sessions` and how many, dim; `sessions` teal while the sidebar has the keys.
fn draw_head(buf: &mut Buffer, r: Rect, model: &[Proj], ink: Ink, lit: bool) {
    let st = Style::default().bg(ink.bg);
    let n = model.iter().map(|p| p.sessions().count()).sum::<usize>();
    let host = crate::ipc::remote().map(|h| format!("⇄ {h}  ")).unwrap_or_default();
    let count = format!("{host}{n}");
    let end = text_end(r);
    let cx = end.saturating_sub(count.width() as u16);
    put(buf, r.x + TEXT_IN, r.y, &[seg("sessions", st.fg(if lit { ink.teal } else { ink.dim }))], cx.saturating_sub(1));
    put(buf, cx, r.y, &[seg(count, st.fg(ink.dim))], end);
}

/// A project's header: its name dim, its branch (or how many it hides, folded) dim at the
/// right. A click folds it; pointed at, `+` and `⋯` take the right.
#[allow(clippy::too_many_arguments)]
fn draw_proj(app: &mut App, buf: &mut Buffer, model: &[Proj], pi: usize, r: Rect, y: u16, ink: Ink, t: &Theme) {
    let p = &model[pi];
    let row = Rect { x: r.x, y, width: r.width, height: 1 };
    let open = !app.screen.saved.closed.contains(&format!("p:{}", p.key));
    let picked = side_focused(app) && app.screen.cursor_proj.as_ref() == Some(&p.key);
    let bg = if picked { ink.raised } else { ink.bg };
    fill(buf, row, bg);
    let st = Style::default().bg(bg);
    if picked {
        put(buf, r.x, y, &[seg("▎", st.fg(ink.teal))], r.x + 1);
    }
    let end = text_end(r);
    hit(app, row, ScreenHit::ToggleProj(pi));
    let pointed = hovered(app, row);
    let right = if pointed {
        String::new()
    } else if open {
        p.wts.iter().find(|w| w.main && !w.branch.is_empty()).map(|w| format!("{} {}", check::marks(app).branch, w.branch)).unwrap_or_default()
    } else {
        format!("{} hidden", p.sessions().count())
    };
    let name_room = end.saturating_sub(r.x + TEXT_IN + if right.is_empty() { 0 } else { right.width() as u16 + 1 });
    put(buf, r.x + TEXT_IN, y, &[seg(truncate(&p.name, name_room as usize), st.fg(ink.dim))], end);
    if pointed {
        plus_button(app, buf, Rect { x: end.saturating_sub(3), y, width: 1, height: 1 }, bg, ink, ScreenHit::ShellIn(pi));
        row_menu_button(app, buf, Rect { x: end.saturating_sub(1), y, width: 1, height: 1 }, bg, t, ScreenHit::RowMenuProj(pi));
    } else if !right.is_empty() && name_room > 0 {
        put(buf, end.saturating_sub(right.width() as u16), y, &[seg(right, st.fg(ink.dim))], end);
    }
}

/// A row's `+` button at `r`: dim, brightening under the pointer.
fn plus_button(app: &mut App, buf: &mut Buffer, r: Rect, bg: Color, ink: Ink, h: ScreenHit) {
    let fg = if hovered(app, r) { ink.strong } else { ink.dim };
    put(buf, r.x, r.y, &[seg("+", Style::default().fg(fg).bg(bg))], r.right());
    hit(app, r, h);
}

/// Where a session's row is drawn and how it looks.
#[derive(Debug, Clone, Copy)]
struct Row {
    /// The sidebar.
    r: Rect,
    y: u16,
    /// Columns a helper steps in under the session that started it.
    indent: u16,
    state: RowState,
}

/// One session: `▎` marker, glyph, name and at the right its age, or `needs you` in amber.
/// Pointed at (and not the open one), its `⋯` menu takes the age's place.
fn draw_session(app: &mut App, buf: &mut Buffer, s: &Session, at: Row, model: &[Proj], ink: Ink, t: &Theme) {
    let Row { r, y, indent, state } = at;
    let row = Rect { x: r.x, y, width: r.width, height: 1 };
    let needs = check::needs_you(s);
    let bg = if state == RowState::Plain { ink.bg } else { ink.raised };
    fill(buf, row, bg);
    let st = Style::default().bg(bg);
    hit(app, row, ScreenHit::Session(s.term));
    let marker = if state == RowState::Focused { Some(ink.teal) } else { needs.then_some(ink.amber) };
    if let Some(c) = marker {
        put(buf, r.x, y, &[seg("▎", st.fg(c))], r.x + 1);
    }
    let working = working_glyph(app);
    let (g, gc) = if plain_shell(s) { ("›".to_string(), ink.dim) } else {
        let m = check::mark(app, s, t, &working);
        (m.glyph.to_string(), m.color)
    };
    let gx = r.x + GLYPH_IN + indent;
    let nx = r.x + NAME_IN + indent;
    put(buf, gx, y, &[seg(g, st.fg(gc).add_modifier(Modifier::BOLD))], nx);
    hit(app, Rect { x: gx, y, width: 2, height: 1 }, ScreenHit::Why(s.term));
    let end = text_end(r);
    let menu = state != RowState::Focused && hovered(app, row);
    let right = if menu {
        String::new()
    } else if needs {
        needs_words(app, s, model, end.saturating_sub(nx))
    } else if s.is_agent {
        age(s.since)
    } else {
        String::new()
    };
    let right_fg = if needs { ink.amber } else { ink.dim };
    let rw = if menu { 1 } else { right.width() as u16 };
    let room = end.saturating_sub(nx + if rw > 0 { rw + 1 } else { 0 });
    let colour = if needs { ink.amber } else if state == RowState::Focused { ink.teal } else { ink.text };
    put(buf, nx, y, &[seg(truncate(&s.name, room as usize), st.fg(colour).add_modifier(Modifier::BOLD))], nx + room);
    if menu {
        row_menu_button(app, buf, Rect { x: end.saturating_sub(1), y, width: 1, height: 1 }, bg, t, ScreenHit::RowMenuSess(s.term));
    } else if rw > 0 && room > 0 {
        put(buf, end.saturating_sub(rw), y, &[seg(right, st.fg(right_fg))], end);
    }
}

/// The most of a row's name kept when its right names the pane that needs you, and the least
/// of that pane's name worth showing.
const NAME_KEPT: u16 = 10;
const PANE_NAME_KEPT: usize = 4;
/// Points from a split's row to the pane beside it that needs you.
const POINTER: &str = "▸ ";

/// What a row that needs you says at its right, in amber: `needs you`, or, when it is a split
/// and not its own pane but one beside it asks, that pane's name (`▸ warm-finch`), so the row
/// and its own pane, which may be `done`, don't seem to disagree. Without room, `needs you`.
fn needs_words(app: &App, s: &Session, model: &[Proj], room: u16) -> String {
    let own = app.snap.terms.get(&s.term).is_none_or(pane_needs);
    let beside = app.screen.tabs.iter().filter(|tab| tab.owner == s.term).flat_map(|tab| tab.layout.leaves()).find(|term| *term != s.term && app.snap.terms.get(term).is_some_and(pane_needs));
    match beside.filter(|_| !own) {
        Some(term) => {
            let kept = (s.name.width() as u16).min(NAME_KEPT);
            let fits = room.saturating_sub(1 + kept + POINTER.width() as u16) as usize;
            if fits < PANE_NAME_KEPT { "needs you".into() } else { format!("{POINTER}{}", truncate(&session_name(app, model, term), fits)) }
        }
        None => "needs you".into(),
    }
}

/// The foot, pinned to the bottom row: `new` at the left and `menu` at the right, dim words with
/// no keys, each clickable.
fn draw_foot(app: &mut App, buf: &mut Buffer, r: Rect, ink: Ink) {
    let y = r.bottom().saturating_sub(1);
    let (new, menu) = ("new", "menu");
    let (x0, end) = (r.x + TEXT_IN, text_end(r));
    if (end.saturating_sub(x0) as usize) < new.width() + 2 + menu.width() {
        return;
    }
    let st = Style::default().fg(ink.dim).bg(ink.bg);
    put(buf, x0, y, &[seg(new, st)], end);
    hit(app, Rect { x: x0, y, width: new.width() as u16, height: 1 }, ScreenHit::NewShell);
    let mx = end.saturating_sub(menu.width() as u16);
    put(buf, mx, y, &[seg(menu, st)], end);
    hit(app, Rect { x: mx, y, width: menu.width() as u16, height: 1 }, ScreenHit::Actions);
}
