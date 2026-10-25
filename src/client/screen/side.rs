//! The sidebar card: projects, their sessions in one to three lines each, and the lines they
//! are drawn from. Its rows are held to the reference frames (`side_frames.txt`).

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

/// A session has a second line only when it says more than the row does: the mark and the
/// name's colour already say working, needs you, done or idle. What is left: a question, a check
/// result, helpers it asks for, a wait for a plan limit, a new agent's first step, asleep, a
/// quiet spell, its helpers at work, or a message just sent.
fn has_detail(app: &App, s: &Session) -> bool {
    let helpers_at_work = app.cfg.ui.subagent_count && s.status == Status::Working && agent_count(s) > 0;
    s.asleep
        || is_new(s)
        || check::shown(s).is_some()
        || s.question.is_some()
        || s.resume_at.is_some()
        || s.helpers_wanted > 0
        || (s.status == Status::Working && s.quiet_since.is_some())
        || helpers_at_work
        || app.screen.receipts.contains_key(&s.term)
}

/// A finished agent with nothing to say: its row is one line, since its `○` already says idle.
/// Anything that needs a look (working, asking, new, a check result, a wait for a plan limit)
/// keeps its second.
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

pub(in crate::client) fn session_lines(app: &App, s: &Session, t: &Theme, out: &mut Vec<Line>) {
    let notes_c = t.muted;
    if has_detail(app, s) {
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
            session_lines(app, s, t, &mut out);
        }
        if !any {
            out.push(Line::Empty(pi));
        }
        out.push(Line::Gap);
    }
    out
}

