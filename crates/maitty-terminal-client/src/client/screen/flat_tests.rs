//! The sidebar (each machine with its sessions), the status glyphs, the tab row and the workspace
//! frames, held to the TUI redesign. `MAITTY_SHOW=1 cargo test <name> -- --nocapture` prints the frames.
use super::status_mark::State;
use super::*;
use crate::client::reference_fixture::{self, Stage};
use crate::layout::{Dir, Node};
use crate::protocol::ClientMsg;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc;

fn cells(app: &mut App, w: u16, h: u16) -> Buffer {
    app.cfg.ui.gap = "1".into();
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f))
        .expect("a frame");
    term.backend().buffer().clone()
}

/// `app` drawn at `w`×`h` as rows of text, trailing spaces trimmed.
fn text(app: &mut App, w: u16, h: u16) -> Vec<String> {
    let buf = cells(app, w, h);
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn show(rows: &[String], title: &str) {
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        println!("---- {title}");
        for row in rows {
            println!("{row}");
        }
    }
}

fn key(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

/// The sidebar's columns of row `y` of a frame with the full sidebar.
fn side(rows: &[String], y: usize) -> String {
    rows[y].chars().take(34).collect()
}

fn click(app: &mut App, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.on_mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        });
    }
}

/// The world with its first session split into `n` panes (2 to 4).
fn split_world(n: usize) -> App {
    let mut app = reference_fixture::world(Stage::None);
    let focus = app.focused().expect("a focused pane");
    let tab = app.screen.tab;
    let layout = &mut app.screen.tabs[tab].layout;
    layout.split(focus.term, Dir::Right, 2);
    if n > 2 {
        layout.split(2, Dir::Down, 3);
    }
    if n > 3 {
        layout.split(focus.term, Dir::Down, 4);
    }
    app
}

// ---- the sidebar ---------------------------------------------------------------------------

/// The row a session's first line is drawn on.
fn row_of(app: &App, term: u32) -> usize {
    app.screen.row_y[&ClientTerm::local(term)] as usize
}

#[test]
fn the_sidebar_lists_each_machine_with_its_sessions_two_lines_each() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    show(&rows, "140x36");
    let head = side(&rows, 3);
    assert!(head.contains(" This computer 5 ") && head.contains(" + "), "{head}");
    assert!(head.trim_end().ends_with("● │"), "the connection dot: {head}");
    let y = row_of(&app, 1);
    assert!(side(&rows, y).contains("● fix-footer"), "{}", side(&rows, y));
    assert!(
        side(&rows, y + 1).contains("■ affiliate-site fix-footer"),
        "the project square, the project and the branch: {}",
        side(&rows, y + 1)
    );
    let finch = row_of(&app, 3);
    assert!(
        side(&rows, finch + 1).trim_end_matches(['│', ' ']).ends_with("■ pixls"),
        "no repo, no branch: {}",
        side(&rows, finch + 1)
    );
    for word in ["FLEETS", "SHIPS", "inbox", "worktree", ">_", "needs you", "working"] {
        assert!(!rows.iter().any(|r| side_of(r).contains(word)), "no {word} in the sidebar");
    }
}

fn side_of(row: &str) -> String {
    row.chars().take(34).collect()
}

#[test]
fn a_row_keeps_its_place_whatever_its_status() {
    let mut app = reference_fixture::world(Stage::None);
    let order = |app: &mut App| {
        let model = app.screen_model();
        side_keys(app, &model)
    };
    let before = order(&mut app);
    for status in [Status::Working, Status::Blocked, Status::Done, Status::Idle] {
        app.snap.terms.get_mut(&5).unwrap().status = status;
        app.screen_fresh();
        assert_eq!(order(&mut app), before, "{status:?}");
    }
}

#[test]
fn a_row_shows_no_diff_counts() {
    let mut app = reference_fixture::world(Stage::None);
    let info = app.snap.terms.get_mut(&1).unwrap();
    (info.diff_added, info.diff_removed) = (104, 61);
    let rows = text(&mut app, 140, 36);
    assert!(!rows.iter().any(|r| side_of(r).contains("+104")));
}

