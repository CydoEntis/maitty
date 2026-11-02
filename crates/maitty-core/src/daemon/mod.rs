//! The daemon: owns every PTY and the workspace tree, survives clients detaching.
//!
//! All state lives in one event loop (`Daemon::run`). PTY reader threads, the process
//! scanner, background git work and client connections talk to it through a single
//! channel, so there is no locking around the model.

mod approvals;
mod check;
pub use check::first_failing_line;
mod check_source;
mod check_stale;
mod commands;
mod contain;
mod conversation;
mod git;
mod helpers;
mod pane_terminal;
mod persist;
mod pr;
mod repo;
mod restore;
mod scan;
mod secrets;
mod status;
mod term;
mod usage;
mod worktrees;

use crate::brand::APP_COMMAND;
use status::*;
use worktrees::*;

use crate::config::{CompiledAgent, Config, same_path};
use crate::ipc;
use crate::layout::Node;
use crate::protocol::*;
use anyhow::Result;
use interprocess::local_socket::traits::tokio::Listener as _;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use term::{SpawnSpec, Term};
use tokio::sync::mpsc;

type ClientId = u64;

/// How often the daemon looks at its timers (statuses, saves and git).
const TICK: Duration = Duration::from_millis(250);
const INTEGRATION_STARTUP_GRACE: Duration = Duration::from_secs(3);
/// How often each workspace's git state is read again.
const GIT_POLL_EVERY: Duration = Duration::from_secs(3);
/// How often a changed session is saved to disk.
const SAVE_EVERY: Duration = Duration::from_secs(2);
/// How often panes' new output is saved to disk (what's lost if the server is killed).
const OUTPUT_SAVE_EVERY: Duration = Duration::from_secs(30);
/// A pane's CPU share is passed on when it has moved this many points.
const CPU_STEP: f32 = 5.0;
/// How long a pane started as an agent may take to show that agent's process.
const AGENT_SHOWS_WITHIN: Duration = Duration::from_secs(45);
/// The most follow-ups one agent can have waiting.
const QUEUE_MOST: usize = 20;
/// A helper just started counts as working this long: its agent hasn't reported yet.
const HELPER_SETTLES: Duration = Duration::from_secs(45);
/// How often the app-made worktrees whose folder is gone are forgotten, and how soon after
/// the server starts the first check is.
const VANISHED_CHECK_EVERY: Duration = Duration::from_secs(10 * 60);
const VANISHED_CHECK_FIRST: Duration = Duration::from_secs(60);
/// A daemon nothing ever used (no panes, no clients) exits after this.
const EXIT_WHEN_UNUSED_FOR: Duration = Duration::from_secs(30);
/// How long a pane or agent ending on its own is kept out of the saved session. A shutdown
/// or reboot kills agents and shells a moment before the server: a server still running
/// after this saw a real exit, one killed within it keeps every session for the next start.
const LOSS_HOLD: Duration = Duration::from_secs(15);

pub(crate) enum ConversationFileResponse {
    Entries(Vec<crate::conversation::ConversationFileEntry>),
    Content(String),
}

#[allow(clippy::large_enum_variant)]
pub enum Ev {
    Connected(ClientId, mpsc::Sender<ServerMsg>, bool, Surface),
    StructuredConnected {
        id: ClientId,
        tx: mpsc::Sender<ServerMsg>,
        attach: bool,
        claim: StructuredIdentity,
    },
    /// The connection's request comes from inside this pane (with this secret).
    From(ClientId, TermId, String),
    Msg(ClientId, ClientMsg),
    Disconnected(ClientId),
    Conversation(conversation::Event),
    ConversationValidated {
        token: u64,
        result: Result<conversation::transport::Launch, String>,
    },
    ConversationStorage {
        id: crate::conversation::ConversationId,
        revision: Option<u64>,
        result: Result<(), String>,
    },
    ConversationChecksResolved {
        id: crate::conversation::ConversationId,
        generation: u64,
        run: u64,
        resolved: Vec<check_source::Resolved>,
        tree: Option<String>,
    },
    ConversationCheckRan {
        id: crate::conversation::ConversationId,
        generation: u64,
        run: u64,
        name: String,
        result: check::CheckRun,
    },
    ConversationChecksStale(Vec<conversation::checks::StaleFound>),
    SessionSaved {
        ticket: u64,
        text: String,
        result: Result<persist::SessionCommit, String>,
        barrier: Option<u64>,
    },
    Output(TermId, Vec<u8>),
    Exited(TermId),
    ProcessExited(TermId),
    Scan(Vec<scan::Found>),
    Git(Vec<(WsId, Option<GitInfo>)>),
    Prs(Vec<(pr::Place, pr::Read)>),
    WorktreeCreated {
        client: ClientId,
        branch: String,
        cmd: Option<String>,
        split: Option<TermId>,
        /// The pane that asked, noted when it asked: its connection is usually gone by now.
        by: Option<TermId>,
        /// The repo's setup (its on_create hook) has run: open what was asked for.
        set_up: bool,
        result: Result<(PathBuf, String), String>,
    },
    WorktreeRemoved {
        client: ClientId,
        path: PathBuf,
        result: Result<(), String>,
    },
    /// Numbers read from Codex's session files (`usage::poll_codex`).
    /// What Codex's session files say: usage, limits, and what runs under each pane's session.
    CodexUsage {
        facts: Vec<usage::CodexPaneFacts>,
        limits: Option<Vec<Limit>>,
    },
    /// A worktree made for an agent started from a shell (`Command::AgentWorktree`).
    AgentWorktreeMade {
        client: ClientId,
        result: Result<PathBuf, String>,
    },
    /// What became of a worktree after you closed its last pane.
    WorktreeAutoRemoved {
        client: ClientId,
        path: PathBuf,
        result: Removal,
    },
    /// App-made worktrees whose folder is already gone.
    WorktreesVanished {
        gone: Vec<PathBuf>,
    },
    /// A clean-up you asked for is done: what went (and whether its merged branch went
    /// too), and what stayed and why.
    WorktreesTidied {
        client: ClientId,
        removed: Vec<(String, bool)>,
        kept: Vec<(String, String)>,
    },
    WorktreeList {
        client: ClientId,
        result: Result<Vec<WorktreeEntry>, String>,
    },
    RepoReady {
        client: ClientId,
        tag: u64,
        result: Result<crate::repo::RepoReply, String>,
    },
    ConversationDiffReady {
        client: ClientId,
        id: crate::conversation::ConversationId,
        generation: u64,
        result: Result<crate::conversation::ConversationDiff, String>,
    },
    ConversationFilesReady {
        client: ClientId,
        id: crate::conversation::ConversationId,
        generation: u64,
        result: Result<ConversationFileResponse, String>,
    },
    /// A worktree hook ran: what to tell people.
    HookRan(String),
    /// A spare worktree for prewarming: (repo, result).
    SpareMade(PathBuf, Result<(PathBuf, String), String>),
    /// A worktree made for an agent to move into: (client, pane, branch, result).
    MoveReady {
        client: ClientId,
        term: TermId,
        branch: String,
        result: Result<(PathBuf, String), String>,
    },
    /// A status report, its sender checked (see `HookJob`).
    HookChecked {
        msg: ClientMsg,
        verdict: Option<bool>,
        chain: Vec<u32>,
    },
    /// A run's checkpoints, resolved and gated, and the worktree's tree as it started (see
    /// `Daemon::turn_ended`).
    ChecksResolved {
        term: TermId,
        run: u64,
        resolved: Vec<check_source::Resolved>,
        tree: Option<String>,
    },
    /// One checkpoint of run `run` ended.
    CheckRan {
        term: TermId,
        run: u64,
        name: String,
        result: check::CheckRun,
    },
    /// Passes the git poll found the files changed under.
    ChecksStale(Vec<check_stale::Found>),
    /// Which panes' worktrees have a repo checks file: (pane, the worktree looked in, found).
    RepoChecksFound(Vec<(TermId, PathBuf, bool)>),
    /// Check output tails a background save could not write.
    CheckOutputsUnsaved(Vec<TermId>),
}

/// How to put back what an automatic workspace changed.
enum AutoUndo {
    /// The pane moved out of `from_ws`/`from_tab` (beside `beside`).
    Moved {
        term: TermId,
        from_ws: WsId,
        from_tab: TabId,
        beside: Option<TermId>,
    },
    /// The pane was alone, so its workspace was re-homed; restore name and folder.
    Rehomed {
        ws: WsId,
        name: String,
        cwd: PathBuf,
    },
}

struct Client {
    tx: mpsc::Sender<ServerMsg>,
    attach: bool,
    /// The app it is (from its handshake): decides which terminal sessions it may use.
    surface: Surface,
    /// The pane it acts from (its secret checked): what it may do is that pane's grants.
    /// None: you (the app's window, or a terminal outside the app).
    from: Option<TermId>,
    structured: Option<conversation::Owner>,
    rejected: bool,
    /// Its queue filled up (a stalled terminal, a slow SSH link): it's dropped, and can
    /// attach again for a fresh copy of everything.
    behind: std::cell::Cell<bool>,
}

/// Messages waiting for one client before it counts as too far behind.
const CLIENT_QUEUE: usize = 8192;

impl Client {
    fn push(&self, msg: ServerMsg) {
        if let Err(mpsc::error::TrySendError::Full(_)) = self.tx.try_send(msg) {
            self.behind.set(true);
        }
    }
}

