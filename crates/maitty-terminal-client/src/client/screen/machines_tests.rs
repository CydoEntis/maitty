//! The Add Machine dialog drawn: loading, with hosts and a typed target, checking, added in each
//! way and naming. Every frame is looked at at 80x24, 100x30 and 160x45.

use super::machines::*;
use super::*;
use crate::client::reference_fixture::{Stage, world};
use crate::client::render;
use crate::client::ssh_hosts::SshHost;
use maitty_settings::config::MachineDef;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

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
        top.starts_with(['┌', '╭']) && top.trim_end().trim_end_matches(['┐', '╮']).trim_end().ends_with('×'),
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

fn shown(
    app: &mut App,
    v: MachinesView,
    w: u16,
    h: u16,
) -> (String, Rect, ratatui::buffer::Buffer) {
    open(app, v);
    frame(app, w, h)
}

#[test]
fn it_asks_for_the_machine_and_lists_the_hosts_with_the_chosen_one_marked() {
    for (w, h) in SIZES {
        let mut app = app_with(&[]);
        let (text, r, buf) = shown(&mut app, view(Some(hosts())), w, h);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        for want in [
            "ADD MACHINE",
            "Where's the machine? Type it like scp or ssh.",
            "FROM ~/.ssh/config",
            "build dev@build.example.com",
            "Enter connect",
            "↑↓ input / hosts",
            "Esc cancel",
        ] {
            assert!(said.contains(want), "{w}x{h}: {want:?}\n{text}");
        }
        assert!(
            !said.contains("YOURS") && !said.contains("Machines"),
            "{w}x{h}: no list of machines"
        );
    }
}

#[test]
fn the_parse_line_reads_user_and_host_and_fills_an_alias_from_ssh_config() {
    let mut app = app_with(&[]);
    let typed = |q: &str| MachinesView {
        query: q.into(),
        ..view(Some(hosts()))
    };
    let (text, r, _) = shown(&mut app, typed("dev@kingdom.lan"), 100, 30);
    let said = words(&text, r);
    assert!(said.contains("user dev host kingdom.lan"), "{text}");
    let (text, r, _) = shown(&mut app, typed("web"), 100, 30);
    let said = words(&text, r);
    assert!(
        said.contains("user deploy host web"),
        "the alias's user: {text}"
    );
    let (text, r, _) = shown(&mut app, typed(""), 100, 30);
    assert!(words(&text, r).contains("user — host —"), "{text}");
}

#[test]
fn typing_filters_the_suggestions_by_the_host_part_only() {
    let v = MachinesView {
        query: "dev@bui".into(),
        ..view(Some(hosts()))
    };
    let names: Vec<String> = suggestions(&v, &[]).into_iter().map(|h| h.alias).collect();
    assert_eq!(names, ["build"]);
    let v = MachinesView {
        query: "10.0".into(),
        ..view(Some(hosts()))
    };
    let names: Vec<String> = suggestions(&v, &[]).into_iter().map(|h| h.alias).collect();
    assert_eq!(names, ["gpu"]);
    let taken = [def("b", "build")];
    let v = view(Some(hosts()));
    assert!(
        !suggestions(&v, &taken).iter().any(|h| h.alias == "build"),
        "a host that is a machine is not offered"
    );
}

#[test]
fn enter_connects_to_the_typed_text_or_the_host_the_keys_are_on() {
    let at = |q: &str, in_list: bool, sel: usize| {
        let v = MachinesView {
            query: q.into(),
            in_list,
            host_sel: sel,
            ..view(Some(hosts()))
        };
        connect_target(&v, &[])
    };
    assert_eq!(
        at("dev@kingdom.lan", false, 0).as_deref(),
        Some("dev@kingdom.lan")
    );
    assert_eq!(
        at("g", false, 0).as_deref(),
        Some("g"),
        "in the input: what was typed"
    );
    assert_eq!(at("", false, 0), None, "an empty input connects to nothing");
    assert_eq!(
        at("", true, 1).as_deref(),
        Some("gpu"),
        "in the list: the chosen host"
    );
    assert_eq!(
        at("g", true, 0).as_deref(),
        Some("gpu"),
        "the list is filtered by the input"
    );
}

