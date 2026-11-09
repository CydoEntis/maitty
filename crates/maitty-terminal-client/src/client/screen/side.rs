//! The sidebar: every machine with its sessions under it, agents and shells alike. A session is
//! two lines (its status mark and name, then its project's colour square, the project and the
//! branch); an agent's subagents follow it, as a row of marks or, for the session you are on,
//! one row each. `SideModel` is the one list the drawing, the arrow keys and Enter all read.

use super::chips::{Chip, chip};
use super::status_mark::State;
use super::*;
use crate::client::ids::ClientSession;
use crate::protocol::{SubagentActivity, SubagentState, TurnKind};

/// Rows a session takes before its subagents.
const SESSION_H: u16 = 2;
/// Columns from the sidebar's left edge: the selection bar, the status mark, the name, and the
/// subagents' tree.
const BAR_X: u16 = 1;
const MARK_X: u16 = 2;
const NAME_X: u16 = 4;
const TREE_X: u16 = 6;
const ONLINE_DOT: &str = "●";
const RECONNECTING_MARK: &str = "┆";
/// What the machine heading calls the computer the window runs on.
const THIS_COMPUTER: &str = "This computer";
/// The sidebar's title chip, and the chips that hide it and bring it back.
const TITLE: &str = "Maitty";
pub(in crate::client) const HIDE: &str = "«";
pub(in crate::client) const SHOW: &str = "»";
/// The fold arrows of a heading: open (it folds), and folded (it opens).
const OPEN: &str = "▾";
const FOLDED: &str = "▸";

/// The collapsed sidebar's width: its frame, a margin and a mark with an initial.
pub(in crate::client) const RAIL_W: u16 = 7;
/// The chip that opens the Add Machine dialog.
/// The footer's menu chip.
const MENU: &str = "Menu";

/// A subagent as its parent's row lists it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SubRow {
    pub name: String,
    pub state: State,
}

/// A session as the sidebar lists it.
#[derive(Debug, Clone)]
pub(super) struct SideRow {
    pub key: ClientSession,
    pub term: Option<ClientTerm>,
    /// Your name for it, else what runs in it.
    pub name: String,
    pub project: String,
    pub branch: String,
    pub state: State,
    /// An agent (its name is bold while it has something going on), not a shell.
    pub agent: bool,
    pub process: String,
    pub machine: String,
    pub subs: Vec<SubRow>,
    /// Folded away: its machine or its folder is closed, so it is not drawn and the cursor skips it.
    pub hidden: bool,
    /// Creation order: ids only count up.
    order: (u64, MachineId),
}

impl SideRow {
    fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        q.is_empty()
            || [&self.name, &self.project, &self.process, &self.machine, &self.branch]
                .iter()
                .any(|field| field.to_lowercase().contains(&q))
    }

    /// Bold, in the strong colour: an agent that is doing or asking something.
    fn busy(&self) -> bool {
        self.agent && self.state != State::Idle
    }
}

