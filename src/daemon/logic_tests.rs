//! The daemon's own logic, driven through a real daemon with real panes (plain shells).

use super::*;
use std::time::Duration;

fn daemon() -> (Daemon, mpsc::Receiver<Ev>) {
    let (tx, rx) = mpsc::channel(4096);
    let mut d = Daemon::new(Config::default(), tx);
    // Its own folder (made only if something is written to it), so tests never share check tails.
    d.check_dir = persist::scratch_dir().path().to_path_buf();
    (d, rx)
}

fn pane(d: &mut Daemon) -> TermId {
    d.spawn(None, &std::env::temp_dir(), 80, 24).expect("a shell pane")
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
        term, session: Some(session.into()), usage: None,
        subagents: vec![(format!("{session}-child"), name.into())],
        subagent_activity: Vec::new(),
    };
    d.codex_usage(vec![facts(a, "root-a", "planner"), facts(b, "root-b", "reviewer")], None);
    assert_eq!(d.terms[&a].subagents, vec![("root-a-child".into(), "planner".into())]);
    assert_eq!(d.terms[&b].subagents, vec![("root-b-child".into(), "reviewer".into())]);
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
    assert!(d.terms[&t].subagent_seen.is_some(), "disk polling supplies helper liveness");
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Working, "the parent remains visibly active without hooks");

    d.codex_usage(vec![facts(Vec::new())], None);
    assert!(d.terms[&t].subagent_seen.is_none(), "a completed helper clears its liveness timestamp");
    d.terms.get_mut(&t).unwrap().last_output = Instant::now() - Duration::from_secs(10);
    d.update_statuses();
    assert_ne!(d.terms[&t].status, Status::Working, "the parent settles after the helper disappears");
    close(&mut d, &[t]);
}

#[test]
fn codex_live_helper_hooks_attach_the_first_child_and_outvote_stale_disk_data() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    let mut start = report(t, &secret, 0, HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { agent, session, subagent, .. } = &mut start {
        *agent = "codex".into();
        *session = Some("parent-thread".into());
        *subagent = Some(crate::protocol::Subagent { id: "child-thread".into(), kind: "reviewer".into(), start: true, reported_at: 0, task: None, model: None, effort: None, activity: None });
    }
    d.apply_hook(start, Some(true), Vec::new());
    assert_eq!(d.terms[&t].session.as_deref(), Some("parent-thread"));
    assert_eq!(d.terms[&t].subagents, vec![("child-thread".into(), "reviewer".into())]);
    assert!(d.terms[&t].subagent_hooks_seen);

    let facts = |subagents| usage::CodexPaneFacts {
        term: t,
        session: Some("parent-thread".into()),
        usage: None,
        subagents,
        subagent_activity: Vec::new(),
    };
    d.codex_usage(vec![facts(Vec::new())], None);
    assert_eq!(d.terms[&t].subagents.len(), 1, "an empty disk snapshot cannot erase the live hook entry");

    let mut stop = report(t, &secret, 0, HookStatus::Same, "SubagentStop");
    if let ClientMsg::Hook { agent, session, subagent, .. } = &mut stop {
        *agent = "codex".into();
        *session = Some("parent-thread".into());
        *subagent = Some(crate::protocol::Subagent { id: "child-thread".into(), kind: "reviewer".into(), start: false, reported_at: 0, task: None, model: None, effort: None, activity: None });
    }
    d.apply_hook(stop, Some(true), Vec::new());
    assert!(d.terms[&t].subagents.is_empty());
    d.codex_usage(vec![facts(vec![("child-thread".into(), "stale reviewer".into())])], None);
    assert!(d.terms[&t].subagents.is_empty(), "stale disk data cannot resurrect a stopped helper");
    close(&mut d, &[t]);
}

#[test]
fn status_reports_need_the_pane_secret() {
    let (mut d, mut rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    d.message(0, report(t, "not-the-secret", 0, HookStatus::Blocked, "PreToolUse"));
    d.message(0, report(t, "", 0, HookStatus::Blocked, "PreToolUse"));
    assert_eq!(settle_within(&mut d, &mut rx, 1, Duration::from_secs(1)), 0, "a report without the secret is never even checked");
    assert_eq!(d.terms[&t].status, Status::None);
    d.message(0, report(t, &secret, 0, HookStatus::Blocked, "PreToolUse"));
    assert_eq!(settle(&mut d, &mut rx, 1), 1);
    assert_eq!(d.terms[&t].status, Status::Blocked, "with the secret it counts");
    assert_eq!(d.terms[&t].status_why, "hook: PreToolUse", "and it says what set it");
    close(&mut d, &[t]);
}

#[test]
fn a_report_traced_outside_the_pane_is_dropped() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    // The hook thread found its process chain doesn't lead to the pane (a desktop app that
    // inherited the pane's environment, say): the report is dropped, secret or not.
    d.apply_hook(report(t, &secret, 4242, HookStatus::Blocked, "PreToolUse"), Some(false), Vec::new());
    assert_eq!(d.terms[&t].status, Status::None);
    // Traced to the pane: it counts, and the chain is remembered for next time.
    d.apply_hook(report(t, &secret, 4242, HookStatus::Blocked, "PreToolUse"), Some(true), vec![4242, 4243]);
    assert_eq!(d.terms[&t].status, Status::Blocked);
    assert!(d.terms[&t].trusted.contains(&4243));
    close(&mut d, &[t]);
}

#[test]
fn a_restore_never_starts_an_agents_task_over() {
    let (d, _rx) = daemon();
    let saved = |agent: Option<&str>, cmd: &str| persist::SavedPane { agent: agent.map(String::from), cmd: Some(cmd.into()), ..Default::default() };
    // A helper saved without its agent (it lost it asleep): its command still names codex,
    // so it's resumed, not handed its task again.
    let lost = saved(None, r#"codex "Fix the starting kit: give each faction its units""#);
    assert_eq!(d.agent_in_cmd(lost.cmd.as_deref().unwrap()).as_deref(), Some("codex"));
    assert_eq!(d.restore_cmd(&lost), None, "no session to resume: the agent isn't run again");
    let named = persist::SavedPane { agent: Some("codex".into()), ..lost.clone() };
    assert_eq!(d.restore_cmd(&named).as_deref(), Some("codex resume --last"), "known: resumed");
    // By path, quoted, with .exe; an agent with no way to resume opens as a shell.
    assert_eq!(d.agent_in_cmd(r#""C:\Users\me\.local\bin\claude.exe" --model opus "do it""#).as_deref(), Some("claude"));
    assert_eq!(d.restore_cmd(&saved(Some("gemini"), r#"gemini "write the docs""#)), None);
    // Anything else still comes back running what it ran.
    assert_eq!(d.agent_in_cmd("npm run dev"), None);
    assert_eq!(d.restore_cmd(&saved(None, "npm run dev")).as_deref(), Some("npm run dev"));
}

#[test]
fn a_session_keeps_its_account_through_sleep_and_restart() {
    let (mut d, _rx) = daemon();
    d.cfg.quick.agents.push(crate::config::QuickAgent { name: "claude work".into(), command: "claude {prompt}".into(), env: [("CLAUDE_CONFIG_DIR".to_string(), "~/.claude-work".to_string())].into(), ..Default::default() });
    let wrapped = d.cfg.as_account("claude work", "cmd /c exit");
    // Started as the account: the command line says so, and the pane remembers.
    let t = d.spawn(Some(&wrapped), &std::env::temp_dir(), 80, 24).expect("it starts");
    assert_eq!(d.terms[&t].account.as_deref(), Some("claude work"));
    let plain = pane(&mut d);
    assert_eq!(d.terms[&plain].account, None);
    // Resumed (after sleep, or a move): under the same account, where its conversation is.
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.session) = (Some("claude".into()), Some("abc123".into()));
    }
    let resume = d.resume_cmd(&d.terms[&t]).expect("claude can be resumed");
    assert!(resume.starts_with("seshi account ") && resume.contains("claude work") && resume.contains("abc123"), "{resume}");
    // And after the server restarts.
    let saved = d.saved();
    assert!(saved.workspaces.is_empty(), "these panes are in no workspace");
    let pane = persist::SavedPane { agent: Some("claude".into()), session: Some("abc123".into()), account: Some("claude work".into()), ..Default::default() };
    let again = d.restore_cmd(&pane).expect("restored as a resume");
    assert_eq!(again, resume);
    let old: persist::SavedPane = serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":"claude","session":"abc123","aside":0}"#).unwrap();
    assert!(!d.restore_cmd(&old).unwrap().starts_with("seshi account"), "one saved before accounts resumes as itself");
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
    d.command(0, Command::QueueFollowUp { term: t, text: "  then run the tests ".into() }).unwrap();
    d.command(0, Command::QueueFollowUp { term: t, text: "and open a PR".into() }).unwrap();
    assert_eq!(queued(&d), ["then run the tests", "and open a PR"], "kept by the server, in order");
    assert!(d.command(0, Command::QueueFollowUp { term: t, text: "  ".into() }).is_err(), "nothing to queue");
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
    assert_eq!(queued(&d).len(), 1, "the next waits for the turn that one starts");
    // That turn starts and ends: the next goes.
    d.set_status(t, Status::Working, "test");
    d.set_status(t, Status::Done, "test");
    d.update_statuses();
    assert!(queued(&d).is_empty());
    // Dropping what's queued; a shell takes none.
    d.set_status(t, Status::Working, "test");
    d.command(0, Command::QueueFollowUp { term: t, text: "never mind".into() }).unwrap();
    d.command(0, Command::ClearQueue { term: t }).unwrap();
    assert!(queued(&d).is_empty());
    d.terms.get_mut(&t).unwrap().agent = None;
    assert!(d.command(0, Command::QueueFollowUp { term: t, text: "ls".into() }).is_err());
    close(&mut d, &[t]);
}

#[test]
fn a_pane_started_as_an_agent_keeps_its_name_while_the_agent_starts() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    // Woken (or moved, or restored): it is claude, with a conversation to resume, before
    // claude's own process exists.
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.session, p.summary) = (Some("claude".into()), Some("abc".into()), "the task".into());
    }
    let nothing_yet = |term| scan::Found { term, process: "powershell".into(), agent: None, cwd: None, agent_cwd: None, mem: 0, cpu: 0.0, remote: None };
    d.handle(Ev::Scan(vec![nothing_yet(t)]));
    let p = &d.terms[&t];
    assert_eq!((p.agent.as_deref(), p.session.as_deref(), p.summary.as_str()), (Some("claude"), Some("abc"), "the task"), "still claude, conversation and all");
    // Asleep, long after: there's no process to find, and it's still claude.
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.asleep, p.born) = (true, Instant::now() - super::AGENT_SHOWS_WITHIN - Duration::from_secs(1));
    }
    d.handle(Ev::Scan(vec![nothing_yet(t)]));
    assert_eq!((d.terms[&t].agent.as_deref(), d.terms[&t].session.as_deref()), (Some("claude"), Some("abc")), "a sleeping agent stays an agent");
    d.terms.get_mut(&t).unwrap().asleep = false;
    // Long past the time an agent takes to start, with still no agent there: it's a shell.
    d.terms.get_mut(&t).unwrap().born = Instant::now() - super::AGENT_SHOWS_WITHIN - Duration::from_secs(1);
    d.handle(Ev::Scan(vec![nothing_yet(t)]));
    assert_eq!(d.terms[&t].agent, None);
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
    assert_eq!(d.terms[&t].status, Status::Working, "still working: its own agent is");
    assert!(d.terms[&t].done_held.is_some(), "and held, so that agent finishing ends it");
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Working, "steady: it doesn't flip back and forth");
    // The background agent finishes: now the turn is over.
    let secret = d.terms[&t].token.clone();
    let mut stop = report(t, &secret, 0, HookStatus::Same, "SubagentStop");
    if let ClientMsg::Hook { subagent, .. } = &mut stop {
        *subagent = Some(crate::protocol::Subagent { id: "a1".into(), kind: "general-purpose".into(), start: false, reported_at: 0, task: None, model: None, effort: None, activity: None });
    }
    d.apply_hook(stop, Some(true), Vec::new());
    assert!(d.terms[&t].subagents.is_empty());
    assert_eq!(d.terms[&t].status, Status::Done, "its last one finished: done");
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done, "and it stays done");
    // One that went silent long ago (it died without saying so) doesn't hold it working.
    {
        let p = d.terms.get_mut(&t).unwrap();
        p.status = Status::Idle;
        p.subagents.push(("a2".into(), "general-purpose".into()));
        p.subagent_seen = Some(Instant::now() - super::status::SUBAGENTS_QUIET_AT_MOST - Duration::from_secs(1));
    }
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Idle);
    close(&mut d, &[t]);
}

