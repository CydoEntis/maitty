//! What the worktree manager asks of a project's worktrees on this computer: where one stands
//! (changes not committed, whether its work is also elsewhere) and merging it into the main
//! checkout's branch. Removing one is the daemon's job (`maitty_desktop_client::tidy_worktrees`),
//! which also stops what runs in it and keeps its books. Everything here is git run as a program
//! with arguments, off the UI thread, only when a button asks.

use crate::new_chat::run_program;
use std::path::Path;
use std::time::Duration;

const QUICK: Duration = Duration::from_secs(5);
/// A merge can run the repository's hooks.
const MERGE_TIMEOUT: Duration = Duration::from_secs(120);
/// What git calls the checkout's branch when it is on no branch.
const DETACHED: &str = "HEAD";

/// Where a worktree stands: what removing it would lose.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Standing {
    /// Changed and new files that are not committed.
    pub dirty: usize,
    /// Its branch; none for a detached HEAD.
    pub branch: Option<String>,
    /// Every commit it has is also on another branch or on a remote: its branch could go and nothing would be lost.
    pub merged: bool,
    /// When it last committed, as git says it ("3 days ago").
    pub last_commit: String,
    /// Git still lists it but its folder is gone (deleted by hand): it can be made again from its branch.
    pub missing: bool,
}

fn git(folder: &Path, args: &[&str]) -> Result<String, String> {
    run_program("git", folder, args, QUICK)
}

