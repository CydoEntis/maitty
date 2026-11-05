//! The sidebar card: projects, their sessions in one to three lines each, and the lines they
//! are drawn from. Its cards are held to the sidebar handoff (ADR 0050).

use super::check;
use super::glyphs::Kind as State;
use super::machine_group;
use super::side_tabs::{draw_tab_line, shown_tab_indexes, tab_line};
use super::*;
use crate::client::ids::ClientSession;
use crate::client::link::ConnectionState;
use crate::protocol::CheckState;

/// A line of the sidebar tree.
#[derive(Debug, Clone)]
pub(in crate::client) enum Line {
    Proj(usize),
    /// An agent or shell (pi, wi, si), and how deep it sits under the session that started it (0: not under anyone).
    Sess(usize, usize, usize, u8),
    /// A session's last line: its check result, its question, or what it is doing.
    Detail(ClientSession),
    /// One of a session's tabs (its owner, and where the tab is in `screen.tabs`), under its
    /// row while it has two or more (`side_tabs`).
    Tab(ClientTerm, usize),
    /// Nothing running in a folder project.
    Empty(usize),
    /// A machine's header, drawn only while there is more than one machine.
    Machine(MachineId),
    /// A line under a machine's header: why it is down, or what its daemon reports.
    MachineNote(Vec<String>, Color),
    /// The second row of a machine shown as one item (no sessions, or its line is not up).
    MachineState(MachineId),
    Gap,
}

/// Below this many screen columns a row's age gives way.
const AGES_FROM: u16 = 100;
/// The folder meta is shown only when at least this many columns are left for it.
const META_MIN_COLS: u16 = 6;

/// An agent that has started and been told nothing yet.
pub(super) fn is_new(s: &Session) -> bool {
    s.is_agent
        && !s.asleep
        && s.status == Status::Idle
        && s.title == WAITING
        && check::shown(s).is_none()
}

/// A row that wants a look: its name is strong and bold. Finished-and-seen and asleep rows are calm.
pub(super) fn is_hot(s: &Session, state: State, new: bool) -> bool {
    new || s.status == Status::Done || !matches!(state, State::Idle | State::Asleep | State::Passed)
}

/// How many helpers the agent reports running: the recorded ones, and the older name-only ones
/// no record stands for. Zero when it reports none (Codex doesn't).
fn agent_count(s: &Session) -> usize {
    let active = s.subagent_activity.iter().filter(|a| a.state.active());
    let named = |name: &str| {
        s.subagent_activity
            .iter()
            .any(|a| a.state.active() && a.name.eq_ignore_ascii_case(name))
    };
    active.count() + s.subagents.iter().filter(|n| !named(n)).count()
}

pub(in crate::client) fn session_lines(app: &App, s: &Session, out: &mut Vec<Line>) {
    // Every card has its three lines: the name, the badge, and what it is doing.
    out.push(Line::Detail(s.key));
    if let Some(term) = s.native_term {
        out.extend(
            shown_tab_indexes(app, term)
                .into_iter()
                .map(|i| Line::Tab(term, i)),
        );
    }
}

/// A helper's row steps in this far under the session that started it, per level.
pub(super) const HELPER_INDENT: u16 = 2;
/// Deeper helpers stay at this level: the names need the width more than the tree does.
const HELPER_DEPTH: u8 = 2;

/// Sessions of a project, each by (worktree, session) index.
type Rows = Vec<(usize, usize)>;

/// The machines to head, this computer first: none until the window knows another one.
fn machines_headed(app: &App) -> impl Iterator<Item = MachineId> {
    let others = app.remotes.keys().copied().collect::<Vec<_>>();
    (!others.is_empty())
        .then_some(MachineId::LOCAL)
        .into_iter()
        .chain(others)
}

pub(in crate::client) fn side_lines(app: &App, model: &[Proj], t: &Theme) -> Vec<Line> {
    let mut out = Vec::new();
    let mut headed = machines_headed(app).peekable();
    for (pi, p) in model.iter().enumerate() {
        while let Some(machine) = headed.next_if(|m| *m <= p.machine) {
            out.extend(machine_group::lines(app, machine, t));
            if machine != p.machine {
                out.push(Line::Gap);
            }
        }
        out.push(Line::Proj(pi));
        if app.screen.saved.closed.contains(&format!("p:{}", p.key)) {
            out.push(Line::Gap);
            continue;
        }
        // Just the sessions, where they were (the repo folder's before the worktrees').
        let every: Rows = (0..p.wts.len())
            .flat_map(|wi| (0..p.wts[wi].sessions.len()).map(move |si| (wi, si)))
            .collect();
        let mut rows = every;
        // Your order (dragged) otherwise.
        let at = |key: ClientSession| {
            app.screen
                .saved
                .session_order_of(app.machine_name(key.machine))
                .iter()
                .position(|saved| *saved == key.key)
                .unwrap_or(usize::MAX)
        };
        rows.sort_by_key(|&(wi, si)| {
            let s = &p.wts[wi].sessions[si];
            let mine = at(s.key);
            (
                if mine == usize::MAX {
                    !p.wts[wi].main
                } else {
                    false
                },
                mine,
                s.key,
            )
        });
        // Helpers: the sessions a session started sit under it, in the order above.
        let native_term_of = |r: &(usize, usize)| p.wts[r.0].sessions[r.1].native_term;
        let starter = |r: &(usize, usize)| {
            let s = &p.wts[r.0].sessions[r.1];
            s.started_by.filter(|by| {
                Some(*by) != s.native_term && rows.iter().any(|x| native_term_of(x) == Some(*by))
            })
        };
        let mut placed: Vec<((usize, usize), u8)> = Vec::with_capacity(rows.len());
        let mut next: Vec<((usize, usize), u8)> = rows
            .iter()
            .rev()
            .filter(|r| starter(r).is_none())
            .map(|r| (*r, 0))
            .collect();
        while let Some((r, depth)) = next.pop() {
            placed.push((r, depth));
            let below = (depth + 1).min(HELPER_DEPTH);
            next.extend(
                rows.iter()
                    .rev()
                    .filter(|k| {
                        starter(k).is_some_and(|id| native_term_of(&r) == Some(id))
                            && !placed.iter().any(|(x, _)| x == *k)
                    })
                    .map(|k| (*k, below)),
            );
        }
        // Sessions that started each other can't come from the daemon; if they did, they're
        // still listed.
        let lost: Vec<(usize, usize)> = rows
            .iter()
            .filter(|r| !placed.iter().any(|(x, _)| x == *r))
            .copied()
            .collect();
        placed.extend(lost.into_iter().map(|r| (r, 0)));
        let mut any = false;
        for ((wi, si), depth) in placed {
            any = true;
            out.push(Line::Sess(pi, wi, si, depth));
            let s = &p.wts[wi].sessions[si];
            session_lines(app, s, &mut out);
        }
        if !any {
            out.push(Line::Empty(pi));
        }
        out.push(Line::Gap);
    }
    for machine in headed {
        out.extend(machine_group::lines(app, machine, t));
        out.push(Line::Gap);
    }
    out
}

/// The lines drawn: this computer's projects and their cards, and the other machines' (each
/// under a header of its own; a machine whose line is not up, or that has no sessions, is that
/// header and one line of state instead of its projects).
fn card_lines(app: &App, model: &[Proj], t: &Theme) -> (Vec<Line>, Vec<Line>) {
    let mut by_machine: std::collections::BTreeMap<MachineId, Vec<Line>> = Default::default();
    let mut at = MachineId::LOCAL;
    for l in side_lines(app, model, t) {
        match &l {
            Line::Proj(pi) | Line::Empty(pi) => {
                at = model[*pi].machine;
                by_machine.entry(at).or_default().push(l);
            }
            Line::Sess(..) | Line::Detail(_) | Line::Tab(..) | Line::Gap => {
                by_machine.entry(at).or_default().push(l);
            }
            // Headers and their notes are drawn from the connection, not from these.
            Line::Machine(m) => at = *m,
            Line::MachineNote(..) | Line::MachineState(_) => {}
        }
    }
    let mut local = by_machine.remove(&MachineId::LOCAL).unwrap_or_default();
    if !local.iter().any(|l| matches!(l, Line::Proj(_))) {
        local = vec![Line::Machine(MachineId::LOCAL)];
    }
    let mut remote = Vec::new();
    for machine in app.remotes.keys().copied() {
        let up = app
            .remotes
            .get(&machine)
            .is_some_and(|r| matches!(r.state, ConnectionState::Connected));
        remote.push(Line::Machine(machine));
        match by_machine.remove(&machine) {
            Some(lines) if up && lines.iter().any(|l| matches!(l, Line::Proj(_))) => {
                remote.extend(lines);
            }
            _ => {
                remote.push(Line::MachineState(machine));
                remote.push(Line::Gap);
            }
        }
    }
    (local, remote)
}

