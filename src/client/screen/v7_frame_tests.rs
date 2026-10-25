//! The v7 frame set beside the reference frames:
//! the grid, the pane's title strip and right-hand mark, corners, where a sheet goes, the
//! modal and popover frames, and the pointer's tint. Sidebar rows, sheet and modal contents
//! and the pane's own terminal text belong to other screens and are left out.
use super::*;
use crate::client::v7_fixture::{self, LAYOUTS, Layout, SIZES, Stage};
use crate::layout::Node;
use ratatui::Terminal;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;

/// Preset 01 "No check set" from the reference: row 0 from the pane column on, and the last
/// row (Flat: from the pane column on, the sidebar's foot being its own).
const ROWS_01: [(Layout, u16, u16, &str, &str, &str); 10] = [
    (Layout::Floating, 160, 45, "unicode", "┌─ 1  fix-footer  2  add-payments ────────────────────────────────────────────────────────────────────────────── ✓ done  ✕  ┐", "└────────────────────────────────┘ └───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘"),
    (Layout::Floating, 100, 30, "unicode", "┌─ 1  fix-footer  2  add-payments ────────────────────────── ✓ done  ✕  ┐", "└────────────────────────┘ └───────────────────────────────────────────────────────────────────────┘"),
    (Layout::Floating, 80, 24, "unicode", "┌─ 1  fix-footer  2  add-payments ────── ✓ done  ✕  ┐", "└────────────────────────┘ └───────────────────────────────────────────────────┘"),
    (Layout::Tiled, 160, 45, "unicode", "┌─ 1  fix-footer  2  add-payments ─────────────────────────────────────────────────────────────────────────────── ✓ done  ✕  ┐", "└────────────────────────────────┘└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘"),
    (Layout::Tiled, 100, 30, "unicode", "┌─ 1  fix-footer  2  add-payments ─────────────────────────── ✓ done  ✕  ┐", "└────────────────────────┘└────────────────────────────────────────────────────────────────────────┘"),
    (Layout::Tiled, 80, 24, "unicode", "┌─ 1  fix-footer  2  add-payments ─────── ✓ done  ✕  ┐", "└────────────────────────┘└────────────────────────────────────────────────────┘"),
    (Layout::Flat, 160, 45, "unicode", "│ 1  fix-footer  2  add-payments                                                                                    ✓ done  ✕", "│"),
    (Layout::Flat, 100, 30, "unicode", "│ 1  fix-footer  2  add-payments                                ✓ done  ✕", "│"),
    (Layout::Flat, 80, 24, "unicode", "│ 1  fix-footer  2  add-payments            ✓ done  ✕", "│"),
    (Layout::Flat, 80, 24, "ascii", "│ 1  fix-footer  2  add-payments            + done  x", "│"),
];

/// The modal sizes of presets 12 (Override?), A2 (Approve · read first) and 15 (Settings ·
/// set) on a screen `w`×`h`.
fn modal_sizes(w: u16, h: u16) -> [(&'static str, u16, u16); 3] {
    [("12", (w - 2).min(70), 15), ("A2", (w - 2).min(84), (h - 1).min(20) - u16::from(h < 30)), ("15", (w - 2).min(80), (h - 2).min(20))]
}

/// The reference's world with fix-footer's session in two tabs, as the reference draws it.
fn world(stage: Stage) -> App {
    let mut app = v7_fixture::world(stage);
    let owner = app.screen.tabs[app.screen.tab].owner;
    app.screen.tabs.push(ScreenTab { layout: Node::Leaf(2), focus: 2, owner, used: 0, name: String::new() });
    app
}

/// Where the reference puts the pane column: right of the sidebar and the gap; Flat's starts
/// on the sidebar's last column.
fn pane_x(layout: Layout, w: u16) -> u16 {
    match layout {
        Layout::Floating => side_w(w) + 1,
        Layout::Tiled => side_w(w),
        Layout::Flat => side_w(w) - 1,
    }
}

fn rows(text: &str) -> Vec<Vec<char>> {
    text.lines().map(|l| l.chars().collect()).collect()
}

fn from(row: &[char], x: u16) -> String {
    row.iter().skip(x as usize).collect::<String>().trim_end().to_string()
}

fn col(rows: &[Vec<char>], x: u16, ys: std::ops::Range<u16>) -> String {
    ys.map(|y| rows[y as usize].get(x as usize).copied().unwrap_or(' ')).collect()
}

/// `app` drawn in `layout` at `w`×`h`, as cells.
fn buffer(app: &mut App, layout: Layout, w: u16, h: u16) -> Buffer {
    v7_fixture::frame(app, layout, w, h, "unicode");
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f)).expect("a frame");
    term.backend().buffer().clone()
}

