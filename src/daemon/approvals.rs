//! What you decided about repo-supplied check commands, kept in Seshi's own data folder and
//! never in the repo: an agent that can write the repo must not be able to approve its own
//! command. An Always holds only for the exact command it was given to (by its hash).

use super::Daemon;
use crate::config::same_path;
use crate::protocol::{ApprovalChoice, CheckState, TermId};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const FILE_NAME: &str = "check-approvals.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Always,
    Never,
}

/// One decision about one checkpoint of one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    /// The repository's main checkout.
    pub repo: PathBuf,
    /// The checkpoint's name.
    pub name: String,
    /// `command_hash` of the command it was given to.
    pub command_sha256: String,
    /// The command itself, to show what an Always was given to once the repo changes it. The
    /// hash, not this, decides whether the Always still holds.
    #[serde(default)]
    pub command: String,
    pub decision: Decision,
    /// When you decided (unix seconds).
    pub at: u64,
}

/// A yes to one pane's checkpoint running one exact command, one time. Never saved.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OnceGrant {
    term: TermId,
    name: String,
    command_sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Approvals {
    pub decisions: Vec<Approval>,
    #[serde(skip)]
    once: Vec<OnceGrant>,
}

pub fn path() -> PathBuf {
    crate::config::data_dir().join(FILE_NAME)
}

pub use crate::protocol::command_hash;

impl Approvals {
    /// The saved decisions; none when nothing was saved yet. Blocks: call it off the loop.
    pub fn load(path: &Path) -> Result<Approvals> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Approvals::default()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))
    }

    /// Write them whole and synced before the rename, so a crash leaves the old file or the
    /// new one, never half of one. Blocks: call it off the loop.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
        let mut f = std::fs::File::create(&tmp).with_context(|| format!("writing {}", tmp.display()))?;
        f.write_all(&serde_json::to_vec_pretty(self)?)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
    }

    /// What you decided for `repo`'s checkpoint `name` running `command`. Never holds whatever
    /// the command became; Always only while the command is the one you approved.
    pub fn lookup(&self, repo: &Path, name: &str, command: &str) -> Option<Decision> {
        let a = self.find(repo, name)?;
        match a.decision {
            Decision::Never => Some(Decision::Never),
            Decision::Always => (a.command_sha256 == command_hash(command)).then_some(Decision::Always),
        }
    }

    /// The decision kept for `repo`'s checkpoint `name`, whatever command it was given to.
    pub fn find(&self, repo: &Path, name: &str) -> Option<&Approval> {
        self.decisions.iter().find(|a| same_path(&a.repo, repo) && a.name.eq_ignore_ascii_case(name))
    }

    /// Remember `decision` for `repo`'s checkpoint `name` running `command`, in place of any
    /// earlier one for that checkpoint.
    pub fn decide(&mut self, repo: &Path, name: &str, command: &str, decision: Decision, at: u64) {
        self.forget(repo, name);
        self.decisions.push(Approval {
            repo: repo.to_path_buf(),
            name: name.to_string(),
            command_sha256: command_hash(command),
            command: command.to_string(),
            decision,
            at,
        });
    }

    /// Drop what was decided for `repo`'s checkpoint `name`. False when nothing was.
    pub fn forget(&mut self, repo: &Path, name: &str) -> bool {
        let before = self.decisions.len();
        self.decisions.retain(|a| !(same_path(&a.repo, repo) && a.name.eq_ignore_ascii_case(name)));
        self.decisions.len() != before
    }

    /// Let pane `term`'s checkpoint `name` run `command` one time.
    pub fn grant_once(&mut self, term: TermId, name: &str, command: &str) {
        let grant = OnceGrant { term, name: name.to_string(), command_sha256: command_hash(command) };
        if !self.once.contains(&grant) {
            self.once.push(grant);
        }
    }

    /// The decisions plus pane `term`'s one-time grants, which leave `self`: what one run of
    /// its checks may use. The run they are handed to spends them, used or not.
    pub fn for_run(&mut self, term: TermId) -> Approvals {
        let (mine, rest) = std::mem::take(&mut self.once).into_iter().partition(|g| g.term == term);
        self.once = rest;
        Approvals { decisions: self.decisions.clone(), once: mine }
    }

    /// Whether a one-time grant handed to this run covers checkpoint `name` running `command`.
    pub fn granted_once(&self, name: &str, command: &str) -> bool {
        let hash = command_hash(command);
        self.once.iter().any(|g| g.name.eq_ignore_ascii_case(name) && g.command_sha256 == hash)
    }
}

