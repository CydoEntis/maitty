//! The conversations Claude and Codex saved for a folder (`Query::SavedConversations`): the id, the
//! first thing the person typed, and when the file was last written. Read only from each agent's own
//! store; the folder just picks which of its files count. Runs on a blocking thread.

use super::follow_transcript::{ClaudeLine, claude_line, codex_text, is_codex_context, without_paste_tags};
use super::tool_input::clean_text;
use crate::config::same_path;
use crate::protocol::{SAVED_CONVERSATIONS_MAX, SAVED_FIRST_MESSAGE_MAX_CHARS, SavedConversation, SavedConversations};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Most files looked at for one answer, newest first.
const FILES_MAX: usize = 500;
/// How much of a file's start is read for its first message: Codex writes about 150 KiB of its own
/// instructions before the person's first words.
const HEAD_MAX_BYTES: u64 = 1024 * 1024;
/// How much of a Codex file's first line is read for its folder and id.
const CODEX_META_MAX_BYTES: u64 = 64 * 1024;
/// The longest id passed on: it is what a later resume is given.
const ID_MAX_CHARS: usize = 128;

/// Where each agent keeps its saved conversations.
pub(in crate::daemon) struct Stores {
    /// Claude's `projects` folder: one folder per working folder, one `<session>.jsonl` each.
    pub claude: Option<PathBuf>,
    /// Codex's `sessions` folder: `YYYY/MM/DD/rollout-….jsonl`.
    pub codex: Option<PathBuf>,
}

impl Stores {
    pub fn of_this_user(home: Option<&Path>) -> Stores {
        let claude_root = std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| home.map(|home| home.join(".claude")));
        Stores {
            claude: claude_root.map(|root| root.join("projects")),
            codex: crate::daemon::usage::codex_sessions(),
        }
    }
}

/// The conversations `agent` saved for `folder`, newest first. A folder with none, or a store that
/// is not there, is an empty list; an agent whose conversations are not read is `NotSupported`.
pub(in crate::daemon) fn list(agent: &str, folder: &Path, stores: &Stores) -> SavedConversations {
    let found = match agent.trim().to_ascii_lowercase().as_str() {
        "claude" => stores.claude.as_deref().map(|root| claude(root, folder)),
        "codex" => stores.codex.as_deref().map(|root| codex(root, folder)),
        _ => return SavedConversations::NotSupported,
    };
    SavedConversations::Listed(found.unwrap_or_default())
}

fn claude(root: &Path, folder: &Path) -> Vec<SavedConversation> {
    let files = newest_first(jsonl_files(&root.join(claude_folder_name(folder))));
    files
        .into_iter()
        .filter_map(|(file, modified)| {
            let id = file.file_stem()?.to_str()?.to_owned();
            let head = read_or_log(&file, HEAD_MAX_BYTES)?;
            let first_message = claude_first_message(&head, folder)?;
            saved(id, first_message, modified)
        })
        .take(SAVED_CONVERSATIONS_MAX)
        .collect()
}

/// The folder Claude keeps a working folder's transcripts in: every character but an ASCII letter or
/// digit becomes `-` (`C:\dev\app` is `C--dev-app`). Two folders can share a name, so each
/// transcript's own `cwd` is checked too.
fn claude_folder_name(folder: &Path) -> String {
    folder
        .to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The first thing the person typed in a Claude transcript, when the transcript is `folder`'s.
fn claude_first_message(head: &str, folder: &Path) -> Option<String> {
    for line in complete_lines(head).filter_map(|line| serde_json::from_str::<Value>(line).ok()) {
        if line.get("type").and_then(Value::as_str) != Some("user")
            || line.get("isMeta").and_then(Value::as_bool) == Some(true)
            || line.get("isSidechain").and_then(Value::as_bool) == Some(true)
        {
            continue;
        }
        let cwd = line.get("cwd").and_then(Value::as_str)?;
        if !same_path(Path::new(cwd), folder) {
            return None;
        }
        let text = match line.pointer("/message/content") {
            Some(Value::String(text)) => text.clone(),
            Some(Value::Array(blocks)) => blocks
                .iter()
                .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect(),
            _ => continue,
        };
        if let ClaudeLine::Said = claude_line(&text)
            && let Some(said) = one_line(&without_paste_tags(&text))
        {
            return Some(said);
        }
    }
    None
}

fn codex(root: &Path, folder: &Path) -> Vec<SavedConversation> {
    newest_first(codex_files(root))
        .into_iter()
        .filter_map(|(file, modified)| {
            let meta = read_or_log(&file, CODEX_META_MAX_BYTES)?;
            let id = codex_id_for(complete_lines(&meta).next()?, folder)?;
            let head = read_or_log(&file, HEAD_MAX_BYTES)?;
            saved(id, codex_first_message(&head)?, modified)
        })
        .take(SAVED_CONVERSATIONS_MAX)
        .collect()
}

/// The session's id, when its first line says it ran in `folder` and a person started it (a
/// subagent's session is its parent's business).
fn codex_id_for(first_line: &str, folder: &Path) -> Option<String> {
    let line: Value = serde_json::from_str(first_line).ok()?;
    if line.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = line.get("payload")?;
    if payload.get("thread_source").and_then(Value::as_str) == Some("subagent") {
        return None;
    }
    let cwd = payload.get("cwd").and_then(Value::as_str)?;
    if !same_path(Path::new(cwd), folder) {
        return None;
    }
    payload.get("id").and_then(Value::as_str).map(str::to_owned)
}

fn codex_first_message(head: &str) -> Option<String> {
    complete_lines(head)
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|line| {
            let payload = line.get("payload")?;
            let said = match (
                line.get("type").and_then(Value::as_str),
                payload.get("type").and_then(Value::as_str),
            ) {
                (Some("event_msg"), Some("user_message")) => payload.get("message")?.as_str()?.to_owned(),
                (Some("response_item"), Some("message"))
                    if payload.get("role").and_then(Value::as_str) == Some("user") && !is_codex_context(payload) =>
                {
                    codex_text(payload)
                }
                _ => return None,
            };
            one_line(&said)
        })
}

