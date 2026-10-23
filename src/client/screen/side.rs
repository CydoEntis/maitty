//! The sidebar card: projects, their sessions in one to three lines each, and the lines they
//! are drawn from. The layout is the v7 handoff's `drawSide` (`docs/design/v7/checks7.js`).

use super::check;
use super::glyphs::Kind as State;
use super::*;

/// A line of the sidebar tree.
#[derive(Debug, Clone)]
pub(in crate::client) enum Line {
    /// The heading over the other machines' sessions (with how many).
    Section(Kind, usize),
    Proj(usize),
    /// An agent or shell (pi, wi, si), and how deep it sits under the session that started it (0: not under anyone).
    Sess(usize, usize, usize, u8),
    /// A session's second line: its check result, its question, or what it is doing.
    Detail(TermId),
    /// A session's third line, from `NARROW` columns: a failed check's first failing line, or
    /// its checkpoints by name.
    Third(TermId),
    /// A dim line under an agent: a queued follow-up. Shares the agent's highlight.
    Note(String, Color, TermId),
    /// Nothing running in a folder project.
    Empty(usize),
    Gap,
}

/// A session's load is said under its row from here: a fifth of the machine's CPU, or two
/// gigabytes. And in the working colour from here, where it's the reason the PC is slow.
const LOAD_SAID_CPU: f32 = 20.0;
const LOAD_SAID_MEM: u64 = 2 << 30;
const LOAD_HEAVY_CPU: f32 = 50.0;
const LOAD_HEAVY_MEM: u64 = 6 << 30;

/// Below this many screen columns a row's age gives way.
const AGES_FROM: u16 = 100;
/// The folder meta is shown only when at least this many columns are left for it.
const META_MIN_COLS: u16 = 6;

/// "▲ 38% CPU · 2.1 GB" for a session that's taking a real share of the machine, and whether
/// it's heavy. Nothing for one that isn't.
pub(in crate::client) fn load_note(cpu: f32, mem: u64) -> Option<(String, bool)> {
    if cpu < LOAD_SAID_CPU && mem < LOAD_SAID_MEM {
        return None;
    }
    let gb = mem as f64 / f64::from(1u32 << 30);
    Some((format!("▲ {cpu:.0}% CPU · {gb:.1} GB"), cpu >= LOAD_HEAVY_CPU || mem >= LOAD_HEAVY_MEM))
}

/// A session has a second line: an agent always does, a plain shell only for a check result.
fn has_detail(s: &Session) -> bool {
    s.is_agent || s.asleep || check::shown(s).is_some() || s.question.is_some()
}

/// Its second line stays below `NARROW` columns: a check result, a question, or a wait for a
/// plan limit.
/// A finished agent with nothing to say: below `NARROW` its row is one line. Anything that
/// needs a look (working, asking, new, a check result, a wait for a plan limit) keeps its second.
fn quiet_done(s: &Session) -> bool {
    !is_new(s) && check::shown(s).is_none() && s.question.is_none() && s.resume_at.is_none() && matches!(s.status, Status::Idle | Status::None)
}

/// An agent that has started and been told nothing yet.
pub(super) fn is_new(s: &Session) -> bool {
    s.is_agent && !s.asleep && s.status == Status::Idle && s.title == WAITING && check::shown(s).is_none()
}

/// A row that wants a look: its name is strong and bold. Finished-and-seen and asleep rows are calm.
pub(super) fn is_hot(s: &Session, state: State, new: bool) -> bool {
    new || s.status == Status::Done || !matches!(state, State::Idle | State::Asleep | State::Passed)
}

/// How many helpers the agent reports running: the recorded ones, and the older name-only ones
/// no record stands for. Zero when it reports none (Codex doesn't).
fn agent_count(s: &Session) -> usize {
    let active = s.subagent_activity.iter().filter(|a| a.state.active());
    let named = |name: &str| s.subagent_activity.iter().any(|a| a.state.active() && a.name.eq_ignore_ascii_case(name));
    active.count() + s.subagents.iter().filter(|n| !named(n)).count()
}

fn has_third(s: &Session) -> bool {
    check::reported(s).len() > 1 || check::shown(s).is_some_and(|c| check::failed(c) && !c.first_failing_line.is_empty())
}

pub(in crate::client) fn session_lines(s: &Session, t: &Theme, out: &mut Vec<Line>) {
    let notes_c = t.muted;
    if has_detail(s) {
        out.push(Line::Detail(s.term));
    }
    if has_third(s) {
        out.push(Line::Third(s.term));
    }
    // What it's been told to do next, once this turn is over.
    if let Some(next) = s.queued.first() {
        let more = if s.queued.len() > 1 { format!(" (+{})", s.queued.len() - 1) } else { String::new() };
        out.push(Line::Note(format!("⏵ next: {next}{more}"), notes_c, s.term));
    }
}

/// A helper's row steps in this far under the session that started it, per level.
const HELPER_INDENT: u16 = 2;
/// Deeper helpers stay at this level: the names need the width more than the tree does.
const HELPER_DEPTH: u8 = 2;

/// Sessions of a project, each by (worktree, session) index.
type Rows = Vec<(usize, usize)>;

pub(in crate::client) fn side_lines(app: &App, model: &[Proj], t: &Theme) -> Vec<Line> {
    let mut out = Vec::new();
    for (pi, p) in model.iter().enumerate() {
        if p.kind == Kind::Ssh && (pi == 0 || model[pi - 1].kind != p.kind) {
            let n = model.iter().filter(|q| q.kind == p.kind).map(|q| q.sessions().count()).sum();
            out.push(Line::Section(p.kind, n));
        }
        out.push(Line::Proj(pi));
        if app.screen.saved.closed.contains(&format!("p:{}", p.key)) {
            out.push(Line::Gap);
            continue;
        }
        // Just the sessions: the ones that need you first, the rest where they were (the repo
        // folder's before the worktrees' when equal).
        let every: Rows = (0..p.wts.len()).flat_map(|wi| (0..p.wts[wi].sessions.len()).map(move |si| (wi, si))).collect();
        let mut rows = every;
        // Your order (dragged) otherwise.
        let order = &app.screen.saved.session_order;
        let at = |t: TermId| order.iter().position(|x| *x == t).unwrap_or(usize::MAX);
        let sort = app.cfg.ui.attention_sort;
        rows.sort_by_key(|&(wi, si)| {
            let s = &p.wts[wi].sessions[si];
            let mine = at(s.term);
            (sort && settled(s.status), if mine == usize::MAX { !p.wts[wi].main } else { false }, mine, s.term)
        });
        // Helpers: the sessions a session started sit under it, in the order above. One that
        // needs you has sorted to the top and stays there, on its own.
        let term_of = |r: &(usize, usize)| p.wts[r.0].sessions[r.1].term;
        let starter = |r: &(usize, usize)| {
            let s = &p.wts[r.0].sessions[r.1];
            let floats = sort && !settled(s.status);
            s.started_by.filter(|by| !floats && *by != s.term && rows.iter().any(|x| term_of(x) == *by))
        };
        let mut placed: Vec<((usize, usize), u8)> = Vec::with_capacity(rows.len());
        let mut next: Vec<((usize, usize), u8)> = rows.iter().rev().filter(|r| starter(r).is_none()).map(|r| (*r, 0)).collect();
        while let Some((r, depth)) = next.pop() {
            placed.push((r, depth));
            let below = (depth + 1).min(HELPER_DEPTH);
            next.extend(rows.iter().rev().filter(|k| starter(k) == Some(term_of(&r)) && !placed.iter().any(|(x, _)| x == *k)).map(|k| (*k, below)));
        }
        // Sessions that started each other can't come from the daemon; if they did, they're
        // still listed.
        let lost: Vec<(usize, usize)> = rows.iter().filter(|r| !placed.iter().any(|(x, _)| x == *r)).copied().collect();
        placed.extend(lost.into_iter().map(|r| (r, 0)));
        let mut any = false;
        for ((wi, si), depth) in placed {
            any = true;
            out.push(Line::Sess(pi, wi, si, depth));
            let s = &p.wts[wi].sessions[si];
            session_lines(s, t, &mut out);
        }
        if !any {
            out.push(Line::Empty(pi));
        }
        out.push(Line::Gap);
    }
    out
}

