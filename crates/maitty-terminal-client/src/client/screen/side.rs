//! The sidebar: fleets above, ships below. The top half lists each machine with its fleets, the
//! bottom half lists the agent ships of each fleet, two lines each. A divider with a title chip
//! sits between the halves and each half clips at its own edge. `SideModel::lists` is the one
//! ordered list of selectable rows; drawing, the arrow keys and Enter all read it.

use super::chips::{Chip, chip, right_chips};
use super::fleets::{Fleet, FleetId, MachineGroup};
use super::status_mark::State;
use super::*;
use crate::client::ids::ClientSession;

/// How long a new row's accent bar shows.
const FLASH: std::time::Duration = std::time::Duration::from_millis(500);
/// Rows an agent ship takes.
const SHIP_H: u16 = 2;
/// Columns from the sidebar's left edge: the current-row bar, the fold arrow, the icon, the name.
const BAR_X: u16 = 1;
const ARROW_X: u16 = 2;
const ICON_X: u16 = 4;
const NAME_X: u16 = 6;
/// The search field's text when nothing is typed.
const FILTER_HINT: &str = "Filter sessions";
const FILTER_ICON: &str = "⌕";
/// The mark after a name for a pinned session.
const PIN: &str = "⌖";
/// The mark after a name for a session with output you have not looked at.
const UNREAD: &str = "•";
/// What a session stopped by a restart offers in place of its view mark.
const RESUME: &str = "⏎ resume";
/// Placed before a remote machine's name, and before a reconnecting one's.
const REMOTE_MARK: &str = "⇄";
const RECONNECTING_MARK: &str = "┆";
const LOCAL: &str = "local";
/// A machine heading's glyph for this computer and for another.
const LOCAL_GLYPH: &str = "⌂";
const REMOTE_GLYPH: &str = "▤";
const ONLINE_DOT: &str = "●";
/// The fold arrows.
const OPEN: &str = "▾";
const FOLDED: &str = "▸";
/// A fleet's icon when nothing in it needs you.
const DIAMOND: &str = "◆";
/// The agent count's mark and the shell count's.
const AGENTS: &str = "✻";
const SHELLS: &str = ">_";
/// An agent ship's view mark: the terminal, or a chat.
const VIEW_TERMINAL: &str = ">_";
const VIEW_CHAT: &str = "◌";
/// Below this many rows tall the footer keeps only its buttons.
const FOOTER_FULL_FROM: u16 = 10;
/// From this height the halves have a blank row under the border, the field and each divider.
const AIRY_FROM: u16 = 14;
/// The row the FLEETS divider is on when the sidebar is tall.
const FLEETS_DIVIDER: u16 = 4;

/// Where a session runs, as the filter reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Where {
    Local,
    Remote(String),
    /// Its machine's line is down; the row is the last report.
    Reconnecting(String),
}

impl Where {
    /// The text, with its mark.
    fn text(&self) -> String {
        match self {
            Where::Local => LOCAL.to_string(),
            Where::Remote(name) => format!("{REMOTE_MARK} {name}"),
            Where::Reconnecting(name) => format!("{RECONNECTING_MARK} {name}"),
        }
    }
}

/// A session as the sidebar lists it.
#[derive(Debug, Clone)]
pub(super) struct FlatRow {
    pub key: ClientSession,
    pub term: Option<ClientTerm>,
    /// Your name for it, else what runs in it.
    pub name: String,
    /// The last segment of its project's folder.
    pub project: String,
    pub place: Where,
    pub state: State,
    pub unread: bool,
    pub pinned: bool,
    /// A restart stopped its agent: Enter resumes it.
    pub resume: bool,
    pub process: String,
    /// The fleet it is in; a chat with no pane goes in its machine's first fleet.
    pub fleet: Option<FleetId>,
    /// An agent (a ship in the lower half), not a shell.
    pub agent: bool,
    /// The git branch it works on, and whether that is a linked worktree.
    pub branch: String,
    pub worktree: bool,
    /// A chat with no terminal of its own.
    pub chat: bool,
    /// Creation order: ids only count up.
    order: (u64, MachineId),
}

impl FlatRow {
    fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        q.is_empty()
            || [&self.name, &self.project, &self.process, &self.place.text()]
                .iter()
                .any(|field| field.to_lowercase().contains(&q))
    }
}

/// What a session is called: your name for it, else what runs in it, else `fallback`.
pub(in crate::client) fn display_name(info: Option<&TermInfo>, fallback: &str) -> String {
    let custom = info.map_or("", |i| i.label.trim());
    let process = info.map_or("", |i| i.process.trim());
    if !custom.is_empty() {
        custom.to_string()
    } else if !process.is_empty() {
        process.to_string()
    } else {
        fallback.to_string()
    }
}

/// The last folder of a path, as the project's name.
fn last_segment(p: &Proj) -> String {
    p.name.clone()
}