/// A machine and the sessions on it.
#[derive(Debug, Clone)]
pub(super) struct MachineSection {
    pub id: MachineId,
    pub label: String,
    pub down: bool,
    /// The app there is missing or another version: the heading offers the fix.
    pub fix: Option<super::machine_group::AppFix>,
    /// Its heading is folded: the sessions under it are hidden.
    pub folded: bool,
    pub rows: Vec<SideRow>,
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

/// A subagent's state as its mark draws it.
pub(super) fn sub_state(s: SubagentState) -> State {
    match s {
        SubagentState::Starting | SubagentState::Working => State::Working,
        SubagentState::WaitingForInput | SubagentState::AwaitingApproval => State::Needs,
        SubagentState::Completed => State::Done,
        SubagentState::Failed | SubagentState::ConnectionLost => State::Problem,
        SubagentState::Cancelled => State::Idle,
    }
}

/// When the agent's current turn began: the first `Working` after its last `Done` or `Idle`,
/// else when its status last changed.
fn turn_start(info: &TermInfo) -> u64 {
    let ended = info
        .timeline
        .iter()
        .rposition(|e| matches!(e.kind, TurnKind::Done | TurnKind::Idle));
    info.timeline
        .iter()
        .skip(ended.map_or(0, |i| i + 1))
        .find(|e| e.kind == TurnKind::Working)
        .map_or(info.since, |e| e.at)
}

/// The subagents of the turn the agent is in: none once the turn is over, since the daemon keeps
/// every record the session ever reported.
pub(super) fn turn_subagents(info: &TermInfo) -> Vec<&SubagentActivity> {
    if !matches!(info.status, Status::Working | Status::Blocked) {
        return Vec::new();
    }
    let from = turn_start(info);
    info.subagent_activity
        .iter()
        .filter(|a| a.state.active() || a.started_at.unwrap_or(a.updated_at) >= from)
        .collect()
}

/// Every session in creation order.
fn side_rows(app: &App, model: &[Proj]) -> Vec<SideRow> {
    let mut rows: Vec<SideRow> = model
        .iter()
        .flat_map(|p| p.sessions().map(move |s| (p, s)))
        .map(|(p, s)| {
            let info = s.native_term.and_then(|t| app.term_info(t));
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
            let subs = info
                .filter(|_| app.cfg.ui.subagent_count)
                .map(|i| {
                    turn_subagents(i)
                        .into_iter()
                        .map(|a| SubRow {
                            name: a.name.clone(),
                            state: sub_state(a.state),
                        })
                        .collect()
                })
                .unwrap_or_default();
            SideRow {
                key: s.key,
                term: s.native_term,
                name,
                project: p.name.clone(),
                branch: info.and_then(|i| i.branch.clone()).unwrap_or_default(),
                state: State::of(s.status, failed, s.down),
                agent: s.is_agent,
                process: info.map_or(String::new(), |i| i.process.clone()),
                machine: app.machine_label_of(s.key.machine),
                subs,
                hidden: false,
                order: (
                    s.native_term.map_or(u64::MAX, |t| u64::from(t.term)),
                    s.key.machine,
                ),
            }
        })
        .collect();
    // Agents first, then shells; each in the order they started, so a row never moves when its
    // status changes.
    rows.sort_by_key(|r| (!r.agent, r.order, r.key));
    rows
}

/// What the sidebar shows once the filter has had its say.
pub(super) struct SideModel {
    pub machines: Vec<MachineSection>,
    pub filtering: bool,
    /// Each machine's sessions are kept together by folder, under a heading for each.
    pub grouped: bool,
}

/// `rows` with the sessions of one folder together, folders in the order they first appear and
/// each folder's sessions in theirs.
fn by_folder(rows: &mut [SideRow]) {
    let mut seen: Vec<String> = Vec::new();
    for r in rows.iter() {
        if !seen.contains(&r.project) {
            seen.push(r.project.clone());
        }
    }
    rows.sort_by_key(|r| seen.iter().position(|p| *p == r.project));
}

impl SideModel {
    pub fn build(app: &App, model: &[Proj]) -> SideModel {
        let query = app.screen.filter.trim().to_lowercase();
        let all = side_rows(app, model);
        let grouped = app.screen.saved.group_folders;
        let machines = app
            .machines()
            .into_iter()
            .map(|m| MachineSection {
                id: m.id,
                label: if m.id == MachineId::LOCAL {
                    THIS_COMPUTER.to_string()
                } else {
                    app.machine_label_of(m.id)
                },
                down: app.machine_down(m.id),
                fix: app.machine_fix(m.id),
                folded: false,
                rows: all
                    .iter()
                    .filter(|r| r.key.machine == m.id && r.matches(&query))
                    .cloned()
                    .collect(),
            })
            .filter(|m| query.is_empty() || !m.rows.is_empty())
            .map(|mut m| {
                if grouped {
                    by_folder(&mut m.rows);
                }
                // A closed machine or folder hides its sessions; a filter shows every match.
                if query.is_empty() {
                    let machine_closed = app.screen.saved.closed.contains(&machine_fold_key(&m.label));
                    m.folded = machine_closed;
                    for r in &mut m.rows {
                        r.hidden = machine_closed || (grouped && app.screen.saved.closed.contains(&folder_fold_key(&m.label, &r.project)));
                    }
                }
                m
            })
            .collect();
        SideModel {
            machines,
            filtering: !query.is_empty(),
            grouped,
        }
    }

