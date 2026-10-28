//! A session's tabs under its sidebar row, one line each with its own state, so a tab you are
//! not on still says it works or needs you (ADR 0027). Both sidebars draw them.

use super::check;
use super::glyphs::{self, Kind as State};
use super::*;

/// One of a session's tabs as its sidebar line shows it.
pub(super) struct TabLine {
    /// Its name: its own, else its focused session's.
    pub label: String,
    /// The tab on screen.
    pub current: bool,
    /// An agent in it needs you.
    pub needs: bool,
    /// Its most urgent agent's glyph, word and colour; none for a tab of shells.
    pub mark: Option<(String, String, Color)>,
    /// When that agent's state began, for its age at the right.
    pub since: Option<u64>,
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
    session_tab_indexes(app, owner).contains(&tab).then_some(())?;
    let screen_tab = app.screen.tabs.get(tab)?;
    let name = tab_name(screen_tab).unwrap_or_else(|| session_name(app, model, screen_tab.focus));
    let agent = tab_agent(app, tab);
    Some(TabLine {
        label: name,
        current: tab == app.screen.tab,
        needs: agent.is_some_and(|a| check::needs(a.status, a.check())),
        mark: tab_mark(app, tab, t),
        since: agent.map(|a| a.since),
    })
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

/// The line's left (state glyph and name) and right (the agent's age) in `room` columns on `st`. The name shortens first; the glyph stays.
pub(super) fn tab_segs(line: &TabLine, room: u16, st: Style, name_fg: Color, dim: Color) -> (Vec<Seg>, Vec<Seg>) {
    let right = line.since.map(age).unwrap_or_default();
    let right_w = right.width() as u16;
    // A tab of shells has no glyph, but its name keeps the column an agent's would.
    let glyph = line.mark.as_ref().map_or_else(|| seg("  ", st), |(g, _, c)| seg(format!("{g} "), st.fg(*c).add_modifier(Modifier::BOLD)));
    let glyph_w = glyph.0.width() as u16;
    let name_room = room.saturating_sub(glyph_w + right_w + 1) as usize;
    let name_st = if line.current { st.fg(name_fg).add_modifier(Modifier::BOLD) } else { st.fg(name_fg) };
    let mut left = vec![glyph];
    left.push(seg(truncate(&line.label, name_room), name_st));
    (left, vec![seg(right, st.fg(dim))])
}

/// Draw `line`'s pieces between `x` and `end` on row `y`, the mark against `end`; a click on the
/// row picks the tab.
pub(super) fn draw_tab_line(app: &mut App, buf: &mut Buffer, row: Rect, x: u16, end: u16, segs: (Vec<Seg>, Vec<Seg>), tab: usize) {
    let (left, right) = segs;
    put(buf, x, row.y, &left, end);
    put(buf, end.saturating_sub(segs_width(&right)), row.y, &right, end);
    hit(app, row, ScreenHit::TabPick(tab));
}
