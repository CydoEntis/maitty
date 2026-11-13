//! Cloning a git repository into a new folder, for a person bringing a project onto this machine
//! (`Query::CloneRepo`). The address comes from a client, so it is checked before git sees it, and
//! git is given no way to ask a question: a clone that needs a password it cannot find fails.

use super::folders;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The longest address worth cloning; a longer one is a mistake or an attack.
pub(super) const ADDRESS_MAX_BYTES: usize = 2048;
/// How long one clone may take before it is stopped.
pub(super) const CLONE_LIMIT: Duration = Duration::from_secs(600);
const POLL: Duration = Duration::from_millis(50);
/// The address forms git fetches over a network. Anything else (a local path, `file://`, `ext::`)
/// would read this machine's own disk or run a program, and is refused.
const SCHEMES: [&str; 3] = ["https://", "ssh://", "git://"];
/// Characters no folder name may hold on at least one of the systems this runs on.
const NOT_IN_A_NAME: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// The address as git should get it, or why it is refused.
pub(super) fn check_address(address: &str) -> Result<&str, String> {
    let address = address.trim();
    if address.is_empty() {
        return Err("Give the address of the repository to clone".into());
    }
    if address.len() > ADDRESS_MAX_BYTES {
        return Err("That address is too long to clone".into());
    }
    if address.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("A repository address has no spaces or line breaks in it".into());
    }
    // git would read one that starts with a dash as an option.
    if address.starts_with('-') {
        return Err("A repository address does not start with a dash".into());
    }
    let named = SCHEMES
        .iter()
        .any(|scheme| address.strip_prefix(scheme).is_some_and(|rest| !rest.is_empty()));
    if named || is_scp_form(address) {
        Ok(address)
    } else {
        Err("Use an https://, ssh:// or git:// address, or the git@host:owner/name form".into())
    }
}

/// `user@host:path`, the short form of an ssh address. A Windows path (`C:\x`) has no `@` before
/// its colon, so it is not one.
fn is_scp_form(address: &str) -> bool {
    let Some((who, path)) = address.split_once(':') else {
        return false;
    };
    let Some((user, host)) = who.split_once('@') else {
        return false;
    };
    !user.is_empty() && !host.is_empty() && !who.contains('/') && !path.is_empty()
}

/// The folder git would name the clone: the last part of the address, without `.git`.
pub(super) fn folder_name(address: &str) -> Option<String> {
    let last = address.trim_end_matches('/').rsplit(['/', ':']).next()?;
    let name = last.strip_suffix(".git").unwrap_or(last);
    let usable = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.chars().any(|c| c.is_control() || NOT_IN_A_NAME.contains(&c));
    usable.then(|| name.to_owned())
}

/// Clones `address` into a new folder inside `parent` (a path as a client types it) and gives the
/// new folder's path with `/` separators. Blocks until git ends or `limit` passes; a clone that
/// fails or is stopped leaves no folder behind.
pub(super) fn clone(address: &str, parent: &str, home: Option<&Path>, limit: Duration) -> Result<String, String> {
    let address = check_address(address)?;
    let name = folder_name(address).ok_or("That address does not end in a repository name")?;
    let parent = folders::resolve(parent, home).ok_or("Choose the folder to clone into by its full path")?;
    if !parent.is_dir() {
        return Err(format!("There is no folder {}", shown(&parent)));
    }
    let target = parent.join(&name);
    // Also true for a link that points nowhere, which git would refuse to clone over.
    if target.symlink_metadata().is_ok() {
        return Err(format!("{} is already there", shown(&target)));
    }
    let mut command = Command::new("git");
    command
        .args(["clone", "--quiet", "--"])
        .arg(address)
        .arg(&target)
        // Its messages are read below: keep them in English whatever the person's locale.
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    crate::proc::quiet(&mut command);
    match run(&mut command, limit) {
        Ok(()) => Ok(shown(&target)),
        Err(why) => {
            // The folder was not there before this clone, so what is there now is only its remains.
            if target.symlink_metadata().is_ok()
                && let Err(error) = std::fs::remove_dir_all(&target)
            {
                tracing::warn!("couldn't remove what a failed clone left in {}: {error}", target.display());
            }
            Err(why)
        }
    }
}

fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Runs git to its end or to `limit`, whichever is first. On failure, the line of what it said
/// that names the problem.
fn run(command: &mut Command, limit: Duration) -> Result<(), String> {
    let mut child = command.spawn().map_err(|error| format!("couldn't run git: {error}"))?;
    // Read as it is written, so a long complaint cannot fill the pipe and stall git.
    let said = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut text = String::new();
            // What could not be read is not shown; the exit status still decides the outcome.
            let _ = pipe.read_to_string(&mut text);
            text
        })
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= limit => {
                // Both can fail only when git has just ended by itself, which is what is wanted.
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("The clone was stopped after {} seconds", limit.as_secs()));
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(error) => return Err(format!("couldn't wait for git: {error}")),
        }
    };
    if status.success() {
        return Ok(());
    }
    let said = said.and_then(|reader| reader.join().ok()).unwrap_or_default();
    // git says "warning: …" before the "fatal:" line that matters.
    let line = said
        .lines()
        .find(|line| line.starts_with("fatal") || line.starts_with("error"))
        .or_else(|| said.lines().find(|line| !line.trim().is_empty()))
        .map_or("git could not clone it", str::trim);
    Err(line.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
        assert!(status.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&status.stderr));
    }

    /// A repository with one commit, to clone from.
    fn source(root: &Path) -> std::path::PathBuf {
        let repo = root.join("origin").join("widgets.git");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);
        std::fs::write(repo.join("README.md"), "hello").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-q", "-m", "first"]);
        repo
    }

    #[test]
    fn network_addresses_are_accepted_as_written() {
        for address in ["https://github.com/o/r.git", "ssh://git@github.com/o/r", "git://host/r", "git@github.com:o/r.git", "  https://h/r  "] {
            assert_eq!(check_address(address), Ok(address.trim()), "{address}");
        }
    }

    #[test]
    fn an_address_git_could_misread_or_that_reads_this_disk_is_refused() {
        let long = format!("https://h/{}", "a".repeat(ADDRESS_MAX_BYTES));
        for address in ["", "   ", "--upload-pack=touch x", "-u", "https://h/a b", "https://h/a\nb", "https://", "http://h/r", "file:///etc", "ext::sh -c x", "/home/me/r", "C:\\code\\r", "../r", "host:path", "@host:path", "git@:path", "git@host:", long.as_str()] {
            assert!(check_address(address).is_err(), "{address:?} was accepted");
        }
    }

    #[test]
    fn the_new_folder_is_named_after_the_repository() {
        assert_eq!(folder_name("https://github.com/o/widgets.git").as_deref(), Some("widgets"));
        assert_eq!(folder_name("git@github.com:o/widgets").as_deref(), Some("widgets"));
        assert_eq!(folder_name("git@github.com:widgets.git/").as_deref(), Some("widgets"));
        assert_eq!(folder_name("https://h/.."), None);
        assert_eq!(folder_name("https://h/.git"), None);
        assert_eq!(folder_name("https://h/a?b"), None);
    }

    /// The checks an address must pass are the caller's; this runs the clone itself on a local one.
    fn clone_local(source: &Path, parent: &Path, limit: Duration) -> Result<(), String> {
        let target = parent.join("widgets");
        let mut command = Command::new("git");
        command.args(["clone", "--quiet", "--"]).arg(source).arg(&target).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
        run(&mut command, limit)
    }

    #[test]
    fn a_clone_that_works_leaves_the_repository_in_the_new_folder() {
        let root = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(root.path()).unwrap();
        let parent = root.path().join("code");
        std::fs::create_dir_all(&parent).unwrap();

        clone_local(&source(root.path()), &parent, CLONE_LIMIT).unwrap();

        assert_eq!(std::fs::read_to_string(parent.join("widgets").join("README.md")).unwrap(), "hello");
    }

    #[test]
    fn a_clone_that_fails_says_gits_own_reason() {
        let root = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(root.path()).unwrap();

        let why = clone_local(&root.path().join("nothing-here"), root.path(), CLONE_LIMIT).unwrap_err();

        assert!(why.starts_with("fatal"), "{why}");
    }

    #[test]
    fn a_clone_that_takes_too_long_is_stopped() {
        let root = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(root.path()).unwrap();

        let why = clone_local(&source(root.path()), root.path(), Duration::ZERO).unwrap_err();

        assert_eq!(why, "The clone was stopped after 0 seconds");
    }

    #[test]
    fn nothing_is_cloned_over_what_is_already_there_or_into_a_folder_that_is_not() {
        let root = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(root.path()).unwrap();
        let parent = root.path().to_string_lossy().into_owned();
        std::fs::create_dir(root.path().join("widgets")).unwrap();

        let taken = clone("https://example.invalid/o/widgets.git", &parent, None, CLONE_LIMIT).unwrap_err();
        let missing = clone("https://example.invalid/o/widgets.git", &format!("{parent}/none"), None, CLONE_LIMIT).unwrap_err();
        let relative = clone("https://example.invalid/o/widgets.git", "code", None, CLONE_LIMIT).unwrap_err();

        assert!(taken.ends_with("/widgets is already there"), "{taken}");
        assert!(missing.starts_with("There is no folder "), "{missing}");
        assert_eq!(relative, "Choose the folder to clone into by its full path");
        assert_eq!(std::fs::read_dir(root.path().join("widgets")).unwrap().count(), 0);
    }

    #[test]
    fn a_clone_that_fails_leaves_no_folder_behind() {
        let root = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(root.path()).unwrap();
        let parent = root.path().to_string_lossy().into_owned();

        // `.invalid` never resolves (RFC 2606), so this fails without reaching any server.
        let why = clone("https://example.invalid/o/widgets.git", &parent, None, CLONE_LIMIT).unwrap_err();

        assert!(!why.is_empty());
        assert!(!root.path().join("widgets").exists());
    }
}
