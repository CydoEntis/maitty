//! A Hairline pane: a title row over a full-width `─` rule, `│` / `─` hairlines only between
//! panes, never round the outside.

use super::*;

/// The fewest cells the dim `project · ⎇ branch` meta keeps before it goes: `x…`.
const META_MIN: u16 = 2;
/// Cells between the pieces of the title.
const GAP: u16 = 2;

/// Where a pane sits among the tab's panes: whether another pane lies right of it or below it.
#[derive(Debug, Clone, Copy)]
struct Edges {
    right: bool,
    below: bool,
}

impl Edges {
    /// A pane that is not one of the tab's (a floating popup) has no neighbours.
    fn of(app: &App, term: TermId, r: Rect) -> Edges {
        let mine = app.screen.leaf_rects.iter().any(|(id, _)| *id == term);
        let right = app.screen.leaf_rects.iter().map(|(_, lr)| lr.right()).max().unwrap_or(0);
        let bottom = app.screen.leaf_rects.iter().map(|(_, lr)| lr.bottom()).max().unwrap_or(0);
        Edges { right: mine && r.right() < right, below: mine && r.bottom() < bottom }
    }
}

/// Draw the pane at `r` with its terminal on `surface`: the hairlines to the panes beside and
/// below it, the title and the rule. Returns the inside, under the rule.
#[allow(clippy::too_many_arguments)]
pub(in crate::client) fn pane_frame(app: &mut App, buf: &mut Buffer, r: Rect, info: &TermInfo, focused: bool, model: &[Proj], surface: Color, t: &Theme) -> Rect {
    let ink = Ink::of(t);
    let r = r.intersection(buf.area);
    let edges = Edges::of(app, info.id, r);
    let body = Rect { width: r.width.saturating_sub(u16::from(edges.right)), height: r.height.saturating_sub(u16::from(edges.below)), ..r };
    if body.width < 4 || body.height < 3 {
        return Rect { width: 0, height: 0, ..r };
    }
    // The title row and rule focus the pane, under the title's own controls drawn after.
    app.hits.push((Rect { height: 2, ..body }, Hit::Pane(info.id)));
    fill(buf, Rect { height: 2, ..body }, ink.bg);
    let inside = Rect { y: body.y + 2, height: body.height - 2, ..body };
    fill(buf, inside, surface);
    if edges.below {
        rule(buf, r.x, r.bottom() - 1, r.width, ink.line, ink.bg);
    }
    if edges.right {
        vline(buf, Rect { x: r.right() - 1, ..r }, ink.line, ink.bg);
    }
    draw_title(app, buf, body, info, focused, model, t);
    rule(buf, body.x, body.y + 1, body.width, rule_colour(app, info, focused, ink, t), ink.bg);
    inside
}

/// The rule's colour: red while a confirm asks to close the pane, teal on the focused pane (sky
/// in shortcut mode, as every focused edge), amber on one that needs you, else `line`. Focus
/// wins over amber.
fn rule_colour(app: &App, info: &TermInfo, focused: bool, ink: Ink, t: &Theme) -> Color {
    if closing(app, info.id) {
        return t.err;
    }
    match (focused, pane_needs(info)) {
        (true, _) if armed(app) => t.sky(),
        (true, _) => ink.teal,
        (false, true) => ink.amber,
        _ => ink.line,
    }
}

