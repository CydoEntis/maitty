//! `config.toml`: everything user-facing is configurable here. Every field has a default,
//! so a config file only needs the keys you want to change.

use crate::keys::{self, Action, KeySpec};
use crate::brand::{APP_COMMAND};
use crate::theme::{Theme, ThemeOverrides};
use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub const EXAMPLE: &str = include_str!("../config.example.toml");
pub const CODEX_QUEUED_QUESTION_PATTERN: &str = r"(?m)^[ \t]*\?[ \t]+[1-9][0-9]*[ \t]+questions?[ \t]*$|^[ \t]*enter[ \t]+submit[ \t]+ctrl\+\][ \t]+skip[ \t]+shift\+[^\r\n]*main prompt[ \t]*$";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Key that arms command mode, tmux style.
    pub prefix: String,
    /// Teach Claude (when the app starts it) to move itself into a worktree when asked.
    pub teach_agents: bool,
    /// What agents may do through `<app> mcp`.
    pub mcp: Mcp,
    /// Named setups for + New: e.g. a worktree with claude, a dev server and lazygit.
    pub recipes: Vec<Recipe>,
    /// Saved launches: an agent, a model and a prompt, e.g. "commit and push" (Ctrl+Space .).
    pub presets: Vec<Preset>,
    /// Editor for "open in editor" (Files, Changes). Empty: $VISUAL, $EDITOR, then `code`.
    /// Terminal editors (nvim, vim, hx, nano, micro, …) open inside the app beside the agent.
    pub editor: String,
    pub theme: String,
    pub theme_overrides: ThemeOverrides,
    /// Shell for new panes. Default: pwsh (or powershell) on Windows, $SHELL elsewhere.
    pub shell: Option<String>,
    pub shell_args: Vec<String>,
    /// Teach PowerShell panes to report their directory (needed for the sidebar to follow
    /// `cd`; other shells report it on their own).
    pub shell_integration: bool,
    /// Extra environment for every pane.
    pub env: BTreeMap<String, String>,
    /// Scrollback lines kept per pane by the client.
    pub scrollback: usize,
    /// Bytes of output the daemon keeps per pane to replay on reattach.
    pub replay_bytes: usize,
    pub ui: Ui,
    pub restore: Restore,
    /// Starting an agent inside a repo that isn't a workspace yet makes it one.
    pub auto_workspace: bool,
    /// An agent stopped by a plan limit is told "continue" once the limit resets.
    pub auto_continue: bool,
    pub worktree: Worktree,
    /// How a new session starts and whether sessions may start helpers (`[sessions]`).
    pub sessions: Sessions,
    pub quick: Quick,
    pub icons: Icons,
    pub keys: Keys,
    pub detection: Detection,
    pub notify: Notify,
    pub snapshots: Snapshots,
    /// Agent definitions merged over the built-ins by `name`.
    pub agents: Vec<AgentDef>,
    /// Drop the built-in agent list and use only `agents`.
    pub agents_replace_defaults: bool,
    /// Commands the app runs when an agent's turn ends in the named project (`[[check]]`).
    pub check: Vec<CheckDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui {
    pub sidebar: bool,
    /// "left" or "right".
    pub sidebar_position: String,
    /// Milliseconds after the prefix before the which-key popup shows (0 = immediately).
    pub which_key_delay_ms: u64,
    pub which_key: bool,
    pub mouse: bool,
    /// Animate the working icon with these frames (empty = use `icons.working`).
    pub spinner: Vec<String>,
    /// One colour per workspace, assigned in order and kept across restarts.
    pub workspace_colors: Vec<String>,
    /// The welcome screen when the app starts.
    pub splash: bool,
    /// When the splash's wordmark falls into place: "cold start" (a first start of a version, or
    /// nothing left running), "always" or "never".
    pub splash_animation: String,
    /// Look for a newer the app when the window opens (and every few hours) and say so.
    pub update_check: bool,
    /// Short slides and glides (the sidebar, the sheet, the highlight, toasts). Always off
    /// over SSH.
    pub motion: bool,
    /// Alt+arrows move between panes without the leader (off: they go to the program, for
    /// shells that jump words with them).
    pub alt_arrows: bool,
    /// Where plain `<app>` opens its first shell (and the one after you close everything).
    /// Empty: wherever you run the app. `~` is your home folder.
    pub start_dir: String,
    /// Sidebar sessions (and worktrees) that need you go first; the rest keep their place.
    pub attention_sort: bool,
    /// The look (ADR 0015): "floating", "tiled", "compact" or "grid". Read with `layout_kind`,
    /// which also takes older files' "cards" (with `panes`) and "flat"; empty means unset.
    pub layout: String,
    /// "terminal" leaves chrome on the terminal's own background (so transparency and a
    /// background image show through evenly); "solid" paints everything the theme's `bg`.
    pub background: String,
    /// Older files' Cards spacing, "floating" or "tiled": read only while `layout` is unset or
    /// "cards".
    pub panes: String,
    /// Card edges: "square" (lines on the card's edge, its colour right up to them) or "rounded"
    /// (a centred line with rounded corners, a thin strip of desk inside it). "flush", the old
    /// name for square, reads as square.
    #[serde(deserialize_with = "corners_named")]
    pub corners: String,
    /// Which glyph set marks sessions and checks: "auto" (ASCII when the blocked, done and idle
    /// icons are all ASCII, else Unicode), "unicode", "ascii" or "nerd".
    pub glyphs: String,
    /// Space between floating cards: "0", "1" or "2" rows stacked (and columns side by side).
    pub gap: String,
    /// How far unfocused panes fade toward their background: "off", "subtle", "30%", "60%" ("40%" from older configs counts as 30%).
    pub dim: String,
    /// The focused card's border: "accent", "bright" or "none".
    pub focus_border: String,
    /// One shape for every selection, chip, button and row: round ends (Nerd Font half-circles) or, off, square blocks.
    pub rounded: bool,
    /// Show `· N agents` on a working session's row when the agent reports helpers.
    pub subagent_count: bool,
}

pub const NEW_AGENTS: &[&str] = &["last used", "claude", "codex", "gemini"];
pub const NEW_WHERE: &[&str] = &["new worktree", "this folder"];
pub const HELPERS_MAY: &[&str] = &["ask", "always", "never"];
pub const HELPERS_AT_ONCE: &[&str] = &["2", "3", "4"];

/// Settings > Sessions. `helpers` and `helpers_at_once` are read by the daemon when an agent
/// starts a session (`Config::helpers_at_once`); the rest by the client's new-session flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Sessions {
    /// Which agent leader `s` starts: one of `NEW_AGENTS`.
    pub agent: String,
    /// Where a new session opens: one of `NEW_WHERE`.
    #[serde(rename = "where")]
    pub place: String,
    /// Whether a session may start helpers: one of `HELPERS_MAY`.
    pub helpers: String,
    /// Helpers one session runs at a time: one of `HELPERS_AT_ONCE`. An old `[mcp]
    /// helpers_at_once` fills it in when this is not set (`fold_legacy_helpers`).
    pub helpers_at_once: String,
}

impl Default for Sessions {
    fn default() -> Self {
        Sessions { agent: "last used".into(), place: "new worktree".into(), helpers: "ask".into(), helpers_at_once: "3".into() }
    }
}

/// An old `[mcp] helpers_at_once` (0 to 16, 0 for no limit) is the one limit now unless
/// `[sessions] helpers_at_once` is set: it becomes the nearest of `HELPERS_AT_ONCE`, and 0 the
/// largest. The file itself is not rewritten.
fn fold_legacy_helpers(value: &mut toml::Table) {
    let sessions_set = value.get("sessions").and_then(toml::Value::as_table).is_some_and(|t| t.contains_key("helpers_at_once"));
    let old = value.get("mcp").and_then(toml::Value::as_table).and_then(|t| t.get("helpers_at_once")).and_then(toml::Value::as_integer);
    let (false, Some(old)) = (sessions_set, old) else { return };
    let nearest = match old {
        ..=2 if old > 0 => "2",
        3 => "3",
        _ => "4",
    };
    let sessions = value.entry("sessions").or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let Some(t) = sessions.as_table_mut() {
        t.insert("helpers_at_once".into(), toml::Value::String(nearest.into()));
    }
}

fn validate_sessions(s: &Sessions) -> Result<()> {
    let one_of = |key: &str, value: &str, allowed: &[&str]| -> Result<()> {
        anyhow::ensure!(allowed.contains(&value), "sessions.{key} is \"{value}\"; use one of: {}", allowed.join(", "));
        Ok(())
    };
    one_of("agent", &s.agent, NEW_AGENTS)?;
    one_of("where", &s.place, NEW_WHERE)?;
    one_of("helpers", &s.helpers, HELPERS_MAY)?;
    one_of("helpers_at_once", &s.helpers_at_once, HELPERS_AT_ONCE)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Restore {
    /// Rebuild the saved workspaces, tabs and panes when the server starts.
    pub enabled: bool,
    /// Relaunch agents with their resume command (`claude --resume <id>`, ...).
    pub agents: bool,
    /// Re-run commands panes were started with (`spawn-right:lazygit`, `<app> split -- x`).
    pub commands: bool,
    /// Keep each pane's recent output on disk so it is there after a restart. It can hold
    /// secrets that were on screen; off deletes what is saved.
    pub output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Worktree {
    /// Where new worktrees go. `{repo}`, `{repo_parent}`, `{branch}` are substituted.
    pub dir: String,
    /// Command to start in a new worktree's first pane (e.g. "claude"). Empty = a shell.
    pub command: String,
    /// New agents (+ New, the quick prompt, or a quick-prompt agent typed into a pane's
    /// PowerShell, bash or fish) get their own worktree when started in a repo's main checkout.
    pub per_agent: bool,
    /// Closing the last thing running in a worktree the app made removes its folder (the
    /// branch is kept; a worktree with uncommitted changes is left alone).
    pub delete_with_last: bool,
    /// Keep this agent (e.g. "claude") booted in a spare worktree of the repo you last
    /// used, so the next one starts instantly. Costs one idle agent. Empty: off.
    pub prewarm: String,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Mcp {
    /// May agents answer other agents' prompts with "yes"? never | safe | always
    /// ("no" is always allowed).
    pub approve: String,
    /// "safe": the prompt must mention one of these.
    pub safe: Vec<String>,
    /// project: only sessions in the calling agent's repo; all: every session.
    pub scope: String,
    /// What an agent's pane may do through the app by default (`<app> grant` changes one
    /// pane): read (other panes' screens), write (type into them), start (new sessions),
    /// respond (answer another agent's prompt or question), admin (close other panes, stop
    /// the server).
    pub grants: Vec<String>,
}

impl Default for Mcp {
    fn default() -> Self {
        Mcp {
            approve: "never".into(),
            safe: [
                "npm test", "npm run test", "npm run lint", "pnpm test", "yarn test", "cargo test", "cargo check", "cargo clippy", "pytest",
                "go test", "git status", "git diff", "git log",
            ]
            .map(String::from)
            .to_vec(),
            scope: "project".into(),
            grants: ["read", "write", "start"].map(String::from).to_vec(),
        }
    }
}

const CORNERS_SQUARE: &str = "square";

/// The four looks over the one session layout (ADR 0015).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Floating,
    Tiled,
    Compact,
    Grid,
}

impl LayoutKind {
    /// Its value in `ui.layout`.
    pub fn name(self) -> &'static str {
        match self {
            LayoutKind::Floating => "floating",
            LayoutKind::Tiled => "tiled",
            LayoutKind::Compact => "compact",
            LayoutKind::Grid => "grid",
        }
    }
}

impl Ui {
    /// The look chosen, reading older files too: "cards" (or no `layout` at all) with
    /// `panes = "tiled"` is Tiled, other "cards" Floating, and "flat" Compact.
    pub fn layout_kind(&self) -> LayoutKind {
        match self.layout.as_str() {
            "tiled" => LayoutKind::Tiled,
            "compact" | "flat" => LayoutKind::Compact,
            "grid" => LayoutKind::Grid,
            "" | "cards" if self.panes == "tiled" => LayoutKind::Tiled,
            _ => LayoutKind::Floating,
        }
    }
}

fn corners_named<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let name = String::deserialize(d)?;
    Ok(if name == "flush" { CORNERS_SQUARE.into() } else { name })
}

