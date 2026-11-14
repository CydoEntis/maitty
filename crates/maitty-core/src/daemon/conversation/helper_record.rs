//! A helper's own record, read once for a view of its work (ADR 0010; what each provider leaves
//! is in docs/agent-status.md, "Each helper's own record").
//!
//! The helper is one the pane's own verified reports named, and its file is found from them, never
//! from what a client says: Claude's beside the session's transcript, Codex's by the helper's
//! thread, which must descend from the pane's. Reading is bounded and never changes a status.

use super::follow::{ReadFailure, checked_path};
use super::follow_transcript::{Format, Transcript};
use super::history::{Entry, History, MAX_READ_BYTES};
use crate::conversation::{ConversationId, Provider};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// The most of a helper's file read: its end, where its latest work is.
const MAX_HELPER_BYTES: u64 = 4 * 1024 * 1024;
/// Lines longer than this are skipped, as in a followed transcript.
const MAX_LINE_BYTES: usize = 1024 * 1024;
/// The longest helper id taken from a pane's reports.
const HELPER_ID_MAX: usize = 128;
/// A helper's entries belong to no conversation of the daemon's; their ids name this one.
const HELPER_CONVERSATION: ConversationId = ConversationId(0);

/// Where a helper's record is, from what the pane's own verified reports said.
pub(in crate::daemon) enum HelperFile {
    /// `<session>/subagents/agent-<helper>.jsonl`, beside the session's `<session>.jsonl`.
    Claude {
        transcript: PathBuf,
        session: String,
        helper: String,
    },
    /// The helper's own rollout; `find` gives its file only if the helper descends from `parent`.
    Codex { parent: String, helper: String },
}

/// What a helper did, newest last.
#[derive(Debug)]
pub(in crate::daemon) struct Record {
    pub entries: Vec<Entry>,
    /// The file held more than is shown: its start is left out.
    pub start_missing: bool,
}

/// Reads a helper's record. Runs on a blocking thread. `find_codex(parent, helper)` is the Codex
/// helper's file when it descends from `parent`.
pub(in crate::daemon) fn read(
    file: HelperFile,
    home: &Path,
    find_codex: impl FnOnce(&str, &str) -> Option<PathBuf>,
) -> Result<Record, ReadFailure> {
    let (path, format, helper) = match file {
        HelperFile::Claude {
            transcript,
            session,
            helper,
        } => {
            if !is_helper_id(&helper) {
                return Err(ReadFailure::Refused("The helper id is not one an agent gives"));
            }
            let parent = checked_path(&transcript, Provider::Claude, &session, home)?;
            let expected = parent
                .with_extension("")
                .join("subagents")
                .join(format!("agent-{helper}.jsonl"));
            let real = std::fs::canonicalize(&expected).map_err(|_| ReadFailure::NotFound)?;
            // A link out of the session's folder is not the helper's record.
            if real != expected || !real.is_file() {
                return Err(ReadFailure::Refused("The helper's record is not beside its session's"));
            }
            (real, Format::Claude, helper)
        }
        HelperFile::Codex { parent, helper } => {
            if !is_helper_id(&helper) {
                return Err(ReadFailure::Refused("The helper id is not one an agent gives"));
            }
            let found = find_codex(&parent, &helper).ok_or(ReadFailure::NotFound)?;
            (checked_path(&found, Provider::Codex, &helper, home)?, Format::Codex, helper)
        }
    };
    read_tail(&path, Transcript::helper(format, &helper), &helper)
}

