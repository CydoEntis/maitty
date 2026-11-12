//! What the side panel's Changes tab does to a session's checkout on this computer: commit
//! everything that changed, and open a pull request for the branch. Both run git (and GitHub's
//! `gh`) as programs with their arguments, off the UI thread, only when the user presses the button.

use crate::new_chat::run_program;
use std::path::Path;
use std::time::Duration;

/// A commit can run the repository's own hooks (formatters, tests), so it gets longer than a question.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);
/// A push and `gh` both talk to the network.
const NETWORK_TIMEOUT: Duration = Duration::from_secs(60);
const QUICK: Duration = Duration::from_secs(5);
/// What git calls the checkout's branch when it is on no branch.
const DETACHED: &str = "HEAD";

/// Stages every change in the checkout and commits it; answers the new commit's short hash.
fn commit_all(folder: &Path, message: &str) -> Result<String, String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("write a commit message first".into());
    }
    run_program("git", folder, &["add", "-A"], QUICK)?;
    run_program("git", folder, &["commit", "-m", message], COMMIT_TIMEOUT)?;
    Ok(run_program("git", folder, &["rev-parse", "--short", "HEAD"], QUICK)?.trim().to_string())
}

/// The branch the checkout is on.
fn current_branch(folder: &Path) -> Result<String, String> {
    let branch = run_program("git", folder, &["rev-parse", "--abbrev-ref", "HEAD"], QUICK)?.trim().to_string();
    if branch.is_empty() || branch == DETACHED {
        return Err("this checkout is on no branch, so there is nothing to open a pull request from".into());
    }
    Ok(branch)
}

/// The address `gh pr create` prints: the last line that is one.
fn pull_request_url(printed: &str) -> Option<String> {
    printed.lines().rev().map(str::trim).find(|line| line.starts_with("https://")).map(str::to_string)
}

/// Pushes the checkout's branch and opens a pull request for it with `gh`, titled and described
/// from its commits; answers the pull request's address.
fn create_pull_request(folder: &Path) -> Result<String, String> {
    let branch = current_branch(folder)?;
    run_program("git", folder, &["push", "-u", "origin", &branch], NETWORK_TIMEOUT)?;
    let printed = run_program("gh", folder, &["pr", "create", "--fill", "--head", &branch], NETWORK_TIMEOUT).map_err(|error| {
        if error.starts_with("could not run gh") {
            "the branch was pushed, but GitHub's gh program is needed to open the pull request (https://cli.github.com)".to_string()
        } else {
            error
        }
    })?;
    pull_request_url(&printed).ok_or_else(|| "gh did not say where the pull request is".to_string())
}

/// What the Changes tab's remote button asks of the checkout's remote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncAction {
    /// Learn what the remote has; no branch moves.
    Fetch,
    /// Bring the branch up to its upstream when that is only a fast-forward.
    Pull,
    /// Send the branch, setting its upstream on `origin` the first time.
    Push,
}

/// Fetches, pulls (fast-forward only) or pushes the checkout; answers a line to show.
fn sync(folder: &Path, action: SyncAction) -> Result<String, String> {
    match action {
        SyncAction::Fetch => {
            run_program("git", folder, &["fetch", "--prune"], NETWORK_TIMEOUT)?;
            Ok("Fetched".into())
        }
        SyncAction::Pull => {
            let before = run_program("git", folder, &["rev-parse", "HEAD"], QUICK)?;
            run_program("git", folder, &["pull", "--ff-only"], NETWORK_TIMEOUT)?;
            let after = run_program("git", folder, &["rev-parse", "HEAD"], QUICK)?;
            Ok(if before == after { "Already up to date".into() } else { "Pulled".into() })
        }
        SyncAction::Push => {
            let branch = current_branch(folder).map_err(|_| "this checkout is on no branch, so there is nothing to push".to_string())?;
            if run_program("git", folder, &["rev-parse", "--abbrev-ref", "@{upstream}"], QUICK).is_ok() {
                run_program("git", folder, &["push"], NETWORK_TIMEOUT)?;
            } else {
                run_program("git", folder, &["push", "-u", "origin", &branch], NETWORK_TIMEOUT)?;
            }
            Ok(format!("Pushed {branch}"))
        }
    }
}

/// Fetches, pulls or pushes `folder`'s checkout.
#[tauri::command]
pub async fn git_sync(wire: tauri::State<'_, crate::terminal_stream::Wire>, machine: Option<u16>, folder: String, action: SyncAction) -> Result<String, String> {
    wire.require_local(machine, "Changes")?;
    tauri::async_runtime::spawn_blocking(move || sync(Path::new(&folder), action))
        .await
        .map_err(|error| error.to_string())?
}

/// Commits every change in `folder`'s checkout with `message`.
#[tauri::command]
pub async fn git_commit_all(wire: tauri::State<'_, crate::terminal_stream::Wire>, machine: Option<u16>, folder: String, message: String) -> Result<String, String> {
    wire.require_local(machine, "Changes")?;
    tauri::async_runtime::spawn_blocking(move || commit_all(Path::new(&folder), &message))
        .await
        .map_err(|error| error.to_string())?
}

