//! Snapshots: a checkout's files as they were when an agent's turn began, so a turn that
//! went wrong can be taken back. Each is a commit under the checkout's own refs
//! (`refs/worktree/…`), so none shows up in branches or history, and they go when the
//! worktree goes. The real index is never touched.

use crate::proc;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where a checkout's snapshots are kept: refs of that worktree alone.
const REFS: &str = "refs/worktree/seshi/snapshots";
/// How many a checkout keeps; older ones are dropped as new ones come.
pub const KEEP: usize = 30;
/// Said of the snapshot a restore takes first, so the restore can itself be taken back.
pub const BEFORE_RESTORE: &str = "before a restore";

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub commit: String,
    /// When it was taken (unix seconds).
    pub at: u64,
    /// What it was taken before ("before claude's turn").
    pub why: String,
}

fn git(top: &Path, args: &[&str]) -> Result<String, String> {
    proc::git(top, args).map(|s| s.trim().to_string())
}

/// git with an index of its own, so staging in the real one is left as it is.
fn git_with_index(top: &Path, index: &Path, args: &[&str]) -> Result<String, String> {
    proc::run(Command::new("git").arg("-C").arg(top).args(args).env("LC_ALL", "C").env("GIT_INDEX_FILE", index)).map(|s| s.trim().to_string())
}

/// The top of the checkout `dir` is in, or `None` outside git.
fn top_of(dir: &Path) -> Option<PathBuf> {
    git(dir, &["rev-parse", "--show-toplevel"]).ok().filter(|t| !t.is_empty()).map(PathBuf::from)
}

/// Everything in the checkout right now (tracked or not, never what's ignored) as a tree.
fn tree_of_now(top: &Path) -> Result<String, String> {
    let real = PathBuf::from(git(top, &["rev-parse", "--absolute-git-dir"])?).join("index");
    let index = real.with_file_name(format!("seshi-snapshot-index-{}", std::process::id()));
    // Started from the real index, git only reads the files that changed since.
    if real.exists() {
        std::fs::copy(&real, &index).map_err(|e| format!("couldn't copy the index: {e}"))?;
    }
    let tree = git_with_index(top, &index, &["add", "-A"]).and_then(|_| git_with_index(top, &index, &["write-tree"]));
    // Its lock goes with it if git was stopped halfway.
    for leftover in [index.clone(), index.with_extension("lock")] {
        match std::fs::remove_file(&leftover) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("couldn't remove {}: {e}", leftover.display())),
        }
    }
    tree
}

/// A checkout's snapshots, newest first.
pub fn list(dir: &Path) -> Vec<Snapshot> {
    let Some(top) = top_of(dir) else { return Vec::new() };
    let format = "--format=%(refname)%09%(objectname)%09%(contents:subject)";
    let Ok(out) = git(&top, &["for-each-ref", "--sort=-refname", format, REFS]) else { return Vec::new() };
    out.lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, '\t');
            let (name, commit, why) = (parts.next()?, parts.next()?, parts.next().unwrap_or(""));
            let millis: u64 = name.rsplit('/').next()?.parse().ok()?;
            Some(Snapshot { commit: commit.to_string(), at: millis / 1000, why: why.to_string() })
        })
        .collect()
}

/// Keep the checkout as it is now. `None` outside git. When nothing changed since the last
/// one, that one is the answer and nothing new is kept.
pub fn take(dir: &Path, why: &str) -> Result<Option<Snapshot>, String> {
    let Some(top) = top_of(dir) else { return Ok(None) };
    let tree = tree_of_now(&top)?;
    let kept = list(&top);
    if let Some(last) = kept.first()
        && git(&top, &["rev-parse", &format!("{}^{{tree}}", last.commit)]).as_deref() == Ok(tree.as_str())
    {
        return Ok(Some(last.clone()));
    }
    // Named as seshi's: a machine with no git identity set can still take one.
    let mut args = vec!["-c", "user.name=seshi", "-c", "user.email=seshi@localhost", "commit-tree", &tree, "-m", why];
    let head = git(&top, &["rev-parse", "--verify", "--quiet", "HEAD"]).ok();
    if let Some(head) = &head {
        args.extend(["-p", head]);
    }
    let commit = git(&top, &args)?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    // Milliseconds, zero-padded: names sort by time, and two in a second don't collide.
    git(&top, &["update-ref", &format!("{REFS}/{now:015}"), &commit])?;
    for old in kept.iter().skip(KEEP.saturating_sub(1)) {
        git(&top, &["update-ref", "-d", &format!("{REFS}/{:015}", ref_millis(&top, old)?)])?;
    }
    Ok(Some(Snapshot { commit, at: now / 1000, why: why.to_string() }))
}

/// The time in a snapshot's ref name (its `at` has lost the milliseconds).
fn ref_millis(top: &Path, s: &Snapshot) -> Result<u64, String> {
    let out = git(top, &["for-each-ref", "--format=%(refname)", "--points-at", &s.commit, REFS])?;
    out.lines().next().and_then(|n| n.rsplit('/').next()).and_then(|n| n.parse().ok()).ok_or_else(|| format!("no ref for snapshot {}", s.commit))
}

