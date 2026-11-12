//! Tasks: a worktree workspace with an agent working in it. The list shows what stage each
//! is at; the review screen shows what it changed and ships it (commit, merge, PR) or throws
//! it away.

use crate::client::ids::ClientTerm;
use crate::proc::git;
use crate::protocol::WsId;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Ready,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskRow {
    pub ws: WsId,
    pub name: String,
    pub branch: String,
    pub base: String,
    pub stage: Stage,
    pub summary: String,
    pub dirty: u32,
    pub ahead: u32,
    pub agent: Option<ClientTerm>,
    pub dir: PathBuf,
    pub root: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Changed {
    pub path: String,
    pub added: i64,
    pub removed: i64,
    pub untracked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Review {
    pub task: TaskRow,
    /// The commit the task branched from.
    pub merge_base: String,
    pub files: Vec<Changed>,
    pub sel: usize,
    pub diff: Vec<String>,
    /// Which file `diff` is for (or being loaded for); None: load it.
    pub diff_sel: Option<usize>,
    pub scroll: u16,
    /// An action waiting for y/n: (label, key).
    pub confirm: Option<(String, char)>,
}

/// The most of an untracked file read to show or count it.
const UNTRACKED_READ_CAP: u64 = 512 * 1024;

/// The start of a file, at most `cap` bytes, as text.
fn read_head(path: &std::path::Path, cap: u64) -> std::io::Result<String> {
    use std::io::Read;
    let mut s = Vec::new();
    std::fs::File::open(path)?.take(cap).read_to_end(&mut s)?;
    Ok(String::from_utf8_lossy(&s).into_owned())
}

/// What a diff needs, so it can be worked out off the UI thread.
#[derive(Debug, Clone)]
pub struct DiffJob {
    pub dir: std::path::PathBuf,
    pub merge_base: String,
    pub file: Option<Changed>,
}

impl Review {
    pub fn diff_job(&self) -> DiffJob {
        DiffJob {
            dir: self.task.dir.clone(),
            merge_base: self.merge_base.clone(),
            file: self.files.get(self.sel).cloned(),
        }
    }
}

/// Files changed in `dir` since commit `from`, with their line counts, and the new files git
/// doesn't ignore (each counted as all added).
fn changed_since(dir: &std::path::Path, from: &str) -> Result<Vec<Changed>, String> {
    let mut files: Vec<Changed> = git(dir, &["diff", "--numstat", from])?
        .lines()
        .filter_map(|l| {
            let mut it = l.splitn(3, '\t');
            let added = it.next()?.parse().unwrap_or(0);
            let removed = it.next()?.parse().unwrap_or(0);
            Some(Changed {
                added,
                removed,
                path: it.next()?.to_string(),
                untracked: false,
            })
        })
        .collect();
    for path in git(dir, &["ls-files", "--others", "--exclude-standard"])?
        .lines()
        .filter(|l| !l.is_empty())
    {
        let added = read_head(&dir.join(path), UNTRACKED_READ_CAP)
            .map(|s| s.lines().count() as i64)
            .unwrap_or(0);
        files.push(Changed {
            path: path.to_string(),
            added,
            removed: 0,
            untracked: true,
        });
    }
    Ok(files)
}

/// Lines added and removed in `dir` and not committed yet, new files included: what the sidebar
/// shows as `+N −N`.
pub fn uncommitted_lines(dir: &std::path::Path) -> Result<(i64, i64), String> {
    let files = changed_since(dir, "HEAD")?;
    Ok(files
        .iter()
        .fold((0, 0), |(a, r), f| (a + f.added, r + f.removed)))
}

/// Everything the task changed since it branched: commits plus uncommitted and new files.
pub fn load_review(task: TaskRow) -> Result<Review, String> {
    let dir = &task.dir;
    let merge_base = git(dir, &["merge-base", "HEAD", &task.base])?
        .trim()
        .to_string();
    let files = changed_since(dir, &merge_base)?;
    let mut r = Review {
        task,
        merge_base,
        files,
        sel: 0,
        diff: Vec::new(),
        diff_sel: Some(0),
        scroll: 0,
        confirm: None,
    };
    r.diff = file_diff(&r);
    Ok(r)
}

/// The selected file's diff against where the task started.
pub fn file_diff(r: &Review) -> Vec<String> {
    diff_of(&r.diff_job())
}

pub fn diff_of(job: &DiffJob) -> Vec<String> {
    let Some(f) = &job.file else {
        return vec!["No changes yet.".into()];
    };
    if f.untracked {
        let body = read_head(&job.dir.join(&f.path), UNTRACKED_READ_CAP)
            .unwrap_or_else(|_| "(binary or unreadable)".into());
        let mut out = vec![format!("new file {}", f.path)];
        out.extend(body.lines().take(2000).map(|l| format!("+{l}")));
        return out;
    }
    match git(&job.dir, &["diff", &job.merge_base, "--", &f.path]) {
        Ok(d) => d
            .lines()
            .skip_while(|l| !l.starts_with("@@"))
            .take(4000)
            .map(|l| l.replace('\t', "    "))
            .collect(),
        Err(e) => vec![e],
    }
}

fn commit_all(t: &TaskRow) -> Result<bool, String> {
    if git(&t.dir, &["status", "--porcelain"])?.trim().is_empty() {
        return Ok(false);
    }
    git(&t.dir, &["add", "-A"])?;
    let msg = if t.summary.is_empty() {
        t.name.clone()
    } else {
        t.summary.clone()
    };
    git(&t.dir, &["commit", "-m", &msg])?;
    Ok(true)
}

pub fn commit(t: &TaskRow) -> Result<String, String> {
    Ok(if commit_all(t)? {
        format!("committed {} on {}", t.name, t.branch)
    } else {
        "nothing to commit".into()
    })
}

/// Commit what's left, then merge the task branch into the main checkout's branch.
pub fn merge(t: &TaskRow) -> Result<String, String> {
    commit_all(t)?;
    if !git(&t.root, &["status", "--porcelain"])?.trim().is_empty() {
        return Err(format!(
            "{} has uncommitted changes; commit or stash them first",
            t.root.display()
        ));
    }
    let msg = format!("Merge {}", t.branch);
    if let Err(e) = git(&t.root, &["merge", "--no-ff", &t.branch, "-m", &msg]) {
        // Leave the main checkout as it was, not mid-merge.
        let _ = git(&t.root, &["merge", "--abort"]);
        return Err(format!("couldn't merge {} (conflicts?): {e}", t.branch));
    }
    Ok(format!("merged {} into {}", t.branch, t.base))
}

/// Commit what's left, push the branch and open a pull request with `gh`.
pub fn pull_request(t: &TaskRow) -> Result<String, String> {
    commit_all(t)?;
    git(&t.dir, &["push", "-u", "origin", &t.branch])?;
    let mut cmd = Command::new("gh");
    cmd.current_dir(&t.dir)
        .args(["pr", "create", "--fill", "--head", &t.branch]);
    crate::proc::quiet(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("running gh: {e} (is the GitHub CLI installed?)"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr)
            .lines()
            .next()
            .unwrap_or("gh pr create failed")
            .to_string());
    }
    Ok(format!(
        "opened {}",
        text.lines().last().unwrap_or("the pull request")
    ))
}

/// What the Branch view's keys do with the remote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sync {
    /// Learn what the remote has, changing no branch.
    Fetch,
    /// Bring the branch up to its upstream when that is only a fast-forward.
    Pull,
    /// Send the branch, and set where it goes the first time.
    Push,
}

