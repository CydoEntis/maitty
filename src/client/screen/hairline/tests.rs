//! Hairline frames, drawn from the reference world: fix-footer and quick-fox done, add-payments
//! working, warm-finch needing you, old-moth idle.

use super::*;
use crate::client::reference_fixture::{Stage, world};
use crate::layout::Node;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];

/// The world with `panes` panes on its open tab: 1, 2 side by side, or 4 in a 2×2 split.
pub(super) fn hairline_world(panes: usize) -> App {
    let mut app = world(Stage::None);
    app.cfg.ui.layout = "hairline".into();
    app.cfg.ui.rounded = crate::config::Rounding::Full;
    let tab = &mut app.screen.tabs[0];
    let f = tab.focus;
    let others: Vec<TermId> = [1, 2, 3, 4, 5].into_iter().filter(|t| *t != f).collect();
    let leaf = |t: TermId| Box::new(Node::Leaf(t));
    let side = |a: TermId, b: TermId| Node::Split { horizontal: true, ratio: 0.5, a: leaf(a), b: leaf(b) };
    tab.layout = match panes {
        1 => Node::Leaf(f),
        2 => side(f, others[1]),
        _ => Node::Split { horizontal: false, ratio: 0.5, a: Box::new(side(f, others[1])), b: Box::new(side(others[2], others[3])) },
    };
    app
}

pub(super) fn draw(app: &mut App, w: u16, h: u16) -> Buffer {
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f)).expect("a frame");
    term.backend().buffer().clone()
}

pub(super) fn text(buf: &Buffer) -> String {
    let a = buf.area;
    (a.y..a.bottom()).map(|y| (a.x..a.right()).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end().to_string() + "\n").collect()
}

pub(super) fn show(label: &str, buf: &Buffer) {
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        println!("── {label} ──\n{}", text(buf));
    }
}

/// Every size and pane count, with `ui.rounded = full` asked for.
fn every_frame() -> impl Iterator<Item = (String, App, Buffer)> {
    [1, 2, 4].into_iter().flat_map(|panes| {
        SIZES.into_iter().map(move |(w, h)| {
            let mut app = hairline_world(panes);
            let buf = draw(&mut app, w, h);
            let label = format!("hairline, {panes} pane(s), {w}x{h}");
            show(&label, &buf);
            (label, app, buf)
        })
    })
}

/// The pane `term` was drawn at.
fn pane_at(app: &App, term: TermId) -> Rect {
    app.screen.leaf_rects.iter().find(|(id, _)| *id == term).map(|(_, r)| *r).expect("the pane is on screen")
}

