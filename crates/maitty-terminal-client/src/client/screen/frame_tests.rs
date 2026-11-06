//! The frame set beside the reference frames:
//! the grid, the pane's title strip and right-hand mark, corners, where a sheet goes, the
//! modal and popover frames, and the pointer's tint. Sidebar rows, sheet and modal contents
//! and the pane's own terminal text belong to other screens and are left out.
use super::*;
use crate::client::reference_fixture::{self, LAYOUTS, Layout, SIZES, Stage};
use crate::layout::Node;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// The modal sizes of presets 12 (Override?), A2 (Approve · read first) and 15 (Settings ·
/// set) on a screen `w`×`h`.
fn modal_sizes(w: u16, h: u16) -> [(&'static str, u16, u16); 3] {
    [
        ("12", (w - 2).min(70), 15),
        ("A2", (w - 2).min(84), (h - 1).min(20) - u16::from(h < 30)),
        ("15", (w - 2).min(80), (h - 2).min(20)),
    ]
}

/// The reference's world with fix-footer's session in two tabs, as the reference draws it.
fn world(stage: Stage) -> App {
    let mut app = reference_fixture::world(stage);
    let owner = app.screen.tabs[app.screen.tab].owner;
    app.screen.tabs.push(ScreenTab {
        layout: Node::Leaf(2),
        focus: ClientTerm::local(2),
        owner,
        used: 0,
        name: String::new(),
    });
    app
}

/// The row a lone pane's top border is on: under the tab row, which is level with the sidebar's top edge.
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

/// `app` drawn in `layout` at `w`×`h`, as cells.
fn buffer(app: &mut App, layout: Layout, w: u16, h: u16) -> Buffer {
    reference_fixture::frame(app, layout, w, h, "unicode");
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f))
        .expect("a frame");
    term.backend().buffer().clone()
}

#[test]
fn rounded_corners_round_every_card() {
    for layout in [Layout::Floating, Layout::Tiled] {
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        let text = reference_fixture::frame(&mut app, layout, 100, 30, "unicode");
        let r = rows(&text);
        let (sw, px) = (side_w(100), pane_x(layout, 100));
        let corners = |top: usize, x0: u16, x1: u16| {
            [
                r[top][x0 as usize],
                r[top][x1 as usize],
                r[29][x0 as usize],
                r[29][x1 as usize],
            ]
        };
        assert_eq!(
            corners(0, 0, sw - 1),
            ['╭', '╮', '╰', '╯'],
            "sidebar, {layout:?}\n{text}"
        );
        assert_eq!(
            corners(pane_y(layout) as usize, px, 99),
            ['╭', '╮', '╰', '╯'],
            "pane, {layout:?}\n{text}"
        );
    }
}

#[test]
fn a_sheet_docks_72_wide_at_160_and_takes_the_pane_column_below_140() {
    for open in [
        |app: &mut App| app.open_check_sheet(ClientTerm::local(1)),
        |app: &mut App| app.act(Action::Jump),
    ] {
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
                    assert!(
                        g.panes.x == px && g.panes.width > 0,
                        "the panes stay between: {at}"
                    );
                    assert_eq!(r[0][(w - 72) as usize], '┌', "{at}");
                } else {
                    assert_eq!(
                        (g.sheet.x, g.panes.width),
                        (px, 0),
                        "the sheet takes the pane column: {at}"
                    );
                    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                    let back = reference_fixture::frame(&mut app, layout, w, h, "unicode");
                    assert!(
                        back.lines()
                            .next()
                            .unwrap_or_default()
                            .contains(" fix-footer "),
                        "Esc brings the pane back: {back}"
                    );
                }
            }
        }
    }
}

