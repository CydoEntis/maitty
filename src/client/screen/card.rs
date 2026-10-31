//! The frame's pieces: cards with their title chip set into the top border, pills (buttons,
//! selected rows), and fading what isn't in focus.

use super::*;

/// A pill's two ends: a plain cell of `bg` each.
pub(in crate::client) fn ends(bg: Color) -> (Seg, Seg) {
    let st = Style::default().bg(bg);
    (seg(" ", st), seg(" ", st))
}

/// A one-row strip of `bg` across `r` (an input, a bar of buttons). Draw the strip's text from
/// a column in.
pub(in crate::client) fn strip(buf: &mut Buffer, r: Rect, bg: Color) {
    fill(buf, r, bg);
}

/// `text` on a pill in the style's background, as wide as `" text "` would be.
pub(in crate::client) fn chip(text: &str, st: Style) -> Vec<Seg> {
    let (l, r) = ends(st.bg.unwrap_or(Color::Reset));
    vec![l, seg(text, st), r]
}

/// How far the inside of an unfocused card fades toward its background: normal is the design's v5.
const DIM_NORMAL: f32 = 0.30;
const DIM_LIGHT: f32 = 0.2;
const DIM_HEAVY: f32 = 0.6;

/// The inside of a card: the same ground as the gaps between cards, so a card is told apart
/// by its edge and not by a slightly different grey.
pub(in crate::client) fn pane_bg(t: &Theme) -> Color {
    t.bg
}

/// How the cards are laid out and drawn, from the settings.
#[derive(Clone, Copy)]
pub(in crate::client) struct Look {
    /// Pane colours: each pane a box in its own colour (Floating only, ADR 0027).
    pub pane_colors: bool,
    /// Rounded card corners (`ui.rounded`).
    pub rounded: bool,
    /// A cell clear between one tab chip and the next (`ui.tab_gap`).
    pub tab_gap: bool,
    /// Rows between stacked cards, and columns between cards side by side.
    pub gap: u16,
    pub dim: f32,
}

impl Look {
    pub fn of(ui: &crate::config::Ui) -> Look {
        Look {
            pane_colors: ui.pane_colors,
            rounded: ui.rounded,
            tab_gap: ui.tab_gap,
            gap: crate::config::size_cells(&ui.gap).min(3),
            dim: match ui.dim.as_str() {
                "off" => 0.0,
                "light" | "subtle" => DIM_LIGHT,
                "heavy" | "60%" => DIM_HEAVY,
                _ => DIM_NORMAL,
            },
        }
    }
}

/// `segs` on a pill of `bg`, a space of it each end.
pub(in crate::client) fn pill(segs: Vec<Seg>, bg: Color) -> Vec<Seg> {
    let (l, r) = ends(bg);
    let mut out = vec![l];
    out.extend(segs.into_iter().map(|(x, st)| (x, st.bg(bg))));
    out.push(r);
    out
}

/// A pill as wide as `w`: the selected row of a list, an input, the answer bar.
pub(in crate::client) fn row_pill(buf: &mut Buffer, x: u16, y: u16, w: u16, bg: Color) {
    if w < 2 {
        return;
    }
    let segs = pill(vec![seg(" ".repeat((w - 2) as usize), Style::default())], bg);
    put(buf, x, y, &segs, x + w);
}

/// A card's corners by the setting: top left, top right, bottom left, bottom right.
pub(in crate::client) fn corners(look: Look) -> [&'static str; 4] {
    if look.rounded { ["╭", "╮", "╰", "╯"] } else { ["┌", "┐", "└", "┘"] }
}

/// A row's highlight laid over what's already drawn there (a glide passing over rows): its
/// ground becomes `bg`, its text stays.
pub(in crate::client) fn tint_row(buf: &mut Buffer, x: u16, y: u16, w: u16, bg: Color) {
    let area = buf.area;
    if w < 2 || y < area.y || y >= area.bottom() {
        return;
    }
    let right = (x + w).min(area.right());
    for xx in x.max(area.x)..right {
        buf[(xx, y)].set_bg(bg);
    }
}

