//! Tells the desktop app that a conversation changed, so an open chat reads it then instead of
//! asking several times a second. What changed is not sent: the app reads it as it always has
//! (`Query::ConversationRead`), under the same checks, so this adds no second path for content.

use crate::conversation::ConversationId;
use crate::daemon::{Client, Daemon, ServerMsg};
use crate::protocol::Surface;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// A conversation is announced at most this often. An agent streams its reply in many small
/// events; each one need not wake the window.
const AT_MOST_EVERY: Duration = Duration::from_millis(100);

/// What each conversation looked like when the app was last told of it.
#[derive(Default)]
pub(in crate::daemon) struct News {
    told: HashMap<ConversationId, Told>,
}

struct Told {
    stamp: u64,
    at: Instant,
}

/// A conversation as it is now: its generation, and a number that differs when a read of it
/// would answer differently.
pub(in crate::daemon) struct Seen {
    pub id: ConversationId,
    pub generation: u64,
    pub stamp: u64,
}

impl News {
    /// The conversations to announce: those that changed since they were last announced, and
    /// not within `AT_MOST_EVERY` of it (those are found again on the next look). One seen for
    /// the first time is not announced: nobody has read it yet.
    pub fn due(&mut self, seen: &[Seen], now: Instant) -> Vec<(ConversationId, u64)> {
        self.told
            .retain(|id, _| seen.iter().any(|conversation| conversation.id == *id));
        let mut due = Vec::new();
        for conversation in seen {
            match self.told.get_mut(&conversation.id) {
                None => {
                    self.told.insert(
                        conversation.id,
                        Told {
                            stamp: conversation.stamp,
                            at: now,
                        },
                    );
                }
                Some(told) if told.stamp == conversation.stamp => {}
                Some(told) if now.duration_since(told.at) < AT_MOST_EVERY => {}
                Some(told) => {
                    *told = Told {
                        stamp: conversation.stamp,
                        at: now,
                    };
                    due.push((conversation.id, conversation.generation));
                }
            }
        }
        due
    }
}

fn stamp_of(parts: impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut hasher);
    hasher.finish()
}

impl Daemon {
    /// Every conversation the desktop app can read, as it is now.
    fn conversations_seen(&self) -> Vec<Seen> {
        let followed = self
            .followed
            .iter()
            .filter(|follower| self.shown_to(follower.term, Surface::Desktop))
            .map(|follower| Seen {
                id: follower.id,
                generation: follower.generation,
                stamp: stamp_of((
                    follower.changes(),
                    self.terms
                        .get(&follower.term)
                        .map(|term| std::mem::discriminant(&term.status)),
                )),
            });
        let structured = self
            .conversations
            .iter()
            .filter(|conversation| !self.is_followed(conversation.id))
            .map(|conversation| {
                let id = conversation.id;
                let state = self.conversation_states.get(&id);
                Seen {
                    id,
                    generation: conversation.generation,
                    stamp: stamp_of((
                        conversation.generation,
                        self.conversation_history.revision_of(id),
                        state.map(|state| {
                            (
                                std::mem::discriminant(&state.effective_status()),
                                state.error.is_some(),
                                state.approvals.len(),
                                state.answered_approvals.len(),
                                state.cancelling.is_some(),
                            )
                        }),
                        self.conversation_queues.get(&id).map(|queue| queue.len()),
                        self.conversation_operations
                            .iter()
                            .filter(|operation| operation.receipt.conversation == id)
                            .count(),
                    )),
                }
            });
        followed.chain(structured).collect()
    }

    /// Announces the conversations that changed to the desktop app's attached clients.
    pub(in crate::daemon) fn publish_conversation_news(&mut self) {
        let reads = |client: &Client| client.attach && client.surface == Surface::Desktop;
        if !self.clients.values().any(reads) {
            return;
        }
        let seen = self.conversations_seen();
        for (id, generation) in self.conversation_news.due(&seen, Instant::now()) {
            self.broadcast(reads, ServerMsg::ConversationChanged { id, generation });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AT_MOST_EVERY, News, Seen};
    use crate::conversation::ConversationId;
    use std::time::Instant;

    fn seen(id: u64, stamp: u64) -> Seen {
        Seen {
            id: ConversationId(id),
            generation: 1,
            stamp,
        }
    }

    #[test]
    fn a_conversation_is_announced_when_it_changed_and_not_when_first_seen() {
        let mut news = News::default();
        let start = Instant::now();
        assert!(news.due(&[seen(1, 10)], start).is_empty(), "first seen");
        let later = start + AT_MOST_EVERY;
        assert!(news.due(&[seen(1, 10)], later).is_empty(), "unchanged");
        assert_eq!(news.due(&[seen(1, 11)], later), [(ConversationId(1), 1)]);
        assert!(news.due(&[seen(1, 11)], later).is_empty(), "told already");
    }

    #[test]
    fn a_change_soon_after_an_announcement_waits_and_is_not_lost() {
        let mut news = News::default();
        let start = Instant::now();
        news.due(&[seen(1, 10)], start);
        let told = start + AT_MOST_EVERY;
        assert_eq!(news.due(&[seen(1, 11)], told).len(), 1);
        assert!(news.due(&[seen(1, 12)], told).is_empty(), "too soon");
        assert_eq!(news.due(&[seen(1, 12)], told + AT_MOST_EVERY).len(), 1);
    }

    #[test]
    fn a_conversation_that_went_and_came_back_is_new_again() {
        let mut news = News::default();
        let start = Instant::now();
        news.due(&[seen(1, 10)], start);
        news.due(&[], start);
        assert!(news.due(&[seen(1, 99)], start + AT_MOST_EVERY).is_empty());
    }
}
