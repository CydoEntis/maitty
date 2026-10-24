//! Markdown drawn for the Files preview: the text, a width and the theme in, styled lines out.
//! Pretty lines don't map to source lines; anything that needs a line number shows the raw text.

use std::path::Path;

use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::design::{Seg, seg};
use crate::theme::Theme;

/// A table column is cut no narrower than this, so its text stays readable.
const MIN_COLUMN: usize = 3;
const COLUMN_GAP: &str = " │ ";

pub(super) fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
}

/// `text` drawn as lines of at most `width` columns.
pub(super) fn render(text: &str, width: u16, t: &Theme) -> Vec<Vec<Seg>> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut r = Renderer::new(width.max(1) as usize, t);
    for event in Parser::new_ext(text, options) {
        r.event(event);
    }
    r.finish()
}

struct Table {
    aligns: Vec<Alignment>,
    rows: Vec<Vec<Vec<Seg>>>,
    head_rows: usize,
}

struct Renderer<'t> {
    t: &'t Theme,
    width: usize,
    out: Vec<Vec<Seg>>,
    /// The last line is the blank between two blocks.
    gap: bool,
    /// The words of the block being read.
    inline: Vec<Seg>,
    bold: usize,
    italic: usize,
    strike: usize,
    heading: Option<HeadingLevel>,
    /// Each open link: its URL, and where its text starts in `inline`.
    links: Vec<(String, usize)>,
    image: bool,
    quotes: usize,
    /// Each open list: the next number, or None for bullets.
    lists: Vec<Option<u64>>,
    /// Each open item's indent: the width of its marker.
    indents: Vec<usize>,
    /// The marker the item's first line still has to show.
    marker: Option<String>,
    code: Option<String>,
    table: Option<Table>,
}

