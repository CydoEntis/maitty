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
    Message { role: Role, text: String },
    Tool { kind: String, output: String },
    Unsupported { kind: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub source: SourceId,
    pub content: Content,
    pub state: ItemState,
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
    pub(crate) generation: u64,
    pub(crate) conversation: ConversationId,
    pub(crate) revision: u64,
    pub(crate) next_position: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub entries: Vec<Entry>,
    pub next: Option<Cursor>,
    pub completeness: Completeness,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    StaleCursor,
    InvalidLimit,
    EntryTooLarge,
}
