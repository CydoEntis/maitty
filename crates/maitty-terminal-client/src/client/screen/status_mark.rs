//! The status glyph beside a session: one of six states, drawn the same in the sidebar, the
//! collapsed column and the pane's title. The state is the daemon's (`Status`, `TermInfo.exit`)
//! and whether the machine is reachable; this file only chooses how to draw it.
//!
//! By default a state differs by colour only (`●`); `ui.glyphs = "unicode"` ("Status glyphs:
//! unicode" in Settings) gives each state a shape of its own for colour-blind users.

use super::*;

/// What a session is doing, as far as the glyph is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum State {
    /// An agent or command is running: the spinner.
    Working,
    /// Stopped and waiting for you.
    Needs,
    /// Finished its turn and nobody has looked yet.
    Done,
    /// The last command ended in failure (`TermInfo.exit`).
    Problem,
    Idle,
    /// Its machine or the daemon cannot be reached, so what the row says is the last report.
    Unknown,
}

/// Whether the last command ended in failure (`TermInfo.exit`) or the check the app ran after
/// the turn failed: either is a problem to look at.
pub(in crate::client) fn failed(
    exit: bool,
    status: Status,
    check: Option<&crate::protocol::CheckInfo>,
) -> bool {
    exit || crate::protocol::reported_check(status, check)
        .is_some_and(|c| c.state == crate::protocol::CheckState::Failed)
}

impl State {
    /// The state the daemon reported. A reachable session that needs you stays `Needs` even
    /// when a command also failed; a failure otherwise outranks done and idle, since it is
    /// the thing to look at.
    pub fn of(status: Status, failed: bool, unreachable: bool) -> State {
        if unreachable {
            return State::Unknown;
        }
        match status {
            Status::Blocked => State::Needs,
            Status::Working => State::Working,
            _ if failed => State::Problem,
            Status::Done => State::Done,
            Status::Idle | Status::None => State::Idle,
        }
    }
}

/// The `unicode` set: a shape per state.
const SHAPE_NEEDS: &str = "●";
const SHAPE_DONE: &str = "✓";
const SHAPE_PROBLEM: &str = "✕";
const SHAPE_IDLE: &str = "○";
/// The same dot for every state, told apart by colour.
const DOT: &str = "●";
/// An unreachable session's mark, in the set either way.
const UNKNOWN: &str = "┆";

/// Milliseconds one frame of each spinner is held (the design's timings). A spinner the user
/// drew themselves keeps the old pace.
const PACE_BRAILLE_MS: u128 = 80;
const PACE_SNAKE_MS: u128 = 90;
const PACE_BARS_MS: u128 = 85;
const PACE_DEFAULT_MS: u128 = 90;

/// How long a frame of the chosen spinner is held. "Dots" is the design's Braille spinner.
pub(in crate::client) fn pace_ms(frames: &[String]) -> u128 {
    match crate::config::spinner_name(frames) {
        "Dots" => PACE_BRAILLE_MS,
        "Snake" => PACE_SNAKE_MS,
        "Bars" => PACE_BARS_MS,
        _ => PACE_DEFAULT_MS,
    }
}

/// The spinner frame for `elapsed_ms` into the run; the first frame when motion is off.
pub(in crate::client) fn frame_at(frames: &[String], elapsed_ms: u128, moving: bool) -> &str {
    if frames.is_empty() {
        return DOT;
    }
    let step = if moving {
        (elapsed_ms / pace_ms(frames)) as usize % frames.len()
    } else {
        0
    };
    &frames[step]
}

/// A glyph and its colour.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct Mark {
    pub glyph: String,
    pub color: Color,
}

impl App {
    /// The working spinner's current frame: still under Reduce motion.
    pub(in crate::client) fn working_frame(&self) -> String {
        frame_at(
            &self.cfg.ui.spinner,
            self.started.elapsed().as_millis(),
            self.motion_on(),
        )
        .to_string()
    }

    /// The mark for a session in `state`.
    pub(in crate::client) fn state_mark(&self, state: State, t: &Theme) -> Mark {
        let shapes = self.cfg.ui.glyphs == "unicode";
        let (glyph, color) = match state {
            State::Working => (self.working_frame(), t.accent),
            State::Needs => (SHAPE_NEEDS.to_string(), t.blocked),
            State::Done => (if shapes { SHAPE_DONE } else { DOT }.to_string(), t.done),
            State::Problem => (if shapes { SHAPE_PROBLEM } else { DOT }.to_string(), t.err),
            State::Idle => (if shapes { SHAPE_IDLE } else { DOT }.to_string(), t.idle),
            State::Unknown => (UNKNOWN.to_string(), t.idle),
        };
        Mark { glyph, color }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(name: &str) -> Vec<String> {
        crate::config::SPINNERS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, f)| f.iter().map(|s| (*s).to_string()).collect())
            .expect("a preset")
    }

    #[test]
    fn the_daemons_status_picks_the_state_and_a_failure_outranks_done_and_idle_only() {
        assert_eq!(State::of(Status::Blocked, true, false), State::Needs);
        assert_eq!(State::of(Status::Working, true, false), State::Working);
        assert_eq!(State::of(Status::Done, true, false), State::Problem);
        assert_eq!(State::of(Status::Idle, true, false), State::Problem);
        assert_eq!(State::of(Status::Done, false, false), State::Done);
        assert_eq!(State::of(Status::None, false, false), State::Idle);
        assert_eq!(State::of(Status::Blocked, false, true), State::Unknown);
    }

    #[test]
    fn each_spinner_is_held_for_its_own_time() {
        let braille = frames("Dots");
        assert_eq!(frame_at(&braille, 79, true), "⠋");
        assert_eq!(frame_at(&braille, 80, true), "⠙");
        assert_eq!(frame_at(&braille, 800, true), "⠋", "ten frames, then round");
        let snake = frames("Snake");
        assert_eq!(frame_at(&snake, 89, true), snake[0]);
        assert_eq!(frame_at(&snake, 90, true), snake[1]);
        let bars = frames("Bars");
        assert_eq!(pace_ms(&bars), 85);
    }

    #[test]
    fn reduce_motion_holds_the_first_frame() {
        let braille = frames("Dots");
        assert_eq!(frame_at(&braille, 5_000, false), "⠋");
    }
}