#[test]
fn only_so_many_helpers_work_at_once() {
    let (mut d, _rx) = daemon();
    d.cfg.mcp.helpers_at_once = 2;
    let here = Some(std::env::temp_dir());
    d.command(0, Command::NewWorkspace { cwd: here.clone(), name: None, cmd: None }).unwrap();
    let commander = d.workspaces[0].tabs[0].focus;
    let token = d.terms[&commander].token.clone();
    let mut c = from_pane(&mut d, 1, commander, &token);
    let start = |d: &mut Daemon| d.handle(Ev::Msg(1, ClientMsg::Command(Command::NewWorkspace { cwd: Some(std::env::temp_dir()), name: None, cmd: None })));
    let refused = |c: &mut mpsc::Receiver<ServerMsg>| std::iter::from_fn(|| c.try_recv().ok()).find_map(|m| if let ServerMsg::Error(e) = m { Some(e) } else { None });
    // Two helpers, just started: both count as working until their agents report.
    start(&mut d);
    start(&mut d);
    assert_eq!(d.terms.len(), 3);
    assert!(refused(&mut c).is_none());
    // A third is refused, and told what to do.
    start(&mut d);
    let why = refused(&mut c).expect("the third is refused");
    assert!(d.terms.len() == 3 && why.contains("2 helpers are already working") && why.contains("Wait for one to finish"), "{why}");
    // One finishes (and has been up a while): there's room again.
    let helpers: Vec<TermId> = d.terms.keys().copied().filter(|t| *t != commander).collect();
    let done = d.terms.get_mut(&helpers[0]).unwrap();
    (done.status, done.born) = (Status::Done, Instant::now() - super::HELPER_SETTLES - Duration::from_secs(1));
    start(&mut d);
    assert_eq!(d.terms.len(), 4, "a helper finished: the next may start");
    // What you start yourself never counts, and never waits.
    d.command(0, Command::NewWorkspace { cwd: here.clone(), name: None, cmd: None }).unwrap();
    assert_eq!(d.terms.len(), 5);
    // No limit: as many as asked for.
    d.cfg.mcp.helpers_at_once = 0;
    start(&mut d);
    assert_eq!(d.terms.len(), 6);
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
    d.terms.get_mut(&term).unwrap().alert_due = Some((Status::Blocked, Instant::now() - Duration::from_millis(1)));
    d.settled_alerts();
    assert!(d.terms[&term].alerted.is_none());
    // A real question remains red and gets one request sound after settling.
    d.set_status(term, Status::Blocked, "test");
    d.terms.get_mut(&term).unwrap().alert_due.as_mut().unwrap().1 = Instant::now() - Duration::from_millis(1);
    d.settled_alerts();
    assert_eq!(d.terms[&term].alerted.unwrap().0, Status::Blocked);
    close(&mut d, &[term]);
}

#[test]
fn with_no_window_open_finished_waits_and_a_helper_stays_quiet() {
    let (mut d, _rx) = daemon();
    let (commander, helper) = (pane(&mut d), pane(&mut d));
    for t in [commander, helper] {
        d.terms.get_mut(&t).unwrap().agent = Some("claude".into());
    }
    let rang = |d: &Daemon, t: TermId| d.terms[&t].alerted.map(|(s, _)| s);
    let overdue = |d: &mut Daemon, t: TermId| d.terms.get_mut(&t).unwrap().alert_due.as_mut().unwrap().1 = Instant::now() - Duration::from_millis(1);
    // Finished: not said yet.
    d.set_status(commander, Status::Working, "test");
    d.set_status(commander, Status::Done, "test");
    assert!(rang(&d, commander).is_none() && d.terms[&commander].alert_due.is_some(), "it waits");
    // Back to work before the wait is over: never said.
    d.set_status(commander, Status::Working, "test");
    assert!(d.terms[&commander].alert_due.is_none());
    d.settled_alerts();
    assert!(rang(&d, commander).is_none());
    // Finished and still finished: said.
    d.set_status(commander, Status::Done, "test");
    overdue(&mut d, commander);
    d.settled_alerts();
    assert_eq!(rang(&d, commander), Some(Status::Done));
    // A helper's finish is its commander's news.
    d.terms.get_mut(&helper).unwrap().started_by = Some(commander);
    d.set_status(helper, Status::Working, "test");
    d.set_status(helper, Status::Done, "test");
    assert!(d.terms[&helper].alert_due.is_none() && rang(&d, helper).is_none());
    // A stable question rings for helpers too.
    d.set_status(helper, Status::Blocked, "test");
    assert!(rang(&d, helper).is_none());
    overdue(&mut d, helper);
    d.settled_alerts();
    assert_eq!(rang(&d, helper), Some(Status::Blocked));
    close(&mut d, &[commander, helper]);
}

#[test]
fn a_clean_up_only_takes_worktrees_nothing_runs_in() {
    let (mut d, _rx) = daemon();
    let root = std::env::temp_dir().join(format!("seshi-tidy-plan-{}", std::process::id()));
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
    git(&repo, &["worktree", "add", "-q", "-b", "idle", idle.to_str().unwrap()]);
    git(&repo, &["worktree", "add", "-q", "-b", "busy", busy.to_str().unwrap()]);
    // A session in one of them.
    let t = d.spawn(None, &busy, 80, 24).expect("a shell there");
    d.terms.get_mut(&t).unwrap().refresh_head();
    assert!(d.runs_in(&crate::gitfs::head(&busy).unwrap().top) && !d.runs_in(&crate::gitfs::head(&idle).unwrap().top));
    d.terms.get_mut(&t).unwrap().head = None;
    assert!(d.runs_in(&busy), "a restored pane protects its checkout before git detection completes");
    let (go, stay) = d.tidy_plan(vec![(idle.clone(), false), (busy.clone(), true), (repo.clone(), false), (root.join("nowhere"), false)]);
    assert_eq!(go.len(), 1, "only the idle one goes: {go:?}");
    assert!(same_path(&go[0].0, &idle) && !go[0].1);
    let why: Vec<(&str, &str)> = stay.iter().map(|(n, w)| (n.as_str(), w.as_str())).collect();
    assert_eq!(why, [("busy", "something is running in it"), ("repo", "it isn't a worktree"), ("nowhere", "it isn't a worktree")], "the main folder is never one");
    let sibling = d.spawn(None, &busy, 80, 24).expect("a second pane in the checkout");
    d.terms.get_mut(&t).unwrap().refresh_head();
    d.terms.get_mut(&sibling).unwrap().head = None;
    d.made_worktrees.push(busy.clone());
    let tops = d.worktrees_of(&[t]);
    d.close_term(t);
    d.cleanup_worktrees(EVERYONE, tops);
    assert!(busy.is_dir(), "closing one pane must not delete a worktree while its sibling is still starting");
    assert!(d.terms.contains_key(&sibling), "the sibling pane remains alive");
    close(&mut d, &[sibling]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_clean_up_says_what_went_and_what_stayed() {
    let note = super::tidied_note;
    assert_eq!(note(&[("a".into(), false)], &[]), "removed 1 worktree");
    assert_eq!(note(&[("a".into(), true), ("b".into(), false), ("c".into(), true)], &[]), "removed 3 worktrees (2 merged branches went too)");
    assert_eq!(note(&[("a".into(), true)], &[("rate".into(), "something is running in it".into())]), "removed 1 worktree (1 merged branch went too); kept rate: something is running in it");
    assert_eq!(note(&[], &[]), "nothing to clean up");
}

#[test]
fn a_finish_you_have_seen_is_not_announced_again() {
    let (mut d, _rx) = daemon();
    d.command(0, Command::NewWorkspace { cwd: Some(std::env::temp_dir()), name: None, cmd: None }).unwrap();
    let t = d.workspaces[0].tabs[0].focus;
    let secret = d.terms[&t].token.clone();
    let say = |d: &mut Daemon, status, event: &str| d.apply_hook(report(t, &secret, 0, status, event), Some(true), Vec::new());
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
        ("amp", "seshi - amp - project"),
        ("grok", "grok"),
    ] {
        let (mut d, _rx) = daemon();
        let t = pane(&mut d);
        {
            let p = d.terms.get_mut(&t).unwrap();
            p.agent = Some(agent.into());
            p.hooked = true;
            p.status = Status::Working;
            p.subagents.push(("child-1".into(), "general-purpose".into()));
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
        }

        d.update_statuses();

        assert_eq!(
            d.terms[&t].status,
            Status::Done,
            "{agent}: explicit idle screen evidence ends a missed turn"
        );
        assert!(d.terms[&t].subagents.is_empty(), "{agent}: clear stale child runners");
        assert_eq!(
            d.terms[&t].subagent_activity[0].state,
            SubagentState::ConnectionLost,
            "{agent}: retain lifecycle history without showing the child as active"
        );
        close(&mut d, &[t]);
    }
}

#[test]
fn a_permission_ask_settled_without_you_stops_needing_you() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    d.apply_hook(report(t, &secret, 0, HookStatus::Blocked, "Notification:permission_prompt"), None, Vec::new());
    assert_eq!(d.terms[&t].status, Status::Blocked);
    let long_ago = Instant::now() - super::status::UNASKED_QUESTION_GONE_AFTER - Duration::from_secs(1);
    // The question still on screen: it still needs you.
    d.terms.get_mut(&t).unwrap().parser.process(&b"\r\n".repeat(30));
    d.terms.get_mut(&t).unwrap().parser.process(b"\r\n Do you want to proceed?\r\n \xe2\x9d\xaf 1. Yes\r\n   2. No\r\n");
    d.terms.get_mut(&t).unwrap().blocked_at = Some(long_ago);
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Blocked, "asking on screen");
    // Auto mode allowed it and the screen moved on, with no hook to say so: over.
    d.terms.get_mut(&t).unwrap().parser.process(b"\x1b[2J\x1b[H* Waiting for 1 dynamic workflow to finish\r\n");
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Done, "nothing asked on screen");
    close(&mut d, &[t]);
}

#[test]
fn status_reports_apply_in_the_order_they_came() {
    let (mut d, mut rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    d.message(0, report(t, &secret, 0, HookStatus::Working, "UserPromptSubmit"));
    d.message(0, report(t, &secret, 0, HookStatus::Blocked, "PreToolUse"));
    d.message(0, report(t, &secret, 0, HookStatus::Done, "Stop"));
    assert_eq!(settle(&mut d, &mut rx, 3), 3);
    assert_eq!(d.terms[&t].status, Status::Done, "the last report wins: done (no one is looking)");
    close(&mut d, &[t]);
}

#[test]
fn waking_a_sleeping_session_keeps_what_you_called_it() {
    let (mut d, _rx) = daemon();
    d.command(0, Command::NewWorkspace { cwd: Some(std::env::temp_dir()), name: None, cmd: None }).unwrap();
    let old = d.workspaces[0].tabs[0].focus;
    d.command(0, Command::RenamePane { term: old, name: "The Planner".into() }).unwrap();
    let t = d.terms.get_mut(&old).unwrap();
    (t.name, t.model, t.asleep) = ("roadmap".into(), "opus".into(), true);
    t.kill_tree();
    let new = d.wake(old).expect("it wakes");
    assert!(new != old && !d.terms.contains_key(&old), "a new process in its place");
    let t = &d.terms[&new];
    assert_eq!((t.label.as_str(), t.name.as_str(), t.model.as_str()), ("The Planner", "roadmap", "opus"), "the same session to you");
    assert_eq!(d.workspaces[0].tabs[0].focus, new, "in the same spot");
    close(&mut d, &[new]);
}

/// A connection acting from `term`, as its agent's `seshi` commands do.
fn from_pane(d: &mut Daemon, client: ClientId, term: TermId, token: &str) -> mpsc::Receiver<ServerMsg> {
    let (tx, rx) = mpsc::channel(64);
    d.handle(Ev::Connected(client, tx, false));
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
    d.command(0, Command::NewWorkspace { cwd: here.clone(), name: None, cmd: None }).unwrap();
    let commander = d.workspaces[0].tabs[0].focus;
    assert_eq!(d.terms[&commander].started_by, None, "you started it");
    let token = d.terms[&commander].token.clone();
    let _c = from_pane(&mut d, 1, commander, &token);
    let mut made = vec![commander];
    d.handle(Ev::Msg(1, ClientMsg::Command(Command::NewWorkspace { cwd: here.clone(), name: None, cmd: None })));
    made.push(newest(&d));
    let ws = d.workspaces[0].id;
    d.handle(Ev::Msg(1, ClientMsg::Command(Command::NewTab { ws, name: None, cmd: None })));
    made.push(newest(&d));
    d.handle(Ev::Msg(1, ClientMsg::Command(Command::Split { term: commander, dir: crate::layout::Dir::Right, cmd: None, cwd: None })));
    made.push(newest(&d));
    assert_eq!(made.len(), 4, "a workspace, a tab and a split");
    for t in &made[1..] {
        assert_eq!(d.terms[t].started_by, Some(commander), "started from the commander's pane");
    }
    assert_eq!(d.snapshot().terms[&made[1]].started_by, Some(commander), "and clients are told");
    close(&mut d, &made);
}

#[test]
fn only_the_pane_itself_can_be_named_as_the_starter() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let _c = from_pane(&mut d, 1, t, "not-the-secret");
    d.handle(Ev::Msg(1, ClientMsg::Command(Command::NewWorkspace { cwd: Some(std::env::temp_dir()), name: None, cmd: None })));
    let new = newest(&d);
    assert!(new != t, "it may still start a session");
    assert_eq!(d.terms[&new].started_by, None, "but not in that pane's name");
    close(&mut d, &[t, new]);
}

