//! Which machine a request goes to and what the host may take for an answer (ADR 0007). The
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

/// How much longer than the daemon's own limit a clone's answer is waited for: the time the answer
/// takes to cross the line.
const CLONE_ANSWER_MARGIN: Duration = Duration::from_secs(30);

/// How long a list of GitHub repositories or owners is waited for. The daemon gives `gh` 15 seconds
/// to say who is signed in, then `GITHUB_LIST_LIMIT_SECS` to list, and the answer has to cross.
const GITHUB_LIST_ANSWER_WINDOW: Duration = Duration::from_secs(15 + maitty_shared::protocol::GITHUB_LIST_LIMIT_SECS + 30);

/// The requests one machine has not answered yet, each as the moment its answer stops being expected.
#[derive(Debug, Default)]
pub struct ReplyAllowance {
    until: VecDeque<Instant>,
}

impl ReplyAllowance {
    /// A command or query went to the machine: a reply or error from it may now be its answer.
    /// Its answer is expected for `window`.
    pub fn asked(&mut self, now: Instant, window: Duration) {
        self.until.push_back(now + window);
        while self.until.len() > ASKED_MAX {
            self.until.pop_front();
        }
    }

    /// Whether a request is out that is still waiting for an answer.
    pub fn awaiting(&mut self, now: Instant) -> bool {
        self.until.retain(|until| *until > now);
        !self.until.is_empty()
    }

    /// An answer was taken: one request is no longer waiting. Which request an answer is for is not
    /// known here, so the one that would stop waiting soonest is taken: a quick answer never uses up
    /// the long wait of a clone that is still running.
    pub fn answered(&mut self) {
        let soonest = self.until.iter().enumerate().min_by_key(|(_, until)| **until).map(|(at, _)| at);
        if let Some(at) = soonest {
            self.until.remove(at);
        }
    }
}

/// Whether sending `msg` to another machine leaves a reply or error to wait for. Keystrokes and
/// a request for a replay are answered by the stream itself.
pub fn expects_answer(msg: &ClientMsg) -> bool {
    answer_window(msg).is_some()
}

/// How long the answer to `msg` is expected for, or `None` when it has none. A clone answers only
/// when git has finished, which the daemon allows minutes for.
pub fn answer_window(msg: &ClientMsg) -> Option<Duration> {
    match msg {
        ClientMsg::Query(maitty_shared::protocol::Query::CloneRepo { .. }) => {
            Some(Duration::from_secs(maitty_shared::protocol::CLONE_LIMIT_SECS) + CLONE_ANSWER_MARGIN)
        }
        ClientMsg::Query(
            maitty_shared::protocol::Query::GitHubRepos { .. } | maitty_shared::protocol::Query::GitHubOwners { .. },
        ) => Some(GITHUB_LIST_ANSWER_WINDOW),
        ClientMsg::Command(_) | ClientMsg::Query(_) => Some(ANSWER_WINDOW),
        _ => None,
    }
}

/// The words for a feature that reads or changes a pane's files, asked of a pane of another
/// machine: the files are there, not here. The terminal app says the same.
pub fn local_only_message(label: &str, machine: &str) -> String {
    format!("{label} is local-only in v1: the files are on {machine}, not on this computer.")
}

/// Refuse a request about a pane's files when the pane is on another machine. The window does not
/// ask this of the host for such a pane; this is the host saying no if it ever does, since a pane
/// number alone would otherwise reach a pane of this computer.
pub fn require_local(
    machine: Option<u16>,
    label: &str,
    name_of: impl FnOnce(MachineId) -> Option<String>,
) -> Result<(), String> {
    if !is_remote(machine) {
        return Ok(());
    }
    let name = name_of(machine_of(machine)).unwrap_or_else(|| "another machine".to_string());
    Err(local_only_message(label, &name))
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
    fn a_pane_of_another_machine_is_refused_for_its_files_in_the_terminal_apps_words() {
        let named = |_| Some("build".to_string());
        assert_eq!(
            require_local(Some(2), "Changes", named).unwrap_err(),
            "Changes is local-only in v1: the files are on build, not on this computer."
        );
        assert_eq!(
            require_local(Some(2), "Worktrees", |_| None).unwrap_err(),
            "Worktrees is local-only in v1: the files are on another machine, not on this computer."
        );
        assert!(require_local(None, "Changes", named).is_ok());
        assert!(require_local(Some(0), "Changes", named).is_ok());
    }

    #[test]
    fn a_request_out_lets_one_answer_in() {
        let now = Instant::now();
        let mut allowance = ReplyAllowance::default();
        assert!(!allowance.awaiting(now), "nothing was asked");
        allowance.asked(now, ANSWER_WINDOW);
        assert!(allowance.awaiting(now));
        allowance.answered();
        assert!(!allowance.awaiting(now), "one request, one answer");
    }

    #[test]
    fn an_answer_that_comes_too_late_is_no_longer_expected() {
        let now = Instant::now();
        let mut allowance = ReplyAllowance::default();
        allowance.asked(now, ANSWER_WINDOW);
        assert!(allowance.awaiting(now + ANSWER_WINDOW - Duration::from_millis(1)));
        assert!(!allowance.awaiting(now + ANSWER_WINDOW));
    }

    #[test]
    fn a_github_list_is_waited_for_longer_than_gh_is_given_to_make_it() {
        use maitty_shared::protocol::{GITHUB_LIST_LIMIT_SECS, Query};
        let repos = ClientMsg::Query(Query::GitHubRepos { owner: None, search: None, tag: 1 });
        for msg in [repos, ClientMsg::Query(Query::GitHubOwners { tag: 2 })] {
            let window = answer_window(&msg).expect("a list is answered");
            assert!(window > Duration::from_secs(GITHUB_LIST_LIMIT_SECS) && window > ANSWER_WINDOW);
        }
    }

    #[test]
    fn a_clone_still_running_keeps_its_answer_expected_while_quick_answers_come_and_go() {
        let now = Instant::now();
        let mut allowance = ReplyAllowance::default();
        let clone = ClientMsg::Query(maitty_shared::protocol::Query::CloneRepo {
            address: "https://example.com/o/r.git".into(),
            parent: "~".into(),
            tag: 1,
        });
        allowance.asked(now, answer_window(&clone).expect("a clone is answered"));
        allowance.asked(now, ANSWER_WINDOW);
        allowance.answered();
        let later = now + ANSWER_WINDOW * 4;
        assert!(allowance.awaiting(later), "the clone's answer is still to come");
        allowance.answered();
        assert!(!allowance.awaiting(later));
    }

    #[test]
    fn commands_nothing_answers_cannot_pile_up_an_allowance() {
        let now = Instant::now();
        let mut allowance = ReplyAllowance::default();
        for _ in 0..ASKED_MAX * 3 {
            allowance.asked(now, ANSWER_WINDOW);
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
            PaneCommand::SendWhenReady("m".into()),
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
