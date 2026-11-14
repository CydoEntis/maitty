//! Which machine a message goes to, and what a pane's machine lets this window do.

use super::routing::{Feature, local_only_message};
use super::*;
use crate::client::ids::{ClientSession, MachineId, Remote};
use crate::client::reference_fixture::{Stage, world};
use crate::conversation::{ExecutionKind, Provider, RecoveryState};
use crate::protocol::Reply;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const THERE: MachineId = MachineId(1);

const ALL: [Feature; 8] = [
    Feature::Files,
    Feature::Changes,
    Feature::Find,
    Feature::Branches,
    Feature::References,
    Feature::AgentStatus,
    Feature::Worktrees,
    Feature::ResumeConversation,
];

/// Two machines, "build" holding a copy of this one's sessions, and the channel the app sends on.
fn two_machines() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let mut app = world(Stage::None);
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
    let there = app.snap.clone();
    app.remotes.insert(
        THERE,
        Remote {
            name: "build".into(),
            snap: there,
            ..Remote::default()
        },
    );
    app.screen_fresh();
    (app, rx)
}

fn sent(rx: &mut mpsc::UnboundedReceiver<Outgoing>) -> Vec<Outgoing> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn focus(app: &App, machine: MachineId) {
    let tab = app
        .snap_of(machine)
        .unwrap()
        .active()
        .unwrap()
        .tab()
        .unwrap();
    app.cmd_on(machine, Command::FocusPane { term: tab.focus });
}

fn type_a(app: &mut App) {
    app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
}

#[test]
fn typing_in_a_pane_reaches_only_its_machine() {
    let (mut app, mut rx) = two_machines();
    focus(&app, THERE);
    sent(&mut rx);
    type_a(&mut app);
    let out = sent(&mut rx);
    assert!(
        matches!(
            out.as_slice(),
            [Outgoing {
                machine: THERE,
                msg: ClientMsg::Input { .. }
            }]
        ),
        "{out:?}"
    );

    focus(&app, MachineId::LOCAL);
    sent(&mut rx);
    type_a(&mut app);
    let out = sent(&mut rx);
    assert!(
        matches!(
            out.as_slice(),
            [Outgoing {
                machine: MachineId::LOCAL,
                msg: ClientMsg::Input { .. }
            }]
        ),
        "{out:?}"
    );
}

#[test]
fn a_resize_goes_to_the_machine_that_owns_the_pane() {
    let (mut app, mut rx) = two_machines();
    let term = ClientTerm::on(THERE, 1);
    app.panes = vec![(term, ratatui::layout::Rect::new(0, 0, 100, 30))];
    app.sync_sizes();
    let out = sent(&mut rx);
    assert!(
        matches!(
            out.as_slice(),
            [Outgoing {
                machine: THERE,
                msg: ClientMsg::Resize {
                    term: 1,
                    cols: 100,
                    rows: 30
                }
            }]
        ),
        "{out:?}"
    );
}

#[test]
fn focus_reports_and_pastes_go_to_the_pane_s_machine() {
    let (app, mut rx) = two_machines();
    let term = ClientTerm::on(THERE, 1);
    app.send_input(term, b"\x1b[I".to_vec());
    app.send_input(term, b"\x1b[200~x\x1b[201~".to_vec());
    assert!(sent(&mut rx).iter().all(|o| o.machine == THERE));
}

fn new_session(app: &mut App) {
    app.screen_new_session("/srv/app".into(), None, false);
}

fn new_workspace_machines(out: &[Outgoing]) -> Vec<MachineId> {
    out.iter()
        .filter(|o| matches!(o.msg, ClientMsg::Command(Command::NewWorkspace { .. })))
        .map(|o| o.machine)
        .collect()
}

#[test]
fn a_new_session_starts_on_the_focused_sessions_machine() {
    let (mut app, mut rx) = two_machines();
    focus(&app, THERE);
    sent(&mut rx);
    new_session(&mut app);
    assert_eq!(new_workspace_machines(&sent(&mut rx)), [THERE]);

    focus(&app, MachineId::LOCAL);
    sent(&mut rx);
    new_session(&mut app);
    assert_eq!(new_workspace_machines(&sent(&mut rx)), [MachineId::LOCAL]);
}