#[test]
fn a_worktree_start_keeps_its_starter_after_the_caller_has_gone() {
    let (mut d, _rx) = daemon();
    let commander = pane(&mut d);
    let dir = std::env::temp_dir().join(format!("seshi-started-by-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // The worktree was asked for from the commander's pane; that connection closed while git ran.
    d.pending_ops += 1;
    d.handle(Ev::WorktreeCreated { client: 9, branch: "helper".into(), cmd: None, split: None, by: Some(commander), set_up: false, result: Ok((dir.clone(), String::new())) });
    let helper = newest(&d);
    assert_eq!(d.terms[&helper].started_by, Some(commander));
    // The next session you start yourself isn't the commander's.
    d.command(0, Command::NewWorkspace { cwd: Some(std::env::temp_dir()), name: None, cmd: None }).unwrap();
    let yours = newest(&d);
    assert_eq!(d.terms[&yours].started_by, None);
    close(&mut d, &[commander, helper, yours]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_replaced_session_keeps_its_place_in_a_fleet() {
    let (mut d, _rx) = daemon();
    let here = Some(std::env::temp_dir());
    for _ in 0..3 {
        d.command(0, Command::NewWorkspace { cwd: here.clone(), name: None, cmd: None }).unwrap();
    }
    let [lead, commander, helper] = [0, 1, 2].map(|i| d.workspaces[i].tabs[0].focus);
    d.terms.get_mut(&commander).unwrap().started_by = Some(lead);
    d.terms.get_mut(&helper).unwrap().started_by = Some(commander);
    // Woken from sleep: a new process, the same session.
    let t = d.terms.get_mut(&commander).unwrap();
    t.asleep = true;
    t.kill_tree();
    let woken = d.wake(commander).expect("it wakes");
    assert_eq!(d.terms[&woken].started_by, Some(lead), "still under the one that started it");
    assert_eq!(d.terms[&helper].started_by, Some(woken), "and its helper is still its own");
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
    d.command(0, Command::NewWorkspace { cwd: Some(dir.clone()), name: None, cmd: None }).unwrap();
    assert_eq!(d.workspaces.len(), 1);
    let first = d.workspaces[0].tabs[0].focus;
    d.command(0, Command::Split { term: first, dir: crate::layout::Dir::Right, cmd: None, cwd: None }).unwrap();
    let leaves = d.workspaces[0].tabs[0].layout.leaves();
    assert_eq!(leaves.len(), 2);
    let second = leaves.into_iter().find(|t| *t != first).unwrap();
    // Out of the tab, still running.
    d.detach(first);
    assert!(d.terms.contains_key(&first), "detach keeps the terminal");
    assert_eq!(d.workspaces[0].tabs[0].layout.leaves(), vec![second]);
    assert_eq!(d.workspaces[0].tabs[0].focus, second, "focus moves to what's left");
    // The last pane goes: the tab and the workspace go with it.
    d.remove_term(second);
    assert!(d.workspaces.is_empty(), "an emptied workspace disappears");
    assert!(!d.terms.contains_key(&second));
    close(&mut d, &[first]);
}

#[test]
fn a_spare_fits_only_its_repo_and_its_agent() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let repo = std::env::temp_dir().join("seshi-spare-repo");
    d.cfg.worktree.prewarm = "claude".into();
    d.spare = Some((repo.clone(), repo.join("wt"), t));
    assert_eq!(d.spare_fits(&repo, "claude").as_deref(), Some(""), "the agent alone");
    let with_task = format!("claude {}", d.cfg.quote_for_shell("fix the flaky test"));
    assert_eq!(d.spare_fits(&repo, &with_task).as_deref(), Some("fix the flaky test"), "the agent and a task");
    assert_eq!(d.spare_fits(&repo, "codex"), None, "another agent");
    assert_eq!(d.spare_fits(&repo, "claude --model opus"), None, "other options");
    assert_eq!(d.spare_fits(&std::env::temp_dir().join("other"), "claude"), None, "another repo");
    d.cfg.worktree.prewarm.clear();
    assert_eq!(d.spare_fits(&repo, "claude"), None, "prewarming off");
    d.spare = None;
    close(&mut d, &[t]);
}

#[test]
fn a_fleet_comes_back_from_a_restart() {
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
    // Saved ids: 70 is the commander, 71 its helper, 72 a helper whose commander (99) is gone.
    // The helpers are saved ahead of the commander, so they come back before it does.
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
            panes: [(id, persist::SavedPane { cwd: Some(dir.clone()), started_by: by, subagent_activity: if id == 71 { vec![child_activity.clone(), active_activity.clone()] } else { Vec::new() }, ..Default::default() })].into(),
        }],
        active_tab: 0,
    };
    d.restore(persist::Saved { workspaces: vec![ws(71, Some(70)), ws(72, Some(99)), ws(70, None)], active: 0, made_worktrees: Vec::new() });
    let [helper, orphan, commander] = [0, 1, 2].map(|i| d.workspaces[i].tabs[0].focus);
    assert!(commander != 70, "the commander has this run's id");
    assert_eq!(d.terms[&helper].started_by, Some(commander), "its helper found it again");
    assert_eq!(d.terms[&helper].subagent_activity[0], child_activity, "completed child and result survive restoration");
    assert_eq!(d.terms[&helper].subagent_activity[1].id, active_activity.id);
    assert_eq!(d.terms[&helper].subagent_activity[1].state, SubagentState::ConnectionLost, "active child is unconfirmed after a daemon restart");
    assert_eq!(d.terms[&orphan].started_by, None, "a helper whose commander didn't come back is on its own");
    assert_eq!(d.terms[&commander].started_by, None);
    // And it's saved that way again.
    let saved = d.saved();
    assert_eq!(saved.workspaces[0].tabs[0].panes[&helper].started_by, Some(commander));
    assert_eq!(saved.workspaces[0].tabs[0].panes[&helper].subagent_activity.len(), 2);
    // A session saved before fleets existed still loads.
    let old: persist::SavedPane = serde_json::from_str(r#"{"cwd":null,"cmd":null,"agent":null,"session":null}"#).expect("an older pane");
    assert_eq!(old.started_by, None);
    assert!(old.subagent_activity.is_empty(), "panes saved before child activity still load");
    close(&mut d, &[helper, orphan, commander]);
}

#[test]
fn restore_gives_panes_new_ids_and_keeps_their_layout() {
    let (mut d, _rx) = daemon();
    let dir = std::env::temp_dir();
    let mut layout = Node::Leaf(70);
    layout.split(70, crate::layout::Dir::Right, 71);
    layout.split(71, crate::layout::Dir::Down, 72);
    let mut panes = std::collections::BTreeMap::new();
    panes.insert(70, persist::SavedPane { cwd: Some(dir.clone()), name: "left one".into(), label: "Map Gen".into(), ..Default::default() });
    panes.insert(71, persist::SavedPane { cwd: Some(dir.clone()), model: "opus".into(), ..Default::default() });
    panes.insert(72, persist::SavedPane { cwd: Some(dir.clone()), ..Default::default() });
    let saved = persist::Saved {
        workspaces: vec![persist::SavedWs {
            name: "api".into(),
            cwd: dir.clone(),
            worktree: false,
            color: None,
            group: None,
            tabs: vec![persist::SavedTab { name: "main".into(), layout, focus: 71, panes }],
            active_tab: 0,
        }],
        active: 0,
        made_worktrees: Vec::new(),
    };
    d.restore(saved);
    assert_eq!(d.workspaces.len(), 1);
    let tab = &d.workspaces[0].tabs[0];
    let leaves = tab.layout.leaves();
    assert_eq!(leaves.len(), 3, "all three panes came back");
    assert!(leaves.iter().all(|t| d.terms.contains_key(t) && ![70, 71, 72].contains(t)), "with this run's ids");
    let (left, right, below) = (leaves[0], leaves[1], leaves[2]);
    assert_eq!(tab.focus, right, "focus follows the pane it was on");
    let rects = tab.layout.rects(ratatui::layout::Rect::new(0, 0, 120, 40));
    let rect = |id| rects.iter().find(|(term, _)| *term == id).unwrap().1;
    assert_eq!(rect(below).x, rect(right).x, "restoration retains the right-side column");
    assert!(rect(below).y > rect(right).y, "restoration retains the down split");
    assert_eq!(d.terms[&left].first_prompt, "left one");
    assert_eq!(d.terms[&left].label, "Map Gen", "the name you gave it comes back");
    assert_eq!(d.saved().workspaces[0].tabs[0].panes[&left].label, "Map Gen", "and is saved again");
    assert_eq!(d.terms[&right].model, "opus");
    close(&mut d, &leaves);
}

#[test]
fn worktree_names_that_look_like_options_are_refused() {
    let dir = std::env::temp_dir();
    for bad in ["", "-x", "a b", "--force"] {
        assert!(git::create_worktree(&dir, bad, None, "{repo_parent}/{repo}-worktrees/{branch}").is_err(), "branch {bad:?}");
    }
}

#[test]
fn an_agent_typed_into_a_shell_gets_its_own_worktree() {
    let tmp = std::env::temp_dir().join(format!("seshi-agent-wt-{}", std::process::id()));
    let repo = tmp.join("shop");
    std::fs::create_dir_all(repo.join("web")).unwrap();
    let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(&repo).args(args).output().unwrap().status.success(), "git {args:?}");
    git(&["init", "-q"]);
    git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]);
    std::fs::write(repo.join("web").join("keep"), "").unwrap();
    git(&["add", "."]);
    git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "web"]);
    let made = worktrees::agent_worktree(&repo.join("web"), "{repo_parent}/{repo}-worktrees/{branch}").unwrap();
    assert!(made.ends_with("web") && made.is_dir(), "in the same subfolder of the new worktree: {}", made.display());
    let head = crate::gitfs::head(&made).unwrap();
    assert!(head.linked && crate::gitfs::WT_NAMES.contains(&head.branch.as_str()), "a linked worktree on a made-up branch: {head:?}");
    let again = worktrees::agent_worktree(&repo, "{repo_parent}/{repo}-worktrees/{branch}").unwrap();
    let other = crate::gitfs::head(&again).unwrap().branch;
    assert_ne!(other, head.branch, "the next one gets its own branch");
    // Closing them: a merged branch goes with its worktree, one with work of its own stays.
    let wt = |dir: &std::path::Path, args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(dir).args(args).output().unwrap().status.success(), "git {args:?}");
    wt(&again, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "work"]);
    git::remove_worktree(&crate::gitfs::head(&made).unwrap().top, false, false).unwrap();
    git::remove_worktree(&again, false, false).unwrap();
    assert!(git::delete_if_merged(&repo, &head.branch), "nothing on it that main doesn't have: deleted");
    assert!(!git::delete_if_merged(&repo, &other), "a commit main doesn't have: kept");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn only_a_pane_shell_asks_for_an_agent_worktree() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(1, tx, false));
    d.handle(Ev::Msg(1, ClientMsg::Command(Command::AgentWorktree { dir: std::env::current_dir().unwrap() })));
    assert!(matches!(crx.try_recv(), Ok(ServerMsg::Reply(Reply::Text(p))) if p.is_empty()), "not from a pane: start where it is");
    d.terms.get_mut(&t).unwrap().agent = Some("claude".into());
    let (tx, mut crx) = mpsc::channel(64);
    d.handle(Ev::Connected(2, tx, false));
    let token = d.terms[&t].token.clone();
    d.handle(Ev::From(2, t, token));
    d.handle(Ev::Msg(2, ClientMsg::Command(Command::AgentWorktree { dir: std::env::current_dir().unwrap() })));
    assert!(matches!(crx.try_recv(), Ok(ServerMsg::Reply(Reply::Text(p))) if p.is_empty()), "an agent's own agents work where it does");
    close(&mut d, &[t]);
}

#[test]
fn an_agent_stopped_by_a_limit_is_told_to_continue_when_it_resets() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let now = term::unix_now();
    d.limits.insert("claude".into(), vec![Limit { name: "5h".into(), used: 100.0, resets_at: now + 600 }, Limit { name: "week".into(), used: 40.0, resets_at: now + 90_000 }]);
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.status) = (Some("claude".into()), Status::Idle);
        p.parser.process(&b"\r\n".repeat(30));
        p.parser.process(b"\r\n  \xe2\x8e\xbf  Claude usage limit reached. Your limit will reset at 3pm\r\n> ");
    }
    d.auto_continue();
    assert_eq!(d.terms[&t].resume_at, Some(now + 600 + 60), "a minute after the spent limit resets");
    assert!(d.snapshot().terms[&t].resume_at.is_some(), "and the window says so");
    d.auto_continue();
    assert!(d.terms[&t].pending_input.is_none(), "not before");
    d.terms.get_mut(&t).unwrap().resume_at = Some(now - 1);
    d.auto_continue();
    assert_eq!(d.terms[&t].pending_input.as_ref().map(|(b, _)| b.as_slice()), Some(&b"continue\r"[..]), "then it's told to go on");
    d.terms.get_mut(&t).unwrap().pending_input = None;
    d.auto_continue();
    assert!(d.terms[&t].resume_at.is_none(), "the old line still on screen doesn't start another wait");
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
    let usage = |cost: f64| Usage { context: Some(30.0), cost: Some(cost) };
    let limits = vec![Limit { name: "5h".into(), used: 23.0, resets_at: term::unix_now() + 100 }];
    d.handle(Ev::Msg(1, ClientMsg::Usage { term: t, token: "wrong".into(), usage: usage(9.0), limits: Vec::new() }));
    assert_eq!(d.terms[&t].usage, Usage::default(), "only with the pane's secret");
    d.handle(Ev::Msg(1, ClientMsg::Usage { term: t, token: token.clone(), usage: usage(1.0), limits: limits.clone() }));
    d.handle(Ev::Msg(1, ClientMsg::Usage { term: t, token: token.clone(), usage: usage(1.5), limits }));
    // /clear: a new session, counting from zero again.
    d.handle(Ev::Msg(1, ClientMsg::Usage { term: t, token, usage: usage(0.25), limits: Vec::new() }));
    let snap = d.snapshot();
    assert_eq!(snap.terms[&t].usage.context, Some(30.0));
    assert!((snap.spent_today - 1.75).abs() < 1e-9, "1.50 then 0.25 after the clear: {}", snap.spent_today);
    assert_eq!(snap.limits[0].0, "claude");
    close(&mut d, &[t]);
}



