//! The evidence sheet: what a check ran, where, when, who saw it, and what it printed.
//! The summary is read live from the snapshot; only the output is fetched, by
//! `Query::CheckOutput`, and fetched again when a new run replaces the result.
use super::check;
use super::*;
use crate::client::{Btn, View};
use crate::protocol::{CheckState, ClientMsg, Query};

/// Label column of the Command / Folder / Ran / Seen by rows.
const LABEL_W: usize = 9;
/// Line-number column of the output block, with its gap.
const NUM_W: usize = 5;
const PAGE: usize = 10;
const EXIT_NOT_EXECUTABLE: i32 = 126;
const EXIT_NOT_FOUND: i32 = 127;
/// Below this the output block is not worth drawing.
const MIN_BLOCK_ROWS: u16 = 2;

/// What identifies a run: when it starts, how it ends. A change means the fetched output is old.
type RunKey = (u64, CheckState, Option<i32>);

/// A check's evidence in the sheet.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CheckView {
    pub term: TermId,
    pub output: Option<String>,
    run: RunKey,
    /// Showing the whole output from line `scroll`, rather than the tail that fits.
    full: bool,
    scroll: usize,
    /// Output rows on screen at the last draw, so keys know how far there is to scroll.
    room: usize,
}

fn run_key(info: &CheckInfo) -> RunKey {
    (info.started_at, info.state, info.exit_code)
}

fn count_lines(output: &str) -> usize {
    if output.trim().is_empty() { 0 } else { output.lines().count() }
}

impl CheckView {
    fn total(&self) -> usize {
        self.output.as_deref().map_or(0, count_lines)
    }

    fn last_start(&self) -> usize {
        self.total().saturating_sub(self.room.max(1))
    }

    /// The first line on screen: the top of the tail that fits, or where the user scrolled to.
    fn start(&self) -> usize {
        if self.full { self.scroll.min(self.last_start()) } else { self.last_start() }
    }

    fn scrollable(&self) -> bool {
        self.total() > self.room
    }

    fn toggle_full(&mut self) {
        if self.scrollable() {
            self.full = !self.full;
            self.scroll = 0;
        }
    }

    /// Scrolling the tail leaves it from where it starts.
    fn scroll_by(&mut self, up: bool, lines: usize) {
        if !self.scrollable() {
            return;
        }
        let now = self.start();
        self.scroll = if up { now.saturating_sub(lines) } else { (now + lines).min(self.last_start()) };
        self.full = true;
    }
}

/// The states that ask to be looked at: Enter on their row opens the sheet.
fn needs_a_look(info: &CheckInfo) -> bool {
    matches!(info.state, CheckState::Failed | CheckState::CouldNotRun | CheckState::Overridden)
}

impl App {
    /// Whether Enter on `term`'s sidebar row shows its evidence rather than focusing it.
    pub(in crate::client) fn check_wants_a_look(&self, term: TermId) -> bool {
        let model = self.hy_model();
        find(&model, term).and_then(|(.., s)| check::shown(s)).is_some_and(needs_a_look)
    }

    pub(in crate::client) fn open_check_sheet(&mut self, term: TermId) {
        let Some(info) = self.snap.terms.get(&term).and_then(|t| t.check.as_ref()) else { return };
        let run = run_key(info);
        self.mode = Mode::Normal;
        self.view = Some(View::Check(Box::new(CheckView { term, output: None, run, full: false, scroll: 0, room: 0 })));
        self.send(ClientMsg::Query(Query::CheckOutput { term }));
    }

    /// After a snapshot: a sheet whose check is gone closes, and one whose run was replaced
    /// forgets the output it had and asks for the new one.
    pub(in crate::client) fn check_view_sync(&mut self) {
        let Some(View::Check(v)) = &mut self.view else { return };
        let Some(info) = self.snap.terms.get(&v.term).and_then(|t| t.check.as_ref()) else {
            self.view = None;
            return;
        };
        if run_key(info) == v.run {
            return;
        }
        let term = v.term;
        (v.run, v.output, v.full, v.scroll) = (run_key(info), None, false, 0);
        self.send(ClientMsg::Query(Query::CheckOutput { term }));
    }

    /// The output the daemon sent for the open sheet.
    pub(in crate::client) fn check_output_arrived(&mut self, text: String) {
        if let Some(View::Check(v)) = &mut self.view {
            v.output = Some(text);
        }
    }

    /// Keys in the sheet; returns false when it closes. Re-run, override and undo are
    /// drawn but not wired yet.
    pub(in crate::client) fn on_check_key(&mut self, v: &mut CheckView, k: &KeyEvent) -> bool {
        match k.code {
            KeyCode::Esc => return false,
            KeyCode::Char('f') => v.toggle_full(),
            KeyCode::Up => v.scroll_by(true, 1),
            KeyCode::Down => v.scroll_by(false, 1),
            KeyCode::PageUp => v.scroll_by(true, PAGE),
            KeyCode::PageDown => v.scroll_by(false, PAGE),
            _ => {}
        }
        true
    }
}

