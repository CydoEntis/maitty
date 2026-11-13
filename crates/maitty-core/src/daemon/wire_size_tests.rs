//! How big the two largest things the daemon sends routinely are on the wire: a full page of chat
//! history and the session list. Each test builds a realistic one, encodes it the way the wire
//! does (`protocol::encode`, named MessagePack) and fails when it outgrows its budget. A budget is
//! the size measured when it was set, plus 20%; `docs/PERFORMANCE.md` lists both.
//!
//! The fixtures use fixed values and ordered maps, so a size changes only when what the daemon
//! sends changes.

use super::conversation::history::{
    Completeness, Content, Entry, History, ItemState, MAX_READ_BYTES, MAX_READ_ENTRIES, Role,
    SourceId, Update,
};
use crate::conversation::{ConversationId, ExecutionKind, Provider, RecoveryState};
use crate::layout::Node;
use crate::protocol::*;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Measured 2026-11-13 on `dev` at dcb1091d2 with the command in `within_budget`: 106,020 bytes,
/// plus 20%.
const PAGE_BUDGET_BYTES: usize = 127_224;
/// Measured 2026-11-13 on `dev` at dcb1091d2 with the command in `within_budget`: 352,295 bytes,
/// plus 20%.
const SNAPSHOT_BUDGET_BYTES: usize = 422_754;

const SESSIONS: u32 = 50;
const PROJECTS: [&str; 5] = [
    "maitty",
    "affiliate-site",
    "pixls",
    "dev-deck",
    "plane-sync",
];
const ENTRIES_PER_TURN: usize = 8;
/// About 2026-11-13, in unix seconds; every time in the fixtures is an offset from it.
const NOW: u64 = 1_794_000_000;

const PROSE: &str = "The sidebar now keeps a session that is still starting, so a chat opened a moment \
ago no longer drops out of the list while its agent loads. The daemon sends the reason a pane left, \
and the window shows it instead of guessing. ";

fn within_budget(what: &str, encoded: usize, budget: usize) {
    println!("{what}: {encoded} bytes encoded, budget {budget}");
    assert!(
        encoded <= budget,
        "{what} encodes to {encoded} bytes, over its budget of {budget}. Something it carries \
         grew: find the field. If the growth is meant, re-measure with \
         `cargo test -p maitty-core wire_size -- --nocapture`, set the budget to the new size plus \
         20%, and update the figure in docs/PERFORMANCE.md."
    );
}

/// `len` bytes of plain English, the same every run.
fn prose(len: usize) -> String {
    PROSE.repeat(len / PROSE.len() + 1)[..len].to_string()
}

fn source(turn: usize, item: usize) -> SourceId {
    SourceId {
        conversation: ConversationId(7),
        turn: format!("019a5c1e-7b2d-7c40-9e11-{turn:012}"),
        item: if item == 0 {
            String::new()
        } else {
            format!("item_{turn:04}_{item:02}")
        },
    }
}

fn tool(label: &str, kind: &str, output: String, input: Option<serde_json::Value>) -> Content {
    Content::Tool {
        kind: kind.into(),
        label: label.into(),
        output,
        input,
        exit_code: Some(0),
        duration_ms: Some(4_812),
    }
}

/// One finished turn, as a person and an agent produce it: the turn's own status row, the
/// prompt, a short reply, a file read, a test run, an edit with its diff, a plan, and the
/// closing reply.
fn turn_contents() -> [Content; ENTRIES_PER_TURN] {
    let file = "crates/maitty-core/src/daemon/conversation/history.rs";
    [
        Content::TurnStatus,
        Content::Message {
            role: Role::User,
            text: prose(240),
        },
        Content::Message {
            role: Role::Assistant,
            text: prose(300),
        },
        tool(
            &format!("Read {file}"),
            "commandExecution",
            prose(1_200),
            Some(json!({ "file_path": format!("/home/dev/src/maitty/{file}") })),
        ),
        tool(
            "cargo test -p maitty-core history",
            "commandExecution",
            prose(1_000),
            Some(json!({
                "command": "cargo test -p maitty-core history",
                "description": "Run the history tests",
            })),
        ),
        tool(
            &format!("Edited {file}"),
            "fileChange",
            format!("{file} (update)\n@@ -320,7 +320,9 @@\n{}", prose(800)),
            None,
        ),
        tool("Plan", "plan", prose(350), None),
        Content::Message {
            role: Role::Assistant,
            text: prose(700),
        },
    ]
}

