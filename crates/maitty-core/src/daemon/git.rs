//! Git plumbing for workspaces: branch / dirty status, and worktree create / remove.
//! Everything here blocks; callers run it on a blocking thread.

use crate::protocol::{GitInfo, WorktreeEntry};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MAX_DIFF_FILES: usize = 100;
const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;
const MAX_UNTRACKED_FILE_BYTES: u64 = 256 * 1024;

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    // Its messages are matched below: keep them in English whatever the user's locale.
    cmd.arg("-C").arg(dir).args(args).env("LC_ALL", "C");
    crate::proc::quiet(&mut cmd);
    let out = cmd.output().context("running git")?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn abs(dir: &Path, p: &str) -> PathBuf {
    let p = PathBuf::from(p);
    if p.is_absolute() { p } else { dir.join(p) }
}

pub fn status(dir: &Path) -> Option<GitInfo> {
    let out = git(dir, &["status", "--porcelain=v2", "--branch"]).ok()?;
    let mut branch = String::new();
    let mut dirty = 0;
    for line in out.lines() {
        if let Some(b) = line.strip_prefix("# branch.head ") {
            branch = b.to_string();
        } else if !line.starts_with('#') && !line.is_empty() {
            dirty += 1;
        }
    }
    if branch == "(detached)" {
        branch = git(dir, &["rev-parse", "--short", "HEAD"]).unwrap_or(branch);
    }
    let dirs = git(dir, &["rev-parse", "--git-dir", "--git-common-dir"]).ok()?;
    let mut it = dirs.lines();
    let git_dir = abs(dir, it.next()?);
    let common = abs(dir, it.next()?);
    let linked = git_dir.canonicalize().ok() != common.canonicalize().ok();
    let root = repo_root_of_common(&common).unwrap_or_else(|| dir.to_path_buf());
    let root = crate::daemon::clean_path(root);
    let repo = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Some(GitInfo {
        repo,
        branch,
        dirty,
        linked,
        root,
        worktrees: Vec::new(),
        ahead: 0,
    })
}

/// The main checkout's root: the parent of the common `.git` directory.
fn repo_root_of_common(common: &Path) -> Option<PathBuf> {
    let c = common.canonicalize().ok()?;
    if c.file_name().is_some_and(|n| n == ".git") {
        c.parent().map(Path::to_path_buf)
    } else {
        Some(c)
    }
}

/// Every worktree of the repository `dir` belongs to, main checkout first.
pub fn list_worktrees(dir: &Path) -> Result<Vec<WorktreeEntry>> {
    let common = abs(
        dir,
        &git(dir, &["rev-parse", "--git-common-dir"]).context("not a git repository")?,
    );
    let root = repo_root_of_common(&common).context("finding the repository root")?;
    let repo = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let out = git(&root, &["worktree", "list", "--porcelain"])?;
    let mut list = Vec::new();
    for block in out.split("\n\n") {
        let mut path = None;
        let mut branch = String::new();
        let mut head = String::new();
        let mut bare = false;
        for line in block.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                path = Some(PathBuf::from(p.replace('/', std::path::MAIN_SEPARATOR_STR)));
            } else if let Some(b) = line.strip_prefix("branch ") {
                branch = b.trim_start_matches("refs/heads/").to_string();
            } else if let Some(h) = line.strip_prefix("HEAD ") {
                head = h.chars().take(7).collect();
            } else if line == "bare" {
                bare = true;
            }
        }
        let Some(path) = path else { continue };
        if bare {
            continue;
        }
        if branch.is_empty() {
            branch = format!("({head})");
        }
        let main = path.canonicalize().ok() == root.canonicalize().ok();
        let path = crate::daemon::clean_path(path);
        list.push(WorktreeEntry {
            path,
            branch,
            main,
            repo: repo.clone(),
        });
    }
    list.sort_by_key(|w| !w.main);
    Ok(list)
}