/// "Check · fix-footer › npm test".
pub(in crate::client) fn check_title(app: &App) -> String {
    let Some(View::Check(v)) = &app.view else { return "Check".into() };
    let model = app.hy_model();
    let name = find(&model, v.term).map(|(.., s)| s.name.clone()).unwrap_or_default();
    let command = app.snap.terms.get(&v.term).and_then(|t| t.check.as_ref()).map(|c| c.command.clone()).unwrap_or_default();
    format!("Check · {name} › {command}")
}

fn state_word(info: &CheckInfo) -> &'static str {
    match info.state {
        CheckState::Checking => "checking",
        CheckState::Passed => "passed",
        CheckState::Failed => "failed",
        CheckState::Overridden => "overridden",
        CheckState::CouldNotRun => "could not run",
    }
}

fn exit_words(info: &CheckInfo) -> String {
    match info.exit_code {
        Some(EXIT_NOT_FOUND) => format!("exit {EXIT_NOT_FOUND} command not found"),
        Some(EXIT_NOT_EXECUTABLE) => format!("exit {EXIT_NOT_EXECUTABLE} not executable"),
        Some(code) => format!("exit {code}"),
        None => String::new(),
    }
}

fn ago(at: u64) -> String {
    if at == 0 { "never".into() } else { format!("{} ago", age(at)) }
}

fn ran_words(info: &CheckInfo) -> String {
    let mut parts = vec![if info.state == CheckState::Checking { format!("started {}", ago(info.started_at)) } else { ago(info.started_at) }];
    if !matches!(info.state, CheckState::Checking | CheckState::CouldNotRun) {
        parts.push(format!("took {}", check::duration(info.duration_ms)));
    }
    if info.after_turn_ended_at > 0 {
        parts.push(format!("after the turn ended {}", ago(info.after_turn_ended_at)));
    }
    parts.join(" · ")
}

fn seen_by_words(said: &str) -> String {
    let last = said.lines().rev().map(str::trim).find(|l| !l.is_empty());
    match last {
        Some(s) => format!("Seshi ran this itself. The agent said \"{s}\"."),
        None => "Seshi ran this itself.".into(),
    }
}

/// `text` wrapped to `width`, one row of segments per line, all in `style`.
fn wrapped(text: &str, width: usize, style: Style) -> Vec<Vec<Seg>> {
    super::popover::wrap(text, width).into_iter().map(|l| vec![seg(l, style)]).collect()
}

/// The rows of a banner under the result chip, if its state has one.
fn banner(info: &CheckInfo, width: usize, t: &Theme) -> Vec<Vec<Seg>> {
    let c = Style::default().bg(t.card);
    match info.state {
        CheckState::Overridden => {
            let by = match info.overridden_at {
                Some(at) => format!("⊘ Overridden by you {}", ago(at)),
                None => "⊘ Overridden by you".into(),
            };
            let mut rows = vec![[vec![seg(by, c.fg(t.strong).add_modifier(Modifier::BOLD)), seg(" · ", c.fg(t.muted))], cap_hints(t, t.card, &[("u", "undo")])].concat()];
            if !info.override_reason.is_empty() {
                rows.extend(wrapped(&info.override_reason, width, c.fg(t.text).add_modifier(Modifier::ITALIC)));
            }
            rows.push(vec![seg("next run replaces this", c.fg(t.muted))]);
            rows
        }
        CheckState::CouldNotRun => {
            let program = info.command.split_whitespace().next().unwrap_or_default();
            let fix = if info.exit_code == Some(EXIT_NOT_FOUND) && !program.is_empty() {
                format!("Fix the command in settings (,) or install {program}, then r.")
            } else {
                "Fix the command in settings (,), then r.".into()
            };
            let mut rows = wrapped("Not a test failure. The work hasn't been checked.", width, c.fg(t.strong).add_modifier(Modifier::BOLD));
            rows.extend(wrapped(&fix, width, c.fg(t.muted)));
            rows
        }
        _ => Vec::new(),
    }
}

/// A line of output in the colour of what it says.
fn line_color(line: &str, t: &Theme) -> Color {
    let l = line.trim_start();
    let lower = l.to_lowercase();
    if l.starts_with(['✓', '✔']) || l.starts_with("ok ") || l.starts_with("PASS") {
        t.done
    } else if l.starts_with(['✗', '✕', '✘']) || l.starts_with("FAIL") || ["error", "failed", "panicked"].iter().any(|w| lower.contains(w)) {
        t.err
    } else {
        t.text
    }
}