#[test]
fn preset_01_s_frame_is_the_reference_s_in_every_layout_and_size() {
    for (layout, w, h, glyphs, top, last) in ROWS_01 {
        let mut app = world(Stage::None);
        let text = v7_fixture::frame(&mut app, layout, w, h, glyphs);
        let at = format!("{layout:?} {w}x{h} {glyphs}\n{text}");
        let r = rows(&text);
        let px = pane_x(layout, w);
        assert_eq!(from(&r[0], px), top, "row 0: {at}");
        let bottom = if layout == Layout::Flat { from(&r[h as usize - 1], px) } else { from(&r[h as usize - 1], 0) };
        assert_eq!(bottom, last, "last row: {at}");
        if layout == Layout::Flat {
            assert_eq!(col(&r, px, 0..h), "│".repeat(h as usize), "one rule between the sidebar and the pane: {at}");
        } else {
            let sw = side_w(w);
            assert_eq!((r[0][0], r[0][sw as usize - 1]), ('┌', '┐'), "the sidebar's top corners: {at}");
            for x in [0, sw - 1, px, w - 1] {
                assert_eq!(col(&r, x, 1..h - 1), "│".repeat(h as usize - 2), "column {x}: {at}");
            }
        }
    }
}

#[test]
fn rounded_corners_round_every_card() {
    for layout in [Layout::Floating, Layout::Tiled] {
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        let text = v7_fixture::frame(&mut app, layout, 100, 30, "unicode");
        let r = rows(&text);
        let (sw, px) = (side_w(100), pane_x(layout, 100));
        let corners = |x0: u16, x1: u16| [r[0][x0 as usize], r[0][x1 as usize], r[29][x0 as usize], r[29][x1 as usize]];
        assert_eq!(corners(0, sw - 1), ['╭', '╮', '╰', '╯'], "sidebar, {layout:?}\n{text}");
        assert_eq!(corners(px, 99), ['╭', '╮', '╰', '╯'], "pane, {layout:?}\n{text}");
    }
}

#[test]
fn a_failed_session_reads_failed_in_plain_ascii() {
    let mut app = world(Stage::Failed);
    let text = v7_fixture::frame(&mut app, Layout::Flat, 80, 24, "ascii");
    let top = text.lines().next().unwrap_or_default();
    assert!(top.ends_with("x failed  x"), "{text}");
}

#[test]
fn a_sheet_docks_72_wide_at_160_and_takes_the_pane_column_below_140() {
    for open in [|app: &mut App| app.open_check_sheet(1), |app: &mut App| app.act(Action::Jump)] {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut app = world(Stage::Failed);
                open(&mut app);
                let text = v7_fixture::frame(&mut app, layout, w, h, "unicode");
                let g = grid(&app, Rect::new(0, 0, w, h));
                let at = format!("{layout:?} {w}x{h}\n{text}");
                let px = pane_x(layout, w);
                let r = rows(&text);
                if w >= NARROW {
                    assert_eq!((g.sheet.x, g.sheet.width), (w - 72, 72), "{at}");
                    assert!(g.panes.x == px && g.panes.width > 0, "the panes stay between: {at}");
                    assert_eq!(r[0][(w - 72) as usize], if layout == Layout::Flat { '│' } else { '┌' }, "{at}");
                } else {
                    assert_eq!((g.sheet.x, g.panes.width), (px, 0), "the sheet takes the pane column: {at}");
                    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                    let back = v7_fixture::frame(&mut app, layout, w, h, "unicode");
                    assert!(back.lines().next().unwrap_or_default().contains(" fix-footer "), "Esc brings the pane back: {back}");
                }
            }
        }
    }
}

/// The cells a column right of `r` and a row under it, where a shadow would fall.
fn beside_and_under(r: Rect, w: u16, h: u16) -> impl Iterator<Item = (u16, u16)> {
    (r.y + 1..=r.bottom()).map(move |y| (r.right(), y)).chain((r.x + 1..=r.right()).map(move |x| (x, r.bottom()))).filter(move |&(x, y)| x < w && y < h)
}

#[test]
fn a_modal_is_centred_with_a_border_all_round_and_no_shadow() {
    for layout in LAYOUTS {
        for (w, h) in SIZES {
            for (preset, mw, mh) in modal_sizes(w, h) {
                let mut app = world(Stage::Failed);
                let mut buf = buffer(&mut app, layout, w, h);
                let t = app.theme.clone();
                let area = Rect::new(0, 0, w, h);
                let before = buf.clone();
                let r = panel(&mut app, &mut buf, area, mw, mh, "OVERRIDE A FAILED CHECK", &[], &t);
                let at = format!("{preset} {layout:?} {w}x{h}");
                assert_eq!(r, Rect::new((w - mw) / 2, (h - mh) / 2, mw, mh), "{at}");
                let line = |y: u16, x0: u16, x1: u16| (x0..x1).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>();
                let fill = |n: usize| "─".repeat(n);
                assert_eq!(line(r.y, r.x, r.right()), format!("┌─ OVERRIDE A FAILED CHECK {} Esc close ─┐", fill(mw as usize - 40)), "{at}");
                assert_eq!(line(r.bottom() - 1, r.x, r.right()), format!("└{}┘", fill(mw as usize - 2)), "{at}");
                assert!((r.y + 1..r.bottom() - 1).all(|y| buf[(r.x, y)].symbol() == "│" && buf[(r.right() - 1, y)].symbol() == "│"), "{at}");
                assert!(beside_and_under(r, w, h).all(|p| buf[p] == before[p]), "no shadow: {at}");
            }
        }
    }
}

