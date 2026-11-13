//! Sends to a terminal-backed agent that name an operation (`QueueFollowUp`, `SendWhenReady`),
//! remembered so a client's retry of one is acknowledged instead of typed a second time.
//!
//! Kept in memory only, keyed by a digest of the text: after a daemon restart a retry of a send
//! made before it is taken as a new send.

use crate::protocol::TermId;
use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How long a send is remembered: a retry later than this is a new send.
const REMEMBERED_FOR: Duration = Duration::from_secs(10 * 60);
/// The most sends remembered at once; past it the oldest is forgotten early.
const REMEMBERED_MOST: usize = 1024;

struct Sent {
    term: TermId,
    operation: u64,
    digest: [u8; 32],
    at: Instant,
}

/// The sends made in the last ten minutes, oldest first.
#[derive(Default)]
pub(super) struct TerminalSends {
    sent: VecDeque<Sent>,
}

impl TerminalSends {
    /// Whether this send was already made: true for the same pane, operation and text within
    /// ten minutes, an error for the same pane and operation with other text, or for operation
    /// zero. A send without an operation is never a repeat.
    pub(super) fn already_sent(
        &mut self,
        term: TermId,
        operation: Option<u64>,
        text: &str,
        now: Instant,
    ) -> Result<bool> {
        self.forget_before(now);
        let Some(operation) = operation else {
            return Ok(false);
        };
        if operation == 0 {
            bail!("an operation ID must be nonzero");
        }
        match self.sent.iter().find(|s| s.term == term && s.operation == operation) {
            Some(s) if s.digest == digest(text) => Ok(true),
            Some(_) => bail!("that operation ID was already used for other text"),
            None => Ok(false),
        }
    }

    /// Note a send that was taken, so a retry of it is recognised.
    pub(super) fn remember(&mut self, term: TermId, operation: Option<u64>, text: &str, now: Instant) {
        let Some(operation) = operation else {
            return;
        };
        if self.sent.len() >= REMEMBERED_MOST {
            self.sent.pop_front();
        }
        self.sent.push_back(Sent {
            term,
            operation,
            digest: digest(text),
            at: now,
        });
    }

    fn forget_before(&mut self, now: Instant) {
        while self
            .sent
            .front()
            .is_some_and(|s| now.duration_since(s.at) >= REMEMBERED_FOR)
        {
            self.sent.pop_front();
        }
    }
}

fn digest(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_send_is_a_repeat_for_ten_minutes_and_only_on_its_own_pane() {
        let mut sends = TerminalSends::default();
        let start = Instant::now();
        sends.remember(3, Some(1), "run the tests", start);
        let later = |minutes: u64| start + Duration::from_secs(minutes * 60);
        assert!(sends.already_sent(3, Some(1), "run the tests", later(9)).unwrap());
        assert!(!sends.already_sent(4, Some(1), "run the tests", later(9)).unwrap());
        assert!(!sends.already_sent(3, Some(2), "run the tests", later(9)).unwrap());
        assert!(!sends.already_sent(3, None, "run the tests", later(9)).unwrap());
        assert!(sends.already_sent(3, Some(1), "open a PR", later(9)).is_err());
        assert!(!sends.already_sent(3, Some(1), "run the tests", later(10)).unwrap());
        assert!(!sends.already_sent(3, Some(1), "open a PR", later(10)).unwrap());
    }

    #[test]
    fn past_the_limit_the_oldest_send_is_forgotten() {
        let mut sends = TerminalSends::default();
        let now = Instant::now();
        for operation in 1..=REMEMBERED_MOST as u64 + 1 {
            sends.remember(3, Some(operation), "x", now);
        }
        assert_eq!(sends.sent.len(), REMEMBERED_MOST);
        assert!(!sends.already_sent(3, Some(1), "x", now).unwrap());
        assert!(sends.already_sent(3, Some(2), "x", now).unwrap());
    }
}