struct Daemon {
    cfg: Config,
    agents: Vec<CompiledAgent>,
    terms: HashMap<TermId, Term>,
    workspaces: Vec<WorkspaceInfo>,
    active_ws: Option<WsId>,
    next_id: u32,
    clients: HashMap<ClientId, Client>,
    tx: mpsc::Sender<Ev>,
    /// To the hook thread (see `hook_thread`).
    hook_jobs: std::sync::mpsc::Sender<HookJob>,
    scan: Arc<Mutex<scan::Shared>>,
    dirty: bool,
    had_terms: bool,
    empty_since: Instant,
    /// Background git work in flight; the server stays up until it lands.
    pending_ops: u32,
    /// Questions agents asked you: the question, who's waiting for the answer, and the
    /// pane's status before it asked.
    questions: Vec<(HumanQuestion, ClientId, Status)>,
    secrets: secrets::Secrets,
    prs: pr::Prs,
    last_secret_files: Instant,
    git_busy: bool,
    last_git: Instant,
    last_save: Instant,
    last_output_save: Instant,
    last_saved: String,
    /// What was last noted for whoever replaces this server (see `restart`).
    last_activity: Option<crate::restart::Activity>,
    /// Where check output tails are kept.
    check_dir: PathBuf,
    /// Which checkpoints a project has and whether each may run (tests put in their own).
    resolver: check_source::Resolver,
    /// What you decided about repos' check commands, read once at start: a hand edit of the
    /// file counts only after a restart.
    approvals: approvals::Approvals,
    approvals_file: PathBuf,
    /// The projects where you said sessions may always start helpers, read once at start.
    helper_projects: helpers::Projects,
    helper_projects_file: PathBuf,
    /// Worktrees the app created; closing the last thing in one removes it.
    made_worktrees: Vec<PathBuf>,
    conversations: Vec<crate::conversation::Conversation>,
    pending_starts: Vec<crate::conversation::PendingStart>,
    conversation_operations: Vec<crate::conversation::Operation>,
    conversation_states: BTreeMap<crate::conversation::ConversationId, conversation::State>,
    /// Conversations whose agent was stopped on purpose, so its exit is not a failure.
    conversation_ending: std::collections::BTreeSet<crate::conversation::ConversationId>,
    conversation_requests: BTreeMap<u64, conversation::commands::Pending>,
    conversation_request_id: u64,
    conversation_barriers: BTreeMap<u64, u64>,
    conversation_histories:
        BTreeMap<crate::conversation::ConversationId, conversation::history::History>,
    conversation_cache: Option<conversation::history_storage::Cache>,
    conversation_cache_dirty: std::collections::BTreeSet<crate::conversation::ConversationId>,
    conversation_cache_writes: BTreeMap<crate::conversation::ConversationId, u64>,
    session_path: PathBuf,
    session_saved_ticket: u64,
    conversation_runtimes: conversation::Registry,
    conversation_history: conversation::history::Attachments,
    /// Plan limits by agent, as last reported.
    limits: BTreeMap<String, Vec<Limit>>,
    /// What sessions cost, as it was reported (when, US dollars), over the last day.
    /// Costs reported over the last day: (when, cost, agent, folder).
    spent: Vec<(u64, f64, String, String)>,
    /// Turns agents finished over the last day: (when, agent, folder, seconds working).
    turns: Vec<(u64, String, String, u64)>,
    codex_busy: bool,
    last_codex: Instant,
    /// The next pane runs its command once and ends with it (a popup).
    next_once: bool,
    /// The pane the next session is started for, when its connection can't say (the
    /// worktree it asked for took a while).
    next_starter: Option<TermId>,
    /// The prewarmed agent: (repo, its worktree, its pane), and whether one is being made.
    spare: Option<(PathBuf, PathBuf, TermId)>,
    spare_making: bool,
    /// A spare being handed over: the next pane spawned in this folder is it.
    adopt: Option<(PathBuf, TermId)>,
    /// The spare's folder, handed over: its on_create hook already ran.
    adopted: Option<PathBuf>,
    last_vanished_check: Instant,
    /// The last automatic workspace move, for undo.
    auto_undo: Option<AutoUndo>,
    /// When a pane's process or a pane's agent last ended on its own (see `LOSS_HOLD`).
    last_loss: Option<Instant>,
}

pub fn run(on_start: impl FnOnce()) -> Result<()> {
    crate::proc::ahead_of_the_panes();
    init_logging();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(async move {
        // Refuse to start a second daemon on the same socket.
        if ipc::connect().await.is_ok() {
            anyhow::bail!(
                concat!(
                    "a ",
                    env!("APP_COMMAND"),
                    " daemon is already running on {}"
                ),
                ipc::socket_id()
            );
        }
        let listener = ipc::listen()?;
        tracing::info!("daemon listening on {}", ipc::socket_id());
        // Lets `<app> kill-server` from another version stop it (it can't talk to this one).
        let _ = std::fs::write(ipc::pid_file(), std::process::id().to_string());
        let (tx, rx) = mpsc::channel::<Ev>(4096);

        let accept_tx = tx.clone();
        tokio::spawn(async move {
            let mut next: ClientId = 1;
            loop {
                match listener.accept().await {
                    Ok(stream) => {
                        let id = next;
                        next += 1;
                        tokio::spawn(serve(id, stream, accept_tx.clone()));
                    }
                    Err(e) => {
                        tracing::error!("accept failed: {e}");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });

        if let Ok(p) = std::fs::read_to_string(spare_file()) {
            let p = PathBuf::from(p.trim());
            tokio::task::spawn_blocking(move || drop_spare_dir(&p));
            save_spare(None);
        }
        let (cfg, err) = Config::load_or_default();
        if let Some(e) = err {
            tracing::warn!("config: {e}");
        }
        let mut d = Daemon::new(cfg, tx, on_start);
        match approvals::Approvals::load(&d.approvals_file) {
            Ok(a) => d.approvals = a,
            Err(e) => tracing::warn!("check approvals: {e:#}; every repo check asks again"),
        }
        match helpers::Projects::load(&d.helper_projects_file) {
            Ok(p) => d.helper_projects = p,
            Err(e) => tracing::warn!("helper projects: {e:#}; every project asks again"),
        }
        d.drop_saved_outputs();
        if d.cfg.restore.enabled
            && let Some(saved) = persist::load()
        {
            d.restore_startup(saved);
        }
        d.initialize_conversation_cache().await?;
        d.run(rx).await;
        Ok(())
    })
}

fn init_logging() {
    let dir = crate::config::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("daemon.log"))
    {
        let _ = tracing_subscriber::fmt()
            .with_writer(Mutex::new(file))
            .with_ansi(false)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_env(concat!(env!("APP_ENV"), "_LOG"))
                    .unwrap_or_else(|_| "info".into()),
            )
            .try_init();
    }
}

async fn serve(
    id: ClientId,
    stream: interprocess::local_socket::tokio::Stream,
    ev: mpsc::Sender<Ev>,
) {
    let (mut r, mut w) = ipc::framed(stream);
    let (attach, from, structured, surface) = match ipc::recv_client(&mut r).await {
        Ok(Some(ClientMsg::Hello {
            attach,
            from,
            structured,
            surface,
            ..
        })) if from.is_none() || structured.is_none() => (attach, from, structured, surface),
        _ => return,
    };
    if ipc::send(
        &mut w,
        &ServerMsg::Welcome {
            version: PROTOCOL_VERSION,
        },
    )
    .await
    .is_err()
    {
        return;
    }
    let (tx, mut rx) = mpsc::channel::<ServerMsg>(CLIENT_QUEUE);
    let connected = match structured {
        Some(claim) => Ev::StructuredConnected {
            id,
            tx,
            attach,
            claim,
        },
        None => Ev::Connected(id, tx, attach, surface),
    };
    if ev.send(connected).await.is_err() {
        return;
    }
    if let Some((term, token)) = from
        && ev.send(Ev::From(id, term, token)).await.is_err()
    {
        return;
    }
    let mut writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let bye = matches!(msg, ServerMsg::Bye);
            if ipc::send(&mut w, &msg).await.is_err() || bye {
                break;
            }
        }
    });
    loop {
        tokio::select! {
            // Nothing more will be sent (the daemon let go of a client that fell behind, or
            // said goodbye): close the connection. Left half open, a window keeps sending
            // commands and is never told what they did, so it looks frozen.
            _ = &mut writer => break,
            msg = ipc::recv_client(&mut r) => match msg {
                Ok(Some(msg)) => {
                    if ev.send(Ev::Msg(id, msg)).await.is_err() {
                        break;
                    }
                }
                _ => break,
            },
        }
    }
    let _ = ev.send(Ev::Disconnected(id)).await;
    writer.abort();
}

/// "removed 3 worktrees (1 merged branch went too); kept rate: it has uncommitted changes".
pub(super) fn tidied_note(removed: &[(String, bool)], kept: &[(String, String)]) -> String {
    let mut parts = Vec::new();
    if !removed.is_empty() {
        let also = match removed.iter().filter(|(_, branch)| *branch).count() {
            0 => String::new(),
            1 => " (1 merged branch went too)".to_string(),
            n => format!(" ({n} merged branches went too)"),
        };
        parts.push(format!(
            "removed {} worktree{}{also}",
            removed.len(),
            if removed.len() == 1 { "" } else { "s" }
        ));
    }
    parts.extend(kept.iter().map(|(name, why)| format!("kept {name}: {why}")));
    if parts.is_empty() {
        "nothing to clean up".into()
    } else {
        parts.join("; ")
    }
}

/// What a pane may do through the app (see `[mcp] grants`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Grant {
    Read,
    Write,
    Start,
    Respond,
    Admin,
}

impl Grant {
    fn name(self) -> &'static str {
        match self {
            Grant::Read => "read",
            Grant::Write => "write",
            Grant::Start => "start",
            Grant::Respond => "respond",
            Grant::Admin => "admin",
        }
    }
    fn says(self) -> &'static str {
        match self {
            Grant::Read => "read other panes",
            Grant::Write => "type into other panes",
            Grant::Start => "start sessions",
            Grant::Respond => "answer another agent's prompt or question",
            Grant::Admin => concat!("close other panes or stop ", env!("APP_COMMAND")),
        }
    }
}

impl Daemon {
    fn client_is_agent(&self, id: ClientId) -> bool {
        self.clients.get(&id).is_some_and(|client| {
            client.from.is_some() || client.structured.is_some() || client.rejected
        })
    }
    fn conversation_generation_current(&self, owner: conversation::Owner) -> bool {
        self.conversations
            .iter()
            .any(|record| record.id == owner.conversation && record.generation == owner.generation)
            || self.pending_starts.iter().any(|pending| {
                pending.id == owner.conversation && pending.generation == owner.generation
            })
    }
    fn structured_current(&self, owner: conversation::Owner) -> bool {
        self.conversation_runtimes.authenticates(owner)
            && self.conversation_generation_current(owner)
    }
    /// May this connection do `g` (to pane `target`)? You always may; a pane may do anything
    /// to itself, and the rest per its grants.
    pub(super) fn may(
        &self,
        client: ClientId,
        g: Grant,
        target: Option<TermId>,
    ) -> anyhow::Result<()> {
        if self
            .clients
            .get(&client)
            .is_some_and(|client| client.rejected)
        {
            anyhow::bail!("Structured runtime identity was rejected");
        }
        if self
            .clients
            .get(&client)
            .is_some_and(|client| client.structured.is_some())
        {
            if self.cfg.mcp.grants.iter().any(|grant| grant == g.name()) {
                return Ok(());
            }
            anyhow::bail!(
                "this structured runtime may not {} through {APP_COMMAND}",
                g.says()
            );
        }
        let Some(from) = self.clients.get(&client).and_then(|c| c.from) else {
            return Ok(());
        };
        if target == Some(from) && g != Grant::Admin {
            return Ok(());
        }
        let grants = self
            .terms
            .get(&from)
            .and_then(|t| t.grants.clone())
            .unwrap_or_else(|| self.cfg.mcp.grants.clone());
        // Letting agents approve prompts (Settings → Agents) is letting them respond.
        let respond_by_setting = g == Grant::Respond && self.cfg.mcp.approve != "never";
        if respond_by_setting || grants.iter().any(|x| x == g.name()) {
            return Ok(());
        }
        anyhow::bail!(
            "this pane may not {} through {APP_COMMAND} ({APP_COMMAND} grant {from} {} allows it)",
            g.says(),
            g.name()
        )
    }

