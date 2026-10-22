//! The saved session: enough to rebuild every workspace, tab and pane after the server (or
//! the machine) restarts. Written atomically, a few seconds after the model settles.

use crate::layout::Node;
use super::check;
use crate::protocol::{CheckInfo, SubagentActivity, TermId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// The account it ran as.
    #[serde(default)]
    pub account: Option<String>,
    /// Follow-ups that were waiting for its turn to end.
    #[serde(default)]
    pub queue: Vec<String>,
    /// Child activity records survive daemon restarts and completed turns.
    #[serde(default)]
    pub subagent_activity: Vec<SubagentActivity>,
    /// The latest check result; its output tail is kept beside the pane's output files.
    #[serde(default)]
    pub check: Option<CheckInfo>,
    /// The failed result the user overrode, kept so the override stays visible.
    #[serde(default)]
    pub check_overridden: Option<CheckInfo>,
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
    let _ = std::fs::remove_dir_all(check_output_dir());
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

/// Which of a pane's two kept check results an output tail belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckSlot {
    Latest,
    Overridden,
}

impl CheckSlot {
    fn file_suffix(self) -> &'static str {
        match self {
            CheckSlot::Latest => "latest",
            CheckSlot::Overridden => "overridden",
        }
    }
}

/// Where check output tails are kept: apart from `output_dir`, whose cleanup treats every
/// file in it as a pane's output.
pub fn check_output_dir() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join(format!("seshi-test-check-output-{}", std::process::id()));
    }
    let label = std::env::var("SESHI_SOCKET").unwrap_or_else(|_| "default".into());
    crate::config::data_dir().join(format!("check-output-{label}"))
}

fn check_output_file(dir: &Path, term: TermId, slot: CheckSlot) -> PathBuf {
    dir.join(format!("{term}-{}.txt", slot.file_suffix()))
}

/// What a folder of check tails has had written to it, so a slow background save can't put
/// an older tail over a newer one (as `Written` does for panes' output).
#[derive(Default)]
struct CheckWritten {
    tails: std::collections::HashMap<(TermId, CheckSlot), u64>,
    cleaned: u64,
}

static CHECK_WRITTEN: std::sync::Mutex<Option<std::collections::HashMap<PathBuf, CheckWritten>>> = std::sync::Mutex::new(None);

/// Keep each check output tail in `dir`, and drop the files of panes not in `live`. Saves
/// run one at a time; a tail already written by a newer `ticket` (from `save_ticket`) is left
/// alone. One file failing doesn't stop the rest. Returns the panes whose tail was not
/// written, so the caller can try again.
pub fn save_check_outputs(dir: &Path, ticket: u64, outputs: &[(TermId, CheckSlot, String)], live: &[TermId]) -> Vec<TermId> {
    let mut guard = CHECK_WRITTEN.lock().unwrap_or_else(|e| e.into_inner());
    let written = guard.get_or_insert_with(Default::default).entry(dir.to_path_buf()).or_default();
    let mut failed: Vec<TermId> = Vec::new();
    if !outputs.is_empty()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        tracing::warn!("saving checks' output: {e}");
        return outputs.iter().map(|(term, ..)| *term).collect();
    }
    for (term, slot, text) in outputs {
        if written.tails.get(&(*term, *slot)).is_some_and(|&t| t > ticket) {
            continue;
        }
        let path = check_output_file(dir, *term, *slot);
        let tmp = path.with_extension("tmp");
        match std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &path)) {
            Ok(()) => {
                written.tails.insert((*term, *slot), ticket);
            }
            Err(e) => {
                tracing::warn!("saving checks' output: {e}");
                failed.push(*term);
            }
        }
    }
    if ticket > written.cleaned {
        written.cleaned = ticket;
        written.tails.retain(|(id, _), _| live.contains(id));
        // A folder that was never made has nothing to prune.
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = e.file_name();
            let id = name.to_str().and_then(|n| n.split('-').next()).and_then(|n| n.parse::<TermId>().ok());
            if id.is_none_or(|id| !live.contains(&id)) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    failed.sort_unstable();
    failed.dedup();
    failed
}

