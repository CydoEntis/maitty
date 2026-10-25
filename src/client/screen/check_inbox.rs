//! What an Inbox item says: which sessions a check puts in which group, and each item's
//! glyph, words and buttons. A needs-you question always sorts above a check, and a failure
//! never reads as just finished.
use super::glyphs::{self, Glyphs};
use super::*;
use crate::protocol::CheckState;

pub(super) const NEEDS_HEAD: &str = "NEEDS YOU";
pub(super) const FAILED_HEAD: &str = "FAILED CHECKS";
pub(super) const NOT_RUN_HEAD: &str = "COULD NOT RUN";
pub(super) const FINISHED_HEAD: &str = "FINISHED";
const EXIT_NOT_EXECUTABLE: i32 = 126;
const EXIT_NOT_FOUND: i32 = 127;

/// Where a session's check puts it in the Inbox, if anywhere. A failure stays until a later
/// run replaces it, even after you have looked; unchecked work is listed only until you look.
pub(super) fn row(s: &Session) -> Option<GoRow> {
    let info = check::shown(s)?;
    match info.state {
        CheckState::Failed => Some(GoRow::Failed(s.term)),
        CheckState::NeedsApproval => Some(GoRow::Approve(s.term)),
        CheckState::CouldNotRun if s.status == Status::Done => Some(GoRow::NotRun(s.term)),
        _ => None,
    }
}

/// The rows of one group, the longest waiting first like the others.
pub(super) fn rows(model: &[Proj], want: fn(&GoRow) -> bool) -> Vec<GoRow> {
    let mut found: Vec<(u64, GoRow)> = model.iter().flat_map(Proj::sessions).filter_map(|s| row(s).filter(want).map(|r| (s.since, r))).collect();
    found.sort_by_key(|(since, row)| (*since, row.term()));
    found.into_iter().map(|(_, row)| row).collect()
}

/// Whether a check key does something for this result: `r` re-run, `O` override, `u` undo
/// it, `,` its settings.
pub(super) fn check_key_acts(key: char, info: &CheckInfo) -> bool {
    match key {
        'r' => check::settled(info),
        'O' => check::failed(info),
        'u' => info.state == CheckState::Overridden,
        ',' => true,
        _ => false,
    }
}

/// A run of text on an item's line: its words, colour and weight.
pub(super) type Words = (String, Color, bool);

/// One button under a selected item: its label, the key shown on it, the key it presses.
pub(super) type Button = (&'static str, &'static str, char);

const OUTPUT: Button = ("Output", "⏎", '\n');
const RERUN: Button = ("Re-run", "r", 'r');
const SEND: Button = ("Send to agent", "m", 'm');
pub(super) const OVERRIDE: Button = ("Override…", "O", 'O');

/// An Inbox item as drawn: its head line, the line under it, a third for a failure, and the
/// buttons it grows when selected (the first is what Enter does).
pub(super) struct Item {
    pub glyph: &'static str,
    pub color: Color,
    pub name: String,
    pub folder: String,
    pub age: String,
    pub line: Vec<Words>,
    /// Several checkpoints: the line in full, and the glyph strip when that doesn't fit.
    pub strip: Option<Vec<Words>>,
    pub third: Option<String>,
    pub buttons: Vec<Button>,
    pub failed: bool,
}

/// What a checkpoint is called on a line: its name, else its command.
fn label(info: &CheckInfo) -> &str {
    if info.name.is_empty() { &info.command } else { &info.name }
}

/// One checkpoint's glyph and colour in a list of them.
fn checkpoint_mark(g: &Glyphs, state: CheckState, t: &Theme) -> (&'static str, Color) {
    match state {
        CheckState::Passed => (g.done, t.done),
        CheckState::Failed => (g.failed, t.err),
        CheckState::Checking => (g.ring[0], t.sky()),
        CheckState::CouldNotRun | CheckState::Never => (g.couldnt, t.violet()),
        CheckState::Stale => (g.stale, t.muted),
        CheckState::Overridden => (g.over, t.text),
        CheckState::NeedsApproval => (g.needs, t.blocked),
        CheckState::Pending => (g.pend, t.muted),
    }
}

/// Several checkpoints read as one word first: what failed, then what needs you, then what
/// runs next, else that all passed.
fn checkpoints_word(checks: &[CheckInfo], t: &Theme) -> Words {
    let find = |states: &[CheckState]| checks.iter().find(|c| states.contains(&c.state));
    if let Some(c) = find(&[CheckState::Failed, CheckState::CouldNotRun]) {
        let color = if c.state == CheckState::Failed { t.err } else { t.violet() };
        return (format!("failed: {}", label(c)), color, true);
    }
    if let Some(c) = find(&[CheckState::NeedsApproval]) {
        return (format!("needs you: {}", label(c)), t.blocked, true);
    }
    if let Some(c) = find(&[CheckState::Checking]) {
        return (format!("checking: {}", label(c)), t.sky(), false);
    }
    if let Some(c) = find(&[CheckState::Pending]) {
        return (format!("next: {}", label(c)), t.muted, false);
    }
    ("all passed".into(), t.done, true)
}

