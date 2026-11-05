use crate::conversation::ConversationId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(in crate::daemon) const MAX_READ_ENTRIES: usize = 128;
pub(in crate::daemon) const MAX_READ_BYTES: usize = 256 * 1024;

pub(crate) use crate::conversation::history::*;
pub(in crate::daemon) enum Update {
    ProviderError { message: String, will_retry: bool },
    UpsertItem(Entry),
    AppendDelta { source: SourceId, delta: String },
    SetTurnStatus {
        turn: String,
        state: ItemState,
        started_at_ms: Option<u64>,
        completed_at_ms: Option<u64>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::daemon) struct History {
    pub conversation: ConversationId,
    pub revision: u64,
    pub entries: Vec<Entry>,
    pub completeness: Completeness,
    // Drafts have no replay sequence in Codex, so they never enter a durable page.
    #[serde(skip)]
    drafts: Vec<SourceId>,
    // What changed when, for reads that want only the news. Not part of the history's identity.
    #[serde(skip)]
    track: Track,
}

/// The revision at which each entry last changed, and which run of the history this is.
///
/// Entries are added at the end and changed in place, never moved or removed, until the history
/// is rebuilt; a rebuild starts a new `epoch`. That is what lets a reader ask for "what changed
/// after revision R" instead of reading everything again.
#[derive(Clone, Debug)]
struct Track {
    epoch: u64,
    /// Parallel to `entries`; an entry without a stamp (a history loaded from the cache) counts
    /// as unchanged since before every cursor.
    changed_at: Vec<u64>,
    /// The revision the update being applied will have.
    stamp: u64,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            epoch: fresh_epoch(),
            changed_at: Vec::new(),
            stamp: 0,
        }
    }
}

