//! What removing a worktree would lose: the one question behind the server's quiet sweep
//! of worktrees nothing runs in any more, and behind the window's clean-up list.

use std::path::Path;

/// Where a worktree stands. Read with git: call it off the UI thread and off the server's
/// loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    /// Changed and new files that aren't committed.
    pub dirty: usize,
    /// Its branch (none: a detached HEAD).
    pub branch: Option<String>,
    /// Every commit it has is also on another branch or on a remote: its own branch could
    /// go and nothing would be lost.
    pub merged: bool,
    /// When it last committed, as git says it ("3 days ago").
    pub last_commit: String,
}

impl Standing {
    /// Nothing here exists only here: it can be removed without asking.
    pub fn safe(&self) -> bool {
        self.dirty == 0 && self.merged
    }

    /// Why it's still around, in a few words.
    pub fn says(&self) -> String {
        let ago = if self.last_commit.is_empty() { String::new() } else { format!(" · last commit {}", self.last_commit) };
        match (self.dirty, self.merged) {
            (0, true) => format!("merged, nothing uncommitted{ago}"),
            (0, false) => format!("not merged anywhere{ago}"),
            (n, _) => format!("{n} uncommitted change{}{ago}", if n == 1 { "" } else { "s" }),
        }
    }
}

pub fn standing(dir: &Path) -> Standing {
    let git = |args: &[&str]| crate::proc::git(dir, args).map(|o| o.trim().to_string()).unwrap_or_default();
    let dirty = git(&["status", "--porcelain"]).lines().filter(|l| !l.trim().is_empty()).count();
    let branch = Some(git(&["rev-parse", "--abbrev-ref", "HEAD"])).filter(|b| !b.is_empty() && b != "HEAD");
    // Every ref that has this checkout's commit, other than its own branch and that
    // branch on a remote.
    let own = branch.as_deref().unwrap_or("");
    let merged = git(&["for-each-ref", "--contains", "HEAD", "--format=%(refname)", "refs/heads", "refs/remotes"])
        .lines()
        .map(str::trim)
        .filter(|r| !r.is_empty() && !r.ends_with("/HEAD"))
        .any(|r| own.is_empty() || (r != format!("refs/heads/{own}") && !(r.starts_with("refs/remotes/") && r.ends_with(&format!("/{own}")))));
    Standing { dirty, branch, merged, last_commit: git(&["log", "-1", "--format=%cr"]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_worktree_is_safe_to_remove_only_when_nothing_lives_only_there() {
        let root = std::env::temp_dir().join(format!("vibecasso-tidy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let git = |dir: &Path, args: &[&str]| crate::proc::git(dir, args).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        git(&repo, &["config", "user.email", "t@t"]);
        git(&repo, &["config", "user.name", "t"]);
        std::fs::write(repo.join("a.ts"), "a").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-qm", "one"]);
        let wt = root.join("task");
        git(&repo, &["worktree", "add", "-q", "-b", "task", wt.to_str().unwrap()]);
        // Just cut from main, nothing of its own yet.
        let s = standing(&wt);
        assert_eq!((s.dirty, s.branch.as_deref(), s.merged), (0, Some("task"), true));
        assert!(s.safe() && s.says().starts_with("merged, nothing uncommitted"), "{}", s.says());
        // An edit that isn't committed.
        std::fs::write(wt.join("a.ts"), "b").unwrap();
        std::fs::write(wt.join("new.ts"), "c").unwrap();
        let s = standing(&wt);
        assert!(s.dirty == 2 && !s.safe() && s.says().starts_with("2 uncommitted changes"), "{}", s.says());
        // Committed, but only on its own branch.
        git(&wt, &["add", "."]);
        git(&wt, &["commit", "-qm", "two"]);
        let s = standing(&wt);
        assert!(s.dirty == 0 && !s.merged && !s.safe() && s.says().starts_with("not merged anywhere"), "{}", s.says());
        // Merged into main: safe again.
        git(&repo, &["merge", "-q", "task"]);
        assert!(standing(&wt).safe());
        // A detached checkout of a commit main has: safe. One with a commit of its own: not.
        let det = root.join("look");
        git(&repo, &["worktree", "add", "-q", "--detach", det.to_str().unwrap()]);
        let s = standing(&det);
        assert!(s.branch.is_none() && s.safe());
        std::fs::write(det.join("only-here.ts"), "x").unwrap();
        git(&det, &["add", "."]);
        git(&det, &["commit", "-qm", "loose"]);
        assert!(!standing(&det).safe(), "a commit no branch has would be lost");
        let _ = std::fs::remove_dir_all(&root);
    }
}
