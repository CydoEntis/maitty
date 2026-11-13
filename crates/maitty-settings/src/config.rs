use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Space sizes offered in settings for `ui.padding` and `ui.gap`.
pub const SIZES: &[&str] = &["small", "medium", "large"];

/// Fade strengths offered in settings for `ui.dim`.
pub const DIMS: &[&str] = &["off", "light", "normal", "heavy"];

/// Convert current and legacy size names to their cell counts.
pub fn size_cells(size: &str) -> u16 {
    match size {
        "small" | "top" | "1" => 1,
        "medium" | "2" => 2,
        "large" => 3,
        "0" => 0,
        _ => 1,
    }
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
    /// The sidebar's group-by-folder button and a folder's heading (default: the Nerd Font folder).
    pub folder: String,
    /// The sidebar's group-by button (default: the Nerd Font filter funnel).
    pub filter: String,
}

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
            folder: "\u{f07b}".into(),
            filter: "\u{f0b0}".into(),
        }
    }
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

impl Default for Detection {
    fn default() -> Self {
        Detection {
            working_window_ms: 1500,
            echo_grace_ms: 300,
            scan_interval_ms: 1000,
            pattern_rows: 12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshots {
    /// Keep a checkout's files as they were each time an agent's turn begins and ends there,
    /// so a turn can be taken back from the agent's menu.
    pub enabled: bool,
}

impl Default for Snapshots {
    fn default() -> Self {
        Snapshots { enabled: true }
    }
}

/// How loud the alert sounds are (`notify.volume`), quietest first.
pub const VOLUMES: &[&str] = &["quiet", VOLUME_NORMAL, "loud"];
pub const VOLUME_NORMAL: &str = "normal";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Notify {
    /// Play the app's sounds at all. Off silences both of the ones below.
    pub sound: bool,
    /// The sound when an agent needs you (two rising notes).
    pub sound_on_needs: bool,
    /// The sound when an agent you are not looking at finishes (one soft note).
    pub sound_on_done: bool,
    /// How loud the sounds are: `quiet`, `normal` or `loud`.
    pub volume: String,
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

impl Default for Notify {
    fn default() -> Self {
        Notify {
            sound: true,
            sound_on_needs: true,
            sound_on_done: true,
            volume: VOLUME_NORMAL.into(),
            phone_topic: String::new(),
            phone_server: "https://ntfy.sh".into(),
            phone_after: 60,
            phone_done: false,
            phone_text: false,
        }
    }
}

/// The default padding leaves a row above the tabs for the terminal's own tab bar.
pub const PADDING_TOP: &str = "top";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui {
    pub sidebar: bool,
    /// "left" or "right". The desktop app's spelling, `sidebar_side`, is read too.
    #[serde(alias = "sidebar_side")]
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
    /// Alt with the leader's letter runs the everyday actions without it (`keys::ALT_LETTER_KEYS`:
    /// new shell, session and tab, close, split). Off: those keys go to the program.
    pub alt_keys: bool,
    /// Where plain `<app>` opens its first shell (and the one after you close everything).
    /// Empty: wherever you run the app. `~` is your home folder.
    pub start_dir: String,
    /// A session's tabs as lines under its sidebar row (`SIDEBAR_TABS`): "all", "busy" (those
    /// working or needing you, and the one on screen) or "off".
    pub sidebar_tabs: String,
    /// Legacy layout name, folded to Floating when loaded.
    pub layout: String,
    /// "solid" (the default) paints everything the theme's `bg`, so a plain terminal shows the
    /// theme whole; "terminal" leaves chrome on the terminal's own background (so transparency
    /// and a background image show through evenly).
    pub background: String,
    /// Older files' Cards spacing, "floating" or "tiled": read only while `layout` is unset or
    /// "cards".
    pub panes: String,
    /// Which glyph set marks sessions and checks: "dots" (a coloured dot and a word) or "unicode"
    /// (a shape per state). Older "auto", "ascii" and "nerd" read as "dots".
    pub glyphs: String,
    /// Space between floating cards: one of `SIZES` (or a legacy value before loading).
    pub gap: String,
    /// Space round the whole screen: one of `SIZES` (or a legacy value before loading).
    pub padding: String,
    /// How far unfocused panes fade toward their background: one of `DIMS`.
    pub dim: String,
    /// The focused card's border: "accent", "bright" or "none".
    pub focus_border: String,
    /// How round the shapes are: `full` (pills), `corners` (rounded boxes, square chips) or `off`.
    pub rounded: Rounding,
    /// What `fold_retired_layout` changed, to say once and write back: the retired look's name
    /// and the settings that stand for it now. Never read from or written to the file.
    #[serde(skip)]
    pub retired_layout: Option<RetiredLayout>,
    /// Show `· N agents` on a working session's row when the agent reports helpers.
    pub subagent_count: bool,
    /// The cursor's shape when the program in a pane doesn't ask for one: "block", "underline"
    /// or "line" (`CURSOR_SHAPES`). An older "terminal" reads as block and "bar" as line.
    pub cursor: String,
    /// That cursor blinks.
    pub cursor_blink: bool,
    /// What a new session opens as: "chat" or "terminal" (the desktop app's choice; the
    /// terminal app keeps it for it).
    pub new_session_view: maitty_shared::settings_view::NewSessionView,
    /// How the desktop window is drawn: "flat" or "glass".
    pub window_style: maitty_shared::settings_view::WindowStyle,
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
    pub pr_wake: bool,
}

impl Default for Sessions {
    fn default() -> Self {
        Sessions {
            agent: "last used".into(),
            place: "new worktree".into(),
            helpers: "ask".into(),
            helpers_at_once: "3".into(),
            pr_wake: true,
        }
    }
}

/// Refuse session preference values that are not offered by the settings UI.
pub fn validate_sessions(s: &Sessions) -> Result<()> {
    let one_of = |key: &str, value: &str, allowed: &[&str]| -> Result<()> {
        anyhow::ensure!(
            allowed.contains(&value),
            "sessions.{key} is \"{value}\"; use one of: {}",
            allowed.join(", ")
        );
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
    /// Resume chat conversations that were running when the server stopped, on the same
    /// provider session.
    pub conversations: bool,
    /// Tell a chat conversation resumed that way to continue where it left off. Off unless
    /// asked for: it sends the agent a message nobody typed.
    pub continue_turn: bool,
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
    /// A new branch's worktree starts from the repository's default branch (what `origin/HEAD`
    /// names, else `main` or `master`) rather than from whatever the main checkout has out.
    pub base_on_default: bool,
    /// A worktree with no commit and no file change for this many days is flagged inactive in
    /// the clean-up list. Nothing is removed for it. 0: never flagged.
    pub inactive_days: u32,
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
                "npm test",
                "npm run test",
                "npm run lint",
                "pnpm test",
                "yarn test",
                "cargo test",
                "cargo check",
                "cargo clippy",
                "pytest",
                "go test",
                "git status",
                "git diff",
                "git log",
            ]
            .map(String::from)
            .to_vec(),
            scope: "project".into(),
            grants: ["read", "write", "start"].map(String::from).to_vec(),
        }
    }
}

impl Default for Restore {
    fn default() -> Self {
        Restore {
            enabled: true,
            agents: true,
            conversations: true,
            continue_turn: false,
            commands: true,
            output: true,
        }
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
            base_on_default: false,
            inactive_days: 14,
        }
    }
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
        self.name
            .as_deref()
            .unwrap_or_else(|| self.command.split_whitespace().last().unwrap_or_default())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_s)
    }
}