/// git in `dir` as a program with its arguments. It never asks for a password on the terminal
/// the window is drawn on (that would hang the key that started it); a remote that needs one
/// says so as an error.
fn remote_git(dir: &Path, args: &[&str]) -> Result<String, String> {
    crate::proc::run(
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null()),
    )
}

/// Fetch, pull or push `dir`'s checkout; answers a line for the status bar. Blocks on the
/// network: run it off the UI thread.
pub fn sync(dir: &Path, op: Sync) -> Result<String, String> {
    let branch = || {
        let b = remote_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
        let b = b.trim();
        if b.is_empty() || b == "HEAD" {
            Err("this checkout is on no branch".to_string())
        } else {
            Ok(b.to_string())
        }
    };
    match op {
        Sync::Fetch => {
            remote_git(dir, &["fetch", "--prune"])?;
            Ok("fetched".into())
        }
        Sync::Pull => {
            let before = remote_git(dir, &["rev-parse", "HEAD"])?;
            remote_git(dir, &["pull", "--ff-only"])?;
            let after = remote_git(dir, &["rev-parse", "HEAD"])?;
            Ok(if before == after {
                "already up to date".into()
            } else {
                "pulled".into()
            })
        }
        Sync::Push => {
            let branch = branch()?;
            let has_upstream = remote_git(dir, &["rev-parse", "--abbrev-ref", "@{upstream}"]).is_ok();
            if has_upstream {
                remote_git(dir, &["push"])?;
            } else {
                remote_git(dir, &["push", "-u", "origin", &branch])?;
            }
            Ok(format!("pushed {branch}"))
        }
    }
}

