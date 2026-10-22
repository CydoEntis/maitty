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
    /// An agent or shell (pi, wi, si), and how deep it sits in a fleet (0: not under anyone).
    Sess(usize, usize, usize, u8),
    /// A session's second line: its check result, its question, or what it is doing.
    Detail(TermId),
    /// A session's third line, from `NARROW` columns: a failed check's first failing line, or
    /// its checkpoints by name.
    Third(TermId),
    /// A dim line under an agent: a queued follow-up, how many more subagents. Shares the
    /// agent's highlight.
    Note(String, Color, TermId),
    /// A legacy live helper label.
    Subagent(String, TermId),
    /// A runtime-reported child with a stable identity and retained lifecycle.
    SubagentRecord(Box<crate::protocol::SubagentActivity>, TermId, usize),
    /// The heading over a session's subagents: how many, and whether they're listed.
    Subs(TermId, usize, bool),
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
fn keeps_detail(s: &Session) -> bool {
    check::shown(s).is_some() || s.question.is_some() || s.resume_at.is_some()
}

fn has_third(s: &Session) -> bool {
    check::reported(s).len() > 1 || check::shown(s).is_some_and(|c| check::failed(c) && !c.first_failing_line.is_empty())
}

pub(in crate::client) fn session_lines(s: &Session, subs_closed: bool, t: &Theme, out: &mut Vec<Line>) {
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
    // Its subagents, each on a line, under a heading that folds them away.
    if !s.subagents.is_empty()
        || s.subagent_activity
            .iter()
            .any(|activity| activity.state.active())
    {
        let has_record_for_live_name = |name: &str| {
            s.subagent_activity.iter().any(|activity| {
                activity.state.active() && activity.name.eq_ignore_ascii_case(name)
            })
        };
        let legacy: Vec<_> = s.subagents.iter().filter(|name| !has_record_for_live_name(name)).collect();
        let total = s.subagent_activity.iter().filter(|activity| activity.state.active()).count() + legacy.len();
        out.push(Line::Subs(s.term, total, !subs_closed));
        if !subs_closed {
            let mut shown = 0;
            for (index, activity) in s.subagent_activity.iter().enumerate().filter(|(_, activity)| activity.state.active()) {
                if shown == SUBS_LISTED { break; }
                out.push(Line::SubagentRecord(Box::new(activity.clone()), s.term, index));
                shown += 1;
            }
            for sub in &legacy {
                if shown == SUBS_LISTED { break; }
                let stage = subagent_stage(sub, None);
                out.push(Line::Subagent(stage.to_string(), s.term));
                shown += 1;
            }
            if total > shown {
                out.push(Line::Note(format!("  +{} more", total - shown), notes_c, s.term));
            }
        }
    }
}

/// Infer a short stage only when the task or role provides recognizable evidence.
pub(super) fn subagent_stage(name: &str, task: Option<&str>) -> &'static str {
    let task = task.unwrap_or_default().to_ascii_lowercase();
    let role: Vec<_> = name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let has_role = |words: &[&str]| words.iter().any(|word| role.iter().any(|part| part == word));
    if ["review", "audit", "verif"].iter().any(|word| task.contains(word)) || has_role(&["review", "reviewer", "audit", "auditor", "verify", "verifier"]) {
        "Reviewing"
    } else if ["plan", "architect", "design"].iter().any(|word| task.contains(word)) || has_role(&["plan", "planner", "architect", "design", "designer"]) {
        "Planning"
    } else if ["research", "explore", "search", "investigat"].iter().any(|word| task.contains(word)) || has_role(&["research", "researcher", "explore", "explorer", "search", "investigator"]) {
        "Researching"
    } else if ["build", "implement", "code"].iter().any(|word| task.contains(word)) || has_role(&["build", "builder", "implementer", "coder"]) {
        "Building"
    } else {
        "Working"
    }
}