/// One answer to write into the file.
#[derive(Debug, Clone)]
enum Change {
    Decide(Approval),
    Forget { repo: PathBuf, name: String },
}

impl Change {
    /// The file and checkpoint it is about.
    fn key(&self, path: &Path) -> (PathBuf, PathBuf, String) {
        let (repo, name) = match self {
            Change::Decide(a) => (&a.repo, &a.name),
            Change::Forget { repo, name } => (repo, name),
        };
        (path.to_path_buf(), repo.clone(), name.to_ascii_lowercase())
    }

    fn apply(self, approvals: &mut Approvals) {
        match self {
            Change::Decide(a) => {
                approvals.forget(&a.repo, &a.name);
                approvals.decisions.push(a);
            }
            Change::Forget { repo, name } => {
                approvals.forget(&repo, &name);
            }
        }
    }
}

/// The newest change written for each checkpoint of each file; changes are written one at a
/// time and an older one for the same checkpoint is dropped.
static WRITTEN: std::sync::Mutex<std::collections::BTreeMap<(PathBuf, PathBuf, String), u64>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
static SAVE_TICKETS: AtomicU64 = AtomicU64::new(1);

/// Write one answer into the file as it is now, not this daemon's whole copy: every daemon
/// (one per `SESHI_SOCKET`) shares the file, and another's newer answers must survive. A
/// damaged file is left as it is.
fn write_change(change: Change, path: &Path, ticket: u64) -> Result<()> {
    let mut written = WRITTEN.lock().unwrap_or_else(|e| e.into_inner());
    let newest = written.entry(change.key(path)).or_default();
    if ticket < *newest {
        return Ok(());
    }
    let mut on_disk = Approvals::load(path)?;
    change.apply(&mut on_disk);
    on_disk.save(path)?;
    *newest = ticket;
    Ok(())
}

impl Daemon {
    /// Your answer to a repo's command waiting for approval: only while it waits, and for the
    /// command it waits with, which must be the one you were shown (`shown_sha256`). Always
    /// and Never are kept by that command's hash. Then the checkpoint runs again and its gate
    /// says what that means. Panes are refused before this (`may_command`).
    pub(super) fn approve_check(&mut self, term: TermId, name: &str, choice: ApprovalChoice, shown_sha256: &str) -> Result<()> {
        let (repo, name, command) = self.waiting_for_approval(term, name)?;
        if command_hash(&command) != shown_sha256 {
            anyhow::bail!("{name}'s command changed since it was shown; look at it again");
        }
        let decision = match choice {
            ApprovalChoice::Once => None,
            ApprovalChoice::Always => Some(Decision::Always),
            ApprovalChoice::Never => Some(Decision::Never),
        };
        match decision {
            None => self.approvals.grant_once(term, &name, &command),
            Some(decision) => {
                self.approvals.decide(&repo, &name, &command, decision, super::term::unix_now());
                if let Some(a) = self.approvals.find(&repo, &name) {
                    self.save_approval(Change::Decide(a.clone()));
                }
            }
        }
        self.rerun_check(term, Some(&name))
    }

