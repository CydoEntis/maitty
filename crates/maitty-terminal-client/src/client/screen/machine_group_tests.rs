//! A second machine in the sidebar, the rail and the Inbox: its group, its state, its agents'
//! statuses as its own daemon reported them, and what a dropped line does to them.

use super::*;
use crate::client::ids::{Outgoing, Remote};
use crate::client::link::ConnectionState;
use crate::client::reference_fixture::{
    Layout, SIZES, Stage, cells, entry, git, in_repo, now, raw_text, term, workspace, world,
};
use crate::link_failure::FIX_LOGIN;
use crate::protocol::{
    CheckInfo, CheckState, ClientMsg, Command, HumanQuestion, Limit, ServerMsg, Snapshot,
};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::time::{Duration, Instant};

const THERE: MachineId = MachineId(1);

fn path(p: &str) -> PathBuf {
    PathBuf::from(p)
}

/// What a build box's daemon reports: one agent in each state, in two folders. Its panes are
/// numbered from 1 like this computer's.
pub(in crate::client) fn remote_snapshot() -> Snapshot {
    let (api, web) = (path("/srv/api"), path("/srv/web"));
    let at = |mins: u64| now() - mins * 60;
    let working = in_repo(
        term(1, "fix-auth", "claude", Status::Working, &api, at(1)),
        &api,
        "main",
    );
    let asking = in_repo(
        term(2, "add-cache", "codex", Status::Blocked, &api, at(4)),
        &api,
        "main",
    );
    let done = in_repo(
        term(3, "ship-ui", "claude", Status::Done, &web, at(9)),
        &web,
        "main",
    );
    let idle = in_repo(
        term(4, "docs", "codex", Status::Idle, &web, at(30)),
        &web,
        "main",
    );
    let mut failed = in_repo(
        term(5, "lint-pass", "claude", Status::Done, &web, at(12)),
        &web,
        "main",
    );
    failed.checks = vec![CheckInfo {
        name: "test".into(),
        command: "npm test".into(),
        state: CheckState::Failed,
        exit_code: Some(1),
        first_failing_line: "x footer shows the year".into(),
        cwd: web.clone(),
        ..Default::default()
    }];
    let repo = |root: &Path, name: &str| git(root, "main", vec![entry(root, "main", true, name)]);
    Snapshot {
        terms: [working, asking, done, idle, failed]
            .into_iter()
            .map(|t| (t.id, t))
            .collect(),
        workspaces: vec![
            workspace(10, "api", &api, &[1, 2], Some(repo(&api, "api")), 0),
            workspace(20, "web", &web, &[3, 4, 5], Some(repo(&web, "web")), 1),
        ],
        active_ws: Some(10),
        questions: vec![HumanQuestion {
            id: 1,
            term: 2,
            text: "use redis or memcached?".into(),
            options: vec!["redis".into(), "memcached".into()],
        }],
        ..Snapshot::default()
    }
}

/// The reference world plus a machine called `build` in `state`, dropped two and a half minutes ago.
pub(in crate::client) fn two_machines(state: ConnectionState) -> App {
    let mut app = world(Stage::None);
    app.remotes.insert(
        THERE,
        Remote {
            name: "build".into(),
            snap: remote_snapshot(),
            down_since: state.is_down().then(|| ago(150)),
            state,
        },
    );
    app.screen_fresh();
    app.screen_sync();
    app
}

fn ago(secs: u64) -> Instant {
    Instant::now()
        .checked_sub(Duration::from_secs(secs))
        .expect("an instant that far back")
}

fn reconnecting() -> ConnectionState {
    ConnectionState::Reconnecting {
        attempt: 2,
        since: ago(150),
    }
}

fn unreachable(why: &str) -> ConnectionState {
    ConnectionState::Unreachable {
        since: ago(150),
        why: why.into(),
    }
}

fn show() -> bool {
    std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok()
}

fn frame(app: &mut App, w: u16, h: u16) -> String {
    let text = raw_text(app, Layout::Floating, w, h, "unicode");
    if show() {
        println!("===== {w}x{h}\n{text}");
    }
    text
}

/// The text of the row `needle` is on, or the frame in the panic.
fn row<'a>(text: &'a str, needle: &str) -> &'a str {
    text.lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no row with {needle:?} in\n{text}"))
}

/// The reason a down machine gives, a string per sidebar row.
fn reason_lines(lines: &[side::Line], t: &Theme) -> Vec<String> {
    lines
        .iter()
        .filter_map(|l| match l {
            side::Line::MachineNote(forms, color) if *color == t.text => forms.first().cloned(),
            _ => None,
        })
        .collect()
}

