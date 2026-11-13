//! The daemon's own logic, driven through a real daemon with real panes (plain shells).

use super::worktrees::Removal;
use super::*;
use std::time::Duration;

/// Shell panes write their startup files and agent launchers to the data folder: point it at a
/// scratch folder (Linux honours this) so tests leave the real one alone.
fn own_data_folder() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-test-data-{}"),
            std::process::id()
        ));
        // SAFETY: first thing a daemon test does, before the panes it starts read the environment.
        unsafe { std::env::set_var("XDG_DATA_HOME", dir) };
    });
}

fn daemon() -> (Daemon, mpsc::Receiver<Ev>) {
    own_data_folder();
    let (tx, rx) = mpsc::channel(4096);
    let mut d = Daemon::new(Config::default(), tx, || {});
    // Its own folder (made only if something is written to it), so tests never share check tails.
    d.check_dir = persist::scratch_dir().path().to_path_buf();
    (d, rx)
}

#[test]
fn startup_callback_runs_once_when_the_daemon_is_created() {
    own_data_folder();
    let (tx, _rx) = mpsc::channel(4096);
    let calls = std::cell::Cell::new(0);
    let _daemon = Daemon::new(Config::default(), tx, || calls.set(calls.get() + 1));
    assert_eq!(calls.get(), 1);
}

fn pane(d: &mut Daemon) -> TermId {
    d.spawn(None, &std::env::temp_dir(), 80, 24)
        .expect("a shell pane")
}

fn report(term: TermId, token: &str, pid: u32, status: HookStatus, event: &str) -> ClientMsg {
    ClientMsg::Hook {
        term,
        agent: "claude".into(),
        status,
        session: None,
        cwd: None,
        prompt: None,
        said: None,
        subagent: None,
        event: event.into(),
        pid,
        token: token.into(),
        transcript: None,
        model: None,
        name: None,
        background: Vec::new(),
        background_agents: Vec::new(),
        background_reported: false,
        ask: None,
        call: None,
    }
}

/// Apply checked status reports as the daemon loop would, until `n` have come back (or
/// `wait` passes). Returns how many did.
fn settle_within(d: &mut Daemon, rx: &mut mpsc::Receiver<Ev>, n: usize, wait: Duration) -> usize {
    let until = Instant::now() + wait;
    let mut got = 0;
    while got < n && Instant::now() < until {
        match rx.try_recv() {
            Ok(ev @ Ev::HookChecked { .. }) => {
                d.handle(ev);
                got += 1;
            }
            Ok(_) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    got
}

fn settle(d: &mut Daemon, rx: &mut mpsc::Receiver<Ev>, n: usize) -> usize {
    settle_within(d, rx, n, Duration::from_secs(10))
}

fn close(d: &mut Daemon, terms: &[TermId]) {
    for t in terms {
        d.close_term(*t);
    }
}

#[test]
fn codex_disk_results_cannot_cross_panes_or_survive_a_thread_change() {
    let (mut d, _) = daemon();
    let a = pane(&mut d);
    let b = pane(&mut d);
    for (term, session) in [(a, "root-a"), (b, "root-b")] {
        let t = d.terms.get_mut(&term).unwrap();
        t.agent = Some("codex".into());
        t.session = Some(session.into());
    }
    let facts = |term, session: &str, name: &str| usage::CodexPaneFacts {
        term,
        session: Some(session.into()),
        usage: None,
        subagents: vec![(format!("{session}-child"), name.into())],
        subagent_activity: Vec::new(),
    };
    d.codex_usage(
        vec![
            facts(a, "root-a", "planner"),
            facts(b, "root-b", "reviewer"),
        ],
        None,
    );
    assert_eq!(
        d.terms[&a].subagents,
        vec![("root-a-child".into(), "planner".into())]
    );
    assert_eq!(
        d.terms[&b].subagents,
        vec![("root-b-child".into(), "reviewer".into())]
    );
    let t = d.terms.get_mut(&a).unwrap();
    t.session = Some("replacement".into());
    t.subagents.clear();
    d.codex_usage(vec![facts(a, "root-a", "stale helper")], None);
    assert!(d.terms[&a].subagents.is_empty());
    d.terms.get_mut(&b).unwrap().agent = Some("pi".into());
    d.terms.get_mut(&b).unwrap().subagents.clear();
    d.codex_usage(vec![facts(b, "root-b", "wrong agent")], None);
    assert!(d.terms[&b].subagents.is_empty());
    close(&mut d, &[a, b]);
}

#[test]
fn disk_reported_helpers_keep_an_unhooked_codex_parent_working() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    {
        let parent = d.terms.get_mut(&t).unwrap();
        parent.agent = Some("codex".into());
        parent.session = Some("root".into());
        parent.status = Status::Idle;
    }
    let facts = |subagents| usage::CodexPaneFacts {
        term: t,
        session: Some("root".into()),
        usage: None,
        subagents,
        subagent_activity: Vec::new(),
    };

    d.codex_usage(vec![facts(vec![("child".into(), "planner".into())])], None);
    assert!(
        d.terms[&t].subagent_seen.is_some(),
        "disk polling supplies helper liveness"
    );
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "the parent remains visibly active without hooks"
    );

    d.codex_usage(vec![facts(Vec::new())], None);
    assert!(
        d.terms[&t].subagent_seen.is_none(),
        "a completed helper clears its liveness timestamp"
    );
    d.terms.get_mut(&t).unwrap().last_output = Instant::now() - Duration::from_secs(10);
    d.update_statuses();
    assert_ne!(
        d.terms[&t].status,
        Status::Working,
        "the parent settles after the helper disappears"
    );
    close(&mut d, &[t]);
}

#[test]
fn codex_live_helper_hooks_attach_the_first_child_and_outvote_stale_disk_data() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    let mut start = report(t, &secret, 0, HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook {
        agent,
        session,
        subagent,
        ..
    } = &mut start
    {
        *agent = "codex".into();
        *session = Some("parent-thread".into());
        *subagent = Some(crate::protocol::Subagent {
            id: "child-thread".into(),
            kind: "reviewer".into(),
            start: true,
            reported_at: 0,
            task: None,
            model: None,
            effort: None,
            activity: None,
        });
    }
    d.apply_hook(start, Some(true), Vec::new());
    assert_eq!(d.terms[&t].session.as_deref(), Some("parent-thread"));
    assert_eq!(
        d.terms[&t].subagents,
        vec![("child-thread".into(), "reviewer".into())]
    );
    assert!(d.terms[&t].subagent_hooks_seen);

    let facts = |subagents| usage::CodexPaneFacts {
        term: t,
        session: Some("parent-thread".into()),
        usage: None,
        subagents,
        subagent_activity: Vec::new(),
    };
    d.codex_usage(vec![facts(Vec::new())], None);
    assert_eq!(
        d.terms[&t].subagents.len(),
        1,
        "an empty disk snapshot cannot erase the live hook entry"
    );

    let mut stop = report(t, &secret, 0, HookStatus::Same, "SubagentStop");
    if let ClientMsg::Hook {
        agent,
        session,
        subagent,
        ..
    } = &mut stop
    {
        *agent = "codex".into();
        *session = Some("parent-thread".into());
        *subagent = Some(crate::protocol::Subagent {
            id: "child-thread".into(),
            kind: "reviewer".into(),
            start: false,
            reported_at: 0,
            task: None,
            model: None,
            effort: None,
            activity: None,
        });
    }
    d.apply_hook(stop, Some(true), Vec::new());
    assert!(d.terms[&t].subagents.is_empty());
    d.codex_usage(
        vec![facts(vec![(
            "child-thread".into(),
            "stale reviewer".into(),
        )])],
        None,
    );
    assert!(
        d.terms[&t].subagents.is_empty(),
        "stale disk data cannot resurrect a stopped helper"
    );
    close(&mut d, &[t]);
}

#[test]
fn status_reports_need_the_pane_secret() {
    let (mut d, mut rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    d.message(
        0,
        report(t, "not-the-secret", 0, HookStatus::Blocked, "PreToolUse"),
    );
    d.message(0, report(t, "", 0, HookStatus::Blocked, "PreToolUse"));
    assert_eq!(
        settle_within(&mut d, &mut rx, 1, Duration::from_secs(1)),
        0,
        "a report without the secret is never even checked"
    );
    assert_eq!(d.terms[&t].status, Status::None);
    d.message(0, report(t, &secret, 0, HookStatus::Blocked, "PreToolUse"));
    assert_eq!(settle(&mut d, &mut rx, 1), 1);
    assert_eq!(
        d.terms[&t].status,
        Status::Blocked,
        "with the secret it counts"
    );
    assert_eq!(
        d.terms[&t].status_why, "hook: PreToolUse",
        "and it says what set it"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_report_traced_outside_the_pane_is_dropped() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    // The hook thread found its process chain doesn't lead to the pane (a desktop app that
    // inherited the pane's environment, say): the report is dropped, secret or not.
    d.apply_hook(
        report(t, &secret, 4242, HookStatus::Blocked, "PreToolUse"),
        Some(false),
        Vec::new(),
    );
    assert_eq!(d.terms[&t].status, Status::None);
    // Traced to the pane: it counts, and the chain is remembered for next time.
    d.apply_hook(
        report(t, &secret, 4242, HookStatus::Blocked, "PreToolUse"),
        Some(true),
        vec![4242, 4243],
    );
    assert_eq!(d.terms[&t].status, Status::Blocked);
    assert!(d.terms[&t].trusted.contains(&4243));
    close(&mut d, &[t]);
}

#[test]
fn a_restore_never_starts_an_agents_task_over() {
    let (d, _rx) = daemon();
    let saved = |agent: Option<&str>, cmd: &str| persist::SavedPane {
        agent: agent.map(String::from),
        cmd: Some(cmd.into()),
        ..Default::default()
    };
    // A helper saved without its agent (it lost it asleep): its command still names codex,
    // so it's resumed, not handed its task again.
    let lost = saved(
        None,
        r#"codex "Fix the starting kit: give each faction its units""#,
    );
    assert_eq!(
        d.agent_in_cmd(lost.cmd.as_deref().unwrap()).as_deref(),
        Some("codex")
    );
    assert_eq!(
        d.restore_cmd(&lost),
        None,
        "no session to resume: the agent isn't run again"
    );
    let named = persist::SavedPane {
        agent: Some("codex".into()),
        ..lost.clone()
    };
    assert_eq!(
        d.restore_cmd(&named).as_deref(),
        Some("codex resume --last"),
        "known: resumed"
    );
    // By path, quoted, with .exe; an agent with no way to resume opens as a shell.
    assert_eq!(
        d.agent_in_cmd(r#""C:\Users\me\.local\bin\claude.exe" --model opus "do it""#)
            .as_deref(),
        Some("claude")
    );
    assert_eq!(
        d.restore_cmd(&saved(Some("gemini"), r#"gemini "write the docs""#)),
        None
    );
    // Anything else still comes back running what it ran.
    assert_eq!(d.agent_in_cmd("npm run dev"), None);
    assert_eq!(
        d.restore_cmd(&saved(None, "npm run dev")).as_deref(),
        Some("npm run dev")
    );
}

#[test]
fn a_session_keeps_its_account_through_sleep_and_restart() {
    let (mut d, _rx) = daemon();
    d.cfg.quick.agents.push(crate::config::QuickAgent {
        name: "claude work".into(),
        command: "claude {prompt}".into(),
        env: [(
            "CLAUDE_CONFIG_DIR".to_string(),
            "~/.claude-work".to_string(),
        )]
        .into(),
        ..Default::default()
    });
    let wrapped = d.cfg.as_account("claude work", "cmd /c exit");
    // Started as the account: the command line says so, and the pane remembers.
    let t = d
        .spawn(Some(&wrapped), &std::env::temp_dir(), 80, 24)
        .expect("it starts");
    assert_eq!(d.terms[&t].account.as_deref(), Some("claude work"));
    let plain = pane(&mut d);
    assert_eq!(d.terms[&plain].account, None);
    // Resumed (after sleep, or a move): under the same account, where its conversation is.
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.session) = (Some("claude".into()), Some("abc123".into()));
    }
    let resume = d.resume_cmd(&d.terms[&t]).expect("claude can be resumed");
    assert!(
        resume.starts_with(concat!(env!("APP_COMMAND"), " account "))
            && resume.contains("claude work")
            && resume.contains("abc123"),
        "{resume}"
    );
    // And after the server restarts.
    let saved = d.saved();
    assert!(
        saved.workspaces.is_empty(),
        "these panes are in no workspace"
    );
    let pane = persist::SavedPane {
        agent: Some("claude".into()),
        session: Some("abc123".into()),
        account: Some("claude work".into()),
        ..Default::default()
    };
    let again = d.restore_cmd(&pane).expect("restored as a resume");
    assert_eq!(again, resume);
    let old: persist::SavedPane = serde_json::from_str(
        r#"{"cwd":null,"cmd":null,"agent":"claude","session":"abc123","aside":0}"#,
    )
    .unwrap();
    assert!(
        !d.restore_cmd(&old)
            .unwrap()
            .starts_with(concat!(env!("APP_COMMAND"), " account")),
        "one saved before accounts resumes as itself"
    );
    close(&mut d, &[t, plain]);
}

#[test]
fn a_queued_follow_up_goes_in_when_the_turn_is_over() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Working);
    }
    let queued = |d: &Daemon| d.snapshot().terms[&t].queued.clone();
    d.command(
        0,
        Command::QueueFollowUp {
            term: t,
            text: "  then run the tests ".into(),
            first: false,
        },
    )
    .unwrap();
    d.command(
        0,
        Command::QueueFollowUp {
            term: t,
            text: "and open a PR".into(),
            first: false,
        },
    )
    .unwrap();
    assert_eq!(
        queued(&d),
        ["then run the tests", "and open a PR"],
        "kept by the server, in order"
    );
    assert!(
        d.command(
            0,
            Command::QueueFollowUp {
                term: t,
                text: "  ".into(),
                first: false,
            }
        )
        .is_err(),
        "nothing to queue"
    );
    // Still working: nothing is typed.
    d.update_statuses();
    assert_eq!(queued(&d).len(), 2);
    // Asking you something: nothing is typed into the question either.
    d.set_status(t, Status::Blocked, "test");
    d.update_statuses();
    assert_eq!(queued(&d).len(), 2, "never into a question");
    // The turn ends: the first goes in, and only the first.
    d.set_status(t, Status::Working, "test");
    d.set_status(t, Status::Done, "test");
    d.update_statuses();
    assert_eq!(queued(&d), ["and open a PR"], "one at a time");
    d.update_statuses();
    assert_eq!(
        queued(&d).len(),
        1,
        "the next waits for the turn that one starts"
    );
    // That turn starts and ends: the next goes.
    d.set_status(t, Status::Working, "test");
    d.set_status(t, Status::Done, "test");
    d.update_statuses();
    assert!(queued(&d).is_empty());
    // Dropping what's queued; a shell takes none.
    d.set_status(t, Status::Working, "test");
    d.command(
        0,
        Command::QueueFollowUp {
            term: t,
            text: "never mind".into(),
            first: false,
        },
    )
    .unwrap();
    d.command(0, Command::ClearQueue { term: t }).unwrap();
    assert!(queued(&d).is_empty());
    // One goes to the front; one is dropped by where it is and what it says.
    for (text, first) in [("one", false), ("two", false), ("/model opus", true)] {
        d.command(
            0,
            Command::QueueFollowUp {
                term: t,
                text: text.into(),
                first,
            },
        )
        .unwrap();
    }
    assert_eq!(queued(&d), ["/model opus", "one", "two"]);
    let drop = |d: &mut Daemon, index, text: &str| {
        d.command(
            0,
            Command::DropQueued {
                term: t,
                index,
                text: text.into(),
            },
        )
    };
    assert!(drop(&mut d, 1, "two").is_err(), "not the one there");
    assert!(drop(&mut d, 9, "one").is_err());
    assert_eq!(queued(&d).len(), 3, "naming the wrong one drops nothing");
    drop(&mut d, 1, "one").unwrap();
    assert_eq!(queued(&d), ["/model opus", "two"]);
    d.command(0, Command::ClearQueue { term: t }).unwrap();
    d.terms.get_mut(&t).unwrap().agent = None;
    assert!(
        d.command(
            0,
            Command::QueueFollowUp {
                term: t,
                text: "ls".into(),
                first: false,
            }
        )
        .is_err()
    );
    close(&mut d, &[t]);
}

#[test]
fn a_message_sent_to_a_starting_agent_is_held_until_it_takes_input() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let held = |d: &Daemon| d.terms[&t].held_message.is_some();
    let send = |d: &mut Daemon, text: &str| {
        d.command(
            0,
            Command::SendWhenReady {
                term: t,
                text: text.into(),
            },
        )
    };
    assert!(send(&mut d, "   ").is_err(), "nothing to send");
    send(&mut d, "  fix the parser ").unwrap();
    assert!(
        send(&mut d, "and another").is_err(),
        "one message waits at a time"
    );
    // Still a shell: the agent has not started.
    d.update_statuses();
    assert!(held(&d));
    // The agent is up but asking something: nothing is typed into its question.
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.hooked) = (Some("claude".into()), true);
    }
    d.set_status(t, Status::Blocked, "test");
    d.update_statuses();
    assert!(held(&d), "never into a question");
    // It takes input: the message goes in, once.
    d.set_status(t, Status::Idle, "test");
    d.update_statuses();
    assert!(!held(&d));
    assert!(
        d.terms[&t].queue_sent.is_some(),
        "a queued follow-up waits for the turn this starts"
    );
    close(&mut d, &[t]);
}

#[test]
fn enter_is_pressed_again_only_when_a_hooked_agent_never_reported_the_message() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let again_after = super::term::ENTER_AGAIN_AFTER;
    let sent = Instant::now();
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Idle);
        p.last_input = sent;
        p.enter_sent = Some(sent);
    }
    let due = |d: &Daemon, rest: bool, now: Instant| d.terms[&t].enter_again_due(rest, now);
    assert!(!due(&d, true, sent + again_after / 2), "the agent has a moment to report it");
    assert!(due(&d, true, sent + again_after), "no report: the message is still in its box");
    assert!(!due(&d, false, sent + again_after), "it is working or asking: it took something");
    // Something was typed into the pane since: Enter would send what the person is writing.
    d.terms.get_mut(&t).unwrap().last_input = sent + Duration::from_millis(500);
    assert!(!due(&d, true, sent + again_after));
    d.terms.get_mut(&t).unwrap().last_input = sent;
    // An agent without hooks reports nothing either way, so nothing can be told.
    d.terms.get_mut(&t).unwrap().hooked = false;
    assert!(!due(&d, true, sent + again_after));
    d.terms.get_mut(&t).unwrap().hooked = true;
    // The agent's report of the prompt is what says it was submitted.
    d.apply_hook(report(t, "", 0, HookStatus::Working, "UserPromptSubmit"), Some(true), Vec::new());
    assert!(d.terms[&t].enter_sent.is_none());
    assert!(!due(&d, true, sent + again_after));
    close(&mut d, &[t]);
}

#[test]
fn a_queued_message_of_several_lines_goes_in_as_one_paste() {
    use super::status::queued_as_typed;
    assert_eq!(
        queued_as_typed("exit 1\nFAIL a\r\nFAIL b", true),
        "\x1b[200~exit 1\rFAIL a\rFAIL b\x1b[201~"
    );
    assert_eq!(
        queued_as_typed("exit 1\nFAIL a", false),
        "exit 1 FAIL a",
        "no paste mode: one line"
    );
    assert_eq!(
        queued_as_typed("then run the tests", true),
        "then run the tests",
        "one line is typed as is"
    );
}

#[test]
fn a_pane_started_as_an_agent_keeps_its_name_while_the_agent_starts() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    // Woken (or moved, or restored): it is claude, with a conversation to resume, before
    // claude's own process exists.
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.session, p.summary) =
            (Some("claude".into()), Some("abc".into()), "the task".into());
    }
    let nothing_yet = |term| scan::Found {
        term,
        process: "powershell".into(),
        agent: None,
        cwd: None,
        agent_cwd: None,
        mem: 0,
        cpu: 0.0,
        remote: None,
    };
    d.handle(Ev::Scan(vec![nothing_yet(t)]));
    let p = &d.terms[&t];
    assert_eq!(
        (p.agent.as_deref(), p.session.as_deref(), p.summary.as_str()),
        (Some("claude"), Some("abc"), "the task"),
        "still claude, conversation and all"
    );
    // Asleep, long after: there's no process to find, and it's still claude.
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.asleep, p.born) = (
            true,
            Instant::now() - super::AGENT_SHOWS_WITHIN - Duration::from_secs(1),
        );
    }
    d.handle(Ev::Scan(vec![nothing_yet(t)]));
    assert_eq!(
        (d.terms[&t].agent.as_deref(), d.terms[&t].session.as_deref()),
        (Some("claude"), Some("abc")),
        "a sleeping agent stays an agent"
    );
    d.terms.get_mut(&t).unwrap().asleep = false;
    // Long past the time an agent takes to start, with still no agent there: it's a shell.
    d.terms.get_mut(&t).unwrap().born =
        Instant::now() - super::AGENT_SHOWS_WITHIN - Duration::from_secs(1);
    d.handle(Ev::Scan(vec![nothing_yet(t)]));
    assert_eq!(d.terms[&t].agent, None);
    close(&mut d, &[t]);
}

#[test]
fn process_scan_cannot_erase_a_native_conversations_provider_identity() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    {
        let p = d.terms.get_mut(&t).unwrap();
        p.conversation_id = Some(crate::conversation::ConversationId(7));
        p.agent = Some("codex".into());
        p.session = Some("exact-thread".into());
        p.status = Status::Done;
    }
    d.handle(Ev::Scan(vec![scan::Found {
        term: t,
        process: "node".into(),
        agent: None,
        cwd: None,
        agent_cwd: None,
        mem: 0,
        cpu: 0.0,
        remote: None,
    }]));
    let p = &d.terms[&t];
    assert_eq!(p.agent.as_deref(), Some("codex"));
    assert_eq!(p.session.as_deref(), Some("exact-thread"));
    assert_eq!(p.status, Status::Done);
    close(&mut d, &[t]);
}

#[test]
fn an_agent_is_working_while_one_of_its_own_runs_in_the_background() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Idle);
        // Its turn ended, however that came about, with a background agent of its own still going.
        p.subagents.push(("a1".into(), "general-purpose".into()));
        p.subagent_seen = Some(Instant::now());
    }
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "still working: its own agent is"
    );
    assert!(
        d.terms[&t].done_held.is_some(),
        "and held, so that agent finishing ends it"
    );
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "steady: it doesn't flip back and forth"
    );
    // The background agent finishes: now the turn is over.
    let secret = d.terms[&t].token.clone();
    let mut stop = report(t, &secret, 0, HookStatus::Same, "SubagentStop");
    if let ClientMsg::Hook { subagent, .. } = &mut stop {
        *subagent = Some(crate::protocol::Subagent {
            id: "a1".into(),
            kind: "general-purpose".into(),
            start: false,
            reported_at: 0,
            task: None,
            model: None,
            effort: None,
            activity: None,
        });
    }
    d.apply_hook(stop, Some(true), Vec::new());
    assert!(d.terms[&t].subagents.is_empty());
    assert_eq!(
        d.terms[&t].status,
        Status::Done,
        "its last one finished: done"
    );
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done, "and it stays done");
    // One that went silent long ago (it died without saying so) doesn't hold it working.
    {
        let p = d.terms.get_mut(&t).unwrap();
        p.status = Status::Idle;
        p.subagents.push(("a2".into(), "general-purpose".into()));
        p.subagent_seen =
            Some(Instant::now() - super::status::SUBAGENTS_QUIET_AT_MOST - Duration::from_secs(1));
    }
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Idle);
    close(&mut d, &[t]);
}

#[test]
fn only_so_many_helpers_work_at_once() {
    let (mut d, _rx) = daemon();
    d.cfg.sessions.helpers_at_once = "2".into();
    let here = Some(std::env::temp_dir());
    d.command(
        0,
        Command::NewWorkspace {
            cwd: here.clone(),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let starter = d.workspaces[0].tabs[0].focus;
    let token = d.terms[&starter].token.clone();
    let mut c = from_pane(&mut d, 1, starter, &token);
    let start = |d: &mut Daemon| {
        d.handle(Ev::Msg(
            1,
            ClientMsg::Command(Command::NewWorkspace {
                cwd: Some(std::env::temp_dir()),
                name: None,
                cmd: None,
            }),
        ))
    };
    let refused = |c: &mut mpsc::Receiver<ServerMsg>| {
        std::iter::from_fn(|| c.try_recv().ok()).find_map(|m| {
            if let ServerMsg::Error(e) = m {
                Some(e)
            } else {
                None
            }
        })
    };
    // Two helpers, just started: both count as working until their agents report.
    start(&mut d);
    start(&mut d);
    assert_eq!(d.terms.len(), 3);
    assert!(refused(&mut c).is_none());
    // A third is refused, and told what to do.
    start(&mut d);
    let why = refused(&mut c).expect("the third is refused");
    assert!(
        d.terms.len() == 3
            && why.contains("2 helpers are already working")
            && why.contains("Wait for one to finish"),
        "{why}"
    );
    // One finishes (and has been up a while): there's room again.
    let helpers: Vec<TermId> = d.terms.keys().copied().filter(|t| *t != starter).collect();
    let done = d.terms.get_mut(&helpers[0]).unwrap();
    (done.status, done.born) = (
        Status::Done,
        Instant::now() - super::HELPER_SETTLES - Duration::from_secs(1),
    );
    start(&mut d);
    assert_eq!(d.terms.len(), 4, "a helper finished: the next may start");
    // What you start yourself never counts, and never waits.
    d.command(
        0,
        Command::NewWorkspace {
            cwd: here.clone(),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    assert_eq!(d.terms.len(), 5);
    let all: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &all);
}

#[test]
fn with_no_window_a_transient_question_cannot_ring_after_work_resumes() {
    let (mut d, _) = daemon();
    let term = pane(&mut d);
    d.terms.get_mut(&term).unwrap().agent = Some("pi".into());
    d.set_status(term, Status::Working, "test");
    d.set_status(term, Status::Blocked, "test");
    assert!(d.terms[&term].alerted.is_none());
    assert_eq!(d.terms[&term].alert_due.unwrap().0, Status::Blocked);
    d.set_status(term, Status::Working, "test");
    assert!(d.terms[&term].alert_due.is_none());
    d.settled_alerts();
    assert!(d.terms[&term].alerted.is_none());
    // Even an obsolete queued deadline must match the current status.
    d.terms.get_mut(&term).unwrap().alert_due =
        Some((Status::Blocked, Instant::now() - Duration::from_millis(1)));
    d.settled_alerts();
    assert!(d.terms[&term].alerted.is_none());
    // A real question remains red and gets one request sound after settling.
    d.set_status(term, Status::Blocked, "test");
    d.terms
        .get_mut(&term)
        .unwrap()
        .alert_due
        .as_mut()
        .unwrap()
        .1 = Instant::now() - Duration::from_millis(1);
    d.settled_alerts();
    assert_eq!(d.terms[&term].alerted.unwrap().0, Status::Blocked);
    close(&mut d, &[term]);
}

#[test]
fn with_no_window_open_finished_waits_and_a_helper_stays_quiet() {
    let (mut d, _rx) = daemon();
    let (starter, helper) = (pane(&mut d), pane(&mut d));
    for t in [starter, helper] {
        d.terms.get_mut(&t).unwrap().agent = Some("claude".into());
    }
    let rang = |d: &Daemon, t: TermId| d.terms[&t].alerted.map(|(s, _)| s);
    let overdue = |d: &mut Daemon, t: TermId| {
        d.terms.get_mut(&t).unwrap().alert_due.as_mut().unwrap().1 =
            Instant::now() - Duration::from_millis(1)
    };
    // Finished: not said yet.
    d.set_status(starter, Status::Working, "test");
    d.set_status(starter, Status::Done, "test");
    assert!(
        rang(&d, starter).is_none() && d.terms[&starter].alert_due.is_some(),
        "it waits"
    );
    // Back to work before the wait is over: never said.
    d.set_status(starter, Status::Working, "test");
    assert!(d.terms[&starter].alert_due.is_none());
    d.settled_alerts();
    assert!(rang(&d, starter).is_none());
    // Finished and still finished: said.
    d.set_status(starter, Status::Done, "test");
    overdue(&mut d, starter);
    d.settled_alerts();
    assert_eq!(rang(&d, starter), Some(Status::Done));
    // A helper's finish is its starter's news.
    d.terms.get_mut(&helper).unwrap().started_by = Some(starter);
    d.set_status(helper, Status::Working, "test");
    d.set_status(helper, Status::Done, "test");
    assert!(d.terms[&helper].alert_due.is_none() && rang(&d, helper).is_none());
    // A stable question rings for helpers too.
    d.set_status(helper, Status::Blocked, "test");
    assert!(rang(&d, helper).is_none());
    overdue(&mut d, helper);
    d.settled_alerts();
    assert_eq!(rang(&d, helper), Some(Status::Blocked));
    close(&mut d, &[starter, helper]);
}

#[test]
fn a_clean_up_only_takes_worktrees_nothing_runs_in() {
    let (mut d, _rx) = daemon();
    let root = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-tidy-plan-{}"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let git = |dir: &std::path::Path, args: &[&str]| crate::proc::git(dir, args).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.email", "t@t"]);
    git(&repo, &["config", "user.name", "t"]);
    std::fs::write(repo.join("a.ts"), "a").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "one"]);
    let (idle, busy) = (root.join("idle"), root.join("busy"));
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "idle",
            idle.to_str().unwrap(),
        ],
    );
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "busy",
            busy.to_str().unwrap(),
        ],
    );
    // A session in one of them.
    let t = d.spawn(None, &busy, 80, 24).expect("a shell there");
    d.terms.get_mut(&t).unwrap().refresh_head();
    assert!(
        d.runs_in(&crate::gitfs::head(&busy).unwrap().top)
            && !d.runs_in(&crate::gitfs::head(&idle).unwrap().top)
    );
    d.terms.get_mut(&t).unwrap().head = None;
    assert!(
        d.runs_in(&busy),
        "a restored pane protects its checkout before git detection completes"
    );
    let (go, stay) = d.tidy_plan(vec![
        (idle.clone(), false),
        (busy.clone(), true),
        (repo.clone(), false),
        (root.join("nowhere"), false),
    ]);
    assert_eq!(go.len(), 1, "only the idle one goes: {go:?}");
    assert!(same_path(&go[0].0, &idle) && !go[0].1);
    let why: Vec<(&str, &str)> = stay.iter().map(|(n, w)| (n.as_str(), w.as_str())).collect();
    assert_eq!(
        why,
        [
            ("busy", "something is running in it"),
            ("repo", "it isn't a worktree"),
            ("nowhere", "it isn't a worktree")
        ],
        "the main folder is never one"
    );
    let sibling = d
        .spawn(None, &busy, 80, 24)
        .expect("a second pane in the checkout");
    d.terms.get_mut(&t).unwrap().refresh_head();
    d.terms.get_mut(&sibling).unwrap().head = None;
    d.made_worktrees.push(busy.clone());
    let tops = d.worktrees_of(&[t]);
    d.close_term(t);
    d.cleanup_worktrees(0, tops);
    assert!(
        busy.is_dir(),
        "closing one pane must not delete a worktree while its sibling is still starting"
    );
    assert!(
        d.terms.contains_key(&sibling),
        "the sibling pane remains alive"
    );
    close(&mut d, &[sibling]);
    let _ = std::fs::remove_dir_all(&root);
}

/// Handle daemon events until a worktree's automatic removal reports, and return how it went.
fn auto_removal(d: &mut Daemon, rx: &mut mpsc::Receiver<Ev>) -> Removal {
    let until = Instant::now() + Duration::from_secs(60);
    while Instant::now() < until {
        match rx.try_recv() {
            Ok(Ev::WorktreeAutoRemoved {
                client,
                path,
                result,
            }) => {
                d.handle(Ev::WorktreeAutoRemoved {
                    client,
                    path,
                    result: result.clone(),
                });
                return result;
            }
            Ok(ev) => d.handle(ev),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    panic!("no worktree removal reported");
}

/// A repo with `merged` (cut from main, nothing of its own) and `mine` (a commit only there),
/// each with a workspace the app made.
fn repo_with_two_made_worktrees(
    d: &mut Daemon,
    name: &str,
) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-{}-{}"),
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // A pane's shell reports its folder by its real name, and the temp folder may be spelled
    // another way (an 8.3 name such as `RUNNER~1` on Windows). Named as the shell names them,
    // the folders match a pane both before and after its first prompt.
    let root = clean_path(root.canonicalize().unwrap());
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let git = |dir: &std::path::Path, args: &[&str]| crate::proc::git(dir, args).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.email", "t@t"]);
    git(&repo, &["config", "user.name", "t"]);
    std::fs::write(repo.join("a.ts"), "a").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "one"]);
    let (merged, mine) = (root.join("merged"), root.join("mine"));
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "merged",
            merged.to_str().unwrap(),
        ],
    );
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "mine",
            mine.to_str().unwrap(),
        ],
    );
    std::fs::write(mine.join("b.ts"), "b").unwrap();
    git(&mine, &["add", "."]);
    git(&mine, &["commit", "-qm", "only here"]);
    for wt in [&merged, &mine] {
        d.command(
            0,
            Command::NewWorkspace {
                cwd: Some(wt.clone()),
                name: None,
                cmd: None,
            },
        )
        .unwrap();
        d.made_worktrees.push(wt.clone());
    }
    (root, merged, mine)
}