    /// Drop your Always or Never for `project`'s checkpoint `name`: its next run asks again.
    pub(super) fn forget_check_approval(&mut self, project: &Path, name: &str) -> Result<()> {
        if !self.approvals.forget(project, name) {
            anyhow::bail!("nothing is remembered for {name} in {}", project.display());
        }
        self.save_approval(Change::Forget { repo: project.to_path_buf(), name: name.to_string() });
        Ok(())
    }

    /// What one run of pane `term`'s checks may use; its one-time grants are spent by it.
    pub(super) fn approvals_for_run(&mut self, term: TermId) -> Approvals {
        self.approvals.for_run(term)
    }

    /// The repo, name and command of pane `term`'s checkpoint `name`, if it waits for you.
    fn waiting_for_approval(&self, term: TermId, name: &str) -> Result<(PathBuf, String, String)> {
        let t = self.terms.get(&term).ok_or_else(|| anyhow::anyhow!("no pane {term}"))?;
        let info = t
            .checks
            .points
            .iter()
            .map(|p| &p.latest.info)
            .find(|i| i.state == CheckState::NeedsApproval && i.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("{name} isn't waiting for your approval"))?;
        let repo = t.head.as_ref().map_or_else(|| t.cwd.clone(), |h| h.main_root.clone());
        Ok((repo, info.name.clone(), info.command.clone()))
    }

