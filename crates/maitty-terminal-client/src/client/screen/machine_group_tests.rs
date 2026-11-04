//! A second machine in the sidebar, the rail and the Inbox: its group, its state, its agents'
//! statuses as its own daemon reported them, and what a dropped line does to them.

use super::*;
use crate::client::ids::{Outgoing, Remote};
use crate::client::link::ConnectionState;
use crate::client::reference_fixture::{
    Layout, SIZES, Stage, cells, entry, git, in_repo, now, raw_text, term, workspace, world,
};
use crate::link_failure::{FIX_INSTALL, FIX_LOGIN, FIX_NO_DAEMON, FIX_UPDATE};
use crate::protocol::{
    CheckInfo, CheckState, ClientMsg, Command, HumanQuestion, Limit, ServerMsg, Snapshot,
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

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

/// The sidebar's inside at 160 columns, a row each, its selection bar blanked.
fn sidebar_rows(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| {
            let inside: String = l.chars().skip(1).take(32).collect();
            inside.replace('▎', " ").trim_end().to_string()
        })
        .collect()
}

/// The rows under the one containing `head`, down to the foot's divider.
fn group_after(rows: &[String], head: &str) -> Vec<String> {
    rows.iter()
        .skip_while(|r| !r.contains(head))
        .skip(1)
        .take_while(|r| !r.contains("────"))
        .cloned()
        .collect()
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

/// What a floating sidebar shows on each of its rows, between its border and the margin.
fn inside_rows(buf: &ratatui::buffer::Buffer, side: Rect) -> Vec<String> {
    (side.top()..side.bottom())
        .map(|y| {
            (side.x + 2..side.right() - 2)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

/// The style of the cell where `name`'s mark is drawn, two cells before it on its row.
fn mark_style(app: &mut App, name: &str) -> ratatui::style::Style {
    let buf = cells(app, Layout::Floating, 160, 45, "unicode");
    let letters: Vec<char> = name.chars().collect();
    for y in 0..buf.area.height {
        for x in 2..40u16 {
            let at = |dx: usize| buf[(x + dx as u16, y)].symbol().chars().next();
            if (0..letters.len()).all(|i| at(i) == Some(letters[i]))
                && buf[(x - 1, y)].symbol() == " "
            {
                return buf[(x - 2, y)].style();
            }
        }
    }
    panic!("no row with {name:?}");
}

#[test]
fn a_window_with_no_other_machine_draws_nothing_new() {
    let mut solo = world(Stage::None);
    let before: Vec<String> = SIZES.iter().map(|&(w, h)| frame(&mut solo, w, h)).collect();
    for text in &before {
        assert!(
            !text.contains("this computer") && !text.contains('⇄'),
            "{text}"
        );
    }
    let model = solo.screen_model();
    let lines = side::side_lines(&solo, &model, &solo.theme);
    assert!(
        lines
            .iter()
            .all(|l| !matches!(l, side::Line::Machine(_) | side::Line::MachineNote(..))),
        "{lines:?}"
    );

    solo.remotes.insert(THERE, Remote::default());
    solo.screen_fresh();
    let with = frame(&mut solo, 100, 30);
    assert!(with.contains("this computer"), "{with}");
    solo.remotes.remove(&THERE);
    solo.screen_fresh();
    let after: Vec<String> = SIZES.iter().map(|&(w, h)| frame(&mut solo, w, h)).collect();
    assert_eq!(before, after, "no machine, the frames of before");
}

#[test]
fn a_machine_nobody_has_heard_from_yet_is_a_group_that_says_connecting() {
    let mut app = world(Stage::None);
    app.remotes.insert(
        THERE,
        Remote {
            name: "build".into(),
            ..Remote::default()
        },
    );
    app.screen_fresh();
    for (w, h) in SIZES {
        let text = frame(&mut app, w, h);
        let header = row(&text, "build");
        assert!(
            header.contains("⇄ build") && header.contains('◌'),
            "{w}x{h}: {header}"
        );
        assert!(text.contains("this computer"), "{text}");
    }
    let text = frame(&mut app, 160, 45);
    assert!(row(&text, "build").contains("◌ connecting"));
    let rows = sidebar_rows(&text);
    assert!(
        group_after(&rows, "build")
            .iter()
            .all(|r| r.trim().is_empty()),
        "no sessions yet: {rows:#?}"
    );
}

#[test]
fn the_machine_group_follows_this_computers_in_every_size_and_glyph_set() {
    for glyphs in ["unicode", "ascii", "nerd"] {
        for (w, h) in SIZES {
            let mut app = two_machines(ConnectionState::Connected);
            let text = raw_text(&mut app, Layout::Floating, w, h, glyphs);
            if show() {
                println!("===== {w}x{h} {glyphs}\n{text}");
            }
            let at = |needle: &str| text.lines().position(|l| l.contains(needle)).unwrap();
            let (here, there, first) = (at("this computer"), at("build"), at("fix-auth"));
            assert!(here < there && there < first, "{glyphs} {w}x{h}\n{text}");
            let header = row(&text, "build");
            let (lead, chip) = if glyphs == "ascii" {
                ("<> build", "+ connected")
            } else {
                ("⇄ build", "● connected")
            };
            assert!(header.contains(lead), "{glyphs} {w}x{h}: {header}");
            assert!(header.contains(chip), "{glyphs} {w}x{h}: {header}");
            assert!(text.lines().all(|l| l.width() <= w as usize));
        }
    }
}

#[test]
fn working_blocked_done_idle_and_failed_read_on_a_remote_agent_as_they_do_on_this_computer() {
    let mut app = two_machines(ConnectionState::Connected);
    let text = frame(&mut app, 160, 45);
    let there = group_after(&sidebar_rows(&text), "build");
    assert!(
        there
            .iter()
            .any(|r| r.contains("asks: use redis or memcac")),
        "{there:#?}"
    );
    for (name, glyph) in [
        ("add-cache", "●"),
        ("ship-ui", "✓"),
        ("docs", "○"),
        ("lint-pass", "✕"),
    ] {
        let r = there.iter().find(|r| r.contains(name)).unwrap();
        assert!(r.contains(&format!("{glyph} {name}")), "{r}");
    }
    assert!(
        there.iter().any(|r| r.contains("failed: test")),
        "{there:#?}"
    );

    // The same snapshot as this computer's own draws the same rows, row for row.
    let mut own = world(Stage::None);
    own.snap = remote_snapshot();
    own.screen.saved.order.clear();
    own.screen_fresh();
    own.screen_sync();
    let own_rows = sidebar_rows(&frame(&mut own, 160, 45));
    let own_rows: Vec<String> = own_rows
        .into_iter()
        .skip(2)
        .take_while(|r| !r.contains("────"))
        .collect();
    let trimmed = |rows: Vec<String>| -> Vec<String> {
        let kept = rows
            .iter()
            .rposition(|r| !r.is_empty())
            .map_or(0, |i| i + 1);
        rows[..kept].to_vec()
    };
    assert_eq!(trimmed(own_rows), trimmed(there));

    let t = app.theme.clone();
    let model = app.screen_model();
    let remote: Vec<&Session> = model
        .iter()
        .filter(|p| p.machine == THERE)
        .flat_map(Proj::sessions)
        .collect();
    let local: Vec<Session> = own
        .screen_model()
        .iter()
        .flat_map(Proj::sessions)
        .cloned()
        .collect();
    assert_eq!(remote.len(), 5);
    for r in remote {
        let reported = app.remotes[&THERE].snap.terms[&r.native_term.unwrap().term].status;
        assert_eq!(
            r.status, reported,
            "{}: the daemon's word, not recomputed",
            r.name
        );
        let l = local.iter().find(|l| l.name == r.name).unwrap();
        assert_eq!(
            check::mark(&app, r, &t, "W"),
            check::mark(&own, l, &t, "W"),
            "{}",
            r.name
        );
    }
}

#[test]
fn a_dropped_line_keeps_the_agents_greyed_with_since_and_why_and_the_line_back_ungreys_them() {
    let needs = |why: &str| ConnectionState::NeedsUpdate { why: why.into() };
    let cases = [
        (
            reconnecting(),
            "reconnecting #2",
            "the connection was lost; trying again",
        ),
        (
            ConnectionState::NeedsLogin {
                why: FIX_LOGIN.into(),
            },
            "needs login",
            FIX_LOGIN,
        ),
        (needs(FIX_UPDATE), "needs update", FIX_UPDATE),
        (needs(FIX_INSTALL), "needs update", FIX_INSTALL),
        (unreachable(FIX_NO_DAEMON), "unreachable", FIX_NO_DAEMON),
        (
            unreachable("no route to host"),
            "unreachable",
            "no route to host",
        ),
    ];
    for (state, word, why) in cases {
        let mut app = two_machines(state);
        let t = app.theme.clone();
        let text = frame(&mut app, 160, 45);
        assert!(row(&text, "build").contains(word), "{word}\n{text}");
        assert!(text.contains("disconnected since 2m ago"), "{word}\n{text}");
        let shown: String = why.chars().take(20).collect();
        assert!(text.contains(&shown), "{word}: {shown}\n{text}");
        let model = app.screen_model();
        let lines = side::side_lines(&app, &model, &t);
        let reason = reason_lines(&lines, &t);
        assert!(reason.len() <= 3, "{word}: {reason:?}");
        let joined = reason.join(" ");
        match joined.strip_suffix('…') {
            Some(head) => assert!(why.starts_with(head), "{word}: {joined:?}"),
            None => assert_eq!(joined, why, "{word}: the reason is whole, wrapped"),
        }
        assert!(
            model
                .iter()
                .filter(|p| p.machine == THERE)
                .flat_map(Proj::sessions)
                .all(|s| s.down)
        );
        assert!(
            text.contains("fix-auth") && text.contains("add-cache"),
            "they stay"
        );
        for name in ["add-cache", "ship-ui", "lint-pass"] {
            let style = mark_style(&mut app, name);
            assert_eq!(style.fg, Some(t.muted), "{word}: {name} is grey");
            assert!(!style.add_modifier.contains(Modifier::BOLD));
        }
        assert_ne!(
            mark_style(&mut app, "warm-finch").fg,
            Some(t.muted),
            "this computer's rows are not touched"
        );

        app.on_link_state(THERE, "build".into(), ConnectionState::Connected);
        let text = frame(&mut app, 160, 45);
        assert!(!text.contains("disconnected since"), "{word}\n{text}");
        assert!(row(&text, "build").contains("● connected"));
        assert_eq!(
            mark_style(&mut app, "add-cache").fg,
            Some(t.blocked),
            "{word}: amber again"
        );
        assert!(app.remotes[&THERE].down_since.is_none());
    }
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
fn what_a_down_machine_last_said_is_not_counted_or_listed_as_waiting() {
    let mut app = two_machines(ConnectionState::Connected);
    let up = frame(&mut app, 160, 45);
    assert!(row(&up, "SESSIONS").contains("●2"), "{up}");
    app.on_link_state(THERE, "build".into(), reconnecting());
    let down = frame(&mut app, 160, 45);
    assert!(row(&down, "SESSIONS").contains("●1"), "{down}");
    let rows = goto_rows(&app.screen_model(), "", 0);
    assert!(
        rows.iter()
            .all(|r| r.term().is_none_or(|t| t.machine == MachineId::LOCAL))
    );
}

#[test]
fn a_down_machines_working_mark_stands_still() {
    let mut app = two_machines(reconnecting());
    let a = frame(&mut app, 160, 45);
    app.started -= Duration::from_secs(5);
    let b = frame(&mut app, 160, 45);
    assert_eq!(row(&a, "fix-auth"), row(&b, "fix-auth"));
}

#[test]
fn a_long_machine_name_and_a_long_fix_are_cut_at_every_width_inside_the_sidebar() {
    let long = "build-キーボード-and-a-very-long-machine-name-that-goes-on-and-on";
    for (w, h) in [(40, 20), (60, 24), (80, 24), (100, 30), (160, 45)] {
        for state in [
            reconnecting(),
            ConnectionState::NeedsLogin {
                why: FIX_LOGIN.into(),
            },
        ] {
            let mut app = two_machines(state);
            app.remotes.get_mut(&THERE).unwrap().name = long.into();
            app.screen_fresh();
            let buf = cells(&mut app, Layout::Floating, w, h, "unicode");
            let side = app.screen.side_rect;
            for y in side.top()..side.bottom() {
                let edge = buf[(side.right() - 1, y)].symbol();
                assert!(
                    "│┐┘╮╯".contains(edge),
                    "{w}x{h} row {y}: {edge:?} where the border is"
                );
            }
            let text = frame(&mut app, w, h);
            assert!(text.contains("build-"), "{w}x{h}\n{text}");
        }
    }
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
fn each_machines_limits_and_spend_are_its_own_and_never_added() {
    let mut app = two_machines(ConnectionState::Connected);
    let window = |name: &str, used| Limit {
        name: name.into(),
        used,
        resets_at: 0,
    };
    app.snap.spent_today = 1.5;
    app.snap.limits = vec![("claude".into(), vec![window("5h", 23.4)])];
    let there = &mut app.remotes.get_mut(&THERE).unwrap().snap;
    there.spent_today = 9.0;
    there.daemon_version = "0.0.1".into();
    there.limits = vec![("codex".into(), vec![window("week", 41.0)])];
    app.screen_fresh();
    let text = frame(&mut app, 160, 45);
    let rows = sidebar_rows(&text);
    let at = |needle: &str| rows.iter().position(|r| r.contains(needle)).unwrap();
    let (ours, theirs) = (at("this computer"), at("⇄ build"));
    assert!(rows[ours + 1].contains("claude 5h 23%"), "{rows:#?}");
    assert!(rows[ours + 2].contains("$1.50 today"), "{rows:#?}");
    assert!(rows[theirs + 1].contains("codex week 41%"), "{rows:#?}");
    assert!(
        rows[theirs + 2].contains("$9.00 today · daemon 0.0.1"),
        "{rows:#?}"
    );
    assert!(!rows[theirs + 1].contains("claude"));
    assert!(!text.contains("$10.50"), "{text}");
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
    assert_eq!(
        text.lines().filter(|l| l.contains("●B")).count(),
        1,
        "{text}"
    );
    app.on_link_state(THERE, "build".into(), reconnecting());
    let buf = cells(&mut app, Layout::Floating, 100, 30, "unicode");
    let text = frame(&mut app, 100, 30);
    let y = text
        .lines()
        .position(|l| l.contains("↻B"))
        .expect("the mark") as u16;
    // The folder's initial, then the first session.
    let below: Vec<_> = (0..5).map(|x| &buf[(x, y + 2)]).collect();
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
fn a_long_reason_wraps_onto_up_to_three_rows_at_every_sidebar_width() {
    for (w, h) in [(80, 24), (100, 30), (160, 45)] {
        let mut app = down_for_login();
        let buf = cells(&mut app, Layout::Floating, w, h, "unicode");
        let side = app.screen.side_rect;
        let shown = inside_rows(&buf, side);
        let since = shown
            .iter()
            .position(|r| r.starts_with("disconnected"))
            .unwrap_or_else(|| panic!("{w}x{h}: no since row in {shown:#?}"));
        let model = app.screen_model();
        let reason = reason_lines(&side::side_lines(&app, &model, &app.theme), &app.theme);
        assert!(
            (2..=3).contains(&reason.len()),
            "{w}x{h} (sidebar {} wide): {reason:?}",
            side.width
        );
        assert_eq!(
            shown[since + 1..since + 1 + reason.len()],
            reason[..],
            "{w}x{h}: each wrapped line is its own row, right under the since line"
        );
        let joined = reason.join(" ");
        match joined.strip_suffix('…') {
            Some(head) => assert!(FIX_LOGIN.starts_with(head), "{w}x{h}: {joined:?}"),
            None => assert_eq!(joined, FIX_LOGIN, "{w}x{h}: nothing is lost"),
        }
        for y in side.top()..side.bottom() {
            let edge = buf[(side.right() - 1, y)].symbol();
            assert!(
                "│┐┘╮╯".contains(edge),
                "{w}x{h} row {y}: {edge:?} on the border"
            );
        }
        frame(&mut app, w, h);
    }
}

#[test]
fn the_rows_under_a_wrapped_reason_shift_down_and_still_take_the_click() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let mut app = down_for_login();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    app.out = tx;
    let buf = cells(&mut app, Layout::Floating, 100, 30, "unicode");
    let shown = inside_rows(&buf, app.screen.side_rect);
    let side_top = app.screen.side_rect.top();
    let last_reason = shown
        .iter()
        .rposition(|r| r.ends_with("retry") || r.ends_with('…'))
        .expect("the reason's last row");
    let target = ClientSession::on(THERE, SessionKey::Pane(3));
    let at = app
        .hits
        .iter()
        .find(|(_, h)| *h == Hit::Screen(ScreenHit::Session(target)))
        .map(|(r, _)| *r)
        .expect("the remote row is a hit");
    assert!(
        at.y > side_top + last_reason as u16,
        "{at:?} is under the reason's last row {last_reason}"
    );
    let row_text = &shown[(at.y - side_top) as usize];
    assert!(
        row_text.contains("ship-ui"),
        "the hit is on its own row: {row_text:?}"
    );
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
        "the click lands on that session: {sent:?}"
    );
}

#[test]
fn a_resize_that_changes_the_sidebar_width_wraps_the_reason_again() {
    let mut app = down_for_login();
    let wide = {
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let side = app.screen.side_rect;
        (side.width, inside_rows(&buf, side))
    };
    let narrow = {
        let buf = cells(&mut app, Layout::Floating, 80, 24, "unicode");
        let side = app.screen.side_rect;
        (side.width, inside_rows(&buf, side))
    };
    assert!(narrow.0 < wide.0, "{} vs {}", narrow.0, wide.0);
    let reason = |rows: &[String]| -> Vec<String> {
        let since = rows
            .iter()
            .position(|r| r.starts_with("disconnected"))
            .unwrap();
        rows[since + 1..since + 4].to_vec()
    };
    let (wide_rows, narrow_rows) = (reason(&wide.1), reason(&narrow.1));
    assert_ne!(
        wide_rows, narrow_rows,
        "wrapped for the new width, not the old"
    );
    let longest = |rows: &[String]| rows.iter().map(|r| r.width()).max().unwrap();
    assert!(longest(&narrow_rows) < longest(&wide_rows));
    let back = cells(&mut app, Layout::Floating, 160, 45, "unicode");
    assert_eq!(reason(&inside_rows(&back, app.screen.side_rect)), wide_rows);
}

#[test]
fn a_dragged_sidebar_width_decides_where_the_reason_wraps() {
    let mut app = down_for_login();
    let mut longest = Vec::new();
    for side_w in [24, 30, 40] {
        app.screen.saved.side_w = Some(side_w);
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let side = app.screen.side_rect;
        assert_eq!(side.width, side_w);
        let model = app.screen_model();
        let reason = reason_lines(&side::side_lines(&app, &model, &app.theme), &app.theme);
        assert!(reason.len() <= 3 && reason.iter().all(|r| r.width() <= usize::from(side_w) - 4));
        let shown = inside_rows(&buf, side);
        assert!(
            reason.iter().all(|r| shown.contains(r)),
            "{side_w}: {reason:?} in {shown:#?}"
        );
        longest.push(reason.iter().map(|r| r.width()).max().unwrap());
    }
    assert!(
        longest[0] < longest[1] && longest[1] < longest[2],
        "{longest:?}"
    );
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
