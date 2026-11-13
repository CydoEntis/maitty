//! Committing changed files of the checkout a pane is in, when a person's client asks
//! (`GitRequest::Commit`). The message and the paths come from a client, so both are checked
//! before git sees them, and git reads every path literally. Nothing is amended, no hook is
//! skipped and signing is left as the repository has it.

use super::sync::Git;
use crate::protocol::{GIT_COMMIT_MESSAGE_MAX_BYTES, GIT_COMMIT_PATHS_MAX_BYTES};
use std::path::{Component, Path};

/// Commits `paths`, or everything that changed when there are none, with `message`. The new
/// commit's short id and subject, or one line saying why nothing was committed. Whatever else
/// is staged stays staged and out of the commit when paths are named.
pub(super) fn commit(git: &Git, message: &str, paths: Option<&[String]>) -> Result<String, String> {
    let message = checked_message(message)?;
    match paths {
        None => {
            git.run(&["add", "-A"])?;
            git.run(&["commit", "--quiet", "-m", message])?;
        }
        Some([]) => return Err("pick at least one file to commit".into()),
        Some(paths) => {
            let named = checked_paths(paths)?;
            // A path is a file's name here, never a pattern or one of git's `:(…)` forms.
            let mut add = vec!["--literal-pathspecs", "add", "-A", "--"];
            add.extend(&named);
            git.run(&add)?;
            // `--only` is what a path on `git commit` means; saying it keeps other staged files out.
            let mut commit = vec!["--literal-pathspecs", "commit", "--quiet", "--only", "-m", message, "--"];
            commit.extend(&named);
            git.run(&commit)?;
        }
    }
    git.run(&["log", "-1", "--format=%h %s"])
}

fn checked_message(message: &str) -> Result<&str, String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("write a commit message first".into());
    }
    if message.len() > GIT_COMMIT_MESSAGE_MAX_BYTES {
        return Err("that commit message is too long".into());
    }
    if message.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')) {
        return Err("that commit message has a character that cannot be in one".into());
    }
    Ok(message)
}

fn checked_paths(paths: &[String]) -> Result<Vec<&str>, String> {
    // They go to git as arguments, and a command line only holds so much.
    if paths.iter().map(|path| path.len() + 1).sum::<usize>() > GIT_COMMIT_PATHS_MAX_BYTES {
        return Err("that is too many files to name at once: commit them all, or fewer at a time".into());
    }
    paths.iter().map(|path| checked_path(path)).collect()
}

