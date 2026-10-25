//! Panes you can tell apart, in every look: their numbers, the close confirm that names and
//! lights its pane, focus that stays lit under a menu, and clicks that give a pane the keys.

use super::*;
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
    let others: Vec<TermId> = [1, 2, 3, 4, 5].into_iter().filter(|t| *t != f).collect();
    let leaf = |t: TermId| Box::new(Node::Leaf(t));
    let side = |a: TermId, b: TermId| Node::Split { horizontal: true, ratio: 0.5, a: leaf(a), b: leaf(b) };
    tab.layout = match panes {
        1 => Node::Leaf(f),
        2 => side(f, others[1]),
        _ => Node::Split { horizontal: false, ratio: 0.5, a: Box::new(side(f, others[1])), b: leaf(others[2]) },
    };
    app
}

/// A screen wide enough for `look` to draw a split beside a side panel: Compact splits only
/// from 150 columns of panes.
fn wide(look: &str) -> (u16, u16) {
    if look == "compact" { (260, 50) } else { (160, 45) }
}

fn draw(app: &mut App, w: u16, h: u16) -> Buffer {
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f)).expect("a frame");
    term.backend().buffer().clone()
}

/// A left click, pressed and let go, at `x`, `y`.
fn click(app: &mut App, x: u16, y: u16) {
    for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
        app.on_mouse(MouseEvent { kind, column: x, row: y, modifiers: crossterm::event::KeyModifiers::NONE });
    }
}

/// The panes focused by the commands sent so far.
fn focused_by(rx: &mut mpsc::UnboundedReceiver<ClientMsg>) -> Vec<TermId> {
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| match m {
            ClientMsg::Command(Command::FocusPane { term }) => Some(term),
            _ => None,
        })
        .collect()
}

/// A cell on `term`'s title row that no chip, tab or ✕ takes: the bare border or rule.
fn bare_title_cell(app: &App, term: TermId) -> (u16, u16) {
    assert!(app.screen.leaf_rects.len() > 1, "{} draws the split: {:?}", app.cfg.ui.layout, app.screen.leaf_rects);
    let r = app.screen.leaf_rects.iter().find(|(id, _)| *id == term).map(|(_, r)| *r).expect("the pane is on screen");
    let x = (r.x + 1..r.right() - 1)
        .rev()
        .find(|&x| app.hits.iter().rev().find(|(hr, _)| hr.contains(Position::new(x, r.y))).is_none_or(|(_, h)| matches!(h, Hit::Pane(_))))
        .expect("a bare cell on the title row");
    (x, r.y)
}

#[test]
fn a_click_on_a_pane_s_title_row_takes_the_keys_from_the_sidebar_in_every_look() {
    for look in LOOKS {
        let mut app = split_world(look, 2);
        let other = app.screen.tabs[0].layout.leaves()[1];
        app.screen_side_set(SideItem::Sess(1));
        let (w, h) = wide(look);
        draw(&mut app, w, h);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        let (x, y) = bare_title_cell(&app, other);
        click(&mut app, x, y);
        assert_eq!(app.mode, Mode::Normal, "{look}: the sidebar lets go");
        assert_eq!(focused_by(&mut rx), vec![other], "{look}: the pane clicked is focused");
    }
}

#[test]
fn a_click_on_a_pane_s_title_row_takes_the_keys_from_a_side_panel_in_every_look() {
    for look in LOOKS {
        let mut app = split_world(look, 2);
        let other = app.screen.tabs[0].layout.leaves()[1];
        app.act(Action::Jump);
        assert!(sheet_has_keys(&app), "{look}: the Inbox has the keys");
        let (w, h) = wide(look);
        draw(&mut app, w, h);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        let (x, y) = bare_title_cell(&app, other);
        click(&mut app, x, y);
        assert!(!sheet_has_keys(&app) && app.mode == Mode::Normal, "{look}: the panel lets go: {:?}", app.mode);
        assert_eq!(focused_by(&mut rx), vec![other], "{look}: the pane clicked is focused");
    }
}

/// Where `term`'s strip segment was drawn.
fn segment_of(app: &App, term: TermId) -> Rect {
    app.hits.iter().find(|(_, h)| *h == Hit::Screen(ScreenHit::StripSession(term))).map(|(r, _)| *r).expect("a strip segment")
}

/// The colours along the focused pane's rule.
fn rule_colours(app: &App, buf: &Buffer) -> Vec<Color> {
    let focus = app.focused().expect("a focused pane");
    let r = app.screen.leaf_rects.iter().find(|(id, _)| *id == focus).map(|(_, r)| *r).expect("the pane is on screen");
    let mut c: Vec<Color> = (r.x..r.right() - 1).map(|x| buf[(x, r.y + 1)].fg).collect();
    c.dedup();
    c
}

#[test]
fn a_strip_segment_gives_its_pane_the_keys_while_the_sidebar_holds_them() {
    let mut app = split_world("hairline", 2);
    let focus = app.focused().expect("a focused pane");
    app.screen_side_set(SideItem::Sess(focus));
    let buf = draw(&mut app, 160, 45);
    assert_eq!(rule_colours(&app, &buf), vec![app.theme.line], "the sidebar has the keys: the rule is not lit");
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    let seg = segment_of(&app, focus);
    click(&mut app, seg.x + 1, seg.y);
    assert_eq!(app.mode, Mode::Normal, "the sidebar lets go");
    assert_eq!(focused_by(&mut rx), vec![focus]);
    let buf = draw(&mut app, 160, 45);
    assert_eq!(rule_colours(&app, &buf), vec![app.theme.accent], "the pane's rule is teal");
    let bar: String = (0..160).map(|x| buf[(x, 44)].symbol()).collect();
    assert!(bar.contains("TYPING"), "{bar}");
    let other = 4;
    app.screen_side_set(SideItem::Sess(focus));
    draw(&mut app, 160, 45);
    let seg = segment_of(&app, other);
    click(&mut app, seg.x + 1, seg.y);
    assert_eq!((app.mode.clone(), focused_by(&mut rx)), (Mode::Normal, vec![other]), "another session's segment opens it with the keys");
}
