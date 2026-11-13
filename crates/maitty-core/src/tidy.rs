//! What removing a worktree would lose: the one question behind the server's quiet sweep
//! of worktrees nothing runs in any more, and behind the window's clean-up list.

use std::path::Path;
use std::time::{Duration, SystemTime};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// The most files looked at for a worktree's newest change. One with more is never flagged
/// inactive: a long walk is not worth it, and being unsure must not flag one.
const NEWEST_CHANGE_FILES: usize = 20_000;

pub use crate::protocol::WorktreeStanding as Standing;

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

/// Whole days since anything happened in `dir`, its last commit or its newest file change,
/// when that is at least `after` days before `now`. None when `after` is 0, when there is no
/// commit, or when the newest change can't be told.
fn inactive_days(dir: &Path, after: u32, now: SystemTime) -> Option<u32> {
    if after == 0 {
        return None;
    }
    let days_since = |t: SystemTime| {
        now.duration_since(t)
            .map_or(0, |d| d.as_secs() / DAY.as_secs())
    };
    let committed = days_since(last_commit_at(dir)?);
    // Looking at the files costs more than one git call: only for a commit already old enough.
    if committed < u64::from(after) {
        return None;
    }
    let idle = committed.min(days_since(newest_change(dir, NEWEST_CHANGE_FILES)?));
    (idle >= u64::from(after)).then(|| u32::try_from(idle).unwrap_or(u32::MAX))
}

fn last_commit_at(dir: &Path) -> Option<SystemTime> {
    let secs = crate::proc::git(dir, &["log", "-1", "--format=%ct"])
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

/// When the newest file git sees in `dir` (tracked, or new and not ignored) last changed; the
/// epoch when there are none. None when git can't list them or there are more than `cap`.
fn newest_change(dir: &Path, cap: usize) -> Option<SystemTime> {
    let listed = crate::proc::git(
        dir,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )
    .ok()?;
    let files: Vec<&str> = listed.split('\0').filter(|f| !f.is_empty()).collect();
    if files.len() > cap {
        return None;
    }
    Some(
        files
            .iter()
            // A tracked file deleted from disk has no time to give; `dirty` already counts it.
            .filter_map(|f| {
                std::fs::metadata(dir.join(f))
                    .and_then(|m| m.modified())
                    .ok()
            })
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH),
    )
}

/// `inactive_after`: `worktree.inactive_days`, the days without a commit or a file change
/// after which it is flagged (0: never).
pub fn standing(dir: &Path, inactive_after: u32) -> Standing {
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
        inactive_days: inactive_days(dir, inactive_after, SystemTime::now()),
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
        let s = standing(&wt, 14);
        assert_eq!(
            (s.dirty, s.branch.as_deref(), s.merged, s.inactive_days),
            (0, Some("task"), true, None)
        );
        assert!(
            s.safe() && s.says().starts_with("merged, nothing uncommitted"),
            "{}",
            s.says()
        );
        // An edit that isn't committed.
        std::fs::write(wt.join("a.ts"), "b").unwrap();
        std::fs::write(wt.join("new.ts"), "c").unwrap();
        let s = standing(&wt, 14);
        assert!(
            s.dirty == 2 && !s.safe() && s.says().starts_with("2 uncommitted changes"),
            "{}",
            s.says()
        );
        // Committed, but only on its own branch.
        git(&wt, &["add", "."]);
        git(&wt, &["commit", "-qm", "two"]);
        let s = standing(&wt, 14);
        assert!(
            s.dirty == 0 && !s.merged && !s.safe() && s.says().starts_with("not merged anywhere"),
            "{}",
            s.says()
        );
        // Merged into main: safe again.
        git(&repo, &["merge", "-q", "task"]);
        assert!(standing(&wt, 14).safe());
        // A detached checkout of a commit main has: safe. One with a commit of its own: not.
        let det = root.join("look");
        git(
            &repo,
            &["worktree", "add", "-q", "--detach", det.to_str().unwrap()],
        );
        let s = standing(&det, 14);
        assert!(s.branch.is_none() && s.safe());
        std::fs::write(det.join("only-here.ts"), "x").unwrap();
        git(&det, &["add", "."]);
        git(&det, &["commit", "-qm", "loose"]);
        assert!(
            !standing(&det, 14).safe(),
            "a commit no branch has would be lost"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_worktree_is_inactive_only_when_its_last_commit_and_newest_file_are_both_old() {
        let root = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-tidy-idle-{}"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let git = |args: &[&str]| crate::proc::git(&root, args).unwrap();
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(root.join(".gitignore"), "node_modules/\n").unwrap();
        std::fs::write(root.join("a.ts"), "a").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "one"]);
        // The clock is the commit's own time plus some days, and every file's time is set, so
        // nothing hangs on when the test runs.
        let committed = last_commit_at(&root).unwrap();
        let later = |days: u32| committed + DAY * days;
        let touch = |name: &str, at: SystemTime| {
            let path = root.join(name);
            if !path.exists() {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, "x").unwrap();
            }
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(at)
                .unwrap();
        };
        touch(".gitignore", committed);
        touch("a.ts", committed);

        assert_eq!(inactive_days(&root, 14, later(30)), Some(30));
        assert_eq!(inactive_days(&root, 14, later(10)), None, "a recent commit");
        assert_eq!(inactive_days(&root, 0, later(30)), None, "0 turns it off");
        touch("node_modules/dep.js", later(29));
        assert_eq!(
            inactive_days(&root, 14, later(30)),
            Some(30),
            "an ignored file is not a change"
        );
        touch("notes.md", later(25));
        assert_eq!(
            inactive_days(&root, 14, later(30)),
            None,
            "a new file changed 5 days ago"
        );
        touch("notes.md", later(10));
        assert_eq!(inactive_days(&root, 14, later(30)), Some(20));
        assert_eq!(
            newest_change(&root, 1),
            None,
            "past the cap, the newest change is not known"
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
