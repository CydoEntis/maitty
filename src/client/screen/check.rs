//! A session's check result on its sidebar row: the glyph that stands for it, its colour,
//! and the line of words under the row. Shape and word carry the meaning; colour backs them.
use super::*;
use super::glyphs::{self, Glyphs, Kind, Mark};
use crate::protocol::{CheckInfo, CheckState};

/// The ring steps once per this many spinner frames.
const RING_TICKS: u64 = 2;
const RING_FRAMES: usize = 4;
const EXIT_NOT_EXECUTABLE: i32 = 126;
const EXIT_NOT_FOUND: i32 = 127;
/// Below this the command is not worth showing, so the whole line is cut at its end instead.
const MIN_COMMAND_COLS: usize = 2;
const SECS_PER_MIN: u64 = 60;

/// The set `[ui] glyphs` picks; plain ASCII icons get ASCII marks unless it says otherwise.
pub(super) fn marks(app: &App) -> &'static Glyphs {
    glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons)
}

/// The result to show for a session. While the agent works or asks, the last result is
/// about an older turn, so the row says only what the agent is doing.
pub(super) fn shown(s: &Session) -> Option<&CheckInfo> {
    crate::protocol::reported_check(s.status, s.check.as_ref())
}

/// Every checkpoint the row reports: none while the agent works or asks.
pub(super) fn reported(s: &Session) -> &[CheckInfo] {
    if shown(s).is_some() { &s.checks } else { &[] }
}

/// A turn that ended and whose work is not known to be bad: no check, or one that passed, went
/// stale, was overridden or is never run here. A running, failed or could-not-run check, or one
/// waiting for approval, keeps it from counting as finished.
pub(super) fn finished(s: &Session) -> bool {
    s.status == Status::Done
        && shown(s).is_none_or(|info| matches!(info.state, CheckState::Passed | CheckState::Overridden | CheckState::Stale | CheckState::Never))
}

/// It needs you: it asks, or its repo's check command waits for your approval.
pub(super) fn needs_you(s: &Session) -> bool {
    s.status == Status::Blocked || shown(s).is_some_and(|info| info.state == CheckState::NeedsApproval)
}

/// What the row reads as.
pub(super) fn kind(s: &Session) -> Kind {
    Kind::of(s.status, s.check.as_ref(), s.asleep)
}

/// The row's mark: its glyph from the chosen set (the ring turning with motion, `working` for
/// the working spinner), the colour that backs it and its word.
pub(super) fn mark<'a>(app: &App, s: &Session, t: &Theme, working: &'a str) -> Mark<'a> {
    let step = if app.motion_on() { ring_step(app.spinner_frame(), RING_FRAMES) } else { 0 };
    kind(s).mark(marks(app), t, step, working)
}

/// The mark that stands for a failed check, in the same set as the sidebar's glyphs.
pub(super) fn failed_mark(app: &App) -> &'static str {
    marks(app).failed
}

pub(super) fn failed(info: &CheckInfo) -> bool {
    info.state == CheckState::Failed
}

/// A result that has come in, so it can be run again.
pub(super) fn settled(info: &CheckInfo) -> bool {
    info.state != CheckState::Checking
}

/// Which ring glyph a spinner frame shows. Apart from the clock so a test can step it.
pub(super) fn ring_step(frame: u64, len: usize) -> usize {
    (frame / RING_TICKS % len as u64) as usize
}

pub(super) fn glyph(app: &App, info: &CheckInfo) -> &'static str {
    let m = marks(app);
    match info.state {
        CheckState::Checking => m.ring[if app.motion_on() { ring_step(app.spinner_frame(), m.ring.len()) } else { 0 }],
        CheckState::Passed => m.done,
        CheckState::Failed => m.failed,
        CheckState::Overridden => m.over,
        CheckState::CouldNotRun | CheckState::Never => m.couldnt,
        CheckState::Stale => m.stale,
        CheckState::NeedsApproval => m.needs,
        CheckState::Pending => m.pend,
    }
}

/// An override is never green and could-not-run is not a failure, so neither borrows those colours.
pub(super) fn color(t: &Theme, info: &CheckInfo) -> Color {
    match info.state {
        CheckState::Checking => t.sky(),
        CheckState::Passed => t.done,
        CheckState::Failed => t.err,
        CheckState::Overridden | CheckState::Stale => t.text,
        CheckState::CouldNotRun | CheckState::Never => t.violet(),
        CheckState::Pending => t.muted,
        CheckState::NeedsApproval => t.blocked,
    }
}

/// A checkpoint's glyph in the strip of several: as on the row, but a stale pass is muted.
fn strip_color(t: &Theme, info: &CheckInfo) -> Color {
    if info.state == CheckState::Stale { t.muted } else { color(t, info) }
}

fn styled(text: impl Into<String>, fg: Color, bold: bool) -> Seg {
    let st = Style::default().fg(fg);
    seg(text, if bold { st.add_modifier(Modifier::BOLD) } else { st })
}

