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
    /// Every commit it has is also on another branch or on a remote, or its pull request was
    /// merged at exactly this commit (a squash merge leaves the commits on no other branch):
    /// its own branch could go and nothing would be lost.
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
        let ago = if self.last_commit.is_empty() {
            String::new()
        } else {
            format!(" · last commit {}", self.last_commit)
        };
        match (self.dirty, self.merged) {
            (0, true) => format!("merged, nothing uncommitted{ago}"),
            (0, false) => format!("not merged anywhere{ago}"),
            (n, _) => format!(
                "{n} uncommitted change{}{ago}",
                if n == 1 { "" } else { "s" }
            ),
        }
    }
}

/// Whether a pull request from `branch` was merged with `head` as its last commit. Asks `gh`
/// (blocks); when it isn't installed, isn't logged in or can't be reached, the answer is no,
/// which only means the person is asked.
fn merged_by_pull_request(dir: &Path, branch: &str, head: &str) -> bool {
    if head.is_empty() || !crate::proc::on_path("gh") {
        return false;
    }
    let mut cmd = std::process::Command::new("gh");
    if let Some(path) = crate::login_path::resolved() {
        cmd.env("PATH", path);
    }
    cmd.current_dir(dir)
        .args([
            "pr",
            "list",
            "--head",
            branch,
            "--state",
            "merged",
            "--json",
            "headRefOid",
            "--limit",
            "10",
        ])
        .env("GH_PROMPT_DISABLED", "1")
        .env("NO_COLOR", "1");
    crate::proc::run(&mut cmd).is_ok_and(|json| merged_with_head(&json, head))
}

/// Whether `gh pr list --json headRefOid` output has a pull request whose last commit is `head`.
fn merged_with_head(json: &str, head: &str) -> bool {
    #[derive(serde::Deserialize)]
    struct Listed {
        #[serde(rename = "headRefOid")]
        head: String,
    }
    serde_json::from_str::<Vec<Listed>>(json)
        .is_ok_and(|list| list.iter().any(|pr| pr.head == head))
}

pub fn standing(dir: &Path) -> Standing {
    let git = |args: &[&str]| {
        crate::proc::git(dir, args)
            .map(|o| o.trim().to_string())
            .unwrap_or_default()
    };
    let dirty = git(&["status", "--porcelain"])
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    let branch =
        Some(git(&["rev-parse", "--abbrev-ref", "HEAD"])).filter(|b| !b.is_empty() && b != "HEAD");
    // Every ref that has this checkout's commit, other than its own branch and that
    // branch on a remote.
    let own = branch.as_deref().unwrap_or("");
    let merged = git(&[
        "for-each-ref",
        "--contains",
        "HEAD",
        "--format=%(refname)",
        "refs/heads",
        "refs/remotes",
    ])
    .lines()
    .map(str::trim)
    .filter(|r| !r.is_empty() && !r.ends_with("/HEAD"))
    .any(|r| {
        own.is_empty()
            || (r != format!("refs/heads/{own}")
                && !(r.starts_with("refs/remotes/") && r.ends_with(&format!("/{own}"))))
    });
    // A squash or rebase merge puts no commit of this branch on another one, so git alone calls
    // it unmerged. The pull request says otherwise, but only for the commit it merged.
    let merged = merged
        || branch
            .as_deref()
            .is_some_and(|b| merged_by_pull_request(dir, b, &git(&["rev-parse", "HEAD"])));
    Standing {
        dirty,
        branch,
        merged,
        last_commit: git(&["log", "-1", "--format=%cr"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_worktree_is_safe_to_remove_only_when_nothing_lives_only_there() {
        let root = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-tidy-{}"),
            std::process::id()
        ));
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
        git(
            &repo,
            &["worktree", "add", "-q", "-b", "task", wt.to_str().unwrap()],
        );
        // Just cut from main, nothing of its own yet.
        let s = standing(&wt);
        assert_eq!(
            (s.dirty, s.branch.as_deref(), s.merged),
            (0, Some("task"), true)
        );
        assert!(
            s.safe() && s.says().starts_with("merged, nothing uncommitted"),
            "{}",
            s.says()
        );
        // An edit that isn't committed.
        std::fs::write(wt.join("a.ts"), "b").unwrap();
        std::fs::write(wt.join("new.ts"), "c").unwrap();
        let s = standing(&wt);
        assert!(
            s.dirty == 2 && !s.safe() && s.says().starts_with("2 uncommitted changes"),
            "{}",
            s.says()
        );
        // Committed, but only on its own branch.
        git(&wt, &["add", "."]);
        git(&wt, &["commit", "-qm", "two"]);
        let s = standing(&wt);
        assert!(
            s.dirty == 0 && !s.merged && !s.safe() && s.says().starts_with("not merged anywhere"),
            "{}",
            s.says()
        );
        // Merged into main: safe again.
        git(&repo, &["merge", "-q", "task"]);
        assert!(standing(&wt).safe());
        // A detached checkout of a commit main has: safe. One with a commit of its own: not.
        let det = root.join("look");
        git(
            &repo,
            &["worktree", "add", "-q", "--detach", det.to_str().unwrap()],
        );
        let s = standing(&det);
        assert!(s.branch.is_none() && s.safe());
        std::fs::write(det.join("only-here.ts"), "x").unwrap();
        git(&det, &["add", "."]);
        git(&det, &["commit", "-qm", "loose"]);
        assert!(
            !standing(&det).safe(),
            "a commit no branch has would be lost"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_merged_pull_request_counts_only_at_the_commit_it_merged() {
        let listed = r#"[{"headRefOid":"abc123"},{"headRefOid":"def456"}]"#;
        assert!(merged_with_head(listed, "def456"));
        // Commits made after the merge are not in it.
        assert!(!merged_with_head(listed, "fff000"));
        assert!(!merged_with_head("[]", "abc123"));
        assert!(!merged_with_head("not json", "abc123"));
    }
}
