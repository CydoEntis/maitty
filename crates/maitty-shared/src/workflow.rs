//! A workflow's progress, as the workflow reports it from inside a pane (ADR 0010). It is display
//! only: it never changes a pane's status. Maitty runs no workflow itself.

use crate::hidden::sanitize;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Longest workflow name, run id, step name or ticket key, in characters.
pub const NAME_MAX_CHARS: usize = 64;
/// Longest ticket title, in characters.
pub const TITLE_MAX_CHARS: usize = 200;
/// Longest step note or helper label, in characters.
pub const NOTE_MAX_CHARS: usize = 240;
/// Longest ticket link, in characters.
pub const LINK_MAX_CHARS: usize = 500;
/// Most steps one run may have.
pub const STEPS_MAX: usize = 32;

/// One run of a workflow: one job, usually for one ticket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRun {
    /// The workflow's own name ("kitchen").
    pub workflow: String,
    /// The workflow's id for this run; a report with the same workflow and run replaces the last.
    pub run: String,
    #[serde(default)]
    pub ticket: Option<WorkflowTicket>,
    /// In order.
    pub steps: Vec<WorkflowStep>,
    /// When the daemon took the report (unix seconds). What a reporter sends here is ignored.
    #[serde(default)]
    pub reported_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTicket {
    pub key: String,
    #[serde(default)]
    pub title: Option<String>,
    /// An http or https address.
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub state: StepState,
    #[serde(default)]
    pub note: Option<String>,
    /// The name the workflow gave the helper doing this step (Claude's Agent description, Codex's
    /// task name), so a view can link the step to that helper in the same pane.
    #[serde(default)]
    pub helper: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Waiting,
    Working,
    Passed,
    Failed,
    SentBack,
    NeedsYou,
    Skipped,
    /// A state added in a later version. A reporter may not send it.
    #[serde(other)]
    Unknown,
}

/// `run` made safe to keep and draw: every string loses control and hidden characters and is
/// trimmed, blank optional fields become `None`, and `reported_at` is zeroed for the daemon to
/// set. `Err` names the first field that is missing, too long or not allowed.
pub fn checked(run: WorkflowRun) -> Result<WorkflowRun, String> {
    let ticket = run.ticket.map(checked_ticket).transpose()?;
    if run.steps.is_empty() {
        return Err("a run needs at least one step".into());
    }
    if run.steps.len() > STEPS_MAX {
        return Err(format!("a run has at most {STEPS_MAX} steps"));
    }
    let steps = run
        .steps
        .into_iter()
        .map(checked_step)
        .collect::<Result<Vec<_>, _>>()?;
    let mut names = HashSet::new();
    if let Some(twice) = steps.iter().find(|step| !names.insert(step.name.as_str())) {
        return Err(format!("step \"{}\" is named twice", twice.name));
    }
    Ok(WorkflowRun {
        workflow: required("workflow", &run.workflow, NAME_MAX_CHARS)?,
        run: required("run", &run.run, NAME_MAX_CHARS)?,
        ticket,
        steps,
        reported_at: 0,
    })
}

fn checked_ticket(ticket: WorkflowTicket) -> Result<WorkflowTicket, String> {
    let link = optional("ticket link", ticket.link.as_deref(), LINK_MAX_CHARS)?;
    if let Some(link) = &link
        && !(link.starts_with("https://") || link.starts_with("http://"))
    {
        return Err("ticket link must be an http or https address".into());
    }
    Ok(WorkflowTicket {
        key: required("ticket key", &ticket.key, NAME_MAX_CHARS)?,
        title: optional("ticket title", ticket.title.as_deref(), TITLE_MAX_CHARS)?,
        link,
    })
}

fn checked_step(step: WorkflowStep) -> Result<WorkflowStep, String> {
    let name = required("step name", &step.name, NAME_MAX_CHARS)?;
    if step.state == StepState::Unknown {
        return Err(format!("step \"{name}\" has an unknown state"));
    }
    Ok(WorkflowStep {
        state: step.state,
        note: optional("step note", step.note.as_deref(), NOTE_MAX_CHARS)?,
        helper: optional("step helper", step.helper.as_deref(), NOTE_MAX_CHARS)?,
        name,
    })
}

