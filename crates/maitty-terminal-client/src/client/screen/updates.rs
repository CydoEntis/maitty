//! The Updates box (prefix `u`, or the sidebar footer's marker): what is new, download, restart.
//! It reads the app's existing update state and uses its install and restart paths; restarting
//! the window never stops the daemon (`relaunch_window`). Docs: UI.md, "Service and update states".

use super::*;

/// The widest the box gets.
const BOX_W: u16 = 58;
/// The most lines of notes it lists.
const NOTES_SHOWN: usize = 8;
/// Rows the sidebar footer takes under the box: the rule and the `+ New` row, and the border.
const FOOTER_GAP: u16 = 3;

/// Where the update stands, from the app's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::client) enum Phase {
    Available { version: String },
    Downloading,
    Ready,
    Failed { error: String },
    UpToDate,
}

impl App {
    pub(in crate::client) fn update_phase(&self) -> Phase {
        if let Some(error) = &self.update_failed {
            return Phase::Failed { error: error.clone() };
        }
        if self.updating {
            return Phase::Downloading;
        }
        if self.restart_needed() {
            return Phase::Ready;
        }
        match &self.update_available {
            Some(v) if self.update_skipped.as_ref() != Some(v) => Phase::Available { version: v.clone() },
            _ => Phase::UpToDate,
        }
    }

    pub(in crate::client) fn open_updates(&mut self) {
        self.leave_shortcut_mode();
        self.screen.cursor = None;
        self.mode = Mode::Updates;
    }

    pub(in crate::client) fn on_updates_key(&mut self, k: &KeyEvent) {
        let phase = self.update_phase();
        match (k.code, &phase) {
            (KeyCode::Esc | KeyCode::Char('l'), _) => self.mode = Mode::Normal,
            (KeyCode::Char('d'), Phase::Available { .. }) => self.start_update(),
            (KeyCode::Char('s'), Phase::Available { version }) => {
                self.update_skipped = Some(version.clone());
                self.mode = Mode::Normal;
            }
            (KeyCode::Char('r'), Phase::Ready) => {
                self.mode = Mode::Normal;
                self.ask_restart();
            }
            (KeyCode::Char('r'), Phase::Failed { .. }) => {
                self.update_failed = None;
                self.start_update();
            }
            (KeyCode::Char('c'), Phase::UpToDate) => self.check_for_update(true),
            _ => {}
        }
    }
}

/// The keys the box offers in `phase`, as (key, words).
pub(in crate::client) fn updates_keys(phase: &Phase) -> Vec<(&'static str, &'static str)> {
    match phase {
        Phase::Available { .. } => vec![("d", "Download"), ("s", "Skip"), ("l", "Later")],
        Phase::Downloading => vec![("l", "Later")],
        Phase::Ready => vec![("r", "Restart now"), ("l", "Later")],
        Phase::Failed { .. } => vec![("r", "Retry"), ("l", "Close")],
        Phase::UpToDate => vec![("c", "Check now"), ("l", "Close")],
    }
}

