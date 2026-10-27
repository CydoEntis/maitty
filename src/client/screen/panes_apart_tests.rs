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

fn text(buf: &Buffer) -> String {
    let a = buf.area;
    (a.y..a.bottom()).map(|y| (a.x..a.right()).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end().to_string() + "\n").collect()
}

/// `r`'s title row as text.
fn title_of(buf: &Buffer, r: Rect) -> String {
    (r.x..r.right()).map(|x| buf[(x, r.y)].symbol()).collect()
}

/// Every look at 80×24 and 160×45 (and Compact wide enough to split) with 1, 2 and 3 panes:
/// the label, the app and its frame.
fn every_split() -> impl Iterator<Item = (String, App, Buffer)> {
    LOOKS.into_iter().flat_map(|look| {
        [1, 2, 3].into_iter().flat_map(move |panes| {
            [(80, 24), (160, 45), wide(look)].into_iter().collect::<std::collections::BTreeSet<_>>().into_iter().map(move |(w, h)| {
                let mut app = split_world(look, panes);
                let buf = draw(&mut app, w, h);
                (format!("{look}, {panes} pane(s), {w}x{h}"), app, buf)
            })
        })
    })
}

#[test]
fn every_pane_of_a_split_shows_its_number_in_reading_order_in_every_look() {
    for (label, app, buf) in every_split() {
        let mut shown = app.screen.leaf_rects.clone();
        shown.sort_by_key(|(_, r)| (r.y, r.x));
        for (n, (term, r)) in shown.iter().enumerate() {
            let title = title_of(&buf, *r);
            if shown.len() < 2 {
                assert!(!title.contains("1 ·") && !title.trim_start().starts_with("1 "), "{label}: a lone pane has no number: {title}");
                continue;
            }
            // A narrow chip cuts the name before the number: `2 · warm-f…`, at the least a bare `2`.
            let want = if label.starts_with("hairline") { vec![format!("  {} ", n + 1)] } else { vec![format!(" {} · ", n + 1), format!(" {} ─", n + 1), format!(" {}  ", n + 1)] };
            assert!(want.iter().any(|w| title.contains(w)), "{label}: pane {term} reads {want:?}: {title}\n{}", text(&buf));
            assert!(!title.contains(&format!(" {}…", n + 1)), "{label}: a name cut away leaves the bare number: {title}");
        }
    }
}

#[test]
fn the_top_left_pane_names_its_own_session_whichever_pane_has_focus() {
    for look in ["floating", "tiled", "compact", "grid"] {
        for focus_on in [1, 2] {
            let mut app = split_world(look, 3);
            let (w, h) = wide(look);
            draw(&mut app, w, h);
            let mut shown = app.screen.leaf_rects.clone();
            shown.sort_by_key(|(_, r)| (r.y, r.x));
            app.screen.tabs[0].focus = shown[focus_on].0;
            let buf = draw(&mut app, w, h);
            let model = app.screen_model();
            for (n, (term, r)) in shown.iter().enumerate() {
                let title = title_of(&buf, *r);
                let want = format!("{} · {}", n + 1, session_name(&app, &model, *term));
                assert!(title.contains(&want), "{look}, focus on pane {}: pane {} reads {want:?}: {title}", focus_on + 1, n + 1);
            }
        }
    }
}

#[test]
fn a_named_tab_keeps_its_name_on_its_tab_and_the_pane_its_own() {
    let mut app = split_world("floating", 2);
    app.screen.tabs[0].name = "build".into();
    let buf = draw(&mut app, 160, 45);
    let r = app.screen.leaf_rects.iter().min_by_key(|(_, r)| (r.y, r.x)).map(|(_, r)| *r).expect("a pane");
    let tabs = |app: &App, buf: &Buffer| title_of(buf, crate::client::screen::grid(app, buf.area).tabs);
    let title = title_of(&buf, r);
    assert!(tabs(&app, &buf).contains(" build ") && title.contains(" 1 · fix-footer ") && !title.contains("build"), "the tab's name on its tab, the pane's in its title: {} / {title}", tabs(&app, &buf));
    let mut second = app.screen.tabs[0].clone();
    (second.layout, second.focus, second.name) = (Node::Leaf(5), 5, String::new());
    app.screen.tabs.push(second);
    let buf = draw(&mut app, 160, 45);
    assert!(tabs(&app, &buf).contains(" 1  build "), "with several tabs the tab adds its number: {}", tabs(&app, &buf));
}

#[test]
fn a_hairline_number_is_teal_focused_amber_when_it_needs_you_else_dim() {
    let mut app = split_world("hairline", 3);
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    for (term, want) in [(1, t.accent), (3, t.blocked), (4, t.muted)] {
        let r = app.screen.leaf_rects.iter().find(|(id, _)| *id == term).map(|(_, r)| *r).expect("the pane is on screen");
        let cell = &buf[(r.x + hairline::PANE_PAD, r.y)];
        assert!(cell.symbol().chars().all(|c| c.is_ascii_digit()), "pane {term} leads with its number: {}", title_of(&buf, r));
        assert_eq!(cell.fg, want, "pane {term}");
        assert!(cell.modifier.contains(Modifier::BOLD), "pane {term}");
    }
}

