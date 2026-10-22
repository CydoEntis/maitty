//! Stale passes: a checkpoint passed, then the files changed under it. A run records the
//! worktree's tree when it starts (with a private index, as a snapshot does, never the
//! real one); the git poll compares it with the worktree now. Only a pass goes stale.

use super::{Daemon, Ev};
use crate::protocol::{ChangedFile, CheckState, Status, TermId};
use std::path::{Path, PathBuf};

/// The most changed files kept on a stale result; the rest only make the snapshot bigger.
pub const CHANGED_FILES_MOST: usize = 50;

/// The tree of everything in the worktree at `dir` (tracked or not, never what's ignored),
/// or `None` outside git. Blocks: call it off the loop.
pub fn fingerprint(dir: &Path) -> Option<String> {
    match crate::snapshot::tree_now(dir)? {
        Ok(tree) => Some(tree),
        Err(e) => {
            tracing::warn!("couldn't fingerprint the worktree for its check: {e}");
            None
        }
    }
}

/// A pane whose passes may have gone stale: its worktree, and the trees its passes saw.
#[derive(Debug, Clone)]
pub struct Probe {
    pub term: TermId,
    pub dir: PathBuf,
    pub trees: Vec<String>,
}

/// The passes run on `tree` are stale: the worktree has these changes since.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub term: TermId,
    pub tree: String,
    pub changed: Vec<ChangedFile>,
}

/// Run `probes` and report what went stale. Blocks: call it off the loop.
pub fn report(probes: &[Probe], tx: &tokio::sync::mpsc::Sender<Ev>) {
    let found: Vec<Found> = probes.iter().flat_map(probe).collect();
    if !found.is_empty() {
        let _ = tx.blocking_send(Ev::ChecksStale(found));
    }
}

/// What changed in `p`'s worktree since each of its trees. Blocks: call it off the loop.
pub fn probe(p: &Probe) -> Vec<Found> {
    let Some(now) = fingerprint(&p.dir) else { return Vec::new() };
    p.trees
        .iter()
        .filter(|tree| **tree != now)
        .filter_map(|tree| match changed_files(&p.dir, tree, &now) {
            Ok(changed) => Some(Found { term: p.term, tree: tree.clone(), changed }),
            Err(e) => {
                tracing::warn!("couldn't list what changed since a check: {e}");
                None
            }
        })
        .collect()
}

/// `git diff --name-status` between two trees: what kind of change and to which file
/// (a rename names its new path), at most `CHANGED_FILES_MOST`.
fn changed_files(dir: &Path, from: &str, to: &str) -> Result<Vec<ChangedFile>, String> {
    let out = crate::proc::git(dir, &["diff", "--name-status", "-z", from, to])?;
    Ok(parse_name_status(&out))
}

fn parse_name_status(out: &str) -> Vec<ChangedFile> {
    let mut fields = out.split('\0').filter(|f| !f.is_empty());
    let mut changed = Vec::new();
    while let Some(status) = fields.next() {
        let kind = status.chars().next().unwrap_or('M');
        // A rename or copy is followed by its old path, then its new one.
        if matches!(kind, 'R' | 'C') {
            fields.next();
        }
        let Some(path) = fields.next() else { break };
        if changed.len() < CHANGED_FILES_MOST {
            changed.push(ChangedFile { kind, path: path.to_string() });
        }
    }
    changed
}

impl Daemon {
    /// The panes to look at on this git poll: an agent not at work, with a pass whose run
    /// saw a tree.
    pub(super) fn stale_probes(&self) -> Vec<Probe> {
        self.terms
            .values()
            .filter(|t| t.status != Status::Working && !t.checks.running())
            .filter_map(|t| {
                let mut trees: Vec<String> = t.checks.points.iter().filter(|p| p.latest.info.state == CheckState::Passed).filter_map(|p| p.tree.clone()).collect();
                trees.sort();
                trees.dedup();
                let dir = t.head.as_ref().map_or_else(|| t.cwd.clone(), |h| h.top.clone());
                (!trees.is_empty()).then_some(Probe { term: t.id, dir, trees })
            })
            .collect()
    }

    /// Mark the passes the poll found out of date, unless the agent went back to work
    /// meanwhile (its turn end will check it again).
    pub(super) fn checks_went_stale(&mut self, found: Vec<Found>) {
        let now = super::term::unix_now();
        for f in found {
            let Some(t) = self.terms.get_mut(&f.term).filter(|t| t.status != Status::Working) else { continue };
            let mut changed = false;
            for p in t.checks.points.iter_mut().filter(|p| p.latest.info.state == CheckState::Passed && p.tree.as_deref() == Some(f.tree.as_str())) {
                let info = &mut p.latest.info;
                info.state = CheckState::Stale;
                info.stale_since = Some(now);
                info.changed_files = f.changed.clone();
                changed = true;
            }
            if changed {
                self.mark_check_changed(f.term);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_status_reads_each_kind_and_a_rename_by_its_new_path() {
        let out = "M\0src/a.rs\0A\0tests/b.rs\0D\0old.txt\0R087\0from.rs\0to.rs\0";
        let got = parse_name_status(out);
        let want = [('M', "src/a.rs"), ('A', "tests/b.rs"), ('D', "old.txt"), ('R', "to.rs")];
        assert_eq!(got.iter().map(|c| (c.kind, c.path.as_str())).collect::<Vec<_>>(), want);
    }

    #[test]
    fn at_most_fifty_changed_files_are_kept() {
        let out: String = (0..80).map(|i| format!("M\0f{i}\0")).collect();
        let got = parse_name_status(&out);
        assert_eq!((got.len(), got.last().map(|c| c.path.as_str())), (CHANGED_FILES_MOST, Some("f49")));
    }

    #[test]
    fn outside_git_there_is_no_fingerprint_and_nothing_goes_stale() {
        let dir = super::super::persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        if crate::proc::git(dir.path(), &["rev-parse", "--show-toplevel"]).is_ok() {
            return;
        }
        assert_eq!(fingerprint(dir.path()), None);
        assert!(probe(&Probe { term: 1, dir: dir.path().to_path_buf(), trees: vec!["abc".into()] }).is_empty());
    }
}
