//! Every menu, popup and side panel drawn in Hairline: no rounded shapes, no filled chips, the
//! selection a `raised` row with an accent `▎`, and clicks where the words are.

use super::tests::{draw, find_in, hairline_world, show, text};
use super::*;

const SIZES: [(u16, u16); 2] = [(80, 24), (160, 45)];

/// A surface and how to open it on the hairline world.
type Open = fn(&mut App);

fn surfaces() -> Vec<(&'static str, Open)> {
    vec![
        ("menu", |app| app.act(Action::Menu)),
        ("settings", |app| app.act(Action::Settings)),
        ("settings appearance", |app| {
            app.act(Action::Settings);
            if let Mode::ScreenSettings(v) = &mut app.mode {
                v.cat = crate::client::modal::Cat::ALL.iter().position(|c| *c == crate::client::modal::Cat::Appearance).unwrap_or(0);
            }
        }),
        ("key map", |app| app.act(Action::Palette)),
        ("all keys", |app| app.act(Action::Help)),
        ("connect agents", |app| app.act(Action::AgentStatusSetup)),
        ("row menu", |app| app.menu_for_session(4, (10, 10))),
        ("why", |app| app.mode = Mode::Why { term: 3, side: true }),
        ("message", |app| app.act(Action::Message)),
        ("confirm close", |app| app.act(Action::ClosePane)),
        ("rename", |app| app.act(Action::RenameTab)),
        ("new session dialog", |app| app.act(Action::NewSession)),
        ("history", |app| app.act(Action::History)),
        ("themes", |app| app.open_themes()),
        ("inbox", |app| app.act(Action::Jump)),
        ("files", |app| app.act(Action::Files)),
        ("changes", |app| app.act(Action::Changes)),
        ("branch", |app| app.act(Action::Branches)),
        ("references", |app| app.act(Action::References)),
        ("toast", |app| app.notify("Saved the theme".into(), false)),
        ("collapsed sidebar", |app| app.sidebar = false),
        ("splash", |app| app.splash = true),
    ]
}

/// The colours a chip or a card is filled with outside Hairline.
fn fills(t: &Theme) -> Vec<Color> {
    vec![t.accent, t.btn, t.card, t.blocked, t.danger_fill()]
}

#[test]
fn every_surface_is_square_and_unfilled() {
    let shapes = ["╭", "╮", "╰", "╯", crate::client::screen::CAP_L, crate::client::screen::CAP_R];
    let mut wrong = Vec::new();
    for (name, open) in surfaces() {
        for (w, h) in SIZES {
            let mut app = hairline_world(2);
            open(&mut app);
            let buf = draw(&mut app, w, h);
            let label = format!("{name}, {w}x{h}");
            show(&label, &buf);
            let t = app.theme.clone();
            let mine = |p: Position| !app.panes.iter().any(|(_, r)| r.contains(p));
            // The splash's wordmark is drawn in filled cells: it is a picture, not a chip.
            let filled = |c: Color| name != "splash" && fills(&t).contains(&c);
            for p in buf.area.positions().filter(|p| mine(*p)) {
                let c = &buf[(p.x, p.y)];
                if shapes.contains(&c.symbol()) || filled(c.bg) {
                    wrong.push(format!("{label}: {:?} on {:?} at {p:?}", c.symbol(), c.bg));
                    break;
                }
            }
        }
    }
    assert!(wrong.is_empty(), "rounded or filled:\n{}", wrong.join("\n"));
}

/// The mono theme's colours are named, not RGB: a chip's accent fill must still turn into its
/// words' colour, never leave black words on the terminal's own (dark) ground.
#[test]
fn named_colours_stay_readable_as_plain_words() {
    let mut unreadable = Vec::new();
    for (name, open) in surfaces() {
        for (w, h) in SIZES {
            let mut app = hairline_world(2);
            app.theme = crate::theme::Theme::named("mono");
            open(&mut app);
            let buf = draw(&mut app, w, h);
            let mine = |p: Position| !app.panes.iter().any(|(_, r)| r.contains(p));
            let dark = |c: Color| matches!(c, Color::Reset | Color::Black);
            let bad = buf.area.positions().filter(|p| mine(*p)).find(|p| {
                let c = &buf[(p.x, p.y)];
                !c.symbol().trim().is_empty() && c.fg == Color::Black && dark(c.bg)
            });
            if let Some(p) = bad {
                unreadable.push(format!("{name}, {w}x{h}: black on dark at {p:?}"));
            }
        }
    }
    assert!(unreadable.is_empty(), "{}", unreadable.join("\n"));
}