/// The cells of `r` drawn in exactly `colour`.
fn cells_in(buf: &Buffer, r: Rect, colour: Color) -> usize {
    r.positions().filter(|p| buf[(p.x, p.y)].fg == colour).count()
}

#[test]
fn closing_a_pane_names_it_and_lights_it_in_every_look() {
    for look in LOOKS {
        let mut app = split_world(look, 2);
        let (w, h) = wide(look);
        draw(&mut app, w, h);
        let [first, second] = [0, 1].map(|i| app.screen.tabs[0].layout.leaves()[i]);
        app.menu_act(crate::client::menu::Act::End(vec![second]));
        let buf = draw(&mut app, w, h);
        let screen = text(&buf);
        assert!(screen.contains("Close pane 2 · warm-finch"), "{look}: the confirm names the pane:\n{screen}");
        assert!(screen.contains("pixls"), "{look}: and its folder:\n{screen}");
        let rect = |term: TermId| app.screen.leaf_rects.iter().find(|(id, _)| *id == term).map(|(_, r)| *r).expect("the pane is on screen");
        let (closing, kept) = (rect(second), rect(first));
        let t = app.theme.clone();
        let line = |r: Rect| if look == "hairline" { (r.x + 1, r.y + 1) } else if look == "compact" { (r.x, r.y + 1) } else { (r.x, r.y) };
        assert_eq!(buf[line(closing)].fg, t.err, "{look}: the pane being closed is lit, bright over the dimmed screen:\n{screen}");
        // Compact lights its zone's `│` rule, as it does for focus; the others their border or rule.
        assert!(cells_in(&buf, closing, t.err) > 20, "{look}: its lines, not a cell");
        assert_ne!(buf[line(kept)].fg, t.err, "{look}: only that pane");
        app.on_key(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE));
        let buf = draw(&mut app, w, h);
        assert_eq!(cells_in(&buf, closing, t.err), 0, "{look}: Esc puts it out");
    }
}

#[test]
fn a_lone_pane_s_confirm_says_close_pane_and_its_name() {
    let mut app = split_world("floating", 1);
    let focus = app.focused().expect("a focused pane");
    draw(&mut app, 160, 45);
    app.menu_act(crate::client::menu::Act::End(vec![focus]));
    let screen = text(&draw(&mut app, 160, 45));
    assert!(screen.contains("Close pane · fix-footer"), "no number for a lone pane:\n{screen}");
}

/// The cell that carries a pane's focus in `look`: a box's corner, Compact's `│` rule, or
/// Hairline's rule.
fn focus_cell(look: &str, r: Rect) -> (u16, u16) {
    match look {
        "hairline" => (r.x + 1, r.y + 1),
        "compact" => (r.x, r.y + 1),
        _ => (r.x, r.y),
    }
}

#[test]
fn the_focused_pane_stays_lit_under_a_menu_popover_or_confirm_in_every_look() {
    type Open = fn(&mut App, TermId, TermId);
    let overlays: [(&str, Open, bool); 4] = [
        ("the pane's menu", |app, focus, _| app.menu_for_pane(focus, (60, 5)), false),
        ("why", |app, focus, _| app.mode = Mode::Why { term: focus, side: false }, false),
        ("the Menu", |app, _, _| app.mode = Mode::Actions { sel: 0 }, true),
        ("a close confirm", |app, _, other| app.menu_act(crate::client::menu::Act::End(vec![other])), true),
    ];
    for look in LOOKS {
        for (what, open, dims) in overlays {
            let mut app = split_world(look, 2);
            let (w, h) = wide(look);
            let before = draw(&mut app, w, h);
            let [focus, other] = [0, 1].map(|i| app.screen.tabs[0].layout.leaves()[i]);
            let r = app.screen.leaf_rects.iter().find(|(id, _)| *id == focus).map(|(_, r)| *r).expect("the pane is on screen");
            let at = focus_cell(look, r);
            let lit = before[at].fg;
            open(&mut app, focus, other);
            let buf = draw(&mut app, w, h);
            let label = format!("{look}, {what}: {:?}", app.mode);
            if dims {
                let mut behind = before.clone();
                let (area, t) = (behind.area, app.theme.clone());
                dim_all(&mut app, &mut behind, area, &t);
                assert_eq!(buf[at].fg, behind[at].fg, "{label}: the focus colour, dimmed with the rest");
            } else {
                assert_eq!(buf[at].fg, lit, "{label}: the focus colour as it was");
            }
            let o = app.screen.leaf_rects.iter().find(|(id, _)| *id == other).map(|(_, r)| *r).expect("the other pane");
            assert_ne!(buf[at].fg, buf[focus_cell(look, o)].fg, "{label}: the focused pane still stands apart");
        }
    }
}