/// The lines drawn at `screen_w` columns. Below `NARROW` a finished, seen row is one line and
/// the third line goes; a row that needs a look keeps its second.
fn fitted_lines(app: &App, model: &[Proj], t: &Theme, screen_w: u16) -> Vec<Line> {
    let mut lines = side_lines(app, model, t);
    if screen_w < NARROW {
        lines.retain(|l| match l {
            Line::Third(_) => false,
            Line::Detail(term) => find(model, *term).is_some_and(|(.., s)| !quiet_done(s)) || app.screen.receipts.contains_key(term),
            _ => true,
        });
    }
    lines
}

/// The session a sidebar line stands for, if any.
pub(in crate::client) fn line_term(model: &[Proj], l: &Line) -> Option<TermId> {
    match l {
        Line::Sess(pi, wi, si, _) => Some(model[*pi].wts[*wi].sessions[*si].term),
        _ => None,
    }
}

/// The session a line belongs to as part of its row (its first, second or third line).
fn row_term(model: &[Proj], l: &Line) -> Option<TermId> {
    match l {
        Line::Detail(term) | Line::Third(term) => Some(*term),
        _ => line_term(model, l),
    }
}

/// Where the sidebar's rows go (the handoff's X and RX): the selection bar's column, and the
/// last column right-aligned text reaches. Tints and clipped lines run one past it.
#[derive(Debug, Clone, Copy)]
struct Cols {
    x: u16,
    rx: u16,
    flat: bool,
}

impl Cols {
    fn of(r: Rect, flat: bool) -> Cols {
        let (left, right) = if flat { (1, 2) } else { (2, 3) };
        Cols { x: r.x + left, rx: r.right().saturating_sub(right), flat }
    }

    /// The end (exclusive) of anything drawn on a row: a column past RX inside a card; in Flat
    /// that column is the pane's divider, so a cut line's `…` stays in sight.
    fn end(self) -> u16 {
        self.rx + if self.flat { 1 } else { 2 }
    }

    fn row(self, y: u16) -> Rect {
        Rect { x: self.x, y, width: self.end().saturating_sub(self.x), height: 1 }
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
        segs.into_iter().map(|(text, st)| (text, st.fg(self.paint(t, st.fg.unwrap_or(t.text))).bg(self.bg))).collect()
    }
}

/// The sidebar card: its title and counts, projects with their sessions under them, new and
/// actions at the foot.
pub(in crate::client) fn draw_side(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
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
    draw_head(app, buf, r, model, t, focused_side, look.flat);
    let cols = Cols::of(r, look.flat);
    let screen_w = buf.area.width;
    let lines = fitted_lines(app, model, t, screen_w);
    let shown: Vec<TermId> = app.screen.tabs.get(app.screen.tab).map(|t| t.layout.leaves()).unwrap_or_default();
    // In a split, the split's row (its first pane's) is the open one, whichever side you're on.
    let focus = app.focused().map(|f| if shown.len() > 1 && shown.contains(&f) { shown[0] } else { f });
    let top = r.y + 2;
    // The foot is lifted a row off the bottom (Cards: it sits above the border), a divider over it.
    let foot_y = r.bottom().saturating_sub(2);
    let list_h = foot_y.saturating_sub(1).saturating_sub(top) as usize;
    let scroll = side_scroll(app, model, &lines, focus, list_h);

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

    let on = app.motion_on();
    let mut gliding: Option<u16> = None;
    // The lines under a session (what it asks) step in as far as it does.
    let mut indents: Vec<u16> = Vec::with_capacity(lines.len());
    for l in &lines {
        let last = indents.last().copied().unwrap_or(0);
        indents.push(match l {
            Line::Sess(.., depth) => u16::from(*depth) * HELPER_INDENT,
            Line::Note(..) | Line::Detail(..) | Line::Third(..) => last,
            _ => 0,
        });
    }
    // A session's row is its lines together: one click target, so the pointer's tint lights
    // them as one.
    let bottom = top + list_h as u16;
    let row_rect = |i: usize, y: u16| {
        let mut n = 1;
        while matches!(lines.get(i + n), Some(Line::Detail(_) | Line::Third(_))) {
            n += 1;
        }
        Rect { height: (n as u16).min(bottom.saturating_sub(y)), ..cols.row(y) }
    };
    let mut looks: HashMap<TermId, RowLook> = HashMap::new();
    for (i, line) in lines.iter().enumerate().skip(scroll).take(list_h) {
        let y = top + (i - scroll) as u16;
        let indent = indents[i];
        let row = cols.row(y);
        if let Some(term) = row_term(model, line)
            && let Some((.., s)) = find(model, term)
        {
            let rl = *looks.entry(term).or_insert_with(|| {
                let picked = if focused_side { app.screen.cursor == Some(term) } else { Some(term) == focus };
                row_look(s, picked, Some(term) == focus, focused_side, pb, t)
            });
            if rl.bg != pb {
                fill(buf, row, rl.bg);
            }
            if let Some(c) = rl.bar {
                check::edge(app, buf, cols.x, y, c, rl.bg);
            }
            if matches!(line, Line::Sess(..)) || i == scroll {
                hit(app, row_rect(i, y), ScreenHit::Session(term));
            }
            let x = cols.x + 4 + indent;
            match line {
                Line::Sess(..) => {
                    app.screen.row_y.insert(term, y);
                    let keyboard = focused_side && app.screen.cursor == Some(term);
                    if keyboard && Some(term) != focus && let Some(gy) = app.motion.glide("side", u64::from(term), y, on) {
                        gliding = Some(gy);
                    }
                    draw_session(app, buf, s, cols, y, indent, rl, t, screen_w >= AGES_FROM);
                }
                Line::Detail(_) => {
                    put(buf, x, y, &detail(app, s, rl, t, cols.end().saturating_sub(x) as usize), cols.end());
                    // Its question is this line: a click answers it, not just opens the session.
                    if s.helpers_wanted > 0 {
                        hit(app, row, ScreenHit::HelperAsk(term));
                    }
                }
                _ => {
                    put(buf, x, y, &rl.on(third(app, s, t), t), cols.end());
                }
            }
            continue;
        }
        draw_line(app, buf, model, line, cols, y, indent, pb, t, screen_w < NARROW);
    }
    let plain = Style::default().bg(pb);
    if scroll > 0 {
        put(buf, cols.rx + 1, top, &[seg("▲", plain.fg(t.muted))], cols.end());
    }
    if scroll + list_h < lines.len() {
        put(buf, cols.rx + 1, top + list_h as u16 - 1, &[seg("▼", plain.fg(t.muted))], cols.end());
    }
    draw_foot(app, buf, r, cols, pb, t);
    if let Some(gy) = gliding.filter(|gy| (top..top + list_h as u16).contains(gy))
        && app.screen.row_y.get(&focus.unwrap_or_default()) != Some(&gy)
    {
        tint_row(look, buf, cols.x, gy, cols.end() - cols.x, t.sel_bg(), pb);
    }
}

