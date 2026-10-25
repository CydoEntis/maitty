//! State for the app's own views that replace the pane area: Changes (a worktree's
//! git review) and Files (the worktree's file tree with a preview).

use super::files::FileEntry;
use crate::brand::{APP_COMMAND};
use super::tasks::Review;
use crate::protocol::{TermId, WsId};
use crate::theme::Theme;
use ratatui::style::{Color, Modifier, Style};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ---- Changes ---------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ChangesView {
    pub dir: PathBuf,
    pub review: Option<Review>,
    pub error: Option<String>,
    /// The agent working in this worktree, and what it last said.
    pub agent: String,
    pub said: String,
    pub term: Option<TermId>,
    pub ws: Option<WsId>,
    /// A linked worktree (can be merged or discarded), not the main checkout.
    pub linked: bool,
    pub confirm: Option<(String, char)>,
    /// Files you've marked reviewed (and that haven't changed since).
    pub reviewed: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangesRow {
    /// Folder name, depth.
    Dir(String, usize),
    /// Index into `review.files`, depth.
    File(usize, usize),
    /// The reviewed files start here (how many).
    Reviewed(usize),
}

/// What a file is like right now, so a review mark clears when it changes.
pub fn fingerprint(dir: &Path, file: &str) -> u64 {
    let bytes = std::fs::read(dir.join(file)).unwrap_or_else(|_| b"(deleted)".to_vec());
    // FNV-1a: the same everywhere and across versions.
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// Which of `files` are still as they were when marked.
pub fn still_reviewed(dir: &Path, files: &[super::tasks::Changed], marks: Option<&HashMap<String, u64>>) -> HashSet<String> {
    let Some(marks) = marks else { return HashSet::new() };
    files.iter().filter(|f| marks.get(&f.path).is_some_and(|m| *m == fingerprint(dir, &f.path))).map(|f| f.path.clone()).collect()
}

impl ChangesView {
    /// The files in the order they're shown (Up/Down follow it).
    pub fn order(&self) -> Vec<usize> {
        self.rows().into_iter().filter_map(|r| if let ChangesRow::File(i, _) = r { Some(i) } else { None }).collect()
    }

    /// The changed files as a tree: folders, then their files; the ones you've reviewed
    /// sink to the bottom.
    pub fn rows(&self) -> Vec<ChangesRow> {
        let Some(r) = &self.review else { return Vec::new() };
        let (done, todo): (Vec<usize>, Vec<usize>) = (0..r.files.len()).partition(|i| self.reviewed.contains(&r.files[*i].path));
        let mut rows = self.tree(todo);
        if !done.is_empty() {
            rows.push(ChangesRow::Reviewed(done.len()));
            rows.extend(self.tree(done));
        }
        rows
    }

    fn tree(&self, mut idx: Vec<usize>) -> Vec<ChangesRow> {
        let Some(r) = &self.review else { return Vec::new() };
        idx.sort_by(|a, b| r.files[*a].path.cmp(&r.files[*b].path));
        let mut rows = Vec::new();
        let mut open: Vec<String> = Vec::new();
        for i in idx {
            let parts: Vec<&str> = r.files[i].path.split('/').collect();
            let dirs = &parts[..parts.len().saturating_sub(1)];
            let common = open.iter().zip(dirs.iter()).take_while(|(a, b)| a == *b).count();
            open.truncate(common);
            for (d, name) in dirs.iter().enumerate().skip(common) {
                rows.push(ChangesRow::Dir(name.to_string(), d));
                open.push(name.to_string());
            }
            rows.push(ChangesRow::File(i, dirs.len()));
        }
        rows
    }
}

/// Word-wrap to `width` columns.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.lines() {
        let mut cur = String::new();
        for word in para.split_whitespace() {
            if !cur.is_empty() && cur.chars().count() + word.chars().count() + 1 > width.max(8) {
                out.push(std::mem::take(&mut cur));
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
        }
        out.push(cur);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out
}

// ---- Files -----------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct FileNode {
    pub path: PathBuf,
    /// Relative to the root, with `/` separators (matches git's paths).
    pub rel: String,
    pub label: String,
    pub depth: usize,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilesTree {
    pub root: PathBuf,
    pub branch: Option<String>,
    pub all: Vec<FileNode>,
    pub expanded: HashSet<PathBuf>,
    pub sel: usize,
    pub filter: String,
    pub filtering: bool,
    /// rel path -> 'M' (changed) or 'A' (new)
    pub git: HashMap<String, char>,
    /// rel path -> the agent changing it
    pub editing: HashMap<String, String>,
    pub preview: Option<(PathBuf, Vec<String>)>,
    /// Showing recent files (Downloads, Desktop, …) instead of the tree.
    pub recent: bool,
    pub recent_list: Option<Vec<FileEntry>>,
    pub loading: bool,
    /// How far the preview is scrolled (lines).
    pub scroll: usize,
    /// Editing the previewed file right here.
    pub edit: Option<Edit>,
    /// The arrows are in the preview (→ on a file), not the tree.
    pub in_preview: bool,
    /// Once the tree is read: select this file (and scroll to this line).
    pub reveal: Option<(PathBuf, Option<u32>)>,
    /// The markdown file shown as its text rather than drawn (`m`).
    pub raw: Option<PathBuf>,
    /// The drawn markdown, kept until the file, the width or the theme changes.
    pub(super) drawn_cache: Option<DrawnCache>,
    /// The left button went down in the preview and is still held: moving selects.
    pub dragging: bool,
    /// The last press in the preview, to tell a double-click.
    pub last_press: Option<(Pos, std::time::Instant)>,
}

/// Markdown drawn for the preview, and what it was drawn for.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DrawnCache {
    path: PathBuf,
    width: u16,
    theme: String,
    pub(super) lines: Vec<Vec<super::design::Seg>>,
}

/// A small editor for the file in the preview.
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub path: PathBuf,
    pub lines: Vec<String>,
    /// Cursor: line, and character in it.
    pub row: usize,
    pub col: usize,
    pub dirty: bool,
    /// Esc once with unsaved changes: the next Esc throws them away.
    pub warned: bool,
    crlf: bool,
    trailing_newline: bool,
    /// The file as it was when opened (or last saved), to notice someone else changing it.
    seen: u64,
    /// Ctrl+S found it changed on disk: the next Ctrl+S overwrites anyway.
    pub conflict: bool,
    /// Where the selection started; it runs to the cursor.
    pub anchor: Option<Pos>,
}

