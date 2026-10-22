//! The collapsed sidebar keeps agent state and session switching within reach.
use super::*;

pub(super) fn draw(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    let pb = pane_bg(t);
    let mut c = Card::new(t, "");
    if app.mode == Mode::Side {
        c = c.lit(t.accent);
    }
    card(app, buf, r, &c, t);
    app.screen.side_rect = r;
    hit(app, r, ScreenHit::SideFocus);
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
        ScreenHit::ExpandSidebar,
        expand.right(),
    );
    hit(app, expand, ScreenHit::ExpandSidebar);

    // Folded projects still expose their sessions and attention in the rail.
    let sessions: Vec<_> = model.iter().flat_map(Proj::sessions).collect();
    app.screen.visible = sessions.iter().map(|s| s.term).collect();
    app.screen.side_items = sessions.iter().map(|s| SideItem::Sess(s.term)).collect();
    app.screen.row_y.clear();
    app.screen.proj_keys = model.iter().map(|p| p.key.clone()).collect();
    let count = r.height.saturating_sub(5) as usize;
    let mut scroll = app.screen.side_scroll as usize;
    let focus = app.focused();
    if app.screen.follow {
        if let Some(i) = sessions
            .iter()
            .position(|s| Some(s.term) == app.screen.cursor.or(focus))
        {
            if i < scroll {
                scroll = i;
            } else if i >= scroll + count {
                scroll = i.saturating_sub(count.saturating_sub(1));
            }
        }
        app.screen.follow = false;
    }
    scroll = scroll.min(sessions.len().saturating_sub(count));
    app.screen.side_scroll = scroll as u16;
    for (i, s) in sessions.iter().skip(scroll).take(count).enumerate() {
        let rr = row(r.y + 3 + i as u16);
        let active = Some(s.term) == focus;
        let bg = if active {
            t.hov
        } else if app.mode == Mode::Side && app.screen.cursor == Some(s.term) {
            t.card2
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
        // The sidebar row's own mark; a plain shell keeps its prompt arrow.
        let spinner = glyph(app, Status::Working);
        let mark = check::mark(app, s, t, &spinner);
        let (icon, icon_color) = if super::side::plain_shell(s) { ("›", t.muted) } else { (mark.glyph, mark.color) };
        put(
            buf,
            rr.x,
            rr.y,
            &[
                seg(icon, style.fg(icon_color)),
                seg(format!(" {initials}"), style),
            ],
            rr.right(),
        );
        hit(app, rr, ScreenHit::Session(s.term));
        app.screen.row_y.insert(s.term, rr.y);
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
    hit(app, foot, ScreenHit::Actions);
}

pub(super) fn hint(app: &App, buf: &mut Buffer, panes: Rect, model: &[Proj], t: &Theme) {
    if app.sidebar || !matches!(app.mode, Mode::Normal | Mode::Side) {
        return;
    }
    let Some(pos) = app.hover.filter(|p| app.screen.side_rect.contains(*p)) else {
        return;
    };
    let Some((&term, _)) = app.screen.row_y.iter().find(|(_, y)| **y == pos.y) else {
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