// Tracking is bookkeeping, not content: two histories with the same entries are equal.
impl PartialEq for Track {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for Track {}

/// A number no earlier history of this daemon, or of an earlier run of it, had. Never 0, which a
/// cursor from a client that predates epochs carries.
fn fresh_epoch() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |elapsed| elapsed.as_nanos() as u64 | 1);
    // The first call seeds from the clock, so a restarted daemon does not reuse the old numbers.
    let _ = NEXT.compare_exchange(0, start, Ordering::Relaxed, Ordering::Relaxed);
    NEXT.fetch_add(1, Ordering::Relaxed).max(1)
}
impl History {
    pub fn new(conversation: ConversationId) -> Self {
        Self {
            conversation,
            revision: 1,
            entries: Vec::new(),
            completeness: Completeness::Unknown,
            drafts: Vec::new(),
            track: Track::default(),
        }
    }
    /// Notes that entry `index` changed in the update being applied.
    fn touch(&mut self, index: usize) {
        let track = &mut self.track;
        if track.changed_at.len() < self.entries.len() {
            track.changed_at.resize(self.entries.len(), 0);
        }
        track.changed_at[index] = track.stamp;
    }
    /// The history was rebuilt rather than extended: readers must start over.
    fn rebuilt(&mut self) {
        self.track.epoch = fresh_epoch();
        self.track.changed_at.clear();
    }
    pub fn apply(&mut self, update: Update) -> Result<(), IncompleteReason> {
        let next_revision = self.next_revision()?;
        self.track.stamp = next_revision;
        let changed = match update {
            Update::ProviderError { .. } => false,
            Update::UpsertItem(mut entry) => {
                super::redact::redact_entry(&mut entry);
                self.upsert(entry)
            }
            Update::AppendDelta { source, delta } => self.append_delta(source, &delta),
            Update::SetTurnStatus {
                turn,
                state,
                started_at_ms,
                completed_at_ms,
            } => {
                let mut changed = self.upsert(Entry {
                    source: SourceId {
                        conversation: self.conversation,
                        turn: turn.clone(),
                        item: String::new(),
                    },
                    content: Content::TurnStatus,
                    state: state.clone(),
                    started_at_ms,
                    completed_at_ms,
                });
                if matches!(state, ItemState::Failed | ItemState::Interrupted) {
                    let mut touched = Vec::new();
                    for (index, entry) in self.entries.iter_mut().enumerate() {
                        if entry.source.turn == turn
                            && entry.content != Content::TurnStatus
                            && entry.state == ItemState::Running
                        {
                            entry.state = state.clone();
                            touched.push(index);
                            changed = true;
                        }
                    }
                    for index in touched {
                        self.touch(index);
                    }
                    self.drafts.retain(|source| source.turn != turn);
                }
                changed
            }
        };
        if changed {
            self.revision = next_revision;
        }
        Ok(())
    }
    fn upsert(&mut self, mut entry: Entry) -> bool {
        if entry.source.conversation != self.conversation {
            return false;
        }
        if entry.state == ItemState::Running
            && self
                .entries
                .iter()
                .any(|old| old.source == entry.source && old.state != ItemState::Running)
        {
            return false;
        }
        let incomplete = matches!(entry.content, Content::Unsupported { .. })
            && self.completeness != Completeness::Incomplete(IncompleteReason::UnsupportedItem);
        if incomplete {
            self.completeness = Completeness::Incomplete(IncompleteReason::UnsupportedItem);
        }
        if entry.state == ItemState::Running && entry.content != Content::TurnStatus {
            if !self.drafts.contains(&entry.source) {
                self.drafts.push(entry.source.clone());
            }
        } else {
            self.drafts.retain(|key| key != &entry.source);
        }
        if let Some(index) = self
            .entries
            .iter()
            .position(|old| old.source == entry.source)
        {
            let old = &mut self.entries[index];
            // A completion event carries no start time; the row keeps the one its start event set.
            entry.started_at_ms = entry.started_at_ms.or(old.started_at_ms);
            if *old == entry {
                return incomplete;
            }
            *old = entry;
            self.touch(index);
        } else {
            self.entries.push(entry);
            self.touch(self.entries.len() - 1);
        }
        true
    }
    fn append_delta(&mut self, source: SourceId, delta: &str) -> bool {
        if delta.is_empty() || source.conversation != self.conversation {
            return false;
        }
        if let Some(entry) = self.entries.iter().find(|entry| entry.source == source)
            && entry.state != ItemState::Running
        {
            // An authoritative complete native item wins over replayed streaming fragments.
            return false;
        }
        if !self.drafts.contains(&source) {
            self.upsert(Entry {
                source: source.clone(),
                content: Content::Message {
                    role: Role::Assistant,
                    text: String::new(),
                },
                state: ItemState::Running,
                started_at_ms: None,
                completed_at_ms: None,
            });
        }
        if let Some(index) = self.entries.iter().position(|old| old.source == source)
            && let Content::Message { text, .. } = &mut self.entries[index].content
        {
            text.push_str(delta);
            // A secret can arrive split across deltas, so the whole text is masked again; masking
            // is idempotent, so what was masked before stays as it is.
            super::redact::redact_in_place(text);
            self.touch(index);
            return !delta.is_empty();
        }
        false
    }
    fn next_revision(&self) -> Result<u64, IncompleteReason> {
        self.revision
            .checked_add(1)
            .ok_or(IncompleteReason::RevisionExhausted)
    }
    pub fn set_completeness(&mut self, completeness: Completeness) -> Result<(), IncompleteReason> {
        if self.completeness != completeness {
            let next_revision = self.next_revision()?;
            self.completeness = completeness;
            self.revision = next_revision;
        }
        Ok(())
    }
    pub fn reconnect(&mut self) -> Result<(), IncompleteReason> {
        let next_revision = self.next_revision()?;
        let before = self.entries.len();
        self.entries
            .retain(|entry| !self.drafts.contains(&entry.source));
        if self.entries.len() != before {
            self.rebuilt();
        }
        self.drafts.clear();
        self.completeness = Completeness::Incomplete(IncompleteReason::Disconnected);
        self.revision = next_revision;
        Ok(())
    }
    pub fn hydrate(
        &mut self,
        entries: Vec<Entry>,
        completeness: Completeness,
    ) -> Result<(), IncompleteReason> {
        let next_revision = self.next_revision()?;
        for (index, entry) in entries.iter().enumerate() {
            if entry.source.conversation != self.conversation
                || entries[..index]
                    .iter()
                    .any(|old| old.source == entry.source)
            {
                return Err(IncompleteReason::InvalidEvent);
            }
        }
        // The native pager supplies canonical order; IDs and timestamps are not sorting keys.
        self.entries = entries;
        self.redact_stored();
        self.rebuilt();
        self.drafts = self
            .entries
            .iter()
            .filter(|entry| {
                entry.state == ItemState::Running && entry.content != Content::TurnStatus
            })
            .map(|entry| entry.source.clone())
            .collect();
        self.completeness = if self
            .entries
            .iter()
            .any(|e| matches!(e.content, Content::Unsupported { .. }))
        {
            Completeness::Incomplete(IncompleteReason::UnsupportedItem)
        } else {
            completeness
        };
        self.revision = next_revision;
        Ok(())
    }
    /// Masks every entry again. Entries pass the choke point as they arrive; this covers the ones
    /// that did not: a history read back from the cache, or from a native pager.
    pub(in crate::daemon) fn redact_stored(&mut self) {
        for entry in &mut self.entries {
            super::redact::redact_entry(entry);
        }
    }
    pub fn durable(&self) -> Self {
        let mut copy = self.clone();
        copy.entries
            .retain(|entry| !copy.drafts.contains(&entry.source));
        if !copy.drafts.is_empty() {
            copy.completeness = Completeness::Incomplete(IncompleteReason::Disconnected);
        }
        copy.drafts.clear();
        copy
    }
    /// A page of entries.
    ///
    /// - No cursor, or a `next` cursor of a read still under way: the entries from a position on.
    ///   The cursor is stale once the history has changed, which a client answers by starting over.
    /// - A `resume` cursor (`delta`): only the entries added or changed since its revision, in
    ///   order, so a long history is followed without being read again. If the history was rebuilt
    ///   since (a different `epoch`) the page starts from the beginning and says `reset`.
    ///
    /// Every page carries the `resume` cursor for the read after it.
    pub fn read(
        &self,
        cursor: Option<&Cursor>,
        max_entries: usize,
        max_bytes: usize,
    ) -> Result<Page, ReadError> {
        if max_entries == 0
            || max_entries > MAX_READ_ENTRIES
            || max_bytes == 0
            || max_bytes > MAX_READ_BYTES
        {
            return Err(ReadError::InvalidLimit);
        }
        if cursor.is_some_and(|c| c.conversation != self.conversation) {
            return Err(ReadError::StaleCursor);
        }
        let mut reset = false;
        let mut delta = None;
        let mut position = 0;
        if let Some(c) = cursor {
            if c.delta {
                if c.epoch != self.track.epoch || c.revision > self.revision || c.snapshot > self.revision {
                    reset = true;
                } else {
                    // The first page of a delta read fixes the revision its later pages report from.
                    let snapshot = if c.snapshot == 0 { self.revision } else { c.snapshot };
                    delta = Some((c.revision, snapshot));
                    position = c.next_position.min(self.entries.len());
                }
            } else if c.revision != self.revision
                || c.next_position > self.entries.len()
                || (c.epoch != 0 && c.epoch != self.track.epoch)
            {
                return Err(ReadError::StaleCursor);
            } else {
                position = c.next_position;
            }
        }
        let wanted: Vec<usize> = (position..self.entries.len())
            .filter(|index| {
                delta.is_none_or(|(since, _)| {
                    self.track.changed_at.get(*index).copied().unwrap_or(0) > since
                })
            })
            .collect();
        let resume_from = delta.map_or(self.revision, |(_, snapshot)| snapshot);
        let resume = Cursor {
            generation: 0,
            conversation: self.conversation,
            revision: resume_from,
            next_position: 0,
            epoch: self.track.epoch,
            delta: true,
            snapshot: 0,
        };
        let more_from = |index: usize| Cursor {
            generation: 0,
            conversation: self.conversation,
            revision: delta.map_or(self.revision, |(since, _)| since),
            next_position: index,
            epoch: self.track.epoch,
            delta: delta.is_some(),
            snapshot: delta.map_or(0, |(_, snapshot)| snapshot),
        };
        let mut page = Page {
            entries: Vec::new(),
            next: None,
            completeness: self.completeness.clone(),
            resume: Some(resume),
            reset,
        };
        for (taken, index) in wanted.iter().take(max_entries).enumerate() {
            page.entries.push(self.entries[*index].clone());
            page.next = wanted.get(taken + 1).map(|following| more_from(*following));
            if serde_json::to_vec(&page)
                .map_err(|_| ReadError::EntryTooLarge)?
                .len()
                > max_bytes
            {
                page.entries.pop();
                if page.entries.is_empty() {
                    return Err(ReadError::EntryTooLarge);
                }
                page.next = Some(more_from(*index));
                break;
            }
        }
        if serde_json::to_vec(&page)
            .map_err(|_| ReadError::EntryTooLarge)?
            .len()
            > max_bytes
        {
            return Err(ReadError::EntryTooLarge);
        }
        Ok(page)
    }