    /// The grant a command needs.
    fn may_command(&self, client: ClientId, cmd: &Command) -> anyhow::Result<()> {
        let structured = self
            .clients
            .get(&client)
            .is_some_and(|client| client.structured.is_some());
        if structured
            && matches!(
                cmd,
                Command::ConversationStart { .. }
                    | Command::NewWorkspace { .. }
                    | Command::NewTab { .. }
                    | Command::Split { .. }
                    | Command::NewWorktree { .. }
                    | Command::Popup { .. }
            )
        {
            anyhow::bail!(
                "Structured runtimes cannot start descendants until conversation-owned helper consent is supported"
            );
        }
        match cmd {
            Command::ConversationSend { .. } if structured => self.may(client, Grant::Write, None),
            Command::ConversationApprove { .. } | Command::ConversationAnswerInput { .. }
                if structured
                    || self
                        .clients
                        .get(&client)
                        .is_some_and(|client| client.from.is_some()) =>
            {
                anyhow::bail!("Only a human client can answer provider requests")
            }
            Command::ConversationApprove { .. } => self.may(client, Grant::Respond, None),
            Command::ConversationAnswerInput { .. } => self.may(client, Grant::Respond, None),
            Command::ConversationCancel { .. }
                if structured
                    || self
                        .clients
                        .get(&client)
                        .is_some_and(|client| client.from.is_some()) =>
            {
                anyhow::bail!("Only a human client can cancel a structured turn")
            }
            Command::ConversationCancel { .. } => self.may(client, Grant::Write, None),
            Command::ConversationEnd { .. }
                if structured
                    || self
                        .clients
                        .get(&client)
                        .is_some_and(|client| client.from.is_some()) =>
            {
                anyhow::bail!("Only a human client can end a conversation")
            }
            Command::ConversationEnd { .. } => self.may(client, Grant::Write, None),
            Command::ConversationRename { .. } | Command::ConversationArchive { .. }
                if structured
                    || self
                        .clients
                        .get(&client)
                        .is_some_and(|client| client.from.is_some()) =>
            {
                anyhow::bail!("Only a human client can rename or put away a conversation")
            }
            Command::ConversationRename { .. } | Command::ConversationArchive { .. } => {
                self.may(client, Grant::Write, None)
            }
            Command::ConversationTerminalInput { .. }
            | Command::ConversationTerminalResize { .. }
                if structured
                    || self
                        .clients
                        .get(&client)
                        .is_some_and(|client| client.from.is_some()) =>
            {
                anyhow::bail!("Only a human client can type into a native conversation terminal")
            }
            Command::ConversationTerminalInput { .. }
            | Command::ConversationTerminalResize { .. } => self.may(client, Grant::Write, None),
            Command::TerminalInput { .. } | Command::TerminalResize { .. }
                if structured
                    || self
                        .clients
                        .get(&client)
                        .is_some_and(|client| client.from.is_some()) =>
            {
                anyhow::bail!("Only a human client can type into a terminal this way")
            }
            Command::TerminalInput { term, .. } | Command::TerminalResize { term, .. } => {
                self.may(client, Grant::Write, Some(*term))
            }
            Command::NewWorkspace { .. }
            | Command::NewTab { .. }
            | Command::Split { .. }
            | Command::NewWorktree { .. }
            | Command::Popup { .. } => self.may(client, Grant::Start, None),
            Command::ClosePane { term } => {
                self.may_use_terminal(client, *term)?;
                self.may(client, Grant::Admin, Some(*term)).or_else(|e| {
                    // Closing itself is fine.
                    if self.clients.get(&client).and_then(|c| c.from) == Some(*term) {
                        Ok(())
                    } else {
                        Err(e)
                    }
                })
            }
            Command::CloseWorkspace { ws } => {
                for term in self.workspace_panes(*ws) {
                    self.may_use_terminal(client, term)?;
                }
                self.may(client, Grant::Admin, None)
            }
            Command::CloseTab { ws, tab } => {
                let panes: Vec<TermId> = self
                    .workspaces
                    .iter()
                    .filter(|w| w.id == *ws)
                    .flat_map(|w| w.tabs.iter().filter(|t| t.id == *tab))
                    .flat_map(|t| t.layout.leaves())
                    .collect();
                for term in panes {
                    self.may_use_terminal(client, term)?;
                }
                self.may(client, Grant::Admin, None)
            }
            Command::RemoveWorktree { .. }
            | Command::CloseWorktree { .. }
            | Command::TidyWorktrees { .. }
            | Command::KillServer { .. } => self.may(client, Grant::Admin, None),
            // Typing into another pane, only later.
            Command::QueueFollowUp { term, .. } | Command::ClearQueue { term } => {
                self.may(client, Grant::Write, Some(*term))
            }
            Command::RerunCheck { term, .. } => self.may(client, Grant::Write, Some(*term)),
            // The pane's own claim never counts, so a pane may not accept its own failure.
            Command::OverrideCheck { term, .. } | Command::UndoOverride { term, .. } => {
                if self.clients.get(&client).and_then(|c| c.from) == Some(*term)
                    || self
                        .clients
                        .get(&client)
                        .and_then(|c| c.structured)
                        .is_some_and(|owner| {
                            self.terms.get(term).is_some_and(|pane| {
                                pane.conversation_id == Some(owner.conversation)
                            })
                        })
                {
                    anyhow::bail!("a pane may not override its own check; that is yours to decide");
                }
                self.may(client, Grant::Write, Some(*term))
            }
            Command::AnswerHuman { id, .. } => {
                let asker = self
                    .questions
                    .iter()
                    .find(|(q, ..)| q.id == *id)
                    .map(|(q, ..)| q.term);
                self.may(client, Grant::Respond, asker.filter(|_| false))
            }
            // Only you hand out grants.
            Command::Grant { .. } if self.client_is_agent(client) => {
                anyhow::bail!(concat!(
                    "grants are yours to give: run ",
                    env!("APP_COMMAND"),
                    " grant outside ",
                    env!("APP_COMMAND"),
                    "'s panes, or use the pane's menu"
                ))
            }
            // Approving a repo's command is approving what it runs: an agent may not.
            Command::ApproveCheck { .. } | Command::ForgetCheckApproval { .. }
                if self.client_is_agent(client) =>
            {
                anyhow::bail!("check approvals are yours to give, not a pane's")
            }
            Command::RequestSecret { term, .. }
                if self.clients.get(&client).and_then(|c| c.from) != Some(*term) =>
            {
                anyhow::bail!("only a pane asks for its own secret")
            }
            Command::AnswerSecret { .. } if self.client_is_agent(client) => {
                anyhow::bail!("a secret is yours to give, not a pane's")
            }
            Command::LinkPr { term, .. }
                if self
                    .clients
                    .get(&client)
                    .and_then(|c| c.from)
                    .is_some_and(|from| from != *term) =>
            {
                anyhow::bail!("a pane links only its own pull request")
            }
            Command::SpendSecret { .. } if !self.client_is_agent(client) => {
                anyhow::bail!("only an agent pane can spend a secret reference")
            }
            // A session may not let itself (or another) start helpers.
            Command::AnswerHelpers { .. } if self.client_is_agent(client) => {
                anyhow::bail!("starting helpers is yours to allow, not a pane's")
            }
            _ if structured => {
                anyhow::bail!("This control is not supported for structured runtime callers")
            }
            _ => Ok(()),
        }
    }
}

/// Discard the saved session (for `kill-server --forget` when the server couldn't be asked).
pub fn forget_session() {
    persist::forget();
}

/// Agents in the saved session: what a restart of a server that can't be asked would resume.
pub fn saved_agent_count() -> usize {
    persist::load().map_or(0, |s| {
        s.workspaces
            .iter()
            .flat_map(|w| &w.tabs)
            .flat_map(|t| t.panes.values())
            .filter(|p| p.agent.is_some())
            .count()
    })
}

impl Daemon {
    /// A daemon with no panes yet; its process scanner and hook thread are running.
    fn new(cfg: Config, tx: mpsc::Sender<Ev>, on_start: impl FnOnce()) -> Daemon {
        contain::children_die_with_us();
        // Clicking a notification opens its the app:// link (Windows needs to be told how).
        if !cfg!(test) {
            crate::reveal::register();
        }
        on_start();
        let agents = cfg.agent_defs();
        let scan = Arc::new(Mutex::new(scan::Shared {
            roots: Vec::new(),
            agents: agents.clone(),
            interval: Duration::from_millis(cfg.detection.scan_interval_ms),
        }));
        scan::start(scan.clone(), tx.clone());
        let hook_jobs = hook_thread(tx.clone());
        Daemon {
            cfg,
            agents,
            terms: HashMap::new(),
            workspaces: Vec::new(),
            active_ws: None,
            next_id: 1,
            clients: HashMap::new(),
            hook_jobs,
            tx,
            scan,
            dirty: false,
            had_terms: false,
            empty_since: Instant::now(),
            pending_ops: 0,
            questions: Vec::new(),
            secrets: secrets::Secrets::default(),
            prs: pr::Prs::default(),
            last_secret_files: Instant::now(),
            git_busy: false,
            last_git: crate::clock::ago(GIT_POLL_EVERY),
            last_save: Instant::now(),
            last_output_save: Instant::now(),
            last_saved: String::new(),
            last_activity: None,
            check_dir: persist::check_output_dir(),
            resolver: check_source::resolve,
            approvals: approvals::Approvals::default(),
            approvals_file: approvals::path(),
            helper_projects: helpers::Projects::default(),
            helper_projects_file: helpers::path(),
            made_worktrees: Vec::new(),
            conversations: Vec::new(),
            pending_starts: Vec::new(),
            conversation_operations: Vec::new(),
            conversation_states: BTreeMap::new(),
            conversation_ending: std::collections::BTreeSet::new(),
            conversation_requests: BTreeMap::new(),
            conversation_request_id: 0,
            conversation_barriers: BTreeMap::new(),
            conversation_histories: BTreeMap::new(),
            conversation_cache: None,
            conversation_cache_dirty: Default::default(),
            conversation_cache_writes: Default::default(),
            session_path: if cfg!(test) {
                PathBuf::new()
            } else {
                persist::path()
            },
            session_saved_ticket: 0,
            conversation_runtimes: conversation::Registry::default(),
            conversation_history: conversation::history::Attachments::default(),
            limits: BTreeMap::new(),
            spent: Vec::new(),
            turns: Vec::new(),
            codex_busy: false,
            last_codex: Instant::now() - Duration::from_secs(3600),
            next_once: false,
            next_starter: None,
            spare: None,
            spare_making: false,
            adopt: None,
            adopted: None,
            last_vanished_check: crate::clock::ago(VANISHED_CHECK_EVERY - VANISHED_CHECK_FIRST),
            last_loss: None,
            auto_undo: None,
        }
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<Ev>) {
        let mut tick = tokio::time::interval(TICK);
        loop {
            tokio::select! {
                Some(ev) = rx.recv() => {
                    self.handle(ev);
                    // Drain whatever else is queued before broadcasting state once.
                    while let Ok(ev) = rx.try_recv() {
                        self.handle(ev);
                    }
                }
                _ = tick.tick() => {
                    self.update_statuses();
                    self.settled_alerts();
                    self.phone_alerts();
                    self.auto_continue();
                    self.poll_codex();
                    self.poll_git();
                    self.sweep_secrets();
                    self.poll_prs();
                    if self.last_vanished_check.elapsed() >= VANISHED_CHECK_EVERY {
                        self.last_vanished_check = Instant::now();
                        self.forget_vanished_worktrees();
                    }
                    self.publish_activity();
                    if self.last_save.elapsed() > SAVE_EVERY {
                        self.last_save = Instant::now();
                        self.persist(false);
                        self.save_conversation_caches();
                    }
                    if self.last_output_save.elapsed() > OUTPUT_SAVE_EVERY && !self.losing() {
                        self.last_output_save = Instant::now();
                        self.save_outputs(false);
                    }
                }
            }
            if self.dirty {
                self.dirty = false;
                self.sync_scan_roots();
                for surface in [Surface::Terminal, Surface::Desktop] {
                    let snap = self.snapshot_for(surface);
                    self.broadcast(
                        |client| client.attach && client.surface == surface,
                        ServerMsg::State(snap),
                    );
                }
            }
            if self.should_exit() {
                tracing::info!("no panes left; exiting");
                // The user closed everything, so start fresh next time.
                persist::forget();
                self.broadcast(|_| true, ServerMsg::Bye);
                tokio::time::sleep(Duration::from_millis(100)).await;
                let _ = std::fs::remove_file(ipc::pid_file());
                crate::restart::clear();
                std::process::exit(0);
            }
        }
    }

