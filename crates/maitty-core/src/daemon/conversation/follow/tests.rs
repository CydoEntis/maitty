use super::*;
use crate::config::Config;
use crate::conversation::{ExecutionKind, Provider};
use crate::daemon::conversation::history::{Content, Role};
use crate::daemon::{ServerMsg, persist};
use crate::protocol::{Reply, Surface};
use serde_json::json;
use tokio::sync::mpsc;

const SESSION: &str = "aaaaaaaa-0000-4000-8000-000000000001";

fn claude_prompt(text: &str) -> String {
    format!(
        "{}\n",
        json!({
            "type": "user", "uuid": format!("u-{text}"), "isSidechain": false,
            "timestamp": "2026-11-04T09:30:00.000Z",
            "message": { "role": "user", "content": text },
        })
    )
}

fn claude_reply(text: &str) -> String {
    format!(
        "{}\n",
        json!({
            "type": "assistant", "uuid": format!("a-{text}"), "isSidechain": false,
            "timestamp": "2026-11-04T09:30:05.000Z",
            "message": { "role": "assistant", "content": [{ "type": "text", "text": text }] },
        })
    )
}

/// A scratch "home" with a Claude transcript in it.
struct Home {
    dir: persist::ScratchDir,
}

impl Home {
    fn new() -> Self {
        let dir = persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        Self { dir }
    }

    fn transcript(&self, session: &str) -> PathBuf {
        let folder = self.dir.path().join(".claude/projects/proj");
        std::fs::create_dir_all(&folder).unwrap();
        folder.join(format!("{session}.jsonl"))
    }
}

fn read(
    home: &Home,
    path: &Path,
    offset: Option<u64>,
    parser: Transcript,
) -> Result<Chunk, ReadFailure> {
    read_new(
        Provider::Claude,
        SESSION,
        Some(path),
        offset,
        parser,
        home.dir.path(),
    )
}

fn fresh() -> Transcript {
    Transcript::new(Format::Claude, SESSION)
}

#[test]
fn reads_only_what_was_added_and_waits_for_a_whole_line() {
    let home = Home::new();
    let path = home.transcript(SESSION);
    let whole = format!("{}{}", claude_prompt("one"), claude_reply("two"));
    let half = claude_prompt("three");
    std::fs::write(&path, format!("{whole}{}", &half[..half.len() / 2])).unwrap();

    let first = read(&home, &path, None, fresh()).ok().unwrap();

    assert_eq!(first.offset, whole.len() as u64);
    assert!(!first.notes.is_empty() && !first.start_missing && !first.restarted);

    std::fs::write(&path, format!("{whole}{half}")).unwrap();
    let second = read(&home, &path, Some(first.offset), first.parser)
        .ok()
        .unwrap();

    assert_eq!(second.offset, (whole.len() + half.len()) as u64);
    // The third line is a new prompt: it ends the first turn and starts a second.
    assert!(second.notes.iter().any(|n| n.method == "turn/started"));
    assert!(
        !second
            .notes
            .iter()
            .any(|n| n.params.to_string().contains("\"two\""))
    );
}

#[test]
fn a_file_that_shrank_is_read_again_from_its_start() {
    let home = Home::new();
    let path = home.transcript(SESSION);
    std::fs::write(
        &path,
        format!("{}{}", claude_prompt("one"), claude_reply("two")),
    )
    .unwrap();
    let first = read(&home, &path, None, fresh()).ok().unwrap();

    std::fs::write(&path, claude_prompt("fresh")).unwrap();
    let again = read(&home, &path, Some(first.offset), first.parser)
        .ok()
        .unwrap();

    assert!(again.restarted);
    assert!(
        again
            .notes
            .iter()
            .any(|n| n.params.to_string().contains("fresh"))
    );
}

#[test]
fn only_the_end_of_a_long_file_is_read_and_it_says_so() {
    let home = Home::new();
    let path = home.transcript(SESSION);
    let filler = format!(
        "{}\n",
        json!({ "type": "queue-operation", "content": "x".repeat(1000) })
    );
    let mut text = filler.repeat((MAX_FIRST_READ_BYTES as usize / filler.len()) + 8);
    text.push_str(&claude_prompt("the end"));
    std::fs::write(&path, &text).unwrap();

    let chunk = read(&home, &path, None, fresh()).ok().unwrap();

    assert!(chunk.start_missing);
    // One poll reads at most a chunk, and what it read is whole lines.
    assert!(chunk.offset <= text.len() as u64);
    assert!(chunk.offset > text.len() as u64 - MAX_FIRST_READ_BYTES);
}

