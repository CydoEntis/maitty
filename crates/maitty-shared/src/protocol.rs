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
pub const PROTOCOL_VERSION: u32 = 74;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredIdentity {
    pub conversation: crate::conversation::ConversationId,
    pub generation: u64,
    pub connection: u64,
    pub token: String,
}

impl std::fmt::Debug for StructuredIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructuredIdentity")
            .field("conversation", &self.conversation)
            .field("generation", &self.generation)
            .field("connection", &self.connection)
            .finish_non_exhaustive()
    }
}

pub type TermId = u32;
pub type WsId = u32;
pub type TabId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SessionKey {
    Pane(TermId),
    Conversation(crate::conversation::ConversationId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: crate::conversation::ConversationId,
    pub generation: u64,
    pub provider: crate::conversation::Provider,
    #[serde(default)]
    pub execution: crate::conversation::ExecutionKind,
    pub cwd: PathBuf,
    pub worktree: Option<PathBuf>,
    pub state: crate::conversation::RecoveryState,
    pub status: Status,
    pub native_term: Option<TermId>,
    pub checks: Vec<CheckInfo>,
    /// Your name for it, shared by every client.
    #[serde(default)]
    pub title: Option<String>,
    /// Put away: hidden from the lists, still running.
    #[serde(default)]
    pub archived: bool,
    /// The main checkout of the repository the conversation works in, so a chat session sits with its
    /// project in a list (ADR 0003). Absent outside git.
    #[serde(default)]
    pub root: Option<PathBuf>,
    /// The branch checked out there.
    #[serde(default)]
    pub branch: Option<String>,
}

/// Which app a client is. A terminal session belongs to the app that started it, and only
/// that app may list, type into or resize it (ADR 0004). Clients that don't say are terminal
/// clients, which is what the terminal app has always been.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    #[default]
    Terminal,
    Desktop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ClientMsg {
    /// First frame of every connection. `attach` clients get live output and, with `replays`,
    /// every pane's replay up front; one that asks for panes as it shows them (the desktop
    /// window) sends `WantReplay` instead. `from`: the pane this request comes from (its id and
    /// secret), for a command run inside a pane (an agent); the server checks what that pane may do.
    Hello {
        version: u32,
        attach: bool,
        #[serde(default)]
        from: Option<(TermId, String)>,
        #[serde(default)]
        structured: Option<StructuredIdentity>,
        #[serde(default)]
        surface: Surface,
        #[serde(default = "replays_by_default")]
        replays: bool,
    },
    Command(Command),
    Input {
        term: TermId,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    /// An attached client wants one pane's replay now, on this connection, in order with its
    /// output.
    WantReplay {
        term: TermId,
    },
    Resize {
        term: TermId,
        cols: u16,
        rows: u16,
    },
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
        /// The question a permission prompt asks, from the hook's own payload (a
        /// `PermissionRequest`). The daemon cleans it and decides `answerable` itself.
        #[serde(default)]
        ask: Option<PermissionAsk>,
        /// A fingerprint of the tool call that is starting (16 hex characters, from the tool's
        /// name and input), so the daemon can tell the same call repeating. Never the call itself.
        #[serde(default)]
        call: Option<String>,
    },
    /// What an agent in a pane has used (from Claude's status line), with the pane's secret.
    Usage {
        term: TermId,
        token: String,
        usage: Usage,
        limits: Vec<Limit>,
    },
    Query(Query),
}

/// The question an agent's permission prompt asks, as the agent's own hook reported it (never read
/// off the screen, ADR 0003). Only the tool and a one-line subject (the command, the file or the
/// address) are carried: not the content a tool would write, which can hold secrets.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionAsk {
    /// The tool that wants to run: "Bash", "Write", "WebFetch".
    pub tool: String,
    /// What it would run or touch, on one line, with secret-shaped text masked.
    pub detail: String,
    /// The daemon knows the key that answers this agent's prompt, so the chat view offers Approve and Deny.
    /// Set by the daemon; a hook cannot claim it.
    #[serde(default)]
    pub answerable: bool,
    /// The choices of a one-question multiple-choice prompt, in the order the agent numbers them, so
    /// the chat view can offer one button each. Empty for any other prompt.
    #[serde(default)]
    pub choices: Vec<String>,
}