fn required(field: &str, text: &str, max: usize) -> Result<String, String> {
    optional(field, Some(text), max)?.ok_or_else(|| format!("{field} is empty"))
}

fn optional(field: &str, text: Option<&str>, max: usize) -> Result<Option<String>, String> {
    let Some(text) = text else { return Ok(None) };
    let clean = sanitize(text, false).trim().to_string();
    if clean.chars().count() > max {
        return Err(format!("{field} is longer than {max} characters"));
    }
    Ok(Some(clean).filter(|text| !text.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(name: &str, state: StepState) -> WorkflowStep {
        WorkflowStep {
            name: name.into(),
            state,
            note: None,
            helper: None,
        }
    }

    fn run(steps: Vec<WorkflowStep>) -> WorkflowRun {
        WorkflowRun {
            workflow: "kitchen".into(),
            run: "001".into(),
            ticket: None,
            steps,
            reported_at: 99,
        }
    }

    #[test]
    fn a_report_is_cleaned_and_its_time_left_to_the_daemon() {
        let mut report = run(vec![WorkflowStep {
            note: Some("  \x1b[31mchecks\x1b[0m failed  ".into()),
            helper: Some("   ".into()),
            ..step("build", StepState::SentBack)
        }]);
        report.ticket = Some(WorkflowTicket {
            key: " MAITTY-42 ".into(),
            title: Some("".into()),
            link: Some("https://plane.example/MAITTY-42".into()),
        });
        let clean = checked(report).unwrap();
        assert_eq!(clean.reported_at, 0);
        assert_eq!(clean.steps[0].note.as_deref(), Some("[31mchecks[0m failed"));
        assert_eq!(clean.steps[0].helper, None);
        let ticket = clean.ticket.unwrap();
        assert_eq!(ticket.key, "MAITTY-42");
        assert_eq!(ticket.title, None);
    }

    #[test]
    fn a_report_that_cannot_be_drawn_safely_is_refused_with_its_reason() {
        assert_eq!(checked(run(vec![])).unwrap_err(), "a run needs at least one step");
        let many = (0..=STEPS_MAX).map(|n| step(&n.to_string(), StepState::Waiting)).collect();
        assert!(checked(run(many)).unwrap_err().contains("at most"));
        let twice = vec![step("build", StepState::Passed), step("build", StepState::Working)];
        assert_eq!(checked(run(twice)).unwrap_err(), "step \"build\" is named twice");
        assert!(checked(run(vec![step("build", StepState::Unknown)])).unwrap_err().contains("unknown state"));
        let long = "x".repeat(NAME_MAX_CHARS + 1);
        assert!(checked(run(vec![step(&long, StepState::Waiting)])).unwrap_err().contains("longer than"));
        let mut nameless = run(vec![step("build", StepState::Waiting)]);
        nameless.workflow = " \t".into();
        assert_eq!(checked(nameless).unwrap_err(), "workflow is empty");
        let mut linked = run(vec![step("build", StepState::Waiting)]);
        linked.ticket = Some(WorkflowTicket {
            key: "A-1".into(),
            title: None,
            link: Some("javascript:alert(1)".into()),
        });
        assert!(checked(linked).unwrap_err().contains("http or https"));
    }

    #[test]
    fn a_reporter_writes_states_in_snake_case_and_a_later_state_reads_as_unknown() {
        let step: WorkflowStep =
            serde_json::from_str(r#"{"name":"review","state":"needs_you"}"#).unwrap();
        assert_eq!(step.state, StepState::NeedsYou);
        let later: WorkflowStep =
            serde_json::from_str(r#"{"name":"review","state":"paused"}"#).unwrap();
        assert_eq!(later.state, StepState::Unknown);
    }
}
