//! The saved session: enough to rebuild every workspace, tab and pane after the server (or
//! the machine) restarts. Written atomically, a few seconds after the model settles.

use super::check;
use crate::layout::Node;
use crate::protocol::{CheckInfo, SubagentActivity, TermId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Saved {
    #[serde(default)]
    pub conversations: Vec<crate::conversation::Conversation>,
    #[serde(default)]
    pub pending_starts: Vec<crate::conversation::PendingStart>,
    #[serde(default)]
    pub conversation_operations: Vec<crate::conversation::Operation>,
    /// When the daemon told a resumed conversation to continue; see `continue_turn`.
    #[serde(default)]
    pub turn_continues: super::conversation::continue_turn::Continues,
    /// Chat conversations that were in a turn when this was saved: what a restart cut.
    #[serde(default)]
    pub turns_in_flight: Vec<crate::conversation::ConversationId>,
    pub workspaces: Vec<SavedWs>,
    /// Index into `workspaces`.
    pub active: usize,
    /// Worktrees the app created (the only ones it may remove by itself).
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
    #[serde(default)]
    pub conversation_id: Option<crate::conversation::ConversationId>,
    pub cwd: Option<PathBuf>,
    /// The command the pane was started with, if not a plain shell.
    pub cmd: Option<String>,
    pub agent: Option<String>,
    /// Agent session id reported by hooks, for `resume`.
    pub session: Option<String>,
    /// Finished and not looked at yet.
    #[serde(default)]
    pub unseen: bool,
    /// It was in the middle of a turn when this was saved.
    #[serde(default)]
    pub was_working: bool,
    /// What it was called (its /rename, else its first prompt) and its model.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub model: String,
    /// The name you gave it (rename), which wins over everything else.
    #[serde(default)]
    pub label: String,
    /// Pinned to the top of its project, and put away from the lists.
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    /// Snoozed until this time (unix seconds).
    #[serde(default)]
    pub snoozed_until: Option<u64>,
    /// The pane that started it, by its id when this was saved (a key of some tab's `panes`).
    #[serde(default)]
    pub started_by: Option<TermId>,
    /// The app that started it: only that app may use it after a restart (ADR 0004).
    #[serde(default)]
    pub owner: crate::protocol::Surface,
    /// The session whose tab it is in, by its id when this was saved; none: its own.
    #[serde(default)]
    pub tab_of: Option<TermId>,
    /// The account it ran as.
    #[serde(default)]
    pub account: Option<String>,
    /// Follow-ups that were waiting for its turn to end.
    #[serde(default)]
    pub queue: Vec<String>,
    /// A message that was waiting for the agent starting here to take input.
    #[serde(default)]
    pub held_message: Option<String>,
    /// Child activity records survive daemon restarts and completed turns.
    #[serde(default)]
    pub subagent_activity: Vec<SubagentActivity>,
    /// The latest result of each checkpoint; their output tails are kept beside the pane's
    /// output files. A pane saved with a single `check` loads it as one checkpoint.
    #[serde(
        default,
        alias = "check",
        deserialize_with = "crate::protocol::one_or_many_checks"
    )]
    pub checks: Vec<CheckInfo>,
    /// The failed results the user overrode, kept so each override stays visible.
    #[serde(
        default,
        alias = "check_overridden",
        deserialize_with = "crate::protocol::one_or_many_checks"
    )]
    pub checks_overridden: Vec<CheckInfo>,
    /// The worktree's tree id each checkpoint's latest run saw, by checkpoint name, so a pass
    /// can still go stale after a restart.
    #[serde(default)]
    pub check_trees: BTreeMap<String, String>,
}

pub fn path() -> PathBuf {
    let label =
        std::env::var(concat!(env!("APP_ENV"), "_SOCKET")).unwrap_or_else(|_| "default".into());
    crate::config::data_dir().join(format!("session-{label}.json"))
}

pub fn load() -> Option<Saved> {
    let s = std::fs::read_to_string(path()).ok()?;
    match serde_json::from_str(&s) {
        Ok(saved) => Some(saved),
        Err(e) => {
            tracing::warn!("ignoring unreadable session file: {e}");
            keep_unusable_now(&path());
            None
        }
    }
}

/// How many copies of a session file the daemon could not use are kept.
const KEPT_UNUSABLE: usize = 3;

