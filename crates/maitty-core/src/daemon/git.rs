//! Git plumbing for workspaces: branch / dirty status, and worktree create / remove.
//! Everything here blocks; callers run it on a blocking thread.

use crate::protocol::{GitInfo, WorktreeEntry};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MAX_DIFF_FILES: usize = 100;
const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;
const MAX_UNTRACKED_FILE_BYTES: u64 = 256 * 1024;
const MAX_FILE_BROWSER_ENTRIES: usize = 500;
const MAX_FILE_BROWSER_BYTES: u64 = 256 * 1024;

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
                "--src-prefix=a/",
                "--dst-prefix=b/",
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
                "--src-prefix=a/",
                "--dst-prefix=b/",
                "--unified=3",
                "--cached",
                "--",
            ],
        )?;
        let unstaged = git(
            dir,
            &[
                "diff",
                "--no-ext-diff",
                "--no-color",
                "--src-prefix=a/",
                "--dst-prefix=b/",
                "--unified=3",
                "--",
            ],
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

pub fn conversation_files(
    root: &Path,
    relative: Option<&str>,
) -> Result<Vec<crate::conversation::ConversationFileEntry>> {
    let directory = resolve_conversation_path(root, relative.unwrap_or(""))?;
    if !directory.is_dir() {
        bail!("The selected project path is not a directory");
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory).context("reading project directory")? {
        let entry = entry.context("reading project directory entry")?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name == ".git" {
            continue;
        }
        let metadata = std::fs::symlink_metadata(entry.path()).context("reading project entry")?;
        if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
            continue;
        }
        entries.push(crate::conversation::ConversationFileEntry {
            name: name.to_string(),
            is_directory: metadata.is_dir(),
        });
        if entries.len() > MAX_FILE_BROWSER_ENTRIES {
            bail!("This folder has too many entries to show safely");
        }
    }
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(entries)
}

pub fn conversation_file(root: &Path, relative: &str) -> Result<String> {
    let path = resolve_conversation_path(root, relative)?
        .canonicalize()
        .context("resolving project file")?;
    let metadata = std::fs::metadata(&path).context("reading project file metadata")?;
    if !metadata.is_file() {
        bail!("The selected project path is not a file");
    }
    if metadata.len() > MAX_FILE_BROWSER_BYTES {
        bail!("This file is too large to preview");
    }
    let content = std::fs::read(path).context("reading project file")?;
    String::from_utf8(content).context("The selected file is not UTF-8 text")
}

fn resolve_conversation_path(root: &Path, relative: &str) -> Result<PathBuf> {
    use std::path::Component;

    let root = root
        .canonicalize()
        .context("opening the conversation project")?;
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("The project path is invalid");
    }

    let mut path = root.clone();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            unreachable!()
        };
        path.push(name);
        let metadata = std::fs::symlink_metadata(&path).context("reading project path")?;
        if metadata.file_type().is_symlink() {
            bail!("Symbolic links cannot be opened in the file browser");
        }
    }
    if !path.starts_with(&root) {
        bail!("The project path is outside the conversation checkout");
    }
    Ok(path)
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
            "--src-prefix=a/",
            "--dst-prefix=b/",
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
        // Many users set diff.mnemonicPrefix (c/ and w/ instead of a/ and b/); the diff must not depend on it.
        git_at(
            repository.path(),
            &["config", "diff.mnemonicPrefix", "true"],
        );
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

    #[test]
    fn conversation_file_browser_lists_and_reads_only_relative_project_files() {
        let project = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(project.path().join("src")).expect("project directories");
        std::fs::write(project.path().join("README.md"), "project notes\n").expect("project file");
        std::fs::write(project.path().join("src/lib.rs"), "pub fn ready() {}\n")
            .expect("source file");
        std::fs::create_dir(project.path().join(".git")).expect("git metadata directory");

        let entries = conversation_files(project.path(), None).expect("project entries");
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["src", "README.md"]
        );
        assert_eq!(
            conversation_file(project.path(), "src/lib.rs").expect("file contents"),
            "pub fn ready() {}\n"
        );
        assert!(conversation_file(project.path(), "../outside.txt").is_err());
        assert!(conversation_file(project.path(), "").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn conversation_file_browser_refuses_symbolic_links() {
        let project = crate::daemon::persist::scratch_dir();
        let outside = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(project.path()).expect("project directory");
        std::fs::create_dir_all(outside.path()).expect("outside directory");
        std::fs::write(outside.path().join("secret.txt"), "private\n").expect("outside file");
        std::os::unix::fs::symlink(outside.path(), project.path().join("linked"))
            .expect("symbolic link");

        assert!(
            conversation_files(project.path(), None)
                .expect("entries")
                .is_empty()
        );
        assert!(conversation_file(project.path(), "linked/secret.txt").is_err());
    }
}