#[test]
fn closing_a_worktrees_last_pane_removes_it_only_when_nothing_lives_only_there() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (root, merged, mine) = repo_with_two_made_worktrees(&mut d, "close-last");
    let last_pane = |d: &Daemon, wt: &std::path::Path| {
        d.terms
            .values()
            .find(|t| same_path(&t.cwd, wt))
            .map(|t| t.id)
            .unwrap_or_else(|| {
                let panes: Vec<_> = d.terms.values().map(|t| (t.id, &t.cwd)).collect();
                panic!(
                    "no pane in {}; panes: {panes:?}; made worktrees: {:?}",
                    wt.display(),
                    d.made_worktrees
                )
            })
    };

    let t = last_pane(&d, &mine);
    d.command(0, Command::ClosePane { term: t }).unwrap();
    let asked = auto_removal(&mut d, &mut rx);
    assert!(
        matches!(&asked, Removal::Unmerged(why) if why.contains("not merged anywhere")),
        "a commit only there is offered, got: {asked:?}"
    );
    assert!(mine.is_dir(), "and its folder stands until you say");

    let dirty = root.join("dirty");
    crate::proc::git(
        &root.join("repo"),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "dirty",
            dirty.to_str().unwrap(),
        ],
    )
    .unwrap();
    std::fs::write(dirty.join("wip.ts"), "w").unwrap();
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(dirty.clone()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    d.made_worktrees.push(dirty.clone());
    let t = last_pane(&d, &dirty);
    d.command(0, Command::ClosePane { term: t }).unwrap();
    let kept = auto_removal(&mut d, &mut rx);
    assert!(
        matches!(&kept, Removal::Kept(why) if why.contains("uncommitted")) && dirty.is_dir(),
        "uncommitted work stays, got: {kept:?}"
    );

    let t = last_pane(&d, &merged);
    d.command(0, Command::ClosePane { term: t }).unwrap();
    let gone = auto_removal(&mut d, &mut rx);
    assert!(
        matches!(gone, Removal::Removed { .. }) && !merged.is_dir(),
        "a merged, clean one goes, got: {gone:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn nothing_removes_a_worktree_on_its_own_while_no_pane_is_closed() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (root, merged, mine) = repo_with_two_made_worktrees(&mut d, "idle");
    // A tab and a split beside the first pane: closing either leaves the worktree in use.
    let ws = d
        .workspaces
        .iter()
        .find(|w| same_path(&w.cwd, &merged))
        .map(|w| w.id)
        .unwrap();
    d.command(
        0,
        Command::NewTab {
            ws,
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let tab = d.workspaces.iter().find(|w| w.id == ws).unwrap().active_tab;
    d.command(0, Command::CloseTab { ws, tab }).unwrap();
    let first = d.workspaces.iter().find(|w| w.id == ws).unwrap().tabs[0].focus;
    d.command(
        0,
        Command::Split {
            term: first,
            dir: crate::layout::Dir::Right,
            cmd: None,
            cwd: None,
        },
    )
    .unwrap();
    let split = d.workspaces.iter().find(|w| w.id == ws).unwrap().tabs[0].focus;
    d.command(0, Command::ClosePane { term: split }).unwrap();
    // The checks that used to run on a timer only forget folders that are gone.
    d.forget_vanished_worktrees();
    std::thread::sleep(Duration::from_millis(1500));
    while let Ok(ev) = rx.try_recv() {
        assert!(
            !matches!(ev, Ev::WorktreeAutoRemoved { .. }),
            "no removal was asked for"
        );
        d.handle(ev);
    }
    assert!(merged.is_dir() && mine.is_dir(), "both folders stand");
    assert!(d.terms.contains_key(&first), "the first pane still runs");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_clean_up_says_what_went_and_what_stayed() {
    let note = super::tidied_note;
    assert_eq!(note(&[("a".into(), false)], &[]), "removed 1 worktree");
    assert_eq!(
        note(
            &[("a".into(), true), ("b".into(), false), ("c".into(), true)],
            &[]
        ),
        "removed 3 worktrees (2 merged branches went too)"
    );
    assert_eq!(
        note(
            &[("a".into(), true)],
            &[("rate".into(), "something is running in it".into())]
        ),
        "removed 1 worktree (1 merged branch went too); kept rate: something is running in it"
    );
    assert_eq!(note(&[], &[]), "nothing to clean up");
}

#[test]
fn a_finish_you_have_seen_is_not_announced_again() {
    let (mut d, _rx) = daemon();
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let t = d.workspaces[0].tabs[0].focus;
    let secret = d.terms[&t].token.clone();
    let say = |d: &mut Daemon, status, event: &str| {
        d.apply_hook(report(t, &secret, 0, status, event), Some(true), Vec::new())
    };
    say(&mut d, HookStatus::Working, "UserPromptSubmit");
    say(&mut d, HookStatus::Done, "Stop");
    assert_eq!(d.terms[&t].status, Status::Done);
    d.command(0, Command::MarkSeen { term: t }).unwrap();
    assert_eq!(d.terms[&t].status, Status::Idle, "you looked");
    // A minute on, Claude says it's still waiting: nothing new.
    say(&mut d, HookStatus::Done, "Notification:idle_prompt");
    assert_eq!(d.terms[&t].status, Status::Idle, "still seen");
    // The same notice with the finish itself missed still counts as the finish.
    say(&mut d, HookStatus::Working, "UserPromptSubmit");
    say(&mut d, HookStatus::Done, "Notification:idle_prompt");
    assert_eq!(d.terms[&t].status, Status::Done);
    close(&mut d, &[t]);
}

#[test]
fn explicit_idle_screen_evidence_recovers_missed_turn_ends_across_agents() {
    for (agent, title) in [
        ("claude", "✳ project"),
        ("codex", "project"),
        ("amp", concat!(env!("APP_COMMAND"), " - amp - project")),
        ("grok", "grok"),
    ] {
        let (mut d, _rx) = daemon();
        let t = pane(&mut d);
        {
            let p = d.terms.get_mut(&t).unwrap();
            p.agent = Some(agent.into());
            p.hooked = true;
            p.status = Status::Working;
            p.subagents
                .push(("child-1".into(), "general-purpose".into()));
            p.subagent_activity.push(SubagentActivity {
                id: "child-1".into(),
                name: "general-purpose".into(),
                parent: None,
                state: SubagentState::Working,
                task: Some("old task".into()),
                model: None,
                effort: None,
                started_at: None,
                updated_at: 0,
                latest_activity: None,
                result: None,
            });
            p.parser.process(format!("\x1b]0;{title}\x07").as_bytes());
            p.last_output = Instant::now() - Duration::from_secs(10);
        }

        d.update_statuses();

        assert_eq!(
            d.terms[&t].status,
            Status::Done,
            "{agent}: explicit idle screen evidence ends a missed turn"
        );
        assert!(
            d.terms[&t].subagents.is_empty(),
            "{agent}: clear stale child runners"
        );
        assert_eq!(
            d.terms[&t].subagent_activity[0].state,
            SubagentState::ConnectionLost,
            "{agent}: retain lifecycle history without showing the child as active"
        );
        close(&mut d, &[t]);
    }
}

/// Claude's screen while its turn-end hooks run: the title already says idle and the
/// spinner reads "(running Stop hooks…", which its working rule doesn't know.
const CLAUDE_RUNNING_STOP_HOOKS: &str = "\x1b]0;\u{2733} project\x07\x1b[2J\x1b[H\
● Pushed the branch.\r\n\r\n\
\u{273b} Symbioting\u{2026} (running Stop hooks\u{2026} 0/2 \u{b7} 1m 0s \u{b7} \u{2193} 1.1k tokens)\r\n\r\n\
\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\n\
\u{276f} \r\n\
\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\n\
  \u{23f5}\u{23f5} auto mode on (shift+tab to cycle) \u{b7} \u{2190} 1 agent";

#[test]
fn a_hooked_claude_still_animating_is_not_read_idle_from_its_prompt_box() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    {
        let p = d.terms.get_mut(&t).unwrap();
        p.agent = Some("claude".into());
        p.hooked = true;
        p.status = Status::Working;
        p.parser.process(CLAUDE_RUNNING_STOP_HOOKS.as_bytes());
        p.last_output = Instant::now();
    }
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "its spinner still redraws: wait for the hook"
    );
    assert!(d.terms[&t].alert_due.is_none(), "nothing rings yet");
    // No turn-end report ever came and the screen has gone still: the missed hook is recovered.
    d.terms.get_mut(&t).unwrap().last_output = Instant::now() - Duration::from_secs(10);
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done);
    close(&mut d, &[t]);
}

#[test]
fn an_unhooked_claude_still_animating_is_not_read_idle_from_its_prompt_box() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut seen = viewer(&mut d);
    {
        let p = d.terms.get_mut(&t).unwrap();
        p.agent = Some("claude".into());
        p.hooked = false;
        p.status = Status::Working;
        p.status_since = term::unix_now() - 60;
        p.parser.process(CLAUDE_RUNNING_STOP_HOOKS.as_bytes());
        p.last_output = Instant::now();
    }
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "its input box shows all turn long: a screen still redrawing is a turn still running"
    );
    assert!(!told_done(&mut seen), "nothing rings mid-turn");
    // With no hooks the screen is all there is: once it has gone still, the turn is over.
    d.terms.get_mut(&t).unwrap().last_output = Instant::now() - Duration::from_secs(10);
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done);
    close(&mut d, &[t]);
}

#[test]
fn a_permission_ask_settled_without_you_stops_needing_you() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    d.apply_hook(
        report(
            t,
            &secret,
            0,
            HookStatus::Blocked,
            "Notification:permission_prompt",
        ),
        None,
        Vec::new(),
    );
    assert_eq!(d.terms[&t].status, Status::Blocked);
    let long_ago =
        Instant::now() - super::status::UNASKED_QUESTION_GONE_AFTER - Duration::from_secs(1);
    // The question still on screen: it still needs you.
    d.terms
        .get_mut(&t)
        .unwrap()
        .parser
        .process(&b"\r\n".repeat(30));
    d.terms
        .get_mut(&t)
        .unwrap()
        .parser
        .process(b"\r\n Do you want to proceed?\r\n \xe2\x9d\xaf 1. Yes\r\n   2. No\r\n");
    d.terms.get_mut(&t).unwrap().blocked_at = Some(long_ago);
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Blocked, "asking on screen");
    // Auto mode allowed it and the screen moved on, with no hook to say so: over.
    d.terms
        .get_mut(&t)
        .unwrap()
        .parser
        .process(b"\x1b[2J\x1b[H* Waiting for 1 dynamic workflow to finish\r\n");
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done, "nothing asked on screen");
    close(&mut d, &[t]);
}

#[test]
fn status_reports_apply_in_the_order_they_came() {
    let (mut d, mut rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    d.message(
        0,
        report(t, &secret, 0, HookStatus::Working, "UserPromptSubmit"),
    );
    d.message(0, report(t, &secret, 0, HookStatus::Blocked, "PreToolUse"));
    d.message(0, report(t, &secret, 0, HookStatus::Done, "Stop"));
    assert_eq!(settle(&mut d, &mut rx, 3), 3);
    assert_eq!(
        d.terms[&t].status,
        Status::Done,
        "the last report wins: done (no one is looking)"
    );
    close(&mut d, &[t]);
}

#[test]
fn waking_a_sleeping_session_keeps_what_you_called_it() {
    let (mut d, _rx) = daemon();
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let old = d.workspaces[0].tabs[0].focus;
    d.command(
        0,
        Command::RenamePane {
            term: old,
            name: "The Planner".into(),
        },
    )
    .unwrap();
    let t = d.terms.get_mut(&old).unwrap();
    (t.name, t.model, t.asleep) = ("roadmap".into(), "opus".into(), true);
    t.kill_tree();
    let new = d.wake(old).expect("it wakes");
    assert!(
        new != old && !d.terms.contains_key(&old),
        "a new process in its place"
    );
    let t = &d.terms[&new];
    assert_eq!(
        (t.label.as_str(), t.name.as_str(), t.model.as_str()),
        ("The Planner", "roadmap", "opus"),
        "the same session to you"
    );
    assert_eq!(d.workspaces[0].tabs[0].focus, new, "in the same spot");
    close(&mut d, &[new]);
}

#[test]
fn reloading_an_agent_restarts_it_in_place_and_keeps_what_you_called_it() {
    let (mut d, _rx) = daemon();
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let old = d.workspaces[0].tabs[0].focus;
    assert!(
        d.command(0, Command::ReloadAgent { term: old }).is_err(),
        "a plain shell is not an agent, and must not be swapped for another shell"
    );
    d.command(
        0,
        Command::RenamePane {
            term: old,
            name: "The Planner".into(),
        },
    )
    .unwrap();
    let t = d.terms.get_mut(&old).unwrap();
    (t.agent, t.session, t.hooked) = (Some("claude".into()), Some("abc123".into()), false);

    d.command(0, Command::ReloadAgent { term: old }).unwrap();

    let new = d.workspaces[0].tabs[0].focus;
    assert!(new != old && !d.terms.contains_key(&old), "a new process in its place");
    assert_eq!(d.terms[&new].label, "The Planner", "the same session to you");
    assert!(!d.terms[&new].hooked, "it is hooked again only once its hooks report");
    close(&mut d, &[new]);
}

/// A connection acting from `term`, as its agent's `<app>` commands do.
fn from_pane(
    d: &mut Daemon,
    client: ClientId,
    term: TermId,
    token: &str,
) -> mpsc::Receiver<ServerMsg> {
    let (tx, rx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        client,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    d.handle(Ev::From(client, term, token.into()));
    rx
}

fn newest(d: &Daemon) -> TermId {
    *d.terms.keys().max().unwrap()
}

#[test]
fn a_session_knows_which_pane_started_it() {
    let (mut d, _rx) = daemon();
    let here = Some(std::env::temp_dir());
    d.command(
        0,
        Command::NewWorkspace {
            cwd: here.clone(),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let starter = d.workspaces[0].tabs[0].focus;
    assert_eq!(d.terms[&starter].started_by, None, "you started it");
    let token = d.terms[&starter].token.clone();
    let _c = from_pane(&mut d, 1, starter, &token);
    let mut made = vec![starter];
    d.handle(Ev::Msg(
        1,
        ClientMsg::Command(Command::NewWorkspace {
            cwd: here.clone(),
            name: None,
            cmd: None,
        }),
    ));
    made.push(newest(&d));
    let ws = d.workspaces[0].id;
    d.handle(Ev::Msg(
        1,
        ClientMsg::Command(Command::NewTab {
            ws,
            name: None,
            cmd: None,
        }),
    ));
    made.push(newest(&d));
    d.handle(Ev::Msg(
        1,
        ClientMsg::Command(Command::Split {
            term: starter,
            dir: crate::layout::Dir::Right,
            cmd: None,
            cwd: None,
        }),
    ));
    made.push(newest(&d));
    assert_eq!(made.len(), 4, "a workspace, a tab and a split");
    for t in &made[1..] {
        assert_eq!(
            d.terms[t].started_by,
            Some(starter),
            "started from the starter's pane"
        );
    }
    assert_eq!(
        d.snapshot().terms[&made[1]].started_by,
        Some(starter),
        "and clients are told"
    );
    close(&mut d, &made);
}

#[test]
fn only_the_pane_itself_can_be_named_as_the_starter() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let _c = from_pane(&mut d, 1, t, "not-the-secret");
    d.handle(Ev::Msg(
        1,
        ClientMsg::Command(Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        }),
    ));
    let new = newest(&d);
    assert!(new != t, "it may still start a session");
    assert_eq!(
        d.terms[&new].started_by, None,
        "but not in that pane's name"
    );
    close(&mut d, &[t, new]);
}

#[test]
fn a_worktree_start_keeps_its_starter_after_the_caller_has_gone() {
    let (mut d, _rx) = daemon();
    let starter = pane(&mut d);
    let dir = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-started-by-{}"),
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    // The worktree was asked for from the starter's pane; that connection closed while git ran.
    d.pending_ops += 1;
    d.handle(Ev::WorktreeCreated {
        client: 9,
        branch: "helper".into(),
        cmd: None,
        split: None,
        by: Some(starter),
        set_up: false,
        result: Ok((dir.clone(), String::new())),
    });
    let helper = newest(&d);
    assert_eq!(d.terms[&helper].started_by, Some(starter));
    // The next session you start yourself isn't the starter's.
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let yours = newest(&d);
    assert_eq!(d.terms[&yours].started_by, None);
    close(&mut d, &[starter, helper, yours]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_replaced_session_keeps_its_place_among_its_helpers() {
    let (mut d, _rx) = daemon();
    let here = Some(std::env::temp_dir());
    for _ in 0..3 {
        d.command(
            0,
            Command::NewWorkspace {
                cwd: here.clone(),
                name: None,
                cmd: None,
            },
        )
        .unwrap();
    }
    let [lead, starter, helper] = [0, 1, 2].map(|i| d.workspaces[i].tabs[0].focus);
    d.terms.get_mut(&starter).unwrap().started_by = Some(lead);
    d.terms.get_mut(&helper).unwrap().started_by = Some(starter);
    // Woken from sleep: a new process, the same session.
    let t = d.terms.get_mut(&starter).unwrap();
    t.asleep = true;
    t.kill_tree();
    let woken = d.wake(starter).expect("it wakes");
    assert_eq!(
        d.terms[&woken].started_by,
        Some(lead),
        "still under the one that started it"
    );
    assert_eq!(
        d.terms[&helper].started_by,
        Some(woken),
        "and its helper is still its own"
    );
    // Moved into a worktree: the same again.
    d.relocate(woken, &std::env::temp_dir());
    let moved = d.workspaces[1].tabs[0].focus;
    assert!(moved != woken, "a new process in the new folder");
    assert_eq!(d.terms[&moved].started_by, Some(lead));
    assert_eq!(d.terms[&helper].started_by, Some(moved));
    close(&mut d, &[lead, moved, helper]);
}

#[test]
fn detaching_and_closing_prune_tabs_and_workspaces() {
    let (mut d, _rx) = daemon();
    let dir = std::env::temp_dir();
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(dir.clone()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    assert_eq!(d.workspaces.len(), 1);
    let first = d.workspaces[0].tabs[0].focus;
    d.command(
        0,
        Command::Split {
            term: first,
            dir: crate::layout::Dir::Right,
            cmd: None,
            cwd: None,
        },
    )
    .unwrap();
    let leaves = d.workspaces[0].tabs[0].layout.leaves();
    assert_eq!(leaves.len(), 2);
    let second = leaves.into_iter().find(|t| *t != first).unwrap();
    // Out of the tab, still running.
    d.detach(first);
    assert!(d.terms.contains_key(&first), "detach keeps the terminal");
    assert_eq!(d.workspaces[0].tabs[0].layout.leaves(), vec![second]);
    assert_eq!(
        d.workspaces[0].tabs[0].focus, second,
        "focus moves to what's left"
    );
    // The last pane goes: the tab and the workspace go with it.
    d.remove_term(second, RemovalReason::Exited);
    assert!(d.workspaces.is_empty(), "an emptied workspace disappears");
    assert!(!d.terms.contains_key(&second));
    close(&mut d, &[first]);
}

#[test]
fn swapped_panes_trade_places_and_come_back_swapped_after_a_restart() {
    let (mut d, _rx) = daemon();
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let left = d.workspaces[0].tabs[0].focus;
    d.command(
        0,
        Command::Split {
            term: left,
            dir: crate::layout::Dir::Right,
            cmd: None,
            cwd: None,
        },
    )
    .unwrap();
    let right = newest(&d);
    d.command(
        0,
        Command::RenamePane {
            term: left,
            name: "was left".into(),
        },
    )
    .unwrap();
    d.command(
        0,
        Command::RenamePane {
            term: right,
            name: "was right".into(),
        },
    )
    .unwrap();

    d.command(0, Command::SwapPanes { a: left, b: right })
        .unwrap();
    assert_eq!(d.workspaces[0].tabs[0].layout.leaves(), vec![right, left]);

    let (mut d2, _rx2) = daemon();
    d2.restore(d.saved());
    let back = d2.workspaces[0].tabs[0].layout.leaves();
    let labels: Vec<&str> = back.iter().map(|t| d2.terms[t].label.as_str()).collect();
    assert_eq!(
        labels,
        ["was right", "was left"],
        "the swapped order is what's saved"
    );
    close(&mut d2, &back);
    close(&mut d, &[left, right]);
}

#[test]
fn panes_in_different_tabs_cannot_swap() {
    let (mut d, _rx) = daemon();
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let first = d.workspaces[0].tabs[0].focus;
    let ws = d.workspaces[0].id;
    d.command(
        0,
        Command::NewTab {
            ws,
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let other = newest(&d);
    let before: Vec<_> = d.workspaces[0]
        .tabs
        .iter()
        .map(|t| t.layout.clone())
        .collect();
    let err = d
        .command(0, Command::SwapPanes { a: first, b: other })
        .unwrap_err();
    assert!(err.to_string().contains("same tab"), "{err}");
    assert!(
        d.command(0, Command::SwapPanes { a: first, b: 9999 })
            .is_err(),
        "a pane that isn't there"
    );
    assert_eq!(
        d.workspaces[0]
            .tabs
            .iter()
            .map(|t| t.layout.clone())
            .collect::<Vec<_>>(),
        before
    );
    close(&mut d, &[first, other]);
}

#[test]
fn a_spare_fits_only_its_repo_and_its_agent() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let repo = std::env::temp_dir().join(concat!(env!("APP_COMMAND"), "-spare-repo"));
    d.cfg.worktree.prewarm = "claude".into();
    d.spare = Some((repo.clone(), repo.join("wt"), t));
    assert_eq!(
        d.spare_fits(&repo, "claude").as_deref(),
        Some(""),
        "the agent alone"
    );
    let with_task = format!("claude {}", d.cfg.quote_for_shell("fix the flaky test"));
    assert_eq!(
        d.spare_fits(&repo, &with_task).as_deref(),
        Some("fix the flaky test"),
        "the agent and a task"
    );
    assert_eq!(d.spare_fits(&repo, "codex"), None, "another agent");
    assert_eq!(
        d.spare_fits(&repo, "claude --model opus"),
        None,
        "other options"
    );
    assert_eq!(
        d.spare_fits(&std::env::temp_dir().join("other"), "claude"),
        None,
        "another repo"
    );
    d.cfg.worktree.prewarm.clear();
    assert_eq!(d.spare_fits(&repo, "claude"), None, "prewarming off");
    d.spare = None;
    close(&mut d, &[t]);
}

#[test]
fn helpers_and_tabs_come_back_from_a_restart_under_their_session() {
    let (mut d, _rx) = daemon();
    let dir = std::env::temp_dir();
    let child_activity = SubagentActivity {
        id: "child-session".into(),
        name: "reviewer".into(),
        parent: Some("parent-session".into()),
        state: SubagentState::Completed,
        task: Some("review the patch".into()),
        model: Some("gpt-5.6".into()),
        effort: None,
        started_at: Some(100),
        updated_at: 200,
        latest_activity: Some("review returned".into()),
        result: Some("one finding".into()),
    };
    let active_activity = SubagentActivity {
        id: "still-running".into(),
        name: "builder".into(),
        parent: Some("parent-session".into()),
        state: SubagentState::Working,
        task: Some("finish implementation".into()),
        model: None,
        effort: None,
        started_at: Some(150),
        updated_at: 180,
        latest_activity: Some("working".into()),
        result: None,
    };
    // Saved ids: 70 is the starter, 71 its helper, 72 a helper whose starter (99) is gone.
    // The helpers are saved ahead of the starter, so they come back before it does.
    let ws = |id: TermId, by: Option<TermId>| persist::SavedWs {
        name: String::new(),
        cwd: dir.clone(),
        worktree: false,
        color: None,
        group: None,
        tabs: vec![persist::SavedTab {
            name: String::new(),
            layout: Node::Leaf(id),
            focus: id,
            panes: [(
                id,
                persist::SavedPane {
                    cwd: Some(dir.clone()),
                    started_by: by,
                    tab_of: by.filter(|_| id == 71),
                    subagent_activity: if id == 71 {
                        vec![child_activity.clone(), active_activity.clone()]
                    } else {
                        Vec::new()
                    },
                    ..Default::default()
                },
            )]
            .into(),
        }],
        active_tab: 0,
    };
    d.restore(persist::Saved {
        workspaces: vec![ws(71, Some(70)), ws(72, Some(99)), ws(70, None)],
        active: 0,
        made_worktrees: Vec::new(),
        conversations: Vec::new(),
        pending_starts: Vec::new(),
        conversation_operations: Vec::new(),
        turn_continues: Default::default(),
        turns_in_flight: Vec::new(),
    });
    let [helper, orphan, starter] = [0, 1, 2].map(|i| d.workspaces[i].tabs[0].focus);
    assert!(starter != 70, "the starter has this run's id");
    assert_eq!(
        d.terms[&helper].started_by,
        Some(starter),
        "its helper found it again"
    );
    assert_eq!(
        d.terms[&helper].subagent_activity[0], child_activity,
        "completed child and result survive restoration"
    );
    assert_eq!(d.terms[&helper].subagent_activity[1].id, active_activity.id);
    assert_eq!(
        d.terms[&helper].subagent_activity[1].state,
        SubagentState::ConnectionLost,
        "active child is unconfirmed after a daemon restart"
    );
    assert_eq!(
        d.terms[&orphan].started_by, None,
        "a helper whose starter didn't come back is on its own"
    );
    assert_eq!(d.terms[&starter].started_by, None);
    // A pane in another session's tab comes back in that session's tab, by its new id.
    assert_eq!(
        d.terms[&helper].tab_of,
        Some(starter),
        "71 was a tab of 70's: it is again"
    );
    assert_eq!(
        (d.terms[&orphan].tab_of, d.terms[&starter].tab_of),
        (None, None),
        "the rest are their own"
    );
    // And it's saved that way again.
    let saved = d.saved();
    assert_eq!(
        saved.workspaces[0].tabs[0].panes[&helper].started_by,
        Some(starter)
    );
    assert_eq!(
        saved.workspaces[0].tabs[0].panes[&helper].tab_of,
        Some(starter),
        "and saved that way again"
    );
    assert_eq!(
        saved.workspaces[0].tabs[0].panes[&helper]
            .subagent_activity
            .len(),
        2
    );
    // A session saved before helpers were tracked still loads.
    let old: persist::SavedPane =
        serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":null,"session":null}"#)
            .expect("an older pane");
    assert_eq!(old.started_by, None);
    assert_eq!(
        old.tab_of, None,
        "a pane saved before tabs were kept is its own"
    );
    assert!(
        old.subagent_activity.is_empty(),
        "panes saved before child activity still load"
    );
    // Fleet and commander state an older version saved is read and ignored.
    let fleet: persist::SavedPane = serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":"claude","session":null,"commander":true,"fleet":{"goal":"ship it","size":3},"started_by":4}"#).expect("a pane from a fleet");
    assert_eq!(
        (fleet.agent.as_deref(), fleet.started_by),
        (Some("claude"), Some(4))
    );
    let old_save = r#"{"workspaces":[],"active":0,"made_worktrees":[],"fleets":[{"commander":1}]}"#;
    assert!(
        serde_json::from_str::<persist::Saved>(old_save).is_ok(),
        "a save with fleet tables still loads"
    );
    close(&mut d, &[helper, orphan, starter]);
}

#[test]
fn restore_gives_panes_new_ids_and_keeps_their_layout() {
    let (mut d, _rx) = daemon();
    let dir = std::env::temp_dir();
    let mut layout = Node::Leaf(70);
    layout.split(70, crate::layout::Dir::Right, 71);
    layout.split(71, crate::layout::Dir::Down, 72);
    let mut panes = std::collections::BTreeMap::new();
    panes.insert(
        70,
        persist::SavedPane {
            cwd: Some(dir.clone()),
            name: "left one".into(),
            label: "Map Gen".into(),
            ..Default::default()
        },
    );
    panes.insert(
        71,
        persist::SavedPane {
            cwd: Some(dir.clone()),
            model: "opus".into(),
            ..Default::default()
        },
    );
    panes.insert(
        72,
        persist::SavedPane {
            cwd: Some(dir.clone()),
            ..Default::default()
        },
    );
    let saved = persist::Saved {
        workspaces: vec![persist::SavedWs {
            name: "api".into(),
            cwd: dir.clone(),
            worktree: false,
            color: None,
            group: None,
            tabs: vec![persist::SavedTab {
                name: "main".into(),
                layout,
                focus: 71,
                panes,
            }],
            active_tab: 0,
        }],
        active: 0,
        made_worktrees: Vec::new(),
        conversations: Vec::new(),
        pending_starts: Vec::new(),
        conversation_operations: Vec::new(),
        turn_continues: Default::default(),
        turns_in_flight: Vec::new(),
    };
    d.restore(saved);
    assert_eq!(d.workspaces.len(), 1);
    let tab = &d.workspaces[0].tabs[0];
    let leaves = tab.layout.leaves();
    assert_eq!(leaves.len(), 3, "all three panes came back");
    assert!(
        leaves
            .iter()
            .all(|t| d.terms.contains_key(t) && ![70, 71, 72].contains(t)),
        "with this run's ids"
    );
    let (left, right, below) = (leaves[0], leaves[1], leaves[2]);
    assert_eq!(tab.focus, right, "focus follows the pane it was on");
    let rects = tab.layout.rects(ratatui::layout::Rect::new(0, 0, 120, 40));
    let rect = |id| rects.iter().find(|(term, _)| *term == id).unwrap().1;
    assert_eq!(
        rect(below).x,
        rect(right).x,
        "restoration retains the right-side column"
    );
    assert!(
        rect(below).y > rect(right).y,
        "restoration retains the down split"
    );
    assert_eq!(d.terms[&left].first_prompt, "left one");
    assert_eq!(
        d.terms[&left].label, "Map Gen",
        "the name you gave it comes back"
    );
    assert_eq!(
        d.saved().workspaces[0].tabs[0].panes[&left].label,
        "Map Gen",
        "and is saved again"
    );
    assert_eq!(d.terms[&right].model, "opus");
    close(&mut d, &leaves);
}

#[test]
fn worktree_names_that_look_like_options_are_refused() {
    let dir = std::env::temp_dir();
    for bad in ["", "-x", "a b", "--force"] {
        assert!(
            git::create_worktree(&dir, bad, None, "{repo_parent}/{repo}-worktrees/{branch}")
                .is_err(),
            "branch {bad:?}"
        );
    }
}

#[test]
fn a_repository_with_no_commits_says_so_instead_of_showing_a_git_error() {
    let tmp = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-no-commits-{}"),
        std::process::id()
    ));
    let repo = tmp.join("fresh");
    std::fs::create_dir_all(&repo).unwrap();
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["init", "-q"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let error = git::create_worktree(
        &repo,
        "warm-finch",
        None,
        "{repo_parent}/{repo}-worktrees/{branch}",
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("no commits yet"), "{error:#}");
    assert!(
        !tmp.join("fresh-worktrees").exists(),
        "nothing is created for a worktree that cannot be made"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn a_made_up_branch_skips_names_already_in_use() {
    let names = crate::gitfs::WT_NAMES;
    let seed = 5;
    assert_eq!(worktrees::made_up_branch(&[], seed), names[seed]);

    let taken = vec![names[seed].to_string(), names[seed + 1].to_string()];
    assert_eq!(worktrees::made_up_branch(&taken, seed), names[seed + 2]);

    let all: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    let numbered = worktrees::made_up_branch(&all, seed);
    assert!(
        numbered.starts_with(&format!("{}-", names[seed])) && !all.contains(&numbered),
        "every name taken: {numbered}"
    );
}

#[test]
fn an_agent_typed_into_a_shell_gets_its_own_worktree() {
    let tmp = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-agent-wt-{}"),
        std::process::id()
    ));
    let repo = tmp.join("shop");
    std::fs::create_dir_all(repo.join("web")).unwrap();
    let git = |args: &[&str]| {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(args)
                .output()
                .unwrap()
                .status
                .success(),
            "git {args:?}"
        )
    };
    git(&["init", "-q"]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "first",
    ]);
    std::fs::write(repo.join("web").join("keep"), "").unwrap();
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "-m",
        "web",
    ]);
    let made =
        worktrees::agent_worktree(&repo.join("web"), "{repo_parent}/{repo}-worktrees/{branch}", false)
            .unwrap();
    assert!(
        made.ends_with("web") && made.is_dir(),
        "in the same subfolder of the new worktree: {}",
        made.display()
    );
    let head = crate::gitfs::head(&made).unwrap();
    assert!(
        head.linked && crate::gitfs::WT_NAMES.contains(&head.branch.as_str()),
        "a linked worktree on a made-up branch: {head:?}"
    );
    let again =
        worktrees::agent_worktree(&repo, "{repo_parent}/{repo}-worktrees/{branch}", false).unwrap();
    let other = crate::gitfs::head(&again).unwrap().branch;
    assert_ne!(other, head.branch, "the next one gets its own branch");
    // Closing them: a merged branch goes with its worktree, one with work of its own stays.
    let wt = |dir: &std::path::Path, args: &[&str]| {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .output()
                .unwrap()
                .status
                .success(),
            "git {args:?}"
        )
    };
    wt(
        &again,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "work",
        ],
    );
    git::remove_worktree(&crate::gitfs::head(&made).unwrap().top, false, false).unwrap();
    git::remove_worktree(&again, false, false).unwrap();
    assert!(
        git::delete_if_merged(&repo, &head.branch),
        "nothing on it that main doesn't have: deleted"
    );
    assert!(
        !git::delete_if_merged(&repo, &other),
        "a commit main doesn't have: kept"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn a_new_worktree_starts_from_the_default_branch_only_when_asked() {
    let tmp = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-base-wt-{}"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    let repo = tmp.join("shop");
    std::fs::create_dir_all(&repo).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["commit", "-q", "--allow-empty", "-m", "main work"]);
    let main_tip = git(&["rev-parse", "main"]);
    // The main checkout is on a feature branch with a commit main does not have.
    git(&["checkout", "-q", "-b", "feature"]);
    git(&["commit", "-q", "--allow-empty", "-m", "feature work"]);
    let template = "{repo_parent}/{repo}-worktrees/{branch}";
    assert_eq!(git::default_base(&repo).as_deref(), Some("main"));

    let (from_head, _) =
        worktrees::create_trusted_worktree(&repo, "from-head", None, template, false).unwrap();
    let (from_main, _) =
        worktrees::create_trusted_worktree(&repo, "from-main", None, template, true).unwrap();
    let tip = |dir: &std::path::Path| {
        String::from_utf8_lossy(
            &std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string()
    };
    assert_eq!(tip(&from_head), git(&["rev-parse", "feature"]));
    assert_eq!(tip(&from_main), main_tip);
    // A base the caller names is never overruled.
    let (named, _) =
        worktrees::create_trusted_worktree(&repo, "named", Some("feature"), template, true)
            .unwrap();
    assert_eq!(tip(&named), git(&["rev-parse", "feature"]));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn only_a_pane_shell_asks_for_an_agent_worktree() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        1,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    d.handle(Ev::Msg(
        1,
        ClientMsg::Command(Command::AgentWorktree {
            dir: std::env::current_dir().unwrap(),
        }),
    ));
    assert!(
        matches!(crx.try_recv(), Ok(ServerMsg::Reply(Reply::Text(p))) if p.is_empty()),
        "not from a pane: start where it is"
    );
    d.terms.get_mut(&t).unwrap().agent = Some("claude".into());
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        2,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let token = d.terms[&t].token.clone();
    d.handle(Ev::From(2, t, token));
    d.handle(Ev::Msg(
        2,
        ClientMsg::Command(Command::AgentWorktree {
            dir: std::env::current_dir().unwrap(),
        }),
    ));
    assert!(
        matches!(crx.try_recv(), Ok(ServerMsg::Reply(Reply::Text(p))) if p.is_empty()),
        "an agent's own agents work where it does"
    );
    // Another pane's id with the wrong secret is not that pane: nothing is made for it.
    d.terms.get_mut(&t).unwrap().agent = None;
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        3,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    d.handle(Ev::From(3, t, "not-the-secret".into()));
    d.handle(Ev::Msg(
        3,
        ClientMsg::Command(Command::AgentWorktree {
            dir: std::env::current_dir().unwrap(),
        }),
    ));
    assert!(
        matches!(crx.try_recv(), Ok(ServerMsg::Reply(Reply::Text(p))) if p.is_empty()),
        "a forged pane id gets no worktree"
    );
    close(&mut d, &[t]);
}

#[test]
fn an_agent_stopped_by_a_limit_is_told_to_continue_when_it_resets() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let now = term::unix_now();
    d.limits.insert(
        "claude".into(),
        vec![
            Limit {
                name: "5h".into(),
                used: 100.0,
                resets_at: now + 600,
            },
            Limit {
                name: "week".into(),
                used: 40.0,
                resets_at: now + 90_000,
            },
        ],
    );
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.status) = (Some("claude".into()), Status::Idle);
        p.parser.process(&b"\r\n".repeat(30));
        p.parser.process(
            b"\r\n  \xe2\x8e\xbf  Claude usage limit reached. Your limit will reset at 3pm\r\n> ",
        );
    }
    d.auto_continue();
    assert_eq!(
        d.terms[&t].resume_at,
        Some(now + 600 + 60),
        "a minute after the spent limit resets"
    );
    assert!(
        d.snapshot().terms[&t].resume_at.is_some(),
        "and the window says so"
    );
    d.auto_continue();
    assert!(d.terms[&t].pending_input.is_none(), "not before");
    d.terms.get_mut(&t).unwrap().resume_at = Some(now - 1);
    d.auto_continue();
    assert_eq!(
        d.terms[&t]
            .pending_input
            .as_ref()
            .map(|(b, _)| b.as_slice()),
        Some(&b"continue\r"[..]),
        "then it's told to go on"
    );
    d.terms.get_mut(&t).unwrap().pending_input = None;
    d.auto_continue();
    assert!(
        d.terms[&t].resume_at.is_none(),
        "the old line still on screen doesn't start another wait"
    );
    // Off in Settings: nothing.
    d.cfg.auto_continue = false;
    d.terms.get_mut(&t).unwrap().continued = None;
    d.auto_continue();
    assert!(d.terms[&t].resume_at.is_none());
    close(&mut d, &[t]);
}

#[test]
fn claude_reports_what_its_session_used() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let token = d.terms[&t].token.clone();
    d.terms.get_mut(&t).unwrap().agent = Some("claude".into());
    let usage = |cost: f64| Usage {
        context: Some(30.0),
        cost: Some(cost),
        ..Default::default()
    };
    let limits = vec![Limit {
        name: "5h".into(),
        used: 23.0,
        resets_at: term::unix_now() + 100,
    }];
    d.handle(Ev::Msg(
        1,
        ClientMsg::Usage {
            term: t,
            token: "wrong".into(),
            usage: usage(9.0),
            limits: Vec::new(),
        },
    ));
    assert_eq!(
        d.terms[&t].usage,
        Usage::default(),
        "only with the pane's secret"
    );
    d.handle(Ev::Msg(
        1,
        ClientMsg::Usage {
            term: t,
            token: token.clone(),
            usage: usage(1.0),
            limits: limits.clone(),
        },
    ));
    d.handle(Ev::Msg(
        1,
        ClientMsg::Usage {
            term: t,
            token: token.clone(),
            usage: usage(1.5),
            limits,
        },
    ));
    // /clear: a new session, counting from zero again.
    d.handle(Ev::Msg(
        1,
        ClientMsg::Usage {
            term: t,
            token,
            usage: usage(0.25),
            limits: Vec::new(),
        },
    ));
    let snap = d.snapshot();
    assert_eq!(snap.terms[&t].usage.context, Some(30.0));
    assert!(
        (snap.spent_today - 1.75).abs() < 1e-9,
        "1.50 then 0.25 after the clear: {}",
        snap.spent_today
    );
    assert_eq!(snap.limits[0].0, "claude");
    close(&mut d, &[t]);
}

#[test]
fn the_usage_store_keeps_what_claude_reported_and_drops_cost_once_plan_limits_show() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let token = d.terms[&t].token.clone();
    {
        let pane = d.terms.get_mut(&t).unwrap();
        pane.agent = Some("claude".into());
        pane.session = Some("claude-session-1".into());
    }
    let report = |tokens_in, cost, limits| ClientMsg::Usage {
        term: t,
        token: token.clone(),
        usage: Usage {
            cost: Some(cost),
            tokens_in: Some(tokens_in),
            tokens_out: Some(tokens_in / 10),
            ..Default::default()
        },
        limits,
    };
    d.handle(Ev::Msg(1, report(1000, 0.5, Vec::new())));
    d.handle(Ev::Msg(1, report(1500, 0.75, Vec::new())));
    let week = d.usage_report(crate::usage_report::UsagePeriod::Week);
    assert_eq!(week.totals.tokens_in, Some(1500));
    assert_eq!(week.totals.tokens_out, Some(150));
    assert_eq!(week.totals.cost, Some(0.75));
    assert_eq!(week.sessions[0].session, "claude-session-1");
    assert_eq!(week.sessions[0].term, Some(t));

    // Plan limits arrive: a subscription, so the cost is not kept from then on.
    let limits = vec![Limit {
        name: "5h".into(),
        used: 10.0,
        resets_at: term::unix_now() + 100,
    }];
    d.handle(Ev::Msg(1, report(2000, 9.0, limits)));
    let week = d.usage_report(crate::usage_report::UsagePeriod::Week);
    assert_eq!(week.totals.tokens_in, Some(2000));
    assert_eq!(week.totals.cost, Some(0.75), "no cost added after the limits showed");

    // The old "today" report of the terminal app is untouched.
    assert!(d.spent_today() > 0.0);
    close(&mut d, &[t]);
}

