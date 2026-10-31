//! The collapsed sidebar: a narrow column down the side. A project is its first letter in the
//! project's colour; under it each session is its state mark and the first letter of its name.
use super::side::Line;
use super::*;

/// The column's width: a mark and a letter, a cell of room round each.
pub(in crate::client) const RAIL_W: u16 = 5;
/// Rows above the list: the expand chip, then a blank one.
const HEAD_ROWS: u16 = 2;
/// Rows below the list: a blank one, then the New and Menu chips.
const FOOT_ROWS: u16 = 3;

/// A row of the column.
enum Row {
    Head(usize),
    Sess(TermId),
    Gap,
}

/// The first letter of `name`, upper case; a dot for a name with none.
fn initial(name: &str) -> String {
    name.chars().find(|c| !c.is_whitespace()).map_or_else(|| "·".to_string(), |c| c.to_uppercase().collect())
}

/// A session's row: its mark and the first letter of its name. `on` is the selection, `lit`
/// when the column has the keys (ink on the accent), else a raised row.
fn session_segs(app: &App, s: &Session, on: bool, lit: bool, ground: Color, t: &Theme) -> Vec<Seg> {
    let spinner = glyph(app, Status::Working);
    let mark = check::mark(app, s, t, &spinner);
    let state = check::kind(s);
    let new = super::side::is_new(s);
    let (bg, ink) = match (on, lit) {
        (true, true) => (t.accent, Some(t.acc_ink)),
        (true, false) => (t.btn, None),
        _ => (ground, None),
    };
    let plain = super::side::plain_shell(s);
    let (glyph, mut colour) = if plain { ("›", t.muted) } else { (mark.glyph, mark.color) };
    if new {
        colour = t.accent;
    }
    let hot = super::side::is_hot(s, state, new);
    let st = Style::default().bg(bg);
    let letter_fg = if on || hot { t.strong } else if plain { t.muted } else { t.text };
    let bold = if on || hot { Modifier::BOLD } else { Modifier::empty() };
    vec![
        seg(" ", st),
        seg(glyph, st.fg(ink.unwrap_or(colour)).add_modifier(Modifier::BOLD)),
        seg(" ", st),
        seg(initial(&s.name), st.fg(ink.unwrap_or(letter_fg)).add_modifier(bold)),
        seg(" ", st),
    ]
}

/// A small chip (`»`, `+`, `≡`) on the raised ground, a cell in from the column's edge.
fn chip(app: &mut App, buf: &mut Buffer, area: Rect, y: u16, symbol: &str, hit_as: ScreenHit, t: &Theme) {
    let segs = vec![seg(format!(" {symbol} "), Style::default().fg(t.strong).bg(t.btn).add_modifier(Modifier::BOLD))];
    let x = area.x + 1;
    put(buf, x, y, &segs, area.right());
    hit(app, Rect { x, y, width: segs_width(&segs), height: 1 }, hit_as);
}

/// Draw the column in `area`. Sessions that don't fit scroll, the selected one kept in view,
/// and a `…` row in the last place says there are more (it opens the sidebar).
pub(super) fn draw(app: &mut App, buf: &mut Buffer, area: Rect, model: &[Proj], t: &Theme) {
    let ground = t.bg;
    fill(buf, area, ground);
    app.screen.side_rect = Rect::default();
    hit(app, area, ScreenHit::SideFocus);

    let lines = super::side::side_lines(app, model, t);
    let rows: Vec<Row> = lines
        .iter()
        .filter_map(|l| match l {
            Line::Proj(pi) => Some(Row::Head(*pi)),
            Line::Gap => Some(Row::Gap),
            other => super::side::line_term(model, other).map(Row::Sess),
        })
        .collect();
    let sessions: Vec<TermId> = rows.iter().filter_map(|r| if let Row::Sess(term) = r { Some(*term) } else { None }).collect();
    app.screen.visible = sessions.clone();
    app.screen.side_items = sessions.iter().map(|term| SideItem::Sess(*term)).collect();
    app.screen.row_y.clear();
    app.screen.proj_keys = model.iter().map(|p| p.key.clone()).collect();

    let lit = side_focused(app);
    let selected = if lit { app.screen.cursor.or(app.focused()) } else { app.focused() };

    let arrow = if app.cfg.ui.sidebar_position == "right" { "«" } else { "»" };
    chip(app, buf, area, area.y, arrow, ScreenHit::ExpandSidebar, t);
    if area.height > HEAD_ROWS + FOOT_ROWS {
        chip(app, buf, area, area.bottom() - 2, "+", ScreenHit::NewShell, t);
        chip(app, buf, area, area.bottom() - 1, "≡", ScreenHit::Actions, t);
    }

    let room = area.height.saturating_sub(HEAD_ROWS + FOOT_ROWS) as usize;
    let at = selected.and_then(|sel| rows.iter().position(|r| matches!(r, Row::Sess(term) if *term == sel)));
    let from = match at {
        Some(i) if i >= room => i + 1 - room,
        _ => 0,
    };
    let more_below = from + room < rows.len();
    for (n, row) in rows.iter().skip(from).take(room).enumerate() {
        let y = area.y + HEAD_ROWS + n as u16;
        let row_area = Rect { x: area.x, y, width: area.width, height: 1 };
        if more_below && n + 1 == room {
            put(buf, area.x + 1, y, &[seg(" … ", Style::default().fg(t.muted).bg(ground))], area.right());
            hit(app, row_area, ScreenHit::ExpandSidebar);
            break;
        }
        match row {
            Row::Gap => {}
            Row::Head(pi) => {
                let p = &model[*pi];
                put(buf, area.x + 1, y, &[seg(initial(&p.name), Style::default().fg(p.color).bg(ground).add_modifier(Modifier::BOLD))], area.right());
                hit(app, row_area, ScreenHit::ExpandSidebar);
            }
            Row::Sess(term) => {
                let Some((.., s)) = find(model, *term) else { continue };
                put(buf, area.x, y, &session_segs(app, s, Some(*term) == selected, lit, ground, t), area.right());
                hit(app, row_area, ScreenHit::Session(*term));
                app.screen.row_y.insert(*term, y);
            }
        }
    }
}
