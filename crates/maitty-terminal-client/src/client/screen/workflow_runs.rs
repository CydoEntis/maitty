//! A session's workflow runs (ADR 0010): under the open session in the sidebar, and one run's steps
//! in a modal. Display only: a run never sets a session's status, and nothing here starts or stops one.

use super::*;
use super::side::{SideRow, TREE_X, put_cut};
use super::status_mark::State;
use crate::workflow::{StepState, WorkflowRun, WorkflowStep};

/// A run as the sidebar lists it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct RunRow {
    /// The workflow and run id: what names the run among the session's.
    pub key: String,
    /// The ticket's key, else the workflow and run id.
    pub label: String,
    /// The step the run is on, and its state.
    pub step: Option<(String, StepState)>,
}

/// Some step is not over yet: the run is still going.
pub(in crate::client) fn run_active(run: &WorkflowRun) -> bool {
    run.steps.iter().any(|s| {
        !matches!(s.state, StepState::Passed | StepState::Failed | StepState::Skipped | StepState::Unknown)
    })
}

/// The step a run is on: the first that needs you, else the first in hand, else the first still to
/// do, else the last that failed.
pub(in crate::client) fn current_step(run: &WorkflowRun) -> Option<&WorkflowStep> {
    [StepState::NeedsYou, StepState::Working, StepState::SentBack, StepState::Waiting]
        .iter()
        .find_map(|state| run.steps.iter().find(|s| s.state == *state))
        .or_else(|| run.steps.iter().rev().find(|s| s.state == StepState::Failed))
}

pub(in crate::client) fn run_key(run: &WorkflowRun) -> String {
    format!("{}:{}", run.workflow, run.run)
}

fn run_label(run: &WorkflowRun) -> String {
    run.ticket
        .as_ref()
        .map_or_else(|| format!("{} {}", run.workflow, run.run), |t| t.key.clone())
}

/// The runs still going, as the sidebar lists them.
pub(in crate::client) fn run_rows(runs: &[WorkflowRun]) -> Vec<RunRow> {
    runs.iter()
        .filter(|r| run_active(r))
        .map(|r| RunRow {
            key: run_key(r),
            label: run_label(r),
            step: current_step(r).map(|s| (s.name.clone(), s.state)),
        })
        .collect()
}

/// A step's mark: a shape of its own per state, so it reads without colour; amber only for needs you.
pub(in crate::client) fn step_mark(app: &App, state: StepState, t: &Theme) -> (String, Color) {
    let (glyph, color) = match state {
        StepState::Working => {
            let m = app.state_mark(State::Working, t);
            return (m.glyph, m.color);
        }
        StepState::NeedsYou => ("●", t.blocked),
        StepState::Passed => ("✓", t.done),
        StepState::Failed => ("✕", t.err),
        StepState::SentBack => ("↺", t.text),
        StepState::Waiting => ("·", t.muted),
        StepState::Skipped => ("–", t.muted),
        StepState::Unknown => ("?", t.muted),
    };
    (glyph.to_string(), color)
}

/// Under the open session, after its subagents: a line per run still going (its step's mark, the
/// ticket and the step), then "N runs done". A run's line opens its steps.
pub(super) fn draw_runs(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, row: &SideRow, t: &Theme) {
    let Some(term) = row.term else {
        return;
    };
    let base = Style::default().bg(t.bg);
    let end = r.right().saturating_sub(2);
    for (i, run) in row.runs.iter().enumerate() {
        let yy = y + i as u16;
        let line = Rect { x: r.x + 1, y: yy, width: r.width.saturating_sub(2), height: 1 };
        fill(buf, line, t.bg);
        let last = i + 1 == row.runs.len() && row.runs_done == 0;
        let mut segs = vec![seg(if last { "└" } else { "├" }, base.fg(t.faint())), seg(" ", base)];
        if let Some((step, state)) = &run.step {
            let (glyph, color) = step_mark(app, *state, t);
            segs.extend([seg(glyph, base.fg(color)), seg(" ", base)]);
            segs.extend([seg(run.label.clone(), base.fg(t.text)), seg(" ", base), seg(step.clone(), base.fg(t.muted))]);
        } else {
            segs.push(seg(run.label.clone(), base.fg(t.text)));
        }
        put_cut(buf, r.x + TREE_X, yy, segs, end);
        app.screen.run_rows.push((term, run.key.clone()));
        hit(app, line, ScreenHit::Run(app.screen.run_rows.len() - 1));
    }
    if row.runs_done > 0 {
        let yy = y + row.runs.len() as u16;
        fill(buf, Rect { x: r.x + 1, y: yy, width: r.width.saturating_sub(2), height: 1 }, t.bg);
        let word = if row.runs_done == 1 { "run" } else { "runs" };
        let segs = vec![seg("└", base.fg(t.faint())), seg(" ", base), seg(format!("{} {word} done", row.runs_done), base.fg(t.muted))];
        put_cut(buf, r.x + TREE_X, yy, segs, end);
    }
}

