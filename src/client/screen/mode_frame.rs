//! The frame round the whole window while shortcut mode is on (ADR 0023): the one sign of the
//! mode you can't miss, wherever you are looking.

use super::*;

/// How far the frame fades toward the ground while a popup or view holds the mode.
const PAUSED_FADE: f32 = 0.6;

/// The frame's colour: sky while shortcut mode is lit, faded while it is held under a popup or
/// view, none while you type.
fn frame_color(app: &App, t: &Theme) -> Option<Color> {
    if armed(app) {
        Some(t.sky())
    } else if shortcut_paused(app) {
        Some(crate::theme::mix(t.sky(), t.bg, PAUSED_FADE))
    } else {
        None
    }
}

/// Draw the frame on the window's outermost ring. With padding, the ring is empty: a half-row
/// line along the top and bottom and a full column at each side, so all four read equally thick.
/// Without padding, only the boxes' own edges on the ring take the colour; chips, words and
/// strips stay exactly as they are.
pub(in crate::client) fn draw_mode_frame(app: &App, buf: &mut Buffer, area: Rect, t: &Theme) {
    let Some(color) = frame_color(app, t) else { return };
    let area = area.intersection(buf.area);
    if area.width < 2 || area.height < 2 {
        return;
    }
    let padded = framed(app, area) != area;
    for (x, y) in ring(area) {
        let cell = &mut buf[(x, y)];
        if padded {
            cell.set_symbol(block(area, x, y)).set_fg(color);
        } else if is_box_line(cell.symbol()) {
            cell.set_fg(color);
        }
    }
}

/// Every cell on the outermost ring of `r`.
fn ring(r: Rect) -> impl Iterator<Item = (u16, u16)> {
    let rows = (r.left()..r.right()).flat_map(move |x| [(x, r.top()), (x, r.bottom() - 1)]);
    let cols = (r.top() + 1..r.bottom() - 1).flat_map(move |y| [(r.left(), y), (r.right() - 1, y)]);
    rows.chain(cols)
}

/// The solid frame's glyph at a ring cell: the upper half along the top, the lower half along
/// the bottom, a full cell at the sides and corners.
fn block(r: Rect, x: u16, y: u16) -> &'static str {
    let side = x == r.left() || x == r.right() - 1;
    match (side, y == r.top()) {
        (true, _) => "█",
        (false, true) => "▀",
        (false, false) => "▄",
    }
}

/// A box-drawing line or corner: a card's border.
fn is_box_line(symbol: &str) -> bool {
    symbol.chars().next().is_some_and(|c| ('\u{2500}'..='\u{257f}').contains(&c))
}
