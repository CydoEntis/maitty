//! A send to a chat whose worktree folder was deleted makes the folder again from its branch.

use super::*;
use crate::conversation::WorktreeOrigin;
use crate::daemon::conversation::transport::LaunchIdentity;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir).args(args);
    let output = crate::proc::quiet(&mut command).output().unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

/// A repository with one commit, a worktree `task` on its own branch, and a launch of a chat
/// that runs in that worktree. The worktree's folder is then deleted, as a person would.
fn chat_in_deleted_worktree(name: &str) -> (Launch, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("maitty-recreate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let main = root.join("main");
    std::fs::create_dir_all(&main).unwrap();
    git(&main, &["init", "-q", "-b", "main"]);
    git(&main, &["config", "user.email", "test@example.invalid"]);
    git(&main, &["config", "user.name", "Test"]);
    git(&main, &["config", "commit.gpgsign", "false"]);
    std::fs::write(main.join("a.txt"), "one\n").unwrap();
    git(&main, &["add", "a.txt"]);
    git(&main, &["commit", "-q", "-m", "first"]);
    let folder = root.join("task");
    git(&main, &["worktree", "add", "-q", "-b", "task", folder.to_str().unwrap()]);
    let (mut launch, _) = claude_launch(&root.join("agent"), None);
    let LaunchIdentity::Existing(chat) = &mut launch.identity else { unreachable!() };
    // The plain spelling, as the daemon stores it: a canonicalised Windows path is `\\?\`-prefixed.
    let main = crate::daemon::clean_path(std::fs::canonicalize(&main).unwrap());
    let folder = crate::daemon::clean_path(std::fs::canonicalize(&folder).unwrap());
    chat.binding.cwd = folder.clone();
    chat.settings.as_mut().unwrap().cwd = folder.clone();
    chat.worktree_origin = Some(WorktreeOrigin { main: main.clone(), folder: folder.clone(), branch: "task".into() });
    std::fs::remove_dir_all(&folder).unwrap();
    (launch, main, folder)
}

#[tokio::test]
async fn a_missing_worktree_is_made_again_from_its_branch_before_the_send() {
    let (launch, _main, folder) = chat_in_deleted_worktree("branch-kept");
    assert!(!folder.exists());
    let launch = launch.validate().await.expect("the folder comes back");
    assert!(folder.join("a.txt").is_file());
    let LaunchIdentity::Existing(chat) = &launch.identity else { unreachable!() };
    assert_eq!(chat.binding.cwd, crate::daemon::clean_path(folder));
}

#[tokio::test]
async fn a_missing_worktree_whose_branch_is_gone_fails_naming_the_branch() {
    let (launch, main, folder) = chat_in_deleted_worktree("branch-gone");
    git(&main, &["worktree", "prune"]);
    git(&main, &["branch", "-D", "task"]);
    let error = launch.validate().await.err().expect("nothing to make it from").to_string();
    assert!(error.contains("`task`") && error.contains("no longer exists"), "{error}");
    assert!(!folder.exists());
}

#[tokio::test]
async fn a_chat_folder_missing_from_the_remade_worktree_says_so() {
    let (mut launch, _main, folder) = chat_in_deleted_worktree("subfolder-gone");
    let LaunchIdentity::Existing(chat) = &mut launch.identity else { unreachable!() };
    chat.binding.cwd = folder.join("not-in-the-branch");
    chat.settings.as_mut().unwrap().cwd = chat.binding.cwd.clone();
    let error = launch.validate().await.err().expect("the subfolder is not there").to_string();
    assert!(error.contains("is back") && error.contains("not-in-the-branch"), "{error}");
}

#[tokio::test]
async fn two_sends_to_one_deleted_folder_both_succeed() {
    let (launch, _main, folder) = chat_in_deleted_worktree("two-sends");
    let LaunchIdentity::Existing(chat) = &launch.identity else { unreachable!() };
    let twin = Launch {
        identity: LaunchIdentity::Existing(chat.clone()),
        environment: Vec::new(),
        failure_point: None,
    };
    let (a, b) = tokio::join!(twin.validate(), launch.validate());
    a.expect("first send");
    b.expect("second send");
    assert!(folder.join("a.txt").is_file());
}
