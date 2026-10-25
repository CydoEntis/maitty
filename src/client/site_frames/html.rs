//! A drawn buffer as an HTML `<pre>`: the same cells, colours and weights, readable anywhere
//! without the app's fonts.
//!
//! A glyph the page's font may lack is wrapped in `<span class="g">` (one cell) or `g2` (two),
//! so a wider fallback font cannot push the rest of its row right. The page must hold them to
//! their cells:
//!
//! ```css
//! .frame .g,.frame .g2{display:inline-block;overflow:hidden;text-align:center;vertical-align:top}
//! .frame .g{width:1ch}
//! .frame .g2{width:2ch}
//! ```

use crate::client::render::blend;
use crate::client::screen::{CAP_L, CAP_R};
use crate::theme::Theme;
use ratatui::buffer::{Buffer, Cell};
use ratatui::style::{Color, Modifier};
use std::fmt::Write as _;
use unicode_width::UnicodeWidthStr;

/// `buf` as a `<pre class="frame">`: one line a row, runs of cells drawn alike in one `<span>`
/// with inline colours (the terminal's default ones are the theme's, so the frame is opaque).
/// A wide character is written once, for both its cells; a chip's half-circle end (a Nerd Font
/// glyph) is drawn with a gradient, so no Nerd Font is needed.
pub(super) fn frame(buf: &Buffer, t: &Theme) -> String {
    let a = buf.area;
    let mut out = format!("<pre class=\"frame\" data-cols=\"{}\" data-rows=\"{}\">", a.width, a.height);
    for y in 0..a.height {
        let mut run: Option<(String, String)> = None;
        let mut x = 0;
        while x < a.width {
            let cell = &buf[(x, y)];
            let (style, text) = cell_html(cell, t);
            match &mut run {
                Some((s, txt)) if *s == style => txt.push_str(&text),
                _ => {
                    flush(&mut out, run.take());
                    run = Some((style, text));
                }
            }
            x += cell.symbol().width().max(1) as u16;
        }
        flush(&mut out, run);
        if y + 1 < a.height {
            out.push('\n');
        }
    }
    out.push_str("</pre>\n");
    out
}

fn flush(out: &mut String, run: Option<(String, String)>) {
    if let Some((style, text)) = run {
        let _ = write!(out, "<span style=\"{style}\">{text}</span>");
    }
}

/// A cell's inline style and its escaped text.
fn cell_html(cell: &Cell, t: &Theme) -> (String, String) {
    let m = cell.modifier;
    let (mut fg, mut bg) = (resolve(cell.fg, t.fg), resolve(cell.bg, t.bg));
    if m.contains(Modifier::REVERSED) {
        (fg, bg) = (bg, fg);
    }
    if m.contains(Modifier::DIM) {
        fg = blend(bg, fg, 0.5);
    }
    if m.contains(Modifier::HIDDEN) {
        fg = bg;
    }
    let sym = cell.symbol();
    // The flat side of a cap meets its chip: a left cap is a half disc centred on its right edge.
    let flat_side = match sym {
        CAP_L => Some("100%"),
        CAP_R => Some("0%"),
        _ => None,
    };
    if let Some(side) = flat_side {
        return (format!("background:radial-gradient(ellipse 100% 50% at {side} 50%,{} 98%,{} 100%)", hex(fg), hex(bg)), " ".into());
    }
    let mut style = format!("color:{};background-color:{}", hex(fg), hex(bg));
    for (bit, css) in [
        (Modifier::BOLD, ";font-weight:bold"),
        (Modifier::ITALIC, ";font-style:italic"),
    ] {
        if m.contains(bit) {
            style.push_str(css);
        }
    }
    // One declaration for both lines, since a second `text-decoration` would replace the first.
    match (m.contains(Modifier::UNDERLINED), m.contains(Modifier::CROSSED_OUT)) {
        (true, true) => style.push_str(";text-decoration:underline line-through"),
        (true, false) => style.push_str(";text-decoration:underline"),
        (false, true) => style.push_str(";text-decoration:line-through"),
        (false, false) => {}
    }
    (style, glyph_html(if sym.is_empty() { " " } else { sym }))
}

/// `sym` escaped, and wrapped to its cell width unless every monospace font draws it in one cell.
fn glyph_html(sym: &str) -> String {
    let text = escape(sym);
    if sym.chars().all(fits_any_font) {
        return text;
    }
    let class = if sym.width() > 1 { "g2" } else { "g" };
    format!("<span class=\"{class}\">{text}</span>")
}

