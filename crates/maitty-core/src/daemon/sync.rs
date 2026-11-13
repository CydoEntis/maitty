//! What a person's client may have done to the checkout a pane is in (`Query::Git`): fetch, pull,
//! push, commit (`commit.rs`) and open a pull request (`pull_request.rs`). Nothing here merges, rebases or forces: a pull only
//! fast-forwards. git is given no way to ask a question, so it fails when it would need a
//! password, and the whole request is stopped at its deadline. Each function blocks, so it runs
//! off the event loop.

use crate::proc::RunError;
use crate::protocol::GitRequest;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// How long `request` may take, all its git runs together. A commit can run the repository's own
/// hooks (formatters, tests), so it gets longer than talking to a remote.
fn limit_of(request: &GitRequest) -> Duration {
    Duration::from_secs(match request {
        GitRequest::Commit { .. } => crate::protocol::GIT_COMMIT_LIMIT_SECS,
        GitRequest::Fetch | GitRequest::Pull | GitRequest::Push | GitRequest::PullRequest => crate::protocol::GIT_SYNC_LIMIT_SECS,
    })
}

/// Does `request` to the checkout at `folder`, within the time that request is given.
pub(super) fn answer(folder: &Path, request: &GitRequest) -> Result<String, String> {
    sync(folder, request, limit_of(request))
}

/// Does `request` to the checkout at `folder` and says what happened in one line, or why nothing
/// did, also in one line.
fn sync(folder: &Path, request: &GitRequest, limit: Duration) -> Result<String, String> {
    let git = Git::new(folder, limit);
    match request {
        GitRequest::Fetch => {
            git.run(&["fetch", "--prune"])?;
            Ok("Fetched".into())
        }
        GitRequest::Pull => pull(&git),
        GitRequest::Push => push(&git),
        GitRequest::Commit { message, paths } => super::commit::commit(&git, message, paths.as_deref()),
        GitRequest::PullRequest => super::pull_request::open(&git),
    }
}

fn pull(git: &Git) -> Result<String, String> {
    let branch = git.branch().ok_or("this checkout is on no branch, so there is nothing to pull")?;
    if !git.has_upstream() {
        return Err(format!("{branch} has no upstream to pull from"));
    }
    let before = git.run(&["rev-parse", "HEAD"])?;
    if let Err(why) = git.run(&["pull", "--ff-only"]) {
        // The failed pull fetched first, so the two counts are of what the remote has now.
        let moved = |range: &str| git.run(&["rev-list", "--count", range]).is_ok_and(|count| count != "0");
        return Err(if moved("@{upstream}..HEAD") && moved("HEAD..@{upstream}") {
            format!("{branch} and its upstream both have new commits, so it cannot be fast-forwarded")
        } else {
            why
        });
    }
    let after = git.run(&["rev-parse", "HEAD"])?;
    Ok(if before == after { "Already up to date" } else { "Pulled" }.into())
}

/// Sends the branch to its upstream, or to `origin` under its own name when it has none yet.
pub(super) fn push(git: &Git) -> Result<String, String> {
    let branch = git.branch().ok_or("this checkout is on no branch, so there is nothing to push")?;
    if git.has_upstream() {
        git.run(&["push"])?;
    } else if git.run(&["remote", "get-url", "origin"]).is_err() {
        return Err(format!("{branch} has no upstream, and there is no remote named origin to push it to"));
    } else {
        git.run(&["push", "-u", "origin", &branch])?;
    }
    Ok(format!("Pushed {branch}"))
}

/// git in one checkout, until one deadline.
pub(super) struct Git<'a> {
    folder: &'a Path,
    deadline: Instant,
    limit: Duration,
}

impl<'a> Git<'a> {
    /// git in `folder`, for `limit` from now.
    pub(super) fn new(folder: &'a Path, limit: Duration) -> Self {
        Git { folder, deadline: Instant::now() + limit, limit }
    }

    pub(super) fn folder(&self) -> &Path {
        self.folder
    }

    /// How long is left before the deadline, for a program that is not git.
    pub(super) fn time_left(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    /// What git printed, trimmed, or the line of what it said that names the problem.
    pub(super) fn run(&self, args: &[&str]) -> Result<String, String> {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(self.folder)
            .args(args)
            // Its messages are read here and shown: keep them in English whatever the locale.
            .env("LC_ALL", "C")
            .env("GIT_TERMINAL_PROMPT", "0");
        let ran = crate::proc::run_within(&mut command, self.time_left()).map_err(|error| match error {
            RunError::Missing => "git is not installed on this machine".to_owned(),
            RunError::Start(why) => format!("couldn't run git: {why}"),
            RunError::TimedOut => format!("git was stopped after {} seconds", self.limit.as_secs()),
        })?;
        if ran.success {
            return Ok(ran.stdout.trim().to_owned());
        }
        // git says "warning: …" and "hint: …" around the line that matters.
        let line = ran
            .stderr
            .lines()
            .find(|line| line.starts_with("fatal") || line.starts_with("error") || line.contains("[rejected]"))
            .or_else(|| ran.stderr.lines().find(|line| !line.trim().is_empty()))
            // "nothing to commit" is said where answers go, on the last line.
            .or_else(|| ran.stdout.lines().rev().find(|line| !line.trim().is_empty()))
            .map_or("git could not do it", str::trim);
        Err(line.to_owned())
    }

    /// The branch the checkout is on; none when its head is detached or it is no repository.
    pub(super) fn branch(&self) -> Option<String> {
        self.run(&["symbolic-ref", "--quiet", "--short", "HEAD"]).ok().filter(|name| !name.is_empty())
    }

    fn has_upstream(&self) -> bool {
        self.run(&["rev-parse", "--abbrev-ref", "@{upstream}"]).is_ok()
    }
}

/// Repositories for the tests of what is done to a checkout.
#[cfg(test)]
pub(super) mod fixtures {
    use super::*;