/// The lines drawn at `screen_w` columns. A finished, seen row is one line (an asleep one only
/// below `NARROW`, where the third line goes too). A row that needs a look keeps its second.
fn fitted_lines(app: &App, model: &[Proj], t: &Theme, screen_w: u16) -> Vec<Line> {
    let mut lines = side_lines(app, model, t);
    lines.retain(|l| match l {
        Line::Third(_) => screen_w >= NARROW,
        Line::Detail(term) => {
            find(model, *term).is_some_and(|(.., s)| !quiet_done(s) || (s.asleep && screen_w >= NARROW)) || app.screen.receipts.contains_key(term)
        }
        _ => true,
    });
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

/// Where the sidebar's rows go (the reference's X and RX): the selection bar's column, and the
/// last column right-aligned text reaches. Tints and clipped lines run one past it.
#[derive(Debug, Clone, Copy)]
struct Cols {
    x: u16,
    rx: u16,
    flat: bool,
    /// Rounded rows end in a cap on the last cell of their tint, so nothing else is drawn there.
    cap: bool,
    /// A folder row starts a cell before X, so its fold button keeps X and its left end is a
    /// blank cell that can take a cap: with caps on, unless that cell is a Flat zone's left rule.
    lead: bool,
}

impl Cols {
    /// `ruled`: a Flat zone has a `│` rule down its left (`card::ruled`).
    fn of(r: Rect, flat: bool, cap: bool, ruled: bool) -> Cols {
        let (left, right) = if flat { (1, 2) } else { (2, 3) };
        // In Flat the tint ends on RX itself, so right-aligned text steps back a cell for the cap.
        let rx = r.right().saturating_sub(right + u16::from(cap && flat));
        Cols { x: r.x + left, rx, flat, cap, lead: cap && !(flat && ruled) }
    }

    /// A folder's row: `row`, a cell longer on the left when it can end in a cap there.
    fn proj_row(self, y: u16) -> Rect {
        let lead = u16::from(self.lead);
        Rect { x: self.x - lead, width: self.row(y).width + lead, ..self.row(y) }
    }

    /// The end (exclusive) of a row's tint: a column past RX inside a card; in Flat that
    /// column is the pane's divider.
    fn tint_end(self) -> u16 {
        self.rx + if self.flat { 1 + u16::from(self.cap) } else { 2 }
    }

    /// The end (exclusive) of anything drawn on a row, so a cut line's `…` stays in sight. A cap
    /// takes the last cell of the tint.
    fn end(self) -> u16 {
        self.tint_end() - u16::from(self.cap)
    }

    fn row(self, y: u16) -> Rect {
        Rect { x: self.x, y, width: self.tint_end().saturating_sub(self.x), height: 1 }
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
    let cols = Cols::of(r, look.flat, look.round_rows, ruled(app, buf, r));
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
            && let Some((_, wt, s)) = find(model, term)
        {
            let rl = *looks.entry(term).or_insert_with(|| {
                let picked = if focused_side { app.screen.cursor == Some(term) } else { Some(term) == focus };
                row_look(picked, pb, t)
            });
            if rl.bg != pb {
                fill(buf, row, rl.bg);
            }
            if look.round_rows && rl.bg != pb && row.width > 2 {
                // A one-line row is a pill. A block of lines keeps square ends: caps would
                // scallop it and quarter-cell corners notch it, so its lines meet as one block.
                let first = (scroll..=i).rev().find(|&j| matches!(lines[j], Line::Sess(..)) || j == scroll).unwrap_or(i);
                if row_rect(first, top + (first - scroll) as u16).height == 1 {
                    buf[(row.x, y)].set_symbol(CAP_L).set_fg(rl.bg).set_bg(pb);
                    buf[(row.right() - 1, y)].set_symbol(CAP_R).set_fg(rl.bg).set_bg(pb);
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
                    app.screen.row_y.insert(term, y);
                    let keyboard = focused_side && app.screen.cursor == Some(term);
                    if keyboard && Some(term) != focus && let Some(gy) = app.motion.glide("side", u64::from(term), y, on) {
                        gliding = Some(gy);
                    }
                    draw_session(app, buf, s, app.screen.lines_changed.get(&wt.path).copied(), cols, y, indent, rl, t, screen_w >= AGES_FROM);
                }
                Line::Detail(_) => {
                    put(buf, x, y, &quiet(app, s, detail(app, s, rl, t, cols.end().saturating_sub(x) as usize), t), cols.end());
                    // Its question is this line: a click answers it, not just opens the session.
                    if s.helpers_wanted > 0 {
                        hit(app, row, ScreenHit::HelperAsk(term));
                    }
                }
                _ => {
                    put(buf, x, y, &quiet(app, s, rl.on(third(app, s, t), t), t), cols.end());
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
        tint_row(look, buf, cols.x, gy, cols.tint_end() - cols.x, t.sel_bg(), pb);
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
/// tinted and carries an accent bar; no other row has one.
fn row_look(picked: bool, ground: Color, t: &Theme) -> RowLook {
    let bg = if picked { t.sel_bg() } else { ground };
    let bar = picked.then_some(t.accent);
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
    let mut title = vec![seg(" SESSIONS", chip.fg(ink).add_modifier(Modifier::BOLD)), seg(format!(" {}{host} ", all.len()), chip.fg(count))];
    if Look::of(&app.cfg.ui).caps {
        title = round_ends(title, if flat { t.card } else { Color::Reset });
    }
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

/// The mark for the agent a session runs, so a row tells Claude from Codex at a glance: a shape,
/// or a letter with the ASCII glyph set. An agent with no mark of its own shows its first letter.
fn agent_mark(agent: &str, ascii: bool) -> String {
    let (shape, letter) = match agent {
        "claude" => ("✻", "C"),
        "codex" => ("◆", "X"),
        "gemini" => ("✦", "G"),
        "opencode" => ("◇", "O"),
        other => return other.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default(),
    };
    (if ascii { letter } else { shape }).into()
}

/// The least room a name keeps before the row's right-hand marks give way.
const NAME_MIN: usize = 8;
/// Context shows on the row once it is past this full, so a row stays quiet until it matters.
const CONTEXT_SHOWN_ABOVE: u8 = 50;
/// From this full the context reads in `err`: the agent is about to compact or stall.
const CONTEXT_NEARLY_FULL: u8 = 85;

/// The context a row shows (`72` for `72%`) and whether it is nearly full, once past half
/// full. Rounded first, so the number drawn is the number judged.
fn context_shown(context: Option<f32>) -> Option<(u8, bool)> {
    let pct = context?.clamp(0.0, 100.0).round() as u8;
    (pct > CONTEXT_SHOWN_ABOVE).then_some((pct, pct >= CONTEXT_NEARLY_FULL))
}

/// The right of a session's row, at most `room` wide: its agent's mark, its context once past
/// half full, its checkout's uncommitted lines (`+N −N`) and its age. The lines give way
/// first, then the mark, then the context. Also where the lines sit in it, as (offset,
/// width), when they are shown.
fn row_meta(s: &Session, ascii: bool, changed: Option<(i64, i64)>, when: &str, room: usize, st: Style, t: &Theme) -> (Vec<Seg>, Option<(u16, u16)>) {
    let mark = if s.is_agent { agent_mark(&s.agent, ascii) } else { String::new() };
    // Never amber: amber is for needs you.
    let context = context_shown(s.context).map(|(pct, full)| seg(format!("{pct}%"), st.fg(if full { t.err } else { t.muted })));
    let lines: Vec<Seg> = match changed {
        Some((a, r)) if a > 0 || r > 0 => vec![seg(format!("+{a}"), st.fg(t.done)), seg(format!(" −{r}"), st.fg(t.err))],
        _ => Vec::new(),
    };
    let gap = |v: &mut Vec<Seg>| {
        if !v.is_empty() {
            v.push(seg("  ", st));
        }
    };
    let build = |with_mark: bool, with_context: bool, with_lines: bool| {
        let mut v: Vec<Seg> = Vec::new();
        let mut span = None;
        if with_mark && !mark.is_empty() {
            v.push(seg(mark.clone(), st.fg(t.muted)));
        }
        if let Some(c) = context.as_ref().filter(|_| with_context) {
            // The agent's mark and its context read as one: `✻ 72%`.
            if !v.is_empty() {
                v.push(seg(" ", st));
            }
            v.push(c.clone());
        }
        if with_lines && !lines.is_empty() {
            gap(&mut v);
            span = Some((segs_width(&v), segs_width(&lines)));
            v.extend(lines.iter().cloned());
        }
        if !when.is_empty() {
            gap(&mut v);
            v.push(seg(when.to_string(), st.fg(t.muted)));
        }
        (v, span)
    };
    [(true, true, true), (true, true, false), (false, true, false), (false, false, false)]
        .into_iter()
        .map(|(m, c, l)| build(m, c, l))
        .find(|(v, _)| segs_width(v) as usize <= room)
        .unwrap_or_default()
}

/// A session's first line: its mark, its name in its state's colour, and at the right its
/// agent, its uncommitted lines and how long ago its state began.
#[allow(clippy::too_many_arguments)]
fn draw_session(app: &mut App, buf: &mut Buffer, s: &Session, changed: Option<(i64, i64)>, cols: Cols, y: u16, indent: u16, rl: RowLook, t: &Theme, ages: bool) {
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
    // The 3-bar spinner is two cells wide; the name steps right to make room, as the reference's does.
    let gw = (g.width() as u16).max(1);
    put(buf, gx, y, &[seg(g, gs)], gx + gw);
    hit(app, Rect { x: gx, y, width: gw + 1, height: 1 }, ScreenHit::Why(s.term));
    let menu = hovered(app, cols.row(y)) && !rl.picked;
    let when = if ages && s.is_agent { age(s.since) } else { String::new() };
    let nx = gx + gw + 1;
    // The hovered row's ⋯ takes the last column, a space before it: over the age's place (at
    // least two wide), else in two columns of its own. A row with `+N −N` and no age keeps
    // those two free unhovered too, so the counts never move from under the pointer.
    let counts = changed.is_some_and(|(a, r)| a > 0 || r > 0);
    let meta_end = if when.is_empty() && (menu || counts) { cols.rx.saturating_sub(1) } else { cols.rx + 1 };
    let when = if menu { " ".repeat(when.width()) } else { when };
    let free = meta_end.saturating_sub(nx) as usize;
    let ascii = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons).needs.is_ascii();
    let (meta, lines_at) = row_meta(s, ascii, changed, &when, free.saturating_sub(NAME_MIN + 1), st, t);
    let mw = segs_width(&meta);
    let room = free.saturating_sub(if mw > 0 { mw as usize + 1 } else { 0 });
    let name = truncate(&s.name, room);
    // A name in a state worth a look (working, checking, finished and unseen, needs you) takes
    // the state's colour, and moves with motion on; a calm one is `text`, or `strong` when picked.
    let coloured = !plain_shell(s) && NameMotion::of(s, state) != NameMotion::Still;
    let colour = if coloured { gc } else if rl.picked || (hot && !plain_shell(s)) { t.strong } else { t.text };
    let ns = if hot && !plain_shell(s) { st.add_modifier(Modifier::BOLD) } else { st };
    let anim = if coloured && app.motion_on() { NameMotion::of(s, state) } else { NameMotion::Still };
    let left = name_segs(&name, anim, app.spinner_frame(), colour, t.strong, ns);
    put(buf, nx, y, &left, cols.rx + 1);
    if mw > 0 {
        let mx = meta_end.saturating_sub(mw);
        put(buf, mx, y, &meta, meta_end);
        if let Some((at, width)) = lines_at {
            hit(app, Rect { x: mx + at, y, width, height: 1 }, ScreenHit::RowChanges(s.term));
        }
    }
    if menu {
        row_menu_button(app, buf, Rect { x: cols.rx, y, width: 1, height: 1 }, rl.bg, t, ScreenHit::RowMenuSess(s.term));
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
fn name_segs(text: &str, motion: NameMotion, frame: u64, base: Color, bright: Color, st: Style) -> Vec<Seg> {
    match motion {
        NameMotion::Still => vec![seg(text.to_string(), st.fg(base))],
        NameMotion::Shimmer => shimmer(text, frame, base, blend(base, bright, 0.6), st),
        NameMotion::Pulse => {
            let half = PULSE_FRAMES / 2;
            let k = (frame % PULSE_FRAMES).abs_diff(half) as f32 / half as f32;
            vec![seg(text.to_string(), st.fg(blend(base, bright, (1.0 - k) * PULSE_PEAK)))]
        }
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
        // The spinner and the name's colour already say working: only its helpers and a quiet
        // spell are left to say.
        Status::Working => {
            let mut words: Vec<Seg> = Vec::new();
            // Only an agent that reports helpers has a count (Codex reports none).
            match agent_count(s) {
                0 => {}
                _ if !app.cfg.ui.subagent_count => {}
                n => words.push(seg(format!("{n} agent{}", if n == 1 { "" } else { "s" }), Style::default().fg(t.text))),
            }
            if let Some(at) = s.quiet_since {
                let sep = if words.is_empty() { "" } else { " · " };
                words.push(seg(format!("{sep}quiet {}", age(at)), Style::default().fg(t.muted)));
            }
            words
        }
        Status::Blocked => vec![seg("needs you", Style::default().fg(t.blocked))],
        Status::Done => vec![seg("done · not seen", Style::default().fg(t.done).add_modifier(Modifier::BOLD))],
        Status::Idle if is_new(s) => vec![seg(format!("{} · ", s.agent), Style::default().fg(t.muted)), seg("new · type to begin", Style::default().fg(t.accent))],
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
    // A − (open) or + (closed) in a colour of its own, in the state bars' column: a caret is
    // too small to read as a button.
    let button = app.screen.fold_colors.get(&p.key).copied().unwrap_or(t.accent);
    put(buf, cols.x, y, &[seg(if open { "−" } else { "+" }, st.fg(button).add_modifier(Modifier::BOLD))], text_end);
    let name = truncate(&p.name, cols.rx.saturating_sub(cols.x + 2) as usize);
    let mut left = vec![seg("  ", st), seg(name, st.fg(t.strong).add_modifier(Modifier::BOLD))];
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

/// A `btn` chip: `word` in `text`, after an `accent` bold `icon` when it has one. Its ends
/// follow `ui.rounded`.
pub(super) fn word_chip(app: &App, icon: Option<&str>, word: &str, t: &Theme, outer: Color) -> Vec<Seg> {
    let look = Look::of(&app.cfg.ui);
    let mut body = Vec::new();
    if let Some(icon) = icon {
        body.push(seg(format!("{icon} "), Style::default().fg(t.accent).bg(t.btn).add_modifier(Modifier::BOLD)));
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
    hline(buf, cols.x, y - 1, (cols.rx + 1).saturating_sub(cols.x), t, pb);
    let new = word_chip(app, Some("+"), "New", t, pb);
    let menu = word_chip(app, None, "Menu", t, pb);
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
    use crate::client::reference_fixture::{LAYOUTS, Layout, SIZES, STAGES, Stage, frame, world};

    /// The reference's sidebar rows, as the fixture should draw them (see the file's header).
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
    /// needs you goes first, where the reference's sample keeps its own order.
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

    /// `claude · working` read as `working`: the line has nothing to its right.
    fn without_agent_on_working(row: &str) -> String {
        row.replacen("claude · working", "working", 1)
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
        // The app leaves the agent's name off a working line; it is in the pane's title.
        let rows: Vec<Vec<char>> = want.rows.iter().map(|r| without_agent_on_working(r).chars().collect()).collect();
        // Sessions are one line now, each with its agent's mark (ADR 0021).
        let rows = crate::client::reference_fixture::as_one_line_rows(rows, glyphs == "ascii", ANY);
        let rows: Vec<String> = rows.iter().map(|r| r.iter().collect::<String>().trim_end().to_string()).collect();
        let want = Want { rows: in_app_order(&rows, &got), ..want };
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
    fn a_folder_s_fold_button_is_a_coloured_mark_in_the_state_bars_column() {
        let mut app = world(Stage::Failed);
        let buf = crate::client::reference_fixture::cells(&mut app, Layout::Flat, 160, 45, "unicode");
        let side = app.screen.side_rect;
        let y = (side.y..side.bottom()).find(|&y| (side.x..side.right()).any(|x| buf[(x, y)].symbol() == "−")).expect("an open folder");
        let x = (side.x..side.right()).find(|&x| buf[(x, y)].symbol() == "−").unwrap();
        let bar_x = (side.y..side.bottom()).find_map(|y| (side.x..side.right()).find(|&x| buf[(x, y)].symbol() == "▎")).expect("a row's state bar");
        assert_eq!(x, bar_x, "in the state bars' column");
        let (cell, ground) = (&buf[(x, y)], buf[(x + 1, y)].bg);
        assert_eq!(cell.bg, ground, "no box of its own");
        let key = app.screen_model().into_iter().find(|p| (x..side.right()).map(|x| buf[(x, y)].symbol()).collect::<String>().contains(&p.name)).expect("the folder").key;
        assert_eq!(cell.fg, app.screen.fold_colors.get(&key).copied().unwrap_or(app.theme.accent), "the mark is the folder's colour");
    }

    #[test]
    fn rows_give_way_in_order_meta_age_then_third_line() {
        let at = |w| rows_at(Stage::Failed, Layout::Floating, w, 30);
        let wide = at(160);
        assert!(has(&wide, "⎇ main · ch…") && has(&wide, "2m") && has(&wide, "✗ src/pages"), "{wide:#?}");
        let narrow = at(140);
        assert!(has(&narrow, "✗ src/pages"), "140 columns are still wide: {narrow:#?}");
        let slim = at(139);
        assert!(!has(&slim, "⎇ main") && has(&slim, "2m"), "the meta goes first, the age stays: {slim:#?}");
        assert!(!has(&slim, "✗ src/pages"), "then line 3: {slim:#?}");
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
        assert_ne!(second, "working", "the spinner says working: there is no line for it");
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
        let buf = crate::client::reference_fixture::cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let (gx, gy) = (0..45u16).flat_map(|y| (0..160u16).map(move |x| (x, y))).find(|&(x, y)| buf[(x, y)].symbol() == "⣶").expect("the mark");
        assert!(buf[(gx, gy)].modifier.contains(Modifier::BOLD), "the mark is bold");
        let name: Vec<_> = (gx + 3..gx + 3 + "add-payments".len() as u16).map(|x| buf[(x, gy)].fg).collect();
        assert!(name.windows(2).all(|w| w[0] == w[1]), "no shimmer band without motion: {name:?}");
    }

    #[test]
    fn the_sidebar_rows_match_the_reference_in_every_layout_and_size() {
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
    use crate::client::reference_fixture::{LAYOUTS, Layout, Stage, cells, raw_text, world};
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
    fn a_quiet_agent_says_how_long_on_its_row() {
        let mut app = world(Stage::None);
        let before = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(!before.iter().any(|r| r.contains("quiet")), "{before:#?}");
        app.snap.terms.get_mut(&2).unwrap().quiet_since = Some(crate::client::screen::now_secs() - 12 * 60);
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(has(&rows, "quiet 12m") && !has(&rows, "working"), "the quiet spell, not the state: {rows:#?}");
    }

    #[test]
    fn line_two_says_only_what_the_row_does_not() {
        let mut app = world(Stage::None);
        app.snap.terms.get_mut(&2).unwrap().subagents = vec!["a".into(), "b".into()];
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(rows.iter().any(|r| said(r) == "2 agents"), "a count when the agent reports helpers: {rows:#?}");
        app.cfg.ui.subagent_count = false;
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(!has(&rows, "agents"), "Settings > Agents can turn the count off: {rows:#?}");
        app.cfg.ui.subagent_count = true;
        for state_only in ["working", "done · not seen", "needs you"] {
            assert!(!rows.iter().any(|r| said(r) == state_only), "the mark and the name say {state_only:?}: {rows:#?}");
        }
        assert!(has(&rows, "asks: use pnpm or npm?"), "{rows:#?}");
        // Seen and finished is one line; one that has been told nothing is new.
        app.snap.terms.get_mut(&4).unwrap().status = Status::Idle;
        let new = app.snap.terms.get_mut(&5).unwrap();
        (new.asleep, new.status, new.label) = (false, Status::Idle, String::new());
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        assert!(!rows.iter().any(|r| said(r) == "idle"), "{rows:#?}");
        assert!(has(&rows, "codex · new · type to begin"), "{rows:#?}");
    }

    #[test]
    fn a_row_ends_in_its_agent_s_mark_its_uncommitted_lines_and_its_age() {
        let mut app = world(Stage::None);
        let path = app.screen_model().iter().flat_map(|p| p.wts.iter()).find(|w| w.sessions.iter().any(|s| s.term == 2)).expect("add-payments' checkout").path.clone();
        app.screen.lines_changed.insert(path, (12, 3));
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        let row = rows.iter().find(|r| r.contains("add-payments")).expect("the row");
        assert!(row.contains("✻  +12 −3  0s"), "{row:?}");
        assert!(rows.iter().any(|r| r.contains("warm-finch") && r.contains("◆  3m")), "no lines counted: the mark and the age: {rows:#?}");
        let ascii = raw_text(&mut app, Layout::Floating, 160, 45, "ascii");
        assert!(ascii.lines().any(|l| l.contains("warm-finch") && l.contains("X  3m")), "the ASCII set marks Codex with a letter");
    }

    /// add-payments (term 2) with 12 lines added and 3 removed, and its checkout.
    fn payments_with_changes() -> (App, std::path::PathBuf) {
        let mut app = world(Stage::None);
        let path = app.screen_model().iter().flat_map(|p| p.wts.iter()).find(|w| w.sessions.iter().any(|s| s.term == 2)).expect("add-payments' checkout").path.clone();
        app.screen.lines_changed.insert(path.clone(), (12, 3));
        (app, path)
    }

    /// add-payments' row at a screen `w` wide, its context `context` full.
    fn payments_row(app: &mut App, context: Option<f32>, w: u16) -> String {
        app.snap.terms.get_mut(&2).unwrap().usage.context = context;
        let h = if w < 100 { 24 } else { 45 };
        let text = raw_text(app, Layout::Floating, w, h, "unicode");
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("--- {w}x{h}, context {context:?}\n{text}");
        }
        side_rows(app, Layout::Floating, w, h).into_iter().find(|r| r.contains("add-pay")).expect("add-payments' row")
    }

    #[test]
    fn a_row_shows_its_context_once_past_half_full() {
        let mut app = world(Stage::None);
        for (context, said) in [(None, None), (Some(49.0), None), (Some(50.0), None), (Some(50.6), Some("51%")), (Some(84.0), Some("84%")), (Some(85.0), Some("85%")), (Some(100.0), Some("100%"))] {
            let row = payments_row(&mut app, context, 160);
            match said {
                Some(pct) => assert!(row.contains(&format!("✻ {pct}  0s")), "{context:?}: beside the agent's mark: {row:?}"),
                None => assert!(!row.contains('%') && row.contains("✻  0s"), "{context:?}: nothing: {row:?}"),
            }
        }
    }

    #[test]
    fn context_is_muted_until_nearly_full_then_err_and_never_amber() {
        let mut app = world(Stage::None);
        let t = app.theme.clone();
        for (context, colour) in [(84.0, t.muted), (85.0, t.err), (100.0, t.err)] {
            app.snap.terms.get_mut(&2).unwrap().usage.context = Some(context);
            let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
            let pct = format!("{}%", context as u8);
            let at = find(&buf, &app, &pct);
            assert_eq!(buf[at].fg, colour, "{pct}");
            assert_ne!(buf[at].fg, t.blocked, "{pct}: amber is for needs you");
        }
    }

    #[test]
    fn a_narrow_row_keeps_its_context_over_its_lines() {
        let (mut app, _) = payments_with_changes();
        for (w, said) in [(160, "✻ 72%  +12 −3  0s"), (120, "✻ 72%  0s"), (100, "✻ 72%  0s"), (80, "✻ 72%")] {
            let row = payments_row(&mut app, Some(72.0), w);
            assert!(row.contains(said), "{w}: {row:?}");
            assert!(w == 160 || !row.contains("+12"), "{w}: the lines go first: {row:?}");
        }
    }

    #[test]
    fn the_meta_gives_way_lines_then_mark_then_context() {
        let (mut app, _) = payments_with_changes();
        app.snap.terms.get_mut(&2).unwrap().usage.context = Some(72.0);
        app.screen_fresh();
        let s = app.screen_model().iter().flat_map(Proj::sessions).find(|s| s.term == 2).cloned().expect("add-payments");
        let t = app.theme.clone();
        let said = |room| {
            let (segs, _) = row_meta(&s, false, Some((12, 3)), "2m", room, Style::default(), &t);
            segs.iter().map(|g| g.0.as_str()).collect::<String>()
        };
        assert_eq!(said(40), "✻ 72%  +12 −3  2m");
        assert_eq!(said(16), "✻ 72%  2m", "the lines first");
        assert_eq!(said(8), "72%  2m", "then the agent's mark");
        assert_eq!(said(6), "2m", "then the context; the age stays");
    }

    fn mouse(app: &mut App, kind: crossterm::event::MouseEventKind, (column, row): (u16, u16)) {
        app.on_mouse(crossterm::event::MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE });
    }

    fn click(app: &mut App, at: (u16, u16)) {
        use crossterm::event::{MouseButton, MouseEventKind};
        mouse(app, MouseEventKind::Down(MouseButton::Left), at);
        mouse(app, MouseEventKind::Up(MouseButton::Left), at);
    }

    /// The pointer moved onto add-payments' `+12` where an unhovered frame drew it, and the
    /// frame drawn with it there.
    fn hover_counts(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        let buf = cells(app, Layout::Floating, w, h, "unicode");
        let counts = find(&buf, app, "+12");
        mouse(app, crossterm::event::MouseEventKind::Moved, counts);
        cells(app, Layout::Floating, w, h, "unicode")
    }

    fn opened_changes(app: &App) -> bool {
        matches!(&app.mode, Mode::Confirm(c) if matches!(c.act, crate::client::menu::Act::GitInit(_)))
    }

    #[test]
    fn the_changed_lines_stay_put_when_the_pointer_lands_on_them() {
        // An agent's row has an age (the ⋯ takes its place); a shell's has none (two columns kept).
        for agent in [true, false] {
            let (mut app, _) = payments_with_changes();
            if !agent {
                app.snap.terms.get_mut(&2).unwrap().agent = None;
                app.screen_sync();
            }
            let before = find(&cells(&mut app, Layout::Floating, 160, 45, "unicode"), &app, "+12");
            let after = find(&hover_counts(&mut app, 160, 45), &app, "+12");
            assert_eq!(after, before, "agent {agent}: hovering the row doesn't move the counts");
            click(&mut app, before);
            assert!(opened_changes(&app), "agent {agent}: a click where they were drawn opens Changes");
        }
    }

    #[test]
    fn right_clicking_a_row_s_changed_lines_opens_its_menu() {
        let (mut app, _) = payments_with_changes();
        let counts = find(&hover_counts(&mut app, 160, 45), &app, "+12");
        mouse(&mut app, crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right), counts);
        assert!(matches!(app.mode, Mode::ScreenMenu(_)), "the session's menu, as on the rest of the row");
    }

    #[test]
    fn dragging_from_a_row_s_changed_lines_moves_the_row() {
        use crossterm::event::{MouseButton, MouseEventKind};
        let (mut app, _) = payments_with_changes();
        let counts = find(&hover_counts(&mut app, 160, 45), &app, "+12");
        let onto = find(&cells(&mut app, Layout::Floating, 160, 45, "unicode"), &app, "fix-footer");
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), counts);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), onto);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), onto);
        assert!(!opened_changes(&app), "a drag is not a click");
        let rows = side_rows(&mut app, Layout::Floating, 160, 45);
        let at = |name: &str| rows.iter().position(|r| r.contains(name)).unwrap_or_else(|| panic!("{name}: {rows:#?}"));
        assert!(at("add-payments") < at("fix-footer"), "add-payments took fix-footer's place: {rows:#?}");
    }

    #[test]
    fn clicking_a_row_s_changed_lines_opens_its_changes_like_d() {
        let (mut app, path) = payments_with_changes();
        let buf = hover_counts(&mut app, 160, 45);
        let counts = find(&buf, &app, "+12");
        click(&mut app, counts);
        let Mode::Confirm(c) = &app.mode else { panic!("the checkout's Changes (here: not a repo yet) opens") };
        assert_eq!(c.act, crate::client::menu::Act::GitInit(path), "for this row's checkout");
        let clicked = app.mode.clone();
        let mut by_key = payments_with_changes().0;
        by_key.screen.cursor = Some(2);
        by_key.act(Action::Changes);
        assert_eq!(by_key.mode, clicked, "the same as d on that session");
    }

    #[test]
    fn clicking_the_rest_of_a_row_with_changed_lines_still_opens_the_session() {
        let (mut app, _) = payments_with_changes();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let at = find(&buf, &app, "add-payments");
        mouse(&mut app, crossterm::event::MouseEventKind::Moved, at);
        let _ = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        click(&mut app, at);
        assert!(!matches!(app.mode, Mode::Confirm(_)) && app.view.is_none(), "no Changes");
        assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, crate::protocol::ClientMsg::Command(crate::protocol::Command::FocusPane { term: 2 }))), "the session opens");
    }

    #[test]
    fn the_hover_tint_shows_on_the_changed_lines_alone() {
        let (mut app, _) = payments_with_changes();
        let buf = hover_counts(&mut app, 160, 45);
        let (x, y) = find(&buf, &app, "+12");
        let (nx, _) = find(&buf, &app, "add-payments");
        let counts_bg = buf[(x, y)].bg;
        assert_eq!(buf[(x + "+12 −3".chars().count() as u16 - 1, y)].bg, counts_bg, "the whole of +12 −3 is lit");
        assert_ne!(buf[(x - 1, y)].bg, counts_bg, "not the gap before it");
        assert_ne!(buf[(nx, y)].bg, counts_bg, "not the row's name");
    }

    #[test]
    fn a_row_with_no_changed_lines_has_no_changes_target() {
        let mut app = world(Stage::None);
        let _ = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        assert!(!app.hits.iter().any(|(_, h)| matches!(h, crate::client::Hit::Screen(ScreenHit::RowChanges(_)))));
    }

    #[test]
    fn with_full_rounding_the_open_row_has_round_ends() {
        use crate::client::screen::{CAP_L, CAP_R};
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let (x, y) = find(&buf, &app, "fix-footer");
        let r = app.screen.side_rect;
        let row: Vec<String> = (r.x..r.right()).map(|xx| buf[(xx, y)].symbol().to_string()).collect();
        let first = row.iter().position(|c| c == CAP_L).expect("a round left end");
        let last = row.iter().rposition(|c| c == CAP_R).expect("a round right end");
        assert!(r.x + (first as u16) < x && r.x + (last as u16) > x, "round ends either side of the name: {row:?}");
    }

    #[test]
    fn a_finished_seen_row_is_one_line_at_every_width() {
        let mut app = world(Stage::None);
        app.snap.terms.get_mut(&4).unwrap().status = Status::Idle;
        for layout in LAYOUTS {
            let rows = side_rows(&mut app, layout, 100, 30);
            assert!(!rows.iter().any(|r| said(r) == "working"), "{layout:?}: a working row is one line: {rows:#?}");
            assert!(has(&rows, "asks: use pnpm"), "{layout:?}: {rows:#?}");
            assert!(!rows.iter().any(|r| said(r) == "idle"), "{layout:?}: a plain idle row is one line: {rows:#?}");
            let rows = side_rows(&mut app, layout, 160, 45);
            assert!(!rows.iter().any(|r| said(r) == "idle"), "{layout:?}: wide, it is one line too: {rows:#?}");
            assert!(!rows.iter().any(|r| said(r) == "working"), "{layout:?}: wide too: {rows:#?}");
        }
    }

    #[test]
    fn only_the_selected_row_has_a_bar_and_it_is_the_accent() {
        let mut app = world(Stage::Failed);
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let side = app.screen.side_rect;
        let bars: Vec<(u16, Color)> = (side.y..side.bottom()).flat_map(|y| (side.x..side.right()).map(move |x| (x, y))).filter(|&p| buf[p].symbol() == "▎").map(|(_, y)| (y, buf[(side.x + 2, y)].fg)).collect();
        let open = app.screen.row_y[&app.focused().expect("an open session")];
        assert!(!bars.is_empty() && bars.iter().all(|&(y, c)| (open..open + 3).contains(&y) && c == app.theme.accent), "{bars:?}");
    }

    #[test]
    fn names_shimmer_while_working_or_unseen_and_pulse_while_they_need_you() {
        let (base, bright, st) = (Color::Rgb(0, 0, 0), Color::Rgb(250, 250, 250), Style::default());
        let colours = |m, f| name_segs("quick-fox", m, f, base, bright, st).iter().map(|(_, s)| s.fg).collect::<Vec<_>>();
        assert!(colours(NameMotion::Still, 3).iter().all(|c| *c == Some(base)), "still: one colour");
        let band = colours(NameMotion::Shimmer, 6);
        assert!(band.contains(&Some(base)) && band.iter().any(|c| *c != Some(base)), "a band crosses the name: {band:?}");
        assert_ne!(colours(NameMotion::Shimmer, 6), colours(NameMotion::Shimmer, 9), "and moves");
        let at = |f| colours(NameMotion::Pulse, f)[0];
        assert_eq!(at(0), Some(base), "a pulse starts dim");
        assert_ne!(at(PULSE_FRAMES / 2), Some(base), "brightens");
        assert_eq!(at(PULSE_FRAMES), Some(base), "and dims again");
    }

    #[test]
    fn a_row_that_wants_a_look_is_bold_in_its_state_s_colour_and_its_words_quiet() {
        let mut app = world(Stage::None);
        app.snap.terms.get_mut(&5).unwrap().asleep = false;
        app.snap.terms.get_mut(&5).unwrap().status = Status::Idle;
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let t = app.theme.clone();
        let style = |name: &str| {
            let (x, y) = find(&buf, &app, name);
            (buf[(x, y)].modifier.contains(Modifier::BOLD), buf[(x, y)].fg)
        };
        let glyph = |name: &str| {
            let (x, y) = find(&buf, &app, name);
            buf[(x - 2, y)].fg
        };
        assert_eq!(style("warm-finch"), (true, t.blocked), "needs you: bold, in the amber");
        assert_eq!(style("quick-fox"), (true, glyph("quick-fox")), "done, not seen: bold, in its mark's colour");
        assert_eq!(style("old-moth"), (false, t.text), "done and seen: text, not bold");
        let (x, y) = find(&buf, &app, "asks:");
        assert_eq!(buf[(x, y)].fg, t.muted, "line two is muted: its glyph is the amber");
        assert_eq!(glyph("warm-finch"), t.blocked, "the glyph keeps the state's colour");
    }

    #[test]
    fn the_foot_is_two_chips_lifted_off_the_bottom_under_a_divider_with_no_band() {
        for layout in LAYOUTS {
            let mut app = world(Stage::None);
            let buf = cells(&mut app, layout, 160, 45, "unicode");
            let (r, t) = (app.screen.side_rect, app.theme.clone());
            let foot_y = r.bottom() - 2;
            let row = |y: u16| -> String { (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect() };
            assert!(row(foot_y).contains("+ New") && row(foot_y).contains("Menu"), "{layout:?}: {:?}", row(foot_y));
            assert!(row(foot_y - 1).contains("────"), "{layout:?}: a divider above: {:?}", row(foot_y - 1));
            let line_fg = (r.x..r.right()).find(|x| buf[(*x, foot_y - 1)].symbol() == "─").map(|x| buf[(x, foot_y - 1)].fg);
            assert_eq!(line_fg, Some(t.line), "{layout:?}: the divider is `line`");
            // No band: the cells between the chips are the sidebar's own ground.
            let (n, _) = find(&buf, &app, "+ New");
            let (m, _) = find(&buf, &app, "Menu");
            assert_eq!(buf[(n + 8, foot_y)].bg, buf[(n + 8, foot_y - 2)].bg, "{layout:?}: no band behind the foot");
            assert!(m > n + 8, "{layout:?}: menu is at the right");
            // New's `+` in the accent, bold; the words in text, no letter lit; chips raised.
            let plus = &buf[(n, foot_y)];
            assert_eq!((plus.fg, plus.bg, plus.modifier.contains(Modifier::BOLD)), (t.accent, t.btn, true), "{layout:?}: New's +");
            for (word, x) in [("New", n + 2), ("Menu", m)] {
                assert!((x..x + word.len() as u16).all(|x| buf[(x, foot_y)].fg == t.text && buf[(x, foot_y)].bg == t.btn), "{layout:?}: {word} is plain text on a chip");
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