#[test]
fn since_stays_the_first_drop_when_the_failure_changes_kind() {
    let mut app = two_machines(ConnectionState::Connected);
    app.on_link_state(THERE, "build".into(), reconnecting());
    let first = app.remotes[&THERE].down_since.expect("down");
    app.on_link_state(
        THERE,
        "build".into(),
        ConnectionState::NeedsLogin { why: "x".into() },
    );
    assert_eq!(app.remotes[&THERE].down_since, Some(first));
}

#[test]
fn a_header_fits_whatever_the_room_and_the_name() {
    let t = Theme::named(crate::theme::DEFAULT);
    let names = [
        "build",
        "キーボード-machine",
        "a-very-long-name-that-goes-on-and-on",
        "😀😀😀😀😀😀",
        "",
    ];
    let marks = [
        None,
        Some(machine_group::state_mark(&reconnecting(), false, &t)),
        Some(machine_group::state_mark(
            &ConnectionState::NeedsLogin { why: String::new() },
            true,
            &t,
        )),
    ];
    for name in names {
        for mark in &marks {
            for room in 0..64 {
                let segs = machine_group::header_segs(
                    ("⇄ ", name),
                    mark.as_ref(),
                    false,
                    room,
                    &t,
                    Style::default(),
                );
                assert!(
                    segs_width(&segs) as usize <= room,
                    "{name:?} in {room}: {segs:?}"
                );
            }
        }
    }
}

#[test]
fn a_hostile_machine_renders_without_a_control_character_and_a_wild_colour_does_not_panic() {
    let hostile = "\u{1b}[2J\u{1b}]52;c;AAAA\u{7}\u{202e}\0\u{200b}x";
    let mut snap = remote_snapshot();
    snap.workspaces[0].color = u8::MAX;
    snap.workspaces[1].color = 200;
    snap.spent_today = 1e300;
    snap.limits = vec![(
        format!("{hostile}claude"),
        vec![Limit {
            name: hostile.into(),
            used: 1e30,
            resets_at: u64::MAX,
        }],
    )];
    snap.daemon_version = format!("{hostile}9.9");
    for t in snap.terms.values_mut() {
        t.label = format!("{hostile}{}", t.label);
        t.title = hostile.into();
        t.summary = hostile.into();
        t.name = hostile.into();
        t.said = hostile.into();
        t.branch = Some(hostile.into());
        t.agent = Some(format!("{hostile}claude"));
        t.since = u64::MAX;
        t.resume_at = Some(u64::MAX);
        t.quiet_since = Some(u64::MAX);
        t.usage.context = Some(1e30);
        t.cols = u16::MAX;
        t.rows = 0;
    }
    snap.questions[0].text = hostile.into();
    let mut app = two_machines(ConnectionState::Connected);
    app.remotes.get_mut(&THERE).unwrap().name = format!("{hostile}build");
    app.on_machine_server(THERE, ServerMsg::State(snap));
    assert!(
        app.remotes[&THERE]
            .snap
            .workspaces
            .iter()
            .all(|w| w.color <= crate::client::remote_snapshot::WORKSPACE_COLOR_MAX)
    );
    for (w, h) in SIZES {
        for sidebar in [true, false] {
            app.sidebar = sidebar;
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            terminal
                .draw(|f| crate::client::render::draw(&mut app, f))
                .unwrap();
            for cell in terminal.backend().buffer().content() {
                let s = cell.symbol();
                assert!(
                    !s.chars()
                        .any(|c| c.is_control() || crate::hidden::is_invisible(c)),
                    "{w}x{h}: {s:?}"
                );
            }
        }
    }
}

#[test]
fn a_snapshot_with_wild_numbers_draws_without_a_panic_even_when_it_was_never_cleaned() {
    let mut snap = remote_snapshot();
    for w in &mut snap.workspaces {
        w.color = u8::MAX;
        w.id = u32::MAX;
    }
    snap.active_ws = Some(u32::MAX);
    for t in snap.terms.values_mut() {
        t.since = u64::MAX;
        t.mem = u64::MAX;
        t.pid = u32::MAX;
        t.usage.context = Some(f32::MAX);
    }
    let mut app = two_machines(ConnectionState::Connected);
    app.remotes.get_mut(&THERE).unwrap().snap = snap;
    app.screen_fresh();
    for (w, h) in SIZES {
        frame(&mut app, w, h);
    }
}

