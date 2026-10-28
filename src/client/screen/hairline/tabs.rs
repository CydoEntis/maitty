//! Hairline's tab row: the session's tabs across the top of the panes over a `─` rule, so a
//! tab never reads as a pane's name.

use super::*;

/// Rows the tab row takes: the tabs and the rule under them.
pub(in crate::client) const TAB_ROWS: u16 = 2;

/// Cells between a tab's words and the `│` at its right, either side.
const PAD: u16 = 1;

/// The tab row in `r`: the tabs from the pane's padding in, then the rule under them.
pub(in crate::client) fn draw_tab_row(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    if r.height < TAB_ROWS || r.width == 0 {
        return;
    }
    let ink = Ink::of(t);
    fill(buf, Rect { height: 1, ..r }, ink.bg);
    rule(buf, r.x, r.y + 1, r.width, ink.line, ink.bg);
    let row = Rect { x: r.x + PANE_PAD, width: r.width.saturating_sub(2 * PANE_PAD), height: 1, ..r };
    let end = tab_tools(app, buf, row, ink.dim, ink.strong, ink.bg, ink.teal);
    draw_tabs(app, buf, row.x, r.y, end, model, t);
}

/// The `│` right of a tab (or the `+`) at `x`, meeting the rule under it in a `┴`.
fn divider(buf: &mut Buffer, x: u16, y: u16, ink: Ink) {
    let st = Style::default().fg(ink.line).bg(ink.bg);
    buf[(x, y)].set_symbol("│").set_style(st);
    buf[(x, y + 1)].set_symbol("┴").set_style(st);
}

/// The session's tabs from `x`, each led by its agents' mark: numbered while there are
/// several, the current one teal and bold, and a dim ✕ that closes it; then a dim `+` that
/// opens one; a `│` ends each.
/// A tab that does not fit before `end` is left out, with the `+`.
fn draw_tabs(app: &mut App, buf: &mut Buffer, x: u16, y: u16, end: u16, model: &[Proj], t: &Theme) {
    let ink = Ink::of(t);
    let st = Style::default().bg(ink.bg);
    let tabs = app.session_tabs();
    let mut at = x;
    for (n, &i) in tabs.iter().enumerate() {
        let name = tab_name(&app.screen.tabs[i]).unwrap_or_else(|| session_name(app, model, app.screen.tabs[i].focus));
        let style = if i == app.screen.tab { st.fg(ink.teal).add_modifier(Modifier::BOLD) } else { st.fg(ink.dim) };
        let mut label: Vec<Seg> = tab_mark(app, i, t).map(|(g, c)| seg(format!("{g} "), st.fg(c).add_modifier(Modifier::BOLD))).into_iter().collect();
        label.push(seg(if tabs.len() > 1 { format!("{} {name}", n + 1) } else { name }, style));
        label.push(seg(format!(" {}", super::super::side_tabs::shape(app.screen.tabs[i].layout.leaves().len())), st.fg(ink.dim)));
        let close = if i == app.screen.tab { tab_close(app, i, tabs.len()) } else { None };
        if close.is_some() {
            label.push(seg(format!(" {}", glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons).close), st.fg(ink.dim)));
        }
        let w = segs_width(&label);
        if at + w + PAD >= end {
            return;
        }
        put(buf, at, y, &label, end);
        hit(app, Rect { x: at, y, width: w, height: 1 }, ScreenHit::TabPick(i));
        if let Some(h) = close {
            hit(app, Rect { x: at + w - 1, y, width: 1, height: 1 }, h);
        }
        divider(buf, at + w + PAD, y, ink);
        at += w + 2 * PAD + 1;
    }
    if at + 1 + PAD < end {
        put(buf, at, y, &[seg("+", st.fg(ink.dim))], end);
        hit(app, Rect { x: at, y, width: 1, height: 1 }, ScreenHit::NewTab);
        divider(buf, at + 1 + PAD, y, ink);
    }
}

/// The mark of the tab's most urgent agent, so a tab out of view still shows it works or asks.
fn tab_mark(app: &App, tab: usize, t: &Theme) -> Option<(String, Color)> {
    pane::mark(app, tab_agent(app, tab)?, t)
}