    /// Every row the cursor can rest on, top to bottom.
    pub fn items(&self) -> Vec<SideItem> {
        self.machines
            .iter()
            .flat_map(|m| m.rows.iter().filter(|r| !r.hidden))
            .map(|r| SideItem::Sess(r.key))
            .collect()
    }
}

/// The sessions as the sidebar lists them, top to bottom.
pub(in crate::client) fn side_keys(app: &App, model: &[Proj]) -> Vec<ClientSession> {
    SideModel::build(app, model)
        .machines
        .iter()
        .flat_map(|m| m.rows.iter().filter(|r| !r.hidden).map(|r| r.key))
        .collect()
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

/// One block of the list: a machine heading, a session with its subagents, or the gap between
/// two machines.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Block {
    Machine(usize),
    /// A folder's heading (grouped by folder): the machine, and the first of its sessions.
    Folder(usize, usize),
    /// The line between one folder's sessions and the next folder's heading.
    Divider,
    Session(usize, usize),
    /// The `Remote machines` row with its `+` that adds one; above the first remote machine, or at the end when there is none.
    Gap,
}

/// The session open in the workspace, which shows its subagents one per row.
fn open_session(app: &App) -> Option<ClientSession> {
    app.focused().map(|id| app.session_key(id))
}

fn block_h(sm: &SideModel, b: Block, open: Option<ClientSession>) -> u16 {
    match b {
        Block::Machine(_) | Block::Folder(..) | Block::Divider | Block::Gap => 1,
        Block::Session(mi, ri) => {
            let r = &sm.machines[mi].rows[ri];
            let subs = match r.subs.len() {
                0 => 0,
                n if Some(r.key) == open => n as u16,
                _ => 1,
            };
            SESSION_H + subs
        }
    }
}

/// `word` with its first letter in capitals, for a label.
fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

fn blocks(sm: &SideModel) -> Vec<Block> {
    let mut out = Vec::new();
    for (mi, m) in sm.machines.iter().enumerate() {
        if mi > 0 {
            out.push(Block::Gap);
        }
        out.push(Block::Machine(mi));
        for ri in 0..m.rows.len() {
            if m.folded {
                break;
            }
            if sm.grouped && (ri == 0 || m.rows[ri].project != m.rows[ri - 1].project) {
                if ri > 0 {
                    out.push(Block::Divider);
                }
                out.push(Block::Folder(mi, ri));
            }
            if !m.rows[ri].hidden {
                out.push(Block::Session(mi, ri));
            }
        }
    }
    out
}

/// The colour of row `ri`'s folder: folders take the theme's pane colours in turn, so two folders
/// next to each other never share one.
fn folder_colour(rows: &[SideRow], ri: usize, t: &Theme) -> Color {
    let order = (1..=ri).filter(|&i| rows[i].project != rows[i - 1].project).count();
    t.grid_colors()[order % 4]
}

/// The bar down the left of a folder's heading and sessions, `h` rows from `y`.
fn put_folder_bar(buf: &mut Buffer, r: Rect, y: u16, h: u16, colour: Color, t: &Theme) {
    for dy in 0..h {
        put(buf, r.x + 1, y + dy, &[seg("▏", Style::default().fg(colour).bg(t.bg))], r.x + 2);
    }
}

/// Columns a nested heading or row sits to the right of the one above it.
const NEST: u16 = 2;

/// `r` moved `by` columns to the right, for a level that sits under another.
fn nested(r: Rect, by: u16) -> Rect {
    Rect { x: r.x + by, width: r.width.saturating_sub(by), ..r }
}

/// The key a machine's fold is kept under in the saved state.
pub(super) fn machine_fold_key(label: &str) -> String {
    format!("m:{label}")
}

/// The key a folder's fold is kept under: its machine and its name.
pub(super) fn folder_fold_key(machine: &str, folder: &str) -> String {
    format!("f:{machine}/{folder}")
}

/// The sidebar: the frame with its chips, the filter field and the machines' sessions.
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
    let inside = card(app, buf, r, &c, t);
    fill(buf, inside, t.bg);
    app.screen.side_rect = r;
    hit(app, r, ScreenHit::SideFocus);
    if r.width < 12 || r.height < 5 {
        return;
    }
    draw_top_chips(app, buf, r, t);
    draw_foot_chips(app, buf, r, t);
    let sm = SideModel::build(app, model);
    let open = open_session(app);
    app.screen.side_items = sm.items();
    app.screen.visible = sm
        .machines
        .iter()
        .flat_map(|m| m.rows.iter().filter(|x| !x.hidden).map(|x| x.key))
        .collect();
    app.screen.row_y.clear();
    app.screen.sub_rows.clear();
    app.screen.fold_keys.clear();
    // Every session the panes of this view show, so a split's other panes are marked in the list as well as the focused one.
    app.screen.in_view = app.panes.iter().map(|(term, _)| app.session_key(*term)).collect();
    let top = r.y + 2;
    let bottom = r.bottom().saturating_sub(1);
    draw_list(app, buf, r, (top, bottom), &sm, open, t);
    app.screen.follow = false;
}