#[test]
fn the_inbox_lists_a_remote_agent_that_needs_you_with_its_machine_and_acts_on_that_machine() {
    let mut app = two_machines(ConnectionState::Connected);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    app.out = tx;
    let asks = |app: &App| -> Vec<ClientTerm> {
        goto_rows(&app.screen_model(), "", 0)
            .iter()
            .filter_map(|r| match r {
                GoRow::Ask(t) => Some(*t),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        asks(&app),
        [ClientTerm::on(THERE, 2), ClientTerm::local(3)],
        "waiting longest first: the build box has waited 4 minutes, this computer 3"
    );

    // At the same moment this computer's comes first.
    let since = app.snap.terms[&3].since;
    let remote = app.remotes.get_mut(&THERE).unwrap();
    remote.snap.terms.get_mut(&2).unwrap().since = since;
    app.screen_fresh();
    assert_eq!(asks(&app), [ClientTerm::local(3), ClientTerm::on(THERE, 2)]);

    let rows = goto_rows(&app.screen_model(), "", 0);
    let sel = rows
        .iter()
        .position(|r| *r == GoRow::Ask(ClientTerm::on(THERE, 2)))
        .unwrap();
    app.mode = Mode::GoTo {
        query: String::new(),
        sel,
    };
    let text = frame(&mut app, 160, 45);
    assert!(
        row(&text, "add-cache · api").contains("· on build"),
        "{text}"
    );
    assert!(
        !row(&text, "warm-finch · pixls").contains(" on "),
        "this computer's item names no machine"
    );
    assert!(text.contains("use redis or memcached?"), "{text}");

    app.on_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    let sent: Vec<Outgoing> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|o| o.machine == THERE
            && matches!(o.msg, ClientMsg::Command(Command::FocusPane { term: 2 }))),
        "{sent:?}"
    );
    assert!(
        sent.iter().all(|o| o.machine == THERE),
        "nothing went to this computer: {sent:?}"
    );
}

#[test]
fn a_session_found_by_typing_names_its_machine() {
    let mut app = two_machines(ConnectionState::Connected);
    app.mode = Mode::GoTo {
        query: "docs".into(),
        sel: 0,
    };
    let text = frame(&mut app, 160, 45);
    assert!(row(&text, "docs ·").contains("on build"), "{text}");
}

#[test]
fn a_down_machines_items_are_not_in_the_inbox() {
    let mut app = two_machines(unreachable("x"));
    app.mode = Mode::GoTo {
        query: String::new(),
        sel: 0,
    };
    let text = frame(&mut app, 160, 45);
    assert!(!text.contains("add-cache · api"), "{text}");
    assert!(text.contains("warm-finch"));
}

#[test]
fn a_click_on_a_remote_row_focuses_that_machines_pane() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let mut app = two_machines(ConnectionState::Connected);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    app.out = tx;
    frame(&mut app, 160, 45);
    let target = ClientSession::on(THERE, SessionKey::Pane(3));
    let at = app
        .hits
        .iter()
        .find(|(_, h)| *h == Hit::Screen(ScreenHit::Session(target)))
        .map(|(r, _)| *r)
        .expect("the remote row is a hit");
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.on_mouse(MouseEvent {
            kind,
            column: at.x + 4,
            row: at.y,
            modifiers: KeyModifiers::NONE,
        });
    }
    let sent: Vec<Outgoing> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|o| o.machine == THERE
            && matches!(o.msg, ClientMsg::Command(Command::FocusPane { term: 3 }))),
        "{sent:?}"
    );
}

#[test]
fn the_collapsed_sidebar_marks_a_machine_and_greys_its_rows_when_down() {
    let mut app = two_machines(ConnectionState::Connected);
    let t = app.theme.clone();
    app.sidebar = false;
    let text = frame(&mut app, 100, 30);
    let marks = |text: &str| {
        text.lines()
            .filter(|l| l.chars().take(5).any(|c| c == '⇄'))
            .count()
    };
    assert_eq!(marks(&text), 1, "{text}");
    app.on_link_state(THERE, "build".into(), reconnecting());
    let buf = cells(&mut app, Layout::Floating, 100, 30, "unicode");
    let text = frame(&mut app, 100, 30);
    assert_eq!(marks(&text), 1, "{text}");
    let y = text
        .lines()
        .position(|l| l.chars().take(5).any(|c| c == '⇄'))
        .expect("the mark") as u16;
    // The first session under it.
    let below: Vec<_> = (1..4).map(|x| &buf[(x, y + 1)]).collect();
    assert!(
        below
            .iter()
            .filter(|c| c.symbol() != " ")
            .all(|c| c.fg == t.muted),
        "{below:?}"
    );
}