/// The session a sidebar line stands for, if any.
pub(in crate::client) fn line_term(model: &[Proj], l: &Line) -> Option<ClientSession> {
    match l {
        Line::Sess(pi, wi, si, _) => Some(model[*pi].wts[*wi].sessions[*si].key),
        _ => None,
    }
}

/// The session a line belongs to as part of its card (its name, badge or last line).
fn row_term(model: &[Proj], l: &Line) -> Option<ClientSession> {
    match l {
        Line::Detail(term) => Some(*term),
        _ => line_term(model, l),
    }
}

/// Where the sidebar's rows go (the reference's X and RX): the selection bar's column, and the
/// last column right-aligned text reaches. Tints and clipped lines run one past it.
#[derive(Debug, Clone, Copy)]
struct Cols {
    x: u16,
    rx: u16,
    /// Rounded rows end in a cap on the last cell of their tint, so nothing else is drawn there.
    cap: bool,
    /// A folder row starts a cell before X, so its fold button keeps X and its left end is a
    /// blank cell that can take a cap: with caps on.
    lead: bool,
}

impl Cols {
    fn of(r: Rect, cap: bool) -> Cols {
        Cols {
            x: r.x + 2,
            rx: r.right().saturating_sub(3),
            cap,
            lead: cap,
        }
    }

    /// A folder's row: `row`, a cell longer on the left when it can end in a cap there.
    fn proj_row(self, y: u16) -> Rect {
        let lead = u16::from(self.lead);
        Rect {
            x: self.x - lead,
            width: self.row(y).width + lead,
            ..self.row(y)
        }
    }

    /// The end (exclusive) of a row's tint: a column past RX inside a card.
    fn tint_end(self) -> u16 {
        self.rx + 2
    }

    /// The end (exclusive) of anything drawn on a row, so a cut line's `…` stays in sight. A cap
    /// takes the last cell of the tint.
    fn end(self) -> u16 {
        self.tint_end() - u16::from(self.cap)
    }

    fn row(self, y: u16) -> Rect {
        Rect {
            x: self.x,
            y,
            width: self.tint_end().saturating_sub(self.x),
            height: 1,
        }
    }
}

/// How a session's lines are drawn: their ground, whether they are the selected row, and the
/// colour of the edge bar they carry, if any.
#[derive(Debug, Clone, Copy)]
struct RowLook {
    bg: Color,
    picked: bool,
    bar: Option<Color>,
}

impl RowLook {
    /// A state's colour, kept readable on the selection.
    fn paint(self, t: &Theme, c: Color) -> Color {
        if self.picked { t.on_lit(c) } else { c }
    }

    fn on(self, segs: Vec<Seg>, t: &Theme) -> Vec<Seg> {
        segs.into_iter()
            .map(|(text, st)| {
                (
                    text,
                    st.fg(self.paint(t, st.fg.unwrap_or(t.text))).bg(self.bg),
                )
            })
            .collect()
    }
}

/// What both halves of the sidebar share when their rows are drawn.
struct Pass<'a> {
    model: &'a [Proj],
    look: Look,
    cols: Cols,
    pb: Color,
    focus: Option<ClientSession>,
    focused_side: bool,
    screen_w: u16,
}

/// A half of the sidebar's list: its first row, how many rows it has, and how far it is scrolled.
#[derive(Clone, Copy)]
struct Window {
    top: u16,
    rows: usize,
    scroll: usize,
}

/// A sidebar shorter than this many list rows is not split: it keeps only the Local half.
const SPLIT_MIN_ROWS: usize = 4;

/// The rows of the Local and the Remote half of a list `rows` tall: the Remote title takes one
/// row and the rest is shared evenly, Local keeping any odd one.
fn halve(rows: usize) -> (usize, usize) {
    if rows < SPLIT_MIN_ROWS {
        return (rows, 0);
    }
    let remote = (rows - 1) / 2;
    (rows - 1 - remote, remote)
}

