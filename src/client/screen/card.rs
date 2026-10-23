//! The frame's pieces: cards with their title chip set into the top border (Flat: a header
//! strip), pills (buttons, selected rows), and fading what isn't in focus.

use super::*;

/// Nerd Font half-circles that round a pill's ends.
pub(in crate::client) const CAP_L: &str = "\u{e0b6}";
pub(in crate::client) const CAP_R: &str = "\u{e0b4}";

thread_local! {
    /// Round pill ends (the `ui.pill_caps` setting), for drawing that has no `Look` at hand.
    /// Per thread because frames are drawn on one thread; a process-wide flag let tests that
    /// draw in parallel overwrite each other's setting mid-frame.
    static ROUND: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Set once a frame from the settings.
pub(in crate::client) fn set_round(on: bool) {
    ROUND.set(on);
}

fn round() -> bool {
    ROUND.get()
}

/// A pill's two ends in `bg`: half-circles that keep the ground under them, or plain cells
/// of `bg` (square ends) without a Nerd Font.
pub(in crate::client) fn ends(bg: Color) -> (Seg, Seg) {
    if round() {
        let st = Style::default().fg(bg);
        (seg(CAP_L, st), seg(CAP_R, st))
    } else {
        let st = Style::default().bg(bg);
        (seg(" ", st), seg(" ", st))
    }
}

/// A one-row strip of `bg` across `r` with round ends (an input, a bar of buttons). The ends
/// keep the ground under them; draw the strip's text from a column in.
pub(in crate::client) fn strip(buf: &mut Buffer, r: Rect, bg: Color) {
    if r.width < 3 || !round() {
        fill(buf, r, bg);
        return;
    }
    fill(buf, Rect { x: r.x + 1, width: r.width - 2, ..r }, bg);
    let (l, e) = ends(bg);
    put(buf, r.x, r.y, &[l], r.x + 1);
    put(buf, r.right() - 1, r.y, &[e], r.right());
}

/// `text` on a pill in the style's background, as wide as `" text "` would be.
pub(in crate::client) fn chip(text: &str, st: Style) -> Vec<Seg> {
    let (l, r) = ends(st.bg.unwrap_or(Color::Reset));
    vec![l, seg(text, st), r]
}

/// How far the inside of an unfocused card fades toward its background at "30%" (the design's v5).
const DIM_DEFAULT: f32 = 0.30;
const DIM_SUBTLE: f32 = 0.2;
const DIM_STRONG: f32 = 0.6;

/// The inside of a card: the same ground as the gaps between cards, so a card is told apart
/// by its edge and not by a slightly different grey.
pub(in crate::client) fn pane_bg(t: &Theme) -> Color {
    t.bg
}

/// How the cards are laid out and drawn, from the settings.
#[derive(Clone, Copy)]
pub(in crate::client) struct Look {
    pub flat: bool,
    pub rounded: bool,
    /// Rows between stacked cards, and columns between cards side by side.
    pub gap: u16,
    pub dim: f32,
    pub caps: bool,
    /// Round the ends of the tab chips and the sidebar's selected folder row (`ui.round_tabs`).
    pub round_tabs: bool,
    /// Round the ends of the sidebar's session rows (`ui.round_rows`).
    pub round_rows: bool,
}

impl Look {
    pub fn of(ui: &crate::config::Ui) -> Look {
        let flat = ui.layout == "flat";
        let tiled = ui.panes == "tiled" || flat;
        Look {
            flat,
            rounded: ui.corners == "rounded",
            gap: if tiled { 0 } else { ui.gap.parse().unwrap_or(1).min(2) },
            dim: if flat { 0.0 } else { match ui.dim.as_str() {
                "off" => 0.0,
                "subtle" => DIM_SUBTLE,
                "60%" => DIM_STRONG,
                _ => DIM_DEFAULT,
            } },
            caps: ui.pill_caps,
            round_tabs: ui.round_tabs,
            round_rows: ui.round_rows,
        }
    }
}

/// `segs` on a pill of `bg`, its ends rounded on `outer` (or squared off without a Nerd Font).
pub(in crate::client) fn pill(look: Look, segs: Vec<Seg>, bg: Color, outer: Color) -> Vec<Seg> {
    let (l, r) = if look.caps { (CAP_L, CAP_R) } else { (" ", " ") };
    let end = |s: &str| if look.caps { seg(s, Style::default().fg(bg).bg(outer)) } else { seg(s, Style::default().bg(bg)) };
    let mut out = vec![end(l)];
    out.extend(segs.into_iter().map(|(x, st)| (x, st.bg(bg))));
    out.push(end(r));
    out
}

/// `segs` with a half-circle cap each end, in the first and last segment's background, over
/// `outer`. Unlike `pill` it keeps each segment's own colours, so a number chip and a name
/// chip stay two colours inside one rounded shape.
pub(in crate::client) fn round_ends(segs: Vec<Seg>, outer: Color) -> Vec<Seg> {
    let cap = |glyph: &str, st: Option<&Seg>| {
        let bg = st.and_then(|(_, s)| s.bg).unwrap_or(outer);
        seg(glyph, Style::default().fg(bg).bg(outer))
    };
    let (l, r) = (cap(CAP_L, segs.first()), cap(CAP_R, segs.last()));
    let mut out = vec![l];
    out.extend(segs);
    out.push(r);
    out
}

/// What lies behind a pane's title chips: the header strip in Flat, the desk the top border
/// is drawn on in Cards (no ground of its own).
pub(in crate::client) fn title_ground(look: Look, t: &Theme) -> Color {
    if look.flat { t.card } else { Color::Reset }
}

/// A pill as wide as `w`: the selected row of a list, an input, the answer bar.
pub(in crate::client) fn row_pill(look: Look, buf: &mut Buffer, x: u16, y: u16, w: u16, bg: Color, outer: Color) {
    if w < 2 {
        return;
    }
    let segs = pill(look, vec![seg(" ".repeat((w - 2) as usize), Style::default())], bg, outer);
    put(buf, x, y, &segs, x + w);
}

/// A card's corners by the setting: top left, top right, bottom left, bottom right.
pub(in crate::client) fn corners(look: Look) -> [&'static str; 4] {
    if look.rounded { ["╭", "╮", "╰", "╯"] } else { ["┌", "┐", "└", "┘"] }
}

/// A row's highlight laid over what's already drawn there (a glide passing over rows): its
/// ground becomes `bg`, its text stays, with the pill's ends.
pub(in crate::client) fn tint_row(look: Look, buf: &mut Buffer, x: u16, y: u16, w: u16, bg: Color, outer: Color) {
    let area = buf.area;
    if w < 2 || y < area.y || y >= area.bottom() {
        return;
    }
    let right = (x + w).min(area.right());
    for xx in x.max(area.x)..right {
        buf[(xx, y)].set_bg(bg);
    }
    if look.caps {
        buf[(x, y)].set_symbol(CAP_L).set_fg(bg).set_bg(outer);
        buf[(right - 1, y)].set_symbol(CAP_R).set_fg(bg).set_bg(outer);
    }
}

/// A button as a pill: the primary one in the accent, the others raised with their key lit.
/// The pointer's tint is laid over every clickable region after drawing (`render`).
pub(in crate::client) fn button_pill(look: Look, t: &Theme, label: &str, key: &str, primary: bool, outer: Color) -> Vec<Seg> {
    let (bg, fg, kf) = if primary { (t.accent, t.acc_ink, t.acc_ink) } else { (t.btn, t.strong, t.accent) };
    let b = Style::default().add_modifier(Modifier::BOLD);
    let mut segs = vec![seg(format!("{label} "), b.fg(fg))];
    if !key.is_empty() {
        segs.push(seg(key, b.fg(kf)));
    }
    pill(look, segs, bg, outer)
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
    /// Focused: the border (Cards) or the left rule (Flat), and the title chip, in `border`.
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

/// Where a card's title chip starts: on the top border two in (Cards), one in on the header
/// strip (Flat).
pub(in crate::client) fn title_x(look: Look, r: Rect) -> u16 {
    r.x + if look.flat { 1 } else { 2 }
}

/// Where a card's right-hand segments end (exclusive): a padding space before the corner
/// (Cards), a column before the zone's edge (Flat).
pub(in crate::client) fn right_end(look: Look, r: Rect) -> u16 {
    r.right().saturating_sub(if look.flat { 1 } else { 2 })
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
    let (title, sub) = if fits(&format!("{}{sub}", c.title)) { (c.title.to_string(), sub) } else { (truncate(c.title, room.saturating_sub(2) as usize), String::new()) };
    let mut out = vec![seg(format!(" {title}"), st)];
    if !sub.is_empty() {
        out.push(seg(sub, Style::default().bg(bg).fg(if c.bold { fg } else { t.text })));
    }
    out.push(seg(" ", st));
    out
}

/// Draw a card. Cards: a box in `line` (`border` when focused) with square or rounded
/// corners, the title chip on the top border two in and the right-hand segments ending three
/// before the corner. Flat: no box; the first row is a header strip of `card` with the chip one
/// in and the right-hand segments one from the edge, and a zone that doesn't start at the
/// screen's edge has a `│` rule down its left (in `border` when focused). Returns the inside.
pub(in crate::client) fn card(app: &mut App, buf: &mut Buffer, r: Rect, c: &Card, t: &Theme) -> Rect {
    let look = Look::of(&app.cfg.ui);
    let r = r.intersection(buf.area);
    if r.width < 4 || r.height < 2 {
        return Rect { width: 0, height: 0, ..r };
    }
    let inside = if look.flat { flat_frame(buf, r, c, t) } else { box_frame(buf, r, c, look) };
    let ground = if look.flat { t.card } else { t.bg };
    let mut right = c.right.clone();
    if c.close.is_some() {
        let gap = if right.is_empty() { "" } else { "  " };
        let close = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons).close;
        right.push(seg(format!("{gap}{close} "), Style::default().fg(t.muted)));
    }
    let end = right_end(look, r);
    let rw = segs_width(&right);
    let rx = end.saturating_sub(rw);
    let chip_x = title_x(look, r);
    let shown = rw > 0 && rx > chip_x + 1;
    if shown {
        let pad = u16::from(!look.flat);
        let space = seg(" ".repeat(pad as usize), Style::default());
        let segs: Vec<Seg> = std::iter::once(space.clone()).chain(right).chain(std::iter::once(space)).map(|(s, st)| (s, if st.bg.is_none() { st.bg(ground) } else { st })).collect();
        put(buf, rx - pad, r.y, &segs, end + pad);
        if let Some(h) = c.close {
            hit(app, Rect { x: end.saturating_sub(3), y: r.y, width: 3, height: 1 }, h);
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

/// The Flat zone: a header strip of `card` on its first row, and a `│` rule down its left
/// unless it starts at the screen's edge.
fn flat_frame(buf: &mut Buffer, r: Rect, c: &Card, t: &Theme) -> Rect {
    fill(buf, r, c.bg);
    fill(buf, Rect { height: 1, ..r }, t.card);
    if r.x > buf.area.x {
        buf[(r.x, r.y)].set_symbol("│").set_style(Style::default().fg(t.line).bg(t.card));
        let rule = Style::default().fg(if c.bold { c.border } else { t.line }).bg(c.bg);
        for yy in r.y + 1..r.bottom() {
            buf[(r.x, yy)].set_symbol("│").set_style(rule);
        }
    }
    Rect { x: r.x + 1, y: r.y + 1, width: r.width - 1, height: r.height - 1 }
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

    /// Frames are drawn on the UI thread, but tests draw on many threads at once; one test's
    /// layout must not change the pill ends another is drawing.
    #[test]
    fn the_pill_end_style_is_per_thread() {
        set_round(true);
        std::thread::spawn(|| set_round(false)).join().unwrap();
        assert_eq!(ends(Color::Red).0.0, CAP_L, "another thread's setting leaked into this one");
    }

    fn look(layout: &str, caps: bool) -> Look {
        let ui = crate::config::Ui { layout: layout.into(), pill_caps: caps, ..Default::default() };
        Look::of(&ui)
    }

    #[test]
    fn pill_caps_apply_in_every_layout() {
        for layout in ["cards", "flat"] {
            assert!(look(layout, true).caps, "{layout}: caps on");
            assert!(!look(layout, false).caps, "{layout}: caps off");
        }
    }

    #[test]
    fn a_flat_pill_end_sits_on_the_zones_ground() {
        let (pill_bg, ground) = (Color::Rgb(40, 40, 40), Color::Rgb(10, 20, 30));
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        row_pill(look("flat", true), &mut buf, 1, 0, 6, pill_bg, ground);
        for (x, cap) in [(1, CAP_L), (6, CAP_R)] {
            let c = &buf[(x, 0)];
            assert_eq!((c.symbol(), c.fg, c.bg), (cap, pill_bg, ground), "end at {x}");
        }
        assert_eq!(buf[(3, 0)].bg, pill_bg);

        let mut square = Buffer::empty(Rect::new(0, 0, 8, 1));
        row_pill(look("flat", false), &mut square, 1, 0, 6, pill_bg, ground);
        assert_eq!((square[(1, 0)].symbol(), square[(1, 0)].bg), (" ", pill_bg), "off: a plain cell of the pill's colour");
    }
}