/// Tests that write or delete saved pane output share one folder per test run: one at a time.
static SAVED_OUTPUT: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn a_message_held_for_a_starting_agent_survives_the_server_restarting() {
    let (mut d, _rx) = daemon();
    d.cfg.restore.enabled = true;
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let t = *d.terms.keys().next().unwrap();
    d.terms.get_mut(&t).unwrap().held_message = Some(term::HeldMessage::new("fix the login page"));
    let saved = d.saved();
    let (mut d2, _rx2) = daemon();
    d2.cfg.restore.enabled = true;
    d2.restore(saved);
    let new = *d2.terms.keys().next().unwrap();
    assert_eq!(
        d2.terms[&new]
            .held_message
            .as_ref()
            .map(|held| held.text.as_str()),
        Some("fix the login page")
    );
    let old: persist::SavedPane =
        serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":"claude","session":null}"#)
            .expect("a pane saved before messages were held");
    assert_eq!(old.held_message, None);
    close(&mut d, &[t]);
    close(&mut d2, &[new]);
}

#[test]
fn a_panes_history_survives_the_server_restarting() {
    let _one_at_a_time = SAVED_OUTPUT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (mut d, _rx) = daemon();
    d.cfg.restore.enabled = true;
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let t = *d.terms.keys().next().unwrap();
    d.terms
        .get_mut(&t)
        .unwrap()
        .ring_push_for_test(b"line from before the restart\r\n");
    let saved = d.saved();
    d.save_outputs(true);
    // A new server: the pane comes back, with what it showed above what it shows now.
    let (mut d2, _rx2) = daemon();
    d2.cfg.restore.enabled = true;
    d2.restore(saved);
    let new = *d2.terms.keys().next().unwrap();
    let replay = String::from_utf8_lossy(&d2.terms[&new].replay()).into_owned();
    assert!(
        replay.contains("line from before the restart")
            && replay.contains(concat!(env!("APP_COMMAND"), " restarted here")),
        "{replay}"
    );
    let mut p = vt100::Parser::new(24, 80, 1000);
    p.process(&d2.terms[&new].replay());
    p.screen_mut().set_scrollback(usize::MAX);
    assert!(
        p.screen().scrollback() >= 24,
        "the old screen went up into history: {}",
        p.screen().scrollback()
    );
    assert!(persist::take_output(t).is_none(), "taken once");
    close(&mut d, &[t]);
    close(&mut d2, &[new]);
}

#[test]
fn closing_the_last_pane_keeps_an_open_window() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let (tx, _crx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        1,
        tx,
        true,
        crate::protocol::Surface::Terminal,
        true,
    ));
    d.close_term(t);
    assert!(
        !d.should_exit(),
        concat!("a window is still showing ", env!("APP_COMMAND"))
    );
    d.handle(Ev::Disconnected(1));
    assert!(
        d.should_exit(),
        "everything closed and nobody's looking: exit"
    );
}

#[test]
fn a_turn_waiting_on_background_agents_stays_working() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let hook = |status: HookStatus, event: &str, subagent: Option<Subagent>| {
        let mut m = report(t, "", 0, status, event);
        if let ClientMsg::Hook { subagent: s, .. } = &mut m {
            *s = subagent;
        }
        m
    };
    let agent = |start: bool| {
        Some(Subagent {
            id: "a1".into(),
            kind: "general-purpose".into(),
            start,
            reported_at: 0,
            task: None,
            model: None,
            effort: None,
            activity: None,
        })
    };
    d.apply_hook(
        hook(HookStatus::Working, "UserPromptSubmit", None),
        Some(true),
        Vec::new(),
    );
    d.apply_hook(
        hook(HookStatus::Working, "SubagentStart", agent(true)),
        Some(true),
        Vec::new(),
    );
    d.apply_hook(
        hook(HookStatus::Working, "SubagentStart", agent(true)),
        Some(true),
        Vec::new(),
    );
    assert_eq!(
        d.terms[&t].subagent_activity.len(),
        1,
        "repeated start events do not duplicate the record"
    );
    assert_eq!(
        d.terms[&t].subagent_activity[0].state,
        SubagentState::Working
    );
    // Claude's own turn ends ("waiting for 1 background agent"): still working.
    d.apply_hook(hook(HookStatus::Done, "Stop", None), Some(true), Vec::new());
    assert_eq!(d.terms[&t].status, Status::Working);
    // Long past the old three minutes, the agent is still at it (its own tool use): still
    // working, still listed once.
    let ago = |mins: u64| {
        Instant::now()
            .checked_sub(Duration::from_secs(mins * 60))
            .unwrap()
    };
    d.terms.get_mut(&t).unwrap().done_held = Some(ago(20));
    d.apply_hook(
        hook(HookStatus::Same, "PreToolUse", agent(true)),
        Some(true),
        Vec::new(),
    );
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "held while the agent runs"
    );
    assert_eq!(d.terms[&t].subagents.len(), 1, "listed once");
    // Nothing from it for over an hour: it died without saying; the turn is done.
    let term = d.terms.get_mut(&t).unwrap();
    (term.done_held, term.subagent_seen) = (Some(ago(70)), Some(ago(70)));
    d.update_statuses();
    assert_ne!(d.terms[&t].status, Status::Working, "not held forever");
    assert_eq!(
        d.terms[&t].subagent_activity[0].state,
        SubagentState::ConnectionLost,
        "quiet child is retained as unconfirmed, not completed"
    );
    let terms: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn a_helper_whose_stop_never_came_clears_when_the_agent_says_none_is_running() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let helper = Subagent {
        id: "a1".into(),
        kind: "general-purpose".into(),
        start: true,
        reported_at: 0,
        task: None,
        model: None,
        effort: None,
        activity: None,
    };
    let stop = |running: &[&str]| {
        let mut m = report(t, "", 0, HookStatus::Done, "Stop");
        if let ClientMsg::Hook {
            background,
            background_agents,
            background_reported,
            ..
        } = &mut m
        {
            *background = running.iter().map(|id| id.to_string()).collect();
            *background_agents = background.clone();
            *background_reported = true;
        }
        m
    };
    d.apply_hook(
        report(t, "", 0, HookStatus::Working, "UserPromptSubmit"),
        Some(true),
        Vec::new(),
    );
    let mut start = report(t, "", 0, HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { subagent, .. } = &mut start {
        *subagent = Some(helper);
    }
    d.apply_hook(start, Some(true), Vec::new());

    // The agent says one helper still runs: the turn is paused on it, and it stays listed.
    d.apply_hook(stop(&["a1"]), Some(true), Vec::new());
    assert_eq!(d.terms[&t].status, Status::Working);
    assert_eq!(d.terms[&t].subagents.len(), 1);

    // Its stop never arrives. The next turn end lists nothing running: the agent's word wins.
    d.apply_hook(stop(&[]), Some(true), Vec::new());
    assert_eq!(d.terms[&t].status, Status::Done, "not held on a helper that is gone");
    assert!(d.terms[&t].subagents.is_empty());
    assert_eq!(
        d.terms[&t].subagent_activity[0].state,
        SubagentState::ConnectionLost,
        "kept as unconfirmed, not as completed"
    );
    let terms: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn a_helper_recorded_as_stopped_no_longer_holds_its_session_working() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut start = report(t, "", 0, HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { subagent, .. } = &mut start {
        *subagent = Some(Subagent {
            id: "a1".into(),
            kind: "general-purpose".into(),
            start: true,
            reported_at: 0,
            task: None,
            model: None,
            effort: None,
            activity: None,
        });
    }
    d.apply_hook(start, Some(true), Vec::new());
    assert_eq!(d.terms[&t].subagents.len(), 1);
    // Some path marked it lost without dropping it from the count.
    for record in &mut d.terms.get_mut(&t).expect("pane").subagent_activity {
        record.state = SubagentState::ConnectionLost;
    }
    d.update_statuses();
    assert!(d.terms[&t].subagents.is_empty(), "a stopped helper is not counted as running");
    let terms: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

/// A report from `agent`'s hooks; with `helper_starts`, it is news of its helper `a1`.
fn report_with_helper(
    t: TermId,
    agent: &str,
    status: HookStatus,
    event: &str,
    helper_starts: Option<bool>,
) -> ClientMsg {
    let mut msg = report(t, "", 0, status, event);
    if let ClientMsg::Hook { agent: a, subagent, .. } = &mut msg {
        *a = agent.into();
        *subagent = helper_starts.map(|start| Subagent {
            id: "a1".into(),
            kind: "general-purpose".into(),
            start,
            reported_at: 0,
            task: None,
            model: None,
            effort: None,
            activity: None,
        });
    }
    msg
}

/// The turn-end report; Claude's lists the background agents still running, Codex's lists nothing.
fn turn_end_report(t: TermId, agent: &str, running: Option<&[&str]>) -> ClientMsg {
    let mut msg = report_with_helper(t, agent, HookStatus::Done, "Stop", None);
    if let (
        ClientMsg::Hook {
            background,
            background_agents,
            background_reported,
            ..
        },
        Some(running),
    ) = (&mut msg, running)
    {
        *background = running.iter().map(|id| id.to_string()).collect();
        *background_agents = background.clone();
        *background_reported = true;
    }
    msg
}

/// The agent's screen has shown its idle prompt (`title`) and nothing else for ten seconds.
fn rests_at_its_idle_prompt(d: &mut Daemon, t: TermId, title: &str) {
    let p = d.terms.get_mut(&t).expect("pane");
    p.parser.process(format!("\x1b]0;{title}\x07").as_bytes());
    p.last_output = Instant::now() - Duration::from_secs(10);
    d.update_statuses();
}

fn times_told_done(seen: &mut mpsc::Receiver<ServerMsg>) -> usize {
    heard(seen)
        .iter()
        .filter(|m| matches!(m, ServerMsg::Attention { status: Status::Done, .. }))
        .count()
}

/// A turn that starts a background helper and ends while it runs, as `agent`'s hooks report it.
fn a_turn_that_ends_while_its_helper_runs_stays_working(agent: &str, idle_title: &str) {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut seen = viewer(&mut d);
    let lists_background = agent == "claude";
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, agent, HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, report_with_helper(t, agent, HookStatus::Working, "SubagentStart", Some(true)));
    say(&mut d, turn_end_report(t, agent, lists_background.then_some(&["a1"])));
    rests_at_its_idle_prompt(&mut d, t, idle_title);
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "{agent}: its turn ended, its helper did not"
    );
    // The helper is minutes into one command, then uses another tool.
    say(&mut d, report_with_helper(t, agent, HookStatus::Same, "PostToolUse", Some(true)));
    rests_at_its_idle_prompt(&mut d, t, idle_title);
    assert_eq!(d.terms[&t].status, Status::Working, "{agent}: still working");
    assert_eq!(d.terms[&t].subagents.len(), 1, "{agent}: the helper is still listed");
    assert_eq!(times_told_done(&mut seen), 0, "{agent}: nothing finished yet");

    say(&mut d, report_with_helper(t, agent, HookStatus::Same, "SubagentStop", Some(false)));
    assert_eq!(d.terms[&t].status, Status::Done, "{agent}: its last helper stopped");
    // Claude wakes to read what its helper found and stops again, with nothing left running.
    if lists_background {
        say(&mut d, report_with_helper(t, agent, HookStatus::Working, "PreToolUse", None));
        assert_eq!(d.terms[&t].status, Status::Working, "{agent}: reading the result");
        say(&mut d, turn_end_report(t, agent, Some(&[])));
    }
    rests_at_its_idle_prompt(&mut d, t, idle_title);
    assert_eq!(d.terms[&t].status, Status::Done, "{agent}: and it stays done");
    assert_eq!(times_told_done(&mut seen), 1, "{agent}: one finish, said once");
    close(&mut d, &[t]);
}

#[test]
fn a_claude_turn_that_ends_while_its_background_agent_runs_stays_working() {
    a_turn_that_ends_while_its_helper_runs_stays_working("claude", "✳ project");
}

#[test]
fn a_codex_turn_that_ends_while_its_child_agent_runs_stays_working() {
    a_turn_that_ends_while_its_helper_runs_stays_working("codex", "project");
}

#[test]
fn a_helper_started_after_the_turn_ended_keeps_its_session_working() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, turn_end_report(t, "claude", Some(&[])));
    assert_eq!(d.terms[&t].status, Status::Done);
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "SubagentStart", Some(true)));
    rests_at_its_idle_prompt(&mut d, t, "✳ project");
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "an idle prompt is what Claude shows while a background agent runs"
    );
    // No turn end follows a helper that began outside a turn: its stop is the end.
    say(&mut d, report_with_helper(t, "claude", HookStatus::Same, "SubagentStop", Some(false)));
    rests_at_its_idle_prompt(&mut d, t, "✳ project");
    assert_eq!(d.terms[&t].status, Status::Done);
    close(&mut d, &[t]);
}

#[test]
fn a_turn_cancelled_while_its_helper_ran_is_freed_by_the_idle_prompt() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "SubagentStart", Some(true)));
    // Esc: neither the helper's stop nor the turn's end is ever reported.
    rests_at_its_idle_prompt(&mut d, t, "✳ project");
    assert_ne!(
        d.terms[&t].status,
        Status::Working,
        "no turn end was held, so the idle prompt is a missed one"
    );
    assert!(d.terms[&t].subagents.is_empty(), "the helper is no longer listed");
    assert_eq!(
        d.terms[&t].subagent_activity[0].state,
        SubagentState::ConnectionLost,
        "kept as unconfirmed, not as completed"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_second_helper_started_after_the_turn_ended_does_not_release_the_hold() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "SubagentStart", Some(true)));
    say(&mut d, turn_end_report(t, "claude", Some(&["a1"])));
    let mut second = report_with_helper(t, "claude", HookStatus::Working, "SubagentStart", Some(true));
    if let ClientMsg::Hook { subagent: Some(helper), .. } = &mut second {
        helper.id = "a2".into();
    }
    say(&mut d, second);
    rests_at_its_idle_prompt(&mut d, t, "✳ project");
    assert_eq!(d.terms[&t].status, Status::Working);
    assert_eq!(d.terms[&t].subagents.len(), 2);
    close(&mut d, &[t]);
}

#[test]
fn a_codex_helper_its_files_show_finished_stops_holding_when_its_stop_was_lost() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, "codex", HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, report_with_helper(t, "codex", HookStatus::Working, "SubagentStart", Some(true)));
    say(&mut d, turn_end_report(t, "codex", None));
    d.terms.get_mut(&t).expect("pane").session = Some("root".into());
    rests_at_its_idle_prompt(&mut d, t, "project");
    assert_eq!(d.terms[&t].status, Status::Working, "control: held while the helper runs");

    // Its SubagentStop never arrives; Codex's own file for the helper says it completed.
    let finished = SubagentActivity {
        state: SubagentState::Completed,
        updated_at: term::unix_now() + 5,
        ..d.terms[&t].subagent_activity[0].clone()
    };
    d.codex_usage(
        vec![usage::CodexPaneFacts {
            term: t,
            session: Some("root".into()),
            usage: None,
            subagents: Vec::new(),
            subagent_activity: vec![finished],
        }],
        None,
    );
    rests_at_its_idle_prompt(&mut d, t, "project");
    assert_eq!(d.terms[&t].status, Status::Done);
    assert!(d.terms[&t].subagents.is_empty());
    assert_eq!(d.terms[&t].subagent_activity[0].state, SubagentState::Completed);
    close(&mut d, &[t]);
}

#[test]
fn a_helper_that_dies_without_reporting_frees_its_session_after_the_hour() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut seen = viewer(&mut d);
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "SubagentStart", Some(true)));
    say(&mut d, turn_end_report(t, "claude", Some(&["a1"])));
    // It has been working for over an hour, so its finish is a real one.
    d.terms.get_mut(&t).expect("pane").status_since = term::unix_now() - 2 * 60 * 60;
    let within = Instant::now() - super::status::SUBAGENTS_QUIET_AT_MOST + Duration::from_secs(60);
    let p = d.terms.get_mut(&t).expect("pane");
    (p.done_held, p.subagent_seen) = (Some(within), Some(within));
    rests_at_its_idle_prompt(&mut d, t, "✳ project");
    assert_eq!(d.terms[&t].status, Status::Working, "held for the whole hour");

    let past = Instant::now() - super::status::SUBAGENTS_QUIET_AT_MOST - Duration::from_secs(60);
    let p = d.terms.get_mut(&t).expect("pane");
    (p.done_held, p.subagent_seen) = (Some(past), Some(past));
    rests_at_its_idle_prompt(&mut d, t, "✳ project");
    assert_eq!(d.terms[&t].status, Status::Done, "not held forever");
    assert_eq!(times_told_done(&mut seen), 1, "and its finish is said once");
    assert!(d.terms[&t].subagents.is_empty());
    assert_eq!(
        d.terms[&t].subagent_activity[0].state,
        SubagentState::ConnectionLost,
        "kept as unconfirmed, not as completed"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_helper_that_dies_off_screen_still_has_its_finish_said_after_the_hour() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut seen = viewer(&mut d);
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, report_with_helper(t, "claude", HookStatus::Working, "SubagentStart", Some(true)));
    say(&mut d, turn_end_report(t, "claude", Some(&["a1"])));
    let past = Instant::now() - super::status::SUBAGENTS_QUIET_AT_MOST - Duration::from_secs(60);
    let p = d.terms.get_mut(&t).expect("pane");
    p.status_since = term::unix_now() - 2 * 60 * 60;
    (p.done_held, p.subagent_seen) = (Some(past), Some(past));
    // No idle prompt on screen: the hour alone ends it.
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done);
    assert_eq!(times_told_done(&mut seen), 1, "its finish is said once");
    close(&mut d, &[t]);
}

#[test]
fn an_interrupted_codex_turn_held_on_a_helper_is_not_finished_an_hour_later() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, "codex", HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, report_with_helper(t, "codex", HookStatus::Working, "SubagentStart", Some(true)));
    say(&mut d, turn_end_report(t, "codex", None));
    say(&mut d, report_with_helper(t, "codex", HookStatus::Idle, "Interrupt", None));
    assert_eq!(d.terms[&t].status, Status::Idle);
    let past = Instant::now() - super::status::SUBAGENTS_QUIET_AT_MOST - Duration::from_secs(60);
    let p = d.terms.get_mut(&t).expect("pane");
    p.subagent_seen = Some(past);
    if let Some(held) = p.done_held.as_mut() {
        *held = past;
    }
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Idle,
        "the interrupt ended the turn; nothing is left held to finish it later"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_codex_file_finished_in_the_same_second_as_the_helper_started_still_counts() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let say = |d: &mut Daemon, msg| d.apply_hook(msg, Some(true), Vec::new());
    say(&mut d, report_with_helper(t, "codex", HookStatus::Working, "UserPromptSubmit", None));
    say(&mut d, report_with_helper(t, "codex", HookStatus::Working, "SubagentStart", Some(true)));
    say(&mut d, turn_end_report(t, "codex", None));
    d.terms.get_mut(&t).expect("pane").session = Some("root".into());
    let started = d.terms[&t].subagent_activity[0].clone();
    let finished = SubagentActivity {
        state: SubagentState::Completed,
        ..started.clone()
    };
    d.codex_usage(
        vec![usage::CodexPaneFacts {
            term: t,
            session: Some("root".into()),
            usage: None,
            subagents: Vec::new(),
            subagent_activity: vec![finished],
        }],
        None,
    );
    assert_eq!(d.terms[&t].subagent_activity[0].state, SubagentState::Completed);
    // A working record stamped in the same second does not undo a finish.
    d.codex_usage(
        vec![usage::CodexPaneFacts {
            term: t,
            session: Some("root".into()),
            usage: None,
            subagents: Vec::new(),
            subagent_activity: vec![started],
        }],
        None,
    );
    assert_eq!(d.terms[&t].subagent_activity[0].state, SubagentState::Completed);
    rests_at_its_idle_prompt(&mut d, t, "project");
    assert_eq!(d.terms[&t].status, Status::Done);
    close(&mut d, &[t]);
}

#[test]
fn a_progress_bar_that_goes_off_while_helpers_run_is_not_a_cancel() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let ago = |secs: u64| {
        Instant::now()
            .checked_sub(Duration::from_secs(secs))
            .unwrap()
    };
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Working);
        p.progress_off = Some(ago(5));
    }
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Idle,
        "control: with nothing running, a bar that went off is a cancel"
    );
    {
        let p = d.terms.get_mut(&t).unwrap();
        p.status = Status::Working;
        p.progress_off = Some(ago(5));
        p.subagents.push(("a1".into(), "general-purpose".into()));
        p.subagent_seen = Some(Instant::now());
    }
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "the agent is waiting on its helper, not cancelled"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_stop_for_an_unknown_helper_leaves_the_running_ones_alone() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let hook = |status: HookStatus, event: &str, id: &str, start: bool| {
        let mut m = report(t, "", 0, status, event);
        if let ClientMsg::Hook { subagent, .. } = &mut m {
            *subagent = Some(Subagent {
                id: id.into(),
                kind: "general-purpose".into(),
                start,
                reported_at: 0,
                task: None,
                model: None,
                effort: None,
                activity: None,
            });
        }
        m
    };
    d.apply_hook(
        hook(HookStatus::Working, "SubagentStart", "a1", true),
        Some(true),
        Vec::new(),
    );
    d.apply_hook(
        hook(HookStatus::Working, "SubagentStart", "a2", true),
        Some(true),
        Vec::new(),
    );
    // One that began before the app was listening stops: nobody listed has finished.
    d.apply_hook(
        hook(HookStatus::Same, "SubagentStop", "never-seen", false),
        Some(true),
        Vec::new(),
    );
    assert_eq!(d.terms[&t].subagents.len(), 2, "both still listed");
    d.apply_hook(
        hook(HookStatus::Same, "SubagentStop", "a1", false),
        Some(true),
        Vec::new(),
    );
    assert_eq!(
        d.terms[&t].subagents,
        vec![("a2".to_string(), "general-purpose".to_string())],
        "only the one that stopped goes"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_session_starting_again_while_helpers_run_keeps_the_agent_working() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let plain = |status: HookStatus, event: &str| report(t, "", 0, status, event);
    d.apply_hook(
        plain(HookStatus::Working, "UserPromptSubmit"),
        Some(true),
        Vec::new(),
    );
    let mut start = plain(HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { subagent, .. } = &mut start {
        *subagent = Some(Subagent {
            id: "a1".into(),
            kind: "general-purpose".into(),
            start: true,
            reported_at: 0,
            task: None,
            model: None,
            effort: None,
            activity: None,
        });
    }
    d.apply_hook(start, Some(true), Vec::new());
    // Compaction starts the session again in the middle of the turn.
    d.apply_hook(
        plain(HookStatus::Idle, "SessionStart"),
        Some(true),
        Vec::new(),
    );
    assert_eq!(d.terms[&t].status, Status::Working, "still working");
    assert_eq!(d.terms[&t].subagents.len(), 1, "its helper is still listed");
    close(&mut d, &[t]);
}

/// A helper's own hooks run as separate programs, each stamping its report with the second it ran
/// in, so what it did last and that it finished can reach the daemon in either order.
fn helper_report(t: TermId, event: &str, start: bool, reported_at: u64) -> ClientMsg {
    let mut msg = report(t, "", 0, HookStatus::Same, event);
    if let ClientMsg::Hook { subagent, .. } = &mut msg {
        *subagent = Some(Subagent {
            id: "child-1".into(),
            kind: "reviewer".into(),
            start,
            reported_at,
            task: None,
            model: None,
            effort: None,
            activity: None,
        });
    }
    msg
}

#[test]
fn a_helpers_last_news_arriving_after_it_finished_does_not_bring_it_back() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.apply_hook(helper_report(t, "SubagentStart", true, 100), Some(true), Vec::new());
    d.apply_hook(helper_report(t, "SubagentStop", false, 200), Some(true), Vec::new());
    // Its last tool's report was slower than its stop, and stamped a second later.
    d.apply_hook(helper_report(t, "PostToolUse", true, 201), Some(true), Vec::new());

    assert_eq!(d.terms[&t].subagent_activity[0].state, SubagentState::Completed);
    assert!(d.terms[&t].subagents.is_empty(), "a finished helper is not listed as running again");
    close(&mut d, &[t]);
}

#[test]
fn a_helper_that_says_it_finished_is_finished_whatever_its_news_was_stamped() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.apply_hook(helper_report(t, "SubagentStart", true, 100), Some(true), Vec::new());
    // Its last tool's report ran a second after its stop did, and arrived first.
    d.apply_hook(helper_report(t, "PostToolUse", true, 200), Some(true), Vec::new());
    d.apply_hook(helper_report(t, "SubagentStop", false, 199), Some(true), Vec::new());

    assert_eq!(d.terms[&t].subagent_activity[0].state, SubagentState::Completed);
    assert!(d.terms[&t].subagents.is_empty());
    close(&mut d, &[t]);
}

#[test]
fn a_helper_can_be_started_again_under_the_same_name_after_it_finished() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.apply_hook(helper_report(t, "SubagentStart", true, 100), Some(true), Vec::new());
    d.apply_hook(helper_report(t, "SubagentStop", false, 200), Some(true), Vec::new());
    d.apply_hook(helper_report(t, "SubagentStart", true, 300), Some(true), Vec::new());

    assert_eq!(d.terms[&t].subagent_activity[0].state, SubagentState::Working);
    assert_eq!(d.terms[&t].subagents.len(), 1);
    close(&mut d, &[t]);
}

#[test]
fn delayed_child_start_does_not_undo_its_completion() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.terms.get_mut(&t).unwrap().session = Some("parent-thread".into());
    let child = |start, reported_at| {
        Some(Subagent {
            id: "child-1".into(),
            kind: "reviewer".into(),
            start,
            reported_at,
            task: Some("review the change".into()),
            model: Some("claude-opus".into()),
            effort: None,
            activity: Some(
                if start {
                    "SubagentStart"
                } else {
                    "SubagentStop"
                }
                .into(),
            ),
        })
    };
    let mut start = report(t, "", 0, HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { subagent, .. } = &mut start {
        *subagent = child(true, 100);
    }
    d.apply_hook(start, Some(true), Vec::new());
    let mut stop = report(t, "", 0, HookStatus::Same, "SubagentStop");
    if let ClientMsg::Hook { subagent, .. } = &mut stop {
        *subagent = child(false, 200);
    }
    d.apply_hook(stop, Some(true), Vec::new());
    let mut delayed_start = report(t, "", 0, HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { subagent, .. } = &mut delayed_start {
        *subagent = child(true, 150);
    }
    d.apply_hook(delayed_start, Some(true), Vec::new());

    let activity = &d.terms[&t].subagent_activity[0];
    assert_eq!(activity.state, SubagentState::Completed);
    assert_eq!(activity.updated_at, 200);
    assert!(
        d.terms[&t].subagents.is_empty(),
        "a delayed start cannot recreate a live child"
    );
    assert_eq!(activity.parent.as_deref(), Some("parent-thread"));
    assert_eq!(activity.task.as_deref(), Some("review the change"));
    assert_eq!(activity.model.as_deref(), Some("claude-opus"));
    close(&mut d, &[t]);
}

#[test]
fn an_agent_asks_you_and_gets_your_answer() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    // The agent's `<app> ask-human` is a client waiting for the reply.
    let (tx, mut asker) = mpsc::channel(8);
    d.handle(Ev::Connected(
        5,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let ask = Command::AskHuman {
        term: t,
        text: "Deploy to staging?".into(),
        options: vec!["Yes".into(), "No".into()],
    };
    assert!(!d.command(5, ask).unwrap(), "the reply waits for you");
    assert_eq!(d.terms[&t].status, Status::Blocked, "it needs you");
    let q = d.snapshot().questions;
    assert_eq!((q.len(), q[0].text.as_str()), (1, "Deploy to staging?"));
    // You answer No: the asker gets it, the pane is back at work.
    d.command(
        0,
        Command::AnswerHuman {
            id: q[0].id,
            choice: 1,
        },
    )
    .unwrap();
    assert!(matches!(asker.try_recv(), Ok(ServerMsg::Reply(Reply::Text(a))) if a == "No"));
    assert_eq!(d.terms[&t].status, Status::Working);
    assert!(d.snapshot().questions.is_empty());
    let terms: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn a_pane_does_what_its_grants_allow() {
    let (mut d, _rx) = daemon();
    let (a, b) = (pane(&mut d), pane(&mut d));
    // An agent in pane a, with a's secret.
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(
        7,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    let err =
        |out: &mut mpsc::Receiver<ServerMsg>| matches!(out.try_recv(), Ok(ServerMsg::Error(_)));
    // Reading another pane: allowed by default.
    d.message(7, ClientMsg::Query(Query::Read { term: b }));
    assert!(
        matches!(out.try_recv(), Ok(ServerMsg::Reply(Reply::Text(_)))),
        "read is a default grant"
    );
    // Answering b's prompt: not without respond.
    d.terms.get_mut(&b).unwrap().status = Status::Blocked;
    d.message(
        7,
        ClientMsg::Input {
            term: b,
            data: b"1".to_vec(),
        },
    );
    assert!(err(&mut out), "respond isn't a default grant");
    // Closing b: not without admin; and it can't grant itself more.
    d.message(7, ClientMsg::Command(Command::ClosePane { term: b }));
    assert!(
        err(&mut out) && d.terms.contains_key(&b),
        "admin isn't a default grant"
    );
    d.message(
        7,
        ClientMsg::Command(Command::Grant {
            term: a,
            grants: Some(vec!["admin".into()]),
        }),
    );
    assert!(err(&mut out), "grants are yours to give");
    // You give it admin: now it may.
    d.message(
        0,
        ClientMsg::Command(Command::Grant {
            term: a,
            grants: Some(vec!["admin".into()]),
        }),
    );
    d.message(7, ClientMsg::Command(Command::ClosePane { term: b }));
    assert!(!d.terms.contains_key(&b), "closed once allowed");
    let terms: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn a_popup_floats_in_no_workspace_and_goes_when_done() {
    let (mut d, mut rx) = daemon();
    let cmd = if cfg!(windows) { "cmd /c exit" } else { "true" };
    d.command(
        0,
        Command::Popup {
            cmd: cmd.into(),
            cwd: Some(std::env::temp_dir()),
        },
    )
    .unwrap();
    let (&id, t) = d.terms.iter().next().expect("the popup's pane");
    assert!(
        t.popup && d.workspaces.is_empty(),
        "floating, in no workspace"
    );
    assert!(d.snapshot().terms[&id].popup);
    // Its command exits: the pane goes. (A ceiling, not a wait: on a machine with every core
    // busy, starting and ending even this takes a while.)
    let until = Instant::now() + Duration::from_secs(60);
    while d.terms.contains_key(&id) && Instant::now() < until {
        if let Ok(ev) = rx.try_recv() {
            d.handle(ev);
        } else {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    assert!(!d.terms.contains_key(&id), "gone when its command is done");
}

#[test]
fn a_hook_that_names_a_transcript_not_written_yet_still_gives_the_pane_its_path() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    let not_yet = std::env::temp_dir().join("session-before-its-first-prompt.jsonl");
    let _ = std::fs::remove_file(&not_yet);
    let mut start = report(t, &secret, 0, HookStatus::Idle, "SessionStart");
    if let ClientMsg::Hook { transcript, .. } = &mut start {
        *transcript = Some(not_yet.clone());
    }

    d.apply_hook(start, Some(true), Vec::new());

    // Its chat can then exist from the start, instead of only after a first prompt.
    assert_eq!(d.terms[&t].transcript.as_deref(), Some(not_yet.as_path()));
}

#[test]
fn an_agent_stays_in_the_folder_it_started_in() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    let base = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-pin-{}"),
        std::process::id()
    ));
    let (home, other) = (base.join("home"), base.join("other"));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    let at = |cwd: &std::path::Path, status: HookStatus, event: &str| match report(
        t, &secret, 0, status, event,
    ) {
        ClientMsg::Hook {
            term,
            agent,
            status,
            session,
            prompt,
            said,
            subagent,
            event,
            pid,
            token,
            transcript,
            model,
            name,
            background,
            background_agents,
            ..
        } => ClientMsg::Hook {
            term,
            agent,
            status,
            session,
            cwd: Some(cwd.to_path_buf()),
            prompt,
            said,
            subagent,
            event,
            pid,
            token,
            transcript,
            model,
            name,
            background,
            background_agents,
            background_reported: false,
            ask: None,
            call: None,
        },
        _ => unreachable!(),
    };
    // Its first report places it.
    d.apply_hook(
        at(&home, HookStatus::Working, "UserPromptSubmit"),
        Some(true),
        Vec::new(),
    );
    assert_eq!(d.terms[&t].cwd, home);
    // Working in another folder doesn't move it.
    d.apply_hook(
        at(&other, HookStatus::Working, "PreToolUse"),
        Some(true),
        Vec::new(),
    );
    assert_eq!(
        d.terms[&t].cwd, home,
        "its row stays in the group it started in"
    );
    close(&mut d, &[t]);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn an_agent_read_from_its_screen_isnt_done_while_its_screen_still_moves() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    {
        // Codex with its notify pointed elsewhere: no hooks, only its screen.
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.hooked, p.status) = (Some("codex".into()), false, Status::Working);
        p.parser
            .process(b"\xe2\x80\xa2 Working (3s \xe2\x80\xa2 esc to interrupt)\r\n");
        // A long block printed under it pushes the working line out of the rows looked at.
        for i in 0..40 {
            p.parser.process(format!("+ added line {i}\r\n").as_bytes());
        }
        p.last_output = Instant::now();
    }
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "still printing: not done, and no alert"
    );
    // The screen goes still: now it's done.
    d.terms.get_mut(&t).unwrap().last_output = Instant::now() - Duration::from_secs(10);
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done);
    close(&mut d, &[t]);
}

#[test]
fn codex_shows_working_from_its_screen_after_a_turn() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    {
        let p = d.terms.get_mut(&t).unwrap();
        // Codex finished a turn (its notify hook), ten seconds ago.
        (p.agent, p.hooked, p.status) = (Some("codex".into()), true, Status::Done);
        p.status_since = term::unix_now() - 10;
        // You typed the next prompt, and it's working on it.
        p.last_input = Instant::now();
        // At the bottom of the screen, where it is in Codex.
        p.parser.process(&b"\r\n".repeat(30));
        p.parser.process(b"\xe2\x80\xa2 Working (3s \xe2\x80\xa2 esc to interrupt)\r\n\r\n> Ask Codex to do anything");
    }
    d.update_statuses();
    assert_eq!(
        d.terms[&t].status,
        Status::Working,
        "seen on its screen: its hooks never say a turn started"
    );
    // Without anything typed since the turn ended, the old line on screen doesn't count.
    {
        let p = d.terms.get_mut(&t).unwrap();
        p.status = Status::Done;
        p.status_since = term::unix_now();
        p.last_input = Instant::now() - Duration::from_secs(5);
    }
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done);
    close(&mut d, &[t]);
}

#[test]
fn repeated_status_observations_refresh_evidence_without_restarting_the_turn() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let t = d.terms.get_mut(&id).unwrap();
    t.status = Status::Working;
    t.status_since = 7;
    t.status_why = "hook: working".into();
    t.last_screen = Some(("old observation".into(), 1));
    d.set_status(id, Status::Working, "screen: activity row is present");
    let t = &d.terms[&id];
    let result = (
        t.status_since,
        t.status_why.clone(),
        t.last_screen.clone(),
        t.alert_due.is_none(),
    );
    close(&mut d, &[id]);
    assert_eq!(
        result.0, 7,
        "evidence freshness is separate from turn duration"
    );
    assert_eq!(
        result.1, "hook: working",
        "the original status source stays authoritative"
    );
    assert!(
        result
            .2
            .is_some_and(|(detail, at)| detail == "activity row is present" && at > 1)
    );
    assert!(
        result.3,
        "the same status does not announce another transition"
    );
}

#[test]
fn codex_detects_work_with_background_terminals_and_wrapped_footers() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for hooked in [false, true] {
        for cols in [160, 32] {
            for suffix in [
                "",
                " · 1 background terminal running · /ps to view · /stop to close",
            ] {
                let t = d.terms.get_mut(&id).unwrap();
                t.resize(cols, 24);
                t.agent = Some("codex".into());
                t.hooked = hooked;
                t.status = Status::Done;
                t.status_since = term::unix_now() - 10;
                t.last_input = Instant::now();
                t.parser.process(b"\x1b[2J\x1b[H");
                t.parser.process(&b"\r\n".repeat(30));
                t.parser.process(format!("• Working (2m 40s • esc to interrupt){suffix}\r\n> Ask Codex to do anything").as_bytes());
                d.update_statuses();
                observed.push((hooked, cols, suffix, d.terms[&id].status));
            }
        }
    }
    close(&mut d, &[id]);
    assert!(
        observed
            .iter()
            .all(|(_, _, _, status)| *status == Status::Working),
        "native Codex activity must survive metadata and soft wraps: {observed:?}"
    );
}

