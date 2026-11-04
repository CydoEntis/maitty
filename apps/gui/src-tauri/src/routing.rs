//! Which machine a request goes to and what the host may take for an answer (ADR 0048). The
//! window names a pane by its machine and its number; a request for a pane of another machine
//! goes down that machine's line, and the daemon on it is someone else's computer, so a reply or
//! an error from it counts only while the host has a request out to it.

use maitty_desktop_client::PaneCommand;
use maitty_machines::MachineId;
use maitty_shared::protocol::ClientMsg;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How long a request may wait for its answer before the answer stops being expected.
const ANSWER_WINDOW: Duration = Duration::from_secs(30);

/// Most unanswered requests kept per machine: commands nothing replies to must not pile up.
const ASKED_MAX: usize = 64;

/// The requests one machine has not answered yet.
#[derive(Debug, Default)]
pub struct ReplyAllowance {
    asked: VecDeque<Instant>,
}

impl ReplyAllowance {
    /// A command or query went to the machine: a reply or error from it may now be its answer.
    pub fn asked(&mut self, now: Instant) {
        self.asked.push_back(now);
        while self.asked.len() > ASKED_MAX {
            self.asked.pop_front();
        }
    }

    /// Whether a request is out that is still waiting for an answer.
    pub fn awaiting(&mut self, now: Instant) -> bool {
        while self
            .asked
            .front()
            .is_some_and(|at| now.saturating_duration_since(*at) >= ANSWER_WINDOW)
        {
            self.asked.pop_front();
        }
        !self.asked.is_empty()
    }

    /// An answer was taken: one request is no longer waiting.
    pub fn answered(&mut self) {
        self.asked.pop_front();
    }
}

/// Whether sending `msg` to another machine leaves a reply or error to wait for. Keystrokes and
/// a request for a replay are answered by the stream itself.
pub fn expects_answer(msg: &ClientMsg) -> bool {
    matches!(msg, ClientMsg::Command(_) | ClientMsg::Query(_))
}

/// The machine a window request is for: this computer when it names none.
pub fn machine_of(machine: Option<u16>) -> MachineId {
    MachineId(machine.unwrap_or(MachineId::LOCAL.0))
}

/// Whether a command is for a pane of another machine, so it goes down that machine's line
/// rather than to this computer's daemon.
pub fn is_remote(machine: Option<u16>) -> bool {
    machine_of(machine) != MachineId::LOCAL
}

/// The message for `command` on `term`.
pub fn message(term: u32, command: PaneCommand) -> ClientMsg {
    command.message(term)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_out_lets_one_answer_in() {
        let now = Instant::now();
        let mut allowance = ReplyAllowance::default();
        assert!(!allowance.awaiting(now), "nothing was asked");
        allowance.asked(now);
        assert!(allowance.awaiting(now));
        allowance.answered();
        assert!(!allowance.awaiting(now), "one request, one answer");
    }

    #[test]
    fn an_answer_that_comes_too_late_is_no_longer_expected() {
        let now = Instant::now();
        let mut allowance = ReplyAllowance::default();
        allowance.asked(now);
        assert!(allowance.awaiting(now + ANSWER_WINDOW - Duration::from_millis(1)));
        assert!(!allowance.awaiting(now + ANSWER_WINDOW));
    }

    #[test]
    fn commands_nothing_answers_cannot_pile_up_an_allowance() {
        let now = Instant::now();
        let mut allowance = ReplyAllowance::default();
        for _ in 0..ASKED_MAX * 3 {
            allowance.asked(now);
        }
        for _ in 0..ASKED_MAX {
            assert!(allowance.awaiting(now));
            allowance.answered();
        }
        assert!(
            !allowance.awaiting(now),
            "no more than the cap was ever allowed"
        );
    }

    #[test]
    fn only_commands_and_queries_wait_for_an_answer() {
        assert!(!expects_answer(&message(
            1,
            PaneCommand::Input(b"x".to_vec())
        )));
        assert!(!expects_answer(&message(1, PaneCommand::WantReplay)));
        for command in [
            PaneCommand::Resize { cols: 80, rows: 24 },
            PaneCommand::Rename("n".into()),
            PaneCommand::Pin(true),
            PaneCommand::Archive(true),
            PaneCommand::Seen,
            PaneCommand::Close,
        ] {
            let shown = format!("{command:?}");
            assert!(expects_answer(&message(1, command)), "{shown}");
        }
    }

    #[test]
    fn a_request_without_a_machine_is_for_this_computer() {
        assert_eq!(machine_of(None), MachineId::LOCAL);
        assert_eq!(machine_of(Some(0)), MachineId::LOCAL);
        assert_eq!(machine_of(Some(3)), MachineId(3));
        assert!(!is_remote(None) && !is_remote(Some(0)) && is_remote(Some(1)));
    }
}