/// One line as text safe to draw: what a command printed is not trusted to move the cursor
/// or restyle the screen. Colour codes go, tabs become spaces, a carriage return keeps
/// what was written last.
fn clean(line: &str) -> String {
    let line = line.rsplit('\r').next().unwrap_or_default();
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                if chars.next_if_eq(&'[').is_some() {
                    while chars.next_if(|c| !('@'..='~').contains(c)).is_some() {}
                    chars.next();
                } else if chars.next_if_eq(&']').is_some() {
                    // A title or link: up to the bell, or the ESC that ends it.
                    while chars.next_if(|c| !matches!(c, '\u{7}' | '\u{1b}')).is_some() {}
                    if chars.next() == Some('\u{1b}') {
                        chars.next();
                    }
                }
            }
            '\t' => out.push_str("    "),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// "Command  npm test": a label, then its lines of value. Returns the rows taken.
fn info_row(buf: &mut Buffer, x: u16, y: u16, w: u16, label: &str, value: Vec<Vec<Seg>>, t: &Theme) -> u16 {
    let c = Style::default().bg(t.card);
    put(buf, x, y, &[seg(format!("{label:<LABEL_W$}"), c.fg(t.muted))], x + w);
    for (i, line) in value.iter().enumerate() {
        put(buf, x + LABEL_W as u16, y + i as u16, line, x + w);
    }
    value.len().max(1) as u16
}

fn button(app: &mut App, buf: &mut Buffer, x: u16, y: u16, max: u16, (label, key): (&str, char), t: &Theme) -> u16 {
    let look = Look::of(&app.cfg.ui);
    let key_s = key.to_string();
    let w = segs_width(&button_pill(look, t, label, &key_s, false, false, t.card));
    let r = Rect { x, y, width: w, height: 1 };
    let mut segs = button_pill(look, t, label, &key_s, false, hovered(app, r), t.card);
    if key == 'O'
        && let Some(s) = segs.iter_mut().find(|(text, _)| text.starts_with(label))
    {
        s.1 = s.1.fg(t.err);
    }
    if x + w > max {
        return x;
    }
    put(buf, x, y, &segs, max);
    app.hits.push((r, crate::client::Hit::Button(Btn::ViewKey(key))));
    x + w + 1
}

/// Buttons for what could be done with this result. They are drawn and clickable; the keys
/// they stand for are wired with the actions.
fn buttons(app: &mut App, buf: &mut Buffer, x: u16, y: u16, max: u16, info: &CheckInfo, t: &Theme) {
    let mut at = x;
    if info.state != CheckState::Checking {
        at = button(app, buf, at, y, max, ("Re-run", 'r'), t);
    }
    match info.state {
        CheckState::Failed => {
            button(app, buf, at, y, max, ("Override…", 'O'), t);
        }
        CheckState::Overridden => {
            button(app, buf, at, y, max, ("Undo override", 'u'), t);
        }
        _ => {}
    }
}

/// The output's header: its chip, how much of it shows, and the key that changes that.
fn output_header(app: &mut App, buf: &mut Buffer, x: u16, y: u16, w: u16, v: &CheckView, t: &Theme) {
    let c = Style::default().bg(t.card);
    let total = v.total();
    let mut segs = vec![seg(" OUTPUT ", Style::default().bg(t.btn).fg(t.strong).add_modifier(Modifier::BOLD))];
    let (start, shown) = (v.start(), v.room.min(total));
    let how = if total == 0 || v.output.is_none() {
        String::new()
    } else if !v.scrollable() {
        format!("all {total} lines")
    } else if v.full {
        format!("lines {}–{} of {total}", start + 1, start + shown)
    } else {
        format!("last {shown} of {total} lines")
    };
    if !how.is_empty() {
        segs.push(seg(format!("  {how}"), c.fg(t.muted)));
    }
    put(buf, x, y, &segs, x + w);
    if v.scrollable() {
        let hint = cap_hints(t, t.card, &[("f", if v.full { "tail" } else { "full output" })]);
        let hw = segs_width(&hint);
        let at = (x + w).saturating_sub(hw);
        if at > x + segs_width(&segs) {
            put(buf, at, y, &hint, x + w);
            app.hits.push((Rect { x: at, y, width: hw, height: 1 }, crate::client::Hit::Button(Btn::ViewKey('f'))));
        }
    }
}

