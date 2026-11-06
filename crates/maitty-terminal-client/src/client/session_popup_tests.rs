//! The New Session popup, driven end to end with this computer and a machine named build.

use super::*;
use crate::client::ids::{MachineId, Remote};
use crate::client::screen::ScreenHit;
use crate::client::reference_fixture::{Stage, world};
use crate::protocol::{FolderEntry, FolderListing, Query, Reply};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const THERE: MachineId = MachineId(1);
const OFFLINE: MachineId = MachineId(2);

fn two_machines() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let mut app = world(Stage::None);
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
    app.remotes.insert(
        THERE,
        Remote {
            name: "build".into(),
            state: crate::client::link::ConnectionState::Connected,
            ..Remote::default()
        },
    );
    app.remotes.insert(
        OFFLINE,
        Remote {
            name: "pi-garage".into(),
            state: crate::client::link::ConnectionState::Unreachable {
                since: std::time::Instant::now(),
                why: "no route".into(),
            },
            ..Remote::default()
        },
    );
    app.screen_fresh();
    (app, rx)
}

fn sent(rx: &mut mpsc::UnboundedReceiver<Outgoing>) -> Vec<Outgoing> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn asked(out: &[Outgoing]) -> Vec<(MachineId, String, u64)> {
    out.iter()
        .filter_map(|o| match &o.msg {
            ClientMsg::Query(Query::ListFolders { path, tag }) => Some((o.machine, path.clone(), *tag)),
            _ => None,
        })
        .collect()
}

fn listing(path: &str, names: &[(&str, Option<&str>, bool)]) -> FolderListing {
    FolderListing {
        path: path.into(),
        exists: true,
        entries: names
            .iter()
            .map(|(n, g, k)| FolderEntry { name: (*n).into(), git: g.map(str::to_string), has_children: *k })
            .collect(),
        truncated: false,
    }
}

fn answer(app: &mut App, machine: MachineId, tag: u64, l: FolderListing) {
    app.on_machine_server(machine, ServerMsg::Reply(Reply::Folders { tag, listing: l }));
}

fn key(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        key(app, KeyCode::Char(c));
    }
}

fn frame(app: &mut App, w: u16, h: u16) -> String {
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| render::draw(app, f)).unwrap();
    let buf = term.backend().buffer().clone();
    (0..h)
        .map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n")
        .collect()
}

#[test]
fn opening_it_asks_every_online_machine_for_its_folders_and_no_offline_one() {
    let (mut app, mut rx) = two_machines();
    sent(&mut rx);

    app.screen_open_session_popup("");

    let out = asked(&sent(&mut rx));
    assert!(out.iter().any(|(m, p, _)| *m == MachineId::LOCAL && p == "~/"), "{out:?}");
    assert!(out.iter().any(|(m, p, _)| *m == THERE && p == "~/"), "{out:?}");
    assert!(out.iter().all(|(m, _, _)| *m != OFFLINE), "{out:?}");
    assert!(matches!(app.mode, Mode::NewSession(_)));
}

