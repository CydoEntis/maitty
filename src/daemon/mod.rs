//! The daemon: owns every PTY and the workspace tree, survives clients detaching.
//!
//! All state lives in one event loop (`Daemon::run`). PTY reader threads, the process
//! scanner, background git work and client connections talk to it through a single
//! channel, so there is no locking around the model.

mod check;
mod git;
mod persist;
mod scan;
mod term;
mod commands;
mod contain;
mod restore;
mod status;
mod usage;
mod worktrees;

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
/// How often worktrees left behind (a session that crashed) are looked for, and how soon
/// after the server starts the first look is.
const WORKTREE_SWEEP_EVERY: Duration = Duration::from_secs(10 * 60);
const WORKTREE_SWEEP_FIRST: Duration = Duration::from_secs(60);
/// Stands in for a client when a notice is for every window (nobody asked).
const EVERYONE: ClientId = ClientId::MAX;
/// A daemon nothing ever used (no panes, no clients) exits after this.
const EXIT_WHEN_UNUSED_FOR: Duration = Duration::from_secs(30);
/// Two panes ending on their own within this window means something killed them all
/// (a reboot, a logoff): their sessions are kept for the next start.
const MASS_EXIT_WINDOW: Duration = Duration::from_secs(5);

#[allow(clippy::large_enum_variant)]
pub enum Ev {
    Connected(ClientId, mpsc::Sender<ServerMsg>, bool),
    /// The connection's request comes from inside this pane (with this secret).
    From(ClientId, TermId, String),
    Msg(ClientId, ClientMsg),
    Disconnected(ClientId),
    Output(TermId, Vec<u8>),
    Exited(TermId),
    Scan(Vec<scan::Found>),
    Git(Vec<(WsId, Option<GitInfo>)>),
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
    WorktreeRemoved { client: ClientId, path: PathBuf, result: Result<(), String> },
    /// Numbers read from Codex's session files (`usage::poll_codex`).
    /// What Codex's session files say: usage, limits, and what runs under each pane's session.
    CodexUsage { facts: Vec<usage::CodexPaneFacts>, limits: Option<Vec<Limit>> },
    /// A worktree made for an agent started from a shell (`Command::AgentWorktree`).
    AgentWorktreeMade { client: ClientId, result: Result<PathBuf, String> },
    /// A worktree removed (or kept) after its last pane closed.
    WorktreeAutoRemoved { client: ClientId, path: PathBuf, result: Result<bool, String> },
    /// The sweep's findings: seshi-made worktrees nothing runs in that are safe to remove,
    /// and ones whose folder is already gone.
    WorktreeSwept { safe: Vec<PathBuf>, gone: Vec<PathBuf> },
    /// A clean-up you asked for is done: what went (and whether its merged branch went
    /// too), and what stayed and why.
    WorktreesTidied { client: ClientId, removed: Vec<(String, bool)>, kept: Vec<(String, String)> },
    WorktreeList { client: ClientId, result: Result<Vec<WorktreeEntry>, String> },
    /// A worktree hook ran: what to tell people.
    HookRan(String),
    /// A spare worktree for prewarming: (repo, result).
    SpareMade(PathBuf, Result<(PathBuf, String), String>),
    /// A worktree made for an agent to move into: (client, pane, branch, result).
    MoveReady { client: ClientId, term: TermId, branch: String, result: Result<(PathBuf, String), String> },
    /// A status report, its sender checked (see `HookJob`).
    HookChecked { msg: ClientMsg, verdict: Option<bool>, chain: Vec<u32> },
    /// A pane's check ended (see `Daemon::turn_ended`).
    CheckRan { term: TermId, run: check::CheckRun },
    /// Check output tails a background save could not write.
    CheckOutputsUnsaved(Vec<TermId>),
}

/// How to put back what an automatic workspace changed.
enum AutoUndo {
    /// The pane moved out of `from_ws`/`from_tab` (beside `beside`).
    Moved { term: TermId, from_ws: WsId, from_tab: TabId, beside: Option<TermId> },
    /// The pane was alone, so its workspace was re-homed; restore name and folder.
    Rehomed { ws: WsId, name: String, cwd: PathBuf },
}