fn branch_of(folder: &Path) -> Option<String> {
    let branch = git(folder, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()?.trim().to_string();
    (!branch.is_empty() && branch != DETACHED).then_some(branch)
}

fn dirty_files(folder: &Path) -> Result<usize, String> {
    Ok(git(folder, &["status", "--porcelain"])?.lines().filter(|line| !line.trim().is_empty()).count())
}

fn standing(folder: &Path) -> Result<Standing, String> {
    if !folder.is_dir() {
        return Ok(Standing { dirty: 0, branch: None, merged: false, last_commit: String::new(), missing: true });
    }
    let dirty = dirty_files(folder)?;
    let branch = branch_of(folder);
    let own = branch.as_deref().unwrap_or("");
    // Every branch that has this checkout's commit, other than its own and that branch on a remote.
    let holding = git(folder, &["for-each-ref", "--contains", "HEAD", "--format=%(refname)", "refs/heads", "refs/remotes"]).unwrap_or_default();
    let merged = holding.lines().map(str::trim).filter(|name| !name.is_empty() && !name.ends_with("/HEAD")).any(|name| {
        own.is_empty() || (name != format!("refs/heads/{own}") && !(name.starts_with("refs/remotes/") && name.ends_with(&format!("/{own}"))))
    });
    let last_commit = git(folder, &["log", "-1", "--format=%cr"]).unwrap_or_default().trim().to_string();
    Ok(Standing { dirty, branch, merged, last_commit, missing: false })
}

/// A linked worktree has its own git directory, apart from the one it shares; the main checkout's are the same.
fn is_linked(folder: &Path) -> Result<bool, String> {
    let own = git(folder, &["rev-parse", "--path-format=absolute", "--git-dir"])?;
    let shared = git(folder, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
    Ok(own.trim() != shared.trim())
}

/// The main checkout's folder: the first worktree git lists.
fn main_checkout(folder: &Path) -> Result<std::path::PathBuf, String> {
    git(folder, &["worktree", "list", "--porcelain"])?
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "git did not say where the main checkout is".to_string())
}

/// Merges the worktree's branch into the branch the main checkout has out, leaving the main
/// checkout as it was when that fails. Nothing is committed on the person's behalf: a worktree
/// or main checkout with changes is refused, so the merge only ever takes committed work.
fn merge_into_base(folder: &Path) -> Result<String, String> {
    if !is_linked(folder)? {
        return Err("this is the main checkout, so there is nothing to merge it into".into());
    }
    let branch = branch_of(folder).ok_or_else(|| "this worktree is on no branch, so there is nothing to merge".to_string())?;
    if dirty_files(folder)? > 0 {
        return Err("this worktree has uncommitted changes; commit them first".into());
    }
    let main = main_checkout(folder)?;
    let base = branch_of(&main).ok_or_else(|| "the main checkout is on no branch, so there is nothing to merge into".to_string())?;
    if dirty_files(&main)? > 0 {
        return Err(format!("the main checkout has uncommitted changes; commit or stash them first ({})", main.display()));
    }
    let message = format!("Merge {branch}");
    if let Err(error) = run_program("git", &main, &["merge", "--no-ff", "-m", &message, &branch], MERGE_TIMEOUT) {
        // Leave the main checkout as it was, not half merged.
        let _ = git(&main, &["merge", "--abort"]);
        return Err(format!("could not merge {branch} into {base} (conflicts?): {error}"));
    }
    Ok(format!("Merged {branch} into {base}"))
}

/// Makes a worktree whose folder was deleted by hand again, at the same place on the same branch. Only
/// a folder git itself lists for this repository is made, so this cannot put a checkout anywhere new.
fn recreate(main: &Path, folder: &Path, branch: &str) -> Result<String, String> {
    if folder.exists() {
        return Err("that folder is already there".into());
    }
    if branch.is_empty() || branch.starts_with('-') {
        return Err(format!("`{branch}` is not a branch name"));
    }
    let listed = git(main, &["worktree", "list", "--porcelain"])?
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .any(|path| Path::new(path) == folder);
    if !listed {
        return Err("git does not list a worktree there".into());
    }
    // The stale record would make git refuse a second worktree at the same path.
    git(main, &["worktree", "prune"])?;
    let place = folder.to_str().ok_or_else(|| "the folder's name is not text".to_string())?;
    run_program("git", main, &["worktree", "add", "--", place, branch], MERGE_TIMEOUT)?;
    Ok(format!("Made {branch} again at {place}"))
}

/// Where the worktree at `folder` stands.
#[tauri::command]
pub async fn worktree_standing(wire: tauri::State<'_, crate::terminal_stream::Wire>, machine: Option<u16>, folder: String) -> Result<Standing, String> {
    wire.require_local(machine, "Worktrees")?;
    tauri::async_runtime::spawn_blocking(move || standing(Path::new(&folder)))
        .await
        .map_err(|error| error.to_string())?
}

/// Merges the worktree at `folder` into the main checkout's branch.
#[tauri::command]
pub async fn worktree_merge(wire: tauri::State<'_, crate::terminal_stream::Wire>, machine: Option<u16>, folder: String) -> Result<String, String> {
    wire.require_local(machine, "Worktrees")?;
    tauri::async_runtime::spawn_blocking(move || merge_into_base(Path::new(&folder)))
        .await
        .map_err(|error| error.to_string())?
}

/// Makes the missing worktree at `folder` again on `branch`, from the repository at `main`.
#[tauri::command]
pub async fn worktree_recreate(wire: tauri::State<'_, crate::terminal_stream::Wire>, machine: Option<u16>, main: String, folder: String, branch: String) -> Result<String, String> {
    wire.require_local(machine, "Worktrees")?;
    tauri::async_runtime::spawn_blocking(move || recreate(Path::new(&main), Path::new(&folder), &branch))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(folder: &Path, args: &[&str]) -> String {
        git(folder, args).expect("git")
    }

    /// A repository on `main` with one commit, and a linked worktree `task` beside it.
    fn repository(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("maitty-worktrees-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let main = root.join("main");
        std::fs::create_dir_all(&main).expect("a folder");
        run(&main, &["init", "-q", "-b", "main"]);
        run(&main, &["config", "user.email", "test@example.invalid"]);
        run(&main, &["config", "user.name", "Test"]);
        run(&main, &["config", "commit.gpgsign", "false"]);
        std::fs::write(main.join("a.txt"), "one\n").expect("a file");
        run(&main, &["add", "-A"]);
        run(&main, &["commit", "-q", "-m", "start"]);
        let task = root.join("task");
        run(&main, &["worktree", "add", "-q", "-b", "task", task.to_str().expect("a path")]);
        (root, task)
    }

    #[test]
    fn a_worktree_is_merged_only_when_its_work_is_found_elsewhere() {
        let (root, task) = repository("standing");
        let fresh = standing(&task).expect("standing");
        assert_eq!((fresh.dirty, fresh.branch.as_deref(), fresh.merged), (0, Some("task"), true), "just cut from main");
        std::fs::write(task.join("a.txt"), "two\n").expect("an edit");
        std::fs::write(task.join("new.txt"), "x\n").expect("a new file");
        assert_eq!(standing(&task).expect("standing").dirty, 2);
        run(&task, &["add", "-A"]);
        run(&task, &["commit", "-q", "-m", "work"]);
        let own = standing(&task).expect("standing");
        assert!(own.dirty == 0 && !own.merged, "committed only on its own branch");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_worktree_deleted_by_hand_reads_as_missing_and_comes_back_on_its_branch() {
        let (root, task) = repository("recreate");
        let main = root.join("main");
        assert!(recreate(&main, &task, "task").is_err_and(|error| error.contains("already there")));
        std::fs::remove_dir_all(&task).expect("deleted by hand");
        assert!(standing(&task).expect("standing").missing);
        assert!(recreate(&main, &root.join("elsewhere"), "task").is_err_and(|error| error.contains("does not list")), "only a listed place");
        assert!(recreate(&main, &task, "-x").is_err());
        recreate(&main, &task, "task").expect("made again");
        assert!(task.join("a.txt").is_file());
        assert_eq!(branch_of(&task).as_deref(), Some("task"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_merge_takes_committed_work_into_the_base_and_refuses_anything_unsaved() {
        let (root, task) = repository("merge");
        let main = root.join("main");
        assert!(merge_into_base(&main).is_err_and(|error| error.contains("main checkout")));
        std::fs::write(task.join("b.txt"), "b\n").expect("a file");
        assert!(merge_into_base(&task).is_err_and(|error| error.contains("uncommitted")), "nothing is committed for the person");
        run(&task, &["add", "-A"]);
        run(&task, &["commit", "-q", "-m", "work"]);
        std::fs::write(main.join("a.txt"), "dirty\n").expect("an edit");
        assert!(merge_into_base(&task).is_err_and(|error| error.contains("main checkout has uncommitted")));
        run(&main, &["checkout", "--", "a.txt"]);
        assert_eq!(merge_into_base(&task), Ok("Merged task into main".into()));
        assert!(main.join("b.txt").is_file());
        assert!(standing(&task).expect("standing").merged, "now found on main too");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_conflicting_merge_leaves_the_main_checkout_as_it_was() {
        let (root, task) = repository("conflict");
        let main = root.join("main");
        std::fs::write(task.join("a.txt"), "task\n").expect("an edit");
        run(&task, &["commit", "-q", "-a", "-m", "task edit"]);
        std::fs::write(main.join("a.txt"), "main\n").expect("an edit");
        run(&main, &["commit", "-q", "-a", "-m", "main edit"]);
        let error = merge_into_base(&task).expect_err("they conflict");
        assert!(error.contains("could not merge task into main"), "{error}");
        assert_eq!(run(&main, &["status", "--porcelain"]).trim(), "", "no merge left half done");
        let _ = std::fs::remove_dir_all(&root);
    }
}
