//! A session's check result on its sidebar row: the glyph that stands for it, its colour,
//! and the line of words under the row. Shape and word carry the meaning; colour backs them.
use super::glyphs::{self, Glyphs};
use super::*;
use crate::protocol::{CheckInfo, CheckState};

/// The ring steps once per this many spinner frames.
const RING_TICKS: u64 = 2;
const EXIT_NOT_EXECUTABLE: i32 = 126;
const EXIT_NOT_FOUND: i32 = 127;
const SECS_PER_MIN: u64 = 60;

/// The set `[ui] glyphs` picks; plain ASCII icons get ASCII marks unless it says otherwise.
pub(super) fn marks(app: &App) -> &'static Glyphs {
    glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons)
}

/// The result to show for a session. While the agent works or asks, the last result is
/// about an older turn, so the row says only what the agent is doing.
pub(super) fn shown(s: &Session) -> Option<&CheckInfo> {
    crate::protocol::reported_check(s.status, s.check.as_ref())
}

/// A turn that ended and whose work is not known to be bad: no check, or one that passed, went
/// stale, was overridden or is never run here. A running, failed or could-not-run check, or one
/// waiting for approval, keeps it from counting as finished.
pub(super) fn finished(s: &Session) -> bool {
    s.status == Status::Done
        && shown(s).is_none_or(|info| {
            matches!(
                info.state,
                CheckState::Passed | CheckState::Overridden | CheckState::Stale | CheckState::Never
            )
        })
}

/// It needs you: it asks, or its repo's check command waits for your approval.
pub(super) fn needs_you(s: &Session) -> bool {
    needs(s.status, s.check.as_ref())
}

/// The one rule for needs you, for a sidebar row or a pane: asking, or a check to approve.
pub(super) fn needs(status: Status, check: Option<&CheckInfo>) -> bool {
    status == Status::Blocked || approving(status, check)
}

/// Needs you only because its repo's check command waits for your approval.
pub(super) fn approving(status: Status, check: Option<&CheckInfo>) -> bool {
    crate::protocol::reported_check(status, check)
        .is_some_and(|info| info.state == CheckState::NeedsApproval)
}

/// The mark that stands for a failed check, in the same set as the sidebar's glyphs.
pub(super) fn failed_mark(app: &App) -> &'static str {
    marks(app).failed
}

pub(super) fn failed(info: &CheckInfo) -> bool {
    info.state == CheckState::Failed
}

/// A result that has come in, so it can be run again.
pub(super) fn settled(info: &CheckInfo) -> bool {
    info.state != CheckState::Checking
}

/// Which ring glyph a spinner frame shows. Apart from the clock so a test can step it.
pub(super) fn ring_step(frame: u64, len: usize) -> usize {
    (frame / RING_TICKS % len as u64) as usize
}

pub(super) fn glyph(app: &App, info: &CheckInfo) -> &'static str {
    let m = marks(app);
    match info.state {
        CheckState::Checking => {
            m.ring[if app.motion_on() {
                ring_step(app.spinner_frame(), m.ring.len())
            } else {
                0
            }]
        }
        CheckState::Passed => m.done,
        CheckState::Failed => m.failed,
        CheckState::Overridden => m.over,
        CheckState::CouldNotRun | CheckState::Never => m.couldnt,
        CheckState::Stale => m.stale,
        CheckState::NeedsApproval => m.needs,
        CheckState::Pending => m.pend,
    }
}

fn styled(text: impl Into<String>, fg: Color, bold: bool) -> Seg {
    let st = Style::default().fg(fg);
    seg(
        text,
        if bold {
            st.add_modifier(Modifier::BOLD)
        } else {
            st
        },
    )
}

/// What a checkpoint is called: its name, or for a result saved before checkpoints had names,
/// its command's last word (as `CheckDef::name` does).
pub(super) fn name(info: &CheckInfo) -> &str {
    if info.name.is_empty() {
        info.command.split_whitespace().last().unwrap_or_default()
    } else {
        &info.name
    }
}

/// The line under a row with one checkpoint, named by its checkpoint name (the command, while
/// it waits for approval, since that is what you approve).
pub(super) fn result_words(info: &CheckInfo, t: &Theme) -> Vec<Seg> {
    let name = name(info);
    match info.state {
        CheckState::Checking => vec![styled(
            format!("checking {name} · {}", age(info.started_at)),
            t.sky(),
            false,
        )],
        CheckState::Pending => vec![styled(format!("next: {name}"), t.muted, false)],
        CheckState::Passed => vec![
            styled(format!("passed: {name}"), t.done, true),
            styled(
                format!(
                    " · {} · {}",
                    duration(info.duration_ms),
                    crate::clock::local_hm(info.started_at + info.duration_ms / 1000)
                ),
                t.muted,
                false,
            ),
        ],
        CheckState::Failed => match info.exit_code {
            Some(code) => vec![styled(format!("failed: {name} · exit {code}"), t.err, true)],
            None => vec![styled(format!("failed: {name}"), t.err, true)],
        },
        CheckState::Overridden => vec![styled(format!("failed, overridden: {name}"), t.text, true)],
        CheckState::CouldNotRun => vec![styled(
            format!("{name} could not run{}", why_not_run(info.exit_code)),
            t.violet(),
            false,
        )],
        CheckState::Stale => {
            let n = info.changed_files.len();
            vec![
                styled(format!("stale pass: {name}"), t.text, true),
                styled(
                    format!(" · {n} file{} changed", if n == 1 { "" } else { "s" }),
                    t.muted,
                    false,
                ),
            ]
        }
        CheckState::Never => vec![styled(
            "not checked: never for this repo",
            t.violet(),
            false,
        )],
        CheckState::NeedsApproval => vec![styled(
            format!("approve check: {}", info.command),
            t.blocked,
            false,
        )],
    }
}

pub(super) fn duration(ms: u64) -> String {
    let secs = ms / 1000;
    if secs < SECS_PER_MIN {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m {}s", secs / SECS_PER_MIN, secs % SECS_PER_MIN)
    }
}

pub(super) fn why_not_run(exit: Option<i32>) -> String {
    match exit {
        Some(EXIT_NOT_FOUND) => " · not found".into(),
        Some(EXIT_NOT_EXECUTABLE) => " · not executable".into(),
        Some(code) => format!(" · exit {code}"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_from_before_checkpoints_had_names_is_named_by_its_command() {
        let old = CheckInfo {
            command: "npm run lint".into(),
            ..Default::default()
        };
        assert_eq!(name(&old), "lint");
        assert_eq!(
            name(&CheckInfo {
                name: "build".into(),
                ..old
            }),
            "build"
        );
    }

    #[test]
    fn the_ring_steps_every_second_spinner_frame_and_wraps() {
        let steps: Vec<usize> = (0..10).map(|f| ring_step(f, 4)).collect();
        assert_eq!(steps, [0, 0, 1, 1, 2, 2, 3, 3, 0, 0]);
    }
}