/// The modal's frame works like a pane's (the user's choice, 2026-10-24): corners by
/// `ui.rounded`, the title an accent chip on the top border, `Esc close` a chip at its right
/// that a click closes, chip ends capped with `full`, and no accent bar across the top.
#[test]
fn a_modal_s_title_and_esc_close_are_chips_on_its_top_border_in_every_layout_and_rounding() {
    use crate::config::Rounding;
    for layout in ["cards", "tiled", "compact", "grid"] {
        for rounded in [Rounding::Full, Rounding::Corners, Rounding::Off] {
            let mut app = world(Stage::Failed);
            (app.cfg.ui.layout, app.cfg.ui.rounded) = (layout.into(), rounded);
            let t = app.theme.clone();
            let area = Rect::new(0, 0, 100, 30);
            let mut buf = Buffer::empty(area);
            let r = panel(&mut app, &mut buf, area, 70, 15, "OVERRIDE", &[], &t);
            let at = format!("{layout} {rounded:?}");
            let sym = |x: u16| buf[(x, r.y)].symbol().to_string();
            let (tl, tr) = if rounded == Rounding::Off { ("┌", "┐") } else { ("╭", "╮") };
            assert_eq!((sym(r.x), sym(r.x + 1), sym(r.right() - 2), sym(r.right() - 1)), (tl.into(), "─".into(), "─".into(), tr.into()), "{at}");
            let caps = rounded == Rounding::Full;
            let (title, esc) = (r.x + 2, r.right() - 2 - if caps { 13 } else { 11 });
            let row = |x0: u16, x1: u16| (x0..x1).map(sym).collect::<String>();
            let (l, e) = if caps { (CAP_L, CAP_R) } else { ("", "") };
            assert_eq!(row(title, title + 10 + 2 * u16::from(caps)), format!("{l} OVERRIDE {e}"), "{at}");
            assert_eq!(row(esc, r.right() - 2), format!("{l} Esc close {e}"), "{at}");
            assert_eq!(buf[(title + 2, r.y)].bg, t.accent, "the title chip is the accent: {at}");
            let chip = |x: u16| buf[(x, r.y)].symbol() != "─";
            assert!((r.x + 1..r.right() - 1).filter(|x| !chip(*x)).all(|x| buf[(x, r.y)].bg == t.card), "no accent bar: {at}");
            if caps {
                for x in [title, esc] {
                    assert_eq!(buf[(x, r.y)].bg, t.card, "a cap sits on the border's ground: {at}");
                }
            }
            let close = app.hits.iter().rev().find(|(_, h)| *h == Hit::Screen(ScreenHit::Close)).map(|(r, _)| *r).expect("a close target");
            assert_eq!(close, Rect::new(esc, r.y, r.right() - 2 - esc, 1), "the whole Esc chip closes: {at}");
        }
    }
}

#[test]
fn a_popover_sits_beside_the_sidebar_with_a_raised_title_and_no_shadow() {
    for layout in LAYOUTS {
        for (w, h) in SIZES {
            let mut app = world(Stage::Failed);
            let mut buf = buffer(&mut app, layout, w, h);
            let t = app.theme.clone();
            let area = Rect::new(0, 0, w, h);
            let side = grid(&app, area).side;
            let before = buf.clone();
            let r = popover_at(&mut app, &mut buf, popover_rect(area, side, 60, 8), "WHY · fix-footer reads failed", &t);
            let at = format!("{layout:?} {w}x{h}");
            let x = (side_w(w) + 2).min(w - 40);
            assert_eq!(r, Rect::new(x, if h < 30 { 3 } else { 5 }, 60.min(w - x - 2), 8), "{at}");
            assert!((r.x..r.right()).all(|x| buf[(x, r.y)].bg == t.btn), "the title row is raised: {at}");
            let title: String = (r.x..r.x + 8).map(|x| buf[(x, r.y)].symbol().to_string()).collect();
            assert_eq!(title, "  WHY · ", "{at}");
            assert!((r.y + 1..r.bottom()).all(|y| buf[(r.x, y)].bg == t.card && buf[(r.x, y)].symbol() == " "), "no border: {at}");
            assert!(beside_and_under(r, w, h).all(|p| buf[p] == before[p]), "no shadow: {at}");
        }
    }
}