/// The widest the run modal grows.
const RUN_W: u16 = 72;

/// One run's steps: each with its mark, name, note and the helper doing it.
pub(in crate::client) fn draw_run(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, term: ClientTerm, key: &str) {
    let run = app
        .term_info(term)
        .and_then(|info| info.workflows.iter().find(|r| run_key(r) == key))
        .cloned();
    let buf = f.buffer_mut();
    dim_all(app, buf, area, t);
    let Some(run) = run else {
        let r = panel(app, buf, area, RUN_W, 6, "run", &[], t);
        put(buf, r.x + 3, r.y + 2, &[seg("This run is no longer reported.", Style::default().bg(t.card).fg(t.muted))], r.right().saturating_sub(3));
        return;
    };
    let title = run.ticket.as_ref().and_then(|t| t.title.clone());
    let h = run.steps.len() as u16 + 5 + u16::from(title.is_some());
    let r = panel(app, buf, area, RUN_W, h, &format!("run {}", run_label(&run)), &[], t);
    let ground = Style::default().bg(t.card);
    let (x, end) = (r.x + 3, r.right().saturating_sub(3));
    let mut y = r.y + 2;
    if let Some(title) = title {
        put(buf, x, y, &[seg(title, ground.fg(t.strong))], end);
        y += 1;
    }
    let current = current_step(&run).map(|s| s.name.clone());
    for step in &run.steps {
        let (glyph, color) = step_mark(app, step.state, t);
        let name_style = if current.as_deref() == Some(step.name.as_str()) { ground.fg(t.strong).bold() } else { ground.fg(t.text) };
        let mut segs = vec![seg(glyph, ground.fg(color)), seg(" ", ground), seg(step.name.clone(), name_style)];
        if let Some(note) = &step.note {
            segs.extend([seg("  ", ground), seg(note.clone(), ground.fg(t.muted))]);
        }
        if let Some(helper) = &step.helper {
            segs.extend([seg("  · ", ground.fg(t.muted)), seg(helper.clone(), ground.fg(t.muted))]);
        }
        put_cut(buf, x, y, segs, end);
        y += 1;
    }
}

impl App {
    pub(in crate::client) fn on_run_key(&mut self, k: &KeyEvent) {
        if matches!(k.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
            self.mode = Mode::Normal;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::WorkflowTicket;

    fn run(states: &[StepState], ticket: Option<&str>) -> WorkflowRun {
        WorkflowRun {
            workflow: "kitchen".into(),
            run: "007".into(),
            ticket: ticket.map(|key| WorkflowTicket { key: key.into(), title: None, link: None }),
            steps: ["intake", "build", "review"]
                .iter()
                .zip(states)
                .map(|(name, state)| WorkflowStep { name: (*name).into(), state: *state, note: None, helper: None })
                .collect(),
            reported_at: 1,
        }
    }

    #[test]
    fn a_run_is_on_the_step_that_needs_you_else_the_one_in_hand() {
        use StepState::*;
        let on = |states: &[StepState]| current_step(&run(states, None)).map(|s| s.name.clone());
        assert_eq!(on(&[Passed, Working, NeedsYou]).as_deref(), Some("review"));
        assert_eq!(on(&[Passed, SentBack, Waiting]).as_deref(), Some("build"));
        assert_eq!(on(&[Passed, Failed, Skipped]).as_deref(), Some("build"));
        assert!(run_active(&run(&[Passed, Waiting, Waiting], None)));
        assert!(!run_active(&run(&[Passed, Failed, Skipped], None)));
    }

    #[test]
    fn the_sidebar_lists_only_runs_still_going_by_their_ticket() {
        use StepState::*;
        let rows = run_rows(&[run(&[Passed, Working, Waiting], Some("MAITTY-42")), run(&[Passed, Passed, Passed], None)]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "MAITTY-42");
        assert_eq!(rows[0].step, Some(("build".into(), Working)));
        assert_eq!(run_label(&run(&[Waiting], None)), "kitchen 007");
    }
}