/// Copy a session file the daemon could not use aside. The daemon then starts empty and its next
/// save would replace the file, losing whatever the person had open.
pub(crate) fn keep_unusable_now(path: &Path) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    match keep_unusable(path, stamp) {
        Ok(kept) => tracing::warn!("kept the session file it could not use at {}", kept.display()),
        Err(e) => tracing::warn!("could not keep the unusable session file: {e:#}"),
    }
}

fn keep_unusable(path: &Path, stamp: u64) -> Result<PathBuf> {
    use anyhow::Context as _;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .context("session file name")?;
    let prefix = format!("{name}.unusable-");
    let kept = path.with_file_name(format!("{prefix}{stamp:020}"));
    std::fs::copy(path, &kept).context("copying the unusable session file")?;
    let dir = path.parent().context("session directory")?;
    let mut copies: Vec<PathBuf> = std::fs::read_dir(dir)
        .context("listing kept session files")?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix))
        })
        .collect();
    copies.sort();
    let surplus = copies.len().saturating_sub(KEPT_UNUSABLE);
    for old in copies.into_iter().take(surplus) {
        let _ = std::fs::remove_file(old);
    }
    Ok(kept)
}

#[cfg(test)]
mod keep_unusable_tests {
    use super::*;

    #[test]
    fn an_unusable_session_file_is_copied_aside_and_only_the_newest_copies_stay() {
        let dir = std::env::temp_dir().join(format!("keep-unusable-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("session-test.json");
        std::fs::write(&file, "the person's layout").unwrap();
        let kept: Vec<PathBuf> = (1..=4).map(|n| keep_unusable(&file, n).unwrap()).collect();
        let left = |p: &PathBuf| p.exists();
        let contents = std::fs::read_to_string(&kept[3]).unwrap();
        let original = std::fs::read_to_string(&file).unwrap();
        let remaining: Vec<bool> = kept.iter().map(left).collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(contents, "the person's layout", "the copy holds the original");
        assert_eq!(original, "the person's layout", "the original is untouched");
        assert_eq!(
            remaining,
            [false, true, true, true],
            "the oldest copy is dropped"
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionCommit {
    Committed,
    Superseded { committed_ticket: u64 },
}

static SESSION_TICKETS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
#[derive(Default)]
struct SessionWritten {
    replaced_ticket: u64,
    committed_ticket: u64,
}

type SessionWriters = BTreeMap<PathBuf, std::sync::Arc<std::sync::Mutex<SessionWritten>>>;
static SESSION_WRITERS: std::sync::Mutex<Option<SessionWriters>> = std::sync::Mutex::new(None);

/// Allocate before dispatching a captured snapshot; completion order must not order writes.
pub(super) fn next_session_ticket() -> Result<u64> {
    use std::sync::atomic::Ordering;

    let mut ticket = SESSION_TICKETS.load(Ordering::Relaxed);
    loop {
        let next = ticket
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("session save revision exhausted"))?;
        match SESSION_TICKETS.compare_exchange_weak(
            ticket,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Ok(ticket),
            Err(actual) => ticket = actual,
        }
    }
}

pub(super) fn save_session_ticket(
    path: &Path,
    ticket: u64,
    saved: &Saved,
) -> Result<SessionCommit> {
    use anyhow::Context;
    use std::io::Write;
    anyhow::ensure!(
        path.is_absolute() && path.file_name().is_some(),
        "session save requires an absolute file path"
    );
    anyhow::ensure!(ticket > 0, "session save requires a revision");
    crate::conversation::validate_metadata(
        &saved.conversations,
        &saved.pending_starts,
        &saved.conversation_operations,
    )
    .map_err(|e| anyhow::anyhow!("invalid conversation registry: {e:?}"))?;
    let bytes = serde_json::to_vec_pretty(saved).context("encoding saved session")?;
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("session save requires a parent directory"))?;
    private_dir(dir).context("creating private session directory")?;
    let name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("session save requires a file name"))?;
    let key = std::fs::canonicalize(dir)
        .context("resolving session directory")?
        .join(name);
    let writer = {
        let mut writers = SESSION_WRITERS
            .lock()
            .map_err(|_| anyhow::anyhow!("session writer registry poisoned"))?;
        writers
            .get_or_insert_with(Default::default)
            .entry(key)
            .or_default()
            .clone()
    };
    let mut written = writer
        .lock()
        .map_err(|_| anyhow::anyhow!("session writer poisoned"))?;
    if ticket < written.replaced_ticket || ticket <= written.committed_ticket {
        return Ok(SessionCommit::Superseded {
            committed_ticket: written.committed_ticket,
        });
    }
    let tmp = path.with_extension("json.tmp");
    let mut file = create_private(&tmp).context("opening private session temporary file")?;
    file.write_all(&bytes).context("writing saved session")?;
    file.sync_all().context("syncing saved session")?;
    drop(file);
    std::fs::rename(&tmp, path).context("replacing saved session")?;
    // A rename already excludes older snapshots even if directory durability fails.
    written.replaced_ticket = ticket;
    #[cfg(unix)]
    std::fs::File::open(dir)
        .and_then(|dir| dir.sync_all())
        .context("syncing session directory")?;
    written.committed_ticket = ticket;
    Ok(SessionCommit::Committed)
}

#[cfg(test)]
pub(super) fn save_in(path: &Path, saved: &Saved) -> Result<()> {
    save_session_ticket(path, next_session_ticket()?, saved).map(|_| ())
}

pub fn forget() {
    if cfg!(test) {
        return;
    }
    let _ = std::fs::remove_file(path());
    let _ = std::fs::remove_dir_all(output_dir());
    let _ = std::fs::remove_dir_all(check_output_dir());
}

/// Saved output can hold secrets that were on screen: its folder is yours alone on unix
/// (on Windows it sits in your own profile).
pub(super) fn private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// A new file only you can read on unix, for saved output. Set again after opening, so a
/// leftover file from an older the app is fixed too; the rename that follows carries it over.
pub(super) fn create_private(path: &Path) -> std::io::Result<std::fs::File> {
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    let f = open.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(f)
}

/// Delete every pane's saved output (none saved is fine).
pub fn forget_outputs() -> std::io::Result<()> {
    match std::fs::remove_dir_all(output_dir()) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        r => r,
    }
}

/// Where each pane's recent output is kept, so its history is still there after the server
/// restarts (an update, a reboot): one file per pane, named by its id.
pub fn output_dir() -> PathBuf {
    if cfg!(test) {
        // One folder per test (each runs on a thread of its own): a test that clears the saved
        // output must not take away what another test saved a moment ago.
        return std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-test-output-{}-{:?}"),
            std::process::id(),
            std::thread::current().id()
        ));
    }
    let label =
        std::env::var(concat!(env!("APP_ENV"), "_SOCKET")).unwrap_or_else(|_| "default".into());
    crate::config::data_dir().join(format!("output-{label}"))
}

