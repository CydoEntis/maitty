//! The frame set beside the reference frames:
//! the grid, the pane's title strip and right-hand mark, corners, where a sheet goes, the
//! modal and popover frames, and the pointer's tint. Sidebar rows, sheet and modal contents
//! and the pane's own terminal text belong to other screens and are left out.
use super::*;
use crate::client::reference_fixture::{self, LAYOUTS, Layout, SIZES, Stage};
use crate::layout::Node;
use ratatui::Terminal;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;

/// Preset 01 "No check set" from the reference: the last row. The reference's row 0 drew the
/// tabs in the pane's title; they have a row of their own now (ADR 0028), checked apart.
const ROWS_01: [(Layout, u16, u16, &str, &str); 6] = [
    (Layout::Floating, 160, 45, "unicode", "└────────────────────────────────┘ └───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘"),
    (Layout::Floating, 100, 30, "unicode", "└────────────────────────┘ └───────────────────────────────────────────────────────────────────────┘"),
    (Layout::Floating, 80, 24, "unicode", "└────────────────────────┘ └───────────────────────────────────────────────────┘"),
    (Layout::Tiled, 160, 45, "unicode", "└────────────────────────────────┘└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘"),
    (Layout::Tiled, 100, 30, "unicode", "└────────────────────────┘└────────────────────────────────────────────────────────────────────────┘"),
    (Layout::Tiled, 80, 24, "unicode", "└────────────────────────┘└────────────────────────────────────────────────────┘"),
];

/// The modal sizes of presets 12 (Override?), A2 (Approve · read first) and 15 (Settings ·
/// set) on a screen `w`×`h`.
fn modal_sizes(w: u16, h: u16) -> [(&'static str, u16, u16); 3] {
    [("12", (w - 2).min(70), 15), ("A2", (w - 2).min(84), (h - 1).min(20) - u16::from(h < 30)), ("15", (w - 2).min(80), (h - 2).min(20))]
}

/// The reference's world with fix-footer's session in two tabs, as the reference draws it.
fn world(stage: Stage) -> App {
    let mut app = reference_fixture::world(stage);
    let owner = app.screen.tabs[app.screen.tab].owner;
    app.screen.tabs.push(ScreenTab { layout: Node::Leaf(2), focus: 2, owner, used: 0, name: String::new() });
    app
}

/// The row a lone pane's top border is on: level with the sidebar's top edge, the tabs on it (ADR 0038).
fn pane_y(_: Layout) -> u16 {
    0
}

/// Where the reference puts the pane column: right of the sidebar and the gap.
fn pane_x(layout: Layout, w: u16) -> u16 {
    match layout {
        Layout::Floating => side_w(w) + 1,
        Layout::Tiled => side_w(w),
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
    reference_fixture::frame(app, layout, w, h, "unicode");
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f)).expect("a frame");
    term.backend().buffer().clone()
}

#[test]
fn preset_01_s_frame_is_the_reference_s_in_every_layout_and_size() {
    for (layout, w, h, glyphs, last) in ROWS_01 {
        let mut app = world(Stage::None);
        let text = reference_fixture::frame(&mut app, layout, w, h, glyphs);
        let at = format!("{layout:?} {w}x{h} {glyphs}\n{text}");
        let r = rows(&text);
        let (px, py) = (pane_x(layout, w), pane_y(layout));
        let tabs = from(&r[0], px);
        assert!(tabs.contains(" 1  fix-footer ") && tabs.contains(" 2  add-payments"), "row 0, the pane's top border, carries the tabs, each with its state: {at}");
        let title = from(&r[py as usize], px);
        assert!(title.starts_with("┌─ ") && title.ends_with('┐'), "a lone pane's top border opens with its tabs and closes at the corner: {at}");
        assert_eq!(from(&r[h as usize - 1], 0), last, "last row: {at}");
        let sw = side_w(w);
        assert_eq!((r[0][0], r[0][sw as usize - 1]), ('┌', '┐'), "the sidebar's top corners: {at}");
        for x in [0, sw - 1] {
            assert_eq!(col(&r, x, 1..h - 1), "│".repeat(h as usize - 2), "column {x}: {at}");
        }
        for x in [px, w - 1] {
            assert_eq!(col(&r, x, py + 1..h - 1), "│".repeat((h - py) as usize - 2), "column {x}: {at}");
        }
    }
}

