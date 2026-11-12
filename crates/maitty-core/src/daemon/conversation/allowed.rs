//! Commands you approved "always in this project" for a chat, kept in the app's own data folder and
//! never in the repo: an agent that can write the repo must not be able to approve its own command.
//! A rule holds for one project and the exact command it was given to, by its hash: the same
//! command written differently asks again. Only command requests can be remembered, never a file
//! change, a permission grant or a question (ADR 0005).

use crate::config::same_path;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "chat-allowed-commands.json";

pub fn path() -> PathBuf {
    crate::config::data_dir().join(FILE_NAME)
}

/// One command approved for one project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// The repository's main checkout.
    pub project: PathBuf,
    /// `command_hash` of the command exactly as the agent sent it. This, not `command`, decides.
    pub command_sha256: String,
    /// The command as it was shown to you (secrets hidden), to tell what a rule is for.
    pub command: String,
    /// When you approved it (unix seconds).
    pub at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Allowed {
    pub rules: Vec<Rule>,
}

impl Allowed {
    /// The saved rules; none when nothing was saved yet. Blocks: call it off the loop.
    pub fn load(path: &Path) -> Result<Allowed> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("reading {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Allowed::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    fn save(&self, path: &Path) -> Result<()> {
        super::super::approvals::write_atomic(path, &serde_json::to_vec_pretty(self)?)
    }

    /// Whether `project` has approved exactly this command, as the agent sent it.
    #[cfg(test)]
    pub fn allows(&self, project: &Path, command: &str) -> bool {
        self.allows_hash(project, &crate::protocol::command_hash(command))
    }

    /// `allows` for a command already hashed.
    pub fn allows_hash(&self, project: &Path, command_sha256: &str) -> bool {
        self.rules
            .iter()
            .any(|r| r.command_sha256 == command_sha256 && same_path(&r.project, project))
    }

    /// Add `rule`; false when the project already approved this command.
    pub fn add(&mut self, rule: Rule) -> bool {
        let new = !self
            .rules
            .iter()
            .any(|r| r.command_sha256 == rule.command_sha256 && same_path(&r.project, &rule.project));
        if new {
            self.rules.push(rule);
        }
        new
    }
}

/// Add `rule` to the file as it is now, not this daemon's copy: every daemon (one per
/// `<APP>_SOCKET`) shares it. A damaged file is left as it is. Blocks.
pub fn remember(file: &Path, rule: Rule) -> Result<()> {
    static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one_at_a_time = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let mut on_disk = Allowed::load(file)?;
    if on_disk.add(rule) {
        on_disk.save(file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::command_hash;

    fn rule(project: &str, command: &str) -> Rule {
        Rule {
            project: PathBuf::from(project),
            command_sha256: command_hash(command),
            command: command.into(),
            at: 1,
        }
    }

    #[test]
    fn a_rule_holds_for_one_project_and_the_exact_command() {
        let mut allowed = Allowed::default();
        assert!(allowed.add(rule("/code/app", "cargo test")));
        assert!(!allowed.add(rule("/code/app", "cargo test")), "the same rule once");
        assert!(allowed.allows(Path::new("/code/app"), "cargo test"));
        assert!(!allowed.allows(Path::new("/code/app"), "cargo test "), "written differently asks again");
        assert!(!allowed.allows(Path::new("/code/app"), "cargo test && rm -rf ."));
        assert!(!allowed.allows(Path::new("/code/other"), "cargo test"), "another project asks");
    }

    #[test]
    fn rules_survive_a_restart_and_a_second_daemon_adds_to_the_file_as_it_is() {
        let dir = std::env::temp_dir().join(format!("maitty-allowed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join(FILE_NAME);
        assert_eq!(Allowed::load(&file).unwrap(), Allowed::default(), "nothing saved yet");
        remember(&file, rule("/code/app", "cargo test")).unwrap();
        remember(&file, rule("/code/app", "cargo build")).unwrap();
        let back = Allowed::load(&file).unwrap();
        assert_eq!(back.rules.len(), 2);
        assert!(back.allows(Path::new("/code/app"), "cargo build"));
        std::fs::write(&file, "not json").unwrap();
        assert!(remember(&file, rule("/code/app", "x")).is_err(), "a damaged file is left alone");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "not json");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
