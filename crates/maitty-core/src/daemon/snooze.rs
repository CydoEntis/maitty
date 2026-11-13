//! Snoozing a pane: out of the way until a time, or until it needs you. A snooze never changes
//! the status (ADR 0003); anything that needs you ends the snooze.

use super::*;

/// The longest snooze, so a mistyped time cannot hide a pane for good.
const MOST_SNOOZE_SECS: u64 = 7 * 24 * 60 * 60;

impl Daemon {
    /// Snooze `term` until `until` (unix seconds), or wake it. A time already past wakes it.
    pub(super) fn snooze_pane(&mut self, term: TermId, until: Option<u64>, now: u64) -> Result<()> {
        let Some(t) = self.terms.get(&term) else {
            return Ok(());
        };
        let until = until.filter(|until| *until > now);
        if until.is_some_and(|until| until - now > MOST_SNOOZE_SECS) {
            anyhow::bail!("a snooze lasts at most a week");
        }
        // A chat's questions arrive on its conversation, which nothing here watches.
        if until.is_some() && t.conversation_id.is_some() {
            anyhow::bail!("a chat session cannot be snoozed");
        }
        if until.is_some() && self.asks_for_you(term) {
            anyhow::bail!("it needs you now: answer it before you snooze it");
        }
        if let Some(t) = self.terms.get_mut(&term) {
            t.snoozed_until = until;
        }
        self.dirty = true;
        Ok(())
    }

    /// Something in `term` needs you: its snooze, if any, is over.
    pub(super) fn needs_you(&mut self, term: TermId) {
        if let Some(t) = self.terms.get_mut(&term)
            && t.snoozed_until.take().is_some()
        {
            self.dirty = true;
        }
    }

    /// Wake every pane whose snooze has run out, or lies further off than a snooze may (the
    /// clock went back, or a saved one is out of range).
    pub(super) fn wake_snoozed(&mut self, now: u64) {
        let latest = now.saturating_add(MOST_SNOOZE_SECS);
        for t in self.terms.values_mut() {
            if t.snoozed_until.is_some_and(|until| until <= now || until > latest) {
                t.snoozed_until = None;
                self.dirty = true;
            }
        }
    }

    fn asks_for_you(&self, term: TermId) -> bool {
        self.secrets.is_asking(term)
            || self.terms.get(&term).is_some_and(|t| {
                t.status == Status::Blocked || !t.helpers.asked.is_empty() || t.checks.need_you()
            })
    }
}