/// Codex's session files, newest day first, at most `FILES_MAX`. Only real folders and files are
/// followed, so a link inside the store cannot lead out of it.
fn codex_files(root: &Path) -> Vec<(PathBuf, SystemTime)> {
    let mut files = Vec::new();
    for year in subfolders(root) {
        for month in subfolders(&year) {
            for day in subfolders(&month) {
                files.extend(jsonl_files(&day));
                if files.len() >= FILES_MAX {
                    return files;
                }
            }
        }
    }
    files
}

/// The folders directly inside `folder`, last name first (Codex names them by date).
fn subfolders(folder: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = entries(folder)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    found.sort_unstable_by(|a, b| b.cmp(a));
    found
}

/// The `.jsonl` files directly inside `folder`, with when each was last written.
fn jsonl_files(folder: &Path) -> Vec<(PathBuf, SystemTime)> {
    entries(folder)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|path| {
            let modified = std::fs::metadata(&path).and_then(|meta| meta.modified()).ok()?;
            Some((path, modified))
        })
        .collect()
}

fn entries(folder: &Path) -> impl Iterator<Item = std::fs::DirEntry> {
    std::fs::read_dir(folder).into_iter().flatten().filter_map(Result::ok)
}

fn newest_first(mut files: Vec<(PathBuf, SystemTime)>) -> Vec<(PathBuf, SystemTime)> {
    files.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    files.truncate(FILES_MAX);
    files
}

/// Up to `max` bytes from the start of `file`, as text. A file that cannot be read is skipped, and only
/// its path is logged: what it holds is the person's.
fn read_or_log(file: &Path, max: u64) -> Option<String> {
    let read = std::fs::File::open(file).and_then(|opened| {
        let mut bytes = Vec::new();
        opened.take(max).read_to_end(&mut bytes)?;
        Ok(bytes)
    });
    match read {
        Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        Err(error) => {
            tracing::warn!(file = %file.display(), kind = ?error.kind(), "saved conversations: skipped a file that could not be read");
            None
        }
    }
}

/// The lines of `text` that ended: the last one may have been cut by the read.
fn complete_lines(text: &str) -> impl Iterator<Item = &str> {
    let ended = text.rfind('\n').map_or("", |end| &text[..end]);
    ended.lines().map(|line| line.trim_end_matches('\r')).filter(|line| !line.is_empty())
}

/// `text` as one clean line: no escape sequences or control characters, runs of white space as one
/// space, cut to `SAVED_FIRST_MESSAGE_MAX_CHARS`. None when nothing is left.
fn one_line(text: &str) -> Option<String> {
    let clean = maitty_shared::hidden::sanitize(&clean_text(text), false);
    let joined = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = joined.chars().take(SAVED_FIRST_MESSAGE_MAX_CHARS).collect();
    (!cut.is_empty()).then_some(cut)
}

/// A listed conversation, when its id is one a resume can be given as is. A letter or digit comes
/// first: the id lands on the agent's command line, where a leading `-` would be read as a flag.
fn saved(id: String, first_message: String, modified: SystemTime) -> Option<SavedConversation> {
    let plain = id.starts_with(|c: char| c.is_ascii_alphanumeric())
        && id.chars().count() <= ID_MAX_CHARS
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then(|| SavedConversation {
        id,
        first_message,
        last_active: modified.duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_secs()),
    })
}

#[cfg(test)]
mod tests;
