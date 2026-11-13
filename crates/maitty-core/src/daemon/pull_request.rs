//! Opening a GitHub pull request for the branch a pane's checkout is on, when a person's client
//! asks (`GitRequest::PullRequest`): the branch is pushed, then the machine's GitHub command line
//! (`gh`) opens the pull request, titled and described from the branch's commits. No token is
//! read here; signing in stays `gh`'s own. Only a pull request's address is ever answered.

use super::sync::{Git, push};
use crate::proc::{RunError, run_within};
use maitty_shared::hidden::sanitize;
use std::ffi::OsStr;

/// Longest line of what `gh` said that is passed on, in characters.
const REASON_MAX_CHARS: usize = 300;

/// Pushes the checkout's branch and opens a pull request for it. The pull request's address, or
/// one line saying why there is none. Blocks while git and `gh` run.
pub(super) fn open(git: &Git) -> Result<String, String> {
    open_with(git, OsStr::new("gh"))
}

fn open_with(git: &Git, program: &OsStr) -> Result<String, String> {
    let branch = git
        .branch()
        .ok_or("this checkout is on no branch, so there is nothing to open a pull request from")?;
    push(git)?;
    let mut command = super::github::gh(program);
    command.current_dir(git.folder()).args(["pr", "create", "--fill", "--head", &branch]);
    let ran = run_within(&mut command, git.time_left()).map_err(|error| match error {
        RunError::Missing => {
            "The branch was pushed, but opening the pull request needs the GitHub command line (gh). Get it from cli.github.com".to_owned()
        }
        RunError::Start(why) => format!("The branch was pushed, but gh could not be run: {why}"),
        RunError::TimedOut => "The branch was pushed, but gh did not answer in time".to_owned(),
    })?;
    if ran.success {
        return address_in(&ran.stdout).ok_or_else(|| "gh did not say where the pull request is".to_owned());
    }
    // Asked twice, gh refuses and names the pull request that is already open: that is the answer.
    if ran.stderr.contains("already exists")
        && let Some(address) = address_in(&ran.stderr)
    {
        return Ok(address);
    }
    let said = ran.stderr.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("gh could not open the pull request");
    Err(sanitize(said, false).chars().take(REASON_MAX_CHARS).collect())
}

