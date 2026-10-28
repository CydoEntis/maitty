//! A session's tabs under its sidebar row, one line each with its own state, so a tab you are
//! not on still says it works or needs you (ADR 0027). Both sidebars draw them.

use super::check;
use super::glyphs::{self, Kind as State};
use super::*;

/// One of a session's tabs as its sidebar line shows it.
pub(super) struct TabLine {
    /// The last of the session's tabs: its branch is `└`, the others' `├`.
    pub last: bool,
    /// Its number and name: `2 tests`.
    pub label: String,
    /// The tab on screen.
    pub current: bool,
    /// Its most urgent agent's glyph, word and colour; none for a tab of shells.
    pub mark: Option<(String, String, Color)>,
}

/// The tabs of session `owner`, in order, when it has two or more; a lone tab is the row itself.
fn session_tab_indexes(app: &App, owner: TermId) -> Vec<usize> {
    let tabs: Vec<usize> = (0..app.screen.tabs.len()).filter(|&i| app.screen.tabs[i].owner == owner).collect();
    if tabs.len() > 1 { tabs } else { Vec::new() }
}

/// The tabs of `owner` that get a line, by `ui.sidebar_tabs`: all, the busy ones (working or
/// needing you) and the one on screen, or none. Never the tab the session's own pane is in:
/// the session's row is that one, and a line for it would name the session twice.
pub(super) fn shown_tab_indexes(app: &App, owner: TermId) -> Vec<usize> {
    let busy = |i: usize| i == app.screen.tab || tab_agent(app, i).is_some_and(|a| a.status == Status::Working || check::needs(a.status, a.check()));
    let others = session_tab_indexes(app, owner).into_iter().filter(|&i| !app.screen.tabs[i].layout.contains(owner));
    match app.cfg.ui.sidebar_tabs.as_str() {
        "off" => Vec::new(),
        "busy" => others.filter(|&i| busy(i)).collect(),
        _ => others.collect(),
    }
}

/// Tab `tab` of session `owner` as its line shows it.
pub(super) fn tab_line(app: &App, model: &[Proj], owner: TermId, tab: usize, t: &Theme) -> Option<TabLine> {
    let n = session_tab_indexes(app, owner).iter().position(|&i| i == tab)?;
    let screen_tab = app.screen.tabs.get(tab)?;
    let name = tab_name(screen_tab).unwrap_or_else(|| session_name(app, model, screen_tab.focus));
    Some(TabLine { last: shown_tab_indexes(app, owner).last() == Some(&tab), label: format!("{} {name}", n + 1), current: tab == app.screen.tab, mark: tab_mark(app, tab, t) })
}

/// The mark of the tab's most urgent agent (`tab_agent`): glyph, word and colour.
pub(super) fn tab_mark(app: &App, tab: usize, t: &Theme) -> Option<(String, String, Color)> {
    let info = tab_agent(app, tab)?;
    let set = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let spinner = glyph(app, Status::Working);
    let step = if app.motion_on() { check::ring_step(app.spinner_frame(), set.ring.len()) } else { 0 };
    let m = State::of(info.status, info.check(), info.asleep).mark(set, t, step, &spinner);
    Some((m.glyph.to_string(), m.word.to_string(), m.color))
}

/// The line's left (branch, number and name) and right (the mark) in `room` columns on `st`.
/// The mark's word gives way first, then the name shortens; the glyph stays.
pub(super) fn tab_segs(line: &TabLine, room: u16, st: Style, name_fg: Color, branch_fg: Color) -> (Vec<Seg>, Vec<Seg>) {
    let branch = if line.last { "└ " } else { "├ " };
    let mark = |word: bool| {
        line.mark.as_ref().map(|(g, w, c)| seg(if word { format!("{g} {w}") } else { g.clone() }, st.fg(*c).add_modifier(Modifier::BOLD)))
    };
    let wide = |m: &Option<Seg>| m.as_ref().map_or(0, |(s, _)| s.width() as u16 + 1);
    let left_w = branch.width() as u16 + line.label.width() as u16;
    let full = mark(true);
    let right = if left_w + wide(&full) <= room { full } else { mark(false) };
    let name_room = room.saturating_sub(branch.width() as u16 + wide(&right)) as usize;
    let name_st = if line.current { st.fg(name_fg).add_modifier(Modifier::BOLD) } else { st.fg(name_fg) };
    (vec![seg(branch, st.fg(branch_fg)), seg(truncate(&line.label, name_room), name_st)], right.into_iter().collect())
}

/// Draw `line`'s pieces between `x` and `end` on row `y`, the mark against `end`; a click on the
/// row picks the tab.
pub(super) fn draw_tab_line(app: &mut App, buf: &mut Buffer, row: Rect, x: u16, end: u16, segs: (Vec<Seg>, Vec<Seg>), tab: usize) {
    let (left, right) = segs;
    put(buf, x, row.y, &left, end);
    put(buf, end.saturating_sub(segs_width(&right)), row.y, &right, end);
    hit(app, row, ScreenHit::TabPick(tab));
}
