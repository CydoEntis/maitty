//! The evidence sheet: what a checkpoint ran, where, when, who saw it, and what it printed.
//! The summary is read live from the snapshot; only the output is fetched, by
//! `Query::CheckOutput`, and fetched again when a new run replaces the result.
use super::check;
use crate::brand::{APP_NAME};
use super::*;
use crate::client::{Btn, View};
use crate::output_line::{classify, clean, LineKind};
use crate::protocol::{CheckSource, CheckState, ClientMsg, Query};
use crate::theme::mix;

/// Label column of the Command / Folder / Ran / Seen by rows.
const LABEL_W: usize = 9;
/// Line-number column of the output block.
const NUM_W: usize = 5;
/// Where the output's "all 13 lines" starts, after its chip.
const OUTPUT_NOTE_AT: u16 = 9;
const PAGE: usize = 10;
const EXIT_NOT_EXECUTABLE: i32 = 126;
const EXIT_NOT_FOUND: i32 = 127;
/// The fewest rows the output block gets, even when that pushes it into the buttons.
const MIN_BLOCK_ROWS: u16 = 3;
/// Rows under the output block: a gap, the buttons and the hints.
const FOOT_ROWS: u16 = 3;
/// "Open session o" is only offered when this much is left after the other buttons.
const OPEN_SESSION_ROOM: u16 = 18;
/// How far the output block is darkened from the sheet's ground, and its numbers faded.
const BLOCK_SHADE: f32 = 0.25;
const NUMBER_FADE: f32 = 0.35;
const BANNER_TINT: f32 = 0.12;
/// The sheet's keys, always the same so they are learnt once.
const HINTS: [(&str, &str); 4] = [("Esc", "back"), ("r", "re-run"), ("f", "full output"), ("i", "why")];

/// A banner's tint, and its rows with the column each starts at.
type Banner = (Color, Vec<(u16, Vec<Seg>)>);

/// What identifies a run: when it starts, how it ends. A change means the fetched output is old.
type RunKey = (u64, CheckState, Option<i32>);

/// A checkpoint's evidence in the sheet.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CheckView {
    pub term: TermId,
    /// The checkpoint shown; Tab moves to the session's next one.
    pub name: String,
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
    fn new(term: TermId, info: &CheckInfo) -> CheckView {
        CheckView { term, name: info.name.clone(), output: None, run: run_key(info), full: false, scroll: 0, room: 0 }
    }

    /// Lines of output held: the tail the daemon kept.
    fn held(&self) -> usize {
        self.output.as_deref().map_or(0, count_lines)
    }

    fn last_start(&self) -> usize {
        self.held().saturating_sub(self.room.max(1))
    }

    /// The first line on screen: the top of the tail that fits, or where the user scrolled to.
    fn start(&self) -> usize {
        if self.full { self.scroll.min(self.last_start()) } else { self.last_start() }
    }

    fn scrollable(&self) -> bool {
        self.held() > self.room
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

/// The checkpoint the sheet shows: the one it was opened on, else the headline.
fn shown<'a>(t: &'a TermInfo, name: &str) -> Option<&'a CheckInfo> {
    t.checks.iter().find(|c| c.name == name).or_else(|| t.check())
}

/// The states that ask to be looked at: Enter on their row opens the sheet.
fn needs_a_look(info: &CheckInfo) -> bool {
    matches!(info.state, CheckState::Failed | CheckState::CouldNotRun | CheckState::Overridden | CheckState::Stale | CheckState::Never)
}

impl App {
    /// Whether Enter on `term`'s sidebar row shows its evidence rather than focusing it.
    pub(in crate::client) fn check_wants_a_look(&self, term: TermId) -> bool {
        let model = self.screen_model();
        find(&model, term).and_then(|(.., s)| check::shown(s)).is_some_and(needs_a_look)
    }

    /// The sheet on `term`'s headline checkpoint.
    pub(in crate::client) fn open_check_sheet(&mut self, term: TermId) {
        let Some(info) = self.snap.terms.get(&term).and_then(|t| t.check()) else { return };
        let v = CheckView::new(term, info);
        self.mode = Mode::Normal;
        self.ask_check_output(&v);
        self.show_view(View::Check(Box::new(v)));
    }