#[test]
fn a_line_too_long_to_keep_is_dropped_not_stalled_on() {
    let home = Home::new();
    let path = home.transcript(SESSION);
    let huge = format!(
        "{}\n",
        json!({ "type": "user", "message": { "content": "x".repeat(MAX_LINE_BYTES + 10) } })
    );
    std::fs::write(&path, format!("{huge}{}", claude_prompt("after"))).unwrap();

    let chunk = read(&home, &path, None, fresh()).ok().unwrap();

    assert!(
        chunk
            .notes
            .iter()
            .any(|n| n.params.to_string().contains("after"))
    );
    assert!(
        !chunk
            .notes
            .iter()
            .any(|n| n.params.to_string().contains("xxxxxxxx"))
    );
}

#[test]
fn a_transcript_that_is_not_this_sessions_inside_the_home_folder_is_refused() {
    let home = Home::new();
    let other = home
        .dir
        .path()
        .join(".claude/projects/proj/someone-elses.jsonl");
    std::fs::create_dir_all(other.parent().unwrap()).unwrap();
    std::fs::write(&other, claude_prompt("secret")).unwrap();
    let text = home
        .dir
        .path()
        .join(".claude/projects/proj")
        .join(format!("{SESSION}.txt"));
    std::fs::write(&text, "x").unwrap();

    assert!(matches!(
        read(&home, &other, None, fresh()),
        Err(ReadFailure::Refused(_))
    ));
    assert!(matches!(
        read(&home, &text, None, fresh()),
        Err(ReadFailure::Refused(_))
    ));
    let missing = home.transcript("gone");
    assert!(matches!(
        read(&home, &missing, None, fresh()),
        Err(ReadFailure::NotFound)
    ));
}

#[test]
fn a_transcript_outside_the_home_folder_is_refused() {
    let home = Home::new();
    let elsewhere = Home::new();
    let path = elsewhere.transcript(SESSION);
    std::fs::write(&path, claude_prompt("not yours")).unwrap();

    assert!(matches!(
        read(&home, &path, None, fresh()),
        Err(ReadFailure::Refused(_))
    ));
}

#[cfg(unix)]
#[test]
fn a_link_out_of_the_home_folder_is_refused() {
    let home = Home::new();
    let elsewhere = Home::new();
    let target = elsewhere.transcript(SESSION);
    std::fs::write(&target, claude_prompt("not yours")).unwrap();
    let link = home.transcript(SESSION);
    std::os::unix::fs::symlink(&target, &link).unwrap();

    assert!(matches!(
        read(&home, &link, None, fresh()),
        Err(ReadFailure::Refused(_))
    ));
}

#[test]
fn a_codex_rollout_is_matched_by_the_end_of_its_name() {
    let home = Home::new();
    let folder = home.dir.path().join(".codex/sessions/2026/11/04");
    std::fs::create_dir_all(&folder).unwrap();
    let good = folder.join(format!("rollout-2026-11-04T10-00-00-{SESSION}.jsonl"));
    let wrong = folder.join("rollout-2026-11-04T10-00-00-bbbbbbbb.jsonl");
    std::fs::write(&good, "").unwrap();
    std::fs::write(&wrong, "").unwrap();
    let codex = |path: &Path| {
        read_new(
            Provider::Codex,
            SESSION,
            Some(path),
            None,
            Transcript::new(Format::Codex, SESSION),
            home.dir.path(),
        )
    };

    assert!(codex(&good).is_ok());
    // Another session's file is never read. A Codex hint that is not this session's is not trusted:
    // the file is looked for by the session id instead (`resolve_path`), and a file named for another
    // session is not what that finds either.
    assert!(codex(&wrong).is_err());
}

// ---- the daemon ----

fn own_data_folder() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!(
            "{}-follow-test-data-{}",
            crate::brand::APP_COMMAND,
            std::process::id()
        ));
        // SAFETY: before any pane of this test binary reads the environment, as the other daemon tests do.
        unsafe { std::env::set_var("XDG_DATA_HOME", dir) };
    });
}

struct World {
    daemon: Daemon,
    events: mpsc::Receiver<Ev>,
    home: Home,
    pane: TermId,
    path: PathBuf,
    client: mpsc::Receiver<ServerMsg>,
}