/// A confirm's sub (the folder `changes` found no repo in) is part of its title chip, cut before
/// the Esc chip, which stays whole and closes it.
#[test]
fn a_confirm_s_long_sub_is_cut_inside_its_title_chip() {
    use crate::config::Rounding;
    for rounded in [Rounding::Full, Rounding::Corners, Rounding::Off] {
        let at = format!("{rounded:?}");
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = rounded;
        app.mode = Mode::Confirm(Box::new(super::super::menu::Confirm {
            title: "changes".into(),
            sub: "a-folder-with-a-very-long-name-that-runs-far-past-the-header-of-this-confirm".into(),
            what: "Not a git repository".into(),
            detail: String::new(),
            note: String::new(),
            list: Vec::new(),
            yes: "Init".into(),
            key: '\n',
            danger: false,
            act: super::super::menu::Act::RestartDaemon,
        }));
        let buf = drawn_in(&mut app, "cards");
        let close = app.hits.iter().rev().find(|(_, h)| *h == Hit::Screen(ScreenHit::Close)).map(|(r, _)| *r).expect("the Esc chip");
        let (x0, x1) = ((160 - 58) / 2, (160 - 58) / 2 + 58);
        let row: Vec<String> = (x0..x1).map(|x| buf[(x, close.y)].symbol().to_string()).collect();
        let [l, e] = if rounded == Rounding::Full { [CAP_L, CAP_R] } else { ["", ""] };
        let last = if rounded == Rounding::Full { CAP_R } else { " " };
        let (tl, tr) = if rounded == Rounding::Off { ("┌", "┐") } else { ("╭", "╮") };
        let text = row.concat();
        assert!(text.starts_with(&format!("{tl}─{l} changes · a-folder")), "{at}: {text}");
        assert!(text.ends_with(&format!("─{l} Esc close {e}─{tr}")), "{at}: {text}");
        let esc = close.x - x0;
        assert_eq!((row[esc as usize - 3].as_str(), row[esc as usize - 2].as_str()), (last, "─"), "{at}: the title chip ends whole before the Esc chip: {text}");
        assert!(text.contains('…'), "{at}: the sub is cut: {text}");
        for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
            app.on_mouse(MouseEvent { kind, column: close.x + 2, row: close.y, modifiers: KeyModifiers::NONE });
        }
        assert!(!matches!(app.mode, Mode::Confirm(_)), "{at}: the Esc chip closes it");
    }
}

/// A modal too narrow for the Esc chip beside its title drops the chip rather than draw it
/// over the corner and the title.
#[test]
fn a_modal_too_narrow_for_its_esc_chip_keeps_its_corner_and_title() {
    let mut app = world(Stage::None);
    app.cfg.ui.rounded = crate::config::Rounding::Full;
    let t = app.theme.clone();
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
    let r = panel_at(&mut app, &mut buf, Rect::new(2, 1, 12, 4), "NEW", &[], &t);
    let row: String = (r.x..r.right()).map(|x| buf[(x, r.y)].symbol().to_string()).collect();
    assert!(row.starts_with(&format!("╭─{CAP_L} NEW {CAP_R}")) && row.ends_with("─╮") && !row.contains("Esc"), "{row}");
    assert!(!app.hits.iter().any(|(_, h)| *h == Hit::Screen(ScreenHit::Close)), "nothing to click: Esc closes it");
}

/// Which hit a case points at.
type Pick = fn(&super::super::Hit) -> bool;

/// The region a click at the first point of `what` hits, and the frame with the pointer there
/// against the frame without it.
fn hovered_over(app: &mut App, layout: Layout, what: impl Fn(&super::super::Hit) -> bool) -> (Rect, Buffer, Buffer) {
    let plain = buffer(app, layout, 160, 45);
    let r = app.hits.iter().rev().find(|(_, h)| what(h)).map(|(r, _)| *r).expect("drawn");
    app.hover = Some(Position::new(r.x + r.width / 2, r.y));
    let lit = buffer(app, layout, 160, 45);
    (r, plain, lit)
}

#[test]
fn the_pointer_tints_exactly_the_region_under_it_once() {
    use super::super::Hit;
    let cases: [(&str, Pick); 4] = [
        ("a tab chip", |h| matches!(h, Hit::Screen(ScreenHit::TabPick(_)))),
        ("a pane's ✕", |h| matches!(h, Hit::Screen(ScreenHit::CloseSplit(_)))),
        ("a sidebar row", |h| matches!(h, Hit::Screen(ScreenHit::Session(3)))),
        ("a modal button", |h| matches!(h, Hit::Screen(ScreenHit::ConfirmYes))),
    ];
    for layout in LAYOUTS {
        for (what, is) in cases {
            let mut app = world(Stage::None);
            if what == "a modal button" {
                app.mode = Mode::Confirm(Box::new(super::super::menu::Confirm {
                    title: "Restart daemon".into(),
                    sub: String::new(),
                    what: "5 sessions stop and resume".into(),
                    detail: String::new(),
                    note: String::new(),
                    list: Vec::new(),
                    yes: "Restart".into(),
                    key: '\n',
                    danger: false,
                    act: super::super::menu::Act::RestartDaemon,
                }));
            }
            let t = app.theme.clone();
            let (r, plain, lit) = hovered_over(&mut app, layout, is);
            for y in 0..45 {
                for x in 0..160 {
                    let (before, after) = (plain[(x, y)].bg, lit[(x, y)].bg);
                    let want = if r.contains(Position::new(x, y)) { crate::theme::mix(if before == Color::Reset { t.bg } else { before }, t.text, 0.10) } else { before };
                    assert_eq!(after, want, "{what}, {layout:?}, cell {x},{y} (region {r:?})");
                }
            }
        }
    }
}

/// The cells of every tab chip (number and name) and the + chip in the pane title, left to right.
fn tab_rects(app: &App) -> Vec<Rect> {
    use super::super::Hit;
    let mut v: Vec<Rect> = app.hits.iter().filter(|(_, h)| matches!(h, Hit::Screen(ScreenHit::TabPick(_) | ScreenHit::NewTab))).map(|(r, _)| *r).collect();
    v.sort_by_key(|r| r.x);
    v
}