    fn ask_check_output(&self, v: &CheckView) {
        let name = (!v.name.is_empty()).then(|| v.name.clone());
        self.send(ClientMsg::Query(Query::CheckOutput { term: v.term, name }));
    }

    /// After a snapshot: a sheet whose checks are gone closes, one whose checkpoint is gone
    /// moves to the headline, and one whose run was replaced asks for the new output.
    pub(in crate::client) fn check_view_sync(&mut self) {
        let Some(View::Check(v)) = &self.view else { return };
        let Some(info) = self.snap.terms.get(&v.term).and_then(|t| shown(t, &v.name)) else {
            self.view = None;
            return;
        };
        if run_key(info) == v.run && info.name == v.name {
            return;
        }
        let fresh = CheckView::new(v.term, info);
        self.ask_check_output(&fresh);
        // The same sheet brought up to date, not a new one: if a click into a pane set it
        // aside, it stays aside rather than taking the keys back mid-word.
        self.view = Some(View::Check(Box::new(fresh)));
    }

    /// The output the daemon sent for the open sheet.
    pub(in crate::client) fn check_output_arrived(&mut self, text: String) {
        if let Some(View::Check(v)) = &mut self.view {
            v.output = Some(text);
        }
    }

    /// Tab / Shift+Tab: the session's next or previous checkpoint.
    fn step_checkpoint(&mut self, v: &mut CheckView, back: bool) {
        let Some(t) = self.snap.terms.get(&v.term) else { return };
        let n = t.checks.len();
        if n < 2 {
            return;
        }
        let at = t.checks.iter().position(|c| c.name == v.name).unwrap_or(0);
        let next = if back { (at + n - 1) % n } else { (at + 1) % n };
        *v = CheckView::new(v.term, &t.checks[next]);
        self.ask_check_output(v);
    }

    /// Keys in the sheet; returns false when it closes.
    pub(in crate::client) fn on_check_key(&mut self, v: &mut CheckView, k: &KeyEvent) -> bool {
        match k.code {
            KeyCode::Esc => return false,
            KeyCode::Char(c @ ('r' | 'O' | 'u' | ',')) => {
                self.do_check_key(v.term, (!v.name.is_empty()).then(|| v.name.clone()), c);
            }
            KeyCode::Char('o') => {
                self.screen_focus(v.term);
                return false;
            }
            KeyCode::Char('i') => self.mode = Mode::Why { term: v.term, side: false },
            KeyCode::Char('f') => v.toggle_full(),
            KeyCode::Tab => self.step_checkpoint(v, false),
            KeyCode::BackTab => self.step_checkpoint(v, true),
            KeyCode::Up => v.scroll_by(true, 1),
            KeyCode::Down => v.scroll_by(false, 1),
            KeyCode::PageUp => v.scroll_by(true, PAGE),
            KeyCode::PageDown => v.scroll_by(false, PAGE),
            _ => {}
        }
        true
    }
}

/// The sheet's title, `CHECK`, and what follows it: "· fix-footer › test".
pub(in crate::client) fn check_title(app: &App) -> String {
    let Some(View::Check(v)) = &app.view else { return String::new() };
    let model = app.screen_model();
    let session = find(&model, v.term).map(|(.., s)| s.name.clone()).unwrap_or_default();
    let name = match app.snap.terms.get(&v.term).and_then(|t| shown(t, &v.name)) {
        Some(c) if c.name.is_empty() => c.command.clone(),
        _ => v.name.clone(),
    };
    format!("· {session} › {name}")
}

/// `1284` as `1,284`.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(d);
    }
    out
}