/// A path the commit may name: a file inside the checkout, never an option, an escape from it,
/// or anything of git's own.
fn checked_path(path: &str) -> Result<&str, String> {
    let inside = Path::new(path).components().all(|part| match part {
        Component::Normal(name) => !name.eq_ignore_ascii_case(".git"),
        _ => false,
    });
    if path.is_empty() || path.starts_with('-') || path.chars().any(char::is_control) || !inside {
        // The path may be what is wrong with it, so only its printable part is said back.
        let shown: String = path.chars().filter(|c| !c.is_control()).take(120).collect();
        return Err(format!("`{shown}` is not a file of this checkout"));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const LIMIT: Duration = Duration::from_secs(60);

    /// A repository on `main` with `a.txt` committed.
    fn repository(folder: &Path) -> Git<'_> {
        std::fs::create_dir_all(folder).expect("a folder");
        let git = Git::new(folder, LIMIT);
        for args in [
            &["init", "-q", "-b", "main"][..],
            &["config", "user.email", "test@example.invalid"],
            &["config", "user.name", "Test"],
            &["config", "commit.gpgsign", "false"],
        ] {
            git.run(args).expect("git");
        }
        std::fs::write(folder.join("a.txt"), "one\n").expect("a file");
        git.run(&["add", "--", "a.txt"]).expect("git");
        git.run(&["commit", "-q", "-m", "start"]).expect("git");
        git
    }

    fn committed(git: &Git) -> String {
        git.run(&["show", "--name-only", "--format=", "HEAD"]).expect("git")
    }

    #[test]
    fn everything_that_changed_is_committed_and_the_answer_is_the_short_id_and_subject() {
        let dir = crate::daemon::persist::scratch_dir();
        let git = repository(dir.path());
        std::fs::write(dir.path().join("a.txt"), "two\n").expect("a file");
        std::fs::write(dir.path().join("new.txt"), "new\n").expect("a file");

        let answer = commit(&git, "  change a and add new\n\nwith a body  ", None).expect("the commit");
        let short = git.run(&["rev-parse", "--short", "HEAD"]).expect("git");
        assert_eq!(answer, format!("{short} change a and add new"));
        assert_eq!(committed(&git), "a.txt\nnew.txt");
        assert_eq!(git.run(&["status", "--porcelain"]).expect("git"), "");
    }

    #[test]
    fn only_the_named_files_go_in_and_what_someone_else_staged_stays_staged() {
        let dir = crate::daemon::persist::scratch_dir();
        let git = repository(dir.path());
        std::fs::write(dir.path().join("a.txt"), "two\n").expect("a file");
        std::fs::write(dir.path().join("theirs.txt"), "staged by someone else\n").expect("a file");
        git.run(&["add", "--", "theirs.txt"]).expect("git");
        std::fs::write(dir.path().join("new.txt"), "new\n").expect("a file");

        commit(&git, "only the new one", Some(&["new.txt".into()])).expect("the commit");
        assert_eq!(committed(&git), "new.txt");
        assert_eq!(git.run(&["status", "--porcelain"]).expect("git"), "M a.txt\nA  theirs.txt");
    }

    #[test]
    fn a_deleted_file_and_one_with_pattern_characters_are_committed_by_name() {
        let dir = crate::daemon::persist::scratch_dir();
        let git = repository(dir.path());
        std::fs::remove_file(dir.path().join("a.txt")).expect("removed");
        std::fs::write(dir.path().join("[x].txt"), "odd\n").expect("a file");
        std::fs::write(dir.path().join("x.txt"), "plain\n").expect("a file");

        commit(&git, "by name", Some(&["a.txt".into(), "[x].txt".into()])).expect("the commit");
        assert_eq!(committed(&git), "[x].txt\na.txt");
        assert_eq!(git.run(&["status", "--porcelain"]).expect("git"), "?? x.txt", "the pattern matched no other file");
    }

    #[test]
    fn a_path_outside_the_checkout_under_git_or_with_a_control_character_is_refused() {
        let dir = crate::daemon::persist::scratch_dir();
        let git = repository(dir.path());
        std::fs::write(dir.path().join("new.txt"), "new\n").expect("a file");
        let before = git.run(&["rev-parse", "HEAD"]).expect("git");
        // A drive letter is only a way out of the checkout where there are drives.
        let elsewhere = if cfg!(windows) { "C:\\Windows\\win.ini" } else { "/etc/passwd" };
        for bad in ["../outside.txt", "/etc/passwd", elsewhere, ".git/config", "sub/.GIT/hooks/pre-commit", "-A", "", "a\n.txt", "a\u{1b}[31m.txt", "./new.txt"] {
            let refused = commit(&git, "m", Some(&["new.txt".into(), bad.into()])).expect_err(bad);
            assert!(refused.ends_with("is not a file of this checkout") && !refused.contains(['\n', '\u{1b}']), "{bad:?}: {refused}");
        }
        assert_eq!(commit(&git, "m", Some(&[])), Err("pick at least one file to commit".into()));
        let many: Vec<String> = (0..4000).map(|n| format!("folder/file-{n}.txt")).collect();
        assert!(commit(&git, "m", Some(&many)).is_err_and(|why| why.contains("too many files")));
        assert_eq!(git.run(&["rev-parse", "HEAD"]).expect("git"), before);
        assert_eq!(git.run(&["status", "--porcelain"]).expect("git"), "?? new.txt", "nothing was staged");
    }

    #[test]
    fn a_message_that_is_empty_too_long_or_has_a_control_character_is_refused() {
        let dir = crate::daemon::persist::scratch_dir();
        let git = repository(dir.path());
        std::fs::write(dir.path().join("new.txt"), "new\n").expect("a file");
        assert_eq!(commit(&git, " \n ", None), Err("write a commit message first".into()));
        assert_eq!(commit(&git, &"m".repeat(GIT_COMMIT_MESSAGE_MAX_BYTES + 1), None), Err("that commit message is too long".into()));
        assert!(commit(&git, "bad\u{1b}[31m", None).is_err_and(|why| why.contains("cannot be in one")));
        assert_eq!(git.run(&["status", "--porcelain"]).expect("git"), "?? new.txt");
    }

    #[test]
    fn with_nothing_changed_git_says_so_in_one_line() {
        let dir = crate::daemon::persist::scratch_dir();
        let git = repository(dir.path());
        let why = commit(&git, "nothing", None).expect_err("there is nothing to commit");
        assert!(why.starts_with("nothing to commit"), "{why}");
    }
}
