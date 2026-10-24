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
/// Without padding, the boxes' own edges on the ring take the colour, and the bare ground
/// between them a line, so the edge still reads as one frame; chips and words stay as they are.
pub(in crate::client) fn draw_mode_frame(app: &App, buf: &mut Buffer, area: Rect, t: &Theme) {
    let Some(color) = frame_color(app, t) else { return };
    let area = area.intersection(buf.area);
    if area.width < 2 || area.height < 2 {
        return;
    }
    let padded = framed(app, area) != area;
    let rounded = Look::of(&app.cfg.ui).rounded;
    for (x, y) in ring(area) {
        let edge = Edge::at(area, x, y);
        let cell = &mut buf[(x, y)];
        if padded {
            cell.set_symbol(edge.block()).set_fg(color);
        } else if is_box_line(cell.symbol()) {
            cell.set_fg(color);
        } else if cell.symbol() == " " && (cell.bg == t.bg || cell.bg == Color::Reset) {
            cell.set_symbol(edge.line(rounded)).set_fg(color);
        }
    }
}

/// Every cell on the outermost ring of `r`.
fn ring(r: Rect) -> impl Iterator<Item = (u16, u16)> {
    let rows = (r.left()..r.right()).flat_map(move |x| [(x, r.top()), (x, r.bottom() - 1)]);
    let cols = (r.top() + 1..r.bottom() - 1).flat_map(move |y| [(r.left(), y), (r.right() - 1, y)]);
    rows.chain(cols)
}

/// Where a ring cell sits.
#[derive(Clone, Copy)]
enum Edge {
    Top,
    Bottom,
    Side,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Edge {
    fn at(r: Rect, x: u16, y: u16) -> Edge {
        let (left, right, top) = (x == r.left(), x == r.right() - 1, y == r.top());
        match (left, right, top) {
            (true, _, true) => Edge::TopLeft,
            (_, true, true) => Edge::TopRight,
            (true, _, false) if y == r.bottom() - 1 => Edge::BottomLeft,
            (_, true, false) if y == r.bottom() - 1 => Edge::BottomRight,
            (false, false, true) => Edge::Top,
            (false, false, false) => Edge::Bottom,
            _ => Edge::Side,
        }
    }

    /// The solid frame in the padding.
    fn block(self) -> &'static str {
        match self {
            Edge::Top => "▀",
            Edge::Bottom => "▄",
            _ => "█",
        }
    }

    /// The thin line across a gap between boxes, without padding.
    fn line(self, rounded: bool) -> &'static str {
        match (self, rounded) {
            (Edge::Top | Edge::Bottom, _) => "─",
            (Edge::Side, _) => "│",
            (Edge::TopLeft, false) => "┌",
            (Edge::TopRight, false) => "┐",
            (Edge::BottomLeft, false) => "└",
            (Edge::BottomRight, false) => "┘",
            (Edge::TopLeft, true) => "╭",
            (Edge::TopRight, true) => "╮",
            (Edge::BottomLeft, true) => "╰",
            (Edge::BottomRight, true) => "╯",
        }
    }
}

/// A box-drawing line or corner: a card's border.
fn is_box_line(symbol: &str) -> bool {
    symbol.chars().next().is_some_and(|c| ('\u{2500}'..='\u{257f}').contains(&c))
}