/// Every session in creation order.
pub(super) fn flat_rows(app: &App, model: &[Proj]) -> Vec<FlatRow> {
    let first_fleet = |machine: MachineId| {
        app.snap_of(machine)
            .and_then(|s| s.workspaces.first())
            .map(|w| FleetId { machine, ws: w.id })
    };
    let mut rows: Vec<FlatRow> = model
        .iter()
        .flat_map(|p| p.sessions().map(move |s| (p, s)))
        .map(|(p, s)| {
            let info = s.native_term.and_then(|t| app.term_info(t));
            let process = info.map_or(String::new(), |i| i.process.clone());
            // A chat goes by its title, even once it has a terminal.
            let name = if matches!(s.key.key, SessionKey::Conversation(_)) {
                s.name.clone()
            } else {
                display_name(info, &s.name)
            };
            let failed = super::status_mark::failed(
                info.is_some_and(|i| i.exit.is_some()),
                s.status,
                s.check.as_ref(),
            );
            let place = if s.key.machine == MachineId::LOCAL {
                Where::Local
            } else if s.down {
                Where::Reconnecting(app.machine_label_of(s.key.machine))
            } else {
                Where::Remote(app.machine_label_of(s.key.machine))
            };
            let fleet = s
                .native_term
                .and_then(|t| app.fleet_of(t))
                .or_else(|| first_fleet(s.key.machine));
            FlatRow {
                key: s.key,
                term: s.native_term,
                name,
                project: last_segment(p),
                place,
                state: State::of(s.status, failed, s.down),
                unread: info.is_some_and(|i| i.bell),
                pinned: info.is_some_and(|i| i.pinned),
                resume: info.is_some_and(|i| i.interrupted.is_some()),
                process,
                fleet,
                agent: s.is_agent,
                branch: info
                    .and_then(|i| i.branch.clone())
                    .unwrap_or_default(),
                worktree: info.is_some_and(|i| i.linked),
                chat: s.native_term.is_none(),
                order: (
                    s.native_term.map_or(u64::MAX, |t| u64::from(t.term)),
                    s.key.machine,
                ),
            }
        })
        .collect();
    rows.sort_by_key(|r| (r.order, r.key));
    rows
}

// ---- the lists -------------------------------------------------------------------------------

/// How many sessions of a fleet need you, run an agent, and are shells.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Counts {
    pub needs: usize,
    pub agents: usize,
    pub shells: usize,
}

/// A row of the top half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TopRow {
    /// A machine heading: its index in `SideModel::groups`.
    Machine(usize),
    /// A fleet: its machine's index, then its own.
    Fleet(usize, usize),
}

/// A row of the bottom half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShipLine {
    /// A fleet's ship group heading.
    Group(usize, usize),
    /// An agent: its index in `SideModel::rows`.
    Ship(usize),
}

/// What the sidebar shows once the filter has had its say.
pub(super) struct SideModel {
    pub groups: Vec<MachineGroup>,
    /// The agent ships that pass the filter, in creation order.
    pub rows: Vec<FlatRow>,
    pub counts: HashMap<FleetId, Counts>,
    /// A filter is typed: folds are ignored so every match shows.
    pub filtering: bool,
}

/// The lists in the order the sidebar draws and walks them.
pub(super) struct Lists {
    pub top: Vec<(TopRow, SideItem)>,
    pub ships: Vec<(ShipLine, SideItem)>,
}

impl Lists {
    /// Every selectable row, top to bottom.
    pub fn items(&self) -> Vec<SideItem> {
        self.top
            .iter()
            .map(|(_, i)| i.clone())
            .chain(self.ships.iter().map(|(_, i)| i.clone()))
            .collect()
    }
}

fn contains(haystack: &str, query: &str) -> bool {
    haystack.to_lowercase().contains(query)
}

impl SideModel {
    pub fn build(app: &App, model: &[Proj]) -> SideModel {
        let query = app.screen.filter.trim().to_lowercase();
        let all = flat_rows(app, model);
        let mut groups = app.fleet_groups();
        let mut counts: HashMap<FleetId, Counts> = HashMap::new();
        for g in &groups {
            for f in &g.fleets {
                let mine = all.iter().filter(|r| r.fleet == Some(f.id));
                let c = Counts {
                    needs: mine.clone().filter(|r| r.state == State::Needs).count(),
                    agents: mine.clone().filter(|r| r.agent).count(),
                    shells: f
                        .ships
                        .iter()
                        .filter(|t| app.term_info(**t).is_some_and(|i| i.agent.is_none()))
                        .count(),
                };
                counts.insert(f.id, c);
            }
        }
        let name_of = |id: FleetId| {
            groups
                .iter()
                .flat_map(|g| &g.fleets)
                .find(|f| f.id == id)
                .map(|f| f.name.clone())
                .unwrap_or_default()
        };
        let rows: Vec<FlatRow> = all
            .iter()
            .filter(|r| r.agent)
            .filter(|r| {
                query.is_empty()
                    || r.matches(&query)
                    || r.fleet.is_some_and(|f| contains(&name_of(f), &query))
            })
            .cloned()
            .collect();
        if !query.is_empty() {
            for g in &mut groups {
                g.fleets.retain(|f| {
                    contains(&f.name, &query) || rows.iter().any(|r| r.fleet == Some(f.id))
                });
            }
            groups.retain(|g| !g.fleets.is_empty());
        }
        SideModel {
            groups,
            rows,
            counts,
            filtering: !query.is_empty(),
        }
    }