/// The first line shown: the focused (or cursor) row is kept in view when it changes;
/// otherwise the wheel rules.
fn side_scroll(app: &mut App, model: &[Proj], lines: &[Line], focus: Option<TermId>, list_h: usize) -> usize {
    let mut scroll = app.screen.side_scroll as usize;
    if app.screen.follow {
        let want = app.screen.cursor.or(focus).and_then(|term| lines.iter().position(|l| line_term(model, l) == Some(term)));
        if let Some(i) = want {
            if i < scroll {
                scroll = i.saturating_sub(1);
            } else if i + 2 >= scroll + list_h {
                scroll = i + 3 - list_h.min(i + 3);
            }
        }
        app.screen.follow = false;
    }
    scroll = scroll.min(lines.len().saturating_sub(list_h));
    app.screen.side_scroll = scroll as u16;
    scroll
}

/// The selected row (the keyboard's while the sidebar has the keys, else the open session) is
/// tinted and carries the bar, accent while the sidebar has the keys. While the keyboard browses,
/// the open row keeps a muted bar. A failed row carries a red bar whether selected or not.
fn row_look(s: &Session, picked: bool, open: bool, focused_side: bool, ground: Color, t: &Theme) -> RowLook {
    let bg = if picked { t.sel_bg() } else { ground };
    let bar = if check::shown(s).is_some_and(check::failed) {
        Some(t.err)
    } else if picked && focused_side {
        Some(t.accent)
    } else if picked || open {
        Some(t.muted)
    } else {
        None
    };
    RowLook { bg, picked, bar }
}

/// The title chip, "SESSIONS 5", and right of it ●n needing you (asking, or waiting for a check
/// to be approved) and ✕n failed checks, each only when there are any.
fn draw_head(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme, lit: bool, flat: bool) {
    let all: Vec<&Session> = model.iter().flat_map(|p| p.sessions()).collect();
    let needs = all.iter().filter(|s| check::needs_you(s)).count();
    let failed = all.iter().filter(|s| check::shown(s).is_some_and(check::failed)).count();
    let marks = check::marks(app);
    let ground = if flat { Style::default().bg(t.card) } else { Style::default() };
    let mut right = Vec::new();
    if needs > 0 {
        right.push(seg(format!("{}{needs}", marks.needs), ground.fg(t.blocked).add_modifier(Modifier::BOLD)));
    }
    if failed > 0 {
        let gap = if needs > 0 { " " } else { "" };
        right.push(seg(format!("{gap}{}{failed}", marks.failed), ground.fg(t.err).add_modifier(Modifier::BOLD)));
    }
    let rw = segs_width(&right);
    // A card's counts sit a space in from its corner, with a space either side.
    let rx = r.right().saturating_sub(if flat { 1 } else { 2 } + rw);
    let (chip_bg, ink, count) = if lit { (t.accent, t.acc_ink, t.acc_ink) } else { (t.btn, t.strong, t.text) };
    let chip = Style::default().bg(chip_bg);
    let host = crate::ipc::remote().map(|h| format!(" ⇄ {h}")).unwrap_or_default();
    let title = [seg(" SESSIONS", chip.fg(ink).add_modifier(Modifier::BOLD)), seg(format!(" {}{host} ", all.len()), chip.fg(count))];
    put(buf, r.x + if flat { 1 } else { 2 }, r.y, &title, rx.saturating_sub(1));
    if rw > 0 {
        let pad = if flat { vec![] } else { vec![seg(" ", ground)] };
        put(buf, rx.saturating_sub(pad.len() as u16), r.y, &[pad.clone(), right, pad].concat(), r.right());
    }
}

/// A terminal with nothing to report is marked by its prompt arrow, not an idle state.
pub(super) fn plain_shell(s: &Session) -> bool {
    !s.is_agent && !s.asleep && s.status == Status::None && check::shown(s).is_none()
}

/// A session's first line: its mark, its name and how long ago its state began.
#[allow(clippy::too_many_arguments)]
fn draw_session(app: &mut App, buf: &mut Buffer, s: &Session, cols: Cols, y: u16, indent: u16, rl: RowLook, t: &Theme, ages: bool) {
    let spinner = glyph(app, Status::Working);
    let mark = check::mark(app, s, t, &spinner);
    let state = check::kind(s);
    let st = Style::default().bg(rl.bg);
    let new = is_new(s);
    let hot = is_hot(s, state, new);
    let mark_colour = if new { t.accent } else { mark.color };
    let (g, gc) = if plain_shell(s) { ("›", t.muted) } else { (mark.glyph, rl.paint(t, mark_colour)) };
    let mut gs = st.fg(gc);
    if matches!(state, State::Failed | State::Needs | State::Approve | State::Working) || new {
        gs = gs.add_modifier(Modifier::BOLD);
    }
    let gx = cols.x + 2 + indent;
    // The 3-bar spinner is two cells wide; the name steps right to make room, as the handoff's does.
    let gw = (g.width() as u16).max(1);
    put(buf, gx, y, &[seg(g, gs)], gx + gw);
    hit(app, Rect { x: gx, y, width: gw + 1, height: 1 }, ScreenHit::Why(s.term));
    let menu = hovered(app, cols.row(y)) && !rl.picked;
    let when = if ages && s.is_agent && !menu { age(s.since) } else { String::new() };
    let nx = gx + gw + 1;
    let room = cols.rx.saturating_sub(nx).saturating_sub(when.width() as u16) as usize;
    let name = truncate(&s.name, room);
    // Name: strong and bold when the row wants a look, plain `text` once it is done and seen.
    let ns = st.fg(if rl.picked || hot { t.strong } else { t.text });
    let ns = if hot && !plain_shell(s) { ns.add_modifier(Modifier::BOLD) } else { ns };
    let left = if state == State::Working {
        working_text(app, &name, rl.paint(t, t.accent), t, ns)
    } else {
        vec![seg(name, ns)]
    };
    put(buf, nx, y, &left, cols.rx + 1);
    if !when.is_empty() {
        put(buf, (cols.rx + 1).saturating_sub(when.width() as u16), y, &[seg(when, st.fg(t.muted))], cols.rx + 1);
    }
    if menu {
        row_menu_button(app, buf, Rect { x: cols.rx, y, width: 1, height: 1 }, rl.bg, t, ScreenHit::RowMenuSess(s.term));
    }
}