#[test]
fn a_panes_history_survives_the_server_restarting() {
    let (mut d, _rx) = daemon();
    d.cfg.restore.enabled = true;
    d.command(0, Command::NewWorkspace { cwd: Some(std::env::temp_dir()), name: None, cmd: None }).unwrap();
    let t = *d.terms.keys().next().unwrap();
    d.terms.get_mut(&t).unwrap().ring_push_for_test(b"line from before the restart\r\n");
    let saved = d.saved();
    d.save_outputs(true);
    // A new server: the pane comes back, with what it showed above what it shows now.
    let (mut d2, _rx2) = daemon();
    d2.cfg.restore.enabled = true;
    d2.restore(saved);
    let new = *d2.terms.keys().next().unwrap();
    let replay = String::from_utf8_lossy(&d2.terms[&new].replay()).into_owned();
    assert!(replay.contains("line from before the restart") && replay.contains("seshi restarted here"), "{replay}");
    let mut p = vt100::Parser::new(24, 80, 1000);
    p.process(&d2.terms[&new].replay());
    p.screen_mut().set_scrollback(usize::MAX);
    assert!(p.screen().scrollback() >= 24, "the old screen went up into history: {}", p.screen().scrollback());
    assert!(persist::take_output(t).is_none(), "taken once");
    close(&mut d, &[t]);
    close(&mut d2, &[new]);
}

#[test]
fn closing_the_last_pane_keeps_an_open_window() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let (tx, _crx) = mpsc::channel(64);
    d.handle(Ev::Connected(1, tx, true));
    d.close_term(t);
    assert!(!d.should_exit(), "a window is still showing seshi");
    d.handle(Ev::Disconnected(1));
    assert!(d.should_exit(), "everything closed and nobody's looking: exit");
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
    let agent = |start: bool| Some(Subagent { id: "a1".into(), kind: "general-purpose".into(), start, reported_at: 0, task: None, model: None, effort: None, activity: None });
    d.apply_hook(hook(HookStatus::Working, "UserPromptSubmit", None), Some(true), Vec::new());
    d.apply_hook(hook(HookStatus::Working, "SubagentStart", agent(true)), Some(true), Vec::new());
    d.apply_hook(hook(HookStatus::Working, "SubagentStart", agent(true)), Some(true), Vec::new());
    assert_eq!(d.terms[&t].subagent_activity.len(), 1, "repeated start events do not duplicate the record");
    assert_eq!(d.terms[&t].subagent_activity[0].state, SubagentState::Working);
    // Claude's own turn ends ("waiting for 1 background agent"): still working.
    d.apply_hook(hook(HookStatus::Done, "Stop", None), Some(true), Vec::new());
    assert_eq!(d.terms[&t].status, Status::Working);
    // Long past the old three minutes, the agent is still at it (its own tool use): still
    // working, still listed once.
    let ago = |mins: u64| Instant::now().checked_sub(Duration::from_secs(mins * 60)).unwrap();
    d.terms.get_mut(&t).unwrap().done_held = Some(ago(20));
    d.apply_hook(hook(HookStatus::Same, "PreToolUse", agent(true)), Some(true), Vec::new());
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Working, "held while the agent runs");
    assert_eq!(d.terms[&t].subagents.len(), 1, "listed once");
    // Nothing from it for over an hour: it died without saying; the turn is done.
    let term = d.terms.get_mut(&t).unwrap();
    (term.done_held, term.subagent_seen) = (Some(ago(70)), Some(ago(70)));
    d.update_statuses();
    assert_ne!(d.terms[&t].status, Status::Working, "not held forever");
    assert_eq!(d.terms[&t].subagent_activity[0].state, SubagentState::ConnectionLost, "quiet child is retained as unconfirmed, not completed");
    let terms: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn a_progress_bar_that_goes_off_while_helpers_run_is_not_a_cancel() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let ago = |secs: u64| Instant::now().checked_sub(Duration::from_secs(secs)).unwrap();
    {
        let p = d.terms.get_mut(&t).unwrap();
        (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Working);
        p.progress_off = Some(ago(5));
    }
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Idle, "control: with nothing running, a bar that went off is a cancel");
    {
        let p = d.terms.get_mut(&t).unwrap();
        p.status = Status::Working;
        p.progress_off = Some(ago(5));
        p.subagents.push(("a1".into(), "general-purpose".into()));
        p.subagent_seen = Some(Instant::now());
    }
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Working, "the agent is waiting on its helper, not cancelled");
    close(&mut d, &[t]);
}

#[test]
fn a_stop_for_an_unknown_helper_leaves_the_running_ones_alone() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let hook = |status: HookStatus, event: &str, id: &str, start: bool| {
        let mut m = report(t, "", 0, status, event);
        if let ClientMsg::Hook { subagent, .. } = &mut m {
            *subagent = Some(Subagent { id: id.into(), kind: "general-purpose".into(), start, reported_at: 0, task: None, model: None, effort: None, activity: None });
        }
        m
    };
    d.apply_hook(hook(HookStatus::Working, "SubagentStart", "a1", true), Some(true), Vec::new());
    d.apply_hook(hook(HookStatus::Working, "SubagentStart", "a2", true), Some(true), Vec::new());
    // One that began before seshi was listening stops: nobody listed has finished.
    d.apply_hook(hook(HookStatus::Same, "SubagentStop", "never-seen", false), Some(true), Vec::new());
    assert_eq!(d.terms[&t].subagents.len(), 2, "both still listed");
    d.apply_hook(hook(HookStatus::Same, "SubagentStop", "a1", false), Some(true), Vec::new());
    assert_eq!(d.terms[&t].subagents, vec![("a2".to_string(), "general-purpose".to_string())], "only the one that stopped goes");
    close(&mut d, &[t]);
}

#[test]
fn a_session_starting_again_while_helpers_run_keeps_the_agent_working() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let plain = |status: HookStatus, event: &str| report(t, "", 0, status, event);
    d.apply_hook(plain(HookStatus::Working, "UserPromptSubmit"), Some(true), Vec::new());
    let mut start = plain(HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { subagent, .. } = &mut start {
        *subagent = Some(Subagent { id: "a1".into(), kind: "general-purpose".into(), start: true, reported_at: 0, task: None, model: None, effort: None, activity: None });
    }
    d.apply_hook(start, Some(true), Vec::new());
    // Compaction starts the session again in the middle of the turn.
    d.apply_hook(plain(HookStatus::Idle, "SessionStart"), Some(true), Vec::new());
    assert_eq!(d.terms[&t].status, Status::Working, "still working");
    assert_eq!(d.terms[&t].subagents.len(), 1, "its helper is still listed");
    close(&mut d, &[t]);
}

#[test]
fn delayed_child_start_does_not_undo_its_completion() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    d.terms.get_mut(&t).unwrap().session = Some("parent-thread".into());
    let child = |start, reported_at| Some(Subagent {
        id: "child-1".into(), kind: "reviewer".into(), start, reported_at,
        task: Some("review the change".into()), model: Some("claude-opus".into()), effort: None,
        activity: Some(if start { "SubagentStart" } else { "SubagentStop" }.into()),
    });
    let mut start = report(t, "", 0, HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { subagent, .. } = &mut start { *subagent = child(true, 100); }
    d.apply_hook(start, Some(true), Vec::new());
    let mut stop = report(t, "", 0, HookStatus::Same, "SubagentStop");
    if let ClientMsg::Hook { subagent, .. } = &mut stop { *subagent = child(false, 200); }
    d.apply_hook(stop, Some(true), Vec::new());
    let mut delayed_start = report(t, "", 0, HookStatus::Working, "SubagentStart");
    if let ClientMsg::Hook { subagent, .. } = &mut delayed_start { *subagent = child(true, 150); }
    d.apply_hook(delayed_start, Some(true), Vec::new());

    let activity = &d.terms[&t].subagent_activity[0];
    assert_eq!(activity.state, SubagentState::Completed);
    assert_eq!(activity.updated_at, 200);
    assert!(d.terms[&t].subagents.is_empty(), "a delayed start cannot recreate a live child");
    assert_eq!(activity.parent.as_deref(), Some("parent-thread"));
    assert_eq!(activity.task.as_deref(), Some("review the change"));
    assert_eq!(activity.model.as_deref(), Some("claude-opus"));
    close(&mut d, &[t]);
}

#[test]
fn an_agent_asks_you_and_gets_your_answer() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    // The agent's `seshi ask-human` is a client waiting for the reply.
    let (tx, mut asker) = mpsc::channel(8);
    d.handle(Ev::Connected(5, tx, false));
    let ask = Command::AskHuman { term: t, text: "Deploy to staging?".into(), options: vec!["Yes".into(), "No".into()] };
    assert!(!d.command(5, ask).unwrap(), "the reply waits for you");
    assert_eq!(d.terms[&t].status, Status::Blocked, "it needs you");
    let q = d.snapshot().questions;
    assert_eq!((q.len(), q[0].text.as_str()), (1, "Deploy to staging?"));
    // You answer No: the asker gets it, the pane is back at work.
    d.command(0, Command::AnswerHuman { id: q[0].id, choice: 1 }).unwrap();
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
    d.handle(Ev::Connected(7, tx, false));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    let err = |out: &mut mpsc::Receiver<ServerMsg>| matches!(out.try_recv(), Ok(ServerMsg::Error(_)));
    // Reading another pane: allowed by default.
    d.message(7, ClientMsg::Query(Query::Read { term: b }));
    assert!(matches!(out.try_recv(), Ok(ServerMsg::Reply(Reply::Text(_)))), "read is a default grant");
    // Answering b's prompt: not without respond.
    d.terms.get_mut(&b).unwrap().status = Status::Blocked;
    d.message(7, ClientMsg::Input { term: b, data: b"1".to_vec() });
    assert!(err(&mut out), "respond isn't a default grant");
    // Closing b: not without admin; and it can't grant itself more.
    d.message(7, ClientMsg::Command(Command::ClosePane { term: b }));
    assert!(err(&mut out) && d.terms.contains_key(&b), "admin isn't a default grant");
    d.message(7, ClientMsg::Command(Command::Grant { term: a, grants: Some(vec!["admin".into()]) }));
    assert!(err(&mut out), "grants are yours to give");
    // You give it admin: now it may.
    d.message(0, ClientMsg::Command(Command::Grant { term: a, grants: Some(vec!["admin".into()]) }));
    d.message(7, ClientMsg::Command(Command::ClosePane { term: b }));
    assert!(!d.terms.contains_key(&b), "closed once allowed");
    let terms: Vec<TermId> = d.terms.keys().copied().collect();
    close(&mut d, &terms);
}

#[test]
fn a_popup_floats_in_no_workspace_and_goes_when_done() {
    let (mut d, mut rx) = daemon();
    let cmd = if cfg!(windows) { "cmd /c exit" } else { "true" };
    d.command(0, Command::Popup { cmd: cmd.into(), cwd: Some(std::env::temp_dir()) }).unwrap();
    let (&id, t) = d.terms.iter().next().expect("the popup's pane");
    assert!(t.popup && d.workspaces.is_empty(), "floating, in no workspace");
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
fn an_agent_stays_in_the_folder_it_started_in() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let secret = d.terms[&t].token.clone();
    let base = std::env::temp_dir().join(format!("seshi-pin-{}", std::process::id()));
    let (home, other) = (base.join("home"), base.join("other"));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    let at = |cwd: &std::path::Path, status: HookStatus, event: &str| match report(t, &secret, 0, status, event) {
        ClientMsg::Hook { term, agent, status, session, prompt, said, subagent, event, pid, token, transcript, model, name, .. } => {
            ClientMsg::Hook { term, agent, status, session, cwd: Some(cwd.to_path_buf()), prompt, said, subagent, event, pid, token, transcript, model, name }
        }
        _ => unreachable!(),
    };
    // Its first report places it.
    d.apply_hook(at(&home, HookStatus::Working, "UserPromptSubmit"), Some(true), Vec::new());
    assert_eq!(d.terms[&t].cwd, home);
    // Working in another folder doesn't move it.
    d.apply_hook(at(&other, HookStatus::Working, "PreToolUse"), Some(true), Vec::new());
    assert_eq!(d.terms[&t].cwd, home, "its row stays in the group it started in");
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
        p.parser.process(b"\xe2\x80\xa2 Working (3s \xe2\x80\xa2 esc to interrupt)\r\n");
        // A long block printed under it pushes the working line out of the rows looked at.
        for i in 0..40 {
            p.parser.process(format!("+ added line {i}\r\n").as_bytes());
        }
        p.last_output = Instant::now();
    }
    d.update_statuses();
    assert_eq!(d.terms[&t].status, Status::Working, "still printing: not done, and no alert");
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
    assert_eq!(d.terms[&t].status, Status::Working, "seen on its screen: its hooks never say a turn started");
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
    t.status = Status::Working; t.status_since = 7; t.status_why = "hook: working".into();
    t.last_screen = Some(("old observation".into(), 1));
    d.set_status(id, Status::Working, "screen: activity row is present");
    let t = &d.terms[&id];
    let result = (t.status_since, t.status_why.clone(), t.last_screen.clone(), t.alert_due.is_none());
    close(&mut d, &[id]);
    assert_eq!(result.0, 7, "evidence freshness is separate from turn duration");
    assert_eq!(result.1, "hook: working", "the original status source stays authoritative");
    assert!(result.2.is_some_and(|(detail, at)| detail == "activity row is present" && at > 1));
    assert!(result.3, "the same status does not announce another transition");
}