fn subagent_label(name: &str, task: Option<&str>, latest_activity: Option<&str>) -> String {
    if let Some(task) = task.map(str::trim).filter(|task| !task.is_empty()) {
        return task.to_owned();
    }
    if let Some(activity) = latest_activity.map(str::trim).filter(|activity| {
        !activity.is_empty() && !activity.starts_with("Runtime reported")
    }) {
        if let Some(tool) = activity.strip_prefix("PreToolUse: ") {
            return format!("Using {tool}");
        }
        if let Some(tool) = activity.strip_prefix("PostToolUse: ") {
            return format!("Used {tool}");
        }
        if !activity.starts_with("SubagentStart") && !activity.starts_with("SubagentStop") {
            return activity.to_owned();
        }
    }
    let role = name.trim();
    let generic = ["", "agent", "subagent", "worker", "delegate", "general-purpose", "general purpose", "default", "unknown"];
    if !generic.iter().any(|name| role.eq_ignore_ascii_case(name)) {
        let stage = subagent_stage(role, None);
        if stage != "Working" {
            return stage.to_owned();
        }
        return role.to_owned();
    }
    "Working".to_owned()
}

/// This many of a session's native helper agents are named before they are counted.
const SUBS_LISTED: usize = 8;

/// A helper's row steps in this far under the session that started it, per level.
const FLEET_INDENT: u16 = 2;
/// Deeper helpers stay at this level: the names need the width more than the tree does.
const FLEET_DEPTH: u8 = 2;

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
        // A fleet: the sessions a session started sit under it, in the order above. One that
        // needs you has sorted to the top and stays there, on its own.
        let term_of = |r: &(usize, usize)| p.wts[r.0].sessions[r.1].term;
        let commander = |r: &(usize, usize)| {
            let s = &p.wts[r.0].sessions[r.1];
            let floats = sort && !settled(s.status);
            s.started_by.filter(|by| !floats && *by != s.term && rows.iter().any(|x| term_of(x) == *by))
        };
        let mut placed: Vec<((usize, usize), u8)> = Vec::with_capacity(rows.len());
        let mut next: Vec<((usize, usize), u8)> = rows.iter().rev().filter(|r| commander(r).is_none()).map(|r| (*r, 0)).collect();
        while let Some((r, depth)) = next.pop() {
            placed.push((r, depth));
            let below = (depth + 1).min(FLEET_DEPTH);
            next.extend(rows.iter().rev().filter(|k| commander(k) == Some(term_of(&r)) && !placed.iter().any(|(x, _)| x == *k)).map(|k| (*k, below)));
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
            session_lines(s, app.screen.subs_closed.contains(&s.term), t, &mut out);
        }
        if !any {
            out.push(Line::Empty(pi));
        }
        out.push(Line::Gap);
    }
    out
}

