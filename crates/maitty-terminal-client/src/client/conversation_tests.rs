use super::*;
use crate::conversation::{ConversationId, ExecutionKind, Provider, RecoveryState};
use crate::layout::Node;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn conversation() -> ConversationSummary {
    ConversationSummary {
        id: ConversationId(41),
        generation: 1,
        provider: Provider::Codex,
        execution: ExecutionKind::Structured,
        cwd: PathBuf::from("conversation-project"),
        worktree: None,
        state: RecoveryState::Running,
        status: Status::Working,
        native_term: None,
        checks: Vec::new(),
        title: None,
        archived: false,
        root: None,
        branch: None,
    }
}

fn app() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let (bg_tx, _) = mpsc::unbounded_channel();
    let mut cfg = Config::default();
    cfg.ui.padding = "0".into();
    cfg.ui.motion = false;
    let mut app = App::new(cfg, tx, None, bg_tx);
    app.splash = false;
    app.snap.conversations.push(conversation());
    (app, rx)
}

fn draw(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| {
            render::draw(app, f);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                + "\n"
        })
        .collect::<String>();
    if std::env::var_os(concat!(env!("APP_ENV"), "_SHOW")).is_some() {
        println!("{width}x{height}\n{text}");
    }
    text
}

#[test]
fn structured_only_conversation_renders_without_any_workspace_or_terminal() {
    for (width, height) in [(80, 24), (160, 45)] {
        let (mut app, mut out) = app();
        app.on_first_state();
        assert!(
            out.try_recv().map(|o| o.msg).is_err(),
            "attaching does not implicitly open a terminal"
        );
        let text = draw(&mut app, width, height);
        assert!(text.contains("Codex 41"), "{text}");
        assert!(app.snap.workspaces.is_empty() && app.snap.terms.is_empty());
        assert_eq!(
            app.screen
                .side_items
                .iter()
                .filter(|row| matches!(row, screen::SideItem::Sess(_)))
                .count(),
            1
        );
        assert!(app.screen.tabs.is_empty() && app.parsers.is_empty());
    }
}

#[test]
fn selecting_and_typing_on_an_unbound_conversation_sends_no_terminal_commands() {
    let (mut app, mut out) = app();
    let key = SessionKey::Conversation(ConversationId(41));
    app.on_hy_hit(screen::ScreenHit::Session(ClientSession::local(key)), false);
    for code in [KeyCode::Enter, KeyCode::Right, KeyCode::Char('x')] {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Side));
        assert_eq!(app.screen.cursor, Some(ClientSession::local(key)));
    }
    for action in [
        Action::StopAgent,
        Action::NewTab,
        Action::SplitRight,
        Action::Message,
        Action::Why,
    ] {
        app.act(action);
    }
    app.on_hy_hit(
        screen::ScreenHit::RowMenuSess(ClientSession::local(key)),
        false,
    );
    draw(&mut app, 80, 24);
    assert!(out.try_recv().map(|o| o.msg).is_err());
    assert!(app.screen.tabs.is_empty() && app.parsers.is_empty());
}

#[test]
fn conversation_row_and_selection_survive_native_binding_changes_without_duplicates() {
    let (mut app, mut out) = app();
    let key = SessionKey::Conversation(ConversationId(41));
    app.screen_side_set(screen::SideItem::Sess(ClientSession::local(key)));
    app.screen.saved.session_order.push(key);
    let world = reference_fixture::world(reference_fixture::Stage::None);
    let mut native = world.snap.terms[&1].clone();
    native.conversation_id = Some(ConversationId(41));
    for binding in [None, Some(7), Some(8), None] {
        app.snap.terms.clear();
        app.snap.workspaces.clear();
        if let Some(id) = binding {
            native.id = id;
            app.snap.terms.insert(id, native.clone());
            app.snap.workspaces.push(WorkspaceInfo {
                id: 1,
                name: String::new(),
                cwd: native.cwd.clone(),
                tabs: vec![TabInfo {
                    id: 1,
                    name: String::new(),
                    layout: Node::Leaf(id),
                    focus: id,
                }],
                active_tab: 1,
                git: None,
                color: 0,
                group: None,
                is_new: false,
                worktree: false,
            });
        }
        app.snap.conversations[0].native_term = binding;
        app.screen_fresh();
        let model = app.screen_model();
        let rows = model.iter().flat_map(|p| p.sessions()).collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, ClientSession::local(key));
        assert_eq!(rows[0].native_term, binding.map(ClientTerm::local));
        assert_eq!(app.screen.cursor, Some(ClientSession::local(key)));
        assert_eq!(app.selected_term(), binding.map(ClientTerm::local));
        assert_eq!(app.screen.saved.session_order, [key]);
        assert!(draw(&mut app, 160, 45).contains("Codex 41"));
    }
    assert!(out.try_recv().map(|o| o.msg).is_err());
}