#[test]
fn codex_can_start_its_next_turn_without_waiting_for_the_wall_clock() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    d.hook(
        id,
        "codex".into(),
        HookStatus::Done,
        "agent-turn-complete",
        &[],
    );
    let t = d.terms.get_mut(&id).unwrap();
    // A backwards clock makes the same zero-second interval as a prompt sent immediately.
    t.status_since = u64::MAX;
    t.last_input = Instant::now();
    t.parser.process(&b"\r\n".repeat(30));
    t.parser
        .process("• Working (1s • esc to interrupt)\r\n> Ask Codex to do anything".as_bytes());
    d.update_statuses();
    let mut observed = vec![d.terms[&id].status];
    d.hook(
        id,
        "codex".into(),
        HookStatus::Done,
        "agent-turn-complete",
        &[],
    );
    d.update_statuses();
    observed.push(d.terms[&id].status);
    d.set_status(id, Status::Idle, "you're looking at it");
    d.update_statuses();
    observed.push(d.terms[&id].status);
    // A slow start is still a new turn; there is no thirty-second deadline on its footer.
    let t = d.terms.get_mut(&id).unwrap();
    t.last_turn_end = Instant::now() - Duration::from_secs(60);
    t.last_input = Instant::now() - Duration::from_secs(45);
    d.update_statuses();
    observed.push(d.terms[&id].status);
    close(&mut d, &[id]);
    assert_eq!(
        observed,
        [Status::Working, Status::Done, Status::Idle, Status::Working],
        "new input starts work immediately or after a delay; a stale footer stays finished, even after being seen"
    );
}

#[test]
fn codex_live_footer_restarts_work_without_new_keyboard_input() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let t = d.terms.get_mut(&id).unwrap();
    t.agent = Some("codex".into());
    t.output(format!("\x1b[2J\x1b[H{}Working (3m 11s • esc to interrupt) · 3 background terminals running · /ps to view · /stop to close\r\n› Ask Codex to do anything", "\r\n".repeat(30)).as_bytes());
    d.hook(
        id,
        "codex".into(),
        HookStatus::Done,
        "agent-turn-complete",
        &[],
    );
    d.update_statuses();
    let mut observed = vec![d.terms[&id].status];
    let t = d.terms.get_mut(&id).unwrap();
    t.output(format!("\x1b[2J\x1b[H{}Working (3m 12s • esc to interrupt) · 3 background terminals running · /ps to view · /stop to close\r\n› Ask Codex to do anything", "\r\n".repeat(30)).as_bytes());
    d.update_statuses();
    observed.push(d.terms[&id].status);
    close(&mut d, &[id]);
    assert_eq!(
        observed,
        [Status::Done, Status::Working],
        "an unchanged old footer stays finished; an advancing live footer is working even without new keys"
    );
}

#[test]
fn native_hooks_cannot_keep_an_exited_agent_working_or_hide_its_replacement() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let found = |agent: Option<&str>| scan::Found {
        term: id,
        process: "powershell".into(),
        agent: agent.map(str::to_string),
        cwd: None,
        agent_cwd: None,
        mem: 0,
        cpu: 0.0,
        remote: None,
    };
    d.hook(id, "pi".into(), HookStatus::Working, "pi.agent_start", &[]);
    d.handle(Ev::Scan(vec![found(None)]));
    let startup = d.terms[&id].status;
    d.handle(Ev::Scan(vec![found(Some("pi"))]));
    // One scan that misses the agent is a missed read, not an exit.
    d.handle(Ev::Scan(vec![found(None)]));
    let missed_once = (d.terms[&id].agent.clone(), d.terms[&id].hooked);
    for _ in 1..AGENT_MISSING_SCANS {
        d.handle(Ev::Scan(vec![found(None)]));
    }
    let exited = (
        d.terms[&id].status,
        d.terms[&id].agent.clone(),
        d.terms[&id].hooked,
    );
    d.hook(id, "codex".into(), HookStatus::Working, "native", &[]);
    d.terms.get_mut(&id).unwrap().last_hook_at = Some(Instant::now() - Duration::from_secs(4));
    d.handle(Ev::Scan(vec![found(Some("opencode"))]));
    let replacement = (
        d.terms[&id].agent.clone(),
        d.terms[&id].hooked,
        d.terms[&id].status,
    );
    close(&mut d, &[id]);
    assert_eq!(
        startup,
        Status::Working,
        "initial process scan can lag a native hook"
    );
    assert_eq!(missed_once, (Some("pi".into()), true));
    assert_eq!(exited, (Status::None, None, false));
    assert_eq!(replacement, (Some("opencode".into()), false, Status::Idle));
}

#[test]
fn codex_status_ignores_activity_in_a_finished_answer() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for hooked in [false, true] {
        let t = d.terms.get_mut(&id).unwrap();
        t.agent = Some("codex".into());
        t.hooked = hooked;
        t.status = if hooked {
            Status::Done
        } else {
            Status::Working
        };
        t.status_since = term::unix_now() - 10;
        t.last_input = Instant::now();
        t.last_output = Instant::now() - Duration::from_secs(10);
        t.parser.process(b"\x1b[2J\x1b[H");
        t.parser.process(&b"\r\n".repeat(30));
        t.parser.process(b"I checked the live daemon: it matches Codex's esc to interrupt spinner.\r\nWorked for 50s\r\n> Ask Codex to do anything");
        d.update_statuses();
        observed.push(d.terms[&id].status);
        let t = d.terms.get_mut(&id).unwrap();
        t.status_since = term::unix_now() - 10;
        t.last_input = Instant::now();
        t.parser.process(b"\x1b[2J\x1b[H");
        t.parser.process(&b"\r\n".repeat(30));
        t.parser.process(
            "• Working (8m 05s • esc to interrupt)\r\n> Ask Codex to do anything".as_bytes(),
        );
        d.update_statuses();
        observed.push(d.terms[&id].status);
    }
    close(&mut d, &[id]);
    assert_eq!(
        observed,
        [Status::Done, Status::Working, Status::Done, Status::Working],
        "finished prose never starts or extends a turn, with or without hooks; the actual activity row does"
    );
}

#[test]
fn a_codex_pane_goes_by_hooks_once_a_lifecycle_hook_arrives() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    d.terms.get_mut(&id).unwrap().agent = Some("codex".into());
    for event in ["SessionStart", "Stop", "SubagentStart"] {
        d.hook(id, "codex".into(), HookStatus::Same, event, &[]);
        assert!(
            !d.terms[&id].lifecycle_hooks,
            "{event} alone does not say its lifecycle hooks are on"
        );
    }
    for event in [
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "PermissionRequest",
    ] {
        d.terms.get_mut(&id).unwrap().lifecycle_hooks = false;
        d.hook(id, "codex".into(), HookStatus::Working, event, &[]);
        assert!(d.terms[&id].lifecycle_hooks, "{event} turns it on");
    }
    d.hook(id, "claude".into(), HookStatus::Gone, "SessionEnd", &[]);
    assert!(
        !d.terms[&id].lifecycle_hooks,
        "a session that ended starts over"
    );
    close(&mut d, &[id]);
}

#[test]
fn a_codex_pane_with_lifecycle_hooks_ignores_a_question_shown_on_screen() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let t = d.terms.get_mut(&id).unwrap();
    t.agent = Some("codex".into());
    t.hooked = true;
    t.lifecycle_hooks = true;
    t.status = Status::Working;
    t.parser.process(b"[2J[H");
    t.parser.process(
        &b"
"
        .repeat(30),
    );
    t.parser.process(
        b"Working (8m 01s - esc to interrupt)
Queued follow-up inputs
  ? 1 question
  shift+tab to answer
Ask Codex to do anything",
    );
    d.update_statuses();
    assert_eq!(
        d.terms[&id].status,
        Status::Working,
        "its hooks say when it needs you, not the screen"
    );
    close(&mut d, &[id]);
}

#[test]
fn codex_queued_questions_need_attention_even_while_work_continues() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for hooked in [false, true] {
        let t = d.terms.get_mut(&id).unwrap();
        t.agent = Some("codex".into());
        t.hooked = hooked;
        t.status = Status::Working;
        t.parser.process(b"\x1b[2J\x1b[H");
        t.parser.process(&b"\r\n".repeat(30));
        t.parser.process(b"Working (8m 01s - esc to interrupt)\r\nQueued follow-up inputs\r\n  ? 1 question\r\n  shift+tab to answer\r\nAsk Codex to do anything");
        d.update_statuses();
        observed.push((hooked, d.terms[&id].status));
        let t = d.terms.get_mut(&id).unwrap();
        t.parser.process(b"\x1b[2J\x1b[H");
        t.parser.process(&b"\r\n".repeat(30));
        t.parser
            .process(b"Working (8m 05s - esc to interrupt)\r\nAsk Codex to do anything");
        d.update_statuses();
        observed.push((hooked, d.terms[&id].status));
    }
    let t = d.terms.get_mut(&id).unwrap();
    t.parser.process(b"\x1b[2J\x1b[H");
    t.parser.process(&b"\r\n".repeat(30));
    t.parser
        .process(b"The report discusses 1 question\r\n? 0 questions\r\nWorking (esc to interrupt)");
    d.update_statuses();
    observed.push((true, d.terms[&id].status));
    close(&mut d, &[id]);
    assert_eq!(
        observed,
        [
            (false, Status::Blocked),
            (false, Status::Working),
            (true, Status::Blocked),
            (true, Status::Working),
            (true, Status::Working)
        ],
        "questions take precedence with or without hooks, then ongoing work resumes; prose and zero questions do not block"
    );
}

#[test]
fn native_pi_and_opencode_questions_stay_blocked_until_their_reply_event() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for agent in ["pi", "opencode"] {
        d.hook(id, agent.into(), HookStatus::Working, "native start", &[]);
        d.hook(
            id,
            agent.into(),
            HookStatus::Blocked,
            "native question",
            &[],
        );
        let t = d.terms.get_mut(&id).unwrap();
        t.parser
            .process(b"\x1b[2J\x1b[HCustom extension dialog\r\nPick a project");
        t.blocked_at = Some(Instant::now() - Duration::from_secs(20));
        t.last_input = Instant::now() - Duration::from_secs(10);
        t.last_output = t.last_input;
        d.update_statuses();
        observed.push(d.terms[&id].status);
        d.hook(id, agent.into(), HookStatus::Working, "native reply", &[]);
        observed.push(d.terms[&id].status);
        d.hook(id, agent.into(), HookStatus::Done, "native settled", &[]);
        observed.push(d.terms[&id].status);
    }
    close(&mut d, &[id]);
    assert_eq!(
        observed,
        [
            Status::Blocked,
            Status::Working,
            Status::Done,
            Status::Blocked,
            Status::Working,
            Status::Done
        ]
    );
}

#[test]
fn codex_expanded_question_overrides_working_until_dismissed() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for hooked in [false, true] {
        let t = d.terms.get_mut(&id).unwrap();
        t.agent = Some("codex".into());
        t.hooked = hooked;
        t.status = Status::Working;
        t.parser.process(b"\x1b[2J\x1b[H");
        t.parser.process(&b"\r\n".repeat(30));
        t.parser.process("Working (8m 01s - esc to interrupt)\r\nChoose an app:\r\n> 1. Pi / OpenCode\r\nenter submit   ctrl+] skip   shift+↔ main prompt".as_bytes());
        d.update_statuses();
        observed.push(d.terms[&id].status);
        let t = d.terms.get_mut(&id).unwrap();
        t.parser.process(b"\x1b[2J\x1b[H");
        t.parser.process(&b"\r\n".repeat(30));
        t.parser.process(b"Working (8m 02s - esc to interrupt)");
        d.update_statuses();
        observed.push(d.terms[&id].status);
    }
    close(&mut d, &[id]);
    assert_eq!(
        observed,
        [
            Status::Blocked,
            Status::Working,
            Status::Blocked,
            Status::Working
        ]
    );
}

fn failed_check(output: &str) -> check::CheckRecord {
    let info = CheckInfo {
        command: "npm test".into(),
        state: CheckState::Failed,
        exit_code: Some(1),
        first_failing_line: "FAIL footer.test.ts".into(),
        ..Default::default()
    };
    check::CheckRecord::new(info, output)
}

#[test]
fn the_snapshot_carries_a_check_summary_and_a_query_returns_its_output() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(
        7,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    d.message(
        7,
        ClientMsg::Query(Query::CheckOutput {
            term: t,
            name: None,
        }),
    );
    assert!(
        matches!(out.try_recv(), Ok(ServerMsg::Error(_))),
        "no check, no output"
    );
    d.terms.get_mut(&t).unwrap().checks.points =
        vec![failed_check("ok a\nFAIL footer.test.ts\n").into()];
    let info = d.snapshot().terms[&t]
        .check()
        .cloned()
        .expect("the summary is in the snapshot");
    assert_eq!(
        (info.state, info.first_failing_line.as_str()),
        (CheckState::Failed, "FAIL footer.test.ts")
    );
    d.message(
        7,
        ClientMsg::Query(Query::CheckOutput {
            term: t,
            name: None,
        }),
    );
    assert!(
        matches!(out.try_recv(), Ok(ServerMsg::Reply(Reply::Text(text))) if text == "ok a\nFAIL footer.test.ts\n")
    );
    close(&mut d, &[t]);
}

#[test]
fn a_pane_may_not_read_another_panes_check_output_without_the_read_grant() {
    let (mut d, _rx) = daemon();
    let (a, b) = (pane(&mut d), pane(&mut d));
    d.terms.get_mut(&b).unwrap().checks.points = vec![failed_check("secret-ish output").into()];
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(
        7,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    d.terms.get_mut(&a).unwrap().grants = Some(Vec::new());
    d.message(
        7,
        ClientMsg::Query(Query::CheckOutput {
            term: b,
            name: None,
        }),
    );
    assert!(
        matches!(out.try_recv(), Ok(ServerMsg::Error(_))),
        "read is a grant"
    );
    close(&mut d, &[a, b]);
}

#[test]
fn check_results_survive_a_restart_with_their_output() {
    let (mut d, _rx) = daemon();
    let dir = std::env::temp_dir();
    let tails = persist::scratch_dir();
    d.check_dir = tails.path().to_path_buf();
    let old: TermId = 9_201;
    let overridden = CheckInfo {
        state: CheckState::Overridden,
        overridden_at: Some(5),
        override_reason: "flaky".into(),
        ..failed_check("").info
    };
    let unsaved = persist::save_check_outputs(
        tails.path(),
        persist::save_ticket(),
        &[
            persist::CheckTail {
                term: old,
                name: String::new(),
                slot: persist::CheckSlot::Latest,
                text: "latest tail".into(),
            },
            persist::CheckTail {
                term: old,
                name: String::new(),
                slot: persist::CheckSlot::Overridden,
                text: "overridden tail".into(),
            },
        ],
        &[old],
    );
    assert!(unsaved.is_empty());
    let pane = persist::SavedPane {
        cwd: Some(dir.clone()),
        checks: vec![failed_check("").info],
        checks_overridden: vec![overridden.clone()],
        check_trees: [(String::new(), "4b825dc".to_string())].into(),
        ..Default::default()
    };
    let tab = persist::SavedTab {
        name: String::new(),
        layout: Node::Leaf(old),
        focus: old,
        panes: [(old, pane)].into(),
    };
    let ws = persist::SavedWs {
        name: String::new(),
        cwd: dir,
        worktree: false,
        color: None,
        group: None,
        tabs: vec![tab],
        active_tab: 0,
    };
    d.restore(persist::Saved {
        workspaces: vec![ws],
        active: 0,
        made_worktrees: Vec::new(),
        conversations: Vec::new(),
        pending_starts: Vec::new(),
        conversation_operations: Vec::new(),
        turn_continues: Default::default(),
        turns_in_flight: Vec::new(),
    });
    let t = d.workspaces[0].tabs[0].focus;
    let restored = &d.terms[&t];
    assert_eq!(
        restored.checks.headline().map(|c| c.output.as_str()),
        Some("latest tail")
    );
    assert_eq!(
        restored
            .checks
            .points
            .first()
            .and_then(|p| p.overridden.as_ref())
            .map(|c| (&c.info, c.output.as_str())),
        Some((&overridden, "overridden tail"))
    );
    let saved = d.saved();
    let again = &saved.workspaces[0].tabs[0].panes[&t];
    assert_eq!(
        (
            again.checks.iter().map(|c| c.state).collect::<Vec<_>>(),
            &again.checks_overridden
        ),
        (vec![CheckState::Failed], &vec![overridden.clone()])
    );
    assert_eq!(
        again.check_trees.get(""),
        Some(&"4b825dc".to_string()),
        "the tree its run saw is saved again"
    );
    let mut written = d.take_dirty_check_outputs();
    written.sort_by_key(|w| w.slot as u8);
    let tail = |slot, text: &str| persist::CheckTail {
        term: t,
        name: String::new(),
        slot,
        text: text.into(),
    };
    let expected = [
        tail(persist::CheckSlot::Latest, "latest tail"),
        tail(persist::CheckSlot::Overridden, "overridden tail"),
    ];
    assert_eq!(
        written, expected,
        "its output is written again under the new id"
    );
    assert!(d.take_dirty_check_outputs().is_empty(), "and only once");
    close(&mut d, &[t]);
}

#[test]
fn a_check_that_was_running_when_the_server_stopped_reads_as_not_run() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let running = CheckInfo {
        state: CheckState::Checking,
        ..Default::default()
    };
    let saved = persist::SavedPane {
        checks: vec![running],
        ..Default::default()
    };
    d.terms
        .get_mut(&t)
        .unwrap()
        .restore_checks(&d.check_dir.clone(), 9_202, &saved);
    assert_eq!(
        d.terms[&t].checks.headline().map(|c| c.info.state),
        Some(CheckState::CouldNotRun)
    );
    close(&mut d, &[t]);
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("a runtime for the blocking tasks")
}

const LONG_CHECK_S: u64 = 600;

/// An agent session in the temp folder (not a repo, so the folder is its project), whose
/// project has a check running `command`.
fn checked_agent(d: &mut Daemon, command: &str, timeout_s: u64) -> TermId {
    d.cfg.check = vec![crate::config::CheckDef {
        project: std::env::temp_dir(),
        name: None,
        command: command.into(),
        timeout_s,
    }];
    let t = pane(d);
    let p = d.terms.get_mut(&t).unwrap();
    (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Working);
    t
}

fn end_turn(d: &mut Daemon, t: TermId) {
    d.hook(t, "claude".into(), HookStatus::Done, "Stop", &[]);
}

/// Apply the next finished check, as the daemon loop would (and the resolving before it).
/// False if none came in time.
fn finish_check(d: &mut Daemon, rx: &mut mpsc::Receiver<Ev>, wait: Duration) -> bool {
    let until = Instant::now() + wait;
    while Instant::now() < until {
        match rx.try_recv() {
            Ok(ev @ Ev::ChecksResolved { .. }) => d.handle(ev),
            Ok(ev @ Ev::CheckRan { .. }) => {
                d.handle(ev);
                return true;
            }
            Ok(_) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    false
}

fn check_of(d: &Daemon, t: TermId) -> CheckInfo {
    d.terms[&t].checks.headline().expect("a check").info.clone()
}

const SLOW_CHECK_WAIT: Duration = Duration::from_secs(30);

#[test]
fn a_turn_end_runs_the_projects_check_and_a_pass_is_recorded() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "echo \"suite ok\"; exit 0", LONG_CHECK_S);
    end_turn(&mut d, t);
    let running = check_of(&d, t);
    assert_eq!(
        (running.state, running.command.as_str()),
        (CheckState::Checking, "echo \"suite ok\"; exit 0")
    );
    assert!(
        d.snapshot().terms[&t].check().is_some(),
        "clients see it checking"
    );
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    let done = check_of(&d, t);
    assert_eq!((done.state, done.exit_code), (CheckState::Passed, Some(0)));
    assert_eq!(
        d.terms[&t].checks.headline().unwrap().output.trim(),
        "suite ok"
    );
    assert!(d.terms[&t].checks.dirty, "its tail is due to be saved");
    close(&mut d, &[t]);
}

#[test]
fn a_failing_check_records_its_exit_code_and_output() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "echo \"1 test failed\"; exit 3", LONG_CHECK_S);
    end_turn(&mut d, t);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    let info = check_of(&d, t);
    assert_eq!((info.state, info.exit_code), (CheckState::Failed, Some(3)));
    assert_eq!(
        d.terms[&t].checks.headline().unwrap().output.trim(),
        "1 test failed"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_failed_check_names_its_first_failing_line_and_a_rerun_clears_it() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(
        &mut d,
        "echo running; echo \"FAIL tests/foo: boom\"; exit 1",
        LONG_CHECK_S,
    );
    end_turn(&mut d, t);
    assert!(
        check_of(&d, t).first_failing_line.is_empty(),
        "nothing while it runs"
    );
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).first_failing_line, "FAIL tests/foo: boom");
    let seen = d.snapshot().terms[&t]
        .check()
        .cloned()
        .expect("the summary is in the snapshot");
    assert_eq!(seen.first_failing_line, "FAIL tests/foo: boom");
    d.terms.get_mut(&t).unwrap().status = Status::Idle;
    d.rerun_check(t, None)
        .expect("a failed check can be run again");
    assert!(
        check_of(&d, t).first_failing_line.is_empty(),
        "a re-run starts clean"
    );
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    close(&mut d, &[t]);
}

#[test]
fn a_check_that_outruns_its_timeout_reads_as_not_run() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let slow = if cfg!(windows) {
        "Start-Sleep 60"
    } else {
        "sleep 60"
    };
    let t = checked_agent(&mut d, slow, 1);
    end_turn(&mut d, t);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    let info = check_of(&d, t);
    assert_eq!(
        (info.state, info.exit_code),
        (CheckState::CouldNotRun, None)
    );
    close(&mut d, &[t]);
}

#[cfg(unix)]
#[test]
fn a_check_whose_command_is_not_found_reads_as_not_run() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(
        &mut d,
        concat!(env!("APP_COMMAND"), "-no-such-command-xyz"),
        LONG_CHECK_S,
    );
    end_turn(&mut d, t);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).state, CheckState::CouldNotRun);
    close(&mut d, &[t]);
}

#[test]
fn a_shell_that_cannot_start_leaves_the_check_not_run() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    d.cfg.shell = Some(concat!("/no/such/shell-for-", env!("APP_COMMAND")).into());
    end_turn(&mut d, t);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(
        check_of(&d, t).state,
        CheckState::CouldNotRun,
        "never stuck checking"
    );
    close(&mut d, &[t]);
}

#[test]
fn the_screen_ending_a_turn_runs_the_check_too_but_a_cancelled_turn_does_not() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    d.set_status(t, Status::Idle, "progress bar gone, no turn end: cancelled");
    assert!(
        d.terms[&t].checks.points.is_empty(),
        "an interrupted turn finished nothing"
    );
    d.set_status(t, Status::Working, "test");
    d.set_status(t, Status::Done, "screen: went quiet after working");
    assert_eq!(check_of(&d, t).state, CheckState::Checking);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).state, CheckState::Passed);
    close(&mut d, &[t]);
}

#[test]
fn sessions_without_a_check_a_shell_or_a_finished_turn_behave_as_before() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    // A project with no check.
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    d.cfg.check[0].project = std::env::temp_dir().join("some-other-project");
    end_turn(&mut d, t);
    assert!(
        d.terms[&t].checks.points.is_empty(),
        "no check in this project"
    );
    assert_eq!(d.terms[&t].status, Status::Done, "and done as ever");
    // A shell pane in a project with a check.
    d.cfg.check[0].project = std::env::temp_dir();
    let shell = pane(&mut d);
    d.terms.get_mut(&shell).unwrap().status = Status::Working;
    d.set_status(shell, Status::Done, "test");
    assert!(
        d.terms[&shell].checks.points.is_empty(),
        "a shell is not an agent"
    );
    // A turn that has not ended: working, asking, or held for its helpers.
    let busy = pane(&mut d);
    (
        d.terms.get_mut(&busy).unwrap().agent,
        d.terms.get_mut(&busy).unwrap().hooked,
    ) = (Some("claude".into()), true);
    d.hook(
        busy,
        "claude".into(),
        HookStatus::Working,
        "UserPromptSubmit",
        &[],
    );
    d.hook(
        busy,
        "claude".into(),
        HookStatus::Blocked,
        "PreToolUse",
        &[],
    );
    d.terms
        .get_mut(&busy)
        .unwrap()
        .subagents
        .push(("sub".into(), "explore".into()));
    d.hook(busy, "claude".into(), HookStatus::Done, "Stop", &[]);
    assert!(
        d.terms[&busy].checks.points.is_empty(),
        "a turn waiting on its helpers has not ended"
    );
    d.hook(
        busy,
        "claude".into(),
        HookStatus::Done,
        "Notification:idle_prompt",
        &[],
    );
    assert!(
        d.terms[&busy].checks.points.is_empty(),
        "a repeated idle ping is not a turn end"
    );
    close(&mut d, &[t, shell, busy]);
}

#[test]
fn a_turn_ending_while_its_check_runs_does_not_start_a_second_run() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    end_turn(&mut d, t);
    let first = check_of(&d, t);
    d.hook(
        t,
        "claude".into(),
        HookStatus::Working,
        "UserPromptSubmit",
        &[],
    );
    end_turn(&mut d, t);
    assert_eq!(
        check_of(&d, t),
        first,
        "the running check is left as it was"
    );
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert!(
        !finish_check(&mut d, &mut rx, Duration::from_millis(500)),
        "only one run was started"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_done_hook_after_the_screen_ended_the_turn_and_its_check_settled_starts_no_second_run() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    d.set_status(t, Status::Done, "screen: went quiet after working");
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    let settled = check_of(&d, t);
    assert_eq!(settled.state, CheckState::Passed);
    end_turn(&mut d, t);
    assert_eq!(
        check_of(&d, t),
        settled,
        "the same turn is not checked twice"
    );
    assert!(
        !finish_check(&mut d, &mut rx, Duration::from_millis(500)),
        "no second run was started"
    );
    d.hook(
        t,
        "claude".into(),
        HookStatus::Working,
        "UserPromptSubmit",
        &[],
    );
    end_turn(&mut d, t);
    assert_eq!(
        check_of(&d, t).state,
        CheckState::Checking,
        "a new turn is checked"
    );
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    close(&mut d, &[t]);
}

#[test]
fn the_next_turn_end_checks_again() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 1", LONG_CHECK_S);
    end_turn(&mut d, t);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).state, CheckState::Failed);
    d.hook(
        t,
        "claude".into(),
        HookStatus::Working,
        "UserPromptSubmit",
        &[],
    );
    end_turn(&mut d, t);
    assert_eq!(check_of(&d, t).state, CheckState::Checking);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    close(&mut d, &[t]);
}

#[test]
fn a_result_for_a_pane_that_closed_is_dropped() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    end_turn(&mut d, t);
    close(&mut d, &[t]);
    assert!(
        finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT),
        "the result still arrives and is ignored"
    );
    assert!(!d.terms.contains_key(&t));
}

#[test]
fn the_tail_of_a_closed_pane_goes_on_a_full_save_even_when_no_tail_changed() {
    let (mut d, _rx) = daemon();
    let tails = persist::scratch_dir();
    d.check_dir = tails.path().to_path_buf();
    let (open, closed): (TermId, TermId) = (pane(&mut d), 7_001);
    let unsaved = persist::save_check_outputs(
        tails.path(),
        persist::save_ticket(),
        &[
            persist::CheckTail {
                term: open,
                name: String::new(),
                slot: persist::CheckSlot::Latest,
                text: "kept".into(),
            },
            persist::CheckTail {
                term: closed,
                name: String::new(),
                slot: persist::CheckSlot::Latest,
                text: "gone".into(),
            },
        ],
        &[open, closed],
    );
    assert!(unsaved.is_empty());
    d.save_check_tails(persist::save_ticket(), vec![open], true);
    assert_eq!(
        persist::read_check_output(tails.path(), closed, "", persist::CheckSlot::Latest),
        None,
        "pruned at shutdown"
    );
    assert_eq!(
        persist::read_check_output(tails.path(), open, "", persist::CheckSlot::Latest).as_deref(),
        Some("kept")
    );
    close(&mut d, &[open]);
}

#[test]
fn a_tail_that_failed_to_write_is_written_again_at_the_next_save() {
    let (mut d, _rx) = daemon();
    let tails = persist::scratch_dir();
    let blocked = tails.path().to_path_buf();
    std::fs::create_dir_all(&blocked).unwrap();
    // A file where the folder should be: every write fails.
    d.check_dir = blocked.join("tails");
    std::fs::write(&d.check_dir, "in the way").unwrap();
    let t = pane(&mut d);
    d.terms.get_mut(&t).unwrap().checks.points = vec![failed_check("FAIL footer\n").into()];
    d.terms.get_mut(&t).unwrap().checks.dirty = true;
    d.save_check_tails(persist::save_ticket(), vec![t], true);
    assert!(
        d.terms[&t].checks.dirty,
        "still due: the write did not happen"
    );
    std::fs::remove_file(&d.check_dir).unwrap();
    d.save_check_tails(persist::save_ticket(), vec![t], true);
    assert!(!d.terms[&t].checks.dirty);
    assert_eq!(
        persist::read_check_output(&d.check_dir, t, "", persist::CheckSlot::Latest).as_deref(),
        Some("FAIL footer\n")
    );
    close(&mut d, &[t]);
}

#[test]
fn a_background_save_that_fails_marks_its_tails_due_again() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.terms.get_mut(&t).unwrap().checks.points = vec![failed_check("x").into()];
    assert!(d.take_dirty_check_outputs().is_empty(), "nothing due");
    d.terms.get_mut(&t).unwrap().checks.dirty = true;
    assert_eq!(d.take_dirty_check_outputs().len(), 1);
    d.handle(Ev::CheckOutputsUnsaved(vec![t, 9_999]));
    assert_eq!(
        d.take_dirty_check_outputs().len(),
        1,
        "due again; a pane that is gone is skipped"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_restored_tail_stays_on_disk_until_the_new_one_is_saved() {
    let (mut d, _rx) = daemon();
    let tails = persist::scratch_dir();
    d.check_dir = tails.path().to_path_buf();
    let old: TermId = 9_301;
    assert!(
        persist::save_check_outputs(
            tails.path(),
            persist::save_ticket(),
            &[persist::CheckTail {
                term: old,
                name: String::new(),
                slot: persist::CheckSlot::Latest,
                text: "tail".into()
            }],
            &[old]
        )
        .is_empty()
    );
    let t = pane(&mut d);
    let saved = persist::SavedPane {
        checks: vec![failed_check("").info],
        ..Default::default()
    };
    let dir = d.check_dir.clone();
    d.terms
        .get_mut(&t)
        .unwrap()
        .restore_checks(&dir, old, &saved);
    assert_eq!(
        persist::read_check_output(tails.path(), old, "", persist::CheckSlot::Latest).as_deref(),
        Some("tail"),
        "a crash now loses nothing"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_tail_kept_before_checkpoints_had_names_is_restored_only_into_a_lone_checkpoint() {
    let (mut d, _rx) = daemon();
    let tails = persist::scratch_dir();
    std::fs::create_dir_all(tails.path()).unwrap();
    let old: TermId = 9_302;
    std::fs::write(
        tails.path().join(format!("{old}-latest.txt")),
        "old single check",
    )
    .unwrap();
    let dir = tails.path().to_path_buf();
    let info = |name: &str| CheckInfo {
        name: name.into(),
        state: CheckState::Failed,
        ..Default::default()
    };
    let t = pane(&mut d);
    let two = persist::SavedPane {
        checks: vec![info("build"), info("test")],
        ..Default::default()
    };
    d.terms.get_mut(&t).unwrap().restore_checks(&dir, old, &two);
    let outputs: Vec<&str> = d.terms[&t]
        .checks
        .points
        .iter()
        .map(|p| p.latest.output.as_str())
        .collect();
    assert_eq!(
        outputs,
        ["", ""],
        "neither of two checkpoints is the old single check"
    );
    let one = persist::SavedPane {
        checks: vec![info("test")],
        ..Default::default()
    };
    d.terms.get_mut(&t).unwrap().restore_checks(&dir, old, &one);
    assert_eq!(
        d.terms[&t].checks.points[0].latest.output, "old single check",
        "a lone checkpoint is the migrated one"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_repo_checks_file_is_only_noticed_once_the_worktree_was_looked_at() {
    let (rt, (mut d, mut rx, _file)) = (runtime(), approving_daemon());
    let _in_rt = rt.enter();
    let repo = temp_repo();
    std::fs::create_dir_all(repo.path().join(concat!(".", env!("APP_COMMAND")))).unwrap();
    std::fs::write(
        repo.path().join(crate::config::REPO_CHECKS_FILE),
        "[[check]]
name = \"test\"
command = \"echo tested\"
",
    )
    .unwrap();
    let t = agent_with_checkpoints(&mut d, repo.path(), &[]);
    let err = d
        .command(
            0,
            Command::RerunCheck {
                term: t,
                name: None,
            },
        )
        .expect_err("not looked at yet");
    assert!(err.to_string().contains("no check"), "{err}");
    look_for_repo_checks(&mut d, &mut rx, t);
    std::fs::remove_file(repo.path().join(crate::config::REPO_CHECKS_FILE)).unwrap();
    let p = d.terms.get_mut(&t).unwrap();
    p.status = Status::Done;
    assert!(
        d.command(
            0,
            Command::RerunCheck {
                term: t,
                name: None
            }
        )
        .is_ok(),
        "the last look is what counts; the run settles what the file names"
    );
    finish_run(&mut d, &mut rx, t);
    look_for_repo_checks(&mut d, &mut rx, t);
    let err = d
        .command(
            0,
            Command::RerunCheck {
                term: t,
                name: None,
            },
        )
        .expect_err("gone at the next look");
    assert!(err.to_string().contains("no check"), "{err}");
    close(&mut d, &[t]);
}

fn with_check(d: &mut Daemon, t: TermId, state: CheckState, output: &str) {
    let record = failed_check(output);
    // Named as the project's checkpoint is, so a run finds it.
    let name = d
        .cfg
        .check
        .first()
        .map(|c| c.name().to_string())
        .unwrap_or_default();
    let info = CheckInfo {
        name,
        state,
        exit_code: (state == CheckState::Failed).then_some(1),
        after_turn_ended_at: 77,
        ..record.info
    };
    d.terms.get_mut(&t).unwrap().checks.points = vec![check::CheckRecord::new(info, output).into()];
}

#[test]
fn a_rerun_while_the_agent_works_waits_for_the_turn_to_end() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "echo again; exit 0", LONG_CHECK_S);
    with_check(&mut d, t, CheckState::Failed, "old failure");
    d.command(
        0,
        Command::RerunCheck {
            term: t,
            name: None,
        },
    )
    .unwrap();
    let waiting = check_of(&d, t);
    assert_eq!(
        (waiting.state, waiting.rerun_queued),
        (CheckState::Failed, true),
        "the result stands, and says a re-run is queued"
    );
    assert!(
        d.snapshot().terms[&t]
            .check()
            .is_some_and(|c| c.rerun_queued),
        "clients see it"
    );
    assert!(
        !finish_check(&mut d, &mut rx, Duration::from_millis(300)),
        "nothing runs mid-turn"
    );
    end_turn(&mut d, t);
    let running = check_of(&d, t);
    assert_eq!(
        (running.state, running.rerun_queued),
        (CheckState::Checking, false)
    );
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert!(
        !finish_check(&mut d, &mut rx, Duration::from_millis(500)),
        "the queued re-run and the turn end are one run"
    );
    assert_eq!(check_of(&d, t).state, CheckState::Passed);
    close(&mut d, &[t]);
}

#[test]
fn a_queued_rerun_is_dropped_when_the_project_no_longer_has_a_check() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    with_check(&mut d, t, CheckState::Failed, "old");
    d.command(
        0,
        Command::RerunCheck {
            term: t,
            name: None,
        },
    )
    .unwrap();
    d.cfg.check.clear();
    end_turn(&mut d, t);
    assert!(!check_of(&d, t).rerun_queued, "nothing is queued any more");
    close(&mut d, &[t]);
}

#[test]
fn a_rerun_of_an_idle_agent_starts_at_once_and_replaces_the_settled_result() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "echo fresh; exit 0", LONG_CHECK_S);
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    with_check(&mut d, t, CheckState::Failed, "old failure");
    d.command(
        0,
        Command::RerunCheck {
            term: t,
            name: None,
        },
    )
    .unwrap();
    let running = check_of(&d, t);
    assert_eq!(
        (
            running.state,
            running.after_turn_ended_at,
            running.rerun_queued
        ),
        (CheckState::Checking, 77, false)
    );
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).state, CheckState::Passed);
    assert_eq!(
        d.terms[&t].checks.headline().unwrap().output.trim(),
        "fresh"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_rerun_that_has_nothing_to_run_is_an_error() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    assert!(
        d.command(
            0,
            Command::RerunCheck {
                term: t,
                name: None
            }
        )
        .is_err(),
        "working, never checked: its turn end checks it"
    );
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    with_check(&mut d, t, CheckState::Checking, "");
    assert!(
        d.command(
            0,
            Command::RerunCheck {
                term: t,
                name: None
            }
        )
        .is_err(),
        "already running"
    );
    d.cfg.check.clear();
    with_check(&mut d, t, CheckState::Failed, "");
    assert!(
        d.command(
            0,
            Command::RerunCheck {
                term: t,
                name: None
            }
        )
        .is_err(),
        "no check in this project"
    );
    let shell = pane(&mut d);
    assert!(
        d.command(
            0,
            Command::RerunCheck {
                term: shell,
                name: None
            }
        )
        .is_err(),
        "a shell is not checked"
    );
    assert!(
        d.command(
            0,
            Command::RerunCheck {
                term: 9_999,
                name: None
            }
        )
        .is_err(),
        "no such pane"
    );
    close(&mut d, &[t, shell]);
}