/// How long a check may run unless its `[[check]]` says otherwise.
pub const CHECK_TIMEOUT_S_DEFAULT: u64 = 600;

/// The command the app runs in a session's worktree when an agent's turn ends in `project`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CheckDef {
    /// The repository's main checkout; sessions in any of its worktrees use this check.
    pub project: PathBuf,
    /// The checkpoint's name, unique in its project; unset, the command's last word.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Run through the user's shell.
    pub command: String,
    /// Seconds before the run is stopped and reads as not run.
    pub timeout_s: u64,
}

impl CheckDef {
    /// What the checkpoint is called: its `name`, else the command's last word (`npm test`: `test`).
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or_else(|| self.command.split_whitespace().last().unwrap_or_default())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_s)
    }
}

impl Default for CheckDef {
    fn default() -> Self {
        Self { project: PathBuf::new(), name: None, command: String::new(), timeout_s: CHECK_TIMEOUT_S_DEFAULT }
    }
}

/// Whether two paths name the same place, ignoring separator style, a trailing separator and case.
pub fn same_path(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    n(a) == n(b)
}

/// Refuse a `[[check]]` that could never run, or two of one name in a project, so a typo shows
/// when the config loads and not as a check that silently never happens.
pub fn validate_checks(checks: &[CheckDef]) -> Result<()> {
    for (i, c) in checks.iter().enumerate() {
        let n = i + 1;
        anyhow::ensure!(!c.project.as_os_str().is_empty(), "[[check]] #{n} needs a `project`");
        validate_check(n, c, &c.project.display().to_string())?;
        anyhow::ensure!(
            !c.project.to_string_lossy().starts_with('~'),
            "[[check]] #{n}: `project` {} starts with `~`, which is not expanded; give the full path",
            c.project.display()
        );
        anyhow::ensure!(
            !checks[..i].iter().any(|o| same_path(&o.project, &c.project) && o.name().eq_ignore_ascii_case(c.name())),
            "[[check]] #{n}: {} already has a check named `{}`; give each a different `name`",
            c.project.display(),
            c.name()
        );
    }
    Ok(())
}

/// The rules a check meets wherever it is named: in config or in a repo's file (`place`).
fn validate_check(n: usize, c: &CheckDef, place: &str) -> Result<()> {
    anyhow::ensure!(!c.command.trim().is_empty(), "[[check]] #{n} ({place}) needs a `command`");
    anyhow::ensure!(c.timeout_s > 0, "[[check]] #{n} ({place}) needs a `timeout_s` above 0");
    anyhow::ensure!(c.name.as_deref().is_none_or(|s| !s.trim().is_empty()), "[[check]] #{n} ({place}) has an empty `name`");
    Ok(())
}

/// The file a repository names its own checks in, relative to its root.
pub const REPO_CHECKS_FILE: &str = concat!(".", env!("APP_COMMAND"), "/checks.toml");
/// The repo's checks file in `worktree`: the current name, else one under a name the app had
/// before if that is there.
pub fn repo_checks_path(worktree: &Path) -> PathBuf {
    let new = worktree.join(REPO_CHECKS_FILE);
    if new.exists() {
        return new;
    }
    crate::brand::previous().map(|p| worktree.join(format!(".{p}/checks.toml"))).find(|old| old.exists()).unwrap_or(new)
}