#[test]
fn the_hairline_look_heads_a_machine_and_greys_its_rows() {
    let mut app = two_machines(reconnecting());
    app.cfg.ui.layout = "hairline".into();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|f| crate::client::render::draw(&mut app, f))
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let rows: Vec<String> = (0..30)
        .map(|y| (0..100).map(|x| buf[(x, y)].symbol().to_string()).collect())
        .collect();
    let text = rows.join("\n");
    assert!(
        text.contains("⇄ build") && text.contains("disconnected since 2m ago"),
        "{text}"
    );
    let y = rows.iter().position(|r| r.contains("fix-auth")).unwrap();
    let x = rows[y].find("fix-auth").unwrap();
    let x = rows[y][..x].chars().count() as u16;
    assert_eq!(buf[(x, y as u16)].fg, app.theme.muted);
}

fn down_for_login() -> App {
    two_machines(ConnectionState::NeedsLogin {
        why: FIX_LOGIN.into(),
    })
}

#[test]
fn the_hairline_look_wraps_the_reason_too() {
    let mut app = down_for_login();
    app.cfg.ui.layout = "hairline".into();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|f| crate::client::render::draw(&mut app, f))
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let side = app.screen.side_rect;
    let rows: Vec<String> = (side.top()..side.bottom())
        .map(|y| {
            (side.x + 1..side.right() - 1)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();
    let model = app.screen_model();
    let reason = reason_lines(&side::side_lines(&app, &model, &app.theme), &app.theme);
    assert!(reason.len() >= 2, "it wraps: {reason:?}");
    let since = rows
        .iter()
        .position(|r| r.starts_with("disconnected"))
        .unwrap();
    assert_eq!(rows[since + 1..since + 1 + reason.len()], reason[..]);
}

// ---- the sidebar's Local and Remote halves (ADR 0050) -------------------------------------------

/// The sidebar's rows as text at 100×30, the first 30 columns.
fn sidebar_text(app: &mut App) -> String {
    raw_text(app, Layout::Floating, 100, 30, "unicode")
        .lines()
        .map(|l| l.chars().take(30).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn this_computer_is_local_and_has_no_machine_header() {
    let mut app = world(Stage::None);
    let side = sidebar_text(&mut app);
    assert!(side.contains(" LOCAL "), "{side}");
    assert!(!side.contains("SESSIONS"), "{side}");
    assert!(
        side.contains('▾') && side.contains("pixls"),
        "its projects fold: {side}"
    );
    assert!(!side.contains("⇄"), "no machine header in Local: {side}");
}

#[test]
fn with_no_other_machine_remote_says_so_and_offers_to_add_one() {
    let mut app = world(Stage::None);
    let side = sidebar_text(&mut app);
    assert!(
        side.contains("├─ REMOTE "),
        "the rule joins the card's sides: {side}"
    );
    assert!(side.contains("no remote machines"), "{side}");
    let add = app
        .hits
        .iter()
        .find(|(_, h)| *h == crate::client::Hit::Screen(ScreenHit::AddMachine))
        .map(|(r, _)| *r);
    assert!(add.is_some(), "a click opens the Machines dialog");
}

#[test]
fn a_connected_machine_is_a_header_with_its_projects_under_remote() {
    let mut app = two_machines(ConnectionState::Connected);
    let side = raw_text(&mut app, Layout::Floating, 160, 45, "unicode")
        .lines()
        .map(|l| l.chars().take(34).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    let remote = side.find("REMOTE 1").expect("REMOTE 1");
    let header = side.find("⇄ build").expect("build's header");
    assert!(header > remote, "build is below the Remote rule: {side}");
    assert!(
        side[header..].contains('▾') && side[header..].contains("api"),
        "its projects sit under it: {side}"
    );
    assert!(side.find('▾').is_some_and(|i| i < remote), "{side}");
}

#[test]
fn a_machine_whose_line_is_down_is_one_item_that_says_how_it_stands() {
    let mut app = two_machines(reconnecting());
    let side = side_columns(&mut app);
    assert!(side.contains("⇄ build"), "{side}");
    assert!(side.contains("reconnecting…"), "{side}");
    assert!(
        !side.contains("fix-auth"),
        "its cards are not drawn: {side}"
    );
    assert_eq!(machine_rows_hit(&app).len(), 2, "both rows are the machine's");

    let mut app = two_machines(ConnectionState::NeedsUpdate {
        why: crate::link_failure::FIX_UPDATE.into(),
    });
    let side = side_columns(&mut app);
    assert!(
        side.contains("needs update") && !side.contains("disconnected"),
        "the state has its own word, not a general one: {side}"
    );
    assert_eq!(
        machine_group::state_forms(&app, THERE),
        ["needs update · click for the fix", "needs update"],
        "the hint gives way to the word in a narrow sidebar"
    );
}

/// The sidebar's columns of a wide frame, as text.
fn side_columns(app: &mut App) -> String {
    raw_text(app, Layout::Floating, 160, 45, "unicode")
        .lines()
        .map(|l| l.chars().take(34).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where a click or a right-click is the other machine's, topmost target only.
fn machine_rows_hit(app: &App) -> Vec<Rect> {
    app.hits
        .iter()
        .filter(|(r, h)| {
            *h == crate::client::Hit::Screen(ScreenHit::MachineRow(THERE))
                && app
                    .hover_hit(Position::new(r.x, r.y))
                    .is_some_and(|(_, top)| top == h)
        })
        .map(|(r, _)| *r)
        .collect()
}

fn mouse(app: &mut App, button: MouseButton, at: Rect) {
    for kind in [MouseEventKind::Down(button), MouseEventKind::Up(button)] {
        app.on_mouse(MouseEvent {
            kind,
            column: at.x + 2,
            row: at.y,
            modifiers: KeyModifiers::NONE,
        });
    }
}

#[test]
fn a_right_click_on_either_row_of_a_machine_opens_its_menu() {
    for row in 0..2 {
        let mut app = two_machines(ConnectionState::Unreachable {
            since: ago(150),
            why: "no route to host".into(),
        });
        side_columns(&mut app);
        let at = machine_rows_hit(&app)[row];
        mouse(&mut app, MouseButton::Right, at);
        let Mode::ScreenMenu(menu) = &app.mode else {
            panic!("row {row}: no menu, the mode is {:?}", app.mode);
        };
        let items: Vec<&str> = menu.items.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(menu.title, "build");
        assert_eq!(
            items,
            ["Reconnect", "Update…", "Edit…", "Remove"],
            "Remove is last"
        );
    }
}

#[test]
fn a_click_on_a_machine_opens_the_dialog_on_it_with_the_fix() {
    let mut app = two_machines(ConnectionState::NeedsUpdate {
        why: crate::link_failure::FIX_UPDATE.into(),
    });
    app.cfg.machines.push(maitty_settings::config::MachineDef {
        name: "build".into(),
        target: "ci@build".into(),
        ..Default::default()
    });
    side_columns(&mut app);
    let at = machine_rows_hit(&app)[1];
    mouse(&mut app, MouseButton::Left, at);
    let Mode::Machines(view) = &app.mode else {
        panic!("no dialog, the mode is {:?}", app.mode);
    };
    assert_eq!(view.machine_sel, Some(0), "the clicked machine is chosen");
    let frame = raw_text(&mut app, Layout::Floating, 160, 45, "unicode");
    for want in [
        "MACHINES",
        "YOUR MACHINES",
        "ci@build",
        "needs update",
        "run `maitty machine update build`, then retry",
        "reconnect",
        "update",
        "remove",
    ] {
        assert!(frame.contains(want), "{want:?} is not in:\n{frame}");
    }
}

#[test]
fn a_project_has_a_line_down_its_contents_and_a_rule_divides_it_from_the_next() {
    let mut app = world(Stage::None);
    let side = sidebar_text(&mut app);
    let rows: Vec<&str> = side.lines().collect();
    let first = rows
        .iter()
        .position(|r| r.contains('▾'))
        .expect("a project");
    assert!(
        rows[first + 1].chars().nth(2) == Some('▏'),
        "a line starts under the project row: {side}"
    );
    let divider = rows
        .iter()
        .skip(first + 1)
        .position(|r| r.chars().filter(|c| *c == '─').count() > 8 && !r.contains("REMOTE"))
        .expect("a rule between two projects");
    assert!(
        rows.iter()
            .skip(first + 1 + divider + 1)
            .any(|r| r.contains('▾')),
        "the rule has a project after it: {side}"
    );
}

#[test]
fn full_rounding_makes_the_machine_badges_pills() {
    for rounded in [
        crate::config::Rounding::Corners,
        crate::config::Rounding::Full,
    ] {
        let mut app = two_machines(ConnectionState::Connected);
        app.cfg.ui.rounded = rounded;
        let text = raw_text(&mut app, Layout::Floating, 160, 45, "unicode");
        let header = text
            .lines()
            .find(|l| l.contains("⇄ build"))
            .expect("the machine's header");
        assert_eq!(
            header.contains(CAP_L),
            rounded == crate::config::Rounding::Full,
            "{rounded:?}: {header}"
        );
        let badge = text
            .lines()
            .next()
            .expect("the top border")
            .contains("⌂ Local");
        assert!(badge, "the local pane badge is on the top border");
    }
}
