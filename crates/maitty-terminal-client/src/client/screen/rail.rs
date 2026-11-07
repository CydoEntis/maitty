//! The collapsed sidebar: a 10-column box down the side. A `»` chip on the top border, the
//! inbox count, then the fleets (FLT) and the agent ships (SHP), each half under a divider and
//! one line per row, and the `+` and `≡` chips at the foot. Rows go by initials.
use super::chips::{Chip, chip};
use super::fleets::Fleet;
use super::side::{Counts, FlatRow, ShipLine, SideModel, TopRow, divider, scroll_marks, window};
use super::status_mark::State;
use super::*;

/// The column's width: the box's two edges and eight cells inside.
pub(in crate::client) const RAIL_W: u16 = 10;
/// The screen width below which the sidebar collapses by itself, and the full one opens over
/// the workspace instead of beside it.
pub(in crate::client) const COLLAPSE_BELOW: u16 = 110;
/// Inside rows above the lists: the inbox count, then the FLT divider.
const HEAD_ROWS: u16 = 2;
/// Inside rows below the lists: the rule over the footer, then the `+` and `≡` chips.
const FOOT_ROWS: u16 = 3;
/// The mark before the current row.
const BAR: &str = "▌";
const UNREAD: &str = "•";
const DIAMOND: &str = "◆";
const OFFLINE: &str = "┆";

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

/// The ground of a row: selected, current or plain.
fn ground_of((current, picked): (bool, bool), t: &Theme) -> Color {
    if picked {
        t.sel_bg()
    } else if current {
        t.card
    } else {
        t.bg
    }
}

/// A fleet's line in the column: its needs count, a diamond in its colour or the offline mark,
/// then its initials.
fn fleet_segs(f: &Fleet, counts: Counts, down: bool, flags: (bool, bool), t: &Theme) -> Vec<Seg> {
    let current = flags.0;
    let st = Style::default().bg(ground_of(flags, t));
    let icon = if counts.needs > 0 {
        seg(
            counts.needs.min(9).to_string(),
            st.fg(t.blocked).add_modifier(Modifier::BOLD),
        )
    } else if down {
        seg(OFFLINE, st.fg(t.idle))
    } else {
        seg(DIAMOND, st.fg(f.colour.unwrap_or(t.muted)))
    };
    let ink = if down { t.idle } else { t.strong };
    vec![
        if current {
            seg(BAR, st.fg(t.accent))
        } else {
            seg(" ", st)
        },
        seg(" ", st),
        icon,
        seg(" ", st),
        seg(initials(&f.name), st.fg(ink).add_modifier(Modifier::BOLD)),
        seg("   ", st),
    ]
}

