//! Snapshots: a checkout's files as they were when an agent's turn began and when it ended,
//! so a turn that went wrong can be taken back. Each is a commit under the checkout's own refs
//! (`refs/worktree/…`), so none shows up in branches or history, and they go when the
//! worktree goes. The real index is never touched.

use crate::brand::APP_COMMAND;
use crate::proc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

/// Where a checkout's snapshots are kept: refs of that worktree alone.
const REFS: &str = concat!("refs/worktree/", env!("APP_COMMAND"), "/snapshots");
/// How many a checkout keeps; older ones are dropped as new ones come. Two a turn (before
/// and after), so at least thirty turns.
pub const KEEP: usize = 60;
/// Said of the snapshot a restore takes first, so the restore can itself be taken back.
pub const BEFORE_RESTORE: &str = "before a restore";

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub commit: String,
    /// When it was taken (unix seconds).
    pub at: u64,
    /// When it was taken ("before claude's turn", "after claude's turn").
    pub why: String,
}

fn git(top: &Path, args: &[&str]) -> Result<String, String> {
    proc::git(top, args).map(|s| s.trim().to_string())
}

/// git with an index of its own, so staging in the real one is left as it is.
fn git_with_index(top: &Path, index: &Path, args: &[&str]) -> Result<String, String> {
    proc::run(
        Command::new("git")
            .arg("-C")
            .arg(top)
            .args(args)
            .env("LC_ALL", "C")
            .env("GIT_INDEX_FILE", index),
    )
    .map(|s| s.trim().to_string())
}

/// The top of the checkout `dir` is in, or `None` outside git.
fn top_of(dir: &Path) -> Option<PathBuf> {
    git(dir, &["rev-parse", "--show-toplevel"])
        .ok()
        .filter(|t| !t.is_empty())
        .map(PathBuf::from)
}

/// Everything in the checkout right now (tracked or not, never what's ignored) as a tree.
fn tree_of_now(top: &Path) -> Result<String, String> {
    let real = PathBuf::from(git(top, &["rev-parse", "--absolute-git-dir"])?).join("index");
    // Numbered too: a check's fingerprint and a snapshot may be taken of one checkout at once.
    static TAKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = TAKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let index = real.with_file_name(format!(
        "{APP_COMMAND}-snapshot-index-{}-{n}",
        std::process::id()
    ));
    // Started from the real index, git only reads the files that changed since.
    // A failed copy still falls through to the clean-up below, so no private index is left in .git.
    let copied = if real.exists() {
        copy_index(&real, &index)
    } else {
        Ok(())
    };
    let tree = copied
        .and_then(|_| git_with_index(top, &index, &["add", "-A"]))
        .and_then(|_| git_with_index(top, &index, &["write-tree"]));
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

/// Copy the index with its modified time. git takes a file whose stat matches its entry as
/// unchanged only when the index is newer than the file, so a copy dated now would hide an
/// edit made in the second the real index was written. Windows' copy keeps the time; Linux's
/// does not.
fn copy_index(real: &Path, copy: &Path) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("couldn't copy the index: {e}");
    // The time is read first: if git rewrites the index meanwhile, the copy is dated too early,
    // which only makes git re-read more files, never fewer.
    let written = std::fs::metadata(real)
        .and_then(|m| m.modified())
        .map_err(failed)?;
    std::fs::copy(real, copy).map_err(failed)?;
    std::fs::File::options()
        .write(true)
        .open(copy)
        .and_then(|f| f.set_modified(written))
        .map_err(failed)
}

/// The tree of everything in `dir`'s checkout right now, as a snapshot would keep it.
/// `None` outside git.
pub fn tree_now(dir: &Path) -> Option<Result<String, String>> {
    top_of(dir).map(|top| tree_of_now(&top))
}

