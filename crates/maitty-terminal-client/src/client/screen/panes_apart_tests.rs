//! Panes you can tell apart, in every look: their numbers, the close confirm that names and
//! lights its pane, focus that stays lit under a menu, and clicks that give a pane the keys.

use super::*;
use crate::client::ids::Outgoing;
use crate::client::reference_fixture::{Stage, world};
use crate::layout::Node;
use crate::protocol::ClientMsg;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc;

const LOOKS: [&str; 5] = ["floating", "tiled", "compact", "grid", "hairline"];

/// The reference world in `look` with `panes` panes on its open tab: 1, 2 side by side, or 3
/// (two side by side over one).
fn split_world(look: &str, panes: usize) -> App {
    let mut app = world(Stage::None);
    (app.cfg.ui.layout, app.cfg.ui.panes) = match look {
        "floating" | "tiled" => ("cards".into(), look.into()),
        other => (other.into(), "floating".into()),
    };
    let tab = &mut app.screen.tabs[0];
    let f = tab.focus;
    let others: Vec<TermId> = [1, 2, 3, 4, 5]
        .into_iter()
        .filter(|t| *t != f.term)
        .collect();
    let leaf = |t: TermId| Box::new(Node::Leaf(t));
    let side = |a: TermId, b: TermId| Node::Split {
        horizontal: true,
        ratio: 0.5,
        a: leaf(a),
        b: leaf(b),
    };
    tab.layout = match panes {
        1 => Node::Leaf(f.term),
        2 => side(f.term, others[1]),
        _ => Node::Split {
            horizontal: false,
            ratio: 0.5,
            a: Box::new(side(f.term, others[1])),
            b: leaf(others[2]),
        },
    };
    app
}

/// A screen wide enough for `look` to draw a split beside a side panel: Compact splits only
/// from 150 columns of panes.
fn wide(look: &str) -> (u16, u16) {
    if look == "compact" {
        (260, 50)
    } else {
        (160, 45)
    }
}

fn draw(app: &mut App, w: u16, h: u16) -> Buffer {
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f))
        .expect("a frame");
    term.backend().buffer().clone()
}

/// A left click, pressed and let go, at `x`, `y`.
fn click(app: &mut App, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.on_mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
    }
}

/// The panes focused by the commands sent so far.
fn focused_by(rx: &mut mpsc::UnboundedReceiver<Outgoing>) -> Vec<TermId> {
    std::iter::from_fn(|| rx.try_recv().map(|o| o.msg).ok())
        .filter_map(|m| match m {
            ClientMsg::Command(Command::FocusPane { term }) => Some(term),
            _ => None,
        })
        .collect()
}

/// A cell on `term`'s title row that no chip, tab or ✕ takes: the bare border or rule.
fn bare_title_cell(app: &App, term: ClientTerm) -> (u16, u16) {
    assert!(
        app.screen.leaf_rects.len() > 1,
        "{} draws the split: {:?}",
        app.cfg.ui.layout,
        app.screen.leaf_rects
    );
    let r = app
        .pane_frames
        .iter()
        .find(|(id, _)| *id == term)
        .map(|(_, r)| *r)
        .expect("the pane is on screen");
    let x = (r.x + 1..r.right() - 1)
        .rev()
        .find(|&x| {
            app.hits
                .iter()
                .rev()
                .find(|(hr, _)| hr.contains(Position::new(x, r.y)))
                .is_none_or(|(_, h)| matches!(h, Hit::Pane(_)))
        })
        .expect("a bare cell on the title row");
    (x, r.y)
}

#[test]
fn a_click_on_a_pane_s_title_row_takes_the_keys_from_the_sidebar_in_every_look() {
    for look in LOOKS {
        let mut app = split_world(look, 2);
        let other = app.screen.tabs[0].leaves()[1];
        app.screen_side_set(SideItem::Sess(ClientSession::local(SessionKey::Pane(1))));
        let (w, h) = wide(look);
        draw(&mut app, w, h);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        let (x, y) = bare_title_cell(&app, other);
        click(&mut app, x, y);
        assert_eq!(app.mode, Mode::Normal, "{look}: the sidebar lets go");
        assert_eq!(
            focused_by(&mut rx),
            vec![other.term],
            "{look}: the pane clicked is focused"
        );
    }
}