/// The title row: number, glyph, name, the tabs, `project · ⎇ branch` and at the right the
/// load note, the state word and the ✕. The number, glyph and name never shrink while anything
/// else is left to give way: the meta first, then the tabs, then the note, the ✕ and the state
/// word.
fn draw_title(app: &mut App, buf: &mut Buffer, body: Rect, info: &TermInfo, focused: bool, model: &[Proj], t: &Theme) {
    let ink = Ink::of(t);
    let y = body.y;
    let (x0, end) = (body.x + PANE_PAD, body.right().saturating_sub(PANE_PAD));
    let st = Style::default().bg(ink.bg);
    let (mut head, glyph_at) = head_segs(app, info, focused, model, t);
    let room = end.saturating_sub(x0);
    // The name shortens only once nothing else is left to give way.
    let before_name = segs_width(&head[..head.len() - 1]);
    if let Some(name) = head.last_mut() {
        name.0 = truncate(&name.0, room.saturating_sub(before_name) as usize);
    }
    let head_w = segs_width(&head);
    let right = fit_right(app, info, ink, room.saturating_sub(head_w + GAP));
    let right_w = segs_width(&right.segs);
    let left_end = end.saturating_sub(if right_w > 0 { right_w + GAP } else { 0 });
    let mut x = put(buf, x0, y, &head, left_end);
    if let Some(at) = glyph_at {
        hit(app, Rect { x: x0 + at, y, width: 1, height: 1 }, ScreenHit::Why(info.id));
    }
    if is_top_left(app, info.id) && app.session_tabs().len() > 1 {
        x = draw_tabs(app, buf, x, y, left_end, model, ink);
    }
    let meta = meta_text(app, info, model);
    let meta_x = x + GAP;
    if !meta.is_empty() && left_end >= meta_x + META_MIN {
        put(buf, meta_x, y, &[seg(truncate(&meta, (left_end - meta_x) as usize), st.fg(ink.dim))], left_end);
    }
    let rx = end.saturating_sub(right_w);
    put(buf, rx, y, &right.segs, end);
    if let Some((at, w)) = right.word {
        hit(app, Rect { x: rx + at, y, width: w, height: 1 }, ScreenHit::Why(info.id));
    }
    if right.close {
        hit(app, Rect { x: end.saturating_sub(1), y, width: 1, height: 1 }, ScreenHit::CloseSplit(info.id));
    }
}

/// Number, glyph and name: the pane's number while the tab is split (teal focused, amber when
/// it needs you, else dim), its state's mark and its name in `strong`, bold. Also where the mark
/// sits in them, when there is one.
fn head_segs(app: &App, info: &TermInfo, focused: bool, model: &[Proj], t: &Theme) -> (Vec<Seg>, Option<u16>) {
    let ink = Ink::of(t);
    let st = Style::default().bg(ink.bg);
    let mut out = Vec::new();
    if let Some(n) = pane_number(app, info.id) {
        let colour = match (focused, pane_needs(info)) {
            (true, _) => ink.teal,
            (false, true) => ink.amber,
            _ => ink.dim,
        };
        out.push(seg(format!("{n} "), st.fg(colour).add_modifier(Modifier::BOLD)));
    }
    let glyph_at = mark(app, info, t).map(|(g, c)| {
        let at = segs_width(&out);
        out.push(seg(format!("{g} "), st.fg(c).add_modifier(Modifier::BOLD)));
        at
    });
    out.push(seg(session_name(app, model, info.id), st.fg(ink.strong).add_modifier(Modifier::BOLD)));
    (out, glyph_at)
}

/// An agent's mark in its state's colour; a shell has none.
fn mark(app: &App, info: &TermInfo, t: &Theme) -> Option<(String, Color)> {
    info.agent.as_ref()?;
    let set = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let working = working_glyph(app);
    let step = if app.motion_on() { check::ring_step(app.spinner_frame(), set.ring.len()) } else { 0 };
    let m = glyphs::Kind::of(info.status, info.check(), info.asleep).mark(set, t, step, &working);
    Some((m.glyph.to_string(), m.color))
}

/// The right of the title as it fits `room`: its pieces, where the state word sits (offset,
/// width) for its click, and whether the ✕ is drawn.
struct Right {
    segs: Vec<Seg>,
    word: Option<(u16, u16)>,
    close: bool,
}