/// Columns the preview's line numbers take (`{:>4}` and two spaces).
pub const GUTTER: u16 = 6;

/// A place in some text: line, and character in it.
pub type Pos = (usize, usize);

/// A selection's two ends in reading order; None when it is empty.
pub fn ordered(a: Pos, b: Pos) -> Option<(Pos, Pos)> {
    (a != b).then(|| if a < b { (a, b) } else { (b, a) })
}

/// The characters of `line` (which has `len`) inside `sel`. A selection that runs on past the
/// line's end takes one more, so the line break it includes shows.
pub fn line_span(sel: (Pos, Pos), line: usize, len: usize) -> Option<(usize, usize)> {
    let ((r0, c0), (r1, c1)) = sel;
    if line < r0 || line > r1 {
        return None;
    }
    let from = if line == r0 { c0.min(len) } else { 0 };
    let to = if line == r1 { c1.min(len) } else { len + 1 };
    (to > from).then_some((from, to))
}

/// The text of `lines` between the two ends of `sel`, lines joined by `\n`.
pub fn text_between<S: AsRef<str>>(lines: &[S], sel: (Pos, Pos)) -> String {
    let ((r0, c0), (r1, c1)) = sel;
    let mut out = String::new();
    for (i, l) in lines.iter().enumerate().take(r1 + 1).skip(r0) {
        let chars = l.as_ref().chars();
        let from = if i == r0 { c0 } else { 0 };
        let piece: String = if i == r1 { chars.take(c1).skip(from).collect() } else { chars.skip(from).collect() };
        out.push_str(&piece);
        if i < r1 {
            out.push('\n');
        }
    }
    out
}

/// Screen columns a character takes in the preview (tabs are four wide).
fn cell_width(c: char) -> usize {
    if c == '\t' { 4 } else { 1 }
}

/// The character of `line` drawn at screen column `x` (past the end: the end).
pub fn col_at(line: &str, x: usize) -> usize {
    let mut w = 0;
    for (i, c) in line.chars().enumerate() {
        w += cell_width(c);
        if w > x {
            return i;
        }
    }
    line.chars().count()
}

/// Screen columns the first `col` characters of `line` take.
pub fn x_of(line: &str, col: usize) -> usize {
    line.chars().take(col).map(cell_width).sum()
}