/// A pane's kept check output. Left in place: the pane it is restored into writes its own
/// under its new id, and the next save drops this one, so a crash in between loses nothing.
pub fn read_check_output(dir: &Path, term: TermId, slot: CheckSlot) -> Option<String> {
    let bytes = std::fs::read(check_output_file(dir, term, slot)).ok()?;
    Some(check::cap_output(&String::from_utf8_lossy(&bytes)))
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

/// A folder of its own for one test, removed when the test ends however it ends.
#[cfg(test)]
pub(super) struct ScratchDir(PathBuf);

#[cfg(test)]
impl ScratchDir {
    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
pub(super) fn scratch_dir() -> ScratchDir {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    ScratchDir(std::env::temp_dir().join(format!("seshi-test-scratch-{}-{n}", std::process::id())))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn check_results_round_trip_and_older_panes_load_without_them() {
        let info = CheckInfo { command: "npm test".into(), state: crate::protocol::CheckState::Failed, exit_code: Some(1), ..Default::default() };
        let pane = SavedPane { check: Some(info.clone()), check_overridden: Some(info.clone()), ..Default::default() };
        let back: SavedPane = serde_json::from_str(&serde_json::to_string(&pane).unwrap()).unwrap();
        assert_eq!((back.check, back.check_overridden), (Some(info.clone()), Some(info)));
        let old: SavedPane = serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":null,"session":null}"#).expect("a pane saved before checks");
        assert_eq!((old.check, old.check_overridden), (None, None));
    }

    #[test]
    fn check_output_is_kept_per_slot_read_back_and_dropped_with_its_pane() {
        let dir = scratch_dir();
        let (kept, gone): (crate::protocol::TermId, crate::protocol::TermId) = (7, 8);
        let outputs = [
            (kept, CheckSlot::Latest, "FAIL a\n".to_string()),
            (kept, CheckSlot::Overridden, "FAIL b\n".to_string()),
            (gone, CheckSlot::Latest, "FAIL c\n".to_string()),
        ];
        assert!(save_check_outputs(dir.path(), save_ticket(), &outputs, &[kept, gone]).is_empty());
        assert!(save_check_outputs(dir.path(), save_ticket(), &[], &[kept]).is_empty());
        assert_eq!(read_check_output(dir.path(), gone, CheckSlot::Latest), None, "a closed pane's tail goes");
        assert_eq!(read_check_output(dir.path(), kept, CheckSlot::Overridden).as_deref(), Some("FAIL b\n"));
        assert_eq!(read_check_output(dir.path(), kept, CheckSlot::Latest).as_deref(), Some("FAIL a\n"));
        assert_eq!(read_check_output(dir.path(), kept, CheckSlot::Latest).as_deref(), Some("FAIL a\n"), "reading leaves it for the next restart");
    }

    #[test]
    fn a_slow_older_check_save_cannot_overwrite_a_newer_tail() {
        let dir = scratch_dir();
        let term: crate::protocol::TermId = 3;
        let (old, new) = (save_ticket(), save_ticket());
        assert!(save_check_outputs(dir.path(), new, &[(term, CheckSlot::Latest, "newer".into())], &[term]).is_empty());
        assert!(save_check_outputs(dir.path(), old, &[(term, CheckSlot::Latest, "older".into())], &[term]).is_empty());
        assert_eq!(read_check_output(dir.path(), term, CheckSlot::Latest).as_deref(), Some("newer"));
    }

    #[test]
    fn a_tail_that_cannot_be_written_is_reported_so_it_can_be_tried_again() {
        let dir = scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        let blocker = dir.path().join("not-a-folder");
        std::fs::write(&blocker, "x").unwrap();
        let failed = save_check_outputs(&blocker, save_ticket(), &[(4, CheckSlot::Latest, "t".into()), (5, CheckSlot::Overridden, "u".into())], &[4, 5]);
        assert_eq!(failed, [4, 5]);
    }

    #[test]
    fn two_folders_do_not_share_what_was_written() {
        let (a, b) = (scratch_dir(), scratch_dir());
        let term: crate::protocol::TermId = 6;
        let (old, new) = (save_ticket(), save_ticket());
        assert!(save_check_outputs(a.path(), new, &[(term, CheckSlot::Latest, "a".into())], &[term]).is_empty());
        assert!(save_check_outputs(b.path(), old, &[(term, CheckSlot::Latest, "b".into())], &[term]).is_empty());
        assert_eq!(read_check_output(b.path(), term, CheckSlot::Latest).as_deref(), Some("b"));
    }

    #[test]
    fn sessions_saved_with_workflow_runs_still_restore_the_workspace() {
        let saved: Saved = serde_json::from_str(r#"{"workflows":[{"id":7}],"workspaces":[],"active":0,"made_worktrees":[]}"#)
            .expect("unknown legacy workflow data is ignored");
        assert!(saved.workspaces.is_empty());
    }
}
