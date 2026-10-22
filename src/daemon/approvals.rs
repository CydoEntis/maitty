//! What you decided about repo-supplied check commands, kept in Seshi's own data folder and
//! never in the repo: an agent that can write the repo must not be able to approve its own
//! command. An Always holds only for the exact command it was given to (by its hash).

use crate::config::same_path;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "check-approvals.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Always,
    Never,
}

/// One decision about one checkpoint of one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    /// The repository's main checkout.
    pub repo: PathBuf,
    /// The checkpoint's name.
    pub name: String,
    /// `command_hash` of the command it was given to.
    pub command_sha256: String,
    pub decision: Decision,
    /// When you decided (unix seconds).
    pub at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Approvals(pub Vec<Approval>);

pub fn path() -> PathBuf {
    crate::config::data_dir().join(FILE_NAME)
}

pub fn command_hash(command: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(command.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

impl Approvals {
    /// The saved decisions; none when nothing was saved yet. Blocks: call it off the loop.
    pub fn load(path: &Path) -> Result<Approvals> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Approvals::default()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))
    }

    /// Write them whole and synced before the rename, so a crash leaves the old file or the
    /// new one, never half of one. Blocks: call it off the loop.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
        let mut f = std::fs::File::create(&tmp).with_context(|| format!("writing {}", tmp.display()))?;
        f.write_all(&serde_json::to_vec_pretty(self)?)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
    }

    /// What you decided for `repo`'s checkpoint `name` running `command`. Never holds whatever
    /// the command became; Always only while the command is the one you approved.
    pub fn lookup(&self, repo: &Path, name: &str, command: &str) -> Option<Decision> {
        let a = self.0.iter().find(|a| same_path(&a.repo, repo) && a.name.eq_ignore_ascii_case(name))?;
        match a.decision {
            Decision::Never => Some(Decision::Never),
            Decision::Always => (a.command_sha256 == command_hash(command)).then_some(Decision::Always),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approval(repo: &str, name: &str, command: &str, decision: Decision) -> Approval {
        Approval { repo: repo.into(), name: name.into(), command_sha256: command_hash(command), decision, at: 100 }
    }

    #[test]
    fn a_command_hashes_to_its_sha256_in_hex() {
        assert_eq!(command_hash(""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_ne!(command_hash("npm test"), command_hash("npm test "), "the exact command, not a near one");
    }

    #[test]
    fn always_holds_only_for_the_exact_command_it_was_given_to() {
        let a = Approvals(vec![approval("/code/site", "test", "npm test", Decision::Always)]);
        assert_eq!(a.lookup(Path::new("/code/site"), "test", "npm test"), Some(Decision::Always));
        assert_eq!(a.lookup(Path::new("/code/site"), "test", "npm test && node seed.js"), None, "a changed command asks again");
        assert_eq!(a.lookup(Path::new("/code/site/"), "TEST", "npm test"), Some(Decision::Always), "same repo and name however spelled");
        assert_eq!(a.lookup(Path::new("/code/other"), "test", "npm test"), None, "another repo");
        assert_eq!(a.lookup(Path::new("/code/site"), "build", "npm test"), None, "another checkpoint");
    }

    #[test]
    fn never_holds_whatever_the_command_becomes() {
        let a = Approvals(vec![approval("/code/site", "test", "npm test", Decision::Never)]);
        assert_eq!(a.lookup(Path::new("/code/site"), "test", "npm test"), Some(Decision::Never));
        assert_eq!(a.lookup(Path::new("/code/site"), "test", "rm -rf ~"), Some(Decision::Never));
    }

    #[test]
    fn decisions_survive_a_save_and_load_and_nothing_saved_is_none() {
        let dir = super::super::persist::scratch_dir();
        let file = dir.path().join(FILE_NAME);
        assert_eq!(Approvals::load(&file).unwrap(), Approvals::default());
        let a = Approvals(vec![approval("/code/site", "test", "npm test", Decision::Always), approval("/code/site", "build", "make", Decision::Never)]);
        a.save(&file).unwrap();
        assert_eq!(Approvals::load(&file).unwrap(), a);
        assert!(!file.with_extension("json.tmp").exists(), "nothing left half-written");
    }

    #[test]
    fn a_damaged_file_is_an_error_not_an_empty_list() {
        let dir = super::super::persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        let file = dir.path().join(FILE_NAME);
        std::fs::write(&file, "{ not json").unwrap();
        assert!(Approvals::load(&file).is_err());
    }

    #[test]
    fn they_are_kept_in_seshi_s_data_folder() {
        assert_eq!(path(), crate::config::data_dir().join("check-approvals.json"));
    }
}