#[test]
fn rounded_corners_round_every_card() {
    for layout in [Layout::Floating, Layout::Tiled] {
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = true;
        let text = reference_fixture::frame(&mut app, layout, 100, 30, "unicode");
        let r = rows(&text);
        let (sw, px) = (side_w(100), pane_x(layout, 100));
        let corners = |top: usize, x0: u16, x1: u16| [r[top][x0 as usize], r[top][x1 as usize], r[29][x0 as usize], r[29][x1 as usize]];
        assert_eq!(corners(0, 0, sw - 1), ['╭', '╮', '╰', '╯'], "sidebar, {layout:?}\n{text}");
        assert_eq!(corners(pane_y(layout) as usize, px, 99), ['╭', '╮', '╰', '╯'], "pane, {layout:?}\n{text}");
    }
}

#[test]
fn a_failed_session_reads_failed_in_plain_ascii() {
    let mut app = world(Stage::Failed);
    let text = reference_fixture::frame(&mut app, Layout::Tiled, 80, 24, "ascii");
    let tabs = text.lines().next().unwrap_or_default();
    assert!(tabs.contains("x 1  fix-footer"), "its tab says failed by its glyph: {text}");
}

#[test]
fn a_sheet_docks_72_wide_at_160_and_takes_the_pane_column_below_140() {
    for open in [|app: &mut App| app.open_check_sheet(1), |app: &mut App| app.act(Action::Jump)] {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut app = world(Stage::Failed);
                open(&mut app);
                let text = reference_fixture::frame(&mut app, layout, w, h, "unicode");
                let g = grid(&app, Rect::new(0, 0, w, h));
                let at = format!("{layout:?} {w}x{h}\n{text}");
                let px = pane_x(layout, w);
                let r = rows(&text);
                if w >= NARROW {
                    assert_eq!((g.sheet.x, g.sheet.width), (w - 72, 72), "{at}");
                    assert!(g.panes.x == px && g.panes.width > 0, "the panes stay between: {at}");
                    assert_eq!(r[0][(w - 72) as usize], '┌', "{at}");
                } else {
                    assert_eq!((g.sheet.x, g.panes.width), (px, 0), "the sheet takes the pane column: {at}");
                    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                    let back = reference_fixture::frame(&mut app, layout, w, h, "unicode");
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
                assert_eq!(line(r.y, r.x, r.right()), format!("┌─ OVERRIDE A FAILED CHECK {} × ", fill(mw as usize - 30)), "{at}");
                assert_eq!(line(r.bottom() - 1, r.x, r.right()), format!("└{}┘", fill(mw as usize - 2)), "{at}");
                assert!((r.y + 1..r.bottom() - 1).all(|y| buf[(r.x, y)].symbol() == "│" && buf[(r.right() - 1, y)].symbol() == "│"), "{at}");
                assert!(beside_and_under(r, w, h).all(|p| buf[p] == before[p]), "no shadow: {at}");
            }
        }
    }
}

/// The modal's frame works like a pane's (the user's choice, 2026-10-24): corners by
/// `ui.rounded`, the title an accent chip on the top border, a red `✕` chip at its right
/// that a click closes, and no accent bar across the top.
#[test]
fn a_modal_s_title_and_esc_close_are_chips_on_its_top_border_in_every_layout_and_rounding() {
    for layout in ["cards", "tiled", "compact", "grid"] {
        for rounded in [true, false] {
            let mut app = world(Stage::Failed);
            (app.cfg.ui.layout, app.cfg.ui.rounded) = (layout.into(), rounded);
            let t = app.theme.clone();
            let area = Rect::new(0, 0, 100, 30);
            let mut buf = Buffer::empty(area);
            let r = panel(&mut app, &mut buf, area, 70, 15, "OVERRIDE", &[], &t);
            let at = format!("{layout} {rounded:?}");
            let sym = |x: u16| buf[(x, r.y)].symbol().to_string();
            let tl = if rounded { "╭" } else { "┌" };
            // The red × covers the corner: its last padding cell is the corner cell.
            assert_eq!((sym(r.x), sym(r.x + 1), sym(r.right() - 1)), (tl.into(), "─".into(), " ".into()), "{at}");
            let (title, esc) = (r.x + 2, r.right() - 3);
            let row = |x0: u16, x1: u16| (x0..x1).map(sym).collect::<String>();
            assert_eq!(row(title, title + 10), " OVERRIDE ", "{at}");
            assert_eq!(row(esc, r.right()), " × ", "{at}");
            assert_eq!(buf[(title + 2, r.y)].bg, t.accent, "the title chip is the accent: {at}");
            let chip = |x: u16| buf[(x, r.y)].symbol() != "─";
            assert!((r.x + 1..r.right() - 1).filter(|x| !chip(*x)).all(|x| buf[(x, r.y)].bg == t.card), "no accent bar: {at}");
            let close = app.hits.iter().rev().find(|(_, h)| *h == Hit::Screen(ScreenHit::Close)).map(|(r, _)| *r).expect("a close target");
            assert_eq!(close, Rect::new(esc, r.y, r.right() - esc, 1), "the whole Esc chip closes: {at}");
        }
    }
}

