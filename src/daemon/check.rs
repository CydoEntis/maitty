//! A pane's check result as the daemon holds and saves it: the summary every client sees,
//! and the output tail only `Query::CheckOutput` returns.

use super::persist::{self, CheckSlot, SavedPane};
use super::term::Term;
use crate::protocol::{CheckInfo, CheckState, TermId};

/// The most output kept per result. A check's tail is what explains a failure; the rest
/// would only grow the saved files.
pub const CHECK_OUTPUT_MAX_BYTES: usize = 1024 * 1024;

/// The shell's exit code for a command it could not find.
#[allow(dead_code)]
const EXIT_COMMAND_NOT_FOUND: i32 = 127;

/// How a check's process ended, before it is read as a result.
// Nothing produces an outcome until the runner lands.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    Exited(i32),
    TimedOut,
    SpawnFailed,
}

/// A command that never ran, or never finished, says nothing about the work, so it is not
/// a failure.
#[allow(dead_code)]
pub fn state_for(outcome: CheckOutcome) -> CheckState {
    match outcome {
        CheckOutcome::Exited(0) => CheckState::Passed,
        CheckOutcome::Exited(EXIT_COMMAND_NOT_FOUND) => CheckState::CouldNotRun,
        CheckOutcome::Exited(_) => CheckState::Failed,
        CheckOutcome::TimedOut | CheckOutcome::SpawnFailed => CheckState::CouldNotRun,
    }
}

/// The last `CHECK_OUTPUT_MAX_BYTES` of `text`, starting on a character boundary.
pub fn cap_output(text: &str) -> String {
    let Some(excess) = text.len().checked_sub(CHECK_OUTPUT_MAX_BYTES) else { return text.to_string() };
    let start = (excess..=text.len()).find(|&i| text.is_char_boundary(i)).unwrap_or(text.len());
    text[start..].to_string()
}

/// One check result: the summary clients see, and its output tail.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckRecord {
    pub info: CheckInfo,
    pub output: String,
}

impl CheckRecord {
    pub fn new(info: CheckInfo, output: &str) -> Self {
        Self { info, output: cap_output(output) }
    }
}

impl Term {
    /// Take back the check results a pane had before the server restarted. A check that was
    /// running died with the server, so it reads as not run rather than checking forever.
    pub fn restore_checks(&mut self, old: TermId, saved: &SavedPane) {
        let record = |info: &Option<CheckInfo>, slot| {
            let mut info = info.clone()?;
            if info.state == CheckState::Checking {
                info.state = CheckState::CouldNotRun;
            }
            Some(CheckRecord::new(info, &persist::take_check_output(old, slot).unwrap_or_default()))
        };
        self.check = record(&saved.check, CheckSlot::Latest);
        self.check_overridden = record(&saved.check_overridden, CheckSlot::Overridden);
        self.check_dirty = self.check.is_some() || self.check_overridden.is_some();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exit_code_a_timeout_and_a_spawn_failure_each_read_as_one_state() {
        let table = [
            (CheckOutcome::Exited(0), CheckState::Passed),
            (CheckOutcome::Exited(1), CheckState::Failed),
            (CheckOutcome::Exited(2), CheckState::Failed),
            (CheckOutcome::Exited(126), CheckState::Failed),
            (CheckOutcome::Exited(-1), CheckState::Failed),
            (CheckOutcome::Exited(127), CheckState::CouldNotRun),
            (CheckOutcome::TimedOut, CheckState::CouldNotRun),
            (CheckOutcome::SpawnFailed, CheckState::CouldNotRun),
        ];
        for (outcome, state) in table {
            assert_eq!(state_for(outcome), state, "{outcome:?}");
        }
    }

    #[test]
    fn output_over_the_cap_keeps_its_tail_whole_characters_only() {
        assert_eq!(cap_output("short"), "short");
        let exact = "x".repeat(CHECK_OUTPUT_MAX_BYTES);
        assert_eq!(cap_output(&exact), exact, "exactly the cap is kept whole");
        // The cut would fall inside the second 3-byte character, so that one is dropped whole.
        let text = format!("{}{}", "\u{2713}".repeat(2), "y".repeat(CHECK_OUTPUT_MAX_BYTES - 1));
        assert_eq!(cap_output(&text), "y".repeat(CHECK_OUTPUT_MAX_BYTES - 1));
    }
}