/// The result chip: its words, fill and ink; then what it came to beside it.
fn chip(app: &App, info: &CheckInfo, t: &Theme) -> (String, Color, Color, String) {
    let g = check::glyph(app, info);
    let exit = info.exit_code.map(|c| format!("exit {c}")).unwrap_or_default();
    let (word, fill, beside) = match info.state {
        CheckState::Failed => ("failed", t.err, exit),
        CheckState::Overridden => ("failed · overridden", t.text, exit),
        CheckState::CouldNotRun => ("could not run", t.violet(), exit),
        CheckState::Passed => ("passed", t.done, exit),
        CheckState::Stale => ("stale pass", t.btn, format!("{exit} on older code")),
        CheckState::Never => ("not checked", t.violet(), "never for this repo".into()),
        CheckState::NeedsApproval => ("needs you", t.blocked, "waits for your approval".into()),
        CheckState::Checking => ("checking", t.sky(), format!("running {}", age(info.started_at))),
        CheckState::Pending => ("next", t.btn, "waits its turn".into()),
    };
    let ink = if fill == t.btn { t.strong } else { t.ink_on(fill) };
    (format!(" {g} {word} "), fill, ink, beside)
}

fn why_not_run(info: &CheckInfo) -> Option<&'static str> {
    match (info.state, info.exit_code) {
        (CheckState::CouldNotRun, Some(EXIT_NOT_FOUND)) => Some("command not found"),
        (CheckState::CouldNotRun, Some(EXIT_NOT_EXECUTABLE)) => Some("not executable"),
        _ => None,
    }
}

/// "14:22:05 · took 14.2s · after turn 3 ended 14:21:51".
fn ran_row(info: &CheckInfo, t: &Theme, c: Style) -> Vec<Seg> {
    let mut row = if info.started_at == 0 {
        vec![seg("did not run", c.fg(t.text))]
    } else if info.state == CheckState::Checking {
        vec![seg(crate::clock::local_hms(info.started_at), c.fg(t.text)), seg(" · running", c.fg(t.muted))]
    } else {
        vec![seg(crate::clock::local_hms(info.started_at), c.fg(t.text)), seg(" · took ", c.fg(t.muted)), seg(check::duration(info.duration_ms), c.fg(t.text))]
    };
    if info.after_turn_ended_at > 0 {
        let turn = if info.turn > 0 { format!("turn {}", info.turn) } else { "the turn".into() };
        row.push(seg(format!(" · after {turn} ended {}", crate::clock::local_hms(info.after_turn_ended_at)), c.fg(t.muted)));
    }
    row
}

/// "Seen by" for a command you set; "Source" (the repo file it came from) for the repo's own.
fn seen_row(info: &CheckInfo, said: &str, t: &Theme, c: Style) -> (&'static str, Vec<Seg>) {
    if let CheckSource::Repo { file, commit, uncommitted, .. } = &info.source {
        let mut parts = vec![file.clone()];
        match commit {
            Some(sha) => parts.push(sha.clone()),
            None if *uncommitted => parts.push("not committed".into()),
            None => {}
        }
        match info.state {
            CheckState::Never => parts.push("you chose never".into()),
            CheckState::NeedsApproval => parts.push("waits for your approval".into()),
            _ => {}
        }
        return ("Source", vec![seg(parts.join(" · "), c.fg(t.text))]);
    }
    let words = match said.lines().rev().map(str::trim).find(|l| !l.is_empty()) {
        Some(s) => format!("{APP_NAME} ran this itself. The agent said \"{s}\"."),
        None => concat!(env!("APP_NAME"), " ran this itself.").into(),
    };
    ("Seen by", vec![seg(words, c.fg(t.muted))])
}