/// What an agent's session has used.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// How full its context is, 0–100.
    pub context: Option<f32>,
    /// What the session has cost so far at list price, in US dollars (Claude's estimate).
    pub cost: Option<f64>,
    /// Tokens the provider says the session has used (what it reports, never an estimate).
    #[serde(default)]
    pub tokens: Option<u64>,
    /// The input and output halves of `tokens`, as the provider reports them.
    #[serde(default)]
    pub tokens_in: Option<u64>,
    #[serde(default)]
    pub tokens_out: Option<u64>,
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
    ConversationStart {
        target: crate::conversation::StartTarget,
        /// Start in a new checkout of the project at the target's folder, on this branch (a chat
        /// session on a branch, ADR 0003). Only a new conversation takes one. A peer that
        /// predates it would start in the main checkout instead, which is why the protocol
        /// version was raised with it.
        #[serde(default)]
        branch: Option<String>,
    },
    ConversationSend {
        id: crate::conversation::ConversationId,
        generation: u64,
        operation: u64,
        text: String,
    },
    /// Take a message back out of the daemon's queue before it is delivered.
    ConversationDequeue {
        id: crate::conversation::ConversationId,
        generation: u64,
        operation: u64,
    },
    ConversationApprove {
        id: crate::conversation::ConversationId,
        generation: u64,
        request_id: crate::conversation::ProviderRequestId,
        decision: crate::conversation::ApprovalDecision,
    },
    ConversationAnswerInput {
        id: crate::conversation::ConversationId,
        generation: u64,
        request_id: crate::conversation::ProviderRequestId,
        answers: std::collections::BTreeMap<String, Vec<String>>,
    },
    ConversationCancel {
        id: crate::conversation::ConversationId,
        generation: u64,
    },
    /// Stop a chat conversation's agent. Its record stays, stopped, so history is kept.
    ConversationEnd {
        id: crate::conversation::ConversationId,
        generation: u64,
    },
    /// Name a conversation for every client (empty: automatic).
    ConversationRename {
        id: crate::conversation::ConversationId,
        title: String,
    },
    /// Type into a terminal pane (human clients only).
    TerminalInput {
        term: TermId,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    /// Resize a terminal pane (human clients only).
    TerminalResize {
        term: TermId,
        cols: u16,
        rows: u16,
    },
    /// Put a conversation away or bring it back; it keeps running.
    ConversationArchive {
        id: crate::conversation::ConversationId,
        archived: bool,
    },
    ConversationTerminalInput {
        id: crate::conversation::ConversationId,
        generation: u64,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    ConversationTerminalResize {
        id: crate::conversation::ConversationId,
        generation: u64,
        cols: u16,
        rows: u16,
    },
    NewWorkspace {
        cwd: Option<PathBuf>,
        name: Option<String>,
        cmd: Option<String>,
    },
    CloseWorkspace {
        ws: WsId,
    },
    RenameWorkspace {
        ws: WsId,
        name: String,
    },
    SelectWorkspace {
        ws: WsId,
    },
    NewTab {
        ws: WsId,
        name: Option<String>,
        cmd: Option<String>,
    },
    CloseTab {
        ws: WsId,
        tab: TabId,
    },
    RenameTab {
        ws: WsId,
        tab: TabId,
        name: String,
    },
    SelectTab {
        ws: WsId,
        tab: TabId,
    },
    /// `term` is one of `owner`'s tabs (a session's second tab, a split in it); None: its own.
    /// Kept across restarts, so a session's tabs come back as its tabs.
    SetTabOf {
        term: TermId,
        owner: Option<TermId>,
    },
    /// `cwd`: where the new pane starts (default: where the split pane is).
    Split {
        term: TermId,
        dir: Dir,
        cmd: Option<String>,
        cwd: Option<PathBuf>,
    },
    ClosePane {
        term: TermId,
    },
    /// End a session with an undo window: it leaves the snapshot at once but its process runs
    /// on until `UNDO_WINDOW` passes, then it is closed as `ClosePane` closes it.
    CloseSession {
        term: TermId,
    },
    /// Bring back a session `CloseSession` hid, still inside the window. After the window, or
    /// for a session that isn't closing, it does nothing.
    UndoClose {
        term: TermId,
    },
    /// Focus a pane anywhere: switches workspace and tab as needed.
    FocusPane {
        term: TermId,
    },
    /// Focus a pane and bring the windows showing the app to the front (a notification for it
    /// was clicked).
    Reveal {
        term: TermId,
    },
    /// You've seen a finished agent (the cursor rested on it): done → idle.
    MarkSeen {
        term: TermId,
    },
    /// Name a pane yourself (empty: back to the automatic name).
    RenamePane {
        term: TermId,
        name: String,
    },
    /// Keep a pane at the top of its project (and its lead), or let it go back to its place.
    PinPane {
        term: TermId,
        pinned: bool,
    },
    /// Put a pane away from the project lists and tabs without stopping it, or bring it back.
    ArchivePane {
        term: TermId,
        archived: bool,
    },
    /// The agent in `term` moves into a new worktree of its repo: made now, and when its turn
    /// ends it restarts there, resumed (the agent runs `<app> worktree --move`).
    MoveToWorktree {
        term: TermId,
        branch: Option<String>,
    },
    /// Grow (positive) or shrink the pane along `dir`'s axis by `delta` (fraction of parent).
    ResizePane {
        term: TermId,
        dir: Dir,
        delta: f32,
    },
    /// Trade the places of two panes of one tab; each takes the other's size. An error when
    /// they are in different tabs.
    /// Put the checkout `term` is in back as it was in a snapshot the daemon took (one that
    /// `RepoRequest::Snapshots` listed), after saving what is there now. Only you can, and not
    /// while the pane's agent works. The answer is `Reply::Repo { tag, Restored }`.
    RestoreSnapshot {
        term: TermId,
        tag: u64,
        commit: String,
    },
    SwapPanes {
        a: TermId,
        b: TermId,
    },
    /// `git worktree add` next to the workspace's repo, then open it as a new workspace.
    /// `split`: open the worktree as a pane beside this one instead of as a workspace.
    /// `from`: the repo to branch from (default: the workspace's folder).
    /// `agent` / `prompt`: start a known agent preset in it with a first prompt, in place of
    /// `cmd` (naming both is an error). The daemon checks both and quotes the prompt itself.
    NewWorktree {
        ws: WsId,
        branch: String,
        base: Option<String>,
        cmd: Option<String>,
        split: Option<TermId>,
        from: Option<PathBuf>,
        #[serde(default)]
        agent: Option<String>,
        #[serde(default)]
        prompt: Option<String>,
    },
    /// From a pane about to start an agent in `dir`: a new worktree for it when `dir` is
    /// in a repo's main checkout (and per-agent worktrees are on). Replies with the folder
    /// to start in, or empty text to start where it is.
    AgentWorktree {
        dir: PathBuf,
    },
    /// Put a pane in a sidebar group (None: take it out).
    SetGroup {
        ws: WsId,
        group: Option<String>,
    },
    /// Move a pane into another group (`to`), or into a new group named `name`.
    MovePane {
        term: TermId,
        to: Option<WsId>,
        name: Option<String>,
    },
    /// Put back the pane the last automatic workspace took.
    UndoAutoWorkspace,
    /// Close everything running in a linked worktree, remove it and delete its branch (a
    /// branch just merged).
    CloseWorktree {
        path: PathBuf,
    },
    /// Remove these worktrees, none of which has anything running in it: each a folder and
    /// whether to go ahead with uncommitted changes in it (they are lost). A branch is kept
    /// unless it's already merged.
    TidyWorktrees {
        items: Vec<(PathBuf, bool)>,
    },
    /// Close a worktree workspace and `git worktree remove` its checkout.
    /// `delete_branch`: also delete the worktree's branch (discarding a task).
    RemoveWorktree {
        ws: WsId,
        force: bool,
        delete_branch: bool,
    },
    /// Make a pane its own workspace, homed at the pane's current directory. A workspace's
    /// only pane re-homes the workspace instead.
    PaneToWorkspace {
        term: TermId,
    },
    /// Index into the configured workspace colour palette.
    SetWorkspaceColor {
        ws: WsId,
        color: u8,
    },
    ReloadConfig,
    /// Change one shared setting in config.toml (human clients only). The daemon checks the
    /// value, edits the file in place, reloads, and tells every client `SettingsChanged`.
    SetSetting {
        key: crate::settings_view::SettingKey,
        value: crate::settings_view::SettingValue,
    },
    /// The program running in `term` is an agent: add it to config (by its program, or by
    /// its script when node/python runs it) so its status shows.
    TeachAgent {
        term: TermId,
    },
    /// Run a command in a floating pane over everything; it closes when the command exits.
    Popup {
        cmd: String,
        cwd: Option<PathBuf>,
    },
    /// An agent asks you a question with these answers; the reply (the chosen answer) comes
    /// once you pick one.
    AskHuman {
        term: TermId,
        text: String,
        options: Vec<String>,
    },
    /// Your answer (index into its options) to question `id`.
    AnswerHuman {
        id: u64,
        choice: usize,
    },
    /// What a pane may do through the app (read, write, start, respond, admin); None goes back
    /// to the default from config. Only you can (not from inside a pane).
    Grant {
        term: TermId,
        grants: Option<Vec<String>>,
    },
    /// A follow-up for an agent that's working: typed into it when its turn is over.
    QueueFollowUp {
        term: TermId,
        text: String,
    },
    /// Drop the follow-ups queued for a pane.
    ClearQueue {
        term: TermId,
    },
    /// Run the pane's checkpoint `name` again (None: all of them). While its agent is working
    /// the run waits for the turn to end.
    RerunCheck {
        term: TermId,
        name: Option<String>,
    },
    /// Accept a failed checkpoint as it is (None: the headline one); `reason` may be empty.
    /// Covers that one result only.
    OverrideCheck {
        term: TermId,
        name: Option<String>,
        reason: String,
    },
    /// Take an override back (None: the headline checkpoint): it reads as failed again.
    UndoOverride {
        term: TermId,
        name: Option<String>,
    },
    /// Your answer to a repo-supplied check command waiting for approval. Only you can (not
    /// from inside a pane). `command_sha256` (`command_hash`) is of the command you were
    /// shown: the answer is refused if a different one waits by then.
    ApproveCheck {
        term: TermId,
        name: String,
        choice: ApprovalChoice,
        command_sha256: String,
    },
    /// Drop a remembered Always or Never for a repo's checkpoint. Only you can.
    ForgetCheckApproval {
        project: PathBuf,
        name: String,
    },
    /// Your answer to a session that asked to start helpers (`TermInfo::helper_ask`). Only you
    /// can (not from inside a pane), and only while the request is pending.
    AnswerHelpers {
        term: TermId,
        choice: HelperChoice,
    },
    RequestSecret {
        term: TermId,
        label: String,
        reason: String,
    },
    AnswerSecret {
        id: u64,
        value: Option<SecretText>,
    },
    LinkPr {
        term: TermId,
        url: Option<String>,
    },
    SpendSecret {
        reference: String,
    },
    /// `forget`: also discard the saved session, so the next start is a clean slate.
    KillServer {
        forget: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Query {
    /// The shared settings; answered with `Reply::Settings`.
    Settings,
    ConversationRead {
        id: crate::conversation::ConversationId,
        cursor: Option<crate::conversation::history::Cursor>,
        limit: usize,
    },
    ConversationDiff {
        id: crate::conversation::ConversationId,
        generation: u64,
    },
    ConversationFiles {
        id: crate::conversation::ConversationId,
        generation: u64,
        path: Option<String>,
    },
    ConversationFileRead {
        id: crate::conversation::ConversationId,
        generation: u64,
        path: String,
    },
    ConversationTerminalScreen {
        id: crate::conversation::ConversationId,
        generation: u64,
    },
    ConversationTerminalReplay {
        id: crate::conversation::ConversationId,
        generation: u64,
    },
    /// A terminal pane's visible screen as ANSI text (human clients only).
    TerminalScreen {
        term: TermId,
    },
    /// A terminal pane's recent raw output, for scrollback (human clients only).
    TerminalReplay {
        term: TermId,
    },
    /// A read-only question about the repository `term` is in (human clients only). The
    /// answer echoes `tag`, so several can be in flight at once.
    Repo {
        term: TermId,
        tag: u64,
        request: crate::repo::RepoRequest,
    },
    /// Where `query` appears in what the panes this app owns have printed recently (human
    /// clients only; at most 100 hits, and 20 from any one pane).
    PaneSearch {
        query: String,
    },
    /// Provider-reported usage over a period, per session and per day (human clients only).
    UsageReport {
        period: crate::usage_report::UsagePeriod,
    },
    /// The folders inside `path` on this daemon's computer, for a client choosing where to start a
    /// session (human clients only). `path` is absolute or starts with `~`. The answer echoes
    /// `tag`, so a client typing a path can tell which of its questions this answers.
    ListFolders {
        path: String,
        tag: u64,
    },
    List,
    /// Plain-text screen contents of a terminal (the visible screen).
    Read {
        term: TermId,
    },
    /// Every worktree of the repository a workspace is in.
    Worktrees {
        ws: WsId,
    },
    /// The tail of a pane's last output for checkpoint `name` (None: the headline one); the
    /// snapshot carries only a summary.
    CheckOutput {
        term: TermId,
        name: Option<String>,
    },
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
        matches!(
            self,
            Self::Starting | Self::Working | Self::WaitingForInput | Self::AwaitingApproval
        )
    }
}

fn replays_by_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    Welcome {
        version: u32,
    },
    State(Snapshot),
    /// Only some panes changed since the last `State` or `TermsChanged` on this connection:
    /// replace these and drop those. Everything else in the client's snapshot stands.
    TermsChanged {
        changed: Vec<TermInfo>,
        removed: Vec<TermId>,
    },
    /// Sent per terminal on attach (or on `WantReplay`): the recent output ring, to rebuild
    /// scrollback.
    Replay {
        term: TermId,
        cols: u16,
        rows: u16,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    Output {
        term: TermId,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    /// An agent needs attention (blocked, or finished while unfocused).
    Attention {
        term: TermId,
        status: Status,
    },
    Reply(Reply),
    Error(String),
    /// Informational message for the status bar.
    Notice(String),
    /// You closed the last pane of a worktree that holds commits found nowhere else: ask
    /// whether its folder goes too (its branch stays either way).
    AskRemoveWorktree {
        path: PathBuf,
        name: String,
        why: String,
    },
    /// A program in a pane copied text (OSC 52): put it on the user's clipboard.
    Clipboard {
        term: TermId,
        text: String,
    },
    /// An agent started in a new repo, which became a workspace (undoable).
    AutoWorkspace(String),
    /// A shared setting changed (by either app); the settings as they are now.
    SettingsChanged(crate::settings_view::SharedSettings),
    /// Come to the front: a notification for one of this server's sessions was clicked.
    Raise,
    Bye,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Reply {
    Settings(crate::settings_view::SharedSettings),
    ConversationStarted {
        id: crate::conversation::ConversationId,
        generation: u64,
    },
    ConversationSent(crate::conversation::Receipt),
    /// The conversation was busy, so the message waits in the daemon's queue (see `ReadResult::queued`).
    ConversationQueued(crate::conversation::QueuedReceipt),
    ConversationRead(crate::conversation::ReadResult),
    ConversationDiff(crate::conversation::ConversationDiff),
    ConversationFiles(Vec<crate::conversation::ConversationFileEntry>),
    ConversationFile(String),
    TerminalReplay(#[serde(with = "serde_bytes")] Vec<u8>),
    Ok,
    List(Snapshot),
    Text(String),
    Worktrees(Vec<WorktreeEntry>),
    Repo {
        tag: u64,
        reply: crate::repo::RepoReply,
    },
    Search(Vec<SearchHit>),
    UsageReport(crate::usage_report::UsageReport),
    Folders {
        tag: u64,
        listing: FolderListing,
    },
}

/// The most folders one listing carries; a folder with more says so in `truncated`.
pub const FOLDERS_MAX: usize = 1000;

/// What `Query::ListFolders` found: the folders directly inside a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderListing {
    /// The folder, absolute with `/` separators.
    pub path: String,
    /// False when there is no such folder or it cannot be read; `entries` is then empty.
    pub exists: bool,
    pub entries: Vec<FolderEntry>,
    pub truncated: bool,
}

impl FolderListing {
    /// A listing of a folder that is not there.
    pub fn missing(path: &str) -> FolderListing {
        FolderListing {
            path: path.to_string(),
            exists: false,
            entries: Vec::new(),
            truncated: false,
        }
    }
}

/// One folder inside a listed folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderEntry {
    pub name: String,
    /// The branch it is on when it is a git checkout (a short commit for a detached HEAD).
    pub git: Option<String>,
    /// Whether it holds a folder worth browsing into (hidden ones do not count).
    pub has_children: bool,
}

/// One line of a pane's output that matched a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub term: TermId,
    /// Counted from the start of what the daemon kept, from 1.
    pub line: usize,
    pub text: String,
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
    pub conversations: Vec<ConversationSummary>,
    #[serde(default)]
    pub daemon_version: String,
    /// Which daemon this is, random and kept across its restarts. Lets a window tell that a
    /// different machine now answers at a saved name. `None` from a daemon built before it;
    /// not a credential.
    #[serde(default)]
    pub daemon_id: Option<String>,
    pub workspaces: Vec<WorkspaceInfo>,
    pub active_ws: Option<WsId>,
    pub terms: BTreeMap<TermId, TermInfo>,
    /// Questions agents asked you (`<app> ask-human`), waiting for an answer.
    #[serde(default)]
    pub questions: Vec<HumanQuestion>,
    #[serde(default)]
    pub secret_requests: Vec<SecretRequest>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretRequest {
    pub id: u64,
    pub term: TermId,
    pub label: String,
    pub reason: String,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretText(pub String);

impl std::fmt::Debug for SecretText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretText(<hidden>)")
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u32,
    pub url: String,
    pub title: String,
    pub state: PrState,
    pub draft: bool,
    pub checks: PrChecks,
    pub review: PrReview,
    pub conflicting: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrTone {
    Bad,
    Waiting,
    Good,
    Plain,
}
impl PrInfo {
    pub fn headline(&self) -> String {
        let fails = self.checks.failed.len();
        let what = match self.state {
            PrState::Merged => "merged".to_string(),
            PrState::Closed => "closed".to_string(),
            PrState::Open if fails > 0 => format!("{fails} failing"),
            PrState::Open if self.conflicting => "conflict".into(),
            PrState::Open if self.review == PrReview::ChangesRequested => {
                "changes requested".into()
            }
            PrState::Open if self.checks.pending > 0 => "checking".into(),
            PrState::Open if self.review == PrReview::Approved => "approved".into(),
            PrState::Open if self.checks.passed > 0 => "passing".into(),
            PrState::Open => String::new(),
        };
        let draft = if self.draft && self.state == PrState::Open {
            " draft"
        } else {
            ""
        };
        format!(
            "#{}{}{what}{draft}",
            self.number,
            if what.is_empty() { "" } else { " " }
        )
    }
    pub fn tone(&self) -> PrTone {
        match self.state {
            PrState::Merged => PrTone::Good,
            PrState::Closed => PrTone::Bad,
            PrState::Open
                if !self.checks.failed.is_empty()
                    || self.conflicting
                    || self.review == PrReview::ChangesRequested =>
            {
                PrTone::Bad
            }
            PrState::Open if self.checks.pending > 0 => PrTone::Waiting,
            PrState::Open if self.review == PrReview::Approved || self.checks.passed > 0 => {
                PrTone::Good
            }
            PrState::Open => PrTone::Plain,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PrState {
    #[default]
    Open,
    Merged,
    Closed,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PrChecks {
    pub passed: u32,
    pub pending: u32,
    pub failed: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PrReview {
    #[default]
    None,
    Required,
    Approved,
    ChangesRequested,
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
    #[serde(default)]
    pub conversation_id: Option<crate::conversation::ConversationId>,
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
    /// Pinned to the top of its project.
    #[serde(default)]
    pub pinned: bool,
    /// Put away from the project lists; the process keeps running.
    #[serde(default)]
    pub archived: bool,
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
    /// The last lines of text on the pane when its turn ended, for an agent whose hooks
    /// reported no last message. Empty while `said` has one.
    #[serde(default)]
    pub last_lines: String,
    /// The git branch the pane is on, if it's in a repo.
    pub branch: Option<String>,
    /// The pane is in a linked worktree.
    pub linked: bool,
    /// Lines added and removed in its checkout against HEAD (untracked files not counted); zero
    /// outside git. Shared by every pane in the same checkout.
    #[serde(default)]
    pub diff_added: u32,
    #[serde(default)]
    pub diff_removed: u32,
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
    /// The session whose tab this pane is in, when not its own (`Command::SetTabOf`).
    #[serde(default)]
    pub tab_of: Option<TermId>,
    /// The helpers this session asked to start and is waiting on you for, one task each. Not
    /// empty means the row needs you ("wants to start N helpers"); `AnswerHelpers` answers it.
    #[serde(default)]
    pub helper_ask: Vec<String>,
    #[serde(default)]
    pub pr: Option<PrInfo>,
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
    /// Working, and the agent's last calls are one call over and over or two taking turns: how
    /// many calls the repeat has reached. Its status stays working; nothing rings.
    #[serde(default)]
    pub repeats: Option<u32>,
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
    /// What a stopped agent is asking, off its screen: the exact command or file and its choices.
    /// Empty unless the pane needs you and shows numbered choices.
    #[serde(default)]
    pub asking: String,
    /// The agent that was mid-turn when the daemon restarted, left as a plain shell for you to
    /// resume. Never resumed on its own; gone once an agent runs in the pane again.
    #[serde(default)]
    pub interrupted: Option<Interrupted>,
    /// The latest turn events, oldest first: what the agent's own reports and the shell's prompt
    /// marks (OSC 133) said happened, not what the screen looked like.
    #[serde(default)]
    pub timeline: Vec<TurnEvent>,
    /// The permission prompt the pane is stopped on, from its agent's own hook; `None` unless the
    /// pane needs you and the agent said what it asks.
    #[serde(default)]
    pub ask: Option<PermissionAsk>,
    /// How the last command in the pane ended, when it ended in failure. See [`ExitInfo`].
    #[serde(default)]
    pub exit: Option<ExitInfo>,
}

/// A command in a pane that ended in failure, as the shell reported it (OSC 133 D), never read
/// off the screen. Set only for a non-zero exit, so `Some` means "show the red exited-with-error
/// mark" and `None` means nothing failed: a running pane, a clean exit (code 0, which reads as
/// Done, not as a problem) or a shell that reported no code. It is cleared as soon as the next
/// command starts or the agent works again, and it is not kept across a daemon restart (the
/// restored pane is a fresh shell). A signal kill reaches the shell as a code above 128
/// (130 for Ctrl+C, 137 for SIGKILL), so it counts as an error too. Clients display it; only
/// the daemon sets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitInfo {
    /// The exit code the shell reported; `None` if a failure carried no code.
    pub code: Option<i32>,
    /// When it ended (unix seconds).
    pub at: u64,
}

/// An agent that was working when the daemon restarted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interrupted {
    pub agent: String,
    /// The command that brings its conversation back (`codex resume --last`).
    pub resume: String,
}

/// One thing that happened in a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnEvent {
    /// Unix seconds.
    pub at: u64,
    pub kind: TurnKind,
    /// What reported it ("hook: Stop", "exit 1"); may be empty.
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnKind {
    /// The agent started working.
    Working,
    /// The agent stopped for you.
    NeedsYou,
    /// The agent finished its turn.
    Done,
    /// The agent is idle again.
    Idle,
    /// The shell showed a prompt (OSC 133 A).
    Prompt,
    /// A command started running (OSC 133 C).
    CommandStarted,
    /// A command finished (OSC 133 D); `detail` is its exit code when it reported one.
    CommandFinished,
}

pub fn one_or_many_checks<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<CheckInfo>, D::Error> {
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
    Sha256::digest(command.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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
    Repo {
        file: String,
        commit: Option<String>,
        committed_at: Option<u64>,
        uncommitted: bool,
    },
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
        let path_like =
            self.title.contains('\\') || self.title.contains('/') || self.title.ends_with(".exe");
        if !self.title.is_empty() && !path_like {
            return self.title.clone();
        }
        self.process.clone()
    }

    /// The pane is at a shell prompt (nothing else running in it).
    pub fn is_shell(&self) -> bool {
        matches!(
            self.process.to_ascii_lowercase().as_str(),
            "pwsh"
                | "powershell"
                | "cmd"
                | "bash"
                | "zsh"
                | "fish"
                | "nu"
                | "sh"
                | "dash"
                | "elvish"
                | "xonsh"
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
            w.tabs
                .iter()
                .find(|t| t.layout.contains(term))
                .map(|t| (w, t))
        })
    }
}

impl WorkspaceInfo {
    pub fn tab(&self) -> Option<&TabInfo> {
        self.tabs
            .iter()
            .find(|t| t.id == self.active_tab)
            .or(self.tabs.first())
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
            ClientMsg::Hello {
                version: PROTOCOL_VERSION,
                attach: true,
                from: Some((3, "00ff".into())),
                structured: None,
                surface: Surface::Terminal,
                replays: true,
            },
            ClientMsg::Input {
                term: 3,
                data: b"ls\r".to_vec(),
            },
            ClientMsg::WantReplay { term: 3 },
            ClientMsg::Resize {
                term: 3,
                cols: 120,
                rows: 40,
            },
            ClientMsg::Hook {
                term: 3,
                agent: "claude".into(),
                status: HookStatus::Blocked,
                session: Some("abc-123".into()),
                cwd: Some(PathBuf::from("/code/api")),
                prompt: Some("fix it".into()),
                said: Some("done".into()),
                subagent: Some(Subagent {
                    id: "s1".into(),
                    kind: "Explore".into(),
                    start: true,
                    reported_at: 0,
                    task: None,
                    model: None,
                    effort: None,
                    activity: None,
                }),
                event: "PreToolUse".into(),
                pid: 42,
                token: "00ff".into(),
                transcript: Some(PathBuf::from("/t.jsonl")),
                model: Some("opus".into()),
                name: Some("api work".into()),
                background: vec!["b1".into()],
                ask: Some(PermissionAsk {
                    tool: "Bash".into(),
                    detail: "cargo test".into(),
                    answerable: true,
                    choices: vec![],
                }),
                call: Some("00112233445566ff".into()),
            },
            ClientMsg::Query(Query::List),
            ClientMsg::Query(Query::Read { term: 3 }),
            ClientMsg::Query(Query::Worktrees { ws: 1 }),
            ClientMsg::Query(Query::PaneSearch {
                query: "error".into(),
            }),
            ClientMsg::Query(Query::Repo {
                term: 3,
                tag: 9,
                request: crate::repo::RepoRequest::DiffRange {
                    from: "main".into(),
                    to: "HEAD".into(),
                    path: Some("src/lib.rs".into()),
                },
            }),
            ClientMsg::Query(Query::CheckOutput {
                term: 3,
                name: None,
            }),
            ClientMsg::Query(Query::CheckOutput {
                term: 3,
                name: Some("build".into()),
            }),
            ClientMsg::Query(Query::ConversationTerminalReplay {
                id: crate::conversation::ConversationId(47),
                generation: 2,
            }),
            ClientMsg::Command(Command::NewWorkspace {
                cwd: Some(PathBuf::from("/code")),
                name: None,
                cmd: Some("claude".into()),
            }),
            ClientMsg::Command(Command::Split {
                term: 3,
                dir: crate::layout::Dir::Down,
                cmd: None,
                cwd: None,
            }),
            ClientMsg::Command(Command::ConversationTerminalResize {
                id: crate::conversation::ConversationId(47),
                generation: 2,
                cols: 96,
                rows: 32,
            }),
            ClientMsg::Command(Command::ClosePane { term: 3 }),
            ClientMsg::Command(Command::Reveal { term: 3 }),
            ClientMsg::Command(Command::TeachAgent { term: 3 }),
            ClientMsg::Command(Command::Popup {
                cmd: "fzf".into(),
                cwd: None,
            }),
            ClientMsg::Command(Command::AskHuman {
                term: 3,
                text: "Deploy?".into(),
                options: vec!["Yes".into(), "No".into()],
            }),
            ClientMsg::Command(Command::AnswerHuman { id: 1, choice: 0 }),
            ClientMsg::Command(Command::Grant {
                term: 3,
                grants: Some(vec!["read".into()]),
            }),
            ClientMsg::Command(Command::QueueFollowUp {
                term: 3,
                text: "then run the tests".into(),
            }),
            ClientMsg::Command(Command::ClearQueue { term: 3 }),
            ClientMsg::Command(Command::RerunCheck {
                term: 3,
                name: None,
            }),
            ClientMsg::Command(Command::RerunCheck {
                term: 3,
                name: Some("test".into()),
            }),
            ClientMsg::Command(Command::OverrideCheck {
                term: 3,
                name: Some("test".into()),
                reason: "flaky on CI".into(),
            }),
            ClientMsg::Command(Command::UndoOverride {
                term: 3,
                name: None,
            }),
            ClientMsg::Command(Command::ApproveCheck {
                term: 3,
                name: "test".into(),
                choice: ApprovalChoice::Once,
                command_sha256: command_hash("npm test"),
            }),
            ClientMsg::Command(Command::ApproveCheck {
                term: 3,
                name: "test".into(),
                choice: ApprovalChoice::Always,
                command_sha256: command_hash("npm test"),
            }),
            ClientMsg::Command(Command::ApproveCheck {
                term: 3,
                name: "test".into(),
                choice: ApprovalChoice::Never,
                command_sha256: command_hash("npm test"),
            }),
            ClientMsg::Command(Command::ForgetCheckApproval {
                project: PathBuf::from("/code/api"),
                name: "test".into(),
            }),
            ClientMsg::Command(Command::AnswerHelpers {
                term: 3,
                choice: HelperChoice::Allow,
            }),
            ClientMsg::Command(Command::AnswerHelpers {
                term: 3,
                choice: HelperChoice::Always,
            }),
            ClientMsg::Command(Command::AnswerHelpers {
                term: 3,
                choice: HelperChoice::No,
            }),
            ClientMsg::Command(Command::TidyWorktrees {
                items: vec![(PathBuf::from("/code/.wt/rate"), false)],
            }),
            ClientMsg::Command(Command::RenamePane {
                term: 3,
                name: "x".into(),
            }),
            ClientMsg::Command(Command::MoveToWorktree {
                term: 3,
                branch: Some("feat/x".into()),
            }),
            ClientMsg::Command(Command::SwapPanes { a: 3, b: 4 }),
            ClientMsg::Command(Command::RestoreSnapshot {
                term: 3,
                tag: 4,
                commit: "abc123".into(),
            }),
        ];
        for m in &clients {
            round_trips(m);
        }
        let servers = vec![
            ServerMsg::Welcome {
                version: PROTOCOL_VERSION,
            },
            ServerMsg::State(Snapshot::default()),
            ServerMsg::TermsChanged {
                changed: Vec::new(),
                removed: vec![3],
            },
            ServerMsg::Replay {
                term: 3,
                cols: 80,
                rows: 24,
                data: vec![27, b'[', b'H'],
            },
            ServerMsg::Output {
                term: 3,
                data: b"hello".to_vec(),
            },
            ServerMsg::Attention {
                term: 3,
                status: Status::Done,
            },
            ServerMsg::Reply(Reply::Ok),
            ServerMsg::Reply(Reply::Text("screen".into())),
            ServerMsg::Reply(Reply::Folders {
                tag: 7,
                listing: FolderListing {
                    path: "/home/me/code".into(),
                    exists: true,
                    entries: vec![FolderEntry {
                        name: "maitty".into(),
                        git: Some("dev".into()),
                        has_children: true,
                    }],
                    truncated: false,
                },
            }),
            ServerMsg::Reply(Reply::TerminalReplay(vec![27, b'[', b'H'])),
            ServerMsg::Error("nope".into()),
            ServerMsg::Notice("saved".into()),
            ServerMsg::AskRemoveWorktree {
                path: PathBuf::from("/w/task"),
                name: "task".into(),
                why: "not merged anywhere".into(),
            },
            ServerMsg::Clipboard {
                term: 3,
                text: "copied".into(),
            },
            ServerMsg::AutoWorkspace("api".into()),
            ServerMsg::Raise,
            ServerMsg::Bye,
        ];
        for m in &servers {
            round_trips(m);
        }
    }

    #[test]
    fn conversation_summary_from_an_older_daemon_defaults_to_structured_execution() {
        let summary: ConversationSummary = serde_json::from_value(serde_json::json!({
            "id": 7,
            "generation": 3,
            "provider": "Codex",
            "cwd": "/code",
            "worktree": null,
            "state": "Running",
            "status": "Idle",
            "native_term": null,
            "checks": []
        }))
        .expect("older summary without execution still decodes");

        assert_eq!(
            summary.execution,
            crate::conversation::ExecutionKind::Structured
        );
        // `root` and `branch` (a chat session's project, ADR 0003) are defaulted too.
        assert_eq!((summary.root, summary.branch), (None, None));
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
        let Command::SwapPanes { a, b } = decode(&bytes).unwrap() else {
            panic!("expected a swap")
        };
        assert_eq!((a, b), (3, 4));
    }

    /// Removing a queued message and the reply to a queued send are a new command and a new reply:
    /// a peer built before them cannot read either, which is why protocol 71 refuses protocol 70.
    #[test]
    fn a_peer_from_before_the_send_queue_cannot_read_its_messages() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        enum OldCommand {
            ConversationCancel { id: u64, generation: u64 },
        }
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        enum OldReply {
            Ok,
        }
        let id = crate::conversation::ConversationId(3);
        let dequeue = encode(&Command::ConversationDequeue { id, generation: 1, operation: 4 }).unwrap();
        assert!(decode::<OldCommand>(&dequeue).is_err());
        assert!(matches!(
            decode::<Command>(&dequeue).unwrap(),
            Command::ConversationDequeue { operation: 4, .. }
        ));
        let queued = Reply::ConversationQueued(crate::conversation::QueuedReceipt {
            conversation: id,
            generation: 1,
            operation: 4,
        });
        assert!(decode::<OldReply>(&encode(&queued).unwrap()).is_err());
    }

    /// `queued` was added to the read as a defaulted field: a reply from before it still decodes,
    /// as an empty queue.
    #[test]
    fn a_conversation_read_from_before_the_send_queue_still_decodes() {
        let read: crate::conversation::ReadResult = serde_json::from_value(serde_json::json!({
            "conversation": 3, "generation": 1, "status": "Idle", "receipt": null,
            "history": { "entries": [], "next": null, "completeness": "Complete" },
            "error": null, "checks": [], "approvals": []
        }))
        .expect("a read from before the queue");
        assert!(read.queued.is_empty() && read.history.resume.is_none() && !read.history.reset);
    }

    /// `agent` and `prompt` were added to `NewWorktree` as defaulted fields: a peer that does not
    /// send them still decodes, as "no agent", so the version stays as it was.
    #[test]
    fn a_worktree_request_from_before_agent_and_prompt_still_decodes() {
        #[derive(Serialize)]
        enum OldCommand {
            NewWorktree {
                ws: WsId,
                branch: String,
                base: Option<String>,
                cmd: Option<String>,
                split: Option<TermId>,
                from: Option<PathBuf>,
            },
        }
        let bytes = encode(&OldCommand::NewWorktree {
            ws: 1,
            branch: "feat/x".into(),
            base: None,
            cmd: Some("claude".into()),
            split: None,
            from: None,
        })
        .unwrap();
        let Command::NewWorktree {
            branch,
            cmd,
            agent,
            prompt,
            ..
        } = decode(&bytes).unwrap()
        else {
            panic!("expected a worktree request")
        };
        assert_eq!(
            (branch.as_str(), cmd.as_deref()),
            ("feat/x", Some("claude"))
        );
        assert_eq!((agent, prompt), (None, None));
    }

    /// `branch` was added to `ConversationStart` as a defaulted field (protocol 72, chat sessions
    /// on a branch): the shape from before it still decodes, as "no branch", and a start that
    /// names one round-trips. The version was raised too, because a peer that predates the field
    /// would drop it and start in the main checkout.
    #[test]
    fn a_conversation_start_from_before_branch_still_decodes() {
        #[derive(Serialize)]
        enum OldCommand {
            ConversationStart {
                target: crate::conversation::StartTarget,
            },
        }
        let target = crate::conversation::StartTarget::Conversation {
            id: crate::conversation::ConversationId(5),
            generation: 2,
        };
        let bytes = encode(&OldCommand::ConversationStart {
            target: target.clone(),
        })
        .unwrap();
        let Command::ConversationStart { target: read, branch } = decode(&bytes).unwrap() else {
            panic!("expected a conversation start")
        };
        assert_eq!((read, branch), (target.clone(), None));

        let bytes = encode(&Command::ConversationStart {
            target,
            branch: Some("chat/one".into()),
        })
        .unwrap();
        assert!(matches!(
            decode::<Command>(&bytes).unwrap(),
            Command::ConversationStart { branch: Some(ref b), .. } if b == "chat/one"
        ));
    }

    /// A `Snapshot` as a daemon built before `daemon_id` encoded it.
    #[derive(Serialize)]
    struct SnapshotWithoutId {
        conversations: Vec<ConversationSummary>,
        daemon_version: String,
        workspaces: Vec<WorkspaceInfo>,
        active_ws: Option<WsId>,
        terms: BTreeMap<TermId, TermInfo>,
    }

    #[test]
    fn a_snapshot_from_a_daemon_without_an_id_decodes_with_none() {
        let old = SnapshotWithoutId {
            conversations: Vec::new(),
            daemon_version: "0.50.0".into(),
            workspaces: Vec::new(),
            active_ws: None,
            terms: BTreeMap::new(),
        };
        let snapshot: Snapshot = decode(&encode(&old).unwrap()).unwrap();
        assert_eq!(snapshot.daemon_id, None);
        assert_eq!(snapshot.daemon_version, "0.50.0");
    }

    #[test]
    fn a_daemon_id_round_trips_without_a_new_protocol_version() {
        assert_eq!(PROTOCOL_VERSION, 74);
        let snapshot = Snapshot {
            daemon_id: Some("0123456789abcdef0123456789abcdef".into()),
            ..Snapshot::default()
        };
        let back: Snapshot = decode(&encode(&snapshot).unwrap()).unwrap();
        assert_eq!(back.daemon_id, snapshot.daemon_id);
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
        round_trips(&ServerMsg::State(Snapshot {
            terms: [(3, term)].into(),
            ..Default::default()
        }));
    }

    #[test]
    fn a_pane_asking_for_helpers_round_trips_and_one_not_asking_has_none() {
        let mut term = pane_from(serde_json::json!({}));
        assert!(
            term.helper_ask.is_empty(),
            "a peer that knows no helper requests"
        );
        term.helper_ask = vec!["fix the login test".into(), "port the footer".into()];
        round_trips(&ServerMsg::State(Snapshot {
            terms: [(3, term)].into(),
            ..Default::default()
        }));
    }

    #[test]
    fn a_pane_from_a_peer_that_reads_no_screens_has_no_last_lines() {
        let mut term = pane_from(serde_json::json!({}));
        assert!(term.last_lines.is_empty(), "an older peer sends none");
        term.last_lines = "All three migrations ran clean.".into();
        round_trips(&ServerMsg::State(Snapshot {
            terms: [(3, term)].into(),
            ..Default::default()
        }));
    }

    #[test]
    fn a_check_from_an_older_peer_fills_in_what_it_lacks() {
        let bytes = rmp_serde::to_vec_named(&BTreeMap::from([("command", "npm test")])).unwrap();
        let info: CheckInfo = decode(&bytes).expect("only the command");
        assert_eq!(
            (info.command.as_str(), info.exit_code, info.rerun_queued),
            ("npm test", None, false)
        );
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
            changed_files: vec![
                ChangedFile {
                    kind: 'M',
                    path: "src/a.rs".into(),
                },
                ChangedFile {
                    kind: 'A',
                    path: "tests/b.rs".into(),
                },
            ],
            overridden_by: "you".into(),
            source: CheckSource::Repo {
                file: ".maitty/checks.toml".into(),
                commit: Some("a1b2c3d".into()),
                committed_at: Some(80),
                uncommitted: true,
            },
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
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        decode(&rmp_serde::to_vec_named(&v).unwrap()).expect("a pane")
    }

    /// A peer from before diff counts sends none: the pane reads as having no changes.
    #[test]
    fn a_pane_from_before_diff_counts_reads_as_zero() {
        let old = pane_from(serde_json::json!({}));
        assert_eq!((old.diff_added, old.diff_removed), (0, 0));
        let new = pane_from(serde_json::json!({"diff_added": 7, "diff_removed": 2}));
        assert_eq!((new.diff_added, new.diff_removed), (7, 2));
    }

    /// Pin and archive are new commands: a peer built before them can't read them.
    #[test]
    fn a_peer_from_before_pin_and_archive_cannot_read_them() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        enum OldCommand {
            RenamePane { term: TermId, name: String },
        }
        for command in [
            Command::PinPane {
                term: 3,
                pinned: true,
            },
            Command::ArchivePane {
                term: 3,
                archived: true,
            },
        ] {
            let bytes = encode(&command).unwrap();
            assert!(decode::<OldCommand>(&bytes).is_err());
            assert!(decode::<Command>(&bytes).is_ok());
        }
    }

    /// Soft close and undo are new commands: a peer built before them can't read them.
    #[test]
    fn a_peer_from_before_soft_close_cannot_read_it() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        enum OldCommand {
            ClosePane { term: TermId },
        }
        for command in [
            Command::CloseSession { term: 3 },
            Command::UndoClose { term: 3 },
        ] {
            let bytes = encode(&command).unwrap();
            assert!(decode::<OldCommand>(&bytes).is_err());
            assert!(decode::<Command>(&bytes).is_ok());
        }
    }

    fn sample_settings() -> crate::settings_view::SharedSettings {
        use crate::settings_view::*;
        SharedSettings {
            default_agent: "claude".into(),
            new_session_view: NewSessionView::Terminal,
            sidebar_side: SidebarSide::Right,
            theme_dark: "night".into(),
            theme_light: "day".into(),
            appearance: Appearance::Light,
            spinner: vec!["a".into(), "b".into()],
            start_dir: "C:\\Users\\me\\code".into(),
            reduce_motion: true,
            window_style: WindowStyle::Glass,
        }
    }

    #[test]
    fn settings_round_trip_through_the_codec() {
        use crate::settings_view::*;
        let settings = sample_settings();
        let bytes = encode(&ServerMsg::Reply(Reply::Settings(settings.clone()))).unwrap();
        match decode::<ServerMsg>(&bytes).unwrap() {
            ServerMsg::Reply(Reply::Settings(back)) => assert_eq!(back, settings),
            other => panic!("{other:?}"),
        }
        let bytes = encode(&ServerMsg::SettingsChanged(settings.clone())).unwrap();
        match decode::<ServerMsg>(&bytes).unwrap() {
            ServerMsg::SettingsChanged(back) => assert_eq!(back, settings),
            other => panic!("{other:?}"),
        }
        round_trips(&ClientMsg::Query(Query::Settings));
        for (key, value) in [
            (SettingKey::ReduceMotion, SettingValue::Bool(true)),
            (SettingKey::Spinner, SettingValue::List(vec!["x".into()])),
            (SettingKey::StartDir, SettingValue::Text("~/code".into())),
            (
                SettingKey::WindowStyle,
                SettingValue::WindowStyle(WindowStyle::Glass),
            ),
        ] {
            let command = Command::SetSetting {
                key,
                value: value.clone(),
            };
            let bytes = encode(&ClientMsg::Command(command)).unwrap();
            match decode::<ClientMsg>(&bytes).unwrap() {
                ClientMsg::Command(Command::SetSetting { key: k, value: v }) => {
                    assert_eq!((k, v), (key, value));
                }
                other => panic!("{other:?}"),
            }
        }
    }

    /// The settings messages are new variants: a peer built before them can't read them.
    #[test]
    fn a_peer_from_before_the_settings_messages_cannot_read_them() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        enum OldCommand {
            RenamePane { term: TermId, name: String },
        }
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        enum OldQuery {
            UsageReport,
        }
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        enum OldServerMsg {
            Bye,
        }
        let command = Command::SetSetting {
            key: crate::settings_view::SettingKey::ReduceMotion,
            value: crate::settings_view::SettingValue::Bool(true),
        };
        let bytes = encode(&command).unwrap();
        assert!(decode::<OldCommand>(&bytes).is_err());
        assert!(decode::<Command>(&bytes).is_ok());
        let bytes = encode(&Query::Settings).unwrap();
        assert!(decode::<OldQuery>(&bytes).is_err());
        assert!(decode::<Query>(&bytes).is_ok());
        let bytes = encode(&ServerMsg::SettingsChanged(sample_settings())).unwrap();
        assert!(decode::<OldServerMsg>(&bytes).is_err());
        assert!(decode::<ServerMsg>(&bytes).is_ok());
    }

    #[test]
    fn a_pane_from_before_pin_and_archive_decodes_as_neither() {
        let term = pane_from(serde_json::json!({}));
        assert!(!term.pinned && !term.archived);
        let term = pane_from(serde_json::json!({ "pinned": true, "archived": true }));
        assert!(term.pinned && term.archived);
    }

    #[test]
    fn a_check_with_every_field_round_trips() {
        round_trips(&every_field_check());
        let mut term = pane_from(serde_json::json!({}));
        term.checks = vec![
            every_field_check(),
            CheckInfo {
                name: "build".into(),
                state: CheckState::Pending,
                ..Default::default()
            },
        ];
        round_trips(&ServerMsg::State(Snapshot {
            terms: [(3, term)].into(),
            ..Default::default()
        }));
    }

    #[test]
    fn a_pane_from_before_the_permission_question_reads_with_none_and_one_with_it_round_trips() {
        let mut term = pane_from(serde_json::json!({}));
        assert_eq!(term.ask, None, "an older peer sends none");
        term.ask = Some(PermissionAsk {
            tool: "Bash".into(),
            detail: "cargo test".into(),
            answerable: true,
            choices: vec!["A".into(), "B".into()],
        });
        round_trips(&ServerMsg::State(Snapshot {
            terms: [(3, term)].into(),
            ..Default::default()
        }));
    }

    #[test]
    fn a_pane_from_before_the_exit_mark_reads_with_none_and_one_with_it_round_trips() {
        let mut term = pane_from(serde_json::json!({}));
        assert_eq!(term.exit, None, "an older peer sends none");
        term.exit = Some(ExitInfo {
            code: Some(137),
            at: 9,
        });
        round_trips(&ServerMsg::State(Snapshot {
            terms: [(3, term)].into(),
            ..Default::default()
        }));
    }

    #[test]
    fn a_pane_from_before_the_repeat_count_reads_with_none_and_one_with_it_round_trips() {
        let mut term = pane_from(serde_json::json!({}));
        assert_eq!(term.repeats, None, "an older peer sends none");
        term.repeats = Some(6);
        round_trips(&ServerMsg::State(Snapshot {
            terms: [(3, term)].into(),
            ..Default::default()
        }));
    }

    #[test]
    fn a_hook_report_reads_across_the_background_field() {
        let old = serde_json::json!({ "Hook": {
            "term": 3, "agent": "claude", "status": "Done", "session": null, "cwd": null, "prompt": null,
            "said": null, "subagent": null, "event": "Stop", "pid": 42, "token": "00ff",
            "transcript": null, "model": null, "name": null,
        }});
        let msg: ClientMsg =
            decode(&rmp_serde::to_vec_named(&old).unwrap()).expect("a hook from before the field");
        let ClientMsg::Hook {
            event,
            background,
            ask,
            call,
            ..
        } = msg
        else {
            panic!("expected a hook report")
        };
        assert_eq!((event.as_str(), background.len(), ask), ("Stop", 0, None));
        assert_eq!(call, None, "an older peer sends no call fingerprint");

        #[derive(Deserialize)]
        enum OldClientMsg {
            Hook {
                term: TermId,
                status: HookStatus,
                event: String,
                pid: u32,
                token: String,
            },
        }
        let new = ClientMsg::Hook {
            term: 3,
            agent: "claude".into(),
            status: HookStatus::Done,
            session: None,
            cwd: None,
            prompt: None,
            said: None,
            subagent: None,
            event: "Stop".into(),
            pid: 42,
            token: "00ff".into(),
            transcript: None,
            model: None,
            name: None,
            background: vec!["b1".into()],
            ask: None,
            call: None,
        };
        let OldClientMsg::Hook {
            term,
            status,
            event,
            pid,
            token,
        } = decode(&encode(&new).unwrap()).expect("an older daemon reads it");
        assert_eq!(
            (term, status, event.as_str(), pid, token.as_str()),
            (3, HookStatus::Done, "Stop", 42, "00ff")
        );
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
        assert_eq!(
            (
                info.name.as_str(),
                info.source,
                info.changed_files.len(),
                info.previous_command
            ),
            ("", CheckSource::User, 0, None)
        );
    }

    #[test]
    fn a_pane_with_the_old_single_check_reads_as_one_checkpoint() {
        let old = pane_from(
            serde_json::json!({ "check": { "command": "npm test", "state": "Failed", "exit_code": 1 } }),
        );
        assert_eq!(
            old.checks
                .iter()
                .map(|c| (c.command.as_str(), c.state))
                .collect::<Vec<_>>(),
            [("npm test", CheckState::Failed)]
        );
        assert!(
            pane_from(serde_json::json!({ "check": null }))
                .checks
                .is_empty()
        );
        assert!(pane_from(serde_json::json!({})).checks.is_empty());
    }

    #[test]
    fn the_headline_is_the_worst_checkpoint_and_the_first_on_ties() {
        use CheckState::*;
        let order = [
            Failed,
            CouldNotRun,
            NeedsApproval,
            Checking,
            Pending,
            Stale,
            Never,
            Overridden,
            Passed,
        ];
        let at = |state: CheckState, name: &str| CheckInfo {
            state,
            name: name.into(),
            ..Default::default()
        };
        for (i, &worse) in order.iter().enumerate() {
            for &better in &order[i + 1..] {
                for checks in [
                    vec![at(worse, "a"), at(better, "b")],
                    vec![at(better, "b"), at(worse, "a")],
                ] {
                    assert_eq!(
                        headline(&checks).map(|c| c.state),
                        Some(worse),
                        "{worse:?} over {better:?}"
                    );
                }
            }
            let tie = [at(worse, "first"), at(worse, "second")];
            assert_eq!(
                headline(&tie).map(|c| c.name.as_str()),
                Some("first"),
                "{worse:?} tie"
            );
        }
        assert_eq!(headline(&[]), None);
    }

    #[test]
    fn the_headline_is_not_reported_while_the_agent_works_or_asks() {
        let mut term = pane_from(serde_json::json!({}));
        term.checks = vec![
            CheckInfo {
                state: CheckState::Passed,
                ..Default::default()
            },
            CheckInfo {
                state: CheckState::Failed,
                name: "test".into(),
                ..Default::default()
            },
        ];
        assert_eq!(term.reported_check().map(|c| c.name.as_str()), Some("test"));
        for status in [Status::Working, Status::Blocked] {
            term.status = status;
            assert_eq!(term.reported_check(), None, "{status:?}");
        }
    }

    #[test]
    fn new_check_states_have_their_script_labels() {
        let got = [
            CheckState::Pending,
            CheckState::NeedsApproval,
            CheckState::Stale,
            CheckState::Never,
        ]
        .map(CheckState::label);
        assert_eq!(got, ["pending", "needs_approval", "stale", "never"]);
    }

    #[test]
    fn a_permission_ask_from_before_choices_existed_still_reads() {
        let ask: PermissionAsk =
            serde_json::from_value(serde_json::json!({"tool": "Bash", "detail": "ls", "answerable": true})).unwrap();
        assert!(ask.choices.is_empty(), "an older peer sends no choices");
    }
}