/// The pull request address in what `gh` printed: its last line that is one, cut to the address.
fn address_in(printed: &str) -> Option<String> {
    printed.lines().rev().find_map(|line| super::pr::pull_request_url(line).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::sync::fixtures::{LIMIT, git, shared};
    use std::path::{Path, PathBuf};

    /// A program called `gh`, in a folder of its own, that writes what it was asked to `args.txt`
    /// beside it, prints `out` and `err`, and ends with `status`.
    fn fake_gh(root: &Path, out: &str, err: &str, status: i32) -> PathBuf {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("out.txt"), out).unwrap();
        std::fs::write(root.join("err.txt"), err).unwrap();
        if cfg!(windows) {
            let program = root.join("gh.cmd");
            let script = format!(
                "@echo off\r\n>\"%~dp0args.txt\" echo %*\r\ntype \"%~dp0out.txt\"\r\ntype \"%~dp0err.txt\" 1>&2\r\nexit /b {status}\r\n"
            );
            std::fs::write(&program, script).unwrap();
            program
        } else {
            let program = root.join("gh");
            let script = format!(
                "#!/bin/sh\nhere=\"$(dirname \"$0\")\"\necho \"$@\" > \"$here/args.txt\"\ncat \"$here/out.txt\"\ncat \"$here/err.txt\" >&2\nexit {status}\n"
            );
            std::fs::write(&program, script).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            program
        }
    }

    /// A checkout on a new branch `topic`, one commit ahead of a remote it has not been pushed to.
    fn topic(parent: &Path) -> PathBuf {
        let (mine, _other) = shared(parent);
        git(&mine, &["checkout", "-q", "-b", "topic"]);
        std::fs::write(mine.join("t.txt"), "topic\n").unwrap();
        git(&mine, &["add", "--", "t.txt"]);
        git(&mine, &["commit", "-q", "-m", "topic"]);
        mine
    }

    #[test]
    fn the_branch_is_pushed_and_the_answer_is_the_pull_requests_address() {
        let dir = crate::daemon::persist::scratch_dir();
        let mine = topic(dir.path());
        let said = "Creating pull request for topic into main in o/r\n\nhttps://github.com/o/r/pull/12\n";
        let gh = fake_gh(&dir.path().join("bin"), said, "", 0);

        let answer = open_with(&Git::new(&mine, LIMIT), gh.as_os_str());
        assert_eq!(answer, Ok("https://github.com/o/r/pull/12".into()));
        assert_eq!(git(&mine, &["rev-parse", "--abbrev-ref", "@{upstream}"]), "origin/topic");
        let asked = std::fs::read_to_string(dir.path().join("bin").join("args.txt")).unwrap();
        assert_eq!(asked.replace('"', "").trim(), "pr create --fill --head topic");
    }

    #[test]
    fn only_a_github_pull_request_address_is_answered() {
        let dir = crate::daemon::persist::scratch_dir();
        let mine = topic(dir.path());
        for (said, kept) in [
            ("https://github.com/o/r/pull/12/files?x=1\n", Some("https://github.com/o/r/pull/12")),
            ("https://evil.example/o/r/pull/12\n", None),
            ("https://github.com/o/r/issues/12\n", None),
            ("javascript:alert(1)\n", None),
            ("", None),
        ] {
            let gh = fake_gh(&dir.path().join("bin"), said, "", 0);
            let answer = open_with(&Git::new(&mine, LIMIT), gh.as_os_str());
            match kept {
                Some(address) => assert_eq!(answer, Ok(address.into()), "{said:?}"),
                None => assert_eq!(answer, Err("gh did not say where the pull request is".into()), "{said:?}"),
            }
        }
    }

    #[test]
    fn a_pull_request_that_is_already_open_is_the_answer() {
        let dir = crate::daemon::persist::scratch_dir();
        let mine = topic(dir.path());
        let said = "a pull request for branch \"topic\" into branch \"main\" already exists:\nhttps://github.com/o/r/pull/7\n";
        let gh = fake_gh(&dir.path().join("bin"), "", said, 1);
        assert_eq!(open_with(&Git::new(&mine, LIMIT), gh.as_os_str()), Ok("https://github.com/o/r/pull/7".into()));
    }

    #[test]
    fn what_gh_says_when_nobody_is_signed_in_is_passed_on_as_one_clean_line() {
        let dir = crate::daemon::persist::scratch_dir();
        let mine = topic(dir.path());
        let said = "\nTo get started with GitHub CLI, please run:  gh auth login\nAlternatively, populate the GH_TOKEN environment variable.\n";
        let gh = fake_gh(&dir.path().join("bin"), "", said, 4);
        let why = open_with(&Git::new(&mine, LIMIT), gh.as_os_str()).expect_err("nobody is signed in");
        assert_eq!(why, "To get started with GitHub CLI, please run:  gh auth login");
    }

    #[test]
    fn without_gh_the_branch_is_still_pushed_and_the_answer_says_where_to_get_it() {
        let dir = crate::daemon::persist::scratch_dir();
        let mine = topic(dir.path());
        let missing = dir.path().join("bin").join("no-such-gh");
        let why = open_with(&Git::new(&mine, LIMIT), missing.as_os_str()).expect_err("there is no gh");
        assert!(why.starts_with("The branch was pushed") && why.ends_with("Get it from cli.github.com"), "{why}");
        assert_eq!(git(&mine, &["rev-parse", "--abbrev-ref", "@{upstream}"]), "origin/topic");
    }

    #[test]
    fn a_checkout_on_no_branch_or_with_nowhere_to_push_opens_nothing() {
        let dir = crate::daemon::persist::scratch_dir();
        let mine = topic(dir.path());
        let gh = fake_gh(&dir.path().join("bin"), "https://github.com/o/r/pull/12\n", "", 0);
        git(&mine, &["remote", "remove", "origin"]);
        let why = open_with(&Git::new(&mine, LIMIT), gh.as_os_str()).expect_err("there is nowhere to push");
        assert_eq!(why, "topic has no upstream, and there is no remote named origin to push it to");
        git(&mine, &["checkout", "-q", "--detach"]);
        let why = open_with(&Git::new(&mine, LIMIT), gh.as_os_str()).expect_err("there is no branch");
        assert_eq!(why, "this checkout is on no branch, so there is nothing to open a pull request from");
        assert!(!dir.path().join("bin").join("args.txt").exists(), "gh was never asked");
    }
}