#[test]
fn it_shows_the_machines_and_what_is_open_and_closes_on_escape() {
    let (mut app, mut rx) = two_machines();
    app.screen_open_session_popup("");
    sent(&mut rx);

    let shown = frame(&mut app, 120, 40);
    println!("{shown}");

    assert!(shown.contains("NEW SESSION"));
    assert!(shown.contains("project, folder, ~/path or kingdom:~/path"));
    assert!(shown.contains("OPEN"));
    assert!(shown.contains("local"), "{shown}");
    assert!(shown.contains("build") && shown.contains("pi-garage"), "{shown}");
    assert!(shown.contains("switch machine"));

    key(&mut app, KeyCode::Esc);
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn a_machine_path_asks_that_machine_and_starts_a_session_there_from_start_here() {
    let (mut app, mut rx) = two_machines();
    app.screen_open_session_popup("");
    for (m, p, t) in asked(&sent(&mut rx)) {
        answer(&mut app, m, t, listing("/home/x", &[("maitty", Some("main"), true)]));
        let _ = p;
    }

    type_text(&mut app, "build:~/");
    let out = asked(&sent(&mut rx));
    // The root of build was already asked for when the popup opened, so nothing new goes out.
    assert!(out.is_empty(), "{out:?}");
    let shown = frame(&mut app, 120, 40);
    println!("{shown}");
    assert!(shown.contains("start here"), "{shown}");
    assert!(shown.contains("build:~/maitty/"), "{shown}");

    key(&mut app, KeyCode::Enter);

    assert!(matches!(app.mode, Mode::Normal));
    let out = sent(&mut rx);
    assert!(
        out.iter().any(|o| o.machine == THERE
            && matches!(&o.msg, ClientMsg::Command(Command::NewWorkspace { cwd: Some(c), .. }) if c.to_string_lossy() == "/home/x")),
        "{out:?}"
    );
}

#[test]
fn typing_into_a_folder_not_yet_read_asks_for_it_and_waits() {
    let (mut app, mut rx) = two_machines();
    app.screen_open_session_popup("");
    sent(&mut rx);

    type_text(&mut app, "build:~/code/");
    let out = asked(&sent(&mut rx));
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!((out[0].0, out[0].1.as_str()), (THERE, "~/code/"));
    assert!(frame(&mut app, 120, 40).contains("reading"));

    answer(&mut app, THERE, out[0].2, listing("/home/x/code", &[("api", None, false)]));
    let shown = frame(&mut app, 120, 40);
    assert!(shown.contains("build:~/code/api/"), "{shown}");
    assert!(shown.contains("1 folder"), "{shown}");
}

#[test]
fn tab_fills_the_selected_path_and_a_plain_folder_opens_instead_of_starting() {
    let (mut app, mut rx) = two_machines();
    app.screen_open_session_popup("");
    for (m, _, t) in asked(&sent(&mut rx)) {
        answer(&mut app, m, t, listing("/home/x", &[("dir", None, true), ("repo", Some("main"), true)]));
    }
    type_text(&mut app, "~/d");

    key(&mut app, KeyCode::Tab);
    let Mode::NewSession(ns) = &app.mode else { panic!("the popup closed") };
    assert_eq!(ns.q, "~/dir/");

    // Enter on a folder that only holds folders browses into it.
    let mut ns = (**ns).clone();
    ns.q = "~/".into();
    ns.sel = None;
    app.mode = Mode::NewSession(Box::new(ns));
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.mode, Mode::NewSession(_)), "a folder with subfolders is opened, not started");
}

#[test]
fn a_session_started_from_the_popup_is_remembered_and_announced() {
    let (mut app, mut rx) = two_machines();
    app.screen_open_session_popup("");
    for (m, _, t) in asked(&sent(&mut rx)) {
        answer(&mut app, m, t, listing("/home/x", &[]));
    }
    type_text(&mut app, "build:~/");
    key(&mut app, KeyCode::Enter);

    assert_eq!(app.screen.saved.recent_paths, vec!["build:/home/x".to_string()]);
    assert!(app.notice.as_ref().is_some_and(|(text, _, _)| text.starts_with("+ ") && text.ends_with(" started")));
}

#[test]
fn a_connected_machine_with_nothing_running_offers_new_and_opens_the_popup_on_its_home_folder() {
    let (mut app, _rx) = two_machines();
    let shown = frame(&mut app, 120, 40);
    assert!(shown.contains("⏎ new"), "{shown}");
    let offered = app
        .hits
        .iter()
        .any(|(_, h)| *h == Hit::Screen(ScreenHit::NsMachine(THERE)));
    assert!(offered, "the row for build is clickable");
    // The one that is down offers its fix, not a session.
    assert!(!app.hits.iter().any(|(_, h)| *h == Hit::Screen(ScreenHit::NsMachine(OFFLINE))));

    app.on_hy_hit(ScreenHit::NsMachine(THERE), false);

    let Mode::NewSession(ns) = &app.mode else { panic!("the popup did not open") };
    assert_eq!(ns.q, "build:~/");
}

#[test]
fn the_new_button_and_the_n_key_open_it() {
    let (mut app, _rx) = two_machines();
    app.screen_splash_action('n');
    assert!(matches!(app.mode, Mode::NewSession(_)));
}
