//! Chips drawn on a frame's border: ` TEXT ` on a ground of its own. The sidebar, the workspace
//! border and the pane headers share these kinds; a chip's ends follow `ui.rounded`.

use super::*;

/// What a chip is for, which fixes its colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum Chip {
    /// A frame's title; lit (accent) when its frame has the focus.
    Title { focused: bool },
    /// A button.
    Action,
    /// Information, not a button.
    Quiet,
}

/// `text` as a chip over `outer` (the ground under its ends). Its width is `segs_width` of the
/// result: the text plus a cell each side (plus the caps with rounded shapes).
pub(in crate::client) fn chip(
    app: &App,
    text: &str,
    kind: Chip,
    outer: Color,
    t: &Theme,
) -> Vec<Seg> {
    let bold = Modifier::BOLD;
    let (bg, style) = match kind {
        Chip::Title { focused: true } => {
            (t.accent, Style::default().fg(t.acc_ink).add_modifier(bold))
        }
        Chip::Title { focused: false } => (t.btn, Style::default().fg(t.strong).add_modifier(bold)),
        Chip::Action => (t.btn, Style::default().fg(t.accent).add_modifier(bold)),
        Chip::Quiet => (t.btn, Style::default().fg(t.muted)),
    };
    pill(Look::of(&app.cfg.ui), vec![seg(text, style)], bg, outer)
}

