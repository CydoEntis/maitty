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
}
impl History {
    pub fn new(conversation: ConversationId) -> Self {
        Self {
            conversation,
            revision: 1,
            entries: Vec::new(),
            completeness: Completeness::Unknown,
            drafts: Vec::new(),
        }
    }
    pub fn apply(&mut self, update: Update) -> Result<(), IncompleteReason> {
        let next_revision = self.next_revision()?;
        let changed = match update {
            Update::ProviderError { .. } => false,
            Update::UpsertItem(entry) => self.upsert(entry),
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
                    for entry in &mut self.entries {
                        if entry.source.turn == turn
                            && entry.content != Content::TurnStatus
                            && entry.state == ItemState::Running
                        {
                            entry.state = state.clone();
                            changed = true;
                        }
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
        if let Some(old) = self
            .entries
            .iter_mut()
            .find(|old| old.source == entry.source)
        {
            // A completion event carries no start time; the row keeps the one its start event set.
            entry.started_at_ms = entry.started_at_ms.or(old.started_at_ms);
            if *old == entry {
                return incomplete;
            }
            *old = entry;
        } else {
            self.entries.push(entry);
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
        if let Some(Entry {
            content: Content::Message { text, .. },
            ..
        }) = self.entries.iter_mut().find(|old| old.source == source)
        {
            text.push_str(delta);
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
        self.entries
            .retain(|entry| !self.drafts.contains(&entry.source));
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
        let position = match cursor {
            Some(c)
                if c.conversation != self.conversation
                    || c.revision != self.revision
                    || c.next_position > self.entries.len() =>
            {
                return Err(ReadError::StaleCursor);
            }
            Some(c) => c.next_position,
            None => 0,
        };
        let mut page = Page {
            entries: Vec::new(),
            next: None,
            completeness: self.completeness.clone(),
        };
        for entry in self.entries.iter().skip(position).take(max_entries) {
            page.entries.push(entry.clone());
            let next = position + page.entries.len();
            page.next = (next < self.entries.len()).then_some(Cursor {
                generation: 0,
                conversation: self.conversation,
                revision: self.revision,
                next_position: next,
            });
            if serde_json::to_vec(&page)
                .map_err(|_| ReadError::EntryTooLarge)?
                .len()
                > max_bytes
            {
                page.entries.pop();
                if page.entries.is_empty() {
                    return Err(ReadError::EntryTooLarge);
                }
                page.next = Some(Cursor {
                    generation: 0,
                    conversation: self.conversation,
                    revision: self.revision,
                    next_position: next - 1,
                });
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