/// A working agent's name or word: the working colour, with a bright band sweeping across it
/// while motion is on and standing still without.
fn working_text(app: &App, text: &str, base: Color, t: &Theme, st: Style) -> Vec<Seg> {
    if app.motion_on() {
        shimmer(text, app.spinner_frame(), base, blend(base, t.strong, 0.6), st)
    } else {
        vec![seg(text.to_string(), st.fg(base))]
    }
}

/// A session's second line, in `room` columns. With several checkpoints it is the headline
/// word, then a glyph per checkpoint; the glyphs give way, never the word.
fn detail(app: &App, s: &Session, rl: RowLook, t: &Theme, room: usize) -> Vec<Seg> {
    if let Some(receipt) = popover::receipt_segs(app, s.term, t) {
        return rl.on(receipt, t);
    }
    if s.helpers_wanted > 0 {
        return rl.on(vec![seg(truncate(&helpers_line_in(s.helpers_wanted, room), room), Style::default().fg(t.blocked))], t);
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
        Status::Working => {
            let mut words = vec![seg(format!("{} · ", s.agent), Style::default().fg(t.muted))];
            words.extend(working_text(app, "working", t.accent, t, Style::default()));
            // Only an agent that reports helpers has a count (Codex reports none).
            match agent_count(s) {
                0 => {}
                _ if !app.cfg.ui.subagent_count => {}
                n => words.push(seg(format!(" · {n} agent{}", if n == 1 { "" } else { "s" }), Style::default().fg(t.text))),
            }
            words
        }
        Status::Blocked => vec![seg("needs you", Style::default().fg(t.blocked))],
        Status::Done => vec![seg("done · not seen", Style::default().fg(t.done).add_modifier(Modifier::BOLD))],
        Status::Idle if is_new(s) => vec![seg(format!("{} · ", s.agent), Style::default().fg(t.muted)), seg("new · type to begin", Style::default().fg(t.accent))],
        Status::Idle if s.is_agent => muted("done"),
        _ => Vec::new(),
    }
}

/// A session's third line: its checkpoints by name, or a failed check's first failing line.
fn third(app: &App, s: &Session, t: &Theme) -> Vec<Seg> {
    let checks = check::reported(s);
    if checks.len() > 1 {
        return check::names(app, checks, t);
    }
    check::shown(s).map(|c| vec![seg(c.first_failing_line.clone(), Style::default().fg(t.muted))]).unwrap_or_default()
}

/// A line that is not part of a session's row.
#[allow(clippy::too_many_arguments)]
fn draw_line(app: &mut App, buf: &mut Buffer, model: &[Proj], line: &Line, cols: Cols, y: u16, indent: u16, pb: Color, t: &Theme, small: bool) {
    let row = cols.row(y);
    let x = cols.x + 4 + indent;
    let room = cols.end().saturating_sub(x) as usize;
    match line {
        Line::Proj(pi) => draw_proj(app, buf, model, *pi, cols, y, pb, t, small),
        Line::Note(text, c, term) => {
            if small {
                return;
            }
            put(buf, x, y, &[seg(truncate(text, room), Style::default().bg(pb).fg(*c).add_modifier(Modifier::ITALIC))], cols.end());
            hit(app, row, ScreenHit::Session(*term));
        }
        // Nothing running: one click starts a shell there.
        Line::Empty(pi) => {
            let st = Style::default().bg(pb);
            let nk = k(app, &Action::ShellHere);
            put(buf, cols.x + 2, y, &[seg("empty  ", st.fg(t.muted).add_modifier(Modifier::ITALIC)), seg(nk, st.fg(t.accent).add_modifier(Modifier::BOLD)), seg(" new shell", st.fg(t.muted))], cols.end());
            hit(app, row, ScreenHit::ShellIn(*pi));
        }
        // A quiet heading: "SSH 2 ────".
        Line::Section(kind, n) => {
            let st = Style::default().bg(pb);
            let label = format!("{} {n} ", kind.heading().to_uppercase());
            let rest = (cols.rx + 1).saturating_sub(cols.x + 1 + label.width() as u16);
            put(buf, cols.x + 1, y, &[seg(label, st.fg(t.muted).add_modifier(Modifier::BOLD)), seg("─".repeat(rest as usize), st.fg(t.line))], cols.rx + 1);
        }
        Line::Sess(..) | Line::Detail(_) | Line::Third(_) | Line::Gap => {}
    }
}

/// A folder's header: its fold arrow and name, and right-aligned what it is (branch, its
/// checkpoints), which gives way first.
#[allow(clippy::too_many_arguments)]
fn draw_proj(app: &mut App, buf: &mut Buffer, model: &[Proj], pi: usize, cols: Cols, y: u16, pb: Color, t: &Theme, small: bool) {
    let p = &model[pi];
    let row = cols.row(y);
    let open = !app.screen.saved.closed.contains(&format!("p:{}", p.key));
    let on = side_focused(app) && app.screen.cursor_proj.as_ref() == Some(&p.key);
    let hov = hovered(app, row);
    let bg = if on { t.sel_bg() } else { pb };
    if bg != pb {
        fill(buf, row, bg);
    }
    if on {
        check::edge(app, buf, cols.x, y, t.accent, bg);
    }
    // A selected folder row is one line, so it can end in a half-circle; the multi-line session
    // blocks would scallop, so they keep square ends. Text stops a cell short of the cap.
    let round = on && Look::of(&app.cfg.ui).round_tabs;
    let text_end = if round { row.right() - 1 } else { cols.rx + 1 };
    if round {
        buf[(text_end, y)].set_symbol(CAP_R).set_fg(bg).set_bg(pb);
    }
    let st = Style::default().bg(bg);
    let marks = check::marks(app);
    let fold = if open { marks.fold } else { marks.arrow };
    let name = truncate(&p.name, cols.rx.saturating_sub(cols.x + 2) as usize);
    let mut left = vec![seg(format!("{fold} "), st.fg(t.accent)), seg(name, st.fg(t.strong).add_modifier(Modifier::BOLD))];
    if p.fresh {
        left.push(seg(" ", st));
        left.extend(chip("NEW", Style::default().bg(t.accent).fg(t.acc_ink).add_modifier(Modifier::BOLD)));
    }
    let lx = cols.x + 1;
    put(buf, lx, y, &left, text_end);
    hit(app, row, ScreenHit::ToggleProj(pi));
    if hov && !small {
        row_glyph(app, buf, Rect { x: cols.rx.saturating_sub(2), y, width: 1, height: 1 }, "+", bg, t.accent, t, ScreenHit::ShellIn(pi));
        row_menu_button(app, buf, Rect { x: cols.rx, y, width: 1, height: 1 }, bg, t, ScreenHit::RowMenuProj(pi));
        return;
    }
    let room = text_end.saturating_sub(1).saturating_sub(lx + segs_width(&left));
    if room >= META_MIN_COLS {
        let meta = truncate(&proj_meta(app, p, open), room as usize);
        put(buf, text_end.saturating_sub(meta.width() as u16), y, &[seg(meta, st.fg(t.muted))], text_end);
    }
}

