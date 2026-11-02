//! Read-only questions about the repository a pane is in, and the answers.
//! Every request is scoped by the daemon to the pane's own checkout; none changes it.

use crate::conversation::{ConversationDiff, ConversationFileEntry};
use serde::{Deserialize, Serialize};

/// What to read. Paths are relative to the checkout root and may not climb out of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoRequest {
    /// Branch, upstream distance and the changed files.
    Status,
    /// One folder's entries (None: the root).
    Tree { path: Option<String> },
    /// One text file's contents.
    File { path: String },
    /// The working tree against HEAD, or one file of it.
    Diff { path: Option<String> },
    /// Two commits or branches against each other, or one file of that.
    DiffRange {
        from: String,
        to: String,
        path: Option<String>,
    },
    /// Local branches, the current one marked.
    Branches,
    /// The latest commits on HEAD, newest first.
    Log { limit: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoReply {
    Status(RepoStatus),
    Tree(Vec<ConversationFileEntry>),
    File(String),
    Diff(ConversationDiff),
    Branches(Vec<RepoBranch>),
    Log(Vec<RepoCommit>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoStatus {
    /// None when HEAD is detached.
    pub branch: Option<String>,
    /// Commits on HEAD the upstream lacks; 0 with no upstream.
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<RepoChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoChange {
    pub path: String,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoBranch {
    pub name: String,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoCommit {
    pub hash: String,
    pub subject: String,
    pub author: String,
    /// Unix seconds.
    pub time: i64,
}