/// The load note, the state word and the ✕, each dropped in turn when `room` is short: the note
/// first, then the ✕, then the word.
fn fit_right(app: &App, info: &TermInfo, ink: Ink, room: u16) -> Right {
    let st = Style::default().bg(ink.bg);
    let note = match info.resume_at {
        Some(at) => Some(format!("⏸ limit · continues in {}", until(at))),
        None => load_note(info.cpu, info.mem).map(|(load, _)| load),
    };
    let word = state_word(info, ink);
    let close = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons).close;
    let tries = [(true, true, true), (false, true, true), (false, true, false), (false, false, false)];
    for (with_note, with_word, with_close) in tries {
        let mut segs = Vec::new();
        let mut at = None;
        if let Some(n) = note.as_ref().filter(|_| with_note) {
            segs.push(seg(format!("{n}  "), st.fg(ink.dim)));
        }
        if let Some(w) = word.clone().filter(|_| with_word) {
            at = Some((segs_width(&segs), w.0.width() as u16));
            segs.push(w);
        }
        if with_close {
            segs.push(seg(if segs.is_empty() { close.to_string() } else { format!("  {close}") }, st.fg(ink.dim)));
        }
        if segs_width(&segs) <= room {
            return Right { segs, word: at, close: with_close };
        }
    }
    Right { segs: Vec::new(), word: None, close: false }
}

/// What an agent is doing, in the handoff's words and colours; a shell says nothing.
fn state_word(info: &TermInfo, ink: Ink) -> Option<Seg> {
    info.agent.as_ref()?;
    let st = Style::default().bg(ink.bg);
    let needs = st.fg(ink.amber).add_modifier(Modifier::BOLD);
    if check::approving(info.status, info.check()) {
        return Some(seg("needs you · approve check", needs));
    }
    Some(match info.status {
        Status::Blocked => seg(format!("needs you · {}", age(info.since)), needs),
        Status::Working => seg("working", st.fg(ink.teal)),
        Status::Done => seg("done", st.fg(ink.green)),
        _ => seg("idle", st.fg(ink.dim)),
    })
}

/// The tab's top-left pane carries the session's tabs.
fn is_top_left(app: &App, term: TermId) -> bool {
    app.screen.leaf_rects.iter().min_by_key(|(_, lr)| (lr.y, lr.x)).is_some_and(|(id, _)| *id == term)
}

/// The session's tabs as plain words from `x`, the current one teal and bold, then a dim `+`
/// that opens one. A tab that does not fit before `end` is left out, with the `+`; returns
/// where the tabs end.
fn draw_tabs(app: &mut App, buf: &mut Buffer, x: u16, y: u16, end: u16, model: &[Proj], ink: Ink) -> u16 {
    let st = Style::default().bg(ink.bg);
    let tabs = app.session_tabs();
    let mut at = x;
    for (n, &i) in tabs.iter().enumerate() {
        let name = tab_name(&app.screen.tabs[i]).unwrap_or_else(|| session_name(app, model, app.screen.tabs[i].focus));
        let label = format!("{} {name}", n + 1);
        let w = label.width() as u16;
        let tx = at + GAP;
        if tx + w > end {
            return at;
        }
        let style = if i == app.screen.tab { st.fg(ink.teal).add_modifier(Modifier::BOLD) } else { st.fg(ink.dim) };
        put(buf, tx, y, &[seg(label, style)], end);
        hit(app, Rect { x: tx, y, width: w, height: 1 }, ScreenHit::TabPick(i));
        at = tx + w;
    }
    if at + GAP < end {
        put(buf, at + GAP, y, &[seg("+", st.fg(ink.dim))], end);
        hit(app, Rect { x: at + GAP, y, width: 1, height: 1 }, ScreenHit::NewTab);
        at += GAP + 1;
    }
    at
}

/// `project · ⎇ branch`, from the session's sidebar row, else the pane's own branch.
fn meta_text(app: &App, info: &TermInfo, model: &[Proj]) -> String {
    let branch_mark = check::marks(app).branch;
    let (project, branch) = match find(model, info.id) {
        Some((p, w, _)) => (p.name.clone(), w.branch.clone()),
        None => (String::new(), info.branch.clone().unwrap_or_default()),
    };
    let branch = if branch.is_empty() { String::new() } else { format!("{branch_mark} {branch}") };
    [project, branch].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
}