/// The word (letters, digits, `_`) around character `col` of `line`, or that one character.
pub fn word_at(line: &str, col: usize) -> (usize, usize) {
    let chars: Vec<char> = line.chars().collect();
    let word = |c: &char| c.is_alphanumeric() || *c == '_';
    if !chars.get(col).is_some_and(word) {
        return (col.min(chars.len()), (col + 1).min(chars.len()));
    }
    let start = chars[..col].iter().rposition(|c| !word(c)).map_or(0, |i| i + 1);
    let end = chars[col..].iter().position(|c| !word(c)).map_or(chars.len(), |i| col + i);
    (start, end)
}

/// A fingerprint of a file's bytes (same run only).
fn fingerprint_bytes(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

/// What saving did.
#[derive(Debug, PartialEq)]
pub enum Saved {
    Done,
    /// The file changed on disk since it was opened; nothing was written.
    ChangedOnDisk,
}

impl Edit {
    pub fn open(path: &Path) -> Result<Edit, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("can't read it: {e}"))?;
        if bytes.len() > 4 * 1024 * 1024 || bytes.contains(&0) {
            return Err(concat!("not a text file ", env!("APP_COMMAND"), " can edit (too big or binary); press e for your editor").into());
        }
        let seen = fingerprint_bytes(&bytes);
        let text = String::from_utf8(bytes).map_err(|_| "not UTF-8 text; press e for your editor".to_string())?;
        // The file's usual line ending: one stray CRLF doesn't turn every line into CRLF.
        let crlf_lines = text.matches("\r\n").count();
        let crlf = crlf_lines * 2 > text.matches('\n').count();
        let trailing_newline = text.ends_with('\n');
        let mut lines: Vec<String> = text.lines().map(|l| l.trim_end_matches('\r').to_string()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Ok(Edit { path: path.to_path_buf(), lines, row: 0, col: 0, dirty: false, warned: false, crlf, trailing_newline, seen, conflict: false, anchor: None })
    }

    /// Save, unless the file changed on disk since it was opened (an agent edited it): then
    /// nothing is written and `conflict` is set, so saving again overwrites on purpose. The
    /// write goes to a temporary file that replaces the real one, so a crash can't leave
    /// it half written.
    pub fn save(&mut self) -> Result<Saved, String> {
        if !self.conflict
            && let Ok(now) = std::fs::read(&self.path)
            && fingerprint_bytes(&now) != self.seen
        {
            self.conflict = true;
            return Ok(Saved::ChangedOnDisk);
        }
        let nl = if self.crlf { "\r\n" } else { "\n" };
        let mut text = self.lines.join(nl);
        if self.trailing_newline {
            text.push_str(nl);
        }
        let name = self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let tmp = self.path.with_file_name(format!(".{name}.{APP_COMMAND}-save"));
        std::fs::write(&tmp, &text).map_err(|e| format!("couldn't save: {e}"))?;
        if let Err(e) = std::fs::rename(&tmp, &self.path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("couldn't save: {e}"));
        }
        self.seen = fingerprint_bytes(text.as_bytes());
        self.conflict = false;
        self.dirty = false;
        self.warned = false;
        Ok(Saved::Done)
    }

    fn len(&self, row: usize) -> usize {
        self.lines.get(row).map(|l| l.chars().count()).unwrap_or(0)
    }

    fn byte(&self, row: usize, col: usize) -> usize {
        self.lines[row].char_indices().nth(col).map(|(i, _)| i).unwrap_or(self.lines[row].len())
    }

    /// The selection's ends in reading order, if any text is selected.
    pub fn selection(&self) -> Option<(Pos, Pos)> {
        self.anchor.and_then(|a| ordered(a, (self.row, self.col)))
    }

    /// What Ctrl+C copies: the selection, or the cursor's line with its line break.
    pub fn copied(&self) -> String {
        match self.selection() {
            Some(sel) => text_between(&self.lines, sel),
            None => format!("{}\n", self.lines[self.row]),
        }
    }

    /// Ctrl+X: what `copied` gives, removed: the selection, or the cursor's whole line.
    pub fn cut(&mut self) -> String {
        let text = self.copied();
        if self.selection().is_none() {
            let last = self.lines.len() - 1;
            // The last line has no line break to take; the one before it gives up its own.
            (self.anchor, self.row, self.col) = match self.row {
                r if r < last => (Some((r, 0)), r + 1, 0),
                0 => (Some((0, 0)), 0, self.len(0)),
                r => (Some((r - 1, self.len(r - 1))), r, self.len(r)),
            };
        }
        self.take_selection();
        text
    }

    /// Put `text` at the cursor in place of the selection; its line breaks become lines.
    pub fn insert_text(&mut self, text: &str) {
        self.take_selection();
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let at = self.byte(self.row, self.col);
        let rest = self.lines[self.row].split_off(at);
        let mut parts = text.split('\n');
        self.lines[self.row].push_str(parts.next().unwrap_or_default());
        for part in parts {
            self.row += 1;
            self.lines.insert(self.row, part.to_string());
        }
        self.col = self.len(self.row);
        self.lines[self.row].push_str(&rest);
        self.dirty = true;
        self.warned = false;
    }

    /// Before a move: Shift keeps (or starts) the selection, any other move drops it.
    pub fn select(&mut self, extend: bool) {
        self.anchor = if extend { Some(self.anchor.unwrap_or((self.row, self.col))) } else { None };
    }

    pub fn select_all(&mut self) {
        self.anchor = Some((0, 0));
        self.row = self.lines.len() - 1;
        self.col = self.len(self.row);
    }

    /// Put the cursor at `at` (clamped to the text), extending the selection or dropping it.
    pub fn move_to(&mut self, at: Pos, extend: bool) {
        self.select(extend);
        self.row = at.0.min(self.lines.len() - 1);
        self.col = at.1.min(self.len(self.row));
    }

    /// Select the word under the cursor.
    pub fn select_word(&mut self) {
        let (from, to) = word_at(&self.lines[self.row], self.col);
        self.anchor = Some((self.row, from));
        self.col = to;
    }

    /// Remove the selected text, leaving the cursor where it began. False when nothing is selected.
    fn take_selection(&mut self) -> bool {
        let Some(((r0, c0), (r1, c1))) = self.selection() else {
            self.anchor = None;
            return false;
        };
        let tail = self.lines[r1][self.byte(r1, c1)..].to_string();
        let at = self.byte(r0, c0);
        self.lines[r0].truncate(at);
        self.lines[r0].push_str(&tail);
        self.lines.drain(r0 + 1..=r1);
        (self.row, self.col, self.anchor) = (r0, c0, None);
        self.dirty = true;
        true
    }

    pub fn insert(&mut self, c: char) {
        self.take_selection();
        let at = self.byte(self.row, self.col);
        self.lines[self.row].insert(at, c);
        self.col += 1;
        self.dirty = true;
    }

    pub fn newline(&mut self) {
        self.take_selection();
        let at = self.byte(self.row, self.col);
        let rest = self.lines[self.row].split_off(at);
        // Keep the line's indent.
        let indent: String = self.lines[self.row].chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        self.row += 1;
        self.col = indent.chars().count();
        self.lines.insert(self.row, format!("{indent}{rest}"));
        self.dirty = true;
    }

    pub fn backspace(&mut self) {
        if self.take_selection() {
            return;
        }
        if self.col > 0 {
            let at = self.byte(self.row, self.col - 1);
            self.lines[self.row].remove(at);
            self.col -= 1;
        } else if self.row > 0 {
            let line = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.len(self.row);
            self.lines[self.row].push_str(&line);
        } else {
            return;
        }
        self.dirty = true;
    }

    pub fn delete(&mut self) {
        if self.take_selection() {
            return;
        }
        if self.col < self.len(self.row) {
            let at = self.byte(self.row, self.col);
            self.lines[self.row].remove(at);
        } else if self.row + 1 < self.lines.len() {
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
        } else {
            return;
        }
        self.dirty = true;
    }

    pub fn go(&mut self, drow: isize, dcol: isize) {
        if dcol < 0 && self.col == 0 && self.row > 0 {
            self.row -= 1;
            self.col = self.len(self.row);
            return;
        }
        if dcol > 0 && self.col >= self.len(self.row) && self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
            return;
        }
        self.row = (self.row as isize + drow).min(self.lines.len() as isize - 1).max(0) as usize;
        self.col = ((self.col as isize + dcol).max(0) as usize).min(self.len(self.row));
    }

    pub fn home(&mut self) {
        self.col = 0;
    }

    pub fn end(&mut self) {
        self.col = self.len(self.row);
    }

    /// The cursor's screen column within its line (tabs are four wide).
    pub fn cursor_x(&self) -> usize {
        x_of(&self.lines[self.row], self.col)
    }
}