#[test]
fn codex_detects_work_with_background_terminals_and_wrapped_footers() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for hooked in [false, true] {
        for cols in [160, 32] {
            for suffix in ["", " · 1 background terminal running · /ps to view · /stop to close"] {
                let t = d.terms.get_mut(&id).unwrap();
                t.resize(cols, 24);
                t.agent = Some("codex".into()); t.hooked = hooked; t.status = Status::Done;
                t.status_since = term::unix_now() - 10;
                t.last_input = Instant::now();
                t.parser.process(b"\x1b[2J\x1b[H"); t.parser.process(&b"\r\n".repeat(30));
                t.parser.process(format!("• Working (2m 40s • esc to interrupt){suffix}\r\n> Ask Codex to do anything").as_bytes());
                d.update_statuses();
                observed.push((hooked, cols, suffix, d.terms[&id].status));
            }
        }
    }
    close(&mut d, &[id]);
    assert!(observed.iter().all(|(_, _, _, status)| *status == Status::Working), "native Codex activity must survive metadata and soft wraps: {observed:?}");
}

#[test]
fn codex_can_start_its_next_turn_without_waiting_for_the_wall_clock() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    d.hook(id, "codex".into(), HookStatus::Done, "agent-turn-complete");
    let t = d.terms.get_mut(&id).unwrap();
    // A backwards clock makes the same zero-second interval as a prompt sent immediately.
    t.status_since = u64::MAX;
    t.last_input = Instant::now();
    t.parser.process(&b"\r\n".repeat(30));
    t.parser.process("• Working (1s • esc to interrupt)\r\n> Ask Codex to do anything".as_bytes());
    d.update_statuses();
    let mut observed = vec![d.terms[&id].status];
    d.hook(id, "codex".into(), HookStatus::Done, "agent-turn-complete");
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
    assert_eq!(observed, [Status::Working, Status::Done, Status::Idle, Status::Working], "new input starts work immediately or after a delay; a stale footer stays finished, even after being seen");
}

#[test]
fn codex_live_footer_restarts_work_without_new_keyboard_input() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let t = d.terms.get_mut(&id).unwrap();
    t.agent = Some("codex".into());
    t.output(format!("\x1b[2J\x1b[H{}Working (3m 11s • esc to interrupt) · 3 background terminals running · /ps to view · /stop to close\r\n› Ask Codex to do anything", "\r\n".repeat(30)).as_bytes());
    d.hook(id, "codex".into(), HookStatus::Done, "agent-turn-complete");
    d.update_statuses();
    let mut observed = vec![d.terms[&id].status];
    let t = d.terms.get_mut(&id).unwrap();
    t.output(format!("\x1b[2J\x1b[H{}Working (3m 12s • esc to interrupt) · 3 background terminals running · /ps to view · /stop to close\r\n› Ask Codex to do anything", "\r\n".repeat(30)).as_bytes());
    d.update_statuses(); observed.push(d.terms[&id].status);
    close(&mut d, &[id]);
    assert_eq!(observed, [Status::Done, Status::Working], "an unchanged old footer stays finished; an advancing live footer is working even without new keys");
}

#[test]
fn native_hooks_cannot_keep_an_exited_agent_working_or_hide_its_replacement() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let found = |agent: Option<&str>| scan::Found { term: id, process: "powershell".into(), agent: agent.map(str::to_string), cwd: None, agent_cwd: None, mem: 0, cpu: 0.0, remote: None };
    d.hook(id, "pi".into(), HookStatus::Working, "pi.agent_start");
    d.handle(Ev::Scan(vec![found(None)]));
    let startup = d.terms[&id].status;
    d.handle(Ev::Scan(vec![found(Some("pi"))]));
    d.handle(Ev::Scan(vec![found(None)]));
    let exited = (d.terms[&id].status, d.terms[&id].agent.clone(), d.terms[&id].hooked);
    d.hook(id, "codex".into(), HookStatus::Working, "native");
    d.terms.get_mut(&id).unwrap().last_hook_at = Some(Instant::now() - Duration::from_secs(4));
    d.handle(Ev::Scan(vec![found(Some("opencode"))]));
    let replacement = (d.terms[&id].agent.clone(), d.terms[&id].hooked, d.terms[&id].status);
    close(&mut d, &[id]);
    assert_eq!(startup, Status::Working, "initial process scan can lag a native hook");
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
        t.agent = Some("codex".into()); t.hooked = hooked;
        t.status = if hooked { Status::Done } else { Status::Working };
        t.status_since = term::unix_now() - 10;
        t.last_input = Instant::now(); t.last_output = Instant::now() - Duration::from_secs(10);
        t.parser.process(b"\x1b[2J\x1b[H"); t.parser.process(&b"\r\n".repeat(30));
        t.parser.process(b"I checked the live daemon: it matches Codex's esc to interrupt spinner.\r\nWorked for 50s\r\n> Ask Codex to do anything");
        d.update_statuses();
        observed.push(d.terms[&id].status);
        let t = d.terms.get_mut(&id).unwrap();
        t.status_since = term::unix_now() - 10; t.last_input = Instant::now();
        t.parser.process(b"\x1b[2J\x1b[H"); t.parser.process(&b"\r\n".repeat(30));
        t.parser.process("• Working (8m 05s • esc to interrupt)\r\n> Ask Codex to do anything".as_bytes());
        d.update_statuses();
        observed.push(d.terms[&id].status);
    }
    close(&mut d, &[id]);
    assert_eq!(observed, [Status::Done, Status::Working, Status::Done, Status::Working], "finished prose never starts or extends a turn, with or without hooks; the actual activity row does");
}

#[test]
fn codex_queued_questions_need_attention_even_while_work_continues() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for hooked in [false, true] {
        let t = d.terms.get_mut(&id).unwrap();
        t.agent = Some("codex".into()); t.hooked = hooked; t.status = Status::Working;
        t.parser.process(b"\x1b[2J\x1b[H"); t.parser.process(&b"\r\n".repeat(30));
        t.parser.process(b"Working (8m 01s - esc to interrupt)\r\nQueued follow-up inputs\r\n  ? 1 question\r\n  shift+tab to answer\r\nAsk Codex to do anything");
        d.update_statuses();
        observed.push((hooked, d.terms[&id].status));
        let t = d.terms.get_mut(&id).unwrap();
        t.parser.process(b"\x1b[2J\x1b[H"); t.parser.process(&b"\r\n".repeat(30));
        t.parser.process(b"Working (8m 05s - esc to interrupt)\r\nAsk Codex to do anything");
        d.update_statuses();
        observed.push((hooked, d.terms[&id].status));
    }
    let t = d.terms.get_mut(&id).unwrap();
    t.parser.process(b"\x1b[2J\x1b[H"); t.parser.process(&b"\r\n".repeat(30));
    t.parser.process(b"The report discusses 1 question\r\n? 0 questions\r\nWorking (esc to interrupt)");
    d.update_statuses();
    observed.push((true, d.terms[&id].status));
    close(&mut d, &[id]);
    assert_eq!(observed, [(false, Status::Blocked), (false, Status::Working), (true, Status::Blocked), (true, Status::Working), (true, Status::Working)], "questions take precedence with or without hooks, then ongoing work resumes; prose and zero questions do not block");
}

#[test]
fn native_pi_and_opencode_questions_stay_blocked_until_their_reply_event() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for agent in ["pi", "opencode"] {
        d.hook(id, agent.into(), HookStatus::Working, "native start");
        d.hook(id, agent.into(), HookStatus::Blocked, "native question");
        let t = d.terms.get_mut(&id).unwrap();
        t.parser.process(b"\x1b[2J\x1b[HCustom extension dialog\r\nPick a project");
        t.blocked_at = Some(Instant::now() - Duration::from_secs(20));
        t.last_input = Instant::now() - Duration::from_secs(10);
        t.last_output = t.last_input;
        d.update_statuses();
        observed.push(d.terms[&id].status);
        d.hook(id, agent.into(), HookStatus::Working, "native reply");
        observed.push(d.terms[&id].status);
        d.hook(id, agent.into(), HookStatus::Done, "native settled");
        observed.push(d.terms[&id].status);
    }
    close(&mut d, &[id]);
    assert_eq!(observed, [Status::Blocked, Status::Working, Status::Done, Status::Blocked, Status::Working, Status::Done]);
}

#[test]
fn codex_expanded_question_overrides_working_until_dismissed() {
    let (mut d, _rx) = daemon();
    let id = pane(&mut d);
    let mut observed = Vec::new();
    for hooked in [false, true] {
        let t = d.terms.get_mut(&id).unwrap();
        t.agent = Some("codex".into()); t.hooked = hooked; t.status = Status::Working;
        t.parser.process(b"\x1b[2J\x1b[H");
        t.parser.process(&b"\r\n".repeat(30));
        t.parser.process("Working (8m 01s - esc to interrupt)\r\nChoose an app:\r\n> 1. Pi / OpenCode\r\nenter submit   ctrl+] skip   shift+↔ main prompt".as_bytes());
        d.update_statuses(); observed.push(d.terms[&id].status);
        let t = d.terms.get_mut(&id).unwrap();
        t.parser.process(b"\x1b[2J\x1b[H"); t.parser.process(&b"\r\n".repeat(30));
        t.parser.process(b"Working (8m 02s - esc to interrupt)");
        d.update_statuses(); observed.push(d.terms[&id].status);
    }
    close(&mut d, &[id]);
    assert_eq!(observed, [Status::Blocked, Status::Working, Status::Blocked, Status::Working]);
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
    d.handle(Ev::Connected(7, tx, false));
    d.message(7, ClientMsg::Query(Query::CheckOutput { term: t, name: None }));
    assert!(matches!(out.try_recv(), Ok(ServerMsg::Error(_))), "no check, no output");
    d.terms.get_mut(&t).unwrap().checks.points = vec![failed_check("ok a\nFAIL footer.test.ts\n").into()];
    let info = d.snapshot().terms[&t].check().cloned().expect("the summary is in the snapshot");
    assert_eq!((info.state, info.first_failing_line.as_str()), (CheckState::Failed, "FAIL footer.test.ts"));
    d.message(7, ClientMsg::Query(Query::CheckOutput { term: t, name: None }));
    assert!(matches!(out.try_recv(), Ok(ServerMsg::Reply(Reply::Text(text))) if text == "ok a\nFAIL footer.test.ts\n"));
    close(&mut d, &[t]);
}

#[test]
fn a_pane_may_not_read_another_panes_check_output_without_the_read_grant() {
    let (mut d, _rx) = daemon();
    let (a, b) = (pane(&mut d), pane(&mut d));
    d.terms.get_mut(&b).unwrap().checks.points = vec![failed_check("secret-ish output").into()];
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(7, tx, false));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    d.terms.get_mut(&a).unwrap().grants = Some(Vec::new());
    d.message(7, ClientMsg::Query(Query::CheckOutput { term: b, name: None }));
    assert!(matches!(out.try_recv(), Ok(ServerMsg::Error(_))), "read is a grant");
    close(&mut d, &[a, b]);
}

#[test]
fn check_results_survive_a_restart_with_their_output() {
    let (mut d, _rx) = daemon();
    let dir = std::env::temp_dir();
    let tails = persist::scratch_dir();
    d.check_dir = tails.path().to_path_buf();
    let old: TermId = 9_201;
    let overridden = CheckInfo { state: CheckState::Overridden, overridden_at: Some(5), override_reason: "flaky".into(), ..failed_check("").info };
    let unsaved = persist::save_check_outputs(
        tails.path(),
        persist::save_ticket(),
        &[persist::CheckTail { term: old, name: String::new(), slot: persist::CheckSlot::Latest, text: "latest tail".into() }, persist::CheckTail { term: old, name: String::new(), slot: persist::CheckSlot::Overridden, text: "overridden tail".into() }],
        &[old],
    );
    assert!(unsaved.is_empty());
    let pane = persist::SavedPane {
        cwd: Some(dir.clone()),
        checks: vec![failed_check("").info],
        checks_overridden: vec![overridden.clone()],
        ..Default::default()
    };
    let tab = persist::SavedTab { name: String::new(), layout: Node::Leaf(old), focus: old, panes: [(old, pane)].into() };
    let ws = persist::SavedWs { name: String::new(), cwd: dir, worktree: false, color: None, group: None, tabs: vec![tab], active_tab: 0 };
    d.restore(persist::Saved { workspaces: vec![ws], active: 0, made_worktrees: Vec::new() });
    let t = d.workspaces[0].tabs[0].focus;
    let restored = &d.terms[&t];
    assert_eq!(restored.checks.headline().map(|c| c.output.as_str()), Some("latest tail"));
    assert_eq!(restored.checks.points.first().and_then(|p| p.overridden.as_ref()).map(|c| (&c.info, c.output.as_str())), Some((&overridden, "overridden tail")));
    let saved = d.saved();
    let again = &saved.workspaces[0].tabs[0].panes[&t];
    assert_eq!((again.checks.iter().map(|c| c.state).collect::<Vec<_>>(), &again.checks_overridden), (vec![CheckState::Failed], &vec![overridden.clone()]));
    let mut written = d.take_dirty_check_outputs();
    written.sort_by_key(|w| w.slot as u8);
    let tail = |slot, text: &str| persist::CheckTail { term: t, name: String::new(), slot, text: text.into() };
    let expected = [tail(persist::CheckSlot::Latest, "latest tail"), tail(persist::CheckSlot::Overridden, "overridden tail")];
    assert_eq!(written, expected, "its output is written again under the new id");
    assert!(d.take_dirty_check_outputs().is_empty(), "and only once");
    close(&mut d, &[t]);
}

#[test]
fn a_check_that_was_running_when_the_server_stopped_reads_as_not_run() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let running = CheckInfo { state: CheckState::Checking, ..Default::default() };
    let saved = persist::SavedPane { checks: vec![running], ..Default::default() };
    d.terms.get_mut(&t).unwrap().restore_checks(&d.check_dir.clone(), 9_202, &saved);
    assert_eq!(d.terms[&t].checks.headline().map(|c| c.info.state), Some(CheckState::CouldNotRun));
    close(&mut d, &[t]);
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("a runtime for the blocking tasks")
}