/// A checkout's snapshots, newest first.
pub fn list(dir: &Path) -> Vec<Snapshot> {
    let Some(top) = top_of(dir) else {
        return Vec::new();
    };
    let format = "--format=%(refname)%09%(objectname)%09%(contents:subject)";
    let Ok(out) = git(&top, &["for-each-ref", "--sort=-refname", format, REFS]) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, '\t');
            let (name, commit, why) = (parts.next()?, parts.next()?, parts.next().unwrap_or(""));
            let millis: u64 = name.rsplit('/').next()?.parse().ok()?;
            Some(Snapshot {
                commit: commit.to_string(),
                at: millis / 1000,
                why: why.to_string(),
            })
        })
        .collect()
}

/// Keep the checkout as it is now. `None` outside git. When nothing changed since the last
/// one, that one is the answer and nothing new is kept.
pub fn take(dir: &Path, why: &str) -> Result<Option<Snapshot>, String> {
    let Some(top) = top_of(dir) else {
        return Ok(None);
    };
    let lock = checkout_lock(&top);
    let _taking = lock.lock().unwrap_or_else(PoisonError::into_inner);
    take_locked(&top, why).map(Some)
}

/// `take` for a checkout whose lock the caller holds.
fn take_locked(top: &Path, why: &str) -> Result<Snapshot, String> {
    let tree = tree_of_now(top)?;
    let kept = list(top);
    if let Some(last) = kept.first()
        && git(top, &["rev-parse", &format!("{}^{{tree}}", last.commit)]).as_deref()
            == Ok(tree.as_str())
    {
        return Ok(last.clone());
    }
    // Named as the app's: a machine with no git identity set can still take one.
    let mut args = vec![
        "-c",
        concat!("user.name=", env!("APP_COMMAND")),
        "-c",
        concat!("user.email=", env!("APP_COMMAND"), "@localhost"),
        "commit-tree",
        &tree,
        "-m",
        why,
    ];
    let head = git(top, &["rev-parse", "--verify", "--quiet", "HEAD"]).ok();
    if let Some(head) = &head {
        args.extend(["-p", head]);
    }
    let commit = git(top, &args)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    // Milliseconds, zero-padded: names sort by time, and two in a second don't collide.
    git(top, &["update-ref", &format!("{REFS}/{now:015}"), &commit])?;
    prune(top, kept.iter().skip(KEEP.saturating_sub(1)))?;
    Ok(Snapshot {
        commit,
        at: now / 1000,
        why: why.to_string(),
    })
}

/// One take at a time per checkout: two at once would both miss the other's snapshot, keep
/// the same files twice, and both drop the same oldest ref, the second failing.
fn checkout_lock(top: &Path) -> Arc<Mutex<()>> {
    static LOCKS: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
        LazyLock::new(Default::default);
    LOCKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(top.to_path_buf())
        .or_default()
        .clone()
}

/// Drop `old` snapshots. One already gone was dropped by another process, such as the
/// terminal app's restore, which this process's lock does not cover.
fn prune<'a>(top: &Path, old: impl Iterator<Item = &'a Snapshot>) -> Result<(), String> {
    for old in old {
        if let Some(millis) = ref_millis(top, old)? {
            git(top, &["update-ref", "-d", &format!("{REFS}/{millis:015}")])?;
        }
    }
    Ok(())
}

/// The time in a snapshot's ref name (its `at` has lost the milliseconds); `None` when no
/// ref points at it any more.
fn ref_millis(top: &Path, s: &Snapshot) -> Result<Option<u64>, String> {
    let out = git(
        top,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "--points-at",
            &s.commit,
            REFS,
        ],
    )?;
    let Some(name) = out.lines().next() else {
        return Ok(None);
    };
    name.rsplit('/')
        .next()
        .and_then(|n| n.parse().ok())
        .map(Some)
        .ok_or_else(|| format!("snapshot ref {name} has no time in its name"))
}

/// How many files `restore` of `commit` would change now, counted as it counts them, with
/// nothing kept and no file touched.
pub fn restore_count(dir: &Path, commit: &str) -> Result<usize, String> {
    let top = top_of(dir).ok_or("not in a git repo")?;
    let now = tree_of_now(&top)?;
    files_between(&top, commit, &now)
}