/// "⎇ main · check: test", "⎇ main · 2 checkpoints", "no git"; folded, how many it hides.
fn proj_meta(app: &App, p: &Proj, open: bool) -> String {
    if !open {
        let n = p.sessions().count();
        return if n == 0 { "empty".into() } else { format!("{n} hidden") };
    }
    let place = if p.kind == Kind::Ssh {
        String::new()
    } else if p.git {
        p.wts.iter().find(|w| w.main && !w.branch.is_empty()).map(|w| format!("{} {}", check::marks(app).branch, w.branch)).unwrap_or_default()
    } else {
        "no git".into()
    };
    let names = checkpoint_names(app, p);
    let checks = match names.as_slice() {
        [] => String::new(),
        [one] => format!("check: {one}"),
        many => format!("{} checkpoints", many.len()),
    };
    [place, checks].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
}

/// The project's checkpoints: the ones its config names, then any its sessions report.
fn checkpoint_names(app: &App, p: &Proj) -> Vec<String> {
    let mut names: Vec<String> = app.cfg.check.iter().filter(|c| crate::config::same_path(&c.project, &p.path)).map(|c| c.name().to_string()).collect();
    for c in p.sessions().flat_map(|s| &s.checks) {
        let name = check::name(c);
        if !names.iter().any(|n| n == name) {
            names.push(name.to_string());
        }
    }
    names
}

/// A `btn` chip for an action: its word with the letter of its leader key lit in the accent, bold,
/// or the key then the word when the key is not a letter of it. Its ends follow `ui.pill_caps`.
pub(super) fn key_chip(app: &App, word: &str, action: &Action, t: &Theme, outer: Color) -> Vec<Seg> {
    // A cap needs a raised ground to sit on: on the plain `bg` (which a transparent terminal
    // background hands back to the terminal) it would have none, so the chip stays square.
    let look = Look { caps: Look::of(&app.cfg.ui).caps && outer != Color::Reset && outer != t.bg, ..Look::of(&app.cfg.ui) };
    let key = k(app, action);
    let plain = Style::default().fg(t.text).bg(t.btn);
    let lit = Style::default().fg(t.accent).bg(t.btn).add_modifier(Modifier::BOLD);
    let first = word.chars().next();
    let body = if first.is_some_and(|c| key.eq_ignore_ascii_case(&c.to_string())) {
        lit_letter(word, first, plain, lit)
    } else {
        vec![seg(key, lit), seg(format!(" {word}"), plain)]
    };
    pill(look, body, t.btn, outer)
}

/// The foot, lifted a row off the bottom under a `line` divider, with no band behind it: a
/// `new` chip a column in and a `menu` chip at the right. Each lights its key's letter (the
/// leader key that does it); they drop from the end when the sidebar is too narrow.
fn draw_foot(app: &mut App, buf: &mut Buffer, r: Rect, cols: Cols, pb: Color, t: &Theme) {
    if r.height < 6 {
        return;
    }
    let y = r.bottom().saturating_sub(2);
    hline(buf, cols.x, y - 1, (cols.rx + 1).saturating_sub(cols.x), t, pb);
    let new = key_chip(app, "new", &Action::ShellHere, t, pb);
    let menu = key_chip(app, "menu", &Action::Menu, t, pb);
    let (nw, mw) = (segs_width(&new), segs_width(&menu));
    let nx = cols.x + 1;
    if nx + nw > cols.rx + 1 {
        return;
    }
    put(buf, nx, y, &new, cols.rx + 1);
    hit(app, Rect { x: nx, y, width: nw, height: 1 }, ScreenHit::NewShell);
    let mx = cols.rx.saturating_sub(mw);
    if mx > nx + nw {
        put(buf, mx, y, &menu, cols.rx);
        hit(app, Rect { x: mx, y, width: mw, height: 1 }, ScreenHit::Actions);
    }
}

/// The ⋯ that opens a sidebar row's menu (for terminals that keep right-clicks).
pub(in crate::client) fn row_menu_button(app: &mut App, buf: &mut Buffer, r: Rect, bg: Color, t: &Theme, h: ScreenHit) {
    row_glyph(app, buf, r, "⋯", bg, t.muted, t, h);
}

/// A small glyph button on a sidebar row (⋯, +): just the glyph on the row's own ground,
/// brightening when it's the one under the mouse.
#[allow(clippy::too_many_arguments)]
fn row_glyph(app: &mut App, buf: &mut Buffer, r: Rect, glyph: &str, bg: Color, fg: Color, t: &Theme, h: ScreenHit) {
    let mut st = Style::default().bg(bg).fg(if hovered(app, r) { t.strong } else { fg });
    if hovered(app, r) {
        st = st.add_modifier(Modifier::BOLD);
    }
    put(buf, r.x, r.y, &[seg(glyph, st)], r.right());
    hit(app, r, h);
}

#[cfg(test)]
mod frame_tests {
    use crate::client::v7_fixture::{LAYOUTS, Layout, SIZES, STAGES, Stage, frame, world};

    /// The v7 reference's sidebar rows, as the fixture should draw them (see the file's header).
    const FRAMES: &str = include_str!("side_frames.txt");
    /// A cell the reference leaves to the app: the working spinner's frame.
    const ANY: char = '◌';

    struct Want {
        counts: String,
        rows: Vec<String>,
        foot: String,
    }