#[test]
fn busy_agents_are_bold_idle_ones_plain_and_the_open_session_has_the_row_ground_and_bar() {
    let mut app = reference_fixture::world(Stage::None);
    let buf = cells(&mut app, 140, 50);
    let t = app.theme.clone();
    let y = |term: u32| app.screen.row_y[&ClientTerm::local(term)];
    // old-moth (5) is idle: plain text. add-payments (2) is working: bold and strong.
    assert_eq!(buf[(4, y(5))].fg, t.text);
    assert!(!buf[(4, y(5))].modifier.contains(Modifier::BOLD));
    assert_eq!(buf[(4, y(2))].fg, t.strong);
    assert!(buf[(4, y(2))].modifier.contains(Modifier::BOLD));
    // fix-footer (1) is open in the workspace: the row ground and the accent bar on both lines.
    for dy in 0..2 {
        assert_eq!(buf[(10, y(1) + dy)].bg, t.hov);
        assert_eq!((buf[(1, y(1) + dy)].symbol(), buf[(1, y(1) + dy)].fg), ("▎", t.accent));
    }
    assert_eq!(buf[(10, y(3))].bg, t.sidebar_bg, "the others sit on the panel");
}

#[test]
fn the_borders_carry_the_title_hide_release_and_menu_chips() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    let top = side(&rows, 0);
    assert!(top.contains(" maitty ") && top.contains(" « "), "{top}");
    assert!(top.find(" maitty ").unwrap() < top.find(" « ").unwrap());
    assert!(side(&rows, 1).contains("/ filter sessions"), "{}", side(&rows, 1));
    let foot = side(&rows, 34);
    let version = format!(" v{} ", crate::brand::APP_VERSION);
    assert!(foot.contains(&version) && foot.contains(" menu "), "{foot}");
    app.update_available = Some("0.99.0".into());
    let rows = text(&mut app, 140, 36);
    assert!(side(&rows, 34).contains(" ↑ v0.99.0 "), "{}", side(&rows, 34));
}

#[test]
fn the_release_chip_opens_its_menu_upward() {
    let mut app = reference_fixture::world(Stage::None);
    app.update_available = Some("0.99.0".into());
    let rows = text(&mut app, 140, 36);
    let x = side(&rows, 34).find('↑').unwrap() as u16;
    click(&mut app, x, 34);
    let Mode::ScreenMenu(menu) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    let labels: Vec<&str> = menu.items.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(labels[0], "Update to v0.99.0");
    assert!(labels.contains(&"Release notes"), "{labels:?}");
    let rows = text(&mut app, 140, 36);
    show(&rows, "release menu");
    let top = rows.iter().position(|r| r.contains("Update to v0.99.0")).unwrap();
    assert!(top < 34, "it opens above the chip");
}

#[test]
fn the_filter_matches_sessions_and_the_cross_and_esc_clear_it() {
    let mut app = reference_fixture::world(Stage::None);
    app.screen_side_set(SideItem::Sess(app.session_key(ClientTerm::local(1))));
    key(&mut app, KeyCode::Char('/'));
    for c in "pixls".chars() {
        key(&mut app, KeyCode::Char(c));
    }
    let rows = text(&mut app, 140, 36);
    show(&rows, "filtered");
    assert!(side(&rows, 1).contains("pixls"), "{}", side(&rows, 1));
    assert!(rows.iter().any(|r| side_of(r).contains("warm-finch")));
    assert!(!rows.iter().any(|r| side_of(r).contains("fix-footer")), "only the match");
    key(&mut app, KeyCode::Esc);
    let rows = text(&mut app, 140, 36);
    assert!(side(&rows, 1).contains("filter sessions"));
    assert!(rows.iter().any(|r| side_of(r).contains("fix-footer")));
    // A session's project matches too, and the cross clears what is typed.
    app.screen.filter = "affiliate".into();
    let rows = text(&mut app, 140, 36);
    assert!(rows.iter().any(|r| side_of(r).contains("add-payments")));
    let x = side(&rows, 1).chars().position(|c| c == '×').unwrap() as u16;
    click(&mut app, x, 1);
    assert!(app.screen.filter.is_empty());
}

