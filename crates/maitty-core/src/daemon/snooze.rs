//! Snoozing a pane: out of the way until a time, or until it needs you. A snooze never changes
//! the status (ADR 0003); a status that needs you ends the snooze.

use super::*;

/// The longest snooze, so a mistyped time cannot hide a pane for good.
const MOST_SNOOZE_SECS: u64 = 7 * 24 * 60 * 60;

impl Daemon {
    /// Snooze `term` until `until` (unix seconds), or wake it. A time already past wakes it.
    pub(super) fn snooze_pane(&mut self, term: TermId, until: Option<u64>, now: u64) -> Result<()> {
        let Some(t) = self.terms.get_mut(&term) else {
            return Ok(());
        };
        let until = until.filter(|until| *until > now);
        if until.is_some_and(|until| until - now > MOST_SNOOZE_SECS) {
            anyhow::bail!("a snooze lasts at most a week");
        }
        if until.is_some() && (t.status == Status::Blocked || !t.helpers.asked.is_empty()) {
            anyhow::bail!("it needs you now: answer it before you snooze it");
        }
        t.snoozed_until = until;
        self.dirty = true;
        Ok(())
    }

    /// Wake every pane whose snooze has run out.
    pub(super) fn wake_snoozed(&mut self, now: u64) {
        for t in self.terms.values_mut() {
            if t.snoozed_until.is_some_and(|until| until <= now) {
                t.snoozed_until = None;
                self.dirty = true;
            }
        }
    }
}