/// The sidebar collapsed to a narrow column: `»` (`«` on the right) opens it again, then a row
/// for each session with its status mark and the first letter of its name. A click opens it.
pub(in crate::client) fn draw_rail(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    let inside = card(app, buf, r, &Card::new(t, ""), t);
    fill(buf, inside, t.bg);
    if r.width < 5 || r.height < 5 {
        return;
    }
    let right = app.cfg.ui.sidebar_position == "right";
    let expand = chip(app, if right { HIDE } else { SHOW }, Chip::Title { focused: false }, t.bg, t);
    let ew = segs_width(&expand);
    let ex = r.x + r.width.saturating_sub(ew) / 2;
    put(buf, ex, r.y, &expand, r.right());
    hit(app, Rect { x: ex, y: r.y, width: ew, height: 1 }, ScreenHit::ExpandSidebar);
    let open = open_session(app);
    let sm = SideModel::build(app, model);
    let mut y = r.y + 2;
    // A blank row between sessions, as in the full sidebar.
    for row in sm.machines.iter().flat_map(|m| &m.rows) {
        if y + 1 >= r.bottom() {
            break;
        }
        let ground = if Some(row.key) == open { t.hov } else { t.bg };
        let line = Rect { x: r.x + 1, y, width: r.width - 2, height: 1 };
        fill(buf, line, ground);
        let base = Style::default().bg(ground);
        let mark = app.state_mark(row.state, t);
        let initial = row.name.chars().next().map(String::from).unwrap_or_default();
        put(
            buf,
            line.x + 1,
            y,
            &[
                seg(mark.glyph, base.fg(mark.color).add_modifier(Modifier::BOLD)),
                seg(" ", base),
                seg(initial, base.fg(t.text)),
            ],
            line.right(),
        );
        hit(app, line, ScreenHit::Session(row.key));
        y += 2;
    }
}

/// The top border: the `maitty` chip at the left and `«` (hide the sidebar) at the right.
fn draw_top_chips(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme) {
    let x = r.x + 2;
    let title = chip(app, TITLE, Chip::Title { focused: false }, t.bg, t);
    let tw = segs_width(&title);
    put(buf, x, r.y, &title, r.right().saturating_sub(2));
    hit(app, Rect { x, y: r.y, width: tw, height: 1 }, ScreenHit::CollapseSidebar);
    let hide = chip(app, HIDE, Chip::Title { focused: false }, t.bg, t);
    let hw = segs_width(&hide);
    let hx = r.right().saturating_sub(2 + hw);
    if hx > x + tw {
        put(buf, hx, r.y, &hide, r.right().saturating_sub(2));
        hit(app, Rect { x: hx, y: r.y, width: hw, height: 1 }, ScreenHit::CollapseSidebar);
    }
    // The plus chip opens a small menu: a new session, or a new remote machine.
    let plus = chip(app, "+", Chip::Action, t.bg, t);
    let pw = segs_width(&plus);
    let group_w = segs_width(&chip(app, &format!("{} \u{25be}", app.cfg.icons.filter), Chip::Title { focused: false }, t.bg, t));
    let px = hx.saturating_sub(1 + group_w + 1 + pw);
    if px > x + tw {
        put(buf, px, r.y, &plus, hx);
        hit(app, Rect { x: px, y: r.y, width: pw, height: 1 }, ScreenHit::NewMenu);
        app.screen.new_at = (px, r.y + 1);
    }
    // The group-by chip opens a small menu under it: priority or folder. Dropped when the
    // sidebar is too narrow to hold it.
    let label = format!("{} \u{25be}", app.cfg.icons.filter);
    let group = chip(app, &label, Chip::Title { focused: false }, t.bg, t);
    let gw = segs_width(&group);
    let gx = hx.saturating_sub(1 + gw);
    if gx > x + tw {
        put(buf, gx, r.y, &group, hx);
        hit(app, Rect { x: gx, y: r.y, width: gw, height: 1 }, ScreenHit::GroupFolders);
        app.screen.group_at = (gx, r.y + 1);
    }
}

