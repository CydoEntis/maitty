//! A Hairline pane: a full-width `─` rule along its top (a split's edge), `│` hairlines only
//! between panes, never round the outside.

use super::*;

/// Where a pane sits among the tab's panes: whether another pane lies right of it.
#[derive(Debug, Clone, Copy)]
struct Edges {
    right: bool,
}

impl Edges {
    /// A pane that is not one of the tab's (a floating popup) has no neighbours.
    fn of(app: &App, term: TermId, r: Rect) -> Edges {
        let mine = app.screen.leaf_rects.iter().any(|(id, _)| *id == term);
        let right = app.screen.leaf_rects.iter().map(|(_, lr)| lr.right()).max().unwrap_or(0);
        Edges { right: mine && r.right() < right }
    }
}

/// Draw the pane at `r` with its terminal on `surface`: the hairline to the pane beside it and
/// a rule along its top, the edge that says which pane has the keys or needs you. A split has no
/// title; its tab names it. Returns the inside, under the rule.
#[allow(clippy::too_many_arguments)]
pub(in crate::client) fn pane_frame(app: &mut App, buf: &mut Buffer, r: Rect, info: &TermInfo, focused: bool, _model: &[Proj], surface: Color, t: &Theme) -> Rect {
    let ink = Ink::of(t);
    let r = r.intersection(buf.area);
    let edges = Edges::of(app, info.id, r);
    let body = Rect { width: r.width.saturating_sub(u16::from(edges.right)), ..r };
    if body.width < 4 || body.height < 3 {
        return Rect { width: 0, height: 0, ..r };
    }
    if edges.right {
        vline(buf, Rect { x: r.right() - 1, ..r }, ink.line, ink.bg);
    }
    // A tab's only pane has no rule: the tab row over it names it (ADR 0030).
    if lone_pane(app, info.id) {
        fill(buf, body, surface);
        return body;
    }
    app.hits.push((Rect { height: 1, ..body }, Hit::Pane(info.id)));
    fill(buf, Rect { height: 1, ..body }, ink.bg);
    let inside = Rect { y: body.y + 1, height: body.height - 1, ..body };
    fill(buf, inside, surface);
    rule(buf, body.x, body.y, body.width, rule_colour(app, info, focused, ink, t), ink.bg);
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

/// An agent's mark in its state's colour; a shell has none.
pub(super) fn mark(app: &App, info: &TermInfo, t: &Theme) -> Option<(String, Color)> {
    info.agent.as_ref()?;
    let set = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let working = working_glyph(app);
    let step = if app.motion_on() { check::ring_step(app.spinner_frame(), set.ring.len()) } else { 0 };
    let m = glyphs::Kind::of(info.status, info.check(), info.asleep).mark(set, t, step, &working);
    Some((m.glyph.to_string(), m.color))
}