fn history_entries() -> Vec<Entry> {
    let started = NOW * 1000;
    (0..MAX_READ_ENTRIES / ENTRIES_PER_TURN)
        .flat_map(|turn| {
            turn_contents()
                .into_iter()
                .enumerate()
                .map(move |(item, content)| Entry {
                    source: source(turn, item),
                    content,
                    state: ItemState::Completed,
                    started_at_ms: Some(started + (turn * 60_000 + item * 1_000) as u64),
                    completed_at_ms: Some(started + (turn * 60_000 + item * 1_000 + 900) as u64),
                })
        })
        .collect()
}

#[test]
fn a_full_history_page_stays_within_its_wire_budget() {
    let mut history = History::new(ConversationId(7));
    for entry in history_entries() {
        history.apply(Update::UpsertItem(entry)).unwrap();
    }
    history.set_completeness(Completeness::Complete).unwrap();

    let page = history
        .read(None, MAX_READ_ENTRIES, MAX_READ_BYTES)
        .unwrap();

    assert_eq!(
        page.entries.len(),
        MAX_READ_ENTRIES,
        "the page is a full one"
    );
    // The cursor's epoch comes from the clock, but it is always above 2^32 and so always encodes
    // as the same nine bytes.
    within_budget(
        "A history page of 128 entries",
        encode(&page).unwrap().len(),
        PAGE_BUDGET_BYTES,
    );
}

fn project_of(i: u32) -> &'static str {
    PROJECTS[i as usize % PROJECTS.len()]
}

fn root_of(i: u32) -> PathBuf {
    PathBuf::from(format!("/home/dev/src/{}", project_of(i)))
}

fn branch_of(i: u32) -> String {
    format!("feat/session-{i:02}-keep-the-sidebar-steady")
}

fn checkout_of(i: u32) -> PathBuf {
    PathBuf::from(format!(
        "/home/dev/src/{}-worktrees/session-{i:02}",
        project_of(i)
    ))
}

fn status_of(i: u32) -> Status {
    [Status::Working, Status::Blocked, Status::Done, Status::Idle][i as usize % 4]
}

/// A test and a build checkpoint, the test one failing with its first failing line.
fn checks_of(i: u32) -> Vec<CheckInfo> {
    let check = |name: &str, command: &str, state, exit_code| CheckInfo {
        command: command.into(),
        state,
        exit_code: Some(exit_code),
        started_at: NOW - 600,
        duration_ms: 41_250,
        after_turn_ended_at: NOW - 610,
        first_failing_line: if exit_code == 0 {
            String::new()
        } else {
            "test daemon::history::tests::cursor_survives_a_rebuild ... FAILED".into()
        },
        name: name.into(),
        cwd: checkout_of(i),
        turn: 12,
        line_count: 1_840,
        changed_files: vec![ChangedFile {
            kind: 'M',
            path: "crates/maitty-core/src/daemon/mod.rs".into(),
        }],
        source: CheckSource::Repo {
            file: ".maitty/checks.toml".into(),
            commit: Some("dcb1091d2a4f".into()),
            committed_at: Some(NOW - 86_400),
            uncommitted: false,
        },
        ..CheckInfo::default()
    };
    vec![
        check("test", "cargo test --workspace", CheckState::Failed, 101),
        check("build", "cargo build", CheckState::Passed, 0),
    ]
}

/// As many turn events as a pane keeps, a day's worth of prompts, tool runs and stops.
fn timeline() -> Vec<TurnEvent> {
    let kinds = [
        (TurnKind::Working, "hook: UserPromptSubmit"),
        (TurnKind::CommandStarted, ""),
        (TurnKind::CommandFinished, "exit 0"),
        (TurnKind::NeedsYou, "hook: Notification"),
        (TurnKind::Done, "hook: Stop"),
        (TurnKind::Idle, "you looked"),
    ];
    (0..super::term::TIMELINE_KEPT as u64)
        .map(|n| {
            let (kind, detail) = kinds[n as usize % kinds.len()];
            TurnEvent {
                at: NOW - 3_600 + n * 30,
                kind,
                detail: detail.into(),
            }
        })
        .collect()
}

