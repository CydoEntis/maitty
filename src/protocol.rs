//! Wire protocol between the daemon and its clients (the TUI and one-shot CLI calls).
//!
//! Frames are length-delimited MessagePack. The daemon is the single source of truth for
//! the workspace / tab / pane tree; clients render `Snapshot`s and send `Command`s.

use crate::layout::{Dir, Node};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Bump when a message shape changes incompatibly (docs/API_PATTERNS.md, *Project-specific: API
/// style*); client and daemon refuse to talk across versions, and so do hooks.
pub const PROTOCOL_VERSION: u32 = 48;

pub type TermId = u32;
pub type WsId = u32;
pub type TabId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ClientMsg {
    /// First frame of every connection. `attach` clients get replays and live output.
    /// `from`: the pane this request comes from (its id and secret), for a command run
    /// inside a pane (an agent); the server checks what that pane may do.
    Hello {
        version: u32,
        attach: bool,
        #[serde(default)]
        from: Option<(TermId, String)>,
    },
    Command(Command),
    Input { term: TermId, #[serde(with = "serde_bytes")] data: Vec<u8> },
    Resize { term: TermId, cols: u16, rows: u16 },
    /// Lifecycle event reported by an agent's hook (`<app> hook ...`).
    Hook {
        term: TermId,
        agent: String,
        status: HookStatus,
        session: Option<String>,
        cwd: Option<PathBuf>,
        /// The prompt the user just gave the agent, for its card.
        prompt: Option<String>,
        /// The agent's last message (its summary of what it did).
        said: Option<String>,
        /// A subagent starting or stopping.
        subagent: Option<Subagent>,
        /// The hook event ("Stop", "Notification:permission_prompt", …).
        event: String,
        /// The reporting process; only reports from inside the pane's own process tree count.
        pid: u32,
        /// The pane's secret (<APP>_PANE_TOKEN), for when the process chain can't be traced
        /// (Git Bash on Windows hands each command to a fresh process).
        #[serde(default)]
        token: String,
        /// The agent's conversation file (Claude's transcript), for moving it elsewhere.
        transcript: Option<PathBuf>,
        /// The model it's answering with right now (from the transcript).
        model: Option<String>,
        /// The name you gave the conversation in the agent (Claude's /rename).
        name: Option<String>,
        /// Ids of the background tasks (shells, helpers) still running when a turn stopped.
        #[serde(default)]
        background: Vec<String>,
    },
    /// What an agent in a pane has used (from Claude's status line), with the pane's secret.
    Usage { term: TermId, token: String, usage: Usage, limits: Vec<Limit> },
    Query(Query),
}




/// What an agent's session has used.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// How full its context is, 0–100.
    pub context: Option<f32>,
    /// What the session has cost so far at list price, in US dollars (Claude's estimate).
    pub cost: Option<f64>,
}

/// One of an agent's plan limits: how much is used and when it starts over.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Limit {
    /// Which window: "5h", "week".
    pub name: String,
    /// 0–100.
    pub used: f32,
    /// Unix seconds.
    pub resets_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    NewWorkspace { cwd: Option<PathBuf>, name: Option<String>, cmd: Option<String> },
    CloseWorkspace { ws: WsId },
    RenameWorkspace { ws: WsId, name: String },
    SelectWorkspace { ws: WsId },
    NewTab { ws: WsId, name: Option<String>, cmd: Option<String> },
    CloseTab { ws: WsId, tab: TabId },
    RenameTab { ws: WsId, tab: TabId, name: String },
    SelectTab { ws: WsId, tab: TabId },
    /// `cwd`: where the new pane starts (default: where the split pane is).
    Split { term: TermId, dir: Dir, cmd: Option<String>, cwd: Option<PathBuf> },
    ClosePane { term: TermId },
    /// Focus a pane anywhere: switches workspace and tab as needed.
    FocusPane { term: TermId },
    /// Focus a pane and bring the windows showing the app to the front (a notification for it
    /// was clicked).
    Reveal { term: TermId },
    /// You've seen a finished agent (the cursor rested on it): done → idle.
    MarkSeen { term: TermId },
    /// Name a pane yourself (empty: back to the automatic name).
    RenamePane { term: TermId, name: String },
    /// The agent in `term` moves into a new worktree of its repo: made now, and when its turn
    /// ends it restarts there, resumed (the agent runs `<app> worktree --move`).
    MoveToWorktree { term: TermId, branch: Option<String> },
    /// Grow (positive) or shrink the pane along `dir`'s axis by `delta` (fraction of parent).
    ResizePane { term: TermId, dir: Dir, delta: f32 },
    /// Trade the places of two panes of one tab; each takes the other's size. An error when
    /// they are in different tabs.
    SwapPanes { a: TermId, b: TermId },
    /// `git worktree add` next to the workspace's repo, then open it as a new workspace.
    /// `split`: open the worktree as a pane beside this one instead of as a workspace.
    /// `from`: the repo to branch from (default: the workspace's folder).
    NewWorktree {
        ws: WsId,
        branch: String,
        base: Option<String>,
        cmd: Option<String>,
        split: Option<TermId>,
        from: Option<PathBuf>,
    },
    /// From a pane about to start an agent in `dir`: a new worktree for it when `dir` is
    /// in a repo's main checkout (and per-agent worktrees are on). Replies with the folder
    /// to start in, or empty text to start where it is.
    AgentWorktree { dir: PathBuf },
    /// Put a pane in a sidebar group (None: take it out).
    SetGroup { ws: WsId, group: Option<String> },
    /// Move a pane into another group (`to`), or into a new group named `name`.
    MovePane { term: TermId, to: Option<WsId>, name: Option<String> },
    /// Put back the pane the last automatic workspace took.
    UndoAutoWorkspace,
    /// Close everything running in a linked worktree, remove it and delete its branch (a
    /// branch just merged).
    CloseWorktree { path: PathBuf },
    /// Remove these worktrees, none of which has anything running in it: each a folder and
    /// whether to go ahead with uncommitted changes in it (they are lost). A branch is kept
    /// unless it's already merged.
    TidyWorktrees { items: Vec<(PathBuf, bool)> },
    /// Close a worktree workspace and `git worktree remove` its checkout.
    /// `delete_branch`: also delete the worktree's branch (discarding a task).
    RemoveWorktree { ws: WsId, force: bool, delete_branch: bool },
    /// Make a pane its own workspace, homed at the pane's current directory. A workspace's
    /// only pane re-homes the workspace instead.
    PaneToWorkspace { term: TermId },
    /// Index into the configured workspace colour palette.
    SetWorkspaceColor { ws: WsId, color: u8 },
    ReloadConfig,
    /// The program running in `term` is an agent: add it to config (by its program, or by
    /// its script when node/python runs it) so its status shows.
    TeachAgent { term: TermId },
    /// Run a command in a floating pane over everything; it closes when the command exits.
    Popup { cmd: String, cwd: Option<PathBuf> },
    /// An agent asks you a question with these answers; the reply (the chosen answer) comes
    /// once you pick one.
    AskHuman { term: TermId, text: String, options: Vec<String> },
    /// Your answer (index into its options) to question `id`.
    AnswerHuman { id: u64, choice: usize },
    /// What a pane may do through the app (read, write, start, respond, admin); None goes back
    /// to the default from config. Only you can (not from inside a pane).
    Grant { term: TermId, grants: Option<Vec<String>> },
    /// A follow-up for an agent that's working: typed into it when its turn is over.
    QueueFollowUp { term: TermId, text: String },
    /// Drop the follow-ups queued for a pane.
    ClearQueue { term: TermId },
    /// Run the pane's checkpoint `name` again (None: all of them). While its agent is working
    /// the run waits for the turn to end.
    RerunCheck { term: TermId, name: Option<String> },
    /// Accept a failed checkpoint as it is (None: the headline one); `reason` may be empty.
    /// Covers that one result only.
    OverrideCheck { term: TermId, name: Option<String>, reason: String },
    /// Take an override back (None: the headline checkpoint): it reads as failed again.
    UndoOverride { term: TermId, name: Option<String> },
    /// Your answer to a repo-supplied check command waiting for approval. Only you can (not
    /// from inside a pane). `command_sha256` (`command_hash`) is of the command you were
    /// shown: the answer is refused if a different one waits by then.
    ApproveCheck { term: TermId, name: String, choice: ApprovalChoice, command_sha256: String },
    /// Drop a remembered Always or Never for a repo's checkpoint. Only you can.
    ForgetCheckApproval { project: PathBuf, name: String },
    /// Your answer to a session that asked to start helpers (`TermInfo::helper_ask`). Only you
    /// can (not from inside a pane), and only while the request is pending.
    AnswerHelpers { term: TermId, choice: HelperChoice },
    /// `forget`: also discard the saved session, so the next start is a clean slate.
    KillServer { forget: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Query {
    List,
    /// Plain-text screen contents of a terminal (the visible screen).
    Read { term: TermId },
    /// Every worktree of the repository a workspace is in.
    Worktrees { ws: WsId },
    /// The tail of a pane's last output for checkpoint `name` (None: the headline one); the
    /// snapshot carries only a summary.
    CheckOutput { term: TermId, name: Option<String> },
}

/// What you chose when a session asked to start helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelperChoice {
    /// This session may start helpers from now on.
    Allow,
    /// Every session in this session's project may; remembered across restarts.
    Always,
    /// Do it alone: this session's start requests are refused.
    No,
}

/// What you chose for a repo-supplied check command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalChoice {
    Once,
    /// This exact command, until it changes.
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookStatus {
    Working,
    Blocked,
    Done,
    Idle,
    /// The agent session ended; the pane is a plain shell again.
    Gone,
    /// Leave the status as it is (the event only carries other news, e.g. a subagent).
    Same,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subagent {
    pub id: String,
    /// "Explore", "code-reviewer", …
    pub kind: String,
    pub start: bool,
    #[serde(default)]
    pub reported_at: u64,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub activity: Option<String>,
}

/// Lifecycle evidence for one provider-reported child agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentActivity {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub state: SubagentState,
    pub task: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub started_at: Option<u64>,
    pub updated_at: u64,
    pub latest_activity: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubagentState {
    Starting,
    Working,
    WaitingForInput,
    AwaitingApproval,
    Completed,
    Failed,
    Cancelled,
    ConnectionLost,
}

impl SubagentState {
    pub fn active(self) -> bool {
        matches!(self, Self::Starting | Self::Working | Self::WaitingForInput | Self::AwaitingApproval)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    Welcome { version: u32 },
    State(Snapshot),
    /// Sent once per terminal on attach: the recent output ring, to rebuild scrollback.
    Replay { term: TermId, cols: u16, rows: u16, #[serde(with = "serde_bytes")] data: Vec<u8> },
    Output { term: TermId, #[serde(with = "serde_bytes")] data: Vec<u8> },
    /// An agent needs attention (blocked, or finished while unfocused).
    Attention { term: TermId, status: Status },
    Reply(Reply),
    Error(String),
    /// Informational message for the status bar.
    Notice(String),
    /// You closed the last pane of a worktree that holds commits found nowhere else: ask
    /// whether its folder goes too (its branch stays either way).
    AskRemoveWorktree { path: PathBuf, name: String, why: String },
    /// A program in a pane copied text (OSC 52): put it on the user's clipboard.
    Clipboard { term: TermId, text: String },
    /// An agent started in a new repo, which became a workspace (undoable).
    AutoWorkspace(String),
    /// Come to the front: a notification for one of this server's sessions was clicked.
    Raise,
    Bye,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Reply {
    Ok,
    List(Snapshot),
    Text(String),
    Worktrees(Vec<WorktreeEntry>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    /// Branch name, or a short commit for a detached HEAD.
    pub branch: String,
    /// The repository's main checkout.
    pub main: bool,
    pub repo: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Status {
    #[default]
    None,
    Idle,
    Working,
    Blocked,
    /// Finished a turn and nobody has looked yet.
    Done,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::None => "",
            Status::Idle => "idle",
            Status::Working => "working",
            Status::Blocked => "blocked",
            Status::Done => "done",
        }
    }
    /// Sort order for "what needs me": blocked first, then done, working, idle.
    pub fn urgency(self) -> u8 {
        match self {
            Status::Blocked => 0,
            Status::Done => 1,
            Status::Working => 2,
            Status::Idle => 3,
            Status::None => 4,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub daemon_version: String,
    pub workspaces: Vec<WorkspaceInfo>,
    pub active_ws: Option<WsId>,
    pub terms: BTreeMap<TermId, TermInfo>,
    /// Questions agents asked you (`<app> ask-human`), waiting for an answer.
    #[serde(default)]
    pub questions: Vec<HumanQuestion>,
    /// Plan limits by agent ("claude", "codex"), as last reported.
    #[serde(default)]
    pub limits: Vec<(String, Vec<Limit>)>,
    /// What Claude sessions in the app have cost over the last day, at list price.
    #[serde(default)]
    pub spent_today: f64,
    /// What each agent got done over the last day, by agent and folder.
    #[serde(default)]
    pub today: Vec<AgentDay>,
}

/// An agent's last day in one folder: turns it finished, time it spent working, its cost.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentDay {
    pub agent: String,
    pub place: String,
    pub turns: u32,
    pub working_secs: u64,
    pub cost: f64,
}

/// A question an agent asked you, with the answers it allows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanQuestion {
    pub id: u64,
    /// The pane that asked.
    pub term: TermId,
    pub text: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: WsId,
    pub name: String,
    pub cwd: PathBuf,
    pub tabs: Vec<TabInfo>,
    pub active_tab: TabId,
    pub git: Option<GitInfo>,
    /// A worktree the app created; offered for removal when closed.
    pub worktree: bool,
    /// Palette index for this workspace's colour (tint, borders, sidebar marker).
    pub color: u8,
    /// Created automatically and not looked at yet (shows a NEW chip).
    pub is_new: bool,
    /// The sidebar group it's in, if the user put it in one.
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitInfo {
    pub repo: String,
    pub branch: String,
    /// Changed + untracked files.
    pub dirty: u32,
    /// A linked worktree rather than the main checkout.
    pub linked: bool,
    /// The repository's main checkout; workspaces sharing it are one repo.
    pub root: PathBuf,
    /// Every worktree of the repository, open in the app or not.
    pub worktrees: Vec<WorktreeEntry>,
    /// For a linked worktree: commits on its branch that the main checkout's branch lacks.
    pub ahead: u32,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: TabId,
    pub name: String,
    pub layout: Node,
    pub focus: TermId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermInfo {
    pub id: TermId,
    pub cols: u16,
    pub rows: u16,
    /// Title set by the program (OSC 0/2), if any.
    pub title: String,
    /// Name of the most interesting process running in the pane (the leaf of the tree).
    pub process: String,
    /// The machine an ssh (or mosh, …) client in the pane is connected to.
    #[serde(default)]
    pub remote: Option<String>,
    /// A floating pane over everything (`<app> popup`); gone when its command exits.
    #[serde(default)]
    pub popup: bool,
    /// Detected or hook-reported agent name.
    pub agent: Option<String>,
    pub status: Status,
    pub cwd: PathBuf,
    /// What the agent is on: the last prompt it was given (one line), if known.
    pub summary: String,
    /// What to call it: the name you gave it in the agent (/rename), else its first prompt.
    pub name: String,
    /// The name you gave the pane in the app (Rename pane), which wins over everything.
    #[serde(default)]
    pub label: String,
    /// The model it's using, short ("opus 4.5"), if known.
    pub model: String,
    /// Memory used by what runs in it (bytes).
    #[serde(default)]
    pub mem: u64,
    /// CPU used by what runs in it, as a share of the whole machine (0 to 100).
    #[serde(default)]
    pub cpu: f32,
    /// It rang the bell and you haven't looked yet.
    #[serde(default)]
    pub bell: bool,
    /// The agent's last message, if hooks reported it.
    pub said: String,
    /// The git branch the pane is on, if it's in a repo.
    pub branch: Option<String>,
    /// The pane is in a linked worktree.
    pub linked: bool,
    /// Subagents running right now (their kinds).
    pub subagents: Vec<String>,
    /// Persistent lifecycle records reported by the runtime.
    #[serde(default)]
    pub subagent_activity: Vec<SubagentActivity>,
    /// Provider conversation ID that owns these child records.
    #[serde(default)]
    pub session: Option<String>,
    /// The pane whose agent started this one, if one did.
    #[serde(default)]
    pub started_by: Option<TermId>,
    /// The helpers this session asked to start and is waiting on you for, one task each. Not
    /// empty means the row needs you ("wants to start N helpers"); `AnswerHelpers` answers it.
    #[serde(default)]
    pub helper_ask: Vec<String>,
    /// Follow-ups waiting for its turn to end, next first.
    #[serde(default)]
    pub queued: Vec<String>,
    /// The repository's main checkout, if the pane is in a repo.
    pub root: Option<PathBuf>,
    /// The checkout (main or a linked worktree) the pane is in.
    pub top: Option<PathBuf>,
    /// When its status last changed (unix seconds).
    pub since: u64,
    /// What set its status ("hook: Stop", "screen: matched `esc to interrupt`", "you looked").
    #[serde(default)]
    pub status_why: String,
    /// The last hook event and the last screen evidence, with when (unix seconds; 0: none).
    #[serde(default)]
    pub why_hook: String,
    #[serde(default)]
    pub why_hook_at: u64,
    #[serde(default)]
    pub why_screen: String,
    #[serde(default)]
    pub why_screen_at: u64,
    /// Last completed process observation (unix seconds; 0: not observed).
    #[serde(default)]
    pub why_process_at: u64,
    /// Working, but no output and no hook report since then (unix seconds) for long enough
    /// that it may be stuck. Its status stays working.
    #[serde(default)]
    pub quiet_since: Option<u64>,
    /// The process running in it (0: not known).
    #[serde(default)]
    pub pid: u32,
    /// Left asleep by an older daemon; focusing it (or typing) resumes its conversation.
    pub asleep: bool,
    /// Windows' console layer asked for native key records (win32-input-mode), so keys
    /// plain VT can't express (Ctrl+Shift+letter, Ctrl+Enter) can be sent exactly.
    pub win32_input: bool,
    /// Context and cost, when the agent reports them.
    #[serde(default)]
    pub usage: Usage,
    /// It hit a plan limit; the app says "continue" at this time (unix seconds).
    #[serde(default)]
    pub resume_at: Option<u64>,
    /// The latest result of each checkpoint the app runs after its turn ends, in project order.
    /// The output stays on the daemon. An older peer's single `check` reads as one checkpoint.
    #[serde(default, alias = "check", deserialize_with = "one_or_many_checks")]
    pub checks: Vec<CheckInfo>,
}

pub(crate) fn one_or_many_checks<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<CheckInfo>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Shape {
        Many(Vec<CheckInfo>),
        One(Option<Box<CheckInfo>>),
    }
    Ok(match Shape::deserialize(d)? {
        Shape::Many(checks) => checks,
        Shape::One(check) => check.map(|c| *c).into_iter().collect(),
    })
}

/// The longest reason an override may carry, in characters.
pub const OVERRIDE_REASON_MAX_CHARS: usize = 280;

/// What an approval of a repo's check command is tied to: its SHA-256, in lowercase hex.
pub fn command_hash(command: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(command.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Where a check stands. `CouldNotRun` is not a failure: the work was never checked.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckState {
    Checking,
    Passed,
    Failed,
    /// A failed result the user chose to accept; `CheckInfo` still says what failed.
    Overridden,
    #[default]
    CouldNotRun,
    /// Waiting its turn behind another checkpoint in the same run.
    Pending,
    /// A repo-supplied command that nobody has approved yet.
    NeedsApproval,
    /// It passed, but the agent changed files after it ran.
    Stale,
    /// You chose never to run this repo's command.
    Never,
}

impl CheckState {
    /// The name scripts read in `<app> ls --json`.
    pub fn label(self) -> &'static str {
        match self {
            CheckState::Checking => "checking",
            CheckState::Passed => "passed",
            CheckState::Failed => "failed",
            CheckState::Overridden => "overridden",
            CheckState::CouldNotRun => "could_not_run",
            CheckState::Pending => "pending",
            CheckState::NeedsApproval => "needs_approval",
            CheckState::Stale => "stale",
            CheckState::Never => "never",
        }
    }

    /// Rank for the checkpoint a session is reported by: the lowest is the headline.
    fn headline_rank(self) -> u8 {
        match self {
            CheckState::Failed => 0,
            CheckState::CouldNotRun => 1,
            CheckState::NeedsApproval => 2,
            CheckState::Checking => 3,
            CheckState::Pending => 4,
            CheckState::Stale => 5,
            CheckState::Never => 6,
            CheckState::Overridden => 7,
            CheckState::Passed => 8,
        }
    }
}

/// Who named a check command.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckSource {
    /// You, in the app's settings.
    #[default]
    User,
    /// The repository's own file (`.<app>/checks.toml`), as of `commit` when it is committed.
    Repo { file: String, commit: Option<String>, committed_at: Option<u64>, uncommitted: bool },
}

/// A file the agent changed after a check ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedFile {
    /// `M`, `A`, `D` or `R`, as git reports it.
    pub kind: char,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckInfo {
    pub command: String,
    pub state: CheckState,
    /// Absent while checking, or when the command never started or timed out.
    pub exit_code: Option<i32>,
    /// When it started (unix seconds).
    pub started_at: u64,
    pub duration_ms: u64,
    /// When the agent turn that triggered it ended (unix seconds).
    pub after_turn_ended_at: u64,
    /// The first output line that looks like a failure; empty when there is none.
    pub first_failing_line: String,
    /// Another turn ended while it ran, so it runs again when this one settles.
    pub rerun_queued: bool,
    /// When the user overrode a failure (unix seconds); only set on `Overridden`.
    pub overridden_at: Option<u64>,
    pub override_reason: String,
    /// The checkpoint's name (`test`, `build`).
    pub name: String,
    /// The folder it ran in.
    pub cwd: PathBuf,
    /// Which of the session's turns it checked.
    pub turn: u32,
    /// Lines of the full output, before the tail was capped.
    pub line_count: u64,
    /// When files changed after it passed (unix seconds); set on `Stale`.
    pub stale_since: Option<u64>,
    pub changed_files: Vec<ChangedFile>,
    /// Who overrode it; set on `Overridden`.
    pub overridden_by: String,
    pub source: CheckSource,
    /// On `NeedsApproval`: the command you had said Always to, before the repo changed it.
    pub previous_command: Option<String>,
}

/// The one rule for whether a check is reported beside a status, for the daemon's callers and
/// the client's rows alike.
pub fn reported_check(status: Status, check: Option<&CheckInfo>) -> Option<&CheckInfo> {
    check.filter(|_| !matches!(status, Status::Working | Status::Blocked))
}

/// The checkpoint a session is reported by: the worst state wins, the first in order on ties.
pub fn headline(checks: &[CheckInfo]) -> Option<&CheckInfo> {
    checks.iter().min_by_key(|c| c.state.headline_rank())
}

impl TermInfo {
    /// The headline checkpoint (see `headline`), if any ran or waits.
    pub fn check(&self) -> Option<&CheckInfo> {
        headline(&self.checks)
    }

    /// The check to report beside the status. While the agent works or asks, the last result is
    /// about an older turn, so none is reported.
    pub fn reported_check(&self) -> Option<&CheckInfo> {
        reported_check(self.status, self.check())
    }

    /// What to call this pane: the agent's name, else the folder a shell is sitting in, else
    /// the running program's title or name (`vim`, `lazygit`, `node`).
    pub fn display_name(&self) -> String {
        if let Some(a) = &self.agent {
            return a.clone();
        }
        if self.is_shell() && !self.cwd.as_os_str().is_empty() {
            return match self.cwd.file_name() {
                Some(n) => n.to_string_lossy().into_owned(),
                // A drive root like C:\ has no folder name; show it whole.
                None => self.cwd.display().to_string(),
            };
        }
        // Programs often title the window with their own exe path; that says nothing.
        let path_like = self.title.contains('\\') || self.title.contains('/') || self.title.ends_with(".exe");
        if !self.title.is_empty() && !path_like {
            return self.title.clone();
        }
        self.process.clone()
    }

    /// The pane is at a shell prompt (nothing else running in it).
    pub fn is_shell(&self) -> bool {
        matches!(
            self.process.to_ascii_lowercase().as_str(),
            "pwsh" | "powershell" | "cmd" | "bash" | "zsh" | "fish" | "nu" | "sh" | "dash" | "elvish" | "xonsh"
        )
    }
}

impl Snapshot {
    pub fn workspace(&self, id: WsId) -> Option<&WorkspaceInfo> {
        self.workspaces.iter().find(|w| w.id == id)
    }
    pub fn active(&self) -> Option<&WorkspaceInfo> {
        self.active_ws.and_then(|id| self.workspace(id))
    }
    /// (workspace, tab) containing a terminal.
    pub fn locate(&self, term: TermId) -> Option<(&WorkspaceInfo, &TabInfo)> {
        self.workspaces.iter().find_map(|w| {
            w.tabs.iter().find(|t| t.layout.contains(term)).map(|t| (w, t))
        })
    }
}

impl WorkspaceInfo {
    pub fn tab(&self) -> Option<&TabInfo> {
        self.tabs.iter().find(|t| t.id == self.active_tab).or(self.tabs.first())
    }
}

pub fn encode<T: Serialize>(msg: &T) -> anyhow::Result<bytes::Bytes> {
    Ok(rmp_serde::to_vec_named(msg)?.into())
}

pub fn decode<'a, T: Deserialize<'a>>(buf: &'a [u8]) -> anyhow::Result<T> {
    Ok(rmp_serde::from_slice(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encoding what was decoded gives the same bytes: nothing is lost on the way.
    fn round_trips<T: Serialize + for<'a> Deserialize<'a>>(msg: &T) {
        let bytes = encode(msg).unwrap();
        let back: T = decode(&bytes).unwrap();
        assert_eq!(encode(&back).unwrap(), bytes);
    }

    #[test]
    fn every_message_round_trips() {
        let clients = vec![
            ClientMsg::Hello { version: PROTOCOL_VERSION, attach: true, from: Some((3, "00ff".into())) },
            ClientMsg::Input { term: 3, data: b"ls\r".to_vec() },
            ClientMsg::Resize { term: 3, cols: 120, rows: 40 },
            ClientMsg::Hook {
                term: 3,
                agent: "claude".into(),
                status: HookStatus::Blocked,
                session: Some("abc-123".into()),
                cwd: Some(PathBuf::from("/code/api")),
                prompt: Some("fix it".into()),
                said: Some("done".into()),
                subagent: Some(Subagent { id: "s1".into(), kind: "Explore".into(), start: true, reported_at: 0, task: None, model: None, effort: None, activity: None }),
                event: "PreToolUse".into(),
                pid: 42,
                token: "00ff".into(),
                transcript: Some(PathBuf::from("/t.jsonl")),
                model: Some("opus".into()),
                name: Some("api work".into()),
                background: vec!["b1".into()],
            },
            ClientMsg::Query(Query::List),
            ClientMsg::Query(Query::Read { term: 3 }),
            ClientMsg::Query(Query::Worktrees { ws: 1 }),
            ClientMsg::Query(Query::CheckOutput { term: 3, name: None }),
            ClientMsg::Query(Query::CheckOutput { term: 3, name: Some("build".into()) }),
            ClientMsg::Command(Command::NewWorkspace { cwd: Some(PathBuf::from("/code")), name: None, cmd: Some("claude".into()) }),
            ClientMsg::Command(Command::Split { term: 3, dir: crate::layout::Dir::Down, cmd: None, cwd: None }),
            ClientMsg::Command(Command::ClosePane { term: 3 }),
            ClientMsg::Command(Command::Reveal { term: 3 }),
            ClientMsg::Command(Command::TeachAgent { term: 3 }),
            ClientMsg::Command(Command::Popup { cmd: "fzf".into(), cwd: None }),
            ClientMsg::Command(Command::AskHuman { term: 3, text: "Deploy?".into(), options: vec!["Yes".into(), "No".into()] }),
            ClientMsg::Command(Command::AnswerHuman { id: 1, choice: 0 }),
            ClientMsg::Command(Command::Grant { term: 3, grants: Some(vec!["read".into()]) }),
            ClientMsg::Command(Command::QueueFollowUp { term: 3, text: "then run the tests".into() }),
            ClientMsg::Command(Command::ClearQueue { term: 3 }),
            ClientMsg::Command(Command::RerunCheck { term: 3, name: None }),
            ClientMsg::Command(Command::RerunCheck { term: 3, name: Some("test".into()) }),
            ClientMsg::Command(Command::OverrideCheck { term: 3, name: Some("test".into()), reason: "flaky on CI".into() }),
            ClientMsg::Command(Command::UndoOverride { term: 3, name: None }),
            ClientMsg::Command(Command::ApproveCheck { term: 3, name: "test".into(), choice: ApprovalChoice::Once, command_sha256: command_hash("npm test") }),
            ClientMsg::Command(Command::ApproveCheck { term: 3, name: "test".into(), choice: ApprovalChoice::Always, command_sha256: command_hash("npm test") }),
            ClientMsg::Command(Command::ApproveCheck { term: 3, name: "test".into(), choice: ApprovalChoice::Never, command_sha256: command_hash("npm test") }),
            ClientMsg::Command(Command::ForgetCheckApproval { project: PathBuf::from("/code/api"), name: "test".into() }),
            ClientMsg::Command(Command::AnswerHelpers { term: 3, choice: HelperChoice::Allow }),
            ClientMsg::Command(Command::AnswerHelpers { term: 3, choice: HelperChoice::Always }),
            ClientMsg::Command(Command::AnswerHelpers { term: 3, choice: HelperChoice::No }),
            ClientMsg::Command(Command::TidyWorktrees { items: vec![(PathBuf::from("/code/.wt/rate"), false)] }),
            ClientMsg::Command(Command::RenamePane { term: 3, name: "x".into() }),
            ClientMsg::Command(Command::MoveToWorktree { term: 3, branch: Some("feat/x".into()) }),
            ClientMsg::Command(Command::SwapPanes { a: 3, b: 4 }),
        ];
        for m in &clients {
            round_trips(m);
        }
        let servers = vec![
            ServerMsg::Welcome { version: PROTOCOL_VERSION },
            ServerMsg::State(Snapshot::default()),
            ServerMsg::Replay { term: 3, cols: 80, rows: 24, data: vec![27, b'[', b'H'] },
            ServerMsg::Output { term: 3, data: b"hello".to_vec() },
            ServerMsg::Attention { term: 3, status: Status::Done },
            ServerMsg::Reply(Reply::Ok),
            ServerMsg::Reply(Reply::Text("screen".into())),
            ServerMsg::Error("nope".into()),
            ServerMsg::Notice("saved".into()),
            ServerMsg::AskRemoveWorktree { path: PathBuf::from("/w/task"), name: "task".into(), why: "not merged anywhere".into() },
            ServerMsg::Clipboard { term: 3, text: "copied".into() },
            ServerMsg::AutoWorkspace("api".into()),
            ServerMsg::Raise,
            ServerMsg::Bye,
        ];
        for m in &servers {
            round_trips(m);
        }
    }

    /// A new command is a new enum variant: a peer built before it can't read it, which is
    /// why adding one bumps `PROTOCOL_VERSION` (and the two refuse to talk).
    #[test]
    fn a_peer_from_before_swap_panes_cannot_read_it() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        enum OldCommand {
            ResizePane { term: TermId, dir: Dir, delta: f32 },
        }
        let bytes = encode(&Command::SwapPanes { a: 3, b: 4 }).unwrap();
        assert!(decode::<OldCommand>(&bytes).is_err());
        let Command::SwapPanes { a, b } = decode(&bytes).unwrap() else { panic!("expected a swap") };
        assert_eq!((a, b), (3, 4));
    }

    #[test]
    fn a_pane_without_a_check_decodes_and_one_with_a_check_round_trips() {
        let mut term: TermInfo = serde_json::from_value(serde_json::json!({
            "id": 3, "cols": 80, "rows": 24, "title": "", "process": "", "agent": null,
            "status": "Done", "cwd": "/code", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 0, "asleep": false, "win32_input": false,
        }))
        .expect("a pane from a peer that knows no checks");
        assert_eq!(term.check(), None);
        term.checks = vec![CheckInfo {
            command: "npm test".into(),
            state: CheckState::Overridden,
            exit_code: Some(1),
            started_at: 100,
            duration_ms: 14_200,
            after_turn_ended_at: 90,
            first_failing_line: "FAIL footer.test.ts".into(),
            rerun_queued: true,
            overridden_at: Some(130),
            override_reason: "flaky".into(),
            ..Default::default()
        }];
        round_trips(&ServerMsg::State(Snapshot { terms: [(3, term)].into(), ..Default::default() }));
    }

    #[test]
    fn a_pane_asking_for_helpers_round_trips_and_one_not_asking_has_none() {
        let mut term = pane_from(serde_json::json!({}));
        assert!(term.helper_ask.is_empty(), "a peer that knows no helper requests");
        term.helper_ask = vec!["fix the login test".into(), "port the footer".into()];
        round_trips(&ServerMsg::State(Snapshot { terms: [(3, term)].into(), ..Default::default() }));
    }

    #[test]
    fn a_check_from_an_older_peer_fills_in_what_it_lacks() {
        let bytes = rmp_serde::to_vec_named(&BTreeMap::from([("command", "npm test")])).unwrap();
        let info: CheckInfo = decode(&bytes).expect("only the command");
        assert_eq!((info.command.as_str(), info.exit_code, info.rerun_queued), ("npm test", None, false));
    }

    fn every_field_check() -> CheckInfo {
        CheckInfo {
            command: "npm test".into(),
            state: CheckState::NeedsApproval,
            exit_code: Some(1),
            started_at: 100,
            duration_ms: 14_200,
            after_turn_ended_at: 90,
            first_failing_line: "FAIL footer.test.ts".into(),
            rerun_queued: true,
            overridden_at: Some(130),
            override_reason: "flaky".into(),
            name: "test".into(),
            cwd: PathBuf::from("/code/api"),
            turn: 3,
            line_count: 1_284,
            stale_since: Some(140),
            changed_files: vec![ChangedFile { kind: 'M', path: "src/a.rs".into() }, ChangedFile { kind: 'A', path: "tests/b.rs".into() }],
            overridden_by: "you".into(),
            source: CheckSource::Repo { file: concat!(".", env!("APP_COMMAND"), "/checks.toml").into(), commit: Some("a1b2c3d".into()), committed_at: Some(80), uncommitted: true },
            previous_command: Some("npm t".into()),
        }
    }

    /// A pane as a peer sends it, with `extra` fields on top.
    fn pane_from(extra: serde_json::Value) -> TermInfo {
        let mut v = serde_json::json!({
            "id": 3, "cols": 80, "rows": 24, "title": "", "process": "", "agent": null,
            "status": "Done", "cwd": "/code", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 0, "asleep": false, "win32_input": false,
        });
        v.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        decode(&rmp_serde::to_vec_named(&v).unwrap()).expect("a pane")
    }

    #[test]
    fn a_check_with_every_field_round_trips() {
        round_trips(&every_field_check());
        let mut term = pane_from(serde_json::json!({}));
        term.checks = vec![every_field_check(), CheckInfo { name: "build".into(), state: CheckState::Pending, ..Default::default() }];
        round_trips(&ServerMsg::State(Snapshot { terms: [(3, term)].into(), ..Default::default() }));
    }

    #[test]
    fn a_hook_report_reads_across_the_background_field() {
        let old = serde_json::json!({ "Hook": {
            "term": 3, "agent": "claude", "status": "Done", "session": null, "cwd": null, "prompt": null,
            "said": null, "subagent": null, "event": "Stop", "pid": 42, "token": "00ff",
            "transcript": null, "model": null, "name": null,
        }});
        let msg: ClientMsg = decode(&rmp_serde::to_vec_named(&old).unwrap()).expect("a hook from before the field");
        let ClientMsg::Hook { event, background, .. } = msg else { panic!("expected a hook report") };
        assert_eq!((event.as_str(), background.len()), ("Stop", 0));

        #[derive(Deserialize)]
        enum OldClientMsg {
            Hook { term: TermId, status: HookStatus, event: String, pid: u32, token: String },
        }
        let new = ClientMsg::Hook {
            term: 3, agent: "claude".into(), status: HookStatus::Done, session: None, cwd: None, prompt: None, said: None,
            subagent: None, event: "Stop".into(), pid: 42, token: "00ff".into(), transcript: None, model: None, name: None,
            background: vec!["b1".into()],
        };
        let OldClientMsg::Hook { term, status, event, pid, token } = decode(&encode(&new).unwrap()).expect("an older daemon reads it");
        assert_eq!((term, status, event.as_str(), pid, token.as_str()), (3, HookStatus::Done, "Stop", 42, "00ff"));
    }

    #[test]
    fn a_v43_check_decodes_with_the_new_fields_defaulted() {
        let v43 = serde_json::json!({
            "command": "npm test", "state": "Failed", "exit_code": 1, "started_at": 100, "duration_ms": 14200,
            "after_turn_ended_at": 90, "first_failing_line": "FAIL", "rerun_queued": false,
            "overridden_at": null, "override_reason": "",
        });
        let info: CheckInfo = decode(&rmp_serde::to_vec_named(&v43).unwrap()).expect("a v43 check");
        assert_eq!((info.state, info.exit_code), (CheckState::Failed, Some(1)));
        assert_eq!((info.name.as_str(), info.source, info.changed_files.len(), info.previous_command), ("", CheckSource::User, 0, None));
    }

    #[test]
    fn a_pane_with_the_old_single_check_reads_as_one_checkpoint() {
        let old = pane_from(serde_json::json!({ "check": { "command": "npm test", "state": "Failed", "exit_code": 1 } }));
        assert_eq!(old.checks.iter().map(|c| (c.command.as_str(), c.state)).collect::<Vec<_>>(), [("npm test", CheckState::Failed)]);
        assert!(pane_from(serde_json::json!({ "check": null })).checks.is_empty());
        assert!(pane_from(serde_json::json!({})).checks.is_empty());
    }

    #[test]
    fn the_headline_is_the_worst_checkpoint_and_the_first_on_ties() {
        use CheckState::*;
        let order = [Failed, CouldNotRun, NeedsApproval, Checking, Pending, Stale, Never, Overridden, Passed];
        let at = |state: CheckState, name: &str| CheckInfo { state, name: name.into(), ..Default::default() };
        for (i, &worse) in order.iter().enumerate() {
            for &better in &order[i + 1..] {
                for checks in [vec![at(worse, "a"), at(better, "b")], vec![at(better, "b"), at(worse, "a")]] {
                    assert_eq!(headline(&checks).map(|c| c.state), Some(worse), "{worse:?} over {better:?}");
                }
            }
            let tie = [at(worse, "first"), at(worse, "second")];
            assert_eq!(headline(&tie).map(|c| c.name.as_str()), Some("first"), "{worse:?} tie");
        }
        assert_eq!(headline(&[]), None);
    }

    #[test]
    fn the_headline_is_not_reported_while_the_agent_works_or_asks() {
        let mut term = pane_from(serde_json::json!({}));
        term.checks = vec![CheckInfo { state: CheckState::Passed, ..Default::default() }, CheckInfo { state: CheckState::Failed, name: "test".into(), ..Default::default() }];
        assert_eq!(term.reported_check().map(|c| c.name.as_str()), Some("test"));
        for status in [Status::Working, Status::Blocked] {
            term.status = status;
            assert_eq!(term.reported_check(), None, "{status:?}");
        }
    }

    #[test]
    fn new_check_states_have_their_script_labels() {
        let got = [CheckState::Pending, CheckState::NeedsApproval, CheckState::Stale, CheckState::Never].map(CheckState::label);
        assert_eq!(got, ["pending", "needs_approval", "stale", "never"]);
    }
}