/// What a checkpoint is called: its name, or for a result saved before checkpoints had names,
/// its command's last word (as `CheckDef::name` does).
pub(super) fn name(info: &CheckInfo) -> &str {
    if info.name.is_empty() { info.command.split_whitespace().last().unwrap_or_default() } else { &info.name }
}

/// The line under a row with one checkpoint, named by its checkpoint name (the command, while
/// it waits for approval, since that is what you approve).
pub(super) fn result_words(info: &CheckInfo, t: &Theme) -> Vec<Seg> {
    let name = name(info);
    match info.state {
        CheckState::Checking => vec![styled(format!("checking {name} · {}", age(info.started_at)), t.sky(), false)],
        CheckState::Pending => vec![styled(format!("next: {name}"), t.muted, false)],
        CheckState::Passed => vec![
            styled(format!("passed: {name}"), t.done, true),
            styled(format!(" · {} · {}", duration(info.duration_ms), crate::clock::local_hm(info.started_at + info.duration_ms / 1000)), t.muted, false),
        ],
        CheckState::Failed => match info.exit_code {
            Some(code) => vec![styled(format!("failed: {name} · exit {code}"), t.err, true)],
            None => vec![styled(format!("failed: {name}"), t.err, true)],
        },
        CheckState::Overridden => vec![styled(format!("failed, overridden: {name}"), t.text, true)],
        CheckState::CouldNotRun => vec![styled(format!("{name} could not run{}", why_not_run(info.exit_code)), t.violet(), false)],
        CheckState::Stale => {
            let n = info.changed_files.len();
            vec![styled(format!("stale pass: {name}"), t.text, true), styled(format!(" · {n} file{} changed", if n == 1 { "" } else { "s" }), t.muted, false)]
        }
        CheckState::Never => vec![styled("not checked: never for this repo", t.violet(), false)],
        CheckState::NeedsApproval => vec![styled(format!("approve check: {}", info.command), t.blocked, false)],
    }
}

/// The word that heads a row of several checkpoints, from the headline one: the worst wins.
pub(super) fn headline_word(checks: &[CheckInfo], t: &Theme) -> Seg {
    let Some(h) = crate::protocol::headline(checks) else { return styled("", t.muted, false) };
    let name = name(h);
    match h.state {
        CheckState::Failed => styled(format!("failed: {name} "), t.err, true),
        CheckState::CouldNotRun => styled(format!("could not run: {name} "), t.violet(), false),
        CheckState::NeedsApproval => styled(format!("needs you: {name} "), t.blocked, true),
        CheckState::Checking => styled(format!("checking: {name} "), t.sky(), false),
        CheckState::Pending => styled(format!("next: {name} "), t.muted, false),
        CheckState::Stale => styled(format!("stale pass: {name} "), t.text, true),
        CheckState::Never => styled(format!("not checked: {name} "), t.violet(), false),
        CheckState::Overridden => styled(format!("overridden: {name} "), t.text, true),
        CheckState::Passed => styled("all passed ", t.done, true),
    }
}

/// One bold glyph per checkpoint, in order.
pub(super) fn strip(app: &App, checks: &[CheckInfo], t: &Theme) -> Vec<Seg> {
    checks.iter().map(|c| styled(glyph(app, c), strip_color(t, c), true)).collect()
}

/// "build ✓ · test ✕": every checkpoint by name, a failed one's name in the failure colour.
pub(super) fn names(app: &App, checks: &[CheckInfo], t: &Theme) -> Vec<Seg> {
    let mut out = Vec::new();
    for (i, c) in checks.iter().enumerate() {
        if i > 0 {
            out.push(styled(" · ", t.muted, false));
        }
        out.push(styled(format!("{} ", name(c)), if failed(c) { t.err } else { t.text }, false));
        out.push(styled(glyph(app, c), strip_color(t, c), true));
    }
    out
}

pub(super) fn duration(ms: u64) -> String {
    let secs = ms / 1000;
    if secs < SECS_PER_MIN { format!("{:.1}s", ms as f64 / 1000.0) } else { format!("{}m {}s", secs / SECS_PER_MIN, secs % SECS_PER_MIN) }
}

pub(super) fn why_not_run(exit: Option<i32>) -> String {
    match exit {
        Some(EXIT_NOT_FOUND) => " · not found".into(),
        Some(EXIT_NOT_EXECUTABLE) => " · not executable".into(),
        Some(code) => format!(" · exit {code}"),
        None => String::new(),
    }
}

/// The sub-line in `room` columns. A long command gives way first, so the result still reads.
pub(super) fn words(app: &App, info: &CheckInfo, room: usize) -> String {
    let full = compose(app, info, &info.command);
    if full.width() <= room {
        return full;
    }
    let fixed = compose(app, info, "").width();
    let cmd_room = room.saturating_sub(fixed);
    if cmd_room < MIN_COMMAND_COLS {
        return truncate(&full, room);
    }
    compose(app, info, &truncate(&info.command, cmd_room))
}