/// The sidebar card: Local on top, Remote below it, each with its title and counts, projects
/// with their sessions under them, new and actions at the foot.
pub(in crate::client) fn draw_side(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    model: &[Proj],
    t: &Theme,
) {
    let look = Look::of(&app.cfg.ui);
    let pb = pane_bg(t);
    let focused_side = side_focused(app);
    let mut c = Card::new(t, "");
    if focused_side {
        c = c.lit(t.accent);
    }
    card(app, buf, r, &c, t);
    app.screen.side_rect = r;
    hit(app, r, ScreenHit::SideFocus);
    let cols = Cols::of(r, look.round_rows);
    let screen_w = buf.area.width;
    let (local_lines, remote_lines) = card_lines(app, model, t);
    let lines: Vec<Line> = local_lines.iter().chain(&remote_lines).cloned().collect();
    let focus = open_row(app).map(|term| app.session_key(term));
    let top = r.y + 2;
    // The foot is lifted a row off the bottom (Cards: it sits above the border), a divider over it.
    let foot_y = r.bottom().saturating_sub(2);
    let release = if r.height >= 8 {
        release_chip(app, look, cols, pb, t)
    } else {
        None
    };
    app.screen.release_in_side = release.is_some();
    let list_h = foot_y
        .saturating_sub(1 + u16::from(release.is_some()))
        .saturating_sub(top) as usize;
    note_rows(app, model, &lines);
    let (local_rows, remote_rows) = halve(list_h);
    let local_scroll = region_scroll(app, model, &local_lines, focus, local_rows, false);
    let remote_scroll = region_scroll(app, model, &remote_lines, focus, remote_rows, true);
    app.screen.follow = false;

    let host = app
        .machine
        .host()
        .map(|h| format!(" ⇄ {h}"))
        .unwrap_or_default();
    draw_title(
        app,
        buf,
        (r.x + 2, r.y),
        &format!("LOCAL{host}"),
        Color::Reset,
        t,
        focused_side,
    );

    let pass = Pass {
        model,
        look,
        cols,
        pb,
        focus,
        focused_side,
        screen_w,
    };
    draw_rows(
        app,
        buf,
        &pass,
        t,
        &local_lines,
        Window {
            top,
            rows: local_rows,
            scroll: local_scroll,
        },
    );
    app.screen.remote_rect = Rect::default();
    if remote_rows > 0 {
        let head_y = top + local_rows as u16;
        // The Remote title sits on a rule that joins the card's sides, as Local's sits on the
        // top border: `├ REMOTE 2 ───── + add ┤`.
        let rule = Style::default()
            .fg(if focused_side { t.accent } else { t.line })
            .bg(pb);
        put(
            buf,
            r.x,
            head_y,
            &[seg(
                format!("├{}┤", "─".repeat(r.width.saturating_sub(2) as usize)),
                rule,
            )],
            r.right(),
        );
        draw_title(
            app,
            buf,
            (r.x + 2, head_y),
            &match app.remotes.len() {
                0 => "REMOTE".to_string(),
                n => format!("REMOTE {n}"),
            },
            pb,
            t,
            focused_side,
        );
        draw_add_chip(app, buf, (r.right().saturating_sub(2), head_y), pb, t);
        let remote_top = head_y + 1;
        app.screen.remote_rect = Rect {
            x: r.x,
            y: head_y,
            width: r.width,
            height: remote_rows as u16 + 1,
        };
        if remote_lines.is_empty() {
            draw_no_machines(app, buf, &pass, t, remote_top, remote_rows);
        } else {
            draw_rows(
                app,
                buf,
                &pass,
                t,
                &remote_lines,
                Window {
                    top: remote_top,
                    rows: remote_rows,
                    scroll: remote_scroll,
                },
            );
        }
    }
    draw_foot(app, buf, r, cols, pb, t);
    if let Some((segs, h)) = release {
        let (x, y) = (cols.x + 1, r.bottom().saturating_sub(4));
        let w = segs_width(&segs);
        put(buf, x, y, &segs, cols.rx + 1);
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
}

/// What the Remote half says while no other machine is known: one click opens the Machines
/// dialog to add one.
fn draw_no_machines(
    app: &mut App,
    buf: &mut Buffer,
    pass: &Pass,
    t: &Theme,
    top: u16,
    rows: usize,
) {
    let st = Style::default().bg(pass.pb);
    let x = pass.cols.x + 2;
    let end = pass.cols.end();
    put(
        buf,
        x,
        top,
        &[seg(
            "no remote machines",
            st.fg(t.muted).add_modifier(Modifier::ITALIC),
        )],
        end,
    );
    if rows > 1 {
        let add = [seg(
            "+ add one",
            st.fg(t.accent).add_modifier(Modifier::BOLD),
        )];
        put(buf, x, top + 1, &add, end);
        hit(
            app,
            Rect {
                x,
                y: top + 1,
                width: segs_width(&add),
                height: 1,
            },
            ScreenHit::AddMachine,
        );
    }
}

/// One half of the sidebar's list: `lines` from `win.scroll`, `win.rows` of them, from `win.top`.
fn draw_rows(app: &mut App, buf: &mut Buffer, pass: &Pass, t: &Theme, lines: &[Line], win: Window) {
    let Pass {
        model,
        look,
        cols,
        pb,
        focus,
        focused_side,
        screen_w,
    } = *pass;
    let Window {
        top,
        rows: list_h,
        scroll,
    } = win;
    let on = app.motion_on();
    let mut gliding: Option<u16> = None;
    // A session's row is its lines together: one click target, so the pointer's tint lights
    // them as one.
    let bottom = top + list_h as u16;
    let row_rect = |i: usize, y: u16| {
        let mut n = 1;
        while matches!(lines.get(i + n), Some(Line::Detail(_))) {
            n += 1;
        }
        Rect {
            height: (n as u16).min(bottom.saturating_sub(y)),
            ..cols.row(y)
        }
    };
    let mut looks: std::collections::BTreeMap<ClientSession, RowLook> = Default::default();
    for (i, line) in lines.iter().enumerate().skip(scroll).take(list_h) {
        let y = top + (i - scroll) as u16;
        let indent = 0;
        let row = cols.row(y);
        if let Some(term) = row_term(model, line)
            && let Some((_, _, s)) = find_key(model, term)
        {
            let rl = *looks.entry(term).or_insert_with(|| {
                let picked = if focused_side {
                    app.screen.cursor == Some(term)
                } else {
                    Some(term) == focus
                };
                row_look(picked, pb, t)
            });
            if rl.bg != pb {
                fill(buf, row, rl.bg);
            }
            if look.round_rows && rl.bg != pb && row.width > 2 {
                // A one-line row is a pill. A block of lines keeps square ends: caps would
                // scallop it and quarter-cell corners notch it, so its lines meet as one block.
                let first = (scroll..=i)
                    .rev()
                    .find(|&j| matches!(lines[j], Line::Sess(..)) || j == scroll)
                    .unwrap_or(i);
                if row_rect(first, top + (first - scroll) as u16).height == 1 {
                    buf[(row.x, y)].set_symbol(CAP_L).set_fg(rl.bg).set_bg(pb);
                    buf[(row.right() - 1, y)]
                        .set_symbol(CAP_R)
                        .set_fg(rl.bg)
                        .set_bg(pb);
                }
            } else if let Some(c) = rl.bar {
                check::edge(app, buf, cols.x, y, c, rl.bg);
            }
            if matches!(line, Line::Sess(..)) || i == scroll {
                hit(app, row_rect(i, y), ScreenHit::Session(term));
            }
            let x = cols.x + 4 + indent;
            match line {
                Line::Sess(..) => {
                    if let Some(id) = app.session_term(term) {
                        app.screen.row_y.insert(id, y);
                    }
                    let keyboard = focused_side && app.screen.cursor == Some(term);
                    if keyboard
                        && Some(term) != focus
                        && let Some(gy) = app.motion.glide(
                            "side",
                            term.machine.motion_key()
                                | match term.key {
                                    SessionKey::Pane(id) => u64::from(id),
                                    SessionKey::Conversation(id) => id.0,
                                },
                            y,
                            on,
                        )
                    {
                        gliding = Some(gy);
                    }
                    draw_session(app, buf, s, cols, y, indent, rl, t, screen_w >= AGES_FROM);
                }
                Line::Detail(_) => {
                    let text = activity(app, s, rl, t, cols.end().saturating_sub(x) as usize);
                    put(buf, x, y, &text, cols.end());
                    // Its question is this line: a click answers it, not just opens the session.
                    if (s.helpers_wanted > 0 || s.secret_ask.is_some())
                        && let Some(term) = s.native_term
                    {
                        hit(app, row, ScreenHit::HelperAsk(term));
                    }
                }
                _ => {}
            }
            if s.down {
                machine_group::grey(buf, row, t);
            }
            continue;
        }
        if let Line::Tab(owner, tab) = line {
            let last = !matches!(lines.get(i + 1), Some(Line::Tab(o, _)) if o == owner);
            draw_shell_row(app, buf, model, (*owner, *tab, last), (cols, y), pb, t);
            continue;
        }
        draw_line(app, buf, model, line, cols, y, pb, t, screen_w < NARROW);
    }
    scroll_marks(
        buf,
        cols.rx + 1,
        top,
        (scroll, list_h, lines.len()),
        Style::default().bg(pb).fg(t.muted),
    );
    if let Some(gy) = gliding.filter(|gy| (top..top + list_h as u16).contains(gy))
        && focus
            .and_then(|key| app.session_term(key))
            .and_then(|id| app.screen.row_y.get(&id))
            .copied()
            != Some(gy)
    {
        tint_row(
            look,
            buf,
            cols.x,
            gy,
            cols.tint_end() - cols.x,
            t.sel_bg(),
            pb,
        );
    }
}

/// The session row that is open: the focused pane's, or in a split the split's (its first
/// pane's), whichever side of it has the keys.
pub(super) fn open_row(app: &App) -> Option<ClientTerm> {
    let shown: Vec<ClientTerm> = app
        .screen
        .tabs
        .get(app.screen.tab)
        .map(|t| t.leaves())
        .unwrap_or_default();
    app.focused().map(|f| {
        if shown.len() > 1 && shown.contains(&f) {
            shown[0]
        } else {
            f
        }
    })
}

/// What the keyboard and the mouse read back from the rows drawn: their order and places.
pub(super) fn note_rows(app: &mut App, model: &[Proj], lines: &[Line]) {
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

/// `▲` at the list's top when lines are scrolled off above, `▼` at its bottom when more are
/// below, in column `x`. `(scroll, list_h, len)`: the first line shown, the rows, the lines.
pub(super) fn scroll_marks(
    buf: &mut Buffer,
    x: u16,
    top: u16,
    (scroll, list_h, len): (usize, usize, usize),
    st: Style,
) {
    if scroll > 0 {
        put(buf, x, top, &[seg("▲", st)], x + 1);
    }
    if scroll + list_h < len {
        put(buf, x, top + list_h as u16 - 1, &[seg("▼", st)], x + 1);
    }
}

/// The first line shown: the focused (or cursor) row is kept in view when it changes;
/// otherwise the wheel rules. Clears the follow request, so for the single list.
pub(super) fn side_scroll(
    app: &mut App,
    model: &[Proj],
    lines: &[Line],
    focus: Option<ClientSession>,
    list_h: usize,
) -> usize {
    let scroll = region_scroll(app, model, lines, focus, list_h, false);
    app.screen.follow = false;
    scroll
}

/// `side_scroll` for one half of the sidebar (`remote`: the lower one), which leaves the follow
/// request for the other half to see; the caller clears it.
fn region_scroll(
    app: &mut App,
    model: &[Proj],
    lines: &[Line],
    focus: Option<ClientSession>,
    list_h: usize,
    remote: bool,
) -> usize {
    let mut scroll = if remote {
        app.screen.remote_scroll
    } else {
        app.screen.side_scroll
    } as usize;
    if app.screen.follow {
        let want = app
            .screen
            .cursor
            .or(focus)
            .and_then(|term| lines.iter().position(|l| line_term(model, l) == Some(term)));
        if let Some(i) = want {
            if i < scroll {
                scroll = i.saturating_sub(1);
            } else if i + 2 >= scroll + list_h {
                scroll = i + 3 - list_h.min(i + 3);
            }
        }
    }
    scroll = scroll.min(lines.len().saturating_sub(list_h));
    if remote {
        app.screen.remote_scroll = scroll as u16;
    } else {
        app.screen.side_scroll = scroll as u16;
    }
    scroll
}

/// The selected row (the keyboard's while the sidebar has the keys, else the open session) is
/// tinted and carries an accent bar; no other row has one.
fn row_look(picked: bool, ground: Color, t: &Theme) -> RowLook {
    let bg = if picked { t.sel_bg() } else { ground };
    let bar = picked.then_some(t.accent);
    RowLook { bg, picked, bar }
}

/// A half's title chip, " LOCAL " or " REMOTE 2 ", set into a border or a rule at `at`. `outer` is
/// what lies under the chip's round ends.
fn draw_title(
    app: &App,
    buf: &mut Buffer,
    at: (u16, u16),
    label: &str,
    outer: Color,
    t: &Theme,
    lit: bool,
) {
    let (chip_bg, ink) = if lit {
        (t.accent, t.acc_ink)
    } else {
        (t.btn, t.strong)
    };
    let mut title = vec![seg(
        format!(" {label} "),
        Style::default()
            .bg(chip_bg)
            .fg(ink)
            .add_modifier(Modifier::BOLD),
    )];
    if Look::of(&app.cfg.ui).caps {
        title = round_ends(title, outer);
    }
    put(buf, at.0, at.1, &title, buf.area.right());
}

/// The Remote rule's `+ add` chip, ending at `end`: a click opens the Machines dialog.
fn draw_add_chip(app: &mut App, buf: &mut Buffer, (end, y): (u16, u16), ground: Color, t: &Theme) {
    let chip = Style::default().bg(t.btn);
    let mut segs = vec![
        seg(" + ", chip.fg(t.accent).add_modifier(Modifier::BOLD)),
        seg("add ", chip.fg(t.text)),
    ];
    if Look::of(&app.cfg.ui).caps {
        segs = round_ends(segs, ground);
    }
    let w = segs_width(&segs);
    let x = end.saturating_sub(w);
    put(buf, x, y, &segs, end);
    hit(
        app,
        Rect {
            x,
            y,
            width: w,
            height: 1,
        },
        ScreenHit::AddMachine,
    );
}

/// A terminal with nothing to report is marked by its prompt arrow, not an idle state.
pub(super) fn plain_shell(s: &Session) -> bool {
    !s.is_agent && !s.asleep && s.status == Status::None && check::shown(s).is_none()
}

/// The mark for the agent a session runs, so a row tells Claude from Codex at a glance: a shape,
/// or a letter with the ASCII glyph set. An agent with no mark of its own shows its first letter.
fn agent_mark(agent: &str, ascii: bool) -> String {
    let (shape, letter) = match agent {
        "claude" => ("✻", "C"),
        "codex" => ("◆", "X"),
        "gemini" => ("✦", "G"),
        "opencode" => ("◇", "O"),
        other => {
            return other
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default();
        }
    };
    (if ascii { letter } else { shape }).into()
}

/// The least room a name keeps before the row's right-hand marks give way.
const NAME_MIN: usize = 8;
/// The right of a session's first line, at most `room` wide: `● you` when it needs you, else its
/// agent's mark and the age its state began; the mark gives way first.
fn row_meta(
    s: &Session,
    (ascii, needs): (bool, Option<&str>),
    when: &str,
    room: usize,
    st: Style,
    t: &Theme,
) -> Vec<Seg> {
    if let Some(mark) = needs {
        let you = vec![seg(
            format!("{mark} you"),
            st.fg(t.blocked).add_modifier(Modifier::BOLD),
        )];
        return if segs_width(&you) as usize <= room {
            you
        } else {
            Vec::new()
        };
    }
    let mark = if s.is_agent {
        agent_mark(&s.agent, ascii)
    } else {
        String::new()
    };
    let build = |with_mark: bool| {
        let mut v: Vec<Seg> = Vec::new();
        if with_mark && !mark.is_empty() {
            v.push(seg(mark.clone(), st.fg(t.muted)));
        }
        if !when.is_empty() {
            if !v.is_empty() {
                v.push(seg(" ", st));
            }
            v.push(seg(when.to_string(), st.fg(t.muted)));
        }
        v
    };
    [true, false]
        .into_iter()
        .map(build)
        .find(|v| segs_width(v) as usize <= room)
        .unwrap_or_default()
}

/// A session's first line: its mark, its name in its state's colour, and at the right its
/// agent, its uncommitted lines and how long ago its state began.
#[allow(clippy::too_many_arguments)]
fn draw_session(
    app: &mut App,
    buf: &mut Buffer,
    s: &Session,
    cols: Cols,
    y: u16,
    indent: u16,
    rl: RowLook,
    t: &Theme,
    ages: bool,
) {
    let spinner = machine_group::working_mark(app, s.down);
    let mark = check::mark(app, s, t, &spinner);
    let state = check::kind(s);
    let st = Style::default().bg(rl.bg);
    let new = is_new(s);
    let hot = is_hot(s, state, new);
    let mark_colour = if new { t.accent } else { mark.color };
    let (g, gc) = if plain_shell(s) {
        ("›", t.muted)
    } else {
        (mark.glyph, rl.paint(t, mark_colour))
    };
    let mut gs = st.fg(gc);
    if matches!(
        state,
        State::Failed | State::Needs | State::Approve | State::Working
    ) || new
    {
        gs = gs.add_modifier(Modifier::BOLD);
    }
    let gx = cols.x + 2 + indent;
    // The 3-bar spinner is two cells wide; the name steps right to make room, as the reference's does.
    let gw = (g.width() as u16).max(1);
    put(buf, gx, y, &[seg(g, gs)], gx + gw);
    if let Some(term) = s.native_term {
        hit(
            app,
            Rect {
                x: gx,
                y,
                width: gw + 1,
                height: 1,
            },
            ScreenHit::Why(term),
        );
    }
    let menu = hovered(app, cols.row(y)) && !rl.picked;
    let when = if ages && s.is_agent && matches!(s.key.key, SessionKey::Pane(_)) {
        age(s.since)
    } else {
        String::new()
    };
    let nx = gx + gw + 1;
    // The hovered row's ⋯ takes the last column, a space before it: over the age's place.
    let meta_end = if when.is_empty() && menu {
        cols.rx.saturating_sub(1)
    } else {
        cols.rx + 1
    };
    let when = if menu { " ".repeat(when.width()) } else { when };
    let free = meta_end.saturating_sub(nx) as usize;
    let ascii = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons)
        .needs
        .is_ascii();
    let needs = (!s.down && check::needs_you(s)).then(|| check::marks(app).needs);
    let meta = row_meta(
        s,
        (ascii, needs),
        &when,
        free.saturating_sub(NAME_MIN + 1),
        st,
        t,
    );
    let mw = segs_width(&meta);
    let room = free.saturating_sub(if mw > 0 { mw as usize + 1 } else { 0 });
    let name = truncate(&s.name, room);
    // A name in a state worth a look (working, checking, finished and unseen, needs you) takes
    // the state's colour, and moves with motion on; a calm one is `text`, or `strong` when picked.
    let coloured = !plain_shell(s) && NameMotion::of(s, state) != NameMotion::Still;
    let colour = if coloured {
        gc
    } else if rl.picked || (hot && !plain_shell(s)) {
        t.strong
    } else {
        t.text
    };
    let ns = if hot && !plain_shell(s) {
        st.add_modifier(Modifier::BOLD)
    } else {
        st
    };
    let anim = if coloured && app.motion_on() && !s.down {
        NameMotion::of(s, state)
    } else {
        NameMotion::Still
    };
    let left = name_segs(&name, anim, app.spinner_frame(), colour, t.strong, ns);
    put(buf, nx, y, &left, cols.rx + 1);
    if mw > 0 {
        let mx = meta_end.saturating_sub(mw);
        put(buf, mx, y, &meta, meta_end);
    }
    if menu {
        row_menu_button(
            app,
            buf,
            Rect {
                x: cols.rx,
                y,
                width: 1,
                height: 1,
            },
            rl.bg,
            t,
            ScreenHit::RowMenuSess(s.key),
        );
    }
}

/// How a session's name moves while motion is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameMotion {
    Still,
    /// A bright band sweeps across it: working, checking, or finished and not yet seen.
    Shimmer,
    /// It slowly brightens and dims: it needs you.
    Pulse,
}