#[test]
fn down_walks_into_the_hosts_and_up_from_the_first_one_is_back_in_the_input() {
    let mut app = app_with(&[]);
    open(&mut app, view(Some(hosts())));
    let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
    let state = |app: &App| match &app.mode {
        Mode::Machines(v) => (v.in_list, v.host_sel),
        _ => panic!("the dialog"),
    };
    assert_eq!(state(&app), (false, 0), "it opens with the input");
    app.on_key(key(KeyCode::Down));
    assert_eq!(state(&app), (true, 0), "Down is the first host");
    app.on_key(key(KeyCode::Down));
    assert_eq!(state(&app), (true, 1));
    app.on_key(key(KeyCode::Up));
    app.on_key(key(KeyCode::Up));
    assert_eq!(
        state(&app),
        (false, 0),
        "Up from the first host is the input"
    );
    app.on_key(key(KeyCode::Down));
    app.on_key(key(KeyCode::Char('x')));
    assert_eq!(state(&app), (false, 0), "typing is the input again");
    let Mode::Machines(v) = &app.mode else {
        panic!("the dialog")
    };
    assert_eq!(v.query, "x");
}

#[test]
fn parse_reads_user_host_and_path() {
    assert_eq!(
        parse_target("dev@kingdom.lan:~/maitty"),
        Some(Parsed {
            user: Some("dev".into()),
            host: "kingdom.lan".into(),
            path: Some("~/maitty".into()),
        })
    );
    assert_eq!(
        parse_target("pi").map(|p| (p.user, p.host)),
        Some((None, "pi".into()))
    );
    assert_eq!(parse_target("  "), None);
    assert_eq!(parse_target("dev@"), None);
    assert_eq!(parse_target("a b"), None, "no spaces in a host");
}

#[test]
fn hosts_still_loading_say_so() {
    let mut app = app_with(&[]);
    let (text, r, _) = shown(&mut app, view(None), 100, 30);
    assert!(
        words(&text, r).contains("loading hosts from ~/.ssh/config"),
        "{text}"
    );
}

#[test]
fn with_no_hosts_it_says_to_type_user_at_host() {
    let mut app = app_with(&[]);
    let (text, r, _) = shown(&mut app, view(Some(vec![])), 100, 30);
    assert!(
        words(&text, r).contains("no hosts in ~/.ssh/config: type user@host above"),
        "{text}"
    );
}

#[test]
fn the_windows_hint_is_said_on_windows_only() {
    let mut app = app_with(&[]);
    let on = MachinesView {
        windows: true,
        ..view(Some(hosts()))
    };
    let (text, r, _) = shown(&mut app, on, 120, 40);
    assert!(
        words(&text, r).contains("Windows: use ssh keys or an agent"),
        "{text}"
    );
    let (text, r, _) = shown(&mut app, view(Some(hosts())), 120, 40);
    assert!(!words(&text, r).contains("Windows:"), "{text}");
}

#[test]
fn checking_is_a_checklist_with_the_first_step_running() {
    for (w, h) in SIZES {
        let mut app = app_with(&[]);
        let v = MachinesView {
            step: MachineStep::Checking {
                target: "dev@kingdom.lan".into(),
            },
            ..view(Some(hosts()))
        };
        let (text, r, buf) = shown(&mut app, v, w, h);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        for want in [
            "ADD MACHINE",
            "reach kingdom.lan",
            "○ verify host key",
            "○ sign in as dev",
            "○ maitty-daemon",
            "Esc cancel",
        ] {
            assert!(said.contains(want), "{w}x{h}: {want:?}\n{text}");
        }
    }
}

