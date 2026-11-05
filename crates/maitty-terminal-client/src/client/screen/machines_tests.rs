//! The Machines dialog drawn: with nothing, while it loads, with hosts and a filter, with a
//! machine in each state, asking before it removes one, and with the Windows hint. Every frame
//! is looked at at 80x24, 100x30 and 160x45.

use super::machines::*;
use super::*;
use crate::client::ids::Remote;
use crate::client::link::ConnectionState;
use crate::client::reference_fixture::{Stage, world};
use crate::client::render;
use crate::client::ssh_hosts::SshHost;
use crate::link_failure::{FIX_INSTALL, FIX_LOGIN, FIX_NO_DAEMON, FIX_UPDATE};
use maitty_settings::config::MachineDef;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::time::{Duration, Instant};

const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];

fn host(alias: &str, user: Option<&str>, name: Option<&str>) -> SshHost {
    SshHost {
        alias: alias.into(),
        host_name: name.map(Into::into),
        user: user.map(Into::into),
    }
}

fn hosts() -> Vec<SshHost> {
    vec![
        host("build", Some("dev"), Some("build.example.com")),
        host("gpu", None, Some("10.0.0.7")),
        host("web", Some("deploy"), None),
    ]
}

fn def(name: &str, target: &str) -> MachineDef {
    MachineDef {
        name: name.into(),
        target: target.into(),
        ..MachineDef::default()
    }
}

fn ago(secs: u64) -> Instant {
    Instant::now()
        .checked_sub(Duration::from_secs(secs))
        .expect("an instant that far back")
}

fn open(app: &mut App, view: MachinesView) {
    app.mode = Mode::Machines(Box::new(view));
}

fn view(hosts: Option<Vec<SshHost>>) -> MachinesView {
    MachinesView {
        hosts,
        ..MachinesView::new(false)
    }
}

/// The frame as text, the dialog's rectangle and the cells.
fn frame(app: &mut App, w: u16, h: u16) -> (String, Rect, ratatui::buffer::Buffer) {
    app.screen.modal_rects.clear();
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| render::draw(app, f)).expect("a frame");
    let buf = term.backend().buffer().clone();
    let text: String = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
                + "\n"
        })
        .collect();
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        println!("===== {w}x{h}\n{text}");
    }
    let rect = *app
        .screen
        .modal_rects
        .last()
        .expect("the dialog drew a panel");
    (text, rect, buf)
}

