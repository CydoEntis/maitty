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

/// The glyphs of the popup whose width differs between terminals.
const AMBIGUOUS: &[char] = &['❯', '█', '⌕', '▶', '◆', '●', '○', '◌', '▸', '·', '⌂', '⇄', '⎇', '…', '↑', '↓', '←', '→'];

fn answered_popup(app: &mut App, rx: &mut mpsc::UnboundedReceiver<Outgoing>) {
    app.screen_open_session_popup("");
    for (m, _, t) in asked(&sent(rx)) {
        answer(app, m, t, listing("/home/x", &[("maitty", Some("main"), true), ("notes", None, false)]));
    }
}

/// The popup's own cells: its columns, from its title row down to its bottom edge (the sidebar
/// beside it and the mode bar under it are drawn by other code).
fn popup_text(shown: &str) -> String {
    let lines: Vec<Vec<char>> = shown.lines().map(|l| l.chars().collect()).collect();
    let top = lines.iter().position(|l| l.iter().collect::<String>().contains("NEW SESSION")).expect("title row");
    let left = lines[top].iter().position(|c| *c == '┌').expect("corner");
    let mut out = Vec::new();
    for l in &lines[top..] {
        out.push(l.iter().skip(left).take(92).collect::<String>());
        if l.get(left) == Some(&'└') {
            break;
        }
    }
    out.join("
")
}

#[test]
fn ascii_glyphs_replace_the_ambiguous_width_ones_in_the_popup() {
    let (mut app, mut rx) = two_machines();
    app.cfg.ui.glyphs = "ascii".into();
    answered_popup(&mut app, &mut rx);
    type_text(&mut app, "build:~/");
    let shown = frame(&mut app, 120, 40);
    println!("{shown}");
    let popup = popup_text(&shown);
    for ch in AMBIGUOUS {
        assert!(!popup.contains(*ch), "{ch} drawn in ASCII mode:\n{popup}");
    }
    assert!(popup.contains("> build:~/"), "{popup}");
    assert!(popup.contains("start here"), "{popup}");
    assert!(popup.contains("@ build"), "{popup}");
    assert!(popup.contains("Left/Right  switch machine"), "{popup}");
    assert!(popup.contains("git:main"), "{popup}");

    key(&mut app, KeyCode::Esc);
    app.cfg.ui.glyphs = "dots".into();
    app.screen_open_session_popup("");
    let plain = frame(&mut app, 120, 40);
    assert!(plain.contains("❯") && plain.contains("⇄ build"), "Unicode stays the default:\n{plain}");
}

#[test]
fn starting_a_session_flashes_the_mode_bar_for_a_moment() {
    let (mut app, mut rx) = two_machines();
    answered_popup(&mut app, &mut rx);
    type_text(&mut app, "build:~/");
    key(&mut app, KeyCode::Enter);

    let last = |s: &str| s.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").to_string();
    let shown = frame(&mut app, 120, 40);
    let bar = last(&shown);
    assert!(bar.contains("started"), "the bar flashes the start:\n{shown}");
    assert!(bar.contains("TYPING"), "the mode tag stays: {bar}");

    let (text, _) = app.bar_flash.take().expect("flash set");
    app.bar_flash = Some((text, std::time::Instant::now() - BAR_FLASH_FOR - std::time::Duration::from_millis(1)));
    let bar = last(&frame(&mut app, 120, 40));
    assert!(!bar.contains("started"), "the flash ends: {bar}");
    assert!(bar.contains("Ctrl+Space"), "the hint is back: {bar}");
}

#[test]
fn the_new_session_becomes_the_focused_card_with_the_sidebar_following() {
    let (mut app, mut rx) = two_machines();
    answered_popup(&mut app, &mut rx);
    // The sidebar had a project cursor and was scrolled by hand.
    app.screen.cursor_proj = Some("somewhere".into());
    app.screen.follow = false;
    type_text(&mut app, "~/");
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.mode, Mode::Normal));
    assert!(app.screen.cursor_proj.is_none() && app.screen.follow);

    // The daemon answers with the new session, focused.
    let started = sent(&mut rx);
    assert!(started.iter().any(|o| matches!(&o.msg, ClientMsg::Command(Command::NewWorkspace { .. }))), "{started:?}");
    let mut next = app.snap.clone();
    let dir = std::path::PathBuf::from("/home/x/notes");
    let new_term = crate::client::reference_fixture::term(900, "notes", "claude", crate::protocol::Status::Working, &dir, 0);
    next.terms.insert(900, new_term);
    next.workspaces.push(crate::client::reference_fixture::workspace(900, "notes", &dir, &[900], None, 3));
    next.active_ws = Some(900);
    app.got_state = true;
    app.on_server(ServerMsg::State(next));

    let shown = frame(&mut app, 120, 40);
    assert_eq!(app.focused(), Some(ClientTerm::local(900)));
    assert!(app.screen.row_y.contains_key(&ClientTerm::local(900)), "the new card is in the sidebar:\n{shown}");
}