/// The first place `needle` is drawn within `r`: its column and row.
pub(super) fn find_in(buf: &Buffer, r: Rect, needle: &str) -> Option<(u16, u16)> {
    (r.y..r.bottom()).find_map(|y| {
        let line: Vec<&str> = (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect();
        let wanted: Vec<String> = needle.chars().map(String::from).collect();
        line.windows(wanted.len()).position(|w| w.iter().zip(&wanted).all(|(a, b)| a == b)).map(|i| (r.x + i as u16, y))
    })
}

/// The first place `needle` is drawn anywhere.
fn find_text(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    find_in(buf, buf.area, needle)
}

#[test]
fn nothing_is_rounded_or_boxed_even_with_rounding_on() {
    let shapes = ["╭", "╮", "╰", "╯", "┌", "┐", "└", "┘", crate::client::screen::CAP_L, crate::client::screen::CAP_R];
    for (label, app, buf) in every_frame() {
        // A terminal's own output may hold any glyph; only what the app draws counts.
        let in_a_terminal = |p: Position| app.panes.iter().any(|(_, r)| r.contains(p));
        let drawn = |s: &str| buf.area.positions().any(|p| buf[(p.x, p.y)].symbol() == s && !in_a_terminal(p));
        let found: Vec<&&str> = shapes.iter().filter(|s| drawn(s)).collect();
        assert!(found.is_empty(), "{label}: {found:?}\n{}", text(&buf));
    }
}

/// The tab row over the panes: the tabs, then its rule.
fn tab_row(app: &App, buf: &Buffer) -> Rect {
    let tabs = grid(app, framed(app, buf.area)).tabs;
    assert_eq!(tabs.height, 2, "a tab row and its rule");
    Rect { height: 1, ..tabs }
}

#[test]
fn a_rule_parts_the_mode_bar_from_the_rest() {
    for (label, app, buf) in every_frame() {
        let bar = mode_bar_row(&app, buf.area);
        assert_eq!(bar.y, buf.area.bottom() - 1, "{label}");
        assert_eq!(framed(&app, buf.area).bottom(), bar.y - 1, "{label}: the layout runs down to the rule");
        let over: String = (bar.x..bar.right()).map(|x| buf[(x, bar.y - 1)].symbol()).collect();
        let edge = grid(&app, framed(&app, buf.area)).edge.x;
        assert_eq!(buf[(edge, bar.y - 1)].symbol(), "┴", "{label}: the sidebar's line meets it");
        assert_eq!(over.replace('┴', "─"), "─".repeat(bar.width as usize), "{label}: one rule, no strip: {over}");
    }
}

#[test]
fn the_tabs_have_their_own_row_over_the_panes() {
    for (label, app, buf) in every_frame() {
        let row = tab_row(&app, &buf);
        let top = app.screen.leaf_rects.iter().map(|(_, r)| r.y).min().unwrap();
        assert_eq!(top, row.y + 2, "{label}: the panes start under the tab row's rule");
        let rule: String = (row.x..row.right()).map(|x| buf[(x, row.y + 1)].symbol()).collect();
        assert_eq!(rule.replace('┴', "─"), "─".repeat(row.width as usize), "{label}");
        let (px, _) = find_in(&buf, row, "+").expect("the + is in the tab row");
        assert_eq!(buf[(px + 2, row.y)].symbol(), "│", "{label}: a line closes the +");
        assert_eq!(buf[(px + 2, row.y + 1)].symbol(), "┴", "{label}: and meets the rule");
    }
}

#[test]
fn a_pane_title_names_only_its_pane() {
    let mut app = hairline_world(1);
    let mut second = app.screen.tabs[app.screen.tab].clone();
    (second.layout, second.focus, second.name) = (Node::Leaf(2), 2, "tests".into());
    app.screen.tabs.push(second);
    let buf = draw(&mut app, 160, 45);
    let title = Rect { height: 1, ..pane_at(&app, app.focused().unwrap()) };
    assert!(find_in(&buf, title, "tests").is_none(), "another tab is not in the pane's title");
    assert!(find_in(&buf, title, "+").is_none(), "nor the +");
    assert!(find_in(&buf, tab_row(&app, &buf), "2 tests").is_some(), "the tab row has it");
}

#[test]
fn a_rule_says_the_panes_state() {
    let mut app = hairline_world(4);
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    let rule = |app: &App, term: TermId| {
        let r = pane_at(app, term);
        { let mut c: Vec<Color> = (r.x..r.right() - 1).map(|x| buf[(x, r.y)].fg).collect(); c.dedup(); c }
    };
    let focus = app.focused().expect("a focused pane");
    assert_eq!(rule(&app, focus), vec![t.accent], "focused: teal");
    assert_eq!(rule(&app, 3), vec![t.blocked], "needs you, not focused: amber");
    assert_eq!(rule(&app, 4), vec![t.line], "done, not focused: line");
    assert_eq!(rule(&app, 5), vec![t.line], "idle, not focused: line");
}

#[test]
fn sidebar_rows_mark_focus_and_needs_you() {
    let mut app = hairline_world(1);
    let buf = draw(&mut app, 100, 30);
    let t = app.theme.clone();
    let side = grid(&app, framed(&app, buf.area)).side;
    let row_of = |name: &str| find_in(&buf, side, name).map(|(_, y)| y).unwrap_or_else(|| panic!("{name} has a row"));
    let (focused, needs, plain) = (row_of("fix-footer"), row_of("warm-finch"), row_of("add-payments"));
    let marker = |y: u16| (buf[(side.x, y)].symbol().to_string(), buf[(side.x, y)].fg);
    assert_eq!(marker(focused), ("▎".into(), t.accent));
    assert_eq!(buf[(side.x + 5, focused)].bg, t.card2, "the focused row is raised");
    assert_eq!(buf[(side.x + 5, focused)].fg, t.accent, "its name is teal");
    assert_eq!(marker(needs), ("▎".into(), t.blocked));
    let (nx, _) = find_in(&buf, side, "warm-finch").unwrap();
    assert_eq!(buf[(nx, needs)].fg, t.blocked, "its name is amber");
    let (wx, _) = find_in(&buf, side, "needs you").unwrap();
    assert_eq!(buf[(wx, needs)].fg, t.blocked, "needs you replaces the time");
    assert_eq!(marker(plain).0, " ", "no marker on a plain row");
    assert_eq!(buf[(side.x + 5, plain)].bg, t.bg);
    assert_eq!(buf[(side.right(), plain)].symbol(), "│", "a hairline beside the sidebar");
}

#[test]
fn one_line_per_session_and_dim_project_headers() {
    let mut app = hairline_world(1);
    let buf = draw(&mut app, 100, 30);
    let t = app.theme.clone();
    let shown = text(&buf);
    assert!(!shown.contains("asks:") && !shown.contains("asleep") && !shown.contains("agent"), "no second lines:\n{shown}");
    let (x, y) = find_text(&buf, "orbit").unwrap();
    assert_eq!(buf[(x, y)].fg, t.muted, "the project's name is dim");
    let (bx, by) = find_text(&buf, "⎇ dev").unwrap();
    assert_eq!((by, buf[(bx, by)].fg), (y, t.muted), "its branch, dim, on its row");
    assert_eq!(text(&buf).lines().nth(y as usize - 1).map(|l| l[..26].trim().is_empty()), Some(true), "a blank row above it");
}

#[test]
fn the_foot_shows_new_and_menu_and_both_click() {
    for (w, h) in SIZES {
        let mut app = hairline_world(1);
        let buf = draw(&mut app, w, h);
        let side = grid(&app, framed(&app, buf.area)).side;
        let y = side.bottom() - 1;
        let foot: String = (side.x..side.right()).map(|x| buf[(x, y)].symbol()).collect();
        let words: Vec<&str> = foot.split_whitespace().collect();
        assert_eq!(words, ["new", "menu"], "{w}x{h}: the words alone, no keys: {foot}");
        let (nx, _) = find_in(&buf, Rect { y, height: 1, ..side }, "new").unwrap();
        assert_eq!(buf[(nx, y)].fg, app.theme.muted, "{w}x{h}: dim");
        for want in [ScreenHit::NewShell, ScreenHit::Actions] {
            let at = app.hits.iter().find(|(r, h)| *h == Hit::Screen(want) && r.y == y).map(|(r, _)| *r);
            let at = at.unwrap_or_else(|| panic!("{w}x{h}: {want:?} is clickable"));
            let word: String = (at.x..at.right()).map(|x| buf[(x, y)].symbol()).collect();
            assert_eq!(word, if want == ScreenHit::NewShell { "new" } else { "menu" }, "{w}x{h}: the click covers its word");
        }
    }
}

#[test]
fn a_check_to_approve_needs_you_everywhere() {
    let mut app = hairline_world(4);
    let approve = crate::client::reference_fixture::world(Stage::Approve).snap.terms[&1].checks.clone();
    let fox = app.snap.terms.get_mut(&4).unwrap();
    (fox.status, fox.checks) = (Status::Done, approve);
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    let r = pane_at(&app, 4);
    assert_eq!(buf[(r.x + 1, r.y)].fg, t.blocked, "its rule is amber");
    let bar = mode_bar_row(&app, buf.area);
    assert!(find_in(&buf, bar, "● 2 needs you").is_some(), "counted with warm-finch in the bar");
}

#[test]
fn a_split_row_names_the_pane_that_needs_you() {
    let mut app = hairline_world(2);
    let buf = draw(&mut app, 160, 45);
    let side = grid(&app, framed(&app, buf.area)).side;
    let (_, y) = find_in(&buf, side, "fix-footer").expect("the split's row");
    let row: String = (side.x..side.right()).map(|x| buf[(x, y)].symbol()).collect();
    assert!(row.contains("fix-footer") && row.contains("▸ warm-finch"), "{row}");
}

#[test]
fn a_helper_steps_in_and_keeps_its_ask_line() {
    let mut app = hairline_world(1);
    let info = app.snap.terms.get_mut(&5).unwrap();
    info.started_by = Some(4);
    info.helper_ask = vec!["one".into(), "two".into()];
    let buf = draw(&mut app, 160, 45);
    show("a helper", &buf);
    let side = grid(&app, framed(&app, buf.area)).side;
    let (fox, _) = find_in(&buf, side, "quick-fox").unwrap();
    let (moth, _) = find_in(&buf, side, "old-moth").unwrap();
    assert_eq!(moth, fox + super::super::side::HELPER_INDENT, "a helper steps in");
    let (_, ask) = find_in(&buf, side, "helpers").expect("the wants-helpers line");
    assert!(app.hits.iter().any(|(hr, h)| *h == Hit::Screen(ScreenHit::HelperAsk(5)) && hr.y == ask), "it takes the click");
}

/// What a click at (`x`, `y`) lands on: the last region drawn there.
fn hit_at(app: &App, x: u16, y: u16) -> Option<Hit> {
    app.hits.iter().rev().find(|(r, _)| r.contains(Position { x, y })).map(|(_, h)| *h)
}

#[test]
fn the_stacked_divider_is_grabbed_on_the_line_drawn() {
    let mut app = hairline_world(4);
    let buf = draw(&mut app, 160, 45);
    let r = pane_at(&app, app.focused().unwrap());
    // The pane below draws its top rule on the row under this pane: that is the line, and the grab.
    let y = r.bottom();
    assert_eq!(buf[(r.x + 3, y)].symbol(), "─");
    assert!(matches!(hit_at(&app, r.x + 3, y), Some(Hit::Screen(ScreenHit::Divider(_)))), "{:?}", hit_at(&app, r.x + 3, y));
    assert!(!matches!(hit_at(&app, r.x + 3, y - 1), Some(Hit::Screen(ScreenHit::Divider(_)))), "the terminal row over it is not the grab");
}

#[test]
fn a_lone_pane_has_no_title_row_and_its_tab_carries_its_close() {
    let mut app = hairline_world(1);
    let buf = draw(&mut app, 160, 45);
    let focus = app.focused().unwrap();
    let r = pane_at(&app, focus);
    let (_, inner) = *app.panes.iter().find(|(id, _)| *id == focus).unwrap();
    assert_eq!(inner.y, r.y, "the terminal starts at the pane's top: no title row, no rule");
    let row = tab_row(&app, &buf);
    let (x, y) = find_in(&buf, row, "×").expect("the tab's ✕");
    assert_eq!(hit_at(&app, x, y), Some(Hit::Screen(ScreenHit::CloseSplit(focus))), "it closes the only pane");
    let mut second = app.screen.tabs[app.screen.tab].clone();
    (second.layout, second.focus) = (Node::Leaf(2), 2);
    app.screen.tabs.push(second);
    let buf = draw(&mut app, 160, 45);
    let (x, y) = find_in(&buf, tab_row(&app, &buf), "×").unwrap();
    assert_eq!(hit_at(&app, x, y), Some(Hit::Screen(ScreenHit::CloseTab(app.screen.tab))), "with several tabs it closes the tab");
}

#[test]
fn a_sessions_tabs_are_plain_words_the_current_one_teal() {
    let mut app = hairline_world(1);
    let mut second = app.screen.tabs[app.screen.tab].clone();
    (second.layout, second.focus, second.name) = (Node::Leaf(2), 2, "tests".into());
    app.screen.tabs.push(second);
    app.screen.tabs[app.screen.tab].name = "main".into();
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    let title = tab_row(&app, &buf);
    let (cx, _) = find_in(&buf, title, " main").expect("the current tab");
    let (ox, _) = find_in(&buf, title, "tests").expect("the other tab");
    let cx = cx + 1;
    assert_eq!(buf[(cx, title.y)].fg, t.accent);
    assert!(buf[(cx, title.y)].modifier.contains(Modifier::BOLD));
    assert_eq!(buf[(ox, title.y)].fg, t.muted);
    assert_eq!(buf[(cx, title.y)].bg, t.bg, "no pill");
    assert!(app.hits.iter().any(|(r, h)| matches!(h, Hit::Screen(ScreenHit::TabPick(_))) && r.y == title.y), "a tab takes a click");
    show("two tabs", &buf);
}

#[test]
fn every_theme_draws_hairline_from_its_own_roles() {
    for name in crate::theme::BUILTIN {
        let mut app = hairline_world(2);
        app.theme = crate::theme::Theme::named(name);
        let t = app.theme.clone();
        let buf = draw(&mut app, 100, 30);
        let r = pane_at(&app, app.focused().unwrap());
        assert_eq!(buf[(r.x + 1, r.y)].fg, t.accent, "{name}: the focused rule");
        let other = pane_at(&app, 3);
        assert_eq!(buf[(other.x + 1, other.y)].fg, t.blocked, "{name}: needs you");
        let bar = mode_bar_row(&app, buf.area).y;
        assert!((0..buf.area.width).any(|x| buf[(x, bar)].fg == t.blocked), "{name}: the bar says needs you");
    }
}

#[test]
fn the_bar_keeps_shortcut_mode_and_names_what_has_the_keys() {
    let mut app = hairline_world(2);
    let t = app.theme.clone();
    app.mode = Mode::Prefix { since: std::time::Instant::now(), acted: false };
    let buf = draw(&mut app, 160, 45);
    show("shortcut mode", &buf);
    let bar = mode_bar_row(&app, buf.area);
    let row = |buf: &Buffer| (bar.x..bar.right()).map(|x| buf[(x, bar.y)].symbol()).collect::<String>();
    assert!(row(&buf).contains("SHORTCUT") && row(&buf).contains("shell"), "{}", row(&buf));
    assert_eq!(buf[(bar.right() - 2, bar.y)].bg, t.sky(), "the whole row lit, as in every look");
    assert!(!row(&buf).contains("needs you"), "no amber on the lit row");
    app.mode = Mode::Side;
    let buf = draw(&mut app, 160, 45);
    show("sidebar has the keys", &buf);
    let (x, _) = find_in(&buf, bar, "SIDEBAR").expect("the holder");
    assert_eq!((buf[(x, bar.y)].fg, buf[(x, bar.y)].bg), (t.strong, t.bg), "a bold word, no chip");
    assert!(row(&buf).contains("● warm-finch needs you"), "{}", row(&buf));
}

#[test]
fn working_holds_still_without_motion() {
    let mut app = hairline_world(1);
    let buf = draw(&mut app, 100, 30);
    let (x, y) = find_text(&buf, "add-payments").unwrap();
    assert_eq!(buf[(x - 2, y)].symbol(), STILL_WORKING);
}

#[test]
fn the_bar_counts_who_needs_you_and_keeps_update_at_its_end() {
    let mut app = hairline_world(1);
    app.update_available = Some("9.9.9".into());
    let buf = draw(&mut app, 160, 45);
    let bar = mode_bar_row(&app, buf.area);
    let row: String = (bar.x..bar.right()).map(|x| buf[(x, bar.y)].symbol()).collect::<String>().trim_end().to_string();
    assert!(row.starts_with(" TYPING  tab 1/1 · 1 split"), "{row}");
    assert!(row.contains("● warm-finch needs you") && row.ends_with("↑ Update 9.9.9"), "Update keeps the right end, needs you before it: {row}");
    app.snap.terms.get_mut(&3).unwrap().status = Status::Done;
    app.snap.questions.clear();
    app.update_available = None;
    let buf = draw(&mut app, 160, 45);
    let row: String = (bar.x..bar.right()).map(|x| buf[(x, bar.y)].symbol()).collect::<String>();
    assert!(row.contains("all clear"), "{row}");
}

#[test]
fn a_lone_tab_still_offers_a_plus_that_opens_a_tab() {
    let mut app = hairline_world(1);
    let buf = draw(&mut app, 120, 30);
    let row = tab_row(&app, &buf);
    assert!(find_in(&buf, row, "1 ").is_none(), "a lone tab has no number");
    let (x, y) = find_in(&buf, row, "+").expect("a + beside the only tab");
    assert!(app.hits.iter().any(|(r, h)| *h == Hit::Screen(ScreenHit::NewTab) && r.x == x && r.y == y), "the + opens a tab");
    show("one tab", &buf);
}

#[test]
fn a_tab_out_of_view_shows_its_agents_state() {
    let mut app = hairline_world(1);
    let mut second = app.screen.tabs[app.screen.tab].clone();
    (second.layout, second.focus, second.name) = (Node::Leaf(2), 2, "tests".into());
    app.screen.tabs.push(second);
    let buf = draw(&mut app, 160, 45);
    let (x, y) = find_in(&buf, tab_row(&app, &buf), "2 tests").expect("the other tab");
    assert_eq!(buf[(x - 2, y)].fg, app.theme.accent, "its working agent's mark leads it, teal");
    assert_ne!(buf[(x - 2, y)].symbol(), " ");
    show("a working tab out of view", &buf);
}

#[test]
fn a_toast_is_a_round_outline_in_floating_and_a_solid_block_in_hairline() {
    for (look, above, ground) in [("hairline", "┃", "card2"), ("floating", "╭", "bg")] {
        let mut app = hairline_world(1);
        app.cfg.ui.layout = look.into();
        app.notify("Saved the theme".into(), false);
        let buf = draw(&mut app, 160, 45);
        show(look, &buf);
        let t = app.theme.clone();
        let want = if ground == "bg" { t.bg } else { t.card2 };
        let (x, y) = find_text(&buf, "Saved the theme").expect("the toast");
        assert_eq!(buf[(x, y)].bg, want, "{look}: its words on its ground");
        let left = (0..x).rev().find(|&xx| buf[(xx, y)].symbol() == "┃").expect("its coloured left side");
        assert_eq!(buf[(left, y)].fg, t.accent, "{look}: the side in the notice's colour");
        assert_eq!(buf[(left, y - 1)].symbol(), above, "{look}: a round corner over the side, or the side runs on");
        if look == "floating" {
            assert_eq!(buf[(left, y - 1)].fg, t.line, "the outline in `line`");
        } else {
            assert!(!(0..160u16).any(|xx| ["┌", "┐", "└", "┘"].contains(&buf[(xx, y - 1)].symbol())), "no box in hairline");
        }
    }
}

#[test]
fn a_split_pane_has_a_rule_and_no_title_and_the_rule_focuses_it() {
    let mut app = hairline_world(2);
    let buf = draw(&mut app, 160, 45);
    let r = pane_at(&app, 3);
    let top: String = (r.x..r.right()).map(|x| buf[(x, r.y)].symbol()).collect();
    assert!(top.trim_end_matches('│').chars().all(|c| c == '─'), "the top row is only the rule: {top}");
    let (_, inner) = *app.panes.iter().find(|(id, _)| *id == 3).unwrap();
    assert_eq!(inner.y, r.y + 1, "the terminal starts under the rule");
    assert_eq!(hit_at(&app, r.x + 3, r.y), Some(Hit::Pane(3)), "the rule focuses the pane");
}

#[test]
fn the_tab_row_ends_in_split_and_zoom_and_zoom_fills_the_tab_with_the_focused_pane() {
    // Floating has no buttons: the tabs sit on the pane's border; splitting and zooming are keys.
    for look in ["hairline"] {
        let mut app = hairline_world(2);
        app.cfg.ui.layout = look.into();
        let buf = draw(&mut app, 160, 45);
        let row = grid(&app, framed(&app, buf.area)).tabs;
        let tools: String = (row.right().saturating_sub(8)..row.right()).map(|x| buf[(x, row.y)].symbol()).collect();
        assert!(tools.contains('◫') && tools.contains('⤢'), "{look}: split and zoom at the row's right end: {tools}");
        assert_eq!(app.screen.leaf_rects.len(), 2, "{look}: two panes before zoom");
        for want in [1, 2] {
            let at = app.hits.iter().find(|(_, h)| *h == Hit::Screen(ScreenHit::TabZoom)).map(|(r, _)| *r).expect("zoom is clickable");
            app.on_mouse(crossterm::event::MouseEvent { kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left), column: at.x + 1, row: at.y, modifiers: crossterm::event::KeyModifiers::NONE });
            draw(&mut app, 160, 45);
            assert_eq!(app.screen.leaf_rects.len(), want, "{look}: zoom toggles the focused pane to fill the tab, and back");
        }
    }
}