impl FilesTree {
    pub fn new(root: PathBuf, branch: Option<String>) -> FilesTree {
        FilesTree {
            root,
            branch,
            all: Vec::new(),
            expanded: HashSet::new(),
            sel: 0,
            filter: String::new(),
            filtering: false,
            git: HashMap::new(),
            editing: HashMap::new(),
            preview: None,
            recent: false,
            recent_list: None,
            loading: true,
            scroll: 0,
            edit: None,
            in_preview: false,
            reveal: None,
            raw: None,
            drawn_cache: None,
            dragging: false,
            last_press: None,
        }
    }

    /// The rows on screen: the tree (folders open or closed), filter matches, or recent files.
    pub fn visible(&self) -> Vec<(usize, FileNode)> {
        if self.recent {
            return self
                .recent_list
                .iter()
                .flatten()
                .filter(|e| self.filter.is_empty() || e.path.to_string_lossy().to_lowercase().contains(&self.filter.to_lowercase()))
                .map(|e| FileNode {
                    path: e.path.clone(),
                    rel: e.path.display().to_string(),
                    label: format!(
                        "{}  {} · {}",
                        e.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                        e.place,
                        super::files::ago(e.modified)
                    ),
                    depth: 0,
                    is_dir: false,
                })
                .enumerate()
                .collect();
        }
        if !self.filter.is_empty() {
            let mut hits: Vec<(i32, FileNode)> = self
                .all
                .iter()
                .filter(|n| !n.is_dir)
                .filter_map(|n| super::files::fuzzy(&self.filter, &n.rel).map(|s| (s, FileNode { label: n.rel.clone(), depth: 0, ..n.clone() })))
                .collect();
            hits.sort_by_key(|(s, _)| -s);
            return hits.into_iter().map(|(_, n)| n).enumerate().collect();
        }
        self.all
            .iter()
            .filter(|n| n.path.ancestors().skip(1).take_while(|a| *a != self.root).all(|a| self.expanded.contains(a)))
            .cloned()
            .enumerate()
            .collect()
    }

