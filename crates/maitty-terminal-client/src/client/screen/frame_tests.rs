//! The frame set beside the reference frames:
//! the grid, the pane's title strip and right-hand mark, corners, where a sheet goes, the
//! modal and popover frames, and the pointer's tint. Sidebar rows, sheet and modal contents
//! and the pane's own terminal text belong to other screens and are left out.
use super::*;
use crate::client::reference_fixture::{self, LAYOUTS, Stage};
use crate::layout::Node;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

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


/// The world drawn at 160×45 in `[ui] layout` `layout` (Grid included, which the reference has not).
fn drawn_in(app: &mut App, layout: &str) -> Buffer {
    (app.cfg.ui.layout, app.cfg.ui.panes) = (layout.into(), "floating".into());
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(160, 45)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f))
        .expect("a frame");
    term.backend().buffer().clone()
}

/// Rounded shapes reach every chip and floating card: the sidebar's title, machine and menu chips on the
/// plain ground, and the Menu.
#[test]
fn rounded_shapes_round_cards_and_give_chips_pill_ends() {
    for layout in LAYOUTS {
        let mut app = world(Stage::None);
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        let text = reference_fixture::raw_text(&mut app, layout, 140, 36, "unicode");
        for chip in ["maitty", "This computer", "menu"] {
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
