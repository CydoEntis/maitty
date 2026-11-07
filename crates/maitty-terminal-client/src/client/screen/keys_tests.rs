//! The keys of the v26 handoff end to end: the prefix and what is taken inside a pane, Search
//! commands (Ctrl+K) with its sessions, the Menu, and the Updates box.

use super::*;
use crate::client::reference_fixture::{Stage, world};
use crate::client::{Outgoing, render};
use crate::protocol::{ClientMsg, Command};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

fn app_with_channel() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let mut app = world(Stage::None);
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
    app.screen_fresh();
    (app, rx)
}

fn press(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    app.on_key(KeyEvent::new(code, mods));
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c), KeyModifiers::NONE);
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
fn the_prefix_is_ctrl_a_and_ctrl_space_still_works_for_one_release() {
    let (app, _rx) = app_with_channel();
    let key = |s: &str| s.parse::<crate::keys::KeySpec>().unwrap();
    assert_eq!(app.keymap.prefix, key("ctrl+a"));
    assert!(app.keymap.is_prefix(&key("ctrl+space")), "the old prefix still opens shortcut mode");
    assert!(!app.keymap.is_prefix(&key("ctrl+b")));
    let cfg = crate::config::Config { prefix: "ctrl+b".into(), ..Default::default() };
    let km = cfg.keymap();
    assert!(km.is_prefix(&key("ctrl+b")) && !km.is_prefix(&key("ctrl+space")), "a prefix set on purpose is the only one");
}

#[test]
fn inside_a_pane_only_the_listed_keys_are_taken() {
    let (app, _rx) = app_with_channel();
    let key = |s: &str| s.parse::<crate::keys::KeySpec>().unwrap();
    let taken: Vec<_> = app.keymap.global.keys().copied().collect();
    for passes in ["ctrl+c", "ctrl+d", "ctrl+z", "ctrl+l", "ctrl+r", "up", "down", "left", "right", "tab", "esc", "enter"] {
        assert!(!taken.contains(&key(passes)), "{passes} goes to the program");
    }
    for takes in ["ctrl+t", "ctrl+k", "ctrl+j", "f2", "alt+up", "alt+down"] {
        assert!(taken.contains(&key(takes)), "{takes} is taken");
    }
}

#[test]
fn ctrl_k_opens_search_commands_and_it_lists_commands_then_sessions() {
    let (mut app, _rx) = app_with_channel();
    press(&mut app, KeyCode::Char('k'), KeyModifiers::CONTROL);
    assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == KeyView::Search));
    let shown = frame(&mut app, 140, 40);
    println!("{shown}");
    assert!(shown.contains("Search commands") && shown.contains("type what you want to do"));
    let commands = shown.find("COMMANDS").expect("COMMANDS");
    let sessions = shown.find("SESSIONS").expect("SESSIONS");
    assert!(commands < sessions, "commands first");
    assert!(shown.contains("↑ ↓") && shown.contains("Enter") && shown.contains("Esc"));
    assert!(shown.contains("General") || shown.contains("Panes") || shown.contains("Find"), "a category column");
}

#[test]
fn typing_is_a_fuzzy_in_order_subsequence_and_the_group_reads_results() {
    let (mut app, _rx) = app_with_channel();
    press(&mut app, KeyCode::Char('k'), KeyModifiers::CONTROL);
    type_text(&mut app, "spld");
    let shown = frame(&mut app, 140, 40);
    println!("{shown}");
    assert!(shown.contains("RESULTS"), "{shown}");
    assert!(shown.contains("Split down"), "s-p-l-d are in order in it: {shown}");
    assert!(!shown.contains("Settings"), "and not in Settings, which has no p: {shown}");
}

#[test]
fn a_session_row_shows_its_name_and_project_and_enter_goes_to_it() {
    let (mut app, mut rx) = app_with_channel();
    press(&mut app, KeyCode::Char('k'), KeyModifiers::CONTROL);
    type_text(&mut app, "fixfoot");
    let shown = frame(&mut app, 140, 40);
    println!("{shown}");
    assert!(shown.contains("fix-footer") && shown.contains("affiliate-site") && shown.contains("local"), "{shown}");
    while rx.try_recv().is_ok() {}
    // The command rows come first; the session is the last row.
    for _ in 0..40 {
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    }
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::Normal);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|o| matches!(o.msg, ClientMsg::Command(Command::FocusPane { .. }))),
        "the session takes the focus: {sent:?}"
    );
}

#[test]
fn the_menu_has_the_v26_sections_and_skips_their_headers() {
    let rows = action_items();
    let labels: Vec<&str> = rows.iter().map(|(l, _)| *l).collect();
    assert_eq!(
        labels,
        [
            "Shell here", "Session...", "Pane beside this", "Files", "Changes", "Branches", "History",
            "References", "Inbox", "Search commands", "All keys", "Collapse sidebar", "Settings",
        ]
    );
    let (mut app, _rx) = app_with_channel();
    press(&mut app, KeyCode::Char('a'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('.'), KeyModifiers::NONE);
    assert!(matches!(app.mode, Mode::Actions { sel: 0 }));
    let shown = frame(&mut app, 140, 40);
    for header in ["NEW", "TOOLS", "FIND", "SIDEBAR"] {
        assert!(shown.contains(header), "{header}: {shown}");
    }
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    assert!(matches!(app.mode, Mode::Actions { sel: 1 }), "arrows move over rows, not headers");
}

#[test]
fn the_updates_box_follows_the_install_and_offers_its_keys() {
    let (mut app, _rx) = app_with_channel();
    app.update_available = Some("9.9.9".into());
    app.update_notes = vec!["Flat sessions".into()];
    press(&mut app, KeyCode::Char('a'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('u'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::Updates);
    let shown = frame(&mut app, 140, 40);
    println!("{shown}");
    assert!(shown.contains("9.9.9") && shown.contains("Flat sessions"), "{shown}");
    assert!(shown.contains("Download") && shown.contains("Skip") && shown.contains("Later"), "{shown}");
    press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::Normal, "skip closes it");
    assert_eq!(app.update_phase(), super::updates::Phase::UpToDate, "and this version is skipped for the run");
    app.update_failed = Some("offline".into());
    assert!(matches!(app.update_phase(), super::updates::Phase::Failed { .. }));
}
