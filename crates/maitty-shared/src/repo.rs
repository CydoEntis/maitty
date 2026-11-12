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
    /// Where a path seen in the pane's output is in this repository: absolute, or relative to
    /// the pane's folder. Refused when it is not a file inside the repository.
    Locate { path: String },
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
    /// Every worktree of the repository, open in the app or not.
    Worktrees,
    /// What the daemon saved of the checkout before each agent turn, newest first.
    Snapshots,
    /// The project's reference files (`.maitty/references/`), newest first.
    References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RepoReply {
    Status(RepoStatus),
    Tree(Vec<ConversationFileEntry>),
    File(String),
    /// The repository-relative path (with `/`) that `Locate` found.
    Located(String),
    Diff(ConversationDiff),
    Branches(Vec<RepoBranch>),
    Log(Vec<RepoCommit>),
    Worktrees(Vec<crate::protocol::WorktreeEntry>),
    Snapshots(Vec<RepoSnapshot>),
    References(Vec<RepoReference>),
    /// A restore finished: how many files changed.
    Restored(usize),
    /// A file was written.
    Written,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoSnapshot {
    pub commit: String,
    /// Unix seconds.
    pub at: u64,
    /// What it was taken before ("before claude's turn").
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoReference {
    /// Relative to the references folder.
    pub name: String,
    /// "image", "pdf", "text" and so on, as the terminal app labels it.
    pub kind: String,
    pub bytes: u64,
    /// Unix seconds.
    pub modified: u64,
}