#[test]
fn a_click_on_a_pane_s_title_row_takes_the_keys_from_a_side_panel_in_every_look() {
    for look in LOOKS {
        let mut app = split_world(look, 2);
        let other = app.screen.tabs[0].leaves()[1];
        app.act(Action::Jump);
        assert!(sheet_has_keys(&app), "{look}: the Inbox has the keys");
        let (w, h) = wide(look);
        draw(&mut app, w, h);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        let (x, y) = bare_title_cell(&app, other);
        click(&mut app, x, y);
        assert!(
            !sheet_has_keys(&app) && app.mode == Mode::Normal,
            "{look}: the panel lets go: {:?}",
            app.mode
        );
        assert_eq!(
            focused_by(&mut rx),
            vec![other.term],
            "{look}: the pane clicked is focused"
        );
    }
}

fn text(buf: &Buffer) -> String {
    let a = buf.area;
    (a.y..a.bottom())
        .map(|y| {
            (a.x..a.right())
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
                + "\n"
        })
        .collect()
}

#[test]
fn closing_a_pane_hides_it_and_offers_undo_in_every_look() {
    for look in LOOKS {
        let mut app = split_world(look, 2);
        let (w, h) = wide(look);
        draw(&mut app, w, h);
        let [_, second] = [0, 1].map(|i| app.screen.tabs[0].leaves()[i]);
        app.menu_act(crate::client::menu::Act::End(vec![second]));
        // A working agent asks for the same press again; nothing else is asked, a close can be undone.
        if app.closing_banner().is_some_and(|b| b.warn) {
            app.menu_act(crate::client::menu::Act::End(vec![second]));
        }
        assert_eq!(app.mode, Mode::Normal, "{look}: no dialog opens");
        let banner = app.closing_banner().unwrap_or_else(|| panic!("{look}: the undo offer"));
        assert!(
            banner.text.starts_with("Closed ") && banner.text.contains("then z undo 6s"),
            "{look}: {}",
            banner.text
        );
        let screen = text(&draw(&mut app, w, h));
        assert!(screen.contains("then z undo"), "{look}: the banner is drawn:\n{screen}");
    }
}


#[test]
fn a_lone_session_closes_at_once_and_the_banner_names_it() {
    let mut app = split_world("floating", 1);
    let focus = app.focused().expect("a focused pane");
    draw(&mut app, 160, 45);
    app.menu_act(crate::client::menu::Act::End(vec![focus]));
    let banner = app.closing_banner().expect("the undo offer");
    assert!(
        banner.text.starts_with("Closed fix-footer"),
        "the banner names the session: {}",
        banner.text
    );
}

/// The cell that carries a pane's focus: its frame's top left corner.
fn focus_cell(_look: &str, r: Rect) -> (u16, u16) {
    (r.x, r.y)
}

#[test]
fn the_focused_pane_stays_lit_under_a_menu_popover_or_confirm_in_every_look() {
    type Open = fn(&mut App, ClientTerm, ClientTerm);
    let overlays: [(&str, Open, bool); 3] = [
        (
            "the pane's menu",
            |app, focus, _| app.menu_for_pane(focus, (60, 5)),
            false,
        ),
        (
            "the Menu",
            |app, _, _| app.mode = Mode::Actions { sel: 0 },
            true,
        ),
        ("the Updates box", |app, _, _| app.mode = Mode::Updates, true),
    ];
    for look in LOOKS {
        for (what, open, dims) in overlays {
            let mut app = split_world(look, 2);
            let (w, h) = wide(look);
            let before = draw(&mut app, w, h);
            let [focus, other] = [0, 1].map(|i| app.screen.tabs[0].leaves()[i]);
            let r = app
                .screen
                .leaf_rects
                .iter()
                .find(|(id, _)| *id == focus)
                .map(|(_, r)| *r)
                .expect("the pane is on screen");
            let at = focus_cell(look, r);
            let lit = before[at].fg;
            open(&mut app, focus, other);
            let buf = draw(&mut app, w, h);
            let label = format!("{look}, {what}: {:?}", app.mode);
            if dims {
                let mut behind = before.clone();
                let (area, t) = (behind.area, app.theme.clone());
                dim_all(&mut app, &mut behind, area, &t);
                assert_eq!(
                    buf[at].fg, behind[at].fg,
                    "{label}: the focus colour, dimmed with the rest"
                );
            } else {
                assert_eq!(buf[at].fg, lit, "{label}: the focus colour as it was");
            }
            let o = app
                .screen
                .leaf_rects
                .iter()
                .find(|(id, _)| *id == other)
                .map(|(_, r)| *r)
                .expect("the other pane");
            assert_ne!(
                buf[at].fg,
                buf[focus_cell(look, o)].fg,
                "{label}: the focused pane still stands apart"
            );
        }
    }
}

