//! Sessions starting helpers. Any session may; what happens the first time is
//! `[sessions] helpers`: `always` lets it through, `never` refuses with a clear error, and
//! `ask` makes the session's row need you ("wants to start N helpers") until you answer.
//!
//! The request is only ever made by a start that came from a pane proving it is that pane
//! (its secret), and only you can answer it: `AnswerHelpers` is refused from inside a pane and
//! accepted only for a request the daemon holds. "Always in this project" is kept in
//! `helper-projects.json` in Vibecasso's data folder, never in the repo, so an agent that can
//! write the repo cannot grant itself helpers.

use super::{Daemon, approvals::write_atomic};
use crate::config::same_path;
use crate::protocol::{HelperChoice, TermId};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "helper-projects.json";
/// Longest task shown on the row and in the popover.
const TASK_CHARS: usize = 160;
/// Most tasks one request lists; more are counted by the ones already there.
const MOST_TASKS: usize = 12;

pub fn path() -> PathBuf {
    crate::config::data_dir().join(FILE_NAME)
}

/// The projects (main checkouts) where sessions may always start helpers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Projects {
    pub always: Vec<PathBuf>,
}

impl Projects {
    /// The saved projects; none when nothing was saved yet. Blocks: call it off the loop.
    pub fn load(path: &Path) -> Result<Projects> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).with_context(|| format!("reading {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Projects::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    pub fn allows(&self, project: &Path) -> bool {
        self.always.iter().any(|p| same_path(p, project))
    }

    /// Add `project`; false when it was already there.
    pub fn add(&mut self, project: &Path) -> bool {
        let new = !self.allows(project);
        if new {
            self.always.push(project.to_path_buf());
        }
        new
    }

    fn save(&self, path: &Path) -> Result<()> {
        write_atomic(path, &serde_json::to_vec_pretty(self)?)
    }
}

/// Add `project` to the file as it is now, not this daemon's copy: every daemon (one per
/// `VIBECASSO_SOCKET`) shares it. A damaged file is left as it is. Blocks.
fn remember(file: &Path, project: &Path) -> Result<()> {
    static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one_at_a_time = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let mut on_disk = Projects::load(file)?;
    if on_disk.add(project) {
        on_disk.save(file)?;
    }
    Ok(())
}

/// What one session has been asked and told about helpers. Lives and dies with the pane.
#[derive(Debug, Clone, Default)]
pub struct HelperState {
    /// The tasks it wants helpers for and is waiting on you for; empty: not asking.
    pub asked: Vec<String>,
    /// You said Allow: it may start helpers for as long as it runs.
    pub allowed: bool,
    /// You said no: it does the work alone.
    pub refused: bool,
}

/// A line to show for a start: the quoted prompt that ends the agent's command line, else the
/// worktree's branch, else the command.
pub fn task_label(cmd: Option<&str>, branch: Option<&str>) -> String {
    let from_cmd = cmd.map(str::trim).filter(|c| !c.is_empty()).map(|c| quoted_tail(c).unwrap_or_else(|| c.to_string()));
    let label = from_cmd.or_else(|| branch.map(str::to_string)).unwrap_or_else(|| "a shell".to_string());
    let one_line = label.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(TASK_CHARS).collect()
}

/// The text of a trailing `'quoted'` or `"quoted"` word (a shell-quoted prompt), unescaped.
fn quoted_tail(cmd: &str) -> Option<String> {
    let quote = cmd.chars().last().filter(|c| *c == '\'' || *c == '"')?;
    let body = &cmd[..cmd.len() - 1];
    // Walk back to the quote that opens it. Inside, a quote is escaped: POSIX writes `'\''`,
    // PowerShell doubles it, and a double-quoted word has `\"`.
    let mut open = None;
    let mut i = body.len();
    while i > 0 {
        i -= 1;
        if body.as_bytes()[i] != quote as u8 {
            continue;
        }
        let before = &body[..i];
        if quote == '\'' && before.ends_with("'\\'") {
            i -= 3;
        } else if (quote == '\'' && before.ends_with('\'')) || (quote == '"' && before.ends_with('\\')) {
            i -= 1;
        } else {
            open = Some(i);
            break;
        }
    }
    let inner = &body[open? + 1..];
    Some(if quote == '\'' { inner.replace("'\\''", "'").replace("''", "'") } else { inner.replace("\\\"", "\"") })
}

impl Daemon {
    /// The project a session works in: its repo's main checkout, else its folder.
    fn helper_project(&self, term: TermId) -> Option<PathBuf> {
        self.terms.get(&term).map(|t| t.head.as_ref().map_or_else(|| t.cwd.clone(), |h| h.main_root.clone()))
    }