/// The checks in a repository's `.<app>/checks.toml`, refused as a whole when any is unusable,
/// two share a name, or the file has a key the app doesn't know. Their `project` is left empty.
pub fn parse_repo_checks(text: &str) -> Result<Vec<CheckDef>> {
    /// `.<app>/checks.toml`: checks a repository ships. No `project`: it is the repo.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RepoChecks {
        #[serde(default)]
        check: Vec<RepoCheck>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RepoCheck {
        name: Option<String>,
        #[serde(default)]
        command: String,
        #[serde(default = "timeout_default")]
        timeout_s: u64,
    }
    fn timeout_default() -> u64 {
        CHECK_TIMEOUT_S_DEFAULT
    }
    let file: RepoChecks = toml::from_str(text).with_context(|| format!("reading {REPO_CHECKS_FILE}"))?;
    let checks: Vec<CheckDef> = file
        .check
        .into_iter()
        .map(|c| CheckDef { project: PathBuf::new(), name: c.name, command: c.command, timeout_s: c.timeout_s })
        .collect();
    for (i, c) in checks.iter().enumerate() {
        let n = i + 1;
        validate_check(n, c, REPO_CHECKS_FILE)?;
        // You approve what you read: a line break or other control character could hide part
        // of the command from the approval modal.
        anyhow::ensure!(
            !c.command.chars().chain(c.name().chars()).any(char::is_control),
            "[[check]] #{n} ({REPO_CHECKS_FILE}): a `command` or `name` may not hold a line break, tab or other control character"
        );
        anyhow::ensure!(
            !checks[..i].iter().any(|o| o.name().eq_ignore_ascii_case(c.name())),
            "[[check]] #{n} ({REPO_CHECKS_FILE}): another check is named `{}`; give each a different `name`",
            c.name()
        );
    }
    Ok(checks)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Recipe {
    pub name: String,
    /// Start in its own worktree (in a git project).
    pub worktree: bool,
    /// Commands: the first is the main one; the rest start beside it in the same folder.
    pub run: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Preset {
    pub name: String,
    /// Which agent (a name from + New's RUN row).
    pub agent: String,
    /// One of the agent's models; empty is its default.
    pub model: String,
    /// What to ask. `{task}` is replaced with what you type; with no `{task}` the preset
    /// starts in one key, without asking.
    pub prompt: String,
    /// "send": to the agent you're on; "beside": a new one beside it, in its folder;
    /// "worktree": a new one in a new worktree.
    #[serde(rename = "where")]
    pub place: String,
}

impl Default for Preset {
    fn default() -> Self {
        Preset { name: String::new(), agent: "claude".into(), model: String::new(), prompt: String::new(), place: "beside".into() }
    }
}

impl Preset {
    pub fn asks(&self) -> bool {
        self.prompt.contains("{task}") || self.prompt.trim().is_empty()
    }

    /// The prompt with the task in it.
    pub fn fill(&self, task: &str) -> String {
        let task = task.trim();
        if self.prompt.contains("{task}") {
            self.prompt.replace("{task}", task).trim().to_string()
        } else if self.prompt.trim().is_empty() {
            task.to_string()
        } else if task.is_empty() {
            self.prompt.trim().to_string()
        } else {
            format!("{} {task}", self.prompt.trim())
        }
    }
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe { name: String::new(), worktree: true, run: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Quick {
    /// The agents the app can start (the new-agent dialog lists them).
    pub agents: Vec<QuickAgent>,
}

/// The flag that picks a model when an agent's `model_flag` is empty.
pub const DEFAULT_MODEL_FLAG: &str = "--model";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QuickAgent {
    pub name: String,
    /// `{prompt}` is replaced with the task, quoted for your shell.
    pub command: String,
    /// Models to choose from when starting it (the first choice is always its default).
    pub models: Vec<String>,
    /// The flag that picks one (default `--model`).
    pub model_flag: String,
    /// Its own environment, which makes it an account: `CLAUDE_CONFIG_DIR = "~/.claude-work"`
    /// is claude signed in as someone else. A leading `~` is your home folder.
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Icons {
    pub working: String,
    pub blocked: String,
    pub done: String,
    pub idle: String,
    pub shell: String,
    pub active_workspace: String,
    /// Shown before a workspace's branch (default: the Nerd Font / Powerline branch glyph).
    pub branch: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Keys {
    /// Bindings after the prefix: key spec -> action. Merged over the defaults.
    pub prefix: BTreeMap<String, String>,
    /// Bindings that work without the prefix.
    pub global: BTreeMap<String, String>,
    /// Ignore the built-in bindings entirely.
    pub replace_defaults: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Detection {
    /// Output within this window counts as "working" for agents without hooks or patterns.
    pub working_window_ms: u64,
    /// Output this soon after a keystroke is treated as echo, not work.
    pub echo_grace_ms: u64,
    /// How often the process tree is scanned for agents.
    pub scan_interval_ms: u64,
    /// Rows from the bottom of the screen that status patterns are matched against.
    pub pattern_rows: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshots {
    /// Keep a checkout's files as they were each time an agent's turn begins there, so a
    /// turn can be taken back from the agent's menu.
    pub enabled: bool,
}

impl Default for Snapshots {
    fn default() -> Self {
        Snapshots { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Notify {
    /// Play the app's sound when an agent needs you.
    pub sound: bool,
    /// Alerts on your phone through ntfy (the free ntfy app): your topic's name. Empty is off.
    /// On the public server the name is all that keeps them private, so make it hard to guess.
    pub phone_topic: String,
    /// The ntfy server (your own, if you host one).
    pub phone_server: String,
    /// Only once it has waited this long (seconds) with nobody answering: at your desk you
    /// answer first and the phone stays quiet.
    pub phone_after: u64,
    /// Also when an agent finishes and nobody has looked.
    pub phone_done: bool,
    /// Say what the agent is doing in the alert, not just which agent and where.
    pub phone_text: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentDef {
    pub name: String,
    /// Executable names (without .exe), matched case-insensitively.
    pub process: Vec<String>,
    /// Substrings of the command line, for agents that run under node/bun/python.
    pub cmdline: Vec<String>,
    /// Regexes (case-insensitive) on the bottom of the screen meaning "waiting on you".
    pub blocked_patterns: Vec<String>,
    /// Regexes meaning "mid-turn". When absent, recent output is used instead.
    pub working_patterns: Vec<String>,
    /// Command that resumes a known session; `{session}` is the id hooks reported.
    pub resume: Option<String>,
    /// Command that resumes the most recent session in the pane's directory.
    pub resume_last: Option<String>,
    /// Set to false to disable a built-in agent.
    pub enabled: Option<bool>,
}

/// Programs that run an agent's script rather than being the agent: they can't name it.
const INTERPRETERS: &[&str] = &["node", "nodejs", "bun", "deno", "python", "python3", "py", "ruby", "java", "uv", "uvx", "npx", "pnpm", "tsx", "ts-node"];

/// What the app learns from a program you say is an agent: matched by its name, or (run by
/// node, python, …) by its package or script. The usual screen signs of a working agent;
/// None when there's nothing to go on (a shell, no script).
pub fn agent_from_command(program: &str, args: &[String]) -> Option<AgentDef> {
    let program = program.to_lowercase();
    let shells = ["pwsh", "powershell", "cmd", "bash", "zsh", "fish", "nu", "sh", "dash", "elvish", "xonsh"];
    if program.is_empty() || shells.contains(&program.as_str()) {
        return None;
    }
    let (name, process, cmdline) = if INTERPRETERS.contains(&program.as_str()) {
        let script = args.iter().skip(1).find(|a| !a.starts_with('-') && !["run", "x", "exec", "dlx"].contains(&a.as_str()))?;
        let path = script.replace('\\', "/");
        let name = match path.split_once("node_modules/") {
            // An npm package: its name (the part after a scope).
            Some((_, rest)) => {
                let mut parts = rest.split('/');
                let first = parts.next().unwrap_or_default();
                if first.starts_with('@') { parts.next().unwrap_or(first) } else { first }.to_string()
            }
            None => {
                let file = path.rsplit('/').next().unwrap_or(&path);
                let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
                // cli.js, main.py, index.ts: the folder says more.
                if ["cli", "main", "index", "__main__", "app", "run"].contains(&stem) {
                    path.rsplit('/').nth(1).filter(|d| !d.is_empty() && *d != "dist" && *d != "bin").unwrap_or(stem).to_string()
                } else {
                    stem.to_string()
                }
            }
        };
        (name.clone(), Vec::new(), vec![name])
    } else {
        (program.clone(), vec![program.clone()], Vec::new())
    };
    Some(AgentDef {
        name,
        process,
        cmdline,
        working_patterns: [r"esc to interrupt", r"esc to cancel", r"ctrl\+c to (stop|interrupt|cancel)"].map(String::from).to_vec(),
        blocked_patterns: [r"Do you want to", r"\(y/n\)", r"\[y/N\]"].map(String::from).to_vec(),
        resume: None,
        resume_last: None,
        enabled: None,
    })
}

/// Add an agent to config.toml (`[[agents]]`), replacing one of the same name; the rest of
/// the file stays as it is.
pub fn add_agent(def: &AgentDef) -> Result<()> {
    add_agent_to(&config_path(), def)
}

fn add_agent_to(file: &std::path::Path, def: &AgentDef) -> Result<()> {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context("reading config"),
    };
    let mut doc: toml_edit::DocumentMut = text.parse().context("config.toml has a syntax error; fix it first")?;
    if !doc.contains_key("agents") {
        doc.insert("agents", toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()));
    }
    let agents = doc["agents"].as_array_of_tables_mut().context("`agents` in config.toml isn't a list of [[agents]]")?;
    agents.retain(|t| t.get("name").and_then(|n| n.as_str()) != Some(def.name.as_str()));
    let list = |xs: &[String]| toml_edit::value(xs.iter().map(String::as_str).collect::<toml_edit::Array>());
    let mut t = toml_edit::Table::new();
    t.insert("name", toml_edit::value(def.name.as_str()));
    if !def.process.is_empty() {
        t.insert("process", list(&def.process));
    }
    if !def.cmdline.is_empty() {
        t.insert("cmdline", list(&def.cmdline));
    }
    t.insert("working_patterns", list(&def.working_patterns));
    t.insert("blocked_patterns", list(&def.blocked_patterns));
    // At the end of the file, after what's there.
    t.set_position(isize::MAX);
    agents.push(t);
    write_atomic(file, doc.to_string()).context("writing config")?;
    crate::sync::push_soon();
    Ok(())
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: "ctrl+space".into(),
            editor: String::new(),
            teach_agents: true,
            mcp: Mcp::default(),
            recipes: Vec::new(),
            presets: Vec::new(),
            theme: crate::theme::DEFAULT.into(),
            theme_overrides: ThemeOverrides::default(),
            shell: None,
            shell_args: Vec::new(),
            shell_integration: true,
            env: BTreeMap::new(),
            scrollback: 10_000,
            replay_bytes: 2 * 1024 * 1024,
            ui: Ui::default(),
            restore: Restore::default(),
            auto_workspace: false,
            auto_continue: true,
            worktree: Worktree::default(),
            sessions: Sessions::default(),
            quick: Quick::default(),
            icons: Icons::default(),
            keys: Keys::default(),
            detection: Detection::default(),
            notify: Notify::default(),
            snapshots: Snapshots::default(),
            agents: Vec::new(),
            agents_replace_defaults: false,
            check: Vec::new(),
        }
    }
}

impl Default for Ui {
    fn default() -> Self {
        Ui {
            sidebar: true,
            sidebar_position: "left".into(),
            which_key_delay_ms: 600,
            which_key: false,
            mouse: true,
            spinner: SPINNERS[0].1.iter().map(|frame| (*frame).into()).collect(),
            workspace_colors: ["#a593ff", "#5aa9ff", "#ff7ab6", "#3dd6c0", "#e8c565", "#ff9f6b", "#c792ea", "#7fd8a4"]
                .map(String::from)
                .to_vec(),
            splash: true,
            splash_animation: "cold start".into(),
            update_check: true,
            motion: true,
            alt_arrows: true,
            start_dir: String::new(),
            attention_sort: true,
            layout: String::new(),
            background: "terminal".into(),
            panes: "floating".into(),
            corners: CORNERS_SQUARE.into(),
            glyphs: "auto".into(),
            gap: "1".into(),
            dim: "30%".into(),
            focus_border: "accent".into(),
            rounded: false,
            subagent_count: true,
        }
    }
}

impl Default for Quick {
    fn default() -> Self {
        let a = |name: &str, command: &str, models: &[&str], flag: &str| QuickAgent {
            name: name.into(),
            command: command.into(),
            models: models.iter().map(|m| m.to_string()).collect(),
            model_flag: flag.into(),
            env: BTreeMap::new(),
        };
        Quick {
            agents: vec![a("claude", "claude {prompt}", &["opus", "sonnet", "haiku"], "--model"), a("codex", "codex {prompt}", &[], "-m")],
        }
    }
}

impl Default for Restore {
    fn default() -> Self {
        Restore { enabled: true, agents: true, commands: true, output: true }
    }
}

impl Default for Worktree {
    fn default() -> Self {
        Worktree {
            dir: "{repo_parent}/{repo}-worktrees/{branch}".into(),
            command: String::new(),
            per_agent: true,
            delete_with_last: true,
            prewarm: String::new(),
        }
    }
}

pub const SPINNERS: &[(&str, &[&str])] = &[
    ("Braille", &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]),
    ("Dots", &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    ("Line (ASCII)", &["|", "/", "-", "\\"]),
    // Three dots chasing round the cell's edge.
    ("Snake", &["⠇", "⠋", "⠙", "⠸", "⢰", "⣠", "⣄", "⡆"]),
    // Each frame held twice: the pulse breathes at half the other styles' pace.
    ("Pulse", &["·", "·", "•", "•", "●", "●", "•", "•"]),
    // From ratatui-spinner's FluxFrames (MIT): Fill is its PULSE.
    ("Fill", &["⣀", "⣤", "⣶", "⣾", "⣿", "⣾", "⣶", "⣤"]),
    ("Orbit", &["⠁", "⠈", "⠐", "⠠", "⢀", "⡀", "⠄", "⠂"]),
    ("Bounce", &["⠉", "⠒", "⣀", "⠒"]),
    ("Pair", &["⠉", "⠘", "⠰", "⢠", "⣀", "⡄", "⠆", "⠃"]),
    ("Arc", &["◜", "◝", "◞", "◟"]),
    ("Moon", &["◓", "◑", "◒", "◐"]),
    ("Star", &["✶", "✷", "✸", "✹"]),
    ("Blocks", &["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂"]),
    ("Bars", BARS_FRAMES),
];
/// The v7 handoff's working spinner: three Braille bars, two columns wide. The ASCII glyph set
/// swaps it for `BARS_ASCII` (see `client::render::status_icon`).
pub const BARS_FRAMES: &[&str] = &["⣰⡄", "⣼⡀", "⣶⡆", "⣧⡇", "⣆⡆", "⣤⡄", "⣸⡆", "⣴⡇", "⣇⡄", "⣦⡀"];
pub const BARS_ASCII: &[&str] = &[". ", "o ", "O ", "o "];
pub const MAX_SPINNER_FRAMES: usize = 64;

pub fn spinner_name(frames: &[String]) -> &'static str {
    SPINNERS.iter().find(|(_, preset)| frames == *preset).map_or("Custom", |(name, _)| name)
}

/// One column, or two when every character is Braille: the 3-bar spinner needs two cells, and
/// Braille is the one block every terminal draws one cell wide each.
fn fits_cells(frame: &str) -> bool {
    use unicode_width::UnicodeWidthStr;
    match frame.width() {
        1 => true,
        2 => frame.chars().all(|c| ('\u{2800}'..='\u{28ff}').contains(&c)),
        _ => false,
    }
}

pub fn validate_spinner(frames: &[String]) -> Result<()> {
    anyhow::ensure!(frames.len() <= MAX_SPINNER_FRAMES, "a working spinner can have at most {MAX_SPINNER_FRAMES} frames");
    anyhow::ensure!(frames.iter().all(|frame| !frame.trim().is_empty() && !frame.chars().any(char::is_control) && fits_cells(frame)),
        "each spinner frame must fit one terminal column (two for Braille) and contain no control characters");
    Ok(())
}

#[cfg(test)]
mod spinner_tests {
    use super::*;

    #[test]
    fn presets_and_custom_frames_are_safe_single_column_indicators() {
        for (name, frames) in SPINNERS {
            let frames = frames.iter().map(|frame| (*frame).into()).collect::<Vec<_>>();
            validate_spinner(&frames).unwrap();
            assert_eq!(spinner_name(&frames), *name);
        }
        for frames in [vec!["".into()], vec![" ".into()], vec!["🌈".into()], vec!["\x1b[31m".into()], vec!["a".into(); MAX_SPINNER_FRAMES+1]] {
            assert!(validate_spinner(&frames).is_err());
        }
    }

    #[test]
    fn the_bars_spinner_is_two_braille_cells_and_other_wide_frames_stay_out() {
        use unicode_width::UnicodeWidthStr;
        assert_eq!(BARS_FRAMES.len(), 10);
        assert!(BARS_FRAMES.iter().all(|f| f.width() == 2 && f.chars().count() == 2));
        assert!(BARS_ASCII.iter().all(|f| f.is_ascii() && f.width() == 2));
        for bad in ["ab", "🌈", "⣰a", "⣰⡄⣰"] {
            assert!(validate_spinner(&[bad.to_string()]).is_err(), "{bad}");
        }
    }
}

/// The "done" icon before it became a green dot.
const OLD_DONE_ICON: &str = "✓";
/// The shell icon before it became a terminal.
const OLD_SHELL_ICON: &str = "›";

impl Default for Icons {
    fn default() -> Self {
        Icons {
            working: "⠹".into(),
            blocked: "●".into(),
            // Filled like "needs you", told apart by colour: green.
            done: "●".into(),
            idle: "○".into(),
            // A terminal (Nerd Font); set it to ">" or "$" without one.
            shell: "\u{f489}".into(),
            active_workspace: "▌".into(),
            branch: "\u{e0a0} ".into(),
        }
    }
}

impl Default for Detection {
    fn default() -> Self {
        Detection { working_window_ms: 1500, echo_grace_ms: 300, scan_interval_ms: 1000, pattern_rows: 12 }
    }
}

impl Default for Notify {
    fn default() -> Self {
        Notify {
            sound: true,
            phone_topic: String::new(),
            phone_server: "https://ntfy.sh".into(),
            phone_after: 60,
            phone_done: false,
            phone_text: false,
        }
    }
}

fn agent(name: &str, process: &[&str], cmdline: &[&str], working: &[&str], blocked: &[&str]) -> AgentDef {
    let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect();
    AgentDef {
        name: name.into(),
        process: v(process),
        cmdline: v(cmdline),
        working_patterns: v(working),
        blocked_patterns: v(blocked),
        resume: None,
        resume_last: None,
        enabled: None,
    }
}

fn resumable(mut a: AgentDef, resume: &str, last: &str) -> AgentDef {
    a.resume = Some(resume.into());
    a.resume_last = Some(last.into());
    a
}

/// A built-in or configured agent kind, named plainly (letters, digits, - _ .).
fn known_agent(cfg: &Config, name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        && cfg.agent_defs().iter().any(|d| d.name == name)
}

pub fn builtin_agents() -> Vec<AgentDef> {
    vec![
        resumable(agent(
            "claude",
            &["claude", "claude-code"],
            &["@anthropic-ai/claude-code", "claude-code/cli"],
            // "esc to interrupt", or the spinner line: "Misting… (5s · ↓ 219 tokens)".
            &[r"esc to interrupt", r"…\s*\(\d+[smh][^)]*tokens"],
            &[
                r"Do you want to (proceed|make this edit|create|run|allow)",
                r"❯ 1\. Yes",
                r"Would you like to proceed",
                r"Do you trust the files",
                // Its question and permission dialogs.
                r"Enter to select",
                r"Esc to cancel",
            ],
        ), "claude --resume {session}", "claude --continue"),
        resumable(agent(
            "codex",
            &["codex"],
            &["@openai/codex"],
            // Finished answers can discuss the shortcut; only a complete activity row counts.
            &[r"(?m)^[ \t]*(?:•[ \t]*)?[^()\r\n]+ \((?:\d+[smh][^()\r\n]*[•·-][ \t]*)?esc to interrupt[^()\r\n]*\)(?:[ \t]+[·•][ \t]+[^\r\n]*)?[ \t]*$"],
            &[r"Allow command\?", r"Would you like to (run|make|apply)", r"approve this", r"Trust this folder\?", r"› 1\. Yes"],
        ), "codex resume {session}", "codex resume --last"),
        agent("gemini", &["gemini"], &["@google/gemini-cli"], &[r"esc to cancel"], &[r"Allow execution", r"Apply this change\?"]),
        resumable(agent("opencode", &["opencode", "opencode2", "open-code"], &["opencode-ai"], &[r"esc interrupt"], &[r"Permission required"]), "opencode --session {session}", "opencode --continue"),
        agent("cursor", &["cursor-agent"], &[], &[r"ctrl\+c to stop"], &[r"Run this command\?"]),
        resumable(agent("copilot", &["copilot", "github-copilot", "ghcs"], &["@github/copilot"], &[r"esc to cancel"], &[r"Do you want to"]), "copilot --resume {session}", "copilot --continue"),
        agent("amp", &["amp", "amp-local"], &["@sourcegraph/amp"], &[r"esc to cancel"], &[r"Allow\?"]),
        agent("qwen", &["qwen", "qwen-code"], &["@qwen-code/qwen-code"], &[r"esc to cancel"], &[r"Allow execution"]),
        agent("aider", &["aider"], &["aider-chat", "\\aider", "/aider"], &[], &[r"\(Y\)es/\(N\)o"]),
        agent("goose", &["goose"], &[], &[], &[]),
        agent("crush", &["crush"], &[], &[], &[]),
        agent("droid", &["droid"], &[], &[], &[]),
        agent("pi", &["pi"], &["@mariozechner/pi-coding-agent", "@earendil-works/pi-coding-agent"], &[], &[]),
        agent("kiro", &["kiro-cli", "kiro", "q"], &[], &[], &[]),
        agent("grok", &["grok", "grok-build"], &["@vibe-kit/grok-cli", "grok-cli"], &[r"esc to interrupt"], &[r"Do you want to"]),
        agent("auggie", &["auggie"], &["@augmentcode/auggie"], &[], &[]),
        agent("kimi", &["kimi", "kimi-code"], &["kimi-cli", "@moonshot-ai/kimi-code"], &[], &[]),
        // The DeepSeek harness (dsh-TUI), whatever alias starts it.
        agent("deepseek", &["dsh", "dsh-tui", "deepseek"], &["@deepseek-ai/dsh", "dsh-tui", "deepseek-harness"], &[r"esc to interrupt", r"esc to cancel"], &[r"Do you want to", r"\(y/n\)"]),
        agent("devin", &["devin", "devin-cli"], &[], &[], &[]),
        agent("agy", &["agy", "antigravity", "antigravity-cli"], &[], &[], &[]),
        agent("cline", &["cline", ".cline"], &["@cline/cli", "/cline/bin/"], &[], &[]),
        agent("omp", &["omp"], &["@oh-my-pi/pi-coding-agent"], &[], &[]),
        agent("mastracode", &["mastracode", "mastra-code"], &["mastracode/dist/cli"], &[], &[]),
        agent("hermes", &["hermes", "hermes-agent"], &["hermes_cli/main.py", "/bin/hermes", "/bin/hermes-agent"], &[], &[]),
        agent("kilo", &["kilo", "kilo-code"], &["@kilocode/cli"], &[], &[]),
        agent("qodercli", &["qodercli", "qoderclicn", "qoder", "qodercn"], &[], &[], &[]),
        agent("letta", &["letta", "letta-code"], &["@letta-ai/letta-code"], &[], &[]),
        agent("maki", &["maki"], &[], &[], &[]),
        agent("muse", &["muse", "muse-code", "muse-cli"], &[], &[], &[]),
    ]
}

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var(concat!(env!("APP_ENV"), "_CONFIG")) {
        return PathBuf::from(p);
    }
    if cfg!(windows)
        && let Some(d) = directories::BaseDirs::new() {
            return d.config_dir().join(env!("APP_COMMAND")).join("config.toml");
        }
    // XDG-style on macOS too: that's where terminal people look.
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(env!("APP_COMMAND")).join("config.toml")
}

/// Write a file so a reader (or a crash) never sees half of it: write a temporary file
/// beside it, then swap it in.
pub fn write_atomic(path: &std::path::Path, data: impl AsRef<[u8]>) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{APP_COMMAND}-tmp"));
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}


/// Read a JSON state file; one that's there but won't parse is kept as `<name>.bak` (so a
/// bad write or a hand edit loses nothing) and the defaults are used.
pub fn read_state<T: serde::de::DeserializeOwned + Default>(path: &std::path::Path) -> T {
    let Ok(text) = std::fs::read_to_string(path) else { return T::default() };
    match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let bak = path.with_file_name(format!("{name}.bak"));
            tracing::warn!("{} doesn't parse ({e}); kept it as {}", path.display(), bak.display());
            let _ = std::fs::copy(path, &bak);
            T::default()
        }
    }
}

pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", env!("APP_COMMAND"))
        .map(|d| d.data_local_dir().to_path_buf())
        .unwrap_or_else(std::env::temp_dir)
}

/// Lay `over` on top of `base`: tables merge key by key, anything else is replaced.
fn merge(base: &mut toml::Table, over: toml::Table) {
    for (k, v) in over {
        match (base.get_mut(&k), v) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge(b, o),
            (_, v) => {
                base.insert(k, v);
            }
        }
    }
}

/// Left in the data folder once the move is done, so it happens only once.
const MIGRATED: &str = "migrated-from";

/// The app had other names before (`brand::previous`): bring the newest one's config (with
/// hydra's default theme moved to the new one) and its data (saved sessions, pane history, UI
/// state) over once. The old folders are left in place.
pub fn migrate_old_names() {
    if std::env::var_os(concat!(env!("APP_ENV"), "_CONFIG")).is_some() {
        return;
    }
    let new_cfg = config_path();
    let new_dir = new_cfg.parent().map(Path::to_path_buf);
    let new_data = data_dir();
    if new_data.join(MIGRATED).exists() {
        return;
    }
    // The names the app had before, newest first: the first found is brought over.
    for old in crate::brand::previous() {
        let old_cfg_dir = new_dir.as_ref().and_then(|d| d.parent()).map(|d| d.join(old));
        let old_data = directories::ProjectDirs::from("", "", old).map(|d| d.data_local_dir().to_path_buf());
        let has_cfg = old_cfg_dir.as_ref().is_some_and(|d| d.join("config.toml").exists());
        let has_data = old_data.as_ref().is_some_and(|d| d.is_dir());
        if !has_cfg && !has_data {
            continue;
        }
        let cfg = old_cfg_dir.filter(|_| !new_cfg.exists()).zip(new_dir.clone());
        bring_over(old, cfg, old_data.as_deref(), &new_data);
        tracing::info!("brought {old}'s config and data over");
        return;
    }
}

/// Copy an old install's config folder (`cfg`: from, to) and data folder over, the old
/// default theme moved to the new one, and leave the marker.
fn bring_over(old: &str, cfg: Option<(PathBuf, PathBuf)>, old_data: Option<&Path>, new_data: &Path) {
    if let Some((from, to)) = cfg {
        copy_renamed(&from, &to, old);
        for name in ["config.toml", "config.local.toml"] {
            let f = to.join(name);
            if let Ok(text) = std::fs::read_to_string(&f) {
                let moved = text
                    .replace("theme = \"hydra\"", &format!("theme = \"{}\"", crate::theme::DEFAULT))
                    .replace("theme = \"drover\"", &format!("theme = \"{}\"", crate::theme::DEFAULT));
                let _ = std::fs::write(&f, moved);
            }
        }
    }
    if let Some(from) = old_data {
        copy_renamed(from, new_data, old);
    }
    let _ = std::fs::create_dir_all(new_data);
    let _ = std::fs::write(new_data.join(MIGRATED), old);
}

/// Copy a folder's contents into `to`, files named after the old app (`hydra-ui.json`) taking
/// the new name. What's already there is kept; a running server's pid and socket files aren't
/// copied (they belong to the old one).
fn copy_renamed(from: &Path, to: &Path, old: &str) {
    let Ok(entries) = std::fs::read_dir(from) else { return };
    let _ = std::fs::create_dir_all(to);
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(".pid") || name.ends_with(".sock") || name == "daemon.log" {
            continue;
        }
        let renamed = match name.strip_prefix(&format!("{old}-")) {
            Some(rest) => format!("{APP_COMMAND}-{rest}"),
            None => name,
        };
        let dest = to.join(renamed);
        let path = e.path();
        if path.is_dir() {
            copy_renamed(&path, &dest, old);
        } else if !dest.exists() {
            let _ = std::fs::copy(&path, &dest);
        }
    }
}