    fn wanted(stage: Stage, layout: Layout, (w, h): (u16, u16), glyphs: &str) -> Option<Want> {
        let head = format!("===== {stage:?} | {} | {w}x{h} | {glyphs}", format!("{layout:?}").to_lowercase());
        let mut block = FRAMES.lines().skip_while(|l| *l != head).skip(1);
        let counts = block.next()?.strip_prefix("+++++ ")?.to_string();
        let rows: Vec<String> = block.by_ref().take_while(|l| !l.starts_with("-----")).map(String::from).collect();
        let foot = FRAMES.lines().skip_while(|l| *l != head).find_map(|l| l.strip_prefix("----- "))?.to_string();
        (!rows.is_empty()).then_some(Want { counts, rows, foot })
    }

    /// The title's counts: right-aligned a column in from the corner on a card, at the edge in Flat.
    fn counts(text: &str, r: ratatui::layout::Rect, flat: bool, n: usize) -> String {
        let row: Vec<char> = text.lines().nth(r.y as usize).unwrap_or_default().chars().collect();
        let end = (r.right() - if flat { 1 } else { 2 }) as usize;
        (end - n..end).map(|x| row.get(x).copied().unwrap_or(' ')).collect()
    }

    /// The sidebar's inner rows: columns X-1..RX+1 of a card (X..RX in Flat), between its border rows.
    fn inner(text: &str, r: ratatui::layout::Rect, flat: bool) -> Vec<String> {
        let last = if flat { r.bottom() - 1 } else { r.bottom() - 2 };
        let lines: Vec<Vec<char>> = text.lines().map(|l| l.chars().collect()).collect();
        (r.y + 1..=last)
            .map(|y| {
                let row = &lines[y as usize];
                (r.x + 1..r.right() - 1).map(|x| row.get(x as usize).copied().unwrap_or(' ')).collect::<String>().trim_end().to_string()
            })
            .collect()
    }

    /// The reference's folders in the order the app lists them: a folder with a session that
    /// needs you goes first, where the handoff's sample keeps its own order.
    fn in_app_order(rows: &[String], got: &[String]) -> Vec<String> {
        let blocks: Vec<&[String]> = rows.split(String::is_empty).filter(|b| !b.is_empty()).collect();
        let name = |b: &[String]| b[0].split_whitespace().nth(1).unwrap_or_default().to_string();
        let at = |b: &[String]| got.iter().position(|g| g.split_whitespace().nth(1) == Some(name(b).as_str())).unwrap_or(usize::MAX);
        let mut sorted = blocks.clone();
        sorted.sort_by_key(|b| at(b));
        let mut out = vec![String::new()];
        for b in sorted {
            out.extend(b.iter().cloned());
            out.push(String::new());
        }
        while out.last().is_some_and(String::is_empty) {
            out.pop();
        }
        out
    }

    fn same(want: &str, got: &str) -> bool {
        let (w, g): (Vec<char>, Vec<char>) = (want.chars().collect(), got.chars().collect());
        w.len() == g.len() && w.iter().zip(&g).all(|(a, b)| a == b || *a == ANY)
    }

    fn check(stage: Stage, layout: Layout, size: (u16, u16), glyphs: &str) {
        let want = wanted(stage, layout, size, glyphs).unwrap_or_else(|| panic!("no reference for {stage:?} {layout:?} {size:?} {glyphs}"));
        let mut app = world(stage);
        let text = frame(&mut app, layout, size.0, size.1, glyphs);
        let got = inner(&text, app.screen.side_rect, layout == Layout::Flat);
        let want = Want { rows: in_app_order(&want.rows, &got), ..want };
        let n = want.rows.len();
        let ok = got.len() > n
            && want.rows.iter().zip(&got).all(|(w, g)| same(w, g))
            && got[n..got.len() - 1].iter().all(String::is_empty)
            && got.last() == Some(&want.foot);
        assert!(ok, "{stage:?} {layout:?} {size:?} {glyphs}\nwant:\n{}\n----- {}\ngot:\n{}", want.rows.join("\n"), want.foot, got.join("\n"));
        let flat = layout == Layout::Flat;
        assert!(text.lines().nth(app.screen.side_rect.y as usize).is_some_and(|l| l.contains(" SESSIONS 5 ")), "{stage:?} {layout:?} {size:?}: the title");
        assert_eq!(counts(&text, app.screen.side_rect, flat, want.counts.chars().count()), want.counts, "{stage:?} {layout:?} {size:?} {glyphs}: the title's counts");
    }

    fn rows_at(stage: Stage, layout: Layout, w: u16, h: u16) -> Vec<String> {
        let mut app = world(stage);
        let text = frame(&mut app, layout, w, h, "unicode");
        inner(&text, app.screen.side_rect, layout == Layout::Flat)
    }

    fn has(rows: &[String], needle: &str) -> bool {
        rows.iter().any(|r| r.contains(needle))
    }

    #[test]
    fn rows_give_way_in_order_meta_age_third_line_then_quiet_second_lines() {
        let at = |w| rows_at(Stage::Failed, Layout::Floating, w, 30);
        let wide = at(160);
        assert!(has(&wide, "⎇ main · ch…") && has(&wide, "2m") && has(&wide, "✗ src/pages") && has(&wide, "claude · working"), "{wide:#?}");
        let narrow = at(140);
        assert!(has(&narrow, "✗ src/pages") && has(&narrow, "claude · working"), "140 columns are still wide: {narrow:#?}");
        let slim = at(139);
        assert!(!has(&slim, "⎇ main") && has(&slim, "2m"), "the meta goes first, the age stays: {slim:#?}");
        assert!(!has(&slim, "✗ src/pages") && !has(&slim, "claude · working"), "then line 3 and quiet second lines: {slim:#?}");
        assert!(has(&slim, "failed: test · exit") && has(&slim, "asks: use pnpm"), "a result and a question keep theirs: {slim:#?}");
        assert!(has(&at(100), "2m") && !has(&at(99), "2m"), "the age goes below 100 columns");
    }

    #[test]
    fn several_checkpoints_clip_their_glyphs_never_their_word() {
        let row_for = |name: &str| {
            let mut app = world(Stage::Multi);
            app.snap.terms.get_mut(&1).unwrap().checks[1].name = name.into();
            let text = frame(&mut app, Layout::Flat, 80, 24, "unicode");
            let rows = inner(&text, app.screen.side_rect, true);
            rows.iter().find(|r| r.contains("failed:")).cloned().unwrap_or_default()
        };
        assert_eq!(row_for("e2e").trim_start_matches(['▎', ' ']), "failed: e2e ✓✕", "room for both");
        assert_eq!(row_for("integration").trim_start_matches(['▎', ' ']), "failed: integration", "the glyphs give way");
        assert_eq!(row_for("integration-suite").trim_start_matches(['▎', ' ']), "failed: integration…", "a word that can't fit is cut, not dropped");
    }