impl Default for CheckDef {
    fn default() -> Self {
        Self {
            project: PathBuf::new(),
            name: None,
            command: String::new(),
            timeout_s: CHECK_TIMEOUT_S_DEFAULT,
        }
    }
}

/// Whether two paths name the same place, ignoring separator style, a trailing separator and case.
pub fn same_path(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    n(a) == n(b)
}

/// Refuse a `[[check]]` that could never run, or two of one name in a project, so a typo shows
/// when the config loads and not as a check that silently never happens.
pub fn validate_checks(checks: &[CheckDef]) -> Result<()> {
    for (i, c) in checks.iter().enumerate() {
        let n = i + 1;
        anyhow::ensure!(
            !c.project.as_os_str().is_empty(),
            "[[check]] #{n} needs a `project`"
        );
        validate_check(n, c, &c.project.display().to_string())?;
        anyhow::ensure!(
            !c.project.to_string_lossy().starts_with('~'),
            "[[check]] #{n}: `project` {} starts with `~`, which is not expanded; give the full path",
            c.project.display()
        );
        anyhow::ensure!(
            !checks[..i]
                .iter()
                .any(|o| same_path(&o.project, &c.project)
                    && o.name().eq_ignore_ascii_case(c.name())),
            "[[check]] #{n}: {} already has a check named `{}`; give each a different `name`",
            c.project.display(),
            c.name()
        );
    }
    Ok(())
}

