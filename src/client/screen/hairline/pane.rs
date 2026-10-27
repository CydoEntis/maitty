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

/// The title row: number, glyph, name, `project · ⎇ branch` and at the right the
/// load note and the ✕. The number, glyph and name never shrink while anything else is left
/// to give way: the meta first, then the note, then the ✕.
fn draw_title(app: &mut App, buf: &mut Buffer, body: Rect, info: &TermInfo, focused: bool, model: &[Proj], t: &Theme) {
    let ink = Ink::of(t);
    let y = body.y;
    let (x0, end) = (body.x + PANE_PAD, body.right().saturating_sub(PANE_PAD));
    let st = Style::default().bg(ink.bg);
    let (mut head, glyph_at) = head_segs(app, info, focused, model, t);
    let room = end.saturating_sub(x0);
    // The name shortens only once nothing else is left to give way.
    let before_name = segs_width(&head[..head.len().saturating_sub(1)]);
    if let Some(name) = head.last_mut() {
        name.0 = truncate(&name.0, room.saturating_sub(before_name) as usize);
    }
    let head_w = segs_width(&head);
    let right = fit_right(app, info, ink, room.saturating_sub(head_w + GAP));
    let right_w = segs_width(&right.segs);
    let left_end = end.saturating_sub(if right_w > 0 { right_w + GAP } else { 0 });
    let x = put(buf, x0, y, &head, left_end);
    if let Some(at) = glyph_at {
        hit(app, Rect { x: x0 + at, y, width: 1, height: 1 }, ScreenHit::Why(info.id));
    }
    let meta = project_branch(app, info, model);
    let meta_x = if head.is_empty() { x } else { x + GAP };
    if !meta.is_empty() && left_end >= meta_x + META_MIN {
        put(buf, meta_x, y, &[seg(truncate(&meta, (left_end - meta_x) as usize), st.fg(ink.dim))], left_end);
    }
    let rx = end.saturating_sub(right_w);
    put(buf, rx, y, &right.segs, end);
    if right.close {
        hit(app, Rect { x: end.saturating_sub(1), y, width: 1, height: 1 }, ScreenHit::CloseSplit(info.id));
    }
}

/// Number, glyph and name: the pane's number while the tab is split (teal focused, amber when
/// it needs you, else dim), its state's mark and its name in `strong`, bold. Also where the mark
/// sits in them, when there is one. A tab's only pane has none: the tab row already shows them.
fn head_segs(app: &App, info: &TermInfo, focused: bool, model: &[Proj], t: &Theme) -> (Vec<Seg>, Option<u16>) {
    if lone_pane(app, info.id) {
        return (Vec::new(), None);
    }
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
pub(super) fn mark(app: &App, info: &TermInfo, t: &Theme) -> Option<(String, Color)> {
    info.agent.as_ref()?;
    let set = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let working = working_glyph(app);
    let step = if app.motion_on() { check::ring_step(app.spinner_frame(), set.ring.len()) } else { 0 };
    let m = glyphs::Kind::of(info.status, info.check(), info.asleep).mark(set, t, step, &working);
    Some((m.glyph.to_string(), m.color))
}

/// The right of the title as it fits `room`, and whether the ✕ is in it.
struct Right {
    segs: Vec<Seg>,
    close: bool,
}

/// The load note and the ✕, the note dropped first when `room` is short. A tab's only pane
/// has no ✕: its tab's closes it. The state shows as the glyph, so it has no words here.
fn fit_right(app: &App, info: &TermInfo, ink: Ink, room: u16) -> Right {
    let st = Style::default().bg(ink.bg);
    let note = match info.resume_at {
        Some(at) => Some(format!("⏸ limit · continues in {}", until(at))),
        None => load_note(info.cpu, info.mem).map(|(load, _)| load),
    };
    let close = (!lone_pane(app, info.id)).then(|| glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons).close);
    let tries = [(true, true), (false, true), (false, false)];
    for (with_note, with_close) in tries {
        let mut segs = Vec::new();
        if let Some(n) = note.as_ref().filter(|_| with_note) {
            segs.push(seg(n.clone(), st.fg(ink.dim)));
        }
        if let Some(c) = close.filter(|_| with_close) {
            segs.push(seg(if segs.is_empty() { c.to_string() } else { format!("  {c}") }, st.fg(ink.dim)));
        }
        if segs_width(&segs) <= room {
            return Right { close: with_close && close.is_some(), segs };
        }
    }
    Right { segs: Vec::new(), close: false }
}

