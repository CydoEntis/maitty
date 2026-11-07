//! The collapsed sidebar: a 10-column box down the side. A `»` chip on the top border, the
//! inbox count, then one line per session (its status, the initials of its name and a dot
//! when it has something unread), and the `+` and `≡` chips at the foot.
use super::chips::{Chip, chip};
use super::side::{FlatRow, shown_rows};
use super::status_mark::State;
use super::*;

/// The column's width: the box's two edges and eight cells inside.
pub(in crate::client) const RAIL_W: u16 = 10;
/// The screen width below which the sidebar collapses by itself, and the full one opens over
/// the workspace instead of beside it.
pub(in crate::client) const COLLAPSE_BELOW: u16 = 110;
/// Inside rows above the list: the inbox count, then the rule under it.
const HEAD_ROWS: u16 = 2;
/// Inside rows below the list: the rule over the footer, then the `+` and `≡` chips.
const FOOT_ROWS: u16 = 3;
/// The mark before the current session's row.
const BAR: &str = "▌";
const UNREAD: &str = "•";

/// The two letters a session goes by: the first letters of its first two words, or the first
/// two letters of a single word.
pub(super) fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let letters: Vec<char> = match words.as_slice() {
        [] => return "··".to_string(),
        [one] => one.chars().take(2).collect(),
        [first, second, ..] => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
    };
    letters.iter().flat_map(|c| c.to_uppercase()).collect()
}

/// A session's line in the column.
fn row_segs(app: &App, row: &FlatRow, (current, picked): (bool, bool), t: &Theme) -> Vec<Seg> {
    let unknown = row.state == State::Unknown;
    let ground = if picked {
        t.sel_bg()
    } else if current {
        t.card
    } else {
        t.bg
    };
    let st = Style::default().bg(ground);
    let mark = app.state_mark(row.state, t);
    let ink = if unknown { t.idle } else { t.strong };
    vec![
        if current {
            seg(BAR, st.fg(t.accent))
        } else {
            seg(" ", st)
        },
        seg(" ", st),
        seg(mark.glyph, st.fg(mark.color)),
        seg(" ", st),
        seg(initials(&row.name), st.fg(ink).add_modifier(Modifier::BOLD)),
        if row.unread {
            seg(UNREAD, st.fg(t.accent))
        } else {
            seg(" ", st)
        },
        seg(" ", st),
    ]
}

/// A chip centred on row `y` of `area`, with its click target.
fn centred(app: &mut App, buf: &mut Buffer, area: Rect, y: u16, segs: Vec<Seg>, h: ScreenHit) {
    let w = segs_width(&segs);
    let x = area.x + area.width.saturating_sub(w) / 2;
    put(buf, x, y, &segs, area.right());
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

/// A rule across the box at `y`, joined to its sides.
fn rule(buf: &mut Buffer, area: Rect, y: u16, colour: Color, ground: Color) {
    let line = format!("├{}┤", "─".repeat(area.width.saturating_sub(2) as usize));
    put(
        buf,
        area.x,
        y,
        &[seg(line, Style::default().fg(colour).bg(ground))],
        area.right(),
    );
}

/// Draw the column in `area`. Sessions that don't fit scroll, the current one kept in view,
/// and a `…` row in the last place says there are more (it opens the sidebar).
pub(super) fn draw(app: &mut App, buf: &mut Buffer, area: Rect, model: &[Proj], t: &Theme) {
    fill(buf, area, t.bg);
    app.screen.side_rect = Rect::default();
    hit(app, area, ScreenHit::SideFocus);
    let look = Look::of(&app.cfg.ui);
    let lit = side_focused(app);
    let border = if lit { t.accent } else { t.line };
    panel_box(buf, area, look, Style::default().fg(border).bg(t.bg));
    if area.width < RAIL_W || area.height < HEAD_ROWS + FOOT_ROWS + 4 {
        return;
    }
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width - 2,
        height: area.height - 2,
    };

    // The top border: the chip that brings the sidebar back.
    let arrow = if app.cfg.ui.sidebar_position == "right" {
        "«"
    } else {
        "»"
    };
    let expand = chip(app, arrow, Chip::Action, t.bg, t);
    centred(app, buf, area, area.y, expand, ScreenHit::ExpandSidebar);

    // The inbox count, amber while anything needs you.
    let needing = model
        .iter()
        .flat_map(|p| p.sessions())
        .filter(|s| check::needs_you(s))
        .count();
    let count = chip(
        app,
        &super::inbox::inbox_count(app).to_string(),
        if needing > 0 {
            Chip::Alert
        } else {
            Chip::Quiet
        },
        t.bg,
        t,
    );
    centred(app, buf, area, inner.y, count, ScreenHit::OpenInbox);
    rule(buf, area, inner.y + 1, border, t.bg);
    rule(buf, area, area.bottom() - FOOT_ROWS - 1, border, t.bg);
    let plus = chip(app, "+", Chip::Action, t.bg, t);
    centred(app, buf, area, area.bottom() - 3, plus, ScreenHit::NewShell);
    let menu = chip(app, "≡", Chip::Quiet, t.bg, t);
    centred(app, buf, area, area.bottom() - 2, menu, ScreenHit::Actions);

    // The sessions.
    let rows = shown_rows(app, model);
    let keys: Vec<ClientSession> = rows.iter().map(|r| r.key).collect();
    app.screen.visible = keys.clone();
    app.screen.side_items = keys.iter().map(|k| SideItem::Sess(*k)).collect();
    app.screen.row_y.clear();
    let current = app.focused().map(|id| app.session_key(id));
    let picked = if lit { app.screen.cursor } else { None };
    let top = inner.y + HEAD_ROWS;
    let room = (area.bottom() - FOOT_ROWS - 1).saturating_sub(top) as usize;
    let at = picked
        .or(current)
        .and_then(|k| keys.iter().position(|x| *x == k));
    let from = match at {
        Some(i) if i >= room => i + 1 - room,
        _ => 0,
    };
    let more_below = from + room < rows.len();
    for (n, row) in rows.iter().skip(from).take(room).enumerate() {
        let y = top + n as u16;
        let row_area = Rect {
            y,
            height: 1,
            ..inner
        };
        if more_below && n + 1 == room {
            put(
                buf,
                inner.x,
                y,
                &[seg("   …    ", Style::default().fg(t.muted).bg(t.bg))],
                inner.right(),
            );
            hit(app, row_area, ScreenHit::ExpandSidebar);
            break;
        }
        let segs = row_segs(
            app,
            row,
            (current == Some(row.key), picked == Some(row.key)),
            t,
        );
        put(buf, inner.x, y, &segs, inner.right());
        hit(app, row_area, ScreenHit::Session(row.key));
        if let Some(term) = row.term {
            app.screen.row_y.insert(term, y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::initials;

    #[test]
    fn initials_are_two_words_first_letters_or_a_single_words_first_two() {
        assert_eq!(initials("Growth Tests"), "GT");
        assert_eq!(initials("pacman"), "PA");
        assert_eq!(initials("fix-footer now"), "FF");
        assert_eq!(initials("  "), "··");
        assert_eq!(initials("x"), "X");
    }
}
