use super::ConversationId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceId {
    pub conversation: ConversationId,
    pub turn: String,
    pub item: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemState {
    Running,
    Completed,
    Failed,
    Interrupted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content {
    TurnStatus,
    Message {
        role: Role,
        text: String,
    },
    Tool {
        kind: String,
        output: String,
        #[serde(default)]
        label: String,
        /// The tool's own input, bounded and with terminal escapes stripped: an edit's file and
        /// text, a to-do list, a helper agent's title. Absent when the provider sends none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input: Option<serde_json::Value>,
        /// The command's exit status, when the provider reports one (Codex does, Claude does not).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i64>,
        /// How long the provider says the tool ran.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    Unsupported {
        kind: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub source: SourceId,
    pub content: Content,
    pub state: ItemState,
    /// Unix milliseconds when the item started, as the provider (or, for Claude, the daemon as
    /// it saw the event) recorded it. Absent for items read back from a provider's saved history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<u64>,
    /// Unix milliseconds when the item reached a final state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncompleteReason {
    Disconnected,
    UnsupportedItem,
    InvalidEvent,
    MissingPages,
    RevisionExhausted,
}
impl std::fmt::Display for IncompleteReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for IncompleteReason {}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Completeness {
    Unknown,
    Loading,
    Complete,
    Incomplete(IncompleteReason),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    #[serde(default)]
    pub generation: u64,
    pub conversation: ConversationId,
    pub revision: u64,
    pub next_position: usize,
    /// Which run of the daemon's history this cursor belongs to; 0 from a client that predates it.
    /// A history that is rebuilt (a reconnect that drops drafts, a new hydration) gets a new one, so
    /// a `delta` cursor from before it is answered with a reset instead of silently missing entries.
    #[serde(default)]
    pub epoch: u64,
    /// A `resume` cursor (see [`Page::resume`]): the entries that changed after `revision`, not a
    /// place in the list.
    #[serde(default)]
    pub delta: bool,
    /// While paging through a delta read: the revision the read began at. The next `resume` cursor
    /// starts there, so a change made while the pages were being fetched is not missed.
    #[serde(default)]
    pub snapshot: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub entries: Vec<Entry>,
    pub next: Option<Cursor>,
    pub completeness: Completeness,
    /// Pass this to the next read to get only the entries that were added or changed since this
    /// page. Entries are updated in place (a tool finishing, a message growing), so a position in
    /// the list is not enough; a client that applies entries by their source id and reads from here
    /// follows a long history without reading it again. Absent from a daemon that predates it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<Cursor>,
    /// The cursor could not be followed (the history was rebuilt, or the conversation started over):
    /// this page starts from the beginning and the client discards what it held.
    #[serde(default, skip_serializing_if = "is_false")]
    pub reset: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    StaleCursor,
    InvalidLimit,
    EntryTooLarge,
}

#[cfg(test)]
mod tests {
    use super::Content;
    use serde_json::json;

    #[test]
    fn old_tool_history_without_a_label_still_decodes() {
        let content: Content = serde_json::from_value(json!({
            "Tool": { "kind": "commandExecution", "output": "completed" }
        }))
        .unwrap();

        assert_eq!(
            content,
            Content::Tool {
                kind: "commandExecution".into(),
                output: "completed".into(),
                label: String::new(),
                input: None,
                exit_code: None,
                duration_ms: None,
            }
        );
    }

    #[test]
    fn old_entries_without_timestamps_still_decode() {
        let entry: super::Entry = serde_json::from_value(json!({
            "source": { "conversation": 1, "turn": "t", "item": "i" },
            "content": { "Tool": { "kind": "commandExecution", "output": "", "label": "ls" } },
            "state": "Completed"
        }))
        .unwrap();

        assert_eq!(entry.started_at_ms, None);
        assert_eq!(entry.completed_at_ms, None);
    }

    #[test]
    fn absent_tool_detail_is_not_written() {
        let content = Content::Tool {
            kind: "commandExecution".into(),
            output: String::new(),
            label: "ls".into(),
            input: None,
            exit_code: None,
            duration_ms: None,
        };

        assert_eq!(
            serde_json::to_value(&content).unwrap(),
            json!({ "Tool": { "kind": "commandExecution", "output": "", "label": "ls" } })
        );
    }
}