struct Client {
    tx: mpsc::Sender<ServerMsg>,
    attach: bool,
    /// The pane it acts from (its secret checked): what it may do is that pane's grants.
    /// None: you (seshi's window, or a terminal outside seshi).
    from: Option<TermId>,
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
    git_busy: bool,
    last_git: Instant,
    last_save: Instant,
    last_output_save: Instant,
    last_saved: String,
    /// Where check output tails are kept.
    check_dir: PathBuf,
    /// Worktrees seshi created; closing the last thing in one removes it.
    made_worktrees: Vec<PathBuf>,
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
    last_worktree_sweep: Instant,
    /// The last automatic workspace move, for undo.
    auto_undo: Option<AutoUndo>,
    /// Panes whose process ended on its own, recently. Several at once means a crash,
    /// logoff or reboot is tearing things down, which must not overwrite the saved session.
    natural_exits: Vec<Instant>,
}

pub fn run() -> Result<()> {
    crate::proc::ahead_of_the_panes();
    init_logging();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;
    rt.block_on(async {
        // Refuse to start a second daemon on the same socket.
        if ipc::connect().await.is_ok() {
            anyhow::bail!("a seshi daemon is already running on {}", ipc::socket_id());
        }
        let listener = ipc::listen()?;
        tracing::info!("daemon listening on {}", ipc::socket_id());
        // Lets `seshi kill-server` from another version stop it (it can't talk to this one).
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
        let mut d = Daemon::new(cfg, tx);
        if d.cfg.restore.enabled
            && let Some(saved) = persist::load()
        {
            d.restore(saved);
            d.last_saved = serde_json::to_string(&d.saved()).unwrap_or_default();
        }
        d.run(rx).await;
        Ok(())
    })
}

fn init_logging() {
    let dir = crate::config::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("daemon.log")) {
        let _ = tracing_subscriber::fmt()
            .with_writer(Mutex::new(file))
            .with_ansi(false)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_env("SESHI_LOG").unwrap_or_else(|_| "info".into()),
            )
            .try_init();
    }
}

async fn serve(id: ClientId, stream: interprocess::local_socket::tokio::Stream, ev: mpsc::Sender<Ev>) {
    let (mut r, mut w) = ipc::framed(stream);
    let (attach, from) = match ipc::recv_client(&mut r).await {
        Ok(Some(ClientMsg::Hello { attach, from, .. })) => (attach, from),
        _ => return,
    };
    if ipc::send(&mut w, &ServerMsg::Welcome { version: PROTOCOL_VERSION }).await.is_err() {
        return;
    }
    let (tx, mut rx) = mpsc::channel::<ServerMsg>(CLIENT_QUEUE);
    if ev.send(Ev::Connected(id, tx, attach)).await.is_err() {
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
        parts.push(format!("removed {} worktree{}{also}", removed.len(), if removed.len() == 1 { "" } else { "s" }));
    }
    parts.extend(kept.iter().map(|(name, why)| format!("kept {name}: {why}")));
    if parts.is_empty() { "nothing to clean up".into() } else { parts.join("; ") }
}

/// What a pane may do through seshi (see `[mcp] grants`).
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
            Grant::Admin => "close other panes or stop seshi",
        }
    }
}

impl Daemon {
    /// May this connection do `g` (to pane `target`)? You always may; a pane may do anything
    /// to itself, and the rest per its grants.
    pub(super) fn may(&self, client: ClientId, g: Grant, target: Option<TermId>) -> anyhow::Result<()> {
        let Some(from) = self.clients.get(&client).and_then(|c| c.from) else { return Ok(()) };
        if target == Some(from) && g != Grant::Admin {
            return Ok(());
        }
        let grants = self.terms.get(&from).and_then(|t| t.grants.clone()).unwrap_or_else(|| self.cfg.mcp.grants.clone());
        // Letting agents approve prompts (Settings → Agents) is letting them respond.
        let respond_by_setting = g == Grant::Respond && self.cfg.mcp.approve != "never";
        if respond_by_setting || grants.iter().any(|x| x == g.name()) {
            return Ok(());
        }
        anyhow::bail!("this pane may not {} through seshi (seshi grant {from} {} allows it)", g.says(), g.name())
    }

