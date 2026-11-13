//! Put away and Bring back: the row action, the lists that leave a put-away session out, and the
//! Put away list. `MAITTY_SHOW=1 cargo test put_away_tests -- --nocapture` prints the frames.

use super::*;
use crate::client::ids::Outgoing;
use crate::client::menu::Act;
use crate::client::reference_fixture::{Layout, Stage, raw_text, world};
use crate::protocol::ClientMsg;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

/// The reference world's finished quick-fox, in orbit.
const FOX: ClientTerm = ClientTerm::local(4);
/// The sidebar's width in the reference world.
const SIDE_W: usize = 34;

fn app_with_outbox() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let mut app = world(Stage::None);
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
    (app, rx)
}

/// The `(term, archived)` of each ArchivePane sent.
fn archived_sent(rx: &mut mpsc::UnboundedReceiver<Outgoing>) -> Vec<(TermId, bool)> {
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|o| match o.msg {
            ClientMsg::Command(Command::ArchivePane { term, archived }) => Some((term, archived)),
            _ => None,
        })
        .collect()
}

fn frame(app: &mut App, title: &str) -> String {
    let text = raw_text(app, Layout::Floating, 120, 30, "unicode");
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        println!("---- {title}\n{text}");
    }
    text
}

fn sidebar(text: &str) -> String {
    text.lines()
        .map(|l| l.chars().take(SIDE_W).collect::<String>() + "\n")
        .collect()
}

/// What the daemon's next state says once it has the flag.
fn daemon_sets(app: &mut App, term: ClientTerm, archived: bool) {
    app.snap.terms.get_mut(&term.term).expect("the pane").archived = archived;
    app.screen_fresh();
}

fn menu_labels(app: &App) -> Vec<String> {
    match &app.mode {
        Mode::ScreenMenu(m) => m.items.iter().map(|(l, _)| l.clone()).collect(),
        other => panic!("no menu open: {other:?}"),
    }
}

fn pick(app: &mut App, label: &str) {
    let i = menu_labels(app)
        .iter()
        .position(|l| l == label)
        .unwrap_or_else(|| panic!("no {label} in {:?}", menu_labels(app)));
    app.menu_pick(i);
}

fn listed(app: &App, term: ClientTerm) -> bool {
    find(&app.screen_model(), term).is_some()
}

#[test]
fn put_away_from_a_finished_row_sends_the_flag_and_the_row_leaves_the_sidebar() {
    let (mut app, mut rx) = app_with_outbox();
    archived_sent(&mut rx);
    app.menu_for_session(FOX, (2, 20));
    let text = frame(&mut app, "quick-fox's row menu");
    assert!(text.contains("Put away"), "{text}");
    pick(&mut app, "Put away");
    assert_eq!(archived_sent(&mut rx), [(FOX.term, true)]);
    daemon_sets(&mut app, FOX, true);
    let text = frame(&mut app, "quick-fox put away");
    assert!(!sidebar(&text).contains("quick-fox"), "{text}");
    assert!(!listed(&app, FOX));
    assert!(text.contains("Put away quick-fox"), "the undo banner: {text}");
}

#[test]
fn undo_brings_back_what_was_put_away() {
    let (mut app, mut rx) = app_with_outbox();
    app.put_away(FOX);
    archived_sent(&mut rx);
    app.act(Action::UndoClose);
    assert_eq!(archived_sent(&mut rx), [(FOX.term, false)]);
}

#[test]
fn only_a_finished_or_idle_terminal_session_can_be_put_away() {
    let (mut app, _rx) = app_with_outbox();
    let can = |app: &App, id| app.can_put_away(ClientTerm::local(id));
    assert!(can(&app, 4), "done");
    assert!(can(&app, 5), "idle");
    assert!(!can(&app, 2), "working");
    assert!(!can(&app, 3), "needs you");
    app.snap.terms.get_mut(&4).unwrap().conversation_id =
        Some(crate::conversation::ConversationId(9));
    assert!(!can(&app, 4), "a chat's terminal: archiving a chat closes it");
    let (app, _rx) = app_with_outbox();
    let items = app.session_items(ClientTerm::local(2)).expect("a row").1;
    assert!(
        !items.iter().any(|(_, a)| matches!(a, Act::PutAway(_))),
        "a working agent's menu has no Put away: {items:?}"
    );
}

#[test]
fn a_put_away_session_that_needs_you_is_listed_while_it_does() {
    let (mut app, _rx) = app_with_outbox();
    daemon_sets(&mut app, FOX, true);
    app.snap.terms.get_mut(&FOX.term).unwrap().status = Status::Blocked;
    app.screen_fresh();
    let text = frame(&mut app, "put away, needs you");
    assert!(sidebar(&text).contains("quick-fox"), "{text}");
    app.snap.terms.get_mut(&FOX.term).unwrap().status = Status::Idle;
    app.screen_fresh();
    assert!(!listed(&app, FOX), "answered, it goes back where it was left");
}

#[test]
fn the_menu_opens_the_put_away_list_and_a_row_brings_it_back() {
    let (mut app, mut rx) = app_with_outbox();
    daemon_sets(&mut app, FOX, true);
    daemon_sets(&mut app, ClientTerm::local(5), true);
    app.sidebar_menu();
    frame(&mut app, "the sidebar menu");
    pick(&mut app, "Put away…");
    let text = frame(&mut app, "the Put away list");
    let labels = menu_labels(&app);
    assert_eq!(
        labels,
        ["Bring back quick-fox · orbit", "Bring back old-moth · orbit"]
    );
    for label in &labels {
        assert!(text.contains(label.as_str()), "drawn: {label}\n{text}");
    }
    archived_sent(&mut rx);
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(archived_sent(&mut rx), [(FOX.term, false)]);
    let (said, _, error) = app.notice.clone().expect("a toast");
    assert!(!error && said == "Brought back quick-fox", "{said}");
}

#[test]
fn with_nothing_put_away_the_list_says_so() {
    let (mut app, _rx) = app_with_outbox();
    app.act(Action::PutAway);
    let (said, _, error) = app.notice.clone().expect("a toast");
    assert!(!error && said == "Nothing is put away", "{said}");
    assert!(matches!(app.mode, Mode::Normal), "{:?}", app.mode);
}