#[test]
fn only_a_failed_check_can_be_overridden_and_the_override_is_recorded() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let cmd = |reason: &str| Command::OverrideCheck {
        term: t,
        name: None,
        reason: reason.into(),
    };
    assert!(d.command(0, cmd("")).is_err(), "no result");
    for state in [
        CheckState::Passed,
        CheckState::Checking,
        CheckState::CouldNotRun,
        CheckState::Overridden,
    ] {
        with_check(&mut d, t, state, "out");
        assert!(d.command(0, cmd("")).is_err(), "{state:?}");
        assert_eq!(check_of(&d, t).state, state, "left as it was");
    }
    with_check(&mut d, t, CheckState::Failed, "FAIL a\nFAIL b\n");
    let before = term::unix_now();
    d.command(0, cmd("  flaky on CI  ")).unwrap();
    let info = check_of(&d, t);
    assert_eq!(
        (info.state, info.override_reason.as_str(), info.exit_code),
        (CheckState::Overridden, "flaky on CI", Some(1))
    );
    assert!(
        info.overridden_at.is_some_and(|at| at >= before),
        "the time is recorded"
    );
    assert_eq!(
        d.snapshot().terms[&t].check().map(|c| c.state),
        Some(CheckState::Overridden)
    );
    let term = &d.terms[&t];
    assert_eq!(
        term.checks.headline().unwrap().output,
        "FAIL a\nFAIL b\n",
        "the failure's output stays"
    );
    assert_eq!(
        term.checks
            .points
            .first()
            .and_then(|p| p.overridden.as_ref())
            .map(|c| (c.info.clone(), c.output.as_str())),
        Some((info, "FAIL a\nFAIL b\n"))
    );
    assert!(term.checks.dirty, "due to be saved");
    close(&mut d, &[t]);
}

#[test]
fn an_override_reason_is_optional_and_bounded() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    with_check(&mut d, t, CheckState::Failed, "");
    let too_long = "x".repeat(super::check::OVERRIDE_REASON_MAX_CHARS + 1);
    assert!(
        d.command(
            0,
            Command::OverrideCheck {
                term: t,
                name: None,
                reason: too_long
            }
        )
        .is_err()
    );
    assert_eq!(check_of(&d, t).state, CheckState::Failed, "refused whole");
    d.command(
        0,
        Command::OverrideCheck {
            term: t,
            name: None,
            reason: String::new(),
        },
    )
    .unwrap();
    assert_eq!(check_of(&d, t).override_reason, "");
    close(&mut d, &[t]);
}

#[test]
fn undoing_an_override_restores_the_failed_result() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    with_check(&mut d, t, CheckState::Failed, "FAIL a");
    let failed = check_of(&d, t);
    assert!(
        d.command(
            0,
            Command::UndoOverride {
                term: t,
                name: None
            }
        )
        .is_err(),
        "nothing to undo on a failure"
    );
    d.command(
        0,
        Command::OverrideCheck {
            term: t,
            name: None,
            reason: "r".into(),
        },
    )
    .unwrap();
    d.command(
        0,
        Command::UndoOverride {
            term: t,
            name: None,
        },
    )
    .unwrap();
    assert_eq!(check_of(&d, t), failed, "exactly as it was");
    assert!(
        d.terms[&t]
            .checks
            .points
            .first()
            .is_none_or(|p| p.overridden.is_none())
    );
    assert!(
        d.command(
            0,
            Command::UndoOverride {
                term: t,
                name: None
            }
        )
        .is_err(),
        "and only once"
    );
    close(&mut d, &[t]);
}

#[test]
fn the_next_run_replaces_an_override_but_keeps_the_overridden_output() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "echo green; exit 0", LONG_CHECK_S);
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    with_check(&mut d, t, CheckState::Failed, "FAIL a");
    d.command(
        0,
        Command::OverrideCheck {
            term: t,
            name: None,
            reason: "r".into(),
        },
    )
    .unwrap();
    d.command(
        0,
        Command::RerunCheck {
            term: t,
            name: None,
        },
    )
    .unwrap();
    let running = check_of(&d, t);
    assert_eq!(
        (
            running.state,
            running.overridden_at,
            running.override_reason.as_str()
        ),
        (CheckState::Checking, None, "")
    );
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).state, CheckState::Passed);
    let kept = d.terms[&t]
        .checks
        .points
        .first()
        .and_then(|p| p.overridden.as_ref())
        .expect("the overridden result is kept");
    assert_eq!(
        (kept.info.state, kept.output.as_str()),
        (CheckState::Overridden, "FAIL a")
    );
    close(&mut d, &[t]);
}

#[test]
fn a_read_only_pane_may_not_rerun_or_override_another_panes_check() {
    let (mut d, _rx) = daemon();
    let (a, b) = (pane(&mut d), pane(&mut d));
    with_check(&mut d, b, CheckState::Failed, "FAIL");
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(
        7,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    d.terms.get_mut(&a).unwrap().grants = Some(vec!["read".into()]);
    for cmd in [
        Command::RerunCheck {
            term: b,
            name: None,
        },
        Command::OverrideCheck {
            term: b,
            name: None,
            reason: String::new(),
        },
        Command::UndoOverride {
            term: b,
            name: None,
        },
    ] {
        d.message(7, ClientMsg::Command(cmd.clone()));
        assert!(
            matches!(out.try_recv(), Ok(ServerMsg::Error(_))),
            "{cmd:?} needs write"
        );
    }
    assert_eq!(check_of(&d, b).state, CheckState::Failed, "untouched");
    d.terms.get_mut(&a).unwrap().grants = Some(vec!["read".into(), "write".into()]);
    d.message(
        7,
        ClientMsg::Command(Command::OverrideCheck {
            term: b,
            name: None,
            reason: "ok".into(),
        }),
    );
    assert!(
        matches!(out.try_recv(), Ok(ServerMsg::Reply(Reply::Ok))),
        "write allows it"
    );
    assert_eq!(check_of(&d, b).state, CheckState::Overridden);
    close(&mut d, &[a, b]);
}

/// What the MCP server asks this daemon, as the calling pane, before it reaches another machine:
/// a read of, and a write to, a pane that is not there. The server counts only these answers
/// as "allowed", so they are pinned here (`cli/mcp/machines.rs`, `Reach::caller_may`).
#[test]
fn the_grant_probes_the_mcp_server_uses_are_decided_by_the_panes_grants() {
    let (mut d, _rx) = daemon();
    let a = pane(&mut d);
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(
        7,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    let absent = TermId::MAX;
    let mut ask = |d: &mut Daemon, grants: &[&str]| {
        d.terms.get_mut(&a).unwrap().grants = Some(grants.iter().map(|g| g.to_string()).collect());
        d.message(7, ClientMsg::Query(Query::Read { term: absent }));
        let read = out.try_recv();
        d.message(7, ClientMsg::Command(Command::ClearQueue { term: absent }));
        (read, out.try_recv())
    };
    let (read, write) = ask(&mut d, &["read", "write"]);
    assert!(
        matches!(read, Ok(ServerMsg::Error(e)) if e == format!("no pane {absent}")),
        "read passes the grant and finds no pane"
    );
    assert!(matches!(write, Ok(ServerMsg::Reply(Reply::Ok))));
    let (read, write) = ask(&mut d, &["read"]);
    assert!(matches!(read, Ok(ServerMsg::Error(e)) if e == format!("no pane {absent}")));
    assert!(matches!(write, Ok(ServerMsg::Error(e)) if e.contains("may not")));
    let (read, write) = ask(&mut d, &[]);
    assert!(matches!(read, Ok(ServerMsg::Error(e)) if e.contains("may not")));
    assert!(matches!(write, Ok(ServerMsg::Error(e)) if e.contains("may not")));
    close(&mut d, &[a]);
}

#[test]
fn a_pane_may_not_override_its_own_check() {
    let (mut d, _rx) = daemon();
    let a = pane(&mut d);
    with_check(&mut d, a, CheckState::Failed, "FAIL");
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(
        7,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    d.message(
        7,
        ClientMsg::Command(Command::OverrideCheck {
            term: a,
            name: None,
            reason: "trust me".into(),
        }),
    );
    assert!(
        matches!(out.try_recv(), Ok(ServerMsg::Error(_))),
        "an agent's claim never counts"
    );
    assert_eq!(check_of(&d, a).state, CheckState::Failed);
    close(&mut d, &[a]);
}

#[test]
fn only_you_answer_a_repo_check_approval() {
    let (mut d, _rx, _file) = approving_daemon();
    let a = waiting_for_approval(&mut d, "npm test");
    d.approvals.decide(
        &std::env::temp_dir(),
        "build",
        "make",
        approvals::Decision::Never,
        1,
    );
    let kept = d.approvals.clone();
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(
        7,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    d.terms.get_mut(&a).unwrap().grants = Some(
        ["read", "write", "start", "respond", "admin"]
            .map(String::from)
            .into(),
    );
    let forget = Command::ForgetCheckApproval {
        project: std::env::temp_dir(),
        name: "build".into(),
    };
    for cmd in [
        ApprovalChoice::Once,
        ApprovalChoice::Always,
        ApprovalChoice::Never,
    ]
    .map(|choice| Command::ApproveCheck {
        term: a,
        name: "test".into(),
        choice,
        command_sha256: approvals::command_hash("npm test"),
    })
    .into_iter()
    .chain([forget])
    {
        d.message(7, ClientMsg::Command(cmd.clone()));
        assert!(
            matches!(out.try_recv(), Ok(ServerMsg::Error(e)) if e.contains("yours")),
            "{cmd:?}: a pane may not, whatever its grants"
        );
    }
    assert_eq!(
        d.approvals.clone().for_run(a),
        kept.clone().for_run(a),
        "nothing granted, kept or forgotten"
    );
    assert_eq!(
        check_of(&d, a).state,
        CheckState::NeedsApproval,
        "and nothing ran"
    );
    close(&mut d, &[a]);
}

fn checking(d: &mut Daemon, t: TermId) {
    let info = CheckInfo {
        name: "test".into(),
        command: "npm test".into(),
        state: CheckState::Checking,
        ..Default::default()
    };
    d.terms.get_mut(&t).unwrap().checks.points = vec![check::CheckRecord::new(info, "").into()];
}

fn ran(outcome: check::CheckOutcome) -> check::CheckRun {
    check::CheckRun {
        outcome,
        output: String::new(),
        lines: 0,
        duration: Duration::from_millis(5),
    }
}

/// The running checkpoint `test` (see `checking`) ends with `outcome`.
fn check_ends(d: &mut Daemon, t: TermId, outcome: check::CheckOutcome) {
    let run = d.terms[&t].checks.run;
    d.check_ran(t, run, "test", ran(outcome));
}

fn working_agent(d: &mut Daemon) -> TermId {
    let t = pane(d);
    d.terms.get_mut(&t).unwrap().agent = Some("claude".into());
    d.set_status(t, Status::Working, "test");
    t
}

#[test]
fn an_agent_that_reports_no_last_message_gets_its_screen_s_last_lines() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    d.terms.get_mut(&t).unwrap().parser.process(
        b"All three migrations ran clean.\r\nWant me to open the pull request?\r\n\r\n> \r\n",
    );
    d.set_status(t, Status::Done, "test");
    assert_eq!(
        d.terms[&t].last_lines,
        "All three migrations ran clean.\nWant me to open the pull request?",
        "what stands above its input line when the turn ends"
    );

    d.set_status(t, Status::Working, "test");
    assert!(
        d.terms[&t].last_lines.is_empty(),
        "a turn under way has said nothing yet"
    );

    // An agent whose hooks told us what it said needs nothing read off its screen.
    d.terms.get_mut(&t).unwrap().said = "Opened the pull request.".into();
    d.set_status(t, Status::Blocked, "test");
    assert!(d.terms[&t].last_lines.is_empty());
    close(&mut d, &[t]);
}

fn overdue(d: &mut Daemon, t: TermId) {
    d.terms.get_mut(&t).unwrap().alert_due.as_mut().unwrap().1 =
        Instant::now() - Duration::from_millis(1);
}

fn viewer(d: &mut Daemon) -> mpsc::Receiver<ServerMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        40,
        tx,
        true,
        crate::protocol::Surface::Terminal,
        true,
    ));
    while rx.try_recv().is_ok() {}
    rx
}

fn heard(rx: &mut mpsc::Receiver<ServerMsg>) -> Vec<ServerMsg> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn viewer_as(d: &mut Daemon, id: u64) -> mpsc::Receiver<ServerMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        id,
        tx,
        true,
        crate::protocol::Surface::Terminal,
        true,
    ));
    while rx.try_recv().is_ok() {}
    rx
}

fn told_attention(rx: &mut mpsc::Receiver<ServerMsg>) -> bool {
    heard(rx)
        .iter()
        .any(|m| matches!(m, ServerMsg::Attention { .. }))
}

#[test]
fn only_the_window_a_person_used_last_is_told_a_pane_needs_it() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut first = viewer_as(&mut d, 40);
    let mut second = viewer_as(&mut d, 41);
    let typed = |d: &mut Daemon, id| {
        d.handle(Ev::Msg(
            id,
            ClientMsg::Input {
                term: t,
                data: b"x".to_vec(),
            },
        ))
    };
    typed(&mut d, 40);
    heard(&mut first);
    heard(&mut second);
    d.announce(t, Status::Blocked);
    let told_first = (told_attention(&mut first), told_attention(&mut second));
    typed(&mut d, 41);
    d.announce(t, Status::Blocked);
    let told_second = (told_attention(&mut first), told_attention(&mut second));
    close(&mut d, &[t]);
    assert_eq!(told_first, (true, false), "the window used last hears it");
    assert_eq!(told_second, (false, true), "and it follows the person");
}

#[test]
fn a_finished_alert_waits_for_the_check_and_rings_once_it_passes() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    checking(&mut d, t);
    d.set_status(t, Status::Done, "test");
    assert!(
        !heard(&mut seen).iter().any(|m| matches!(
            m,
            ServerMsg::Attention {
                status: Status::Done,
                ..
            }
        )),
        "not announced while checking"
    );
    // Even a deadline that is already due holds back.
    d.terms.get_mut(&t).unwrap().alert_due =
        Some((Status::Done, Instant::now() - Duration::from_millis(1)));
    d.settled_alerts();
    assert!(
        d.terms[&t].alerted.is_none() && d.terms[&t].alert_due.is_some(),
        "still held"
    );
    check_ends(&mut d, t, check::CheckOutcome::Exited(0));
    assert!(
        heard(&mut seen).iter().any(|m| matches!(
            m,
            ServerMsg::Attention {
                status: Status::Done,
                ..
            }
        )),
        "announced once it passed"
    );
    d.clients.clear();
    overdue(&mut d, t);
    d.settled_alerts();
    assert_eq!(d.terms[&t].alerted.map(|(s, _)| s), Some(Status::Done));
    close(&mut d, &[t]);
}

const SCREEN_RECOVERY: &str = "screen: explicit idle prompt recovered a missed turn end";

fn told_done(rx: &mut mpsc::Receiver<ServerMsg>) -> bool {
    heard(rx).iter().any(|m| {
        matches!(
            m,
            ServerMsg::Attention {
                status: Status::Done,
                ..
            }
        )
    })
}

#[test]
fn a_screen_finish_seconds_after_working_began_is_not_announced() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    d.set_status(t, Status::Done, SCREEN_RECOVERY);
    assert_eq!(
        d.terms[&t].status,
        Status::Done,
        "the status still shows it"
    );
    assert!(
        !told_done(&mut seen),
        "but no window is told, so nothing rings"
    );
    assert!(d.terms[&t].alert_due.is_none());
    close(&mut d, &[t]);
}

#[test]
fn a_turn_you_asked_for_forgets_a_finish_but_one_the_agent_began_itself_does_not() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    // (what was said, whether you typed since, whether it is still remembered)
    for (said, typed, kept) in [
        (Status::Blocked, true, true),
        (Status::Done, false, true),
        (Status::Done, true, false),
    ] {
        d.set_status(t, Status::Idle, "test");
        let said_at = Instant::now();
        let pane = d.terms.get_mut(&t).unwrap();
        pane.alerted = Some((said, said_at));
        pane.last_input = if typed {
            said_at + Duration::from_millis(1)
        } else {
            said_at - Duration::from_secs(5)
        };
        d.set_status(t, Status::Working, "test");
        assert_eq!(
            d.terms[&t].alerted.is_some(),
            kept,
            "{said:?}, typed since: {typed}: only the finish of something you asked for is news again"
        );
    }
    close(&mut d, &[t]);
}

/// The pane finishes and stays finished long enough for the windows to say it.
fn finishes_and_settles(d: &mut Daemon, t: TermId) {
    d.terms.get_mut(&t).unwrap().status_since = term::unix_now() - 60;
    d.set_status(t, Status::Done, "test");
    // Nothing waits to be said when the finish was not announced.
    if d.terms[&t].alert_due.is_some() {
        overdue(d, t);
    }
    d.settled_alerts();
}

#[test]
fn an_agent_that_picks_itself_back_up_and_finishes_again_is_not_said_twice() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    d.terms.get_mut(&t).unwrap().last_input = Instant::now() - Duration::from_secs(30);
    finishes_and_settles(&mut d, t);
    assert!(told_done(&mut seen), "the finish of what you asked is said");

    // A background job reports in: it works a little more and stops again. Nobody typed.
    d.set_status(t, Status::Working, "test");
    finishes_and_settles(&mut d, t);
    assert!(
        !told_done(&mut seen),
        "the same work finishing again rings nothing"
    );

    // You ask for something else: its finish is news.
    d.terms.get_mut(&t).unwrap().last_input = Instant::now();
    d.set_status(t, Status::Working, "test");
    finishes_and_settles(&mut d, t);
    assert!(told_done(&mut seen), "a new request's finish is said");
    close(&mut d, &[t]);
}

#[test]
fn a_finish_you_looked_at_straight_away_still_counts_as_said() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    d.terms.get_mut(&t).unwrap().last_input = Instant::now() - Duration::from_secs(30);
    d.terms.get_mut(&t).unwrap().status_since = term::unix_now() - 60;
    d.set_status(t, Status::Done, "test");
    assert!(told_done(&mut seen), "the finish of what you asked is said");
    // You were watching it: seen before the desktop alert's wait was over.
    d.set_status(t, Status::Idle, "you looked at it");

    // A background job reports in: it works a little more and stops again. Nobody typed.
    d.set_status(t, Status::Working, "test");
    finishes_and_settles(&mut d, t);
    assert!(
        !told_done(&mut seen),
        "the window already heard this work finish: looking at it is not a reason to ring again"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_finish_that_did_not_last_is_not_counted_as_said() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    d.terms.get_mut(&t).unwrap().last_input = Instant::now() - Duration::from_secs(30);
    d.terms.get_mut(&t).unwrap().status_since = term::unix_now() - 60;
    // It stops and carries straight on, before a window would have rung.
    d.set_status(t, Status::Done, "test");
    d.set_status(t, Status::Working, "test");
    heard(&mut seen);
    finishes_and_settles(&mut d, t);
    assert!(
        told_done(&mut seen),
        "the finish that stands is the first one anybody hears"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_screen_finish_after_a_real_turn_is_announced() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    d.terms.get_mut(&t).unwrap().status_since = term::unix_now() - 60;
    d.set_status(t, Status::Done, SCREEN_RECOVERY);
    assert!(told_done(&mut seen));
    close(&mut d, &[t]);
}

#[test]
fn a_failed_check_raises_its_own_alert_and_never_a_finished_one() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    checking(&mut d, t);
    d.set_status(t, Status::Done, "test");
    check_ends(&mut d, t, check::CheckOutcome::Exited(1));
    let got = heard(&mut seen);
    assert!(got.iter().any(
        |m| matches!(m, ServerMsg::Attention { term, status: Status::Blocked } if *term == t)
    ));
    assert!(
        got.iter()
            .any(|m| matches!(m, ServerMsg::Notice(n) if n.contains("check failed")))
    );
    assert!(!got.iter().any(|m| matches!(
        m,
        ServerMsg::Attention {
            status: Status::Done,
            ..
        }
    )));
    assert!(
        d.terms[&t].alert_due.is_none(),
        "no finished alert is left to ring"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_check_that_could_not_run_gives_a_notice_and_the_session_finishes() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    checking(&mut d, t);
    d.set_status(t, Status::Done, "test");
    check_ends(&mut d, t, check::CheckOutcome::SpawnFailed);
    let got = heard(&mut seen);
    assert!(
        got.iter()
            .any(|m| matches!(m, ServerMsg::Notice(n) if n.contains("could not run")))
    );
    assert!(got.iter().any(|m| matches!(
        m,
        ServerMsg::Attention {
            status: Status::Done,
            ..
        }
    )));
    assert!(d.terms[&t].alert_due.is_some());
    close(&mut d, &[t]);
}

#[test]
fn a_result_for_a_session_that_went_back_to_work_says_nothing() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    checking(&mut d, t);
    d.set_status(t, Status::Done, "test");
    d.set_status(t, Status::Working, "test");
    heard(&mut seen);
    check_ends(&mut d, t, check::CheckOutcome::Exited(1));
    assert!(heard(&mut seen).is_empty() && d.terms[&t].alert_due.is_none());
    close(&mut d, &[t]);
}

#[test]
fn a_session_with_no_check_announces_its_finish_at_once() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    d.set_status(t, Status::Done, "test");
    assert!(heard(&mut seen).iter().any(|m| matches!(
        m,
        ServerMsg::Attention {
            status: Status::Done,
            ..
        }
    )));
    d.clients.clear();
    overdue(&mut d, t);
    d.settled_alerts();
    assert_eq!(d.terms[&t].alerted.map(|(s, _)| s), Some(Status::Done));
    close(&mut d, &[t]);
}

fn claude_reports(
    d: &mut Daemon,
    t: TermId,
) -> impl FnMut(&mut Daemon, HookStatus, &str, &[&str]) + use<> {
    let secret = d.terms[&t].token.clone();
    move |d: &mut Daemon, status, event: &str, background: &[&str]| {
        let mut msg = report(t, &secret, 0, status, event);
        if let ClientMsg::Hook { background: b, .. } = &mut msg {
            *b = background.iter().map(|id| id.to_string()).collect();
        }
        d.apply_hook(msg, Some(true), Vec::new())
    }
}

#[test]
fn a_turn_paused_on_background_work_does_not_ring_finished() {
    let (mut d, _rx) = daemon();
    d.cfg.notify.phone_topic = concat!(env!("APP_COMMAND"), "-test-topic").into();
    d.cfg.notify.phone_done = true;
    d.cfg.notify.phone_after = 0;
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    let mut seen = viewer(&mut d);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["ci-watch"]);
    assert_eq!(
        d.terms[&t].status,
        Status::Done,
        "Claude itself shows the turn done, with a shell still running"
    );
    assert!(
        !announced(&heard(&mut seen), Status::Done),
        "no finished sound while it waits on its own work"
    );
    assert!(
        d.terms[&t].alert_due.is_none(),
        "no desktop notification queued"
    );
    d.phone_alerts();
    assert_eq!(d.terms[&t].phoned, 0, "no phone alert");
    // The shell finished and Claude woke by itself to carry on.
    say(&mut d, HookStatus::Working, "PreToolUse", &[]);
    say(&mut d, HookStatus::Done, "Stop", &[]);
    assert!(
        announced(&heard(&mut seen), Status::Done),
        "the real end of the work rings"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_background_task_that_never_ends_does_not_silence_later_turns() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    let mut seen = viewer(&mut d);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["dev-server"]);
    assert!(
        !announced(&heard(&mut seen), Status::Done),
        "the turn that started it is read as paused"
    );
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["dev-server"]);
    assert!(
        announced(&heard(&mut seen), Status::Done),
        "the same server still running doesn't hold the next turn"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_wake_up_with_an_older_task_still_running_rings() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    let mut seen = viewer(&mut d);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["b1", "b2"]);
    assert!(
        !announced(&heard(&mut seen), Status::Done),
        "both are new: paused"
    );
    // b1 ended and woke it; b2 was already running at the last stop.
    say(&mut d, HookStatus::Working, "PreToolUse", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["b2"]);
    assert!(announced(&heard(&mut seen), Status::Done));
    close(&mut d, &[t]);
}

#[test]
fn a_wake_up_with_a_helper_agent_still_running_stays_paused() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    let mut seen = viewer(&mut d);
    let say = |d: &mut Daemon, status, event: &str, agents: &[&str]| {
        let mut msg = report(t, &secret, 0, status, event);
        if let ClientMsg::Hook {
            background,
            background_agents,
            ..
        } = &mut msg
        {
            *background = agents.iter().map(|id| id.to_string()).collect();
            *background_agents = background.clone();
        }
        d.apply_hook(msg, Some(true), Vec::new())
    };
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["a1", "a2"]);
    assert!(!announced(&heard(&mut seen), Status::Done), "both are new: paused");
    // a1 reported in and woke it; a2 is a helper that will end and wake it again, unlike a
    // shell that may be a server.
    say(&mut d, HookStatus::Working, "PreToolUse", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["a2"]);
    assert!(
        !announced(&heard(&mut seen), Status::Done),
        "a helper is still running: the work is not finished"
    );
    say(&mut d, HookStatus::Working, "PreToolUse", &[]);
    say(&mut d, HookStatus::Done, "Stop", &[]);
    assert!(
        announced(&heard(&mut seen), Status::Done),
        "the last helper is in: the real end rings"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_paused_turn_is_checked_only_when_the_work_really_ends() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    let before = d.terms[&t].checks.turn;
    say(&mut d, HookStatus::Done, "Stop", &["ci-watch"]);
    assert_eq!(
        d.terms[&t].checks.turn, before,
        "no check while it waits on its own work"
    );
    say(&mut d, HookStatus::Working, "PreToolUse", &[]);
    say(&mut d, HookStatus::Done, "Stop", &[]);
    assert_eq!(
        d.terms[&t].checks.turn,
        before + 1,
        "checked at the real end"
    );
    close(&mut d, &[t]);
}

fn kept_after(d: &Daemon) -> usize {
    d.pane_snapshots
        .iter()
        .filter(|why| *why == "after claude's turn")
        .count()
}

#[test]
fn a_turn_is_kept_once_when_it_ends_not_while_it_waits_for_you() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    assert_eq!(d.pane_snapshots, ["before claude's turn"]);
    d.set_status(t, Status::Blocked, "hook: PermissionRequest");
    say(&mut d, HookStatus::Working, "PreToolUse", &[]);
    assert_eq!(kept_after(&d), 0, "an approval mid-turn is not its end");
    say(&mut d, HookStatus::Done, "Stop", &[]);
    say(&mut d, HookStatus::Done, "Stop", &[]);
    assert_eq!(kept_after(&d), 1);
    assert_eq!(d.pane_snapshots.last().unwrap(), "after claude's turn");
    close(&mut d, &[t]);
}

#[test]
fn a_paused_turn_is_kept_only_when_the_work_really_ends() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["ci-watch"]);
    assert_eq!(kept_after(&d), 0, "it is still waiting on its own work");
    say(&mut d, HookStatus::Working, "PreToolUse", &[]);
    say(&mut d, HookStatus::Done, "Stop", &[]);
    assert_eq!(kept_after(&d), 1);
    close(&mut d, &[t]);
}

#[test]
fn a_cancelled_turn_is_kept_as_it_was_left() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    d.set_status(t, Status::Idle, super::status::CANCELLED_WHY);
    assert_eq!(kept_after(&d), 1);
    say(&mut d, HookStatus::Done, "Stop", &[]);
    assert_eq!(kept_after(&d), 1, "a late turn end is the same turn");
    close(&mut d, &[t]);
}

#[test]
fn a_finish_guessed_from_a_title_blink_is_not_kept() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.terms.get_mut(&t).unwrap().agent = Some("claude".into());
    d.set_status(t, Status::Working, "screen: title says working");
    d.set_status(t, Status::Done, "screen: idle prompt");
    assert_eq!(kept_after(&d), 0);
    close(&mut d, &[t]);
}

#[test]
fn working_again_by_any_route_ends_the_pause() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["helper"]);
    assert!(d.terms[&t].awaiting_background);
    d.set_status(t, Status::Working, "one of its own agents is still running");
    assert!(
        !d.terms[&t].awaiting_background,
        "a later finish is news again"
    );
    close(&mut d, &[t]);
}

#[test]
fn desktop_and_phone_alerts_name_the_branch_of_a_worktree() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let p = d.terms.get_mut(&t).unwrap();
    p.head = Some(crate::gitfs::Head {
        top: std::path::PathBuf::from("/code/shop-api-worktrees/grey-wolf"),
        branch: "fix/early-alerts".into(),
        linked: true,
        main_root: std::path::PathBuf::from("/code/shop-api"),
    });
    assert_eq!(p.alert_place(), "shop-api · fix/early-alerts");
    p.head = None;
    p.cwd = std::path::PathBuf::from("/home/me/notes");
    assert_eq!(p.alert_place(), "notes", "no repository: the folder");
    close(&mut d, &[t]);
}

#[test]
fn a_wake_up_that_only_talks_still_rings_when_it_ends() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let mut say = claude_reports(&mut d, t);
    let mut seen = viewer(&mut d);
    say(&mut d, HookStatus::Working, "UserPromptSubmit", &[]);
    say(&mut d, HookStatus::Done, "Stop", &["ci-watch"]);
    assert!(!announced(&heard(&mut seen), Status::Done));
    // A wake-up turn with no tool call sends no working report before its stop.
    say(&mut d, HookStatus::Done, "Stop", &[]);
    assert!(
        announced(&heard(&mut seen), Status::Done),
        "finished once nothing is left in flight"
    );
    close(&mut d, &[t]);
}

#[test]
fn the_phone_hears_nothing_while_a_check_runs_or_after_it_fails() {
    let (mut d, _rx) = daemon();
    d.cfg.notify.phone_topic = concat!(env!("APP_COMMAND"), "-test-topic").into();
    d.cfg.notify.phone_done = true;
    d.cfg.notify.phone_after = 0;
    let t = working_agent(&mut d);
    checking(&mut d, t);
    d.set_status(t, Status::Done, "test");
    d.phone_alerts();
    assert_eq!(d.terms[&t].phoned, 0, "held while checking");
    check_ends(&mut d, t, check::CheckOutcome::Exited(1));
    d.phone_alerts();
    assert_eq!(d.terms[&t].phoned, 0, "a failed check is not a finish");
    close(&mut d, &[t]);
}

#[test]
fn a_queued_follow_up_still_sends_when_the_turn_ends_during_a_check() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    d.terms
        .get_mut(&t)
        .unwrap()
        .queue
        .push_back("and the docs".into());
    checking(&mut d, t);
    d.set_status(t, Status::Done, "test");
    d.update_statuses();
    assert!(
        d.terms[&t].queue.is_empty() && d.terms[&t].queue_sent.is_some(),
        "sent as today"
    );
    close(&mut d, &[t]);
}

// ---- several checkpoints and stale passes ---------------------------------------------

fn checkpoint_def(project: &std::path::Path, name: &str, command: &str) -> crate::config::CheckDef {
    crate::config::CheckDef {
        project: project.to_path_buf(),
        name: Some(name.into()),
        command: command.into(),
        timeout_s: LONG_CHECK_S,
    }
}

/// An agent at work in `dir`, whose project names checkpoints `(name, command)` in order.
fn agent_with_checkpoints(
    d: &mut Daemon,
    dir: &std::path::Path,
    points: &[(&str, &str)],
) -> TermId {
    d.cfg.check = points
        .iter()
        .map(|(name, command)| checkpoint_def(dir, name, command))
        .collect();
    let t = d.spawn(None, dir, 80, 24).expect("a shell pane");
    let p = d.terms.get_mut(&t).unwrap();
    (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Working);
    t
}

/// Apply a run's events as the daemon loop would, until none of `t`'s checkpoints runs.
fn finish_run(d: &mut Daemon, rx: &mut mpsc::Receiver<Ev>, t: TermId) {
    let until = Instant::now() + SLOW_CHECK_WAIT;
    while d.terms[&t].checks.running() {
        assert!(Instant::now() < until, "the run did not finish");
        match rx.try_recv() {
            Ok(ev @ (Ev::ChecksResolved { .. } | Ev::CheckRan { .. })) => d.handle(ev),
            Ok(_) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

/// Look for `t`'s repo checks file and apply the answer, as the git poll would.
fn look_for_repo_checks(d: &mut Daemon, rx: &mut mpsc::Receiver<Ev>, t: TermId) {
    d.probe_repo_checks([t]);
    let until = Instant::now() + SLOW_CHECK_WAIT;
    loop {
        assert!(Instant::now() < until, "the worktree was not looked at");
        match rx.try_recv() {
            Ok(ev @ Ev::RepoChecksFound(_)) => break d.handle(ev),
            Ok(_) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn states(d: &Daemon, t: TermId) -> Vec<(String, CheckState)> {
    d.terms[&t]
        .checks
        .points
        .iter()
        .map(|p| (p.latest.info.name.clone(), p.latest.info.state))
        .collect()
}

fn named_states(pairs: &[(&str, CheckState)]) -> Vec<(String, CheckState)> {
    pairs.iter().map(|(n, s)| (n.to_string(), *s)).collect()
}

fn point(d: &Daemon, t: TermId, name: &str) -> CheckInfo {
    d.terms[&t]
        .checks
        .named(name)
        .expect("the checkpoint")
        .latest
        .info
        .clone()
}

fn announced(seen: &[ServerMsg], status: Status) -> bool {
    seen.iter()
        .any(|m| matches!(m, ServerMsg::Attention { status: s, .. } if *s == status))
}

fn noticed(seen: &[ServerMsg], text: &str) -> usize {
    seen.iter()
        .filter(|m| matches!(m, ServerMsg::Notice(n) if n.contains(text)))
        .count()
}

#[test]
fn two_checkpoints_that_pass_run_in_order_and_finish_the_turn() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = agent_with_checkpoints(
        &mut d,
        &std::env::temp_dir(),
        &[
            ("build", "echo built; exit 0"),
            ("test", "echo tested; exit 0"),
        ],
    );
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    assert_eq!(
        states(&d, t),
        named_states(&[
            ("build", CheckState::Checking),
            ("test", CheckState::Pending)
        ])
    );
    assert!(
        !announced(&heard(&mut seen), Status::Done),
        "held while they run"
    );
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        states(&d, t),
        named_states(&[("build", CheckState::Passed), ("test", CheckState::Passed)])
    );
    assert!(
        announced(&heard(&mut seen), Status::Done),
        "done once both passed"
    );
    let test = point(&d, t, "test");
    assert_eq!(
        (test.turn, test.cwd.as_path(), test.line_count),
        (1, std::env::temp_dir().as_path(), 1)
    );
    assert_eq!(
        d.check_output(t, Some("test")).map(str::trim),
        Some("tested")
    );
    assert_eq!(
        d.check_output(t, Some("build")).map(str::trim),
        Some("built")
    );
    assert_eq!(d.snapshot().terms[&t].checks.len(), 2, "clients see both");
    close(&mut d, &[t]);
}

#[test]
fn a_failed_checkpoint_does_not_stop_the_next_and_the_turn_reads_failed() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = agent_with_checkpoints(
        &mut d,
        &std::env::temp_dir(),
        &[
            ("build", "echo \"FAIL build\"; exit 2"),
            ("test", "echo tested; exit 0"),
        ],
    );
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        states(&d, t),
        named_states(&[("build", CheckState::Failed), ("test", CheckState::Passed)])
    );
    assert_eq!(
        (
            point(&d, t, "build").exit_code,
            point(&d, t, "build").first_failing_line.as_str()
        ),
        (Some(2), "FAIL build")
    );
    let got = heard(&mut seen);
    assert!(
        announced(&got, Status::Blocked) && !announced(&got, Status::Done),
        "a failure, never a finish"
    );
    assert_eq!(noticed(&got, "check failed"), 1);
    assert_eq!(
        d.snapshot().terms[&t].check().map(|c| c.name.as_str()),
        Some("build"),
        "the failure is the headline"
    );
    assert!(d.terms[&t].check_holds_done());
    close(&mut d, &[t]);
}

/// `build` runs; `test` waits for your approval of a changed command; `lint` you said never to.
fn gated(
    user: &[crate::config::CheckDef],
    project: &std::path::Path,
    worktree: &std::path::Path,
    approvals: &approvals::Approvals,
) -> Vec<check_source::Resolved> {
    let mut all = check_source::resolve(user, project, worktree, approvals);
    for r in &mut all {
        r.gate = match r.def.name() {
            "test" => check_source::Gate::NeedsApproval {
                previous: Some("npm t".into()),
            },
            "lint" => check_source::Gate::Never,
            _ => check_source::Gate::Run,
        };
    }
    all
}

fn never_any(
    user: &[crate::config::CheckDef],
    project: &std::path::Path,
    worktree: &std::path::Path,
    approvals: &approvals::Approvals,
) -> Vec<check_source::Resolved> {
    let mut all = check_source::resolve(user, project, worktree, approvals);
    for r in &mut all {
        r.gate = check_source::Gate::Never;
    }
    all
}

#[test]
fn a_checkpoint_waiting_for_approval_is_not_run_and_asks_for_you_once_a_turn() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    d.resolver = gated;
    let points = [
        ("build", "echo built; exit 0"),
        ("test", "npm test"),
        ("lint", "npm run lint"),
    ];
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &points);
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        states(&d, t),
        named_states(&[
            ("build", CheckState::Passed),
            ("test", CheckState::NeedsApproval),
            ("lint", CheckState::Never)
        ])
    );
    assert_eq!(
        point(&d, t, "test").previous_command.as_deref(),
        Some("npm t")
    );
    assert_eq!(d.check_output(t, Some("test")), Some(""), "it never ran");
    let got = heard(&mut seen);
    assert!(announced(&got, Status::Blocked) && !announced(&got, Status::Done));
    assert_eq!(noticed(&got, "approve check: npm test"), 1);
    assert!(d.terms[&t].check_holds_done(), "not done until you decide");
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    d.command(
        0,
        Command::RerunCheck {
            term: t,
            name: Some("build".into()),
        },
    )
    .unwrap();
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        noticed(&heard(&mut seen), "approve check"),
        0,
        "said once a turn"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_checkpoint_you_said_never_to_is_not_run_and_the_turn_finishes() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    d.resolver = never_any;
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &[("test", "npm test")]);
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("test", CheckState::Never)]));
    let got = heard(&mut seen);
    assert!(announced(&got, Status::Done), "it finishes");
    assert_eq!(noticed(&got, "not checked: never for this repo"), 1);
    close(&mut d, &[t]);
}