#[test]
fn a_completed_structured_turn_keeps_its_failed_checkpoint_visible() {
    let (mut app, _) = app();
    let world = reference_fixture::world(reference_fixture::Stage::Failed);
    app.snap.conversations[0].status = Status::Done;
    app.snap.conversations[0].checks = world.snap.terms[&1].checks.clone();
    app.screen_fresh();
    let text = draw(&mut app, 160, 45);
    assert!(text.contains("failed"), "{text}");
    assert!(text.contains("Codex 41"));
}

#[test]
fn saved_sidebar_order_and_placement_accept_legacy_integer_pane_ids() {
    let saved: screen::Saved =
        serde_json::from_str(r#"{"session_order":[9,3],"placed":[[9,"sample",false]]}"#).unwrap();
    assert_eq!(
        saved.session_order,
        [SessionKey::Pane(9), SessionKey::Pane(3)]
    );
    assert_eq!(saved.placed[0].0, SessionKey::Pane(9));
    let encoded = serde_json::to_string(&saved).unwrap();
    let restored: screen::Saved = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored.session_order, saved.session_order);
    assert_eq!(restored.placed, saved.placed);
}

#[test]
fn an_unbound_selected_row_never_sends_typing_to_the_focused_native_pane() {
    let (mut app, mut out) = app();
    let world = reference_fixture::world(reference_fixture::Stage::None);
    app.snap.workspaces = world.snap.workspaces;
    app.snap.terms = world.snap.terms;
    app.snap.active_ws = world.snap.active_ws;
    assert!(app.focused().is_some());
    let key = SessionKey::Conversation(ConversationId(41));
    app.screen_side_set(screen::SideItem::Sess(ClientSession::local(key)));
    app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    app.act(Action::StopAgent);
    // Closing the row puts that chat away; nothing reaches the pane that happens to be focused.
    app.act(Action::ClosePane);
    let sent = std::iter::from_fn(|| out.try_recv().map(|o| o.msg).ok()).collect::<Vec<_>>();
    assert!(
        matches!(
            sent.as_slice(),
            [ClientMsg::Command(Command::ConversationArchive {
                id: ConversationId(41),
                archived: true
            })]
        ),
        "{sent:?}"
    );
    assert!(matches!(app.mode, Mode::Side));
    assert_eq!(app.screen.cursor, Some(ClientSession::local(key)));
}

fn app_with_unbound_selection() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let (mut app, out) = app();
    let world = reference_fixture::world(reference_fixture::Stage::None);
    app.snap.workspaces = world.snap.workspaces;
    app.snap.terms = world.snap.terms;
    app.snap.active_ws = world.snap.active_ws;
    assert!(app.focused().is_some());
    app.screen_side_set(screen::SideItem::Sess(ClientSession::local(
        SessionKey::Conversation(ConversationId(41)),
    )));
    (app, out)
}

#[test]
fn pasting_text_on_an_unbound_conversation_never_reaches_the_native_pane() {
    for mode in [Mode::Side, Mode::Normal] {
        let (mut app, mut out) = app_with_unbound_selection();
        app.mode = mode;
        app.on_event(Event::Paste("first\nsecond\n".into()));
        assert!(
            out.try_recv().map(|o| o.msg).is_err(),
            "pasted newlines must not execute in another pane"
        );
    }
}

#[test]
fn pasting_a_windows_key_burst_on_an_unbound_conversation_never_reaches_the_native_pane() {
    let (mut app, mut out) = app_with_unbound_selection();
    let burst = "first\nsecond\n"
        .chars()
        .map(|c| {
            let code = if c == '\n' {
                KeyCode::Enter
            } else {
                KeyCode::Char(c)
            };
            Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
        })
        .collect();
    app.on_burst(burst);
    assert!(
        out.try_recv().map(|o| o.msg).is_err(),
        "burst paste must not execute in another pane"
    );
}

#[test]
fn an_unbound_conversation_has_no_native_input_target() {
    let (app, _) = app_with_unbound_selection();
    assert_eq!(app.typing_to(), None);
}