const LONG_CHECK_S: u64 = 600;

/// An agent session in the temp folder (not a repo, so the folder is its project), whose
/// project has a check running `command`.
fn checked_agent(d: &mut Daemon, command: &str, timeout_s: u64) -> TermId {
    d.cfg.check = vec![crate::config::CheckDef { project: std::env::temp_dir(), name: None, command: command.into(), timeout_s }];
    let t = pane(d);
    let p = d.terms.get_mut(&t).unwrap();
    (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Working);
    t
}

fn end_turn(d: &mut Daemon, t: TermId) {
    d.hook(t, "claude".into(), HookStatus::Done, "Stop");
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
    assert_eq!((running.state, running.command.as_str()), (CheckState::Checking, "echo \"suite ok\"; exit 0"));
    assert!(d.snapshot().terms[&t].check().is_some(), "clients see it checking");
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    let done = check_of(&d, t);
    assert_eq!((done.state, done.exit_code), (CheckState::Passed, Some(0)));
    assert_eq!(d.terms[&t].checks.headline().unwrap().output.trim(), "suite ok");
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
    assert_eq!(d.terms[&t].checks.headline().unwrap().output.trim(), "1 test failed");
    close(&mut d, &[t]);
}

#[test]
fn a_failed_check_names_its_first_failing_line_and_a_rerun_clears_it() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "echo running; echo \"FAIL tests/foo: boom\"; exit 1", LONG_CHECK_S);
    end_turn(&mut d, t);
    assert!(check_of(&d, t).first_failing_line.is_empty(), "nothing while it runs");
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).first_failing_line, "FAIL tests/foo: boom");
    let seen = d.snapshot().terms[&t].check().cloned().expect("the summary is in the snapshot");
    assert_eq!(seen.first_failing_line, "FAIL tests/foo: boom");
    d.terms.get_mut(&t).unwrap().status = Status::Idle;
    d.rerun_check(t, None).expect("a failed check can be run again");
    assert!(check_of(&d, t).first_failing_line.is_empty(), "a re-run starts clean");
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    close(&mut d, &[t]);
}

#[test]
fn a_check_that_outruns_its_timeout_reads_as_not_run() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let slow = if cfg!(windows) { "Start-Sleep 60" } else { "sleep 60" };
    let t = checked_agent(&mut d, slow, 1);
    end_turn(&mut d, t);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    let info = check_of(&d, t);
    assert_eq!((info.state, info.exit_code), (CheckState::CouldNotRun, None));
    close(&mut d, &[t]);
}

#[cfg(unix)]
#[test]
fn a_check_whose_command_is_not_found_reads_as_not_run() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "seshi-no-such-command-xyz", LONG_CHECK_S);
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
    d.cfg.shell = Some("/no/such/shell-for-seshi".into());
    end_turn(&mut d, t);
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).state, CheckState::CouldNotRun, "never stuck checking");
    close(&mut d, &[t]);
}

#[test]
fn the_screen_ending_a_turn_runs_the_check_too_but_a_cancelled_turn_does_not() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    d.set_status(t, Status::Idle, "progress bar gone, no turn end: cancelled");
    assert!(d.terms[&t].checks.points.is_empty(), "an interrupted turn finished nothing");
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
    assert!(d.terms[&t].checks.points.is_empty(), "no check in this project");
    assert_eq!(d.terms[&t].status, Status::Done, "and done as ever");
    // A shell pane in a project with a check.
    d.cfg.check[0].project = std::env::temp_dir();
    let shell = pane(&mut d);
    d.terms.get_mut(&shell).unwrap().status = Status::Working;
    d.set_status(shell, Status::Done, "test");
    assert!(d.terms[&shell].checks.points.is_empty(), "a shell is not an agent");
    // A turn that has not ended: working, asking, or held for its helpers.
    let busy = pane(&mut d);
    (d.terms.get_mut(&busy).unwrap().agent, d.terms.get_mut(&busy).unwrap().hooked) = (Some("claude".into()), true);
    d.hook(busy, "claude".into(), HookStatus::Working, "UserPromptSubmit");
    d.hook(busy, "claude".into(), HookStatus::Blocked, "PreToolUse");
    d.terms.get_mut(&busy).unwrap().subagents.push(("sub".into(), "explore".into()));
    d.hook(busy, "claude".into(), HookStatus::Done, "Stop");
    assert!(d.terms[&busy].checks.points.is_empty(), "a turn waiting on its helpers has not ended");
    d.hook(busy, "claude".into(), HookStatus::Done, "Notification:idle_prompt");
    assert!(d.terms[&busy].checks.points.is_empty(), "a repeated idle ping is not a turn end");
    close(&mut d, &[t, shell, busy]);
}

#[test]
fn a_turn_ending_while_its_check_runs_does_not_start_a_second_run() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    end_turn(&mut d, t);
    let first = check_of(&d, t);
    d.hook(t, "claude".into(), HookStatus::Working, "UserPromptSubmit");
    end_turn(&mut d, t);
    assert_eq!(check_of(&d, t), first, "the running check is left as it was");
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert!(!finish_check(&mut d, &mut rx, Duration::from_millis(500)), "only one run was started");
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
    assert_eq!(check_of(&d, t), settled, "the same turn is not checked twice");
    assert!(!finish_check(&mut d, &mut rx, Duration::from_millis(500)), "no second run was started");
    d.hook(t, "claude".into(), HookStatus::Working, "UserPromptSubmit");
    end_turn(&mut d, t);
    assert_eq!(check_of(&d, t).state, CheckState::Checking, "a new turn is checked");
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
    d.hook(t, "claude".into(), HookStatus::Working, "UserPromptSubmit");
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
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT), "the result still arrives and is ignored");
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
        &[persist::CheckTail { term: open, name: String::new(), slot: persist::CheckSlot::Latest, text: "kept".into() }, persist::CheckTail { term: closed, name: String::new(), slot: persist::CheckSlot::Latest, text: "gone".into() }],
        &[open, closed],
    );
    assert!(unsaved.is_empty());
    d.save_check_tails(persist::save_ticket(), vec![open], true);
    assert_eq!(persist::read_check_output(tails.path(), closed, "", persist::CheckSlot::Latest), None, "pruned at shutdown");
    assert_eq!(persist::read_check_output(tails.path(), open, "", persist::CheckSlot::Latest).as_deref(), Some("kept"));
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
    assert!(d.terms[&t].checks.dirty, "still due: the write did not happen");
    std::fs::remove_file(&d.check_dir).unwrap();
    d.save_check_tails(persist::save_ticket(), vec![t], true);
    assert!(!d.terms[&t].checks.dirty);
    assert_eq!(persist::read_check_output(&d.check_dir, t, "", persist::CheckSlot::Latest).as_deref(), Some("FAIL footer\n"));
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
    assert_eq!(d.take_dirty_check_outputs().len(), 1, "due again; a pane that is gone is skipped");
    close(&mut d, &[t]);
}

#[test]
fn a_restored_tail_stays_on_disk_until_the_new_one_is_saved() {
    let (mut d, _rx) = daemon();
    let tails = persist::scratch_dir();
    d.check_dir = tails.path().to_path_buf();
    let old: TermId = 9_301;
    assert!(persist::save_check_outputs(tails.path(), persist::save_ticket(), &[persist::CheckTail { term: old, name: String::new(), slot: persist::CheckSlot::Latest, text: "tail".into() }], &[old]).is_empty());
    let t = pane(&mut d);
    let saved = persist::SavedPane { checks: vec![failed_check("").info], ..Default::default() };
    let dir = d.check_dir.clone();
    d.terms.get_mut(&t).unwrap().restore_checks(&dir, old, &saved);
    assert_eq!(persist::read_check_output(tails.path(), old, "", persist::CheckSlot::Latest).as_deref(), Some("tail"), "a crash now loses nothing");
    close(&mut d, &[t]);
}

fn with_check(d: &mut Daemon, t: TermId, state: CheckState, output: &str) {
    let record = failed_check(output);
    // Named as the project's checkpoint is, so a run finds it.
    let name = d.cfg.check.first().map(|c| c.name().to_string()).unwrap_or_default();
    let info = CheckInfo { name, state, exit_code: (state == CheckState::Failed).then_some(1), after_turn_ended_at: 77, ..record.info };
    d.terms.get_mut(&t).unwrap().checks.points = vec![check::CheckRecord::new(info, output).into()];
}

#[test]
fn a_rerun_while_the_agent_works_waits_for_the_turn_to_end() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "echo again; exit 0", LONG_CHECK_S);
    with_check(&mut d, t, CheckState::Failed, "old failure");
    d.command(0, Command::RerunCheck { term: t, name: None }).unwrap();
    let waiting = check_of(&d, t);
    assert_eq!((waiting.state, waiting.rerun_queued), (CheckState::Failed, true), "the result stands, and says a re-run is queued");
    assert!(d.snapshot().terms[&t].check().is_some_and(|c| c.rerun_queued), "clients see it");
    assert!(!finish_check(&mut d, &mut rx, Duration::from_millis(300)), "nothing runs mid-turn");
    end_turn(&mut d, t);
    let running = check_of(&d, t);
    assert_eq!((running.state, running.rerun_queued), (CheckState::Checking, false));
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert!(!finish_check(&mut d, &mut rx, Duration::from_millis(500)), "the queued re-run and the turn end are one run");
    assert_eq!(check_of(&d, t).state, CheckState::Passed);
    close(&mut d, &[t]);
}

#[test]
fn a_queued_rerun_is_dropped_when_the_project_no_longer_has_a_check() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    with_check(&mut d, t, CheckState::Failed, "old");
    d.command(0, Command::RerunCheck { term: t, name: None }).unwrap();
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
    d.command(0, Command::RerunCheck { term: t, name: None }).unwrap();
    let running = check_of(&d, t);
    assert_eq!((running.state, running.after_turn_ended_at, running.rerun_queued), (CheckState::Checking, 77, false));
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).state, CheckState::Passed);
    assert_eq!(d.terms[&t].checks.headline().unwrap().output.trim(), "fresh");
    close(&mut d, &[t]);
}

#[test]
fn a_rerun_that_has_nothing_to_run_is_an_error() {
    let (rt, (mut d, _rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "exit 0", LONG_CHECK_S);
    assert!(d.command(0, Command::RerunCheck { term: t, name: None }).is_err(), "working, never checked: its turn end checks it");
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    with_check(&mut d, t, CheckState::Checking, "");
    assert!(d.command(0, Command::RerunCheck { term: t, name: None }).is_err(), "already running");
    d.cfg.check.clear();
    with_check(&mut d, t, CheckState::Failed, "");
    assert!(d.command(0, Command::RerunCheck { term: t, name: None }).is_err(), "no check in this project");
    let shell = pane(&mut d);
    assert!(d.command(0, Command::RerunCheck { term: shell, name: None }).is_err(), "a shell is not checked");
    assert!(d.command(0, Command::RerunCheck { term: 9_999, name: None }).is_err(), "no such pane");
    close(&mut d, &[t, shell]);
}

#[test]
fn only_a_failed_check_can_be_overridden_and_the_override_is_recorded() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    let cmd = |reason: &str| Command::OverrideCheck { term: t, name: None, reason: reason.into() };
    assert!(d.command(0, cmd("")).is_err(), "no result");
    for state in [CheckState::Passed, CheckState::Checking, CheckState::CouldNotRun, CheckState::Overridden] {
        with_check(&mut d, t, state, "out");
        assert!(d.command(0, cmd("")).is_err(), "{state:?}");
        assert_eq!(check_of(&d, t).state, state, "left as it was");
    }
    with_check(&mut d, t, CheckState::Failed, "FAIL a\nFAIL b\n");
    let before = term::unix_now();
    d.command(0, cmd("  flaky on CI  ")).unwrap();
    let info = check_of(&d, t);
    assert_eq!((info.state, info.override_reason.as_str(), info.exit_code), (CheckState::Overridden, "flaky on CI", Some(1)));
    assert!(info.overridden_at.is_some_and(|at| at >= before), "the time is recorded");
    assert_eq!(d.snapshot().terms[&t].check().map(|c| c.state), Some(CheckState::Overridden));
    let term = &d.terms[&t];
    assert_eq!(term.checks.headline().unwrap().output, "FAIL a\nFAIL b\n", "the failure's output stays");
    assert_eq!(term.checks.points.first().and_then(|p| p.overridden.as_ref()).map(|c| (c.info.clone(), c.output.as_str())), Some((info, "FAIL a\nFAIL b\n")));
    assert!(term.checks.dirty, "due to be saved");
    close(&mut d, &[t]);
}

#[test]
fn an_override_reason_is_optional_and_bounded() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    with_check(&mut d, t, CheckState::Failed, "");
    let too_long = "x".repeat(super::check::OVERRIDE_REASON_MAX_CHARS + 1);
    assert!(d.command(0, Command::OverrideCheck { term: t, name: None, reason: too_long }).is_err());
    assert_eq!(check_of(&d, t).state, CheckState::Failed, "refused whole");
    d.command(0, Command::OverrideCheck { term: t, name: None, reason: String::new() }).unwrap();
    assert_eq!(check_of(&d, t).override_reason, "");
    close(&mut d, &[t]);
}