#[test]
fn square_tabs_draw_no_caps_and_keep_their_widths_by_default() {
    for layout in LAYOUTS {
        let mut app = world(Stage::None);
        assert_eq!(app.cfg.ui.rounded, crate::config::Rounding::Off, "off by default");
        let buf = buffer(&mut app, layout, 160, 45);
        let rects = tab_rects(&app);
        assert_eq!(rects.len(), 3, "two tabs and the + chip: {layout:?}");
        assert!(rects.iter().all(|r| (r.x..r.right()).all(|x| ![CAP_L, CAP_R].contains(&buf[(x, r.y)].symbol()))), "no caps: {layout:?}");
        // " 1 " + " fix-footer " is 14 wide, " + " is 3.
        assert_eq!(rects.last().map(|r| r.width), Some(3), "{layout:?}");
    }
}

#[test]
fn round_tabs_cap_every_chip_on_the_ground_behind_the_title() {
    for layout in LAYOUTS {
        let mut off = world(Stage::None);
        buffer(&mut off, layout, 160, 45);
        let square = tab_rects(&off);

        let mut app = world(Stage::None);
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        let buf = buffer(&mut app, layout, 160, 45);
        let t = app.theme.clone();
        let rects = tab_rects(&app);
        assert_eq!(rects.len(), 3, "{layout:?}");
        let ground = if layout == Layout::Flat { t.card } else { Color::Reset };
        for (r, was) in rects.iter().zip(&square) {
            assert_eq!(r.width, was.width + 2, "the hit rect holds both caps: {layout:?}");
            let (l, rt) = (&buf[(r.x, r.y)], &buf[(r.right() - 1, r.y)]);
            assert_eq!((l.symbol(), l.bg), (CAP_L, ground), "left cap: {layout:?}");
            assert_eq!((rt.symbol(), rt.bg), (CAP_R, ground), "right cap: {layout:?}");
            assert_eq!(l.fg, buf[(r.x + 1, r.y)].bg, "the left cap is the chip's colour: {layout:?}");
            assert_eq!(rt.fg, buf[(r.right() - 2, r.y)].bg, "the right cap is the chip's colour: {layout:?}");
        }
        assert_eq!(buf[(rects[0].x + 1, rects[0].y)].symbol(), " ", "{layout:?}");
        let current = &buf[(rects[0].x, rects[0].y)];
        assert_eq!(current.fg, t.accent, "the current tab's number chip is the accent: {layout:?}");
    }
}

/// Every session row drawn on more than one line, in the order the sidebar lists them.
fn block_rows(app: &App) -> Vec<(TermId, Rect)> {
    let mut v: Vec<(TermId, Rect)> = app.hits.iter().filter_map(|(r, h)| match h {
        super::super::Hit::Screen(ScreenHit::Session(t)) if r.height >= 2 => Some((*t, *r)),
        _ => None,
    }).collect();
    v.sort_by_key(|(_, r)| r.y);
    v
}

/// The first and last column of line `y` on the row's colour `c`.
fn tinted_span(buf: &Buffer, row: Rect, y: u16, c: Color) -> Option<(u16, u16)> {
    let lit: Vec<u16> = (row.x..row.right()).filter(|&x| buf[(x, y)].bg == c).collect();
    Some((*lit.first()?, *lit.last()?))
}

/// The row's lines with, under each, `#` where the row's colour is the ground.
fn shown(buf: &Buffer, row: Rect, c: Color) -> String {
    (row.y..row.bottom()).map(|y| {
        let text: String = (row.x..row.right()).map(|x| buf[(x, y)].symbol().to_string()).collect();
        let tint: String = (row.x..row.right()).map(|x| if buf[(x, y)].bg == c { '#' } else { '.' }).collect();
        format!("{text}\n{tint}\n")
    }).collect()
}