#[test]
fn a_new_session_with_nothing_focused_is_local() {
    let (mut app, mut rx) = two_machines();
    app.snap.active_ws = None;
    app.remotes.get_mut(&THERE).unwrap().snap.active_ws = None;
    assert_eq!(app.focused(), None);
    new_session(&mut app);
    assert_eq!(new_workspace_machines(&sent(&mut rx)), [MachineId::LOCAL]);
}

#[test]
fn the_selected_row_decides_where_a_new_session_starts() {
    let (mut app, mut rx) = two_machines();
    app.screen.cursor = Some(ClientSession::on(
        THERE,
        crate::protocol::SessionKey::Pane(1),
    ));
    new_session(&mut app);
    assert_eq!(new_workspace_machines(&sent(&mut rx)), [THERE]);
}

#[test]
fn a_new_tab_and_a_split_start_on_the_focused_sessions_machine() {
    let (mut app, mut rx) = two_machines();
    focus(&app, THERE);
    sent(&mut rx);
    app.act(Action::SpawnTab("htop".into()));
    app.act(Action::SplitRight);
    let out = sent(&mut rx);
    assert!(
        out.iter()
            .filter(|o| matches!(
                o.msg,
                ClientMsg::Command(Command::NewTab { .. } | Command::NewWorkspace { .. })
            ))
            .all(|o| o.machine == THERE),
        "{out:?}"
    );
    assert!(
        out.iter()
            .any(|o| matches!(o.msg, ClientMsg::Command(Command::NewTab { .. })))
    );
}

#[test]
fn a_tab_prompt_and_a_close_go_to_the_machine_holding_the_focus() {
    let (mut app, mut rx) = two_machines();
    focus(&app, THERE);
    sent(&mut rx);
    app.submit_prompt(PromptKind::RenameTab(1, 1), "logs".into());
    app.cmd_on(app.focus_machine(), Command::CloseWorkspace { ws: 1 });
    let out = sent(&mut rx);
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|o| o.machine == THERE), "{out:?}");
}

#[test]
fn a_chat_command_goes_to_the_machine_that_lists_the_chat() {
    let (mut app, mut rx) = two_machines();
    let id = crate::conversation::ConversationId(41);
    let chat = ConversationSummary {
        id,
        generation: 1,
        provider: Provider::Codex,
        execution: ExecutionKind::Structured,
        cwd: PathBuf::from("/srv/app"),
        worktree: None,
        state: RecoveryState::Running,
        status: Status::Working,
        native_term: None,
        checks: Vec::new(),
        title: None,
        archived: false,
        root: None,
        branch: None,
    };
    app.remotes.get_mut(&THERE).unwrap().snap.conversations = vec![chat];
    app.archive_conversation(id);
    let out = sent(&mut rx);
    assert!(
        matches!(
            out.as_slice(),
            [Outgoing {
                machine: THERE,
                msg: ClientMsg::Command(Command::ConversationArchive { .. })
            }]
        ),
        "{out:?}"
    );
}

#[test]
fn the_gate_is_per_pane_and_names_the_machine_and_the_reason() {
    let (app, _rx) = two_machines();
    for feature in ALL {
        assert_eq!(
            app.local_only(MachineId::LOCAL, feature),
            Ok(()),
            "{feature:?}"
        );
        let why = app.local_only(THERE, feature).unwrap_err();
        assert_eq!(why, local_only_message(feature, "build"));
        assert!(
            why.contains("build") && why.contains("local-only in v1"),
            "{why}"
        );
        assert!(why.contains("not on this computer"), "{why}");
    }
    assert_eq!(
        local_only_message(Feature::Files, "build"),
        "Files is local-only in v1: the files are on build, not on this computer."
    );
}

#[test]
fn focus_moving_between_a_local_and_a_remote_pane_toggles_the_gate() {
    let (mut app, mut rx) = two_machines();
    let verdict = |app: &App| {
        let machine = app.selected_term().unwrap().machine;
        app.local_only(machine, Feature::Files)
    };
    assert_eq!(verdict(&app), Ok(()));

    focus(&app, THERE);
    assert!(verdict(&app).is_err());
    app.notice = None;
    app.act(Action::Files);
    let (text, _, error) = app.notice.clone().expect("a toast");
    assert_eq!(text, local_only_message(Feature::Files, "build"));
    assert!(error && app.view.is_none());

    focus(&app, MachineId::LOCAL);
    assert_eq!(verdict(&app), Ok(()));
    sent(&mut rx);
}