#[test]
fn undoing_an_override_restores_the_failed_result() {
    let (mut d, _rx) = daemon();
    let t = pane(&mut d);
    with_check(&mut d, t, CheckState::Failed, "FAIL a");
    let failed = check_of(&d, t);
    assert!(d.command(0, Command::UndoOverride { term: t, name: None }).is_err(), "nothing to undo on a failure");
    d.command(0, Command::OverrideCheck { term: t, name: None, reason: "r".into() }).unwrap();
    d.command(0, Command::UndoOverride { term: t, name: None }).unwrap();
    assert_eq!(check_of(&d, t), failed, "exactly as it was");
    assert!(d.terms[&t].checks.points.first().is_none_or(|p| p.overridden.is_none()));
    assert!(d.command(0, Command::UndoOverride { term: t, name: None }).is_err(), "and only once");
    close(&mut d, &[t]);
}

#[test]
fn the_next_run_replaces_an_override_but_keeps_the_overridden_output() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = checked_agent(&mut d, "echo green; exit 0", LONG_CHECK_S);
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    with_check(&mut d, t, CheckState::Failed, "FAIL a");
    d.command(0, Command::OverrideCheck { term: t, name: None, reason: "r".into() }).unwrap();
    d.command(0, Command::RerunCheck { term: t, name: None }).unwrap();
    let running = check_of(&d, t);
    assert_eq!((running.state, running.overridden_at, running.override_reason.as_str()), (CheckState::Checking, None, ""));
    assert!(finish_check(&mut d, &mut rx, SLOW_CHECK_WAIT));
    assert_eq!(check_of(&d, t).state, CheckState::Passed);
    let kept = d.terms[&t].checks.points.first().and_then(|p| p.overridden.as_ref()).expect("the overridden result is kept");
    assert_eq!((kept.info.state, kept.output.as_str()), (CheckState::Overridden, "FAIL a"));
    close(&mut d, &[t]);
}

#[test]
fn a_read_only_pane_may_not_rerun_or_override_another_panes_check() {
    let (mut d, _rx) = daemon();
    let (a, b) = (pane(&mut d), pane(&mut d));
    with_check(&mut d, b, CheckState::Failed, "FAIL");
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(7, tx, false));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    d.terms.get_mut(&a).unwrap().grants = Some(vec!["read".into()]);
    for cmd in [Command::RerunCheck { term: b, name: None }, Command::OverrideCheck { term: b, name: None, reason: String::new() }, Command::UndoOverride { term: b, name: None }] {
        d.message(7, ClientMsg::Command(cmd.clone()));
        assert!(matches!(out.try_recv(), Ok(ServerMsg::Error(_))), "{cmd:?} needs write");
    }
    assert_eq!(check_of(&d, b).state, CheckState::Failed, "untouched");
    d.terms.get_mut(&a).unwrap().grants = Some(vec!["read".into(), "write".into()]);
    d.message(7, ClientMsg::Command(Command::OverrideCheck { term: b, name: None, reason: "ok".into() }));
    assert!(matches!(out.try_recv(), Ok(ServerMsg::Reply(Reply::Ok))), "write allows it");
    assert_eq!(check_of(&d, b).state, CheckState::Overridden);
    close(&mut d, &[a, b]);
}

#[test]
fn a_pane_may_not_override_its_own_check() {
    let (mut d, _rx) = daemon();
    let a = pane(&mut d);
    with_check(&mut d, a, CheckState::Failed, "FAIL");
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(7, tx, false));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    d.message(7, ClientMsg::Command(Command::OverrideCheck { term: a, name: None, reason: "trust me".into() }));
    assert!(matches!(out.try_recv(), Ok(ServerMsg::Error(_))), "an agent's claim never counts");
    assert_eq!(check_of(&d, a).state, CheckState::Failed);
    close(&mut d, &[a]);
}

#[test]
fn only_you_answer_a_repo_check_approval() {
    let (mut d, _rx, _file) = approving_daemon();
    let a = waiting_for_approval(&mut d, "npm test");
    d.approvals.decide(&std::env::temp_dir(), "build", "make", approvals::Decision::Never, 1);
    let kept = d.approvals.clone();
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(7, tx, false));
    let token = d.terms[&a].token.clone();
    d.handle(Ev::From(7, a, token));
    d.terms.get_mut(&a).unwrap().grants = Some(["read", "write", "start", "respond", "admin"].map(String::from).into());
    let forget = Command::ForgetCheckApproval { project: std::env::temp_dir(), name: "build".into() };
    for cmd in [ApprovalChoice::Once, ApprovalChoice::Always, ApprovalChoice::Never].map(|choice| Command::ApproveCheck { term: a, name: "test".into(), choice }).into_iter().chain([forget]) {
        d.message(7, ClientMsg::Command(cmd.clone()));
        assert!(matches!(out.try_recv(), Ok(ServerMsg::Error(e)) if e.contains("yours")), "{cmd:?}: a pane may not, whatever its grants");
    }
    assert_eq!(d.approvals.clone().for_run(a), kept.clone().for_run(a), "nothing granted, kept or forgotten");
    assert_eq!(check_of(&d, a).state, CheckState::NeedsApproval, "and nothing ran");
    close(&mut d, &[a]);
}

fn checking(d: &mut Daemon, t: TermId) {
    let info = CheckInfo { name: "test".into(), command: "npm test".into(), state: CheckState::Checking, ..Default::default() };
    d.terms.get_mut(&t).unwrap().checks.points = vec![check::CheckRecord::new(info, "").into()];
}

fn ran(outcome: check::CheckOutcome) -> check::CheckRun {
    check::CheckRun { outcome, output: String::new(), lines: 0, duration: Duration::from_millis(5) }
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

fn overdue(d: &mut Daemon, t: TermId) {
    d.terms.get_mut(&t).unwrap().alert_due.as_mut().unwrap().1 = Instant::now() - Duration::from_millis(1);
}

fn viewer(d: &mut Daemon) -> mpsc::Receiver<ServerMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    d.handle(Ev::Connected(40, tx, true));
    while rx.try_recv().is_ok() {}
    rx
}

fn heard(rx: &mut mpsc::Receiver<ServerMsg>) -> Vec<ServerMsg> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

#[test]
fn a_finished_alert_waits_for_the_check_and_rings_once_it_passes() {
    let (mut d, _rx) = daemon();
    let t = working_agent(&mut d);
    let mut seen = viewer(&mut d);
    checking(&mut d, t);
    d.set_status(t, Status::Done, "test");
    assert!(!heard(&mut seen).iter().any(|m| matches!(m, ServerMsg::Attention { status: Status::Done, .. })), "not announced while checking");
    // Even a deadline that is already due holds back.
    d.terms.get_mut(&t).unwrap().alert_due = Some((Status::Done, Instant::now() - Duration::from_millis(1)));
    d.settled_alerts();
    assert!(d.terms[&t].alerted.is_none() && d.terms[&t].alert_due.is_some(), "still held");
    check_ends(&mut d, t, check::CheckOutcome::Exited(0));
    assert!(heard(&mut seen).iter().any(|m| matches!(m, ServerMsg::Attention { status: Status::Done, .. })), "announced once it passed");
    d.clients.clear();
    overdue(&mut d, t);
    d.settled_alerts();
    assert_eq!(d.terms[&t].alerted.map(|(s, _)| s), Some(Status::Done));
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
    assert!(got.iter().any(|m| matches!(m, ServerMsg::Attention { term, status: Status::Blocked } if *term == t)));
    assert!(got.iter().any(|m| matches!(m, ServerMsg::Notice(n) if n.contains("check failed"))));
    assert!(!got.iter().any(|m| matches!(m, ServerMsg::Attention { status: Status::Done, .. })));
    assert!(d.terms[&t].alert_due.is_none(), "no finished alert is left to ring");
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
    assert!(got.iter().any(|m| matches!(m, ServerMsg::Notice(n) if n.contains("could not run"))));
    assert!(got.iter().any(|m| matches!(m, ServerMsg::Attention { status: Status::Done, .. })));
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
    assert!(heard(&mut seen).iter().any(|m| matches!(m, ServerMsg::Attention { status: Status::Done, .. })));
    d.clients.clear();
    overdue(&mut d, t);
    d.settled_alerts();
    assert_eq!(d.terms[&t].alerted.map(|(s, _)| s), Some(Status::Done));
    close(&mut d, &[t]);
}

#[test]
fn the_phone_hears_nothing_while_a_check_runs_or_after_it_fails() {
    let (mut d, _rx) = daemon();
    d.cfg.notify.phone_topic = "seshi-test-topic".into();
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
    d.terms.get_mut(&t).unwrap().queue.push_back("and the docs".into());
    checking(&mut d, t);
    d.set_status(t, Status::Done, "test");
    d.update_statuses();
    assert!(d.terms[&t].queue.is_empty() && d.terms[&t].queue_sent.is_some(), "sent as today");
    close(&mut d, &[t]);
}

// ---- several checkpoints and stale passes ---------------------------------------------

fn checkpoint_def(project: &std::path::Path, name: &str, command: &str) -> crate::config::CheckDef {
    crate::config::CheckDef { project: project.to_path_buf(), name: Some(name.into()), command: command.into(), timeout_s: LONG_CHECK_S }
}

/// An agent at work in `dir`, whose project names checkpoints `(name, command)` in order.
fn agent_with_checkpoints(d: &mut Daemon, dir: &std::path::Path, points: &[(&str, &str)]) -> TermId {
    d.cfg.check = points.iter().map(|(name, command)| checkpoint_def(dir, name, command)).collect();
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

fn states(d: &Daemon, t: TermId) -> Vec<(String, CheckState)> {
    d.terms[&t].checks.points.iter().map(|p| (p.latest.info.name.clone(), p.latest.info.state)).collect()
}

fn named_states(pairs: &[(&str, CheckState)]) -> Vec<(String, CheckState)> {
    pairs.iter().map(|(n, s)| (n.to_string(), *s)).collect()
}

fn point(d: &Daemon, t: TermId, name: &str) -> CheckInfo {
    d.terms[&t].checks.named(name).expect("the checkpoint").latest.info.clone()
}

fn announced(seen: &[ServerMsg], status: Status) -> bool {
    seen.iter().any(|m| matches!(m, ServerMsg::Attention { status: s, .. } if *s == status))
}

fn noticed(seen: &[ServerMsg], text: &str) -> usize {
    seen.iter().filter(|m| matches!(m, ServerMsg::Notice(n) if n.contains(text))).count()
}

#[test]
fn two_checkpoints_that_pass_run_in_order_and_finish_the_turn() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &[("build", "echo built; exit 0"), ("test", "echo tested; exit 0")]);
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Checking), ("test", CheckState::Pending)]));
    assert!(!announced(&heard(&mut seen), Status::Done), "held while they run");
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Passed), ("test", CheckState::Passed)]));
    assert!(announced(&heard(&mut seen), Status::Done), "done once both passed");
    let test = point(&d, t, "test");
    assert_eq!((test.turn, test.cwd.as_path(), test.line_count), (1, std::env::temp_dir().as_path(), 1));
    assert_eq!(d.check_output(t, Some("test")).map(str::trim), Some("tested"));
    assert_eq!(d.check_output(t, Some("build")).map(str::trim), Some("built"));
    assert_eq!(d.snapshot().terms[&t].checks.len(), 2, "clients see both");
    close(&mut d, &[t]);
}

#[test]
fn a_failed_checkpoint_does_not_stop_the_next_and_the_turn_reads_failed() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &[("build", "echo \"FAIL build\"; exit 2"), ("test", "echo tested; exit 0")]);
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Failed), ("test", CheckState::Passed)]));
    assert_eq!((point(&d, t, "build").exit_code, point(&d, t, "build").first_failing_line.as_str()), (Some(2), "FAIL build"));
    let got = heard(&mut seen);
    assert!(announced(&got, Status::Blocked) && !announced(&got, Status::Done), "a failure, never a finish");
    assert_eq!(noticed(&got, "check failed"), 1);
    assert_eq!(d.snapshot().terms[&t].check().map(|c| c.name.as_str()), Some("build"), "the failure is the headline");
    assert!(d.terms[&t].check_holds_done());
    close(&mut d, &[t]);
}

/// `build` runs; `test` waits for your approval of a changed command; `lint` you said never to.
fn gated(user: &[crate::config::CheckDef], project: &std::path::Path, worktree: &std::path::Path, approvals: &approvals::Approvals) -> Vec<check_source::Resolved> {
    let mut all = check_source::resolve(user, project, worktree, approvals);
    for r in &mut all {
        r.gate = match r.def.name() {
            "test" => check_source::Gate::NeedsApproval { previous: Some("npm t".into()) },
            "lint" => check_source::Gate::Never,
            _ => check_source::Gate::Run,
        };
    }
    all
}

fn never_any(user: &[crate::config::CheckDef], project: &std::path::Path, worktree: &std::path::Path, approvals: &approvals::Approvals) -> Vec<check_source::Resolved> {
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
    let points = [("build", "echo built; exit 0"), ("test", "npm test"), ("lint", "npm run lint")];
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &points);
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Passed), ("test", CheckState::NeedsApproval), ("lint", CheckState::Never)]));
    assert_eq!(point(&d, t, "test").previous_command.as_deref(), Some("npm t"));
    assert_eq!(d.check_output(t, Some("test")), Some(""), "it never ran");
    let got = heard(&mut seen);
    assert!(announced(&got, Status::Blocked) && !announced(&got, Status::Done));
    assert_eq!(noticed(&got, "approve check: npm test"), 1);
    assert!(d.terms[&t].check_holds_done(), "not done until you decide");
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    d.command(0, Command::RerunCheck { term: t, name: Some("build".into()) }).unwrap();
    finish_run(&mut d, &mut rx, t);
    assert_eq!(noticed(&heard(&mut seen), "approve check"), 0, "said once a turn");
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
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &[("build", "echo built; exit 0"), ("test", "echo tested; exit 0")]);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    let build = point(&d, t, "build");
    assert!(d.command(0, Command::RerunCheck { term: t, name: Some("lint".into()) }).is_err(), "no such checkpoint");
    d.command(0, Command::RerunCheck { term: t, name: Some("test".into()) }).unwrap();
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Passed), ("test", CheckState::Checking)]));
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Passed), ("test", CheckState::Passed)]));
    assert_eq!(point(&d, t, "build"), build, "the other is left as it was");
    assert_eq!(point(&d, t, "test").turn, 1, "a re-run checks the same turn");
    close(&mut d, &[t]);
}