    pub fn selected(&self) -> Option<FileNode> {
        self.visible().into_iter().nth(self.sel).map(|(_, n)| n)
    }

    pub fn editing_by(&self, path: &Path) -> Option<String> {
        let rel = rel_of(&self.root, path)?;
        self.editing.get(&rel).cloned()
    }

    /// The previewed file is markdown, drawn or shown raw.
    pub fn markdown(&self) -> bool {
        self.preview.as_ref().is_some_and(|(p, _)| super::markdown::is_markdown(p))
    }

    pub fn toggle_raw(&mut self) {
        let Some((path, _)) = self.preview.as_ref().filter(|_| self.markdown()) else { return };
        self.raw = if self.raw.as_ref() == Some(path) { None } else { Some(path.clone()) };
        self.drawn_cache = None;
    }

    /// The previewed file is markdown and drawn, not shown raw.
    pub fn drawn(&self) -> bool {
        self.preview.as_ref().is_some_and(|(p, _)| super::markdown::is_markdown(p) && self.raw.as_ref() != Some(p))
    }

    /// Draw the previewed markdown `width` columns wide in `theme`, unless it is shown raw or
    /// already drawn so: parsing a large file costs too much to repeat every frame and key.
    pub(super) fn lay_out(&mut self, width: u16, t: &Theme, theme: &str) {
        // A preview that isn't on screen has width 0; laid out that narrow, every character
        // would take a line of its own.
        const MIN_WIDTH: u16 = 20;
        let width = width.max(MIN_WIDTH);
        let Some((path, lines)) = self.preview.as_ref().filter(|_| self.drawn()) else { return };
        if self.drawn_cache.as_ref().is_some_and(|c| c.path == *path && c.width == width && c.theme == theme) {
            return;
        }
        let lines = super::markdown::render(&lines.join("\n"), width, t);
        self.drawn_cache = Some(DrawnCache { path: path.clone(), width, theme: theme.to_string(), lines });
    }

    /// The drawn markdown from the last `lay_out`, unless the file is shown raw.
    pub(super) fn pretty(&self) -> Option<&[Vec<super::design::Seg>]> {
        let (path, _) = self.preview.as_ref().filter(|_| self.drawn())?;
        self.drawn_cache.as_ref().filter(|c| c.path == *path).map(|c| c.lines.as_slice())
    }