#[test]
fn a_rerun_by_name_runs_only_that_checkpoint() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = agent_with_checkpoints(
        &mut d,
        &std::env::temp_dir(),
        &[
            ("build", "echo built; exit 0"),
            ("test", "echo tested; exit 0"),
        ],
    );
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    let build = point(&d, t, "build");
    assert!(
        d.command(
            0,
            Command::RerunCheck {
                term: t,
                name: Some("lint".into())
            }
        )
        .is_err(),
        "no such checkpoint"
    );
    d.command(
        0,
        Command::RerunCheck {
            term: t,
            name: Some("test".into()),
        },
    )
    .unwrap();
    assert_eq!(
        states(&d, t),
        named_states(&[
            ("build", CheckState::Passed),
            ("test", CheckState::Checking)
        ])
    );
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        states(&d, t),
        named_states(&[("build", CheckState::Passed), ("test", CheckState::Passed)])
    );
    assert_eq!(point(&d, t, "build"), build, "the other is left as it was");
    assert_eq!(
        point(&d, t, "test").turn,
        1,
        "a re-run checks the same turn"
    );
    close(&mut d, &[t]);
}

#[test]
fn an_override_by_name_acts_on_that_checkpoint_and_says_who() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = agent_with_checkpoints(
        &mut d,
        &std::env::temp_dir(),
        &[("build", "exit 1"), ("test", "exit 1")],
    );
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    d.command(
        0,
        Command::OverrideCheck {
            term: t,
            name: Some("test".into()),
            reason: "flaky".into(),
        },
    )
    .unwrap();
    assert_eq!(
        states(&d, t),
        named_states(&[
            ("build", CheckState::Failed),
            ("test", CheckState::Overridden)
        ])
    );
    assert_eq!(point(&d, t, "test").overridden_by, "you");
    let reviewer = pane(&mut d);
    d.terms.get_mut(&reviewer).unwrap().label = "reviewer".into();
    d.terms.get_mut(&reviewer).unwrap().grants = Some(vec!["write".into()]);
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(
        7,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let token = d.terms[&reviewer].token.clone();
    d.handle(Ev::From(7, reviewer, token));
    d.message(
        7,
        ClientMsg::Command(Command::OverrideCheck {
            term: t,
            name: None,
            reason: String::new(),
        }),
    );
    assert!(matches!(out.try_recv(), Ok(ServerMsg::Reply(Reply::Ok))));
    assert_eq!(
        point(&d, t, "build").overridden_by,
        "reviewer",
        "None is the headline, and a pane signs with its name"
    );
    d.command(
        0,
        Command::UndoOverride {
            term: t,
            name: Some("test".into()),
        },
    )
    .unwrap();
    assert_eq!(
        states(&d, t),
        named_states(&[
            ("build", CheckState::Overridden),
            ("test", CheckState::Failed)
        ])
    );
    close(&mut d, &[t, reviewer]);
}

#[test]
fn a_restart_restores_each_checkpoint_with_its_own_output() {
    let (mut d, _rx) = daemon();
    let tails = persist::scratch_dir();
    d.check_dir = tails.path().to_path_buf();
    let old: TermId = 9_401;
    let tail = |name: &str, slot, text: &str| persist::CheckTail {
        term: old,
        name: name.into(),
        slot,
        text: text.into(),
    };
    let written = [
        tail("build", persist::CheckSlot::Latest, "build out"),
        tail("build", persist::CheckSlot::Overridden, "build failed"),
        tail("test", persist::CheckSlot::Latest, ""),
    ];
    assert!(
        persist::save_check_outputs(tails.path(), persist::save_ticket(), &written, &[old])
            .is_empty()
    );
    let info = |name: &str, state| CheckInfo {
        name: name.into(),
        command: format!("make {name}"),
        state,
        turn: 3,
        ..Default::default()
    };
    let saved = persist::SavedPane {
        checks: vec![
            info("build", CheckState::Overridden),
            info("test", CheckState::Pending),
        ],
        checks_overridden: vec![info("build", CheckState::Overridden)],
        check_trees: [("build".to_string(), "4b825dc".to_string())].into(),
        ..Default::default()
    };
    let t = pane(&mut d);
    let dir = d.check_dir.clone();
    d.terms
        .get_mut(&t)
        .unwrap()
        .restore_checks(&dir, old, &saved);
    assert_eq!(
        states(&d, t),
        named_states(&[
            ("build", CheckState::Overridden),
            ("test", CheckState::CouldNotRun)
        ]),
        "a waiting one died with the server"
    );
    let checks = &d.terms[&t].checks;
    assert_eq!(checks.named("build").unwrap().latest.output, "build out");
    assert_eq!(
        checks
            .named("build")
            .unwrap()
            .overridden
            .as_ref()
            .map(|o| o.output.as_str()),
        Some("build failed")
    );
    assert!(checks.named("test").unwrap().overridden.is_none());
    assert_eq!(checks.turn, 3, "the turn count carries on");
    assert_eq!(
        checks.named("build").unwrap().tree.as_deref(),
        Some("4b825dc"),
        "its pass can still go stale"
    );
    let mut written = d.take_dirty_check_outputs();
    written.sort_by(|a, b| (&a.name, a.slot as u8).cmp(&(&b.name, b.slot as u8)));
    assert_eq!(
        written
            .iter()
            .map(|w| (w.name.as_str(), w.text.as_str()))
            .collect::<Vec<_>>(),
        [
            ("build", "build out"),
            ("build", "build failed"),
            ("test", "")
        ]
    );
    close(&mut d, &[t]);
}

#[test]
fn a_checkpoint_never_sees_the_panes_token() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    // SAFETY: the daemon's own environment, as when it was started from inside a pane; the
    // panes it spawns set their own token.
    unsafe {
        std::env::set_var(
            concat!(env!("APP_ENV"), "_PANE_TOKEN"),
            concat!(env!("APP_COMMAND"), "-test-secret"),
        )
    };
    let print = if cfg!(windows) {
        concat!("echo \"token=$env:", env!("APP_ENV"), "_PANE_TOKEN\"")
    } else {
        concat!("echo \"token=$", env!("APP_ENV"), "_PANE_TOKEN\"")
    };
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &[("env", print)]);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    assert_eq!(point(&d, t, "env").state, CheckState::Passed);
    assert_eq!(
        d.check_output(t, Some("env")).map(str::trim),
        Some("token=")
    );
    close(&mut d, &[t]);
}

/// A repository of its own: one committed file, and a file it ignores.
fn temp_repo() -> persist::ScratchDir {
    let dir = persist::scratch_dir();
    std::fs::create_dir_all(dir.path()).unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "t"],
        &["config", "user.email", "t@t"],
        &["config", "core.autocrlf", "false"],
    ] {
        crate::proc::git(dir.path(), args).unwrap();
    }
    std::fs::write(dir.path().join("kept.txt"), "one\n").unwrap();
    std::fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
    crate::proc::git(dir.path(), &["add", "-A"]).unwrap();
    crate::proc::git(dir.path(), &["commit", "-qm", "first"]).unwrap();
    dir
}

/// The git poll's stale probe, run here instead of on its background task.
fn poll_stale(d: &mut Daemon) {
    let found: Vec<_> = d
        .stale_probes()
        .iter()
        .flat_map(check_stale::probe)
        .collect();
    if !found.is_empty() {
        d.handle(Ev::ChecksStale(found));
    }
}

#[test]
fn a_pass_goes_stale_when_the_files_change_and_a_failure_does_not() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let repo = temp_repo();
    let t = agent_with_checkpoints(
        &mut d,
        repo.path(),
        &[("build", "echo built; exit 0"), ("test", "exit 1")],
    );
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    poll_stale(&mut d);
    std::fs::write(repo.path().join("ignored.txt"), "local\n").unwrap();
    poll_stale(&mut d);
    assert_eq!(
        states(&d, t),
        named_states(&[("build", CheckState::Passed), ("test", CheckState::Failed)]),
        "nothing that counts changed"
    );
    let staged = crate::proc::git(repo.path(), &["diff", "--cached", "--name-only"]).unwrap();
    std::fs::write(repo.path().join("kept.txt"), "two\n").unwrap();
    std::fs::write(repo.path().join("new.txt"), "new\n").unwrap();
    d.terms.get_mut(&t).unwrap().status = Status::Working;
    poll_stale(&mut d);
    assert_eq!(
        point(&d, t, "build").state,
        CheckState::Passed,
        "not while the agent works"
    );
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    let before = term::unix_now();
    poll_stale(&mut d);
    assert_eq!(
        states(&d, t),
        named_states(&[("build", CheckState::Stale), ("test", CheckState::Failed)]),
        "only a pass goes stale"
    );
    let build = point(&d, t, "build");
    assert!(build.stale_since.is_some_and(|at| at >= before));
    let changed: Vec<_> = build
        .changed_files
        .iter()
        .map(|f| (f.kind, f.path.as_str()))
        .collect();
    assert_eq!(changed, [('M', "kept.txt"), ('A', "new.txt")]);
    assert_eq!(
        crate::proc::git(repo.path(), &["diff", "--cached", "--name-only"]).unwrap(),
        staged,
        "the real index is untouched"
    );
    d.command(
        0,
        Command::RerunCheck {
            term: t,
            name: Some("build".into()),
        },
    )
    .unwrap();
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        point(&d, t, "build").state,
        CheckState::Passed,
        "a re-run replaces it"
    );
    poll_stale(&mut d);
    assert_eq!(
        point(&d, t, "build").state,
        CheckState::Passed,
        "fresh against the files as they are"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_stale_pass_counts_as_finished_but_not_as_a_fresh_pass() {
    let mut checks = check::Checks::default();
    let info = CheckInfo {
        name: "test".into(),
        state: CheckState::Stale,
        ..Default::default()
    };
    checks.points.push(check::CheckRecord::new(info, "").into());
    assert!(!checks.holds_done() && !checks.running(), "finished");
    assert_ne!(
        checks.headline().map(|c| c.info.state),
        Some(CheckState::Passed)
    );
}

#[test]
fn outside_git_a_pass_never_goes_stale() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let dir = persist::scratch_dir();
    std::fs::create_dir_all(dir.path()).unwrap();
    let t = agent_with_checkpoints(&mut d, dir.path(), &[("build", "exit 0")]);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    std::fs::write(dir.path().join("x.txt"), "x").unwrap();
    assert!(d.stale_probes().is_empty(), "no tree was recorded");
    assert_eq!(point(&d, t, "build").state, CheckState::Passed);
    close(&mut d, &[t]);
}

/// A daemon whose approvals are kept in a scratch file (held by the returned guard), never
/// in your own data folder.
fn approving_daemon() -> (Daemon, mpsc::Receiver<Ev>, persist::ScratchDir) {
    let (mut d, rx) = daemon();
    let dir = persist::scratch_dir();
    d.approvals_file = dir.path().join("check-approvals.json");
    (d, rx, dir)
}

/// Whatever the checks are, `test` waits for your approval again: so an answer's re-run
/// starts and settles without running anything.
fn still_waiting(
    user: &[crate::config::CheckDef],
    project: &std::path::Path,
    worktree: &std::path::Path,
    approvals: &approvals::Approvals,
) -> Vec<check_source::Resolved> {
    let mut all = check_source::resolve(user, project, worktree, approvals);
    for r in &mut all {
        r.gate = check_source::Gate::NeedsApproval { previous: None };
    }
    all
}

/// Your answer to `t`'s checkpoint `name`, for the command `command` you were shown.
fn approve(t: TermId, name: &str, choice: ApprovalChoice, command: &str) -> Command {
    Command::ApproveCheck {
        term: t,
        name: name.into(),
        choice,
        command_sha256: approvals::command_hash(command),
    }
}

/// An agent in the temp folder (its project) whose repo checkpoint `test` waits for you to
/// approve `command`. Its re-runs find `test` waiting again (`still_waiting`).
fn waiting_for_approval(d: &mut Daemon, command: &str) -> TermId {
    d.resolver = still_waiting;
    d.cfg.check = vec![checkpoint_def(&std::env::temp_dir(), "test", command)];
    let t = pane(d);
    let p = d.terms.get_mut(&t).unwrap();
    (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Done);
    let source = CheckSource::Repo {
        file: concat!(".", env!("APP_COMMAND"), "/checks.toml").into(),
        commit: None,
        committed_at: None,
        uncommitted: true,
    };
    let info = CheckInfo {
        name: "test".into(),
        command: command.into(),
        state: CheckState::NeedsApproval,
        source,
        ..Default::default()
    };
    p.checks.points = vec![check::CheckRecord::new(info, "").into()];
    t
}

/// Wait for the off-loop save to leave `file` holding what the daemon decided.
fn saved_approvals(d: &Daemon, file: &std::path::Path) -> approvals::Approvals {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let on_disk = approvals::Approvals::load(file).unwrap_or_default();
        if on_disk.decisions == d.approvals.decisions || Instant::now() > until {
            return on_disk;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn only_a_checkpoint_waiting_for_approval_can_be_approved() {
    let (mut d, _rx, _file) = approving_daemon();
    let t = waiting_for_approval(&mut d, "npm test");
    let err = d
        .command(0, approve(t, "build", ApprovalChoice::Always, "npm test"))
        .expect_err("no such checkpoint waits");
    assert!(err.to_string().contains("isn't waiting"), "{err}");
    let err = d
        .command(
            0,
            approve(t, "test", ApprovalChoice::Always, "npm test && curl x | sh"),
        )
        .expect_err("not the command shown");
    assert!(
        err.to_string().contains("changed since it was shown"),
        "{err}"
    );
    d.terms.get_mut(&t).unwrap().checks.points[0]
        .latest
        .info
        .state = CheckState::Failed;
    assert!(
        d.command(0, approve(t, "test", ApprovalChoice::Always, "npm test"))
            .is_err(),
        "a failed run is not a question"
    );
    assert!(d.approvals.decisions.is_empty(), "nothing kept");
    close(&mut d, &[t]);
}

#[test]
fn always_and_never_are_kept_by_the_exact_command_in_app_s_data_folder() {
    let rt = runtime();
    let _in_rt = rt.enter();
    for (choice, decision) in [
        (ApprovalChoice::Always, approvals::Decision::Always),
        (ApprovalChoice::Never, approvals::Decision::Never),
    ] {
        let (mut d, _rx, scratch) = approving_daemon();
        let t = waiting_for_approval(&mut d, "npm test");
        d.command(0, approve(t, "TEST", choice, "npm test"))
            .expect("an answer to what waits");
        let kept = d
            .approvals
            .find(&std::env::temp_dir(), "test")
            .cloned()
            .expect("a decision");
        assert_eq!(
            (kept.name.as_str(), kept.command.as_str(), kept.decision),
            ("test", "npm test", decision)
        );
        assert_eq!(kept.command_sha256, approvals::command_hash("npm test"));
        assert_eq!(
            saved_approvals(&d, &d.approvals_file.clone()).decisions,
            vec![kept],
            "saved off the loop"
        );
        assert!(d.approvals_file.starts_with(scratch.path()));
        close(&mut d, &[t]);
    }
    let (d, _rx) = daemon();
    assert_eq!(
        d.approvals_file,
        approvals::path(),
        concat!(
            "outside every repo: ",
            env!("APP_NAME"),
            "'s own data folder"
        )
    );
    assert!(d.approvals_file.starts_with(crate::config::data_dir()));
}

#[test]
fn once_lets_the_next_run_of_that_pane_go_and_then_asks_again() {
    let (mut d, _rx, _file) = approving_daemon();
    let (t, other) = (
        waiting_for_approval(&mut d, "npm test"),
        waiting_for_approval(&mut d, "npm test"),
    );
    // While its agent works, the run the grant is for waits for the turn to end.
    d.terms.get_mut(&t).unwrap().status = Status::Working;
    d.command(0, approve(t, "test", ApprovalChoice::Once, "npm test"))
        .expect("an answer to what waits");
    assert!(d.approvals.decisions.is_empty(), "once is never remembered");
    assert!(
        !d.approvals_for_run(other).granted_once("test", "npm test"),
        "another pane must ask for itself"
    );
    assert!(
        d.approvals_for_run(t).granted_once("test", "npm test"),
        "this pane's next run may go"
    );
    assert!(
        !d.approvals_for_run(t).granted_once("test", "npm test"),
        "and only that run"
    );
    close(&mut d, &[t, other]);
}

#[test]
fn forgetting_a_decision_makes_the_next_run_ask_again() {
    let rt = runtime();
    let _in_rt = rt.enter();
    let (mut d, _rx, _file) = approving_daemon();
    let repo = std::env::temp_dir();
    d.approvals
        .decide(&repo, "test", "npm test", approvals::Decision::Never, 1);
    d.command(
        0,
        Command::ForgetCheckApproval {
            project: repo.clone(),
            name: "test".into(),
        },
    )
    .expect("yours to forget");
    assert_eq!(d.approvals.lookup(&repo, "test", "npm test"), None);
    assert!(
        saved_approvals(&d, &d.approvals_file.clone())
            .decisions
            .is_empty(),
        "and the file forgets it too"
    );
    let err = d
        .command(
            0,
            Command::ForgetCheckApproval {
                project: repo,
                name: "test".into(),
            },
        )
        .expect_err("nothing left");
    assert!(err.to_string().contains("nothing is remembered"), "{err}");
}

#[test]
fn a_repo_checkpoint_waits_at_the_turn_end_and_once_runs_it_one_time() {
    let (rt, (mut d, mut rx, _file)) = (runtime(), approving_daemon());
    let _in_rt = rt.enter();
    let repo = temp_repo();
    std::fs::create_dir_all(repo.path().join(concat!(".", env!("APP_COMMAND")))).unwrap();
    std::fs::write(
        repo.path().join(crate::config::REPO_CHECKS_FILE),
        "[[check]]\nname = \"test\"\ncommand = \"echo tested\"\n",
    )
    .unwrap();
    let t = agent_with_checkpoints(&mut d, repo.path(), &[]);
    look_for_repo_checks(&mut d, &mut rx, t);
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    assert!(
        d.terms[&t].check_running(),
        "the file is read off the loop; done waits for it"
    );
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        states(&d, t),
        named_states(&[("test", CheckState::NeedsApproval)])
    );
    assert!(announced(&heard(&mut seen), Status::Blocked));
    d.command(0, approve(t, "test", ApprovalChoice::Once, "echo tested"))
        .unwrap();
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("test", CheckState::Passed)]));
    assert_eq!(
        d.check_output(t, Some("test")).map(str::trim),
        Some("tested")
    );
    d.command(
        0,
        Command::RerunCheck {
            term: t,
            name: None,
        },
    )
    .unwrap();
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        states(&d, t),
        named_states(&[("test", CheckState::NeedsApproval)]),
        "once was one run"
    );
    close(&mut d, &[t]);
}

#[test]
fn approving_one_checkpoint_while_another_runs_runs_it_after_in_the_same_turn() {
    let (rt, (mut d, mut rx, _file)) = (runtime(), approving_daemon());
    let _in_rt = rt.enter();
    let repo = temp_repo();
    std::fs::create_dir_all(repo.path().join(concat!(".", env!("APP_COMMAND")))).unwrap();
    let file = "[[check]]\nname = \"build\"\ncommand = \"echo built\"\n\n[[check]]\nname = \"test\"\ncommand = \"echo tested\"\n";
    std::fs::write(repo.path().join(crate::config::REPO_CHECKS_FILE), file).unwrap();
    let t = agent_with_checkpoints(&mut d, repo.path(), &[]);
    look_for_repo_checks(&mut d, &mut rx, t);
    let project = d.terms[&t]
        .head
        .as_ref()
        .expect("in the repo")
        .main_root
        .clone();
    d.approvals.decide(
        &project,
        "build",
        "echo built",
        approvals::Decision::Always,
        1,
    );
    end_turn(&mut d, t);
    let until = Instant::now() + SLOW_CHECK_WAIT;
    loop {
        assert!(Instant::now() < until, "the checks were not resolved");
        match rx.try_recv() {
            Ok(ev @ Ev::ChecksResolved { .. }) => break d.handle(ev),
            Ok(_) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    assert_eq!(
        states(&d, t),
        named_states(&[
            ("build", CheckState::Checking),
            ("test", CheckState::NeedsApproval)
        ])
    );
    d.command(0, approve(t, "test", ApprovalChoice::Once, "echo tested"))
        .expect("approving while build runs");
    assert!(
        point(&d, t, "test").rerun_queued,
        "it runs once build is done"
    );
    finish_run(&mut d, &mut rx, t);
    assert_eq!(
        states(&d, t),
        named_states(&[("build", CheckState::Passed), ("test", CheckState::Passed)])
    );
    let test = point(&d, t, "test");
    assert_eq!(
        (test.turn, test.rerun_queued),
        (1, false),
        "in the same turn"
    );
    assert_eq!(
        d.check_output(t, Some("test")).map(str::trim),
        Some("tested")
    );
    close(&mut d, &[t]);
}

/// A claude pane in its own workspace, and a connection that proved it is that pane.
fn agent_pane_in(
    d: &mut Daemon,
    client: ClientId,
    dir: &std::path::Path,
) -> (TermId, mpsc::Receiver<ServerMsg>) {
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(dir.to_path_buf()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let term = d.workspaces.last().unwrap().tabs[0].focus;
    d.terms.get_mut(&term).unwrap().agent = Some("claude".into());
    let token = d.terms[&term].token.clone();
    (term, from_pane(d, client, term, &token))
}

/// The pane's connection asks to start a session running `cmd`; what the daemon answered.
fn helper_start(
    d: &mut Daemon,
    client: ClientId,
    c: &mut mpsc::Receiver<ServerMsg>,
    cmd: Option<&str>,
) -> Option<String> {
    let cwd = Some(std::env::temp_dir());
    d.handle(Ev::Msg(
        client,
        ClientMsg::Command(Command::NewWorkspace {
            cwd,
            name: None,
            cmd: cmd.map(String::from),
        }),
    ));
    std::iter::from_fn(|| c.try_recv().ok()).find_map(|m| {
        if let ServerMsg::Error(e) = m {
            Some(e)
        } else {
            None
        }
    })
}

fn answer_helpers(term: TermId, choice: HelperChoice) -> Command {
    Command::AnswerHelpers { term, choice }
}

#[test]
fn the_first_helper_start_asks_and_the_row_needs_you_until_you_answer() {
    let (mut d, _rx) = daemon();
    let (a, mut c) = agent_pane_in(&mut d, 1, &std::env::temp_dir());
    let panes = d.terms.len();
    // Nothing starts; the agent is told you were asked, and every task it names is listed once.
    let why = helper_start(&mut d, 1, &mut c, Some("claude 'fix the login test'"))
        .expect("refused for now");
    assert!(
        why.contains("needs the user's OK") && why.contains("1 helper"),
        "{why}"
    );
    helper_start(&mut d, 1, &mut c, Some("claude 'port the footer'"));
    helper_start(&mut d, 1, &mut c, Some("claude 'port the footer'"));
    assert_eq!(d.terms.len(), panes, "no session started");
    assert_eq!(
        d.terms[&a].helpers.asked,
        ["fix the login test", "port the footer"]
    );
    let row = &d.snapshot().terms[&a];
    assert_eq!(
        (row.status, row.helper_ask.len()),
        (Status::Blocked, 2),
        "needs you, whatever its hooks say"
    );
    assert_ne!(
        d.terms[&a].status,
        Status::Blocked,
        "an overlay: the agent's own status is untouched"
    );
    // The pane cannot answer for you, and the ask stays.
    d.handle(Ev::Msg(
        1,
        ClientMsg::Command(answer_helpers(a, HelperChoice::Allow)),
    ));
    let refused = std::iter::from_fn(|| c.try_recv().ok()).find_map(|m| {
        if let ServerMsg::Error(e) = m {
            Some(e)
        } else {
            None
        }
    });
    assert!(refused.is_some_and(|e| e.contains("yours to allow")));
    assert_eq!(d.terms[&a].helpers.asked.len(), 2);
    // You allow: it may start them, now and for the rest of its run.
    d.command(0, answer_helpers(a, HelperChoice::Allow))
        .unwrap();
    assert!(d.snapshot().terms[&a].helper_ask.is_empty() && !d.terms[&a].helpers.refused);
    assert!(helper_start(&mut d, 1, &mut c, None).is_none(), "allowed");
    assert_eq!(
        (d.terms.len(), d.terms[&newest(&d)].started_by),
        (panes + 1, Some(a))
    );
    // An answer to nothing is an error, not a grant.
    assert!(d.command(0, answer_helpers(a, HelperChoice::No)).is_err());
    assert!(
        d.command(0, answer_helpers(999, HelperChoice::Allow))
            .is_err(),
        "no such pane"
    );
    let all: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &all);
}

#[test]
fn no_means_it_does_the_work_alone() {
    let (mut d, _rx) = daemon();
    let (a, mut c) = agent_pane_in(&mut d, 1, &std::env::temp_dir());
    let panes = d.terms.len();
    helper_start(&mut d, 1, &mut c, Some("claude 'one'"));
    d.command(0, answer_helpers(a, HelperChoice::No)).unwrap();
    assert_eq!(
        d.snapshot().terms[&a].status,
        d.terms[&a].status,
        "the row is back to what the agent says"
    );
    let why = helper_start(&mut d, 1, &mut c, None).expect("refused");
    assert!(
        why.contains("said no helpers") && why.contains("Do the work yourself"),
        "{why}"
    );
    assert!(
        d.terms[&a].helpers.asked.is_empty(),
        "it is not asked about again"
    );
    assert_eq!(d.terms.len(), panes);
    close(&mut d, &[a]);
}

#[test]
fn always_in_this_project_is_remembered_for_its_sessions_and_across_restarts() {
    let rt = runtime();
    let _in_rt = rt.enter();
    let (mut d, _rx) = daemon();
    let scratch = persist::scratch_dir();
    d.helper_projects_file = scratch.path().join("helper-projects.json");
    let other = scratch.path().join("elsewhere");
    std::fs::create_dir_all(&other).unwrap();
    let (a, mut ca) = agent_pane_in(&mut d, 1, &std::env::temp_dir());
    let (b, mut cb) = agent_pane_in(&mut d, 2, &std::env::temp_dir());
    let (e, mut ce) = agent_pane_in(&mut d, 3, &other);
    helper_start(&mut d, 1, &mut ca, Some("claude 'one'"));
    d.command(0, answer_helpers(a, HelperChoice::Always))
        .unwrap();
    assert!(helper_start(&mut d, 1, &mut ca, None).is_none());
    assert!(
        helper_start(&mut d, 2, &mut cb, None).is_none(),
        "another session in the same project doesn't ask"
    );
    assert!(d.terms[&b].helpers.asked.is_empty());
    assert!(
        helper_start(&mut d, 3, &mut ce, Some("claude 'two'")).is_some(),
        "another project still asks"
    );
    assert_eq!(d.terms[&e].helpers.asked, ["two"]);
    // It is on disk, off the loop, by place not by spelling; a new daemon starts knowing it.
    let until = Instant::now() + Duration::from_secs(10);
    let saved = loop {
        let on_disk = helpers::Projects::load(&d.helper_projects_file).unwrap();
        if on_disk.allows(&std::env::temp_dir()) || Instant::now() > until {
            break on_disk;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        saved.allows(&std::env::temp_dir()) && !saved.allows(&other),
        "{saved:?}"
    );
    let (mut again, _rx2) = daemon();
    again.helper_projects = saved;
    let (_, mut c2) = agent_pane_in(&mut again, 1, &std::env::temp_dir());
    assert!(
        helper_start(&mut again, 1, &mut c2, None).is_none(),
        "remembered after a restart"
    );
    let all: Vec<TermId> = again.terms.keys().copied().collect();
    close(&mut again, &all);
    let all: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &all);
}

#[test]
fn the_setting_decides_always_lets_it_through_never_refuses_and_nothing_is_asked() {
    let (mut d, _rx) = daemon();
    let (a, mut c) = agent_pane_in(&mut d, 1, &std::env::temp_dir());
    let panes = d.terms.len();
    d.cfg.sessions.helpers = "never".into();
    let why = helper_start(&mut d, 1, &mut c, Some("claude 'one'")).expect("refused");
    assert!(
        why.contains("turned helpers off") && why.contains("Do the work yourself"),
        "{why}"
    );
    assert!(d.terms[&a].helpers.asked.is_empty() && d.terms.len() == panes);
    d.cfg.sessions.helpers = "always".into();
    assert!(helper_start(&mut d, 1, &mut c, None).is_none());
    assert_eq!(d.terms.len(), panes + 1);
    // A person's shell is not an agent asking for help: it starts what it likes.
    d.cfg.sessions.helpers = "never".into();
    d.terms.get_mut(&a).unwrap().agent = None;
    assert!(helper_start(&mut d, 1, &mut c, None).is_none());
    let all: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &all);
}

#[test]
fn an_asking_session_alerts_you_once_even_though_its_own_status_is_not_blocked() {
    let (mut d, _rx) = daemon();
    let (a, mut c) = agent_pane_in(&mut d, 1, &std::env::temp_dir());
    d.set_status(a, Status::Working, "test");
    helper_start(&mut d, 1, &mut c, Some("claude 'one'"));
    assert_eq!(
        d.terms[&a].alert_due.map(|(s, _)| s),
        Some(Status::Blocked),
        "the desktop alert is queued"
    );
    helper_start(&mut d, 1, &mut c, Some("claude 'two'"));
    let first = d.terms[&a].alert_due;
    helper_start(&mut d, 1, &mut c, Some("claude 'three'"));
    assert_eq!(
        d.terms[&a].alert_due, first,
        "asking again doesn't ring again"
    );
    close(&mut d, &[a]);
}

#[test]
fn the_activity_a_newer_app_reads_counts_working_and_waiting_sessions() {
    let (mut d, _rx) = daemon();
    let (a, b, c) = (pane(&mut d), pane(&mut d), pane(&mut d));
    d.terms.get_mut(&a).unwrap().status = Status::Working;
    d.terms.get_mut(&b).unwrap().status = Status::Blocked;
    d.terms.get_mut(&c).unwrap().status = Status::Idle;
    let seen = d.activity();
    assert_eq!(
        (seen.sessions, seen.working, seen.waiting, seen.busy()),
        (3, 1, 1, 2)
    );
    assert_eq!(
        (seen.pid, seen.version.as_str()),
        (std::process::id(), env!("APP_VERSION"))
    );
    close(&mut d, &[a, b, c]);
}

#[test]
fn an_agent_saved_mid_turn_comes_back_as_a_resume_of_its_conversation() {
    let (d, _rx) = daemon();
    // What the file holds when the server stops for an update: the agent and its session,
    // whatever it was doing (status is not saved, so a turn in flight resumes, not repeats).
    let text = serde_json::to_string(&persist::SavedPane {
        agent: Some("claude".into()),
        session: Some("abc123".into()),
        cmd: Some("claude \"do the thing\"".into()),
        ..Default::default()
    })
    .unwrap();
    let pane: persist::SavedPane = serde_json::from_str(&text).unwrap();
    let resume = d.restore_cmd(&pane).expect("claude resumes");
    assert!(
        resume.contains("abc123") && !resume.contains("do the thing"),
        "{resume}"
    );
}

#[test]
fn a_conversation_comes_back_in_one_pane_only() {
    let (d, _rx) = daemon();
    let folder = std::env::temp_dir();
    let mut resuming = std::collections::HashSet::new();
    let saved = |session: Option<&str>| persist::SavedPane {
        agent: Some("claude".into()),
        session: session.map(Into::into),
        ..Default::default()
    };

    // Two panes of one session: the second would resume the conversation the first holds.
    let (mut first, mut second) = (saved(Some("abc123")), saved(Some("abc123")));
    assert!(
        d.restore_cmd_once(&mut first, &folder, &mut resuming)
            .is_some_and(|resume| resume.contains("abc123"))
    );
    assert_eq!(d.restore_cmd_once(&mut second, &folder, &mut resuming), None);
    assert_eq!(
        (first.agent.as_deref(), second.agent, second.session),
        (Some("claude"), None, None),
        "the second is a shell and no longer claims the agent"
    );

    // Two with no session in one folder would both resume "the last one".
    let (mut third, mut fourth) = (saved(None), saved(None));
    assert!(d.restore_cmd_once(&mut third, &folder, &mut resuming).is_some());
    assert_eq!(d.restore_cmd_once(&mut fourth, &folder, &mut resuming), None);

    // Another session, or the same agent in another folder, is its own conversation.
    assert!(
        d.restore_cmd_once(&mut saved(Some("other")), &folder, &mut resuming)
            .is_some()
    );
    let elsewhere = folder.join("elsewhere");
    assert!(d.restore_cmd_once(&mut saved(None), &elsewhere, &mut resuming).is_some());

    // A plain command may run in as many panes as it did.
    let server = || persist::SavedPane {
        cmd: Some("npm run dev".into()),
        ..Default::default()
    };
    for _ in 0..2 {
        assert_eq!(
            d.restore_cmd_once(&mut server(), &folder, &mut resuming).as_deref(),
            Some("npm run dev")
        );
    }
}

/// A hooked agent left working, its last output and hook report `secs` ago.
fn silent_working_agent(d: &mut Daemon, secs: u64) -> TermId {
    let id = pane(d);
    d.hook(id, "claude".into(), HookStatus::Working, "PreToolUse", &[]);
    let ago = Instant::now()
        .checked_sub(Duration::from_secs(secs))
        .expect("the clock reaches back that far");
    let t = d.terms.get_mut(&id).unwrap();
    t.last_output = ago;
    t.last_hook_at = Some(ago);
    id
}

#[test]
fn a_working_agent_silent_for_ten_minutes_is_quiet() {
    let (mut d, _rx) = daemon();
    let early = silent_working_agent(&mut d, 9 * 60);
    let late = silent_working_agent(&mut d, 11 * 60);
    d.update_statuses();
    let (early_quiet, late_quiet, late_status) = (
        d.terms[&early].quiet_since,
        d.terms[&late].quiet_since,
        d.terms[&late].status,
    );
    let wire = d.snapshot().terms[&late].quiet_since;
    close(&mut d, &[early, late]);
    assert_eq!(early_quiet, None, "nine minutes is not yet quiet");
    let since = late_quiet.expect("eleven silent minutes is quiet");
    assert!(
        term::unix_now().saturating_sub(since) >= 11 * 60,
        "quiet since its last sign of life"
    );
    assert_eq!(
        late_status,
        Status::Working,
        "quiet never changes the status"
    );
    assert_eq!(wire, late_quiet, "clients see it");
}

#[test]
fn quiet_clears_on_output_or_a_hook() {
    let (mut d, _rx) = daemon();
    let printed = silent_working_agent(&mut d, 11 * 60);
    let reported = silent_working_agent(&mut d, 11 * 60);
    d.update_statuses();
    assert!(d.terms[&printed].quiet_since.is_some() && d.terms[&reported].quiet_since.is_some());
    d.terms.get_mut(&printed).unwrap().last_output = Instant::now();
    d.hook(
        reported,
        "claude".into(),
        HookStatus::Working,
        "PostToolUse",
        &[],
    );
    d.update_statuses();
    let after = (
        d.terms[&printed].quiet_since,
        d.terms[&reported].quiet_since,
    );
    close(&mut d, &[printed, reported]);
    assert_eq!(after, (None, None));
}

#[test]
fn helpers_running_or_a_finished_turn_is_never_quiet() {
    let (mut d, _rx) = daemon();
    let waiting = silent_working_agent(&mut d, 11 * 60);
    let t = d.terms.get_mut(&waiting).unwrap();
    t.subagents.push(("h1".into(), "reviewer".into()));
    t.subagent_seen = Some(Instant::now());
    let finished = silent_working_agent(&mut d, 11 * 60);
    d.terms.get_mut(&finished).unwrap().status = Status::Done;
    d.update_statuses();
    let quiet = (
        d.terms[&waiting].quiet_since,
        d.terms[&finished].quiet_since,
    );
    close(&mut d, &[waiting, finished]);
    assert_eq!(quiet, (None, None));
}

#[test]
fn going_quiet_rings_no_alert() {
    let (mut d, _rx) = daemon();
    let id = silent_working_agent(&mut d, 11 * 60);
    d.terms.get_mut(&id).unwrap().alert_due = None;
    d.update_statuses();
    let (quiet, due, alerted) = (
        d.terms[&id].quiet_since,
        d.terms[&id].alert_due,
        d.terms[&id].alerted,
    );
    close(&mut d, &[id]);
    assert!(quiet.is_some());
    assert_eq!(
        (due, alerted.map(|a| a.0)),
        (None, None),
        "quiet is shown, never rung"
    );
}

const CALL_A: &str = "00000000000000aa";
const CALL_B: &str = "00000000000000bb";

/// A working agent, as the first prompt of a turn leaves it.
fn agent_at_work(d: &mut Daemon) -> TermId {
    let id = pane(d);
    d.hook(id, "claude".into(), HookStatus::Working, "UserPromptSubmit", &[]);
    id
}

/// The agent's hook reporting a tool call starting, with that call's fingerprint.
fn tool_start(d: &mut Daemon, id: TermId, call: &str) {
    let mut msg = report(id, "", 0, HookStatus::Working, "PreToolUse");
    if let ClientMsg::Hook { call: c, .. } = &mut msg {
        *c = Some(call.into());
    }
    d.apply_hook(msg, Some(true), Vec::new());
}

fn repeating(d: &mut Daemon, id: TermId) -> Option<u32> {
    d.update_statuses();
    d.terms[&id].repeats
}

#[test]
fn a_call_made_five_times_running_marks_the_agent_repeating_and_it_grows() {
    let (mut d, _rx) = daemon();
    let id = agent_at_work(&mut d);
    let mut seen = Vec::new();
    for _ in 0..6 {
        tool_start(&mut d, id, CALL_A);
        seen.push(repeating(&mut d, id));
    }
    let (status, wire) = (d.terms[&id].status, d.snapshot().terms[&id].repeats);
    close(&mut d, &[id]);
    assert_eq!(seen, [None, None, None, None, Some(5), Some(6)]);
    assert_eq!(status, Status::Working, "repeating never changes the status");
    assert_eq!(wire, Some(6), "clients see it");
}

#[test]
fn a_different_call_clears_the_mark() {
    let (mut d, _rx) = daemon();
    let id = agent_at_work(&mut d);
    for _ in 0..5 {
        tool_start(&mut d, id, CALL_A);
    }
    assert_eq!(repeating(&mut d, id), Some(5));
    tool_start(&mut d, id, CALL_B);
    let after = repeating(&mut d, id);
    close(&mut d, &[id]);
    assert_eq!(after, None);
}

#[test]
fn two_calls_taking_turns_for_eight_calls_mark_the_agent_repeating() {
    let (mut d, _rx) = daemon();
    let id = agent_at_work(&mut d);
    let mut seen = Vec::new();
    for turn in 0..8 {
        tool_start(&mut d, id, if turn % 2 == 0 { CALL_A } else { CALL_B });
        seen.push(repeating(&mut d, id));
    }
    close(&mut d, &[id]);
    assert_eq!(seen[6], None, "seven calls taking turns is not yet");
    assert_eq!(seen[7], Some(8));
}

#[test]
fn a_new_prompt_clears_the_mark_and_the_count_starts_again() {
    let (mut d, _rx) = daemon();
    let id = agent_at_work(&mut d);
    for _ in 0..5 {
        tool_start(&mut d, id, CALL_A);
    }
    assert_eq!(repeating(&mut d, id), Some(5));
    d.apply_hook(report(id, "", 0, HookStatus::Working, "UserPromptSubmit"), Some(true), Vec::new());
    let cleared = repeating(&mut d, id);
    for _ in 0..4 {
        tool_start(&mut d, id, CALL_A);
    }
    let four_more = repeating(&mut d, id);
    close(&mut d, &[id]);
    assert_eq!((cleared, four_more), (None, None), "five in a row means in one turn");
}

#[test]
fn leaving_working_clears_the_mark_and_what_led_to_it() {
    let (mut d, _rx) = daemon();
    let id = agent_at_work(&mut d);
    for _ in 0..5 {
        tool_start(&mut d, id, CALL_A);
    }
    assert_eq!(repeating(&mut d, id), Some(5));
    d.hook(id, "claude".into(), HookStatus::Done, "Stop", &[]);
    let done = repeating(&mut d, id);
    d.hook(id, "claude".into(), HookStatus::Working, "PreToolUse", &[]);
    tool_start(&mut d, id, CALL_A);
    let back = repeating(&mut d, id);
    close(&mut d, &[id]);
    assert_eq!((done, back), (None, None), "the sixth call of an old turn is not the first of a new one");
}

#[test]
fn only_a_tool_start_counts_and_a_report_without_a_call_leaves_the_run_alone() {
    let (mut d, _rx) = daemon();
    let id = agent_at_work(&mut d);
    for _ in 0..3 {
        tool_start(&mut d, id, CALL_A);
    }
    d.apply_hook(report(id, "", 0, HookStatus::Working, "PostToolUse"), Some(true), Vec::new());
    for _ in 0..2 {
        tool_start(&mut d, id, CALL_A);
    }
    let marked = repeating(&mut d, id);
    close(&mut d, &[id]);
    assert_eq!(marked, Some(5));
}

#[test]
fn a_fingerprint_that_is_not_sixteen_hex_characters_is_dropped() {
    let (mut d, _rx) = daemon();
    let id = agent_at_work(&mut d);
    for bad in ["zzzzzzzzzzzzzzzz", "00000000000000a", "00000000000000aaa", ""] {
        for _ in 0..6 {
            tool_start(&mut d, id, bad);
        }
    }
    let marked = repeating(&mut d, id);
    close(&mut d, &[id]);
    assert_eq!(marked, None);
}

#[test]
fn a_repeating_agent_is_shown_and_never_rung() {
    let (mut d, _rx) = daemon();
    let id = agent_at_work(&mut d);
    d.terms.get_mut(&id).unwrap().alert_due = None;
    for _ in 0..8 {
        tool_start(&mut d, id, CALL_A);
    }
    d.update_statuses();
    let (marked, due, alerted) = (d.terms[&id].repeats, d.terms[&id].alert_due, d.terms[&id].alerted);
    close(&mut d, &[id]);
    assert_eq!(marked, Some(8));
    assert_eq!((due, alerted.map(|a| a.0)), (None, None));
}

#[test]
fn with_output_off_nothing_is_saved_and_what_was_saved_goes() {
    let _one_at_a_time = SAVED_OUTPUT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (mut d, _rx) = daemon();
    d.cfg.restore.enabled = true;
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let t = *d.terms.keys().next().unwrap();
    let file = persist::output_dir().join(format!("{t}.bin"));
    d.terms
        .get_mut(&t)
        .unwrap()
        .ring_push_for_test(b"secret=hunter2\r\n");
    d.save_outputs(true);
    assert!(file.exists(), "on: saved");
    d.cfg.restore.output = false;
    d.drop_saved_outputs();
    assert!(!file.exists(), "turned off: what was saved is deleted");
    d.terms.get_mut(&t).unwrap().ring_push_for_test(b"more\r\n");
    let saved = d.saved();
    d.save_outputs(true);
    assert!(!file.exists(), "off: nothing new is saved");
    let (mut d2, _rx2) = daemon();
    (d2.cfg.restore.enabled, d2.cfg.restore.output) = (true, false);
    d2.restore(saved);
    let new = *d2.terms.keys().next().expect("the pane still comes back");
    let replay = String::from_utf8_lossy(&d2.terms[&new].replay()).into_owned();
    close(&mut d, &[t]);
    close(&mut d2, &[new]);
    assert!(
        !replay.contains(concat!(env!("APP_COMMAND"), " restarted here"))
            && !replay.contains("hunter2"),
        "{replay}"
    );
}

/// One saved workspace with a Claude pane, restored without starting Claude.
fn restored_claude_pane(d: &mut Daemon) -> TermId {
    d.cfg.restore.agents = false;
    let dir = std::env::temp_dir();
    let pane = persist::SavedPane {
        cwd: Some(dir.clone()),
        agent: Some("claude".into()),
        session: Some("abc123".into()),
        ..Default::default()
    };
    let tab = persist::SavedTab {
        name: String::new(),
        layout: crate::layout::Node::Leaf(1),
        focus: 1,
        panes: [(1, pane)].into(),
    };
    let ws = persist::SavedWs {
        name: "w".into(),
        cwd: dir,
        worktree: false,
        color: None,
        group: None,
        tabs: vec![tab],
        active_tab: 0,
    };
    d.restore(persist::Saved {
        workspaces: vec![ws],
        active: 0,
        made_worktrees: Vec::new(),
        conversations: Vec::new(),
        pending_starts: Vec::new(),
        conversation_operations: Vec::new(),
        turn_continues: Default::default(),
        turns_in_flight: Vec::new(),
    });
    d.workspaces[0].tabs[0].focus
}

#[test]
fn restore_gives_no_shell_to_a_layout_slot_whose_pane_was_not_saved() {
    let (mut d, _rx) = daemon();
    let dir = std::env::temp_dir();
    // Pane 2 was closed (or lost) before the save: the layout still names it, the panes map doesn't.
    let tab = persist::SavedTab {
        name: String::new(),
        layout: crate::layout::Node::Split {
            horizontal: true,
            ratio: 0.5,
            a: Box::new(crate::layout::Node::Leaf(1)),
            b: Box::new(crate::layout::Node::Leaf(2)),
        },
        focus: 1,
        panes: [(
            1,
            persist::SavedPane {
                cwd: Some(dir.clone()),
                ..Default::default()
            },
        )]
        .into(),
    };
    d.restore(persist::Saved {
        workspaces: vec![persist::SavedWs {
            name: "w".into(),
            cwd: dir,
            worktree: false,
            color: None,
            group: None,
            tabs: vec![tab],
            active_tab: 0,
        }],
        active: 0,
        made_worktrees: Vec::new(),
        conversations: Vec::new(),
        pending_starts: Vec::new(),
        conversation_operations: Vec::new(),
        turn_continues: Default::default(),
        turns_in_flight: Vec::new(),
    });
    let panes = d.terms.len();
    let leaves = d.workspaces[0].tabs[0].layout.leaves().len();
    let ids: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &ids);
    assert_eq!(panes, 1, "only the saved pane comes back, not a bare shell");
    assert_eq!(leaves, 1, "the layout shrinks to the pane that exists");
}