impl NameMotion {
    fn of(s: &Session, state: State) -> NameMotion {
        match state {
            State::Working | State::Checking => NameMotion::Shimmer,
            State::Done | State::Passed if s.status == Status::Done => NameMotion::Shimmer,
            State::Needs | State::Approve => NameMotion::Pulse,
            _ => NameMotion::Still,
        }
    }
}

/// Spinner frames in one pulse, dim to bright and back (about 1.6 s).
const PULSE_FRAMES: u64 = 18;
/// How far toward `strong` a pulse brightens a name at its peak.
const PULSE_PEAK: f32 = 0.5;

/// `text` in `base`, moving as `motion` says at spinner frame `frame`.
fn name_segs(
    text: &str,
    motion: NameMotion,
    frame: u64,
    base: Color,
    bright: Color,
    st: Style,
) -> Vec<Seg> {
    match motion {
        NameMotion::Still => vec![seg(text.to_string(), st.fg(base))],
        NameMotion::Shimmer => shimmer(text, frame, base, blend(base, bright, 0.6), st),
        NameMotion::Pulse => {
            let half = PULSE_FRAMES / 2;
            let k = (frame % PULSE_FRAMES).abs_diff(half) as f32 / half as f32;
            vec![seg(
                text.to_string(),
                st.fg(blend(base, bright, (1.0 - k) * PULSE_PEAK)),
            )]
        }
    }
}