fn files_between(top: &Path, from: &str, to: &str) -> Result<usize, String> {
    Ok(git(top, &["diff", "--name-only", from, to])?.lines().count())
}

/// Put the checkout's files back as they were in `commit`. What's there now is kept as a
/// snapshot first, so this can be undone by restoring that one. Returns how many files
/// changed. Ignored files, the index and the branch are left alone.
pub fn restore(dir: &Path, commit: &str) -> Result<usize, String> {
    let top = top_of(dir).ok_or("not in a git repo")?;
    // Held to the end: a snapshot taken meanwhile would keep half-restored files.
    let lock = checkout_lock(&top);
    let _restoring = lock.lock().unwrap_or_else(PoisonError::into_inner);
    let now = take_locked(&top, BEFORE_RESTORE)?;
    let differ = files_between(&top, commit, &now.commit)?;
    // Files made since then aren't part of how it was (they're in the snapshot just taken).
    let added = proc::git(
        &top,
        &[
            "diff",
            "--name-only",
            "--diff-filter=A",
            "-z",
            commit,
            &now.commit,
        ],
    )?;
    for file in added.split('\0').filter(|f| !f.is_empty()) {
        match std::fs::remove_file(top.join(file)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("couldn't remove {file}: {e}")),
        }
    }
    if differ > 0 {
        git(
            &top,
            &["restore", "--source", commit, "--worktree", "--", ":/"],
        )?;
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
            let dir = std::env::temp_dir().join(format!(
                "{APP_COMMAND}-snapshot-{tag}-{}",
                std::process::id()
            ));
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
        for args in [
            &["init", "-q"][..],
            &["config", "user.name", "t"],
            &["config", "user.email", "t@t"],
            &["config", "core.autocrlf", "false"],
        ] {
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
        let before = take(dir.path(), "before claude's turn")
            .unwrap()
            .expect("it's a repo");
        assert_eq!(
            take(dir.path(), "again").unwrap().unwrap().commit,
            before.commit,
            "nothing changed: nothing new kept"
        );
        assert_eq!(list(dir.path()).len(), 1);
        assert_eq!(
            proc::git(dir.path(), &["diff", "--cached", "--name-only"]).unwrap(),
            staged_before,
            "the real index is as it was"
        );
        assert_eq!(
            proc::git(dir.path(), &["branch", "--list"])
                .unwrap()
                .lines()
                .count(),
            1,
            "no branch made"
        );
        // The turn: edits one, deletes one, makes one, and writes a file git ignores.
        std::fs::write(at("kept.txt"), "ruined\n").unwrap();
        std::fs::remove_file(at("new.txt")).unwrap();
        std::fs::write(at("made.txt"), "by the agent\n").unwrap();
        std::fs::write(at("ignored.txt"), "local\n").unwrap();
        assert_eq!(restore(dir.path(), &before.commit).unwrap(), 3);
        assert_eq!(
            read("kept.txt").as_deref(),
            Some("one\ntwo\n"),
            "the edit is undone"
        );
        assert_eq!(
            read("new.txt").as_deref(),
            Some("untracked\n"),
            "the deleted file is back"
        );
        assert_eq!(read("made.txt"), None, "what the turn made is gone");
        assert_eq!(
            read("ignored.txt").as_deref(),
            Some("local\n"),
            "ignored files are left alone"
        );
        // What was there is the newest snapshot: restoring it takes the restore back.
        let kept = list(dir.path());
        assert_eq!(
            (kept.len(), kept[0].why.as_str(), kept[1].commit.as_str()),
            (2, BEFORE_RESTORE, before.commit.as_str())
        );
        restore(dir.path(), &kept[0].commit).unwrap();
        assert_eq!(
            (
                read("kept.txt").as_deref(),
                read("made.txt").as_deref(),
                read("new.txt")
            ),
            (Some("ruined\n"), Some("by the agent\n"), None)
        );
    }

    #[test]
    fn each_worktree_keeps_its_own() {
        let dir = repo("worktrees");
        let other = dir.path().join("wt");
        proc::git(
            dir.path(),
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "side",
                &other.to_string_lossy(),
            ],
        )
        .unwrap();
        std::fs::write(other.join("kept.txt"), "in the worktree\n").unwrap();
        let taken = take(&other, "before codex's turn").unwrap().unwrap();
        assert_eq!(list(&other), vec![taken.clone()]);
        assert!(
            list(dir.path()).is_empty(),
            "the main checkout has none of them"
        );
        // Restored there, the main checkout's files are not touched.
        std::fs::write(other.join("kept.txt"), "ruined\n").unwrap();
        restore(&other, &taken.commit).unwrap();
        assert_eq!(
            std::fs::read_to_string(other.join("kept.txt")).unwrap(),
            "in the worktree\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("kept.txt")).unwrap(),
            "one\n"
        );
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

    /// A turn's end and the next turn's start can be kept at the same moment, as can two panes
    /// in one checkout. Nothing changed between them, so one snapshot answers them all.
    #[test]
    fn snapshots_taken_at_once_keep_one_and_none_fails() {
        let dir = repo("at-once");
        std::fs::write(dir.path().join("kept.txt"), "changed\n").unwrap();
        let path = dir.path();
        let taken: Vec<_> = std::thread::scope(|s| {
            let runs: Vec<_> = (0..8)
                .map(|i| s.spawn(move || take(path, &format!("turn {i}"))))
                .collect();
            runs.into_iter().map(|r| r.join().unwrap()).collect()
        });
        let kept = list(dir.path());
        assert_eq!(kept.len(), 1, "{kept:?}");
        for t in taken {
            assert_eq!(t.unwrap().unwrap().commit, kept[0].commit);
        }
    }

    /// Another process (the terminal app's restore) can drop the same old snapshot first.
    #[test]
    fn pruning_a_snapshot_already_gone_is_not_an_error() {
        let dir = repo("pruned");
        take(dir.path(), "old").unwrap();
        let gone = list(dir.path());
        for name in git(dir.path(), &["for-each-ref", "--format=%(refname)", REFS])
            .unwrap()
            .lines()
        {
            git(dir.path(), &["update-ref", "-d", name]).unwrap();
        }
        assert_eq!(prune(dir.path(), gone.iter()), Ok(()));
    }

    /// git trusts a file that looks unchanged only when the index was written after the
    /// file's last change; an edit in the second the index was written is read in full. Here
    /// stat sees whole seconds only, as with a git built without sub-second times, and that
    /// second is in the past, as when a fingerprint is taken a second or more later.
    #[test]
    fn an_edit_in_the_second_the_index_was_written_is_seen() {
        let dir = repo("racy");
        let at = |name: &str| dir.path().join(name);
        let index = PathBuf::from(git(dir.path(), &["rev-parse", "--absolute-git-dir"]).unwrap())
            .join("index");
        let back_then = std::time::SystemTime::now() - std::time::Duration::from_secs(10);
        let date = |file: &Path| {
            std::fs::File::options()
                .write(true)
                .open(file)
                .unwrap()
                .set_modified(back_then)
                .unwrap()
        };
        for (key, value) in [("core.checkStat", "minimal"), ("core.trustCtime", "false")] {
            proc::git(dir.path(), &["config", key, value]).unwrap();
        }
        date(&at("kept.txt"));
        proc::git(dir.path(), &["update-index", "--refresh"]).unwrap();
        std::fs::write(at("kept.txt"), "two\n").unwrap();
        date(&at("kept.txt"));
        date(&index);
        let tree = tree_now(dir.path()).unwrap().unwrap();
        assert_eq!(
            git(dir.path(), &["rev-parse", &format!("{tree}:kept.txt")]),
            git(dir.path(), &["hash-object", "kept.txt"])
        );
    }
}
