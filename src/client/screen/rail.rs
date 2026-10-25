//! The collapsed sidebar: the sessions as chips along row 0 (v8 `drawStrip`,
//! `docs/design/v8/cleanup/app8.js`). The panes keep the full width and their tabs sit on row 1.
use super::*;

/// The longest a session's name is drawn on its chip.
const CHIP_NAME_MAX: usize = 16;

/// A session's chip: its mark and name. `on` is the strip's selection, `lit` when the strip has
/// the keys (ink on the accent), else a raised chip.
fn chip_segs(app: &App, s: &Session, on: bool, lit: bool, ground: Color, t: &Theme) -> Vec<Seg> {
    let spinner = glyph(app, Status::Working);
    let mark = check::mark(app, s, t, &spinner);
    let state = check::kind(s);
    let new = super::side::is_new(s);
    let (bg, ink) = match (on, lit) {
        (true, true) => (t.accent, Some(t.acc_ink)),
        (true, false) => (t.btn, None),
        _ => (ground, None),
    };
    let (glyph, mut colour) = if super::side::plain_shell(s) { ("›", t.muted) } else { (mark.glyph, mark.color) };
    if new {
        colour = t.accent;
    }
    let hot = super::side::is_hot(s, state, new);
    let st = Style::default().bg(bg);
    let name_fg = if on || hot { t.strong } else { t.text };
    let bold = if on || hot { Modifier::BOLD } else { Modifier::empty() };
    vec![
        seg(" ", st),
        seg(glyph, st.fg(ink.unwrap_or(colour)).add_modifier(Modifier::BOLD)),
        seg(" ", st),
        seg(format!("{} ", truncate(&s.name, CHIP_NAME_MAX)), st.fg(ink.unwrap_or(name_fg)).add_modifier(bold)),
    ]
}

/// Draw the strip on row 0 of `area`. Chips that don't fit give way to a `+n more` chip, and
/// the selected one is always kept in view.
pub(super) fn draw(app: &mut App, buf: &mut Buffer, area: Rect, model: &[Proj], t: &Theme) {
    let look = Look::of(&app.cfg.ui);
    let ground = if look.flat { t.card } else { t.bg };
    let chip_bg = if look.flat { t.card2 } else { t.card };
    let row = Rect { height: 1, ..area };
    fill(buf, row, ground);
    app.screen.side_rect = Rect::default();
    hit(app, row, ScreenHit::SideFocus);

    let lines = super::side::side_lines(app, model, t);
    let sessions: Vec<&Session> = lines.iter().filter_map(|l| super::side::line_term(model, l)).filter_map(|term| find(model, term).map(|(.., s)| s)).collect();
    app.screen.visible = sessions.iter().map(|s| s.term).collect();
    app.screen.side_items = sessions.iter().map(|s| SideItem::Sess(s.term)).collect();
    app.screen.row_y.clear();
    app.screen.proj_keys = model.iter().map(|p| p.key.clone()).collect();

    let lit = side_focused(app);
    let selected = if lit { app.screen.cursor.or(app.focused()) } else { app.focused() };
    let round = |segs: Vec<Seg>| if look.round_tabs { round_ends(segs, ground) } else { segs };

    // The expand chip.
    let arrow = if app.cfg.ui.sidebar_position == "right" { "«" } else { "»" };
    let expand = round(vec![seg(format!(" {arrow} "), Style::default().fg(t.strong).bg(t.btn).add_modifier(Modifier::BOLD))]);
    let x0 = area.x + 1;
    let ew = segs_width(&expand);
    put(buf, x0, area.y, &expand, area.right());
    hit(app, Rect { x: x0, y: area.y, width: ew, height: 1 }, ScreenHit::ExpandSidebar);
    let start_x = x0 + ew + 1;

    // The Menu keeps a chip at the right, so mouse access to it doesn't need the sidebar open.
    let menu = super::side::word_chip(app, None, "Menu", t, ground);
    let mw = segs_width(&menu);
    let menu_x = area.right().saturating_sub(mw + 1);
    let limit = menu_x.saturating_sub(1);
    if menu_x > start_x + 4 {
        put(buf, menu_x, area.y, &menu, area.right());
        hit(app, Rect { x: menu_x, y: area.y, width: mw, height: 1 }, ScreenHit::Actions);
    }

    let chips: Vec<(TermId, Vec<Seg>)> = sessions
        .iter()
        .map(|s| (s.term, round(chip_segs(app, s, Some(s.term) == selected, lit, chip_bg, t))))
        .collect();
    let needs_mark = check::marks(app).needs;
    let more_for = |hidden: &[&Session]| -> Vec<Seg> {
        let needs = hidden.iter().filter(|s| check::needs_you(s)).count();
        let st = Style::default().bg(chip_bg);
        let mut segs = vec![seg(format!(" +{} more ", hidden.len()), st.fg(t.muted))];
        if needs > 0 {
            segs.push(seg(format!("{}{needs} ", needs_mark), st.fg(t.blocked).add_modifier(Modifier::BOLD)));
        }
        round(segs)
    };
    // How many chips fit from `from`, leaving room for the `+n more` chip when some don't.
    let fit = |from: usize| -> usize {
        let mut x = start_x;
        let mut n = 0;
        for (i, (_, segs)) in chips.iter().enumerate().skip(from) {
            let w = segs_width(segs);
            let last = i + 1 == chips.len();
            let more_w = if last { 0 } else { segs_width(&more_for(&sessions[i + 1..])) + 1 };
            if x + w + more_w > limit {
                break;
            }
            x += w + 1;
            n += 1;
        }
        n
    };
    let at = selected.and_then(|sel| chips.iter().position(|(term, _)| *term == sel));
    let mut from = 0;
    while let Some(at) = at && from < at && at >= from + fit(from) {
        from += 1;
    }
    let shown = fit(from);
    let mut x = start_x;
    for (term, segs) in chips.iter().skip(from).take(shown) {
        let w = segs_width(segs);
        put(buf, x, area.y, segs, limit);
        hit(app, Rect { x, y: area.y, width: w, height: 1 }, ScreenHit::Session(*term));
        x += w + 1;
    }
    let rest = from + shown;
    if rest < chips.len() {
        let more = more_for(&sessions[rest..]);
        let w = segs_width(&more);
        if x + w <= limit {
            put(buf, x, area.y, &more, limit);
            hit(app, Rect { x, y: area.y, width: w, height: 1 }, ScreenHit::ExpandSidebar);
        }
    }
}