    /// The working row's first line and its second, with the Appearance spinner `frames` in
    /// glyph set `glyphs`.
    fn working_rows(frames: &[&str], glyphs: &str, layout: Layout) -> (String, String) {
        let mut app = world(Stage::None);
        app.cfg.ui.spinner = frames.iter().map(|f| f.to_string()).collect();
        let text = frame(&mut app, layout, 160, 45, glyphs);
        let rows = inner(&text, app.screen.side_rect, layout == Layout::Flat);
        let at = rows.iter().position(|r| r.contains("add-payments")).expect("the working row");
        (rows[at].trim().trim_start_matches('▎').trim().to_string(), rows[at + 1].trim().to_string())
    }

    #[test]
    fn the_bars_spinner_marks_a_working_row_and_the_name_steps_right_for_it() {
        let (first, second) = working_rows(crate::config::BARS_FRAMES, "unicode", Layout::Floating);
        assert!(first.starts_with("⣶⡆ add-payments"), "motion off holds the third frame: {first:?}");
        assert_eq!(second, "claude · working");
        let (one_cell, _) = working_rows(&["|"], "unicode", Layout::Floating);
        assert!(one_cell.starts_with("| add-payments"), "a one-column choice keeps the name where it was: {one_cell:?}");
        let (flat, _) = working_rows(crate::config::BARS_FRAMES, "unicode", Layout::Flat);
        assert!(flat.starts_with("⣶⡆ add-payments"), "{flat:?}");
    }

    #[test]
    fn the_ascii_glyph_set_draws_the_bars_in_ascii_and_leaves_other_choices_alone() {
        let (bars, _) = working_rows(crate::config::BARS_FRAMES, "ascii", Layout::Floating);
        assert!(bars.is_ascii() && bars.starts_with("O  add-payments"), "{bars:?}");
        let (custom, _) = working_rows(&["a", "b", "c"], "ascii", Layout::Floating);
        assert!(custom.starts_with("c add-payments"), "a custom choice is the user's: {custom:?}");
    }

    #[test]
    fn the_working_mark_is_bold_and_its_name_holds_one_colour_without_motion() {
        use ratatui::style::Modifier;
        let mut app = world(Stage::None);
        app.cfg.ui.spinner = crate::config::BARS_FRAMES.iter().map(|f| f.to_string()).collect();
        let buf = crate::client::v7_fixture::cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let (gx, gy) = (0..45u16).flat_map(|y| (0..160u16).map(move |x| (x, y))).find(|&(x, y)| buf[(x, y)].symbol() == "⣶").expect("the mark");
        assert!(buf[(gx, gy)].modifier.contains(Modifier::BOLD), "the mark is bold");
        let name: Vec<_> = (gx + 3..gx + 3 + "add-payments".len() as u16).map(|x| buf[(x, gy)].fg).collect();
        assert!(name.windows(2).all(|w| w[0] == w[1]), "no shimmer band without motion: {name:?}");
    }

    #[test]
    fn the_sidebar_rows_match_the_v7_reference_in_every_layout_and_size() {
        for stage in STAGES {
            for layout in LAYOUTS {
                for size in SIZES {
                    check(stage, layout, size, "unicode");
                }
            }
            check(stage, Layout::Flat, (80, 24), "ascii");
        }
    }
}

#[cfg(test)]
mod v8_tests {
    use super::*;
    use crate::client::v7_fixture::{LAYOUTS, Layout, Stage, cells, raw_text, world};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::style::Modifier;

    fn side_rows(app: &mut App, layout: Layout, w: u16, h: u16) -> Vec<String> {
        let text = raw_text(app, layout, w, h, "unicode");
        let r = app.screen.side_rect;
        text.lines().map(|l| l.chars().skip(r.x as usize).take(r.width as usize).collect::<String>()).collect()
    }

    fn has(rows: &[String], needle: &str) -> bool {
        rows.iter().any(|r| r.contains(needle))
    }

    /// A sidebar row's words, without its border, bar and padding.
    fn said(row: &str) -> &str {
        row.trim_matches(|c| "│▎ ".contains(c))
    }