#[test]
fn an_override_by_name_acts_on_that_checkpoint_and_says_who() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &[("build", "exit 1"), ("test", "exit 1")]);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    d.command(0, Command::OverrideCheck { term: t, name: Some("test".into()), reason: "flaky".into() }).unwrap();
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Failed), ("test", CheckState::Overridden)]));
    assert_eq!(point(&d, t, "test").overridden_by, "you");
    let reviewer = pane(&mut d);
    d.terms.get_mut(&reviewer).unwrap().label = "reviewer".into();
    d.terms.get_mut(&reviewer).unwrap().grants = Some(vec!["write".into()]);
    let (tx, mut out) = mpsc::channel(16);
    d.handle(Ev::Connected(7, tx, false));
    let token = d.terms[&reviewer].token.clone();
    d.handle(Ev::From(7, reviewer, token));
    d.message(7, ClientMsg::Command(Command::OverrideCheck { term: t, name: None, reason: String::new() }));
    assert!(matches!(out.try_recv(), Ok(ServerMsg::Reply(Reply::Ok))));
    assert_eq!(point(&d, t, "build").overridden_by, "reviewer", "None is the headline, and a pane signs with its name");
    d.command(0, Command::UndoOverride { term: t, name: Some("test".into()) }).unwrap();
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Overridden), ("test", CheckState::Failed)]));
    close(&mut d, &[t, reviewer]);
}

#[test]
fn a_restart_restores_each_checkpoint_with_its_own_output() {
    let (mut d, _rx) = daemon();
    let tails = persist::scratch_dir();
    d.check_dir = tails.path().to_path_buf();
    let old: TermId = 9_401;
    let tail = |name: &str, slot, text: &str| persist::CheckTail { term: old, name: name.into(), slot, text: text.into() };
    let written = [tail("build", persist::CheckSlot::Latest, "build out"), tail("build", persist::CheckSlot::Overridden, "build failed"), tail("test", persist::CheckSlot::Latest, "")];
    assert!(persist::save_check_outputs(tails.path(), persist::save_ticket(), &written, &[old]).is_empty());
    let info = |name: &str, state| CheckInfo { name: name.into(), command: format!("make {name}"), state, turn: 3, ..Default::default() };
    let saved = persist::SavedPane {
        checks: vec![info("build", CheckState::Overridden), info("test", CheckState::Pending)],
        checks_overridden: vec![info("build", CheckState::Overridden)],
        ..Default::default()
    };
    let t = pane(&mut d);
    let dir = d.check_dir.clone();
    d.terms.get_mut(&t).unwrap().restore_checks(&dir, old, &saved);
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Overridden), ("test", CheckState::CouldNotRun)]), "a waiting one died with the server");
    let checks = &d.terms[&t].checks;
    assert_eq!(checks.named("build").unwrap().latest.output, "build out");
    assert_eq!(checks.named("build").unwrap().overridden.as_ref().map(|o| o.output.as_str()), Some("build failed"));
    assert!(checks.named("test").unwrap().overridden.is_none());
    assert_eq!(checks.turn, 3, "the turn count carries on");
    let mut written = d.take_dirty_check_outputs();
    written.sort_by(|a, b| (&a.name, a.slot as u8).cmp(&(&b.name, b.slot as u8)));
    assert_eq!(written.iter().map(|w| (w.name.as_str(), w.text.as_str())).collect::<Vec<_>>(), [("build", "build out"), ("build", "build failed"), ("test", "")]);
    close(&mut d, &[t]);
}

#[test]
fn a_checkpoint_never_sees_the_panes_token() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    // SAFETY: the daemon's own environment, as when it was started from inside a pane; the
    // panes it spawns set their own token.
    unsafe { std::env::set_var("SESHI_PANE_TOKEN", "seshi-test-secret") };
    let print = if cfg!(windows) { "echo \"token=$env:SESHI_PANE_TOKEN\"" } else { "echo \"token=$SESHI_PANE_TOKEN\"" };
    let t = agent_with_checkpoints(&mut d, &std::env::temp_dir(), &[("env", print)]);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    assert_eq!(point(&d, t, "env").state, CheckState::Passed);
    assert_eq!(d.check_output(t, Some("env")).map(str::trim), Some("token="));
    close(&mut d, &[t]);
}

/// A repository of its own: one committed file, and a file it ignores.
fn temp_repo() -> persist::ScratchDir {
    let dir = persist::scratch_dir();
    std::fs::create_dir_all(dir.path()).unwrap();
    for args in [&["init", "-q"][..], &["config", "user.name", "t"], &["config", "user.email", "t@t"], &["config", "core.autocrlf", "false"]] {
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
    let found: Vec<_> = d.stale_probes().iter().flat_map(check_stale::probe).collect();
    if !found.is_empty() {
        d.handle(Ev::ChecksStale(found));
    }
}

#[test]
fn a_pass_goes_stale_when_the_files_change_and_a_failure_does_not() {
    let (rt, (mut d, mut rx)) = (runtime(), daemon());
    let _in_rt = rt.enter();
    let repo = temp_repo();
    let t = agent_with_checkpoints(&mut d, repo.path(), &[("build", "echo built; exit 0"), ("test", "exit 1")]);
    end_turn(&mut d, t);
    finish_run(&mut d, &mut rx, t);
    poll_stale(&mut d);
    std::fs::write(repo.path().join("ignored.txt"), "local\n").unwrap();
    poll_stale(&mut d);
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Passed), ("test", CheckState::Failed)]), "nothing that counts changed");
    let staged = crate::proc::git(repo.path(), &["diff", "--cached", "--name-only"]).unwrap();
    std::fs::write(repo.path().join("kept.txt"), "two\n").unwrap();
    std::fs::write(repo.path().join("new.txt"), "new\n").unwrap();
    d.terms.get_mut(&t).unwrap().status = Status::Working;
    poll_stale(&mut d);
    assert_eq!(point(&d, t, "build").state, CheckState::Passed, "not while the agent works");
    d.terms.get_mut(&t).unwrap().status = Status::Done;
    let before = term::unix_now();
    poll_stale(&mut d);
    assert_eq!(states(&d, t), named_states(&[("build", CheckState::Stale), ("test", CheckState::Failed)]), "only a pass goes stale");
    let build = point(&d, t, "build");
    assert!(build.stale_since.is_some_and(|at| at >= before));
    let changed: Vec<_> = build.changed_files.iter().map(|f| (f.kind, f.path.as_str())).collect();
    assert_eq!(changed, [('M', "kept.txt"), ('A', "new.txt")]);
    assert_eq!(crate::proc::git(repo.path(), &["diff", "--cached", "--name-only"]).unwrap(), staged, "the real index is untouched");
    d.command(0, Command::RerunCheck { term: t, name: Some("build".into()) }).unwrap();
    finish_run(&mut d, &mut rx, t);
    assert_eq!(point(&d, t, "build").state, CheckState::Passed, "a re-run replaces it");
    poll_stale(&mut d);
    assert_eq!(point(&d, t, "build").state, CheckState::Passed, "fresh against the files as they are");
    close(&mut d, &[t]);
}

#[test]
fn a_stale_pass_counts_as_finished_but_not_as_a_fresh_pass() {
    let mut checks = check::Checks::default();
    let info = CheckInfo { name: "test".into(), state: CheckState::Stale, ..Default::default() };
    checks.points.push(check::CheckRecord::new(info, "").into());
    assert!(!checks.holds_done() && !checks.running(), "finished");
    assert_ne!(checks.headline().map(|c| c.info.state), Some(CheckState::Passed));
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

/// An agent in the temp folder (its project) whose repo checkpoint `test` waits for you to
/// approve `command`.
fn waiting_for_approval(d: &mut Daemon, command: &str) -> TermId {
    let t = pane(d);
    let p = d.terms.get_mut(&t).unwrap();
    (p.agent, p.hooked, p.status) = (Some("claude".into()), true, Status::Done);
    let source = CheckSource::Repo { file: ".seshi/checks.toml".into(), commit: None, committed_at: None, uncommitted: true };
    let info = CheckInfo { name: "test".into(), command: command.into(), state: CheckState::NeedsApproval, source, ..Default::default() };
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
    let approve = |name: &str| Command::ApproveCheck { term: t, name: name.into(), choice: ApprovalChoice::Always };
    let err = d.command(0, approve("build")).expect_err("no such checkpoint waits");
    assert!(err.to_string().contains("isn't waiting"), "{err}");
    d.terms.get_mut(&t).unwrap().checks.points[0].latest.info.state = CheckState::Failed;
    assert!(d.command(0, approve("test")).is_err(), "a failed run is not a question");
    assert!(d.approvals.decisions.is_empty(), "nothing kept");
    close(&mut d, &[t]);
}

#[test]
fn always_and_never_are_kept_by_the_exact_command_in_seshi_s_data_folder() {
    let rt = runtime();
    let _in_rt = rt.enter();
    for (choice, decision) in [(ApprovalChoice::Always, approvals::Decision::Always), (ApprovalChoice::Never, approvals::Decision::Never)] {
        let (mut d, _rx, scratch) = approving_daemon();
        let t = waiting_for_approval(&mut d, "npm test");
        // The re-run that follows is the checkpoint runner's; only the decision is asserted here.
        let _ = d.command(0, Command::ApproveCheck { term: t, name: "TEST".into(), choice });
        let kept = d.approvals.find(&std::env::temp_dir(), "test").cloned().expect("a decision");
        assert_eq!((kept.name.as_str(), kept.command.as_str(), kept.decision), ("test", "npm test", decision));
        assert_eq!(kept.command_sha256, approvals::command_hash("npm test"));
        assert_eq!(saved_approvals(&d, &d.approvals_file.clone()).decisions, vec![kept], "saved off the loop");
        assert!(d.approvals_file.starts_with(scratch.path()));
        close(&mut d, &[t]);
    }
    let (d, _rx) = daemon();
    assert_eq!(d.approvals_file, approvals::path(), "outside every repo: Seshi's own data folder");
    assert!(d.approvals_file.starts_with(crate::config::data_dir()));
}

#[test]
fn once_lets_the_next_run_of_that_pane_go_and_then_asks_again() {
    let (mut d, _rx, _file) = approving_daemon();
    let (t, other) = (waiting_for_approval(&mut d, "npm test"), waiting_for_approval(&mut d, "npm test"));
    let _ = d.command(0, Command::ApproveCheck { term: t, name: "test".into(), choice: ApprovalChoice::Once });
    assert!(d.approvals.decisions.is_empty(), "once is never remembered");
    assert!(!d.approvals_for_run(other).granted_once("test", "npm test"), "another pane must ask for itself");
    assert!(d.approvals_for_run(t).granted_once("test", "npm test"), "this pane's next run may go");
    assert!(!d.approvals_for_run(t).granted_once("test", "npm test"), "and only that run");
    close(&mut d, &[t, other]);
}

#[test]
fn forgetting_a_decision_makes_the_next_run_ask_again() {
    let rt = runtime();
    let _in_rt = rt.enter();
    let (mut d, _rx, _file) = approving_daemon();
    let repo = std::env::temp_dir();
    d.approvals.decide(&repo, "test", "npm test", approvals::Decision::Never, 1);
    d.command(0, Command::ForgetCheckApproval { project: repo.clone(), name: "test".into() }).expect("yours to forget");
    assert_eq!(d.approvals.lookup(&repo, "test", "npm test"), None);
    assert!(saved_approvals(&d, &d.approvals_file.clone()).decisions.is_empty(), "and the file forgets it too");
    let err = d.command(0, Command::ForgetCheckApproval { project: repo, name: "test".into() }).expect_err("nothing left");
    assert!(err.to_string().contains("nothing is remembered"), "{err}");
}

#[test]
fn a_repo_checkpoint_waits_at_the_turn_end_and_once_runs_it_one_time() {
    let (rt, (mut d, mut rx, _file)) = (runtime(), approving_daemon());
    let _in_rt = rt.enter();
    let repo = temp_repo();
    std::fs::create_dir_all(repo.path().join(".seshi")).unwrap();
    std::fs::write(repo.path().join(crate::config::REPO_CHECKS_FILE), "[[check]]\nname = \"test\"\ncommand = \"echo tested\"\n").unwrap();
    let t = agent_with_checkpoints(&mut d, repo.path(), &[]);
    let mut seen = viewer(&mut d);
    end_turn(&mut d, t);
    assert!(d.terms[&t].check_running(), "the file is read off the loop; done waits for it");
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("test", CheckState::NeedsApproval)]));
    assert!(announced(&heard(&mut seen), Status::Blocked));
    d.command(0, Command::ApproveCheck { term: t, name: "test".into(), choice: ApprovalChoice::Once }).unwrap();
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("test", CheckState::Passed)]));
    assert_eq!(d.check_output(t, Some("test")).map(str::trim), Some("tested"));
    d.command(0, Command::RerunCheck { term: t, name: None }).unwrap();
    finish_run(&mut d, &mut rx, t);
    assert_eq!(states(&d, t), named_states(&[("test", CheckState::NeedsApproval)]), "once was one run");
    close(&mut d, &[t]);
}