#[test]
fn an_explicit_native_popup_still_takes_pastes_over_an_unbound_conversation() {
    let (mut app, mut out) = app_with_unbound_selection();
    let mut popup = app.snap.terms.values().next().unwrap().clone();
    popup.id = 999;
    popup.popup = true;
    app.snap.terms.insert(popup.id, popup);
    app.on_paste("popup\ntext".into());
    assert!(
        matches!(out.try_recv().map(|o| o.msg).unwrap(), ClientMsg::Input { term: 999, data } if data == b"popup\rtext")
    );
    assert!(out.try_recv().map(|o| o.msg).is_err());
}

#[test]
fn ordinary_native_pastes_keep_the_focused_pane_and_bracketed_newlines() {
    let (mut app, mut out) = app_with_unbound_selection();
    let term = app.focused().unwrap();
    app.screen.cursor = Some(ClientSession::local(SessionKey::Pane(term.term)));
    app.mode = Mode::Normal;
    let mut parser = app.new_parser(24, 80);
    parser.process(b"\x1b[?2004h");
    app.parsers.insert(term, parser);
    app.on_paste("first\nsecond\n".into());
    assert!(
        matches!(out.try_recv().map(|o| o.msg).unwrap(), ClientMsg::Input { term: target, data } if target == term.term && data == b"\x1b[200~first\rsecond\r\x1b[201~")
    );
    assert!(out.try_recv().map(|o| o.msg).is_err());
}

#[test]
fn an_app_text_box_still_accepts_paste_over_an_unbound_conversation() {
    let (mut app, mut out) = app_with_unbound_selection();
    app.mode = Mode::Compose(Box::new(screen::Compose {
        term: app.focused().unwrap(),
        text: String::new(),
        side: true,
        inbox: None,
    }));
    app.on_paste("draft\r\nmessage".into());
    assert!(matches!(&app.mode, Mode::Compose(c) if c.text == "draft\nmessage"));
    assert!(out.try_recv().map(|o| o.msg).is_err());
}

fn sent(out: &mut mpsc::UnboundedReceiver<Outgoing>) -> Vec<ClientMsg> {
    std::iter::from_fn(|| out.try_recv().map(|o| o.msg).ok()).collect()
}

#[test]
fn a_chat_shows_the_name_given_to_it_and_not_the_automatic_one() {
    let (mut app, _) = app();
    app.snap.conversations[0].title = Some("Fix the login bug".into());
    app.screen_fresh();
    let text = draw(&mut app, 160, 45);
    assert!(text.contains("Fix the login bug"), "{text}");
    assert!(!text.contains("Codex 41"), "{text}");
}

#[test]
fn a_chat_put_away_in_another_client_leaves_this_sidebar() {
    let (mut app, _) = app();
    app.snap.conversations[0].archived = true;
    app.screen_fresh();
    assert_eq!(
        app.screen_model().iter().flat_map(|p| p.sessions()).count(),
        0
    );
}

#[test]
fn closing_a_chat_that_has_no_terminal_puts_it_away_instead_of_refusing() {
    let (mut app, mut out) = app();
    let key = SessionKey::Conversation(ConversationId(41));
    app.on_hy_hit(screen::ScreenHit::Session(ClientSession::local(key)), false);
    app.act(Action::ClosePane);
    assert!(matches!(
        sent(&mut out).as_slice(),
        [ClientMsg::Command(Command::ConversationArchive {
            id: ConversationId(41),
            archived: true
        })]
    ));
}

#[test]
fn renaming_a_chat_sends_the_name_to_the_daemon_so_every_client_shows_it() {
    let (mut app, mut out) = app();
    let key = SessionKey::Conversation(ConversationId(41));
    app.on_hy_hit(screen::ScreenHit::Session(ClientSession::local(key)), false);
    app.act(Action::RenameWorkspace);
    let Mode::Prompt { kind, input } = &mut app.mode else {
        panic!("a rename prompt opens");
    };
    assert_eq!(*kind, PromptKind::RenameConversation(ConversationId(41)));
    *input = "Fix the login bug".into();
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(
        sent(&mut out).as_slice(),
        [ClientMsg::Command(Command::ConversationRename { id: ConversationId(41), title })]
            if title == "Fix the login bug"
    ));
}

#[test]
fn opening_a_chat_in_the_terminal_says_it_lives_in_the_desktop_app_and_sends_nothing() {
    for status in [Status::Idle, Status::Working] {
        let (mut app, mut out) = app();
        app.snap.conversations[0].status = status;
        let key = SessionKey::Conversation(ConversationId(41));
        app.on_hy_hit(screen::ScreenHit::Session(ClientSession::local(key)), false);
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(sent(&mut out).is_empty());
        assert!(matches!(app.mode, Mode::Side));
    }
}
