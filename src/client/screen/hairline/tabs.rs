//! Hairline's tab row: the session's tabs across the top of the panes over a `─` rule, so a
//! tab never reads as a pane's name.

use super::*;

/// Rows the tab row takes: the tabs and the rule under them.
pub(in crate::client) const TAB_ROWS: u16 = 2;

/// Cells between two tabs, and between the last tab and the `+`.
const GAP: u16 = 3;

/// The tab row in `r`: the tabs from the pane's padding in, then the rule under them.
pub(in crate::client) fn draw_tab_row(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    if r.height < TAB_ROWS || r.width == 0 {
        return;
    }
    let ink = Ink::of(t);
    fill(buf, Rect { height: 1, ..r }, ink.bg);
    draw_tabs(app, buf, r.x + PANE_PAD, r.y, r.right().saturating_sub(PANE_PAD), model, t);
    rule(buf, r.x, r.y + 1, r.width, ink.line, ink.bg);
}

/// The session's tabs from `x`, each led by its agents' mark: numbered while there are
/// several, the current one teal and bold, then a dim `+` that opens one. A tab that does not
/// fit before `end` is left out, with the `+`.
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
        let w = segs_width(&label);
        if at + w > end {
            return;
        }
        put(buf, at, y, &label, end);
        hit(app, Rect { x: at, y, width: w, height: 1 }, ScreenHit::TabPick(i));
        at += w + GAP;
    }
    if at < end {
        put(buf, at, y, &[seg("+", st.fg(ink.dim))], end);
        hit(app, Rect { x: at, y, width: 1, height: 1 }, ScreenHit::NewTab);
    }
}

/// The mark of the tab's most urgent agent, so a tab out of view still shows it works or asks.
fn tab_mark(app: &App, tab: usize, t: &Theme) -> Option<(String, Color)> {
    let urgency = |i: &TermInfo| if pane_needs(i) { 0 } else { i.status.urgency() };
    let leaves = app.screen.tabs.get(tab)?.layout.leaves();
    let info = leaves.iter().filter_map(|id| app.snap.terms.get(id)).filter(|i| i.agent.is_some()).min_by_key(|i| urgency(i))?;
    pane::mark(app, info, t)
}