/// The bottom border: the release chip at the left (the version when nothing waits) and `menu`
/// at the right. Each opens its menu upward.
fn draw_foot_chips(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme) {
    let y = r.bottom().saturating_sub(1);
    let x = r.x + 2;
    let end = r.right().saturating_sub(2);
    let (text, kind) = release_chip(app);
    let release = chip(app, &text, kind, t.bg, t);
    let rw = segs_width(&release);
    put(buf, x, y, &release, end);
    hit(app, Rect { x, y, width: rw, height: 1 }, ScreenHit::ReleaseMenu);
    app.screen.release_in_side = app.update_available.is_some() || app.restart_needed();
    let menu = chip(app, MENU, Chip::Title { focused: false }, t.bg, t);
    let mw = segs_width(&menu);
    let mx = end.saturating_sub(mw);
    if mx > x + rw {
        put(buf, mx, y, &menu, end);
        hit(app, Rect { x: mx, y, width: mw, height: 1 }, ScreenHit::Actions);
    }
}

/// What the release chip says: the version on offer, a restart that waits, or the version this
/// is, quiet.
fn release_chip(app: &App) -> (String, Chip) {
    if let Some(version) = app.update_available.as_ref() {
        (format!("↑ v{}", version.trim_start_matches('v')), Chip::Action)
    } else if app.restart_needed() {
        ("↻ restart".into(), Chip::Action)
    } else {
        (format!("v{}{}", crate::brand::APP_VERSION, build_note()), Chip::Quiet)
    }
}

/// ` abc1234* 12m` for a build that is not a clean release: the commit, `*` for uncommitted
/// changes and how long ago it was built. Empty for a release, and in tests.
fn build_note() -> String {
    let build = env!("APP_BUILD");
    if build.is_empty() || cfg!(test) {
        return String::new();
    }
    let built: u64 = env!("APP_BUILT_UNIX").parse().unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let minutes = now.saturating_sub(built) / 60;
    let age = if minutes < 60 { format!("{minutes}m") } else { format!("{}h", minutes / 60) };
    format!(" {build} {age}")
}

/// The block the cursor, or failing that the open session, sits in.
fn wanted_block(app: &App, sm: &SideModel, list: &[Block]) -> Option<usize> {
    let key = app
        .screen
        .cursor
        .filter(|_| side_focused(app))
        .or_else(|| open_session(app))?;
    list.iter().position(|b| {
        matches!(*b, Block::Session(mi, ri) if sm.machines[mi].rows[ri].key == key)
    })
}