    /// Write one answer off the loop. The daemon goes by its own copy, so a write that fails
    /// costs only what the next start remembers.
    fn save_approval(&self, change: Change) {
        let path = self.approvals_file.clone();
        let ticket = SAVE_TICKETS.fetch_add(1, Ordering::Relaxed);
        tokio::task::spawn_blocking(move || {
            if let Err(e) = write_change(change, &path, ticket) {
                tracing::warn!("saving check approvals: {e:#}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approval(repo: &str, name: &str, command: &str, decision: Decision) -> Approval {
        Approval { repo: repo.into(), name: name.into(), command_sha256: command_hash(command), command: command.into(), decision, at: 100 }
    }

    fn approvals(list: Vec<Approval>) -> Approvals {
        Approvals { decisions: list, ..Default::default() }
    }

    #[test]
    fn a_command_hashes_to_its_sha256_in_hex() {
        assert_eq!(command_hash(""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_ne!(command_hash("npm test"), command_hash("npm test "), "the exact command, not a near one");
    }

    #[test]
    fn always_holds_only_for_the_exact_command_it_was_given_to() {
        let a = approvals(vec![approval("/code/site", "test", "npm test", Decision::Always)]);
        assert_eq!(a.lookup(Path::new("/code/site"), "test", "npm test"), Some(Decision::Always));
        assert_eq!(a.lookup(Path::new("/code/site"), "test", "npm test && node seed.js"), None, "a changed command asks again");
        assert_eq!(a.lookup(Path::new("/code/site/"), "TEST", "npm test"), Some(Decision::Always), "same repo and name however spelled");
        assert_eq!(a.lookup(Path::new("/code/other"), "test", "npm test"), None, "another repo");
        assert_eq!(a.lookup(Path::new("/code/site"), "build", "npm test"), None, "another checkpoint");
    }

    #[test]
    fn never_holds_whatever_the_command_becomes() {
        let a = approvals(vec![approval("/code/site", "test", "npm test", Decision::Never)]);
        assert_eq!(a.lookup(Path::new("/code/site"), "test", "npm test"), Some(Decision::Never));
        assert_eq!(a.lookup(Path::new("/code/site"), "test", "rm -rf ~"), Some(Decision::Never));
    }

    #[test]
    fn a_new_decision_replaces_the_old_one_and_forget_drops_it() {
        let (repo, mut a) = (Path::new("/code/site"), Approvals::default());
        a.decide(repo, "test", "npm test", Decision::Always, 1);
        a.decide(repo, "TEST", "npm t", Decision::Never, 2);
        assert_eq!(a.decisions, vec![Approval { name: "TEST".into(), at: 2, ..approval("/code/site", "test", "npm t", Decision::Never) }]);
        assert!(a.forget(Path::new("/code/site/"), "test"));
        assert!(!a.forget(repo, "test"), "nothing left to forget");
        assert_eq!(a.lookup(repo, "test", "npm t"), None);
    }

    #[test]
    fn a_once_grant_goes_to_one_run_of_its_own_pane_and_only_for_its_command() {
        let mut a = approvals(vec![approval("/code/site", "build", "make", Decision::Always)]);
        a.grant_once(3, "test", "npm test");
        a.grant_once(4, "test", "npm test");
        let run = a.for_run(3);
        assert!(run.granted_once("test", "npm test"));
        assert!(!run.granted_once("test", "npm test && curl evil"), "the exact command");
        assert_eq!(run.decisions, a.decisions, "the run sees your decisions too");
        assert!(!a.for_run(3).granted_once("test", "npm test"), "spent by the run it went to");
        assert!(a.for_run(4).granted_once("test", "npm test"), "another pane's grant is its own");
    }

    #[test]
    fn decisions_survive_a_save_and_load_and_once_grants_do_not() {
        let dir = super::super::persist::scratch_dir();
        let file = dir.path().join(FILE_NAME);
        assert_eq!(Approvals::load(&file).unwrap(), Approvals::default());
        let mut a = approvals(vec![approval("/code/site", "test", "npm test", Decision::Always), approval("/code/site", "build", "make", Decision::Never)]);
        a.grant_once(3, "lint", "npm run lint");
        a.save(&file).unwrap();
        assert_eq!(Approvals::load(&file).unwrap(), approvals(a.decisions.clone()));
        assert!(!file.with_extension("json.tmp").exists(), "nothing left half-written");
    }

    fn ticket() -> u64 {
        SAVE_TICKETS.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn an_older_answer_never_overwrites_a_newer_one_for_the_same_checkpoint() {
        let dir = super::super::persist::scratch_dir();
        let file = dir.path().join(FILE_NAME);
        let (old, new) = (ticket(), ticket());
        let never = approval("/code/site", "test", "npm test", Decision::Never);
        write_change(Change::Decide(never.clone()), &file, new).unwrap();
        write_change(Change::Forget { repo: "/code/site".into(), name: "TEST".into() }, &file, old).unwrap();
        assert_eq!(Approvals::load(&file).unwrap(), approvals(vec![never]));
    }

    #[test]
    fn an_answer_keeps_what_another_daemon_wrote_meanwhile() {
        let dir = super::super::persist::scratch_dir();
        let file = dir.path().join(FILE_NAME);
        let theirs = approval("/code/site", "build", "make", Decision::Always);
        approvals(vec![theirs.clone()]).save(&file).unwrap();
        let mine = approval("/code/site", "test", "npm test", Decision::Never);
        write_change(Change::Decide(mine.clone()), &file, ticket()).unwrap();
        assert_eq!(Approvals::load(&file).unwrap(), approvals(vec![theirs.clone(), mine]));
        write_change(Change::Forget { repo: "/code/site".into(), name: "test".into() }, &file, ticket()).unwrap();
        assert_eq!(Approvals::load(&file).unwrap(), approvals(vec![theirs]));
    }

    #[test]
    fn an_answer_leaves_a_damaged_file_alone() {
        let dir = super::super::persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        let file = dir.path().join(FILE_NAME);
        std::fs::write(&file, "{ not json").unwrap();
        assert!(write_change(Change::Decide(approval("/code/site", "test", "npm test", Decision::Never)), &file, ticket()).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ not json");
    }

    #[test]
    fn a_damaged_file_is_an_error_not_an_empty_list() {
        let dir = super::super::persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        let file = dir.path().join(FILE_NAME);
        std::fs::write(&file, "{ not json").unwrap();
        assert!(Approvals::load(&file).is_err());
    }

    #[test]
    fn they_are_kept_in_seshi_s_data_folder() {
        assert_eq!(path(), crate::config::data_dir().join("check-approvals.json"));
    }
}