fn output_file(dir: &Path, term: TermId) -> PathBuf {
    dir.join(format!("{term}.bin"))
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
        return std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-test-check-output-{}"),
            std::process::id()
        ));
    }
    let label =
        std::env::var(concat!(env!("APP_ENV"), "_SOCKET")).unwrap_or_else(|_| "default".into());
    crate::config::data_dir().join(format!("check-output-{label}"))
}

/// What an unnamed checkpoint's tail is filed under.
const UNNAMED_CHECKPOINT: &str = "check";

/// One checkpoint's output tail, for one of a pane's two kept results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckTail {
    pub term: TermId,
    /// The checkpoint's name.
    pub name: String,
    pub slot: CheckSlot,
    pub text: String,
}

/// `<term>-<name>-<slot>.txt`. A name comes from config or a repo's file, so anything that
/// could leave the folder or clash with the separators is replaced; a name that had to be
/// changed also gets a short hash of itself, so `a-b` and `a.b` keep files of their own.
fn check_output_file(dir: &Path, term: TermId, name: &str, slot: CheckSlot) -> PathBuf {
    dir.join(format!(
        "{term}-{}-{}.txt",
        check_file_key(name),
        slot.file_suffix()
    ))
}

/// Hex digits of a changed name's hash in its file name.
const NAME_HASH_DIGITS: usize = 8;

fn check_file_key(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !safe.is_empty() && safe == name {
        return safe;
    }
    let base = if safe.is_empty() {
        UNNAMED_CHECKPOINT
    } else {
        &safe
    };
    format!(
        "{base}-{}",
        &crate::protocol::command_hash(name)[..NAME_HASH_DIGITS]
    )
}

/// Where a tail was kept before checkpoints had names: one per pane and slot.
fn legacy_check_output_file(dir: &Path, term: TermId, slot: CheckSlot) -> PathBuf {
    dir.join(format!("{term}-{}.txt", slot.file_suffix()))
}

fn is_legacy_check_output(file: &str) -> bool {
    let rest = file.split_once('-').map_or("", |(_, rest)| rest);
    [CheckSlot::Latest, CheckSlot::Overridden]
        .iter()
        .any(|s| rest == format!("{}.txt", s.file_suffix()))
}