#[test]
fn the_default_width_is_34_and_a_dragged_one_stays_between_24_and_60() {
    let mut app = reference_fixture::world(Stage::None);
    cells(&mut app, 140, 36);
    assert_eq!(app.screen.side_rect.width, 34);
    app.screen.saved.side_w = Some(80);
    cells(&mut app, 140, 36);
    assert_eq!(app.screen.side_rect.width, 60);
    app.screen.saved.side_w = Some(10);
    cells(&mut app, 140, 36);
    assert_eq!(app.screen.side_rect.width, 24);
}

#[test]
fn clicking_a_session_focuses_its_pane_wherever_the_daemon_holds_it() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    cells(&mut app, 140, 36);
    let y = app.screen.row_y[&ClientTerm::local(3)];
    click(&mut app, 8, y);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|o| matches!(&o.msg, ClientMsg::Command(Command::FocusPane { term: 3 }))),
        "{} messages, none a focus on pane 3",
        sent.len()
    );
}

#[test]
fn prefix_dot_on_a_session_opens_its_row_menu_and_anywhere_else_the_menu() {
    let mut app = reference_fixture::world(Stage::None);
    text(&mut app, 140, 36);
    app.screen_side_set(SideItem::Sess(app.session_key(ClientTerm::local(3))));
    app.act(Action::Menu);
    let Mode::ScreenMenu(menu) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert!(menu.title.contains("warm-finch"), "{}", menu.title);
    let mut app = reference_fixture::world(Stage::None);
    text(&mut app, 140, 36);
    app.act(Action::Menu);
    assert!(matches!(app.mode, Mode::Actions { .. }));
}

#[test]
fn the_plus_on_a_machine_heading_starts_a_shell_there_at_once() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    let rows = text(&mut app, 140, 36);
    let plus = side(&rows, 3).chars().position(|c| c == '+').unwrap() as u16;
    click(&mut app, plus, 3);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|o| matches!(&o.msg, ClientMsg::Command(Command::NewShip { ws: 10, cmd: None, .. }))),
        "{} messages",
        sent.len()
    );
    assert_eq!(app.mode, Mode::Normal, "no name is asked");
}

#[test]
fn a_machine_that_is_offline_gets_a_warning_and_no_shell() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    app.service_lost();
    app.new_ship_on(MachineId::LOCAL);
    assert!(std::iter::from_fn(|| rx.try_recv().ok()).next().is_none());
    assert!(app.notice.as_ref().is_some_and(|(m, _, err)| *err && m.contains("offline")));
}

#[test]
fn hiding_the_sidebar_leaves_a_show_chip_that_brings_it_back() {
    let mut app = reference_fixture::world(Stage::None);
    app.cfg.ui.motion = false;
    text(&mut app, 140, 36);
    app.act(Action::ToggleSidebar);
    let rows = text(&mut app, 140, 36);
    show(&rows, "hidden");
    assert!(rows[0].starts_with(" » "), "{}", rows[0]);
    assert!(!rows.iter().any(|r| r.contains("This computer")));
    click(&mut app, 1, 0);
    let rows = text(&mut app, 140, 36);
    assert!(side(&rows, 3).contains("This computer"));
}

/// The world with three subagents reported in add-payments' (2) current turn.
fn with_subagents() -> App {
    use crate::protocol::{SubagentActivity, SubagentState};
    let mut app = reference_fixture::world(Stage::None);
    let info = app.snap.terms.get_mut(&2).unwrap();
    let since = info.since;
    info.subagent_activity = [
        ("Check migration order", SubagentState::Completed),
        ("Write drafts tests", SubagentState::Working),
        ("Lint the routes", SubagentState::AwaitingApproval),
    ]
    .into_iter()
    .map(|(name, state)| SubagentActivity {
        id: name.into(),
        name: name.into(),
        parent: None,
        state,
        task: None,
        model: None,
        effort: None,
        started_at: Some(since + 1),
        updated_at: since + 1,
        latest_activity: None,
        result: None,
    })
    .collect();
    app.screen_fresh();
    app
}