/// The banner under the rows for a result that needs explaining: its tint and its rows, each
/// with the column it starts at (1: a headline, 3: under it). `files` is how many changed
/// files there is room to list.
fn banner(info: &CheckInfo, g: &str, files: usize, ground: Color, t: &Theme) -> Option<Banner> {
    let plain = mix(ground, t.text, BANNER_TINT);
    let violet = mix(ground, t.violet(), BANNER_TINT);
    let s = |bg: Color| Style::default().bg(bg);
    match info.state {
        CheckState::Overridden => {
            let by = if info.overridden_by.is_empty() { "you" } else { &info.overridden_by };
            let at = info.overridden_at.map(|a| format!(" {}", crate::clock::local_hm(a))).unwrap_or_default();
            let next = if info.override_reason.is_empty() { "next run replaces this".to_string() } else { format!("reason: {} · next run replaces this", info.override_reason) };
            let b = s(plain);
            Some((plain, vec![
                (1, vec![seg(format!("{g} Overridden by {by}{at}"), b.fg(t.strong).add_modifier(Modifier::BOLD)), seg("  · u undo", b.fg(t.muted))]),
                (3, vec![seg(next, b.fg(t.text))]),
            ]))
        }
        CheckState::Stale => {
            let b = s(plain);
            let n = info.changed_files.len();
            let when = info.stale_since.map(|a| format!(" ({})", crate::clock::local_hm(a))).unwrap_or_default();
            let what = match n {
                0 => format!(" The agent changed files after this ran{when}."),
                1 => format!(" The agent changed 1 file after this ran{when}."),
                n => format!(" The agent changed {n} files after this ran{when}."),
            };
            let mut rows = vec![
                (1, vec![seg(format!("{g} Stale."), b.fg(t.strong).add_modifier(Modifier::BOLD)), seg(what, b.fg(t.text))]),
                (3, vec![seg("This pass is for older code, not for what is there now.", b.fg(t.text))]),
            ];
            rows.extend(info.changed_files.iter().take(files).map(|f| (3, vec![seg(format!("{} ", f.kind), b.fg(t.muted)), seg(f.path.clone(), b.fg(t.text))])));
            Some((plain, rows))
        }
        CheckState::Never => {
            let b = s(violet);
            Some((violet, vec![
                (1, vec![seg(format!("{g} Not checked."), b.fg(t.violet()).add_modifier(Modifier::BOLD)), seg(" You chose never for this repo's command.", b.fg(t.text))]),
                (3, vec![seg("Done means the agent said so. Change it in Settings › Check.", b.fg(t.muted))]),
            ]))
        }
        CheckState::CouldNotRun => {
            let b = s(violet);
            let program = info.command.split_whitespace().next().unwrap_or_default();
            let fix = if info.exit_code == Some(EXIT_NOT_FOUND) && !program.is_empty() {
                format!("Fix the command in settings (,) or install {program}, then r.")
            } else {
                "Fix the command in settings (,), then r.".into()
            };
            Some((violet, vec![
                (1, vec![seg(format!("{g} Not a test failure."), b.fg(t.violet()).add_modifier(Modifier::BOLD)), seg(" The work hasn't been checked.", b.fg(t.text))]),
                (3, vec![seg(fix, b.fg(t.muted))]),
            ]))
        }
        _ => None,
    }
}

/// A line of output in the colour of what it says.
fn line_color(line: &str, t: &Theme) -> Color {
    match classify(line) {
        LineKind::Pass => t.done,
        LineKind::Failure => t.err,
        LineKind::Plain => t.text,
    }
}

/// "all 13 lines", "last 10 of 1,284 lines", "empty": how much of the output shows.
fn output_note(v: &CheckView, info: &CheckInfo, rows: usize) -> String {
    let held = v.held();
    let total = held.max(info.line_count as usize);
    match &v.output {
        None => "reading…".into(),
        Some(_) if held == 0 && info.state != CheckState::Never => "empty".into(),
        Some(_) if v.full && held > rows => format!("lines {}–{} of {}", v.start() + 1, v.start() + rows.min(held), thousands(total)),
        Some(_) if total > rows.min(held) => format!("last {} of {} lines", rows.min(held), thousands(total)),
        Some(_) => format!("all {held} lines"),
    }
}

/// What the block says when there is no output.
fn empty_words(info: &CheckInfo) -> (&'static str, String) {
    match info.state {
        CheckState::Never => ("(nothing ran)", concat!("There is no output because ", env!("APP_NAME"), " did not run it.").into()),
        CheckState::Passed | CheckState::Stale => ("(the command printed nothing)", "exit 0 is what passed it.".into()),
        _ => match info.exit_code {
            Some(code) => ("(no output)", format!("It exited {code} without printing. Run it in the session to see more.")),
            None => ("(no output)", "Run it in the session to see more.".into()),
        },
    }
}