/// The rules a check meets wherever it is named: in config or in a repo's file (`place`).
pub fn validate_check(n: usize, c: &CheckDef, place: &str) -> Result<()> {
    anyhow::ensure!(
        !c.command.trim().is_empty(),
        "[[check]] #{n} ({place}) needs a `command`"
    );
    anyhow::ensure!(
        c.timeout_s > 0,
        "[[check]] #{n} ({place}) needs a `timeout_s` above 0"
    );
    anyhow::ensure!(
        c.name.as_deref().is_none_or(|s| !s.trim().is_empty()),
        "[[check]] #{n} ({place}) has an empty `name`"
    );
    Ok(())
}

pub use maitty_machines::{
    LOCAL_MACHINE, MACHINE_NAME_MAX, MachineDef, VIA_NOT_ON_WINDOWS, VIA_PLACEHOLDER,
    validate_machines, validate_machines_on,
};

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
        Preset {
            name: String::new(),
            agent: "claude".into(),
            model: String::new(),
            prompt: String::new(),
            place: "beside".into(),
        }
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
        Recipe {
            name: String::new(),
            worktree: true,
            run: Vec::new(),
        }
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
            agents: vec![
                a(
                    "claude",
                    "claude {prompt}",
                    &["opus", "sonnet", "haiku"],
                    "--model",
                ),
                a("codex", "codex {prompt}", &[], "-m"),
            ],
        }
    }
}

/// How round the shapes are, all of them at once (`ui.rounded`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// Round card corners, and half-circle ends on every chip, button, row and hover (Nerd Font).
    Full,
    /// Rounded squares: round card corners, square ends everywhere else. No Nerd Font needed.
    Corners,
    /// Everything square.
    Off,
}

impl Serialize for Rounding {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Rounding::Full => "full",
            Rounding::Corners => "corners",
            Rounding::Off => "off",
        })
    }
}

impl Rounding {
    /// Settings' choices, in its order.
    pub const NAMES: &[&str] = &["full", "corners", "off"];

    /// Cards draw `╭╮╰╯`.
    pub fn corners(self) -> bool {
        self != Rounding::Off
    }

    /// Chips, buttons, titles, rows and the hover end in half-circles (pills, a Nerd Font glyph).
    pub fn caps(self) -> bool {
        self == Rounding::Full
    }
}

/// Older files wrote `rounded = true` / `false`: on is `corners`, off is `off`.
impl<'de> Deserialize<'de> for Rounding {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Switch(bool),
            Name(String),
        }
        match Written::deserialize(d)? {
            Written::Switch(on) => Ok(if on { Rounding::Corners } else { Rounding::Off }),
            Written::Name(name) => match name.as_str() {
                "full" => Ok(Rounding::Full),
                "corners" => Ok(Rounding::Corners),
                "off" => Ok(Rounding::Off),
                _ => Err(serde::de::Error::custom(format!(
                    "ui.rounded is \"{name}\"; use one of: {}",
                    Rounding::NAMES.join(", ")
                ))),
            },
        }
    }
}

/// `ui.sidebar_tabs`' choices, in Settings' order.
pub const SIDEBAR_TABS: &[&str] = &["all", "busy", "off"];

/// `ui.cursor`'s choices, in Settings' order.
pub const CURSOR_SHAPES: &[&str] = &["block", "underline", "line"];

/// Reject cursor names outside the shapes supported by the terminal client.
pub fn validate_cursor(ui: &Ui) -> Result<()> {
    anyhow::ensure!(
        CURSOR_SHAPES.contains(&ui.cursor.as_str()),
        "ui.cursor is \"{}\"; use one of: {}",
        ui.cursor,
        CURSOR_SHAPES.join(", ")
    );
    Ok(())
}