    /// The grant a command needs.
    fn may_command(&self, client: ClientId, cmd: &Command) -> anyhow::Result<()> {
        match cmd {
            Command::NewWorkspace { .. }
            | Command::NewTab { .. }
            | Command::Split { .. }
            | Command::NewWorktree { .. }
            | Command::Popup { .. } => {
                self.may(client, Grant::Start, None)
            }
            Command::ClosePane { term } => self.may(client, Grant::Admin, Some(*term)).or_else(|e| {
                // Closing itself is fine.
                if self.clients.get(&client).and_then(|c| c.from) == Some(*term) { Ok(()) } else { Err(e) }
            }),
            Command::CloseWorkspace { .. }
            | Command::CloseTab { .. }
            | Command::RemoveWorktree { .. }
            | Command::CloseWorktree { .. }
            | Command::TidyWorktrees { .. }
            | Command::KillServer { .. } => {
                self.may(client, Grant::Admin, None)
            }
            // Typing into another pane, only later.
            Command::QueueFollowUp { term, .. } | Command::ClearQueue { term } => self.may(client, Grant::Write, Some(*term)),
            Command::AnswerHuman { id, .. } => {
                let asker = self.questions.iter().find(|(q, ..)| q.id == *id).map(|(q, ..)| q.term);
                self.may(client, Grant::Respond, asker.filter(|_| false))
            }
            // Only you hand out grants.
            Command::Grant { .. } if self.clients.get(&client).is_some_and(|c| c.from.is_some()) => {
                anyhow::bail!("grants are yours to give: run seshi grant outside seshi's panes, or use the pane's menu")
            }
            _ => Ok(()),
        }
    }
}

/// Discard the saved session (for `kill-server --forget` when the server couldn't be asked).
pub fn forget_session() {
    persist::forget();
}