/// What a folder of check tails has had written to it, so a slow background save can't put
/// an older tail over a newer one (as `Written` does for panes' output).
#[derive(Default)]
struct CheckWritten {
    tails: std::collections::HashMap<(TermId, String, CheckSlot), u64>,
    cleaned: u64,
}

static CHECK_WRITTEN: std::sync::Mutex<Option<std::collections::HashMap<PathBuf, CheckWritten>>> =
    std::sync::Mutex::new(None);

/// Keep each check output tail in `dir`, and drop the files of panes not in `live` and those
/// named the old way (read once, at restore). Saves run one at a time; a tail already written
/// by a newer `ticket` (from `save_ticket`) is left alone. One file failing doesn't stop the
/// rest. Returns the panes whose tail was not written, so the caller can try again.
pub fn save_check_outputs(
    dir: &Path,
    ticket: u64,
    outputs: &[CheckTail],
    live: &[TermId],
) -> Vec<TermId> {
    let mut guard = CHECK_WRITTEN.lock().unwrap_or_else(|e| e.into_inner());
    let written = guard
        .get_or_insert_with(Default::default)
        .entry(dir.to_path_buf())
        .or_default();
    let mut failed: Vec<TermId> = Vec::new();
    if !outputs.is_empty()
        && let Err(e) = private_dir(dir)
    {
        tracing::warn!("saving checks' output: {e}");
        return outputs.iter().map(|o| o.term).collect();
    }
    for CheckTail {
        term,
        name,
        slot,
        text,
    } in outputs
    {
        let key = (*term, name.clone(), *slot);
        if written.tails.get(&key).is_some_and(|&t| t > ticket) {
            continue;
        }
        let path = check_output_file(dir, *term, name, *slot);
        let tmp = path.with_extension("tmp");
        let write = || -> std::io::Result<()> {
            use std::io::Write;
            create_private(&tmp)?.write_all(text.as_bytes())?;
            std::fs::rename(&tmp, &path)
        };
        match write() {
            Ok(()) => {
                written.tails.insert(key, ticket);
            }
            Err(e) => {
                tracing::warn!("saving checks' output: {e}");
                failed.push(*term);
            }
        }
    }
    if ticket > written.cleaned {
        written.cleaned = ticket;
        written.tails.retain(|(id, ..), _| live.contains(id));
        // A folder that was never made has nothing to prune.
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = e.file_name();
            let name = name.to_str().unwrap_or_default();
            let id = name
                .split('-')
                .next()
                .and_then(|n| n.parse::<TermId>().ok());
            if id.is_none_or(|id| !live.contains(&id)) || is_legacy_check_output(name) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    failed.sort_unstable();
    failed.dedup();
    failed
}

/// A pane's kept output for checkpoint `name`. Left in place: the pane it is restored into
/// writes its own under its new id, and the next save drops this one, so a crash in between
/// loses nothing.
pub fn read_check_output(dir: &Path, term: TermId, name: &str, slot: CheckSlot) -> Option<String> {
    read_tail(&check_output_file(dir, term, name, slot))
}

/// The output kept before checkpoints had names: the pane's one check's. Left in place like
/// `read_check_output`'s.
pub fn read_legacy_check_output(dir: &Path, term: TermId, slot: CheckSlot) -> Option<String> {
    read_tail(&legacy_check_output_file(dir, term, slot))
}

fn read_tail(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(check::cap_output(&String::from_utf8_lossy(&bytes)))
}

/// Which save last wrote each pane, so a slow background save can't put older output over a
/// newer one. `cleaned` is the newest save that has pruned the folder.
#[derive(Default)]
struct Written {
    panes: std::collections::HashMap<TermId, u64>,
    cleaned: u64,
}

static WRITTEN: std::sync::Mutex<Option<std::collections::HashMap<PathBuf, Written>>> =
    std::sync::Mutex::new(None);
static TICKETS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// A number for a save, taken when its output is gathered: a higher one is a newer picture.
pub fn save_ticket() -> u64 {
    TICKETS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Keep `bytes` as pane `term`'s output, and drop the files of panes not in `live`. Saves run
/// one at a time; a pane already written by a newer `ticket` is left alone. One pane failing
/// doesn't stop the rest; the first error is returned after all were tried.
pub fn save_outputs(ticket: u64, outputs: &[(TermId, Vec<u8>)], live: &[TermId]) -> Result<()> {
    save_outputs_in(&output_dir(), ticket, outputs, live)
}

fn save_outputs_in(
    dir: &Path,
    ticket: u64,
    outputs: &[(TermId, Vec<u8>)],
    live: &[TermId],
) -> Result<()> {
    use std::io::Write;
    let mut guard = WRITTEN.lock().unwrap_or_else(|e| e.into_inner());
    let written = guard
        .get_or_insert_with(Default::default)
        .entry(dir.to_path_buf())
        .or_default();
    private_dir(dir)?;
    let mut first_err: Option<anyhow::Error> = None;
    for (term, bytes) in outputs {
        if written.panes.get(term).is_some_and(|&t| t > ticket) {
            continue;
        }
        let path = output_file(dir, *term);
        let tmp = path.with_extension("tmp");
        // Synced before the rename, so a power cut leaves the old file or the new one, never
        // an empty one.
        let result = (|| -> std::io::Result<()> {
            let mut f = create_private(&tmp)?;
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
        for e in std::fs::read_dir(dir)?.flatten() {
            let id = e
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<TermId>().ok());
            if id.is_none_or(|id| !live.contains(&id)) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    first_err.map_or(Ok(()), Err)
}

/// A pane's kept output, taken (its file goes: the pane it's restored into saves its own).
pub fn take_output(term: TermId) -> Option<Vec<u8>> {
    take_output_in(&output_dir(), term)
}

fn take_output_in(dir: &Path, term: TermId) -> Option<Vec<u8>> {
    let path = output_file(dir, term);
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
    ScratchDir(std::env::temp_dir().join(format!(
        "{}-test-scratch-{}-{n}",
        crate::brand::APP_COMMAND,
        std::process::id()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::CheckState;

    #[test]
    fn session_newer_commit_fences_an_older_full_snapshot() {
        let dir = scratch_dir();
        let path = dir.path().join("session.json");
        let old = Saved::default();
        let new = Saved {
            conversations: vec![crate::test_support::conversation()],
            ..Default::default()
        };
        let old_ticket = next_session_ticket().unwrap();
        let new_ticket = next_session_ticket().unwrap();
        assert_eq!(
            save_session_ticket(&path, new_ticket, &new).unwrap(),
            SessionCommit::Committed
        );
        assert_eq!(
            save_session_ticket(&path, old_ticket, &old).unwrap(),
            SessionCommit::Superseded {
                committed_ticket: new_ticket
            }
        );
        let actual: Saved = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(actual.conversations, new.conversations);
        assert_eq!(files_in(dir.path()), ["session.json"]);
    }

    #[test]
    fn session_full_snapshots_keep_the_latest_conversation_revision() {
        let dir = scratch_dir();
        let path = dir.path().join("session.json");
        let mut old = Saved {
            conversations: vec![crate::test_support::conversation()],
            ..Default::default()
        };
        let old_ticket = next_session_ticket().unwrap();
        old.conversations[0].generation += 1;
        let new_ticket = next_session_ticket().unwrap();
        assert_eq!(
            save_session_ticket(&path, new_ticket, &old).unwrap(),
            SessionCommit::Committed
        );
        old.conversations[0].generation -= 1;
        assert_eq!(
            save_session_ticket(&path, old_ticket, &old).unwrap(),
            SessionCommit::Superseded {
                committed_ticket: new_ticket
            }
        );
        let actual: Saved = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            actual.conversations[0].generation,
            old.conversations[0].generation + 1
        );
    }

    #[test]
    fn session_failed_new_write_does_not_fence_a_retry() {
        let dir = scratch_dir();
        let path = dir.path().join("session.json");
        let saved = Saved::default();
        let ticket = next_session_ticket().unwrap();
        private_dir(dir.path()).unwrap();
        let temporary = path.with_extension("json.tmp");
        std::fs::create_dir(&temporary).unwrap();
        assert!(save_session_ticket(&path, ticket, &saved).is_err());
        assert!(!path.exists());
        std::fs::remove_dir(&temporary).unwrap();
        assert_eq!(
            save_session_ticket(&path, ticket, &saved).unwrap(),
            SessionCommit::Committed
        );
    }

    #[test]
    fn session_repeated_ticket_cannot_replace_committed_contents() {
        let dir = scratch_dir();
        let path = dir.path().join("session.json");
        let saved = Saved::default();
        let ticket = next_session_ticket().unwrap();
        assert_eq!(
            save_session_ticket(&path, ticket, &saved).unwrap(),
            SessionCommit::Committed
        );
        let changed = Saved {
            active: 1,
            ..Default::default()
        };
        assert_eq!(
            save_session_ticket(&path, ticket, &changed).unwrap(),
            SessionCommit::Superseded {
                committed_ticket: ticket
            }
        );
        let actual: Saved = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(actual.active, 0);
    }

    #[test]
    fn session_writer_rejects_a_relative_or_missing_destination() {
        let saved = Saved::default();
        for path in [Path::new(""), Path::new("session.json")] {
            assert!(save_session_ticket(path, next_session_ticket().unwrap(), &saved).is_err());
        }
    }

    #[test]
    fn conversation_metadata_is_private_atomic_and_independent_of_output() {
        let dir = scratch_dir();
        let path = dir.path().join("session.json");
        let record = crate::test_support::conversation();
        let saved = Saved {
            conversations: vec![record.clone()],
            ..Default::default()
        };
        save_in(&path, &saved).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let back: Saved = serde_json::from_str(&text).unwrap();
        assert_eq!(back.conversations, [record]);
        assert_eq!(files_in(dir.path()), ["session.json"]);
        assert!(
            !text.contains("approval")
                && !text.contains("prompt")
                && !text.contains("response")
                && !text.contains("token")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn invalid_registry_cannot_replace_a_valid_saved_session() {
        let dir = scratch_dir();
        let path = dir.path().join("session.json");
        let record = crate::test_support::conversation();
        let mut saved = Saved {
            conversations: vec![record.clone()],
            ..Default::default()
        };
        save_in(&path, &saved).unwrap();
        let previous = std::fs::read(&path).unwrap();
        saved.conversations.push(record);
        assert!(save_in(&path, &saved).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), previous);
    }

    #[test]
    fn legacy_saved_sessions_have_no_conversation_authority() {
        let saved: Saved = serde_json::from_str(r#"{"workspaces":[],"active":0}"#).unwrap();
        assert!(saved.conversations.is_empty());
        let pane: SavedPane =
            serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":"codex","session":"old"}"#)
                .unwrap();
        assert_eq!(pane.conversation_id, None);
    }

    fn tail(term: TermId, name: &str, slot: CheckSlot, text: &str) -> CheckTail {
        CheckTail {
            term,
            name: name.into(),
            slot,
            text: text.into(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn saved_output_is_readable_only_by_you() {
        use std::os::unix::fs::PermissionsExt;
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        let dir = scratch_dir();
        let old = check_output_file(dir.path(), 7, "test", CheckSlot::Latest);
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(&old, concat!("from an older ", env!("APP_COMMAND"))).unwrap();
        std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            save_check_outputs(
                dir.path(),
                save_ticket(),
                &[tail(7, "test", CheckSlot::Latest, "tail")],
                &[7]
            )
            .is_empty()
        );
        assert_eq!(
            (mode(dir.path()), mode(&old)),
            (0o700, 0o600),
            "a check tail, and the older file it replaced"
        );
        let pane = dir.path().join("pane.bin");
        let mut f = create_private(&pane).unwrap();
        std::io::Write::write_all(&mut f, b"secret").unwrap();
        assert_eq!(mode(&pane), 0o600, "a pane's output");
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        files
    }

    #[test]
    fn a_slow_older_save_cannot_overwrite_a_newer_one() {
        let dir = scratch_dir();
        let term: crate::protocol::TermId = 9_001;
        let old = super::save_ticket();
        let new = super::save_ticket();
        super::save_outputs_in(dir.path(), new, &[(term, b"newer".to_vec())], &[term]).unwrap();
        super::save_outputs_in(dir.path(), old, &[(term, b"older".to_vec())], &[term]).unwrap();
        assert_eq!(
            super::take_output_in(dir.path(), term).as_deref(),
            Some(b"newer".as_slice())
        );
    }

    #[test]
    fn check_results_round_trip_and_older_panes_load_without_them() {
        let info = CheckInfo {
            command: "npm test".into(),
            name: "test".into(),
            state: CheckState::Failed,
            exit_code: Some(1),
            ..Default::default()
        };
        let build = CheckInfo {
            name: "build".into(),
            ..info.clone()
        };
        let pane = SavedPane {
            checks: vec![build.clone(), info.clone()],
            checks_overridden: vec![info.clone()],
            ..Default::default()
        };
        let back: SavedPane = serde_json::from_str(&serde_json::to_string(&pane).unwrap()).unwrap();
        assert_eq!(
            (back.checks, back.checks_overridden),
            (vec![build, info.clone()], vec![info])
        );
        let old: SavedPane =
            serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":null,"session":null}"#)
                .expect("a pane saved before checks");
        assert_eq!((old.checks, old.checks_overridden), (vec![], vec![]));
    }

    #[test]
    fn a_pane_keeps_the_app_that_started_it_and_older_panes_are_terminal() {
        let pane = SavedPane {
            owner: crate::protocol::Surface::Desktop,
            ..Default::default()
        };
        let back: SavedPane = serde_json::from_str(&serde_json::to_string(&pane).unwrap()).unwrap();
        assert_eq!(back.owner, crate::protocol::Surface::Desktop);
        let old: SavedPane =
            serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":null,"session":null}"#)
                .expect("a pane saved before owners");
        assert_eq!(old.owner, crate::protocol::Surface::Terminal);
    }

    #[test]
    fn a_pane_saved_with_one_check_loads_it_as_one_checkpoint() {
        let old: SavedPane = serde_json::from_str(
            r#"{"cwd":null,"cmd":null,"agent":null,"session":null,
               "check":{"command":"npm test","state":"Failed","exit_code":1},
               "check_overridden":{"command":"npm test","state":"Overridden"}}"#,
        )
        .expect("a pane saved by protocol 43");
        let states = |c: &[CheckInfo]| {
            c.iter()
                .map(|c| (c.command.clone(), c.state))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            states(&old.checks),
            [("npm test".to_string(), CheckState::Failed)]
        );
        assert_eq!(
            states(&old.checks_overridden),
            [("npm test".to_string(), CheckState::Overridden)]
        );
        let none: SavedPane = serde_json::from_str(
            r#"{"cwd":null,"cmd":null,"agent":null,"session":null,"check":null}"#,
        )
        .unwrap();
        assert!(none.checks.is_empty());
    }

    #[test]
    fn each_checkpoint_keeps_its_own_output_file() {
        let dir = scratch_dir();
        let outputs = [
            tail(7, "build", CheckSlot::Latest, "built"),
            tail(7, "test", CheckSlot::Latest, "FAIL a"),
            tail(7, "test", CheckSlot::Overridden, "FAIL b"),
        ];
        assert!(save_check_outputs(dir.path(), save_ticket(), &outputs, &[7]).is_empty());
        assert_eq!(
            files_in(dir.path()),
            [
                "7-build-latest.txt",
                "7-test-latest.txt",
                "7-test-overridden.txt"
            ]
        );
        assert_eq!(
            read_check_output(dir.path(), 7, "build", CheckSlot::Latest).as_deref(),
            Some("built")
        );
        assert_eq!(
            read_check_output(dir.path(), 7, "test", CheckSlot::Latest).as_deref(),
            Some("FAIL a")
        );
    }

    #[test]
    fn a_name_cannot_lead_a_tail_out_of_its_folder() {
        let dir = scratch_dir();
        let outputs = [
            tail(7, "../x/y", CheckSlot::Latest, "t"),
            tail(7, "", CheckSlot::Latest, "u"),
        ];
        assert!(save_check_outputs(dir.path(), save_ticket(), &outputs, &[7]).is_empty());
        assert_eq!(
            files_in(dir.path()),
            [
                format!("7-{}-latest.txt", check_file_key("../x/y")),
                format!("7-{}-latest.txt", check_file_key(""))
            ]
        );
        assert!(
            check_file_key("../x/y").starts_with("___x_y-")
                && check_file_key("").starts_with("check-")
        );
    }

    #[test]
    fn names_that_read_alike_keep_tails_of_their_own() {
        let dir = scratch_dir();
        let outputs = [
            tail(7, "a-b", CheckSlot::Latest, "dash"),
            tail(7, "a.b", CheckSlot::Latest, "dot"),
            tail(7, "a_b", CheckSlot::Latest, "underscore"),
            tail(7, "check", CheckSlot::Latest, "named"),
            tail(7, "", CheckSlot::Latest, "unnamed"),
        ];
        assert!(save_check_outputs(dir.path(), save_ticket(), &outputs, &[7]).is_empty());
        for (name, text) in [
            ("a-b", "dash"),
            ("a.b", "dot"),
            ("a_b", "underscore"),
            ("check", "named"),
            ("", "unnamed"),
        ] {
            assert_eq!(
                read_check_output(dir.path(), 7, name, CheckSlot::Latest).as_deref(),
                Some(text),
                "{name:?}"
            );
        }
    }

    #[test]
    fn a_tail_kept_before_checkpoints_had_names_is_read_once() {
        let dir = scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join("7-latest.txt"), "old tail").unwrap();
        assert_eq!(
            read_legacy_check_output(dir.path(), 7, CheckSlot::Latest).as_deref(),
            Some("old tail")
        );
        assert_eq!(
            read_check_output(dir.path(), 7, "test", CheckSlot::Latest),
            None,
            "not a named checkpoint's"
        );
        assert!(
            save_check_outputs(
                dir.path(),
                save_ticket(),
                &[tail(7, "test", CheckSlot::Latest, "new tail")],
                &[7]
            )
            .is_empty()
        );
        assert_eq!(
            files_in(dir.path()),
            ["7-test-latest.txt"],
            "the next save drops the old file"
        );
        assert_eq!(
            read_check_output(dir.path(), 7, "test", CheckSlot::Latest).as_deref(),
            Some("new tail")
        );
    }

    #[test]
    fn check_output_is_kept_per_slot_read_back_and_dropped_with_its_pane() {
        let dir = scratch_dir();
        let (kept, gone): (crate::protocol::TermId, crate::protocol::TermId) = (7, 8);
        let outputs = [
            tail(kept, "test", CheckSlot::Latest, "FAIL a"),
            tail(kept, "test", CheckSlot::Overridden, "FAIL b"),
            tail(gone, "test", CheckSlot::Latest, "FAIL c"),
        ];
        assert!(save_check_outputs(dir.path(), save_ticket(), &outputs, &[kept, gone]).is_empty());
        assert!(save_check_outputs(dir.path(), save_ticket(), &[], &[kept]).is_empty());
        assert_eq!(
            read_check_output(dir.path(), gone, "test", CheckSlot::Latest),
            None,
            "a closed pane's tail goes"
        );
        assert_eq!(
            read_check_output(dir.path(), kept, "test", CheckSlot::Overridden).as_deref(),
            Some("FAIL b")
        );
        assert_eq!(
            read_check_output(dir.path(), kept, "test", CheckSlot::Latest).as_deref(),
            Some("FAIL a")
        );
        assert_eq!(
            read_check_output(dir.path(), kept, "test", CheckSlot::Latest).as_deref(),
            Some("FAIL a"),
            "reading leaves it for the next restart"
        );
    }

    #[test]
    fn a_slow_older_check_save_cannot_overwrite_a_newer_tail() {
        let dir = scratch_dir();
        let term: crate::protocol::TermId = 3;
        let (old, new) = (save_ticket(), save_ticket());
        assert!(
            save_check_outputs(
                dir.path(),
                new,
                &[tail(term, "test", CheckSlot::Latest, "newer")],
                &[term]
            )
            .is_empty()
        );
        assert!(
            save_check_outputs(
                dir.path(),
                old,
                &[tail(term, "test", CheckSlot::Latest, "older")],
                &[term]
            )
            .is_empty()
        );
        assert_eq!(
            read_check_output(dir.path(), term, "test", CheckSlot::Latest).as_deref(),
            Some("newer")
        );
    }

    #[test]
    fn a_tail_that_cannot_be_written_is_reported_so_it_can_be_tried_again() {
        let dir = scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        let blocker = dir.path().join("not-a-folder");
        std::fs::write(&blocker, "x").unwrap();
        let failed = save_check_outputs(
            &blocker,
            save_ticket(),
            &[
                tail(4, "test", CheckSlot::Latest, "t"),
                tail(5, "test", CheckSlot::Overridden, "u"),
            ],
            &[4, 5],
        );
        assert_eq!(failed, [4, 5]);
    }

    #[test]
    fn two_folders_do_not_share_what_was_written() {
        let (a, b) = (scratch_dir(), scratch_dir());
        let term: crate::protocol::TermId = 6;
        let (old, new) = (save_ticket(), save_ticket());
        assert!(
            save_check_outputs(
                a.path(),
                new,
                &[tail(term, "test", CheckSlot::Latest, "a")],
                &[term]
            )
            .is_empty()
        );
        assert!(
            save_check_outputs(
                b.path(),
                old,
                &[tail(term, "test", CheckSlot::Latest, "b")],
                &[term]
            )
            .is_empty()
        );
        assert_eq!(
            read_check_output(b.path(), term, "test", CheckSlot::Latest).as_deref(),
            Some("b")
        );
    }

    #[test]
    fn sessions_saved_with_workflow_runs_still_restore_the_workspace() {
        let saved: Saved = serde_json::from_str(
            r#"{"workflows":[{"id":7}],"workspaces":[],"active":0,"made_worktrees":[]}"#,
        )
        .expect("unknown legacy workflow data is ignored");
        assert!(saved.workspaces.is_empty());
    }
}