    fn folded(&self, app: &App, key: &str) -> bool {
        !self.filtering && app.screen.saved.closed.iter().any(|c| c == key)
    }

    /// The lists. `column` is the collapsed column: no headings, nothing folded away.
    pub fn lists(&self, app: &App, column: bool) -> Lists {
        let mut top = Vec::new();
        for (gi, g) in self.groups.iter().enumerate() {
            let folded = !column && self.folded(app, &g.fold_key());
            if !column {
                top.push((TopRow::Machine(gi), SideItem::Proj(g.fold_key())));
            }
            if folded {
                continue;
            }
            for (fi, f) in g.fleets.iter().enumerate() {
                top.push((TopRow::Fleet(gi, fi), SideItem::Fleet(f.id)));
            }
        }
        let mut ships = Vec::new();
        for (gi, g) in self.groups.iter().enumerate() {
            for (fi, f) in g.fleets.iter().enumerate() {
                let mine: Vec<usize> = self
                    .rows
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.fleet == Some(f.id))
                    .map(|(i, _)| i)
                    .collect();
                if mine.is_empty() {
                    continue;
                }
                let key = f.fold_key(&g.key);
                if !column {
                    ships.push((ShipLine::Group(gi, fi), SideItem::Proj(key.clone())));
                }
                if !column && self.folded(app, &key) {
                    continue;
                }
                for i in mine {
                    ships.push((ShipLine::Ship(i), SideItem::Sess(self.rows[i].key)));
                }
            }
        }
        // A chat on a machine with no fleet yet has no group to sit under: it is listed on its own.
        for (i, r) in self.rows.iter().enumerate().filter(|(_, r)| r.fleet.is_none()) {
            ships.push((ShipLine::Ship(i), SideItem::Sess(r.key)));
        }
        Lists { top, ships }
    }

    pub fn fleet(&self, gi: usize, fi: usize) -> &Fleet {
        &self.groups[gi].fleets[fi]
    }

    pub fn counts_of(&self, id: FleetId) -> Counts {
        self.counts.get(&id).copied().unwrap_or_default()
    }
}

/// The sessions as the sidebar lists them, top to bottom.
pub(in crate::client) fn side_keys(app: &App, model: &[Proj]) -> Vec<ClientSession> {
    SideModel::build(app, model)
        .rows
        .iter()
        .map(|r| r.key)
        .collect()
}

/// The sessions open in the workspace: the panes of the tab on screen, the focused one among
/// them.
pub(super) fn open_terms(app: &App) -> Vec<ClientTerm> {
    let focus = app.focused();
    let mut open: Vec<ClientTerm> = app
        .screen
        .tabs
        .get(app.screen.tab)
        .filter(|tab| focus.is_some_and(|f| tab.contains(f)))
        .map(ScreenTab::leaves)
        .unwrap_or_default();
    if let Some(f) = focus
        && !open.contains(&f)
    {
        open.push(f);
    }
    open
}

/// `▲` at a list's top when rows are scrolled off above, `▼` at its bottom when more are below,
/// in column `x`.
pub(super) fn scroll_marks(buf: &mut Buffer, x: u16, (top, rows_h): (u16, u16), more: (bool, bool), st: Style) {
    if more.0 {
        put(buf, x, top, &[seg("▲", st)], x + 1);
    }
    if more.1 {
        put(
            buf,
            x,
            top + rows_h.saturating_sub(1),
            &[seg("▼", st)],
            x + 1,
        );
    }
}

/// Which rows of a list show: the first one, and how many fit. The row `want` is kept in view
/// when `follow` is set, otherwise the wheel's scroll stands.
pub(super) fn window(heights: &[u16], scroll: usize, want: Option<usize>, room: u16) -> (usize, usize) {
    let mut first = scroll.min(heights.len().saturating_sub(1));
    if let Some(w) = want.filter(|w| *w < heights.len()) {
        if w < first {
            first = w;
        }
        while first < w && heights[first..=w].iter().sum::<u16>() > room {
            first += 1;
        }
    }
    // Scrolled past the point where the end of the list is already in view.
    while first > 0 && heights[first - 1..].iter().sum::<u16>() <= room {
        first -= 1;
    }
    let mut used = 0;
    let mut shown = 0;
    for h in &heights[first..] {
        if used + h > room {
            break;
        }
        used += h;
        shown += 1;
    }
    (first, shown)
}

/// The row the sidebar cursor, or failing that the open session, sits on in a list.
fn cursor_item(app: &App) -> Option<SideItem> {
    if let Some(k) = &app.screen.cursor_proj {
        return Some(SideItem::Proj(k.clone()));
    }
    if let Some(f) = app.screen.cursor_fleet {
        return Some(SideItem::Fleet(f));
    }
    app.screen.cursor.map(SideItem::Sess)
}