impl Daemon {
    /// A daemon with no panes yet; its process scanner and hook thread are running.
    fn new(cfg: Config, tx: mpsc::Sender<Ev>) -> Daemon {
        contain::children_die_with_us();
        // Clicking a notification opens its seshi:// link (Windows needs to be told how).
        if !cfg!(test) {
            crate::reveal::register();
        }
        // Hooks naming another seshi (an older install) can't reach this server.
        if !cfg!(test) {
            crate::cli::refresh_claude_hooks();
        }
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
            git_busy: false,
            last_git: crate::clock::ago(GIT_POLL_EVERY),
            last_save: Instant::now(),
            last_output_save: Instant::now(),
            last_saved: String::new(),
            check_dir: persist::check_output_dir(),
            made_worktrees: Vec::new(),
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
            last_worktree_sweep: crate::clock::ago(WORKTREE_SWEEP_EVERY - WORKTREE_SWEEP_FIRST),
            natural_exits: Vec::new(),
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
                    if self.last_worktree_sweep.elapsed() >= WORKTREE_SWEEP_EVERY {
                        self.last_worktree_sweep = Instant::now();
                        self.sweep_worktrees();
                    }
                    if self.last_save.elapsed() > SAVE_EVERY {
                        self.last_save = Instant::now();
                        self.persist(false);
                    }
                    if self.last_output_save.elapsed() > OUTPUT_SAVE_EVERY && !self.mass_exit() {
                        self.last_output_save = Instant::now();
                        self.save_outputs(false);
                    }
                }
            }
            if self.dirty {
                self.dirty = false;
                self.sync_scan_roots();
                let snap = self.snapshot();
                self.broadcast(|_| true, ServerMsg::State(snap));
            }
            if self.should_exit() {
                tracing::info!("no panes left; exiting");
                // The user closed everything, so start fresh next time. A mass exit is a
                // crash or shutdown: leave the saved session for restore.
                if !self.mass_exit() {
                    persist::forget();
                }
                self.broadcast(|_| true, ServerMsg::Bye);
                tokio::time::sleep(Duration::from_millis(100)).await;
                let _ = std::fs::remove_file(ipc::pid_file());
                std::process::exit(0);
            }
        }
    }

    /// Look at git again on the next tick (something just changed it).
    fn poll_git_soon(&mut self) {
        self.last_git = crate::clock::ago(GIT_POLL_EVERY);
    }

    fn should_exit(&self) -> bool {
        // A window still showing seshi (empty projects, the splash) keeps it open: closing
        // the last pane mustn't close the app under you.
        if !self.terms.is_empty() || self.pending_ops > 0 || self.clients.values().any(|c| c.attach) {
            return false;
        }
        // Exit once the last pane is closed and no window is left, or if nobody ever started one.
        self.had_terms || (self.clients.is_empty() && self.empty_since.elapsed() > EXIT_WHEN_UNUSED_FOR)
    }

    fn broadcast(&self, filter: impl Fn(&Client) -> bool, msg: ServerMsg) {
        for c in self.clients.values().filter(|c| filter(c)) {
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
        let dirs: Vec<(WsId, PathBuf)> = self.workspaces.iter().map(|w| (w.id, w.cwd.clone())).collect();
        let dir_of: HashMap<WsId, PathBuf> = dirs.iter().cloned().collect();
        let tx = self.tx.clone();
        tokio::task::spawn_blocking(move || {
            let mut res: Vec<(WsId, Option<GitInfo>)> = dirs.into_iter().map(|(id, dir)| (id, git::status(&dir))).collect();
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
                let Some(g) = info.as_mut().filter(|g| g.linked) else { continue };
                if let (Some(base), Some(dir)) = (
                    g.worktrees.iter().find(|e| e.main).map(|e| e.branch.clone()),
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
            Ev::Connected(id, tx, attach) => {
                let c = Client { tx, attach, from: None, behind: std::cell::Cell::new(false) };
                if attach {
                    c.push(ServerMsg::State(self.snapshot()));
                    for t in self.terms.values() {
                        c.push(ServerMsg::Replay { term: t.id, cols: t.cols, rows: t.rows, data: t.replay() });
                    }
                }
                self.clients.insert(id, c);
            }
            Ev::From(id, term, token) => {
                // Only with the pane's secret: anything else can't claim to be it.
                let ok = self.terms.get(&term).is_some_and(|t| term::same_secret(&t.token, &token));
                if let Some(c) = self.clients.get_mut(&id) {
                    // A wrong secret still marks it as from a pane (it gets a pane's default
                    // grants), never as you.
                    c.from = Some(if ok { term } else { TermId::MAX });
                }
            }
            Ev::Disconnected(id) => {
                self.clients.remove(&id);
                // The agent that asked went away (cancelled, killed): its question goes too.
                let gone: Vec<(TermId, Status)> = self.questions.iter().filter(|(_, c, _)| *c == id).map(|(q, _, s)| (q.term, *s)).collect();
                if !gone.is_empty() {
                    self.questions.retain(|(_, c, _)| *c != id);
                    for (term, before) in gone {
                        self.set_status(term, before, "its question was withdrawn");
                    }
                    self.dirty = true;
                }
            }
            Ev::Msg(id, msg) => self.message(id, msg),
            Ev::Output(tid, data) => {
                let Some(t) = self.terms.get_mut(&tid) else { return };
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
                if self.terms.get_mut(&tid).is_some_and(|t| std::mem::take(&mut t.parser.callbacks_mut().rang)) {
                    let seen = self.focused_term() == Some(tid) && self.has_viewer();
                    if let Some(t) = self.terms.get_mut(&tid)
                        && !seen
                        && !t.bell
                    {
                        t.bell = true;
                        self.dirty = true;
                    }
                }
                self.broadcast(|c| c.attach, ServerMsg::Output { term: tid, data });
            }
            Ev::Exited(tid) => {
                // A legacy sleeping agent's process was stopped on purpose; its pane stays.
                if self.terms.get(&tid).is_some_and(|t| t.asleep) {
                    return;
                }
                if self.terms.contains_key(&tid) {
                    self.natural_exits.retain(|t| t.elapsed() < MASS_EXIT_WINDOW);
                    self.natural_exits.push(Instant::now());
                    self.remove_term(tid);
                }
            }
            Ev::Scan(found) => {
                let mut started = Vec::new();
                for f in found {
                    let Some(t) = self.terms.get_mut(&f.term) else { continue };
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
                        let custom = !self.agents.iter().any(|a| Some(&a.name) == t.agent.as_ref());
                        if custom || (!t.hooked_process_seen && t.last_hook_at.is_some_and(|at| at.elapsed() < INTEGRATION_STARTUP_GRACE)) {
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
                    // Started as this agent (woken, moved, restored) and its process isn't up
                    // yet: it is still that agent, not a shell that lost its name and its
                    // conversation.
                    if !hook_expired && f.agent.is_none() && t.agent.is_some() && t.born.elapsed() < AGENT_SHOWS_WITHIN {
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
                        t.status = if f.agent.is_some() { Status::Idle } else { Status::None };
                        t.status_since = term::unix_now();
                        t.last_turn_end = Instant::now();
                        t.status_why = if f.agent.is_some() { "process: an agent started".into() } else { String::new() };
                        if f.agent.is_none() {
                            t.session = None;
                            t.summary.clear();
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
            Ev::WorktreeCreated { client, branch, cmd, split, by, set_up, result } => {
                // The repo's own setup comes first (an install, a .env), so what starts in the
                // new worktree starts with it ready, not halfway through. A spare was set up
                // when it was made.
                if !set_up
                    && let Ok((path, _)) = &result
                    && !self.adopted.as_ref().is_some_and(|a| same_path(a, path))
                    && let Some(job) = self.hook_job(path, true)
                {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    self.send(client, ServerMsg::Notice(format!("setting up {name} before anything starts in it")));
                    let tx = self.tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = tx.blocking_send(Ev::HookRan(job()));
                        let _ = tx.blocking_send(Ev::WorktreeCreated { client, branch, cmd, split, by, set_up: true, result });
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
                            self.command(client, Command::Split { term, dir, cmd, cwd: Some(path.clone()) })?;
                        }
                        None => {
                            let _ = &repo;
                            self.command(client, Command::NewWorkspace { cwd: Some(path.clone()), name: None, cmd })?;
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
                        self.send(client, ServerMsg::Notice(format!("worktree {branch} at {}", path.display())));
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
                        self.send(client, ServerMsg::Reply(Reply::Text(path.to_string_lossy().into_owned())));
                    }
                    Err(e) => self.send(client, ServerMsg::Error(e)),
                }
                self.dirty = true;
            }
            Ev::HookRan(msg) => self.broadcast(|_| true, ServerMsg::Notice(msg)),
            Ev::HookChecked { msg, verdict, chain } => self.apply_hook(msg, verdict, chain),
            Ev::CheckRan { term, run } => self.check_ran(term, run),
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
            Ev::MoveReady { client, term, branch, result } => {
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
                                "Worktree `{branch}` is ready at {}. Finish this turn; seshi then restarts you there, in this same conversation, and every later edit happens in that folder.",
                                path.display()
                            )),
                        );
                        self.send(client, ServerMsg::Reply(Reply::Ok));
                    }
                    Err(e) => self.send(client, ServerMsg::Error(e)),
                }
                self.dirty = true;
            }
            Ev::WorktreeAutoRemoved { client, path, result } => {
                self.pending_ops -= 1;
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let msg = match result {
                    Ok(true) => format!("removed worktree {name} and its merged branch"),
                    Ok(false) => format!("removed worktree {name} (its branch is kept)"),
                    Err(_) => format!("kept worktree {name}: it has uncommitted changes"),
                };
                // The sweep answers to nobody: say what went, to every window, and nothing
                // about what stayed.
                if client != EVERYONE {
                    self.send(client, ServerMsg::Notice(msg));
                } else if result.is_ok() {
                    self.broadcast(|c| c.attach, ServerMsg::Notice(format!("{msg}: nothing was running in it")));
                }
                self.dirty = true;
            }
            Ev::WorktreeSwept { safe, gone } => {
                self.made_worktrees.retain(|m| !gone.iter().any(|g| same_path(g, m)));
                self.cleanup_worktrees(EVERYONE, safe);
            }
            Ev::WorktreesTidied { client, removed, kept } => {
                self.pending_ops -= 1;
                self.send(client, ServerMsg::Notice(tidied_note(&removed, &kept)));
                self.send(client, ServerMsg::Reply(Reply::Ok));
                self.poll_git_soon();
                self.dirty = true;
            }
            Ev::CodexUsage { facts, limits } => self.codex_usage(facts, limits),
            Ev::WorktreeRemoved { client, path, result } => {
                self.pending_ops -= 1;
                match result {
                    Ok(()) => {
                        self.send(client, ServerMsg::Notice(format!("removed worktree {}", path.display())));
                        self.send(client, ServerMsg::Reply(Reply::Ok));
                    }
                    Err(e) => self.send(client, ServerMsg::Error(e)),
                }
            }
        }
    }

    fn message(&mut self, client: ClientId, msg: ClientMsg) {
        match msg {
            ClientMsg::Hello { .. } => {}
            ClientMsg::Input { term, data } => {
                // Typing into another pane needs write; into one that's asking, respond.
                let asking = self.terms.get(&term).is_some_and(|t| t.status == Status::Blocked);
                if let Err(e) = self.may(client, Grant::Write, Some(term)).and_then(|_| if asking { self.may(client, Grant::Respond, Some(term)) } else { Ok(()) }) {
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
                if let Some(t) = self.terms.get_mut(&term)
                    && (t.cols, t.rows) != (cols, rows)
                {
                    t.resize(cols, rows);
                    self.dirty = true;
                }
            }
            ClientMsg::Hook { term, ref token, pid, .. } => {
                // Only the pane's own processes may report its status. They carry its secret;
                // when the chain can be traced it must also lead back to the pane, so a desktop
                // app that inherited the pane's environment still can't. The chain is walked on
                // the hook thread (it's slow on Windows), in arrival order.
                let Some(pane) = self.terms.get(&term) else { return };
                if !term::same_secret(&pane.token, token) {
                    tracing::info!("ignoring a status report for pane {term} without its secret");
                    return;
                }
                let job = HookJob { root: pane.pid.filter(|_| pid != 0), pid, known: pane.trusted.clone(), msg };
                if self.hook_jobs.send(job).is_err() {
                    tracing::error!("the hook thread is gone; status reports are lost");
                }
            }
            ClientMsg::Usage { term, token, usage, limits } => {
                if self.terms.get(&term).is_some_and(|t| term::same_secret(&t.token, &token)) {
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
                    Query::List => ServerMsg::Reply(Reply::List(self.snapshot())),
                    Query::Read { term } if self.may(client, Grant::Read, Some(term)).is_err() => {
                        ServerMsg::Error(format!("{:#}", self.may(client, Grant::Read, Some(term)).unwrap_err()))
                    }
                    Query::Read { term } => match self.terms.get(&term) {
                        Some(t) => ServerMsg::Reply(Reply::Text(t.parser.screen().contents())),
                        None => ServerMsg::Error(format!("no pane {term}")),
                    },
                    Query::CheckOutput { term } if self.may(client, Grant::Read, Some(term)).is_err() => {
                        ServerMsg::Error(format!("{:#}", self.may(client, Grant::Read, Some(term)).unwrap_err()))
                    }
                    Query::CheckOutput { term } => match self.terms.get(&term).and_then(|t| t.check.as_ref()) {
                        Some(record) => ServerMsg::Reply(Reply::Text(record.output.clone())),
                        None => ServerMsg::Error(format!("no check output for pane {term}")),
                    },
                    Query::Worktrees { ws } => {
                        let Some(dir) = self.workspaces.iter().find(|w| w.id == ws).map(|w| w.cwd.clone()) else {
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
        let ws = self.active_ws.and_then(|id| self.workspaces.iter().find(|w| w.id == id))?;
        ws.tab().map(|t| t.focus)
    }



    fn snapshot(&self) -> Snapshot {
        let terms: BTreeMap<TermId, TermInfo> = self
            .terms
            .values()
            .filter(|t| !t.spare)
            .map(|t| {
                (t.id, TermInfo {
                    id: t.id,
                    cols: t.cols,
                    rows: t.rows,
                    title: t.title().to_string(),
                    process: t.process.clone(),
                    remote: t.remote.clone(),
                    popup: t.popup,
                    agent: t.agent.clone(),
                    status: t.status,
                    cwd: t.cwd.clone(),
                    summary: t.summary.clone(),
                    name: if t.name.is_empty() { t.first_prompt.clone() } else { t.name.clone() },
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
                    why_hook: t.last_hook.as_ref().map(|h| h.0.clone()).unwrap_or_default(),
                    why_hook_at: t.last_hook.as_ref().map(|h| h.1).unwrap_or(0),
                    why_screen: t.last_screen.as_ref().map(|h| h.0.clone()).unwrap_or_default(),
                    why_screen_at: t.last_screen.as_ref().map(|h| h.1).unwrap_or(0),
                    why_process_at: t.last_process,
                    pid: t.pid.unwrap_or(0),
                    asleep: t.asleep,
                    win32_input: t.win32_input,
                    usage: t.usage.clone(),
                    resume_at: t.resume_at,
                    check: t.check.as_ref().map(|c| c.info.clone()),
                    subagents: t.subagents.iter().map(|(_, k)| k.clone()).collect(),
                    started_by: t.started_by,
                    queued: t.queue.iter().cloned().collect(),
                    subagent_activity: t.subagent_activity.clone(),
                    session: t.session.clone(),
                })
            })
            .collect();
        Snapshot {
            daemon_version: env!("CARGO_PKG_VERSION").into(),
            workspaces: self.workspaces.clone(),
            active_ws: self.active_ws,
            terms,
            questions: self.questions.iter().map(|(q, ..)| q.clone()).collect(),
            limits: self.limits.iter().map(|(a, l)| (a.clone(), l.clone())).collect(),
            spent_today: self.spent_today(),
            today: usage::today(&self.turns, &self.spent, term::unix_now()),
        }
    }

    fn sync_scan_roots(&self) {
        let roots = self.terms.values().filter_map(|t| t.pid.map(|p| (t.id, p))).collect();
        self.scan.lock().unwrap().roots = roots;
    }

    fn next(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Claude, started by seshi, learns how to move itself into a worktree (and may run
    /// just that command without asking).
    fn teach(&self, cmd: &str) -> String {
        let mut words = cmd.split_whitespace();
        let first = words.next().unwrap_or("");
        let is_claude = std::path::Path::new(first).file_stem().is_some_and(|s| s.eq_ignore_ascii_case("claude"));
        if !self.cfg.teach_agents || !is_claude || cmd.contains("--append-system-prompt") {
            return cmd.to_string();
        }
        let note = "You are running inside Seshi, a terminal where one person runs many coding agents. If the user asks you to do the work in a worktree (or on a separate branch so you don't touch their checkout), run `seshi worktree --move <short-branch-name>` with the Bash tool before editing anything, then end your turn: Seshi creates the worktree and restarts you there in this same conversation.";
        let rest = cmd[first.len()..].trim_start();
        format!(
            "{first} --append-system-prompt {} --allowedTools {} {rest}",
            self.cfg.quote_for_shell(note),
            self.cfg.quote_for_shell("Bash(seshi worktree:*)")
        )
        .trim_end()
        .to_string()
    }

    /// The pane a start comes from, if it comes from one. Only a connection that proved it
    /// is that pane (its secret) counts: a session can't be hung on a pane by naming it.
    fn starter(&mut self, client: ClientId) -> Option<TermId> {
        self.next_starter.take().or_else(|| self.clients.get(&client).and_then(|c| c.from)).filter(|t| self.terms.contains_key(t))
    }

    /// The pane a start comes from, once there's room for what it starts. A session another
    /// agent starts is a helper, and each helper runs builds and tests of its own: a
    /// commander that starts ten takes the whole machine. Past the limit the start is
    /// refused, with what to do instead.
    fn starting(&mut self, client: ClientId) -> Result<Option<TermId>> {
        // A worktree asked for earlier was let through then: don't strand it now.
        let let_through = self.next_starter.is_some();
        let by = self.starter(client);
        let most = self.cfg.mcp.helpers_at_once;
        if by.is_some() && most > 0 && !let_through {
            let busy = self.terms.values().filter(|t| t.started_by.is_some() && (t.status == Status::Working || t.born.elapsed() < HELPER_SETTLES)).count();
            if busy >= most {
                anyhow::bail!("{busy} helpers are already working, and {most} at once is the limit (Settings, Agents). Wait for one to finish, then start this one.");
            }
        }
        Ok(by)
    }

    /// Note which pane's agent started `term`.
    fn started(&mut self, term: TermId, by: Option<TermId>) {
        if let Some(t) = self.terms.get_mut(&term) {
            t.started_by = by.filter(|b| *b != term);
        }
    }

    fn spawn(&mut self, cmd: Option<&str>, cwd: &std::path::Path, cols: u16, rows: u16) -> Result<TermId> {
        // An account's command is taught as the command inside it, and keeps its account.
        let (account, inner) = cmd.map(|c| self.cfg.split_account(c)).unwrap_or((None, ""));
        let taught = cmd.map(|_| {
            let taught = self.teach(inner);
            account.as_deref().map(|a| self.cfg.as_account(a, &taught)).unwrap_or(taught)
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
        let mut t = Term::spawn(&self.cfg, SpawnSpec { id, cmd, cwd, cols, rows, once }, self.tx.clone())?;
        t.account = account;
        self.terms.insert(id, t);
        self.had_terms = true;
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
        self.workspaces.iter_mut().find(|w| w.id == id).ok_or_else(|| anyhow::anyhow!("no workspace {id}"))
    }

    fn locate(&self, term: TermId) -> Option<(WsId, TabId)> {
        self.workspaces
            .iter()
            .find_map(|w| w.tabs.iter().find(|t| t.layout.contains(term)).map(|t| (w.id, t.id)))
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
        let before = self.workspaces.iter().position(|w| Some(w.id) == self.active_ws);
        self.workspaces.retain(|w| !w.tabs.is_empty());
        if self.active_ws.is_none_or(|id| !self.workspaces.iter().any(|w| w.id == id)) {
            let i = before.unwrap_or(0).min(self.workspaces.len().saturating_sub(1));
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
        let dir = std::env::temp_dir().join(format!("seshi-trust-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("claude.json");
        std::fs::write(&f, r#"{"projects":{"c:/code/app":{"hasTrustDialogAccepted":true}},"other":1}"#).unwrap();
        super::trust_in(&f, std::path::Path::new("C:/code/app"), std::path::Path::new("C:/code/app-worktrees/x"));
        super::trust_in(&f, std::path::Path::new("C:/code/nope"), std::path::Path::new("C:/code/nope-wt"));
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v["projects"]["C:/code/app-worktrees/x"]["hasTrustDialogAccepted"], true);
        assert!(v["projects"].get("C:/code/nope-wt").is_none(), "never trusts what you didn't");
        assert_eq!(v["other"], 1, "the rest of the file is kept");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