/// A button as a pill: the primary one in the accent, the others raised with their key lit.
/// The pointer's tint is laid over every clickable region after drawing (`render`).
pub(in crate::client) fn button_pill(t: &Theme, label: &str, key: &str, primary: bool) -> Vec<Seg> {
    let (bg, fg, kf) = if primary { (t.accent, t.acc_ink, t.acc_ink) } else { (t.btn, t.strong, t.accent) };
    let b = Style::default().add_modifier(Modifier::BOLD);
    let mut segs = vec![seg(format!("{label} "), b.fg(fg))];
    if !key.is_empty() {
        segs.push(seg(key, b.fg(kf)));
    }
    pill(segs, bg)
}

/// What a card shows in and around its border.
pub(in crate::client) struct Card<'a> {
    pub title: &'a str,
    /// After the title in its chip, a space on, carrying its own separator: `INBOX 2`,
    /// `CHECK · fix-footer › test`.
    pub sub: &'a str,
    /// Drawn as they are in place of the title chip (a pane's tab chips).
    pub chips: Vec<Seg>,
    /// On the top border at the right, before the ✕.
    pub right: Vec<Seg>,
    /// The ✕ at the right of the top border, and what a click on it does.
    pub close: Option<ScreenHit>,
    pub border: Color,
    /// Focused: the border and the title chip in `border`.
    pub bold: bool,
    /// The title's ink when not focused.
    pub title_fg: Color,
    /// The inside's ground.
    pub bg: Color,
}

impl<'a> Card<'a> {
    pub fn new(t: &Theme, title: &'a str) -> Card<'a> {
        Card { title, sub: "", chips: Vec::new(), right: Vec::new(), close: None, border: t.line, bold: false, title_fg: t.strong, bg: pane_bg(t) }
    }

    /// The focused look: the border and the title chip in `c`.
    pub fn lit(mut self, c: Color) -> Card<'a> {
        self.border = c;
        self.bold = true;
        self
    }
}

/// How wide a card's ✕ is drawn on its top border, the spaces round it included.
pub(in crate::client) fn close_width(after_right: bool) -> u16 {
    close_lead(after_right) + RED_CLOSE_W
}

/// The red close button's width: the × and a cell each side, covering the corner. The padding
/// keeps a font's wide glyph off the border.
const RED_CLOSE_W: u16 = 3;

/// Spaces before the ×: two after other right-hand segments.
fn close_lead(after_right: bool) -> u16 {
    if after_right { 2 } else { 0 }
}

/// Where a card's title chip starts: on the top border two in.
pub(in crate::client) fn title_x(r: Rect) -> u16 {
    r.x + 2
}

/// Where a card's right-hand segments end (exclusive): a padding space before the corner.
pub(in crate::client) fn right_end(r: Rect) -> u16 {
    r.right().saturating_sub(2)
}

/// The title chip, at most `room` wide: the title (and its sub, dropped first) padded by a
/// space each side on `btn`, or on the focus colour when focused.
fn title_chip(c: &Card, t: &Theme, room: u16) -> Vec<Seg> {
    if !c.chips.is_empty() || c.title.is_empty() {
        return c.chips.clone();
    }
    let (bg, fg) = if c.bold { (c.border, if c.border == t.accent { t.acc_ink } else { t.ink_on(c.border) }) } else { (t.btn, c.title_fg) };
    let st = Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD);
    let sub = if c.sub.is_empty() { String::new() } else { format!(" {}", c.sub) };
    let fits = |s: &str| s.width() as u16 + 2 <= room;
    // A long sub is cut to the room left, so it still shows its start; only a title that does
    // not fit on its own loses it.
    let (title, sub) = if fits(&format!("{}{sub}", c.title)) {
        (c.title.to_string(), sub)
    } else if fits(c.title) {
        (c.title.to_string(), truncate(&sub, room.saturating_sub(2 + c.title.width() as u16) as usize))
    } else {
        (truncate(c.title, room.saturating_sub(2) as usize), String::new())
    };
    let mut out = vec![seg(format!(" {title}"), st)];
    if !sub.is_empty() {
        out.push(seg(sub, Style::default().bg(bg).fg(if c.bold { fg } else { t.text })));
    }
    out.push(seg(" ", st));
    out
}