#[test]
fn a_session_shows_its_subagents_as_marks_or_when_open_one_row_each() {
    let mut app = with_subagents();
    let rows = text(&mut app, 140, 36);
    show(&rows, "subagents, not open");
    let y = row_of(&app, 2);
    let peek = side(&rows, y + 2);
    assert!(peek.trim_start_matches('│').trim_start().starts_with('└'), "{peek}");
    assert_eq!(peek.matches('●').count(), 2, "done and needs you are dots, working spins: {peek}");
    assert!(!rows.iter().any(|r| side_of(r).contains("Lint the routes")));
    reference_fixture::focus(&mut app, 2);
    let rows = text(&mut app, 140, 36);
    show(&rows, "subagents, open");
    let y = row_of(&app, 2);
    assert!(side(&rows, y + 2).contains("├ ● Check migration order"), "{}", side(&rows, y + 2));
    assert!(side(&rows, y + 4).contains("└ ● Lint the routes"), "{}", side(&rows, y + 4));
}

#[test]
fn clicking_a_subagent_opens_its_parents_pane() {
    let mut app = with_subagents();
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    reference_fixture::focus(&mut app, 2);
    text(&mut app, 140, 36);
    let y = app.screen.row_y[&ClientTerm::local(2)] + 3;
    // Another pane is focused first, so the click has something to change.
    reference_fixture::focus(&mut app, 1);
    click(&mut app, 12, y);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|o| matches!(&o.msg, ClientMsg::Command(Command::FocusPane { term: 2 }))),
        "{sent:?}"
    );
}

// ---- the status glyphs ---------------------------------------------------------------------

#[test]
fn by_default_every_state_is_a_dot_told_apart_by_colour() {
    let app = reference_fixture::world(Stage::None);
    let t = app.theme.clone();
    let marks: Vec<_> = [State::Needs, State::Done, State::Problem, State::Idle]
        .into_iter()
        .map(|s| app.state_mark(s, &t))
        .collect();
    assert!(marks.iter().all(|m| m.glyph == "●"));
    let colours: Vec<_> = marks.iter().map(|m| m.color).collect();
    assert_eq!(colours, [t.blocked, t.done, t.err, t.idle]);
    assert_eq!(app.state_mark(State::Unknown, &t).glyph, "┆");
}

#[test]
fn while_the_daemon_is_away_every_row_shows_the_unknown_mark() {
    let mut app = reference_fixture::world(Stage::None);
    let unknown_rows = |rows: &[String]| {
        (0..rows.len())
            .filter(|y| side(rows, *y).trim_start_matches(['│', ' ']).starts_with('┆'))
            .count()
    };
    let live = text(&mut app, 140, 30);
    assert_eq!(unknown_rows(&live), 0, "a live daemon has no unknown marks:\n{}", live.join("\n"));
    app.service_lost();
    let away = text(&mut app, 140, 30);
    show(&away, "the daemon is away");
    assert!(unknown_rows(&away) >= 2, "every session's mark is the dashed one:\n{}", away.join("\n"));
}

#[test]
fn the_unicode_setting_gives_each_state_a_shape_of_its_own() {
    let mut app = reference_fixture::world(Stage::None);
    app.cfg.ui.glyphs = "unicode".into();
    let t = app.theme.clone();
    let mut glyphs: Vec<String> = [
        State::Needs,
        State::Done,
        State::Problem,
        State::Idle,
        State::Unknown,
    ]
    .into_iter()
    .map(|s| app.state_mark(s, &t).glyph)
    .collect();
    glyphs.sort();
    glyphs.dedup();
    assert_eq!(glyphs.len(), 5, "{glyphs:?}");
}

#[test]
fn a_failed_command_is_the_problem_state_and_its_row_says_so_in_red() {
    let mut app = reference_fixture::world(Stage::None);
    app.snap.terms.get_mut(&5).unwrap().exit = Some(crate::protocol::ExitInfo {
        code: Some(1),
        at: 1,
    });
    let buf = cells(&mut app, 140, 50);
    let t = app.theme.clone();
    let y = app.screen.row_y[&ClientTerm::local(5)];
    assert_eq!(buf[(2, y)].symbol(), "●");
    assert_eq!(buf[(2, y)].fg, t.err);
}

#[test]
fn the_working_spinner_is_still_with_reduce_motion_and_turns_with_it() {
    let braille: Vec<String> = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".chars().map(String::from).collect();
    assert_eq!(status_mark::frame_at(&braille, 10_000, false), "⠋");
    assert_eq!(status_mark::frame_at(&braille, 80, true), "⠙");
}