#[allow(clippy::too_many_arguments)]
fn draw_list(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    (top, bottom): (u16, u16),
    sm: &SideModel,
    open: Option<ClientSession>,
    t: &Theme,
) {
    let room = bottom.saturating_sub(top);
    if room == 0 {
        return;
    }
    if sm.machines.is_empty() {
        let msg = if sm.filtering { "no match" } else { "no sessions yet" };
        put(
            buf,
            r.x + NAME_X,
            top,
            &[seg(msg, Style::default().fg(t.muted).bg(t.bg))],
            r.right().saturating_sub(1),
        );
        return;
    }
    let list = blocks(sm);
    let heights: Vec<u16> = list.iter().map(|b| block_h(sm, *b, open)).collect();
    let want = if app.screen.follow || side_focused(app) {
        wanted_block(app, sm, &list)
    } else {
        None
    };
    let (first, shown) = window(&heights, app.screen.side_scroll as usize, want, room);
    app.screen.side_scroll = first as u16;
    let mut y = top;
    for (b, h) in list.iter().zip(&heights).skip(first).take(shown) {
        match *b {
            Block::Machine(mi) => {
                let machine = &sm.machines[mi];
                draw_machine(app, buf, r, y, machine, t);
            }
            Block::Folder(mi, ri) => {
                let machine = &sm.machines[mi];
                let project = &machine.rows[ri].project;
                let folded = !sm.filtering && app.screen.saved.closed.contains(&folder_fold_key(&machine.label, project));
                let shift = NEST;
                let room = r.width.saturating_sub(10 + shift) as usize;
                let name = truncate(project, room);
                let quiet = Style::default().fg(t.muted).bg(t.bg);
                let colour = folder_colour(&machine.rows, ri, t);
                put_folder_bar(buf, r, y, *h, colour, t);
                // The arrow says the folder folds; a click on its name does it.
                put(
                    buf,
                    r.x + 1 + shift,
                    y,
                    &[
                        seg(if folded { FOLDED } else { OPEN }, quiet.fg(t.accent)),
                        seg(format!(" {} ", app.cfg.icons.folder), quiet.fg(colour)),
                        seg(name, quiet.fg(t.text).add_modifier(Modifier::BOLD)),
                    ],
                    r.right().saturating_sub(2),
                );
                app.screen.fold_keys.push(folder_fold_key(&machine.label, project));
                let n = app.screen.fold_keys.len() - 1;
                hit(app, Rect { x: r.x + 1 + shift, y, width: r.width.saturating_sub(3 + shift), height: 1 }, ScreenHit::Fold(n));
            }
            Block::Session(mi, ri) => {
                let machine = &sm.machines[mi];
                let row = &machine.rows[ri];
                // Sessions sit one level in from their folder's heading, or from the machine's when not grouped.
                let by = if sm.grouped { NEST } else { 0 };
                if sm.grouped {
                    put_folder_bar(buf, r, y, *h, folder_colour(&machine.rows, ri, t), t);
                }
                draw_session(app, buf, nested(r, by), y, row, Some(row.key) == open, t);
            }
            Block::Divider => {
                let width = r.width.saturating_sub(4) as usize;
                let line = "─".repeat(width);
                put(buf, r.x + 2, y, &[seg(line, Style::default().fg(t.line).bg(t.bg))], r.right().saturating_sub(2));
            }
            Block::Gap => {}
        }
        y += h;
    }
    scroll_marks(
        buf,
        r.right().saturating_sub(2),
        (top, room),
        (first > 0, first + shown < list.len()),
        Style::default().bg(t.bg).fg(t.muted),
    );
}

/// A machine heading: its name chip with the number of sessions, a `+` chip that starts a shell
/// there at once (` install ` or ` update ` instead while the app there needs it), and the
/// connection dot at the right.
fn draw_machine(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, m: &MachineSection, t: &Theme) {
    let x = r.x + 1;
    let end = r.right().saturating_sub(1);
    hit(app, Rect { x, y, width: end.saturating_sub(x), height: 1 }, ScreenHit::MachineHead(m.id));
    let name = chip(app, &m.label, Chip::Title { focused: false }, t.bg, t);
    // The count sits inside the chip, quieter than the name.
    let mut segs = vec![seg(format!("{} ", if m.folded { FOLDED } else { OPEN }), Style::default().fg(t.accent).bg(t.bg))];
    segs.extend(name);
    let count = Style::default().fg(t.muted).bg(t.btn);
    if let Some(last) = segs.pop() {
        segs.push(seg(format!(" {}", m.rows.len()), count));
        segs.push(last);
    }
    let after = put(buf, x, y, &segs, end.saturating_sub(4));
    // The arrow and the name fold the machine; the rest of the row keeps what a click on it did.
    app.screen.fold_keys.push(machine_fold_key(&m.label));
    let n = app.screen.fold_keys.len() - 1;
    hit(app, Rect { x, y, width: after.saturating_sub(x), height: 1 }, ScreenHit::FoldMachine(n, m.id));
    let (text, act) = match m.fix {
        Some(fix) => (capitalised(fix.verb()), ScreenHit::MachineFix(m.id)),
        None => ("+".to_string(), ScreenHit::MachinePlus(m.id)),
    };
    let plus = chip(app, &text, Chip::Action, t.bg, t);
    let pw = segs_width(&plus);
    let px = after + 1;
    if px + pw < end.saturating_sub(2) {
        put(buf, px, y, &plus, end);
        hit(app, Rect { x: px, y, width: pw, height: 1 }, act);
    }
    let (dot, colour) = if m.down {
        (RECONNECTING_MARK, t.idle)
    } else {
        (ONLINE_DOT, t.done)
    };
    let dx = end.saturating_sub(2);
    put(buf, dx, y, &[seg(dot, Style::default().fg(colour).bg(t.bg))], dx + 1);
}