/// A session's second line, in `room` columns. With several checkpoints it is the headline
/// word, then a glyph per checkpoint; the glyphs give way, never the word.
fn detail(app: &App, s: &Session, rl: RowLook, t: &Theme, room: usize) -> Vec<Seg> {
    if let Some(receipt) = s
        .native_term
        .and_then(|id| popover::receipt_segs(app, id, t))
    {
        return rl.on(receipt, t);
    }
    if s.secret_ask.is_some() || s.helpers_wanted > 0 {
        return rl.on(
            vec![seg(
                truncate(&ask_line_in(s, room), room),
                Style::default().fg(t.blocked),
            )],
            t,
        );
    }
    let checks = check::reported(s);
    let segs = if checks.len() > 1 {
        let (word, st) = check::headline_word(checks, t);
        let mut v = vec![(truncate(&word, room), st)];
        v.extend(check::strip(app, checks, t));
        v
    } else if let Some(info) = check::shown(s) {
        check::result_words(info, t)
    } else if s.resume_at.is_some() {
        status_words(app, s, t)
    } else if let Some(q) = &s.question {
        vec![seg(format!("asks: {q}"), Style::default().fg(t.blocked))]
    } else {
        status_words(app, s, t)
    };
    rl.on(segs, t)
}

/// What a session without a check result or a question is doing, in words.
fn status_words(app: &App, s: &Session, t: &Theme) -> Vec<Seg> {
    let muted = |text: &str| vec![seg(text.to_string(), Style::default().fg(t.muted))];
    if s.asleep {
        return muted("asleep zzZ");
    }
    if let Some(at) = s.resume_at {
        return muted(&format!("⏸ resumes in {}", until(at)));
    }
    match s.status {
        // The spinner and the name's colour already say working: only its helpers and a quiet
        // spell are left to say.
        Status::Working => {
            let mut words: Vec<Seg> = Vec::new();
            // Only an agent that reports helpers has a count (Codex reports none).
            match agent_count(s) {
                0 => {}
                _ if !app.cfg.ui.subagent_count => {}
                n => words.push(seg(
                    format!("{n} agent{}", if n == 1 { "" } else { "s" }),
                    Style::default().fg(t.text),
                )),
            }
            if let Some(at) = s.quiet_since {
                let sep = if words.is_empty() { "" } else { " · " };
                words.push(seg(
                    format!("{sep}quiet {}", age(at)),
                    Style::default().fg(t.muted),
                ));
            }
            words
        }
        Status::Blocked => vec![seg("needs you", Style::default().fg(t.blocked))],
        Status::Done => vec![seg(
            "done · not seen",
            Style::default().fg(t.done).add_modifier(Modifier::BOLD),
        )],
        Status::Idle if is_new(s) => vec![
            seg(format!("{} · ", s.agent), Style::default().fg(t.muted)),
            seg("new · type to begin", Style::default().fg(t.accent)),
        ],
        Status::Idle if s.is_agent => muted("idle"),
        _ => Vec::new(),
    }
}

/// A row's lines under its name in `muted`, not bold: only its checkpoints' glyphs keep their
/// colour, so the sidebar's colour is the marks'.
fn quiet(app: &App, s: &Session, segs: Vec<Seg>, t: &Theme) -> Vec<Seg> {
    let glyphs: Vec<&str> = s.checks.iter().map(|c| check::glyph(app, c)).collect();
    segs.into_iter()
        .map(|(text, st)| {
            if glyphs.contains(&text.as_str()) {
                (text, st)
            } else {
                (text, st.fg(t.muted).remove_modifier(Modifier::BOLD))
            }
        })
        .collect()
}

/// A line that is not part of a session's row.
#[allow(clippy::too_many_arguments)]
fn draw_line(
    app: &mut App,
    buf: &mut Buffer,
    model: &[Proj],
    line: &Line,
    cols: Cols,
    y: u16,
    pb: Color,
    t: &Theme,
    small: bool,
) {
    let row = cols.row(y);
    match line {
        Line::Proj(pi) => draw_proj(app, buf, model, *pi, cols, y, pb, t, small),
        // Nothing running: one click starts a shell there.
        Line::Empty(pi) => {
            let st = Style::default().bg(pb);
            put(
                buf,
                cols.x + 2,
                y,
                &[
                    seg("empty  ", st.fg(t.muted).add_modifier(Modifier::ITALIC)),
                    seg("new shell", st.fg(t.accent).add_modifier(Modifier::BOLD)),
                ],
                cols.end(),
            );
            hit(app, row, ScreenHit::ShellIn(*pi));
        }
        Line::Machine(machine) => draw_machine_row(app, buf, *machine, cols, y, pb, t),
        Line::MachineState(machine) => draw_machine_state(app, buf, *machine, cols, y, pb, t),
        Line::Tab(..) | Line::MachineNote(..) | Line::Sess(..) | Line::Detail(_) | Line::Gap => {}
    }
}

/// A card's last line: what the session is doing, in `muted`, or in amber when it needs you.
fn activity(app: &App, s: &Session, rl: RowLook, t: &Theme, room: usize) -> Vec<Seg> {
    let needs = check::needs_you(s) || s.question.is_some();
    let words = detail(app, s, rl, t, room);
    let words = if words.is_empty() {
        let who = if s.is_agent {
            s.agent.as_str()
        } else {
            "shell"
        };
        rl.on(vec![seg(who.to_string(), Style::default().fg(t.muted))], t)
    } else {
        words
    };
    if needs {
        words
    } else {
        quiet(app, s, words, t)
    }
}

