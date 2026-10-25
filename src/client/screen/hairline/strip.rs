//! The status strip: one `━` segment per session, in sidebar order, in its state's colour,
//! always in view over the mode bar.

use super::*;

/// Beyond this many sessions equal segments get too thin to tell apart: each is one `━`.
const EQUAL_UP_TO: usize = 12;

/// Draw the strip on `row`. A click on a segment opens its session.
pub(in crate::client) fn draw_strip(app: &mut App, buf: &mut Buffer, row: Rect, model: &[Proj], t: &Theme) {
    let ink = Ink::of(t);
    fill(buf, row, ink.bg);
    let sessions: Vec<(TermId, Color)> = session_order(app, model, t)
        .into_iter()
        .filter_map(|term| find(model, term).map(|(.., s)| (term, ink.of_session(s))))
        .collect();
    for (term, colour, x, w) in segments(&sessions, row) {
        put(buf, x, row.y, &[seg("━".repeat(w as usize), Style::default().fg(colour).bg(ink.bg))], row.right());
        hit(app, Rect { x, y: row.y, width: w, height: 1 }, ScreenHit::Session(term));
    }
}

/// Where each session's segment goes on `row`: equal shares of the width with a column between
/// them (the first ones a cell wider when it does not divide evenly), or with many sessions a
/// single cell each, left-aligned, as many as fit.
pub(super) fn segments(sessions: &[(TermId, Color)], row: Rect) -> Vec<(TermId, Color, u16, u16)> {
    let n = sessions.len() as u16;
    if n == 0 {
        return Vec::new();
    }
    let gaps = n - 1;
    let equal = sessions.len() <= EQUAL_UP_TO && row.width >= n + gaps;
    let (base, extra) = if equal { ((row.width - gaps) / n, (row.width - gaps) % n) } else { (1, 0) };
    let mut x = row.x;
    let mut out = Vec::new();
    for (i, &(term, colour)) in sessions.iter().enumerate() {
        let w = base + u16::from((i as u16) < extra);
        if x + w > row.right() {
            break;
        }
        out.push((term, colour, x, w));
        x += w + 1;
    }
    out
}