#[test]
fn a_popover_sits_beside_the_sidebar_boxed_with_a_raised_title_and_no_shadow() {
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
            assert!((r.y + 1..r.bottom()).all(|y| buf[(r.x, y)].bg == t.card && buf[(r.x, y)].symbol() == " "), "its words a column in from its edge: {at}");
            // Boxed as a pane is (ADR 0028), a cell outside it; nothing past the box.
            assert!((r.y..r.bottom()).all(|y| buf[(r.x - 1, y)].symbol() == "│" && buf[(r.right(), y)].symbol() == "│"), "its box: {at}");
            let outer = Rect { x: r.x - 1, y: r.y - 1, width: r.width + 2, height: r.height + 2 };
            assert!(beside_and_under(outer, w, h).all(|p| buf[p] == before[p]), "no shadow: {at}");
        }
    }
}

/// A confirm's sub (the folder `changes` found no repo in) is part of its title chip, cut before
/// the Esc chip, which stays whole and closes it.
#[test]
fn a_confirm_s_long_sub_is_cut_inside_its_title_chip() {
    for rounded in [true, false] {
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
        let tl = if rounded { "╭" } else { "┌" };
        let text = row.concat();
        assert!(text.starts_with(&format!("{tl}─ changes · a-folder")), "{at}: {text}");
        assert!(text.ends_with("─ × "), "{at}: {text}");
        let esc = close.x - x0;
        assert_eq!((row[esc as usize - 3].as_str(), row[esc as usize - 2].as_str()), (" ", "─"), "{at}: the title chip ends whole before the Esc chip: {text}");
        assert!(text.contains('…'), "{at}: the sub is cut: {text}");
        for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
            app.on_mouse(MouseEvent { kind, column: close.x + 2, row: close.y, modifiers: KeyModifiers::NONE });
        }
        assert!(!matches!(app.mode, Mode::Confirm(_)), "{at}: the Esc chip closes it");
    }
}

/// A modal too narrow for the red ✕ chip beside its corner drops the chip rather than draw it
/// over the corner.
#[test]
fn a_modal_too_narrow_for_its_close_chip_keeps_its_corners() {
    let mut app = world(Stage::None);
    app.cfg.ui.rounded = true;
    let t = app.theme.clone();
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
    let r = panel_at(&mut app, &mut buf, Rect::new(2, 1, 6, 4), "NEW", &[], &t);
    let row: String = (r.x..r.right()).map(|x| buf[(x, r.y)].symbol().to_string()).collect();
    assert!(row.starts_with("╭─") && row.ends_with("─╮") && !row.contains('×'), "{row}");
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
        ("a tab's ✕", |h| matches!(h, Hit::Screen(ScreenHit::CloseTab(_)))),
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
    // The pane title's, not the sidebar's tab lines.
    let side = app.screen.side_rect;
    let mut v: Vec<Rect> = app.hits.iter().filter(|(r, h)| matches!(h, Hit::Screen(ScreenHit::TabPick(_) | ScreenHit::NewTab)) && !side.intersects(*r)).map(|(r, _)| *r).collect();
    v.sort_by_key(|r| r.x);
    v
}