/// The lines drawn at `screen_w` columns. Below `NARROW` a row is one line unless its second
/// line is a check result or a question, and the third line goes.
fn fitted_lines(app: &App, model: &[Proj], t: &Theme, screen_w: u16) -> Vec<Line> {
    let mut lines = side_lines(app, model, t);
    if screen_w < NARROW {
        lines.retain(|l| match l {
            Line::Third(_) => false,
            Line::Detail(term) => find(model, *term).is_some_and(|(.., s)| keeps_detail(s)),
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
    let pb = if look.flat { t.card } else { pane_bg(t) };
    let focused_side = app.mode == Mode::Side;
    let mut c = Card::new(t, "");
    if look.flat { c.bg = t.card; }
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
    let foot_y = r.bottom().saturating_sub(if look.flat { 1 } else { 2 });
    let list_h = foot_y.saturating_sub(top) as usize;
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

    let hover_bg = if look.flat { t.hov } else { t.hover_bg() };
    let on = app.motion_on();
    let mut gliding: Option<u16> = None;
    // The lines under a session (what it asks, its subagents) step in as far as it does.
    let mut indents: Vec<u16> = Vec::with_capacity(lines.len());
    for l in &lines {
        let last = indents.last().copied().unwrap_or(0);
        indents.push(match l {
            Line::Sess(.., depth) => u16::from(*depth) * FLEET_INDENT,
            Line::Note(..) | Line::Detail(..) | Line::Third(..) | Line::Subagent(..) | Line::SubagentRecord(..) | Line::Subs(..) => last,
            _ => 0,
        });
    }
    // A session's row is its lines together: one tint, one hover.
    let row_rect = |i: usize, y: u16| {
        let mut n = 1;
        while matches!(lines.get(i + n), Some(Line::Detail(_) | Line::Third(_))) {
            n += 1;
        }
        Rect { height: n as u16, ..cols.row(y) }
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
                let ground = if !picked && hovered(app, row_rect(i, y)) { hover_bg } else { pb };
                row_look(s, picked, Some(term) == focus, focused_side, ground, t)
            });
            if rl.bg != pb {
                fill(buf, row, rl.bg);
            }
            if let Some(c) = rl.bar {
                check::edge(app, buf, cols.x, y, c, rl.bg);
            }
            hit(app, row, ScreenHit::Session(term));
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
                }
                _ => {
                    put(buf, x, y, &rl.on(third(app, s, t), t), cols.end());
                }
            }
            continue;
        }
        draw_line(app, buf, model, line, cols, y, indent, pb, hover_bg, t, screen_w < NARROW);
    }
    let plain = Style::default().bg(pb);
    if scroll > 0 {
        put(buf, cols.rx + 1, top, &[seg("▲", plain.fg(t.muted))], cols.end());
    }
    if scroll + list_h < lines.len() {
        put(buf, cols.rx + 1, top + list_h as u16 - 1, &[seg("▼", plain.fg(t.muted))], cols.end());
    }
    draw_foot(app, buf, r, cols, pb, t, look.flat);
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
    let failed = state == State::Failed;
    let st = Style::default().bg(rl.bg);
    let (g, gc) = if plain_shell(s) { ("›", t.muted) } else { (mark.glyph, rl.paint(t, mark.color)) };
    let mut gs = st.fg(gc);
    if matches!(state, State::Failed | State::Needs | State::Approve) {
        gs = gs.add_modifier(Modifier::BOLD);
    }
    let gx = cols.x + 2 + indent;
    put(buf, gx, y, &[seg(g, gs)], gx + 1);
    hit(app, Rect { x: gx, y, width: 2, height: 1 }, ScreenHit::Why(s.term));
    let menu = hovered(app, cols.row(y)) && !rl.picked;
    let when = if ages && s.is_agent && !menu { age(s.since) } else { String::new() };
    let nx = gx + 2;
    let room = cols.rx.saturating_sub(nx).saturating_sub(when.width() as u16) as usize;
    let name = truncate(&s.name, room);
    let ns = st.fg(if rl.picked || failed { t.strong } else { t.text }).add_modifier(Modifier::BOLD);
    let left = if state == State::Working {
        let working = rl.paint(t, t.working);
        shimmer(&name, app.spinner_frame(), working, blend(working, t.strong, 0.6), ns)
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

/// A session's second line, in `room` columns. With several checkpoints it is the headline
/// word, then a glyph per checkpoint; the glyphs give way, never the word.
fn detail(app: &App, s: &Session, rl: RowLook, t: &Theme, room: usize) -> Vec<Seg> {
    let checks = check::reported(s);
    let segs = if checks.len() > 1 {
        let (word, st) = check::headline_word(checks, t);
        let mut v = vec![(truncate(&word, room), st)];
        v.extend(check::strip(app, checks, t));
        v
    } else if let Some(info) = check::shown(s) {
        check::result_words(info, t)
    } else if s.resume_at.is_some() {
        status_words(s, t)
    } else if let Some(q) = &s.question {
        vec![seg(format!("asks: {q}"), Style::default().fg(t.blocked))]
    } else {
        status_words(s, t)
    };
    rl.on(segs, t)
}

/// What a session without a check result or a question is doing, in words.
fn status_words(s: &Session, t: &Theme) -> Vec<Seg> {
    let muted = |text: &str| vec![seg(text.to_string(), Style::default().fg(t.muted))];
    if s.asleep {
        return muted("asleep zzZ");
    }
    if let Some(at) = s.resume_at {
        return muted(&format!("⏸ resumes in {}", until(at)));
    }
    match s.status {
        Status::Working => vec![seg(format!("{} · ", s.agent), Style::default().fg(t.muted)), seg("working", Style::default().fg(t.accent))],
        Status::Blocked => vec![seg("needs you", Style::default().fg(t.blocked))],
        Status::Done => muted("finished · not seen"),
        Status::Idle if s.is_agent && s.title == WAITING => muted(WAITING),
        Status::Idle if s.is_agent => muted("finished"),
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
fn draw_line(app: &mut App, buf: &mut Buffer, model: &[Proj], line: &Line, cols: Cols, y: u16, indent: u16, pb: Color, hover_bg: Color, t: &Theme, small: bool) {
    let row = cols.row(y);
    let x = cols.x + 4 + indent;
    let room = cols.end().saturating_sub(x) as usize;
    match line {
        Line::Proj(pi) => draw_proj(app, buf, model, *pi, cols, y, pb, hover_bg, t, small),
        Line::Note(text, c, term) => {
            if small {
                return;
            }
            put(buf, x, y, &[seg(truncate(text, room), Style::default().bg(pb).fg(*c).add_modifier(Modifier::ITALIC))], cols.end());
            hit(app, row, ScreenHit::Session(*term));
        }
        Line::Subagent(name, term) => {
            let base = Style::default().bg(pb).fg(t.muted).add_modifier(Modifier::DIM);
            put(buf, x, y, &[seg("  ↳ ", base), seg(truncate(name, room.saturating_sub(4)), base)], cols.end());
            hit(app, row, ScreenHit::Session(*term));
        }
        Line::SubagentRecord(activity, term, index) => {
            put(buf, x, y, &subagent_segs(activity, pb, t, room), cols.end());
            hit(app, row, ScreenHit::SubagentInfo(*term, *index));
        }
        // Nothing running: one click starts a shell there.
        Line::Empty(pi) => {
            let bg = if hovered(app, row) { t.hov } else { pb };
            if bg != pb {
                fill(buf, row, bg);
            }
            let st = Style::default().bg(bg);
            let nk = k(app, &Action::ShellHere);
            put(buf, cols.x + 2, y, &[seg("empty  ", st.fg(t.muted).add_modifier(Modifier::ITALIC)), seg(nk, st.fg(t.accent).add_modifier(Modifier::BOLD)), seg(" new pane", st.fg(t.muted))], cols.end());
            hit(app, row, ScreenHit::ShellIn(*pi));
        }
        // A session's subagents: the heading folds and unfolds them.
        Line::Subs(term, n, open) => {
            if small {
                return;
            }
            let st = Style::default().bg(pb).fg(if hovered(app, row) { t.text } else { t.muted });
            let label = format!("{} {n} subagent{}", if *open { "▾" } else { "▸" }, if *n == 1 { "" } else { "s" });
            put(buf, x, y, &[seg(label, st)], cols.end());
            hit(app, row, ScreenHit::ToggleSubs(*term));
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

/// A runtime-reported helper: its state's marker and what it is doing, in its state's colour.
fn subagent_segs(activity: &crate::protocol::SubagentActivity, pb: Color, t: &Theme, room: usize) -> Vec<Seg> {
    use crate::protocol::SubagentState as S;
    let state_color = match activity.state {
        S::Working | S::Cancelled => t.muted,
        S::WaitingForInput | S::AwaitingApproval => t.blocked,
        S::Completed => t.done,
        S::Failed | S::ConnectionLost => t.err,
        S::Starting => t.accent,
    };
    let state_marker = match activity.state {
        S::Working => None,
        S::WaitingForInput | S::ConnectionLost => Some("?"),
        S::AwaitingApproval => Some("!"),
        S::Completed => Some("✓"),
        S::Failed => Some("×"),
        S::Cancelled => Some("–"),
        S::Starting => Some("◷"),
    };
    let detail = subagent_label(&activity.name, activity.task.as_deref(), activity.latest_activity.as_deref());
    let st = Style::default().bg(pb).add_modifier(Modifier::DIM);
    let mut label_style = st.fg(state_color);
    if activity.state != S::Working {
        label_style = label_style.add_modifier(Modifier::BOLD);
    }
    let mut segs = vec![seg("  ↳ ", st.fg(t.muted))];
    if let Some(marker) = state_marker {
        segs.push(seg(format!("{marker} "), st.fg(state_color)));
    }
    let label_room = room.saturating_sub(segs_width(&segs) as usize);
    segs.push(seg(truncate(&detail, label_room), label_style));
    segs
}

/// A folder's header: its fold arrow and name, and right-aligned what it is (branch, its
/// checkpoints), which gives way first.
#[allow(clippy::too_many_arguments)]
fn draw_proj(app: &mut App, buf: &mut Buffer, model: &[Proj], pi: usize, cols: Cols, y: u16, pb: Color, hover_bg: Color, t: &Theme, small: bool) {
    let p = &model[pi];
    let row = cols.row(y);
    let open = !app.screen.saved.closed.contains(&format!("p:{}", p.key));
    let on = app.mode == Mode::Side && app.screen.cursor_proj.as_ref() == Some(&p.key);
    let hov = hovered(app, row);
    let bg = if on { t.sel_bg() } else if hov { hover_bg } else { pb };
    if bg != pb {
        fill(buf, row, bg);
    }
    if on {
        check::edge(app, buf, cols.x, y, t.accent, bg);
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
    put(buf, lx, y, &left, cols.rx + 1);
    hit(app, row, ScreenHit::ToggleProj(pi));
    if hov && !small {
        row_glyph(app, buf, Rect { x: cols.rx.saturating_sub(2), y, width: 1, height: 1 }, "+", bg, t.accent, t, ScreenHit::ShellIn(pi));
        row_menu_button(app, buf, Rect { x: cols.rx, y, width: 1, height: 1 }, bg, t, ScreenHit::RowMenuProj(pi));
        return;
    }
    let room = cols.rx.saturating_sub(lx + segs_width(&left));
    if room >= META_MIN_COLS {
        let meta = truncate(&proj_meta(app, p, open), room as usize);
        put(buf, (cols.rx + 1).saturating_sub(meta.width() as u16), y, &[seg(meta, st.fg(t.muted))], cols.rx + 1);
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

/// The foot on the last row inside: + new on the left, the Actions key on the right.
fn draw_foot(app: &mut App, buf: &mut Buffer, r: Rect, cols: Cols, pb: Color, t: &Theme, flat: bool) {
    if r.height < 5 {
        return;
    }
    let y = r.bottom().saturating_sub(if flat { 1 } else { 2 });
    let st = Style::default().bg(pb);
    let key = st.fg(t.accent).add_modifier(Modifier::BOLD);
    let new = [seg("+", key), seg(" new", st.fg(t.text))];
    let nx = cols.x + 1;
    put(buf, nx, y, &new, cols.rx + 1);
    hit(app, Rect { x: nx, y, width: segs_width(&new), height: 1 }, ScreenHit::NewShell);
    let actions = [seg(k(app, &Action::Actions), key), seg(" actions", st.fg(t.text))];
    let w = segs_width(&actions);
    let ax = (cols.rx + 1).saturating_sub(w);
    put(buf, ax, y, &actions, cols.rx + 1);
    hit(app, Rect { x: ax, y, width: w, height: 1 }, ScreenHit::Actions);
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
mod subagent_tests {
    use super::{subagent_label, subagent_stage};

    #[test]
    fn helpers_show_reported_tasks_and_use_roles_only_as_a_fallback() {
        assert_eq!(subagent_stage("planner", None), "Planning");
        assert_eq!(subagent_stage("general-purpose", Some("Review the test changes")), "Reviewing");
        assert_eq!(subagent_stage("general-purpose", Some("Implement the fix")), "Building");
        assert_eq!(subagent_stage("Explore", None), "Researching");
        assert_eq!(subagent_label("general-purpose", Some(" Review the test changes "), None), "Review the test changes");
        assert_eq!(subagent_label("general-purpose", None, Some("PreToolUse: Bash")), "Using Bash");
        assert_eq!(subagent_label("planner", None, None), "Planning");
        assert_eq!(subagent_label("Tools Redesign", None, None), "Tools Redesign");
        assert_eq!(subagent_label("general-purpose", None, Some("Runtime reported child start")), "Working");
    }
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
