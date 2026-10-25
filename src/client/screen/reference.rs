//! The reference frames for the read-while-working screens, cut down to what is
//! inside each sheet and popover (`reference.txt`, a fixed fixture:
//! per preset, layout and size, the inside's position and size, then its rows with
//! trailing spaces trimmed). Frame lines are not in it; they are tested with the frame.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const FRAMES: &str = include_str!("reference.txt");

/// One reference: where the inside is on the screen, and its rows.
pub(super) struct Ref {
    pub area: Rect,
    pub rows: Vec<String>,
}

impl Ref {
    /// Row `i` as this package draws it where the reference's sample data differs; `new` is
    /// clipped like a drawn row, which keeps the inside's last column clear.
    pub fn set_row(&mut self, i: usize, new: &str) {
        self.rows[i] = new.chars().take(self.area.width.saturating_sub(1) as usize).collect::<String>().trim_end().to_string();
    }

    /// A failure offers Send to agent after Override, a button the reference predates. On a
    /// screen under 100 columns it takes the evidence sheet's room for Open session, and the
    /// Inbox has no room for it.
    pub fn offers_send_to_agent(&mut self, screen_w: u16) {
        let (from, to) = if screen_w >= 100 { ("Override… O", "Override… O   Send to agent m") } else { ("Override… O   Open session o", "Override… O   Send to agent m") };
        for i in 0..self.rows.len() {
            if self.rows[i].contains(from) {
                let new = self.rows[i].replacen(from, to, 1);
                self.set_row(i, &new);
            }
        }
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
pub(super) fn layout_name(l: crate::client::reference_fixture::Layout) -> &'static str {
    use crate::client::reference_fixture::Layout;
    match l {
        Layout::Floating => "floating",
        Layout::Tiled => "tiled",
        Layout::Flat => "flat",
    }
}

/// A key-hint row (the action bar draws each key as a cap, which spaces the words out) is
/// the same row when it reads the same; every other row has to match exactly.
fn same_row(got: &str, want: &str) -> bool {
    let bare = |r: &str| r.replace(['\u{e0b6}', '\u{e0b4}'], " ");
    if got == want {
        return true;
    }
    if !want.contains("Esc") {
        return false;
    }
    // The caps are wider than the plain words, so a narrow sheet gives up hints (never `Esc`)
    // or cuts the row sooner: what is drawn is the reference's words in order.
    let (g, w): (Vec<String>, Vec<String>) = (bare(got).split_whitespace().map(str::to_string).collect(), want.split_whitespace().map(str::to_string).collect());
    let mut rest = w.iter();
    let mut at_end = false;
    g.iter().all(|word| {
        // The reference may stop at `Esc` where this row goes on with what it does.
        if at_end {
            return true;
        }
        let found = rest.any(|x| x == word || x.starts_with(word.as_str()));
        at_end = found && word == "Esc";
        found
    })
}

/// Both sides row by row, so a failure shows where they part.
pub(super) fn assert_same(got: &[String], want: &Ref, what: &str) {
    if got.iter().zip(&want.rows).all(|(g, w)| same_row(g, w)) && got.len() == want.rows.len() {
        return;
    }
    let mut report = String::new();
    for (i, (g, w)) in got.iter().zip(&want.rows).enumerate() {
        let mark = if same_row(g, w) { ' ' } else { '≠' };
        report.push_str(&format!("{mark}{i:>2} got  |{g}\n    want |{w}\n"));
    }
    panic!("{what}: the inside differs from the reference\n{report}");
}