fn helper(i: u32) -> SubagentActivity {
    SubagentActivity {
        id: format!("agent-{i:02}-explore"),
        name: "Explore".into(),
        parent: Some(format!("session-{i:02}")),
        state: SubagentState::Completed,
        task: Some("Find where the session list drops a starting pane".into()),
        model: Some("sonnet 4.5".into()),
        effort: Some("medium".into()),
        started_at: Some(NOW - 900),
        updated_at: NOW - 840,
        latest_activity: Some("Read crates/maitty-core/src/daemon/mod.rs".into()),
        result: Some(prose(200)),
    }
}

fn pr_of(i: u32) -> PrInfo {
    PrInfo {
        number: 340 + i,
        url: format!("https://github.com/example/maitty/pull/{}", 340 + i),
        title: "Keep a starting session in the sidebar".into(),
        state: PrState::Open,
        draft: false,
        checks: PrChecks {
            passed: 6,
            pending: 1,
            failed: vec!["windows-latest / test".into()],
        },
        review: PrReview::Required,
        conflicting: false,
    }
}

fn ask() -> PermissionAsk {
    PermissionAsk {
        tool: "Bash".into(),
        detail: "git push origin feat/session-keep-the-sidebar-steady".into(),
        answerable: true,
        choices: vec![
            "Yes".into(),
            "Yes, and don't ask again for git push".into(),
            "No, and tell Claude what to do differently".into(),
        ],
        questions: Vec::new(),
    }
}

/// One agent session as the daemon lists it: a Claude or Codex pane in its own worktree, with a
/// status and why, checks, a full timeline, usage, a helper, and what its state carries (a
/// permission prompt when it needs you, a pull request when done, a resume time and a failed
/// command when idle, quiet and repeat counts while working).
fn pane(i: u32) -> TermInfo {
    let status = status_of(i);
    let agent = if i.is_multiple_of(2) {
        "claude"
    } else {
        "codex"
    };
    TermInfo {
        conversation_id: Some(ConversationId(1_000 + u64::from(i))),
        id: i + 1,
        cols: 160,
        rows: 48,
        title: format!("✳ Keep session {i:02} in the sidebar"),
        process: agent.into(),
        remote: None,
        popup: false,
        agent: Some(agent.into()),
        hooked: true,
        status,
        cwd: checkout_of(i),
        summary: prose(120),
        name: format!("Keep session {i:02} in the sidebar while it starts"),
        label: String::new(),
        pinned: i.is_multiple_of(10),
        archived: false,
        model: "opus 4.5".into(),
        mem: 412_000_000,
        cpu: 3.5,
        bell: status == Status::Done,
        said: prose(300),
        last_lines: String::new(),
        branch: Some(branch_of(i)),
        linked: true,
        diff_added: 128,
        diff_removed: 42,
        subagents: vec!["Explore".into()],
        subagent_activity: vec![helper(i)],
        session: Some(format!("8f3c2a10-5d4e-4b7a-9c21-{i:012}")),
        effort: Some("high".into()),
        started_by: None,
        tab_of: None,
        helper_ask: Vec::new(),
        pr: (status == Status::Done).then(|| pr_of(i)),
        queued: vec!["Then run the GUI tests too".into()],
        root: Some(root_of(i)),
        top: Some(checkout_of(i)),
        since: NOW - 120,
        status_why: "hook: Stop".into(),
        why_hook: "Stop".into(),
        why_hook_at: NOW - 120,
        why_screen: "matched `esc to interrupt`".into(),
        why_screen_at: NOW - 130,
        why_process_at: NOW - 2,
        quiet_since: (status == Status::Working).then_some(NOW - 300),
        repeats: (status == Status::Working).then_some(4),
        pid: 48_000 + i,
        asleep: false,
        win32_input: false,
        usage: Usage {
            context: Some(62.5),
            cost: Some(4.82),
            tokens: Some(1_284_000),
            tokens_in: Some(1_201_000),
            tokens_out: Some(83_000),
        },
        resume_at: (status == Status::Idle).then_some(NOW + 7_200),
        checks: checks_of(i),
        asking: if status == Status::Blocked {
            "Bash: git push origin feat/session-keep-the-sidebar-steady\n1. Yes\n2. No".into()
        } else {
            String::new()
        },
        interrupted: None,
        timeline: timeline(),
        ask: (status == Status::Blocked).then(ask),
        exit: (status == Status::Idle).then_some(ExitInfo {
            code: Some(1),
            at: NOW - 400,
        }),
    }
}