#[test]
fn every_gated_action_is_refused_on_a_remote_pane_and_none_opens_anything() {
    for action in [
        Action::Files,
        Action::Changes,
        Action::Find(0),
        Action::Branches,
        Action::References,
        Action::AgentStatusSetup,
        Action::NewWorktree(None),
        Action::RemoveWorktree,
        Action::TidyWorktrees,
        Action::ResumeConversation,
    ] {
        let (mut app, mut rx) = two_machines();
        focus(&app, THERE);
        sent(&mut rx);
        app.notice = None;
        app.act(action.clone());
        let (text, _, error) = app
            .notice
            .clone()
            .unwrap_or_else(|| panic!("{action:?}: no toast"));
        assert!(
            error && text.contains("local-only in v1") && text.contains("build"),
            "{text}"
        );
        assert!(
            app.view.is_none() && matches!(app.mode, Mode::Normal),
            "{action:?}"
        );
        assert!(
            sent(&mut rx).is_empty(),
            "{action:?} sent {:?}",
            sent(&mut rx)
        );
    }
}

#[test]
fn pasting_an_image_into_a_remote_pane_is_sent_to_that_machine_and_not_refused() {
    let (mut app, mut rx) = two_machines();
    app.cfg.machines.push(maitty_settings::config::MachineDef {
        name: "build".into(),
        target: "build".into(),
        ..Default::default()
    });
    focus(&app, THERE);
    sent(&mut rx);
    app.notice = None;
    app.act(Action::PasteImage);
    let (text, _, error) = app.notice.clone().expect("a toast");
    assert!(!error, "{text}");
    assert_eq!(text, "sending the clipboard image to build…");
    assert!(
        !text.contains("local-only"),
        "an image can be sent to another machine"
    );
    assert!(
        sent(&mut rx).is_empty(),
        "nothing goes to a daemon: the image goes over ssh"
    );
}

#[test]
fn a_remote_pane_whose_machine_is_not_configured_cannot_be_sent_an_image() {
    let (mut app, mut rx) = two_machines();
    focus(&app, THERE);
    sent(&mut rx);
    app.notice = None;
    app.act(Action::PasteImage);
    let (text, _, error) = app.notice.clone().expect("a toast");
    assert!(error, "{text}");
    assert_eq!(text, "build is not one of this computer's machines");
    assert!(sent(&mut rx).is_empty());
}