/// The output on a darker ground, numbered as the whole output is, with ▲ ▼ where there is more.
fn output_block(buf: &mut Buffer, r: Rect, v: &CheckView, info: &CheckInfo, ground: Color, t: &Theme) {
    let shade = mix(ground, Color::Rgb(0, 0, 0), BLOCK_SHADE);
    fill(buf, r, shade);
    let c = Style::default().bg(shade);
    let Some(output) = &v.output else { return };
    if v.held() == 0 {
        let (head, more) = empty_words(info);
        put(buf, r.x + 2, r.y + 1, &[seg(head, c.fg(t.muted).add_modifier(Modifier::ITALIC))], r.right());
        put(buf, r.x + 2, r.y + 2, &[seg(truncate(&more, r.width.saturating_sub(3) as usize), c.fg(t.muted))], r.right());
        return;
    }
    let start = v.start();
    let first_number = (info.line_count as usize).saturating_sub(v.held()) + 1;
    let text_w = (r.width as usize).saturating_sub(NUM_W + 1);
    for (i, line) in output.lines().enumerate().skip(start).take(r.height as usize) {
        let line = clean(line);
        let num = seg(format!("{:<NUM_W$}", first_number + i), c.fg(mix(t.muted, shade, NUMBER_FADE)));
        put(buf, r.x, r.y + (i - start) as u16, &[num, seg(truncate(&line, text_w), c.fg(line_color(&line, t)))], r.right());
    }
    let mark = |buf: &mut Buffer, y: u16, s: &str| put(buf, r.right().saturating_sub(1), y, &[seg(s, c.fg(t.muted))], r.right());
    // More above: lines scrolled past, or ones the daemon did not keep.
    if (v.full && start > 0) || first_number > 1 {
        mark(buf, r.y, "▲");
    }
    if start + (r.height as usize) < v.held() {
        mark(buf, r.bottom().saturating_sub(1), "▼");
    }
}

/// A button on the sheet: " Label key " on its fill. Returns its rect.
pub(super) fn sheet_button(buf: &mut Buffer, x: u16, y: u16, max: u16, (label, key): (&str, &str), kind: BtnKind, t: &Theme) -> Option<Rect> {
    let (bg, fg, kf) = match kind {
        BtnKind::Primary => (t.accent, t.acc_ink, t.acc_ink),
        BtnKind::Danger | BtnKind::Risky => (t.btn, t.err, t.accent),
        _ => (t.btn, t.strong, t.accent),
    };
    let b = Style::default().bg(bg).add_modifier(Modifier::BOLD);
    let segs = [seg(format!(" {label} "), b.fg(fg)), seg(format!("{key} "), b.fg(kf))];
    let w = segs_width(&segs);
    if x + w > max {
        return None;
    }
    put(buf, x, y, &segs, max);
    Some(Rect { x, y, width: w, height: 1 })
}

/// The buttons for what can be done with this result; each stands for the key on it.
fn buttons(app: &mut App, buf: &mut Buffer, (x, y, max): (u16, u16, u16), info: &CheckInfo, t: &Theme) {
    let mut list: Vec<(&str, char, BtnKind)> = Vec::new();
    match info.state {
        CheckState::Never => list.push(("Check settings", ',', BtnKind::Normal)),
        CheckState::Checking => {}
        CheckState::Stale => list.push(("Re-run", 'r', BtnKind::Primary)),
        _ => list.push(("Re-run", 'r', BtnKind::Normal)),
    }
    match info.state {
        CheckState::Failed => list.push(("Override…", 'O', BtnKind::Danger)),
        CheckState::Overridden => list.push(("Undo override", 'u', BtnKind::Normal)),
        _ => {}
    }
    let mut at = x;
    for (label, key, kind) in list {
        let Some(r) = sheet_button(buf, at, y, max, (label, &key.to_string()), kind, t) else { return };
        app.hits.push((r, crate::client::Hit::Button(Btn::ViewKey(key))));
        at = r.right() + 1;
    }
    if at + OPEN_SESSION_ROOM < max
        && let Some(r) = sheet_button(buf, at, y, max, ("Open session", "o"), BtnKind::Normal, t)
    {
        app.hits.push((r, crate::client::Hit::Button(Btn::ViewKey('o'))));
    }
}

