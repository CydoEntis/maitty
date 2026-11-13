//! Resume a conversation here: the lists asked of the daemon, the menu, and what a pick starts.
//! `MAITTY_SHOW=1 cargo test resume_tests -- --nocapture` prints the frames.

use super::*;
use crate::client::ids::MachineId;
use crate::client::reference_fixture::{Layout, Stage, now, raw_text, world};
use crate::protocol::{SavedConversation, SavedConversations};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn app_with_outbox() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let mut app = world(Stage::None);
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
    (app, rx)
}

fn sent(rx: &mut mpsc::UnboundedReceiver<Outgoing>) -> Vec<Outgoing> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// The (agent, folder, tag) of each saved-conversation question sent.
fn asked(out: &[Outgoing]) -> Vec<(String, String, u64)> {
    out.iter()
        .filter_map(|o| match &o.msg {
            ClientMsg::Query(Query::SavedConversations { agent, folder, tag }) => {
                assert_eq!(o.machine, MachineId::LOCAL);
                Some((agent.clone(), folder.clone(), *tag))
            }
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

fn saved(id: &str, said: &str, ago: u64) -> SavedConversation {
    SavedConversation {
        id: id.into(),
        first_message: said.into(),
        last_active: now() - ago,
    }
}

fn reply(app: &mut App, tag: u64, found: SavedConversations) {
    app.on_machine_server(
        MachineId::LOCAL,
        ServerMsg::Reply(Reply::SavedConversations { tag, found }),
    );
}

fn focused_dir(app: &App) -> PathBuf {
    let term = app.selected_term().expect("a focused pane");
    app.term_info(term).expect("its info").cwd.clone()
}

#[test]
fn the_action_asks_this_computer_for_claudes_and_codexs_lists_in_the_focused_folder() {
    let (mut app, mut rx) = app_with_outbox();
    let dir = focused_dir(&app).display().to_string();
    sent(&mut rx);
    app.act(Action::ResumeConversation);
    let asked = asked(&sent(&mut rx));
    assert_eq!(
        asked
            .iter()
            .map(|(a, f, _)| (a.as_str(), f.as_str()))
            .collect::<Vec<_>>(),
        [("claude", dir.as_str()), ("codex", dir.as_str())]
    );
    assert_ne!(asked[0].2, asked[1].2, "each list has its own tag");
}

#[test]
fn the_answers_list_every_agents_conversations_newest_first() {
    let (mut app, mut rx) = app_with_outbox();
    frame(&mut app, "before");
    app.act(Action::ResumeConversation);
    let tags: Vec<u64> = asked(&sent(&mut rx))
        .into_iter()
        .map(|(_, _, t)| t)
        .collect();
    reply(
        &mut app,
        tags[0],
        SavedConversations::Listed(vec![
            saved("c-new", "fix the footer spacing on mobile", 3 * 60),
            saved(
                "c-old",
                "add a payments page with stripe checkout and webhooks",
                2 * 86_400,
            ),
        ]),
    );
    assert!(
        !matches!(app.mode, Mode::ScreenMenu(_)),
        "waits for every agent"
    );
    reply(
        &mut app,
        tags[1],
        SavedConversations::Listed(vec![saved(
            "x-mid",
            "why does the build fail on windows",
            5 * 3600,
        )]),
    );
    let text = frame(&mut app, "the resume menu");
    let Mode::ScreenMenu(m) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    let labels: Vec<&str> = m.items.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Claude · 3m ago · fix the footer spacing on…",
            "Codex · 5h ago · why does the build fail on…",
            "Claude · 2d ago · add a payments page with …",
        ]
    );
    for label in labels {
        assert!(text.contains(label), "drawn: {label}\n{text}");
    }
}

#[test]
fn with_none_saved_it_says_so_in_one_line() {
    let (mut app, mut rx) = app_with_outbox();
    app.act(Action::ResumeConversation);
    let tags: Vec<u64> = asked(&sent(&mut rx))
        .into_iter()
        .map(|(_, _, t)| t)
        .collect();
    reply(&mut app, tags[0], SavedConversations::Listed(Vec::new()));
    reply(&mut app, tags[1], SavedConversations::NotSupported);
    let text = frame(&mut app, "none saved");
    let (said, _, error) = app.notice.clone().expect("a toast");
    assert_eq!(said, "no saved Claude or Codex chats");
    assert!(!error && !said.contains('\n'));
    assert!(text.contains(&said), "{text}");
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn a_pick_starts_that_agent_in_the_folder_with_the_conversation() {
    let (mut app, mut rx) = app_with_outbox();
    let dir = focused_dir(&app);
    app.act(Action::ResumeConversation);
    let tags: Vec<u64> = asked(&sent(&mut rx))
        .into_iter()
        .map(|(_, _, t)| t)
        .collect();
    reply(&mut app, tags[0], SavedConversations::Listed(Vec::new()));
    reply(
        &mut app,
        tags[1],
        SavedConversations::Listed(vec![saved("019a-codex_id", "hello", 60)]),
    );
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let started: Vec<(Option<PathBuf>, Option<String>)> = sent(&mut rx)
        .into_iter()
        .filter_map(|o| match o.msg {
            ClientMsg::Command(Command::NewTab { cmd, .. }) => Some((Some(dir.clone()), cmd)),
            ClientMsg::Command(Command::NewWorkspace { cwd, cmd, .. }) => Some((cwd, cmd)),
            _ => None,
        })
        .collect();
    assert_eq!(
        started,
        [(Some(dir), Some("codex resume 019a-codex_id".to_string()))]
    );
}

#[test]
fn an_id_that_is_not_plain_is_never_put_on_a_command_line() {
    let (mut app, mut rx) = app_with_outbox();
    let dir = focused_dir(&app);
    sent(&mut rx);
    for id in ["", "abc; rm -rf ~", "../x", "a b", "$(boom)"] {
        app.notice = None;
        app.menu_do(menu::Act::Resume(dir.clone(), "claude".into(), id.into()));
        let (said, _, error) = app.notice.clone().expect("a toast");
        assert!(error, "{id}: {said}");
        assert!(sent(&mut rx).is_empty(), "{id} started something");
    }
}
