//! The saved session: enough to rebuild every workspace, tab and pane after the server (or
//! the machine) restarts. Written atomically, a few seconds after the model settles.

use crate::layout::Node;
use crate::protocol::{SubagentActivity, TermId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Saved {
    pub workspaces: Vec<SavedWs>,
    /// Index into `workspaces`.
    pub active: usize,
    /// Worktrees seshi created (the only ones it may remove by itself).
    #[serde(default)]
    pub made_worktrees: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedWs {
    pub name: String,
    pub cwd: PathBuf,
    #[serde(default)]
    pub worktree: bool,
    #[serde(default)]
    pub color: Option<u8>,
    #[serde(default)]
    pub group: Option<String>,
    pub tabs: Vec<SavedTab>,
    pub active_tab: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedTab {
    pub name: String,
    /// Uses the pane ids of the previous run; remapped on restore.
    pub layout: Node,
    pub focus: TermId,
    pub panes: BTreeMap<TermId, SavedPane>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedPane {
    pub cwd: Option<PathBuf>,
    /// The command the pane was started with, if not a plain shell.
    pub cmd: Option<String>,
    pub agent: Option<String>,
    /// Agent session id reported by hooks, for `resume`.
    pub session: Option<String>,
    /// Finished and not looked at yet.
    #[serde(default)]
    pub unseen: bool,
    /// What it was called (its /rename, else its first prompt) and its model.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub model: String,
    /// The name you gave it (rename), which wins over everything else.
    #[serde(default)]
    pub label: String,
    /// The pane that started it, by its id when this was saved (a key of some tab's `panes`).
    #[serde(default)]
    pub started_by: Option<TermId>,
    /// Set aside: settled, or snoozed until then.
    #[serde(default)]
    pub aside: Option<u64>,
    /// The account it ran as.
    #[serde(default)]
    pub account: Option<String>,
    /// Follow-ups that were waiting for its turn to end.
    #[serde(default)]
    pub queue: Vec<String>,
    /// Child activity records survive daemon restarts and completed turns.
    #[serde(default)]
    pub subagent_activity: Vec<SubagentActivity>,
}

pub fn path() -> PathBuf {
    let label = std::env::var("SESHI_SOCKET").unwrap_or_else(|_| "default".into());
    crate::config::data_dir().join(format!("session-{label}.json"))
}

pub fn load() -> Option<Saved> {
    let s = std::fs::read_to_string(path()).ok()?;
    match serde_json::from_str(&s) {
        Ok(saved) => Some(saved),
        Err(e) => {
            tracing::warn!("ignoring unreadable session file: {e}");
            None
        }
    }
}

pub fn save(saved: &Saved) -> Result<()> {
    if cfg!(test) {
        return Ok(());
    }
    let path = path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(saved)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn forget() {
    if cfg!(test) {
        return;
    }
    let _ = std::fs::remove_file(path());
    let _ = std::fs::remove_dir_all(output_dir());
}

/// Where each pane's recent output is kept, so its history is still there after the server
/// restarts (an update, a reboot): one file per pane, named by its id.
pub fn output_dir() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join(format!("seshi-test-output-{}", std::process::id()));
    }
    let label = std::env::var("SESHI_SOCKET").unwrap_or_else(|_| "default".into());
    crate::config::data_dir().join(format!("output-{label}"))
}

fn output_file(term: TermId) -> PathBuf {
    output_dir().join(format!("{term}.bin"))
}

/// Which save last wrote each pane, so a slow background save can't put older output over a
/// newer one. `cleaned` is the newest save that has pruned the folder.
#[derive(Default)]
struct Written {
    panes: std::collections::HashMap<TermId, u64>,
    cleaned: u64,
}

static WRITTEN: std::sync::Mutex<Option<Written>> = std::sync::Mutex::new(None);
static TICKETS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// A number for a save, taken when its output is gathered: a higher one is a newer picture.
pub fn save_ticket() -> u64 {
    TICKETS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Keep `bytes` as pane `term`'s output, and drop the files of panes not in `live`. Saves run
/// one at a time; a pane already written by a newer `ticket` is left alone. One pane failing
/// doesn't stop the rest; the first error is returned after all were tried.
pub fn save_outputs(ticket: u64, outputs: &[(TermId, Vec<u8>)], live: &[TermId]) -> Result<()> {
    use std::io::Write;
    let mut guard = WRITTEN.lock().unwrap_or_else(|e| e.into_inner());
    let written = guard.get_or_insert_with(Written::default);
    let dir = output_dir();
    std::fs::create_dir_all(&dir)?;
    let mut first_err: Option<anyhow::Error> = None;
    for (term, bytes) in outputs {
        if written.panes.get(term).is_some_and(|&t| t > ticket) {
            continue;
        }
        let path = output_file(*term);
        let tmp = path.with_extension("tmp");
        // Synced before the rename, so a power cut leaves the old file or the new one, never
        // an empty one.
        let result = (|| -> std::io::Result<()> {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            std::fs::rename(&tmp, &path)
        })();
        match result {
            Ok(()) => {
                written.panes.insert(*term, ticket);
            }
            Err(e) => {
                first_err.get_or_insert(e.into());
            }
        }
    }
    if ticket > written.cleaned {
        written.cleaned = ticket;
        written.panes.retain(|id, _| live.contains(id));
        for e in std::fs::read_dir(&dir)?.flatten() {
            let id = e.path().file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<TermId>().ok());
            if id.is_none_or(|id| !live.contains(&id)) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    first_err.map_or(Ok(()), Err)
}

/// A pane's kept output, taken (its file goes: the pane it's restored into saves its own).
pub fn take_output(term: TermId) -> Option<Vec<u8>> {
    let path = output_file(term);
    let bytes = std::fs::read(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::Saved;

    #[test]
    fn a_slow_older_save_cannot_overwrite_a_newer_one() {
        let term: crate::protocol::TermId = 9_001;
        let old = super::save_ticket();
        let new = super::save_ticket();
        super::save_outputs(new, &[(term, b"newer".to_vec())], &[term]).unwrap();
        super::save_outputs(old, &[(term, b"older".to_vec())], &[term]).unwrap();
        assert_eq!(super::take_output(term).as_deref(), Some(b"newer".as_slice()));
    }

    #[test]
    fn sessions_saved_with_workflow_runs_still_restore_the_workspace() {
        let saved: Saved = serde_json::from_str(r#"{"workflows":[{"id":7}],"workspaces":[],"active":0,"made_worktrees":[]}"#)
            .expect("unknown legacy workflow data is ignored");
        assert!(saved.workspaces.is_empty());
    }
}