fn sanitize_branch(b: &str) -> String {
    b.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Create a worktree for `branch` (new from `base`/HEAD, or the existing local / remote
/// branch). Returns (path, repo name).
pub fn create_worktree(
    dir: &Path,
    branch: &str,
    base: Option<&str>,
    template: &str,
) -> Result<(PathBuf, String)> {
    let branch = branch.trim();
    if branch.is_empty() || branch.starts_with('-') || branch.contains(char::is_whitespace) {
        bail!("invalid branch name `{branch}`");
    }
    let common = abs(
        dir,
        &git(dir, &["rev-parse", "--git-common-dir"]).context("not a git repository")?,
    );
    let root = repo_root_of_common(&common).context("finding the repository root")?;
    let repo = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".into());
    let parent = root.parent().unwrap_or(&root);
    let path = PathBuf::from(
        template
            .replace("{repo_parent}", &parent.to_string_lossy())
            .replace("{repo}", &repo)
            .replace("{branch}", &sanitize_branch(branch))
            .replace('/', std::path::MAIN_SEPARATOR_STR),
    );
    let path = crate::daemon::clean_path(path);
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let p = path.to_string_lossy().into_owned();
    let has = |r: &str| git(&root, &["show-ref", "--verify", "--quiet", r]).is_ok();
    if has(&format!("refs/heads/{branch}"))
        || (base.is_none() && has(&format!("refs/remotes/origin/{branch}")))
    {
        // Existing branch (git sets up tracking for a remote-only one).
        git(&root, &["worktree", "add", &p, branch])?;
    } else {
        let mut args = vec!["worktree", "add", "-b", branch, &p];
        if let Some(b) = base {
            // A base that looks like an option would be read as one.
            if b.starts_with('-') {
                bail!("not a branch to start from: {b}");
            }
            args.push(b);
        }
        git(&root, &args)?;
    }
    Ok((path, repo))
}

/// The repository's local branches.
pub fn branches(dir: &Path) -> Vec<String> {
    git(
        dir,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
    .map(|s| s.lines().map(str::to_string).collect())
    .unwrap_or_default()
}

/// Commits on HEAD of `dir` that `base` doesn't have.
pub fn ahead_of(dir: &Path, base: &str) -> u32 {
    git(dir, &["rev-list", "--count", &format!("{base}..HEAD")])
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Read the current working-tree diff for a conversation's checkout.
pub fn conversation_diff(dir: &Path) -> Result<crate::conversation::ConversationDiff> {
    git(dir, &["rev-parse", "--show-toplevel"]).context("opening the project Git repository")?;

    let mut patch = if git(dir, &["rev-parse", "--verify", "HEAD"]).is_ok() {
        git(
            dir,
            &[
                "diff",
                "--no-ext-diff",
                "--no-color",
                "--unified=3",
                "HEAD",
                "--",
            ],
        )?
    } else {
        let staged = git(
            dir,
            &[
                "diff",
                "--no-ext-diff",
                "--no-color",
                "--unified=3",
                "--cached",
                "--",
            ],
        )?;
        let unstaged = git(
            dir,
            &["diff", "--no-ext-diff", "--no-color", "--unified=3", "--"],
        )?;
        format!("{staged}{unstaged}")
    };

    let mut omitted_files = 0;
    let untracked = git(dir, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    for path in untracked.split('\0').filter(|path| !path.is_empty()) {
        let file = dir.join(path);
        let metadata = std::fs::symlink_metadata(&file)
            .with_context(|| format!("reading untracked file metadata for {path}"))?;
        if metadata.file_type().is_symlink() {
            omitted_files += 1;
            continue;
        }
        if metadata.len() > MAX_UNTRACKED_FILE_BYTES {
            omitted_files += 1;
            continue;
        }
        let output = run_untracked_diff(dir, path)
            .with_context(|| format!("reading untracked file diff for {path}"))?;
        if !patch.is_empty() && !patch.ends_with('\n') {
            patch.push('\n');
        }
        patch.push_str(&String::from_utf8_lossy(&output));
    }

    let mut files = parse_diff_files(&patch);
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let total_files = files.len() + omitted_files;
    let mut bytes = 0;
    let mut additions = 0;
    let mut deletions = 0;
    let mut shown = Vec::new();
    let mut truncated = omitted_files > 0;
    for file in files {
        let size = file.patch.len();
        if shown.len() == MAX_DIFF_FILES || bytes + size > MAX_DIFF_BYTES {
            truncated = true;
            continue;
        }
        bytes += size;
        additions += file.additions;
        deletions += file.deletions;
        shown.push(file);
    }

    Ok(crate::conversation::ConversationDiff {
        files: shown,
        additions,
        deletions,
        total_files,
        truncated,
    })
}

fn run_untracked_diff(dir: &Path, path: &str) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args([
            "diff",
            "--no-index",
            "--no-ext-diff",
            "--no-color",
            "--unified=3",
            "--",
            "/dev/null",
            path,
        ])
        .env("LC_ALL", "C");
    crate::proc::quiet(&mut command);
    let Output {
        status,
        stdout,
        stderr,
    } = command
        .output()
        .context("running git diff for an untracked file")?;
    if !status.success() && status.code() != Some(1) {
        bail!(
            "git diff for untracked file failed: {}",
            String::from_utf8_lossy(&stderr).trim()
        );
    }
    Ok(stdout)
}

fn parse_diff_files(diff: &str) -> Vec<crate::conversation::ConversationDiffFile> {
    let mut files = Vec::new();
    let mut path = None;
    let mut patch = String::new();
    for line in diff.split_inclusive('\n') {
        if let Some(header) = line.strip_prefix("diff --git ") {
            if let Some(file) = finish_diff_file(path.take(), std::mem::take(&mut patch)) {
                files.push(file);
            }
            path = header
                .split_once(" b/")
                .map(|(_, path)| path.trim_end_matches('\n').to_owned());
        } else if let Some(header) = line.strip_prefix("diff --cc ") {
            if let Some(file) = finish_diff_file(path.take(), std::mem::take(&mut patch)) {
                files.push(file);
            }
            path = Some(header.trim_end_matches('\n').to_owned());
        }
        patch.push_str(line);
    }
    if let Some(file) = finish_diff_file(path, patch) {
        files.push(file);
    }
    files
}

fn finish_diff_file(
    path: Option<String>,
    patch: String,
) -> Option<crate::conversation::ConversationDiffFile> {
    let path = path?;
    if patch.is_empty() {
        return None;
    }
    let additions = patch
        .lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .count();
    let deletions = patch
        .lines()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .count();
    let binary = patch.lines().any(|line| line.starts_with("Binary files "));
    Some(crate::conversation::ConversationDiffFile {
        path,
        additions,
        deletions,
        patch,
        binary,
    })
}

/// The branch a checkout is on (None when detached).
pub fn branch_of(dir: &Path) -> Option<String> {
    git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .map(|b| b.trim().to_string())
        .filter(|b| b != "HEAD")
}

/// Delete `branch` if it's merged (`git branch -d` refuses otherwise). Whether it went.
pub fn delete_if_merged(repo: &Path, branch: &str) -> bool {
    !branch.starts_with('-') && git(repo, &["branch", "-d", branch]).is_ok()
}

/// Remove a linked worktree (and optionally its branch). Retries briefly: on Windows the
/// panes' processes may still hold the directory for a moment after being killed.
pub fn remove_worktree(path: &Path, force: bool, delete_branch: bool) -> Result<()> {
    let common = abs(path, &git(path, &["rev-parse", "--git-common-dir"])?);
    let root = repo_root_of_common(&common).context("finding the repository root")?;
    let branch = git(path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|b| b != "HEAD");
    if root.canonicalize().ok() == path.canonicalize().ok() {
        bail!("refusing to remove the main checkout");
    }
    let p = path.to_string_lossy().into_owned();
    let mut args = vec!["worktree", "remove", &p];
    if force {
        args.push("--force");
    }
    let mut last = None;
    for _ in 0..10 {
        match git(&root, &args) {
            Ok(_) => {
                // Tidy the `<repo>-worktrees` folder once its last worktree is gone.
                if let Some(parent) = path.parent() {
                    let _ = std::fs::remove_dir(parent);
                }
                if delete_branch && let Some(b) = branch {
                    git(&root, &["branch", "-D", &b])?;
                }
                return Ok(());
            }
            Err(e) if e.to_string().contains("modified or untracked") => return Err(e),
            Err(e) => last = Some(e),
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("git worktree remove failed")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_at(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn conversation_diff_includes_tracked_and_untracked_changes_but_not_ignored_files() {
        let repository = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(repository.path()).expect("repository directory");
        git_at(repository.path(), &["init", "--quiet"]);
        git_at(repository.path(), &["config", "user.name", "Test User"]);
        git_at(
            repository.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        std::fs::write(repository.path().join(".gitignore"), "ignored.txt\n").expect("ignore file");
        std::fs::write(repository.path().join("tracked.txt"), "before\nsame\n")
            .expect("tracked file");
        git_at(repository.path(), &["add", "--all"]);
        git_at(repository.path(), &["commit", "--quiet", "-m", "baseline"]);

        std::fs::write(repository.path().join("tracked.txt"), "after\nsame\n")
            .expect("changed tracked file");
        std::fs::write(repository.path().join("new.txt"), "untracked\n").expect("untracked file");
        std::fs::write(repository.path().join("ignored.txt"), "ignored\n").expect("ignored file");

        let diff = conversation_diff(repository.path()).expect("working tree diff");
        assert_eq!(diff.total_files, 2);
        assert_eq!((diff.additions, diff.deletions), (2, 1));
        assert!(!diff.truncated);
        assert_eq!(
            diff.files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            ["new.txt", "tracked.txt"]
        );
    }
}
