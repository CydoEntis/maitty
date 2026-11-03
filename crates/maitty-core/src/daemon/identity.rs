//! Who this daemon is. A random id is made the first time the daemon starts and kept in the
//! data folder, so a window can tell a different machine answering at a saved name. It is not
//! a secret and proves nothing about the caller: it only tells machines apart.

use std::path::{Path, PathBuf};

const FILE_NAME: &str = "daemon-id";
const ID_BYTES: usize = 16;

pub fn path() -> PathBuf {
    crate::config::data_dir().join(FILE_NAME)
}

/// The saved id, or a new one saved now. A missing, unreadable or malformed file is replaced:
/// starting must not fail over a file that only names the daemon, and the only cost of a new
/// id is that windows record this machine afresh.
pub fn load_or_create(file: &Path) -> String {
    match std::fs::read_to_string(file) {
        Ok(text) if is_id(text.trim()) => return text.trim().to_owned(),
        Ok(_) => tracing::warn!("{} is not a daemon id; making a new one", file.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => tracing::warn!("couldn't read {} ({e}); making a new id", file.display()),
    }
    let id = generate();
    if let Err(e) = crate::config::write_atomic(file, &id) {
        tracing::warn!("couldn't save the daemon id to {}: {e}", file.display());
    }
    id
}

pub(super) fn generate() -> String {
    let mut bytes = [0u8; ID_BYTES];
    if let Err(e) = getrandom::fill(&mut bytes) {
        tracing::warn!("no OS randomness for the daemon id ({e}); using the clock");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        bytes = (nanos ^ u128::from(std::process::id())).to_le_bytes();
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn is_id(text: &str) -> bool {
    text.len() == ID_BYTES * 2 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-identity-{}-{}"),
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_id_is_made_once_and_kept_across_restarts() {
        let file = dir("kept").join(FILE_NAME);
        let first = load_or_create(&file);
        assert!(is_id(&first), "{first}");
        assert_eq!(load_or_create(&file), first);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), first);
    }

    #[test]
    fn two_data_folders_are_two_daemons() {
        let a = load_or_create(&dir("a").join(FILE_NAME));
        let b = load_or_create(&dir("b").join(FILE_NAME));
        assert_ne!(a, b);
    }

    #[test]
    fn a_corrupt_file_is_replaced_not_fatal() {
        let file = dir("corrupt").join(FILE_NAME);
        std::fs::write(&file, "not an id").unwrap();
        let id = load_or_create(&file);
        assert!(is_id(&id));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), id);
    }
}
