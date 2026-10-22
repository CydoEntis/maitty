//! The v7 frame set beside the handoff's reference frames (`docs/design/v7/render-frames.mjs`):
//! the grid, the pane's title strip and right-hand mark, corners, and where a sheet goes.
//! Sidebar rows, sheet contents and the pane's own terminal text belong to other screens and
//! are left out.
use super::*;
use crate::client::v7_fixture::{self, LAYOUTS, Layout, SIZES, Stage};
use crate::layout::Node;

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