impl<'t> Renderer<'t> {
    fn new(width: usize, t: &'t Theme) -> Self {
        Renderer {
            t,
            width,
            out: Vec::new(),
            gap: false,
            inline: Vec::new(),
            bold: 0,
            italic: 0,
            strike: 0,
            heading: None,
            links: Vec::new(),
            image: false,
            quotes: 0,
            lists: Vec::new(),
            indents: Vec::new(),
            marker: None,
            code: None,
            table: None,
        }
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(s) => match self.code.as_mut() {
                Some(code) => code.push_str(&s),
                None => self.inline.push(seg(s.to_string(), self.style())),
            },
            Event::Code(s) => self.inline.push(seg(s.to_string(), Style::default().fg(self.t.strong).bg(self.t.card2))),
            Event::InlineHtml(s) | Event::InlineMath(s) | Event::FootnoteReference(s) => self.inline.push(seg(s.to_string(), self.muted())),
            Event::Html(s) | Event::DisplayMath(s) => {
                for line in s.lines() {
                    let body = cut(&[seg(line.to_string(), self.muted())], self.room());
                    self.emit(body);
                }
            }
            Event::SoftBreak => self.inline.push(seg(" ", self.style())),
            Event::HardBreak => self.flush(),
            Event::Rule => {
                self.flush();
                self.rule();
                self.blank();
            }
            Event::TaskListMarker(done) => {
                let (mark, colour) = if done { ("[x] ", self.t.done) } else { ("[ ] ", self.t.muted) };
                self.inline.push(seg(mark, Style::default().fg(colour)));
            }
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => self.heading = Some(level),
            Tag::BlockQuote(_) => {
                self.flush();
                self.quotes += 1;
            }
            Tag::CodeBlock(_) => {
                self.flush();
                self.code = Some(String::new());
            }
            Tag::List(first) => {
                self.flush();
                self.lists.push(first);
            }
            Tag::Item => self.start_item(),
            Tag::Table(aligns) => {
                self.flush();
                self.table = Some(Table { aligns, rows: Vec::new(), head_rows: 0 });
            }
            Tag::TableHead => {
                self.bold += 1;
                self.table_row();
            }
            Tag::TableRow => self.table_row(),
            Tag::Emphasis => self.italic += 1,
            Tag::Strong => self.bold += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Link { dest_url, .. } => self.links.push((dest_url.to_string(), self.inline.len())),
            Tag::Image { .. } => self.image = true,
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush();
                self.blank();
            }
            TagEnd::Heading(level) => {
                self.flush();
                self.heading = None;
                if level == HeadingLevel::H1 {
                    self.rule();
                }
                self.blank();
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                // The quote's own last blank would keep its bar: end on the text instead.
                if self.gap {
                    self.out.pop();
                    self.gap = false;
                }
                self.quotes = self.quotes.saturating_sub(1);
                self.blank();
            }
            TagEnd::CodeBlock => {
                let code = self.code.take().unwrap_or_default();
                self.code_block(&code);
                self.blank();
            }
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
                if self.lists.is_empty() {
                    self.blank();
                }
            }
            TagEnd::Item => {
                if self.marker.is_some() && self.inline.is_empty() {
                    self.emit(Vec::new());
                }
                self.flush();
                self.indents.pop();
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.draw_table(table);
                }
                self.blank();
            }
            TagEnd::TableHead => {
                self.bold = self.bold.saturating_sub(1);
                if let Some(table) = self.table.as_mut() {
                    table.head_rows = table.rows.len();
                }
            }
            TagEnd::TableCell => {
                let cell = std::mem::take(&mut self.inline);
                if let Some(row) = self.table.as_mut().and_then(|tb| tb.rows.last_mut()) {
                    row.push(cell);
                }
            }
            TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
            TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link => self.end_link(),
            TagEnd::Image => self.image = false,
            _ => {}
        }
    }

    fn start_item(&mut self) {
        self.flush();
        let marker = match self.lists.last_mut() {
            Some(Some(n)) => {
                *n += 1;
                format!("{}. ", *n - 1)
            }
            _ => "• ".to_string(),
        };
        self.indents.push(marker.width());
        // An item that opens with a list shows both markers on that first line.
        let outer = self.marker.take().unwrap_or_default();
        self.marker = Some(outer + &marker);
    }

    /// The URL follows the link's text, unless the text already is the URL.
    fn end_link(&mut self) {
        let Some((url, from)) = self.links.pop() else { return };
        let text: String = self.inline[from.min(self.inline.len())..].iter().map(|(s, _)| s.as_str()).collect();
        if !url.is_empty() && text != url && !url.starts_with("mailto:") {
            self.inline.push(seg(format!(" ({url})"), self.muted()));
        }
    }

    fn table_row(&mut self) {
        if let Some(table) = self.table.as_mut() {
            table.rows.push(Vec::new());
        }
    }

    fn style(&self) -> Style {
        let t = self.t;
        let mut s = Style::default().fg(if self.quotes > 0 { t.text } else { t.fg });
        if self.heading.is_some() || self.bold > 0 {
            s = s.add_modifier(Modifier::BOLD);
        }
        if self.heading.is_some() {
            s = s.fg(t.strong);
        }
        if self.italic > 0 {
            s = s.add_modifier(Modifier::ITALIC);
        }
        if self.strike > 0 {
            s = s.add_modifier(Modifier::CROSSED_OUT);
        }
        if !self.links.is_empty() {
            s = s.fg(t.accent).add_modifier(Modifier::UNDERLINED);
        }
        if self.image {
            s = s.fg(t.muted).add_modifier(Modifier::ITALIC);
        }
        s
    }

    fn muted(&self) -> Style {
        Style::default().fg(self.t.muted)
    }

    /// Columns left for text after the quote bars and list indents.
    fn room(&self) -> usize {
        self.width.saturating_sub(2 * self.quotes + self.indents.iter().sum::<usize>()).max(1)
    }

    /// The quote bars and list indent a line starts with; an item's first line shows its marker.
    fn prefix(&mut self) -> Vec<Seg> {
        let mut segs: Vec<Seg> = (0..self.quotes).map(|_| seg("│ ", self.muted())).collect();
        let indent: usize = self.indents.iter().sum();
        match self.marker.take() {
            Some(marker) => {
                segs.push(seg(" ".repeat(indent.saturating_sub(marker.width())), Style::default()));
                segs.push(seg(marker, Style::default().fg(self.t.accent)));
            }
            None => segs.push(seg(" ".repeat(indent), Style::default())),
        }
        segs
    }

    fn emit(&mut self, body: Vec<Seg>) {
        let mut line = self.prefix();
        line.extend(body);
        self.out.push(line);
        self.gap = false;
    }

    /// The words read so far, wrapped to the room left.
    fn flush(&mut self) {
        if self.inline.is_empty() {
            return;
        }
        let words = std::mem::take(&mut self.inline);
        for line in wrap(&words, self.room()) {
            self.emit(line);
        }
    }

    /// One blank line between blocks, never two and never first.
    fn blank(&mut self) {
        if self.out.is_empty() || self.gap || self.marker.is_some() {
            return;
        }
        let bars = (0..self.quotes).map(|_| seg("│", self.muted())).collect();
        self.out.push(bars);
        self.gap = true;
    }

    fn rule(&mut self) {
        let rule = seg("─".repeat(self.room()), Style::default().fg(self.t.line));
        self.emit(vec![rule]);
    }

    /// Code keeps its lines: each is cut, not wrapped, on the raised ground.
    fn code_block(&mut self, code: &str) {
        let room = self.room();
        let ground = Style::default().fg(self.t.fg).bg(self.t.card);
        for line in code.trim_end_matches('\n').lines() {
            let text = format!(" {}", line.replace('\t', "    "));
            let mut body = cut(&[seg(text, ground)], room);
            let pad = room.saturating_sub(width(&body));
            body.push(seg(" ".repeat(pad), ground));
            self.emit(body);
        }
    }

    fn draw_table(&mut self, table: Table) {
        let columns = table.rows.iter().map(Vec::len).max().unwrap_or(0);
        if columns == 0 {
            return;
        }
        let natural: Vec<usize> =
            (0..columns).map(|c| table.rows.iter().filter_map(|r| r.get(c)).map(|cell| width(cell)).max().unwrap_or(0).max(1)).collect();
        let widths = column_widths(&natural, self.room());
        let line = Style::default().fg(self.t.line);
        for (i, row) in table.rows.iter().enumerate() {
            let mut body = Vec::new();
            for (c, w) in widths.iter().enumerate() {
                if c > 0 {
                    body.push(seg(COLUMN_GAP, line));
                }
                let align = table.aligns.get(c).copied().unwrap_or(Alignment::None);
                body.extend(align_cell(row.get(c).map(Vec::as_slice).unwrap_or(&[]), *w, align));
            }
            self.emit(body);
            if i + 1 == table.head_rows {
                let rule = widths.iter().map(|w| "─".repeat(*w)).collect::<Vec<_>>().join("─┼─");
                self.emit(vec![seg(rule, line)]);
            }
        }
    }

    fn finish(mut self) -> Vec<Vec<Seg>> {
        self.flush();
        if self.gap {
            self.out.pop();
        }
        self.out
    }
}