impl World {
    fn new() -> Self {
        own_data_folder();
        let (tx, events) = mpsc::channel(256);
        let mut daemon = Daemon::new(Config::default(), tx, || {});
        let home = Home::new();
        daemon.followed_home = Some(home.dir.path().to_path_buf());
        let pane = daemon.spawn(None, &std::env::temp_dir(), 80, 24).unwrap();
        let path = home.transcript(SESSION);
        std::fs::write(
            &path,
            format!("{}{}", claude_prompt("fix it"), claude_reply("fixed")),
        )
        .unwrap();
        let term = daemon.terms.get_mut(&pane).unwrap();
        term.owner = Surface::Desktop;
        term.agent = Some("claude".into());
        term.hooked = true;
        term.spare = false;
        term.session = Some(SESSION.into());
        term.transcript = Some(path.clone());
        let (reply_tx, client) = mpsc::channel(64);
        daemon.handle(Ev::Connected(61, reply_tx, false, Surface::Desktop, false));
        Self {
            daemon,
            events,
            home,
            pane,
            path,
            client,
        }
    }

    /// Runs the daemon's tick and applies the background read it starts.
    async fn follow(&mut self) {
        for follower in &mut self.daemon.followed {
            follower.last_read = None;
        }
        self.daemon.follow_tick();
        if !self.daemon.followed.iter().any(|f| f.in_flight) {
            return;
        }
        loop {
            let event = tokio::time::timeout(Duration::from_secs(20), self.events.recv())
                .await
                .expect("the read finishes")
                .unwrap();
            match event {
                Ev::FollowRead(done) => {
                    self.daemon.follow_read_done(*done);
                    return;
                }
                other => self.daemon.handle(other),
            }
        }
    }

    fn id(&self) -> ConversationId {
        self.daemon.followed[0].id
    }

    fn summary(&self) -> Option<crate::protocol::ConversationSummary> {
        self.daemon
            .conversation_summaries()
            .into_iter()
            .find(|c| c.native_term == Some(self.pane))
    }

    fn read_all(
        &mut self,
        who: crate::daemon::ClientId,
    ) -> anyhow::Result<crate::conversation::ReadResult> {
        self.daemon.conversation_read(who, self.id(), None, 100)?;
        loop {
            match self.client.try_recv() {
                Ok(ServerMsg::Reply(Reply::ConversationRead(read))) => return Ok(read),
                Ok(_) => {}
                Err(_) => anyhow::bail!("no reply"),
            }
        }
    }
}