/// An agent ship's line in the column.
fn ship_segs(app: &App, row: &FlatRow, flags: (bool, bool), t: &Theme) -> Vec<Seg> {
    let unknown = row.state == State::Unknown;
    let current = flags.0;
    let st = Style::default().bg(ground_of(flags, t));
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

/// Draw the column in `area`. Rows that don't fit scroll, the current one kept in view.
pub(super) fn draw(app: &mut App, buf: &mut Buffer, area: Rect, model: &[Proj], t: &Theme) {
    fill(buf, area, t.bg);
    app.screen.side_rect = Rect::default();
    hit(app, area, ScreenHit::SideFocus);
    let look = Look::of(&app.cfg.ui);
    let lit = side_focused(app);
    let border = if lit { t.accent } else { t.line };
    panel_box(buf, area, look, Style::default().fg(border).bg(t.bg));
    if area.width < RAIL_W || area.height < HEAD_ROWS + FOOT_ROWS + 5 {
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

    let sm = SideModel::build(app, model);
    let lists = sm.lists(app, true);
    let needing: usize = sm.counts.values().map(|c| c.needs).sum();
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
    let rule_y = area.bottom() - FOOT_ROWS - 1;
    rule(buf, area, rule_y, border, t.bg);
    let plus = chip(app, "+", Chip::Action, t.bg, t);
    centred(app, buf, area, area.bottom() - 3, plus, ScreenHit::NewShell);
    let menu = chip(app, "≡", Chip::Quiet, t.bg, t);
    centred(app, buf, area, area.bottom() - 2, menu, ScreenHit::Actions);

    // FLT above, SHP at the middle of what is left.
    let flt = inner.y + 1;
    let last = rule_y.saturating_sub(1);
    let mid = (flt + last) / 2;
    divider(app, buf, area, flt, "FLT", border, t);
    divider(app, buf, area, mid, "SHP", border, t);
    app.screen.mid_y = mid;
    app.screen.side_items = lists.items();
    app.screen.visible = sm.rows.iter().map(|r| r.key).collect();
    app.screen.row_y.clear();

    let want = if app.screen.follow {
        let picked = if lit { app.screen.cursor } else { None };
        picked
            .or(app.focused().map(|id| app.session_key(id)))
            .map(SideItem::Sess)
    } else {
        None
    };
    let picked_item = lit
        .then(|| app.cursor_head().or(app.screen.cursor.map(SideItem::Sess)))
        .flatten();
    let current_fleet = app.current_fleet();
    let current = app.focused().map(|id| app.session_key(id));

    // The fleets.
    let (top, bottom) = (flt + 1, mid);
    let room = bottom.saturating_sub(top);
    let heights = vec![1u16; lists.top.len()];
    let want_fleet = app
        .screen
        .follow
        .then(|| current_fleet.map(SideItem::Fleet))
        .flatten();
    let want_at = want_fleet
        .as_ref()
        .and_then(|w| lists.top.iter().position(|(_, i)| i == w));
    let (first, shown) = window(&heights, app.screen.side_scroll as usize, want_at, room);
    app.screen.side_scroll = first as u16;
    for (n, (row, item)) in lists.top.iter().enumerate().skip(first).take(shown) {
        let y = top + (n - first) as u16;
        let TopRow::Fleet(gi, fi) = *row else {
            continue;
        };
        let f = sm.fleet(gi, fi);
        let flags = (current_fleet == Some(f.id), picked_item.as_ref() == Some(item));
        let segs = fleet_segs(f, sm.counts_of(f.id), sm.groups[gi].down, flags, t);
        put(buf, inner.x, y, &segs, inner.right());
        hit(
            app,
            Rect {
                y,
                height: 1,
                ..inner
            },
            ScreenHit::SideRow(n),
        );
    }
    scroll_marks(
        buf,
        inner.right().saturating_sub(1),
        (top, room),
        (first > 0, first + shown < lists.top.len()),
        Style::default().bg(t.bg).fg(t.muted),
    );

    // The ships.
    let (top, bottom) = (mid + 1, rule_y);
    let room = bottom.saturating_sub(top);
    let heights = vec![1u16; lists.ships.len()];
    let want_at = want
        .as_ref()
        .and_then(|w| lists.ships.iter().position(|(_, i)| i == w));
    let (first, shown) = window(&heights, app.screen.ships_scroll as usize, want_at, room);
    app.screen.ships_scroll = first as u16;
    for (n, (line, item)) in lists.ships.iter().enumerate().skip(first).take(shown) {
        let y = top + (n - first) as u16;
        let ShipLine::Ship(i) = *line else {
            continue;
        };
        let row = &sm.rows[i];
        let flags = (current == Some(row.key), picked_item.as_ref() == Some(item));
        let segs = ship_segs(app, row, flags, t);
        put(buf, inner.x, y, &segs, inner.right());
        hit(
            app,
            Rect {
                y,
                height: 1,
                ..inner
            },
            ScreenHit::Session(row.key),
        );
        if let Some(term) = row.term {
            app.screen.row_y.insert(term, y);
        }
    }
    scroll_marks(
        buf,
        inner.right().saturating_sub(1),
        (top, room),
        (first > 0, first + shown < lists.ships.len()),
        Style::default().bg(t.bg).fg(t.muted),
    );
    app.screen.follow = false;
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
