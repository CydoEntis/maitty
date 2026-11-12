//! The text brief a new agent starts with when another agent's work moves to it. Text only: the new
//! agent gets no conversation, memory or tool results. Shared so the terminal client's `handoff` command
//! and the desktop app write the same brief; the git and process work stays with each client.

use crate::protocol::TermId;
use std::path::PathBuf;

/// Screen lines carried over, from the bottom.
pub const TAIL_LINES: usize = 20;
/// Changed files listed before the rest are counted.
pub const MAX_FILES: usize = 30;
/// The whole brief, which is passed on the command line as the agent's first prompt.
pub const MAX_BRIEF_CHARS: usize = 4000;

/// What the old pane was doing.
pub struct Facts {
    pub pane: TermId,
    pub agent: String,
    /// `TermInfo.name`: the /rename name, else the first prompt.
    pub task: String,
    /// `TermInfo.summary`.
    pub last_prompt: String,
    pub branch: Option<String>,
    pub checkout: PathBuf,
    pub changed: Result<Changed, String>,
    pub screen: String,
}

/// What a checkout changed, as far as git could tell.
#[derive(Debug, PartialEq)]
pub struct Changed {
    pub files: Vec<String>,
    /// Why the branch's committed changes could not be listed; `files` then holds only the
    /// uncommitted paths.
    pub committed_unknown: Option<String>,
}

/// The brief for a new agent, naming the app `app` it came from.
pub fn brief(facts: &Facts, app: &str) -> String {
    let head = head(facts, app);
    let label = "\nrecent output:\n";
    let room = MAX_BRIEF_CHARS.saturating_sub(head.chars().count() + label.chars().count());
    let tail = tail_within(&facts.screen, room);
    if tail.is_empty() {
        return cut(&head, MAX_BRIEF_CHARS);
    }
    format!("{head}{label}{tail}")
}

/// Everything but the recent output.
fn head(f: &Facts, app: &str) -> String {
    let mut s = format!(
        "Handoff from {} in {app} pane {}. This is a text summary only: you do not have the previous agent's conversation, memory or tool results, so read the files yourself.\n\n",
        f.agent, f.pane
    );
    let task = f.task.trim();
    s += &format!("task: {}\n", if task.is_empty() { "unknown" } else { task });
    let last = f.last_prompt.trim();
    if !last.is_empty() && last != task {
        s += &format!("last prompt: {last}\n");
    }
    s += &format!(
        "branch: {}\ncheckout: {}\n",
        f.branch.as_deref().unwrap_or("none"),
        f.checkout.display()
    );
    s += &changed_section(&f.changed);
    s
}

fn changed_section(changed: &Result<Changed, String>) -> String {
    let c = match changed {
        Err(why) => return format!("changed files: unknown ({why})\n"),
        Ok(c) => c,
    };
    let mut s = String::from(if c.files.is_empty() {
        "changed files: none\n"
    } else {
        "changed files:\n"
    });
    for f in c.files.iter().take(MAX_FILES) {
        s += &format!("- {f}\n");
    }
    if c.files.len() > MAX_FILES {
        s += &format!("… and {} more\n", c.files.len() - MAX_FILES);
    }
    if let Some(why) = &c.committed_unknown {
        s += &format!("committed changes: unknown ({why})\n");
    }
    s
}

/// The last non-blank screen lines (at most `TAIL_LINES`) that fit in `room` characters.
fn tail_within(screen: &str, room: usize) -> String {
    let lines: Vec<&str> = screen
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut used = 0;
    for line in lines.iter().rev().take(TAIL_LINES) {
        let cost = line.chars().count() + 1;
        if used + cost > room {
            break;
        }
        used += cost;
        kept.push(line);
    }
    kept.reverse();
    kept.iter().map(|l| format!("{l}\n")).collect()
}

fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

/// The paths in `git status --porcelain -z`. A rename or copy is followed by its old path,
/// which is skipped.
pub fn uncommitted(porcelain: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut entries = porcelain.split('\0');
    while let Some(e) = entries.next() {
        let Some((xy, path)) = e.split_at_checked(3).filter(|(_, p)| !p.is_empty()) else {
            continue;
        };
        out.push(path.to_string());
        if xy.contains('R') || xy.contains('C') {
            entries.next();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_brief_names_the_app_it_came_from() {
        let facts = Facts {
            pane: 3,
            agent: "claude".into(),
            task: "fix it".into(),
            last_prompt: String::new(),
            branch: None,
            checkout: PathBuf::from("/work/app"),
            changed: Ok(Changed { files: vec![], committed_unknown: None }),
            screen: String::new(),
        };
        assert!(brief(&facts, "someapp").starts_with("Handoff from claude in someapp pane 3."));
    }
}