fn conversation(i: u32) -> ConversationSummary {
    ConversationSummary {
        id: ConversationId(1_000 + u64::from(i)),
        generation: 3,
        provider: if i.is_multiple_of(2) {
            Provider::Claude
        } else {
            Provider::Codex
        },
        execution: ExecutionKind::Native,
        cwd: checkout_of(i),
        worktree: Some(checkout_of(i)),
        state: RecoveryState::Running,
        status: status_of(i),
        native_term: Some(i + 1),
        checks: checks_of(i),
        title: Some(format!("Keep session {i:02} in the sidebar")),
        archived: false,
        root: Some(root_of(i)),
        branch: Some(branch_of(i)),
    }
}

/// A project with its sessions, one tab each, and its worktrees.
fn workspace(index: u32) -> WorkspaceInfo {
    let sessions: Vec<u32> = (0..SESSIONS)
        .filter(|i| *i as usize % PROJECTS.len() == index as usize)
        .collect();
    WorkspaceInfo {
        id: index + 1,
        name: project_of(index).into(),
        cwd: root_of(index),
        tabs: sessions
            .iter()
            .map(|i| TabInfo {
                id: 100 + i,
                name: format!("session {i:02}"),
                layout: Node::Leaf(i + 1),
                focus: i + 1,
            })
            .collect(),
        active_tab: 100 + sessions[0],
        git: Some(GitInfo {
            repo: project_of(index).into(),
            branch: "dev".into(),
            dirty: 3,
            linked: false,
            root: root_of(index),
            worktrees: sessions
                .iter()
                .map(|i| WorktreeEntry {
                    path: checkout_of(*i),
                    branch: branch_of(*i),
                    main: false,
                    repo: project_of(index).into(),
                })
                .collect(),
            ahead: 0,
        }),
        worktree: false,
        color: index as u8,
        is_new: false,
        group: Some("work".into()),
    }
}

/// The session list a window is sent with 50 agent sessions open across five projects, each
/// session also a chat in the desktop app, plus what the daemon carries beside them: questions,
/// a secret request, plan limits, the day's work, and two panes that just left.
fn snapshot() -> Snapshot {
    Snapshot {
        conversations: (0..SESSIONS).map(conversation).collect(),
        daemon_version: "0.76.0".into(),
        daemon_id: Some("d4f1c0a2-6b3e-4e8f-a1d7-3c9b2e5f8a01".into()),
        workspaces: (0..PROJECTS.len() as u32).map(workspace).collect(),
        active_ws: Some(1),
        terms: (0..SESSIONS).map(|i| (i + 1, pane(i))).collect(),
        questions: vec![HumanQuestion {
            id: 9,
            term: 2,
            text: "Should the starting row show a spinner or the agent's name?".into(),
            options: vec!["Spinner".into(), "Agent name".into()],
        }],
        secret_requests: vec![SecretRequest {
            id: 4,
            term: 6,
            label: "NPM_TOKEN".into(),
            reason: "publish the desktop app's package".into(),
        }],
        limits: vec![
            (
                "claude".into(),
                vec![
                    Limit {
                        name: "5h".into(),
                        used: 41.0,
                        resets_at: NOW + 7_200,
                    },
                    Limit {
                        name: "week".into(),
                        used: 63.0,
                        resets_at: NOW + 302_400,
                    },
                ],
            ),
            (
                "codex".into(),
                vec![Limit {
                    name: "5h".into(),
                    used: 12.0,
                    resets_at: NOW + 9_000,
                }],
            ),
        ],
        spent_today: 38.75,
        today: PROJECTS
            .iter()
            .map(|project| AgentDay {
                agent: "claude".into(),
                place: format!("/home/dev/src/{project}"),
                turns: 46,
                working_secs: 9_840,
                cost: 7.75,
            })
            .collect(),
        revision: 4_821,
        removed: BTreeMap::from([(51, RemovalReason::Closed), (52, RemovalReason::Exited)]),
    }
}

#[test]
fn a_fifty_session_list_stays_within_its_wire_budget() {
    let snapshot = snapshot();

    assert_eq!(snapshot.terms.len(), SESSIONS as usize);
    within_budget(
        "A Snapshot of 50 sessions",
        encode(&snapshot).unwrap().len(),
        SNAPSHOT_BUDGET_BYTES,
    );
}