/// The app's own themes were once named after it ("seshi-night", ADR 0019), and the first set
/// gave way to Night and Day: an old choice moves to the one of the same lightness.
fn retired_theme(theme: &str) -> Option<&'static str> {
    let plain = theme
        .split_once('-')
        .filter(|(p, _)| std::iter::once(crate::brand::APP_COMMAND).chain(crate::brand::previous()).any(|n| n == *p))
        .map_or(theme, |(_, rest)| rest);
    match plain {
        "dark" | "walnut" | "night" => Some("night"),
        "light" | "day" => Some("day"),
        _ => None,
    }
}

impl Config {
    /// How many helpers one session may have working at once (`sessions.helpers_at_once`).
    pub fn helpers_at_once(&self) -> usize {
        self.sessions.helpers_at_once.parse().unwrap_or(3)
    }

    /// Load the config file; a missing file is the defaults, a broken one is an error.
    pub fn load() -> Result<Config> {
        let path = config_path();
        let mut value: toml::Table = match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).with_context(|| format!("parsing {}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => toml::Table::new(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        // This machine's own settings (never synced) win over the shared ones.
        let local = path.with_file_name("config.local.toml");
        if std::env::var_os(concat!(env!("APP_ENV"), "_CONFIG")).is_none()
            && let Ok(s) = std::fs::read_to_string(&local)
        {
            let over: toml::Table = toml::from_str(&s).with_context(|| format!("parsing {}", local.display()))?;
            merge(&mut value, over);
        }
        fold_legacy_helpers(&mut value);
        let mut cfg: Config = toml::Value::Table(value).try_into().with_context(|| format!("parsing {}", path.display()))?;
        // Config files written from the old example spell out the old "done" icon; it's a
        // green dot now (any other choice is kept).
        if cfg.icons.done == OLD_DONE_ICON {
            cfg.icons.done = Icons::default().done;
        }
        if cfg.icons.shell == OLD_SHELL_ICON {
            cfg.icons.shell = Icons::default().shell;
        }
        if let Some(t) = retired_theme(&cfg.theme) {
            cfg.theme = t.to_string();
        }
        // Settings shows the look by its name, whatever an older file called it.
        cfg.ui.layout = cfg.ui.layout_kind().name().into();
        validate_spinner(&cfg.ui.spinner)?;
        validate_checks(&cfg.check)?;
        validate_sessions(&cfg.sessions)?;
        Ok(cfg)
    }

    /// Load, falling back to defaults and returning the error message for display.
    pub fn load_or_default() -> (Config, Option<String>) {
        match Config::load() {
            Ok(c) => (c, None),
            Err(e) => (Config::default(), Some(format!("{e:#}"))),
        }
    }

    /// The start folder (`ui.start_dir`), if it's set and there.
    pub fn start_dir(&self) -> Option<PathBuf> {
        let raw = self.ui.start_dir.trim();
        if raw.is_empty() {
            return None;
        }
        let home = || directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf());
        let dir = match raw.strip_prefix('~') {
            Some(rest) => home()?.join(rest.trim_start_matches(['/', '\\'])),
            None => PathBuf::from(raw),
        };
        dir.is_dir().then_some(dir)
    }