#[test]
fn a_finish_on_the_active_pane_stays_done_until_a_window_says_it_looked() {
    let (mut d, _rx) = daemon();
    let id = restored_claude_pane(&mut d);
    let mut window = viewer(&mut d);
    d.hook(id, "claude".into(), HookStatus::Working, "UserPromptSubmit", &[]);
    d.hook(id, "claude".into(), HookStatus::Done, "Stop", &[]);
    let after_finish = d.terms[&id].status;
    let told = told_attention(&mut window);
    d.message(
        40,
        ClientMsg::Command(crate::protocol::Command::MarkSeen { term: id }),
    );
    let after_look = d.terms[&id].status;
    close(&mut d, &[id]);
    assert_eq!(
        after_finish,
        Status::Done,
        "the active pane is not proof anyone is looking: a window in the background is not"
    );
    assert!(told, "so the finish is announced");
    assert_eq!(after_look, Status::Idle, "a window that looked clears it");
}

#[test]
fn an_agent_that_quits_stays_saved_until_the_server_outlives_the_hold() {
    let (mut d, _rx) = daemon();
    let id = restored_claude_pane(&mut d);
    d.persist_with(false, |_| Ok(()));
    let initially_saved = d.last_saved.contains("abc123");
    d.hook(id, "claude".into(), HookStatus::Gone, "SessionEnd", &[]);
    d.persist_with(false, |_| Ok(()));
    let retained_during_hold = d.last_saved.contains("abc123");
    d.last_loss = Some(Instant::now() - LOSS_HOLD - Duration::from_secs(1));
    d.persist_with(false, |_| Ok(()));
    let expired = !d.last_saved.contains("abc123");
    close(&mut d, &[id]);
    assert!(initially_saved, "saved as a Claude conversation");
    assert!(
        retained_during_hold,
        "a shutdown kills agents first: the save still resumes it"
    );
    assert!(
        expired,
        "still running after the hold: you quit it, so it's saved as a shell"
    );
}

#[test]
fn the_server_waits_out_the_hold_before_forgetting_sessions() {
    let (mut d, _rx) = daemon();
    let id = restored_claude_pane(&mut d);
    d.handle(Ev::Exited(id));
    assert!(
        d.terms.is_empty() && d.had_terms,
        "the last pane ended on its own"
    );
    assert!(
        !d.should_exit(),
        "a reboot may be killing everything: don't exit and forget yet"
    );
    d.last_loss = Some(Instant::now() - LOSS_HOLD - Duration::from_secs(1));
    assert!(d.should_exit(), "it was only that pane ending");
}

#[test]
fn conversation_restore_fences_writers_without_starting_a_pty() {
    use crate::conversation::{ConversationId, RecoveryState};
    for reference in [ConversationId(1), ConversationId(99)] {
        let (mut d, _rx) = daemon();
        d.cfg.restore.output = false;
        let mut record = crate::test_support::conversation();
        record.state = RecoveryState::Transferring;
        let pane = persist::SavedPane {
            conversation_id: Some(reference),
            agent: Some("codex".into()),
            cmd: Some("codex resume --last".into()),
            ..Default::default()
        };
        assert_eq!(d.restore_cmd(&pane), None);
        let tab = persist::SavedTab {
            name: String::new(),
            layout: crate::layout::Node::Leaf(1),
            focus: 1,
            panes: [(1, pane)].into(),
        };
        let ws = persist::SavedWs {
            name: "synthetic".into(),
            cwd: std::env::temp_dir(),
            worktree: false,
            color: None,
            group: None,
            tabs: vec![tab],
            active_tab: 0,
        };
        d.restore(persist::Saved {
            conversations: vec![record],
            workspaces: vec![ws],
            ..Default::default()
        });
        assert!(
            d.terms.is_empty(),
            "linked panes never start via legacy fallback, including missing refs"
        );
        assert_eq!(d.conversations[0].state, RecoveryState::RecoveryRequired);
        assert_eq!(d.conversations[0].generation, 8);
        assert_eq!(
            d.saved().conversations,
            d.conversations,
            "structured-only identity survives another save"
        );
    }
}

#[test]
fn legacy_pane_metadata_does_not_invent_a_verified_binding() {
    let (mut d, _rx) = daemon();
    let id = restored_claude_pane(&mut d);
    assert!(d.conversations.is_empty());
    assert!(d.saved().conversations.is_empty());
    assert_eq!(d.terms[&id].conversation_id, None);
    close(&mut d, &[id]);
}

#[test]
fn a_restore_notes_only_the_conversations_that_were_running() {
    use crate::conversation::{ConversationId, RecoveryState};
    let record = |id: u64, state, native_session: &str| {
        let mut record = crate::test_support::conversation();
        record.id = ConversationId(id);
        record.state = state;
        record.binding.session_id = native_session.into();
        record
    };
    let (mut d, _rx) = daemon();
    let before = crate::test_support::conversation().generation;
    d.restore(persist::Saved {
        conversations: vec![
            record(1, RecoveryState::Running, "thread-1"),
            record(2, RecoveryState::Stopped, "thread-2"),
            record(3, RecoveryState::RecoveryRequired, "thread-3"),
            record(4, RecoveryState::Transferring, "thread-4"),
        ],
        ..Default::default()
    });
    assert_eq!(
        d.resumable_conversations,
        vec![(ConversationId(1), before + 1), (ConversationId(4), before + 1)],
        "a stopped one, or one already needing recovery, is not resumed by the restart"
    );
}

#[test]
fn startup_persists_recovered_generations_before_another_restart() {
    use crate::conversation::RecoveryState;
    for state in [RecoveryState::Running, RecoveryState::Transferring] {
        let dir = persist::scratch_dir();
        let path = dir.path().join("session.json");
        let mut record = crate::test_support::conversation();
        record.state = state;
        let saved = persist::Saved {
            conversations: vec![record],
            ..Default::default()
        };
        persist::save_in(&path, &saved).unwrap();
        let (mut d, _rx) = daemon();
        d.cfg.restore.enabled = true;
        d.restore_startup(serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap());
        let before = d.last_saved.clone();
        d.persist_with(false, |_| anyhow::bail!("synthetic write failure"));
        assert_eq!(
            d.last_saved, before,
            "failed persistence must remain due for retry"
        );
        d.persist_with(false, |saved| persist::save_in(&path, saved));
        let persisted: persist::Saved =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            (
                persisted.conversations[0].generation,
                persisted.conversations[0].state
            ),
            (8, RecoveryState::RecoveryRequired)
        );
        let (mut restarted, _rx) = daemon();
        restarted.restore_startup(persisted);
        assert_eq!(restarted.conversations[0].generation, 8);
    }
}

#[test]
fn registry_only_sessions_survive_the_unused_daemon_exit_deadline() {
    let (mut d, _rx) = daemon();
    d.restore_startup(persist::Saved {
        conversations: vec![crate::test_support::conversation()],
        ..Default::default()
    });
    d.empty_since = Instant::now() - EXIT_WHEN_UNUSED_FOR - Duration::from_secs(1);
    assert!(
        !d.should_exit(),
        "retained identity is not an unused daemon; empty exit would forget it"
    );
    d.had_terms = true;
    assert!(
        !d.should_exit(),
        "closing the last PTY does not discard independent identities"
    );
    d.conversations.clear();
    assert!(
        d.should_exit(),
        "ordinary empty daemon exit remains unchanged"
    );
}

#[test]
fn naming_a_conversation_names_it_for_every_client_and_empty_clears_it() {
    let (mut d, _rx) = daemon();
    d.conversations.push(crate::test_support::conversation());
    let id = d.conversations[0].id;
    let title = |d: &Daemon| d.conversation_summaries()[0].title.clone();
    assert_eq!(title(&d), None);

    d.command(
        0,
        Command::ConversationRename {
            id,
            title: "  Fix the\tlogin bug \n".into(),
        },
    )
    .unwrap();
    assert_eq!(title(&d).as_deref(), Some("Fix thelogin bug"));

    d.command(
        0,
        Command::ConversationRename {
            id,
            title: "   ".into(),
        },
    )
    .unwrap();
    assert_eq!(title(&d), None);
}

#[test]
fn a_conversation_put_away_is_marked_for_every_client_and_can_come_back() {
    let (mut d, _rx) = daemon();
    d.conversations.push(crate::test_support::conversation());
    let id = d.conversations[0].id;
    let archived = |d: &Daemon| d.conversation_summaries()[0].archived;
    assert!(!archived(&d));
    d.command(0, Command::ConversationArchive { id, archived: true })
        .unwrap();
    assert!(archived(&d));
    d.command(0, Command::ConversationArchive { id, archived: false })
        .unwrap();
    assert!(!archived(&d));
}

#[test]
fn naming_or_putting_away_a_conversation_that_does_not_exist_is_refused() {
    let (mut d, _rx) = daemon();
    let id = crate::conversation::ConversationId(99);
    assert!(
        d.command(
            0,
            Command::ConversationRename {
                id,
                title: "x".into()
            }
        )
        .is_err()
    );
    assert!(
        d.command(0, Command::ConversationArchive { id, archived: true })
            .is_err()
    );
}

#[test]
fn a_terminal_pane_can_be_read_resized_and_typed_into_by_its_own_number() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);

    let screen = d.pane_screen(t).unwrap();
    assert!(screen.starts_with("\u{1b}[0m\u{1b}[2J\u{1b}[H"), "{screen:?}");
    assert!(d.pane_replay(t).is_ok());

    d.command(0, Command::TerminalResize { term: t, cols: 100, rows: 30 })
        .unwrap();
    assert_eq!(d.terms[&t].parser.screen().size(), (30, 100));

    d.command(
        0,
        Command::TerminalInput {
            term: t,
            data: b"echo hi\r".to_vec(),
        },
    )
    .unwrap();
}

#[test]
fn terminal_requests_with_nothing_to_do_or_nowhere_to_go_are_refused() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let nowhere = 9999;

    for command in [
        Command::TerminalInput { term: t, data: Vec::new() },
        Command::TerminalInput { term: t, data: vec![b'x'; 16 * 1024 + 1] },
        Command::TerminalInput { term: nowhere, data: b"x".to_vec() },
        Command::TerminalResize { term: t, cols: 1, rows: 24 },
        Command::TerminalResize { term: t, cols: 80, rows: 301 },
        Command::TerminalResize { term: nowhere, cols: 80, rows: 24 },
    ] {
        assert!(d.command(0, command.clone()).is_err(), "{command:?}");
    }
    assert!(d.pane_screen(nowhere).is_err());
    assert!(d.pane_replay(nowhere).is_err());
}

#[test]
fn a_terminal_session_belongs_to_the_app_that_started_it() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.terms.get_mut(&t).expect("the pane").owner = Surface::Desktop;
    let (tx, _) = mpsc::channel::<ServerMsg>(8);
    d.clients.insert(
        1,
        Client {
            tx,
            attach: false,
            surface: Surface::Terminal,
            from: None,
            structured: None,
            rejected: false,
            behind: std::cell::Cell::new(false),
        },
    );
    assert!(
        d.may_use_terminal(1, t).is_err(),
        "the terminal app cannot use a desktop session"
    );
    d.clients.get_mut(&1).expect("the client").surface = Surface::Desktop;
    assert!(
        d.may_use_terminal(1, t).is_ok(),
        "the app that started it can use it"
    );
    d.clients.get_mut(&1).expect("the client").from = Some(t);
    d.clients.get_mut(&1).expect("the client").surface = Surface::Terminal;
    assert!(
        d.may_use_terminal(1, t).is_ok(),
        "an agent acting from a pane goes by the pane's own grants"
    );
    close(&mut d, &[t]);
}

#[test]
fn each_app_is_shown_only_its_own_terminal_sessions() {
    let (mut d, _rx) = daemon();
    let terminal = pane(&mut d);
    let desktop = pane(&mut d);
    d.terms.get_mut(&desktop).expect("the pane").owner = Surface::Desktop;
    let terminal_view = d.snapshot_for(Surface::Terminal);
    let desktop_view = d.snapshot_for(Surface::Desktop);
    assert!(terminal_view.terms.contains_key(&terminal));
    assert!(!terminal_view.terms.contains_key(&desktop));
    assert!(desktop_view.terms.contains_key(&desktop));
    assert!(!desktop_view.terms.contains_key(&terminal));
    close(&mut d, &[terminal, desktop]);
}

#[test]
fn the_terminal_app_cannot_close_a_desktop_tab_or_workspace() {
    let (mut d, _rx) = daemon();
    d.clients.insert(1, test_client(Surface::Desktop));
    d.clients.insert(2, test_client(Surface::Terminal));
    d.command(
        1,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let ws = d.workspaces[0].id;
    let tab = d.workspaces[0].tabs[0].id;
    let term = d.workspaces[0].tabs[0].focus;
    assert_eq!(d.terms[&term].owner, Surface::Desktop);

    assert!(d.may_command(2, &Command::CloseTab { ws, tab }).is_err());
    assert!(d.may_command(2, &Command::CloseWorkspace { ws }).is_err());
    assert!(d.may_command(1, &Command::CloseTab { ws, tab }).is_ok());
    assert!(d.may_command(1, &Command::CloseWorkspace { ws }).is_ok());
    close(&mut d, &[term]);
}

fn test_client(surface: Surface) -> Client {
    let (tx, _) = mpsc::channel::<ServerMsg>(8);
    Client {
        tx,
        attach: false,
        surface,
        from: None,
        structured: None,
        rejected: false,
        behind: std::cell::Cell::new(false),
    }
}

#[test]
fn an_agent_that_died_mid_turn_comes_back_as_a_shell_that_offers_the_resume() {
    let (mut d, _rx) = daemon();
    let dir = std::env::temp_dir();
    let saved_pane = |owner: Surface, was_working: bool| persist::SavedPane {
        cwd: Some(dir.clone()),
        cmd: Some("codex".into()),
        agent: Some("codex".into()),
        owner,
        was_working,
        ..Default::default()
    };
    let ws = |pane: persist::SavedPane| persist::SavedWs {
        name: String::new(),
        cwd: dir.clone(),
        worktree: false,
        color: None,
        group: None,
        tabs: vec![persist::SavedTab {
            name: String::new(),
            layout: Node::Leaf(1),
            focus: 1,
            panes: [(1, pane)].into(),
        }],
        active_tab: 0,
    };
    d.restore(persist::Saved {
        workspaces: vec![
            ws(saved_pane(Surface::Desktop, true)),
            ws(saved_pane(Surface::Desktop, false)),
            // Its own session, so it is not taken for a second pane of the conversation above.
            ws(persist::SavedPane {
                session: Some("its-own".into()),
                ..saved_pane(Surface::Terminal, true)
            }),
        ],
        active: 0,
        made_worktrees: Vec::new(),
        conversations: Vec::new(),
        pending_starts: Vec::new(),
        conversation_operations: Vec::new(),
        turn_continues: Default::default(),
        turns_in_flight: Vec::new(),
    });
    let [interrupted, finished, in_terminal] = [0, 1, 2].map(|i| d.workspaces[i].tabs[0].focus);

    let info = d.snapshot();
    let offered = info.terms[&interrupted]
        .interrupted
        .clone()
        .expect("it died mid-turn");
    assert_eq!(offered.agent, "codex");
    assert!(offered.resume.contains("resume"), "{}", offered.resume);
    assert_eq!(
        info.terms[&interrupted].agent, None,
        "it is a shell now: nothing resumed on its own"
    );
    assert_eq!(
        info.terms[&finished].interrupted, None,
        "an agent that was not working is restored as before"
    );
    assert!(
        info.terms[&in_terminal].interrupted.is_some() && info.terms[&in_terminal].agent.is_none(),
        "the terminal app's sessions are told of a cut turn the same way"
    );

    // Once an agent runs in the pane again, there is nothing left to offer.
    d.terms.get_mut(&interrupted).unwrap().agent = Some("codex".into());
    assert_eq!(d.snapshot().terms[&interrupted].interrupted, None);
    close(&mut d, &[interrupted, finished, in_terminal]);
}

#[test]
fn the_timeline_records_status_changes_and_the_shells_prompt_marks() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.set_status(t, Status::Working, "hook: UserPromptSubmit");
    d.set_status(t, Status::Working, "hook: UserPromptSubmit");
    d.set_status(t, Status::Done, "hook: Stop");
    {
        let pane = d.terms.get_mut(&t).unwrap();
        pane.output(b"\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;2\x07");
        pane.take_marks();
    }
    let events: Vec<_> = d.snapshot().terms[&t]
        .timeline
        .iter()
        .map(|e| (e.kind, e.detail.clone()))
        .collect();
    assert_eq!(
        events,
        [
            (TurnKind::Working, "hook: UserPromptSubmit".to_string()),
            (TurnKind::Done, "hook: Stop".to_string()),
            (TurnKind::Prompt, String::new()),
            (TurnKind::CommandStarted, String::new()),
            (TurnKind::CommandFinished, "2".to_string()),
        ],
        "a repeated report is one event"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_failed_command_sets_the_exit_mark_and_the_next_one_clears_it() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let feed = |d: &mut Daemon, bytes: &[u8]| {
        let pane = d.terms.get_mut(&t).unwrap();
        pane.output(bytes);
        pane.take_marks();
        d.snapshot().terms[&t].exit
    };
    assert_eq!(d.snapshot().terms[&t].exit, None, "a running pane has none");
    let failed = feed(&mut d, b"\x1b]133;C\x07\x1b]133;D;137\x07").expect("a failure");
    assert_eq!(failed.code, Some(137));
    assert_eq!(feed(&mut d, b"\x1b]133;C\x07"), None, "a new command clears it");
    assert_eq!(feed(&mut d, b"\x1b]133;D;0\x07"), None, "a clean exit is not a problem");
    assert_eq!(feed(&mut d, b"\x1b]133;D\x07"), None, "no code, no claim");
    feed(&mut d, b"\x1b]133;D;1\x07");
    d.set_status(t, Status::Working, "hook: UserPromptSubmit");
    assert_eq!(d.snapshot().terms[&t].exit, None, "the agent working again clears it");
    close(&mut d, &[t]);
}

#[test]
fn a_blocked_pane_reports_what_it_is_asking_and_a_working_one_does_not() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    {
        let pane = d.terms.get_mut(&t).unwrap();
        pane.output(b"Bash command\r\n\r\n  rm -rf target\r\n\r\nDo you want to proceed?\r\n> 1. Yes\r\n  2. No\r\n");
    }
    d.set_status(t, Status::Working, "hook");
    assert_eq!(d.snapshot().terms[&t].asking, "");
    d.set_status(t, Status::Blocked, "hook: Notification");
    let asking = d.snapshot().terms[&t].asking.clone();
    assert!(asking.contains("rm -rf target"), "{asking}");
    assert!(asking.contains("2. No"), "{asking}");
    close(&mut d, &[t]);
}

#[test]
fn pin_and_archive_stick_survive_a_restart_and_never_stop_the_pane() {
    let (mut d, _rx) = daemon();
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let t = d.workspaces[0].tabs[0].focus;
    d.command(
        0,
        Command::PinPane {
            term: t,
            pinned: true,
        },
    )
    .unwrap();
    d.command(
        0,
        Command::ArchivePane {
            term: t,
            archived: true,
        },
    )
    .unwrap();
    assert!(
        d.terms.contains_key(&t),
        "archiving puts it away without stopping it"
    );
    let info = &d.snapshot().terms[&t];
    assert_eq!((info.pinned, info.archived), (true, true));

    let (mut d2, _rx2) = daemon();
    d2.restore(d.saved());
    let back = d2.workspaces[0].tabs[0].focus;
    assert_eq!(
        (d2.terms[&back].pinned, d2.terms[&back].archived),
        (true, true),
        "both come back after a restart"
    );

    d2.command(
        0,
        Command::ArchivePane {
            term: back,
            archived: false,
        },
    )
    .unwrap();
    d2.command(
        0,
        Command::PinPane {
            term: back,
            pinned: false,
        },
    )
    .unwrap();
    assert_eq!(
        (d2.terms[&back].pinned, d2.terms[&back].archived),
        (false, false)
    );
    close(&mut d, &[t]);
    close(&mut d2, &[back]);
}

#[test]
fn a_change_to_one_pane_reaches_clients_as_a_delta_and_no_change_as_nothing() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let surface = crate::protocol::Surface::Terminal;
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(7, tx, true, surface, false));
    while crx.try_recv().is_ok() {}

    d.publish_state(surface, d.snapshot_for(surface));
    assert!(
        matches!(crx.try_recv(), Ok(ServerMsg::State(_))),
        "the first change after an attach is the whole state"
    );
    d.publish_state(surface, d.snapshot_for(surface));
    assert!(crx.try_recv().is_err(), "nothing changed, so nothing is sent");

    d.terms.get_mut(&t).unwrap().process = "vim".into();
    d.publish_state(surface, d.snapshot_for(surface));
    match crx.try_recv() {
        Ok(ServerMsg::TermsChanged { changed, removed, .. }) => {
            assert_eq!(changed.iter().map(|term| term.id).collect::<Vec<_>>(), vec![t]);
            assert_eq!(changed[0].process, "vim");
            assert!(removed.is_empty());
        }
        other => panic!("one pane changed, so a delta: {other:?}"),
    }

    close(&mut d, &[t]);
    d.publish_state(surface, d.snapshot_for(surface));
    let gone = std::iter::from_fn(|| crx.try_recv().ok()).any(|m| match m {
        ServerMsg::State(s) => !s.terms.contains_key(&t),
        ServerMsg::TermsChanged { removed, .. } => removed.contains(&t),
        _ => false,
    });
    assert!(gone, "a closed pane leaves the clients' state, whole or as a delta");
}

#[test]
fn each_state_or_delta_a_surface_is_sent_carries_the_next_revision() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let surface = crate::protocol::Surface::Terminal;
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(7, tx, true, surface, false));
    while crx.try_recv().is_ok() {}
    let revision_of = |msg| match msg {
        Ok(ServerMsg::State(snap)) => snap.revision,
        Ok(ServerMsg::TermsChanged { revision, .. }) => revision,
        other => panic!("a state or a pane delta: {other:?}"),
    };

    d.publish_state(surface, d.snapshot_for(surface));
    let first = revision_of(crx.try_recv());
    assert_eq!(first, 1, "a surface's first update is revision 1");

    d.publish_state(surface, d.snapshot_for(surface));
    assert!(crx.try_recv().is_err(), "nothing changed, so nothing is sent or numbered");

    d.terms.get_mut(&t).unwrap().process = "vim".into();
    d.publish_state(surface, d.snapshot_for(surface));
    let delta = crx.try_recv();
    assert!(matches!(delta, Ok(ServerMsg::TermsChanged { .. })), "{delta:?}");
    assert_eq!(revision_of(delta), first + 1);

    d.terms.get_mut(&t).unwrap().process = "git".into();
    d.publish_state(surface, d.snapshot_for(surface));
    assert_eq!(revision_of(crx.try_recv()), first + 2);

    d.message(7, ClientMsg::Query(Query::List));
    let Ok(ServerMsg::Reply(Reply::List(listed))) = crx.try_recv() else {
        panic!("a list");
    };
    assert_eq!(listed.revision, first + 2, "a list carries the current revision and uses none");
    assert_eq!(
        d.snapshot_for(crate::protocol::Surface::Desktop).revision,
        0,
        "the other surface counts its own"
    );
    close(&mut d, &[t]);
}

/// A terminal client attached as `id`, past its attach snapshot and the first full state.
fn attached_and_caught_up(d: &mut Daemon, id: ClientId) -> mpsc::Receiver<ServerMsg> {
    let surface = crate::protocol::Surface::Terminal;
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(id, tx, true, surface, false));
    d.publish_state(surface, d.snapshot_for(surface));
    while crx.try_recv().is_ok() {}
    crx
}

/// The panes the next message says left the list, and why.
fn removals(crx: &mut mpsc::Receiver<ServerMsg>) -> BTreeMap<TermId, RemovalReason> {
    match crx.try_recv() {
        Ok(ServerMsg::TermsChanged { removed, reasons, .. }) => {
            assert_eq!(removed, reasons.keys().copied().collect::<Vec<_>>(), "a reason for each");
            reasons
        }
        Ok(ServerMsg::State(snap)) => snap.removed,
        other => panic!("a state or a pane delta: {other:?}"),
    }
}

#[test]
fn a_pane_a_person_closed_leaves_the_list_as_closed() {
    let (mut d, _rx) = daemon();
    let (kept, t) = (pane(&mut d), pane(&mut d));
    let surface = crate::protocol::Surface::Terminal;
    let mut crx = attached_and_caught_up(&mut d, 7);

    d.command(7, Command::ClosePane { term: t }).unwrap();
    d.publish_state(surface, d.snapshot_for(surface));
    assert_eq!(removals(&mut crx), BTreeMap::from([(t, RemovalReason::Closed)]));
    close(&mut d, &[kept]);
}

#[test]
fn a_pane_whose_shell_exited_leaves_the_list_as_exited() {
    let (mut d, _rx) = daemon();
    let (kept, t) = (pane(&mut d), pane(&mut d));
    let surface = crate::protocol::Surface::Terminal;
    let mut crx = attached_and_caught_up(&mut d, 7);

    d.handle(Ev::Exited(t));
    d.publish_state(surface, d.snapshot_for(surface));
    assert_eq!(removals(&mut crx), BTreeMap::from([(t, RemovalReason::Exited)]));
    close(&mut d, &[kept]);
}

#[test]
fn a_whole_state_after_a_removal_says_what_left_since_the_last_message() {
    let (mut d, _rx) = daemon();
    let (kept, closed, exited) = (pane(&mut d), pane(&mut d), pane(&mut d));
    let surface = crate::protocol::Surface::Terminal;
    let mut crx = attached_and_caught_up(&mut d, 7);

    d.command(7, Command::ClosePane { term: closed }).unwrap();
    d.handle(Ev::Exited(exited));
    // A second window attaching makes the next update a whole state for both.
    let (tx, _other) = mpsc::channel(64);
    d.handle(Ev::Connected(8, tx, true, surface, false));
    d.publish_state(surface, d.snapshot_for(surface));
    let Ok(ServerMsg::State(snap)) = crx.try_recv() else {
        panic!("the first change after an attach is the whole state");
    };
    assert_eq!(
        snap.removed,
        BTreeMap::from([(closed, RemovalReason::Closed), (exited, RemovalReason::Exited)])
    );

    d.terms.get_mut(&kept).unwrap().process = "vim".into();
    d.sent_state.remove(&surface);
    d.publish_state(surface, d.snapshot_for(surface));
    let Ok(ServerMsg::State(snap)) = crx.try_recv() else {
        panic!("a whole state");
    };
    assert!(snap.removed.is_empty(), "a removal is told once");
    assert!(d.snapshot_for(surface).removed.is_empty(), "a list names none");
    close(&mut d, &[kept]);
}

