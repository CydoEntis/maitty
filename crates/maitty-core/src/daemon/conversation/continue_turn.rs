//! Telling a chat conversation the daemon resumed after a restart to carry on.

use super::commands::DAEMON_REQUESTER;
use crate::conversation::{ConversationId, TurnChoice};
use crate::daemon::{ClientId, Daemon, term};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MESSAGE: &str = "Continue where you left off.";
/// Each daemon start tells a conversation to continue once, so only a daemon that restarts in
/// a loop reaches this.
const MAX_IN_WINDOW: usize = 3;
const WINDOW_SECS: u64 = 5 * 60;

/// When each conversation was told to continue, in unix seconds. Saved in the session file: a
/// record kept in memory would start empty with every daemon and cap nothing.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Continues(BTreeMap<ConversationId, Vec<u64>>);

impl Continues {
    /// Whether `conversation` may be told to continue at `now`. A yes is recorded.
    fn admit(&mut self, conversation: ConversationId, now: u64) -> bool {
        // A time ahead of `now` (the clock was set back) still counts, so the cap fails closed.
        self.0.retain(|_, sent| {
            sent.retain(|at| at.saturating_add(WINDOW_SECS) > now);
            !sent.is_empty()
        });
        let sent = self.0.entry(conversation).or_default();
        if sent.len() >= MAX_IN_WINDOW {
            return false;
        }
        sent.push(now);
        true
    }
}

impl Daemon {
    /// The daemon started `id` again and it was in a turn when the last daemon stopped: it is
    /// told to continue once that resume, at the generation it runs at, has succeeded.
    pub(in crate::daemon) fn continue_when_resumed(&mut self, id: ConversationId) {
        if !self.cfg.restore.continue_turn {
            return;
        }
        if let Some(conversation) = self.conversations.iter().find(|c| c.id == id) {
            self.turns_to_continue.insert(id, conversation.generation);
        }
    }

    /// A start of `id` at `generation` succeeded for `client`. Only the daemon's own resume of
    /// a conversation noted by `continue_when_resumed` sends the message, and only once.
    pub(in crate::daemon) fn continue_cut_turn(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
    ) {
        if client != DAEMON_REQUESTER || self.turns_to_continue.remove(&id) != Some(generation) {
            return;
        }
        if !self.cfg.restore.continue_turn {
            return;
        }
        // Recorded before the send: the send saves the session before anything reaches the
        // provider, so the record is on disk first and a crash right after cannot lose it.
        if !self.turn_continues.admit(id, term::unix_now()) {
            tracing::warn!(
                "conversation {} was already told to continue {MAX_IN_WINDOW} times in {} minutes; not again",
                id.0,
                WINDOW_SECS / 60
            );
            return;
        }
        self.dirty = true;
        let last = self
            .conversation_operations
            .iter()
            .find(|operation| operation.receipt.conversation == id)
            .map_or(0, |operation| operation.receipt.operation);
        let sent = last
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Conversation operation identity exhausted"))
            .and_then(|operation| {
                self.conversation_send(
                    DAEMON_REQUESTER,
                    id,
                    generation,
                    operation,
                    MESSAGE.into(),
                    TurnChoice::default(),
                )
            });
        if let Err(error) = sent {
            tracing::warn!("conversation {} was not told to continue: {error:#}", id.0);
        }
    }
}

#[cfg(test)]
pub(in crate::daemon) fn message_digest() -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(MESSAGE.as_bytes()))
}

#[cfg(test)]
impl Continues {
    pub(in crate::daemon) fn full(conversation: ConversationId, now: u64) -> Self {
        Continues(BTreeMap::from([(conversation, vec![now; MAX_IN_WINDOW])]))
    }

    pub(in crate::daemon) fn count(&self, conversation: ConversationId) -> usize {
        self.0.get(&conversation).map_or(0, Vec::len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: ConversationId = ConversationId(1);

    #[test]
    fn a_conversation_is_told_to_continue_three_times_in_five_minutes_and_no_more() {
        let mut continues = Continues::default();
        assert!(continues.admit(ONE, 1_000));
        assert!(continues.admit(ONE, 1_100));
        assert!(continues.admit(ONE, 1_299));
        assert!(!continues.admit(ONE, 1_299), "the fourth inside five minutes");
        assert!(!continues.admit(ONE, 900), "nor when the clock went back");
        assert!(continues.admit(ONE, 1_300), "the first one is five minutes old");
        assert!(!continues.admit(ONE, 1_300));
    }

    #[test]
    fn the_cap_is_counted_for_each_conversation_by_itself() {
        let mut continues = Continues::full(ONE, 50);
        assert!(!continues.admit(ONE, 50));
        assert!(continues.admit(ConversationId(2), 50));
    }

    #[test]
    fn the_record_comes_back_from_the_session_file_still_capping() {
        let saved = crate::daemon::persist::Saved {
            turn_continues: Continues::full(ONE, 50),
            ..Default::default()
        };
        let text = serde_json::to_string(&saved).unwrap();
        let mut loaded: crate::daemon::persist::Saved = serde_json::from_str(&text).unwrap();
        assert!(!loaded.turn_continues.admit(ONE, 60));
    }

    #[test]
    fn a_session_file_from_before_the_record_loads_with_none() {
        let old = r#"{"workspaces":[],"active":0}"#;
        let loaded: crate::daemon::persist::Saved = serde_json::from_str(old).unwrap();
        assert_eq!(loaded.turn_continues, Continues::default());
    }
}