#[test]
fn a_one_line_prompt_keeps_its_frame() {
    let mut app = hairline_world(2);
    app.act(Action::RenameTab);
    let buf = draw(&mut app, 160, 45);
    show("rename", &buf);
    let t = app.theme.clone();
    let corner = buf.area.positions().find(|p| buf[(p.x, p.y)].symbol() == "┌").expect("the prompt's box");
    let below = Position { x: corner.x, y: corner.y + 1 };
    assert_eq!((buf[(below.x, below.y)].symbol(), buf[(below.x, below.y)].fg), ("│", t.line), "its side stays");
    assert!(buf.area.positions().any(|p| buf[(p.x, p.y)].symbol() == "└" && p.x == corner.x), "and its bottom");
}

#[test]
fn a_menus_selection_is_a_raised_row_with_an_accent_bar() {
    let mut app = hairline_world(1);
    app.menu_for_session(4, (10, 10));
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    let (x, y) = find_in(&buf, buf.area, "Open beside").expect("the selected row");
    assert_eq!(buf[(x, y)].bg, t.card2, "the selected row is raised");
    let marker = (buf.area.x..x).map(|mx| &buf[(mx, y)]).find(|c| ["▎", "›"].contains(&c.symbol()));
    assert_eq!(marker.map(|c| c.fg), Some(t.accent), "an accent marker on it");
    let (ox, oy) = find_in(&buf, buf.area, "Rename").unwrap();
    assert_eq!(buf[(ox, oy)].bg, t.bg, "the others are not");
}

#[test]
fn menu_rows_and_sheet_tabs_take_clicks_where_their_words_are() {
    let mut app = hairline_world(1);
    app.act(Action::Menu);
    let buf = draw(&mut app, 160, 45);
    let (x, y) = find_in(&buf, buf.area, "Settings").expect("the menu's Settings row");
    let over = app.hits.iter().rev().find(|(r, _)| r.contains(Position { x, y })).map(|(_, h)| *h);
    assert!(matches!(over, Some(Hit::Screen(ScreenHit::ActionRow(_)))), "{over:?}");
    let mut app = hairline_world(1);
    app.act(Action::Files);
    let buf = draw(&mut app, 160, 45);
    for (i, name) in ["Files", "Changes", "Branch", "References"].into_iter().enumerate() {
        let sheet = app.screen.sheet_rect;
        let (x, y) = find_in(&buf, sheet, name).unwrap_or_else(|| panic!("the {name} tab\n{}", text(&buf)));
        let over = app.hits.iter().rev().find(|(r, _)| r.contains(Position { x, y })).map(|(_, h)| *h);
        assert_eq!(over, Some(Hit::Screen(ScreenHit::DrawerTab(i))), "{name}");
    }
}

#[test]
fn the_open_sheet_tab_is_teal_with_a_rule_under_it() {
    let mut app = hairline_world(1);
    app.act(Action::Files);
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    let (x, y) = find_in(&buf, app.screen.sheet_rect, "Files").expect("the Files tab");
    assert_eq!(buf[(x, y)].fg, t.accent);
    assert_eq!(buf[(x, y)].bg, t.bg, "words, not a pill");
    assert_eq!((buf[(x, y + 1)].symbol(), buf[(x, y + 1)].fg), ("─", t.accent), "the rule under it is teal");
    let (ox, _) = find_in(&buf, app.screen.sheet_rect, "Branch").unwrap();
    assert_eq!((buf[(ox, y + 1)].symbol(), buf[(ox, y + 1)].fg), ("─", t.line), "under the others it is line");
}

#[test]
fn a_modal_floats_on_a_square_hairline_frame() {
    let mut app = hairline_world(1);
    app.act(Action::Settings);
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    let corner = buf.area.positions().find(|p| buf[(p.x, p.y)].symbol() == "┌").expect("a square corner");
    assert_eq!(buf[(corner.x, corner.y)].fg, t.line);
    assert_eq!(buf[(corner.x + 2, corner.y + 2)].bg, t.bg, "a solid ground inside");
}
