//! What is left of the retired Hairline look: the plain-chip switch the shared chip helpers
//! consult (never on now) and the count of panes that need you.

use super::check;
use super::*;

/// A pane needs you: its own agent asks, or its own check waits for approval. The sidebar's
/// rows use the same rule (`check::needs`).
fn pane_needs(info: &TermInfo) -> bool {
    check::needs(info.status, info.check())
}

/// How many panes need you, a split's included: the right end of the mode bar says so.
pub(in crate::client) fn needing(app: &App) -> usize {
    app.snap
        .terms
        .values()
        .filter(|info| pane_needs(info))
        .count()
}

/// The theme Hairline draws a frame with: every surface (`card`, `btn`, `sidebar_bg`) is the
/// ground, so popups, menus and sheets sit on `bg` like the workspace. `card2` stays the
/// `raised` of a selected row.
pub(in crate::client) fn flat_theme(t: &Theme) -> Theme {
    let mut flat = t.clone();
    (flat.card, flat.btn, flat.sidebar_bg) = (t.bg, t.bg, t.bg);
    flat
}

/// Below this contrast with the ground a fill reads as a surface (`raised`, a selection or
/// hover tint), not as a colour.
const SURFACE_CONTRAST: f64 = 1.6;

thread_local! {
    /// The ground of a frame drawn in Hairline (`set_plain`), so the chip, pill, keycap and
    /// button helpers that have no `Look` at hand draw plain text. Per thread, as
    /// `card::set_round`.
    static PLAIN: std::cell::Cell<Option<Color>> = const { std::cell::Cell::new(None) };
}

/// Draw this frame's chips as plain text (`Some`, with the flat theme) or as chips.
pub(in crate::client) fn set_plain(t: Option<&Theme>) {
    PLAIN.set(t.map(|t| t.bg));
}

/// This frame is drawn in Hairline: chips are plain text.
pub(in crate::client) fn plain_on() -> bool {
    PLAIN.get().is_some()
}

/// A chip as Hairline draws it: no fill. A surface fill (the ground, `raised`, a tint) just
/// goes, leaving what lies under it; a coloured one (the accent, needs you, danger) becomes the
/// colour of its words, bold, so the chosen or primary one still stands out. Unchanged
/// outside Hairline.
pub(in crate::client) fn plain(segs: Vec<Seg>) -> Vec<Seg> {
    let Some(ground) = PLAIN.get() else {
        return segs;
    };
    // Contrast is only known between RGB colours; a named one (the mono theme's) is a colour
    // the terminal picks, never a tint of the ground.
    let coloured = |c: Color| match (c, ground) {
        (Color::Reset, _) => false,
        (Color::Rgb(..), Color::Rgb(..)) => crate::theme::contrast(c, ground) >= SURFACE_CONTRAST,
        _ => c != ground,
    };
    segs.into_iter()
        .map(|(text, st)| match st.bg {
            Some(c) if coloured(c) => (
                text,
                Style { bg: None, ..st }.fg(c).add_modifier(Modifier::BOLD),
            ),
            _ => (text, Style { bg: None, ..st }),
        })
        .collect()
}