fn compose(app: &App, info: &CheckInfo, cmd: &str) -> String {
    let m = marks(app);
    match info.state {
        CheckState::Checking => format!("checking {cmd} · {}", age(info.started_at)),
        CheckState::Passed => format!("{cmd} {} · {} · {} ago", m.done, duration(info.duration_ms), age(info.started_at + info.duration_ms / 1000)),
        CheckState::Failed => match info.exit_code {
            Some(code) => format!("failed: {cmd} · exit {code}"),
            None => format!("failed: {cmd}"),
        },
        CheckState::Overridden => format!("{cmd} {} overridden by you", m.failed),
        CheckState::CouldNotRun => format!("{cmd} could not run{}", why_not_run(info.exit_code)),
        CheckState::Pending => format!("next: {cmd}"),
        CheckState::NeedsApproval => format!("approve check: {cmd}"),
        CheckState::Stale => format!("stale pass: {cmd}"),
        CheckState::Never => "not checked: never for this repo".into(),
    }
}

/// A button for a check action at (`x`, `y`) if it fits before `max`: its rect, with the
/// override label in the failure colour. The caller records what a click does.
pub(super) fn action_pill(app: &App, buf: &mut Buffer, x: u16, y: u16, max: u16, (label, key): (&str, char), t: &Theme) -> Option<Rect> {
    let look = Look::of(&app.cfg.ui);
    let key_s = key.to_string();
    let w = segs_width(&button_pill(look, t, label, &key_s, false, false, t.card));
    let r = Rect { x, y, width: w, height: 1 };
    if x + w > max {
        return None;
    }
    let mut segs = button_pill(look, t, label, &key_s, false, hovered(app, r), t.card);
    if key == 'O'
        && let Some(s) = segs.iter_mut().find(|(text, _)| text.starts_with(label))
    {
        s.1 = s.1.fg(t.err);
    }
    put(buf, x, y, &segs, max);
    Some(r)
}

/// The edge bar at a row's left: the selection's, or the red one a failed row keeps whether
/// selected or not.
pub(super) fn edge(app: &App, buf: &mut Buffer, x: u16, y: u16, fg: Color, bg: Color) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(marks(app).bar).set_style(Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD));
    }
}

/// The words in the colour of the result, bold when it failed.
pub(super) fn words_style(info: &CheckInfo, t: &Theme, bg: Color, lit: bool) -> Style {
    let c = if matches!(info.state, CheckState::Checking | CheckState::Failed) { color(t, info) } else { t.muted };
    let style = Style::default().bg(bg).fg(if lit { t.on_lit(c) } else { c });
    if failed(info) { style.add_modifier(Modifier::BOLD) } else { style }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoints(states: &[CheckState]) -> Vec<CheckInfo> {
        states.iter().zip(["build", "test"]).map(|(&state, name)| CheckInfo { name: name.into(), state, ..Default::default() }).collect()
    }

    #[test]
    fn several_checkpoints_are_headed_by_the_worst_one() {
        use CheckState::*;
        let t = Theme::named(crate::theme::DEFAULT);
        let cases = [
            ([Passed, Failed], "failed: test ", t.err, true),
            ([Failed, CouldNotRun], "failed: build ", t.err, true),
            ([CouldNotRun, NeedsApproval], "could not run: build ", t.violet(), false),
            ([NeedsApproval, Passed], "needs you: build ", t.blocked, true),
            ([Checking, Pending], "checking: build ", t.sky(), false),
            ([Passed, Pending], "next: test ", t.muted, false),
            ([Passed, Stale], "stale pass: test ", t.text, true),
            ([Passed, Never], "not checked: test ", t.violet(), false),
            ([Overridden, Passed], "overridden: build ", t.text, true),
            ([Passed, Passed], "all passed ", t.done, true),
        ];
        for (states, words, fg, bold) in cases {
            let (text, st) = headline_word(&checkpoints(&states), &t);
            assert_eq!((text.as_str(), st.fg, st.add_modifier.contains(Modifier::BOLD)), (words, Some(fg), bold), "{states:?}");
        }
    }

    #[test]
    fn a_result_from_before_checkpoints_had_names_is_named_by_its_command() {
        let old = CheckInfo { command: "npm run lint".into(), ..Default::default() };
        assert_eq!(name(&old), "lint");
        assert_eq!(name(&CheckInfo { name: "build".into(), ..old }), "build");
    }

    #[test]
    fn the_ring_steps_every_second_spinner_frame_and_wraps() {
        let steps: Vec<usize> = (0..10).map(|f| ring_step(f, 4)).collect();
        assert_eq!(steps, [0, 0, 1, 1, 2, 2, 3, 3, 0, 0]);
    }
}