/// Put the checkout's files back as they were in `commit`. What's there now is kept as a
/// snapshot first, so this can be undone by restoring that one. Returns how many files
/// changed. Ignored files, the index and the branch are left alone.
pub fn restore(dir: &Path, commit: &str) -> Result<usize, String> {
    let top = top_of(dir).ok_or("not in a git repo")?;
    let now = take(&top, BEFORE_RESTORE)?.ok_or("not in a git repo")?;
    let differ = git(&top, &["diff", "--name-only", commit, &now.commit])?.lines().count();
    // Files made since then aren't part of how it was (they're in the snapshot just taken).
    let added = proc::git(&top, &["diff", "--name-only", "--diff-filter=A", "-z", commit, &now.commit])?;
    for file in added.split('\0').filter(|f| !f.is_empty()) {
        match std::fs::remove_file(top.join(file)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("couldn't remove {file}: {e}")),
        }
    }
    if differ > 0 {
        git(&top, &["restore", "--source", commit, "--worktree", "--", ":/"])?;
    }
    Ok(differ)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of this test's own, empty.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir = std::env::temp_dir().join(format!("seshi-snapshot-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn repo(tag: &str) -> Scratch {
        let dir = Scratch::new(tag);
        for args in [&["init", "-q"][..], &["config", "user.name", "t"], &["config", "user.email", "t@t"], &["config", "core.autocrlf", "false"]] {
            proc::git(dir.path(), args).unwrap();
        }
        std::fs::write(dir.path().join("kept.txt"), "one\n").unwrap();
        std::fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        proc::git(dir.path(), &["add", "-A"]).unwrap();
        proc::git(dir.path(), &["commit", "-qm", "first"]).unwrap();
        dir
    }

    #[test]
    fn a_restore_puts_the_files_back_and_can_be_taken_back() {
        let dir = repo("restore");
        let at = |name: &str| dir.path().join(name);
        let read = |name: &str| std::fs::read_to_string(at(name)).ok();
        // Before the turn: one change not committed, one file git hasn't seen, one staged.
        std::fs::write(at("kept.txt"), "one\ntwo\n").unwrap();
        std::fs::write(at("new.txt"), "untracked\n").unwrap();
        std::fs::write(at("staged.txt"), "staged\n").unwrap();
        proc::git(dir.path(), &["add", "staged.txt"]).unwrap();
        let staged_before = proc::git(dir.path(), &["diff", "--cached", "--name-only"]).unwrap();
        let before = take(dir.path(), "before claude's turn").unwrap().expect("it's a repo");
        assert_eq!(take(dir.path(), "again").unwrap().unwrap().commit, before.commit, "nothing changed: nothing new kept");
        assert_eq!(list(dir.path()).len(), 1);
        assert_eq!(proc::git(dir.path(), &["diff", "--cached", "--name-only"]).unwrap(), staged_before, "the real index is as it was");
        assert_eq!(proc::git(dir.path(), &["branch", "--list"]).unwrap().lines().count(), 1, "no branch made");
        // The turn: edits one, deletes one, makes one, and writes a file git ignores.
        std::fs::write(at("kept.txt"), "ruined\n").unwrap();
        std::fs::remove_file(at("new.txt")).unwrap();
        std::fs::write(at("made.txt"), "by the agent\n").unwrap();
        std::fs::write(at("ignored.txt"), "local\n").unwrap();
        assert_eq!(restore(dir.path(), &before.commit).unwrap(), 3);
        assert_eq!(read("kept.txt").as_deref(), Some("one\ntwo\n"), "the edit is undone");
        assert_eq!(read("new.txt").as_deref(), Some("untracked\n"), "the deleted file is back");
        assert_eq!(read("made.txt"), None, "what the turn made is gone");
        assert_eq!(read("ignored.txt").as_deref(), Some("local\n"), "ignored files are left alone");
        // What was there is the newest snapshot: restoring it takes the restore back.
        let kept = list(dir.path());
        assert_eq!((kept.len(), kept[0].why.as_str(), kept[1].commit.as_str()), (2, BEFORE_RESTORE, before.commit.as_str()));
        restore(dir.path(), &kept[0].commit).unwrap();
        assert_eq!((read("kept.txt").as_deref(), read("made.txt").as_deref(), read("new.txt")), (Some("ruined\n"), Some("by the agent\n"), None));
    }

    #[test]
    fn each_worktree_keeps_its_own() {
        let dir = repo("worktrees");
        let other = dir.path().join("wt");
        proc::git(dir.path(), &["worktree", "add", "-q", "-b", "side", &other.to_string_lossy()]).unwrap();
        std::fs::write(other.join("kept.txt"), "in the worktree\n").unwrap();
        let taken = take(&other, "before codex's turn").unwrap().unwrap();
        assert_eq!(list(&other), vec![taken.clone()]);
        assert!(list(dir.path()).is_empty(), "the main checkout has none of them");
        // Restored there, the main checkout's files are not touched.
        std::fs::write(other.join("kept.txt"), "ruined\n").unwrap();
        restore(&other, &taken.commit).unwrap();
        assert_eq!(std::fs::read_to_string(other.join("kept.txt")).unwrap(), "in the worktree\n");
        assert_eq!(std::fs::read_to_string(dir.path().join("kept.txt")).unwrap(), "one\n");
    }

    #[test]
    fn only_the_newest_are_kept_and_outside_git_there_are_none() {
        let dir = repo("keep");
        for i in 0..KEEP + 3 {
            std::fs::write(dir.path().join("kept.txt"), format!("{i}\n")).unwrap();
            take(dir.path(), &format!("turn {i}")).unwrap();
        }
        let kept = list(dir.path());
        assert_eq!(kept.len(), KEEP);
        assert_eq!(kept[0].why, format!("turn {}", KEEP + 2), "newest first");
        let plain = Scratch::new("plain");
        assert_eq!(take(plain.path(), "x"), Ok(None));
        assert!(list(plain.path()).is_empty() && restore(plain.path(), "abc").is_err());
    }
}