/// One session's two lines and its subagents.
fn draw_session(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, row: &SideRow, open: bool, t: &Theme) {
    let picked = side_focused(app) && app.screen.cursor == Some(row.key);
    let ground = if picked {
        t.sel_bg()
    } else if open {
        t.hov
    } else {
        t.bg
    };
    let lines = Rect {
        x: r.x + 1,
        y,
        width: r.width.saturating_sub(2),
        height: SESSION_H,
    };
    fill(buf, lines, ground);
    // A lit row is one soft-cornered block when shapes are rounded, as the pointer's tint is.
    if ground != t.bg && lines.width >= 2 && Look::of(&app.cfg.ui).caps {
        let edge = Style::default().fg(ground).bg(t.bg);
        for dy in 0..SESSION_H {
            let (l, e) = block_ends(dy, SESSION_H);
            put(buf, lines.x, y + dy, &[seg(l, edge)], lines.x + 1);
            put(buf, lines.right() - 1, y + dy, &[seg(e, edge)], lines.right());
        }
    }
    let base = Style::default().bg(ground);
    let end = r.right().saturating_sub(2);
    if open {
        for dy in 0..SESSION_H {
            put(buf, r.x + BAR_X, y + dy, &[seg("▎", base.fg(t.accent))], r.x + BAR_X + 1);
        }
    } else if app.screen.in_view.contains(&row.key) {
        // Shown in another pane of this view: the same bar, quieter, so every pane of a split is marked.
        for dy in 0..SESSION_H {
            put(buf, r.x + BAR_X, y + dy, &[seg("▎", base.fg(t.muted))], r.x + BAR_X + 1);
        }
    }
    let unknown = row.state == State::Unknown;
    let mark = app.state_mark(row.state, t);
    put(
        buf,
        r.x + MARK_X,
        y,
        &[seg(mark.glyph, base.fg(mark.color).add_modifier(Modifier::BOLD))],
        // The room before the name: a spinner may be two cells (the Bars frames), and clipping it to one showed two bars, not three.
        r.x + NAME_X,
    );
    let name_colour = if unknown {
        t.idle
    } else if open || row.busy() {
        t.strong
    } else {
        t.text
    };
    let name_style = if row.busy() {
        base.fg(name_colour).add_modifier(Modifier::BOLD)
    } else {
        base.fg(name_colour)
    };
    let room = end.saturating_sub(r.x + NAME_X) as usize;
    put(buf, r.x + NAME_X, y, &[seg(truncate(&row.name, room.max(1)), name_style)], end);

    // Under its folder's heading a session says only its branch; otherwise the folder first.
    let mut second = Vec::new();
    if !app.screen.saved.group_folders {
        second.push(seg(row.project.clone(), base.fg(t.muted)));
    }
    if !row.branch.is_empty() {
        let lead = if second.is_empty() { "" } else { " " };
        second.push(seg(format!("{lead}{}", row.branch), base.fg(t.muted).add_modifier(Modifier::DIM)));
    }
    put_cut(buf, r.x + NAME_X, y + 1, second, end);
    if let Some(term) = row.term {
        app.screen.row_y.insert(term, y);
    }
    hit(app, lines, ScreenHit::Session(row.key));
    draw_subs(app, buf, r, y + SESSION_H, row, open, t);
}

/// `segs` from `x`, the last one cut with `…` where they would pass `end`.
fn put_cut(buf: &mut Buffer, x: u16, y: u16, segs: Vec<Seg>, end: u16) {
    let room = end.saturating_sub(x) as usize;
    let mut used = 0;
    let mut out = Vec::new();
    for (text, style) in segs {
        let w = text.width();
        if used + w <= room {
            used += w;
            out.push((text, style));
        } else {
            let left = room.saturating_sub(used);
            if left > 1 {
                out.push(seg(truncate(&text, left), style));
            }
            break;
        }
    }
    put(buf, x, y, &out, end);
}