/// A folder on this computer that git would read as a repo.
fn local_repo() -> PathBuf {
    // One folder per call: tests run in parallel, and a shared name lets one delete the
    // other's repo mid-test.
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("routing-repo-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::write(dir.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
    dir
}

#[test]
fn a_remote_cwd_that_exists_here_is_never_read_as_a_local_folder() {
    let repo = local_repo();
    assert!(crate::gitfs::head(&repo).is_some(), "git would read it");
    let (mut app, mut rx) = two_machines();
    for t in app.remotes.get_mut(&THERE).unwrap().snap.terms.values_mut() {
        t.cwd = repo.clone();
        t.top = Some(repo.clone());
        t.root = Some(repo.clone());
    }
    app.screen_fresh();
    focus(&app, THERE);
    sent(&mut rx);
    let remote = app.focused().unwrap();
    assert_eq!(remote.machine, THERE);

    assert_eq!(app.local_cwd(remote), None);
    assert!(app.local_cwd(ClientTerm::local(1)).is_some());
    assert_eq!(
        app.here_dir(),
        repo,
        "kept as the other machine's path, not checked here"
    );

    for action in [Action::Changes, Action::Files, Action::Find(1)] {
        app.notice = None;
        app.act(action);
        assert!(app.notice.is_some());
        assert!(app.view.is_none() && matches!(app.mode, Mode::Normal));
    }
    app.notice = None;
    app.open_path_from(remote, ".", None);
    assert!(app.notice.is_some() && app.view.is_none());

    let model = app.screen_model();
    let local: Vec<_> = app.local_git_projects(&model).map(|p| p.machine).collect();
    assert!(
        !local.contains(&THERE),
        "a remote project is not counted or compared with git here"
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn a_remote_project_is_not_a_menu_to_run_git_on() {
    let repo = local_repo();
    let (mut app, _rx) = two_machines();
    for t in app.remotes.get_mut(&THERE).unwrap().snap.terms.values_mut() {
        t.cwd = repo.clone();
        t.top = Some(repo.clone());
        t.root = Some(repo.clone());
    }
    app.screen_fresh();
    let (_, items) = app.session_items(ClientTerm::on(THERE, 1)).unwrap();
    assert!(
        !items
            .iter()
            .any(|(label, _)| label == "Changes" || label.starts_with("Snapshots")),
        "{:?}",
        items.iter().map(|(l, _)| l).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn an_answer_is_taken_only_for_something_this_window_asked() {
    let (mut app, _rx) = two_machines();
    app.notice = None;
    app.on_machine_server(THERE, ServerMsg::Error("boom".into()));
    assert!(app.notice.is_none(), "nobody asked");

    app.cmd_on(THERE, Command::ClosePane { term: 1 });
    app.on_machine_server(THERE, ServerMsg::Error("no such pane".into()));
    assert_eq!(app.notice.as_ref().unwrap().0, "[build] no such pane");

    app.notice = None;
    app.on_machine_server(THERE, ServerMsg::Error("again".into()));
    assert!(app.notice.is_none(), "one request, one answer");
}

#[test]
fn a_request_to_one_machine_does_not_let_another_one_answer() {
    let (mut app, _rx) = two_machines();
    app.cmd_on(MachineId::LOCAL, Command::ClosePane { term: 1 });
    app.notice = None;
    app.on_machine_server(THERE, ServerMsg::Error("boom".into()));
    assert!(app.notice.is_none());
    app.on_server(ServerMsg::Error("mine".into()));
    assert_eq!(app.notice.as_ref().unwrap().0, "mine");
}

#[test]
fn a_remote_answer_is_cleaned_before_it_is_shown() {
    let (mut app, _rx) = two_machines();
    app.cmd_on(THERE, Command::ClosePane { term: 1 });
    app.on_machine_server(THERE, ServerMsg::Error("\u{1b}[2Jgone\nnow".into()));
    let text = app.notice.as_ref().unwrap().0.clone();
    assert!(!text.chars().any(char::is_control), "{text:?}");
}

#[test]
fn a_remote_worktree_listing_never_fills_this_computers_picker() {
    let (mut app, _rx) = two_machines();
    app.mode = Mode::Worktrees {
        ws: 1,
        cmd: None,
        items: None,
        query: String::new(),
        sel: 0,
    };
    app.send(THERE, ClientMsg::Query(Query::Worktrees { ws: 1 }));
    app.on_machine_server(THERE, ServerMsg::Reply(Reply::Worktrees(Vec::new())));
    assert!(matches!(&app.mode, Mode::Worktrees { items: None, .. }));
}

#[test]
fn a_request_nobody_answers_stops_counting_after_a_while() {
    let (app, _rx) = two_machines();
    for _ in 0..100 {
        app.cmd_on(THERE, Command::FocusPane { term: 1 });
    }
    assert!(app.route.awaiting(THERE));
    for _ in 0..64 {
        app.route.answered(THERE);
    }
    assert!(
        !app.route.awaiting(THERE),
        "the queue is capped, not unbounded"
    );
}

#[test]
fn a_command_for_a_machine_that_is_down_says_so_once_and_does_not_panic() {
    let (mut app, _rx) = two_machines();
    let down = Connections::default();
    let cmd = |c| Outgoing {
        machine: THERE,
        msg: ClientMsg::Command(c),
    };
    app.deliver(
        cmd(Command::NewWorkspace {
            cwd: None,
            name: None,
            cmd: None,
        }),
        &down,
    );
    let (text, _, error) = app.notice.clone().expect("a toast");
    assert_eq!(text, "build is disconnected; new workspace was not sent.");
    assert!(error);

    app.notice = None;
    app.deliver(
        Outgoing {
            machine: THERE,
            msg: ClientMsg::Resize {
                term: 1,
                cols: 80,
                rows: 24,
            },
        },
        &down,
    );
    assert!(app.notice.is_none(), "a resize is not worth a toast");
}

#[test]
fn typing_for_a_machine_that_is_down_is_counted_not_toasted() {
    let (mut app, _rx) = two_machines();
    let down = Connections::default();
    for _ in 0..3 {
        app.deliver(
            Outgoing {
                machine: THERE,
                msg: ClientMsg::Input {
                    term: 1,
                    data: b"a".to_vec(),
                },
            },
            &down,
        );
    }
    assert_eq!(app.unsent_input[&THERE], 3);
    assert!(app.notice.is_none());
}

#[test]
fn a_machine_that_is_gone_gives_the_focus_back_to_this_computer() {
    let (mut app, _rx) = two_machines();
    focus(&app, THERE);
    assert_eq!(app.focus_machine(), THERE);
    app.remotes.remove(&THERE);
    assert_eq!(app.focus_machine(), MachineId::LOCAL);
    assert_eq!(app.focused().map(|t| t.machine), Some(MachineId::LOCAL));
}

#[test]
fn sending_a_file_from_a_local_pane_says_there_is_nothing_to_send() {
    let (mut app, _rx) = two_machines();
    app.notice = None;
    app.act(Action::SendFile);
    let (text, _, _) = app.notice.clone().expect("a toast");
    assert_eq!(
        text,
        "this pane is on this computer; there is nothing to send"
    );
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn sending_a_file_from_a_remote_pane_asks_for_the_path_and_nothing_goes_to_a_daemon() {
    let (mut app, mut rx) = two_machines();
    focus(&app, THERE);
    sent(&mut rx);
    app.act(Action::SendFile);
    assert!(matches!(
        app.mode,
        Mode::Prompt {
            kind: PromptKind::SendFile(_),
            ..
        }
    ));
    // A machine this computer has no entry for cannot be reached: said, not guessed at.
    app.mode = Mode::Normal;
    let term = app.typing_to().expect("a pane");
    app.start_send(term, "/tmp/some-file.txt");
    let (text, _, error) = app.notice.clone().expect("a toast");
    assert!(error, "{text}");
    assert_eq!(text, "build is not one of this computer's machines");
    assert!(app.sending.is_none());
    assert!(sent(&mut rx).is_empty());
}

#[test]
fn a_file_that_cannot_be_sent_ends_as_an_error_toast_and_frees_the_next_send() {
    let (mut app, mut rx) = two_machines();
    app.cfg.machines.push(maitty_settings::config::MachineDef {
        name: "build".into(),
        target: "build".into(),
        ..Default::default()
    });
    let (bg_tx, mut bg_rx) = mpsc::unbounded_channel();
    app.bg = bg_tx;
    app.run_background_jobs_inline = true;
    focus(&app, THERE);
    sent(&mut rx);
    let term = app.typing_to().expect("a pane");
    app.start_send(term, "'/no/such/dir/file.txt'");
    assert!(app.sending.is_some(), "the send is shown while it runs");
    let done = bg_rx.try_recv().expect("the job's result");
    app.on_bg(done);
    let (text, _, error) = app.notice.clone().expect("a toast");
    assert!(error, "{text}");
    assert!(text.starts_with("could not read file.txt"), "{text}");
    assert!(app.sending.is_none(), "the next send may start");
}

#[test]
fn an_agent_s_image_paste_key_on_another_machine_looks_here_first_then_goes_on_as_typed() {
    let (mut app, mut rx) = two_machines();
    let (btx, mut bg) = mpsc::unbounded_channel();
    app.bg = btx;
    focus(&app, THERE);
    sent(&mut rx);
    let alt_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::ALT);
    app.on_key(alt_v);
    assert!(sent(&mut rx).is_empty(), "held while this computer's clipboard is read");
    // Tests read no clipboard, so there is no image: the key reaches the agent unchanged.
    let started = std::time::Instant::now();
    let read = loop {
        if let Ok(read) = bg.try_recv() {
            break read;
        }
        assert!(started.elapsed().as_secs() < 10, "the clipboard read never came back");
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    app.on_bg(read);
    let out = sent(&mut rx);
    assert!(
        matches!(out.as_slice(), [Outgoing { machine: THERE, msg: ClientMsg::Input { data, .. } }] if data == b"\x1bv"),
        "{out:?}"
    );

    focus(&app, MachineId::LOCAL);
    sent(&mut rx);
    app.on_key(alt_v);
    assert_eq!(sent(&mut rx).len(), 1, "an agent here reads this clipboard itself");
}