    pub(in crate::daemon) const LIMIT: Duration = Duration::from_secs(60);

    pub(in crate::daemon) fn git(folder: &Path, args: &[&str]) -> String {
        Git::new(folder, LIMIT).run(args).expect("git")
    }

    /// A repository on `main` with one commit.
    pub(in crate::daemon) fn repository(parent: &Path, name: &str) -> std::path::PathBuf {
        let folder = parent.join(name);
        std::fs::create_dir_all(&folder).expect("a folder");
        git(&folder, &["init", "-q", "-b", "main"]);
        git(&folder, &["config", "user.email", "test@example.invalid"]);
        git(&folder, &["config", "user.name", "Test"]);
        git(&folder, &["config", "commit.gpgsign", "false"]);
        commit(&folder, "a.txt", "start");
        folder
    }

    pub(in crate::daemon) fn commit(folder: &Path, file: &str, subject: &str) {
        std::fs::write(folder.join(file), format!("{subject}\n")).expect("a file");
        git(folder, &["add", "--", file]);
        git(folder, &["commit", "-q", "-m", subject]);
    }

    /// `mine` and `other`, two checkouts of one bare remote that `mine` has pushed `main` to.
    pub(in crate::daemon) fn shared(parent: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let remote = parent.join("remote.git");
        std::fs::create_dir_all(&remote).expect("a folder");
        git(&remote, &["init", "-q", "--bare", "-b", "main"]);
        let address = remote.to_str().expect("a path");
        let mine = repository(parent, "mine");
        git(&mine, &["remote", "add", "origin", address]);
        assert_eq!(sync(&mine, &GitRequest::Push, LIMIT), Ok("Pushed main".into()));
        let other = parent.join("other");
        git(parent, &["clone", "-q", address, "other"]);
        git(&other, &["config", "user.email", "other@example.invalid"]);
        git(&other, &["config", "user.name", "Other"]);
        git(&other, &["config", "commit.gpgsign", "false"]);
        (mine, other)
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{LIMIT, commit, git, repository, shared};
    use super::*;

    #[test]
    fn a_first_push_sets_the_upstream_and_a_pull_brings_in_what_someone_else_pushed() {
        let dir = crate::daemon::persist::scratch_dir();
        let (mine, other) = shared(dir.path());
        assert_eq!(git(&mine, &["rev-parse", "--abbrev-ref", "@{upstream}"]), "origin/main");
        assert_eq!(sync(&mine, &GitRequest::Fetch, LIMIT), Ok("Fetched".into()));
        assert_eq!(sync(&mine, &GitRequest::Pull, LIMIT), Ok("Already up to date".into()));

        commit(&other, "b.txt", "two");
        assert_eq!(sync(&other, &GitRequest::Push, LIMIT), Ok("Pushed main".into()));
        assert_eq!(sync(&mine, &GitRequest::Pull, LIMIT), Ok("Pulled".into()));
        assert!(mine.join("b.txt").is_file());
    }

    #[test]
    fn a_branch_that_is_ahead_and_behind_is_not_pulled_and_stays_as_it_was() {
        let dir = crate::daemon::persist::scratch_dir();
        let (mine, other) = shared(dir.path());
        commit(&other, "b.txt", "theirs");
        git(&other, &["push", "-q"]);
        commit(&mine, "c.txt", "ours");
        let before = git(&mine, &["rev-parse", "HEAD"]);

        let refused = sync(&mine, &GitRequest::Pull, LIMIT);
        assert_eq!(refused, Err("main and its upstream both have new commits, so it cannot be fast-forwarded".into()));
        assert_eq!(git(&mine, &["rev-parse", "HEAD"]), before);
        assert!(!mine.join("b.txt").exists());
        assert_eq!(git(&mine, &["status", "--porcelain"]), "", "no merge was started");
    }

    #[test]
    fn a_checkout_on_no_branch_is_neither_pulled_nor_pushed() {
        let dir = crate::daemon::persist::scratch_dir();
        let (mine, _other) = shared(dir.path());
        git(&mine, &["checkout", "-q", "--detach"]);
        assert_eq!(sync(&mine, &GitRequest::Pull, LIMIT), Err("this checkout is on no branch, so there is nothing to pull".into()));
        assert_eq!(sync(&mine, &GitRequest::Push, LIMIT), Err("this checkout is on no branch, so there is nothing to push".into()));
    }

    #[test]
    fn a_branch_with_no_upstream_says_so() {
        let dir = crate::daemon::persist::scratch_dir();
        let alone = repository(dir.path(), "alone");
        assert_eq!(sync(&alone, &GitRequest::Pull, LIMIT), Err("main has no upstream to pull from".into()));
        assert_eq!(
            sync(&alone, &GitRequest::Push, LIMIT),
            Err("main has no upstream, and there is no remote named origin to push it to".into())
        );
    }

    #[test]
    fn a_folder_that_is_no_repository_answers_with_gits_own_line() {
        let dir = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).expect("a folder");
        let why = sync(dir.path(), &GitRequest::Fetch, LIMIT).expect_err("there is nothing to fetch");
        assert!(why.starts_with("fatal:") && why.contains("not a git repository"), "{why}");
        assert!(!why.contains('\n'));
    }

    #[test]
    fn a_request_out_of_time_is_stopped_and_says_so() {
        let dir = crate::daemon::persist::scratch_dir();
        let alone = repository(dir.path(), "alone");
        assert_eq!(sync(&alone, &GitRequest::Fetch, Duration::ZERO), Err("git was stopped after 0 seconds".into()));
    }
}