/// One of a session's other tabs under its card: `├ shell  2`, the last one `└`.
fn draw_shell_row(
    app: &mut App,
    buf: &mut Buffer,
    model: &[Proj],
    (owner, tab, last): (ClientTerm, usize, bool),
    (cols, y): (Cols, u16),
    pb: Color,
    t: &Theme,
) {
    let Some(line) = tab_line(app, model, owner, tab, t) else {
        return;
    };
    let number = app.screen.tabs[..=tab]
        .iter()
        .filter(|tb| tb.owner == owner)
        .count();
    let name = if line.needs {
        t.blocked
    } else if line.current {
        t.accent
    } else {
        t.text
    };
    let st = Style::default().bg(pb);
    let x = cols.x + 4;
    let room = cols.end().saturating_sub(x) as usize;
    let label = truncate(&line.label, room.saturating_sub(6));
    let mut left = vec![
        seg(if last { "└ " } else { "├ " }, st.fg(t.muted)),
        seg(
            label,
            st.fg(name).add_modifier(if line.current {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
        seg(format!("  {number}"), st.fg(t.muted)),
    ];
    left.retain(|(text, _)| !text.is_empty());
    draw_tab_line(
        app,
        buf,
        cols.row(y),
        x,
        cols.end(),
        (left, Vec::new()),
        tab,
    );
}

/// A machine shown as one item: its badge, and at the right its connection. This computer with
/// nothing running says so and starts a shell.
fn draw_machine_row(
    app: &mut App,
    buf: &mut Buffer,
    machine: MachineId,
    cols: Cols,
    y: u16,
    pb: Color,
    t: &Theme,
) {
    let st = Style::default().bg(pb);
    let row = cols.row(y);
    if machine == MachineId::LOCAL {
        put(
            buf,
            cols.x + 2,
            y,
            &[
                seg(
                    "no sessions  ",
                    st.fg(t.muted).add_modifier(Modifier::ITALIC),
                ),
                seg("new shell", st.fg(t.accent).add_modifier(Modifier::BOLD)),
            ],
            cols.end(),
        );
        hit(app, row, ScreenHit::NewShell);
        return;
    }
    let mark = app.machine_mark(machine, t);
    let dim = mark.as_ref().is_none_or(|m| m.color != t.done);
    let right = mark
        .map(|m| {
            vec![
                seg(format!("{} ", m.glyph), st.fg(m.color)),
                seg(m.word, st.fg(m.color)),
            ]
        })
        .unwrap_or_default();
    let rw = segs_width(&right);
    let badge_style = if dim {
        Style::default().fg(t.muted).bg(t.card2)
    } else {
        Style::default()
            .fg(t.strong)
            .bg(t.btn)
            .add_modifier(Modifier::BOLD)
    };
    let room = (cols.end().saturating_sub(cols.x + 2) as usize).saturating_sub(rw as usize + 1);
    let badge = seg(
        truncate(&format!(" ⇄ {} ", app.machine_label_of(machine)), room),
        badge_style,
    );
    put(buf, cols.x + 2, y, &[badge], cols.end());
    put(buf, cols.end().saturating_sub(rw), y, &right, cols.end());
    hit(app, row, ScreenHit::AddMachine);
}

/// The second row of a machine item: what is wrong, or that it has nothing running.
fn draw_machine_state(
    app: &mut App,
    buf: &mut Buffer,
    machine: MachineId,
    cols: Cols,
    y: u16,
    pb: Color,
    t: &Theme,
) {
    let text = match app.remotes.get(&machine).map(|r| &r.state) {
        Some(ConnectionState::Connected) => "no sessions yet",
        Some(ConnectionState::Connecting) => "connecting…",
        _ => "disconnected · click for the fix",
    };
    put(
        buf,
        cols.x + 4,
        y,
        &[seg(text, Style::default().fg(t.muted).bg(pb))],
        cols.end(),
    );
    hit(app, cols.row(y), ScreenHit::AddMachine);
}

/// A folder's header: its fold arrow and name, and right-aligned what it is (branch, its
/// checkpoints), which gives way first.
#[allow(clippy::too_many_arguments)]
fn draw_proj(
    app: &mut App,
    buf: &mut Buffer,
    model: &[Proj],
    pi: usize,
    cols: Cols,
    y: u16,
    pb: Color,
    t: &Theme,
    small: bool,
) {
    let p = &model[pi];
    let row = cols.proj_row(y);
    let open = !app.screen.saved.closed.contains(&format!("p:{}", p.key));
    let on = side_focused(app) && app.screen.cursor_proj.as_ref() == Some(&p.key);
    let hov = hovered(app, row);
    let bg = if on { t.sel_bg() } else { pb };
    if bg != pb {
        fill(buf, row, bg);
    }
    // A selected folder row is one line, so it can end in half-circles. Text stops a cell short
    // of the right cap; the left one has a cell of its own before the fold button.
    let round = on && Look::of(&app.cfg.ui).round_tabs;
    let text_end = if round { row.right() - 1 } else { cols.rx + 1 };
    if round {
        buf[(text_end, y)].set_symbol(CAP_R).set_fg(bg).set_bg(pb);
        if cols.lead {
            buf[(row.x, y)].set_symbol(CAP_L).set_fg(bg).set_bg(pb);
        }
    }
    let st = Style::default().bg(bg);
    if !app.screen.fold_colors.contains_key(&p.key) {
        app.pick_fold_color(&p.key);
    }
    // ▾ (open) or ▸ (closed) in the state bars' column; `v` and `>` with the ASCII glyph set.
    let ascii = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons)
        .needs
        .is_ascii();
    let fold = match (open, ascii) {
        (true, false) => "▾",
        (false, false) => "▸",
        (true, true) => "v",
        (false, true) => ">",
    };
    put(buf, cols.x, y, &[seg(fold, st.fg(t.muted))], text_end);
    let name = truncate(&p.name, cols.rx.saturating_sub(cols.x + 2) as usize);
    let mut left = vec![
        seg("  ", st),
        seg(name, st.fg(t.strong).add_modifier(Modifier::BOLD)),
    ];
    if p.fresh {
        left.push(seg(" ", st));
        left.extend(chip(
            "NEW",
            Style::default()
                .bg(t.accent)
                .fg(t.acc_ink)
                .add_modifier(Modifier::BOLD),
        ));
    }
    let lx = cols.x + 1;
    put(buf, lx, y, &left, text_end);
    hit(app, row, ScreenHit::ToggleProj(pi));
    if hov && !small {
        row_glyph(
            app,
            buf,
            Rect {
                x: cols.rx.saturating_sub(2),
                y,
                width: 1,
                height: 1,
            },
            "+",
            bg,
            t.accent,
            t,
            ScreenHit::ShellIn(pi),
        );
        row_menu_button(
            app,
            buf,
            Rect {
                x: cols.rx,
                y,
                width: 1,
                height: 1,
            },
            bg,
            t,
            ScreenHit::RowMenuProj(pi),
        );
        return;
    }
    let room = text_end
        .saturating_sub(1)
        .saturating_sub(lx + segs_width(&left));
    if room >= META_MIN_COLS {
        let meta = proj_meta(app, p, open, room as usize, st, t);
        put(
            buf,
            text_end.saturating_sub(segs_width(&meta)),
            y,
            &meta,
            text_end,
        );
    }
}

/// A project's right-hand side in `room` cells: its branch (`⎇ main`, or `no git`) and, once any of
/// its sessions has a check result, `✓` or, if one failed, `✕`. Folded, how many sessions it hides,
/// with an amber `●` when one of them needs you.
fn proj_meta(app: &App, p: &Proj, open: bool, room: usize, st: Style, t: &Theme) -> Vec<Seg> {
    let marks = check::marks(app);
    if !open {
        let n = p.sessions().count();
        let words = if n == 0 {
            "empty".to_string()
        } else {
            format!("{n} hidden")
        };
        let mut out = Vec::new();
        if p.sessions().any(|s| !s.down && check::needs_you(s)) {
            out.push(seg(
                format!("{} ", marks.needs),
                st.fg(t.blocked).add_modifier(Modifier::BOLD),
            ));
        }
        out.push(seg(truncate(&words, room), st.fg(t.muted)));
        return out;
    }
    let place = if p.git {
        p.wts
            .iter()
            .find(|w| w.main && !w.branch.is_empty())
            .map(|w| format!("{} {}", marks.branch, w.branch))
            .unwrap_or_default()
    } else {
        "no git".into()
    };
    let result = if p
        .sessions()
        .any(|s| check::shown(s).is_some_and(check::failed))
    {
        Some((marks.failed, t.err))
    } else if p
        .sessions()
        .any(|s| check::shown(s).is_some_and(|c| c.state == CheckState::Passed))
    {
        Some((marks.done, t.done))
    } else {
        None
    };
    let glyph_room = result.map_or(0, |_| 2);
    let mut out = vec![seg(
        truncate(&place, room.saturating_sub(glyph_room)),
        st.fg(t.muted),
    )];
    if let Some((glyph, colour)) = result
        && room >= segs_width(&out) as usize + glyph_room
    {
        out.push(seg(
            format!(" {glyph}"),
            st.fg(colour).add_modifier(Modifier::BOLD),
        ));
    }
    out
}

fn release_chip(
    app: &App,
    look: Look,
    cols: Cols,
    ground: Color,
    t: &Theme,
) -> Option<(Vec<Seg>, ScreenHit)> {
    let room = (cols.rx + 1).saturating_sub(cols.x + 1);
    [mode_bar::Release::Words, mode_bar::Release::Version]
        .into_iter()
        .find_map(|shown| {
            let (segs, hit) = mode_bar::release_segs(app, shown, t)?;
            let segs = if look.caps {
                round_ends(segs, ground)
            } else {
                segs
            };
            (segs_width(&segs) <= room).then_some((segs, hit))
        })
}

/// A `btn` chip: `word` in `text`, after an `accent` bold `icon` when it has one. Its ends
/// follow `ui.rounded`.
pub(super) fn word_chip(
    app: &App,
    icon: Option<&str>,
    word: &str,
    t: &Theme,
    outer: Color,
) -> Vec<Seg> {
    let look = Look::of(&app.cfg.ui);
    let mut body = Vec::new();
    if let Some(icon) = icon {
        body.push(seg(
            format!("{icon} "),
            Style::default()
                .fg(t.accent)
                .bg(t.btn)
                .add_modifier(Modifier::BOLD),
        ));
    }
    body.push(seg(word, Style::default().fg(t.text).bg(t.btn)));
    pill(look, body, t.btn, outer)
}

/// The foot, lifted a row off the bottom under a `line` divider, with no band behind it: a
/// `+ New` chip a column in and a `Menu` chip at the right; they drop from the end when the
/// sidebar is too narrow.
fn draw_foot(app: &mut App, buf: &mut Buffer, r: Rect, cols: Cols, pb: Color, t: &Theme) {
    if r.height < 6 {
        return;
    }
    let y = r.bottom().saturating_sub(2);
    hline(
        buf,
        cols.x,
        y - 1,
        (cols.rx + 1).saturating_sub(cols.x),
        t,
        pb,
    );
    let new = word_chip(app, Some("+"), "New", t, pb);
    let menu = word_chip(app, None, "Menu", t, pb);
    let (nw, mw) = (segs_width(&new), segs_width(&menu));
    let nx = cols.x + 1;
    if nx + nw > cols.rx + 1 {
        return;
    }
    put(buf, nx, y, &new, cols.rx + 1);
    hit(
        app,
        Rect {
            x: nx,
            y,
            width: nw,
            height: 1,
        },
        ScreenHit::NewShell,
    );
    let mx = cols.rx.saturating_sub(mw);
    if mx > nx + nw {
        put(buf, mx, y, &menu, cols.rx);
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

/// The ⋯ that opens a sidebar row's menu (for terminals that keep right-clicks).
pub(in crate::client) fn row_menu_button(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    bg: Color,
    t: &Theme,
    h: ScreenHit,
) {
    row_glyph(app, buf, r, "⋯", bg, t.muted, t, h);
}

/// A small glyph button on a sidebar row (⋯, +): just the glyph on the row's own ground,
/// brightening when it's the one under the mouse.
#[allow(clippy::too_many_arguments)]
fn row_glyph(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    glyph: &str,
    bg: Color,
    fg: Color,
    t: &Theme,
    h: ScreenHit,
) {
    let mut st = Style::default()
        .bg(bg)
        .fg(if hovered(app, r) { t.strong } else { fg });
    if hovered(app, r) {
        st = st.add_modifier(Modifier::BOLD);
    }
    put(buf, r.x, r.y, &[seg(glyph, st)], r.right());
    hit(app, r, h);
}

#[cfg(test)]
mod frame_tests {
    use crate::client::reference_fixture::{Layout, Stage, frame, world};

    /// The sidebar's inner rows: columns X-1..RX+1 of the card, between its border rows.
    fn inner(text: &str, r: ratatui::layout::Rect) -> Vec<String> {
        let last = r.bottom() - 2;
        let lines: Vec<Vec<char>> = text.lines().map(|l| l.chars().collect()).collect();
        (r.y + 1..=last)
            .map(|y| {
                let row = &lines[y as usize];
                (r.x + 1..r.right() - 1)
                    .map(|x| row.get(x as usize).copied().unwrap_or(' '))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn several_checkpoints_clip_their_glyphs_never_their_word() {
        let row_for = |name: &str| {
            let mut app = world(Stage::Multi);
            app.snap.terms.get_mut(&1).unwrap().checks[1].name = name.into();
            let text = frame(&mut app, Layout::Tiled, 80, 24, "unicode");
            let rows = inner(&text, app.screen.side_rect);
            rows.iter()
                .find(|r| r.contains("failed:"))
                .cloned()
                .unwrap_or_default()
        };
        assert_eq!(
            row_for("e2e").trim_start_matches(['▎', ' ']),
            "failed: e2e ✓✕",
            "room for both"
        );
        assert_eq!(
            row_for("functional").trim_start_matches(['▎', ' ']),
            "failed: functional",
            "the glyphs give way"
        );
        assert_eq!(
            row_for("integration-suite").trim_start_matches(['▎', ' ']),
            "failed: integratio…",
            "a word that can't fit is cut, not dropped"
        );
    }

    /// The working row's first line and its second, with the Appearance spinner `frames` in
    /// glyph set `glyphs`.
    fn working_rows(frames: &[&str], glyphs: &str, layout: Layout) -> (String, String) {
        let mut app = world(Stage::None);
        app.cfg.ui.spinner = frames.iter().map(|f| f.to_string()).collect();
        let text = frame(&mut app, layout, 160, 45, glyphs);
        let rows = inner(&text, app.screen.side_rect);
        let at = rows
            .iter()
            .position(|r| r.contains("add-payments"))
            .expect("the working row");
        (
            rows[at].trim().trim_start_matches('▎').trim().to_string(),
            rows[at + 1].trim().to_string(),
        )
    }

    #[test]
    fn the_bars_spinner_marks_a_working_row_and_the_name_steps_right_for_it() {
        let (first, second) = working_rows(crate::config::BARS_FRAMES, "unicode", Layout::Floating);
        assert!(
            first.starts_with("⣶⡆ add-payments"),
            "motion off holds the third frame: {first:?}"
        );
        assert_ne!(
            second, "working",
            "the spinner says working: there is no line for it"
        );
        let (one_cell, _) = working_rows(&["|"], "unicode", Layout::Floating);
        assert!(
            one_cell.starts_with("| add-payments"),
            "a one-column choice keeps the name where it was: {one_cell:?}"
        );
        let (tiled, _) = working_rows(crate::config::BARS_FRAMES, "unicode", Layout::Tiled);
        assert!(tiled.starts_with("⣶⡆ add-payments"), "{tiled:?}");
    }

    #[test]
    fn the_ascii_glyph_set_draws_the_bars_in_ascii_and_leaves_other_choices_alone() {
        let (bars, _) = working_rows(crate::config::BARS_FRAMES, "ascii", Layout::Floating);
        assert!(
            bars.is_ascii() && bars.starts_with("O  add-payments"),
            "{bars:?}"
        );
        let (custom, _) = working_rows(&["a", "b", "c"], "ascii", Layout::Floating);
        assert!(
            custom.starts_with("c add-payments"),
            "a custom choice is the user's: {custom:?}"
        );
    }

    #[test]
    fn the_working_mark_is_bold_and_its_name_holds_one_colour_without_motion() {
        use ratatui::style::Modifier;
        let mut app = world(Stage::None);
        app.cfg.ui.spinner = crate::config::BARS_FRAMES
            .iter()
            .map(|f| f.to_string())
            .collect();
        let buf =
            crate::client::reference_fixture::cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let (gx, gy) = (0..45u16)
            .flat_map(|y| (0..160u16).map(move |x| (x, y)))
            .find(|&(x, y)| buf[(x, y)].symbol() == "⣶")
            .expect("the mark");
        assert!(
            buf[(gx, gy)].modifier.contains(Modifier::BOLD),
            "the mark is bold"
        );
        let name: Vec<_> = (gx + 3..gx + 3 + "add-payments".len() as u16)
            .map(|x| buf[(x, gy)].fg)
            .collect();
        assert!(
            name.windows(2).all(|w| w[0] == w[1]),
            "no shimmer band without motion: {name:?}"
        );
    }
}

#[cfg(test)]
mod v8_tests {
    use super::*;
    use crate::client::reference_fixture::{LAYOUTS, Layout, Stage, cells, raw_text, world};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::style::Modifier;

    fn side_rows(app: &mut App, layout: Layout, w: u16, h: u16) -> Vec<String> {
        let text = raw_text(app, layout, w, h, "unicode");
        let r = app.screen.side_rect;
        text.lines()
            .map(|l| {
                l.chars()
                    .skip(r.x as usize)
                    .take(r.width as usize)
                    .collect::<String>()
            })
            .collect()
    }

    fn has(rows: &[String], needle: &str) -> bool {
        rows.iter().any(|r| r.contains(needle))
    }

    #[test]
    fn update_chip_uses_the_sidebar_foot_when_there_is_room() {
        let mut app = world(Stage::None);
        app.update_available = Some("9.9.9".into());
        let text = raw_text(&mut app, Layout::Floating, 120, 34, "unicode");
        let rows: Vec<&str> = text.lines().collect();
        let side = app.screen.side_rect;
        let chip = rows
            .iter()
            .position(|line| line.contains("Update 9.9.9"))
            .expect("the sidebar's Update chip");
        assert!(chip + 1 < side.bottom() as usize);
        assert!(rows[chip + 1].contains('─'), "the divider sits below it");
        assert!(!rows.last().unwrap_or(&"").contains("Update"));
        assert!(app.hits.iter().any(|(r, h)| {
            r.y == chip as u16
                && *h == Hit::Screen(ScreenHit::Update)
                && side.contains(Position::new(r.x, r.y))
        }));
    }

    /// The cell where `needle` starts on the first row that has it, among the sidebar's rows.
    fn find(buf: &ratatui::buffer::Buffer, app: &App, needle: &str) -> (u16, u16) {
        let r = app.screen.side_rect;
        (r.y..r.bottom())
            .find_map(|y| {
                let row: String = (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect();
                row.find(needle)
                    .map(|at| (r.x + row[..at].chars().count() as u16, y))
            })
            .unwrap_or_else(|| panic!("{needle:?} is not in the sidebar"))
    }

    #[test]
    fn a_quiet_agent_says_how_long_on_its_row() {
        let mut app = world(Stage::None);
        let before = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(!before.iter().any(|r| r.contains("quiet")), "{before:#?}");
        app.snap.terms.get_mut(&2).unwrap().quiet_since =
            Some(crate::client::screen::now_secs() - 12 * 60);
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(
            has(&rows, "quiet 12m") && !has(&rows, "working"),
            "the quiet spell, not the state: {rows:#?}"
        );
    }

    #[test]
    fn only_the_selected_row_has_a_bar_and_it_is_the_accent() {
        let mut app = world(Stage::Failed);
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let side = app.screen.side_rect;
        let bars: Vec<(u16, Color)> = (side.y..side.bottom())
            .flat_map(|y| (side.x..side.right()).map(move |x| (x, y)))
            .filter(|&p| buf[p].symbol() == "▎")
            .map(|(_, y)| (y, buf[(side.x + 2, y)].fg))
            .collect();
        let open = app.screen.row_y[&app.focused().expect("an open session")];
        assert!(
            !bars.is_empty()
                && bars
                    .iter()
                    .all(|&(y, c)| (open..open + 3).contains(&y) && c == app.theme.accent),
            "{bars:?}"
        );
    }

    #[test]
    fn names_shimmer_while_working_or_unseen_and_pulse_while_they_need_you() {
        let (base, bright, st) = (
            Color::Rgb(0, 0, 0),
            Color::Rgb(250, 250, 250),
            Style::default(),
        );
        let colours = |m, f| {
            name_segs("quick-fox", m, f, base, bright, st)
                .iter()
                .map(|(_, s)| s.fg)
                .collect::<Vec<_>>()
        };
        assert!(
            colours(NameMotion::Still, 3)
                .iter()
                .all(|c| *c == Some(base)),
            "still: one colour"
        );
        let band = colours(NameMotion::Shimmer, 6);
        assert!(
            band.contains(&Some(base)) && band.iter().any(|c| *c != Some(base)),
            "a band crosses the name: {band:?}"
        );
        assert_ne!(
            colours(NameMotion::Shimmer, 6),
            colours(NameMotion::Shimmer, 9),
            "and moves"
        );
        let at = |f| colours(NameMotion::Pulse, f)[0];
        assert_eq!(at(0), Some(base), "a pulse starts dim");
        assert_ne!(at(PULSE_FRAMES / 2), Some(base), "brightens");
        assert_eq!(at(PULSE_FRAMES), Some(base), "and dims again");
    }

    #[test]
    fn the_foot_is_two_chips_lifted_off_the_bottom_under_a_divider_with_no_band() {
        for layout in LAYOUTS {
            let mut app = world(Stage::None);
            let buf = cells(&mut app, layout, 160, 45, "unicode");
            let (r, t) = (app.screen.side_rect, app.theme.clone());
            let foot_y = r.bottom() - 2;
            let row =
                |y: u16| -> String { (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect() };
            assert!(
                row(foot_y).contains("+ New") && row(foot_y).contains("Menu"),
                "{layout:?}: {:?}",
                row(foot_y)
            );
            assert!(
                row(foot_y - 1).contains("────"),
                "{layout:?}: a divider above: {:?}",
                row(foot_y - 1)
            );
            let line_fg = (r.x..r.right())
                .find(|x| buf[(*x, foot_y - 1)].symbol() == "─")
                .map(|x| buf[(x, foot_y - 1)].fg);
            assert_eq!(line_fg, Some(t.line), "{layout:?}: the divider is `line`");
            // No band: the cells between the chips are the sidebar's own ground.
            let (n, _) = find(&buf, &app, "+ New");
            let (m, _) = find(&buf, &app, "Menu");
            assert_eq!(
                buf[(n + 8, foot_y)].bg,
                buf[(n + 8, foot_y - 2)].bg,
                "{layout:?}: no band behind the foot"
            );
            assert!(m > n + 8, "{layout:?}: menu is at the right");
            // New's `+` in the accent, bold; the words in text, no letter lit; chips raised.
            let plus = &buf[(n, foot_y)];
            assert_eq!(
                (plus.fg, plus.bg, plus.modifier.contains(Modifier::BOLD)),
                (t.accent, t.btn, true),
                "{layout:?}: New's +"
            );
            for (word, x) in [("New", n + 2), ("Menu", m)] {
                assert!(
                    (x..x + word.len() as u16)
                        .all(|x| buf[(x, foot_y)].fg == t.text && buf[(x, foot_y)].bg == t.btn),
                    "{layout:?}: {word} is plain text on a chip"
                );
            }
            let hits = |h: ScreenHit| {
                app.hits
                    .iter()
                    .any(|(rect, hit)| *hit == crate::client::Hit::Screen(h) && rect.y == foot_y)
            };
            assert!(
                hits(ScreenHit::NewShell) && hits(ScreenHit::Actions),
                "{layout:?}: both chips are clickable"
            );
        }
    }

    #[test]
    fn z_collapses_into_the_column_and_capital_z_hides_and_each_restores() {
        let mut app = world(Stage::None);
        let area = ratatui::layout::Rect::new(0, 0, 120, 30);
        assert!(app.sidebar && !app.side_hidden);
        app.act(Action::ToggleSidebar);
        app.side_frac = 0.0;
        assert!(
            !app.sidebar && !app.side_hidden && super::super::draw::strip_on(&app),
            "z: the column"
        );
        let g = super::super::draw::grid(&app, area);
        assert_eq!(
            (g.side.width, g.tabs.y),
            (crate::client::screen::RAIL_W, 0),
            "a narrow column stays, and the tab row keeps the top row"
        );
        assert!(
            g.panes.x >= g.side.right() && g.panes.right() == area.right(),
            "the panes take the rest of the width"
        );
        app.act(Action::HideSidebar);
        app.side_frac = 0.0;
        assert!(
            app.side_hidden && !super::super::draw::strip_on(&app),
            "Z: hidden"
        );
        let g = super::super::draw::grid(&app, area);
        assert_eq!(
            (g.tabs.x, g.tabs.y, g.panes.x, g.panes.width),
            (0, 0, 0, 120),
            "hidden gives the tabs and panes everything"
        );
        app.act(Action::ToggleSidebar);
        assert!(app.sidebar && !app.side_hidden, "z restores the sidebar");
        app.act(Action::HideSidebar);
        app.act(Action::HideSidebar);
        assert!(app.sidebar && !app.side_hidden, "Z again restores it");
    }

    #[test]
    fn the_column_switches_with_arrows_and_digits_only_while_it_has_the_keys() {
        let mut app = world(Stage::None);
        let (w, h) = (160, 45);
        app.sidebar = false;
        let _ = raw_text(&mut app, Layout::Floating, w, h, "unicode");
        let text = raw_text(&mut app, Layout::Floating, w, h, "unicode");
        let column: Vec<String> = text.lines().map(|l| l.chars().take(8).collect()).collect();
        for letter in ['F', 'A', 'W', 'Q', 'O'] {
            assert!(
                column.iter().any(|l| l.contains(letter)),
                "{letter} (a session's first letter) is in the column: {column:?}"
            );
        }
        assert!(
            column[0].trim_start().starts_with('»'),
            "the expand chip comes first: {column:?}"
        );
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        // The pane has the keys: a digit is typed into it, nothing switches.
        app.mode = Mode::Normal;
        app.on_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert!(
            !std::iter::from_fn(|| rx.try_recv().map(|o| o.msg).ok()).any(|m| matches!(
                m,
                crate::protocol::ClientMsg::Command(crate::protocol::Command::FocusPane { .. })
            ))
        );
        // The column has them: 1–9 jump, ↑ ↓ and ← → step.
        app.act(Action::BrowseTree);
        let _ = raw_text(&mut app, Layout::Floating, w, h, "unicode");
        app.on_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert_eq!(
            app.screen.cursor,
            app.screen
                .side_items
                .iter()
                .filter_map(|i| if let SideItem::Sess(t) = i {
                    Some(*t)
                } else {
                    None
                })
                .nth(2)
        );
        assert_eq!(app.mode, Mode::Side, "the keys stay on the column");
        let third = app.screen.cursor;
        app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_ne!(app.screen.cursor, third);
        app.on_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.screen.cursor, third);
        app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_ne!(app.screen.cursor, third, "down steps to the next row");
        app.on_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.screen.cursor, third, "up steps back");
        assert!(
            std::iter::from_fn(|| rx.try_recv().map(|o| o.msg).ok()).any(|m| matches!(
                m,
                crate::protocol::ClientMsg::Command(crate::protocol::Command::FocusPane { .. })
            )),
            "the session opens in the pane"
        );
    }

    #[test]
    fn the_selected_row_is_accent_with_ink_when_the_column_has_the_keys_else_raised() {
        let mut app = world(Stage::None);
        app.sidebar = false;
        let t = app.theme.clone();
        let chip = |app: &mut App| {
            let buf = cells(app, Layout::Floating, 160, 45, "unicode");
            let (x, y) = (0..8u16)
                .flat_map(|x| (0..45u16).map(move |y| (x, y)))
                .find(|&(x, y)| buf[(x, y)].symbol() == "F")
                .expect("fix-footer's letter");
            (buf[(x, y)].bg, buf[(x, y)].fg)
        };
        chip(&mut app);
        app.act(Action::BrowseTree);
        assert_eq!(
            chip(&mut app),
            (t.accent, t.acc_ink),
            "the strip has the keys"
        );
        app.mode = Mode::Normal;
        app.screen.cursor = None;
        assert_eq!(chip(&mut app).0, t.btn, "else raised");
    }
}
