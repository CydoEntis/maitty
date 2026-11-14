//! A project's specialist agent files, as Claude Code and Codex keep them, listed read-only (ADR
//! 0010). Maitty never creates, edits or runs them; the providers' own tools do.

use crate::protocol::TermId;
use serde::{Deserialize, Serialize};

/// The most files one listing carries; more are left out and `truncated` says so.
pub const SPECIALISTS_MAX: usize = 200;
/// The most of one file's text a listing carries, in bytes; a longer file is cut and says so.
pub const SPECIALIST_TEXT_MAX: usize = 32 * 1024;
/// The longest name or description kept, in characters.
pub const SPECIALIST_FIELD_MAX: usize = 300;

/// Which tool reads the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialistTool {
    Claude,
    Codex,
    #[serde(other)]
    Unknown,
}

/// Where the file lives: the pane's project, or the user's home, which every project shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialistScope {
    Project,
    Home,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialistFile {
    pub tool: SpecialistTool,
    pub scope: SpecialistScope,
    /// The agent's name from the file, else the file's own name without its extension.
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Where the file is, on the pane's machine.
    pub path: String,
    /// The file as it is, up to `SPECIALIST_TEXT_MAX` bytes.
    pub text: String,
    /// The file is longer than `text`.
    #[serde(default)]
    pub text_cut: bool,
}

/// The specialist files a pane's project and its user's home hold. Project files come first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialistList {
    pub term: TermId,
    /// The folder the project's files were looked for in.
    pub project: String,
    pub files: Vec<SpecialistFile>,
    /// There were more than `SPECIALISTS_MAX` files.
    #[serde(default)]
    pub truncated: bool,
}