/// The sheet's inside, on `ground`: the result, where and when it ran, a banner when it needs
/// explaining, the output, then the buttons and keys on the last rows.
pub(in crate::client) fn draw_check(app: &mut App, buf: &mut Buffer, inside: Rect, ground: Color, t: &Theme, v: &mut CheckView) {
    let Some(term) = app.snap.terms.get(&v.term).cloned() else { return };
    let Some(info) = shown(&term, &v.name).cloned() else { return };
    let (x, right) = (inside.x + 1, inside.right().saturating_sub(1));
    let c = Style::default().bg(ground);
    let mut y = inside.y + 1;
    let (words, fill_c, ink, beside) = chip(app, &info, t);
    let mut row = vec![seg(words, Style::default().bg(fill_c).fg(ink).add_modifier(Modifier::BOLD)), seg(format!("  {beside}"), c.fg(t.strong).add_modifier(Modifier::BOLD))];
    if let Some(why) = why_not_run(&info) {
        row.push(seg(format!("  {why}"), c.fg(t.violet())));
    }
    if info.rerun_queued {
        row.push(seg("  · re-run queued", c.fg(t.muted)));
    }
    put(buf, x, y, &row, right);
    y += 2;
    let folder = if info.cwd.as_os_str().is_empty() { term.top.clone().unwrap_or_else(|| term.cwd.clone()) } else { info.cwd.clone() };
    let (seen_label, seen) = seen_row(&info, &term.said, t, c);
    let rows = [
        ("Command", vec![seg(info.command.clone(), c.fg(t.strong).add_modifier(Modifier::BOLD))]),
        ("Folder", vec![seg(tilde(&folder), c.fg(t.text))]),
        ("Ran", ran_row(&info, t, c)),
        (seen_label, seen),
    ];
    for (label, value) in rows {
        let mut line = vec![seg(format!("{label:<LABEL_W$}"), c.fg(t.muted))];
        line.extend(value);
        put(buf, x, y, &line, right);
        y += 1;
    }
    y += 1;
    let bottom = inside.bottom();
    // Banner rows beyond its two, as many as leave the output its smallest block.
    let files = bottom.saturating_sub(y + 2 + 1 + 1 + MIN_BLOCK_ROWS + FOOT_ROWS) as usize;
    if let Some((tint, lines)) = banner(&info, check::glyph(app, &info), files, ground, t) {
        fill(buf, Rect { x, y, width: right.saturating_sub(x), height: lines.len() as u16 }, tint);
        for (at, line) in lines {
            put(buf, x + at, y, &line, right);
            y += 1;
        }
        y += 1;
    }
    let room = bottom.saturating_sub(y + 1 + FOOT_ROWS).max(MIN_BLOCK_ROWS);
    v.room = room as usize;
    let head = Style::default().bg(t.btn).fg(t.strong).add_modifier(Modifier::BOLD);
    put(buf, x, y, &[seg(" OUTPUT ", head)], right);
    let note = seg(output_note(v, &info, room as usize), c.fg(t.muted));
    let end = put(buf, x + OUTPUT_NOTE_AT, y, &[note], right);
    if (info.line_count as usize) > v.held() && v.output.is_some() {
        let more = [seg("  f full output", c.fg(t.muted))];
        let after = put(buf, end, y, &more, right);
        app.hits.push((Rect { x: end + 2, y, width: after.saturating_sub(end + 2), height: 1 }, crate::client::Hit::Button(Btn::ViewKey('f'))));
    }
    y += 1;
    let block_h = room.min((v.held() as u16).max(MIN_BLOCK_ROWS));
    output_block(buf, Rect { x, y, width: right.saturating_sub(x), height: block_h }, v, &info, ground, t);
    buttons(app, buf, (x, bottom.saturating_sub(2), right), &info, t);
    action_bar(buf, Rect { y: bottom.saturating_sub(1), height: 1, ..inside }, t, t.card, &HINTS);
}

#[cfg(test)]
mod tests {
    use super::super::v7_ref::{Ref, assert_same, layout_name, reference, text};
    use super::*;
    use crate::client::v7_fixture::{LAYOUTS, SIZES, Stage, world};