    fn activity(&self) -> crate::restart::Activity {
        let live = || self.terms.values().filter(|t| !t.spare);
        let structured: Vec<_> = self
            .conversation_summaries()
            .into_iter()
            .filter(|c| c.native_term.is_none())
            .collect();
        crate::restart::Activity {
            pid: std::process::id(),
            version: env!("APP_VERSION").into(),
            sessions: live().count() + structured.len(),
            working: live().filter(|t| t.status == Status::Working).count()
                + structured
                    .iter()
                    .filter(|c| c.status == Status::Working)
                    .count(),
            waiting: live().filter(|t| t.status == Status::Blocked).count()
                + structured
                    .iter()
                    .filter(|c| c.status == Status::Blocked)
                    .count(),
        }
    }

    /// Note how many sessions are mid-turn, so a newer the app can decide whether replacing
    /// this server needs asking (it may speak a protocol this one doesn't).
    fn publish_activity(&mut self) {
        let activity = self.activity();
        if self.last_activity.as_ref() != Some(&activity) && !cfg!(test) {
            crate::restart::publish(&activity);
            self.last_activity = Some(activity);
        }
    }

    /// Look at git again on the next tick (something just changed it).
    fn poll_git_soon(&mut self) {
        self.last_git = crate::clock::ago(GIT_POLL_EVERY);
    }

    fn should_exit(&self) -> bool {
        // A window still showing the app (empty projects, the splash) keeps it open: closing
        // the last pane mustn't close the app under you.
        if !self.terms.is_empty()
            || !self.conversations.is_empty()
            || !self.pending_starts.is_empty()
            || self.pending_ops > 0
            || self.clients.values().any(|c| c.attach)
            || self.losing()
        {
            return false;
        }
        // Exit once the last pane is closed and no window is left, or if nobody ever started one.
        self.had_terms
            || (self.clients.is_empty() && self.empty_since.elapsed() > EXIT_WHEN_UNUSED_FOR)
    }

    fn broadcast(&self, filter: impl Fn(&Client) -> bool, msg: ServerMsg) {
        for c in self.clients.values().filter(|c| {
            !c.rejected
                && filter(c)
                && c.structured.is_none_or(|owner| {
                    self.structured_current(owner)
                        && self.cfg.mcp.grants.iter().any(|grant| grant == "read")
                })
        }) {
            c.push(msg.clone());
        }
    }

    /// Let go of clients whose queue filled up; their connection closes and they can attach
    /// again.
    fn drop_lagging_clients(&mut self) {
        self.clients.retain(|id, c| {
            if c.behind.get() {
                tracing::warn!("client {id} fell too far behind; dropping it");
            }
            !c.behind.get()
        });
    }

    fn send(&self, client: ClientId, msg: ServerMsg) {
        if let Some(c) = self.clients.get(&client) {
            if (c.rejected
                || c.structured
                    .is_some_and(|owner| !self.structured_current(owner)))
                && !matches!(&msg, ServerMsg::Error(_) | ServerMsg::Bye)
            {
                c.push(ServerMsg::Error(
                    "Structured runtime identity is stale or rejected".into(),
                ));
                return;
            }
            c.push(msg);
        }
    }

    // ---- persistence ---------------------------------------------------------------

    /// The first palette colour no workspace is using (cycling once all are taken).
    fn free_color(&self) -> u8 {
        let n = self.cfg.ui.workspace_colors.len().clamp(1, 255) as u8;
        (0..n)
            .find(|c| !self.workspaces.iter().any(|w| w.color == *c))
            .unwrap_or((self.workspaces.len() % n as usize) as u8)
    }

    fn poll_git(&mut self) {
        if self.git_busy || self.workspaces.is_empty() || self.last_git.elapsed() < GIT_POLL_EVERY {
            return;
        }
        for t in self.terms.values_mut() {
            if t.refresh_head() {
                self.dirty = true;
            }
        }
        self.git_busy = true;
        self.last_git = Instant::now();
        self.probe_repo_checks(self.terms.keys().copied().collect::<Vec<_>>());
        let dirs: Vec<(WsId, PathBuf)> = self
            .workspaces
            .iter()
            .map(|w| (w.id, w.cwd.clone()))
            .collect();
        let dir_of: HashMap<WsId, PathBuf> = dirs.iter().cloned().collect();
        let tx = self.tx.clone();
        let stale_probes = self.stale_probes();
        let conversation_stale_probes = self.conversation_stale_probes();
        tokio::task::spawn_blocking(move || {
            check_stale::report(&stale_probes, &tx);
            conversation::checks::report_stale(&conversation_stale_probes, &tx);
            let mut res: Vec<(WsId, Option<GitInfo>)> = dirs
                .into_iter()
                .map(|(id, dir)| (id, git::status(&dir)))
                .collect();
            // One worktree listing per repository, shared by its workspaces.
            let mut lists: HashMap<PathBuf, Vec<WorktreeEntry>> = HashMap::new();
            for (_, info) in res.iter_mut() {
                let Some(g) = info else { continue };
                let list = lists
                    .entry(g.root.clone())
                    .or_insert_with(|| git::list_worktrees(&g.root).unwrap_or_default());
                g.worktrees = list.clone();
            }
            for (id, info) in res.iter_mut() {
                let Some(g) = info.as_mut().filter(|g| g.linked) else {
                    continue;
                };
                if let (Some(base), Some(dir)) = (
                    g.worktrees
                        .iter()
                        .find(|e| e.main)
                        .map(|e| e.branch.clone()),
                    dir_of.get(id),
                ) {
                    g.ahead = git::ahead_of(dir, &base);
                }
            }
            let _ = tx.blocking_send(Ev::Git(res));
        });
    }

    // ---- events --------------------------------------------------------------------

