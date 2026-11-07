//! The New session dialog driven end to end with this computer and a machine named build (up) and
//! one named gpu-01 (reconnecting).

use super::*;
use crate::client::ids::{MachineId, Remote};
use crate::client::reference_fixture::{Stage, world};
use crate::client::{Outgoing, render};
use crate::protocol::{ClientMsg, Command, FolderEntry, FolderListing, Query, Reply, ServerMsg};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

const BUILD: MachineId = MachineId(1);
const GPU: MachineId = MachineId(2);

fn world_with_machines() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let mut app = world(Stage::None);
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
    app.remotes.insert(
        BUILD,
        Remote {
            name: "build".into(),
            state: crate::client::link::ConnectionState::Connected,
            ..Remote::default()
        },
    );
    app.remotes.insert(
        GPU,
        Remote {
            name: "gpu-01".into(),
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

/// A world with only this computer: nothing to choose in step 2.
fn world_alone() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let mut app = world(Stage::None);
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
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

fn listing(path: &str, names: &[&str]) -> FolderListing {
    FolderListing {
        path: path.into(),
        exists: true,
        entries: names
            .iter()
            .map(|n| FolderEntry { name: (*n).into(), git: None, has_children: true })
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

fn ctrl(app: &mut App, c: char) {
    app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
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

/// The lines of `shown` inside the dialog: from its top border to its bottom border.
fn dialog_lines(shown: &str) -> Vec<String> {
    let lines: Vec<&str> = shown.lines().collect();
    let top = lines.iter().position(|l| l.contains("NEW SESSION")).expect("the title");
    let bottom = lines[top..].iter().position(|l| l.contains("Esc")).expect("the bottom border") + top;
    lines[top..=bottom].iter().map(|l| l.to_string()).collect()
}

#[test]
fn ctrl_t_opens_a_box_78_wide_and_19_tall_with_the_sentence_header() {
    let (mut app, _rx) = world_with_machines();
    ctrl(&mut app, 't');
    assert!(matches!(app.mode, Mode::NewSession(_)));

    let shown = frame(&mut app, 120, 40);
    println!("{shown}");
    let r = app.screen.modal_rects.last().copied().expect("the dialog's box");
    assert_eq!((r.width, r.height), (78, 19), "a fixed size");
    assert_eq!(dialog_lines(&shown).len(), 19, "from its title row to its keys: {shown}");
    assert!(shown.contains("NEW SESSION"));
    assert!(shown.contains("Start in") && shown.contains("opens a shell"), "{shown}");
    assert!(shown.contains("Browse…") && shown.contains("Ctrl+O"), "Browse first: {shown}");
    assert!(shown.contains("Enter") && shown.contains("Tab") && shown.contains("Esc"), "the keys on the border");
}

#[test]
fn the_prefix_then_t_opens_it_where_a_terminal_cannot_send_ctrl_t() {
    let (mut app, _rx) = world_with_machines();
    app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    key(&mut app, KeyCode::Char('t'));
    assert!(matches!(app.mode, Mode::NewSession(_)));
}

#[test]
fn a_typed_path_asks_the_chosen_machine_completes_dim_and_tab_accepts() {
    let (mut app, mut rx) = world_with_machines();
    ctrl(&mut app, 't');
    sent(&mut rx);

    type_text(&mut app, "~/co");
    let out = asked(&sent(&mut rx));
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!((out[0].0, out[0].1.as_str()), (MachineId::LOCAL, "~/"));

    answer(&mut app, MachineId::LOCAL, out[0].2, listing("/home/x", &["code", "docs"]));
    let shown = frame(&mut app, 120, 40);
    println!("{shown}");
    assert!(shown.contains("~/code/"), "the completion is drawn after the cursor: {shown}");
    assert!(shown.contains("Tab complete"), "{shown}");

    key(&mut app, KeyCode::Tab);
    let Mode::NewSession(ns) = &app.mode else { panic!("closed") };
    assert_eq!(ns.q, "~/code/");
}

#[test]
fn a_missing_folder_shows_a_red_line_and_blocks_enter() {
    let (mut app, mut rx) = world_with_machines();
    ctrl(&mut app, 't');
    sent(&mut rx);
    type_text(&mut app, "~/nope");
    let out = asked(&sent(&mut rx));
    answer(&mut app, MachineId::LOCAL, out[0].2, listing("/home/x", &["code"]));

    let shown = frame(&mut app, 120, 40);
    assert!(shown.contains("✗ No such folder on"), "{shown}");
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.mode, Mode::NewSession(_)), "Enter does nothing while the folder is missing");
    assert!(sent(&mut rx).iter().all(|o| !matches!(o.msg, ClientMsg::Command(Command::NewWorkspace { .. }))));
}

#[test]
fn with_only_this_computer_enter_on_a_folder_creates_the_shell() {
    let (mut app, mut rx) = world_alone();
    ctrl(&mut app, 't');
    sent(&mut rx);
    type_text(&mut app, "~/code");
    let out = asked(&sent(&mut rx));
    answer(&mut app, MachineId::LOCAL, out[0].2, listing("/home/x", &["code"]));

    key(&mut app, KeyCode::Enter);

    assert_eq!(app.mode, Mode::Normal, "the dialog closes");
    let out = sent(&mut rx);
    assert!(
        out.iter().any(|o| o.machine == MachineId::LOCAL
            && matches!(&o.msg, ClientMsg::Command(Command::NewWorkspace { cwd: Some(c), cmd: None, .. }) if c.to_string_lossy() == "/home/x/code")),
        "a shell (no command) in the folder: {out:?}"
    );
    assert_eq!(app.screen.saved.recent_paths.first().map(String::as_str), Some("/home/x/code"));
}

#[test]
fn step_two_lists_machines_skips_the_offline_one_and_starts_the_shell_on_the_pick() {
    let (mut app, mut rx) = world_with_machines();
    ctrl(&mut app, 't');
    sent(&mut rx);
    type_text(&mut app, "~/code");
    let out = asked(&sent(&mut rx));
    answer(&mut app, MachineId::LOCAL, out[0].2, listing("/home/x", &["code"]));

    key(&mut app, KeyCode::Enter);
    let Mode::NewSession(ns) = &app.mode else { panic!("step two") };
    assert_eq!(ns.step, super::new_session::Step::Machine);

    let shown = frame(&mut app, 120, 40);
    println!("{shown}");
    assert!(shown.contains("build") && shown.contains("gpu-01"), "{shown}");
    assert!(shown.contains("reconnecting"), "the offline machine says so: {shown}");

    // The folder is asked about on the machine picked.
    key(&mut app, KeyCode::Down);
    let out = asked(&sent(&mut rx));
    let out: Vec<_> = out.into_iter().filter(|(m, ..)| *m == BUILD).collect();
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!((out[0].0, out[0].1.as_str()), (BUILD, "/home/x/code/"));
    key(&mut app, KeyCode::Down);
    let Mode::NewSession(ns) = &app.mode else { panic!("step two") };
    assert_eq!(ns.machine, BUILD, "Down skips gpu-01, which is offline");
    answer(&mut app, BUILD, out[0].2, listing("/home/x/code", &[]));

    key(&mut app, KeyCode::Enter);
    assert_eq!(app.mode, Mode::Normal);
    let out = sent(&mut rx);
    assert!(
        out.iter().any(|o| o.machine == BUILD
            && matches!(&o.msg, ClientMsg::Command(Command::NewWorkspace { cwd: Some(c), cmd: None, .. }) if c.to_string_lossy() == "/home/x/code")),
        "the shell starts on build, through its own connection: {out:?}"
    );
}

#[test]
fn a_folder_missing_on_the_picked_machine_blocks_the_start() {
    let (mut app, mut rx) = world_with_machines();
    ctrl(&mut app, 't');
    sent(&mut rx);
    type_text(&mut app, "~/code");
    let out = asked(&sent(&mut rx));
    answer(&mut app, MachineId::LOCAL, out[0].2, listing("/home/x", &["code"]));
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Down);
    let out: Vec<_> = asked(&sent(&mut rx)).into_iter().filter(|(m, ..)| *m == BUILD).collect();
    answer(&mut app, BUILD, out[0].2, FolderListing::missing("/home/x/code"));

    let shown = frame(&mut app, 120, 40);
    assert!(shown.contains("✗ No such folder on build"), "{shown}");
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.mode, Mode::NewSession(_)));
}