/// The pane menu of the focused pane in a split of two, drawn: the app and each item's row.
fn pane_menu(app: &mut App) -> Vec<String> {
    draw(app, 160, 45);
    let focus = app.focused().expect("a focused pane");
    app.menu_for_pane(focus, (60, 5));
    let buf = draw(app, 160, 45);
    let Mode::ScreenMenu(m) = &app.mode else { panic!("no menu: {:?}", app.mode) };
    let labels: Vec<String> = m.items.iter().map(|(l, _)| l.clone()).collect();
    let rows: Vec<String> = text(&buf).lines().map(str::to_string).collect();
    labels.iter().map(|label| rows.iter().find(|r| r.contains(label.as_str())).cloned().unwrap_or_else(|| panic!("{label} is drawn:\n{}", text(&buf)))).collect()
}

/// The keys at the right of `label`'s row, as one word.
fn key_of(rows: &[String], label: &str) -> String {
    let row = rows.iter().find(|r| r.contains(label)).unwrap_or_else(|| panic!("no {label}"));
    let after = &row[row.find(label).unwrap() + label.len()..];
    after.split('│').next().unwrap_or("").chars().filter(|c| !c.is_whitespace() && !CAP_L.contains(*c) && !CAP_R.contains(*c)).collect()
}

fn press(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    app.on_key(KeyEvent::new(code, mods));
}

#[test]
fn the_pane_menu_shows_the_real_bound_keys() {
    let mut app = split_world("floating", 2);
    let rows = pane_menu(&mut app);
    assert_eq!(key_of(&rows, "Split side by side"), "h");
    assert_eq!(key_of(&rows, "Split stacked"), "v");
    assert_eq!(key_of(&rows, "Close pane"), "x");
    assert_eq!(key_of(&rows, "Move pane right"), "Alt+→", "{rows:#?}");
    let rename = key_of(&rows, "Rename pane");
    assert!(rename.len() == 1 && !["v", "h", "x"].contains(&rename.as_str()), "no binding: a letter of its own, clear of the bound keys: {rename}");
}

#[test]
fn the_pane_menu_follows_a_rebinding() {
    let mut app = split_world("floating", 2);
    app.keymap.prefixed.retain(|_, a| *a != Action::ClosePane);
    let w = KeySpec { code: KeyCode::Char('w'), mods: KeyModifiers::NONE };
    app.keymap.prefixed.insert(w, Action::ClosePane);
    app.keymap.prefixed_order.retain(|k| k.code != KeyCode::Char('x') || !k.mods.is_empty());
    app.keymap.prefixed_order.push(w);
    let rows = pane_menu(&mut app);
    assert_eq!(key_of(&rows, "Close pane"), "w");
    press(&mut app, KeyCode::Char('w'), KeyModifiers::NONE);
    assert!(matches!(&app.mode, Mode::Confirm(c) if c.title.starts_with("Close pane")), "the new key runs it: {:?}", app.mode);
}

#[test]
fn a_bound_key_the_menu_moves_with_gives_way_to_a_letter() {
    let mut app = split_world("floating", 2);
    app.keymap.prefixed.retain(|_, a| *a != Action::SplitRight);
    let j = KeySpec { code: KeyCode::Char('j'), mods: KeyModifiers::NONE };
    app.keymap.prefixed.insert(j, Action::SplitRight);
    app.keymap.prefixed_order.retain(|k| *k != j);
    app.keymap.prefixed_order.push(j);
    let rows = pane_menu(&mut app);
    let key = key_of(&rows, "Split side by side");
    assert!(key.len() == 1 && key != "j", "j moves in the menu, so the item takes a letter: {key}");
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert!(matches!(&app.mode, Mode::ScreenMenu(m) if m.sel == 1), "j still moves: {:?}", app.mode);
    let Mode::ScreenMenu(m) = &app.mode else { unreachable!() };
    let keys = crate::client::menu::menu_chars(&app, &m.items);
    let i = m.items.iter().position(|(l, _)| l == "Split side by side").expect("the item");
    assert_eq!(keys[i].map(String::from), Some(key.clone()), "the letter shown is the one it answers to");
    press(&mut app, KeyCode::Char(key.chars().next().unwrap()), KeyModifiers::NONE);
    assert!(!matches!(app.mode, Mode::ScreenMenu(_)), "the letter shown runs it: {:?}", app.mode);
}

#[test]
fn pressing_a_shown_key_runs_its_item() {
    let mut app = split_world("floating", 2);
    pane_menu(&mut app);
    press(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    assert!(matches!(&app.mode, Mode::Confirm(c) if c.title == "Close pane 1"), "x closes this pane, after asking: {:?}", app.mode);
    let mut app = split_world("floating", 2);
    let before = app.screen.tabs[0].layout.leaves();
    pane_menu(&mut app);
    press(&mut app, KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(app.mode, Mode::Normal);
    let after = app.screen.tabs[0].layout.leaves();
    assert_eq!(after, before.iter().rev().copied().collect::<Vec<_>>(), "Alt+→ moves the pane right");
}