/// The pane menu of the focused pane in a split of two, drawn: the app and each item's row.
fn pane_menu(app: &mut App) -> Vec<String> {
    draw(app, 160, 45);
    let focus = app.focused().expect("a focused pane");
    app.menu_for_pane(focus, (60, 5));
    let buf = draw(app, 160, 45);
    let Mode::ScreenMenu(m) = &app.mode else {
        panic!("no menu: {:?}", app.mode)
    };
    let labels: Vec<String> = m.items.iter().map(|(l, _)| l.clone()).collect();
    let rows: Vec<String> = text(&buf).lines().map(str::to_string).collect();
    labels
        .iter()
        .map(|label| {
            rows.iter()
                .find(|r| r.contains(label.as_str()))
                .cloned()
                .unwrap_or_else(|| panic!("{label} is drawn:\n{}", text(&buf)))
        })
        .collect()
}

/// The key that runs `label`'s item while the open menu has the keys (not drawn).
fn key_of(app: &App, label: &str) -> String {
    let Mode::ScreenMenu(m) = &app.mode else {
        panic!("no menu: {:?}", app.mode)
    };
    let i = m
        .items
        .iter()
        .position(|(l, _)| l == label)
        .unwrap_or_else(|| panic!("no {label}"));
    crate::client::menu::menu_keys(app, &m.items)[i]
        .as_ref()
        .map(|k| k.label())
        .unwrap_or_default()
}

fn press(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    app.on_key(KeyEvent::new(code, mods));
}

#[test]
fn the_pane_menu_shows_the_real_bound_keys() {
    let mut app = split_world("floating", 2);
    let rows = pane_menu(&mut app);
    assert_eq!(key_of(&app, "Split right"), "|");
    assert_eq!(key_of(&app, "Split down"), "-");
    assert_eq!(key_of(&app, "Close pane"), "x");
    let split = rows
        .iter()
        .find(|r| r.contains("Split right"))
        .expect("drawn");
    assert!(!split.contains(" h "), "the key isn't drawn: {split}");
    let rename = key_of(&app, "Rename");
    assert!(
        rename.len() == 1 && !["-", "|", "x"].contains(&rename.as_str()),
        "no binding: a letter of its own, clear of the bound keys: {rename}"
    );
}

#[test]
fn the_pane_menu_follows_a_rebinding() {
    let mut app = split_world("floating", 2);
    app.keymap.prefixed.retain(|_, a| *a != Action::ClosePane);
    let w = KeySpec {
        code: KeyCode::Char('w'),
        mods: KeyModifiers::NONE,
    };
    app.keymap.prefixed.insert(w, Action::ClosePane);
    app.keymap
        .prefixed_order
        .retain(|k| k.code != KeyCode::Char('x') || !k.mods.is_empty());
    app.keymap.prefixed_order.push(w);
    pane_menu(&mut app);
    assert_eq!(key_of(&app, "Close pane"), "w");
    press(&mut app, KeyCode::Char('w'), KeyModifiers::NONE);
    assert!(app.closing_banner().is_some(), "the new key runs it: {:?}", app.mode);
}

#[test]
fn a_bound_key_the_menu_moves_with_gives_way_to_a_letter() {
    let mut app = split_world("floating", 2);
    app.keymap.prefixed.retain(|_, a| *a != Action::SplitRight);
    let j = KeySpec {
        code: KeyCode::Char('j'),
        mods: KeyModifiers::NONE,
    };
    app.keymap.prefixed.insert(j, Action::SplitRight);
    app.keymap.prefixed_order.retain(|k| *k != j);
    app.keymap.prefixed_order.push(j);
    pane_menu(&mut app);
    let key = key_of(&app, "Split right");
    assert!(
        key.len() == 1 && key != "j",
        "j moves in the menu, so the item takes a letter: {key}"
    );
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert!(
        matches!(&app.mode, Mode::ScreenMenu(m) if m.sel == 1),
        "j still moves: {:?}",
        app.mode
    );
    let Mode::ScreenMenu(m) = &app.mode else {
        unreachable!()
    };
    let keys = crate::client::menu::menu_chars(&app, &m.items);
    let i = m
        .items
        .iter()
        .position(|(l, _)| l == "Split right")
        .expect("the item");
    assert_eq!(
        keys[i].map(String::from),
        Some(key.clone()),
        "the letter given is the one it answers to"
    );
    press(
        &mut app,
        KeyCode::Char(key.chars().next().unwrap()),
        KeyModifiers::NONE,
    );
    assert!(
        !matches!(app.mode, Mode::ScreenMenu(_)),
        "the letter runs it: {:?}",
        app.mode
    );
}

#[test]
fn pressing_a_shown_key_runs_its_item() {
    let mut app = split_world("floating", 2);
    pane_menu(&mut app);
    press(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    assert!(app.closing_banner().is_some(), "x closes this pane with an undo: {:?}", app.mode);
}
