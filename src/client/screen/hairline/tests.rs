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

#[test]
fn the_bottom_is_the_strip_then_the_bar() {
    for (label, app, buf) in every_frame() {
        let bar = mode_bar_row(&app, buf.area);
        assert_eq!(bar.y, buf.area.bottom() - 1, "{label}");
        assert_eq!(framed(&app, buf.area).bottom(), bar.y - 1, "{label}: the layout ends over the strip");
        let strip: String = (0..buf.area.width).map(|x| buf[(x, bar.y - 1)].symbol()).collect();
        assert!(strip.starts_with('━'), "{label}: {strip}");
    }
}

#[test]
fn a_rule_says_the_panes_state() {
    let mut app = hairline_world(4);
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    let rule = |app: &App, term: TermId| {
        let r = pane_at(app, term);
        { let mut c: Vec<Color> = (r.x..r.right() - 1).map(|x| buf[(x, r.y + 1)].fg).collect(); c.dedup(); c }
    };
    let focus = app.focused().expect("a focused pane");
    assert_eq!(rule(&app, focus), vec![t.accent], "focused: teal");
    assert_eq!(rule(&app, 3), vec![t.blocked], "needs you, not focused: amber");
    assert_eq!(rule(&app, 4), vec![t.line], "done, not focused: line");
    assert_eq!(rule(&app, 5), vec![t.line], "idle, not focused: line");
}

#[test]
fn the_strip_has_a_segment_per_session_in_its_state_colour() {
    let mut app = hairline_world(1);
    let buf = draw(&mut app, 100, 30);
    let t = app.theme.clone();
    let y = mode_bar_row(&app, buf.area).y - 1;
    let mut runs: Vec<(Color, u16)> = Vec::new();
    for x in 0..buf.area.width {
        let c = &buf[(x, y)];
        match (c.symbol(), runs.last_mut()) {
            ("━", Some((fg, n))) if *fg == c.fg && buf[(x - 1, y)].symbol() == "━" => *n += 1,
            ("━", _) => runs.push((c.fg, 1)),
            _ => {}
        }
    }
    // Sidebar order: warm-finch (pixls), fix-footer and add-payments, quick-fox and old-moth.
    let colours: Vec<Color> = runs.iter().map(|(c, _)| *c).collect();
    assert_eq!(colours, [t.blocked, t.done, t.accent, t.done, t.muted]);
    let widths: Vec<u16> = runs.iter().map(|(_, n)| *n).collect();
    assert!(widths.iter().max().unwrap() - widths.iter().min().unwrap() <= 1, "equal shares: {widths:?}");
    assert_eq!(widths.iter().sum::<u16>() + 4, buf.area.width, "full width, a column between segments");
}