/// A divider across the box at `y`, joined to its sides, with a title chip on it.
pub(super) fn divider(app: &App, buf: &mut Buffer, r: Rect, y: u16, title: &str, border: Color, t: &Theme) {
    let line = format!("├{}┤", "─".repeat(r.width.saturating_sub(2) as usize));
    put(
        buf,
        r.x,
        y,
        &[seg(line, Style::default().fg(border).bg(t.bg))],
        r.right(),
    );
    let c = chip(app, title, Chip::Quiet, t.bg, t);
    put(buf, r.x + 2, y, &c, r.right().saturating_sub(2));
}

/// The sidebar: the frame with its chips, the search field, the two halves and the footer.
pub(in crate::client) fn draw_side(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    model: &[Proj],
    t: &Theme,
) {
    let focused = side_focused(app);
    let mut c = Card::new(t, "");
    if focused {
        c = c.lit(t.accent);
    }
    card(app, buf, r, &c, t);
    app.screen.side_rect = r;
    hit(app, r, ScreenHit::SideFocus);
    if r.width < 12 || r.height < 5 {
        return;
    }
    let border = if focused { t.accent } else { t.line };
    let (x0, end) = (r.x + 2, r.right().saturating_sub(2));

    // The top border: « on the left; + and the inbox on the right.
    let needing = SideModel::build(app, model)
        .counts
        .values()
        .map(|c| c.needs)
        .sum::<usize>();
    let inbox_n = super::inbox::inbox_count(app);
    let collapse = chip(app, "«", Chip::Action, t.bg, t);
    let collapse_end = x0 + segs_width(&collapse);
    put(buf, x0, r.y, &collapse, collapse_end);
    hit(
        app,
        Rect {
            x: x0,
            y: r.y,
            width: segs_width(&collapse),
            height: 1,
        },
        ScreenHit::CollapseSidebar,
    );
    let plus = chip(app, "+", Chip::Action, t.bg, t);
    let inbox = chip(
        app,
        &format!("inbox {inbox_n}"),
        if needing > 0 {
            Chip::Alert
        } else {
            Chip::Quiet
        },
        t.bg,
        t,
    );
    right_chips(
        app,
        buf,
        r.y,
        (collapse_end + 1, end),
        vec![
            (plus, Some(ScreenHit::NewShell)),
            (inbox, Some(ScreenHit::OpenInbox)),
        ],
        border,
    );

    // The footer: version or update, a rule, and the buttons.
    let full_foot = r.height >= FOOTER_FULL_FROM;
    let buttons_y = r.bottom().saturating_sub(2);
    draw_buttons(app, buf, r, buttons_y, t);
    let foot_rows = if full_foot { 3 } else { 1 };
    let list_end = r.bottom().saturating_sub(1 + foot_rows);
    if full_foot {
        draw_version(app, buf, r, buttons_y - 2, t);
    }

    // The search field, then the FLEETS divider.
    let airy = r.height >= AIRY_FROM;
    let filter_y = r.y + if airy { 2 } else { 1 };
    draw_filter(app, buf, r, filter_y, t);
    let fleets_div = r.y + if airy { FLEETS_DIVIDER } else { 2 };
    let last = list_end.saturating_sub(1);
    let mid = (fleets_div + last) / 2;
    let model_now = SideModel::build(app, model);
    let lists = model_now.lists(app, false);
    divider(app, buf, r, fleets_div, "FLEETS", border, t);
    divider(
        app,
        buf,
        r,
        mid,
        &format!("SHIPS {}", model_now.rows.len()),
        border,
        t,
    );
    app.screen.mid_y = mid;
    let items = lists.items();
    app.screen.side_items = items.clone();
    app.screen.visible = model_now.rows.iter().map(|x| x.key).collect();
    app.screen.row_y.clear();
    let lead = u16::from(airy);

    // What the half is kept scrolled to when something moved.
    let want = if app.screen.follow {
        cursor_item(app).filter(|_| side_focused(app)).or_else(|| {
            app.focused().map(|id| SideItem::Sess(app.session_key(id)))
        })
    } else {
        None
    };
    let fleets_top = fleets_div + 1 + lead;
    draw_top(
        app,
        buf,
        (r, x0, end),
        (fleets_top, mid),
        &model_now,
        &lists,
        want.clone(),
        t,
    );
    let ships_top = mid + 1 + lead;
    draw_ships(
        app,
        buf,
        (r, x0, end),
        (ships_top, list_end),
        &model_now,
        &lists,
        want,
        t,
    );
    app.screen.follow = false;
}

