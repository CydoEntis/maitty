//! The v7 handoff's reference frames for the read-while-working screens, cut down to what is
//! inside each sheet and popover (`v7_ref.txt`, from `docs/design/v7/render-frames.mjs`'s
//! output: per preset, layout and size, the inside's position and size, then its rows with
//! trailing spaces trimmed). Frame lines are not in it; they are tested with the frame.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const FRAMES: &str = include_str!("v7_ref.txt");

/// One reference: where the inside is on the screen, and its rows.
pub(super) struct Ref {
    pub area: Rect,
    pub rows: Vec<String>,
}

impl Ref {
    /// Row `i` as this package draws it where the handoff's sample data differs; `new` is
    /// clipped like a drawn row, which keeps the inside's last column clear.
    pub fn set_row(&mut self, i: usize, new: &str) {
        self.rows[i] = new.chars().take(self.area.width.saturating_sub(1) as usize).collect::<String>().trim_end().to_string();
    }
}

/// The reference for `preset` (its full name, `07 Evidence · short output`) in `layout`
/// (`floating`, `tiled`, `flat`) at `w`×`h`.
pub(super) fn reference(preset: &str, layout: &str, w: u16, h: u16) -> Ref {
    let head = format!("===== {preset} | {layout} | {w}x{h} | ");
    let mut lines = FRAMES.lines();
    let found = lines.by_ref().find(|l| l.starts_with(&head)).unwrap_or_else(|| panic!("no reference for {head}"));
    let geometry = &found[head.len()..];
    let (at, size) = geometry.split_once(' ').expect("x,y WxH");
    let (x, y) = at.split_once(',').expect("x,y");
    let (rw, rh) = size.split_once('x').expect("WxH");
    let n = |s: &str| s.parse::<u16>().expect("a number");
    let area = Rect { x: n(x), y: n(y), width: n(rw), height: n(rh) };
    let rows = lines.take(area.height as usize).map(str::to_string).collect();
    Ref { area, rows }
}

/// The text of `area` in `buf`, one row per line, trailing spaces trimmed.
pub(super) fn text(buf: &Buffer, area: Rect) -> Vec<String> {
    (area.y..area.bottom()).map(|y| (area.x..area.right()).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end().to_string()).collect()
}

/// The layout names the references use, in the fixture's order.
pub(super) fn layout_name(l: crate::client::v7_fixture::Layout) -> &'static str {
    use crate::client::v7_fixture::Layout;
    match l {
        Layout::Floating => "floating",
        Layout::Tiled => "tiled",
        Layout::Flat => "flat",
    }
}

/// Both sides row by row, so a failure shows where they part.
pub(super) fn assert_same(got: &[String], want: &Ref, what: &str) {
    if got == want.rows.as_slice() {
        return;
    }
    let mut report = String::new();
    for (i, (g, w)) in got.iter().zip(&want.rows).enumerate() {
        let mark = if g == w { ' ' } else { '≠' };
        report.push_str(&format!("{mark}{i:>2} got  |{g}\n    want |{w}\n"));
    }
    panic!("{what}: the inside differs from the v7 reference\n{report}");
}
