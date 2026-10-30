//! A secret handed to an agent as a file (ADR-0036): the path is all the agent gets in chat.
//!
//! The folder is the owner's alone on unix (`0700`, files `0600`). On Windows the folder sits in
//! the user's own app data, which only they and administrators can read; there is no stricter
//! mode to set. Files are never overwritten, and none outlives a day: the daemon removes every
//! one when it starts and the stale ones as it runs, and a request clears the stale ones too.

use anyhow::{Context, Result, bail};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How long a delivered file may stay; the same as a reference's life (`daemon/secrets.rs`).
pub const KEPT_FOR: Duration = Duration::from_secs(24 * 60 * 60);
const FOLDER: &str = "secrets";

pub fn folder() -> PathBuf {
    crate::config::data_dir().join(FOLDER)
}

/// Write `value` to a new file named for `reference` in `dir`, readable by the owner only, and
/// return its path. Blocks. An existing file of that name is an error, never overwritten.
pub fn write(dir: &Path, reference: &str, value: &str) -> Result<PathBuf> {
    let name = reference.strip_prefix("secret-ref:").filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_hexdigit()));
    let Some(name) = name else { bail!("not a secret reference") };
    make_private_dir(dir)?;
    let path = dir.join(name);
    let mut file = open_new_private(&path)?;
    file.write_all(value.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    file.sync_all().with_context(|| format!("saving {}", path.display()))?;
    Ok(path)
}

/// Remove the files in `dir` older than `max_age` (zero: all of them). Blocks. A missing folder
/// has nothing in it; a file that can't be removed is reported with the rest tried.
pub fn remove_stale(dir: &Path, max_age: Duration, now: SystemTime) -> Result<usize> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e).with_context(|| format!("reading {}", dir.display())),
    };
    let (mut removed, mut failed) = (0, Vec::new());
    for entry in entries.flatten() {
        let path = entry.path();
        let age = entry.metadata().and_then(|m| m.modified()).ok().and_then(|t| now.duration_since(t).ok());
        // A file whose age can't be read is as good as old: better gone than lingering.
        if max_age.is_zero() || age.is_none_or(|a| a >= max_age) {
            match std::fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(e) => failed.push(format!("{}: {e}", path.display())),
            }
        }
    }
    if failed.is_empty() { Ok(removed) } else { bail!("couldn't remove {}", failed.join("; ")) }
}

#[cfg(unix)]
fn make_private_dir(dir: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir).with_context(|| format!("making {}", dir.display()))?;
    // A folder made earlier by something else keeps its own mode unless it is tightened.
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).with_context(|| format!("securing {}", dir.display()))
}

#[cfg(not(unix))]
fn make_private_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))
}

#[cfg(unix)]
fn open_new_private(path: &Path) -> Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path).with_context(|| format!("creating {}", path.display()))
}

#[cfg(not(unix))]
fn open_new_private(path: &Path) -> Result<std::fs::File> {
    std::fs::OpenOptions::new().write(true).create_new(true).open(path).with_context(|| format!("creating {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-secret-files-{}-{}"), std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_secret_is_written_once_under_the_reference_name_and_never_overwritten() {
        let dir = scratch("write");
        let path = write(&dir, "secret-ref:00ff", "hunter2").unwrap();
        assert_eq!((path.file_name().unwrap().to_str(), std::fs::read_to_string(&path).unwrap().as_str()), (Some("00ff"), "hunter2"));
        assert!(write(&dir, "secret-ref:00ff", "other").is_err(), "the same name is refused");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hunter2", "and left as it was");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_a_reference_makes_a_file_name() {
        let dir = scratch("names");
        for bad in ["", "secret-ref:", "secret-ref:../x", "secret-ref:a/b", "00ff", "secret-ref:zz"] {
            assert!(write(&dir, bad, "x").is_err(), "{bad:?}");
        }
        assert!(!dir.exists() || std::fs::read_dir(&dir).unwrap().next().is_none(), "nothing was written");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn the_folder_and_file_are_the_owners_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("modes");
        let path = write(&dir, "secret-ref:00ff", "x").unwrap();
        assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_files_go_and_fresh_ones_stay_and_zero_removes_all() {
        let dir = scratch("stale");
        write(&dir, "secret-ref:aa", "x").unwrap();
        write(&dir, "secret-ref:bb", "y").unwrap();
        let now = SystemTime::now();
        assert_eq!(remove_stale(&dir, KEPT_FOR, now).unwrap(), 0, "fresh");
        assert_eq!(remove_stale(&dir, KEPT_FOR, now + KEPT_FOR + Duration::from_secs(1)).unwrap(), 2, "a day on");
        write(&dir, "secret-ref:cc", "z").unwrap();
        assert_eq!(remove_stale(&dir, Duration::ZERO, now).unwrap(), 1, "all of them, whatever their age");
        assert_eq!(remove_stale(&dir.join("missing"), KEPT_FOR, now).unwrap(), 0, "no folder, nothing to do");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