    fn handle(&mut self, ev: Ev) {
        self.drop_lagging_clients();
        match ev {
            Ev::Connected(id, tx, attach, surface) => {
                let c = Client {
                    tx,
                    attach,
                    surface,
                    from: None,
                    structured: None,
                    rejected: false,
                    behind: std::cell::Cell::new(false),
                };
                if attach {
                    c.push(ServerMsg::State(self.snapshot_for(surface)));
                    for t in self.terms.values().filter(|t| self.shown_to(t.id, surface)) {
                        c.push(ServerMsg::Replay {
                            term: t.id,
                            cols: t.cols,
                            rows: t.rows,
                            data: t.replay(),
                        });
                    }
                }
                self.clients.insert(id, c);
            }
            Ev::StructuredConnected {
                id,
                tx,
                attach,
                claim,
            } => {
                let owner = self
                    .conversation_runtimes
                    .authenticate(&claim)
                    .filter(|owner| self.conversation_generation_current(*owner));
                let Some(owner) = owner else {
                    self.clients.insert(
                        id,
                        Client {
                            tx,
                            attach: false,
                            surface: Surface::Terminal,
                            from: None,
                            structured: None,
                            rejected: true,
                            behind: std::cell::Cell::new(false),
                        },
                    );
                    self.send(
                        id,
                        ServerMsg::Error(
                            "Structured runtime identity is invalid or no longer live".into(),
                        ),
                    );
                    return;
                };
                let c = Client {
                    tx,
                    attach,
                    surface: Surface::Terminal,
                    from: None,
                    structured: Some(owner),
                    rejected: false,
                    behind: std::cell::Cell::new(false),
                };
                self.clients.insert(id, c);
                if attach {
                    if let Err(error) = self.may(id, Grant::Read, None) {
                        self.send(id, ServerMsg::Error(error.to_string()));
                        if let Some(client) = self.clients.get_mut(&id) {
                            client.rejected = true;
                            client.attach = false;
                        }
                        return;
                    }
                    self.send(id, ServerMsg::State(self.snapshot_for(Surface::Terminal)));
                    for term in self
                        .terms
                        .values()
                        .filter(|t| self.shown_to(t.id, Surface::Terminal))
                    {
                        self.send(
                            id,
                            ServerMsg::Replay {
                                term: term.id,
                                cols: term.cols,
                                rows: term.rows,
                                data: term.replay(),
                            },
                        );
                    }
                }
            }
            Ev::From(id, term, token) => {
                // Only with the pane's secret: anything else can't claim to be it.
                let ok = self
                    .terms
                    .get(&term)
                    .is_some_and(|t| term::same_secret(&t.token, &token));
                if let Some(c) = self.clients.get_mut(&id) {
                    // A wrong secret still marks it as from a pane (it gets a pane's default
                    // grants), never as you.
                    c.from = Some(if ok { term } else { TermId::MAX });
                }
            }
            Ev::Disconnected(id) => {
                self.conversation_history.detach(id);
                self.clients.remove(&id);
                self.withdraw_secret_requests(id);
                // The agent that asked went away (cancelled, killed): its question goes too.
                let gone: Vec<(TermId, Status)> = self
                    .questions
                    .iter()
                    .filter(|(_, c, _)| *c == id)
                    .map(|(q, _, s)| (q.term, *s))
                    .collect();
                if !gone.is_empty() {
                    self.questions.retain(|(_, c, _)| *c != id);
                    for (term, before) in gone {
                        self.set_status(term, before, "its question was withdrawn");
                    }
                    self.dirty = true;
                }
            }
            Ev::Conversation(event) => self.conversation_event(event),
            Ev::ConversationValidated { token, result } => {
                self.conversation_validated(token, result)
            }
            Ev::SessionSaved {
                ticket,
                text,
                result,
                barrier,
            } => {
                if matches!(result, Ok(persist::SessionCommit::Committed))
                    && ticket >= self.session_saved_ticket
                {
                    self.session_saved_ticket = ticket;
                    self.last_saved = text;
                }
                self.conversation_save_completed(barrier, result);
            }
            Ev::ConversationStorage {
                id,
                revision,
                result,
            } => {
                if let Some(revision) = revision {
                    if self.conversation_cache_writes.get(&id) == Some(&revision) {
                        self.conversation_cache_writes.remove(&id);
                    }
                    if result.is_ok()
                        && self
                            .conversation_histories
                            .get(&id)
                            .is_some_and(|h| h.revision == revision)
                    {
                        self.conversation_cache_dirty.remove(&id);
                    }
                }
                if let Err(error) = result {
                    self.conversation_states.entry(id).or_default().error = Some(error.clone());
                    self.broadcast(|client| client.attach, ServerMsg::Error(error));
                }
                if revision.is_some_and(|revision| {
                    self.conversation_histories
                        .get(&id)
                        .is_some_and(|h| h.revision > revision)
                }) {
                    self.cache_conversation(id);
                }
            }
            Ev::ConversationChecksResolved {
                id,
                generation,
                run,
                resolved,
                tree,
            } => self.conversation_checks_resolved(id, generation, run, resolved, tree),
            Ev::ConversationCheckRan {
                id,
                generation,
                run,
                name,
                result,
            } => self.conversation_check_ran(id, generation, run, &name, result),
            Ev::ConversationChecksStale(found) => self.conversation_checks_went_stale(found),
            Ev::Msg(id, msg) => self.message(id, msg),
            Ev::Output(tid, data) => {
                let Some(t) = self.terms.get_mut(&tid) else {
                    return;
                };
                if t.output(&data) {
                    t.refresh_head();
                    self.dirty = true;
                }
                if std::mem::take(&mut t.parser.callbacks_mut().title_changed) {
                    self.dirty = true;
                }
                if let Some(text) = t.parser.callbacks_mut().copied.take() {
                    self.broadcast(|c| c.attach, ServerMsg::Clipboard { term: tid, text });
                }
                // A bell from a pane you aren't looking at marks it until you do.
                if self
                    .terms
                    .get_mut(&tid)
                    .is_some_and(|t| std::mem::take(&mut t.parser.callbacks_mut().rang))
                {
                    let seen = self.focused_term() == Some(tid) && self.has_viewer();
                    if let Some(t) = self.terms.get_mut(&tid)
                        && !seen
                        && !t.bell
                    {
                        t.bell = true;
                        self.dirty = true;
                    }
                }
                self.broadcast(
                    |c| c.attach && self.shown_to(tid, c.surface),
                    ServerMsg::Output { term: tid, data },
                );
            }
            Ev::Exited(tid) => {
                // A legacy sleeping agent's process was stopped on purpose; its pane stays.
                if self.terms.get(&tid).is_some_and(|t| t.asleep) {
                    return;
                }
                if self.terms.contains_key(&tid) {
                    self.last_loss = Some(Instant::now());
                    self.remove_term(tid);
                }
            }
            Ev::ProcessExited(tid) => {
                if self.terms.contains_key(&tid) {
                    self.last_loss = Some(Instant::now());
                    self.remove_term(tid);
                }
            }
            Ev::Scan(found) => {
                let mut started = Vec::new();
                for f in found {
                    let Some(t) = self.terms.get_mut(&f.term) else {
                        continue;
                    };
                    // Redraw for memory only when it moved by a few MB.
                    let observed_at = term::unix_now();
                    if t.last_process != observed_at {
                        t.last_process = observed_at;
                        self.dirty = true;
                    }
                    if t.mem.abs_diff(f.mem) > 4 << 20 {
                        t.mem = f.mem;
                        self.dirty = true;
                    }
                    // And for CPU only when it moved by a few points: it never sits still.
                    if (t.cpu - f.cpu).abs() >= CPU_STEP {
                        t.cpu = f.cpu;
                        self.dirty = true;
                    }
                    if t.process != f.process && !f.process.is_empty() {
                        t.process = f.process;
                        self.dirty = true;
                    }
                    if t.remote != f.remote {
                        t.remote = f.remote;
                        self.dirty = true;
                    }
                    if !t.cwd_reported
                        && let Some(c) = f.cwd.filter(|c| c.is_dir() && *c != t.cwd)
                    {
                        t.cwd = c;
                        t.refresh_head();
                        self.dirty = true;
                    }
                    // Hooks own live sessions, not processes that have exited. Allow the
                    // process scanner to catch up to a newly loaded native integration.
                    let mut hook_expired = false;
                    if t.hooked && t.agent.is_some() {
                        if t.agent == f.agent {
                            t.hooked_process_seen = true;
                            continue;
                        }
                        let custom = !self
                            .agents
                            .iter()
                            .any(|a| Some(&a.name) == t.agent.as_ref());
                        if custom
                            || (!t.hooked_process_seen
                                && t.last_hook_at
                                    .is_some_and(|at| at.elapsed() < INTEGRATION_STARTUP_GRACE))
                        {
                            continue;
                        }
                        t.hooked = false;
                        t.hooked_process_seen = false;
                        t.subagents.clear();
                        t.done_held = None;
                        t.alert_due = None;
                        t.progress_off = None;
                        t.screen_blocked_from = None;
                        hook_expired = true;
                    }
                    // A sleeping agent has no process to find. It's still that agent: without
                    // this it turned into a terminal, and woke as a bare shell.
                    if t.asleep {
                        continue;
                    }
                    // A conversation pane's agent and exact session are assigned by the
                    // daemon when it starts the provider. Process-name detection is only a
                    // best-effort status signal; for example Codex can appear as `node`.
                    if t.conversation_id.is_some() && t.agent != f.agent {
                        continue;
                    }
                    // Started as this agent (woken, moved, restored) and its process isn't up
                    // yet: it is still that agent, not a shell that lost its name and its
                    // conversation.
                    if !hook_expired
                        && f.agent.is_none()
                        && t.agent.is_some()
                        && t.born.elapsed() < AGENT_SHOWS_WITHIN
                    {
                        continue;
                    }
                    if t.agent != f.agent {
                        if t.agent.is_none() && f.agent.is_some() {
                            // Where the agent really runs, even if the shell never said.
                            if let Some(c) = f.agent_cwd.clone().filter(|c| c.is_dir()) {
                                t.cwd = c;
                                t.refresh_head();
                                // The shell's process folder is stale (PowerShell never moves
                                // it); don't let the next scan put it back.
                                t.cwd_reported = true;
                            }
                            started.push(f.term);
                        }
                        t.status = if f.agent.is_some() {
                            Status::Idle
                        } else {
                            Status::None
                        };
                        t.status_since = term::unix_now();
                        t.last_turn_end = Instant::now();
                        t.status_why = if f.agent.is_some() {
                            "process: an agent started".into()
                        } else {
                            String::new()
                        };
                        if f.agent.is_none() {
                            t.session = None;
                            t.summary.clear();
                            self.last_loss = Some(Instant::now());
                        }
                        t.agent = f.agent;
                        t.hooked = false;
                        self.dirty = true;
                    }
                }
                if self.cfg.auto_workspace {
                    for term in started {
                        self.auto_workspace(term);
                    }
                }
            }
            Ev::Git(results) => {
                self.git_busy = false;
                for (id, info) in results {
                    if let Some(w) = self.workspaces.iter_mut().find(|w| w.id == id)
                        && w.git != info
                    {
                        w.git = info;
                        self.dirty = true;
                    }
                }
            }
            Ev::Prs(reads) => self.prs_read(reads),
            Ev::WorktreeCreated {
                client,
                branch,
                cmd,
                split,
                by,
                set_up,
                result,
            } => {
                // The repo's own setup comes first (an install, a .env), so what starts in the
                // new worktree starts with it ready, not halfway through. A spare was set up
                // when it was made.
                if !set_up
                    && let Ok((path, _)) = &result
                    && !self.adopted.as_ref().is_some_and(|a| same_path(a, path))
                    && let Some(job) = self.hook_job(path, true)
                {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    self.send(
                        client,
                        ServerMsg::Notice(format!(
                            "setting up {name} before anything starts in it"
                        )),
                    );
                    let tx = self.tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = tx.blocking_send(Ev::HookRan(job()));
                        let _ = tx.blocking_send(Ev::WorktreeCreated {
                            client,
                            branch,
                            cmd,
                            split,
                            by,
                            set_up: true,
                            result,
                        });
                    });
                    return;
                }
                self.pending_ops -= 1;
                self.next_starter = by;
                let as_pane = split.filter(|t| self.terms.contains_key(t));
                let opened = result.map_err(anyhow::Error::msg).and_then(|(path, repo)| {
                    match as_pane {
                        // Beside the pane it was asked from, in the same workspace.
                        Some(term) => {
                            let dir = if self.terms.get(&term).is_some_and(|t| t.cols >= 100) {
                                crate::layout::Dir::Right
                            } else {
                                crate::layout::Dir::Down
                            };
                            self.command(
                                client,
                                Command::Split {
                                    term,
                                    dir,
                                    cmd,
                                    cwd: Some(path.clone()),
                                },
                            )?;
                        }
                        None => {
                            let _ = &repo;
                            self.command(
                                client,
                                Command::NewWorkspace {
                                    cwd: Some(path.clone()),
                                    name: None,
                                    cmd,
                                },
                            )?;
                        }
                    }
                    Ok(path)
                });
                self.next_starter = None;
                if let Ok(p) = &opened {
                    self.made_worktrees.push(p.clone());
                    if self.adopted.as_ref().is_some_and(|a| same_path(a, p)) {
                        self.adopted = None;
                    }
                }
                match opened {
                    Ok(path) => {
                        if as_pane.is_none()
                            && let Some(w) = self.workspaces.last_mut()
                        {
                            w.worktree = true;
                        }
                        self.poll_git_soon();
                        self.send(
                            client,
                            ServerMsg::Notice(format!("worktree {branch} at {}", path.display())),
                        );
                        self.send(client, ServerMsg::Reply(Reply::Ok));
                    }
                    Err(e) => self.send(client, ServerMsg::Error(format!("{e:#}"))),
                }
                self.dirty = true;
            }
            Ev::AgentWorktreeMade { client, result } => {
                self.pending_ops -= 1;
                match result {
                    Ok(path) => {
                        if let Some(top) = crate::gitfs::head(&path).map(|h| h.top) {
                            self.worktree_hook(&top, true);
                            self.made_worktrees.push(top);
                        }
                        self.poll_git_soon();
                        self.send(
                            client,
                            ServerMsg::Reply(Reply::Text(path.to_string_lossy().into_owned())),
                        );
                    }
                    Err(e) => self.send(client, ServerMsg::Error(e)),
                }
                self.dirty = true;
            }
            Ev::HookRan(msg) => self.broadcast(|_| true, ServerMsg::Notice(msg)),
            Ev::HookChecked {
                msg,
                verdict,
                chain,
            } => self.apply_hook(msg, verdict, chain),
            Ev::ChecksResolved {
                term,
                run,
                resolved,
                tree,
            } => self.checks_resolved(term, run, resolved, tree),
            Ev::CheckRan {
                term,
                run,
                name,
                result,
            } => self.check_ran(term, run, &name, result),
            Ev::ChecksStale(found) => self.checks_went_stale(found),
            Ev::RepoChecksFound(found) => self.repo_checks_found(found),
            Ev::CheckOutputsUnsaved(terms) => self.check_outputs_unsaved(&terms),
            Ev::SpareMade(repo, result) => {
                self.spare_making = false;
                match result {
                    Ok((path, _)) => {
                        save_spare(Some(&path));
                        self.worktree_hook(&path, true);
                        let cmd = self.cfg.worktree.prewarm.clone();
                        let (cols, rows) = self.guess_size();
                        match self.spawn(Some(&cmd), &path, cols, rows) {
                            Ok(term) => {
                                if let Some(t) = self.terms.get_mut(&term) {
                                    t.spare = true;
                                }
                                self.spare = Some((repo, path, term));
                            }
                            Err(e) => {
                                tracing::warn!("prewarm: {e:#}");
                                tokio::task::spawn_blocking(move || drop_spare_dir(&path));
                            }
                        }
                    }
                    Err(e) => tracing::warn!("prewarm: {e}"),
                }
            }
            Ev::WorktreeList { client, result } => match result {
                Ok(list) => self.send(client, ServerMsg::Reply(Reply::Worktrees(list))),
                Err(e) => self.send(client, ServerMsg::Error(e)),
            },
            Ev::RepoReady {
                client,
                tag,
                result,
            } => match result {
                Ok(reply) => self.send(client, ServerMsg::Reply(Reply::Repo { tag, reply })),
                Err(e) => self.send(client, ServerMsg::Error(e)),
            },
            Ev::ConversationDiffReady {
                client,
                id,
                generation,
                result,
            } => {
                if !self.clients.contains_key(&client) {
                    return;
                }
                if !self.conversations.iter().any(|conversation| {
                    conversation.id == id && conversation.generation == generation
                }) {
                    self.send(
                        client,
                        ServerMsg::Error("Conversation changed while reading its diff".into()),
                    );
                    return;
                }
                match result {
                    Ok(diff) => self.send(client, ServerMsg::Reply(Reply::ConversationDiff(diff))),
                    Err(error) => self.send(client, ServerMsg::Error(error)),
                }
            }
            Ev::ConversationFilesReady {
                client,
                id,
                generation,
                result,
            } => {
                if !self.clients.contains_key(&client) {
                    return;
                }
                if !self.conversations.iter().any(|conversation| {
                    conversation.id == id && conversation.generation == generation
                }) {
                    self.send(
                        client,
                        ServerMsg::Error("Conversation changed while reading project files".into()),
                    );
                    return;
                }
                match result {
                    Ok(ConversationFileResponse::Entries(entries)) => {
                        self.send(client, ServerMsg::Reply(Reply::ConversationFiles(entries)))
                    }
                    Ok(ConversationFileResponse::Content(content)) => {
                        self.send(client, ServerMsg::Reply(Reply::ConversationFile(content)))
                    }
                    Err(error) => self.send(client, ServerMsg::Error(error)),
                }
            }
            Ev::MoveReady {
                client,
                term,
                branch,
                result,
            } => {
                self.pending_ops -= 1;
                match result {
                    Ok((path, _)) => {
                        self.made_worktrees.push(path.clone());
                        self.worktree_hook(&path, true);
                        if let Some(t) = self.terms.get_mut(&term) {
                            t.pending_move = Some(path.clone());
                        }
                        self.poll_git_soon();
                        self.send(
                            client,
                            ServerMsg::Notice(format!(
                                "Worktree `{branch}` is ready at {}. Finish this turn; {APP_COMMAND} then restarts you there, in this same conversation, and every later edit happens in that folder.",
                                path.display()
                            )),
                        );
                        self.send(client, ServerMsg::Reply(Reply::Ok));
                    }
                    Err(e) => self.send(client, ServerMsg::Error(e)),
                }
                self.dirty = true;
            }
            Ev::WorktreeAutoRemoved {
                client,
                path,
                result,
            } => {
                self.pending_ops -= 1;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let msg = match &result {
                    Removal::Removed { with_branch: true } => {
                        format!("removed worktree {name} and its merged branch")
                    }
                    Removal::Removed { with_branch: false } => {
                        format!("removed worktree {name} (its branch is kept)")
                    }
                    Removal::Kept(why) | Removal::Unmerged(why) => {
                        format!("kept worktree {name}: {why}")
                    }
                };
                // The daemon log is the only record of why a folder went, or stayed.
                tracing::info!("{msg} ({}) after its last pane was closed", path.display());
                match result {
                    Removal::Unmerged(why) => {
                        self.send(client, ServerMsg::AskRemoveWorktree { path, name, why })
                    }
                    Removal::Removed { .. } | Removal::Kept(_) => {
                        self.send(client, ServerMsg::Notice(msg))
                    }
                }
                self.dirty = true;
            }
            Ev::WorktreesVanished { gone } => {
                self.made_worktrees
                    .retain(|m| !gone.iter().any(|g| same_path(g, m)));
            }
            Ev::WorktreesTidied {
                client,
                removed,
                kept,
            } => {
                self.pending_ops -= 1;
                self.send(client, ServerMsg::Notice(tidied_note(&removed, &kept)));
                self.send(client, ServerMsg::Reply(Reply::Ok));
                self.poll_git_soon();
                self.dirty = true;
            }
            Ev::CodexUsage { facts, limits } => self.codex_usage(facts, limits),
            Ev::WorktreeRemoved {
                client,
                path,
                result,
            } => {
                self.pending_ops -= 1;
                match result {
                    Ok(()) => {
                        self.send(
                            client,
                            ServerMsg::Notice(format!("removed worktree {}", path.display())),
                        );
                        self.send(client, ServerMsg::Reply(Reply::Ok));
                    }
                    Err(e) => self.send(client, ServerMsg::Error(e)),
                }
            }
        }
    }

    fn message(&mut self, client: ClientId, msg: ClientMsg) {
        if self
            .clients
            .get(&client)
            .is_some_and(|client| client.rejected)
        {
            self.send(
                client,
                ServerMsg::Error("Structured runtime identity was rejected".into()),
            );
            return;
        }
        if let Some(owner) = self
            .clients
            .get(&client)
            .and_then(|client| client.structured)
            && !self.structured_current(owner)
        {
            self.send(
                client,
                ServerMsg::Error("Structured runtime identity is stale or revoked".into()),
            );
            return;
        }
        if self
            .clients
            .get(&client)
            .is_some_and(|client| client.structured.is_some())
        {
            let allowed = match &msg {
                ClientMsg::Query(_) => self.may(client, Grant::Read, None),
                ClientMsg::Resize { .. } => self.may(client, Grant::Write, None),
                ClientMsg::Hook { .. } | ClientMsg::Usage { .. } => Err(anyhow::anyhow!(
                    "Structured callers cannot report pane status or usage"
                )),
                _ => Ok(()),
            };
            if let Err(error) = allowed {
                self.send(client, ServerMsg::Error(error.to_string()));
                return;
            }
        }
        match msg {
            ClientMsg::Hello { .. } => {}
            ClientMsg::Input { term, data } => {
                // Typing into another pane needs write; into one that's asking, respond.
                let asking = self
                    .terms
                    .get(&term)
                    .is_some_and(|t| t.status == Status::Blocked);
                if let Err(e) = self
                    .may_use_terminal(client, term)
                    .and_then(|_| self.may(client, Grant::Write, Some(term)))
                    .and_then(|_| {
                        if asking {
                            self.may(client, Grant::Respond, Some(term))
                        } else {
                            Ok(())
                        }
                    })
                {
                    self.send(client, ServerMsg::Error(format!("{e:#}")));
                    return;
                }
                if self.terms.get(&term).is_some_and(|t| t.asleep) {
                    // Wake it, and hand over what was typed once it's ready.
                    if let Some(new) = self.wake(term)
                        && let Some(t) = self.terms.get_mut(&new)
                    {
                        t.pending_input = Some((data, Instant::now() + Duration::from_secs(8)));
                    }
                } else if let Some(t) = self.terms.get_mut(&term) {
                    t.input(&data);
                }
            }
            ClientMsg::Resize { term, cols, rows } => {
                if let Err(e) = self.may_use_terminal(client, term) {
                    self.send(client, ServerMsg::Error(format!("{e:#}")));
                    return;
                }
                if let Some(t) = self.terms.get_mut(&term)
                    && (t.cols, t.rows) != (cols, rows)
                {
                    t.resize(cols, rows);
                    self.dirty = true;
                }
            }
            ClientMsg::Hook {
                term,
                ref token,
                pid,
                ..
            } => {
                // Only the pane's own processes may report its status. They carry its secret;
                // when the chain can be traced it must also lead back to the pane, so a desktop
                // app that inherited the pane's environment still can't. The chain is walked on
                // the hook thread (it's slow on Windows), in arrival order.
                let Some(pane) = self.terms.get(&term) else {
                    return;
                };
                if !term::same_secret(&pane.token, token) {
                    tracing::info!("ignoring a status report for pane {term} without its secret");
                    return;
                }
                let job = HookJob {
                    root: pane.pid.filter(|_| pid != 0),
                    pid,
                    known: pane.trusted.clone(),
                    msg,
                };
                if self.hook_jobs.send(job).is_err() {
                    tracing::error!("the hook thread is gone; status reports are lost");
                }
            }
            ClientMsg::Usage {
                term,
                token,
                usage,
                limits,
            } => {
                if self
                    .terms
                    .get(&term)
                    .is_some_and(|t| term::same_secret(&t.token, &token))
                {
                    self.report_usage(term, usage, limits);
                }
            }
            ClientMsg::Command(cmd) => {
                if let Err(e) = self.may_command(client, &cmd) {
                    self.send(client, ServerMsg::Error(format!("{e:#}")));
                    return;
                }
                match self.command(client, cmd) {
                    Ok(true) => self.send(client, ServerMsg::Reply(Reply::Ok)),
                    Ok(false) => {} // answered when the background work finishes
                    Err(e) => self.send(client, ServerMsg::Error(format!("{e:#}"))),
                }
                self.dirty = true;
            }
            ClientMsg::Query(q) => {
                let reply = match q {
                    Query::List
                        if self
                            .clients
                            .get(&client)
                            .is_some_and(|client| client.structured.is_some())
                            && self.may(client, Grant::Read, None).is_err() =>
                    {
                        ServerMsg::Error("Structured runtime lacks the read grant".into())
                    }
                    Query::ConversationRead { id, cursor, limit } => {
                        if let Err(error) = self.conversation_read(client, id, cursor, limit) {
                            self.send(client, ServerMsg::Error(error.to_string()));
                        }
                        return;
                    }
                    Query::ConversationDiff { id, generation } => {
                        if let Err(error) = self.may(client, Grant::Read, None) {
                            self.send(client, ServerMsg::Error(format!("{error:#}")));
                            return;
                        }
                        let Some(conversation) =
                            self.conversations.iter().find(|item| item.id == id)
                        else {
                            self.send(client, ServerMsg::Error("No such conversation".into()));
                            return;
                        };
                        if conversation.generation != generation {
                            self.send(
                                client,
                                ServerMsg::Error("Conversation diff request is stale".into()),
                            );
                            return;
                        }
                        let dir = conversation
                            .binding
                            .worktree
                            .as_ref()
                            .unwrap_or(&conversation.binding.cwd)
                            .clone();
                        let tx = self.tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result =
                                git::conversation_diff(&dir).map_err(|error| format!("{error:#}"));
                            let _ = tx.blocking_send(Ev::ConversationDiffReady {
                                client,
                                id,
                                generation,
                                result,
                            });
                        });
                        return;
                    }
                    query @ Query::ConversationFiles { .. }
                    | query @ Query::ConversationFileRead { .. } => {
                        let (id, generation, path, list) = match query {
                            Query::ConversationFiles {
                                id,
                                generation,
                                path,
                            } => (id, generation, path, true),
                            Query::ConversationFileRead {
                                id,
                                generation,
                                path,
                            } => (id, generation, Some(path), false),
                            _ => unreachable!(),
                        };
                        if let Err(error) = self.may(client, Grant::Read, None) {
                            self.send(client, ServerMsg::Error(format!("{error:#}")));
                            return;
                        }
                        let Some(conversation) =
                            self.conversations.iter().find(|item| item.id == id)
                        else {
                            self.send(client, ServerMsg::Error("No such conversation".into()));
                            return;
                        };
                        if conversation.generation != generation {
                            self.send(
                                client,
                                ServerMsg::Error("Conversation file request is stale".into()),
                            );
                            return;
                        }
                        let dir = conversation
                            .binding
                            .worktree
                            .as_ref()
                            .unwrap_or(&conversation.binding.cwd)
                            .clone();
                        let tx = self.tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = if list {
                                git::conversation_files(&dir, path.as_deref())
                                    .map(ConversationFileResponse::Entries)
                                    .map_err(|error| format!("{error:#}"))
                            } else {
                                git::conversation_file(&dir, path.as_deref().unwrap_or_default())
                                    .map(ConversationFileResponse::Content)
                                    .map_err(|error| format!("{error:#}"))
                            };
                            let _ = tx.blocking_send(Ev::ConversationFilesReady {
                                client,
                                id,
                                generation,
                                result,
                            });
                        });
                        return;
                    }
                    Query::ConversationTerminalScreen { id, generation } => {
                        if self.clients.get(&client).is_some_and(|client| {
                            client.from.is_some() || client.structured.is_some()
                        }) {
                            self.send(
                                client,
                                ServerMsg::Error(
                                    "Only a human client can view a native conversation terminal"
                                        .into(),
                                ),
                            );
                            return;
                        }
                        if let Err(error) = self.may(client, Grant::Read, None) {
                            self.send(client, ServerMsg::Error(format!("{error:#}")));
                            return;
                        }
                        let screen = self.conversation_terminal_screen(id, generation);
                        match screen {
                            Ok(screen) => ServerMsg::Reply(Reply::Text(screen)),
                            Err(error) => ServerMsg::Error(error.to_string()),
                        }
                    }
                    Query::ConversationTerminalReplay { id, generation } => {
                        if self.clients.get(&client).is_some_and(|client| {
                            client.from.is_some() || client.structured.is_some()
                        }) {
                            self.send(
                                client,
                                ServerMsg::Error(
                                    "Only a human client can view a native conversation terminal"
                                        .into(),
                                ),
                            );
                            return;
                        }
                        if let Err(error) = self.may(client, Grant::Read, None) {
                            self.send(client, ServerMsg::Error(format!("{error:#}")));
                            return;
                        }
                        match self.conversation_terminal_replay(id, generation) {
                            Ok(replay) => ServerMsg::Reply(Reply::TerminalReplay(replay)),
                            Err(error) => ServerMsg::Error(error.to_string()),
                        }
                    }
                    Query::TerminalScreen { .. } | Query::TerminalReplay { .. }
                        if self.clients.get(&client).is_some_and(|client| {
                            client.from.is_some() || client.structured.is_some()
                        }) =>
                    {
                        ServerMsg::Error("Only a human client can view a terminal this way".into())
                    }
                    Query::TerminalScreen { term } => match self
                        .may_use_terminal(client, term)
                        .and_then(|()| self.may(client, Grant::Read, Some(term)))
                        .and_then(|()| self.pane_screen(term))
                    {
                        Ok(screen) => ServerMsg::Reply(Reply::Text(screen)),
                        Err(error) => ServerMsg::Error(format!("{error:#}")),
                    },
                    Query::TerminalReplay { term } => match self
                        .may_use_terminal(client, term)
                        .and_then(|()| self.may(client, Grant::Read, Some(term)))
                        .and_then(|()| self.pane_replay(term))
                    {
                        Ok(replay) => ServerMsg::Reply(Reply::TerminalReplay(replay)),
                        Err(error) => ServerMsg::Error(format!("{error:#}")),
                    },
                    Query::Repo { .. }
                        if self.clients.get(&client).is_some_and(|client| {
                            client.from.is_some() || client.structured.is_some()
                        }) =>
                    {
                        ServerMsg::Error("Only a human client can read a repository this way".into())
                    }
                    Query::Repo { term, tag, request } => {
                        let root = self
                            .may_use_terminal(client, term)
                            .and_then(|()| self.may(client, Grant::Read, Some(term)))
                            .and_then(|()| {
                                self.terms
                                    .get(&term)
                                    .map(|t| t.cwd.clone())
                                    .ok_or_else(|| anyhow::anyhow!("no pane {term}"))
                            });
                        let root = match root {
                            Ok(root) => root,
                            Err(error) => {
                                self.send(client, ServerMsg::Error(format!("{error:#}")));
                                return;
                            }
                        };
                        let tx = self.tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = repo::answer(&root, &request).map_err(|e| format!("{e:#}"));
                            let _ = tx.blocking_send(Ev::RepoReady { client, tag, result });
                        });
                        return;
                    }
                    Query::List => {
                        let surface = self.surface_of(client);
                        ServerMsg::Reply(Reply::List(self.snapshot_for(surface)))
                    }
                    Query::Read { term } if self.may(client, Grant::Read, Some(term)).is_err() => {
                        ServerMsg::Error(format!(
                            "{:#}",
                            self.may(client, Grant::Read, Some(term)).unwrap_err()
                        ))
                    }
                    Query::Read { term } => match self.terms.get(&term) {
                        Some(t) => ServerMsg::Reply(Reply::Text(t.parser.screen().contents())),
                        None => ServerMsg::Error(format!("no pane {term}")),
                    },
                    Query::CheckOutput { term, .. }
                        if self.may(client, Grant::Read, Some(term)).is_err() =>
                    {
                        ServerMsg::Error(format!(
                            "{:#}",
                            self.may(client, Grant::Read, Some(term)).unwrap_err()
                        ))
                    }
                    Query::CheckOutput { term, name } => {
                        match self.check_output(term, name.as_deref()) {
                            Some(output) => ServerMsg::Reply(Reply::Text(output.to_string())),
                            None => ServerMsg::Error(format!("no check output for pane {term}")),
                        }
                    }
                    Query::Worktrees { ws } => {
                        let Some(dir) = self
                            .workspaces
                            .iter()
                            .find(|w| w.id == ws)
                            .map(|w| w.cwd.clone())
                        else {
                            self.send(client, ServerMsg::Error(format!("no workspace {ws}")));
                            return;
                        };
                        let tx = self.tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = git::list_worktrees(&dir).map_err(|e| format!("{e:#}"));
                            let _ = tx.blocking_send(Ev::WorktreeList { client, result });
                        });
                        return;
                    }
                };
                self.send(client, reply);
            }
        }
    }

    fn has_viewer(&self) -> bool {
        self.clients.values().any(|c| c.attach)
    }

    fn focused_term(&self) -> Option<TermId> {
        let ws = self
            .active_ws
            .and_then(|id| self.workspaces.iter().find(|w| w.id == id))?;
        ws.tab().map(|t| t.focus)
    }

    /// Whether an app may see this pane. A pane that is gone is shown to nobody's list it can
    /// act on anyway, so only a pane we know to belong to the other app is hidden.
    fn shown_to(&self, term: TermId, surface: Surface) -> bool {
        self.terms.get(&term).is_none_or(|t| t.owner == surface)
    }

    /// The app a client is. Clients that never said are terminal clients.
    fn surface_of(&self, client: ClientId) -> Surface {
        self.clients
            .get(&client)
            .map(|c| c.surface)
            .unwrap_or_default()
    }

    /// Every pane in a workspace, across its tabs.
    fn workspace_panes(&self, ws: WsId) -> Vec<TermId> {
        self.workspaces
            .iter()
            .filter(|w| w.id == ws)
            .flat_map(|w| w.tabs.iter().flat_map(|t| t.layout.leaves()))
            .collect()
    }

    /// The state one app is shown: its own terminal sessions, the workspaces made of them, and
    /// its own native conversation terminals (ADR 0045).
    fn snapshot_for(&self, surface: Surface) -> Snapshot {
        let mut snap = self.snapshot();
        snap.terms.retain(|id, _| self.shown_to(*id, surface));
        snap.workspaces.retain(|ws| {
            ws.tabs.iter().all(|tab| {
                tab.layout
                    .leaves()
                    .iter()
                    .all(|t| self.shown_to(*t, surface))
            })
        });
        snap.conversations
            .retain(|c| c.native_term.is_none_or(|t| self.shown_to(t, surface)));
        if snap
            .active_ws
            .is_some_and(|id| !snap.workspaces.iter().any(|ws| ws.id == id))
        {
            snap.active_ws = snap.workspaces.first().map(|ws| ws.id);
        }
        snap
    }

    fn snapshot(&self) -> Snapshot {
        let terms: BTreeMap<TermId, TermInfo> = self
            .terms
            .values()
            .filter(|t| !t.spare)
            .map(|t| {
                (
                    t.id,
                    TermInfo {
                        conversation_id: t.conversation_id,
                        id: t.id,
                        cols: t.cols,
                        rows: t.rows,
                        title: t.title().to_string(),
                        process: t.process.clone(),
                        remote: t.remote.clone(),
                        popup: t.popup,
                        agent: t.agent.clone(),
                        // Asking for helpers needs you, whatever the agent's own hooks say meanwhile.
                        status: if t.helpers.asked.is_empty() {
                            t.status
                        } else {
                            Status::Blocked
                        },
                        cwd: t.cwd.clone(),
                        summary: t.summary.clone(),
                        name: if t.name.is_empty() {
                            t.first_prompt.clone()
                        } else {
                            t.name.clone()
                        },
                        model: t.model.clone(),
                        label: t.label.clone(),
                        mem: t.mem,
                        cpu: t.cpu,
                        bell: t.bell,
                        said: t.said.clone(),
                        branch: t.head.as_ref().map(|h| h.branch.clone()),
                        linked: t.head.as_ref().is_some_and(|h| h.linked),
                        root: t.head.as_ref().map(|h| h.main_root.clone()),
                        top: t.head.as_ref().map(|h| h.top.clone()),
                        since: t.status_since,
                        status_why: t.status_why.clone(),
                        why_hook: t
                            .last_hook
                            .as_ref()
                            .map(|h| h.0.clone())
                            .unwrap_or_default(),
                        why_hook_at: t.last_hook.as_ref().map(|h| h.1).unwrap_or(0),
                        why_screen: t
                            .last_screen
                            .as_ref()
                            .map(|h| h.0.clone())
                            .unwrap_or_default(),
                        why_screen_at: t.last_screen.as_ref().map(|h| h.1).unwrap_or(0),
                        why_process_at: t.last_process,
                        quiet_since: t.quiet_since,
                        pid: t.pid.unwrap_or(0),
                        asleep: t.asleep,
                        win32_input: t.win32_input,
                        usage: t.usage.clone(),
                        resume_at: t.resume_at,
                        checks: t.checks.infos(),
                        subagents: t.subagents.iter().map(|(_, k)| k.clone()).collect(),
                        started_by: t.started_by,
                        tab_of: t.tab_of.filter(|o| self.terms.contains_key(o)),
                        helper_ask: t.helpers.asked.clone(),
                        pr: self.pr_of(t),
                        queued: t.queue.iter().cloned().collect(),
                        subagent_activity: t.subagent_activity.clone(),
                        session: t.session.clone(),
                    },
                )
            })
            .collect();
        Snapshot {
            conversations: self.conversation_summaries(),
            daemon_version: env!("APP_VERSION").into(),
            workspaces: self.workspaces.clone(),
            active_ws: self.active_ws,
            terms,
            questions: self.questions.iter().map(|(q, ..)| q.clone()).collect(),
            secret_requests: self.secrets.requests(),
            limits: self
                .limits
                .iter()
                .map(|(a, l)| (a.clone(), l.clone()))
                .collect(),
            spent_today: self.spent_today(),
            today: usage::today(&self.turns, &self.spent, term::unix_now()),
        }
    }

    fn sync_scan_roots(&self) {
        let roots = self
            .terms
            .values()
            .filter_map(|t| t.pid.map(|p| (t.id, p)))
            .collect();
        self.scan.lock().unwrap().roots = roots;
    }

    fn next(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Claude, started by the app, learns how to move itself into a worktree (and may run
    /// just that command without asking).
    fn teach(&self, cmd: &str) -> String {
        let mut words = cmd.split_whitespace();
        let first = words.next().unwrap_or("");
        let is_claude = std::path::Path::new(first)
            .file_stem()
            .is_some_and(|s| s.eq_ignore_ascii_case("claude"));
        if !self.cfg.teach_agents || !is_claude || cmd.contains("--append-system-prompt") {
            return cmd.to_string();
        }
        let note = concat!(
            "You are running inside ",
            env!("APP_NAME"),
            ", a terminal where one person runs many coding agents. If the user asks you to do the work in a worktree (or on a separate branch so you don't touch their checkout), run `",
            env!("APP_COMMAND"),
            " worktree --move <short-branch-name>` with the Bash tool before editing anything, then end your turn: ",
            env!("APP_NAME"),
            " creates the worktree and restarts you there in this same conversation."
        );
        let rest = cmd[first.len()..].trim_start();
        format!(
            "{first} --append-system-prompt {} --allowedTools {} {rest}",
            self.cfg.quote_for_shell(note),
            self.cfg
                .quote_for_shell(concat!("Bash(", env!("APP_COMMAND"), " worktree:*)"))
        )
        .trim_end()
        .to_string()
    }

    /// The pane a start comes from, if it comes from one. Only a connection that proved it
    /// is that pane (its secret) counts: a session can't be hung on a pane by naming it.
    fn starter(&mut self, client: ClientId) -> Option<TermId> {
        self.next_starter
            .take()
            .or_else(|| self.clients.get(&client).and_then(|c| c.from))
            .filter(|t| self.terms.contains_key(t))
    }

    /// The pane a start comes from, once it may start `task` and there's room for it. A
    /// session another agent starts is a helper (see `helpers.rs` for who may), and each helper
    /// runs builds and tests of its own: an agent that starts ten takes the whole machine. Past
    /// `sessions.helpers_at_once` of its own working at once the start is refused, with what to
    /// do instead.
    fn starting(&mut self, client: ClientId, task: &str) -> Result<Option<TermId>> {
        if self
            .clients
            .get(&client)
            .is_some_and(|client| client.structured.is_some())
        {
            anyhow::bail!(
                "Structured runtimes cannot start descendants until conversation-owned helper consent is supported"
            );
        }
        // A worktree asked for earlier was let through then: don't strand it now.
        let let_through = self.next_starter.is_some();
        let by = self.starter(client);
        if let Some(by) = by
            && !let_through
        {
            self.may_start_helper(by, task)?;
            let most = self.cfg.helpers_at_once();
            let busy = self.busy_helpers(by, None);
            if busy >= most {
                anyhow::bail!(
                    "{busy} helpers are already working, and {most} at once is the limit (Settings, Sessions). Wait for one to finish, then start this one."
                );
            }
        }
        Ok(by)
    }

    fn busy_helpers(
        &self,
        by: TermId,
        except: Option<crate::conversation::ConversationId>,
    ) -> usize {
        let native = self
            .terms
            .values()
            .filter(|t| {
                t.started_by == Some(by)
                    && (t.status == Status::Working || t.born.elapsed() < HELPER_SETTLES)
            })
            .count();
        native
            + self
                .conversation_states
                .iter()
                .filter(|(id, state)| {
                    Some(**id) != except
                        && state.started_by == Some(by)
                        && (self.conversation_runtimes.owns(**id)
                            || self
                                .conversation_requests
                                .values()
                                .any(|p| p.conversation() == **id))
                        && (state.status == Status::Working
                            || state
                                .started_at
                                .is_some_and(|at| at.elapsed() < HELPER_SETTLES))
                })
                .count()
    }

    /// Note which pane's agent started `term`.
    fn started(&mut self, term: TermId, by: Option<TermId>) {
        if let Some(t) = self.terms.get_mut(&term) {
            t.started_by = by.filter(|b| *b != term);
        }
    }

    fn spawn(
        &mut self,
        cmd: Option<&str>,
        cwd: &std::path::Path,
        cols: u16,
        rows: u16,
    ) -> Result<TermId> {
        // An account's command is taught as the command inside it, and keeps its account.
        let (account, inner) = cmd.map(|c| self.cfg.split_account(c)).unwrap_or((None, ""));
        let taught = cmd.map(|_| {
            let taught = self.teach(inner);
            account
                .as_deref()
                .map(|a| self.cfg.as_account(a, &taught))
                .unwrap_or(taught)
        });
        let cmd = taught.as_deref();
        let id = self.next();
        if let Some((dir, term)) = self.adopt.take() {
            if same_path(&dir, cwd) && self.terms.contains_key(&term) {
                if let Some(t) = self.terms.get_mut(&term) {
                    t.spare = false;
                    t.resize(cols, rows);
                }
                return Ok(term);
            }
            self.adopt = Some((dir, term));
        }
        let once = std::mem::take(&mut self.next_once);
        let mut t = Term::spawn(
            &self.cfg,
            SpawnSpec {
                id,
                cmd,
                direct: None,
                cwd,
                cols,
                rows,
                once,
            },
            self.tx.clone(),
        )?;
        t.account = account;
        self.terms.insert(id, t);
        self.had_terms = true;
        // Its worktree is looked at for a repo checks file with the git poll.
        self.poll_git_soon();
        Ok(id)
    }

    /// Size for a brand-new full-tab pane: whatever the focused pane has, as a guess the
    /// client corrects immediately.
    fn guess_size(&self) -> (u16, u16) {
        self.focused_term()
            .and_then(|t| self.terms.get(&t))
            .map(|t| (t.cols, t.rows))
            .unwrap_or((120, 32))
    }

    fn ws_mut(&mut self, id: WsId) -> Result<&mut WorkspaceInfo> {
        self.workspaces
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| anyhow::anyhow!("no workspace {id}"))
    }

    fn locate(&self, term: TermId) -> Option<(WsId, TabId)> {
        self.workspaces.iter().find_map(|w| {
            w.tabs
                .iter()
                .find(|t| t.layout.contains(term))
                .map(|t| (w.id, t.id))
        })
    }

    fn close_term(&mut self, term: TermId) {
        if let Some(t) = self.terms.get_mut(&term) {
            t.kill_tree();
        }
        self.remove_term(term);
    }

    /// Drop a terminal and prune the tree: empty tabs and workspaces disappear.
    fn remove_term(&mut self, term: TermId) {
        // Closing a terminal can block on Windows until its programs let go; never here.
        if let Some(t) = self.terms.remove(&term) {
            std::thread::spawn(move || drop(t));
        }
        self.detach(term);
        if self.terms.is_empty() {
            self.empty_since = Instant::now();
        }
    }

    /// Take a pane out of whatever tab holds it, pruning emptied tabs and workspaces. The
    /// terminal itself keeps running.
    fn detach(&mut self, term: TermId) {
        self.dirty = true;
        for w in &mut self.workspaces {
            let mut i = 0;
            while i < w.tabs.len() {
                let tab = &mut w.tabs[i];
                if !tab.layout.contains(term) {
                    i += 1;
                    continue;
                }
                let layout = std::mem::replace(&mut tab.layout, Node::Leaf(0));
                match layout.remove(term) {
                    Some(l) => {
                        if tab.focus == term {
                            tab.focus = l.first_leaf();
                        }
                        tab.layout = l;
                        i += 1;
                    }
                    None => {
                        let removed = w.tabs.remove(i).id;
                        if w.active_tab == removed {
                            let next = i.min(w.tabs.len().saturating_sub(1));
                            w.active_tab = w.tabs.get(next).map(|t| t.id).unwrap_or(0);
                        }
                    }
                }
            }
        }
        let before = self
            .workspaces
            .iter()
            .position(|w| Some(w.id) == self.active_ws);
        self.workspaces.retain(|w| !w.tabs.is_empty());
        if self
            .active_ws
            .is_none_or(|id| !self.workspaces.iter().any(|w| w.id == id))
        {
            let i = before
                .unwrap_or(0)
                .min(self.workspaces.len().saturating_sub(1));
            self.active_ws = self.workspaces.get(i).map(|w| w.id);
        }
    }
}

/// Strip Windows' verbatim prefix (`\\?\C:\...`) that `canonicalize` adds; shells choke on it.
pub(crate) fn clean_path(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => p,
    }
}

fn home() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

#[cfg(test)]
mod logic_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn worktrees_of_trusted_repos_are_trusted() {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-trust-{}"),
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("claude.json");
        std::fs::write(
            &f,
            r#"{"projects":{"c:/code/app":{"hasTrustDialogAccepted":true}},"other":1}"#,
        )
        .unwrap();
        super::trust_in(
            &f,
            std::path::Path::new("C:/code/app"),
            std::path::Path::new("C:/code/app-worktrees/x"),
        );
        super::trust_in(
            &f,
            std::path::Path::new("C:/code/nope"),
            std::path::Path::new("C:/code/nope-wt"),
        );
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(
            v["projects"]["C:/code/app-worktrees/x"]["hasTrustDialogAccepted"],
            true
        );
        assert!(
            v["projects"].get("C:/code/nope-wt").is_none(),
            "never trusts what you didn't"
        );
        assert_eq!(v["other"], 1, "the rest of the file is kept");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