/// Squash-merge the branch's pull request. First as "merge when ready" (`--auto`: the checks and
/// reviews the repository requires decide when); when the repository doesn't allow that, now.
/// The branch and its worktree are left alone: the pull request being merged is what tidy sees.
pub fn merge_pull_request(t: &TaskRow) -> Result<String, String> {
    let ask = |args: &[&str]| {
        let mut cmd = Command::new("gh");
        cmd.current_dir(&t.dir)
            .args(["pr", "merge"])
            .args(args)
            .env("GH_PROMPT_DISABLED", "1")
            .stdin(Stdio::null());
        crate::proc::run(&mut cmd)
    };
    match ask(&["--squash", "--auto"]) {
        Ok(_) => Ok(format!("{} will merge when its checks pass", t.branch)),
        Err(e) if e.to_lowercase().contains("auto") => {
            ask(&["--squash"]).map(|_| format!("merged the pull request for {}", t.branch))
        }
        Err(e) if e.starts_with("couldn't run gh") => {
            Err("the GitHub CLI (gh) is needed to merge a pull request: https://cli.github.com".into())
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::brand::APP_COMMAND;

    #[test]
    fn uncommitted_lines_count_edits_and_new_files_but_not_commits() {
        let repo = std::env::temp_dir().join(format!("{APP_COMMAND}-lines-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        let git = |args: &[&str]| crate::proc::git(&repo, args).unwrap();
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(repo.join("a.txt"), "one\ntwo\nthree\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "one"]);
        assert_eq!(super::uncommitted_lines(&repo), Ok((0, 0)), "all committed");
        std::fs::write(repo.join("a.txt"), "one\n2\n").unwrap();
        std::fs::write(repo.join("new.txt"), "x\ny\n").unwrap();
        assert_eq!(
            super::uncommitted_lines(&repo),
            Ok((3, 2)),
            "one line changed and one removed, plus a new file's two"
        );
        assert!(
            super::uncommitted_lines(&repo.join("missing")).is_err(),
            "no checkout there"
        );
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn fetch_pull_and_push_go_through_a_remote_and_say_what_they_did() {
        let root = std::env::temp_dir().join(format!("{APP_COMMAND}-sync-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (remote, mine, other) = (root.join("remote.git"), root.join("mine"), root.join("other"));
        std::fs::create_dir_all(&root).unwrap();
        let git = |dir: &std::path::Path, args: &[&str]| crate::proc::git(dir, args).unwrap();
        git(&root, &["init", "-q", "--bare", "-b", "main", remote.to_str().unwrap()]);
        for dir in [&mine, &other] {
            git(&root, &["init", "-q", "-b", "main", dir.to_str().unwrap()]);
            git(dir, &["config", "user.email", "t@t"]);
            git(dir, &["config", "user.name", "t"]);
            git(dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
        }
        std::fs::write(mine.join("a.txt"), "one\n").unwrap();
        git(&mine, &["add", "."]);
        git(&mine, &["commit", "-qm", "one"]);
        // No upstream yet: the first push sets it.
        assert_eq!(super::sync(&mine, super::Sync::Push), Ok("pushed main".into()));
        git(&other, &["pull", "-q", "origin", "main"]);
        git(&other, &["branch", "-q", "--set-upstream-to=origin/main", "main"]);
        // Someone else pushes; fetching learns of it without moving the branch.
        std::fs::write(other.join("b.txt"), "two\n").unwrap();
        git(&other, &["add", "."]);
        git(&other, &["commit", "-qm", "two"]);
        git(&other, &["push", "-q"]);
        let tip = git(&mine, &["rev-parse", "HEAD"]);
        assert_eq!(super::sync(&mine, super::Sync::Fetch), Ok("fetched".into()));
        assert_eq!(git(&mine, &["rev-parse", "HEAD"]), tip);
        assert_eq!(super::sync(&mine, super::Sync::Pull), Ok("pulled".into()));
        assert!(mine.join("b.txt").is_file());
        assert_eq!(super::sync(&mine, super::Sync::Pull), Ok("already up to date".into()));
        // A pull that is not a fast-forward is refused, not merged.
        std::fs::write(mine.join("c.txt"), "mine\n").unwrap();
        git(&mine, &["add", "."]);
        git(&mine, &["commit", "-qm", "mine"]);
        std::fs::write(other.join("d.txt"), "theirs\n").unwrap();
        git(&other, &["add", "."]);
        git(&other, &["commit", "-qm", "theirs"]);
        git(&other, &["push", "-q"]);
        assert!(super::sync(&mine, super::Sync::Pull).is_err());
        // A checkout on no branch has nothing to push.
        git(&mine, &["checkout", "-q", "--detach"]);
        assert!(super::sync(&mine, super::Sync::Push).is_err_and(|e| e.contains("no branch")));
        let _ = std::fs::remove_dir_all(&root);
    }
}