/// The cells a column right of `r` and a row under it, where a shadow would fall.
fn beside_and_under(r: Rect, w: u16, h: u16) -> impl Iterator<Item = (u16, u16)> {
    (r.y + 1..=r.bottom())
        .map(move |y| (r.right(), y))
        .chain((r.x + 1..=r.right()).map(move |x| (x, r.bottom())))
        .filter(move |&(x, y)| x < w && y < h)
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
                let r = panel(
                    &mut app,
                    &mut buf,
                    area,
                    mw,
                    mh,
                    "OVERRIDE A FAILED CHECK",
                    &[],
                    &t,
                );
                let at = format!("{preset} {layout:?} {w}x{h}");
                assert_eq!(r, Rect::new((w - mw) / 2, (h - mh) / 2, mw, mh), "{at}");
                let line = |y: u16, x0: u16, x1: u16| {
                    (x0..x1)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect::<String>()
                };
                let fill = |n: usize| "─".repeat(n);
                assert_eq!(
                    line(r.y, r.x, r.right()),
                    format!("┌─ OVERRIDE A FAILED CHECK {} × ", fill(mw as usize - 30)),
                    "{at}"
                );
                assert_eq!(
                    line(r.bottom() - 1, r.x, r.right()),
                    format!("└{}┘", fill(mw as usize - 2)),
                    "{at}"
                );
                assert!(
                    (r.y + 1..r.bottom() - 1).all(|y| buf[(r.x, y)].symbol() == "│"
                        && buf[(r.right() - 1, y)].symbol() == "│"),
                    "{at}"
                );
                assert!(
                    beside_and_under(r, w, h).all(|p| buf[p] == before[p]),
                    "no shadow: {at}"
                );
            }
        }
    }
}

/// The modal's frame works like a pane's: corners by `ui.rounded`, the title an accent chip,
/// and a red close chip at its right
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
            let tl = if rounded == Rounding::Off {
                "┌"
            } else {
                "╭"
            };
            let caps = rounded == Rounding::Full;
            let corner = if caps { CAP_R } else { " " };
            assert_eq!(
                (sym(r.x), sym(r.x + 1), sym(r.right() - 1)),
                (tl.into(), "─".into(), corner.into()),
                "{at}"
            );
            let (title, esc) = (r.x + 2, if caps { r.right() - 5 } else { r.right() - 3 });
            let row = |x0: u16, x1: u16| (x0..x1).map(sym).collect::<String>();
            let (l, e) = if caps { (CAP_L, CAP_R) } else { ("", "") };
            assert_eq!(
                row(title, title + 10 + 2 * u16::from(caps)),
                format!("{l} OVERRIDE {e}"),
                "{at}"
            );
            assert_eq!(row(esc, r.right()), format!("{l} × {e}"), "{at}");
            assert_eq!(
                buf[(title + 2, r.y)].bg,
                t.accent,
                "the title chip is the accent: {at}"
            );
            let chip = |x: u16| buf[(x, r.y)].symbol() != "─";
            // The empty buffer's ground stands for whatever the modal is drawn over.
            let behind = Color::Reset;
            assert!(
                (r.x + 1..r.right() - 1)
                    .filter(|x| !chip(*x))
                    .all(|x| buf[(x, r.y)].bg == behind),
                "no accent bar, and the line keeps the ground behind it: {at}"
            );
            let outline = (r.x..r.right())
                .map(|x| (x, r.bottom() - 1))
                .chain((r.y + 1..r.bottom()).flat_map(|y| [(r.x, y), (r.right() - 1, y)]));
            for at_cell in outline {
                assert_eq!(
                    buf[at_cell].bg, behind,
                    "the body's colour stops inside the outline, none of it outside the line: {at} {at_cell:?}"
                );
            }
            assert_eq!(buf[(r.x + 1, r.y + 1)].bg, t.card, "the body is filled: {at}");
            if caps {
                for x in [title, esc] {
                    assert_eq!(
                        buf[(x, r.y)].bg,
                        behind,
                        "a cap sits on the border's ground: {at}"
                    );
                }
            }
            let close = app
                .hits
                .iter()
                .rev()
                .find(|(_, h)| *h == Hit::Screen(ScreenHit::Close))
                .map(|(r, _)| *r)
                .expect("a close target");
            assert_eq!(
                close,
                Rect::new(esc, r.y, r.right() - esc, 1),
                "the whole close chip closes: {at}"
            );
        }
    }
}