/// ASCII, box drawing and block elements, left unwrapped to keep the frame small: borders are
/// most of it.
fn fits_any_font(c: char) -> bool {
    c.is_ascii() || ('\u{2500}'..='\u{259F}').contains(&c)
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// `c` as an RGB colour; the terminal's default is `default`.
fn resolve(c: Color, default: Color) -> Color {
    let named = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Gray,
        Color::DarkGray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ];
    match c {
        Color::Reset => default,
        Color::Rgb(..) => c,
        Color::Indexed(i) => xterm(i),
        _ => xterm(named.iter().position(|n| *n == c).unwrap_or(7) as u8),
    }
}

/// The xterm palette's colour `i`.
fn xterm(i: u8) -> Color {
    const BASE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
    match i {
        0..=15 => {
            let (r, g, b) = BASE[i as usize];
            Color::Rgb(r, g, b)
        }
        16..=231 => {
            let n = i - 16;
            Color::Rgb(level(n / 36), level(n / 6 % 6), level(n % 6))
        }
        _ => {
            let v = 8 + (i - 232) * 10;
            Color::Rgb(v, v, v)
        }
    }
}

pub(super) fn hex(c: Color) -> String {
    match resolve(c, Color::Rgb(0, 0, 0)) {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => unreachable!("resolve gives RGB"),
    }
}

mod tests {
    use super::*;
    use ratatui::layout::Rect;
    use ratatui::style::Style;

    fn night() -> Theme {
        Theme::named("night")
    }

    #[test]
    fn a_run_of_alike_cells_is_one_span_and_the_default_colours_are_the_theme_s() {
        let t = night();
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_string(0, 0, "ab", Style::default());
        buf.set_string(2, 0, "cd", Style::default().fg(Color::Rgb(1, 2, 3)).add_modifier(Modifier::BOLD));
        let want = format!(
            "<pre class=\"frame\" data-cols=\"4\" data-rows=\"1\"><span style=\"color:{fg};background-color:{bg}\">ab</span><span style=\"color:#010203;background-color:{bg};font-weight:bold\">cd</span></pre>\n",
            fg = hex(t.fg),
            bg = hex(t.bg)
        );
        assert_eq!(frame(&buf, &t), want);
    }

    #[test]
    fn markup_is_escaped_and_a_wide_character_is_written_once() {
        let t = night();
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 2));
        buf.set_string(0, 0, "<&\">", Style::default());
        buf.set_string(0, 1, "界ab", Style::default());
        let out = frame(&buf, &t);
        assert!(out.contains(">&lt;&amp;&quot;&gt; </span>"), "{out}");
        assert!(out.contains("><span class=\"g2\">界</span>ab </span>"), "the spacer cell after 界 is skipped: {out}");
    }

    #[test]
    fn a_glyph_the_font_may_lack_is_held_to_its_cell() {
        let t = night();
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_string(0, 0, "⎇ a─", Style::default());
        let out = frame(&buf, &t);
        assert!(out.contains("><span class=\"g\">⎇</span> a─</span>"), "{out}");
    }

    #[test]
    fn ascii_is_not_wrapped() {
        let t = night();
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
        buf.set_string(0, 0, "abc", Style::default());
        assert!(!frame(&buf, &t).contains("class=\"g"));
    }

    #[test]
    fn a_half_circle_end_is_drawn_without_a_nerd_font() {
        let t = night();
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        buf.set_string(0, 0, CAP_L, Style::default().fg(Color::Rgb(255, 0, 0)).bg(Color::Rgb(0, 0, 255)));
        let out = frame(&buf, &t);
        assert!(out.contains("radial-gradient(ellipse 100% 50% at 100% 50%,#ff0000 98%,#0000ff 100%)\"> </span>"), "{out}");
        assert!(!out.contains(CAP_L));
    }

    #[test]
    fn indexed_and_named_colours_are_the_xterm_palette() {
        assert_eq!(hex(Color::Indexed(1)), "#cd0000");
        assert_eq!(hex(Color::Blue), "#0000ee");
        assert_eq!(hex(Color::Indexed(196)), "#ff0000");
        assert_eq!(hex(Color::Indexed(244)), "#808080");
    }
}