/// Draw a card: a box in `line` (`border` when focused) with square or rounded corners, the
/// title chip on the top border two in and the right-hand segments ending three before the
/// corner. Returns the inside.
pub(in crate::client) fn card(app: &mut App, buf: &mut Buffer, r: Rect, c: &Card, t: &Theme) -> Rect {
    let look = Look::of(&app.cfg.ui);
    let r = r.intersection(buf.area);
    if r.width < 4 || r.height < 2 {
        return Rect { width: 0, height: 0, ..r };
    }
    let inside = box_frame(buf, r, c, look);
    let ground = t.bg;
    let mut right = c.right.clone();
    // The red button covers the corner cell: left clear, the corner and the padding before the
    // button would read as a gap in the border.
    let flush = c.close.is_some();
    if flush {
        right.push(seg(" ".repeat(close_lead(!right.is_empty()) as usize), Style::default()));
        let close = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons).close;
        right.push(seg(format!(" {close} "), Style::default().fg(t.strong).bg(t.err).add_modifier(Modifier::BOLD)));
    }
    let end = if flush { r.right() } else { right_end(r) };
    let rw = segs_width(&right);
    let rx = end.saturating_sub(rw);
    let chip_x = title_x(r);
    let shown = rw > 0 && rx > chip_x + 1;
    if shown {
        let pad = u16::from(!flush);
        let space = seg(" ", Style::default());
        let before = if flush { None } else { Some(space.clone()) };
        let after = if flush { None } else { Some(space) };
        let segs: Vec<Seg> = before.into_iter().chain(right).chain(after).map(|(s, st)| (s, if st.bg.is_none() { st.bg(ground) } else { st })).collect();
        put(buf, rx - pad, r.y, &segs, end + if flush { 0 } else { pad });
        if let Some(h) = c.close {
            hit(app, Rect { x: end.saturating_sub(RED_CLOSE_W), y: r.y, width: RED_CLOSE_W, height: 1 }, h);
        }
    }
    let chip_end = if shown { rx.saturating_sub(2) } else { end };
    put(buf, chip_x, r.y, &title_chip(c, t, chip_end.saturating_sub(chip_x)), chip_end);
    inside
}

/// The Cards box: `line` (or the focus colour) all round, corners by the setting.
fn box_frame(buf: &mut Buffer, r: Rect, c: &Card, look: Look) -> Rect {
    fill(buf, Rect { x: r.x + 1, y: r.y + 1, width: r.width - 2, height: r.height - 2 }, c.bg);
    let b = Style::default().fg(c.border);
    let b = if look.pane_colors && c.bold { b.add_modifier(Modifier::BOLD) } else { b };
    let (x, y, right, bottom) = (r.x, r.y, r.right() - 1, r.bottom() - 1);
    for xx in x + 1..right {
        buf[(xx, y)].set_symbol("─").set_style(b);
        buf[(xx, bottom)].set_symbol("─").set_style(b);
    }
    for yy in y + 1..bottom {
        buf[(x, yy)].set_symbol("│").set_style(b);
        buf[(right, yy)].set_symbol("│").set_style(b);
    }
    let [tl, tr, bl, br] = corners(look);
    buf[(x, y)].set_symbol(tl).set_style(b);
    buf[(right, y)].set_symbol(tr).set_style(b);
    buf[(x, bottom)].set_symbol(bl).set_style(b);
    buf[(right, bottom)].set_symbol(br).set_style(b);
    Rect { x: x + 1, y: y + 1, width: r.width - 2, height: r.height - 2 }
}

/// Fade everything inside `r` toward its own ground by `amount`, keeping colours recognisable.
pub(in crate::client) fn dim_inside(buf: &mut Buffer, r: Rect, amount: f32, t: &Theme) {
    if amount <= 0.0 {
        return;
    }
    let r = r.intersection(buf.area);
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            let c = &mut buf[(x, y)];
            let bg = if c.bg == Color::Reset { t.bg } else { c.bg };
            let fg = if c.fg == Color::Reset { t.fg } else { c.fg };
            c.fg = blend(fg, bg, amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pill_ends_in_a_plain_cell_of_its_colour() {
        let (pill_bg, ground) = (Color::Rgb(40, 40, 40), Color::Rgb(10, 20, 30));
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        buf.set_style(Rect::new(0, 0, 8, 1), Style::default().bg(ground));
        row_pill(&mut buf, 1, 0, 6, pill_bg);
        for x in [1, 3, 6] {
            let c = &buf[(x, 0)];
            assert_eq!((c.symbol(), c.bg), (" ", pill_bg), "cell {x}");
        }
        assert_eq!(buf[(0, 0)].bg, ground, "nothing beside it");
    }
}