/// "failed: test · build ✓ · test ✕", and "failed: test ✓✕" for when that doesn't fit.
fn checkpoints_lines(g: &Glyphs, checks: &[CheckInfo], t: &Theme) -> (Vec<Words>, Vec<Words>) {
    let word = checkpoints_word(checks, t);
    let mut full = vec![word.clone()];
    for c in checks {
        let (glyph, color) = checkpoint_mark(g, c.state, t);
        full.push((" · ".into(), t.muted, false));
        full.push((format!("{} ", label(c)), if c.state == CheckState::Failed { t.err } else { t.text }, false));
        full.push((glyph.into(), color, true));
    }
    let mut strip = vec![(format!("{} ", word.0), word.1, word.2)];
    strip.extend(checks.iter().map(|c| {
        let (glyph, color) = checkpoint_mark(g, c.state, t);
        (glyph.to_string(), color, true)
    }));
    (full, strip)
}

/// "test could not run: pnpm not found".
fn not_run_words(info: &CheckInfo) -> String {
    let program = info.command.split_whitespace().next().unwrap_or_default();
    let why = match info.exit_code {
        Some(EXIT_NOT_FOUND) => format!(": {program} not found"),
        Some(EXIT_NOT_EXECUTABLE) => format!(": {program} not executable"),
        Some(code) => format!(" · exit {code}"),
        None => String::new(),
    };
    format!("{} could not run{why}", label(info))
}

/// The line under a single check's item.
fn check_line(info: &CheckInfo, t: &Theme) -> Vec<Words> {
    let name = label(info);
    match info.state {
        CheckState::NeedsApproval => vec![(format!("approve check command: {}", info.command), t.blocked, false)],
        CheckState::Failed => {
            let exit = info.exit_code.map(|c| format!(" · exit {c}")).unwrap_or_default();
            let at = crate::clock::local_hm(info.started_at + info.duration_ms / 1000);
            vec![(format!("failed: {name}{exit} · {at}"), t.err, true)]
        }
        CheckState::CouldNotRun => vec![(not_run_words(info), t.violet(), false)],
        CheckState::Passed => vec![(format!("passed: {name}"), t.done, true), (format!(" · {}", check::duration(info.duration_ms)), t.muted, false)],
        CheckState::Overridden => vec![(format!("failed, overridden: {name}"), t.text, true)],
        CheckState::Stale => {
            let n = info.changed_files.len();
            let files = if n == 1 { "1 file".to_string() } else { format!("{n} files") };
            vec![(format!("stale pass: {name}"), t.text, true), (format!(" · {files} changed since"), t.muted, false)]
        }
        CheckState::Never => vec![("not checked: never for this repo".into(), t.violet(), false)],
        CheckState::Checking => vec![(format!("checking {name}"), t.sky(), false)],
        CheckState::Pending => vec![(format!("next: {name}"), t.muted, false)],
    }
}

/// The first failing line, marked as one (`x` in plain ASCII).
fn failing_line(info: &CheckInfo, g: &Glyphs) -> Option<String> {
    let line = info.first_failing_line.trim();
    if line.is_empty() {
        return None;
    }
    let mark = if *g == glyphs::ASCII { g.failed } else { "✗" };
    Some(if line.starts_with(['✗', '✕', '✘', '×']) { line.to_string() } else { format!("{mark} {line}") })
}

fn buttons(row: &GoRow, info: Option<&CheckInfo>) -> Vec<Button> {
    match row {
        GoRow::Ask(_) => vec![("Reply", "m", 'm'), ("Open", "o", 'o')],
        GoRow::Approve(_) => vec![("Read the command", "⏎", '\n')],
        GoRow::Failed(_) => vec![OUTPUT, RERUN, OVERRIDE, SEND],
        GoRow::NotRun(_) => vec![OUTPUT, ("Settings", ",", ',')],
        GoRow::Done(_) if info.is_some_and(|i| i.state == CheckState::Stale) => vec![RERUN, OUTPUT],
        _ => Vec::new(),
    }
}

/// The item for an Inbox row of a session, or None for a heading, a heads-up or a search hit.
pub(super) fn item(app: &App, model: &[Proj], row: &GoRow, t: &Theme) -> Option<Item> {
    if matches!(row, GoRow::Head(_) | GoRow::Heads(_) | GoRow::Sess(..)) {
        return None;
    }
    let (p, _, s) = find(model, row.term()?)?;
    let g = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let info = check::shown(s);
    let mark = glyphs::Kind::of(s.status, info, false).mark(g, t, 0, "");
    let checks = app.snap.terms.get(&s.term).map(|i| i.checks.clone()).unwrap_or_default();
    let (line, strip) = match (row, info) {
        (GoRow::Ask(_), _) => (vec![(s.question.clone().unwrap_or_else(|| "waiting on you".into()), t.blocked, false)], None),
        (GoRow::Quiet(_), _) => (vec![(format!("no output for {} · may be stuck", age(s.quiet_since.unwrap_or(s.since))), t.muted, false)], None),
        (_, Some(_)) if checks.len() > 1 => {
            let (full, strip) = checkpoints_lines(g, &checks, t);
            (full, Some(strip))
        }
        (_, Some(info)) => (check_line(info, t), None),
        (_, None) => (vec![("finished · not seen".into(), t.muted, false)], None),
    };
    let failed = matches!(row, GoRow::Failed(_));
    Some(Item {
        glyph: mark.glyph,
        color: mark.color,
        name: s.name.clone(),
        folder: p.name.clone(),
        age: age(s.since),
        line,
        strip,
        third: info.filter(|_| failed).and_then(|i| failing_line(i, g)),
        buttons: buttons(row, info),
        failed,
    })
}