// ---- the workspace -------------------------------------------------------------------------

/// The workspace's columns of row `y`: everything right of the sidebar and its gap.
fn work(rows: &[String], y: usize) -> String {
    rows[y].chars().skip(35).collect()
}

#[test]
fn a_pane_is_a_box_with_its_title_chip_and_files_changes_and_close_at_the_right() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    show(&rows, "a lone pane");
    let top = work(&rows, 0);
    assert!(top.starts_with("╭─ ● fix-footer "), "the mark and the name in the chip: {top}");
    assert!(top.contains("affiliate-site-worktrees") && top.contains(" fix-footer "), "the folder and the branch: {top}");
    assert!(top.trim_end().ends_with(" files ─ changes ─ × ─╮"), "{top}");
    assert!(rows[1].contains("rename the footer copyright"), "the program starts under the border");
    for word in ["add-payments ×", "Files", "Changes", " + "] {
        assert!(!top.contains(word), "no tab row: {word} in {top}");
    }
}

#[test]
fn the_focused_panes_box_and_title_chip_are_the_accent() {
    let mut app = reference_fixture::world(Stage::None);
    let buf = cells(&mut app, 140, 36);
    let t = app.theme.clone();
    assert_eq!(buf[(35, 0)].fg, t.accent, "the corner");
    assert_eq!(buf[(38, 0)].bg, t.accent, "the title chip");
    assert_eq!(buf[(40, 0)].fg, t.acc_ink);
    // With the sidebar holding the keys the pane is quiet.
    app.screen_side_set(SideItem::Sess(app.session_key(ClientTerm::local(1))));
    let buf = cells(&mut app, 140, 36);
    assert_eq!(buf[(35, 0)].fg, t.line);
    assert_eq!(buf[(38, 0)].bg, t.btn);
}

#[test]
fn the_title_is_cut_short_before_the_chips_at_the_right() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 90, 24);
    let top = work(&rows, 0);
    show(&rows, "90x24");
    assert!(top.contains('…') && top.contains(" × "), "{top}");
}

#[test]
fn the_files_and_changes_chips_open_the_project_panels_and_light_while_open() {
    let mut app = reference_fixture::world(Stage::None);
    cells(&mut app, 140, 36);
    let find = |app: &App, f: fn(&ScreenHit) -> bool| {
        app.hits.iter().find_map(|(r, h)| match h {
            Hit::Screen(h) if f(h) => Some(*r),
            _ => None,
        })
    };
    let files = find(&app, |h| matches!(h, ScreenHit::FilesChip)).expect("the files chip");
    assert_eq!(files.y, 0, "on the top border");
    let t = app.theme.clone();
    let buf = cells(&mut app, 140, 36);
    assert_eq!(buf[(files.x + 1, 0)].bg, t.btn, "quiet while closed");
    click(&mut app, files.x + 2, 0);
    assert!(matches!(app.view, Some(crate::client::View::Files(_))), "it opens Files");
    let buf = cells(&mut app, 140, 36);
    let files = find(&app, |h| matches!(h, ScreenHit::FilesChip)).expect("the files chip");
    assert_eq!(buf[(files.x + 1, 0)].bg, t.accent, "the accent while open");
    click(&mut app, files.x + 2, 0);
    assert!(app.view.is_none(), "a second click closes it");
}

#[test]
fn the_close_chip_closes_its_pane() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    cells(&mut app, 140, 36);
    let close = app
        .hits
        .iter()
        .find_map(|(r, h)| matches!(h, Hit::Screen(ScreenHit::CloseSplit(t)) if t.term == 1).then_some(*r))
        .expect("the × chip");
    click(&mut app, close.x + 1, close.y);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|o| matches!(&o.msg, ClientMsg::Command(Command::CloseSession { .. } | Command::ClosePane { .. })))
            || matches!(app.mode, Mode::Confirm(_)),
        "{sent:?}"
    );
}