    pub fn theme(&self) -> Theme {
        let mut t = Theme::named(&self.theme);
        t.apply(&self.theme_overrides);
        t
    }

    pub fn shell_command(&self) -> Vec<String> {
        let shell = self.shell.clone().filter(|s| !s.is_empty()).unwrap_or_else(default_shell);
        let mut v = vec![shell];
        v.extend(self.shell_args.iter().cloned());
        v
    }

    /// An agent entry with an environment of its own (an account), by name.
    pub fn account(&self, name: &str) -> Option<&QuickAgent> {
        self.quick.agents.iter().find(|a| a.name == name && !a.env.is_empty())
    }

    /// What goes in front of an account's command: `<app> account <name> -- `. The session
    /// then runs with that account's environment, and the command line itself says which
    /// account it is, so nothing else has to remember.
    fn account_prefix(&self, name: &str) -> String {
        format!(concat!(env!("APP_COMMAND"), " account {} -- "), self.quote_for_shell(name))
    }

    /// `cmd` as account `name` runs it (unchanged for an agent that isn't an account).
    pub fn as_account(&self, name: &str, cmd: &str) -> String {
        match self.account(name) {
            Some(a) if !cmd.starts_with(&self.account_prefix(&a.name)) => format!("{}{cmd}", self.account_prefix(&a.name)),
            _ => cmd.to_string(),
        }
    }