/// A retired look an older file named, and what stands for it now (`Ui::fold_retired_layout`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetiredLayout {
    /// The retired setting's name.
    pub name: &'static str,
    pub kind: LayoutKind,
    /// `gap = 0` stands for it (Tiled).
    pub gap: bool,
}

impl RetiredLayout {
    /// What the app says once when it starts with it.
    pub fn note(&self) -> String {
        let now = match (self.kind, self.gap) {
            (_, true) => "Floating with no gap",
            _ => "Floating",
        };
        format!(
            "The {} layout is now {now}. Change it in Settings › Appearance",
            self.name
        )
    }
}

/// The current layout and the legacy variant retained for migrations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Floating,
    /// No boxes: `│` / `─` hairlines, a title and rule per pane, and a status strip.
    Hairline,
}

/// Settings' Layout choices, in its order.
pub const LAYOUT_NAMES: &[&str] = &["floating", "hairline"];

impl LayoutKind {
    /// Its value in `ui.layout`.
    pub fn name(self) -> &'static str {
        match self {
            LayoutKind::Floating => "floating",
            LayoutKind::Hairline => "hairline",
        }
    }
}

impl Ui {
    /// Floating is the only supported look.
    pub fn layout_kind(&self) -> LayoutKind {
        if self.layout == "hairline" {
            LayoutKind::Hairline
        } else {
            LayoutKind::Floating
        }
    }

    /// Fold an older look into Floating while preserving its closest supported settings.
    pub fn fold_retired_layout(&mut self) {
        let tiled = self.layout == "tiled"
            || (matches!(self.layout.as_str(), "" | "cards") && self.panes == "tiled");
        let retired = if tiled {
            self.gap = "0".into();
            Some("Tiled")
        } else if self.layout == "grid" {
            Some("Grid")
        } else if matches!(self.layout.as_str(), "compact" | "flat") {
            Some("Compact")
        } else if self.layout == "hairline" {
            Some("Hairline")
        } else {
            None
        };
        let kind = LayoutKind::Floating;
        self.layout = kind.name().into();
        self.retired_layout = retired.map(|name| RetiredLayout {
            name,
            kind,
            gap: tiled,
        });
    }

    /// Convert older padding, gap, dim and glyph-set names to the values Settings now offers.
    pub fn fold_legacy_sizes(&mut self) {
        // Only Unicode is supported: the ASCII and Nerd sets, and `auto`, read as dots.
        if matches!(self.glyphs.as_str(), "ascii" | "nerd" | "auto") {
            self.glyphs = "dots".into();
        }
        match self.cursor.as_str() {
            "terminal" => self.cursor = "block".into(),
            "bar" => self.cursor = "line".into(),
            _ => {}
        }
        for size in [&mut self.padding, &mut self.gap] {
            match size.as_str() {
                "top" | "1" => *size = "small".into(),
                "2" => *size = "medium".into(),
                _ => {}
            }
        }
        match self.dim.as_str() {
            "subtle" => self.dim = "light".into(),
            "30%" | "40%" => self.dim = "normal".into(),
            "60%" => self.dim = "heavy".into(),
            _ => {}
        }
    }

    /// The drawing equivalent retained for the TUI's shared shape helpers.
    pub fn rounding(&self) -> Rounding {
        if self.layout_kind() == LayoutKind::Hairline {
            Rounding::Off
        } else {
            self.rounded
        }
    }
}

/// The reference frames' working spinner: three Braille bars, two columns wide.
pub const BARS_FRAMES: &[&str] = &["⣰⡄", "⣼⡀", "⣶⡆", "⣧⡇", "⣆⡆", "⣤⡄", "⣸⡆", "⣴⡇", "⣇⡄", "⣦⡀"];

pub const SPINNERS: &[(&str, &[&str])] = &[
    ("Braille", &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]),
    ("Dots", &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    ("Line (ASCII)", &["|", "/", "-", "\\"]),
    // A short body chasing round the cell, as the TUI design draws it.
    ("Snake", &["⠏", "⠛", "⠹", "⢸", "⣰", "⣤", "⣆", "⡇"]),
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
    (
        "Blocks",
        &[
            "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂",
        ],
    ),
    ("Bars", BARS_FRAMES),
];