#[tokio::test]
async fn a_hooked_claude_pane_gets_a_read_only_conversation_the_desktop_can_link() {
    let mut world = World::new();

    world.follow().await;

    let summary = world.summary().expect("the pane has a conversation");
    assert_eq!(summary.execution, ExecutionKind::Native);
    assert_eq!(summary.provider, Provider::Claude);
    assert_eq!(summary.native_term, Some(world.pane));
    assert!(!summary.archived && summary.checks.is_empty());
    let read = world.read_all(61).unwrap();
    assert_eq!(read.generation, summary.generation);
    let said: Vec<_> = read
        .history
        .entries
        .iter()
        .filter_map(|e| match &e.content {
            Content::Message { role, text } => Some((role.clone(), text.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        said,
        [
            (Role::User, "fix it".to_string()),
            (Role::Assistant, "fixed".to_string())
        ]
    );
    assert!(read.approvals.is_empty() && read.error.is_none());
}

#[tokio::test]
async fn a_session_with_no_transcript_file_yet_is_followed_empty_and_without_an_error() {
    let mut world = World::new();
    // A session that has not been given its first prompt: the agent has not written the file.
    std::fs::remove_file(&world.path).unwrap();

    world.follow().await;

    let read = world.read_all(61).unwrap();
    assert!(read.history.entries.is_empty());
    assert!(
        read.error.is_none(),
        "nothing written yet is not a fault: {:?}",
        read.error
    );
    assert!(!matches!(
        read.history.completeness,
        Completeness::Incomplete(_)
    ));

    // The first prompt writes the file, and the chat then shows it.
    std::fs::write(
        &world.path,
        format!("{}{}", claude_prompt("hello"), claude_reply("hi")),
    )
    .unwrap();
    world.follow().await;
    let read = world.read_all(61).unwrap();
    let said: Vec<_> = read
        .history
        .entries
        .iter()
        .filter_map(|e| match &e.content {
            Content::Message { role, text } => Some((role.clone(), text.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        said,
        [
            (Role::User, "hello".to_string()),
            (Role::Assistant, "hi".to_string())
        ]
    );
    assert!(read.error.is_none());
}

#[test]
fn a_codex_hint_that_is_not_this_sessions_file_falls_back_to_finding_it_by_its_session_id() {
    let home = Home::new();
    let dir = home.dir.path();
    let other = dir.join("rollout-2026-11-06T09-00-00-99999999-aaaa-bbbb-cccc-dddddddddddd.jsonl");
    let mine = dir.join(format!("rollout-2026-11-06T11-17-51-{SESSION}.jsonl"));
    std::fs::write(&other, "{}\n").unwrap();
    std::fs::write(&mine, "{}\n").unwrap();

    let found = resolve_path(Provider::Codex, SESSION, Some(&other), dir, || Some(mine.clone()));

    assert_eq!(found.ok(), Some(std::fs::canonicalize(&mine).unwrap()));
    // A hint for a file not written yet is not a reason to give up either.
    let missing = dir.join("rollout-not-yet.jsonl");
    let found = resolve_path(Provider::Codex, SESSION, Some(&missing), dir, || Some(mine.clone()));
    assert_eq!(found.ok(), Some(std::fs::canonicalize(&mine).unwrap()));
}

#[test]
fn a_claude_hint_that_is_not_this_sessions_file_stays_an_error() {
    let home = Home::new();
    let dir = home.dir.path();
    let other = dir.join("99999999-aaaa-bbbb-cccc-dddddddddddd.jsonl");
    std::fs::write(&other, "{}\n").unwrap();

    let found = resolve_path(Provider::Claude, SESSION, Some(&other), dir, || None);

    assert!(matches!(found, Err(ReadFailure::Refused(_))));
}

#[tokio::test]
async fn the_conversation_of_a_followed_claude_pane_reports_its_mode_and_model() {
    let mut world = World::new();
    let prompt = json!({ "type": "user", "uuid": "u-1", "isSidechain": false, "permissionMode": "acceptEdits",
        "timestamp": "2026-11-06T10:00:00.000Z", "message": { "role": "user", "content": "go" } });
    let reply = json!({ "type": "assistant", "uuid": "a-1", "isSidechain": false,
        "timestamp": "2026-11-06T10:00:01.000Z",
        "message": { "role": "assistant", "model": "claude-opus-5-1", "content": [{ "type": "text", "text": "ok" }] } });
    std::fs::write(&world.path, format!("{prompt}\n{reply}\n")).unwrap();

    world.follow().await;

    let meta = world.read_all(61).unwrap().meta.expect("the file said its setup");
    assert_eq!(meta.mode.as_deref(), Some("acceptEdits"));
    assert_eq!(meta.model.as_deref(), Some("claude-opus-5-1"));
}

#[tokio::test]
async fn a_followed_pane_whose_file_says_nothing_of_its_setup_reports_none() {
    let mut world = World::new();

    world.follow().await;

    assert!(world.read_all(61).unwrap().meta.is_none());
}

#[tokio::test]
async fn commands_that_would_drive_the_agent_are_refused_with_a_reason() {
    let mut world = World::new();
    world.follow().await;
    let (id, generation) = (world.id(), world.daemon.followed[0].generation);

    let send = world
        .daemon
        .conversation_send(61, id, generation, 1, "hi".into())
        .unwrap_err();
    let cancel = world
        .daemon
        .conversation_cancel(61, id, generation)
        .unwrap_err();
    let end = world
        .daemon
        .conversation_end(61, id, generation)
        .unwrap_err();

    for error in [send, cancel, end] {
        assert!(error.to_string().contains("read-only"), "{error}");
    }
}

#[tokio::test]
async fn reading_a_transcript_never_changes_the_panes_status() {
    let mut world = World::new();
    world.daemon.terms.get_mut(&world.pane).unwrap().status = Status::Working;
    world.follow().await;
    // The file says the turn ended; the pane's own runtime has not said so.
    let mut text = std::fs::read_to_string(&world.path).unwrap();
    text.push_str(&format!(
        "{}\n",
        json!({ "type": "system", "subtype": "turn_duration", "durationMs": 1 })
    ));
    std::fs::write(&world.path, text).unwrap();

    world.follow().await;

    assert_eq!(world.daemon.terms[&world.pane].status, Status::Working);
    assert_eq!(world.summary().unwrap().status, Status::Working);
    let read = world.read_all(61).unwrap();
    assert_eq!(read.status, Status::Working);
}

#[tokio::test]
async fn a_new_session_in_the_pane_starts_a_new_generation_and_the_old_read_is_dropped() {
    let mut world = World::new();
    world.follow().await;
    let id = world.id();
    let first_generation = world.daemon.followed[0].generation;
    let old_chunk = read(&world.home, &world.path, None, fresh()).ok().unwrap();
    let next = "bbbbbbbb-0000-4000-8000-000000000002";
    let next_path = world.home.transcript(next);
    std::fs::write(&next_path, claude_prompt("second session")).unwrap();
    let term = world.daemon.terms.get_mut(&world.pane).unwrap();
    term.session = Some(next.into());
    term.transcript = Some(next_path);

    world.daemon.follow_tick();

    let follower = &world.daemon.followed[0];
    assert_eq!(
        (follower.id, follower.generation),
        (id, first_generation + 1)
    );
    assert!(follower.history.entries.is_empty());
    // The read of the superseded session arrives late and changes nothing.
    world.daemon.follow_read_done(Done {
        term: world.pane,
        id,
        generation: first_generation,
        session: SESSION.into(),
        result: Ok(old_chunk),
    });
    assert!(world.daemon.followed[0].history.entries.is_empty());
    // A reader still holding a cursor of the old session is told to start over, not refused.
    let stale = Cursor {
        generation: first_generation,
        conversation: id,
        revision: 1,
        next_position: 0,
        epoch: 0,
        delta: false,
        snapshot: 0,
    };
    world.daemon.conversation_read(61, id, Some(stale), 10).unwrap();
    let read = loop {
        if let Ok(ServerMsg::Reply(Reply::ConversationRead(read))) = world.client.try_recv() {
            break read;
        }
    };
    assert!(read.history.reset);
    assert_eq!(read.generation, first_generation + 1);
}

#[tokio::test]
async fn only_verified_desktop_panes_running_claude_or_codex_with_a_known_session_are_followed() {
    type Change = fn(&mut crate::daemon::term::Term);
    let cases: [(&str, Change); 7] = [
        ("terminal app pane", |t| t.owner = Surface::Terminal),
        ("unverified status", |t| t.hooked = false),
        ("another agent", |t| t.agent = Some("gemini".into())),
        ("a shell", |t| t.agent = None),
        ("no session yet", |t| t.session = None),
        ("claude without a transcript", |t| t.transcript = None),
        ("the agent has exited", |t| t.process_exited = true),
    ];
    for (name, change) in cases {
        let mut world = World::new();
        change(world.daemon.terms.get_mut(&world.pane).unwrap());

        world.daemon.follow_tick();

        assert!(world.daemon.followed.is_empty(), "{name}");
        assert!(world.summary().is_none(), "{name}");
    }
    let mut world = World::new();
    world.daemon.follow_tick();
    assert_eq!(world.daemon.followed.len(), 1);
}

#[tokio::test]
async fn the_conversation_goes_when_the_agent_does_and_its_id_is_not_reused() {
    let mut world = World::new();
    world.follow().await;
    let first = world.id();

    world.daemon.terms.get_mut(&world.pane).unwrap().agent = None;
    world.daemon.follow_tick();
    assert!(world.summary().is_none());
    world.daemon.terms.get_mut(&world.pane).unwrap().agent = Some("claude".into());
    world.daemon.follow_tick();

    assert_ne!(world.id(), first);
}

#[tokio::test]
async fn the_terminal_app_cannot_read_what_the_desktop_apps_pane_did() {
    let mut world = World::new();
    world.follow().await;
    let (tui_tx, _tui_rx) = mpsc::channel(8);
    world
        .daemon
        .handle(Ev::Connected(62, tui_tx, false, Surface::Terminal, false));

    let error = world.read_all(62).unwrap_err();

    assert!(
        error.to_string().contains("No such conversation"),
        "{error}"
    );
}

#[tokio::test]
async fn a_file_that_cannot_be_read_is_said_not_shown_empty_as_if_complete() {
    let mut world = World::new();
    world.daemon.followed_home = Some(world.home.dir.path().join("elsewhere"));
    std::fs::create_dir_all(world.home.dir.path().join("elsewhere")).unwrap();

    world.follow().await;

    let read = world.read_all(61).unwrap();
    assert!(read.error.is_some());
    assert!(matches!(
        read.history.completeness,
        Completeness::Incomplete(_)
    ));
}