#[test]
fn split_panes_are_boxes_of_their_own_a_column_apart() {
    let mut app = split_world(2);
    let rows = text(&mut app, 140, 36);
    show(&rows, "2 panes");
    let top = work(&rows, 0);
    assert_eq!(top.matches('╭').count(), 2, "{top}");
    assert!(top.contains("╮ ╭"), "a column of ground between the boxes: {top}");
    assert!(top.contains("fix-footer") && top.contains("add-payments"), "each has its title");
    assert_eq!(top.matches(" × ").count(), 2);
    let all: String = rows.concat();
    for joint in ['┬', '┴', '┼'] {
        assert!(!all.contains(joint), "no joints: {joint}");
    }
    let mut app = split_world(4);
    let rows = text(&mut app, 140, 36);
    show(&rows, "4 panes");
    let corners: usize = (0..rows.len()).map(|y| work(&rows, y).matches('╭').count()).sum();
    assert_eq!(corners, 4);
}

#[test]
fn exactly_one_pane_has_the_accent_border() {
    let mut app = split_world(4);
    let buf = cells(&mut app, 140, 36);
    let t = app.theme.clone();
    let line = |c: &ratatui::buffer::Cell| "│─╭╮╰╯".contains(c.symbol());
    let area = Rect::new(35, 0, 105, 35);
    let accent: Vec<(u16, u16)> = area
        .positions()
        .filter(|p| line(&buf[(p.x, p.y)]) && buf[(p.x, p.y)].fg == t.accent)
        .map(|p| (p.x, p.y))
        .collect();
    assert!(!accent.is_empty(), "the focused pane's lines");
    let (mid_x, mid_y) = (area.x + area.width / 2, area.height / 2);
    assert!(accent.iter().all(|(x, y)| *x < mid_x && *y < mid_y), "{accent:?}");
}

#[test]
fn a_view_holds_four_panes_and_the_split_commands_say_which_side_the_new_one_goes() {
    assert_eq!(split_dir(&Action::SplitUp), Some(Dir::Up));
    assert_eq!(split_dir(&Action::SplitDown), Some(Dir::Down));
    assert_eq!(split_dir(&Action::SplitLeft), Some(Dir::Left));
    assert_eq!(split_dir(&Action::SplitRight), Some(Dir::Right));
    // Up and left insert before the pane, down and right after it.
    let mut node = Node::Leaf(1);
    node.split(1, Dir::Left, 2);
    assert_eq!(node.leaves(), [2, 1]);
    let mut node = Node::Leaf(1);
    node.split(1, Dir::Right, 2);
    assert_eq!(node.leaves(), [1, 2]);

    let mut app = split_world(4);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    text(&mut app, 140, 36);
    app.act(Action::SplitRight);
    assert!(
        std::iter::from_fn(|| rx.try_recv().ok()).next().is_none(),
        "no fifth pane is asked for"
    );
    assert!(
        app.notice
            .as_ref()
            .is_some_and(|(m, _, _)| m.contains("4 panes"))
    );
}

#[test]
fn zoom_fills_the_view_with_the_focused_pane_and_again_brings_the_split_back() {
    let mut app = split_world(2);
    text(&mut app, 140, 36);
    app.act(Action::ZoomPane);
    let rows = text(&mut app, 140, 36);
    assert_eq!(work(&rows, 0).matches('╭').count(), 1);
    app.act(Action::ZoomPane);
    let rows = text(&mut app, 140, 36);
    assert_eq!(work(&rows, 0).matches('╭').count(), 2);
}

#[test]
fn tab_keys_step_through_the_sidebars_sessions() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    text(&mut app, 140, 36);
    app.act(Action::SelectTab(3));
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|o| matches!(&o.msg, ClientMsg::Command(Command::FocusPane { term: 3 }))),
        "the third row is warm-finch: {sent:?}"
    );
}

#[test]
fn the_row_menu_lists_what_the_design_does() {
    let mut app = reference_fixture::world(Stage::None);
    text(&mut app, 140, 36);
    // Not the open session, so it can open in a split.
    let (_, items) = app.session_items(ClientTerm::local(3)).expect("a menu");
    let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Rename",
            "Pin",
            "Open in split right",
            "Open in split down",
            "Copy path",
            "Close session"
        ]
    );
}

#[test]
fn the_status_bar_says_where_the_keys_go_and_who_needs_you() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    let bar = &rows[35];
    assert!(
        bar.contains("TYPING") && bar.contains("keys go to"),
        "{bar}"
    );
    assert!(bar.contains("needs you") && bar.contains("CPU"), "{bar}");
}