    /// [`read`](Self::read) for a reader that holds a cursor of some generation: a cursor of
    /// another generation is not followed, the page starts over and says so. The cursors in the
    /// page carry `generation`.
    pub fn read_generation(
        &self,
        generation: u64,
        cursor: Option<&Cursor>,
        max_entries: usize,
        max_bytes: usize,
    ) -> Result<Page, ReadError> {
        let foreign = cursor.is_some_and(|c| c.generation != generation);
        let mut page = self.read(
            cursor.filter(|_| !foreign),
            max_entries,
            max_bytes,
        )?;
        page.reset |= foreign;
        for cursor in page.next.iter_mut().chain(page.resume.iter_mut()) {
            cursor.generation = generation;
        }
        Ok(page)
    }
}

// Each attachment owns its hydrated content; closing the last view retains no transcript.
#[derive(Default)]
pub(in crate::daemon) struct Attachments {
    histories: BTreeMap<u64, History>,
}
impl Attachments {
    pub fn attach(&mut self, attachment: u64, conversation: ConversationId) {
        self.histories
            .insert(attachment, History::new(conversation));
    }
    pub fn hydrate_native(
        &mut self,
        attachment: u64,
        history: History,
    ) -> Result<(), IncompleteReason> {
        let existing = self
            .histories
            .get_mut(&attachment)
            .ok_or(IncompleteReason::InvalidEvent)?;
        if existing.conversation != history.conversation {
            return Err(IncompleteReason::InvalidEvent);
        }
        let next_revision = existing
            .revision
            .max(history.revision)
            .checked_add(1)
            .ok_or(IncompleteReason::RevisionExhausted)?;
        existing.hydrate(history.entries, history.completeness)?;
        existing.revision = next_revision;
        Ok(())
    }
    pub fn hydrate_cached(
        &mut self,
        attachment: u64,
        mut history: History,
    ) -> Result<(), IncompleteReason> {
        // A complete committed revision cannot prove what happened while no view was attached.
        history.reconnect()?;
        self.hydrate_native(attachment, history)
    }
    pub fn detach(&mut self, attachment: u64) {
        self.histories.remove(&attachment);
    }
    pub fn get(&self, attachment: u64) -> Option<&History> {
        self.histories.get(&attachment)
    }
    pub fn notification(
        &mut self,
        conversation: ConversationId,
        thread: &str,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<(), IncompleteReason> {
        for history in self
            .histories
            .values()
            .filter(|h| h.conversation == conversation)
        {
            history.next_revision()?;
        }
        for history in self
            .histories
            .values_mut()
            .filter(|h| h.conversation == conversation)
        {
            match super::history_provider::notification(conversation, thread, method, params) {
                Ok(Some(update)) => history.apply(update)?,
                Ok(None) => (),
                Err(reason) => history.set_completeness(Completeness::Incomplete(reason))?,
            }
        }
        Ok(())
    }
    pub fn reconnect(&mut self, conversation: ConversationId) -> Result<(), IncompleteReason> {
        for history in self
            .histories
            .values()
            .filter(|h| h.conversation == conversation)
        {
            history.next_revision()?;
        }
        for history in self
            .histories
            .values_mut()
            .filter(|h| h.conversation == conversation)
        {
            history.reconnect()?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(in crate::daemon) mod tests;