#[test]
fn an_added_machine_says_how_it_stands() {
    let cases = [
        (
            Outcome::Ready("0.64.0".into()),
            [
                "✓ reach kingdom.lan",
                "✓ maitty-daemon 0.64.0",
                "Enter done",
            ],
        ),
        (
            Outcome::Login,
            [
                "● sign in as dev log in in the popup",
                "○ maitty-daemon",
                "Esc close",
            ],
        ),
        (
            Outcome::Fix,
            [
                "✓ sign in as dev",
                "✕ maitty-daemon the connection shows the fix",
                "Esc close",
            ],
        ),
    ];
    for (how, wants) in cases {
        for (w, h) in SIZES {
            let mut app = app_with(&[("kingdom", "dev@kingdom.lan")]);
            let v = MachinesView {
                step: MachineStep::Added {
                    name: "kingdom".into(),
                    target: "dev@kingdom.lan".into(),
                    how: how.clone(),
                },
                ..view(Some(hosts()))
            };
            let (text, r, buf) = shown(&mut app, v, w, h);
            assert_box_whole(&text, r, &buf);
            let said = words(&text, r);
            assert!(said.contains("MACHINE ADDED"), "{w}x{h}\n{text}");
            assert!(
                said.contains("⇄ kingdom dev@kingdom.lan"),
                "{w}x{h}\n{text}"
            );
            for want in wants {
                assert!(said.contains(want), "{w}x{h}: {want:?}\n{text}");
            }
        }
    }
}

#[test]
fn naming_a_machine_the_target_gave_no_name_for() {
    for (w, h) in SIZES {
        let mut app = app_with(&[]);
        let v = MachinesView {
            step: MachineStep::Name {
                target: "10.0.0.7".into(),
                name: "gp".into(),
                error: Some("that name is taken".into()),
            },
            ..view(Some(hosts()))
        };
        let (text, r, buf) = shown(&mut app, v, w, h);
        assert_box_whole(&text, r, &buf);
        let said = words(&text, r);
        for want in [
            "Name for 10.0.0.7",
            "› gp",
            "that name is taken",
            "Esc back",
        ] {
            assert!(said.contains(want), "{w}x{h}: {want:?}\n{text}");
        }
    }
}

#[test]
fn the_dialog_keeps_to_a_small_window() {
    let mut app = app_with(&[]);
    let v = MachinesView {
        notice: Some(MachineNotice {
            text:
                "a refusal that wraps over a couple of lines in a narrow window of eighty columns"
                    .into(),
            error: true,
        }),
        query: "dev@build".into(),
        ..view(Some(hosts()))
    };
    let (text, r, buf) = shown(&mut app, v, 80, 24);
    assert_box_whole(&text, r, &buf);
    assert!(words(&text, r).contains("a refusal that wraps"), "{text}");
}

#[test]
fn a_paste_goes_into_the_input_not_the_pane_behind() {
    let mut app = app_with(&[]);
    open(&mut app, view(Some(hosts())));
    app.on_paste("dev@kingdom.lan\nsecond line".into());
    let Mode::Machines(v) = &app.mode else {
        panic!("the dialog")
    };
    assert_eq!(v.query, "dev@kingdom.lan", "the first line, in the input");
    assert!(!v.in_list);
}

/// `kingdom` with no app, `build` on another version and `gpu` connected, as their
/// supervisors said.
fn app_needing_fixes() -> App {
    use crate::client::link::ConnectionState;
    use crate::link_failure::{FIX_INSTALL, FIX_UPDATE};
    let mut app = app_with(&[
        ("build", "dev@build.lan"),
        ("gpu", "gpu"),
        ("kingdom", "dev@kingdom.lan"),
    ]);
    let states = [
        ("build", ConnectionState::NeedsUpdate { why: FIX_UPDATE.into() }),
        ("gpu", ConnectionState::Connected),
        ("kingdom", ConnectionState::NeedsUpdate { why: FIX_INSTALL.into() }),
    ];
    for (n, (name, state)) in states.into_iter().enumerate() {
        app.on_link_state(MachineId(40 + n as u16), name.into(), state);
    }
    app
}

#[test]
fn a_machine_that_needs_the_app_shows_the_key_that_installs_or_updates_it() {
    for (w, h) in SIZES {
        let mut app = app_needing_fixes();
        let v = MachinesView {
            machine_sel: app.machine_listed_at("kingdom"),
            ..view(Some(hosts()))
        };
        let (text, r, buf) = shown(&mut app, v, w, h);
        assert_box_whole(&text, r, &buf);
        let rows: Vec<&str> = text.lines().collect();
        let row = |name: &str| {
            rows.iter()
                .find(|l| l.contains(&format!("⇄ {name}")))
                .copied()
                .unwrap_or_else(|| panic!("{w}x{h}: no row for {name}\n{text}"))
        };
        assert!(row("kingdom").contains(" install "), "{w}x{h}\n{text}");
        assert!(row("build").contains(" update "), "{w}x{h}\n{text}");
        assert!(!row("gpu").contains(" install "), "a connected machine needs no install: {w}x{h}\n{text}");
        let said = words(&text, r);
        assert!(said.contains("u install"), "the bar names what u does: {w}x{h}\n{text}");
        assert!(
            said.contains("run `maitty machine update kingdom --install`"),
            "the fix under the chosen one names the machine: {w}x{h}\n{text}"
        );
    }
}

