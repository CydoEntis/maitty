//! Hairline (ADR 0026): the workspace with no boxes. A sidebar of one-line sessions beside a
//! `│`, each pane a title row over a `─` rule whose colour is its state, `│` / `─` between
//! panes, and a status strip over the mode bar. Always square, whatever `ui.rounded` says.
//!
//! Drawing that has a Hairline variant asks `Look::of(..).hairline` once and comes here; the
//! colours come from `Ink`, the handoff's tokens mapped onto the theme's roles.

use super::check;
use super::glyphs;
use super::*;

mod pane;
mod side;
mod strip;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod surface_tests;

pub(in crate::client) use self::{pane::pane_frame, side::draw_side, strip::draw_strip};

/// The sidebar's width before you drag it, in columns; its `│` is one more.
pub(in crate::client) const SIDE_W: u16 = 28;

/// Columns of space each side of a pane's title and terminal.
pub(in crate::client) const PANE_PAD: u16 = 2;

/// The working mark with motion off: a still braille block, not a frozen spinner frame.
const STILL_WORKING: &str = "⠿";

/// The handoff's colour tokens, each one of the theme's roles, so every theme draws Hairline.
#[derive(Debug, Clone, Copy)]
pub(in crate::client) struct Ink {
    pub bg: Color,
    /// A focused or visible sidebar row.
    pub raised: Color,
    /// Every `│` and `─`, and a pane's rule when nothing lights it.
    pub line: Color,
    pub text: Color,
    pub strong: Color,
    /// Project names, meta, hints and idle.
    pub dim: Color,
    /// Focus, working and the mode.
    pub teal: Color,
    /// Needs you, and nothing else.
    pub amber: Color,
    pub green: Color,
}

impl Ink {
    pub fn of(t: &Theme) -> Ink {
        Ink { bg: t.bg, raised: t.card2, line: t.line, text: t.text, strong: t.strong, dim: t.muted, teal: t.accent, amber: t.blocked, green: t.done }
    }

    /// A state's colour: working teal, needs you amber, done green, anything else dim.
    pub fn of_status(self, status: Status) -> Color {
        match status {
            Status::Working => self.teal,
            Status::Blocked => self.amber,
            Status::Done => self.green,
            _ => self.dim,
        }
    }

    /// A sidebar session's colour: amber whenever it needs you, a check to approve included.
    fn of_session(self, s: &Session) -> Color {
        if check::needs_you(s) { self.amber } else { self.of_status(s.status) }
    }
}

/// The working mark: the chosen spinner while motion is on (or in the ASCII set), else `⠿`.
fn working_glyph(app: &App) -> String {
    let ascii = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons) == &glyphs::ASCII;
    if app.motion_on() || ascii { glyph(app, Status::Working) } else { STILL_WORKING.into() }
}

/// The sessions in sidebar order: the rows as the sidebar lists them, then those of folded
/// projects. The strip draws its segments in this order.
fn session_order(app: &App, model: &[Proj], t: &Theme) -> Vec<TermId> {
    let mut order: Vec<TermId> = side_lines(app, model, t).iter().filter_map(|l| line_term(model, l)).collect();
    for s in model.iter().flat_map(Proj::sessions) {
        if !order.contains(&s.term) {
            order.push(s.term);
        }
    }
    order
}

/// A one-cell-wide `│` down `r`'s first column, in `colour`.
fn vline(buf: &mut Buffer, r: Rect, colour: Color, bg: Color) {
    for y in r.top()..r.bottom() {
        if let Some(px) = buf.cell_mut((r.x, y)) {
            px.set_symbol("│").set_style(Style::default().fg(colour).bg(bg));
        }
    }
}

/// A `─` rule `w` wide from (`x`, `y`), in `colour`.
fn rule(buf: &mut Buffer, x: u16, y: u16, w: u16, colour: Color, bg: Color) {
    for i in 0..w {
        if let Some(px) = buf.cell_mut((x + i, y)) {
            px.set_symbol("─").set_style(Style::default().fg(colour).bg(bg));
        }
    }
}

/// The `│` between the sidebar and the panes, on the column you drag to resize it.
pub(in crate::client) fn draw_side_rule(buf: &mut Buffer, edge: Rect, t: &Theme) {
    vline(buf, Rect { width: 1, ..edge }, t.line, t.bg);
}

/// A pane needs you: its own agent asks, or its own check waits for approval. The sidebar's
/// rows use the same rule (`check::needs`).
fn pane_needs(info: &TermInfo) -> bool {
    check::needs(info.status, info.check())
}

/// How many panes need you, a split's included: the right end of the mode bar says so.
pub(in crate::client) fn needing(app: &App) -> usize {
    app.snap.terms.values().filter(|info| pane_needs(info)).count()
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
    let Some(ground) = PLAIN.get() else { return segs };
    let coloured = |c: Color| c != Color::Reset && crate::theme::contrast(c, ground) >= SURFACE_CONTRAST;
    segs.into_iter()
        .map(|(text, st)| match st.bg {
            Some(c) if coloured(c) => (text, Style { bg: None, ..st }.fg(c).add_modifier(Modifier::BOLD)),
            _ => (text, Style { bg: None, ..st }),
        })
        .collect()
}
