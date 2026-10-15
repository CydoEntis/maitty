# Spec: clear session state and workflow controls

Intent: [intent.md](intent.md) · Status: accepted · Date: 2026-10-15

## Concerns

Preserve running sessions, human plan approval and native agent permissions.
Preview and explicitly copy instructions; preserve files. Run executable probes,
network and clipboard I/O off the UI loop. Use selection markers as well as fill,
retaining semantic status colors. TestBackend timings cannot prove native terminal
smoothness; keep wall-clock thresholds out of ordinary CI tests.

## Requirements and coverage

- **R1:** Active fill and `▸` survive hover over sessions, projects and blank space.
  Hover is quieter; keyboard browsing has `›` separately. Change active selection
  after daemon acknowledgement. Check: active-selection and existing theme tests.
- **R2:** Settings distinguishes Installed here, This window and Running daemon,
  labeling remote hosts. Show restart pending and discovery dots for mismatches.
  Install updates without closing the window or restarting the daemon. Do not
  offer an already-installed release again; expose probe failures. Check:
  version-display and updater tests.
- **R3:** Sidebar status icons and pane status tags open existing status details.
  Show a plain-language explanation, uncertainty without confirmation, and actual
  observation ages. Repeated screen observations must not reset turn duration or
  repeat transition alerts. Check: status-click and repeated-observation tests.
- **R4:** Setup previews custom roles and instructions using the selected saved
  workflow name. Explicitly copy instructions and explain where to paste and that
  existing sessions must reread them. Check: setup test and clipboard/UI check.
- **R5:** Sidebar labels show stage, review round, questions and ended states.
  Inbox includes plan approvals even without a live stage pane. Mouse and keyboard
  open the exact run; navigation never approves. Keep approvals until daemon state
  advances. Check: stage-label, Inbox approval and daemon authorization tests.
- **R6:** An opt-in release-mode test feeds eight panes with output and interleaves
  switching, scrollback and resize/rewrap. Assert focus, bounds, scrolling and
  retained output. Print p50/p95/max per action, messages and bytes. Check:
  `session_interactions_under_heavy_output -- --ignored --nocapture`.

## Interfaces and data

Protocol 39 adds `Snapshot.daemon_version` and `TermInfo.why_process_at`.
`why_screen_at` is the latest observation; `since` stays the status start time.
No persisted-session migration is required. Definitions, authenticated reports
and human approval retain existing contracts. Setup and installed-version probes
are local; Runs and approvals use the connected daemon's snapshot and IPC.

## Verification

Run the full AGENTS.md gate. Inspect rendered frames at 80×24, 100×30 and normal
widths. Record release-mode performance results and limits in TESTING.md. Check
the live UI in a named server, preserving running sessions and settings; state
any check that could not run. Implementation is local until explicitly released.

Automated verification for v0.30.0: `cargo build`, `cargo test` (276 passed, four ignored),
`cargo clippy --all-targets -- -D warnings`, and both cross-target checks passed
in order. The opt-in optimized performance test passed separately; see
[measurements](../../TESTING.md#project-specific-interaction-performance).
Rendered setup frames were inspected at 80×24, 100×30 and 160×45; version,
status-evidence and Inbox approval frames were also inspected. Active-selection
coverage includes sleeping sessions. The sleep-marker regression failed before
the spacing fix. Live named-server UI and clipboard checks remain unverified:
automatic approval review rejected the earlier launch as "blocked by policy"
without a more specific reason. No dependencies, migrations or user config added.