#[test]
fn tabs_keep_their_widths_by_default() {
    for layout in LAYOUTS {
        let mut app = world(Stage::None);
        assert!(!app.cfg.ui.rounded, "off by default");
        buffer(&mut app, layout, 160, 45);
        let rects = tab_rects(&app);
        assert_eq!(rects.len(), 3, "two tabs and the + chip: {layout:?}");
        // " 1 " + " fix-footer " is 14 wide, " + " is 3.
        assert_eq!(rects.last().map(|r| r.width), Some(3), "{layout:?}");
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
/// to notch it.
#[test]
fn a_selected_or_hovered_two_line_session_is_one_even_block() {
    for (stage, layout) in [Stage::None, Stage::Passed, Stage::Failed].into_iter().flat_map(|s| ["cards", "tiled", "compact", "grid"].map(|l| (s, l))) {
        for side in ["left", "right"] {
            for padding in ["0", "1", "2"] {
                for rounded in [false, true] {
                    for selected in [true, false] {
                        let mut app = reference_fixture::world(stage);
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
                                    assert!(!["▗", "▖", "▝", "▘"].contains(&buf[(x, y)].symbol()), "no corner at {x},{y}: {at}\n{}", shown(&buf, row, c));
                                }
                            }
                        }
                    }
                }
            }
        }
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

#[test]
fn a_folder_rows_fold_button_keeps_its_column_and_a_click_on_it_folds() {
    for layout in ["cards", "tiled", "compact", "grid"] {
        let mut button_x = None;
        for rounded in [false, true] {
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

fn shown_row(buf: &Buffer, y: u16, x0: u16, x1: u16, lit: Color) -> String {
    let text: String = (x0..x1).map(|x| buf[(x, y)].symbol().to_string()).collect();
    let tint: String = (x0..x1).map(|x| if buf[(x, y)].bg == lit { '#' } else { '.' }).collect();
    format!("{text}\n{tint}")
}

/// A hovered chip on a border is whole: the × centred in it on the tint and the hit rect the
/// whole chip.
#[test]
fn a_hovered_border_chip_is_whole() {
    use super::super::Hit;
    let chips: [(&str, Pick); 4] = [
        ("a tab chip", |h| matches!(h, Hit::Screen(ScreenHit::TabPick(_)))),
        ("the + chip", |h| matches!(h, Hit::Screen(ScreenHit::NewTab))),
        ("the bar's update pill", |h| matches!(h, Hit::Screen(ScreenHit::Update))),
        ("the bar's mode tag", |h| matches!(h, Hit::Screen(ScreenHit::Leader | ScreenHit::ShortcutMode))),
    ];
    let mut covered = std::collections::BTreeSet::new();
    for layout in ["floating", "tiled"] {
        for rounded in [false, true] {
            for ((w, h), sheet) in SIZES.into_iter().flat_map(|s| [(s, false), (s, true)]) {
                let mut app = border_chips_world(sheet);
                app.cfg.ui.rounded = rounded;
                let size = |app: &mut App| {
                    app.cfg.ui.layout = layout.into();
                    app.cfg.ui.fold_retired_layout();
                    app.screen_fresh();
                    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
                    term.draw(|f| crate::client::render::draw(app, f)).expect("a frame");
                    term.backend().buffer().clone()
                };
                size(&mut app);
                // The tabs and the sheet's ✕ on row 0, a pane's ✕ on its top border, the mode bar's
                // own chips on its row: its tag a cell in (TYPING's hint after it takes the same
                // click but is words, not a chip).
                let bar = crate::client::screen::mode_bar_row(&app, Rect::new(0, 0, w, h));
                let side_bottom = app.screen.side_rect.bottom();
                let on_row = |r: &Rect, what: &str| match what {
                    "the bar's mode tag" => r.y == bar.y && r.x == bar.x + 1,
                    // The sidebar's foot takes the pill while it has room, else the bar does.
                    "the bar's update pill" => r.y == bar.y || r.y + 4 == side_bottom,
                    "a pane's ✕" => true,
                    _ => r.y <= 1,
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
                    if what.ends_with('✕') {
                        let x = r.x + r.width / 2;
                        assert_eq!(buf[(x, r.y)].symbol(), "×", "the ✕ is centred and whole: {at}");
                        assert!((r.x + 1..r.right() - 1).all(|x| buf[(x, r.y)].bg == mid), "the chip's inside is lit: {at}");
                    }
                }
            }
        }
    }
    assert_eq!(covered.len(), chips.len(), "every kind of chip is hovered somewhere: {covered:?}");
}

/// Row 0 of the 160x45 frame as text.
fn top_row(app: &mut App) -> String {
    let buf = drawn_in(app, "floating");
    (0..160u16).map(|x| buf[(x, 0)].symbol().to_string()).collect()
}

#[test]
fn the_tab_gap_is_a_cell_of_the_border_between_tabs_or_none() {
    for gap in [true, false] {
        let mut app = world(Stage::None);
        app.cfg.ui.tab_gap = gap;
        let row = top_row(&mut app);
        let between = &row[row.find("fix-footer").expect("the first tab")..row.find("add-payments").expect("the second tab")];
        assert_eq!(between.matches('─').count(), usize::from(gap), "gap {gap}: a cell of the border between the tabs, or none: {row}");
    }
}

#[test]
fn the_sheets_tabs_and_close_follow_the_gap() {
    for gap in [true, false] {
        let mut app = world(Stage::Failed);
        app.cfg.ui.tab_gap = gap;
        app.act(Action::Files);
        let row = top_row(&mut app);
        let at = format!("gap {gap}: {row}");
        let strip = &row[row.find("Files").expect("the Files tab")..row.find("References").expect("the References tab")];
        assert_eq!(strip.matches('─').count(), if gap { 3 } else { 0 }, "a cell of the border between the sheet's tabs, or none: {at}");
        assert!(row.contains('×'), "the close button: {at}");
    }
}