    /// Scroll so the editor's cursor is on screen, `page` lines tall.
    pub fn follow_cursor(&mut self, page: usize) {
        let Some(row) = self.edit.as_ref().map(|ed| ed.row) else { return };
        if row < self.scroll {
            self.scroll = row;
        } else if row >= self.scroll + page {
            self.scroll = row + 1 - page;
        }
    }

    /// Shown line `i` as plain text: the edit's, the drawn markdown's, or the file's.
    pub fn shown_text(&self, i: usize) -> Option<String> {
        if let Some(ed) = &self.edit {
            return ed.lines.get(i).cloned();
        }
        match self.pretty() {
            Some(drawn) => drawn.get(i).map(|segs| segs.iter().map(|(s, _)| s.as_str()).collect()),
            None => self.preview.as_ref().and_then(|(_, l)| l.get(i).cloned()),
        }
    }

    /// Columns before the text: line numbers, except on drawn markdown.
    pub fn gutter(&self) -> u16 {
        if self.edit.is_none() && self.pretty().is_some() { 0 } else { GUTTER }
    }

    /// The place in the preview's text under the cell (x, y) of `area`, the preview on screen;
    /// a cell outside it counts as the nearest edge.
    pub fn text_at(&self, area: ratatui::layout::Rect, x: u16, y: u16) -> Pos {
        let y = y.clamp(area.y, area.bottom().saturating_sub(1));
        let line = self.scroll + (y - area.y) as usize;
        let text = self.shown_text(line).unwrap_or_default();
        (line, col_at(&text, x.saturating_sub(area.x + self.gutter()) as usize))
    }

    /// How many lines the preview shows at `width`: the edit, the drawn markdown, or the text.
    pub(super) fn shown_lines(&mut self, width: u16, t: &Theme, theme: &str) -> usize {
        if let Some(ed) = &self.edit {
            return ed.lines.len();
        }
        self.lay_out(width, t, theme);
        match self.pretty() {
            Some(drawn) => drawn.len(),
            None => self.preview.as_ref().map_or(0, |(_, l)| l.len()),
        }
    }

    pub fn refresh_preview(&mut self) {
        let Some(n) = self.selected() else {
            self.preview = None;
            return;
        };
        if n.is_dir {
            let kids = self.all.iter().filter(|c| c.path.parent() == Some(n.path.as_path())).count();
            self.preview = Some((n.path.clone(), vec![format!("folder · {kids} items")]));
            return;
        }
        if self.preview.as_ref().is_some_and(|(p, _)| *p == n.path) {
            return;
        }
        self.scroll = 0;
        self.drawn_cache = None;
        self.preview = Some((n.path.clone(), super::files::preview(&n.path)));
    }
}

pub fn rel_of(root: &Path, path: &Path) -> Option<String> {
    Some(path.strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/"))
}

/// Everything under `root` (honouring .gitignore), folders first at each level.
pub fn scan_tree(root: &Path) -> Vec<FileNode> {
    let mut nodes: Vec<FileNode> = ignore::WalkBuilder::new(root)
        .max_depth(Some(14))
        .build()
        .flatten()
        .take(20_000)
        .filter(|e| e.depth() > 0)
        .filter_map(|e| {
            let is_dir = e.file_type()?.is_dir();
            let path = e.into_path();
            let rel = rel_of(root, &path)?;
            let label = path.file_name()?.to_string_lossy().into_owned();
            Some(FileNode { depth: rel.matches('/').count(), rel, label, path, is_dir })
        })
        .collect();
    let key = |n: &FileNode| -> Vec<(u8, String)> {
        let parts: Vec<&str> = n.rel.split('/').collect();
        parts
            .iter()
            .enumerate()
            .map(|(i, p)| (if i + 1 < parts.len() || n.is_dir { 0 } else { 1 }, p.to_lowercase()))
            .collect()
    };
    nodes.sort_by_key(key);
    nodes
}

/// `git status` marks: 'A' for new files, 'M' for anything else changed.
pub fn git_marks(root: &Path) -> HashMap<String, char> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C").arg(root).args(["status", "--porcelain=v1", "-uall"]);
    crate::proc::quiet(&mut cmd);
    let Ok(out) = cmd.output() else { return HashMap::new() };
    // Paths are relative to the repo top, which may sit above `root`.
    let prefix = {
        let mut c = std::process::Command::new("git");
        c.arg("-C").arg(root).args(["rev-parse", "--show-prefix"]);
        crate::proc::quiet(&mut c);
        c.output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.len() > 3)
        .filter_map(|l| {
            let (xy, path) = l.split_at(3);
            let path = path.rsplit(" -> ").next().unwrap_or(path).trim_matches('"');
            let rel = path.strip_prefix(&prefix)?.to_string();
            let mark = if xy.starts_with("??") || xy.starts_with('A') { 'A' } else { 'M' };
            Some((rel, mark))
        })
        .collect()
}