    /// The account a command line runs as, and the command itself.
    pub fn split_account<'a>(&self, cmd: &'a str) -> (Option<String>, &'a str) {
        for a in self.quick.agents.iter().filter(|a| !a.env.is_empty()) {
            if let Some(rest) = cmd.strip_prefix(&self.account_prefix(&a.name)) {
                return (Some(a.name.clone()), rest);
            }
        }
        (None, cmd)
    }

    /// Quote `s` as one argument for the configured shell.
    pub fn quote_for_shell(&self, s: &str) -> String {
        let shell = self.shell_command();
        let exe = std::path::Path::new(&shell[0])
            .file_stem()
            .map(|x| x.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        match exe.as_str() {
            // PowerShell also ends a single-quoted string at the curly single quotes; doubling
            // one gives the literal character back, as for the apostrophe.
            "pwsh" | "powershell" => {
                let mut out = String::from("'");
                for c in s.chars() {
                    if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
                        out.push(c);
                    }
                    out.push(c);
                }
                out.push('\'');
                out
            }
            "nu" => format!("'{}'", s.replace('\'', "''")),
            "cmd" => format!("\"{}\"", s.replace('"', "\"\"").replace(['\r', '\n'], " ")),
            _ => format!("'{}'", s.replace('\'', "'\\''")),
        }
    }

    /// `cmd` with `flag model` right after its program (`--model` when `flag` is empty). A
    /// model with anything but plain name characters is quoted, since it comes from config.
    pub fn with_model(&self, cmd: &str, flag: &str, model: &str) -> String {
        let flag = if flag.is_empty() { DEFAULT_MODEL_FLAG } else { flag };
        let plain = !model.is_empty() && model.chars().all(|c| c.is_ascii_alphanumeric() || "-_.:/@+=".contains(c));
        let model = if plain { model.to_string() } else { self.quote_for_shell(model) };
        match cmd.split_once(' ') {
            Some((first, rest)) => format!("{first} {flag} {model} {rest}"),
            None => format!("{cmd} {flag} {model}"),
        }
    }

    /// The agent started when none is named: the first quick agent, else claude.
    pub fn default_agent(&self) -> String {
        self.quick.agents.first().map(|a| a.name.clone()).unwrap_or_else(|| "claude".into())
    }

    /// The command line that starts `agent` (default: [`Config::default_agent`]) on `model`
    /// (`None`: its default) with `prompt` as its first prompt.
    pub fn agent_command(&self, agent: Option<&str>, model: Option<&str>, prompt: &str) -> Result<String, String> {
        let agent = agent.map(String::from).unwrap_or_else(|| self.default_agent());
        self.agent_command_quoted(&agent, model, &self.quote_for_shell(prompt))
    }

    /// As [`Config::agent_command`], with a prompt that is already quoted for the shell.
    pub fn agent_command_quoted(&self, agent: &str, model: Option<&str>, quoted: &str) -> Result<String, String> {
        let quick = self.quick.agents.iter().find(|a| a.name == agent);
        let cmd = match quick {
            Some(a) => a.command.replace("{prompt}", quoted),
            // Only agents the app knows, by plain name: anything else would run as a command.
            None if known_agent(self, agent) => format!("{agent} {quoted}"),
            None => {
                let mut names: Vec<String> = self.quick.agents.iter().map(|a| a.name.clone()).collect();
                names.extend(self.agent_defs().into_iter().map(|d| d.name));
                names.dedup();
                return Err(format!("unknown agent \"{agent}\"; use one of: {}", names.join(", ")));
            }
        };
        let cmd = match model {
            Some(model) => self.with_model(&cmd, quick.map_or("", |a| a.model_flag.as_str()), model),
            None => cmd,
        };
        Ok(self.as_account(agent, &cmd))
    }

    pub fn keymap(&self) -> Keymap {
        let mut warnings = Vec::new();
        let prefix = self.prefix.parse::<KeySpec>().unwrap_or_else(|e| {
            warnings.push(format!("prefix: {e}"));
            "ctrl+space".parse().unwrap()
        });
        let mut build = |defaults: &[(&str, &str)], user: &BTreeMap<String, String>| {
            let mut map: HashMap<KeySpec, Action> = HashMap::new();
            let mut order: Vec<KeySpec> = Vec::new();
            let pairs = if !self.keys.replace_defaults { defaults } else { Default::default() }
                .iter()
                .map(|(k, a)| (k.to_string(), a.to_string()))
                .chain(user.iter().map(|(k, a)| (k.clone(), a.clone())));
            for (k, a) in pairs {
                match (k.parse::<KeySpec>(), a.parse::<Action>()) {
                    (Ok(k), Ok(Action::None)) => {
                        map.remove(&k);
                    }
                    (Ok(k), Ok(a)) => {
                        if map.insert(k, a).is_none() {
                            order.push(k);
                        }
                    }
                    (Err(e), _) | (_, Err(e)) => warnings.push(format!("key `{k}`: {e}")),
                }
            }
            order.retain(|k| map.contains_key(k));
            (map, order)
        };
        let (prefixed, prefixed_order) = build(keys::DEFAULT_PREFIX_KEYS, &self.keys.prefix);
        let globals: Vec<(&str, &str)> = keys::DEFAULT_GLOBAL_KEYS.iter().chain(if self.ui.alt_arrows { keys::ALT_ARROW_KEYS } else { &[] }).copied().collect();
        let (global, _) = build(&globals, &self.keys.global);
        Keymap { prefix, prefixed, prefixed_order, global, warnings }
    }

    pub fn agent_defs(&self) -> Vec<CompiledAgent> {
        let mut defs: Vec<AgentDef> = if self.agents_replace_defaults { Vec::new() } else { builtin_agents() };
        for user in &self.agents {
            match defs.iter_mut().find(|d| d.name == user.name) {
                Some(d) => {
                    // Non-empty user fields replace the built-in ones.
                    if !user.process.is_empty() {
                        d.process = user.process.clone();
                    }
                    if !user.cmdline.is_empty() {
                        d.cmdline = user.cmdline.clone();
                    }
                    if !user.blocked_patterns.is_empty() {
                        d.blocked_patterns = user.blocked_patterns.clone();
                    }
                    if !user.working_patterns.is_empty() {
                        d.working_patterns = user.working_patterns.clone();
                    }
                    if user.resume.is_some() {
                        d.resume = user.resume.clone();
                    }
                    if user.resume_last.is_some() {
                        d.resume_last = user.resume_last.clone();
                    }
                    d.enabled = user.enabled;
                }
                None => defs.push(user.clone()),
            }
        }
        let re = |ps: &[String]| -> Vec<Regex> {
            ps.iter().filter_map(|p| Regex::new(&format!("(?i){p}")).ok()).collect()
        };
        defs.into_iter()
            .filter(|d| d.enabled != Some(false))
            .map(|d| CompiledAgent {
                builtin_process: !self.agents_replace_defaults && builtin_agents().iter().any(|a| a.name == d.name)
                    && !self.agents.iter().any(|a| a.name == d.name && !a.cmdline.is_empty()),
                builtin_screen: !self.agents_replace_defaults && builtin_agents().iter().any(|a| a.name == d.name)
                    && !self.agents.iter().any(|a| a.name == d.name && (!a.working_patterns.is_empty() || !a.blocked_patterns.is_empty())),
                process: d.process.iter().map(|p| p.to_ascii_lowercase()).collect(),
                cmdline: d.cmdline.clone(),
                blocked: re(&d.blocked_patterns),
                working: re(&d.working_patterns),
                resume: d.resume.filter(|s| !s.is_empty()),
                resume_last: d.resume_last.filter(|s| !s.is_empty()),
                name: d.name,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct CompiledAgent {
    pub name: String,
    pub builtin_screen: bool,
    pub builtin_process: bool,
    pub process: Vec<String>,
    pub cmdline: Vec<String>,
    pub blocked: Vec<Regex>,
    pub working: Vec<Regex>,
    pub resume: Option<String>,
    pub resume_last: Option<String>,
}

pub struct Keymap {
    pub prefix: KeySpec,
    pub prefixed: HashMap<KeySpec, Action>,
    /// Prefix bindings in definition order, for the which-key and help screens.
    pub prefixed_order: Vec<KeySpec>,
    pub global: HashMap<KeySpec, Action>,
    pub warnings: Vec<String>,
}

pub fn default_shell() -> String {
    if cfg!(windows) {
        for candidate in ["pwsh.exe", "powershell.exe"] {
            if which(candidate) {
                return candidate.into();
            }
        }
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }
}

fn which(exe: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(exe).is_file()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_check_table_reads_and_its_timeout_defaults() {
        let c: super::Config = toml::from_str("[[check]]\nproject = \"/work/app\"\ncommand = \"npm test\"\n\n[[check]]\nproject = \"/work/lib\"\ncommand = \"cargo test\"\ntimeout_s = 90\n").unwrap();
        super::validate_checks(&c.check).unwrap();
        let got: Vec<_> = c.check.iter().map(|c| (c.project.to_str().unwrap(), c.command.as_str(), c.timeout_s)).collect();
        assert_eq!(got, [("/work/app", "npm test", super::CHECK_TIMEOUT_S_DEFAULT), ("/work/lib", "cargo test", 90)]);
        assert_eq!(super::CHECK_TIMEOUT_S_DEFAULT, 600);
        assert!(super::Config::default().check.is_empty(), "no check unless the user names one");
    }

    #[test]
    fn a_retired_theme_moves_to_night_or_day() {
        for (old, now) in [("dark", "night"), ("walnut", "night"), ("seshi-night", "night"), ("light", "day"), ("vibecasso-day", "day")] {
            assert_eq!(retired_theme(old), Some(now), "{old}");
        }
        assert_eq!(retired_theme("tokyo-night"), None, "a theme of another name is kept");
        assert_eq!(retired_theme("harbor-day"), None);
    }

    #[test]
    fn a_check_that_could_never_run_fails_the_config_at_load() {
        let check = |toml: &str| -> Result<(), String> {
            let c: super::Config = toml::from_str(toml).map_err(|e| e.to_string())?;
            super::validate_checks(&c.check).map_err(|e| e.to_string())
        };
        assert!(check("[[check]]\nproject = \"/a\"\ncommand = \"t\"\n").is_ok());
        for (bad, why) in [
            ("[[check]]\ncommand = \"t\"\n", "project"),
            ("[[check]]\nproject = \"/a\"\ncommand = \"  \"\n", "command"),
            ("[[check]]\nproject = \"/a\"\n", "command"),
            ("[[check]]\nproject = \"/a\"\ncommand = \"t\"\ntimeout_s = 0\n", "timeout_s"),
            ("[[check]]\nproject = \"/a\"\ncommand = \"t\"\n[[check]]\nproject = \"/a\"\ncommand = \"t\"\n", "already has a check"),
            ("[[check]]\nproject = \"/a\"\ncommand = \"t\"\nname = \" \"\n", "empty `name`"),
            ("[[check]]\nproject = \"/a\"\ncommand = \"t\"\ntimeout_s = -5\n", "timeout_s"),
        ] {
            let err = check(bad).expect_err(bad);
            assert!(err.contains(why), "{bad:?} -> {err}");
        }
    }

    #[test]
    fn two_checks_for_one_project_are_found_however_the_path_is_spelled() {
        let check = |a: &str, b: &str| {
            let c: super::Config = toml::from_str(&format!("[[check]]\nproject = '{a}'\ncommand = \"t\"\n[[check]]\nproject = '{b}'\ncommand = \"t\"\n")).unwrap();
            super::validate_checks(&c.check)
        };
        let err = check("/a", "/a/").expect_err("a trailing slash is the same project").to_string();
        assert!(err.contains("already has a check"), "{err}");
        assert!(check("C:\\work\\App", "c:/work/app/").is_err(), "separators and case do not make a project");
        assert!(check("/a", "/b").is_ok());
    }

    #[test]
    fn a_project_may_have_several_checks_with_different_names() {
        let c: super::Config = toml::from_str(
            "[[check]]\nproject = \"/a\"\ncommand = \"cargo build\"\n[[check]]\nproject = \"/a\"\ncommand = \"cargo test\"\n[[check]]\nproject = \"/a\"\nname = \"lint\"\ncommand = \"cargo clippy\"\n",
        )
        .unwrap();
        super::validate_checks(&c.check).unwrap();
        assert_eq!(c.check.iter().map(super::CheckDef::name).collect::<Vec<_>>(), ["build", "test", "lint"]);
        let twice: super::Config = toml::from_str("[[check]]\nproject = \"/a\"\ncommand = \"npm test\"\n[[check]]\nproject = \"/a\"\nname = \"TEST\"\ncommand = \"cargo test\"\n").unwrap();
        let err = super::validate_checks(&twice.check).expect_err("a name is one checkpoint, whatever its case").to_string();
        assert!(err.contains("already has a check named"), "{err}");
    }

    #[test]
    fn a_checkpoint_is_named_by_its_command_s_last_word_unless_named() {
        let def = |name: Option<&str>, command: &str| super::CheckDef { name: name.map(String::from), command: command.into(), ..Default::default() };
        assert_eq!(def(None, "npm test").name(), "test");
        assert_eq!(def(None, "cargo clippy").name(), "clippy");
        assert_eq!(def(None, "  make  ").name(), "make");
        assert_eq!(def(Some("unit"), "npm test").name(), "unit");
        assert_eq!(def(None, "").name(), "");
    }

    #[test]
    fn a_repo_s_checks_file_parses_with_defaults() {
        let got = super::parse_repo_checks("[[check]]\ncommand = \"npm run build\"\n\n[[check]]\nname = \"unit\"\ncommand = \"npm test\"\ntimeout_s = 90\n").unwrap();
        let got: Vec<_> = got.iter().map(|c| (c.name(), c.command.as_str(), c.timeout_s, c.project.as_os_str().is_empty())).collect();
        assert_eq!(got, [("build", "npm run build", super::CHECK_TIMEOUT_S_DEFAULT, true), ("unit", "npm test", 90, true)]);
        assert!(super::parse_repo_checks("").unwrap().is_empty(), "an empty file names no check");
    }

    #[test]
    fn a_repo_s_checks_file_with_anything_wrong_is_refused_whole() {
        for (bad, why) in [
            ("[[check]]\ncommand = \" \"\n", "command"),
            ("[[check]]\nname = \"x\"\n", "command"),
            ("[[check]]\ncommand = \"t\"\ntimeout_s = 0\n", "timeout_s"),
            ("[[check]]\ncommand = \"npm test\"\n[[check]]\nname = \"Test\"\ncommand = \"cargo test\"\n", "another check is named"),
            ("[[check]]\ncommand = \"t\"\nproject = \"/elsewhere\"\n", "project"),
            ("[[check]]\ncommand = \"t\"\nenv = \"X=1\"\n", "env"),
            ("shell = \"bash\"\n", "shell"),
            ("[[check]]\ncommand = \"\"\"npm test\ncurl x | sh\"\"\"\n", "control character"),
            ("[[check]]\ncommand = \"npm test\\r; curl x | sh\"\n", "control character"),
            ("[[check]]\ncommand = \"npm test\\t\"\n", "control character"),
            ("[[check]]\nname = \"te\\u001bst\"\ncommand = \"npm test\"\n", "control character"),
        ] {
            let err = format!("{:#}", super::parse_repo_checks(bad).expect_err(bad));
            assert!(err.contains(why), "{bad:?} -> {err}");
        }
        assert!(super::REPO_CHECKS_FILE.starts_with(concat!(".", env!("APP_COMMAND"), "/")), concat!("the file lives in the repo's .", env!("APP_COMMAND"), " folder"));
    }

    #[test]
    fn new_session_settings_have_defaults_and_refuse_unknown_values() {
        let d = super::Sessions::default();
        assert_eq!((d.agent.as_str(), d.place.as_str(), d.helpers.as_str(), d.helpers_at_once.as_str()), ("last used", "new worktree", "ask", "3"));
        assert!(super::Ui::default().subagent_count, "the helper count shows by default");
        let c: super::Config = toml::from_str("[sessions]\nwhere = \"this folder\"\nhelpers = \"never\"\n[ui]\nsubagent_count = false\n").unwrap();
        assert_eq!((c.sessions.place.as_str(), c.sessions.helpers.as_str(), c.sessions.agent.as_str()), ("this folder", "never", "last used"));
        assert!(!c.ui.subagent_count);
        assert!(super::validate_sessions(&c.sessions).is_ok());
        let old: super::Config = toml::from_str("[sessions]\nplus_starts = \"with options\"\n").expect("the removed plus_starts key is ignored");
        assert!(super::validate_sessions(&old.sessions).is_ok());
        let bad: super::Config = toml::from_str("[sessions]\nhelpers_at_once = \"9\"\n").unwrap();
        let err = super::validate_sessions(&bad.sessions).unwrap_err().to_string();
        assert!(err.contains("sessions.helpers_at_once") && err.contains("2, 3, 4"), "{err}");
    }

    #[test]
    fn an_old_config_with_a_team_table_and_other_removed_keys_still_loads() {
        let old = "theme = \"nord\"\n[team]\nfamily = \"codex\"\n[[team.jobs]]\nname = \"planner\"\nclaude = \"opus\"\n[[team.projects]]\nproject = \"/a\"\nfamily = \"claude\"\n[fleet]\nsize = 3\n[mcp]\napprove = \"safe\"\nhelpers_at_once = 6\n[keys.prefix]\nstart-fleet = \"F\"\n";
        let c: super::Config = toml::from_str(old).expect("removed keys are ignored, not refused");
        assert_eq!((c.theme.as_str(), c.mcp.approve.as_str()), ("nord", "safe"));
    }

    #[test]
    fn the_old_mcp_helper_limit_is_the_sessions_limit_unless_that_is_set() {
        let read = |text: &str| {
            let mut t: toml::Table = toml::from_str(text).unwrap();
            super::fold_legacy_helpers(&mut t);
            toml::Value::Table(t).try_into::<super::Config>().unwrap().helpers_at_once()
        };
        assert_eq!(read(""), 3, "the default");
        assert_eq!(read("[sessions]\nhelpers_at_once = \"2\"\n"), 2);
        assert_eq!(read("[mcp]\nhelpers_at_once = 6\n"), 4, "the old key still counts, held to what Settings offers");
        assert_eq!(read("[mcp]\nhelpers_at_once = 0\n"), 4, "no limit is no longer offered");
        assert_eq!(read("[mcp]\nhelpers_at_once = 1\n"), 2);
        assert_eq!(read("[mcp]\nhelpers_at_once = 3\n"), 3);
        assert_eq!(read("[mcp]\nhelpers_at_once = 6\n[sessions]\nhelpers_at_once = \"3\"\n"), 3, "the new key wins, even at its default");
    }

    #[test]
    fn rounded_default_off_and_read_from_the_file() {
        assert!(!super::Ui::default().rounded);
        let c: super::Config = toml::from_str("[ui]
rounded = true
").unwrap();
        assert!(c.ui.rounded);
    }

    #[test]
    fn glyphs_default_to_auto_and_corners_to_square_with_flush_read_as_square() {
        let ui = super::Config::default().ui;
        assert_eq!((ui.glyphs.as_str(), ui.corners.as_str()), ("auto", "square"));
        for (written, read) in [("flush", "square"), ("square", "square"), ("rounded", "rounded")] {
            let c: super::Config = toml::from_str(&format!("[ui]\ncorners = \"{written}\"\nglyphs = \"nerd\"\n")).unwrap();
            assert_eq!((c.ui.corners.as_str(), c.ui.glyphs.as_str()), (read, "nerd"), "{written}");
        }
    }

    #[test]
    fn a_project_starting_with_a_tilde_is_refused_because_it_is_never_expanded() {
        let c: super::Config = toml::from_str("[[check]]\nproject = \"~/work/app\"\ncommand = \"t\"\n").unwrap();
        let err = super::validate_checks(&c.check).expect_err("~ is not expanded").to_string();
        assert!(err.contains("~") && err.contains("full path"), "{err}");
    }

    #[test]
    fn the_example_config_documents_the_check_keys() {
        for key in ["[[check]]", "project", "command", "timeout_s"] {
            assert!(super::EXAMPLE.lines().any(|l| l.starts_with("# ") && l.contains(key)), "{key}");
        }
    }

    #[test]
    fn a_model_goes_right_after_the_program_quoted_only_when_it_needs_it() {
        let c = super::Config { shell: Some("bash".into()), ..Default::default() };
        assert_eq!(c.with_model("claude 'fix it'", "", "sonnet"), "claude --model sonnet 'fix it'");
        assert_eq!(c.with_model("codex", "-m", "gpt-6.1-sol"), "codex -m gpt-6.1-sol");
        assert_eq!(c.with_model("claude 'x'", "--model", "a;rm -rf ~"), "claude --model 'a;rm -rf ~' 'x'");
        assert_eq!(c.with_model("claude", "", "it's"), "claude --model 'it'\\''s'");
    }

    #[test]
    fn an_agent_entry_with_an_env_is_an_account() {
        let toml = "[[quick.agents]]\nname = \"claude\"\ncommand = \"claude {prompt}\"\n\n[[quick.agents]]\nname = \"claude work\"\ncommand = \"claude {prompt}\"\nenv = { CLAUDE_CONFIG_DIR = \"~/.claude-work\" }\n";
        for (shell, prefix) in [("pwsh.exe", concat!(env!("APP_COMMAND"), " account 'claude work' -- ")), ("bash", concat!(env!("APP_COMMAND"), " account 'claude work' -- "))] {
            let mut c: super::Config = toml::from_str(toml).unwrap();
            c.shell = Some(shell.into());
            assert!(c.account("claude").is_none() && c.account("claude work").is_some(), "only the one with an env");
            // Its command says which account it is; the plain agent's is left alone.
            let cmd = c.as_account("claude work", "claude --resume abc");
            assert_eq!(cmd, format!("{prefix}claude --resume abc"), "{shell}");
            assert_eq!(c.as_account("claude", "claude 'hi'"), "claude 'hi'");
            assert_eq!(c.as_account("claude work", &cmd), cmd, "never wrapped twice");
            // And it can be read back off the command line.
            assert_eq!(c.split_account(&cmd), (Some("claude work".to_string()), "claude --resume abc"));
            assert_eq!(c.split_account("claude 'hi'"), (None, "claude 'hi'"));
        }
    }

    #[test]
    fn a_config_with_the_old_alert_settings_still_loads() {
        let old = "[notify]\nbell = true\ndesktop = false\nsound_needs = \"ping\"\nsound_done = \"off\"\nphone_after = 90\n";
        let c: super::Config = toml::from_str(old).expect("the settings that are gone are ignored");
        assert!(c.notify.sound && c.notify.phone_after == 90);
        let c: super::Config = toml::from_str("[notify]\nsound = false\n").unwrap();
        assert!(!c.notify.sound);
    }

    #[test]
    fn removed_profile_and_workflow_tables_are_ignored_without_rewriting_them() {
        let old = "[[profiles]]\nname='planner'\nagent='codex'\nmodel='gpt-5'\n\n[[workflows]]\nname='feature'\nplanner='planner'\nworker='worker'\nreviewer='reviewer'\n";
        let cfg: super::Config = toml::from_str(old).expect("legacy user settings remain readable");
        assert_eq!(cfg.theme, super::Config::default().theme);
        assert!(old.contains("[[profiles]]") && old.contains("[[workflows]]"), "loading does not edit the user's config text");
    }

    #[test]
    fn a_program_becomes_an_agent() {
        let args = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
        let native = super::agent_from_command("deepseek", &args("deepseek --model x")).unwrap();
        assert_eq!((native.name.as_str(), native.process.clone(), native.cmdline.is_empty()), ("deepseek", vec!["deepseek".to_string()], true), "by its name, whatever alias ran it");
        let npm = super::agent_from_command("node", &args("node /home/vox/.npm/lib/node_modules/@acme/deepseek-harness/dist/cli.js")).unwrap();
        assert_eq!((npm.name.as_str(), npm.process.is_empty(), npm.cmdline.clone()), ("deepseek-harness", true, vec!["deepseek-harness".to_string()]), "node: by its package");
        let py = super::agent_from_command("python3", &args("python3 -u /opt/harness/main.py")).unwrap();
        assert_eq!(py.name, "harness", "a main.py goes by its folder");
        assert!(super::agent_from_command("bash", &args("bash")).is_none(), "a shell isn't an agent");
        assert!(super::agent_from_command("node", &args("node")).is_none(), "nothing to go on");
        assert!(native.working_patterns.iter().any(|p| p == "esc to interrupt"));
    }

    #[test]
    fn adding_an_agent_keeps_the_rest_of_the_config() {
        let file = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-agents-{}.toml"), std::process::id()));
        std::fs::write(&file, "# mine\ntheme = \"default\"\n\n[ui]\nsplash = true\n\n[notify]\ndesktop = true\n").unwrap();
        let def = super::agent_from_command("dst", &["dst".to_string()]).unwrap();
        super::add_agent_to(&file, &def).unwrap();
        super::add_agent_to(&file, &def).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("# mine") && text.contains("theme = \"default\""), "the rest stays: {text}");
        assert_eq!(text.matches("[[agents]]").count(), 1, "the same name once: {text}");
        assert!(text.find("[[agents]]") > text.find("[notify]"), "added at the end: {text}");
        let cfg: super::Config = toml::from_str(&text).unwrap();
        assert!(cfg.agent_defs().iter().any(|a| a.name == "dst"), concat!("and ", env!("APP_COMMAND"), " reads it back"));
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn the_start_folder() {
        let mut cfg = super::Config::default();
        assert_eq!(cfg.start_dir(), None, concat!("empty: wherever you run ", env!("APP_COMMAND")));
        cfg.ui.start_dir = "~".into();
        let home = directories::BaseDirs::new().unwrap().home_dir().to_path_buf();
        assert_eq!(cfg.start_dir(), Some(home), "~ is home");
        let here = std::env::temp_dir();
        cfg.ui.start_dir = here.display().to_string();
        assert_eq!(cfg.start_dir(), Some(here));
        cfg.ui.start_dir = "/no/such/folder/anywhere".into();
        assert_eq!(cfg.start_dir(), None, "a folder that isn't there is ignored");
    }

    #[test]
    fn state_files_swap_in_and_keep_a_bad_copy() {
        let dir = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-state-{}"), std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("s.json");
        super::write_atomic(&f, "[1,2]").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "[1,2]");
        assert!(!dir.join(concat!(".s.json.", env!("APP_COMMAND"), "-tmp")).exists(), "no temporary file left behind");
        let v: Vec<u32> = super::read_state(&f);
        assert_eq!(v, vec![1, 2]);
        std::fs::write(&f, "[1,2").unwrap();
        let v: Vec<u32> = super::read_state(&f);
        assert!(v.is_empty(), "a broken file gives the defaults");
        assert_eq!(std::fs::read_to_string(dir.join("s.json.bak")).unwrap(), "[1,2", "and is kept as .bak");
        let _ = std::fs::remove_dir_all(&dir);
    }

    use super::*;

    #[test]
    fn an_old_install_is_brought_over_once() {
        let base = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-migrate-{}"), std::process::id()));
        let (old_cfg, new_cfg, old_data, new_data) = (base.join("cfg-hydra"), base.join(concat!("cfg-", env!("APP_COMMAND"))), base.join("data-hydra"), base.join(concat!("data-", env!("APP_COMMAND"))));
        std::fs::create_dir_all(old_cfg.join("ext")).unwrap();
        std::fs::create_dir_all(old_data.join("output-default")).unwrap();
        std::fs::write(old_cfg.join("config.toml"), "theme = \"hydra\"
shell = \"pwsh\"
").unwrap();
        std::fs::write(old_cfg.join("ext").join("x.toml"), "").unwrap();
        std::fs::write(old_data.join("hydra-ui.json"), "{}").unwrap();
        std::fs::write(old_data.join("session-default.json"), "[]").unwrap();
        std::fs::write(old_data.join("output-default").join("1.log"), "hi").unwrap();
        std::fs::write(old_data.join("hydra-me-default.sock.pid"), "123").unwrap();
        bring_over("hydra", Some((old_cfg.clone(), new_cfg.clone())), Some(&old_data), &new_data);
        let cfg = std::fs::read_to_string(new_cfg.join("config.toml")).unwrap();
        assert!(cfg.contains(&format!("theme = \"{}\"", crate::theme::DEFAULT)) && cfg.contains("shell = \"pwsh\""), "settings kept, the old default theme moved: {cfg}");
        assert!(new_cfg.join("ext").join("x.toml").exists(), "folders inside come too");
        assert!(new_data.join(concat!(env!("APP_COMMAND"), "-ui.json")).exists() && !new_data.join("hydra-ui.json").exists(), "files named after the old app take the new name");
        assert!(new_data.join("session-default.json").exists() && new_data.join("output-default").join("1.log").exists(), "saved sessions and pane history");
        assert!(!new_data.join(concat!(env!("APP_COMMAND"), "-me-default.sock.pid")).exists(), "not the old server's pid");
        assert!(new_data.join(MIGRATED).exists() && old_data.join("hydra-ui.json").exists(), "marked done; the old folders stay");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn local_settings_win() {
        let mut base: toml::Table = toml::from_str("theme = 'monokai'\n[ui]\nmouse = true\nsplash = true\n").unwrap();
        let over: toml::Table = toml::from_str("shell = 'zsh'\n[ui]\nsplash = false\n").unwrap();
        merge(&mut base, over);
        let cfg: Config = toml::Value::Table(base).try_into().unwrap();
        assert_eq!(cfg.shell.as_deref(), Some("zsh"));
        assert!(cfg.ui.mouse && !cfg.ui.splash, "tables merge key by key");
        assert_eq!(cfg.theme, "monokai");
    }

    #[test]
    fn example_config_parses_and_binds() {
        let c: Config = toml::from_str(EXAMPLE).expect("config.example.toml must parse");
        let km = c.keymap();
        assert!(km.warnings.is_empty(), "{:?}", km.warnings);
        assert!(!c.agent_defs().is_empty());
    }

    #[test]
    fn older_ui_configs_keep_their_look_under_its_new_name() {
        let kind = |toml: &str| toml::from_str::<Config>(toml).unwrap().ui.layout_kind();
        assert_eq!(kind("[ui]\npanes = 'tiled'"), LayoutKind::Tiled, "no layout line: Cards, tiled");
        assert_eq!(kind("[ui]\nlayout = 'cards'\npanes = 'tiled'"), LayoutKind::Tiled);
        assert_eq!(kind("[ui]\nlayout = 'cards'\npanes = 'floating'"), LayoutKind::Floating);
        assert_eq!(kind("[ui]\nlayout = 'cards'"), LayoutKind::Floating);
        assert_eq!(kind("[ui]\nlayout = 'flat'\npanes = 'floating'"), LayoutKind::Compact);
        assert_eq!(kind(""), LayoutKind::Floating, "the default");
        assert_eq!(kind("[ui]\nlayout = 'floating'\npanes = 'tiled'"), LayoutKind::Floating, "a new name wins over a leftover panes");
        assert_eq!(kind("[ui]\nlayout = 'compact'"), LayoutKind::Compact);
        assert_eq!(kind("[ui]\nlayout = 'grid'"), LayoutKind::Grid);
    }

    #[test]
    fn removed_visibility_settings_are_ignored() {
        let c: Config = toml::from_str("sleep_after = '15m'\nsettle_after = '3d'").expect("older config remains readable");
        assert_eq!(c.prefix, Config::default().prefix);
    }

    #[test]
    fn old_wake_animation_setting_is_ignored() {
        let c: Config = toml::from_str("[ui]\nwake_animation = 'off'").expect("older config remains readable");
        assert!(c.ui.motion);
    }

    #[test]
    fn user_keys_override_and_unbind() {
        let c: Config = toml::from_str(
            r#"
            [keys.prefix]
            x = "none"
            q = "close-pane"
            "#,
        )
        .unwrap();
        let km = c.keymap();
        assert!(!km.prefixed.contains_key(&"x".parse().unwrap()));
        assert_eq!(km.prefixed[&"q".parse().unwrap()], Action::ClosePane);
    }

    #[test]
    fn alt_arrows_move_between_panes_unless_turned_off() {
        let alt_left = "alt+left".parse().unwrap();
        assert_eq!(Config::default().keymap().global[&alt_left], Action::Focus(crate::layout::Dir::Left));
        let off: Config = toml::from_str("[ui]\nalt_arrows = false").unwrap();
        assert!(!off.keymap().global.contains_key(&alt_left), "off: the program gets them");
    }

    fn sh_config(quick: Vec<QuickAgent>) -> Config {
        Config { shell: Some("sh".into()), quick: Quick { agents: quick }, ..Default::default() }
    }

    fn quick(name: &str, command: &str, env: &[(&str, &str)]) -> QuickAgent {
        QuickAgent { name: name.into(), command: command.into(), env: env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), ..Default::default() }
    }

    #[test]
    fn a_quick_agent_gets_the_quoted_prompt_in_place_of_the_marker() {
        let c = sh_config(vec![quick("claude", "claude --fast {prompt}", &[])]);
        assert_eq!(c.agent_command(Some("claude"), None, "fix it's test").unwrap(), r"claude --fast 'fix it'\''s test'");
    }

    #[test]
    fn an_account_agent_runs_through_its_account() {
        let c = sh_config(vec![quick("work", "claude {prompt}", &[("CLAUDE_CONFIG_DIR", "~/.claude-work")])]);
        assert_eq!(c.agent_command(Some("work"), None, "go").unwrap(), concat!(env!("APP_COMMAND"), " account 'work' -- claude 'go'"));
    }

    #[test]
    fn powershell_doubles_curly_quotes_so_they_cannot_end_the_prompt() {
        for shell in ["pwsh.exe", "powershell"] {
            let c = Config { shell: Some(shell.into()), ..Default::default() };
            let q = c.quote_for_shell("don\u{2019}t; Write-Output INJECTED; # \u{2018}a\u{201A}b\u{201B} it's");
            assert_eq!(q, "'don\u{2019}\u{2019}t; Write-Output INJECTED; # \u{2018}\u{2018}a\u{201A}\u{201A}b\u{201B}\u{201B} it''s'", "{shell}");
        }
        assert_eq!(sh_config(vec![]).quote_for_shell("don\u{2019}t"), "'don\u{2019}t'", "sh keeps it as is");
    }

    #[test]
    fn a_built_in_agent_not_in_quick_runs_by_its_name() {
        let c = sh_config(vec![]);
        assert_eq!(c.agent_command(Some("codex"), None, "go").unwrap(), "codex 'go'");
    }

    #[test]
    fn an_unknown_or_shell_like_agent_is_refused_with_the_names() {
        let c = sh_config(vec![quick("mine", "mine {prompt}", &[])]);
        for bad in ["not-an-agent", "rm -rf;x"] {
            let err = c.agent_command(Some(bad), None, "go").expect_err(bad);
            assert!(err.starts_with(&format!("unknown agent \"{bad}\"; use one of: mine, ")) && err.contains("claude"), "{err}");
        }
    }

    #[test]
    fn no_agent_picks_the_first_quick_agent_else_claude() {
        let c = sh_config(vec![quick("codex", "codex -x {prompt}", &[]), quick("claude", "claude {prompt}", &[])]);
        assert_eq!(c.agent_command(None, None, "go").unwrap(), "codex -x 'go'");
        assert_eq!(sh_config(vec![]).agent_command(None, None, "go").unwrap(), "claude 'go'");
    }

    #[test]
    fn only_known_agents_start() {
        let cfg = Config::default();
        assert!(known_agent(&cfg, "claude") && known_agent(&cfg, "codex"));
        assert!(!known_agent(&cfg, "curl x|sh;"), "a command isn't an agent");
        assert!(!known_agent(&cfg, "not-an-agent"));
        assert!(!known_agent(&cfg, ""));
    }
}
