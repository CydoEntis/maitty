//! What the worktree manager asks of a project's worktrees on this computer: where one stands
//! (read by the daemon, `maitty_desktop_client::worktree_standing`) and merging it into the main
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

/// Whether two paths name one place, though the folder itself may be gone: the folder that holds it is
/// compared as the filesystem names it (a Windows temp folder can be spelled with a short user name),
/// then its own name.
fn same_place(a: &Path, b: &Path) -> bool {
    let place = |path: &Path| {
        let parent = path.parent()?.canonicalize().ok()?;
        Some(parent.join(path.file_name()?))
    };
    a == b || place(a).is_some_and(|a| place(b).is_some_and(|b| a == b))
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
        .any(|path| same_place(Path::new(path), folder));
    if !listed {
        return Err("git does not list a worktree there".into());
    }
    // The stale record would make git refuse a second worktree at the same path.
    git(main, &["worktree", "prune"])?;
    let place = folder.to_str().ok_or_else(|| "the folder's name is not text".to_string())?;
    run_program("git", main, &["worktree", "add", "--", place, branch], MERGE_TIMEOUT)?;
    Ok(format!("Made {branch} again at {place}"))
}

/// Where the worktree at `folder` of the repository at `repo` stands, as this computer's daemon reads it.
#[tauri::command]
pub async fn worktree_standing(wire: tauri::State<'_, crate::terminal_stream::Wire>, machine: Option<u16>, repo: String, folder: String) -> Result<serde_json::Value, String> {
    wire.require_local(machine, "Worktrees")?;
    maitty_desktop_client::worktree_standing(repo.into(), folder.into())
        .await
        .map_err(|error| format!("{error:#}"))
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
    fn a_worktree_deleted_by_hand_comes_back_on_its_branch() {
        let (root, task) = repository("recreate");
        let main = root.join("main");
        assert!(recreate(&main, &task, "task").is_err_and(|error| error.contains("already there")));
        std::fs::remove_dir_all(&task).expect("deleted by hand");
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
        assert!(run(&main, &["branch", "--merged", "main"]).contains("task"), "now found on main too");
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
