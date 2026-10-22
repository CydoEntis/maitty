//! A session's check result on its sidebar row: the glyph that stands for it, its colour,
//! and the line of words under the row. Shape and word carry the meaning; colour backs them.
use super::*;
use crate::protocol::{CheckInfo, CheckState};

/// The ring steps once per this many spinner frames.
const RING_TICKS: u64 = 2;
const EXIT_NOT_EXECUTABLE: i32 = 126;
const EXIT_NOT_FOUND: i32 = 127;
/// Below this the command is not worth showing, so the whole line is cut at its end instead.
const MIN_COMMAND_COLS: usize = 2;
const SECS_PER_MIN: u64 = 60;
const EDGE_BAR: &str = "▎";

struct Marks {
    ring: [&'static str; 4],
    passed: &'static str,
    failed: &'static str,
    overridden: &'static str,
    unknown: &'static str,
    stale: &'static str,
    needs: &'static str,
    pending: &'static str,
}

const UNICODE: Marks = Marks { ring: ["◜", "◝", "◞", "◟"], passed: "✓", failed: "✕", overridden: "⊘", unknown: "?", stale: "◷", needs: "●", pending: "…" };
const ASCII: Marks = Marks { ring: ["-", "\\", "|", "/"], passed: "+", failed: "x", overridden: "/", unknown: "?", stale: "~", needs: "!", pending: "." };

/// Plain icons (ASCII status icons in config) get ASCII check glyphs too.
fn marks(app: &App) -> &'static Marks {
    let i = &app.cfg.icons;
    if [&i.blocked, &i.done, &i.idle].iter().all(|icon| icon.is_ascii()) { &ASCII } else { &UNICODE }
}

/// The result to show for a session. While the agent works or asks, the last result is
/// about an older turn, so the row says only what the agent is doing.
pub(super) fn shown(s: &Session) -> Option<&CheckInfo> {
    crate::protocol::reported_check(s.status, s.check.as_ref())
}

/// A turn that ended and whose work is not known to be bad: no check, or one that passed or
/// was overridden. A running, failed or could-not-run check keeps it from counting as finished.
pub(super) fn finished(s: &Session) -> bool {
    s.status == Status::Done && shown(s).is_none_or(|info| matches!(info.state, CheckState::Passed | CheckState::Overridden))
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
fn ring_step(frame: u64, len: usize) -> usize {
    (frame / RING_TICKS % len as u64) as usize
}

pub(super) fn glyph(app: &App, info: &CheckInfo) -> &'static str {
    let m = marks(app);
    match info.state {
        CheckState::Checking => m.ring[if app.motion_on() { ring_step(app.spinner_frame(), m.ring.len()) } else { 0 }],
        CheckState::Passed => m.passed,
        CheckState::Failed => m.failed,
        CheckState::Overridden => m.overridden,
        CheckState::CouldNotRun | CheckState::Never => m.unknown,
        CheckState::Stale => m.stale,
        CheckState::NeedsApproval => m.needs,
        CheckState::Pending => m.pending,
    }
}

/// An override is never green and could-not-run is not a failure, so neither borrows those colours.
pub(super) fn color(t: &Theme, info: &CheckInfo) -> Color {
    match info.state {
        CheckState::Checking => t.sky(),
        CheckState::Passed => t.done,
        CheckState::Failed => t.err,
        CheckState::Overridden | CheckState::Stale => t.text,
        CheckState::CouldNotRun | CheckState::Never | CheckState::Pending => t.muted,
        CheckState::NeedsApproval => t.blocked,
    }
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
        CheckState::Passed => format!("{cmd} {} · {} · {} ago", m.passed, duration(info.duration_ms), age(info.started_at + info.duration_ms / 1000)),
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

/// The red bar a failed row keeps at its left edge, selected or not.
pub(super) fn edge(buf: &mut Buffer, x: u16, y: u16, bg: Color, t: &Theme) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(EDGE_BAR).set_style(Style::default().fg(t.err).bg(bg));
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

    #[test]
    fn the_ring_steps_every_second_spinner_frame_and_wraps() {
        let steps: Vec<usize> = (0..10).map(|f| ring_step(f, 4)).collect();
        assert_eq!(steps, [0, 0, 1, 1, 2, 2, 3, 3, 0, 0]);
    }
}