#[test]
fn the_line_under_the_sidebar_or_the_tabs_is_teal_where_the_keys_are() {
    let mut app = hairline_world(1);
    let t = app.theme.clone();
    for (side_has_keys, label) in [(false, "a lone pane has the keys"), (true, "the sidebar has the keys")] {
        app.mode = if side_has_keys { Mode::Side } else { Mode::Normal };
        let buf = draw(&mut app, 160, 45);
        let g = grid(&app, framed(&app, buf.area));
        assert_eq!(g.side.y, g.tabs.y, "the sidebar's header and the tab row sit on one row");
        let (side_rule, tab_rule) = (&buf[(g.side.x + 3, g.side.y + 1)], &buf[(g.tabs.x + 3, g.tabs.y + 1)]);
        assert_eq!((side_rule.symbol(), tab_rule.symbol()), ("─", "─"), "{label}: both rules are drawn on the same row");
        assert_eq!(side_rule.fg, if side_has_keys { t.accent } else { t.line }, "{label}: the sidebar's line");
        assert_eq!(tab_rule.fg, if side_has_keys { t.line } else { t.accent }, "{label}: the tab row's line");
    }
}

#[test]
fn in_floating_the_tabs_sit_on_the_first_pane_s_top_border_level_with_the_sidebar_s_top() {
    for panes in [1, 2] {
        let mut app = hairline_world(panes);
        app.cfg.ui.layout = "floating".into();
        let buf = draw(&mut app, 160, 45);
        let g = grid(&app, framed(&app, buf.area));
        let top = app.pane_frames.iter().map(|(_, r)| r.y).min().unwrap();
        assert_eq!(g.tabs.height, 0, "{panes} pane(s): the tabs have no row of their own");
        assert_eq!(top, g.side.y, "{panes} pane(s): the pane's top border is on the sidebar's top row");
        let row: String = (g.panes.x..g.panes.right()).map(|x| buf[(x, top)].symbol()).collect();
        assert!(row.contains("fix-footer") && !row.contains('⤢') && !row.contains('◫'), "{panes} pane(s): the tabs are on the border, with no split or zoom buttons: {row}");
        assert!(row.starts_with(['┌', '╭']) && row.ends_with(['┐', '╮']), "{panes} pane(s): and the border is still one line: {row}");
    }
}
