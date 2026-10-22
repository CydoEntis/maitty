//! The Inbox rows for checks that did not pass: which sessions they are, and how one is drawn.
//! A needs-you question always sorts above them, and a failure never reads as just finished.
use super::*;
use crate::protocol::CheckState;

pub(super) const FAILED_HEAD: &str = "FAILED CHECKS";
pub(super) const NOT_RUN_HEAD: &str = "COULD NOT RUN";

/// Rows a failed check takes: the head line, the result, the first failing line, a gap.
pub(super) const FAILED_ROWS: u16 = 4;
/// Rows a could-not-run check takes: the head line, the result, a gap.
pub(super) const NOT_RUN_ROWS: u16 = 3;

/// Where a session's check puts it in the Inbox, if anywhere. A failure stays until a later
/// run replaces it, even after you have looked; unchecked work is listed only until you look.
pub(super) fn row(s: &Session) -> Option<GoRow> {
    let info = check::shown(s)?;
    match info.state {
        CheckState::Failed => Some(GoRow::Failed(s.term)),
        CheckState::CouldNotRun if s.status == Status::Done => Some(GoRow::NotRun(s.term)),
        _ => None,
    }
}

/// The rows of one group, the longest waiting first like the others.
pub(super) fn rows(model: &[Proj], want: fn(&GoRow) -> bool) -> Vec<GoRow> {
    let mut found: Vec<(u64, GoRow)> = model.iter().flat_map(Proj::sessions).filter_map(|s| row(s).filter(want).map(|r| (s.since, r))).collect();
    found.sort_by_key(|(since, row)| (*since, row.term()));
    found.into_iter().map(|(_, row)| row).collect()
}

/// When the result came in, for its age: a run that never started has only the session's.
fn settled_at(info: &CheckInfo, s: &Session) -> u64 {
    if info.started_at == 0 { s.since } else { info.started_at + info.duration_ms / 1000 }
}

/// One row at (`x`, `y`) across `w` columns: glyph, name, folder and age; the result under
/// it; for a failure its first failing line. `head_bg` is the selection's tint.
pub(super) fn draw(app: &mut App, buf: &mut Buffer, (x, y, w): (u16, u16, u16), head_bg: Color, (proj, s): (&Proj, &Session), lit: bool, t: &Theme) {
    let Some(info) = s.check.clone() else { return };
    let info = &info;
    let head = Style::default().bg(head_bg);
    let card = Style::default().bg(t.card);
    let age = seg(super::age(settled_at(info, s)), head.fg(t.muted));
    let left = vec![
        seg(format!("{} ", check::glyph(app, info)), head.fg(check::color(t, info)).add_modifier(Modifier::BOLD)),
        seg(s.name.clone(), head.fg(t.strong).add_modifier(Modifier::BOLD)),
        seg(format!("  {} · {}", s.agent, proj.name), head.fg(t.muted)),
    ];
    let right = segs_width(std::slice::from_ref(&age)) as u16;
    put(buf, x + 1, y, &left, (x + w).saturating_sub(right + 1));
    put(buf, (x + w).saturating_sub(right), y, &[age], x + w);
    let room = w.saturating_sub(4) as usize;
    put(buf, x + 3, y + 1, &[seg(check::words(app, info, room), check::words_style(info, t, t.card, false))], x + w);
    if check::failed(info) && !info.first_failing_line.is_empty() {
        put(buf, x + 3, y + 2, &[seg(truncate(&info.first_failing_line, room), card.fg(t.muted))], x + w);
    }
    if lit && check::failed(info) {
        let term = s.term;
        let mut at = x + 3;
        for action in ACTIONS {
            let Some(r) = check::action_pill(app, buf, at, y + 3, x + w, action, t) else { break };
            hit(app, r, ScreenHit::InboxCheck(term, action.1));
            at = r.right() + 1;
        }
    }
}

/// The buttons of a selected failed row: label and the key it stands for.
const ACTIONS: [(&str, char); 3] = [("Output", 'o'), ("Re-run", 'r'), ("Override…", 'O')];