impl Default for Ui {
    fn default() -> Self {
        Ui {
            sidebar: true,
            sidebar_position: "left".into(),
            which_key_delay_ms: 600,
            which_key: false,
            mouse: true,
            spinner: SPINNERS[0].1.iter().map(|frame| (*frame).into()).collect(),
            workspace_colors: [
                "#a593ff", "#5aa9ff", "#ff7ab6", "#3dd6c0", "#e8c565", "#ff9f6b", "#c792ea",
                "#7fd8a4",
            ]
            .map(String::from)
            .to_vec(),
            splash: true,
            splash_animation: "cold start".into(),
            update_check: true,
            motion: true,
            alt_arrows: true,
            alt_keys: true,
            start_dir: String::new(),
            sidebar_tabs: "all".into(),
            layout: String::new(),
            background: "solid".into(),
            panes: "floating".into(),
            glyphs: "auto".into(),
            gap: "small".into(),
            padding: "medium".into(),
            dim: "normal".into(),
            focus_border: "accent".into(),
            rounded: Rounding::Corners,
            retired_layout: None,
            subagent_count: true,
            cursor: "block".into(),
            cursor_blink: true,
            new_session_view: Default::default(),
            window_style: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentDef, CHECK_TIMEOUT_S_DEFAULT, CURSOR_SHAPES, CheckDef, DEFAULT_MODEL_FLAG, Detection,
        HELPERS_AT_ONCE, HELPERS_MAY, Icons, Keys, LayoutKind, Mcp, NEW_AGENTS, NEW_WHERE, Notify,
        Preset, Quick, Recipe, Restore, Rounding, Sessions, Snapshots, Ui, Worktree, same_path,
        size_cells, validate_checks, validate_cursor, validate_sessions,
    };

    #[test]
    fn display_detection_and_alert_settings_keep_their_defaults_and_serde_shape() {
        let icons = Icons::default();
        assert_eq!((icons.working.as_str(), icons.blocked.as_str()), ("⠹", "●"));
        assert_eq!(icons.branch, "\u{e0a0} ");

        let keys = Keys::default();
        assert!(keys.prefix.is_empty() && keys.global.is_empty());
        assert!(!keys.replace_defaults);

        let detection = Detection::default();
        assert_eq!(
            (
                detection.working_window_ms,
                detection.echo_grace_ms,
                detection.scan_interval_ms,
                detection.pattern_rows,
            ),
            (1500, 300, 1000, 12)
        );

        let snapshots = Snapshots::default();
        assert!(snapshots.enabled);

        let notify = Notify::default();
        assert!(notify.sound);
        assert_eq!(notify.phone_server, "https://ntfy.sh");
        assert_eq!(notify.phone_after, 60);
        assert!(!notify.phone_done && !notify.phone_text);

        let decoded: Notify = toml::from_str("sound = false").unwrap();
        assert!(!decoded.sound);
        assert_eq!(decoded.phone_server, "https://ntfy.sh");
    }

    #[test]
    fn agent_preferences_default_to_empty_and_accept_older_partial_toml() {
        let agent = AgentDef::default();
        assert!(agent.name.is_empty());
        assert!(agent.process.is_empty() && agent.cmdline.is_empty());
        assert!(agent.blocked_patterns.is_empty() && agent.working_patterns.is_empty());
        assert_eq!(agent.resume, None);
        assert_eq!(agent.resume_last, None);
        assert_eq!(agent.enabled, None);

        let decoded: AgentDef = toml::from_str("name = 'codex'").unwrap();
        assert_eq!(decoded.name, "codex");
        assert!(decoded.process.is_empty());
        assert!(decoded.cmdline.is_empty());
    }

    #[test]
    fn ui_defaults_keep_the_existing_appearance_and_cursor_settings() {
        let ui = Ui::default();

        assert_eq!(ui.rounded, Rounding::Corners, "the redesign rounds every box");
        assert_eq!(ui.background, "solid");
        assert_eq!(ui.cursor, "block");
        assert_eq!(ui.padding, "medium");
        assert_eq!(ui.gap, "small");
        assert_eq!(ui.dim, "normal");
        assert_eq!(ui.spinner.first().map(String::as_str), Some("⣾"));
    }

    #[test]
    fn a_config_that_still_names_a_removed_ui_key_loads() {
        let ui: Ui = toml::from_str("pane_colors = true\ntab_gap = false\ntool_rail = true\ngap = 'medium'")
            .expect("removed keys are ignored, not errors");
        assert_eq!(ui.gap, "medium", "and the keys around them are still read");
    }

    #[test]
    fn rounding_reads_legacy_booleans_and_named_values() {
        for (written, expected) in [
            ("true", Rounding::Corners),
            ("false", Rounding::Off),
            ("\"full\"", Rounding::Full),
            ("\"corners\"", Rounding::Corners),
            ("\"off\"", Rounding::Off),
        ] {
            let ui: Ui = toml::from_str(&format!("rounded = {written}")).unwrap();
            assert_eq!(ui.rounded, expected, "{written}");
        }

        assert!(toml::from_str::<Ui>("rounded = 'unknown'").is_err());
    }

    #[test]
    fn legacy_sizes_and_rounding_write_the_new_config_shape() {
        let mut ui: Ui =
            toml::from_str("padding = 'top'\ngap = '2'\ndim = 'subtle'\nrounded = 'full'").unwrap();
        ui.fold_legacy_sizes();
        assert_eq!(
            (ui.padding.as_str(), ui.gap.as_str(), ui.dim.as_str()),
            ("small", "medium", "light")
        );
        assert_eq!(ui.rounded, Rounding::Full);
        assert_eq!(size_cells("large"), 3);
        assert_eq!(size_cells("0"), 0);
        assert!(toml::to_string(&ui).unwrap().contains("rounded = \"full\""));
    }

    #[test]
    fn old_layout_names_fold_into_the_supported_looks() {
        let mut ui: Ui = toml::from_str("layout = 'tiled'").unwrap();
        ui.fold_retired_layout();
        assert_eq!(ui.layout_kind(), LayoutKind::Floating);
        assert_eq!(ui.gap, "0");
        assert_eq!(ui.retired_layout.map(|layout| layout.name), Some("Tiled"));

        let mut ui: Ui = toml::from_str("layout = 'compact'").unwrap();
        ui.fold_retired_layout();
        assert_eq!(ui.layout_kind(), LayoutKind::Floating);
        assert_eq!(ui.retired_layout.map(|layout| layout.name), Some("Compact"));

        let mut ui: Ui = toml::from_str("layout = 'hairline'").unwrap();
        ui.fold_retired_layout();
        assert_eq!(ui.layout_kind(), LayoutKind::Floating);
        assert_eq!(
            ui.retired_layout.map(|layout| layout.name),
            Some("Hairline")
        );
    }

    #[test]
    fn an_older_terminal_or_bar_cursor_reads_as_block_or_line() {
        for (old, now) in [("terminal", "block"), ("bar", "line"), ("underline", "underline")] {
            let mut ui: Ui = toml::from_str(&format!("cursor = '{old}'")).unwrap();
            ui.fold_legacy_sizes();
            assert_eq!(ui.cursor, now, "{old}");
            assert!(validate_cursor(&ui).is_ok());
        }
    }

    #[test]
    fn cursor_validation_accepts_supported_shapes_and_refuses_unknown_values() {
        for shape in CURSOR_SHAPES {
            let ui: Ui = toml::from_str(&format!("cursor = '{shape}'")).unwrap();
            assert!(validate_cursor(&ui).is_ok(), "{shape}");
        }

        let ui: Ui = toml::from_str("cursor = 'beam'").unwrap();
        assert!(validate_cursor(&ui).is_err());
    }

    #[test]
    fn session_defaults_and_choices_are_validated() {
        let defaults = Sessions::default();
        assert_eq!(defaults.agent, "last used");
        assert_eq!(defaults.place, "new worktree");
        assert_eq!(defaults.helpers, "ask");
        assert_eq!(defaults.helpers_at_once, "3");
        assert!(validate_sessions(&defaults).is_ok());

        for agent in NEW_AGENTS {
            let settings: Sessions = toml::from_str(&format!("agent = '{agent}'")).unwrap();
            assert!(validate_sessions(&settings).is_ok(), "{agent}");
        }
        for place in NEW_WHERE {
            let settings: Sessions = toml::from_str(&format!("where = '{place}'")).unwrap();
            assert!(validate_sessions(&settings).is_ok(), "{place}");
        }
        for helpers in HELPERS_MAY {
            let settings: Sessions = toml::from_str(&format!("helpers = '{helpers}'")).unwrap();
            assert!(validate_sessions(&settings).is_ok(), "{helpers}");
        }
        for count in HELPERS_AT_ONCE {
            let settings: Sessions =
                toml::from_str(&format!("helpers_at_once = '{count}'")).unwrap();
            assert!(validate_sessions(&settings).is_ok(), "{count}");
        }

        let invalid: Sessions = toml::from_str("helpers = 'sometimes'").unwrap();
        assert!(validate_sessions(&invalid).is_err());
    }

    #[test]
    fn a_config_can_turn_conversation_resume_off() {
        let restore: Restore = toml::from_str("conversations = false").unwrap();
        assert!(!restore.conversations);
        assert!(restore.agents, "the other restore settings keep their defaults");
    }

    #[test]
    fn continuing_a_cut_turn_is_off_until_a_config_turns_it_on() {
        assert!(!Restore::default().continue_turn);
        let restore: Restore = toml::from_str("continue_turn = true").unwrap();
        assert!(restore.continue_turn);
        assert!(restore.conversations, "the other restore settings keep their defaults");
    }

    #[test]
    fn persistence_worktree_and_mcp_settings_keep_their_defaults() {
        let restore = Restore::default();
        assert!(
            restore.enabled
                && restore.agents
                && restore.conversations
                && restore.commands
                && restore.output
        );

        let worktree = Worktree::default();
        assert_eq!(worktree.dir, "{repo_parent}/{repo}-worktrees/{branch}");
        assert!(worktree.per_agent && worktree.delete_with_last);
        assert!(worktree.command.is_empty() && worktree.prewarm.is_empty());
        assert_eq!(worktree.inactive_days, 14);

        let mcp = Mcp::default();
        assert_eq!(mcp.approve, "never");
        assert_eq!(mcp.scope, "project");
        assert_eq!(mcp.grants, ["read", "write", "start"]);
        assert!(mcp.safe.iter().any(|command| command == "cargo test"));
    }

    #[test]
    fn check_defaults_and_validation_reject_missing_or_duplicate_entries() {
        let default = CheckDef::default();
        assert!(default.project.as_os_str().is_empty());
        assert_eq!(default.timeout_s, CHECK_TIMEOUT_S_DEFAULT);
        assert_eq!(default.name(), "");

        let check = |project: &str, name: Option<&str>| CheckDef {
            project: project.into(),
            name: name.map(str::to_string),
            command: "cargo test".into(),
            timeout_s: 60,
        };
        assert!(validate_checks(&[check("repo", None)]).is_ok());
        assert!(validate_checks(&[check("repo", None), check("REPO\\", Some("test"))]).is_err());
        assert!(validate_checks(&[check("", None)]).is_err());
        assert!(same_path(
            std::path::Path::new("C:\\Repo\\"),
            std::path::Path::new("c:/repo")
        ));
    }

    #[test]
    fn recipes_presets_and_quick_agents_keep_their_defaults_and_prompt_rules() {
        let recipe = Recipe::default();
        assert!(recipe.worktree && recipe.run.is_empty());

        let preset = Preset::default();
        assert_eq!(
            (preset.agent.as_str(), preset.place.as_str()),
            ("claude", "beside")
        );
        assert!(preset.asks());
        assert_eq!(preset.fill("  fix the bug  "), "fix the bug");

        let named = Preset {
            prompt: "review {task}".into(),
            ..Preset::default()
        };
        assert_eq!(named.fill("the diff"), "review the diff");

        let quick = Quick::default();
        assert_eq!(quick.agents.len(), 2);
        assert_eq!(quick.agents[0].name, "claude");
        assert_eq!(quick.agents[1].name, "codex");
        assert_eq!(DEFAULT_MODEL_FLAG, "--model");
    }
}
