//! The workflow runs a pane reports (ADR 0010): kept for display, never read for status.

use maitty_shared::workflow::WorkflowRun;

/// Most runs one pane keeps; the least recently reported go first.
pub(super) const RUNS_PER_PANE_MAX: usize = 20;

/// Takes a checked report into a pane's runs, newest first: it replaces the run's last report,
/// since a report is the run's whole state.
pub(super) fn record(runs: &mut Vec<WorkflowRun>, mut run: WorkflowRun, now: u64) {
    run.reported_at = now;
    runs.retain(|kept| !(kept.workflow == run.workflow && kept.run == run.run));
    runs.insert(0, run);
    runs.truncate(RUNS_PER_PANE_MAX);
}

#[cfg(test)]
mod tests {
    use super::*;
    use maitty_shared::workflow::{StepState, WorkflowStep};

    fn run(workflow: &str, id: &str, state: StepState) -> WorkflowRun {
        WorkflowRun {
            workflow: workflow.into(),
            run: id.into(),
            ticket: None,
            steps: vec![WorkflowStep {
                name: "build".into(),
                state,
                note: None,
                helper: None,
            }],
            reported_at: 0,
        }
    }

    #[test]
    fn a_new_report_replaces_its_run_and_moves_it_to_the_front() {
        let mut runs = Vec::new();
        record(&mut runs, run("kitchen", "001", StepState::Working), 10);
        record(&mut runs, run("kitchen", "002", StepState::Working), 11);
        record(&mut runs, run("other", "001", StepState::Working), 12);
        record(&mut runs, run("kitchen", "001", StepState::Passed), 13);
        let order: Vec<_> = runs.iter().map(|r| (r.workflow.as_str(), r.run.as_str())).collect();
        assert_eq!(order, [("kitchen", "001"), ("other", "001"), ("kitchen", "002")]);
        assert_eq!(runs[0].steps[0].state, StepState::Passed);
        assert_eq!(runs[0].reported_at, 13);
    }

    #[test]
    fn a_pane_keeps_only_its_latest_runs() {
        let mut runs = Vec::new();
        for n in 0..=RUNS_PER_PANE_MAX {
            record(&mut runs, run("kitchen", &n.to_string(), StepState::Passed), n as u64);
        }
        assert_eq!(runs.len(), RUNS_PER_PANE_MAX);
        assert!(runs.iter().all(|r| r.run != "0"));
    }
}