/// The output on the darker ground, numbered, with ▲ ▼ where there is more.
fn output_block(buf: &mut Buffer, r: Rect, v: &CheckView, info: &CheckInfo, t: &Theme) {
    fill(buf, r, t.bg);
    let c = Style::default().bg(t.bg);
    let Some(output) = &v.output else {
        put(buf, r.x + 1, r.y, &[seg("reading the output…", c.fg(t.muted).add_modifier(Modifier::ITALIC))], r.right());
        return;
    };
    if v.total() == 0 {
        put(buf, r.x + 1, r.y, &[seg("(the command printed nothing)", c.fg(t.muted).add_modifier(Modifier::ITALIC))], r.right());
        if r.height > 1 && info.exit_code == Some(0) {
            put(buf, r.x + 1, r.y + 1, &[seg("exit 0 is what passed it.", c.fg(t.muted))], r.right());
        }
        return;
    }
    let start = v.start();
    let text_w = (r.width as usize).saturating_sub(NUM_W + 2);
    for (i, line) in output.lines().enumerate().skip(start).take(r.height as usize) {
        let y = r.y + (i - start) as u16;
        let line = clean(line);
        let num = seg(format!("{:>w$} ", i + 1, w = NUM_W - 1), c.fg(t.muted));
        put(buf, r.x + 1, y, &[num, seg(truncate(&line, text_w), c.fg(line_color(&line, t)))], r.right());
    }
    let mark = |buf: &mut Buffer, y: u16, s: &str| {
        put(buf, r.right().saturating_sub(1), y, &[seg(s, c.fg(t.accent))], r.right());
    };
    if start > 0 {
        mark(buf, r.y, "▲");
    }
    if start + (r.height as usize) < v.total() {
        mark(buf, r.bottom().saturating_sub(1), "▼");
    }
}

/// Rows the output block should get before the spacers between sections are given up.
const WANT_BLOCK_ROWS: usize = 4;

/// The sheet's inside.
pub(in crate::client) fn draw_check(app: &mut App, buf: &mut Buffer, inside: Rect, t: &Theme, v: &mut CheckView) {
    let Some(term) = app.snap.terms.get(&v.term).cloned() else { return };
    let Some(info) = term.check.clone() else { return };
    let (x, w) = (inside.x + 2, inside.width.saturating_sub(4));
    let c = Style::default().bg(t.card);
    let folder = term.top.clone().unwrap_or_else(|| term.cwd.clone());
    let value_w = (w as usize).saturating_sub(LABEL_W);
    let strong = c.fg(t.strong).add_modifier(Modifier::BOLD);
    let banner = banner(&info, w as usize, t);
    let rows = [
        ("Command", vec![vec![seg(truncate(&info.command, value_w), strong)]]),
        ("Folder", vec![vec![seg(truncate(&folder.display().to_string(), value_w), c.fg(t.text))]]),
        ("Ran", wrapped(&ran_words(&info), value_w, c.fg(t.text))),
        ("Seen by", wrapped(&seen_by_words(&term.said), value_w, c.fg(t.text))),
    ];
    let has_buttons = info.state != CheckState::Checking;
    // Chip, banner, rows, buttons and the output's header; the spacers go first when short.
    let fixed = 1 + banner.len() + rows.iter().map(|(_, v)| v.len()).sum::<usize>() + usize::from(has_buttons) + 1;
    let spacers = 2 + usize::from(!banner.is_empty()) + usize::from(has_buttons);
    let tight = (inside.height as usize) < 1 + fixed + spacers + 2 + WANT_BLOCK_ROWS;
    let space = |wanted: bool| u16::from(wanted && !tight);
    let mut y = inside.y + 1;
    let chip = format!(" {} {} ", check::glyph(app, &info), state_word(&info));
    let end = put(buf, x, y, &[seg(chip, Style::default().bg(check::color(t, &info)).fg(t.acc_ink).add_modifier(Modifier::BOLD))], x + w);
    put(buf, end + 2, y, &[seg(exit_words(&info), c.fg(t.strong))], x + w);
    y += 1 + space(true);
    for row in &banner {
        put(buf, x, y, row, x + w);
        y += 1;
    }
    y += space(!banner.is_empty());
    for (label, value) in rows {
        y += info_row(buf, x, y, w, label, value, t);
    }
    y += space(true);
    if has_buttons {
        buttons(app, buf, x, y, x + w, &info, t);
        y += 1 + space(true);
    }
    let bottom = inside.bottom().saturating_sub(2);
    let block = Rect { x: inside.x + 1, y: y + 1, width: inside.width.saturating_sub(2), height: bottom.saturating_sub(y + 1) };
    v.room = if block.height >= MIN_BLOCK_ROWS { block.height as usize } else { 0 };
    if y < bottom {
        output_header(app, buf, x, y, w, v, t);
    }
    if v.room > 0 {
        output_block(buf, block, v, &info, t);
    }
    let mut keys = vec![("Esc", "back")];
    if v.scrollable() {
        keys.extend([("f", if v.full { "tail" } else { "full output" }), ("↑↓", "scroll")]);
    }
    status_bar(buf, inside, t, &keys, &[]);
}