// ---- a small syntax highlighter for previews ---------------------------------------------------

const KEYWORDS: &[&str] = &[
    "import", "from", "export", "default", "const", "let", "var", "function", "return", "if", "else", "for", "while",
    "async", "await", "class", "new", "type", "interface", "extends", "implements", "fn", "pub", "use", "mod", "struct",
    "enum", "impl", "match", "mut", "self", "Self", "trait", "where", "def", "elif", "in", "not", "and", "or", "is",
    "lambda", "with", "as", "try", "catch", "except", "finally", "throw", "raise", "package", "func", "go", "defer",
    "true", "false", "null", "None", "True", "False", "nil", "undefined", "static", "public", "private", "void",
];

/// Colour a line of code: keywords magenta, strings green, calls blue, comments grey.
pub fn highlight(line: &str, t: &Theme) -> Vec<(String, Style)> {
    let magenta = Color::Rgb(0xa5, 0x93, 0xff);
    let green = Color::Rgb(0x7f, 0xd9, 0x62);
    let blue = Color::Rgb(0x5a, 0xa9, 0xff);
    let yellow = Color::Rgb(0xe8, 0xc5, 0x65);
    let plain = Style::default().fg(t.fg);
    let mut out: Vec<(String, Style)> = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut text = String::new();
    let flush = |text: &mut String, out: &mut Vec<(String, Style)>| {
        if !text.is_empty() {
            out.push((std::mem::take(text), plain));
        }
    };
    while i < chars.len() {
        let c = chars[i];
        // Comments run to the end of the line.
        if (c == '/' && chars.get(i + 1) == Some(&'/')) || (c == '#' && (i == 0 || chars[i - 1].is_whitespace())) {
            flush(&mut text, &mut out);
            out.push((chars[i..].iter().collect(), Style::default().fg(t.muted)));
            return out;
        }
        if c == '"' || c == '\'' || c == '`' {
            flush(&mut text, &mut out);
            let mut j = i + 1;
            while j < chars.len() && chars[j] != c {
                if chars[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            let end = (j + 1).min(chars.len());
            out.push((chars[i..end].iter().collect(), Style::default().fg(green)));
            i = end;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();
            let style = if KEYWORDS.contains(&word.as_str()) {
                Some(Style::default().fg(magenta))
            } else if chars.get(j) == Some(&'(') {
                Some(Style::default().fg(blue))
            } else {
                None
            };
            match style {
                Some(s) => {
                    flush(&mut text, &mut out);
                    out.push((word, s));
                }
                None => text.push_str(&word),
            }
            i = j;
            continue;
        }
        if c.is_ascii_digit() && (i == 0 || !chars[i - 1].is_alphanumeric()) {
            flush(&mut text, &mut out);
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '.') {
                j += 1;
            }
            out.push((chars[i..j].iter().collect(), Style::default().fg(yellow)));
            i = j;
            continue;
        }
        text.push(c);
        i += 1;
    }
    flush(&mut text, &mut out);
    let _ = Modifier::empty();
    out
}

#[cfg(test)]
mod tests {

    #[test]
    fn edit_a_file_in_place() {
        let dir = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-edit-{}"), std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.ts");
        std::fs::write(&f, "const a = 1;\r\n  if (x) {\r\n").unwrap();
        let mut ed = Edit::open(&f).unwrap();
        ed.end();
        ed.backspace();
        for c in "2;".chars() {
            ed.insert(c);
        }
        ed.go(1, 0);
        ed.end();
        ed.newline();
        ed.insert('y');
        assert!(ed.dirty);
        assert_eq!(ed.save().unwrap(), Saved::Done);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "const a = 12;\r\n  if (x) {\r\n  y\r\n", "edits, the indent kept, CRLF kept");
        ed.go(0, -100);
        ed.backspace();
        assert_eq!(ed.lines[1], "  if (x) {  y", "backspace at a line's start joins it to the one above");
        // Someone else (an agent) changes the file: saving doesn't overwrite it, until asked twice.
        std::fs::write(&f, "agent's version\r\n").unwrap();
        assert_eq!(ed.save().unwrap(), Saved::ChangedOnDisk);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "agent's version\r\n", "nothing written");
        assert_eq!(ed.save().unwrap(), Saved::Done, "the second Ctrl+S overwrites on purpose");
        assert!(std::fs::read_to_string(&f).unwrap().starts_with("const a = 12;"));
        // One stray CRLF in an LF file keeps it LF.
        let g = dir.join("b.txt");
        std::fs::write(&g, "a\nb\r\nc\nd\n").unwrap();
        let mut ed = Edit::open(&g).unwrap();
        ed.insert('x');
        ed.save().unwrap();
        assert_eq!(std::fs::read_to_string(&g).unwrap(), "xa\nb\nc\nd\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_review_mark_clears_when_the_file_changes() {
        let dir = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-review-{}"), std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "one").unwrap();
        std::fs::write(dir.join("b.rs"), "two").unwrap();
        let f = |p: &str| super::super::tasks::Changed { path: p.into(), added: 1, removed: 0, untracked: false };
        let files = vec![f("a.rs"), f("b.rs")];
        let marks: HashMap<String, u64> = [("a.rs".to_string(), fingerprint(&dir, "a.rs")), ("b.rs".to_string(), fingerprint(&dir, "b.rs"))].into();
        assert_eq!(still_reviewed(&dir, &files, Some(&marks)).len(), 2);
        std::fs::write(dir.join("b.rs"), "two, edited by the agent").unwrap();
        assert_eq!(still_reviewed(&dir, &files, Some(&marks)), HashSet::from(["a.rs".to_string()]), "b changed since");
        let _ = std::fs::remove_dir_all(&dir);
    }
    use super::*;
    use crate::client::tasks::{Changed, Review, Stage, TaskRow};

    fn review(paths: &[&str]) -> Review {
        Review {
            task: TaskRow {
                ws: 1,
                name: "t".into(),
                branch: "b".into(),
                base: "main".into(),
                stage: Stage::Ready,
                summary: String::new(),
                dirty: 0,
                ahead: 0,
                agent: None,
                dir: PathBuf::new(),
                root: PathBuf::new(),
            },
            merge_base: String::new(),
            files: paths.iter().map(|p| Changed { path: p.to_string(), added: 1, removed: 0, untracked: false }).collect(),
            sel: 0,
            diff: Vec::new(),
            diff_sel: None,
            scroll: 0,
            confirm: None,
        }
    }

    #[test]
    fn change_tree_groups_folders() {
        let mut v = ChangesView {
            dir: PathBuf::new(),
            review: Some(review(&["src/rate.ts", "test/rate.test.ts", "src/auth.ts", "README.md"])),
            error: None,
            agent: String::new(),
            said: String::new(),
            term: None,
            ws: None,
            linked: true,
            confirm: None,
            reviewed: HashSet::new(),
        };
        let rows = v.rows();
        let shape: Vec<String> = rows
            .iter()
            .map(|r| match r {
                ChangesRow::Dir(n, d) => format!("{}{n}/", "  ".repeat(*d)),
                ChangesRow::File(i, d) => format!("{}{}", "  ".repeat(*d), v.review.as_ref().unwrap().files[*i].path),
                ChangesRow::Reviewed(n) => format!("REVIEWED {n}"),
            })
            .collect();
        assert_eq!(shape, ["README.md", "src/", "  src/auth.ts", "  src/rate.ts", "test/", "  test/rate.test.ts"]);
        // Reviewed files sink, under their own heading; Up/Down follow what's shown.
        v.reviewed = HashSet::from(["src/auth.ts".to_string(), "README.md".to_string()]);
        let files = &v.review.as_ref().unwrap().files;
        let order: Vec<&str> = v.order().iter().map(|i| files[*i].path.as_str()).collect();
        assert_eq!(order, ["src/rate.ts", "test/rate.test.ts", "README.md", "src/auth.ts"]);
        assert!(v.rows().contains(&ChangesRow::Reviewed(2)));
        v.review = None;
        assert!(v.rows().is_empty());
    }

    #[test]
    fn highlights_code() {
        let t = Theme::named(crate::theme::DEFAULT);
        let segs = highlight("const user = await find('x'); // look", &t);
        let words: Vec<&str> = segs.iter().map(|(s, _)| s.as_str()).collect();
        assert!(words.contains(&"const") && words.contains(&"'x'") && words.contains(&"find"));
        assert!(words.last().unwrap().starts_with("// look"));
    }
}
