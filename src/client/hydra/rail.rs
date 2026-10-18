//! The collapsed sidebar keeps agent state and session switching within reach.
use super::*;

pub(super) fn draw(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    let pb = pane_bg(t);
    let mut c = Card::new(t, "");
    if app.mode == Mode::Side {
        c = c.lit(t.accent);
    }
    card(app, buf, r, &c, t);
    app.hy.side_rect = r;
    hit(app, r, HyHit::SideFocus);
    let row = |y| Rect {
        x: r.x + 1,
        y,
        width: r.width.saturating_sub(2),
        height: 1,
    };
    let expand = row(r.y + 1);
    let arrow = if app.cfg.ui.sidebar_position == "right" {
        "«"
    } else {
        "»"
    };
    btn(
        app,
        buf,
        expand.x,
        expand.y,
        arrow,
        "",
        BtnKind::Ghost,
        HyHit::ExpandSidebar,
        expand.right(),
    );
    hit(app, expand, HyHit::ExpandSidebar);

    // Folded projects still expose their sessions and attention in the rail.
    let sessions: Vec<_> = model.iter().flat_map(Proj::sessions).collect();
    app.hy.visible = sessions.iter().map(|s| s.term).collect();
    app.hy.side_items = sessions.iter().map(|s| SideItem::Sess(s.term)).collect();
    app.hy.row_y.clear();
    app.hy.proj_keys = model.iter().map(|p| p.key.clone()).collect();
    let count = r.height.saturating_sub(5) as usize;
    let mut scroll = app.hy.side_scroll as usize;
    let focus = app.focused();
    if app.hy.follow {
        if let Some(i) = sessions
            .iter()
            .position(|s| Some(s.term) == app.hy.cursor.or(focus))
        {
            if i < scroll {
                scroll = i;
            } else if i >= scroll + count {
                scroll = i.saturating_sub(count.saturating_sub(1));
            }
        }
        app.hy.follow = false;
    }
    scroll = scroll.min(sessions.len().saturating_sub(count));
    app.hy.side_scroll = scroll as u16;
    for (i, s) in sessions.iter().skip(scroll).take(count).enumerate() {
        let rr = row(r.y + 3 + i as u16);
        let active = Some(s.term) == focus;
        let bg = if active {
            t.hov
        } else if app.mode == Mode::Side && app.hy.cursor == Some(s.term) {
            t.card2
        } else if hovered(app, rr) {
            blend(pb, t.hov, 0.4)
        } else {
            pb
        };
        fill(buf, rr, bg);
        let color = state_color(t, s.status);
        let label_color = if matches!(s.status, Status::Blocked | Status::Working | Status::Done) {
            color
        } else if active {
            t.strong
        } else {
            t.text
        };
        let style = Style::default().bg(bg).fg(label_color).add_modifier(
            if active || s.status == Status::Blocked {
                Modifier::BOLD
            } else {
                Modifier::empty()
            },
        );
        let initials: String = s
            .name
            .chars()
            .filter(|c| !c.is_whitespace())
            .take(2)
            .collect();
        let icon = if s.asleep {
            "z".into()
        } else if s.is_agent {
            glyph(app, s.status)
        } else {
            "›".into()
        };
        put(
            buf,
            rr.x,
            rr.y,
            &[
                seg(icon, style.fg(color)),
                seg(format!(" {initials}"), style),
            ],
            rr.right(),
        );
        hit(app, rr, HyHit::Session(s.term));
        app.hy.row_y.insert(s.term, rr.y);
    }
    let foot = row(r.bottom().saturating_sub(2));
    let blocked = sessions
        .iter()
        .filter(|s| s.status == Status::Blocked)
        .count();
    let (text, color) = if blocked > 0 {
        (format!("●{blocked}"), t.blocked)
    } else {
        (format!("{}", sessions.len()), t.muted)
    };
    put(
        buf,
        foot.x,
        foot.y,
        &[seg(text, Style::default().fg(color).bg(pb))],
        foot.right(),
    );
    hit(app, foot, HyHit::Actions);
}

pub(super) fn hint(app: &App, buf: &mut Buffer, panes: Rect, model: &[Proj], t: &Theme) {
    if app.sidebar || !matches!(app.mode, Mode::Normal | Mode::Side) {
        return;
    }
    let Some(pos) = app.hover.filter(|p| app.hy.side_rect.contains(*p)) else {
        return;
    };
    let Some((&term, _)) = app.hy.row_y.iter().find(|(_, y)| **y == pos.y) else {
        return;
    };
    let Some((p, _, s)) = find(model, term) else {
        return;
    };
    let label = format!(" {} · {} ", p.name, s.name);
    let rr = Rect {
        x: panes.x,
        y: panes.y,
        width: panes
            .width
            .min(segs_width(&[seg(&label, Style::default())])),
        height: 1,
    };
    fill(buf, rr, t.card2);
    put(
        buf,
        rr.x,
        rr.y,
        &[seg(label, Style::default().fg(t.text).bg(t.card2))],
        rr.right(),
    );
}