/// Under a session: `└` and a mark per subagent, or for the open session a row each with its
/// name. A row opens the parent's pane: the agent shows its subagents in its own terminal.
fn draw_subs(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, row: &SideRow, open: bool, t: &Theme) {
    if row.subs.is_empty() {
        return;
    }
    let base = Style::default().bg(t.bg);
    let end = r.right().saturating_sub(2);
    let tree = base.fg(t.faint());
    if !open {
        let mut segs = vec![seg("└", tree)];
        for s in &row.subs {
            let m = app.state_mark(s.state, t);
            segs.push(seg(" ", base));
            segs.push(seg(m.glyph, base.fg(m.color)));
        }
        put(buf, r.x + TREE_X, y, &segs, end);
        return;
    }
    let Some(term) = row.term else {
        return;
    };
    for (i, s) in row.subs.iter().enumerate() {
        let yy = y + i as u16;
        let ground = t.bg;
        let line = Rect {
            x: r.x + 1,
            y: yy,
            width: r.width.saturating_sub(2),
            height: 1,
        };
        fill(buf, line, ground);
        let st = Style::default().bg(ground);
        let m = app.state_mark(s.state, t);
        let branch = if i + 1 == row.subs.len() { "└" } else { "├" };
        let segs = vec![
            seg(branch, st.fg(t.faint())),
            seg(" ", st),
            seg(m.glyph, st.fg(m.color)),
            seg(" ", st),
            seg(s.name.clone(), st.fg(t.text)),
        ];
        put_cut(buf, r.x + TREE_X, yy, segs, end);
        app.screen.sub_rows.push(term);
        hit(app, line, ScreenHit::Subagent(app.screen.sub_rows.len() - 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::TurnEvent;

    fn row() -> SideRow {
        SideRow {
            key: ClientSession::local(SessionKey::Pane(1)),
            term: None,
            name: "Growth Tests".into(),
            project: "AeVoxGrowth".into(),
            branch: "agent/grow-025".into(),
            state: State::Idle,
            agent: true,
            process: "claude".into(),
            machine: "devbox".into(),
            subs: Vec::new(),
            hidden: false,
            order: (1, MachineId::LOCAL),
        }
    }

    #[test]
    fn the_filter_matches_name_project_process_machine_and_branch() {
        let row = row();
        for q in ["", "growth t", "AEVOX", "cla", "devbox", "  devbox ", "grow-025"] {
            assert!(row.matches(q), "{q:?}");
        }
        assert!(!row.matches("pacman"));
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

    fn activity(id: &str, state: SubagentState, started: u64) -> SubagentActivity {
        SubagentActivity {
            id: id.into(),
            name: id.into(),
            parent: None,
            state,
            task: None,
            model: None,
            effort: None,
            started_at: Some(started),
            updated_at: started,
            latest_activity: None,
            result: None,
        }
    }

    #[test]
    fn only_the_current_turns_subagents_are_listed() {
        let mut info: TermInfo = serde_json::from_value(serde_json::json!({
            "id": 3, "cols": 80, "rows": 24, "title": "", "process": "claude", "agent": "claude",
            "status": "Working", "cwd": "/code", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 100, "asleep": false, "win32_input": false,
        }))
        .unwrap();
        info.timeline = vec![
            TurnEvent { at: 10, kind: TurnKind::Working, detail: String::new() },
            TurnEvent { at: 20, kind: TurnKind::Done, detail: String::new() },
            TurnEvent { at: 50, kind: TurnKind::Working, detail: String::new() },
        ];
        info.subagent_activity = vec![
            activity("old", SubagentState::Completed, 12),
            activity("this turn", SubagentState::Completed, 60),
            activity("running", SubagentState::Working, 70),
        ];
        let names: Vec<&str> = turn_subagents(&info).iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["this turn", "running"]);
        info.status = Status::Done;
        assert!(turn_subagents(&info).is_empty(), "a finished turn lists none");
    }

    #[test]
    fn a_subagents_state_reads_as_a_status_mark() {
        assert_eq!(sub_state(SubagentState::Working), State::Working);
        assert_eq!(sub_state(SubagentState::AwaitingApproval), State::Needs);
        assert_eq!(sub_state(SubagentState::Completed), State::Done);
        assert_eq!(sub_state(SubagentState::Failed), State::Problem);
    }
}