/// A session that keeps a second line (what it asks, its agents, its check) is one even block
/// when selected or hovered: every line lit across the same columns, no quarter-cell corners
/// to notch it, and with `ui.rounded = full` square ends, since caps on each line would scallop.
#[test]
fn a_selected_or_hovered_two_line_session_is_one_even_block() {
    use crate::config::Rounding;
    for (stage, layout) in [Stage::None, Stage::Passed, Stage::Failed].into_iter().flat_map(|s| ["cards", "tiled", "compact", "grid"].map(|l| (s, l))) {
        for side in ["left", "right"] {
            for padding in ["0", "1", "2"] {
                for rounded in [Rounding::Off, Rounding::Corners, Rounding::Full] {
                    for selected in [true, false] {
                        let mut app = v7_fixture::world(stage);
                        app.snap.terms.get_mut(&2).expect("a session").subagents = vec!["a".into(), "b".into()];
                        (app.cfg.ui.rounded, app.cfg.ui.sidebar_position, app.cfg.ui.padding) = (rounded, side.into(), padding.into());
                        drawn_in(&mut app, layout);
                        let rows = block_rows(&app);
                        assert!(rows.len() >= 2, "sessions with a second line: {layout}");
                        for (term, _) in rows {
                            let at = format!("{stage:?} {layout} {side} pad {padding} {rounded:?} {} {term}", if selected { "selected" } else { "hovered" });
                            app.hover = None;
                            if selected {
                                app.mode = Mode::Side;
                                app.screen.cursor_proj = None;
                                app.screen.cursor = Some(term);
                            } else {
                                app.mode = Mode::Normal;
                                let row = block_rows(&app).into_iter().find(|(t, _)| *t == term).expect("the row").1;
                                app.hover = Some(Position::new(row.x + row.width / 2, row.bottom() - 1));
                            }
                            let buf = drawn_in(&mut app, layout);
                            let (_, row) = block_rows(&app).into_iter().find(|(t, _)| *t == term).expect("the row");
                            let c = buf[(row.x + row.width / 2, row.y)].bg;
                            if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
                                println!("{at}\n{}", shown(&buf, row, c));
                            }
                            let spans: Vec<_> = (row.y..row.bottom()).map(|y| tinted_span(&buf, row, y, c)).collect();
                            assert!(spans.iter().all(|s| s.is_some() && *s == spans[0]), "every line lit across the same columns {spans:?}: {at}\n{}", shown(&buf, row, c));
                            for y in row.y..row.bottom() {
                                for x in row.x..row.right() {
                                    assert!(!["▗", "▖", "▝", "▘", CAP_L, CAP_R].contains(&buf[(x, y)].symbol()), "no corner or cap at {x},{y}: {at}\n{}", shown(&buf, row, c));
                                }
                                if rounded == Rounding::Full {
                                    assert!((row.x..row.right()).all(|x| buf[(x, y)].bg == c), "square, the whole width: {at}\n{}", shown(&buf, row, c));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A session on one line keeps its half-circle caps when selected, with rounded shapes on.
#[test]
fn a_selected_one_line_session_ends_in_caps() {
    for layout in ["cards", "tiled", "compact", "grid"] {
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        drawn_in(&mut app, layout);
        let (term, _) = app.hits.iter().find_map(|(r, h)| match h {
            super::super::Hit::Screen(ScreenHit::Session(t)) if r.height == 1 => Some((*t, *r)),
            _ => None,
        }).expect("a one-line session");
        app.mode = Mode::Side;
        app.screen.cursor_proj = None;
        app.screen.cursor = Some(term);
        let buf = drawn_in(&mut app, layout);
        let row = app.hits.iter().find_map(|(r, h)| matches!(h, super::super::Hit::Screen(ScreenHit::Session(t)) if *t == term).then_some(*r)).expect("the row");
        let t = app.theme.clone();
        let (left, right) = (&buf[(row.x, row.y)], &buf[(row.right() - 1, row.y)]);
        assert_eq!((left.symbol(), right.symbol()), (CAP_L, CAP_R), "{layout}");
        assert_eq!((left.fg, right.fg), (t.sel_bg(), t.sel_bg()), "on the selection's colour: {layout}");
    }
}

/// The world drawn at 160×45 in `[ui] layout` `layout` (Grid included, which the reference has not).
fn drawn_in(app: &mut App, layout: &str) -> Buffer {
    (app.cfg.ui.layout, app.cfg.ui.panes) = (layout.into(), "floating".into());
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(160, 45)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f)).expect("a frame");
    term.backend().buffer().clone()
}

fn folder_row(app: &App) -> Rect {
    app.hits.iter().rev().find_map(|(r, h)| matches!(h, super::super::Hit::Screen(ScreenHit::ToggleProj(0))).then_some(*r)).expect("a folder row")
}

/// A folder row's left cell is its fold button, so with `ui.rounded = full` its highlight and
/// hover start a cell before it: both ends are caps, the button keeps its column and a click on
/// it still folds. With `corners` or `off` both ends are square.
#[test]
fn a_selected_or_hovered_folder_row_ends_in_caps_only_when_rounded_in_full() {
    use crate::config::Rounding;
    for layout in ["cards", "tiled", "compact", "grid"] {
        let mut button_x = None;
        for rounded in [Rounding::Off, Rounding::Corners, Rounding::Full] {
            for selected in [true, false] {
                let at = format!("{layout} {rounded:?} {}", if selected { "selected" } else { "hovered" });
                let mut app = world(Stage::None);
                app.cfg.ui.rounded = rounded;
                if selected {
                    app.mode = Mode::Side;
                    app.screen.cursor = None;
                    app.screen.cursor_proj = Some(app.screen_model()[0].key.clone());
                }
                drawn_in(&mut app, layout);
                let row = folder_row(&app);
                if !selected {
                    app.hover = Some(Position::new(row.x + 3, row.y));
                }
                let buf = drawn_in(&mut app, layout);
                let row = folder_row(&app);
                let (left, right) = (&buf[(row.x, row.y)], &buf[(row.right() - 1, row.y)]);
                if rounded == Rounding::Full {
                    assert_eq!((left.symbol(), right.symbol()), (CAP_L, CAP_R), "{at}");
                    assert_eq!(left.fg, buf[(row.x + 2, row.y)].bg, "the left cap is the row's colour: {at}");
                    assert_eq!(right.fg, buf[(row.right() - 2, row.y)].bg, "the right cap is the row's colour: {at}");
                    assert_ne!(left.bg, left.fg, "the cap stands on what is behind the row: {at}");
                } else {
                    assert!(left.symbol() != CAP_L && right.symbol() != CAP_R, "square: {at}");
                }
                let fold = (row.x..row.right()).find(|&x| buf[(x, row.y)].symbol() == "−").expect("the fold button");
                assert_eq!(*button_x.get_or_insert(fold), fold, "the fold button keeps its column: {at}");
                for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
                    app.on_mouse(MouseEvent { kind, column: fold, row: row.y, modifiers: KeyModifiers::NONE });
                }
                drawn_in(&mut app, layout);
                assert!(!app.screen.saved.closed.is_empty(), "a click on the button folds the folder: {at}");
            }
        }
    }
}

/// Compact rules a zone's left only where it meets another zone, measured in the room left
/// inside the padding: the panes beside a left sidebar, a sidebar on the right. A left sidebar
/// has none at any padding, so its selected or hovered folder row ends in caps.
#[test]
fn compact_rules_only_a_zone_beside_another_at_every_padding() {
    use crate::config::Rounding;
    for padding in ["0", "1", "2"] {
        for side in ["left", "right"] {
            for selected in [true, false] {
                let at = format!("padding {padding}, sidebar {side}, {}", if selected { "selected" } else { "hovered" });
                let mut app = world(Stage::None);
                (app.cfg.ui.padding, app.cfg.ui.sidebar_position, app.cfg.ui.rounded) = (padding.into(), side.into(), Rounding::Full);
                if selected {
                    app.mode = Mode::Side;
                    app.screen.cursor = None;
                    app.screen.cursor_proj = Some(app.screen_model()[0].key.clone());
                }
                drawn_in(&mut app, "compact");
                if !selected {
                    let row = folder_row(&app);
                    app.hover = Some(Position::new(row.x + 3, row.y));
                }
                let buf = drawn_in(&mut app, "compact");
                let panes = grid(&app, framed(&app, buf.area)).panes;
                let rule = |r: Rect| (r.y + 1..r.bottom()).filter(|&y| buf[(r.x, y)].symbol() == "│").count() as u16;
                let (side_r, row) = (app.screen.side_rect, folder_row(&app));
                if side == "left" {
                    assert_eq!(rule(side_r), 0, "no rule left of the sidebar: {at}");
                    assert_eq!(rule(panes), panes.height - 1, "the panes are ruled off the sidebar: {at}");
                    let (left, right) = (&buf[(row.x, row.y)], &buf[(row.right() - 1, row.y)]);
                    assert_eq!((left.symbol(), right.symbol()), (CAP_L, CAP_R), "{at}");
                } else {
                    assert_eq!(rule(side_r), side_r.height - 1, "the sidebar is ruled off the panes: {at}");
                    assert_eq!(rule(panes), 0, "no rule left of the panes: {at}");
                }
            }
        }
    }
}

/// Rounded shapes reach every chip and floating card: the SESSIONS chip, + New and Menu on the
/// plain ground, and the Menu, rounded in Cards and one softened block in Flat.
#[test]
fn rounded_shapes_round_the_sidebar_chips_and_the_menu() {
    for layout in LAYOUTS {
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        let text = v7_fixture::raw_text(&mut app, layout, 100, 30, "unicode");
        for chip in [" SESSIONS 4 ", "+ New", "Menu"] {
            assert!(text.contains(&format!("{CAP_L}{chip}")), "{chip} has a left cap: {layout:?}\n{text}");
        }
        app.act(Action::Menu);
        let text = v7_fixture::raw_text(&mut app, layout, 100, 30, "unicode");
        let title = text.lines().find(|l| l.contains(" Menu ")).expect("the menu's title row");
        assert!(title.contains(&format!("{CAP_L} Menu {CAP_R}")), "the title chip is round: {layout:?}\n{title}");
        let corner = if layout == Layout::Flat { "▗" } else { "╭" };
        assert!(title.contains(corner), "the menu's corner is soft: {layout:?}\n{title}");
    }
}

#[test]
fn a_block_of_lines_ends_in_soft_corners_and_one_line_in_caps() {
    assert_eq!(block_ends(0, 1), (CAP_L, CAP_R));
    assert_eq!(block_ends(0, 2), ("▗", "▖"));
    assert_eq!(block_ends(1, 2), ("▝", "▘"));
    assert_eq!(block_ends(1, 3), ("▐", "▌"), "the sides between");
}

/// The world split into two panes side by side, the second unfocused, an update waiting and,
/// with `sheet`, the Inbox open: every hoverable chip on a border at once.
fn border_chips_world(sheet: bool) -> App {
    let mut app = world(Stage::None);
    let tab = &mut app.screen.tabs[0];
    let other = if tab.focus == 2 { 3 } else { 2 };
    tab.layout = Node::Split { horizontal: true, ratio: 0.5, a: Box::new(Node::Leaf(tab.focus)), b: Box::new(Node::Leaf(other)) };
    app.update_available = Some("9.9.9".into());
    if sheet {
        app.act(Action::Jump);
    }
    app
}

/// Row 0 at `x0..x1` as text, the caps as `(` and `)`, with `#` under the cells on `lit`.
fn shown_row(buf: &Buffer, y: u16, x0: u16, x1: u16, lit: Color) -> String {
    let text: String = (x0..x1).map(|x| match buf[(x, y)].symbol() { CAP_L => "(".into(), CAP_R => ")".into(), s => s.to_string() }).collect();
    let tint: String = (x0..x1).map(|x| if buf[(x, y)].bg == lit || buf[(x, y)].fg == lit && [CAP_L, CAP_R].contains(&buf[(x, y)].symbol()) { '#' } else { '.' }).collect();
    format!("{text}\n{tint}")
}

/// A hovered chip on a border is whole: with `ui.rounded = full` both caps in the chip's tint,
/// the ✕ centred in it on the tint, the hit rect the whole chip, and a ✕ chip a blank cell clear
/// of what is on either side, the border line included.
#[test]
fn a_hovered_border_chip_is_whole() {
    use super::super::Hit;
    use crate::config::Rounding;
    let chips: [(&str, Pick); 6] = [
        ("a pane's ✕", |h| matches!(h, Hit::Screen(ScreenHit::CloseSplit(_)))),
        ("the sheet's ✕", |h| matches!(h, Hit::Screen(ScreenHit::Close | ScreenHit::ViewClose))),
        ("a tab chip", |h| matches!(h, Hit::Screen(ScreenHit::TabPick(_)))),
        ("the + chip", |h| matches!(h, Hit::Screen(ScreenHit::NewTab))),
        ("the bar's update pill", |h| matches!(h, Hit::Screen(ScreenHit::Update))),
        ("the bar's mode tag", |h| matches!(h, Hit::Screen(ScreenHit::Leader | ScreenHit::ShortcutMode))),
    ];
    let mut covered = std::collections::BTreeSet::new();
    for layout in ["cards", "tiled", "compact", "grid"] {
        for rounded in [Rounding::Off, Rounding::Corners, Rounding::Full] {
            for ((w, h), sheet) in SIZES.into_iter().flat_map(|s| [(s, false), (s, true)]) {
                let mut app = border_chips_world(sheet);
                app.cfg.ui.rounded = rounded;
                let size = |app: &mut App| {
                    (app.cfg.ui.layout, app.cfg.ui.panes) = (layout.into(), "floating".into());
                    app.screen_fresh();
                    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
                    term.draw(|f| crate::client::render::draw(app, f)).expect("a frame");
                    term.backend().buffer().clone()
                };
                size(&mut app);
                // The border chips on row 0, the mode bar's own chips on its row: its tag a cell in
                // (TYPING's hint after it takes the same click but is words, not a chip).
                let bar = crate::client::screen::mode_bar_row(&app, Rect::new(0, 0, w, h));
                let on_row = |r: &Rect, what: &str| match what {
                    "the bar's mode tag" => r.y == bar.y && r.x == bar.x + 1,
                    "the bar's update pill" => r.y == bar.y,
                    _ => r.y == 0,
                };
                let targets: Vec<(&str, Rect)> = chips.iter().flat_map(|(what, is)| app.hits.iter().filter(|(r, h)| on_row(r, what) && is(h)).map(|(r, _)| (*what, *r)).collect::<Vec<_>>()).collect();
                for (what, r) in targets {
                    covered.insert(what);
                    app.hover = Some(Position::new(r.x + r.width / 2, r.y));
                    let buf = size(&mut app);
                    app.hover = None;
                    let mid = buf[(r.x + r.width / 2, r.y)].bg;
                    let at = format!("{what}, {layout} {rounded:?} {w}x{h}, {r:?}\n{}", shown_row(&buf, r.y, r.x.saturating_sub(3), (r.right() + 3).min(w), mid));
                    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
                        println!("{at}\n");
                    }
                    if rounded == Rounding::Full {
                        let (l, e) = (&buf[(r.x, r.y)], &buf[(r.right() - 1, r.y)]);
                        assert_eq!((l.symbol(), e.symbol()), (CAP_L, CAP_R), "both caps: {at}");
                        assert!(l.fg != l.bg && e.fg != e.bg, "the caps are drawn, not blank: {at}");
                    }
                    if what.ends_with('✕') {
                        let x = r.x + r.width / 2;
                        assert_eq!(buf[(x, r.y)].symbol(), "✕", "the ✕ is centred and whole: {at}");
                        assert!((r.x + 1..r.right() - 1).all(|x| buf[(x, r.y)].bg == mid), "the chip's inside is lit: {at}");
                        if rounded == Rounding::Full {
                            assert!(r.width == 5 && [r.x + 1, r.right() - 2].iter().all(|&x| buf[(x, r.y)].symbol() == " "), "a space each side of the ✕ inside its caps: {at}");
                        }
                        // A square ✕ keeps the reference's place, flush with the border like every square chip.
                        if rounded != Rounding::Full {
                            continue;
                        }
                        for x in [r.x - 1, r.right()] {
                            assert_eq!(buf[(x, r.y)].symbol(), " ", "a blank cell clear of the chip at {x}: {at}");
                            assert_ne!(buf[(x, r.y)].bg, mid, "the gap is not lit at {x}: {at}");
                        }
                    }
                }
            }
        }
    }
    assert_eq!(covered.len(), chips.len(), "every kind of chip is hovered somewhere: {covered:?}");
}
