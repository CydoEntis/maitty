//! Copy mode: a frozen, scrollable view of a pane's whole history with vim-style motion,
//! selection and search. Entering takes a snapshot; output keeps flowing underneath.

use crate::protocol::TermId;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthChar;

/// How long a copied selection stays accent-tinted before the pane is plain again.
pub const FLASH_FOR: Duration = Duration::from_millis(700);

/// A selection that was just copied, kept as cells of the pane so it can glow for a moment
/// after copy mode has closed.
#[derive(Debug, Clone)]
pub struct Flash {
    pub term: TermId,
    /// (row, first column, one past the last column), relative to the pane's text area.
    pub spans: Vec<(u16, u16, u16)>,
    pub at: Instant,
}

impl Flash {
    pub fn live(&self) -> bool {
        self.at.elapsed() < FLASH_FOR
    }

    /// Tint the copied cells in `buf`; `pane` is where the pane's text is drawn.
    pub fn tint(&self, buf: &mut Buffer, pane: Rect, t: &crate::theme::Theme) {
        let bg = crate::client::render::blend(t.bg, t.accent, 0.34);
        for &(row, x0, x1) in &self.spans {
            for x in x0..x1 {
                if let Some(px) = buf.cell_mut((pane.x + x, pane.y + row)) {
                    px.set_bg(bg).set_fg(t.strong);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Copy {
    pub term: TermId,
    pub lines: Vec<String>,
    /// First line shown.
    pub top: usize,
    /// Visible rows / columns, kept current by the renderer.
    pub height: usize,
    pub width: usize,
    /// Cursor: (line, char index).
    pub cur: (usize, usize),
    pub anchor: Option<(usize, usize)>,
    pub line_mode: bool,
    pub query: Option<String>,
    /// A search being typed, and its direction.
    pub input: Option<String>,
    pub backward: bool,
    /// Entered by dragging the mouse: copy and leave when the button is released.
    pub mouse: bool,
    pub message: Option<String>,
}

/// Pieces copied from several panes, one after another under each pane's name.
pub fn join_pieces(pieces: &[(String, String)]) -> String {
    pieces.iter().map(|(name, text)| format!("── {name} ──\n{}", text.trim_end())).collect::<Vec<_>>().join("\n\n")
}

pub enum Outcome {
    Stay,
    Exit,
    Yank(String),
}

/// Every line of history plus the screen, oldest first. Leaves the parser's scroll as it was.
fn all_lines(parser: &mut vt100::Parser) -> Vec<String> {
    let screen = parser.screen_mut();
    let (rows, cols) = screen.size();
    let rows = rows as usize;
    let original = screen.scrollback();
    screen.set_scrollback(usize::MAX);
    let total = screen.scrollback();
    let mut lines: Vec<String> = Vec::with_capacity(total + rows);
    let mut top = 0;
    loop {
        // With offset `o`, the view starts at absolute line `total - o`.
        let offset = total.saturating_sub(top);
        screen.set_scrollback(offset);
        let view_top = total - offset;
        for (i, row) in screen.rows(0, cols).enumerate() {
            if view_top + i == lines.len() {
                lines.push(row.trim_end().to_string());
            }
        }
        if offset == 0 {
            break;
        }
        top += rows;
    }
    screen.set_scrollback(original);
    lines
}

impl Copy {
    /// Snapshot `parser`, showing what the pane shows now (honouring its scroll offset).
    pub fn new(term: TermId, parser: &mut vt100::Parser) -> Copy {
        let (rows, cols) = parser.screen().size();
        let scroll = parser.screen().scrollback();
        let (crow, ccol) = parser.screen().cursor_position();
        let lines = all_lines(parser);
        let rows = rows as usize;
        let top = lines.len().saturating_sub(rows + scroll);
        let cur = if scroll == 0 { (top + crow as usize, ccol as usize) } else { (top, 0) };
        let mut c = Copy {
            term,
            lines,
            top,
            height: rows,
            width: cols as usize,
            cur,
            anchor: None,
            line_mode: false,
            query: None,
            input: None,
            backward: false,
            mouse: false,
            message: None,
        };
        c.clamp_cursor();
        c
    }

    fn last_line(&self) -> usize {
        self.lines.len().saturating_sub(1)
    }

    fn line_len(&self, l: usize) -> usize {
        self.lines.get(l).map(|s| s.chars().count()).unwrap_or(0)
    }

    fn clamp_cursor(&mut self) {
        self.cur.0 = self.cur.0.min(self.last_line());
        self.cur.1 = self.cur.1.min(self.line_len(self.cur.0));
        self.follow();
    }

    /// Scroll so the cursor is visible.
    fn follow(&mut self) {
        let h = self.height.max(1);
        if self.cur.0 < self.top {
            self.top = self.cur.0;
        } else if self.cur.0 >= self.top + h {
            self.top = self.cur.0 + 1 - h;
        }
        self.top = self.top.min(self.lines.len().saturating_sub(h));
    }

    pub fn scroll(&mut self, delta: isize) {
        let h = self.height.max(1);
        let max_top = self.lines.len().saturating_sub(h);
        self.top = (self.top as isize + delta).clamp(0, max_top as isize) as usize;
        self.cur.0 = self.cur.0.min((self.top + h).saturating_sub(1)).max(self.top).min(self.last_line());
        self.cur.1 = self.cur.1.min(self.line_len(self.cur.0));
    }

    /// Map a cell in the view to (line, char index).
    pub fn at_cell(&self, row: u16, col: u16) -> (usize, usize) {
        let line = (self.top + row as usize).min(self.last_line());
        let mut x = 0usize;
        let text = self.lines.get(line).map(String::as_str).unwrap_or("");
        for (i, ch) in text.chars().enumerate() {
            let w = ch.width().unwrap_or(0);
            if x + w > col as usize {
                return (line, i);
            }
            x += w;
        }
        (line, text.chars().count())
    }

    pub fn move_to(&mut self, pos: (usize, usize)) {
        self.cur = pos;
        self.follow();
    }

    /// Normalised selection: (start, end) inclusive of `end`'s character.
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.anchor?;
        let (s, e) = if a <= self.cur { (a, self.cur) } else { (self.cur, a) };
        if self.line_mode { Some(((s.0, 0), (e.0, usize::MAX))) } else { Some((s, e)) }
    }

    pub fn selected(&self, line: usize, col: usize) -> bool {
        self.selection().is_some_and(|(s, e)| (line, col) >= s && (line, col) <= e)
    }

    /// The selected cells that are on screen, as (row, first column, one past the last
    /// column) in the pane's text area. A selected line also covers the blank cell past its
    /// text, like the renderer draws it.
    pub fn selected_spans(&self) -> Vec<(u16, u16, u16)> {
        let mut out = Vec::new();
        if self.anchor.is_none() {
            return out;
        }
        for row in 0..self.height {
            let li = self.top + row;
            let Some(line) = self.lines.get(li) else { break };
            let mut span: Option<(u16, u16)> = None;
            let mut x = 0u16;
            let mut n = 0;
            for (ci, ch) in line.chars().enumerate() {
                n = ci + 1;
                let w = ch.width().unwrap_or(0) as u16;
                if w == 0 {
                    continue;
                }
                if x as usize + w as usize > self.width {
                    break;
                }
                if self.selected(li, ci) {
                    span = Some((span.map_or(x, |s| s.0), x + w));
                } else if let Some((a, b)) = span.take() {
                    out.push((row as u16, a, b));
                }
                x += w;
            }
            if (x as usize) < self.width && self.selected(li, n) {
                span = Some((span.map_or(x, |s| s.0), x + 1));
            }
            if let Some((a, b)) = span {
                out.push((row as u16, a, b));
            }
        }
        out
    }

    pub fn selected_text(&self) -> String {
        let Some((s, e)) = self.selection() else { return String::new() };
        let mut out = Vec::new();
        for l in s.0..=e.0.min(self.last_line()) {
            let chars: Vec<char> = self.lines[l].chars().collect();
            let from = if l == s.0 { s.1.min(chars.len()) } else { 0 };
            let to = if l == e.0 { e.1.saturating_add(1).min(chars.len()) } else { chars.len() };
            out.push(chars[from..to.max(from)].iter().collect::<String>());
        }
        out.join("\n")
    }

    fn insensitive(q: &str) -> bool {
        !q.chars().any(char::is_uppercase)
    }

    /// Char ranges of the current query in a line.
    pub fn matches(&self, line: usize) -> Vec<(usize, usize)> {
        let Some(q) = self.query.as_deref().filter(|q| !q.is_empty()) else { return Vec::new() };
        let Some(text) = self.lines.get(line) else { return Vec::new() };
        find_all(text, q, Self::insensitive(q))
    }

    /// Jump to the next match of the current search, if there is one.
    pub fn search_next(&mut self) {
        self.search(false);
    }

    /// Jump to the next match in the current direction (reversed by `flip`). Wraps around.
    fn search(&mut self, flip: bool) {
        let Some(q) = self.query.clone().filter(|q| !q.is_empty()) else { return };
        let back = self.backward ^ flip;
        let ci = Self::insensitive(&q);
        let n = self.lines.len();
        for step in 0..=n {
            let l = if back { (self.cur.0 + n - step % n) % n } else { (self.cur.0 + step) % n };
            let hits = find_all(&self.lines[l], &q, ci);
            let hit = if back {
                hits.iter().rev().find(|(s, _)| step > 0 || *s < self.cur.1)
            } else {
                hits.iter().find(|(s, _)| step > 0 || *s > self.cur.1)
            };
            if let Some((s, _)) = hit {
                if step == n && (l, *s) == self.cur {
                    break;
                }
                self.move_to((l, *s));
                self.message = None;
                return;
            }
        }
        self.message = Some(format!("not found: {q}"));
    }

    fn word_forward(&mut self) {
        let (mut l, mut c) = self.cur;
        loop {
            let chars: Vec<char> = self.lines.get(l).map(|s| s.chars().collect()).unwrap_or_default();
            // Skip the rest of the current word, then whitespace.
            while c < chars.len() && !chars[c].is_whitespace() {
                c += 1;
            }
            while c < chars.len() && chars[c].is_whitespace() {
                c += 1;
            }
            if c < chars.len() || l >= self.last_line() {
                self.move_to((l, c.min(chars.len().saturating_sub(1))));
                return;
            }
            l += 1;
            c = 0;
            if self.lines[l].chars().next().is_some_and(|ch| !ch.is_whitespace()) {
                self.move_to((l, 0));
                return;
            }
        }
    }

    fn word_backward(&mut self) {
        let (mut l, mut c) = self.cur;
        loop {
            let chars: Vec<char> = self.lines.get(l).map(|s| s.chars().collect()).unwrap_or_default();
            while c > 0 && chars.get(c - 1).is_some_and(|ch| ch.is_whitespace()) {
                c -= 1;
            }
            if c > 0 {
                while c > 0 && chars.get(c - 1).is_some_and(|ch| !ch.is_whitespace()) {
                    c -= 1;
                }
                self.move_to((l, c));
                return;
            }
            if l == 0 {
                self.move_to((0, 0));
                return;
            }
            l -= 1;
            c = self.line_len(l);
        }
    }

    pub fn key(&mut self, k: &KeyEvent) -> Outcome {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if let Some(input) = &mut self.input {
            match k.code {
                KeyCode::Esc => self.input = None,
                KeyCode::Enter => {
                    let q = std::mem::take(input);
                    self.input = None;
                    if !q.is_empty() {
                        self.query = Some(q);
                    }
                    self.search(false);
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) if !ctrl => input.push(c),
                _ => {}
            }
            return Outcome::Stay;
        }
        self.message = None;
        let h = self.height.max(1);
        let (l, c) = self.cur;
        match k.code {
            KeyCode::Char('c') if ctrl => return Outcome::Exit,
            KeyCode::Esc if self.anchor.is_some() => self.anchor = None,
            KeyCode::Esc | KeyCode::Char('q') => return Outcome::Exit,
            KeyCode::Char('u') if ctrl => self.scroll(-(h as isize / 2)),
            KeyCode::Char('d') if ctrl => self.scroll(h as isize / 2),
            KeyCode::Char('b') if ctrl => self.scroll(-(h as isize)),
            KeyCode::Char('f') if ctrl => self.scroll(h as isize),
            KeyCode::Char('e') if ctrl => self.scroll(1),
            KeyCode::Char('y') if ctrl => self.scroll(-1),
            KeyCode::PageUp => self.scroll(-(h as isize)),
            KeyCode::PageDown => self.scroll(h as isize),
            KeyCode::Char('h') | KeyCode::Left => self.move_to((l, c.saturating_sub(1))),
            KeyCode::Char('l') | KeyCode::Right => self.move_to((l, (c + 1).min(self.line_len(l)))),
            KeyCode::Char('k') | KeyCode::Up => self.move_to((l.saturating_sub(1), c.min(self.line_len(l.saturating_sub(1))))),
            KeyCode::Char('j') | KeyCode::Down => {
                let n = (l + 1).min(self.last_line());
                self.move_to((n, c.min(self.line_len(n))));
            }
            KeyCode::Char('w') => self.word_forward(),
            KeyCode::Char('b') => self.word_backward(),
            KeyCode::Char('0') | KeyCode::Home => self.move_to((l, 0)),
            KeyCode::Char('^') => {
                let first = self.lines[l].chars().position(|ch| !ch.is_whitespace()).unwrap_or(0);
                self.move_to((l, first));
            }
            KeyCode::Char('$') | KeyCode::End => self.move_to((l, self.line_len(l).saturating_sub(1))),
            KeyCode::Char('g') => self.move_to((0, 0)),
            KeyCode::Char('G') => self.move_to((self.last_line(), 0)),
            KeyCode::Char('H') => self.move_to((self.top, 0)),
            KeyCode::Char('M') => self.move_to(((self.top + h / 2).min(self.last_line()), 0)),
            KeyCode::Char('L') => self.move_to(((self.top + h - 1).min(self.last_line()), 0)),
            KeyCode::Char('v') | KeyCode::Char(' ') => {
                if self.anchor.is_some() && !self.line_mode {
                    self.anchor = None;
                } else {
                    self.anchor = Some(self.cur);
                    self.line_mode = false;
                }
            }
            KeyCode::Char('V') => {
                if self.anchor.is_some() && self.line_mode {
                    self.anchor = None;
                } else {
                    self.anchor = Some(self.cur);
                    self.line_mode = true;
                }
            }
            KeyCode::Char('y') | KeyCode::Enter if self.anchor.is_some() => return Outcome::Yank(self.selected_text()),
            KeyCode::Char('y') | KeyCode::Char('Y') => return Outcome::Yank(self.lines[l].clone()),
            KeyCode::Enter => return Outcome::Exit,
            KeyCode::Char('/') => {
                self.input = Some(String::new());
                self.backward = false;
            }
            KeyCode::Char('?') => {
                self.input = Some(String::new());
                self.backward = true;
            }
            KeyCode::Char('n') => self.search(false),
            KeyCode::Char('N') => self.search(true),
            _ => {}
        }
        Outcome::Stay
    }
}

/// Non-overlapping occurrences of `q` in `text` as char ranges.
fn find_all(text: &str, q: &str, insensitive: bool) -> Vec<(usize, usize)> {
    let (hay, needle) = if insensitive { (text.to_lowercase(), q.to_lowercase()) } else { (text.to_string(), q.to_string()) };
    // Lowercasing can change byte lengths; fall back to exact matching if it did.
    let (hay, needle) = if hay.len() == text.len() { (hay, needle) } else { (text.to_string(), q.to_string()) };
    let qlen = needle.chars().count();
    hay.match_indices(&needle)
        .map(|(b, _)| {
            let start = hay[..b].chars().count();
            (start, start + qlen)
        })
        .collect()
}

#[cfg(test)]
thread_local! {
    /// What the last copy on this thread would have put on the clipboard. Tests read it here
    /// instead of the machine's clipboard, which they must never overwrite.
    pub static LAST_COPY: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// What reads the system clipboard's text, to run off the UI thread: on X11 the program
/// that owns the clipboard has to answer, and may be slow to. Under test it reads what the
/// last copy on this thread recorded (`LAST_COPY`), never the machine's clipboard.
#[cfg(test)]
pub fn clipboard_reader() -> impl FnOnce() -> Result<String, String> + Send + 'static {
    let last = LAST_COPY.with(|c| c.borrow().clone());
    move || last.ok_or_else(|| NO_TEXT.to_string())
}

#[cfg(not(test))]
pub fn clipboard_reader() -> impl FnOnce() -> Result<String, String> + Send + 'static {
    || {
        arboard::Clipboard::new().and_then(|mut c| c.get_text()).map_err(|e| match e {
            arboard::Error::ContentNotAvailable => NO_TEXT.to_string(),
            e => format!("can't read the clipboard: {e}"),
        })
    }
}

pub const NO_TEXT: &str = "there's no text on the clipboard";

/// Write `text` to the system clipboard, and to the outer terminal with OSC 52 so it also
/// works over SSH. Under test it is only recorded (`LAST_COPY`).
#[cfg(test)]
pub fn to_clipboard(text: &str) -> bool {
    LAST_COPY.with(|c| *c.borrow_mut() = Some(text.to_string()));
    true
}

/// Write `text` to the system clipboard, and to the outer terminal with OSC 52 so it also
/// works over SSH.
#[cfg(not(test))]
pub fn to_clipboard(text: &str) -> bool {
    use std::io::Write;
    let native = arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string())).is_ok();
    let osc = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
    let mut out = std::io::stdout();
    let _ = out.write_all(osc.as_bytes());
    let _ = out.flush();
    native
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy_of(text: &str) -> Copy {
        let mut p = vt100::Parser::new(4, 20, 100);
        p.process(text.replace('\n', "\r\n").as_bytes());
        let mut c = Copy::new(1, &mut p);
        c.height = 4;
        c
    }

    #[test]
    fn captures_history_beyond_screen() {
        let c = copy_of("one\ntwo\nthree\nfour\nfive\nsix");
        assert_eq!(&c.lines[..6], &["one", "two", "three", "four", "five", "six"]);
    }

    #[test]
    fn selected_spans_are_the_cells_on_screen() {
        let mut c = copy_of("alpha beta\ngamma\ndelta");
        assert!(c.selected_spans().is_empty(), "nothing is selected before an anchor");
        let first = c.lines.iter().position(|l| l == "alpha beta").unwrap();
        c.anchor = Some((first, 6));
        c.cur = (first + 1, 2);
        let rows = (first - c.top) as u16;
        assert_eq!(c.selected_spans(), vec![(rows, 6, 10 + 1), (rows + 1, 0, 3)], "the first line from the anchor (and the blank cell past it), the last up to the cursor");
    }

    #[test]
    fn search_and_select() {
        let mut c = copy_of("alpha beta\ngamma Beta\ndelta");
        c.cur = (2, 0);
        c.query = Some("beta".into());
        c.backward = true;
        c.search(false);
        assert_eq!(c.cur, (1, 6));
        c.search(false);
        assert_eq!(c.cur, (0, 6));
        c.anchor = Some((0, 6));
        c.cur = (1, 4);
        assert_eq!(c.selected_text(), "beta\ngamma");
        c.line_mode = true;
        assert_eq!(c.selected_text(), "alpha beta\ngamma Beta");
    }

    #[test]
    fn base64_matches_reference() {
        assert_eq!(base64(b"hello"), "aGVsbG8=");
        assert_eq!(base64(b"hi!"), "aGkh");
        assert_eq!(base64(b"a"), "YQ==");
    }
}
