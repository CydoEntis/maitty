//! The collapsed sidebar: a 5-column box down the side. At the top a `»` chip and the count of
//! sessions that need you; then this computer's mark and one state glyph per session, a `─`
//! between projects and a rule where the other machines begin; at the foot the `+` and `≡` chips.
use super::side::Line;
use super::*;
use crate::client::ids::ClientSession;

/// The column's width: the box's two edges and three cells inside, one glyph in the middle.
pub(in crate::client) const RAIL_W: u16 = 5;
/// Inside rows above the list: the count of sessions that need you, then a blank one.
const HEAD_ROWS: u16 = 2;
/// Inside rows below the list: the `+` and `≡` chips.
const FOOT_ROWS: u16 = 2;

/// A row of the column.
enum Row {
    /// A machine's mark: `⌂` for this computer, `⇄` and its state's colour for another.
    Machine(MachineId),
    /// Between two projects.
    Divider,
    /// Where the other machines begin.
    Split,
    Sess(ClientSession),
}

/// A session's row: its state glyph in the middle of three cells. `on` is the selection, which
/// has a tinted ground and, while the column has the keys (`lit`), the accent bar at the left.
fn session_segs(app: &App, s: &Session, on: bool, lit: bool, ground: Color, t: &Theme) -> Vec<Seg> {
    let spinner = super::machine_group::working_mark(app, s.down);
    let mark = check::mark(app, s, t, &spinner);
    let plain = super::side::plain_shell(s);
    let (glyph, colour) = if plain {
        ("›", t.muted)
    } else {
        (mark.glyph, mark.color)
    };
    // With `ui.rounded = full` the selection is the pill the open sidebar's rows wear, the glyph
    // between its two caps, and no bar.
    if on && Look::of(&app.cfg.ui).caps {
        let st = Style::default().bg(t.sel_bg());
        return round_ends(
            vec![seg(glyph, st.fg(colour).add_modifier(Modifier::BOLD))],
            ground,
        );
    }
    let bg = if on { t.card2 } else { ground };
    let st = Style::default().bg(bg);
    vec![
        if on && lit {
            seg("▌", Style::default().fg(t.accent).bg(bg))
        } else {
            seg(" ", st)
        },
        seg(glyph, st.fg(colour).add_modifier(Modifier::BOLD)),
        seg(" ", st),
    ]
}

/// What fills the three cells inside the box as a chip: `text` on the raised ground. With
/// `ui.rounded = full` one character is a pill, between two caps on `ground`; anything wider
/// has no cell to spare for them and stays square.
fn chip_segs(app: &App, text: &str, style: Style, ground: Color) -> Vec<Seg> {
    if Look::of(&app.cfg.ui).caps && text.width() == 1 {
        round_ends(vec![seg(text, style)], ground)
    } else {
        vec![seg(format!("{text:^3}"), style)]
    }
}

/// A small chip (`»`, `+`, `≡`) filling the three cells inside the box, on the raised ground.
#[allow(clippy::too_many_arguments)]
fn chip(
    app: &mut App,
    buf: &mut Buffer,
    inner: Rect,
    y: u16,
    symbol: &str,
    hit_as: ScreenHit,
    ground: Color,
    t: &Theme,
) {
    let style = Style::default()
        .fg(t.strong)
        .bg(t.btn)
        .add_modifier(Modifier::BOLD);
    let segs = chip_segs(app, symbol, style, ground);
    put(buf, inner.x, y, &segs, inner.right());
    hit(
        app,
        Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        },
        hit_as,
    );
}