fn width(segs: &[Seg]) -> usize {
    segs.iter().map(|(s, _)| s.width()).sum()
}

/// The columns that fit in `room`, narrowed: the last ones are left out when even
/// `MIN_COLUMN` each is too wide, and a lone column is cut to the room.
fn column_widths(natural: &[usize], room: usize) -> Vec<usize> {
    let gaps = |n: usize| COLUMN_GAP.width() * n.saturating_sub(1);
    let mut keep = natural.len();
    loop {
        let mut widths = natural[..keep].to_vec();
        fit_columns(&mut widths, room.saturating_sub(gaps(keep)));
        if keep == 1 {
            widths[0] = widths[0].min(room);
            return widths;
        }
        if widths.iter().sum::<usize>() + gaps(keep) <= room {
            return widths;
        }
        keep -= 1;
    }
}

/// Narrow the widest columns until they fit `room`, none under `MIN_COLUMN`.
fn fit_columns(widths: &mut [usize], room: usize) {
    while widths.iter().sum::<usize>() > room {
        let Some(widest) = widths.iter_mut().filter(|w| **w > MIN_COLUMN).max_by_key(|w| **w) else { return };
        *widest -= 1;
    }
}

fn align_cell(cell: &[Seg], w: usize, align: Alignment) -> Vec<Seg> {
    let body = cut(cell, w);
    let pad = w.saturating_sub(width(&body));
    let (left, right) = match align {
        Alignment::Right => (pad, 0),
        Alignment::Center => (pad / 2, pad - pad / 2),
        Alignment::Left | Alignment::None => (0, pad),
    };
    let mut out = vec![seg(" ".repeat(left), Style::default())];
    out.extend(body);
    out.push(seg(" ".repeat(right), Style::default()));
    out
}