/// The search field: a filled bar, a key chip at its right end.
fn draw_filter(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, t: &Theme) {
    let bar = Rect {
        x: r.x + 2,
        y,
        width: r.width.saturating_sub(4),
        height: 1,
    };
    let typing = app.screen.filtering;
    let typed = app.screen.filter.clone();
    let ground = if typing { t.sel_bg() } else { t.btn };
    fill(buf, bar, ground);
    let base = Style::default().bg(ground);
    let key = if typed.is_empty() { " / " } else { " x " };
    let key_w = key.width() as u16;
    let text_end = bar.right().saturating_sub(key_w + 1);
    let mut segs = Vec::new();
    if typing {
        segs.push(seg("▌", base.fg(t.accent)));
    } else {
        segs.push(seg(" ", base));
    }
    segs.push(seg(format!("{FILTER_ICON} "), base.fg(t.muted)));
    if typed.is_empty() && !typing {
        segs.push(seg(FILTER_HINT, base.fg(t.muted)));
    } else {
        segs.push(seg(typed, base.fg(t.strong)));
        if typing {
            segs.push(seg("▏", base.fg(t.accent)));
        }
    }
    put(buf, bar.x, y, &segs, text_end);
    let key_x = bar.right().saturating_sub(key_w);
    put(
        buf,
        key_x,
        y,
        &[seg(
            key,
            Style::default()
                .fg(t.accent)
                .bg(t.card)
                .add_modifier(Modifier::BOLD),
        )],
        bar.right(),
    );
    hit(app, bar, ScreenHit::SideFilter);
    if key == " x " {
        hit(
            app,
            Rect {
                x: key_x,
                y,
                width: key_w,
                height: 1,
            },
            ScreenHit::ClearFilter,
        );
    }
}

/// The index of `item` among the rows of a list.
fn position<T>(rows: &[(T, SideItem)], item: Option<&SideItem>) -> Option<usize> {
    let item = item?;
    rows.iter().position(|(_, i)| i == item)
}

