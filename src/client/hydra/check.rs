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
}

const UNICODE: Marks = Marks { ring: ["◜", "◝", "◞", "◟"], passed: "✓", failed: "✕", overridden: "⊘", unknown: "?" };
const ASCII: Marks = Marks { ring: ["-", "\\", "|", "/"], passed: "+", failed: "x", overridden: "/", unknown: "?" };

/// Plain icons (ASCII status icons in config) get ASCII check glyphs too.
fn marks(app: &App) -> &'static Marks {
    let i = &app.cfg.icons;
    if [&i.blocked, &i.done, &i.idle].iter().all(|icon| icon.is_ascii()) { &ASCII } else { &UNICODE }
}

/// The result to show for a session. While the agent works or asks, the last result is
/// about an older turn, so the row says only what the agent is doing.
pub(super) fn shown(s: &Session) -> Option<&CheckInfo> {
    s.check.as_ref().filter(|_| !matches!(s.status, Status::Working | Status::Blocked))
}

pub(super) fn failed(info: &CheckInfo) -> bool {
    info.state == CheckState::Failed
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
        CheckState::CouldNotRun => m.unknown,
    }
}

/// An override is never green and could-not-run is not a failure, so neither borrows those colours.
pub(super) fn color(t: &Theme, info: &CheckInfo) -> Color {
    match info.state {
        CheckState::Checking => t.sky(),
        CheckState::Passed => t.done,
        CheckState::Failed => t.err,
        CheckState::Overridden => t.text,
        CheckState::CouldNotRun => t.muted,
    }
}

pub(super) fn duration(ms: u64) -> String {
    let secs = ms / 1000;
    if secs < SECS_PER_MIN { format!("{:.1}s", ms as f64 / 1000.0) } else { format!("{}m {}s", secs / SECS_PER_MIN, secs % SECS_PER_MIN) }
}

fn why_not_run(exit: Option<i32>) -> String {
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
    }
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