#[test]
fn a_pane_that_left_while_still_running_says_whether_it_was_closed_hidden_or_moved() {
    let (mut d, _rx) = daemon();
    let (kept, closing, spare, moved) = (pane(&mut d), pane(&mut d), pane(&mut d), pane(&mut d));
    let surface = crate::protocol::Surface::Terminal;
    let mut crx = attached_and_caught_up(&mut d, 7);

    d.command(7, Command::CloseSession { term: closing }).unwrap();
    d.terms.get_mut(&spare).unwrap().spare = true;
    d.terms.get_mut(&moved).unwrap().owner = crate::protocol::Surface::Desktop;
    d.publish_state(surface, d.snapshot_for(surface));
    assert_eq!(
        removals(&mut crx),
        BTreeMap::from([
            (closing, RemovalReason::Closed),
            (spare, RemovalReason::PutAway),
            (moved, RemovalReason::Moved),
        ])
    );
    close(&mut d, &[kept, closing, spare, moved]);
}

#[test]
fn a_pane_from_before_the_attach_that_is_closed_before_any_update_leaves_as_closed() {
    let (mut d, _rx) = daemon();
    let (kept, t) = (pane(&mut d), pane(&mut d));
    let surface = crate::protocol::Surface::Terminal;
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(7, tx, true, surface, false));
    while crx.try_recv().is_ok() {}

    d.command(7, Command::ClosePane { term: t }).unwrap();
    d.publish_state(surface, d.snapshot_for(surface));
    assert_eq!(removals(&mut crx), BTreeMap::from([(t, RemovalReason::Closed)]));
    close(&mut d, &[kept]);
}

#[test]
fn a_pane_that_ended_while_nobody_was_attached_is_not_named_to_the_next_client() {
    let (mut d, _rx) = daemon();
    let (kept, t) = (pane(&mut d), pane(&mut d));
    let surface = crate::protocol::Surface::Terminal;
    // The first window saw both panes in an update, then went away.
    let _gone = attached_and_caught_up(&mut d, 7);
    d.handle(Ev::Disconnected(7));
    d.handle(Ev::Exited(t));

    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(8, tx, true, surface, false));
    while crx.try_recv().is_ok() {}
    d.terms.get_mut(&kept).unwrap().process = "vim".into();
    d.publish_state(surface, d.snapshot_for(surface));
    assert!(removals(&mut crx).is_empty(), "its attach snapshot never had that pane");
    close(&mut d, &[kept]);
}

#[test]
fn a_pane_gone_for_no_reason_the_daemon_recorded_leaves_as_unknown() {
    let (mut d, _rx) = daemon();
    let (kept, t) = (pane(&mut d), pane(&mut d));
    let surface = crate::protocol::Surface::Terminal;
    let mut crx = attached_and_caught_up(&mut d, 7);

    // Nothing in the daemon drops a pane without a reason, so it is forced here.
    let mut gone = d.terms.remove(&t).unwrap();
    gone.kill_tree();
    d.publish_state(surface, d.snapshot_for(surface));
    assert_eq!(removals(&mut crx), BTreeMap::from([(t, RemovalReason::Unknown)]));
    close(&mut d, &[kept]);
}

/// `KillServer` kills every pane, sends `Bye` and exits without another update, so this reason
/// reaches a client only for a pane that ends between the stop request and `Bye`. The stop is
/// set by hand here because the command ends the process.
#[test]
fn a_pane_that_ends_between_the_stop_request_and_bye_leaves_as_daemon_stopping() {
    let (mut d, _rx) = daemon();
    let (kept, t) = (pane(&mut d), pane(&mut d));
    let surface = crate::protocol::Surface::Terminal;
    let mut crx = attached_and_caught_up(&mut d, 7);

    d.stopping = true;
    d.handle(Ev::Exited(t));
    d.publish_state(surface, d.snapshot_for(surface));
    assert_eq!(removals(&mut crx), BTreeMap::from([(t, RemovalReason::DaemonStopping)]));
    close(&mut d, &[kept]);
}

#[test]
fn after_an_attached_client_lists_the_next_change_reaches_its_surface_in_full() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let surface = crate::protocol::Surface::Terminal;
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(7, tx, true, surface, false));
    let (asker, mut asked) = mpsc::channel(64);
    d.handle(Ev::Connected(8, asker, false, surface, false));
    d.publish_state(surface, d.snapshot_for(surface));
    while crx.try_recv().is_ok() {}

    d.message(8, ClientMsg::Query(Query::List));
    assert!(matches!(asked.try_recv(), Ok(ServerMsg::Reply(Reply::List(_)))));
    d.terms.get_mut(&t).unwrap().process = "vim".into();
    d.publish_state(surface, d.snapshot_for(surface));
    assert!(
        matches!(crx.try_recv(), Ok(ServerMsg::TermsChanged { .. })),
        "a list for a client that follows no updates leaves the deltas be"
    );

    // The list holds a change the surface was not sent yet, so a delta worked out from what
    // was sent would not fit it.
    d.terms.get_mut(&t).unwrap().process = "git".into();
    d.message(7, ClientMsg::Query(Query::List));
    let Ok(ServerMsg::Reply(Reply::List(listed))) = crx.try_recv() else {
        panic!("a list");
    };
    assert_eq!(listed.terms[&t].process, "git");
    d.terms.get_mut(&t).unwrap().process = "vim".into();
    d.publish_state(surface, d.snapshot_for(surface));
    match crx.try_recv() {
        Ok(ServerMsg::State(state)) => {
            assert_eq!(state.terms[&t].process, "vim");
            assert_eq!(state.revision, listed.revision + 1);
        }
        other => panic!("the whole state after a list: {other:?}"),
    }
    close(&mut d, &[t]);
}

#[test]
fn the_daemon_id_is_in_the_state_a_client_gets_and_a_delta_leaves_it_standing() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let surface = crate::protocol::Surface::Terminal;
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(7, tx, true, surface, true));
    while crx.try_recv().is_ok() {}

    d.publish_state(surface, d.snapshot_for(surface));
    let Ok(ServerMsg::State(held)) = crx.try_recv() else {
        panic!("the first change after an attach is the whole state");
    };
    assert_eq!(held.daemon_id.as_deref(), Some(d.id.as_str()));

    d.terms.get_mut(&t).unwrap().process = "vim".into();
    d.publish_state(surface, d.snapshot_for(surface));
    assert!(
        matches!(crx.try_recv(), Ok(ServerMsg::TermsChanged { .. })),
        "the id is the same, so only the pane travels and the held id stands"
    );
    close(&mut d, &[t]);
}

#[test]
fn a_client_that_attached_without_replays_gets_one_when_it_asks() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        7,
        tx,
        true,
        crate::protocol::Surface::Terminal,
        false,
    ));
    let replays_on_attach = std::iter::from_fn(|| crx.try_recv().ok())
        .filter(|m| matches!(m, ServerMsg::Replay { .. }))
        .count();
    assert_eq!(replays_on_attach, 0);

    d.handle(Ev::Msg(7, ClientMsg::WantReplay { term: t }));

    assert!(matches!(crx.try_recv(), Ok(ServerMsg::Replay { term, .. }) if term == t));
    close(&mut d, &[t]);
}

#[test]
fn a_desktop_replay_is_the_end_of_the_ring_and_then_the_screen() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let term = d.terms.get_mut(&t).unwrap();
    term.ring_push_for_test(b"0123456789");

    let tail = term.replay_tail(4);
    let full = term.replay();

    assert!(tail.starts_with(b"6789[0m[2J[H"), "{tail:?}");
    assert_eq!(term.replay_tail(usize::MAX), full);
    assert!(full.len() > tail.len());
    close(&mut d, &[t]);
}

#[test]
fn a_replay_cut_after_a_full_screen_program_started_still_leaves_it_cleanly() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let term = d.terms.get_mut(&t).unwrap();
    term.output(b"\x1b[2J\x1b[Hbefore the program\r\n\x1b[?1049h\x1b[2J\x1b[Hfull screen");
    let (rows, cols) = term.parser.screen().size();

    // The cut falls after the switch to the alternate screen, so the history no longer says it.
    let replay = term.replay_tail(4);
    let mut view = vt100::Parser::new(rows, cols, 0);
    view.process(&replay);

    assert!(view.screen().alternate_screen());
    assert!(view.screen().contents().contains("full screen"));

    view.process(b"\x1b[?1049lleft");
    let left = view.screen().contents();
    assert!(!left.contains("full screen"), "{left:?}");
    assert!(left.starts_with("before the program\nleft"), "{left:?}");
    close(&mut d, &[t]);
}

fn permission_report(term: TermId, status: HookStatus, event: &str, agent: &str, tool: &str, detail: &str) -> ClientMsg {
    let mut msg = report(term, "", 0, status, event);
    if let ClientMsg::Hook { agent: a, ask, .. } = &mut msg {
        *a = agent.into();
        *ask = (!tool.is_empty()).then(|| PermissionAsk {
            tool: tool.into(),
            detail: detail.into(),
            // A hook cannot claim it; the daemon decides.
            answerable: true,
            choices: Vec::new(),
            questions: Vec::new(),
        });
    }
    msg
}

#[test]
fn a_permission_hook_reports_what_it_asks_until_the_agent_moves_on() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let asking = |d: &Daemon| d.terms[&t].ask_now();

    d.apply_hook(permission_report(t, HookStatus::Blocked, "PermissionRequest", "claude", "Bash", "cargo test"), None, vec![]);
    let ask = asking(&d).expect("the question the hook reported");
    assert_eq!((ask.tool.as_str(), ask.detail.as_str(), ask.answerable), ("Bash", "cargo test", true));
    assert_eq!(d.terms[&t].status, Status::Blocked);

    // The ping Claude sends right after, and a helper's news, leave the question as it was.
    d.apply_hook(permission_report(t, HookStatus::Blocked, "Notification:permission_prompt", "claude", "", ""), None, vec![]);
    d.apply_hook(permission_report(t, HookStatus::Same, "SubagentStop", "claude", "", ""), None, vec![]);
    assert!(asking(&d).is_some());

    // The tool ran, so the question is over.
    d.apply_hook(permission_report(t, HookStatus::Working, "PostToolUse", "claude", "", ""), None, vec![]);
    assert!(asking(&d).is_none());

    // A question that is not a permission prompt does not inherit an old one.
    d.apply_hook(permission_report(t, HookStatus::Blocked, "PermissionRequest", "claude", "Write", "/tmp/x"), None, vec![]);
    d.apply_hook(permission_report(t, HookStatus::Blocked, "PreToolUse", "claude", "", ""), None, vec![]);
    assert!(asking(&d).is_none(), "an AskUserQuestion is another question");
    close(&mut d, &[t]);
}

#[test]
fn only_the_agents_whose_key_is_known_can_be_answered_from_the_chat() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.apply_hook(permission_report(t, HookStatus::Blocked, "PermissionRequest", "codex", "shell", "ls"), None, vec![]);
    let ask = d.terms[&t].ask_now().expect("shown");
    assert!(!ask.answerable, "Codex's keys are not verified, so the chat says to answer in the terminal");
    close(&mut d, &[t]);
}

#[test]
fn a_pane_reports_its_question_only_while_it_needs_you() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.apply_hook(permission_report(t, HookStatus::Blocked, "PermissionRequest", "claude", "Bash", "ls"), None, vec![]);
    d.terms.get_mut(&t).unwrap().status = Status::Idle;
    assert!(d.terms[&t].ask_now().is_none(), "a question left over from before is not shown");
    close(&mut d, &[t]);
}

/// A workspace with one shell, and a second shell split beside it.
fn two_panes(d: &mut Daemon) -> (TermId, TermId) {
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    let a = *d.terms.keys().next().unwrap();
    d.command(
        0,
        Command::Split {
            term: a,
            dir: crate::layout::Dir::Right,
            cmd: None,
            cwd: None,
        },
    )
    .unwrap();
    let b = *d.terms.keys().find(|t| **t != a).unwrap();
    (a, b)
}

/// Pretend the undo window opened `ago` back.
fn closed_ago(d: &mut Daemon, term: TermId, ago: Duration) {
    let t = d.terms.get_mut(&term).unwrap();
    t.closing = t.closing.map(|(_, by)| (crate::clock::ago(ago), by));
}

#[test]
fn a_report_from_an_agent_the_panes_agent_started_changes_nothing() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let from = |session: &str, status: HookStatus, event: &str| {
        let ClientMsg::Hook { term, agent, cwd, prompt, said, subagent, pid, token, transcript, model, name, background, background_agents, background_reported, ask, call, .. } =
            report(id, "", 0, status, event)
        else {
            unreachable!()
        };
        ClientMsg::Hook { term, agent, status, session: Some(session.into()), cwd, prompt, said, subagent, event: event.into(), pid, token, transcript, model, name, background, background_agents, background_reported, ask, call }
    };
    d.apply_hook(from("main", HookStatus::Working, "UserPromptSubmit"), Some(true), Vec::new());

    // A headless agent the pane's agent ran: it starts a session of its own, works, and ends it.
    for (status, event) in [(HookStatus::Idle, "SessionStart"), (HookStatus::Done, "Stop"), (HookStatus::Gone, "SessionEnd")] {
        d.handle(Ev::HookChecked { msg: from("nested", status, event), verdict: Some(true), chain: vec![4242], nested: true });
    }

    let t = &d.terms[&id];
    assert_eq!(t.session.as_deref(), Some("main"), "the pane keeps its own session");
    assert_eq!(t.agent.as_deref(), Some("claude"), "and its agent is not gone");
    assert_eq!(t.status, Status::Working, "and it is still working");
    assert!(!t.trusted.contains(&4242), "nor is the nested agent taken for the pane's own");
    close(&mut d, &[id]);
}

/// A pane the desktop app started as an agent, whose agent has said it ended `ago` ago.
fn ended_agent_pane(d: &mut Daemon, ago: Duration) -> TermId {
    let id = pane(d);
    let t = d.terms.get_mut(&id).unwrap();
    t.owner = crate::protocol::Surface::Desktop;
    t.cmd = Some("claude".into());
    d.hook(id, "claude".into(), HookStatus::Idle, "SessionStart", &[]);
    d.hook(id, "claude".into(), HookStatus::Gone, "SessionEnd", &[]);
    let t = d.terms.get_mut(&id).unwrap();
    t.agent_ended = t.agent_ended.map(|_| crate::clock::ago(ago));
    // Its last keys (the `/exit`) were typed before the agent ended.
    t.last_input = crate::clock::ago(ago + Duration::from_secs(60));
    id
}

#[test]
fn a_pane_started_as_an_agent_closes_once_its_agent_has_ended() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let soon = ended_agent_pane(&mut d, Duration::ZERO);
    let ended = ended_agent_pane(&mut d, AGENT_ENDED_GRACE);

    d.close_ended_agents();

    assert!(!d.terms.contains_key(&ended), "the shell left behind goes with its agent");
    assert!(d.terms.contains_key(&soon), "not before the agent had the time to come back");
    close(&mut d, &[soon]);
}

#[test]
fn an_agent_that_started_over_keeps_its_pane() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    // `/clear` ends the session and starts the next one in the same program.
    let id = ended_agent_pane(&mut d, AGENT_ENDED_GRACE);
    d.hook(id, "claude".into(), HookStatus::Idle, "SessionStart", &[]);

    d.close_ended_agents();

    assert!(d.terms.contains_key(&id));
    assert!(d.terms[&id].agent_ended.is_none());
    close(&mut d, &[id]);
}

#[test]
fn a_shell_someone_went_on_to_use_after_its_agent_ended_is_kept() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let id = ended_agent_pane(&mut d, AGENT_ENDED_GRACE);
    d.terms.get_mut(&id).unwrap().input(b"ls\r");

    d.close_ended_agents();

    assert!(d.terms.contains_key(&id), "typed into after the agent ended: it is a shell in use");
    assert!(d.terms[&id].agent_ended.is_none(), "and it is not asked about again");
    close(&mut d, &[id]);
}

#[test]
fn only_a_desktop_pane_started_as_an_agent_ends_with_its_agent() {
    type Change = fn(&mut crate::daemon::term::Term);
    let cases: [(&str, Change); 3] = [
        ("a shell an agent was typed into", |t| t.cmd = None),
        ("a terminal app's pane", |t| t.owner = crate::protocol::Surface::Terminal),
        ("a popup", |t| t.popup = true),
    ];
    for (name, change) in cases {
        let (rt, (mut d, _rx)) = (runtime(), daemon());
        let _in_rt = rt.enter();
        let id = pane(&mut d);
        let t = d.terms.get_mut(&id).unwrap();
        t.owner = crate::protocol::Surface::Desktop;
        t.cmd = Some("claude".into());
        change(t);
        d.hook(id, "claude".into(), HookStatus::Gone, "SessionEnd", &[]);
        assert!(d.terms[&id].agent_ended.is_none(), "{name}");

        d.close_ended_agents();

        assert!(d.terms.contains_key(&id), "{name}");
        close(&mut d, &[id]);
    }
}

#[test]
fn a_soft_closed_session_leaves_the_snapshot_but_keeps_running() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (a, b) = two_panes(&mut d);
    d.command(0, Command::CloseSession { term: b }).unwrap();
    let snap = d.snapshot();
    assert!(!snap.terms.contains_key(&b) && snap.terms.contains_key(&a));
    for ws in &snap.workspaces {
        for tab in &ws.tabs {
            assert_eq!(tab.layout.leaves(), vec![a], "no hole for clients to draw");
        }
    }
    assert!(d.terms.contains_key(&b) && !d.terms[&b].process_exited);
    assert!(
        d.saved().workspaces[0].tabs[0]
            .panes
            .keys()
            .all(|t| *t != b),
        "a restart inside the window does not bring it back"
    );
    assert_eq!(d.activity().sessions, 1, "it is not counted as a session");
    d.finish_closes();
    assert!(d.terms.contains_key(&b), "not before the window ends");
    let terms: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn undo_within_the_window_brings_back_the_same_session() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (a, b) = two_panes(&mut d);
    d.command(0, Command::CloseSession { term: b }).unwrap();
    closed_ago(&mut d, b, UNDO_WINDOW - Duration::from_secs(1));
    d.command(0, Command::UndoClose { term: b }).unwrap();
    d.finish_closes();
    let snap = d.snapshot();
    assert!(snap.terms.contains_key(&b));
    assert_eq!(snap.workspaces[0].tabs[0].layout.leaves(), vec![a, b]);
    close(&mut d, &[a, b]);
}

#[test]
fn a_soft_closed_session_ends_when_the_window_passes() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (a, b) = two_panes(&mut d);
    d.command(0, Command::CloseSession { term: b }).unwrap();
    // A second close doesn't restart the clock.
    closed_ago(&mut d, b, UNDO_WINDOW);
    d.command(0, Command::CloseSession { term: b }).unwrap();
    d.finish_closes();
    assert!(!d.terms.contains_key(&b), "closed once the window passed");
    assert_eq!(d.snapshot().workspaces[0].tabs[0].layout.leaves(), vec![a]);
    // Undo after that, or for a session that isn't closing, does nothing.
    d.command(0, Command::UndoClose { term: b }).unwrap();
    d.command(0, Command::UndoClose { term: a }).unwrap();
    assert!(!d.terms.contains_key(&b) && d.terms.contains_key(&a));
    close(&mut d, &[a]);
}

#[test]
fn soft_closing_the_only_pane_hides_its_workspace_until_it_ends() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (a, b) = two_panes(&mut d);
    d.command(0, Command::ClosePane { term: b }).unwrap();
    d.command(0, Command::CloseSession { term: a }).unwrap();
    let snap = d.snapshot();
    assert!(snap.workspaces.is_empty() && snap.active_ws.is_none());
    assert_eq!(d.workspaces.len(), 1, "the tree keeps it for an undo");
    d.command(0, Command::UndoClose { term: a }).unwrap();
    assert_eq!(d.snapshot().workspaces.len(), 1);
    d.command(0, Command::CloseSession { term: a }).unwrap();
    closed_ago(&mut d, a, UNDO_WINDOW);
    d.finish_closes();
    assert!(d.workspaces.is_empty() && d.terms.is_empty());
}

fn new_fleet(d: &mut Daemon, name: &str, colors: Vec<u8>) -> Result<bool> {
    d.command(
        0,
        Command::NewFleet {
            name: name.into(),
            cwd: Some(std::env::temp_dir()),
            colors,
        },
    )
}

#[test]
fn a_new_fleet_is_named_holds_one_shell_and_takes_a_wanted_free_colour() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    new_fleet(&mut d, "  api  ", vec![3, 0]).unwrap();
    new_fleet(&mut d, "web", vec![3, 0]).unwrap();
    new_fleet(&mut d, "docs", vec![3, 0]).unwrap();
    let fleets = &d.snapshot().workspaces;
    assert_eq!(fleets.len(), 3);
    assert_eq!(fleets[0].name, "api", "the name is trimmed");
    assert_eq!(fleets[0].tabs[0].layout.leaves().len(), 1);
    assert_eq!(fleets[0].color, 3);
    assert_eq!(fleets[1].color, 0, "the wanted colour in use is skipped");
    assert!(
        !matches!(fleets[2].color, 3 | 0),
        "with every wanted colour taken, any free colour is used"
    );
    let terms: Vec<_> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn a_fleet_needs_a_plain_name_and_a_colour_in_the_palette() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    assert!(new_fleet(&mut d, "   ", vec![]).is_err());
    assert!(new_fleet(&mut d, "a\u{7}b", vec![]).is_err());
    assert!(new_fleet(&mut d, &"x".repeat(81), vec![]).is_err());
    assert!(
        d.workspaces.is_empty() && d.terms.is_empty(),
        "nothing started"
    );
    new_fleet(&mut d, "api", vec![200]).unwrap();
    let ws = d.workspaces[0].id;
    assert!(
        d.workspaces[0].color < 8,
        "a colour past the palette is ignored"
    );
    d.command(0, Command::SetWorkspaceColor { ws, color: 5 })
        .unwrap();
    assert_eq!(d.workspaces[0].color, 5);
    assert!(d
        .command(0, Command::SetWorkspaceColor { ws, color: 8 })
        .is_err());
    assert_eq!(d.workspaces[0].color, 5, "a refused colour changes nothing");
    d.command(
        0,
        Command::RenameWorkspace {
            ws,
            name: "core".into(),
        },
    )
    .unwrap();
    assert_eq!(d.workspaces[0].name, "core");
    let terms: Vec<_> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn a_new_ship_joins_its_fleet_in_the_fleets_folder_or_a_given_one() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    new_fleet(&mut d, "api", vec![]).unwrap();
    let ws = d.workspaces[0].id;
    let first = d.workspaces[0].tabs[0].focus;
    d.command(
        0,
        Command::NewShip {
            ws,
            cwd: None,
            cmd: None,
        },
    )
    .unwrap();
    let elsewhere = std::env::temp_dir().join("maitty-ship-folder");
    std::fs::create_dir_all(&elsewhere).unwrap();
    d.command(
        0,
        Command::NewShip {
            ws,
            cwd: Some(elsewhere.clone()),
            cmd: None,
        },
    )
    .unwrap();
    d.command(
        0,
        Command::NewShip {
            ws,
            cwd: Some(std::env::temp_dir().join("maitty-no-such-folder")),
            cmd: None,
        },
    )
    .unwrap();
    assert_eq!(d.workspaces.len(), 1, "ships join; they start no fleet");
    let ships: Vec<TermId> = d.workspace_panes(ws);
    assert_eq!(ships.len(), 4);
    assert!(ships.contains(&first));
    assert!(same_path(&d.terms[&ships[2]].cwd, &elsewhere));
    assert!(same_path(&d.terms[&ships[3]].cwd, &d.workspaces[0].cwd));
    assert!(d
        .command(
            0,
            Command::NewShip {
                ws: 9999,
                cwd: None,
                cmd: None
            }
        )
        .is_err());
    close(&mut d, &ships);
    let _ = std::fs::remove_dir(&elsewhere);
}

#[test]
fn a_session_moves_to_another_fleet_of_the_same_machine() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    new_fleet(&mut d, "api", vec![]).unwrap();
    new_fleet(&mut d, "web", vec![]).unwrap();
    let (from, to) = (d.workspaces[0].id, d.workspaces[1].id);
    d.command(
        0,
        Command::NewShip {
            ws: from,
            cwd: None,
            cmd: None,
        },
    )
    .unwrap();
    let ship = *d.workspace_panes(from).last().unwrap();
    d.command(
        0,
        Command::MovePane {
            term: ship,
            to: Some(to),
            name: None,
        },
    )
    .unwrap();
    assert_eq!(d.workspace_panes(from).len(), 1);
    assert!(d.workspace_panes(to).contains(&ship));
    let terms: Vec<_> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

fn prewarm(d: &mut Daemon, cmd: &str) -> anyhow::Result<bool> {
    d.command(
        0,
        Command::Prewarm {
            cwd: std::env::temp_dir(),
            cmd: cmd.into(),
        },
    )
}

fn start_in_temp(d: &mut Daemon, cmd: &str) {
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(std::env::temp_dir()),
            name: None,
            cmd: Some(cmd.into()),
        },
    )
    .unwrap();
}

#[test]
fn an_agent_kept_warm_is_hidden_until_the_session_it_was_started_for_takes_it() {
    let (mut d, _rx) = daemon();
    prewarm(&mut d, "echo warm").unwrap();
    let spare = d.warm_term().expect("a spare is running");
    assert!(d.terms[&spare].spare);
    let shown = d.snapshot();
    assert!(shown.terms.is_empty() && shown.workspaces.is_empty());

    start_in_temp(&mut d, "echo warm");
    assert_eq!(d.terms.len(), 1, "the session is the spare, not a second agent");
    assert!(d.warm_term().is_none() && !d.terms[&spare].spare);
    assert_eq!(d.workspaces[0].tabs[0].focus, spare);
    assert!(d.snapshot().terms.contains_key(&spare));
    close(&mut d, &[spare]);
}

#[test]
fn a_session_asked_for_differently_starts_its_own_and_the_spare_stops_when_dropped() {
    let (mut d, _rx) = daemon();
    prewarm(&mut d, "echo warm").unwrap();
    let spare = d.warm_term().unwrap();

    start_in_temp(&mut d, "echo other");
    assert_eq!(d.terms.len(), 2);
    assert_eq!(d.warm_term(), Some(spare), "another command is not the spare");
    let started = d.workspaces[0].tabs[0].focus;
    assert_ne!(started, spare);

    // Asking for another one stops the first; dropping stops whatever is kept.
    prewarm(&mut d, "echo next").unwrap();
    let next = d.warm_term().unwrap();
    assert!(next != spare && !d.terms.contains_key(&spare));
    d.command(0, Command::DropPrewarm).unwrap();
    assert!(d.warm_term().is_none() && !d.terms.contains_key(&next));
    assert_eq!(d.terms.len(), 1);
    close(&mut d, &[started]);
}

#[test]
fn a_spare_nobody_asks_for_again_is_stopped_and_asking_again_keeps_it() {
    let (mut d, _rx) = daemon();
    prewarm(&mut d, "echo warm").unwrap();
    let spare = d.warm_term().unwrap();

    d.age_warm(super::warm::WARM_KEEP - Duration::from_secs(5));
    d.expire_warm();
    assert_eq!(d.warm_term(), Some(spare));

    d.age_warm(super::warm::WARM_KEEP + Duration::from_secs(1));
    prewarm(&mut d, "echo warm").unwrap();
    d.expire_warm();
    assert_eq!(d.warm_term(), Some(spare), "asked for again: kept, and not started twice");
    assert_eq!(d.terms.len(), 1);

    d.age_warm(super::warm::WARM_KEEP + Duration::from_secs(1));
    d.expire_warm();
    assert!(d.warm_term().is_none() && d.terms.is_empty());
}

#[test]
fn a_spare_alerts_nobody_and_a_pane_cannot_ask_for_one() {
    let (mut d, _rx) = daemon();
    prewarm(&mut d, "echo warm").unwrap();
    let spare = d.warm_term().unwrap();
    d.terms.get_mut(&spare).unwrap().status = Status::Blocked;
    d.announce(spare, Status::Blocked);
    assert!(d.terms[&spare].alert_due.is_none(), "nothing is due for a session shown nowhere");

    let starter = pane(&mut d);
    let token = d.terms[&starter].token.clone();
    let mut from = from_pane(&mut d, 1, starter, &token);
    d.handle(Ev::Msg(
        1,
        ClientMsg::Command(Command::Prewarm {
            cwd: std::env::temp_dir(),
            cmd: "echo other".into(),
        }),
    ));
    let refused = std::iter::from_fn(|| from.try_recv().ok()).find_map(|m| match m {
        ServerMsg::Error(e) => Some(e),
        _ => None,
    });
    assert!(refused.is_some_and(|why| why.contains("Only an app window")));
    assert_eq!(d.warm_term(), Some(spare));
    d.drop_warm();
    close(&mut d, &[starter]);
}

#[test]
fn an_agent_that_ended_leaves_no_reason_for_a_status_it_no_longer_has() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    d.terms.get_mut(&id).unwrap().agent = Some("claude".into());
    d.hook(id, "claude".into(), HookStatus::Working, "PreToolUse", &[]);
    assert_eq!(d.terms[&id].status_why, "hook: PreToolUse");
    d.hook(id, "claude".into(), HookStatus::Gone, "SessionEnd", &[]);
    assert_eq!(d.terms[&id].status, Status::None);
    assert_eq!(
        d.terms[&id].status_why, "",
        "the reason was for the status it had, not for none"
    );
    close(&mut d, &[id]);
}

/// Give `term` an agent conversation, as its hooks would, on branch `main`.
fn agent_session(d: &mut Daemon, term: TermId, session: &str) {
    let t = d.terms.get_mut(&term).unwrap();
    t.agent = Some("claude".into());
    t.session = Some(session.into());
    t.label = "api work".into();
    t.head = Some(crate::gitfs::Head {
        top: t.cwd.clone(),
        branch: "main".into(),
        linked: false,
        main_root: t.cwd.clone(),
    });
}

fn closed_ids(d: &Daemon) -> Vec<String> {
    d.closed_sessions.list().iter().map(|s| s.session.clone()).collect()
}

#[test]
fn a_closed_agent_session_is_remembered_once_its_undo_window_passes() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (a, b) = two_panes(&mut d);
    agent_session(&mut d, b, "abc");
    let cwd = d.terms[&b].cwd.clone();

    d.command(0, Command::CloseSession { term: b }).unwrap();
    d.finish_closes();
    assert!(closed_ids(&d).is_empty(), "not while it can still be undone");

    closed_ago(&mut d, b, UNDO_WINDOW);
    d.finish_closes();
    let [kept] = d.closed_sessions.list() else {
        panic!("one record: {:?}", d.closed_sessions.list());
    };
    assert_eq!(
        (kept.agent.as_str(), kept.session.as_str(), &kept.cwd, kept.branch.as_deref(), kept.title.as_deref()),
        ("claude", "abc", &cwd, Some("main"), Some("api work"))
    );
    assert!(kept.at > 0);
    close(&mut d, &[a]);
}

#[test]
fn an_undone_close_leaves_no_record() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (a, b) = two_panes(&mut d);
    agent_session(&mut d, b, "abc");
    d.command(0, Command::CloseSession { term: b }).unwrap();
    closed_ago(&mut d, b, UNDO_WINDOW - Duration::from_secs(1));
    d.command(0, Command::UndoClose { term: b }).unwrap();
    d.finish_closes();
    assert!(closed_ids(&d).is_empty());
    d.terms.get_mut(&b).unwrap().session = None;
    close(&mut d, &[a, b]);
}

#[test]
fn a_closed_pane_with_no_conversation_id_leaves_no_record() {
    let (mut d, _rx) = daemon();
    let shell = pane(&mut d);
    let agent = pane(&mut d);
    d.terms.get_mut(&agent).unwrap().agent = Some("claude".into());
    d.command(0, Command::ClosePane { term: shell }).unwrap();
    d.command(0, Command::ClosePane { term: agent }).unwrap();
    assert!(closed_ids(&d).is_empty());
}

#[test]
fn an_agent_whose_process_failed_is_remembered_and_one_that_succeeded_is_not() {
    let (mut d, _rx) = daemon();
    let (failed, fine) = (pane(&mut d), pane(&mut d));
    agent_session(&mut d, failed, "failed");
    agent_session(&mut d, fine, "fine");
    d.handle(Ev::ProcessExited(fine, false));
    d.handle(Ev::ProcessExited(failed, true));
    assert_eq!(closed_ids(&d), ["failed"]);
}

#[test]
fn a_failure_reported_after_the_panes_output_ended_is_still_remembered() {
    let (mut d, _rx) = daemon();
    let (failed, fine) = (pane(&mut d), pane(&mut d));
    agent_session(&mut d, failed, "failed");
    agent_session(&mut d, fine, "fine");
    d.handle(Ev::Exited(failed));
    d.handle(Ev::Exited(fine));
    assert!(closed_ids(&d).is_empty(), "how it exited is not known yet");
    d.handle(Ev::ProcessExited(fine, false));
    d.handle(Ev::ProcessExited(failed, true));
    assert_eq!(closed_ids(&d), ["failed"]);
    assert!(d.unreaped.is_empty());
}

#[test]
fn a_person_reads_the_closed_sessions_newest_first_and_a_pane_cannot() {
    let (mut d, _rx) = daemon();
    let (older, newer, asking) = (pane(&mut d), pane(&mut d), pane(&mut d));
    agent_session(&mut d, older, "older");
    agent_session(&mut d, newer, "newer");
    d.command(0, Command::ClosePane { term: older }).unwrap();
    d.command(0, Command::ClosePane { term: newer }).unwrap();

    let (tx, mut person) = mpsc::channel(16);
    d.handle(Ev::Connected(7, tx, false, crate::protocol::Surface::Terminal, true));
    d.message(7, ClientMsg::Query(Query::ClosedSessions));
    let Ok(ServerMsg::Reply(Reply::ClosedSessions(list))) = person.try_recv() else {
        panic!("a person is answered with the list");
    };
    assert_eq!(list.iter().map(|s| s.session.as_str()).collect::<Vec<_>>(), ["newer", "older"]);

    let (tx, mut agent) = mpsc::channel(16);
    d.handle(Ev::Connected(8, tx, false, crate::protocol::Surface::Terminal, true));
    let token = d.terms[&asking].token.clone();
    d.handle(Ev::From(8, asking, token));
    d.message(8, ClientMsg::Query(Query::ClosedSessions));
    assert!(matches!(agent.try_recv(), Ok(ServerMsg::Error(_))), "folders of other work are not a pane's to read");
    close(&mut d, &[asking]);
}

#[test]
fn a_closed_pane_whose_killed_process_then_fails_is_remembered_once() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    agent_session(&mut d, t, "abc");
    d.command(0, Command::ClosePane { term: t }).unwrap();
    d.handle(Ev::ProcessExited(t, true));
    assert_eq!(closed_ids(&d), ["abc"]);
}

#[test]
fn a_process_failing_while_the_daemon_stops_is_not_remembered() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    agent_session(&mut d, t, "abc");
    d.stopping = true;
    d.handle(Ev::ProcessExited(t, true));
    assert!(closed_ids(&d).is_empty());
}

#[test]
fn an_agent_the_daemon_stopped_to_reload_is_not_remembered_when_its_restart_fails() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    agent_session(&mut d, t, "abc");
    d.cfg.shell = Some(concat!("/no/such/shell-for-", env!("APP_COMMAND")).into());
    assert!(d.reload_agent(t).is_none(), "the new pane cannot start");
    d.handle(Ev::ProcessExited(t, true));
    assert!(closed_ids(&d).is_empty());
    close(&mut d, &[t]);
}

#[test]
fn a_closed_conversation_terminal_is_remembered_once() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (a, b) = two_panes(&mut d);
    agent_session(&mut d, b, "abc");
    d.terms.get_mut(&b).unwrap().conversation_id = Some(crate::conversation::ConversationId(9));
    d.command(0, Command::CloseSession { term: b }).unwrap();
    assert!(!d.terms.contains_key(&b), "a conversation's terminal closes at once");
    assert_eq!(closed_ids(&d), ["abc"]);
    close(&mut d, &[a]);
}

#[test]
fn a_conversation_terminal_its_hooks_never_named_is_remembered_by_its_conversation() {
    use crate::conversation::{Conversation, ConversationId, ExecutionKind, NativeBinding, Provider, RecoveryState};
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let (a, b) = two_panes(&mut d);
    let cwd = d.terms[&b].cwd.clone();
    d.terms.get_mut(&b).unwrap().conversation_id = Some(ConversationId(9));
    d.conversations.push(Conversation {
        id: ConversationId(9),
        binding: NativeBinding {
            provider: Provider::Codex,
            session_id: "native-1".into(),
            account: None,
            profile: None,
            config_root: cwd.clone(),
            executable: PathBuf::from("codex"),
            cwd: cwd.clone(),
            worktree: None,
        },
        execution: ExecutionKind::Native,
        generation: 1,
        state: RecoveryState::Running,
        settings: None,
        awaiting_first_user_message: false,
        title: Some("parser fix".into()),
        archived: false,
    });
    d.command(0, Command::CloseSession { term: b }).unwrap();
    let [kept] = d.closed_sessions.list() else {
        panic!("one record: {:?}", d.closed_sessions.list());
    };
    assert_eq!(
        (kept.agent.as_str(), kept.session.as_str(), kept.title.as_deref(), &kept.cwd),
        ("codex", "native-1", Some("parser fix"), &cwd)
    );
    close(&mut d, &[a]);
}