#[test]
fn many_sessions_fall_back_to_a_cell_each() {
    let sessions: Vec<(TermId, Color)> = (1..=20).map(|i| (i, Color::Reset)).collect();
    let segs = super::strip::segments(&sessions, Rect::new(0, 0, 160, 1));
    assert_eq!(segs.len(), 20);
    assert!(segs.iter().enumerate().all(|(i, &(_, _, x, w))| w == 1 && x == 2 * i as u16), "left-aligned, one cell each");
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
fn the_foot_names_the_real_keys_and_takes_clicks() {
    let mut app = hairline_world(1);
    app.keymap.prefixed.retain(|_, a| *a != Action::ShellHere);
    app.keymap.prefixed.insert(KeySpec { code: KeyCode::Char('w'), mods: KeyModifiers::NONE }, Action::ShellHere);
    app.keymap.prefixed_order.push(KeySpec { code: KeyCode::Char('w'), mods: KeyModifiers::NONE });
    let buf = draw(&mut app, 100, 30);
    let side = grid(&app, framed(&app, buf.area)).side;
    let foot: String = (side.x..side.right()).map(|x| buf[(x, side.bottom() - 1)].symbol()).collect();
    assert!(foot.contains("Ctrl+Space w new") && foot.contains("m menu"), "{foot}");
    for want in [ScreenHit::NewShell, ScreenHit::Actions] {
        assert!(app.hits.iter().any(|(r, h)| *h == Hit::Screen(want) && r.y == side.bottom() - 1), "{want:?} is clickable");
    }
}

#[test]
fn the_name_stays_whole_while_the_meta_gives_way() {
    let mut app = hairline_world(2);
    let buf = draw(&mut app, 100, 30);
    let t = app.theme.clone();
    let r = pane_at(&app, 1);
    let title: String = (r.x..r.right()).map(|x| buf[(x, r.y)].symbol()).collect();
    assert!(title.contains("fix-footer") && title.contains('…') && title.contains("done"), "{title}");
    let (x, y) = find_in(&buf, Rect { height: 1, ..r }, "● fix-footer").unwrap();
    assert_eq!(x, r.x + PANE_PAD, "the glyph leads the title: no number before it");
    let name = &buf[(x + 2, y)];
    assert_eq!(name.fg, t.strong);
    assert!(name.modifier.contains(Modifier::BOLD));
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
    let (x, _) = find_in(&buf, Rect { height: 1, ..r }, "needs you · approve check").expect("its state word");
    assert_eq!(buf[(x, r.y)].fg, t.blocked);
    assert_eq!(buf[(r.x + 1, r.y + 1)].fg, t.blocked, "its rule is amber");
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
    let r = pane_at(&app, 1);
    assert!(find_in(&buf, Rect { height: 1, ..r }, "done").is_some(), "its own pane is done");
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
    let y = r.bottom() - 1;
    assert_eq!(buf[(r.x + 3, y)].symbol(), "─");
    assert!(matches!(hit_at(&app, r.x + 3, y), Some(Hit::Screen(ScreenHit::Divider(_)))), "{:?}", hit_at(&app, r.x + 3, y));
    assert!(!matches!(hit_at(&app, r.x + 3, y - 1), Some(Hit::Screen(ScreenHit::Divider(_)))), "the terminal row over it is not the grab");
}

#[test]
fn title_controls_win_and_the_rest_of_the_title_focuses() {
    let mut app = hairline_world(2);
    let buf = draw(&mut app, 160, 45);
    let r = pane_at(&app, 3);
    let title = Rect { height: 1, ..r };
    let (cx, _) = find_in(&buf, title, "✕").unwrap();
    assert_eq!(hit_at(&app, cx, r.y), Some(Hit::Screen(ScreenHit::CloseSplit(3))));
    let (wx, _) = find_in(&buf, title, "needs you").unwrap();
    assert_eq!(hit_at(&app, wx, r.y), Some(Hit::Screen(ScreenHit::Why(3))));
    assert_eq!(hit_at(&app, wx - 3, r.y), Some(Hit::Pane(3)), "blank title space focuses the pane");
}

#[test]
fn a_title_says_its_state_in_words() {
    let mut app = hairline_world(4);
    let buf = draw(&mut app, 160, 45);
    let t = app.theme.clone();
    for (term, word, colour) in [(1, "done", t.done), (3, "needs you · 3m", t.blocked), (5, "idle", t.muted)] {
        let r = pane_at(&app, term);
        let title: String = (r.x..r.right()).map(|x| buf[(x, r.y)].symbol()).collect();
        let at = title.find(word).unwrap_or_else(|| panic!("{word} on {term}: {title}"));
        assert_eq!(buf[(r.x + title[..at].chars().count() as u16, r.y)].fg, colour, "{word}");
    }
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
    let title = Rect { height: 1, ..pane_at(&app, app.focused().unwrap()) };
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
        assert_eq!(buf[(r.x + 1, r.y + 1)].fg, t.accent, "{name}: the focused rule");
        let other = pane_at(&app, 3);
        assert_eq!(buf[(other.x + 1, other.y + 1)].fg, t.blocked, "{name}: needs you");
        let strip = mode_bar_row(&app, buf.area).y - 1;
        assert!((0..buf.area.width).any(|x| buf[(x, strip)].fg == t.blocked), "{name}: the strip says needs you");
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
    assert!(row(&buf).trim_end().ends_with("● 1 needs you"), "{}", row(&buf));
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
    assert!(row.starts_with(" TYPING  Hairline · 1 pane"), "{row}");
    assert!(row.ends_with("● 1 needs you   ↑ Update 9.9.9"), "Update keeps the right end, needs you just before it: {row}");
    app.snap.terms.get_mut(&3).unwrap().status = Status::Done;
    app.snap.questions.clear();
    app.update_available = None;
    let buf = draw(&mut app, 160, 45);
    let row: String = (bar.x..bar.right()).map(|x| buf[(x, bar.y)].symbol()).collect::<String>();
    assert!(row.trim_end().ends_with("all clear"), "{row}");
}