fn is_helper_id(id: &str) -> bool {
    (1..=HELPER_ID_MAX).contains(&id.len())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn read_tail(path: &Path, mut parser: Transcript, thread: &str) -> Result<Record, ReadFailure> {
    let mut file = std::fs::File::open(path).map_err(|_| ReadFailure::Io)?;
    let len = file.metadata().map_err(|_| ReadFailure::Io)?.len();
    let from = len.saturating_sub(MAX_HELPER_BYTES);
    file.seek(SeekFrom::Start(from)).map_err(|_| ReadFailure::Io)?;
    let mut buffer = Vec::new();
    file.take(MAX_HELPER_BYTES)
        .read_to_end(&mut buffer)
        .map_err(|_| ReadFailure::Io)?;
    let mut start = 0;
    if from > 0 {
        // The tail starts mid-line.
        start = buffer.iter().position(|b| *b == b'\n').map_or(buffer.len(), |n| n + 1);
    }
    let mut history = History::new(HELPER_CONVERSATION);
    let mut offset = from + start as u64;
    for line in buffer[start..].split(|b| *b == b'\n') {
        let at = offset;
        offset += line.len() as u64 + 1;
        if line.is_empty() || line.len() > MAX_LINE_BYTES {
            continue;
        }
        let text = String::from_utf8_lossy(line);
        for note in parser.feed(at, text.trim_end_matches('\r')) {
            if let Ok(Some(update)) =
                super::history_provider::notification(HELPER_CONVERSATION, thread, note.method, &note.params)
            {
                // A note history cannot take is left out; the rest of the record still shows.
                let _ = history.apply(update);
            }
        }
    }
    let (entries, cut) = newest_within(history.entries, MAX_READ_BYTES);
    Ok(Record {
        entries,
        start_missing: from > 0 || cut,
    })
}

/// The newest entries whose encoded size fits in `budget`, oldest first, and whether any were left out.
fn newest_within(entries: Vec<Entry>, budget: usize) -> (Vec<Entry>, bool) {
    let mut used = 0;
    let mut kept = Vec::new();
    let total = entries.len();
    for entry in entries.into_iter().rev() {
        used += serde_json::to_vec(&entry).map_or(usize::MAX, |bytes| bytes.len());
        if used > budget {
            break;
        }
        kept.push(entry);
    }
    let cut = kept.len() < total;
    kept.reverse();
    (kept, cut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::history::{Content, Role};

    fn texts(record: &Record) -> Vec<String> {
        record
            .entries
            .iter()
            .filter_map(|e| match &e.content {
                Content::Message { role, text } => Some(format!("{role:?}: {text}")),
                _ => None,
            })
            .collect()
    }

    fn claude_line(kind: &str, uuid: &str, content: &str) -> String {
        let message = if kind == "user" {
            serde_json::json!({ "role": "user", "content": content })
        } else {
            serde_json::json!({ "role": "assistant", "content": [{ "type": "text", "text": content }] })
        };
        serde_json::json!({
            "type": kind, "isSidechain": true, "agentId": "a1b2", "uuid": uuid,
            "timestamp": "2026-11-13T19:43:52.811Z", "message": message,
        })
        .to_string()
    }

    #[test]
    fn a_claude_helper_is_read_from_beside_its_session() {
        let home = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(home.path()).unwrap();
        let project = home.path().join("projects").join("p");
        std::fs::create_dir_all(project.join("S1").join("subagents")).unwrap();
        std::fs::write(project.join("S1.jsonl"), "").unwrap();
        let lines = [claude_line("user", "u1", "read a.txt"), claude_line("assistant", "a1", "alpha")];
        std::fs::write(project.join("S1/subagents/agent-a1b2.jsonl"), lines.join("\n") + "\n").unwrap();
        let file = |helper: &str| HelperFile::Claude {
            transcript: project.join("S1.jsonl"),
            session: "S1".into(),
            helper: helper.into(),
        };
        let record = read(file("a1b2"), home.path(), |_, _| None).unwrap();
        assert_eq!(texts(&record), ["User: read a.txt", "Assistant: alpha"]);
        assert!(!record.start_missing);
        assert!(matches!(read(file("../S1"), home.path(), |_, _| None), Err(ReadFailure::Refused(_))));
        assert!(matches!(read(file("ffff"), home.path(), |_, _| None), Err(ReadFailure::NotFound)));
        let other = HelperFile::Claude {
            transcript: project.join("S1.jsonl"),
            session: "S2".into(),
            helper: "a1b2".into(),
        };
        assert!(
            matches!(read(other, home.path(), |_, _| None), Err(ReadFailure::Refused(_))),
            "only beside the pane's own session"
        );
    }

    #[test]
    fn a_codex_helper_shows_its_own_task_and_not_its_parents_copied_history() {
        let home = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(home.path()).unwrap();
        let file = home.path().join("rollout-2026-11-13T14-44-37-child9.jsonl");
        let message = |role: &str, text: &str| {
            let kind = if role == "user" { "input_text" } else { "output_text" };
            serde_json::json!({ "type": "response_item", "payload": { "type": "message", "role": role,
                "content": [{ "type": kind, "text": text }] } })
            .to_string()
        };
        let lines = [
            r#"{"type":"session_meta","payload":{"id":"child9","parent_thread_id":"root1"}}"#.to_string(),
            message("user", "the parent's own question"),
            message("assistant", "the parent's own answer"),
            r#"{"type":"inter_agent_communication_metadata","payload":{"trigger_turn":true}}"#.to_string(),
            message("assistant", "a.txt contains alpha"),
        ];
        std::fs::write(&file, lines.join("\n") + "\n").unwrap();
        let asked = std::cell::RefCell::new(None);
        let record = read(
            HelperFile::Codex { parent: "root1".into(), helper: "child9".into() },
            home.path(),
            |parent, helper| {
                *asked.borrow_mut() = Some((parent.to_string(), helper.to_string()));
                Some(file.clone())
            },
        )
        .unwrap();
        assert_eq!(asked.into_inner(), Some(("root1".into(), "child9".into())));
        assert_eq!(texts(&record), ["Assistant: a.txt contains alpha"]);
        let not_its = read(
            HelperFile::Codex { parent: "root1".into(), helper: "other7".into() },
            home.path(),
            |_, _| Some(file.clone()),
        );
        assert!(matches!(not_its, Err(ReadFailure::Refused(_))), "the file must be the helper's own");
    }

    #[test]
    fn a_long_record_keeps_its_newest_entries() {
        let entry = |n: u32| Entry {
            source: crate::conversation::history::SourceId {
                conversation: HELPER_CONVERSATION,
                turn: "t".into(),
                item: n.to_string(),
            },
            content: Content::Message { role: Role::Assistant, text: "x".repeat(100) },
            state: crate::conversation::history::ItemState::Completed,
            started_at_ms: None,
            completed_at_ms: None,
        };
        let one = serde_json::to_vec(&entry(0)).unwrap().len();
        let (kept, cut) = newest_within((0..10).map(entry).collect(), one * 3);
        assert!(cut);
        assert_eq!(kept.iter().map(|e| e.source.item.as_str()).collect::<Vec<_>>(), ["7", "8", "9"]);
    }
}