    /// Whether session `by` may start a helper for `task` now. An error says why not, and is
    /// what the agent reads: a refusal to do it alone, or that you have been asked.
    pub(super) fn may_start_helper(&mut self, by: TermId, task: &str) -> Result<()> {
        // A person's shell is not an agent asking for help.
        if self.terms.get(&by).is_none_or(|t| t.agent.is_none()) {
            return Ok(());
        }
        match self.cfg.sessions.helpers.as_str() {
            "always" => return Ok(()),
            "never" => anyhow::bail!("The user has turned helpers off (Settings, Sessions). Do the work yourself."),
            _ => {}
        }
        let project = self.helper_project(by).unwrap_or_default();
        let remembered = self.helper_projects.allows(&project);
        let Some(t) = self.terms.get_mut(&by) else { return Ok(()) };
        if t.helpers.refused {
            anyhow::bail!("The user said no helpers for this session. Do the work yourself.");
        }
        if t.helpers.allowed || remembered {
            return Ok(());
        }
        let first = t.helpers.asked.is_empty();
        if !t.helpers.asked.iter().any(|a| a == task) && t.helpers.asked.len() < MOST_TASKS {
            t.helpers.asked.push(task.to_string());
        }
        let n = t.helpers.asked.len();
        self.dirty = true;
        if first {
            self.announce(by, crate::protocol::Status::Blocked);
        }
        anyhow::bail!(
            "Starting helpers needs the user's OK. Their vibecasso row now says it wants to start {n} helper{}. Call vibecasso_start for every helper you want so they all show, then try again later; don't start them another way.",
            if n == 1 { "" } else { "s" }
        )
    }

    /// Your answer to session `term`'s request to start helpers. Only while it asks.
    pub(super) fn answer_helpers(&mut self, term: TermId, choice: HelperChoice) -> Result<()> {
        let project = self.helper_project(term);
        let t = self.terms.get_mut(&term).ok_or_else(|| anyhow::anyhow!("no pane {term}"))?;
        if t.helpers.asked.is_empty() {
            anyhow::bail!("that session isn't asking to start helpers");
        }
        t.helpers.asked.clear();
        match choice {
            HelperChoice::Allow => t.helpers.allowed = true,
            HelperChoice::No => t.helpers.refused = true,
            HelperChoice::Always => {
                t.helpers.allowed = true;
                if let Some(project) = project {
                    self.helper_projects.add(&project);
                    let file = self.helper_projects_file.clone();
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = remember(&file, &project) {
                            tracing::warn!("saving the projects helpers are always allowed in: {e:#}");
                        }
                    });
                }
            }
        }
        self.dirty = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_is_the_quoted_prompt_at_the_end_of_the_command() {
        assert_eq!(task_label(Some("claude 'fix the login bug'"), None), "fix the login bug");
        assert_eq!(task_label(Some("claude --model opus 'it'\\''s late'"), None), "it's late");
        assert_eq!(task_label(Some("claude 'it''s late'"), None), "it's late", "PowerShell doubles the quote");
        assert_eq!(task_label(Some("codex \"port the footer\""), None), "port the footer");
        assert_eq!(task_label(Some("claude 'a\nb  c'"), None), "a b c", "one line");
    }

    #[test]
    fn without_a_quoted_prompt_the_branch_then_the_command_then_a_shell() {
        assert_eq!(task_label(Some("lazygit"), Some("feat-x")), "lazygit");
        assert_eq!(task_label(None, Some("feat-x")), "feat-x");
        assert_eq!(task_label(None, None), "a shell");
        assert_eq!(task_label(Some("claude '"), None), "claude '", "an unclosed quote is left alone");
        assert_eq!(task_label(Some(&format!("claude '{}'", "x".repeat(500))), None).chars().count(), TASK_CHARS);
    }

    #[test]
    fn projects_are_remembered_by_place_not_spelling_and_a_missing_file_is_none() {
        let dir = super::super::persist::scratch_dir();
        let file = dir.path().join(FILE_NAME);
        assert_eq!(Projects::load(&file).unwrap(), Projects::default());
        remember(&file, Path::new("/code/App")).unwrap();
        remember(&file, Path::new("/code/app/")).unwrap();
        let back = Projects::load(&file).unwrap();
        assert_eq!(back.always.len(), 1, "the same place once");
        assert!(back.allows(Path::new("/code/app")) && !back.allows(Path::new("/code/lib")));
        std::fs::write(&file, "not json").unwrap();
        assert!(remember(&file, Path::new("/x")).is_err() && std::fs::read_to_string(&file).unwrap() == "not json", "a damaged file is left as it is");
    }
}