#[allow(clippy::too_many_arguments)]
fn draw_top(
    app: &mut App,
    buf: &mut Buffer,
    (r, x0, end): (Rect, u16, u16),
    (top, bottom): (u16, u16),
    sm: &SideModel,
    lists: &Lists,
    want: Option<SideItem>,
    t: &Theme,
) {
    let room = bottom.saturating_sub(top);
    if room == 0 {
        return;
    }
    if lists.top.is_empty() {
        let msg = if sm.filtering { "no match" } else { "no fleets yet" };
        put(
            buf,
            x0,
            top,
            &[seg(msg, Style::default().fg(t.muted).bg(t.bg))],
            end,
        );
        return;
    }
    let heights = vec![1u16; lists.top.len()];
    let want_at = position(&lists.top, want.as_ref());
    let (first, shown) = window(
        &heights,
        app.screen.side_scroll as usize,
        want_at,
        room,
    );
    app.screen.side_scroll = first as u16;
    let picked = side_focused(app).then(|| cursor_item(app)).flatten();
    let current = app.current_fleet();
    for (n, (row, item)) in lists.top.iter().enumerate().skip(first).take(shown) {
        let y = top + (n - first) as u16;
        let idx = n;
        let is_picked = picked.as_ref() == Some(item);
        match *row {
            TopRow::Machine(gi) => {
                draw_machine(app, buf, (r, x0, end, y), sm, gi, is_picked, idx, t)
            }
            TopRow::Fleet(gi, fi) => {
                let f = sm.fleet(gi, fi);
                let down = sm.groups[gi].down;
                draw_fleet(
                    app,
                    buf,
                    (r, x0, end, y),
                    (f, sm.counts_of(f.id), down),
                    (is_picked, current == Some(f.id)),
                    idx,
                    t,
                );
            }
        }
    }
    scroll_marks(
        buf,
        end,
        (top, room),
        (first > 0, first + shown < lists.top.len()),
        Style::default().bg(t.bg).fg(t.muted),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_machine(
    app: &mut App,
    buf: &mut Buffer,
    (r, x0, end, y): (Rect, u16, u16, u16),
    sm: &SideModel,
    gi: usize,
    picked: bool,
    idx: usize,
    t: &Theme,
) {
    let g = &sm.groups[gi];
    let ground = if picked { t.sel_bg() } else { t.bg };
    let row = Rect {
        x: r.x + 1,
        y,
        width: r.width - 2,
        height: 1,
    };
    fill(buf, row, ground);
    let base = Style::default().bg(ground);
    let folded = sm.folded(app, &g.fold_key());
    let local = g.id == MachineId::LOCAL;
    put(
        buf,
        r.x + ARROW_X,
        y,
        &[seg(if folded { FOLDED } else { OPEN }, base.fg(t.muted))],
        r.x + ARROW_X + 1,
    );
    put(
        buf,
        r.x + ICON_X,
        y,
        &[seg(
            if local { LOCAL_GLYPH } else { REMOTE_GLYPH },
            base.fg(t.muted),
        )],
        r.x + ICON_X + 1,
    );
    // The dot and the + at the right; a name that does not fit is cut short before them.
    let plus_x = end.saturating_sub(1);
    let dot_x = end.saturating_sub(4);
    let name_room = dot_x.saturating_sub(r.x + NAME_X + 1) as usize;
    let mut label = truncate(&g.label, name_room.max(1));
    if folded {
        label.push_str(&format!("  {}", g.fleets.len()));
    }
    put(
        buf,
        r.x + NAME_X,
        y,
        &[seg(
            truncate(&label, name_room.max(1)),
            base.fg(if g.down { t.idle } else { t.muted }),
        )],
        dot_x,
    );
    let (dot, colour) = if g.down {
        (RECONNECTING_MARK, t.idle)
    } else {
        (ONLINE_DOT, t.done)
    };
    put(buf, dot_x, y, &[seg(dot, base.fg(colour))], dot_x + 1);
    put(
        buf,
        plus_x,
        y,
        &[seg("+", base.fg(t.accent).add_modifier(Modifier::BOLD))],
        plus_x + 1,
    );
    let _ = x0;
    hit(app, row, ScreenHit::SideRow(idx));
    hit(
        app,
        Rect {
            x: plus_x.saturating_sub(1),
            y,
            width: 3,
            height: 1,
        },
        ScreenHit::MachinePlus(g.id),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_fleet(
    app: &mut App,
    buf: &mut Buffer,
    (r, _x0, end, y): (Rect, u16, u16, u16),
    (f, counts, down): (&Fleet, Counts, bool),
    (picked, current): (bool, bool),
    idx: usize,
    t: &Theme,
) {
    let ground = if picked {
        t.sel_bg()
    } else if current {
        t.card
    } else {
        t.bg
    };
    let row = Rect {
        x: r.x + 1,
        y,
        width: r.width - 2,
        height: 1,
    };
    fill(buf, row, ground);
    let base = Style::default().bg(ground);
    if current {
        put(
            buf,
            r.x + BAR_X,
            y,
            &[seg("▌", base.fg(t.accent))],
            r.x + BAR_X + 1,
        );
    }
    let icon = if counts.needs > 0 {
        seg(
            counts.needs.min(9).to_string(),
            base.fg(t.blocked).add_modifier(Modifier::BOLD),
        )
    } else if down {
        seg(RECONNECTING_MARK, base.fg(t.idle))
    } else {
        seg(DIAMOND, base.fg(f.colour.unwrap_or(t.muted)))
    };
    put(buf, r.x + ICON_X, y, &[icon], r.x + ICON_X + 1);
    // The counts at the right, when there is room for them beside the name.
    let mut tail: Vec<Seg> = Vec::new();
    if counts.agents > 0 {
        tail.push(seg(AGENTS, base.fg(t.accent)));
        tail.push(seg(format!(" {}", counts.agents), base.fg(t.muted)));
    }
    if counts.shells > 0 {
        if !tail.is_empty() {
            tail.push(seg("   ", base));
        }
        tail.push(seg(SHELLS, base.fg(t.muted)));
        tail.push(seg(format!(" {}", counts.shells), base.fg(t.muted)));
    }
    let tail_w = segs_width(&tail);
    let name_x = r.x + NAME_X;
    let name_w = f.name.width() as u16;
    let fits = tail_w > 0 && name_x + name_w + 2 + tail_w <= end;
    let name_room = if fits {
        end.saturating_sub(name_x + tail_w + 2)
    } else {
        end.saturating_sub(name_x)
    } as usize;
    let ink = if down {
        t.idle
    } else if current {
        t.strong
    } else {
        t.text
    };
    put(
        buf,
        name_x,
        y,
        &[seg(
            truncate(&f.name, name_room.max(1)),
            base.fg(ink).add_modifier(Modifier::BOLD),
        )],
        end,
    );
    if fits {
        put(buf, end.saturating_sub(tail_w), y, &tail, end);
    }
    hit(app, row, ScreenHit::SideRow(idx));
}

#[allow(clippy::too_many_arguments)]
fn draw_ships(
    app: &mut App,
    buf: &mut Buffer,
    (r, x0, end): (Rect, u16, u16),
    (top, bottom): (u16, u16),
    sm: &SideModel,
    lists: &Lists,
    want: Option<SideItem>,
    t: &Theme,
) {
    let room = bottom.saturating_sub(top);
    if room == 0 {
        return;
    }
    let offset = lists.top.len();
    if lists.ships.is_empty() {
        let msg = if sm.filtering { "no match" } else { "no agents yet" };
        put(
            buf,
            x0,
            top,
            &[seg(msg, Style::default().fg(t.muted).bg(t.bg))],
            end,
        );
        return;
    }
    let heights: Vec<u16> = lists
        .ships
        .iter()
        .map(|(l, _)| if matches!(l, ShipLine::Ship(_)) { SHIP_H } else { 1 })
        .collect();
    let want_at = position(&lists.ships, want.as_ref());
    let (first, shown) = window(&heights, app.screen.ships_scroll as usize, want_at, room);
    app.screen.ships_scroll = first as u16;
    let picked = side_focused(app).then(|| cursor_item(app)).flatten();
    let open = open_terms(app);
    let keys: Vec<ClientSession> = sm.rows.iter().map(|x| x.key).collect();
    note_new_rows(app, &keys);
    let mut y = top;
    for (n, (line, item)) in lists.ships.iter().enumerate().skip(first).take(shown) {
        let is_picked = picked.as_ref() == Some(item);
        match *line {
            ShipLine::Group(gi, fi) => {
                draw_group(
                    app,
                    buf,
                    (r, end, y),
                    sm,
                    (gi, fi),
                    is_picked,
                    offset + n,
                    t,
                );
                y += 1;
            }
            ShipLine::Ship(i) => {
                let row = &sm.rows[i];
                let is_open = row.term.is_some_and(|t| open.contains(&t));
                let ground = if is_picked {
                    t.sel_bg()
                } else if is_open {
                    t.card
                } else {
                    t.bg
                };
                let lit = Rect {
                    x: r.x + 1,
                    y,
                    width: r.width - 2,
                    height: SHIP_H,
                };
                fill(buf, lit, ground);
                let flashing = app
                    .screen
                    .flash
                    .get(&row.key)
                    .is_some_and(|at| at.elapsed() < FLASH);
                if is_open || flashing {
                    for dy in 0..SHIP_H {
                        put(
                            buf,
                            r.x + BAR_X,
                            y + dy,
                            &[seg("▌", Style::default().fg(t.accent).bg(ground))],
                            r.x + BAR_X + 1,
                        );
                    }
                }
                draw_ship(app, buf, row, (r.x, end, y), ground, t);
                if let Some(term) = row.term {
                    app.screen.row_y.insert(term, y);
                }
                hit(app, lit, ScreenHit::Session(row.key));
                y += SHIP_H;
            }
        }
    }
    scroll_marks(
        buf,
        end,
        (top, room),
        (first > 0, first + shown < lists.ships.len()),
        Style::default().bg(t.bg).fg(t.muted),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_group(
    app: &mut App,
    buf: &mut Buffer,
    (r, end, y): (Rect, u16, u16),
    sm: &SideModel,
    (gi, fi): (usize, usize),
    picked: bool,
    idx: usize,
    t: &Theme,
) {
    let g = &sm.groups[gi];
    let f = sm.fleet(gi, fi);
    let key = f.fold_key(&g.key);
    let folded = sm.folded(app, &key);
    let ground = if picked { t.sel_bg() } else { t.bg };
    let row = Rect {
        x: r.x + 1,
        y,
        width: r.width - 2,
        height: 1,
    };
    fill(buf, row, ground);
    let base = Style::default().bg(ground);
    put(
        buf,
        r.x + ARROW_X,
        y,
        &[seg(if folded { FOLDED } else { OPEN }, base.fg(t.muted))],
        r.x + ARROW_X + 1,
    );
    put(
        buf,
        r.x + ICON_X,
        y,
        &[seg(DIAMOND, base.fg(f.colour.unwrap_or(t.muted)))],
        r.x + ICON_X + 1,
    );
    let mut segs = vec![seg(f.name.clone(), base.fg(t.muted))];
    if g.id != MachineId::LOCAL {
        segs.push(seg(format!(" · {}", g.label), base.fg(t.idle)));
    }
    if folded {
        let c = sm.counts_of(f.id);
        segs.push(seg(format!("  {}", c.agents), base.fg(t.muted)));
        if c.needs > 0 {
            segs.push(seg(
                format!("  {} need you", c.needs),
                base.fg(t.blocked),
            ));
        }
    }
    put(buf, r.x + NAME_X, y, &segs, end);
    hit(app, row, ScreenHit::SideRow(idx));
}

/// Remember which rows exist; a row new since the last frame flashes its bar. The first
/// frame with sessions flashes nothing: they were not just made.
fn note_new_rows(app: &mut App, keys: &[ClientSession]) {
    let moving = app.motion_on();
    let known = app
        .screen
        .rows_known
        .get_or_insert_with(|| keys.iter().copied().collect());
    let fresh: Vec<ClientSession> = keys.iter().filter(|k| known.insert(**k)).copied().collect();
    if moving {
        for k in fresh {
            app.screen.flash.insert(k, Instant::now());
        }
    }
    app.screen.flash.retain(|_, at| at.elapsed() < FLASH);
}

/// One agent ship's two lines.
fn draw_ship(
    app: &App,
    buf: &mut Buffer,
    row: &FlatRow,
    (left, end, y): (u16, u16, u16),
    ground: Color,
    t: &Theme,
) {
    let unknown = row.state == State::Unknown;
    let mark = app.state_mark(row.state, t);
    let dimmed = |c: Color| if unknown { t.idle } else { c };
    let base = Style::default().bg(ground);
    let mark_style = base
        .fg(mark.color)
        .add_modifier(if row.state == State::Working {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    put(buf, left + ICON_X, y, &[seg(mark.glyph, mark_style)], left + ICON_X + 1);

    // Line 1: the name, its marks, and the view mark (or the resume offer) at the right.
    let nx = left + NAME_X;
    let right: Vec<Seg> = if row.resume {
        vec![seg(RESUME, base.fg(dimmed(t.accent)))]
    } else {
        vec![seg(
            if row.chat { VIEW_CHAT } else { VIEW_TERMINAL },
            base.fg(dimmed(t.muted)),
        )]
    };
    let rw = segs_width(&right);
    let marks_w = u16::from(row.unread) * 2 + u16::from(row.pinned) * 2;
    let name_room = end.saturating_sub(nx).saturating_sub(rw + 1 + marks_w) as usize;
    let name = truncate(&row.name, name_room.max(1));
    let name_color = if row.state == State::Idle {
        t.muted
    } else {
        t.strong
    };
    let name_style = base.fg(dimmed(name_color)).add_modifier(Modifier::BOLD);
    let mut x = put(buf, nx, y, &[seg(name, name_style)], end);
    if row.unread {
        x = put(
            buf,
            x,
            y,
            &[seg(format!(" {UNREAD}"), base.fg(dimmed(t.accent)))],
            end,
        );
    }
    if row.pinned {
        put(
            buf,
            x,
            y,
            &[seg(format!(" {PIN}"), base.fg(dimmed(t.muted)))],
            end,
        );
    }
    put(buf, end.saturating_sub(rw), y, &right, end);

    // Line 2: the branch in magenta, `worktree` dim at the right; a folder when there is no repo.
    let y2 = y + 1;
    let tag = if row.worktree { "worktree" } else { "" };
    let tag_w = tag.width() as u16;
    let room = end
        .saturating_sub(nx)
        .saturating_sub(if tag_w > 0 { tag_w + 1 } else { 0 }) as usize;
    let (text, colour) = if row.branch.is_empty() {
        (row.project.clone(), t.muted)
    } else {
        (format!("⎇ {}", row.branch), t.violet())
    };
    put(
        buf,
        nx,
        y2,
        &[seg(truncate(&text, room.max(1)), base.fg(dimmed(colour)))],
        end,
    );
    if tag_w > 0 {
        put(
            buf,
            end.saturating_sub(tag_w),
            y2,
            &[seg(tag, base.fg(t.idle))],
            end,
        );
    }
}

/// The footer's version line: the update marker when one is on offer, else the version, dim.
fn draw_version(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, t: &Theme) {
    let (x, end) = (r.x + 2, r.right().saturating_sub(2));
    let room = end.saturating_sub(x);
    let marker = [mode_bar::Release::Words, mode_bar::Release::Version]
        .into_iter()
        .find_map(|shown| {
            let (segs, h) = mode_bar::release_segs(app, shown, t)?;
            (segs_width(&segs) <= room).then_some((segs, h))
        });
    app.screen.release_in_side = marker.is_some();
    match marker {
        Some((segs, h)) => {
            let w = segs_width(&segs);
            put(buf, x, y, &segs, end);
            hit(
                app,
                Rect {
                    x,
                    y,
                    width: w,
                    height: 1,
                },
                h,
            );
        }
        None => {
            put(
                buf,
                x,
                y,
                &[seg(
                    format!("{} {}", crate::brand::APP_NAME, crate::brand::APP_VERSION),
                    Style::default().fg(t.idle).bg(t.bg),
                )],
                end,
            );
        }
    }
}

/// The rule over the buttons, and ` + New ` and ` Menu `.
fn draw_buttons(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, t: &Theme) {
    let (x, end) = (r.x + 2, r.right().saturating_sub(2));
    if r.height >= 6 {
        for xx in r.x + 1..r.right() - 1 {
            put(
                buf,
                xx,
                y - 1,
                &[seg("─", Style::default().fg(t.line).bg(t.bg))],
                xx + 1,
            );
        }
    }
    let new = chip(app, "+ New", Chip::Action, t.bg, t);
    let menu = chip(app, "Menu", Chip::Quiet, t.bg, t);
    let (nw, mw) = (segs_width(&new), segs_width(&menu));
    if x + nw > end {
        return;
    }
    put(buf, x, y, &new, end);
    hit(
        app,
        Rect {
            x,
            y,
            width: nw,
            height: 1,
        },
        ScreenHit::NewShell,
    );
    let mx = end.saturating_sub(mw);
    if mx > x + nw {
        put(buf, mx, y, &menu, end);
        hit(
            app,
            Rect {
                x: mx,
                y,
                width: mw,
                height: 1,
            },
            ScreenHit::Actions,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> FlatRow {
        FlatRow {
            key: ClientSession::local(SessionKey::Pane(1)),
            term: None,
            name: "Growth Tests".into(),
            project: "AeVoxGrowth".into(),
            place: Where::Remote("devbox".into()),
            state: State::Idle,
            unread: false,
            pinned: false,
            resume: false,
            process: "claude".into(),
            fleet: None,
            agent: true,
            branch: String::new(),
            worktree: false,
            chat: false,
            order: (1, MachineId::LOCAL),
        }
    }

    #[test]
    fn the_filter_matches_name_project_process_and_machine() {
        let row = row();
        for q in ["", "growth t", "AEVOX", "cla", "devbox", "  devbox "] {
            assert!(row.matches(q), "{q:?}");
        }
        assert!(!row.matches("pacman"));
    }

    #[test]
    fn where_reads_as_the_design_spells_it() {
        assert_eq!(Where::Local.text(), "local");
        assert_eq!(Where::Remote("devbox".into()).text(), "⇄ devbox");
        assert_eq!(Where::Reconnecting("gpu-01".into()).text(), "┆ gpu-01");
    }

    #[test]
    fn a_list_scrolls_to_keep_the_wanted_row_in_view_and_never_past_its_end() {
        let ones = [1u16; 10];
        assert_eq!(window(&ones, 0, None, 4), (0, 4));
        assert_eq!(window(&ones, 0, Some(6), 4), (3, 4), "the row lands at the bottom");
        assert_eq!(window(&ones, 8, None, 4), (6, 4), "no empty space under the end");
        let twos = [2u16; 5];
        assert_eq!(window(&twos, 0, Some(3), 5), (2, 2), "only whole rows are shown");
    }
}