/// At most `w` columns of `segs`, ending in `…` when something was cut.
fn cut(segs: &[Seg], w: usize) -> Vec<Seg> {
    if width(segs) <= w {
        return segs.to_vec();
    }
    let mut out = Vec::new();
    let mut used = 0;
    let room = w.saturating_sub(1);
    for (text, style) in segs {
        let mut kept = String::new();
        for g in text.graphemes(true) {
            let gw = g.width();
            if used + gw > room {
                break;
            }
            used += gw;
            kept.push_str(g);
        }
        let full = kept.len() == text.len();
        out.push(seg(kept, *style));
        if !full {
            out.push(seg("…", *style));
            return out;
        }
    }
    out
}

/// Words wrapped to `w` columns; a word longer than a line is broken.
fn wrap(segs: &[Seg], w: usize) -> Vec<Vec<Seg>> {
    let mut lines: Vec<Vec<Seg>> = Vec::new();
    let mut line: Vec<Seg> = Vec::new();
    let mut used = 0;
    let mut space: Option<Seg> = None;
    for (text, style) in segs {
        for piece in pieces(text) {
            if piece.trim().is_empty() {
                if used > 0 {
                    space = Some(seg(" ", *style));
                }
                continue;
            }
            let pw = piece.width();
            let sw = space.as_ref().map_or(0, |(s, _)| s.width());
            if used > 0 && used + sw + pw > w {
                lines.push(std::mem::take(&mut line));
                used = 0;
                space = None;
            }
            if let Some((s, st)) = space.take() {
                used += s.width();
                push_str(&mut line, " ", st);
            }
            for g in piece.graphemes(true) {
                let gw = g.width();
                if used > 0 && used + gw > w {
                    lines.push(std::mem::take(&mut line));
                    used = 0;
                }
                push_str(&mut line, g, *style);
                used += gw;
            }
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Add `text` to the line, joining the last run when it has the same style.
fn push_str(line: &mut Vec<Seg>, text: &str, style: Style) {
    match line.last_mut() {
        Some((s, st)) if *st == style => s.push_str(text),
        _ => line.push(seg(text, style)),
    }
}

/// Runs of spaces and runs of everything else, in order.
fn pieces(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut last: Option<bool> = None;
    for (i, c) in text.char_indices() {
        let blank = c.is_whitespace();
        if last.is_some_and(|b| b != blank) {
            out.push(&text[start..i]);
            start = i;
        }
        last = Some(blank);
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> Theme {
        Theme::named("night")
    }

    fn text(lines: &[Vec<Seg>]) -> Vec<String> {
        lines.iter().map(|l| l.iter().map(|(s, _)| s.as_str()).collect::<String>().trim_end().to_string()).collect()
    }

    fn style_of(lines: &[Vec<Seg>], word: &str) -> Style {
        lines.iter().flatten().find(|(s, _)| s.contains(word)).map(|(_, st)| *st).unwrap_or_else(|| panic!("{word} is drawn"))
    }

    #[test]
    fn a_first_level_heading_is_bold_and_strong_with_a_rule_under_it() {
        let t = theme();
        let lines = render("# Title\n\n## Part\n\ntext", 20, &t);
        assert_eq!(text(&lines), ["Title", "─".repeat(20).as_str(), "", "Part", "", "text"]);
        let s = style_of(&lines, "Title");
        assert_eq!(s.fg, Some(t.strong));
        assert!(s.add_modifier.contains(Modifier::BOLD));
        assert!(style_of(&lines, "Part").add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn emphasis_strong_and_inline_code_keep_their_styles() {
        let t = theme();
        let lines = render("an *odd* and **bold** `code`", 40, &t);
        assert_eq!(text(&lines), ["an odd and bold code"]);
        assert!(style_of(&lines, "odd").add_modifier.contains(Modifier::ITALIC));
        assert!(style_of(&lines, "bold").add_modifier.contains(Modifier::BOLD));
        assert_eq!(style_of(&lines, "code").bg, Some(t.card2));
    }

    #[test]
    fn fenced_code_sits_on_the_raised_ground_across_the_width_and_is_cut_not_wrapped() {
        let t = theme();
        let lines = render("```rust\nfn main() {}\nlet a_very_long_name_here = 1;\n```", 16, &t);
        let shown = text(&lines);
        assert_eq!(shown[0], " fn main() {}");
        assert!(shown[1].ends_with('…') && shown[1].width() <= 16, "{:?}", shown[1]);
        assert_eq!(lines.len(), 2, "one line per source line");
        for l in &lines {
            assert_eq!(width(l), 16, "the ground reaches the edge");
            assert!(l.iter().filter(|(s, _)| !s.is_empty()).all(|(_, st)| st.bg == Some(t.card)));
        }
    }

    #[test]
    fn lists_show_markers_and_indent_their_nested_items_and_wrapped_lines() {
        let lines = render("- one\n- two words here\n  - inner\n\n1. first\n2. second", 12, &theme());
        assert_eq!(text(&lines), ["• one", "• two words", "  here", "  • inner", "", "1. first", "2. second"]);
    }

    #[test]
    fn a_block_quote_sits_behind_a_muted_bar() {
        let t = theme();
        let lines = render("> quoted words\n\nafter", 40, &t);
        assert_eq!(text(&lines), ["│ quoted words", "", "after"]);
        assert_eq!(lines[0][0].1.fg, Some(t.muted));
    }

    #[test]
    fn tables_align_their_columns_and_are_cut_to_the_width() {
        let md = "| name | n |\n|:--|--:|\n| alpha | 1 |\n| b | 200 |";
        assert_eq!(text(&render(md, 40, &theme())), ["name  │   n", "──────┼────", "alpha │   1", "b     │ 200"]);
        let narrow = text(&render("| a long heading | other long one |\n|--|--|\n| x | y |", 20, &theme()));
        assert!(narrow.iter().all(|l| l.width() <= 20), "{narrow:?}");
        assert!(narrow[0].contains('…'), "{narrow:?}");
    }

    #[test]
    fn a_link_shows_its_text_with_the_url_muted_after_it() {
        let t = theme();
        let lines = render("see [the docs](https://x.dev) or <https://y.dev>", 60, &t);
        assert_eq!(text(&lines), ["see the docs (https://x.dev) or https://y.dev"]);
        assert_eq!(style_of(&lines, "(https://x.dev)").fg, Some(t.muted));
        assert_eq!(style_of(&lines, "docs").fg, Some(t.accent));
    }

    #[test]
    fn a_horizontal_rule_spans_the_width() {
        assert_eq!(text(&render("a\n\n---\n\nb", 8, &theme())), ["a", "", "────────", "", "b"]);
    }

    #[test]
    fn long_prose_wraps_at_words_and_breaks_a_word_too_long_for_a_line() {
        let lines = text(&render("one two three abcdefghijkl", 8, &theme()));
        assert_eq!(lines, ["one two", "three", "abcdefgh", "ijkl"]);
    }

    #[test]
    fn an_item_that_opens_with_a_list_keeps_its_own_marker() {
        assert_eq!(text(&render("- - inner\n- outer", 40, &theme())), ["• • inner", "• outer"]);
    }

    #[test]
    fn an_html_block_is_cut_to_the_width() {
        let lines = text(&render("<div>some long html that runs on</div>", 12, &theme()));
        assert!(lines.iter().all(|l| l.width() <= 12) && lines[0].ends_with('…'), "{lines:?}");
    }

    #[test]
    fn a_table_too_wide_for_its_columns_leaves_the_last_ones_out() {
        let md = "| a | b | c | d | e | f |\n|--|--|--|--|--|--|\n| 1 | 2 | 3 | 4 | 5 | 6 |";
        let lines = text(&render(md, 20, &theme()));
        assert!(lines.iter().all(|l| l.width() <= 20), "{lines:?}");
        assert!(lines[0].starts_with("a │ b") && lines[2].starts_with("1 │ 2"), "{lines:?}");
    }

    #[test]
    fn joined_emoji_are_never_split_when_wrapping_or_cutting() {
        const FAMILY: &str = "\u{1F469}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let wrapped = text(&render(&FAMILY.repeat(3), 4, &theme()));
        assert!(wrapped.len() > 1 && wrapped.iter().all(|l| l.replace(FAMILY, "").is_empty()), "{wrapped:?}");
        let code = text(&render(&format!("```\n{}\n```", FAMILY.repeat(3)), 5, &theme()));
        assert_eq!(code[0], format!(" {FAMILY}…"));
    }

    #[test]
    fn markdown_files_are_told_by_their_extension() {
        assert!(is_markdown(Path::new("README.md")) && is_markdown(Path::new("notes.MARKDOWN")));
        assert!(!is_markdown(Path::new("main.rs")) && !is_markdown(Path::new("md")));
    }
}