/// Pushes `folder`'s branch and opens a pull request for it.
#[tauri::command]
pub async fn git_create_pull_request(wire: tauri::State<'_, crate::terminal_stream::Wire>, machine: Option<u16>, folder: String) -> Result<String, String> {
    wire.require_local(machine, "Changes")?;
    tauri::async_runtime::spawn_blocking(move || create_pull_request(Path::new(&folder)))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(folder: &Path, args: &[&str]) -> String {
        run_program("git", folder, args, QUICK).expect("git")
    }

    /// A repository with one commit, in a folder of its own.
    fn repository(name: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("maitty-changes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder");
        git(&folder, &["init", "-q", "-b", "main"]);
        git(&folder, &["config", "user.email", "test@example.invalid"]);
        git(&folder, &["config", "user.name", "Test"]);
        git(&folder, &["config", "commit.gpgsign", "false"]);
        std::fs::write(folder.join("a.txt"), "one\n").expect("a file");
        git(&folder, &["add", "-A"]);
        git(&folder, &["commit", "-q", "-m", "start"]);
        folder
    }

    #[test]
    fn a_commit_takes_every_change_and_answers_its_hash() {
        let folder = repository("commit");
        std::fs::write(folder.join("a.txt"), "two\n").expect("a change");
        std::fs::write(folder.join("new.txt"), "new\n").expect("a new file");
        let hash = commit_all(&folder, "  change a and add new  ").expect("the commit");
        assert_eq!(git(&folder, &["rev-parse", "--short", "HEAD"]).trim(), hash);
        assert_eq!(git(&folder, &["log", "-1", "--format=%s"]).trim(), "change a and add new");
        assert_eq!(git(&folder, &["status", "--porcelain"]).trim(), "", "nothing is left uncommitted");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_commit_needs_a_message_and_something_to_commit() {
        let folder = repository("refuse");
        assert_eq!(commit_all(&folder, "   "), Err("write a commit message first".into()));
        assert!(commit_all(&folder, "nothing changed").is_err(), "git refuses an empty commit");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_checkout_on_no_branch_has_no_pull_request_to_open() {
        let folder = repository("detached");
        assert_eq!(current_branch(&folder), Ok("main".into()));
        git(&folder, &["checkout", "-q", "--detach"]);
        assert!(create_pull_request(&folder).is_err_and(|error| error.contains("on no branch")));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_push_with_nowhere_to_go_says_what_git_said_went_wrong() {
        let folder = repository("no-remote");
        let error = create_pull_request(&folder).expect_err("there is no origin");
        assert!(error.starts_with("fatal:") && error.contains("origin"), "{error}");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_push_sets_the_upstream_and_a_pull_only_fast_forwards() {
        let mine = repository("sync-mine");
        let remote = std::env::temp_dir().join(format!("maitty-changes-sync-remote-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&remote);
        std::fs::create_dir_all(&remote).expect("a folder");
        git(&remote, &["init", "-q", "--bare", "-b", "main"]);
        git(&mine, &["remote", "add", "origin", remote.to_str().expect("a path")]);
        assert_eq!(sync(&mine, SyncAction::Push), Ok("Pushed main".into()));
        assert_eq!(git(&mine, &["rev-parse", "--abbrev-ref", "@{upstream}"]).trim(), "origin/main");
        assert_eq!(sync(&mine, SyncAction::Fetch), Ok("Fetched".into()));
        assert_eq!(sync(&mine, SyncAction::Pull), Ok("Already up to date".into()));
        // Someone else pushes; a pull brings it in.
        let other = repository("sync-other");
        git(&other, &["remote", "add", "origin", remote.to_str().expect("a path")]);
        git(&other, &["pull", "-q", "origin", "main", "--allow-unrelated-histories", "--no-edit"]);
        std::fs::write(other.join("b.txt"), "two\n").expect("a file");
        git(&other, &["add", "-A"]);
        git(&other, &["commit", "-q", "-m", "two"]);
        git(&other, &["push", "-q", "origin", "main"]);
        assert_eq!(sync(&mine, SyncAction::Pull), Ok("Pulled".into()));
        assert!(mine.join("b.txt").is_file());
        git(&mine, &["checkout", "-q", "--detach"]);
        assert!(sync(&mine, SyncAction::Push).is_err_and(|error| error.contains("on no branch")));
        for folder in [&mine, &other, &remote] {
            let _ = std::fs::remove_dir_all(folder);
        }
    }

    #[test]
    fn the_pull_request_address_is_the_last_one_gh_printed() {
        assert_eq!(
            pull_request_url("Creating pull request for a into main\n\nhttps://github.com/o/r/pull/7\n"),
            Some("https://github.com/o/r/pull/7".into())
        );
        assert_eq!(pull_request_url("nothing here"), None);
    }
}
