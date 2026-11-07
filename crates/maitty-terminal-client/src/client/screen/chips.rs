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
    /// A branch name.
    Branch,
    /// Something needs you: amber.
    Alert,
    /// The red ×.
    Close,
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
        Chip::Branch => (t.btn, Style::default().fg(t.violet())),
        Chip::Alert => (
            t.blocked,
            Style::default().fg(t.ink_on(t.blocked)).add_modifier(bold),
        ),
        Chip::Close => (
            t.err,
            Style::default().fg(t.ink_on(t.err)).add_modifier(bold),
        ),
    };
    pill(Look::of(&app.cfg.ui), vec![seg(text, style)], bg, outer)
}

/// Draw `chips` along the row `y` from the right edge `end` (exclusive) leftward, the first
/// listed being the leftmost. A chip that would reach `left` is left out, and so are all the
/// ones before it. Each is a click target for its hit (`None`: not clickable). Returns the
/// column the leftmost drawn chip starts at.
pub(in crate::client) fn right_chips(
    app: &mut App,
    buf: &mut Buffer,
    y: u16,
    (left, end): (u16, u16),
    chips: Vec<(Vec<Seg>, Option<ScreenHit>)>,
    border: Color,
) -> u16 {
    let mut x = end;
    for (segs, h) in chips.into_iter().rev() {
        let w = segs_width(&segs);
        if x < left + w + 1 {
            break;
        }
        x -= w;
        put(buf, x, y, &segs, x + w);
        if let Some(h) = h {
            hit(
                app,
                Rect {
                    x,
                    y,
                    width: w,
                    height: 1,
                },
                h,
            );
        }
        // A cell of border between neighbours.
        x -= 1;
        put(buf, x, y, &[seg("─", Style::default().fg(border))], x + 1);
    }
    x + 1
}