/// Draw the column in `area`. Sessions that don't fit scroll, the selected one kept in view,
/// and a `…` row in the last place says there are more (it opens the sidebar).
pub(super) fn draw(app: &mut App, buf: &mut Buffer, area: Rect, model: &[Proj], t: &Theme) {
    let ground = t.bg;
    fill(buf, area, ground);
    app.screen.side_rect = Rect::default();
    hit(app, area, ScreenHit::SideFocus);
    let look = Look::of(&app.cfg.ui);
    let lit = side_focused(app);
    panel_box(
        buf,
        area,
        look,
        Style::default()
            .fg(if lit { t.accent } else { t.line })
            .bg(ground),
    );
    if area.width < 5 || area.height < HEAD_ROWS + FOOT_ROWS + 3 {
        return;
    }
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width - 2,
        height: area.height - 2,
    };

    let lines = super::side::side_lines(app, model, t);
    let mut rows: Vec<Row> = vec![Row::Machine(MachineId::LOCAL)];
    for l in &lines {
        match l {
            Line::Proj(_) if matches!(rows.last(), Some(Row::Sess(_))) => rows.push(Row::Divider),
            Line::Machine(machine) if *machine != MachineId::LOCAL => {
                rows.push(Row::Split);
                rows.push(Row::Machine(*machine));
            }
            other => {
                if let Some(key) = super::side::line_term(model, other) {
                    rows.push(Row::Sess(key));
                }
            }
        }
    }
    let sessions: Vec<ClientSession> = rows
        .iter()
        .filter_map(|r| if let Row::Sess(key) = r { Some(*key) } else { None })
        .collect();
    app.screen.visible = sessions.clone();
    app.screen.side_items = sessions.iter().map(|key| SideItem::Sess(*key)).collect();
    app.screen.row_y.clear();
    app.screen.proj_keys = model.iter().map(|p| p.key.clone()).collect();

    let selected = if lit {
        app.screen
            .cursor
            .or(app.focused().map(|id| app.session_key(id)))
    } else {
        app.focused().map(|id| app.session_key(id))
    };

    // The top border: the chip that brings the sidebar back, set into it.
    let arrow = if app.cfg.ui.sidebar_position == "right" {
        "«"
    } else {
        "»"
    };
    chip(
        app,
        buf,
        inner,
        area.y,
        arrow,
        ScreenHit::ExpandSidebar,
        ground,
        t,
    );
    // How many need you, in amber, under it.
    let needing = model
        .iter()
        .flat_map(|p| p.sessions())
        .filter(|s| check::needs_you(s))
        .count();
    if needing > 0 {
        let count = chip_segs(
            app,
            &needing.to_string(),
            Style::default()
                .fg(t.blocked)
                .bg(t.btn)
                .add_modifier(Modifier::BOLD),
            ground,
        );
        put(buf, inner.x, inner.y, &count, inner.right());
        hit(
            app,
            Rect {
                y: inner.y,
                height: 1,
                ..inner
            },
            ScreenHit::ExpandSidebar,
        );
    }
    chip(
        app,
        buf,
        inner,
        inner.bottom() - 2,
        "+",
        ScreenHit::NewShell,
        ground,
        t,
    );
    chip(
        app,
        buf,
        inner,
        inner.bottom() - 1,
        "≡",
        ScreenHit::Actions,
        ground,
        t,
    );

    let room = inner.height.saturating_sub(HEAD_ROWS + FOOT_ROWS) as usize;
    let at = selected.and_then(|sel| {
        rows.iter()
            .position(|r| matches!(r, Row::Sess(term) if *term == sel))
    });
    let from = match at {
        Some(i) if i >= room => i + 1 - room,
        _ => 0,
    };
    let more_below = from + room < rows.len();
    for (n, row) in rows.iter().skip(from).take(room).enumerate() {
        let y = inner.y + HEAD_ROWS + n as u16;
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
                &[seg(" … ", Style::default().fg(t.muted).bg(ground))],
                inner.right(),
            );
            hit(app, row_area, ScreenHit::ExpandSidebar);
            break;
        }
        match row {
            Row::Divider => {
                put(
                    buf,
                    inner.x,
                    y,
                    &[seg(" ─ ", Style::default().fg(t.line).bg(ground))],
                    inner.right(),
                );
            }
            Row::Split => {
                // The rule between this computer's half and the others', joined to the box.
                let rule = Style::default()
                    .fg(if lit { t.accent } else { t.line })
                    .bg(ground);
                put(
                    buf,
                    area.x,
                    y,
                    &[seg(format!("├{}┤", "─".repeat(inner.width as usize)), rule)],
                    area.right(),
                );
            }
            Row::Machine(machine) => {
                let (glyph, colour) = if *machine == MachineId::LOCAL {
                    (super::machine_group::LOCAL_ICON, t.muted)
                } else {
                    let (_, colour) = app.machine_dot(*machine, t);
                    (
                        super::machine_group::REMOTE_ICON,
                        if colour == t.done { t.muted } else { t.line },
                    )
                };
                put(
                    buf,
                    inner.x,
                    y,
                    &[seg(
                        format!(" {glyph} "),
                        Style::default().fg(colour).bg(ground),
                    )],
                    inner.right(),
                );
                hit(app, row_area, ScreenHit::ExpandSidebar);
            }
            Row::Sess(key) => {
                let Some((.., s)) = find_key(model, *key) else {
                    continue;
                };
                put(
                    buf,
                    inner.x,
                    y,
                    &session_segs(app, s, Some(*key) == selected, lit, ground, t),
                    inner.right(),
                );
                hit(app, row_area, ScreenHit::Session(*key));
                if s.down {
                    super::machine_group::grey(buf, row_area, t);
                }
                if let Some(term) = s.native_term {
                    app.screen.row_y.insert(term, y);
                }
            }
        }
    }
}