/// Click the button named `label` on the row of `name`.
fn click_button(app: &mut App, text: &str, name: &str, label: &str) {
    let (y, row) = text
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(&format!("⇄ {name}")))
        .unwrap_or_else(|| panic!("no row for {name}\n{text}"));
    let chars: Vec<char> = row.chars().collect();
    let want: Vec<char> = format!(" {label} ").chars().collect();
    let x = chars
        .windows(want.len())
        .position(|w| w == want.as_slice())
        .unwrap_or_else(|| panic!("no {label} button on {name}: {row}"));
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.on_mouse(MouseEvent {
            kind,
            column: x as u16 + 2,
            row: y as u16,
            modifiers: KeyModifiers::NONE,
        });
    }
}

#[test]
fn every_machine_row_has_its_own_buttons() {
    for (w, h) in [(100, 30), (160, 45)] {
        let mut app = app_with(&[("build", "dev@build.lan"), ("gpu", "gpu")]);
        let (text, r, buf) = shown(&mut app, view(Some(hosts())), w, h);
        assert_box_whole(&text, r, &buf);
        for name in ["build", "gpu"] {
            let row = text.lines().find(|l| l.contains(&format!("⇄ {name}"))).unwrap();
            for label in ["update", "reconnect", "edit", "remove"] {
                assert!(row.contains(&format!(" {label} ")), "{w}x{h}: {name} lacks {label}: {row}");
            }
        }
    }
}

#[test]
fn the_remove_and_edit_buttons_open_their_steps_for_the_row_they_are_on() {
    let mut app = app_with(&[("build", "dev@build.lan"), ("gpu", "gpu")]);
    let (text, ..) = shown(&mut app, view(Some(hosts())), 160, 45);
    click_button(&mut app, &text, "gpu", "remove");
    let Mode::Machines(v) = &app.mode else { panic!("{:?}", app.mode) };
    assert!(matches!(&v.step, MachineStep::Remove { name } if name == "gpu"), "{:?}", v.step);

    let mut app = app_with(&[("build", "dev@build.lan"), ("gpu", "gpu")]);
    let (text, ..) = shown(&mut app, view(Some(hosts())), 160, 45);
    click_button(&mut app, &text, "build", "edit");
    let Mode::Machines(v) = &app.mode else { panic!("{:?}", app.mode) };
    assert!(matches!(&v.step, MachineStep::Edit { name, .. } if name == "build"), "{:?}", v.step);
}

#[test]
fn the_reconnect_button_answers_in_the_dialog_and_a_click_on_a_row_chooses_it() {
    let mut app = app_with(&[("build", "dev@build.lan"), ("gpu", "gpu")]);
    let (text, ..) = shown(&mut app, view(Some(hosts())), 160, 45);
    click_button(&mut app, &text, "gpu", "reconnect");
    let Mode::Machines(v) = &app.mode else { panic!("{:?}", app.mode) };
    assert_eq!(v.machine_sel, Some(1));
    assert!(v.notice.is_some(), "it answers in the dialog");
    let (text, ..) = frame(&mut app, 160, 45);
    let (y, row) = text.lines().enumerate().find(|(_, l)| l.contains("⇄ build")).unwrap();
    let x = row.chars().take_while(|c| *c != '⇄').count() as u16 + 2;
    for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
        app.on_mouse(MouseEvent { kind, column: x, row: y as u16, modifiers: KeyModifiers::NONE });
    }
    let Mode::Machines(v) = &app.mode else { panic!("{:?}", app.mode) };
    assert_eq!(v.machine_sel, Some(0));
}