    /// The cell where `needle` starts on the first row that has it, among the sidebar's rows.
    fn find(buf: &ratatui::buffer::Buffer, app: &App, needle: &str) -> (u16, u16) {
        let r = app.screen.side_rect;
        (r.y..r.bottom())
            .find_map(|y| {
                let row: String = (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect();
                row.find(needle).map(|at| (r.x + row[..at].chars().count() as u16, y))
            })
            .unwrap_or_else(|| panic!("{needle:?} is not in the sidebar"))
    }

    #[test]
    fn line_two_says_what_each_state_is_doing() {
        let mut app = world(Stage::None);
        app.snap.terms.get_mut(&2).unwrap().subagents = vec!["a".into(), "b".into()];
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(has(&rows, "claude · working · 2 agents"), "a count when the agent reports helpers: {rows:#?}");
        app.cfg.ui.subagent_count = false;
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(has(&rows, "claude · working") && !has(&rows, "· 2 agents"), "Settings > Agents can turn the count off: {rows:#?}");
        app.cfg.ui.subagent_count = true;
        assert!(has(&rows, "done · not seen"), "{rows:#?}");
        assert!(has(&rows, "asks: use pnpm or npm?"), "{rows:#?}");
        app.snap.terms.get_mut(&2).unwrap().subagents.clear();
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(has(&rows, "claude · working") && !has(&rows, "agents"), "no count when none is reported: {rows:#?}");
        // Seen and finished reads `done`; one that has been told nothing is new.
        app.snap.terms.get_mut(&4).unwrap().status = Status::Idle;
        let new = app.snap.terms.get_mut(&5).unwrap();
        (new.asleep, new.status, new.label) = (false, Status::Idle, String::new());
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(rows.iter().any(|r| said(r) == "done"), "{rows:#?}");
        assert!(has(&rows, "codex · new · type to begin"), "{rows:#?}");
    }

    #[test]
    fn only_a_finished_seen_row_collapses_below_140_columns() {
        let mut app = world(Stage::None);
        app.snap.terms.get_mut(&4).unwrap().status = Status::Idle;
        for layout in LAYOUTS {
            let rows = side_rows(&mut app, layout, 100, 30);
            assert!(has(&rows, "claude · working"), "{layout:?}: a working row keeps its line: {rows:#?}");
            assert!(has(&rows, "asks: use pnpm"), "{layout:?}: {rows:#?}");
            assert!(!rows.iter().any(|r| said(r) == "done"), "{layout:?}: a plain done row is one line: {rows:#?}");
            let rows = side_rows(&mut app, layout, 160, 45);
            assert!(rows.iter().any(|r| said(r) == "done"), "{layout:?}: from 140 columns it has its line: {rows:#?}");
        }
    }

    #[test]
    fn a_row_that_wants_a_look_is_bold_and_a_done_one_is_plain() {
        let mut app = world(Stage::None);
        app.snap.terms.get_mut(&5).unwrap().asleep = false;
        app.snap.terms.get_mut(&5).unwrap().status = Status::Idle;
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let t = app.theme.clone();
        let style = |name: &str| {
            let (x, y) = find(&buf, &app, name);
            (buf[(x, y)].modifier.contains(Modifier::BOLD), buf[(x, y)].fg)
        };
        assert_eq!(style("warm-finch"), (true, t.strong), "needs you: strong and bold");
        assert_eq!(style("quick-fox"), (true, t.strong), "done, not seen: strong and bold");
        assert_eq!(style("old-moth"), (false, t.text), "done: text, not bold");
        let (x, y) = find(&buf, &app, "done · not seen");
        assert_eq!((buf[(x, y)].fg, buf[(x, y)].modifier.contains(Modifier::BOLD)), (t.done, true), "line two of an unseen row is bold green");
        let (x, y) = find(&buf, &app, "asks:");
        assert_eq!(buf[(x, y)].fg, t.blocked, "the question is amber");
    }

    #[test]
    fn the_foot_is_two_chips_lifted_off_the_bottom_under_a_divider_with_no_band() {
        for layout in LAYOUTS {
            let mut app = world(Stage::None);
            let buf = cells(&mut app, layout, 160, 45, "unicode");
            let (r, t) = (app.screen.side_rect, app.theme.clone());
            let foot_y = r.bottom() - 2;
            let row = |y: u16| -> String { (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect() };
            assert!(row(foot_y).contains("new") && row(foot_y).contains("menu"), "{layout:?}: {:?}", row(foot_y));
            assert!(row(foot_y - 1).contains("────"), "{layout:?}: a divider above: {:?}", row(foot_y - 1));
            let line_fg = (r.x..r.right()).find(|x| buf[(*x, foot_y - 1)].symbol() == "─").map(|x| buf[(x, foot_y - 1)].fg);
            assert_eq!(line_fg, Some(t.line), "{layout:?}: the divider is `line`");
            // No band: the cells between the chips are the sidebar's own ground.
            let (n, _) = find(&buf, &app, "new");
            let (m, _) = find(&buf, &app, "menu");
            assert_eq!(buf[(n + 8, foot_y)].bg, buf[(n + 8, foot_y - 2)].bg, "{layout:?}: no band behind the foot");
            assert!(m > n + 8, "{layout:?}: menu is at the right");
            // Key letters lit in the accent, bold; the rest in text; chips raised.
            for (word, x) in [("new", n), ("menu", m)] {
                let lit = &buf[(x, foot_y)];
                assert_eq!((lit.fg, lit.bg, lit.modifier.contains(Modifier::BOLD)), (t.accent, t.btn, true), "{layout:?}: the {word} chip's key letter");
                assert_eq!(buf[(x + 1, foot_y)].fg, t.text, "{layout:?}: the rest of {word}");
            }
            let hits = |h: ScreenHit| app.hits.iter().any(|(rect, hit)| *hit == crate::client::Hit::Screen(h) && rect.y == foot_y);
            assert!(hits(ScreenHit::NewShell) && hits(ScreenHit::Actions), "{layout:?}: both chips are clickable");
        }
    }

    #[test]
    fn z_collapses_into_the_strip_and_capital_z_hides_and_each_restores() {
        let mut app = world(Stage::None);
        let area = ratatui::layout::Rect::new(0, 0, 120, 30);
        assert!(app.sidebar && !app.side_hidden);
        app.act(Action::ToggleSidebar);
        app.side_frac = 0.0;
        assert!(!app.sidebar && !app.side_hidden && super::super::draw::strip_on(&app), "z: the strip");
        assert_eq!(super::super::draw::grid(&app, area).panes.y, 1, "the panes start under the strip");
        app.act(Action::HideSidebar);
        app.side_frac = 0.0;
        assert!(app.side_hidden && !super::super::draw::strip_on(&app), "Z: hidden");
        let g = super::super::draw::grid(&app, area);
        assert_eq!((g.panes.x, g.panes.y, g.panes.width), (0, 0, 120), "hidden gives the panes everything");
        app.act(Action::ToggleSidebar);
        assert!(app.sidebar && !app.side_hidden, "z restores the sidebar");
        app.act(Action::HideSidebar);
        app.act(Action::HideSidebar);
        assert!(app.sidebar && !app.side_hidden, "Z again restores it");
    }

    #[test]
    fn the_strip_switches_with_arrows_and_digits_only_while_it_has_the_keys() {
        let mut app = world(Stage::None);
        let (w, h) = (160, 45);
        app.sidebar = false;
        let _ = raw_text(&mut app, Layout::Floating, w, h, "unicode");
        let strip: String = raw_text(&mut app, Layout::Floating, w, h, "unicode").lines().next().unwrap().into();
        for name in ["fix-footer", "add-payments", "warm-finch", "quick-fox", "old-moth"] {
            assert!(strip.contains(name), "{name} is a chip: {strip}");
        }
        assert!(strip.trim_start().starts_with('»'), "the expand chip comes first: {strip}");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        // The pane has the keys: a digit is typed into it, nothing switches.
        app.mode = Mode::Normal;
        app.on_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert!(!std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, crate::protocol::ClientMsg::Command(crate::protocol::Command::FocusPane { .. }))));
        // The strip has them: 1–9 jump, ← → step.
        app.act(Action::BrowseTree);
        let _ = raw_text(&mut app, Layout::Floating, w, h, "unicode");
        app.on_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert_eq!(app.screen.cursor, app.screen.side_items.iter().filter_map(|i| if let SideItem::Sess(t) = i { Some(*t) } else { None }).nth(2));
        assert_eq!(app.mode, Mode::Side, "the keys stay on the strip");
        let third = app.screen.cursor;
        app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_ne!(app.screen.cursor, third);
        app.on_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.screen.cursor, third);
        assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, crate::protocol::ClientMsg::Command(crate::protocol::Command::FocusPane { .. }))), "the session opens in the pane");
    }

    #[test]
    fn the_selected_chip_is_accent_with_ink_when_the_strip_has_the_keys_else_raised() {
        let mut app = world(Stage::None);
        app.sidebar = false;
        let t = app.theme.clone();
        let chip = |app: &mut App| {
            let buf = cells(app, Layout::Floating, 160, 45, "unicode");
            let x = (0..160u16).find(|x| (0..3).map(|i| buf[(x + i, 0)].symbol()).collect::<String>() == "fix").expect("the chip");
            (buf[(x, 0)].bg, buf[(x, 0)].fg)
        };
        chip(&mut app);
        app.act(Action::BrowseTree);
        assert_eq!(chip(&mut app), (t.accent, t.acc_ink), "the strip has the keys");
        app.mode = Mode::Normal;
        app.screen.cursor = None;
        assert_eq!(chip(&mut app).0, t.btn, "else raised");
    }
}