    /// The handoff's output of a failing run (`OUT_FAIL`) and a passing one (`OUT_PASS`).
    const OUT_FAIL: &str = "> affiliate-site@1.4.0 test\n> vitest run\n\n ✓ src/components/Footer.test.tsx (3)\n ✓ src/lib/price.test.ts (9)\n ✗ src/pages/legal.test.tsx > footer shows current year\n   AssertionError: expected '© 2025 Affiliate Co.'\n                   to equal '© 2026 Affiliate Co.'\n    ❯ src/pages/legal.test.tsx:18:32\n\n Test Files  1 failed | 11 passed (12)\n      Tests  1 failed | 58 passed (59)\n   Duration  14.21s";
    const OUT_PASS: &str = "> vitest run\n ✓ 12 test files · 59 tests\n   Duration  14.02s";
    /// The full output's length in 08: the daemon kept only its tail.
    const LONG: u64 = 1284;

    /// The world at `stage` with fix-footer's sheet open on `output`, and what the agent said.
    fn sheet_world(stage: Stage, output: &str, line_count: Option<u64>) -> App {
        let mut app = world(stage);
        app.cfg.ui.glyphs = "unicode".into();
        let term = app.snap.terms.get_mut(&1).unwrap();
        term.said = "tests pass".into();
        if let Some(n) = line_count {
            term.checks[0].line_count = n;
        }
        app.open_check_sheet(1);
        app.check_output_arrived(output.into());
        app
    }

    /// The sheet's inside drawn where the reference has it.
    fn draw(app: &mut App, area: Rect, w: u16, h: u16) -> (Vec<String>, Buffer) {
        let t = Theme::named(crate::theme::DEFAULT);
        let mut buf = Buffer::empty(Rect { x: 0, y: 0, width: w, height: h });
        let Some(View::Check(mut v)) = app.view.take() else { panic!("the sheet is open") };
        draw_check(app, &mut buf, area, t.bg, &t, &mut v);
        app.view = Some(View::Check(v));
        let mut rows = text(&buf, area);
        // The handoff's sample folder is a Windows path.
        for row in rows.iter_mut().filter(|r| r.starts_with(" Folder")) {
            *row = row.replace('/', "\\");
        }
        (rows, buf)
    }