/// The dialog's border is whole: `│` on both sides of every row, a bottom edge with its
/// corners, and a top edge that opens with a corner and ends in the close chip.
fn assert_box_whole(text: &str, r: Rect, buf: &ratatui::buffer::Buffer) {
    let line = |y: u16| -> String { (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect() };
    for y in r.y + 1..r.bottom() - 1 {
        let row = line(y);
        assert!(
            row.starts_with('│') && row.ends_with('│'),
            "sides, row {y}:
{text}"
        );
    }
    let (top, bottom) = (line(r.y), line(r.bottom() - 1));
    assert!(
        top.starts_with(['┌', '╭']) && top.trim_end().ends_with('✕'),
        "top edge: {top:?}
{text}"
    );
    let width = r.width as usize;
    assert!(
        bottom.starts_with(['└', '╰'])
            && bottom.ends_with(['┘', '╯'])
            && bottom.chars().filter(|c| *c == '─').count() == width - 2,
        "bottom edge: {bottom:?}
{text}"
    );
}

/// The dialog's words in reading order, its border and whatever is drawn beside it gone and
/// runs of spaces made one, so a line that wrapped can be looked for whole.
fn words(text: &str, r: Rect) -> String {
    text.lines()
        .map(|l| {
            l.chars()
                .skip(r.x as usize + 1)
                .take((r.width as usize).saturating_sub(2))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn app_with(machines: &[(&str, &str)]) -> App {
    let mut app = world(Stage::None);
    app.cfg.machines = machines.iter().map(|(n, t)| def(n, t)).collect();
    app
}

fn remote(app: &mut App, id: u16, name: &str, state: ConnectionState) {
    app.remotes.insert(
        MachineId(id),
        Remote {
            name: name.into(),
            down_since: state.is_down().then(|| ago(90)),
            state,
            ..Remote::default()
        },
    );
}

#[test]
fn with_no_machine_and_no_ssh_hosts_it_says_so_and_how_to_add_one() {
    for (w, h) in SIZES {
        let mut app = app_with(&[]);
        open(&mut app, view(Some(Vec::new())));
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        for want in [
            "Machines",
            "YOURS",
            "none yet: pick a host below, or type user@host",
            "ADD",
            "no hosts in ~/.ssh/config: type user@host above",
            "type user@host, or filter the hosts below",
        ] {
            assert!(said.contains(want), "{w}x{h} lacks {want:?}:\n{text}");
        }
    }
}

#[test]
fn while_the_hosts_are_read_it_says_loading() {
    for (w, h) in SIZES {
        let mut app = app_with(&[]);
        open(&mut app, view(None));
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        assert!(
            words(&text, r).contains("loading hosts from ~/.ssh/config…"),
            "{text}"
        );
    }
}

#[test]
fn the_hosts_are_listed_as_alias_and_where_they_lead() {
    for (w, h) in SIZES {
        let mut app = app_with(&[]);
        open(&mut app, view(Some(hosts())));
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        for want in ["build dev@build.example.com", "gpu 10.0.0.7", "web deploy@"] {
            assert!(said.contains(want), "{w}x{h} lacks {want:?}:\n{text}");
        }
    }
}

#[test]
fn typing_filters_the_hosts_and_offers_the_text_as_a_host() {
    for (w, h) in SIZES {
        let mut app = app_with(&[]);
        let mut typing = view(Some(hosts()));
        typing.query = "bu".into();
        open(&mut app, typing);
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        assert!(said.contains("› bu"), "{text}");
        assert!(said.contains("build dev@build.example.com"), "{text}");
        assert!(said.contains("bu add as typed"), "{text}");
        assert!(!said.contains("gpu 10.0.0.7"), "{text}");
    }
}

#[test]
fn a_host_that_is_a_machine_already_is_not_offered_again() {
    let mut app = app_with(&[("build", "build")]);
    open(&mut app, view(Some(hosts())));
    let (text, ..) = frame(&mut app, 100, 30);
    let list = text.split("ADD").nth(1).expect("the Add side");
    assert!(!list.contains("dev@build.example.com"), "{text}");
    assert!(list.contains("gpu"), "{text}");
}

fn every_state() -> App {
    let mut app = app_with(&[
        ("alpha", "alpha"),
        ("bravo", "me@bravo.example"),
        ("charlie", "charlie"),
        ("delta", "delta"),
        ("echo", "echo"),
        ("foxtrot", "foxtrot"),
    ]);
    remote(&mut app, 1, "alpha", ConnectionState::Connecting);
    remote(&mut app, 2, "bravo", ConnectionState::Connected);
    remote(
        &mut app,
        3,
        "charlie",
        ConnectionState::NeedsLogin {
            why: FIX_LOGIN.into(),
        },
    );
    remote(
        &mut app,
        4,
        "delta",
        ConnectionState::NeedsUpdate {
            why: FIX_UPDATE.into(),
        },
    );
    remote(
        &mut app,
        5,
        "echo",
        ConnectionState::Unreachable {
            since: ago(90),
            why: "ssh: connect to host echo port 22: Connection timed out".into(),
        },
    );
    remote(&mut app, 6, "foxtrot", ConnectionState::Connected);
    app.refused.insert(
        MachineId(6),
        "a different machine answers at foxtrot (id bbbbbbbb instead of aaaaaaaa). If you \
         replaced that machine, open Machines, remove it and add it again; otherwise check the \
         host name and DNS."
            .into(),
    );
    app
}

#[test]
fn each_state_shows_its_word_and_the_exact_fix() {
    let fixes = [
        FIX_LOGIN,
        FIX_UPDATE,
        "Connection timed out",
        "open Machines, remove it and add it again",
    ];
    for (w, h) in [(100, 30), (160, 45)] {
        let mut app = every_state();
        open(&mut app, view(Some(Vec::new())));
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        for chip in [
            "connecting alpha",
            "connected bravo",
            "needs login charlie",
            "needs update delta",
            "unreachable echo",
            "refused foxtrot",
        ] {
            assert!(said.contains(chip), "{w}x{h} lacks {chip:?}:\n{text}");
        }
        for fix in fixes {
            assert!(said.contains(fix), "{w}x{h} lacks {fix:?}:\n{text}");
        }
    }
}

#[test]
fn at_80x24_the_selected_machine_s_fix_is_always_on_screen() {
    let fixes = [
        (2, "charlie", FIX_LOGIN),
        (3, "delta", FIX_UPDATE),
        (5, "foxtrot", "remove it and add it again"),
    ];
    for (sel, name, fix) in fixes {
        let mut app = every_state();
        let mut v = view(Some(Vec::new()));
        v.section = MachineSection::Machines;
        v.sel = sel;
        open(&mut app, v);
        let (text, r, buf) = frame(&mut app, 80, 24);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        assert!(said.contains(name) && said.contains(fix), "{name}:\n{text}");
    }
}

#[test]
fn the_install_and_daemon_fixes_are_shown_in_full_too() {
    let mut app = app_with(&[("one", "one"), ("two", "two")]);
    remote(
        &mut app,
        1,
        "one",
        ConnectionState::NeedsUpdate {
            why: FIX_INSTALL.into(),
        },
    );
    remote(
        &mut app,
        2,
        "two",
        ConnectionState::Unreachable {
            since: ago(5),
            why: FIX_NO_DAEMON.into(),
        },
    );
    open(&mut app, view(Some(Vec::new())));
    let (text, r, _) = frame(&mut app, 100, 30);
    let said = words(&text, r);
    assert!(
        said.contains(FIX_INSTALL) && said.contains(FIX_NO_DAEMON),
        "{text}"
    );
}

#[test]
fn removing_asks_first_and_says_the_sessions_keep_running() {
    for (w, h) in SIZES {
        let mut app = every_state();
        let mut v = view(Some(hosts()));
        v.step = MachineStep::Remove {
            name: "bravo".into(),
        };
        open(&mut app, v);
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        assert!(said.contains("Remove bravo?"), "{text}");
        assert!(
            said.contains("Removes bravo from this list. Its sessions keep running on bravo."),
            "{text}"
        );
        assert!(said.contains("Enter removes it. Esc keeps it."), "{text}");
    }
}

#[test]
fn the_windows_hint_is_said_on_windows_only() {
    for (w, h) in SIZES {
        let mut app = app_with(&[]);
        let mut v = view(Some(hosts()));
        v.windows = true;
        open(&mut app, v);
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        assert!(words(&text, r).contains(WINDOWS_HINT), "{w}x{h}:\n{text}");

        open(&mut app, view(Some(hosts())));
        let (text, r, _) = frame(&mut app, w, h);
        assert!(!words(&text, r).contains("Windows:"), "{text}");
    }
}

#[test]
fn the_steps_that_wait_say_what_they_wait_for() {
    let steps = [
        (
            MachineStep::Checking {
                target: "me@box".into(),
            },
            "Checking me@box…",
        ),
        (
            MachineStep::Name {
                target: "---".into(),
                name: "x".into(),
                error: Some("`name` is up to 32 letters".into()),
            },
            "`name` is up to 32 letters",
        ),
    ];
    for (step, want) in steps {
        for (w, h) in SIZES {
            let mut app = app_with(&[]);
            let mut v = view(Some(hosts()));
            v.step = step.clone();
            open(&mut app, v);
            let (text, r, buf) = frame(&mut app, w, h);
            assert_box_whole(&text, r, &buf);
            assert!(
                words(&text, r).contains(want),
                "{w}x{h} lacks {want:?}:\n{text}"
            );
        }
    }
}

#[test]
fn a_refusal_or_an_ssh_error_is_shown_under_the_lists() {
    let mut app = app_with(&[]);
    let mut v = view(Some(hosts()));
    v.notice = Some(MachineNotice {
        text: "box: ssh: connect to host box port 22: Connection refused".into(),
        error: true,
    });
    open(&mut app, v);
    for (w, h) in SIZES {
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        assert!(words(&text, r).contains("Connection refused"), "{text}");
    }
}

#[test]
fn long_names_and_targets_are_cut_to_the_width_and_the_border_holds() {
    let long = "h".repeat(32);
    let target = format!("someone@{}.example.com", "x".repeat(150));
    let mut app = app_with(&[(&long, &target)]);
    remote(&mut app, 1, &long, ConnectionState::Connected);
    let mut v = view(Some(vec![host(
        &"a".repeat(80),
        Some(&"u".repeat(60)),
        Some(&"n".repeat(200)),
    )]));
    v.section = MachineSection::Machines;
    open(&mut app, v);
    for (w, h) in SIZES {
        let (text, r, buf) = frame(&mut app, w, h);
        assert_box_whole(&text, r, &buf);
        assert!(text.contains('…'), "{w}x{h}:\n{text}");
        for line in text.lines() {
            assert!(line.chars().count() <= w as usize, "{line}");
        }
    }
}

#[test]
fn a_machine_name_with_control_characters_is_drawn_without_them() {
    let mut app = app_with(&[("bad", "evil\u{1b}[2Jhost")]);
    remote(
        &mut app,
        1,
        "bad",
        ConnectionState::Unreachable {
            since: ago(1),
            why: "\u{1b}[31mred\u{7}".into(),
        },
    );
    open(&mut app, view(Some(Vec::new())));
    let (text, ..) = frame(&mut app, 100, 30);
    assert!(
        !text.contains('\u{1b}') && !text.contains('\u{7}'),
        "{text:?}"
    );
    assert!(
        text.contains("evil[2Jhost") && text.contains("[31mred"),
        "{text}"
    );
}

#[test]
fn every_dialog_fits_a_tiny_window() {
    for (w, h) in [(20, 6), (1, 1), (40, 10), (60, 12), (80, 3)] {
        for windows in [false, true] {
            let mut app = every_state();
            let mut v = view(Some(hosts()));
            v.windows = windows;
            open(&mut app, v);
            app.screen_fresh();
            let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
            term.draw(|f| render::draw(&mut app, f)).expect("a frame");
        }
    }
}

fn press(app: &mut App, c: char) {
    app.on_key(ratatui::crossterm::event::KeyEvent::new(
        ratatui::crossterm::event::KeyCode::Char(c),
        ratatui::crossterm::event::KeyModifiers::NONE,
    ));
}

#[test]
fn the_leader_and_o_open_the_dialog_and_escape_closes_it() {
    let mut app = app_with(&[]);
    app.mode = Mode::Prefix {
        since: Instant::now(),
        acted: false,
    };
    press(&mut app, 'w');
    assert!(
        matches!(&app.mode, Mode::Machines(v) if v.hosts.is_none() && v.step == MachineStep::Browse)
    );
    app.on_key(ratatui::crossterm::event::KeyEvent::new(
        ratatui::crossterm::event::KeyCode::Esc,
        ratatui::crossterm::event::KeyModifiers::NONE,
    ));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn the_palette_and_the_key_overlay_list_machines_with_its_key() {
    let mut app = app_with(&[]);
    assert!(app.palette_commands().contains(&Action::Machines));
    assert_eq!(Action::Machines.describe(), "Machines");
    app.act(Action::Help);
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(160, 45)).expect("a test terminal");
    term.draw(|f| render::draw(&mut app, f)).expect("a frame");
    let buf = term.backend().buffer().clone();
    let text: String = (0..45)
        .map(|y| {
            (0..160).map(|x| buf[(x, y)].symbol()).collect::<String>()
                + "
"
        })
        .collect();
    let row = text
        .lines()
        .find(|l| l.contains("Machines"))
        .unwrap_or_else(|| {
            panic!(
                "no Machines row in the key overlay:
{text}"
            )
        });
    assert!(row.contains('w'), "{row}");
}

#[test]
fn the_windows_hint_is_only_asked_for_on_windows() {
    let mut app = app_with(&[]);
    app.open_machines();
    assert!(matches!(&app.mode, Mode::Machines(v) if v.windows == cfg!(windows)));
}
