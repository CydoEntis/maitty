//! The v7 frame set beside the handoff's reference frames (`docs/design/v7/render-frames.mjs`):
//! the grid, the pane's title strip and right-hand mark, corners, where a sheet goes, the
//! modal and popover frames, and the pointer's tint. Sidebar rows, sheet and modal contents
//! and the pane's own terminal text belong to other screens and are left out.
use super::*;
use crate::client::v7_fixture::{self, LAYOUTS, Layout, SIZES, Stage};
use crate::layout::Node;
use ratatui::Terminal;
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

/// The handoff's world with fix-footer's session in two tabs, as the reference draws it.
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
        app.cfg.ui.corners = "rounded".into();
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

#[test]
fn a_modal_is_centred_with_an_accent_title_a_hard_shadow_and_a_bottom_border() {
    for layout in LAYOUTS {
        for (w, h) in SIZES {
            for (preset, mw, mh) in modal_sizes(w, h) {
                let mut app = world(Stage::Failed);
                let mut buf = buffer(&mut app, layout, w, h);
                let t = app.theme.clone();
                let area = Rect::new(0, 0, w, h);
                let r = panel(&mut app, &mut buf, area, mw, mh, "OVERRIDE A FAILED CHECK", &[], &t);
                let at = format!("{preset} {layout:?} {w}x{h}");
                assert_eq!(r, Rect::new((w - mw) / 2, (h - mh) / 2, mw, mh), "{at}");
                let line = |y: u16, x0: u16, x1: u16| (x0..x1).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>();
                assert!((r.x..r.right()).all(|x| buf[(x, r.y)].bg == t.accent), "the title row is the accent: {at}");
                assert_eq!(line(r.y, r.x + 2, r.x + 25), "OVERRIDE A FAILED CHECK", "{at}");
                assert_eq!(line(r.y, r.right() - 11, r.right() - 2), "Esc close", "{at}");
                assert_eq!(line(r.bottom() - 1, r.x, r.right()), format!("└{}┘", "─".repeat(mw as usize - 2)), "{at}");
                assert!((r.y + 1..r.bottom() - 1).all(|y| buf[(r.x, y)].symbol() == "│" && buf[(r.right() - 1, y)].symbol() == "│"), "{at}");
                let shadow = buf[(r.right(), r.y + 1)].bg;
                let cast: Vec<(u16, u16)> = (r.y + 1..=r.bottom()).map(|y| (r.right(), y)).chain((r.x + 1..=r.right()).map(|x| (x, r.bottom()))).filter(|&(x, y)| x < w && y < h).collect();
                assert!(cast.iter().all(|&p| buf[p].bg == shadow && buf[p].symbol() == " "), "one cell right and down: {at}");
                assert!(shadow != t.card && shadow != t.bg, "darker than the ground: {at}");
            }
        }
    }
}

#[test]
fn a_popover_sits_beside_the_sidebar_with_a_raised_title_and_a_shadow() {
    for layout in LAYOUTS {
        for (w, h) in SIZES {
            let mut app = world(Stage::Failed);
            let mut buf = buffer(&mut app, layout, w, h);
            let t = app.theme.clone();
            let area = Rect::new(0, 0, w, h);
            let side = grid(&app, area).side;
            let r = popover_at(&mut app, &mut buf, popover_rect(area, side, 60, 8), "WHY · fix-footer reads failed", &t);
            let at = format!("{layout:?} {w}x{h}");
            let x = (side_w(w) + 2).min(w - 40);
            assert_eq!(r, Rect::new(x, if h < 30 { 3 } else { 5 }, 60.min(w - x - 2), 8), "{at}");
            assert!((r.x..r.right()).all(|x| buf[(x, r.y)].bg == t.btn), "the title row is raised: {at}");
            let title: String = (r.x..r.x + 8).map(|x| buf[(x, r.y)].symbol().to_string()).collect();
            assert_eq!(title, "  WHY · ", "{at}");
            assert!((r.y + 1..r.bottom()).all(|y| buf[(r.x, y)].bg == t.card && buf[(r.x, y)].symbol() == " "), "no border: {at}");
            let shadow = buf[(r.right(), r.y + 1)].bg;
            assert!((r.y + 1..=r.bottom()).all(|y| buf[(r.right(), y)].bg == shadow) && (r.x + 1..=r.right()).all(|x| buf[(x, r.bottom())].bg == shadow), "{at}");
        }
    }
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
        assert!(!app.cfg.ui.rounded, "off by default");
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
        app.cfg.ui.rounded = true;
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

#[test]
fn round_rows_end_every_line_of_the_selected_session_in_a_cap_and_off_leaves_it_square() {
    for layout in LAYOUTS {
        for on in [false, true] {
            let mut app = world(Stage::None);
            app.cfg.ui.rounded = on;
            app.mode = Mode::Side;
            let term = app.screen_model().iter().flat_map(|p| p.sessions().map(|s| s.term).collect::<Vec<_>>()).next().expect("a session");
            app.screen.cursor_proj = None;
            app.screen.cursor = Some(term);
            buffer(&mut app, layout, 160, 45);
            let row = app.hits.iter().find_map(|(r, h)| matches!(h, super::super::Hit::Screen(ScreenHit::Session(t)) if *t == term).then_some(*r)).expect("the session's row");
            assert!(row.height >= 2, "a session is a block of lines: {layout:?}");
            let buf = buffer(&mut app, layout, 160, 45);
            let t = app.theme.clone();
            for y in row.y..row.bottom() {
                let (left, right) = (&buf[(row.x, y)], &buf[(row.right() - 1, y)]);
                if on {
                    assert_eq!((left.symbol(), right.symbol()), (CAP_L, CAP_R), "caps on line {y}: {layout:?}");
                    assert_eq!((left.fg, right.fg), (t.sel_bg(), t.sel_bg()), "on the selection's colour: {layout:?}");
                    assert_ne!(left.bg, t.sel_bg(), "over what is behind it, not the selection: {layout:?}");
                } else {
                    assert!(left.symbol() != CAP_L && right.symbol() != CAP_R, "square when off: {layout:?}");
                }
            }
        }
    }
}

#[test]
fn round_tabs_end_the_selected_folder_row_in_a_cap_and_off_leaves_it_square() {
    for layout in LAYOUTS {
        for on in [false, true] {
            let mut app = world(Stage::None);
            app.cfg.ui.rounded = on;
            app.mode = Mode::Side;
            app.screen.cursor = None;
            app.screen.cursor_proj = Some(app.screen_model()[0].key.clone());
            buffer(&mut app, layout, 160, 45);
            let row = app.hits.iter().find_map(|(r, h)| matches!(h, super::super::Hit::Screen(ScreenHit::ToggleProj(0))).then_some(*r)).expect("a folder row");
            let buf = buffer(&mut app, layout, 160, 45);
            let t = app.theme.clone();
            let last = &buf[(row.right() - 1, row.y)];
            if on {
                assert_eq!((last.symbol(), last.fg, last.bg), (CAP_R, t.sel_bg(), buf[(row.right() - 1, row.y + 1)].bg), "cap on the selection's colour over the pane: {layout:?}");
            } else {
                assert_ne!(last.symbol(), CAP_R, "{layout:?}");
            }
        }
    }
}
