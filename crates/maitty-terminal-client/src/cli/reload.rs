//! `<app> reload-agents`: restart the agents that never loaded their status hooks.
//!
//! An agent started before `<app> integrate` ran does not know about the hooks until it is
//! restarted, so its status is only read off the screen. The daemon restarts one in place and
//! resumes its conversation (`Command::ReloadAgent`); this picks which ones, and keeps the ones
//! in the middle of a turn out of it unless asked, since a turn in flight is not replayed.

use super::integrations::STATUS_AGENTS;
use super::{command_to, snapshot_to};
use crate::protocol::{Command, Snapshot, Status, TermInfo};
use anyhow::Result;

/// Which agents a reload would touch and which it leaves alone.
pub(crate) struct Plan<'a> {
    pub restart: Vec<&'a TermInfo>,
    /// Hookless too, but mid-turn or waiting for you.
    pub skipped: Vec<&'a TermInfo>,
}

/// An agent with a status integration, running, that has not reported through its hooks.
fn needs_hooks(t: &TermInfo) -> bool {
    !t.hooked
        && t.exit.is_none()
        && !t.asleep
        && t.agent.as_deref().is_some_and(|agent| STATUS_AGENTS.iter().any(|known| known.id == agent))
}

pub(crate) fn plan(snap: &Snapshot, include_working: bool) -> Plan<'_> {
    let (mut restart, mut skipped) = (Vec::new(), Vec::new());
    for t in snap.terms.values().filter(|t| needs_hooks(t)) {
        if !include_working && matches!(t.status, Status::Working | Status::Blocked) {
            skipped.push(t);
        } else {
            restart.push(t);
        }
    }
    Plan { restart, skipped }
}

/// What to call a pane in the list: the name you gave it, else the agent's, else the agent.
pub(crate) fn shown_name(t: &TermInfo) -> String {
    [&t.label, &t.name]
        .into_iter()
        .find(|name| !name.is_empty())
        .cloned()
        .or_else(|| t.agent.clone())
        .unwrap_or_default()
}

pub fn reload_agents(target: &super::Target, include_working: bool, dry_run: bool) -> Result<()> {
    let snap = snapshot_to(target)?;
    let plan = plan(&snap, include_working);
    for t in &plan.skipped {
        println!(
            "pane {}  {}  is {}: left alone (--all restarts it too)",
            t.id,
            shown_name(t),
            if t.status == Status::Blocked { "waiting for you" } else { "working" }
        );
    }
    if plan.restart.is_empty() {
        println!("No running agent needs a restart.");
        return Ok(());
    }
    let mut failed = 0;
    for t in &plan.restart {
        if dry_run {
            println!("pane {}  {}  would restart", t.id, shown_name(t));
            continue;
        }
        match command_to(target, Command::ReloadAgent { term: t.id }) {
            Ok(()) => println!("pane {}  {}  restarted", t.id, shown_name(t)),
            Err(error) => {
                failed += 1;
                println!("pane {}  {}  not restarted: {error:#}", t.id, shown_name(t));
            }
        }
    }
    if failed > 0 {
        anyhow::bail!("{failed} agent(s) could not be restarted");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(id: u32, agent: Option<&str>, hooked: bool, status: Status) -> TermInfo {
        let mut t: TermInfo = serde_json::from_value(serde_json::json!({
            "id": id, "cols": 80, "rows": 24, "title": "", "process": "", "agent": agent,
            "status": "Idle", "cwd": "/code", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 0, "asleep": false, "win32_input": false,
        }))
        .unwrap();
        t.hooked = hooked;
        t.status = status;
        t
    }

    fn snapshot(panes: Vec<TermInfo>) -> Snapshot {
        Snapshot { terms: panes.into_iter().map(|t| (t.id, t)).collect(), ..Snapshot::default() }
    }

    #[test]
    fn only_running_agents_that_never_reported_through_hooks_are_restarted() {
        let snap = snapshot(vec![
            pane(1, Some("claude"), false, Status::Idle),
            pane(2, Some("claude"), true, Status::Idle),
            pane(3, None, false, Status::Idle),
            pane(4, Some("not-an-integration"), false, Status::Idle),
            pane(5, Some("codex"), false, Status::Done),
        ]);
        let ids: Vec<u32> = plan(&snap, false).restart.iter().map(|t| t.id).collect();
        assert_eq!(ids, [1, 5]);
    }

    #[test]
    fn a_working_or_waiting_agent_is_left_alone_unless_all_is_asked() {
        let snap = snapshot(vec![
            pane(1, Some("claude"), false, Status::Working),
            pane(2, Some("claude"), false, Status::Blocked),
            pane(3, Some("claude"), false, Status::Idle),
        ]);
        let careful = plan(&snap, false);
        assert_eq!(careful.restart.iter().map(|t| t.id).collect::<Vec<_>>(), [3]);
        assert_eq!(careful.skipped.iter().map(|t| t.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(plan(&snap, true).restart.len(), 3);
    }
}