#[test]
fn ctrl_o_opens_the_picker_which_lists_folders_and_enter_goes_into_one() {
    let (mut app, mut rx) = world_alone();
    ctrl(&mut app, 't');
    sent(&mut rx);
    ctrl(&mut app, 'o');
    let out = asked(&sent(&mut rx));
    assert_eq!((out[0].0, out[0].1.as_str()), (MachineId::LOCAL, "~/"));
    answer(&mut app, MachineId::LOCAL, out[0].2, listing("/home/x", &["code", "docs"]));

    let shown = frame(&mut app, 120, 40);
    println!("{shown}");
    assert!(shown.contains("Use ") && shown.contains("code") && shown.contains("docs"), "{shown}");

    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    let out = asked(&sent(&mut rx));
    assert_eq!(out[0].1, "~/code/", "Enter on a folder reads it: {out:?}");

    key(&mut app, KeyCode::Esc);
    let Mode::NewSession(ns) = &app.mode else { panic!("Esc leaves the picker, not the dialog") };
    assert!(ns.browsing.is_none());
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn recent_projects_are_listed_with_their_name_path_and_machine() {
    let (mut app, _rx) = world_with_machines();
    app.screen.saved.recent_paths = vec!["build:/srv/api".into(), "/home/x/maitty".into()];
    ctrl(&mut app, 't');
    let shown = frame(&mut app, 120, 40);
    println!("{shown}");
    assert!(shown.contains("api") && shown.contains("/srv/api") && shown.contains("⇄ build"), "{shown}");
    assert!(shown.contains("maitty") && shown.contains("local"), "{shown}");
}