/// The box, above the sidebar's footer.
pub(in crate::client) fn draw_updates(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) {
    let phase = app.update_phase();
    let notes: Vec<String> = app.update_notes.iter().take(NOTES_SHOWN).cloned().collect();
    let current = env!("APP_VERSION");
    let mut lines: Vec<Vec<Seg>> = Vec::new();
    let c = Style::default().bg(t.card);
    match &phase {
        Phase::Available { version } => {
            lines.push(vec![seg(format!("{APP_COMMAND} {current} → {version}"), c.fg(t.strong).add_modifier(Modifier::BOLD))]);
            if !notes.is_empty() {
                lines.push(vec![seg("NEW", c.fg(t.accent).add_modifier(Modifier::BOLD))]);
            }
            for n in &notes {
                lines.push(vec![seg("  · ", c.fg(t.muted)), seg(n.clone(), c.fg(t.text))]);
            }
        }
        Phase::Downloading => {
            let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame = spinner[(app.screen.tick as usize / 2) % spinner.len()];
            let step = crate::cli::update::step();
            let words = crate::cli::update::STEPS.get(step.wrapping_sub(1)).copied().unwrap_or("Starting");
            lines.push(vec![seg(format!("{frame} {words}…"), c.fg(t.accent))]);
            lines.push(progress_line(step, usize::from(BOX_W) - 8, c, t));
            lines.push(vec![seg("Sessions keep running while it installs.", c.fg(t.muted))]);
        }
        Phase::Ready => {
            lines.push(vec![seg("✓ Installed. Restart to switch to it.", c.fg(t.done))]);
            lines.push(vec![seg("Sessions keep running; only this window reopens.", c.fg(t.muted))]);
        }
        Phase::Failed { error } => {
            lines.push(vec![seg("! The update failed", c.fg(t.err).add_modifier(Modifier::BOLD))]);
            lines.push(vec![seg(truncate(error, usize::from(BOX_W) - 6), c.fg(t.muted))]);
        }
        Phase::UpToDate => {
            lines.push(vec![seg(format!("{APP_COMMAND} {current} is the latest."), c.fg(t.text))]);
        }
    }
    let keys = updates_keys(&phase);
    let w = BOX_W.min(area.width.saturating_sub(2));
    let h = (lines.len() as u16 + 4).min(area.height);
    let side = grid(app, framed(app, area)).side;
    let x = (if side.width > 0 { side.x + 1 } else { area.x + 2 }).min(area.right().saturating_sub(w));
    let y = area.bottom().saturating_sub(FOOTER_GAP + h).max(area.y);
    let r = Rect { x, y, width: w, height: h };
    let buf = f.buffer_mut();
    super::popups::dim_all(app, buf, area, t);
    hit(app, area, ScreenHit::Close);
    let r = super::popups::panel_at(app, buf, r, "Updates", &[], t);
    hit(app, r, ScreenHit::Noop);
    let inside = Rect { x: r.x + 2, y: r.y + 1, width: r.width.saturating_sub(4), height: r.height.saturating_sub(3) };
    for (i, line) in lines.iter().enumerate().take(inside.height as usize) {
        put(buf, inside.x, inside.y + i as u16, line, inside.right());
    }
    let bar = super::popups::hints(t, &keys);
    put(buf, r.x + 2, r.bottom() - 1, &bar, r.right().saturating_sub(2));
}

/// A bar of the steps done so far, and "step 2 of 4". The tools report no bytes, so it moves in
/// whole steps and never claims a percentage.
fn progress_line(step: usize, width: usize, c: Style, t: &Theme) -> Vec<Seg> {
    let total = crate::cli::update::STEPS.len();
    let done = step.saturating_sub(1).min(total);
    let label = format!(" step {} of {total}", step.clamp(1, total));
    let bar = width.saturating_sub(label.width()).max(total);
    let filled = bar * done / total;
    vec![
        seg("█".repeat(filled), c.fg(t.accent)),
        seg("░".repeat(bar - filled), c.fg(t.line)),
        seg(label, c.fg(t.muted)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_counts_finished_steps_and_never_fills_before_the_last_is_done() {
        let t = Theme::named(crate::theme::DEFAULT);
        let text = |step| -> String {
            progress_line(step, 40, Style::default(), &t)
                .iter()
                .map(|s| s.0.as_str())
                .collect()
        };
        assert!(text(0).starts_with('░') && text(0).ends_with("step 1 of 4"));
        assert!(text(1).starts_with('░'));
        assert_eq!(text(3).matches('█').count(), 2 * (40 - " step 3 of 4".len()) / 4);
        assert!(text(4).contains('░'), "the last step is still running");
    }

    #[test]
    fn every_phase_can_be_left_with_l() {
        for phase in [
            Phase::Available { version: "9.9.9".into() },
            Phase::Downloading,
            Phase::Ready,
            Phase::Failed { error: "offline".into() },
            Phase::UpToDate,
        ] {
            assert!(updates_keys(&phase).iter().any(|(k, _)| *k == "l"), "{phase:?}");
        }
    }
}