#[test]
fn a_confirm_s_long_sub_is_cut_inside_its_title_chip() {
    use crate::config::Rounding;
    for rounded in [Rounding::Full, Rounding::Corners, Rounding::Off] {
        let at = format!("{rounded:?}");
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = rounded;
        app.mode = Mode::Confirm(Box::new(super::super::menu::Confirm {
            title: "changes".into(),
            sub: "a-folder-with-a-very-long-name-that-runs-far-past-the-header-of-this-confirm"
                .into(),
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
        let close = app
            .hits
            .iter()
            .rev()
            .find(|(_, h)| *h == Hit::Screen(ScreenHit::Close))
            .map(|(r, _)| *r)
            .expect("the Esc chip");
        let (x0, x1) = ((160 - 58) / 2, (160 - 58) / 2 + 58);
        let row: Vec<String> = (x0..x1)
            .map(|x| buf[(x, close.y)].symbol().to_string())
            .collect();
        let caps = rounded == Rounding::Full;
        let (l, e) = if caps { (CAP_L, CAP_R) } else { ("", "") };
        let last = if caps { CAP_R } else { " " };
        let tl = if rounded == Rounding::Off {
            "┌"
        } else {
            "╭"
        };
        let text = row.concat();
        assert!(
            text.starts_with(&format!("{tl}─{l} changes · a-folder")),
            "{at}: {text}"
        );
        assert!(text.ends_with(&format!("─{l} × {e}")), "{at}: {text}");
        let esc = close.x - x0;
        assert_eq!(
            (
                row[esc as usize - 3].as_str(),
                row[esc as usize - 2].as_str()
            ),
            (last, "─"),
            "{at}: the title chip ends whole before the close chip: {text}"
        );
        assert!(text.contains('…'), "{at}: the sub is cut: {text}");
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            app.on_mouse(MouseEvent {
                kind,
                column: close.x + 2,
                row: close.y,
                modifiers: KeyModifiers::NONE,
            });
        }
        assert!(
            !matches!(app.mode, Mode::Confirm(_)),
            "{at}: the close chip closes it"
        );
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
    let r = panel_at(&mut app, &mut buf, Rect::new(2, 1, 6, 4), "NEW", &[], &t);
    let row: String = (r.x..r.right())
        .map(|x| buf[(x, r.y)].symbol().to_string())
        .collect();
    assert!(
        row.starts_with("╭─") && row.ends_with("─╮") && !row.contains('×'),
        "{row}"
    );
    assert!(
        !app.hits
            .iter()
            .any(|(_, h)| *h == Hit::Screen(ScreenHit::Close)),
        "nothing to click: Esc closes it"
    );
}

/// Which hit a case points at.
type Pick = fn(&super::super::Hit) -> bool;

/// The region a click at the first point of `what` hits, and the frame with the pointer there
/// against the frame without it.
fn hovered_over(
    app: &mut App,
    layout: Layout,
    what: impl Fn(&super::super::Hit) -> bool,
) -> (Rect, Buffer, Buffer) {
    let plain = buffer(app, layout, 160, 45);
    let r = app
        .hits
        .iter()
        .rev()
        .find(|(_, h)| what(h))
        .map(|(r, _)| *r)
        .expect("drawn");
    app.hover = Some(Position::new(r.x + r.width / 2, r.y));
    let lit = buffer(app, layout, 160, 45);
    (r, plain, lit)
}

#[test]
fn the_pointer_tints_exactly_the_region_under_it_once() {
    use super::super::Hit;
    let cases: [(&str, Pick); 4] = [
        ("a tab chip", |h| {
            matches!(h, Hit::Screen(ScreenHit::TabPick(_)))
        }),
        ("a tab's ✕", |h| {
            matches!(h, Hit::Screen(ScreenHit::CloseTab(_)))
        }),
        ("a sidebar row", |h| {
            matches!(
                h,
                Hit::Screen(ScreenHit::Session(ClientSession {
                    key: SessionKey::Pane(3),
                    ..
                }))
            )
        }),
        ("a modal button", |h| {
            matches!(h, Hit::Screen(ScreenHit::ConfirmYes))
        }),
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
                    let want = if r.contains(Position::new(x, y)) {
                        crate::theme::mix(
                            if before == Color::Reset { t.bg } else { before },
                            t.text,
                            0.10,
                        )
                    } else {
                        before
                    };
                    assert_eq!(
                        after, want,
                        "{what}, {layout:?}, cell {x},{y} (region {r:?})"
                    );
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
    let mut v: Vec<Rect> = app
        .hits
        .iter()
        .filter(|(r, h)| {
            matches!(h, Hit::Screen(ScreenHit::TabPick(_) | ScreenHit::NewTab))
                && !side.intersects(*r)
        })
        .map(|(r, _)| *r)
        .collect();
    v.sort_by_key(|r| r.x);
    v
}

#[test]
fn square_tabs_draw_no_caps_and_keep_their_widths_by_default() {
    for layout in LAYOUTS {
        let mut app = world(Stage::None);
        assert_eq!(
            app.cfg.ui.rounded,
            crate::config::Rounding::Off,
            "off by default"
        );
        let buf = buffer(&mut app, layout, 160, 45);
        let rects = tab_rects(&app);
        assert_eq!(rects.len(), 3, "two tabs and the + chip: {layout:?}");
        assert!(
            rects.iter().all(
                |r| (r.x..r.right()).all(|x| ![CAP_L, CAP_R].contains(&buf[(x, r.y)].symbol()))
            ),
            "no caps: {layout:?}"
        );
        // " 1 " + " fix-footer " is 14 wide, " + " is 3.
        assert_eq!(rects.last().map(|r| r.width), Some(3), "{layout:?}");
    }
}

#[test]
fn rounded_tabs_are_pills_two_cells_wider() {
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
        for (r, was) in rects.iter().zip(&square) {
            assert_eq!(
                r.width,
                was.width + 2,
                "rounded adds a cap each end: {layout:?}"
            );
            assert_eq!(
                (buf[(r.x, r.y)].symbol(), buf[(r.right() - 1, r.y)].symbol()),
                (CAP_L, CAP_R),
                "{layout:?}"
            );
        }
        assert!(
            rects
                .iter()
                .any(|r| (r.x..r.right()).any(|x| buf[(x, r.y)].fg == t.accent)),
            "the current tab's number chip is the accent: {layout:?}"
        );
    }
}

/// The world drawn at 160×45 in `[ui] layout` `layout` (Grid included, which the reference has not).
fn drawn_in(app: &mut App, layout: &str) -> Buffer {
    (app.cfg.ui.layout, app.cfg.ui.panes) = (layout.into(), "floating".into());
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(160, 45)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f))
        .expect("a frame");
    term.backend().buffer().clone()
}

/// Rounded shapes reach every chip and floating card: the SESSIONS chip, + New and Menu on the
/// plain ground, and the Menu.
#[test]
fn rounded_shapes_round_cards_and_give_chips_pill_ends() {
    for layout in LAYOUTS {
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        let text = reference_fixture::raw_text(&mut app, layout, 100, 30, "unicode");
        for chip in [" LOCAL ", "+ New", "Menu"] {
            assert!(
                text.contains(chip),
                "{chip} stays visible: {layout:?}\n{text}"
            );
        }
        assert!(
            text.contains(CAP_L) && text.contains(CAP_R),
            "rounded gives chips pill caps: {layout:?}"
        );
        app.act(Action::Menu);
        let text = reference_fixture::raw_text(&mut app, layout, 100, 30, "unicode");
        let title = text
            .lines()
            .find(|l| l.contains(" Menu "))
            .expect("the menu's title row");
        assert!(
            title.contains("Menu") && title.contains(CAP_L),
            "the menu title is a pill: {layout:?}\n{title}"
        );
        assert!(
            title.contains("╭"),
            "the menu's corner is soft: {layout:?}\n{title}"
        );
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
    let other = if tab.focus == ClientTerm::local(2) {
        3
    } else {
        2
    };
    tab.layout = Node::Split {
        horizontal: true,
        ratio: 0.5,
        a: Box::new(Node::Leaf(tab.focus.term)),
        b: Box::new(Node::Leaf(other)),
    };
    app.update_available = Some("9.9.9".into());
    if sheet {
        app.act(Action::Jump);
    }
    app
}

/// Row 0 at `x0..x1` as text, the caps as `(` and `)`, with `#` under the cells on `lit`.
fn shown_row(buf: &Buffer, y: u16, x0: u16, x1: u16, lit: Color) -> String {
    let text: String = (x0..x1)
        .map(|x| match buf[(x, y)].symbol() {
            CAP_L => "(".into(),
            CAP_R => ")".into(),
            s => s.to_string(),
        })
        .collect();
    let tint: String = (x0..x1)
        .map(|x| {
            if buf[(x, y)].bg == lit
                || buf[(x, y)].fg == lit && [CAP_L, CAP_R].contains(&buf[(x, y)].symbol())
            {
                '#'
            } else {
                '.'
            }
        })
        .collect();
    format!("{text}\n{tint}")
}

/// A hovered chip on a border is whole: with `ui.rounded = full` both caps in the chip's tint,
/// the ✕ centred in it on the tint, the hit rect the whole chip, and a ✕ chip a blank cell clear
/// of what is on either side, the border line included.
#[test]
fn a_hovered_border_chip_is_whole() {
    use super::super::Hit;
    use crate::config::Rounding;
    let chips: [(&str, Pick); 5] = [
        ("the sheet's ✕", |h| {
            matches!(h, Hit::Screen(ScreenHit::Close | ScreenHit::ViewClose))
        }),
        ("a tab chip", |h| {
            matches!(h, Hit::Screen(ScreenHit::TabPick(_)))
        }),
        ("the + chip", |h| {
            matches!(h, Hit::Screen(ScreenHit::NewTab))
        }),
        ("the bar's update pill", |h| {
            matches!(h, Hit::Screen(ScreenHit::Update))
        }),
        ("the bar's mode tag", |h| {
            matches!(h, Hit::Screen(ScreenHit::Leader | ScreenHit::ShortcutMode))
        }),
    ];
    let mut covered = std::collections::BTreeSet::new();
    for layout in ["floating", "tiled"] {
        for rounded in [Rounding::Off, Rounding::Corners, Rounding::Full] {
            for ((w, h), sheet) in SIZES.into_iter().flat_map(|s| [(s, false), (s, true)]) {
                let mut app = border_chips_world(sheet);
                app.cfg.ui.rounded = rounded;
                let size = |app: &mut App| {
                    app.cfg.ui.layout = layout.into();
                    app.cfg.ui.fold_retired_layout();
                    app.screen_fresh();
                    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
                    term.draw(|f| crate::client::render::draw(app, f))
                        .expect("a frame");
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
                    "the bar's update pill" => r.y == bar.y || r.y + 4 == side_bottom,
                    "a pane's ✕" => true,
                    _ => r.y <= 1,
                };
                let targets: Vec<(&str, Rect)> = chips
                    .iter()
                    .flat_map(|(what, is)| {
                        app.hits
                            .iter()
                            .filter(|(r, h)| on_row(r, what) && is(h))
                            .map(|(r, _)| (*what, *r))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                for (what, r) in targets {
                    covered.insert(what);
                    app.hover = Some(Position::new(r.x + r.width / 2, r.y));
                    let buf = size(&mut app);
                    app.hover = None;
                    let mid = buf[(r.x + r.width / 2, r.y)].bg;
                    let at = format!(
                        "{what}, {layout} {rounded:?} {w}x{h}, {r:?}\n{}",
                        shown_row(
                            &buf,
                            r.y,
                            r.x.saturating_sub(3),
                            (r.right() + 3).min(w),
                            mid
                        )
                    );
                    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
                        println!("{at}\n");
                    }
                    if what.contains("✕") {
                        let x = r.x + r.width / 2;
                        assert_eq!(
                            buf[(x, r.y)].symbol(),
                            "×",
                            "the ✕ is centred and whole: {at}"
                        );
                        assert!(
                            (r.x + 1..r.right() - 1).all(|x| buf[(x, r.y)].bg == mid),
                            "the chip's inside is lit: {at}"
                        );
                    }
                }
            }
        }
    }
    assert_eq!(
        covered.len(),
        chips.len(),
        "every kind of chip is hovered somewhere: {covered:?}"
    );
}