    fn matches_reference(preset: &str, stage: Stage, output: &str, line_count: Option<u64>, fix: fn(&mut Ref)) {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut want = reference(preset, layout_name(layout), w, h);
                fix(&mut want);
                let mut app = sheet_world(stage, output, line_count);
                let (got, _) = draw(&mut app, want.area, w, h);
                assert_same(&got, &want, &format!("{preset} {layout:?} {w}x{h}"));
            }
        }
    }

    #[test]
    fn a_short_failing_output_reads_as_the_handoff_draws_it() {
        matches_reference("07 Evidence · short output", Stage::Failed, OUT_FAIL, None, |_| {});
    }

    #[test]
    fn a_long_output_shows_its_tail_numbered_as_the_whole() {
        matches_reference("08 Evidence · long output", Stage::Failed, OUT_FAIL, Some(LONG), |_| {});
    }

    #[test]
    fn a_failure_that_printed_nothing_says_so() {
        matches_reference("09 Evidence · no output", Stage::Failed, "", Some(0), |_| {});
    }

    #[test]
    fn a_pass_that_printed_nothing_says_exit_0_passed_it() {
        matches_reference("10 Evidence · printed nothing", Stage::Passed, "", Some(0), |_| {});
    }

    /// The fixture's never-run check has no run time, and says which commit the repo file is
    /// at; the handoff's sample shows a run and the time you chose.
    #[test]
    fn a_check_you_chose_never_to_run_explains_itself() {
        matches_reference("A5 Never for this repo", Stage::Never, "", Some(0), |want| {
            want.set_row(5, " Ran      did not run · after turn 3 ended 14:21:51");
            want.set_row(6, concat!(" Source   .", env!("APP_COMMAND"), "/checks.toml · a1b2c3d · you chose never"));
        });
    }

    #[test]
    fn a_stale_pass_lists_what_changed_and_asks_for_a_re_run() {
        matches_reference("B2 Stale · evidence", Stage::Stale, OUT_PASS, None, |_| {});
    }

    #[test]
    fn a_stale_pass_offers_re_run_first_and_lit() {
        let mut app = sheet_world(Stage::Stale, OUT_PASS, None);
        let area = reference("B2 Stale · evidence", "floating", 160, 45).area;
        let (rows, buf) = draw(&mut app, area, 160, 45);
        let y = area.bottom() - 2;
        assert!(rows[(y - area.y) as usize].starts_with("  Re-run r   Open session o"), "{rows:#?}");
        let t = Theme::named(crate::theme::DEFAULT);
        assert_eq!(buf[(area.x + 2, y)].bg, t.accent, "Re-run is the primary button");
    }

    #[test]
    fn f_shows_all_of_the_output_and_the_arrows_scroll_it() {
        let mut app = sheet_world(Stage::Failed, OUT_FAIL, Some(LONG));
        let area = reference("08 Evidence · long output", "floating", 80, 24).area;
        let (rows, _) = draw(&mut app, area, 80, 24);
        assert!(rows[9].starts_with(" 1275  ✓"), "the tail first: {rows:#?}");
        app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        let (rows, _) = draw(&mut app, area, 80, 24);
        assert!(rows[9].starts_with(" 1272 > affiliate-site@1.4.0 test"), "from the first line it has: {rows:#?}");
        assert!(rows[8].contains("lines 1–10 of 1,284"), "{:?}", rows[8]);
        app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let (rows, _) = draw(&mut app, area, 80, 24);
        assert!(rows[9].starts_with(" 1273 > vitest run"), "{rows:#?}");
    }

    #[test]
    fn tab_moves_to_the_next_checkpoint_and_the_title_names_it() {
        let mut app = world(Stage::Multi);
        app.open_check_sheet(1);
        assert_eq!(check_title(&app), "· fix-footer › test", "it opens on the headline: the failed one");
        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(check_title(&app), "· fix-footer › build");
        app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(check_title(&app), "· fix-footer › test");
        for layout in ["flat", "cards"] {
            app.cfg.ui.layout = layout.into();
            app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
            assert!(matches!(app.view, Some(View::Check(_))) && app.mode == Mode::Normal, "{layout}: Tab stays in the sheet");
            app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        }
    }

    #[test]
    fn the_sheet_s_keys_act_on_the_checkpoint_it_shows() {
        use crate::client::menu::Act;
        use crate::protocol::Command;
        let mut app = world(Stage::Multi);
        let (tx, mut rx) = crate::client::mpsc::unbounded_channel();
        app.out = tx;
        app.open_check_sheet(1);
        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        app.on_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        let reruns: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|m| match m {
                crate::client::ClientMsg::Command(Command::RerunCheck { term, name }) => Some((term, name)),
                _ => None,
            })
            .collect();
        assert_eq!(reruns, [(1, Some("build".to_string()))]);
        app.on_key(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::NONE));
        assert!(!matches!(app.mode, Mode::Confirm(_)), "build passed: nothing to override");
        app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        app.on_key(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::NONE));
        assert!(matches!(&app.mode, Mode::Confirm(c) if matches!(&c.act, Act::OverrideCheck(1, _, Some(n)) if n == "test")), "{:?}", app.mode);
    }

    #[test]
    fn the_sheet_s_keys_reach_the_session_its_why_and_its_settings() {
        let mut app = sheet_world(Stage::Failed, OUT_FAIL, None);
        app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Why { term: 1, side: false });
        let mut app = sheet_world(Stage::Never, "", Some(0));
        app.on_key(KeyEvent::new(KeyCode::Char(','), KeyModifiers::NONE));
        assert!(matches!(&app.mode, Mode::ProjectCheck(_)), ", opens PROJECT · <name>");
        let mut app = sheet_world(Stage::Failed, OUT_FAIL, None);
        app.on_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        assert!(app.view.is_none(), "o leaves the sheet for the session");
    }

    #[test]
    fn a_number_reads_with_thousands_separators() {
        assert_eq!([thousands(13), thousands(1284), thousands(1_000_000)], ["13", "1,284", "1,000,000"]);
    }
}
