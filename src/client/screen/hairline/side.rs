//! Hairline's sidebar: `sessions` and its count, dim project headers with their branch, one row
//! per session (no second lines), and the new and menu hints pinned to the foot.

use super::*;

/// Where a row's pieces go, from the sidebar's left edge: the `▎` marker on the edge itself,
/// words and headers a column in, the glyph two in and the name two after it.
const TEXT_IN: u16 = 1;
const GLYPH_IN: u16 = 2;
const NAME_IN: u16 = 4;

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
    // A split is one row (its first pane's), whichever side of it has the keys.
    let focus = app.focused().map(|f| if shown.len() > 1 && shown.contains(&f) { shown[0] } else { f });
    let top = r.y + 2;
    let foot = r.height >= 6;
    let list_end = if foot { r.bottom().saturating_sub(2) } else { r.bottom() };
    let list_h = list_end.saturating_sub(top) as usize;
    let scroll = side_scroll(app, model, &lines, focus, list_h);
    note_rows(app, model, &lines);
    for (i, line) in lines.iter().enumerate().skip(scroll).take(list_h) {
        let y = top + (i - scroll) as u16;
        match line {
            Line::Proj(pi) => draw_proj(app, buf, model, *pi, r, y, ink, t),
            Line::Sess(..) => {
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
                draw_session(app, buf, s, r, y, state, ink, t);
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
    let dim = Style::default().fg(ink.dim).bg(ink.bg);
    if scroll > 0 {
        put(buf, text_end(r).saturating_sub(1), top, &[seg("▲", dim)], text_end(r));
    }
    if scroll + list_h < lines.len() {
        put(buf, text_end(r).saturating_sub(1), top + list_h as u16 - 1, &[seg("▼", dim)], text_end(r));
    }
    if foot {
        draw_foot(app, buf, r, ink);
    }
}

/// The end (exclusive) of right-aligned text: a column of padding before the `│`.
fn text_end(r: Rect) -> u16 {
    r.right().saturating_sub(1)
}

/// The sidebar's lines without the second and third lines and notes: one row per session.
fn hairline_lines(app: &App, model: &[Proj], t: &Theme) -> Vec<Line> {
    let mut lines = side_lines(app, model, t);
    lines.retain(|l| matches!(l, Line::Section(..) | Line::Proj(_) | Line::Sess(..) | Line::Empty(_) | Line::Gap));
    if matches!(lines.last(), Some(Line::Gap)) {
        lines.pop();
    }
    lines
}

/// What the keyboard and the mouse read back from the rows drawn: their order and places.
fn note_rows(app: &mut App, model: &[Proj], lines: &[Line]) {
    app.screen.visible = lines.iter().filter_map(|l| line_term(model, l)).collect();
    app.screen.side_items = lines
        .iter()
        .filter_map(|l| match l {
            Line::Proj(pi) => Some(SideItem::Proj(model[*pi].key.clone())),
            Line::Sess(..) => line_term(model, l).map(SideItem::Sess),
            _ => None,
        })
        .collect();
    app.screen.row_y.clear();
    app.screen.proj_keys = model.iter().map(|p| p.key.clone()).collect();
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

/// One session: `▎` marker, glyph, name and at the right its age, or `needs you` in amber.
/// Pointed at (and not the open one), its `⋯` menu takes the age's place.
#[allow(clippy::too_many_arguments)]
fn draw_session(app: &mut App, buf: &mut Buffer, s: &Session, r: Rect, y: u16, state: RowState, ink: Ink, t: &Theme) {
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
    let gx = r.x + GLYPH_IN;
    put(buf, gx, y, &[seg(g, st.fg(gc).add_modifier(Modifier::BOLD))], r.x + NAME_IN);
    hit(app, Rect { x: gx, y, width: 2, height: 1 }, ScreenHit::Why(s.term));
    let end = text_end(r);
    let menu = state != RowState::Focused && hovered(app, row);
    let right = if menu {
        Vec::new()
    } else if needs {
        vec![seg("needs you", st.fg(ink.amber))]
    } else if s.is_agent {
        vec![seg(age(s.since), st.fg(ink.dim))]
    } else {
        Vec::new()
    };
    let rw = if menu { 1 } else { segs_width(&right) };
    let nx = r.x + NAME_IN;
    let room = end.saturating_sub(nx + if rw > 0 { rw + 1 } else { 0 });
    let colour = if needs { ink.amber } else if state == RowState::Focused { ink.teal } else { ink.text };
    put(buf, nx, y, &[seg(truncate(&s.name, room as usize), st.fg(colour).add_modifier(Modifier::BOLD))], nx + room);
    if menu {
        row_menu_button(app, buf, Rect { x: end.saturating_sub(1), y, width: 1, height: 1 }, bg, t, ScreenHit::RowMenuSess(s.term));
    } else if rw > 0 && room > 0 {
        put(buf, end.saturating_sub(rw), y, &right, end);
    }
}

/// The foot, pinned to the bottom row: the new-shell hint at the left and the menu's at the
/// right, dim, each naming its real key (the leader, then the key) and clickable. The keys go
/// before the words do when the sidebar is narrow.
fn draw_foot(app: &mut App, buf: &mut Buffer, r: Rect, ink: Ink) {
    let y = r.bottom().saturating_sub(1);
    let lead = crate::client::design::long_key(&app.keymap.prefix);
    let keyed = |key: String, word: &str| if key.is_empty() { word.to_string() } else { format!("{key} {word}") };
    let (nk, mk) = (k(app, &Action::ShellHere), k(app, &Action::Menu));
    let news = [keyed(if nk.is_empty() { String::new() } else { format!("{lead} {nk}") }, "new"), keyed(nk, "new"), "new".to_string()];
    let menus = [keyed(mk, "menu"), "menu".to_string()];
    let (x0, end) = (r.x + TEXT_IN, text_end(r));
    let room = end.saturating_sub(x0);
    let tries = [(0, 0), (0, 1), (1, 0), (1, 1), (2, 1)];
    let Some((new, menu)) = tries.iter().map(|&(n, m)| (&news[n], &menus[m])).find(|(n, m)| n.width() + 2 + m.width() <= room as usize) else { return };
    let st = Style::default().fg(ink.dim).bg(ink.bg);
    let nw = new.width() as u16;
    put(buf, x0, y, &[seg(new.clone(), st)], end);
    hit(app, Rect { x: x0, y, width: nw, height: 1 }, ScreenHit::NewShell);
    let mw = menu.width() as u16;
    let mx = end.saturating_sub(mw);
    put(buf, mx, y, &[seg(menu.clone(), st)], end);
    hit(app, Rect { x: mx, y, width: mw, height: 1 }, ScreenHit::Actions);
}
