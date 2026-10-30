use anyhow::Result;
use serde::{Deserialize, Serialize};

/// The default padding leaves a row above the tabs for the terminal's own tab bar.
pub const PADDING_TOP: &str = "top";

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
    /// Alt with the leader's letter runs the everyday actions without it (`keys::ALT_LETTER_KEYS`:
    /// new shell, session and tab, close, split). Off: those keys go to the program.
    pub alt_keys: bool,
    /// Where plain `<app>` opens its first shell (and the one after you close everything).
    /// Empty: wherever you run the app. `~` is your home folder.
    pub start_dir: String,
    /// A session's tabs as lines under its sidebar row (`SIDEBAR_TABS`): "all", "busy" (those
    /// working or needing you, and the one on screen) or "off".
    pub sidebar_tabs: String,
    /// The look (ADR 0027): "floating" or "hairline". Read with `layout_kind`; an older file's
    /// look is folded in by `fold_retired_layout`. Empty means unset.
    pub layout: String,
    /// "solid" (the default) paints everything the theme's `bg`, so a plain terminal shows the
    /// theme whole; "terminal" leaves chrome on the terminal's own background (so transparency
    /// and a background image show through evenly).
    pub background: String,
    /// Older files' Cards spacing, "floating" or "tiled": read only while `layout` is unset or
    /// "cards".
    pub panes: String,
    /// Which glyph set marks sessions and checks: "auto" (ASCII when the blocked, done and idle
    /// icons are all ASCII, else Unicode), "unicode", "ascii" or "nerd".
    pub glyphs: String,
    /// Space between floating cards: "0", "1" or "2" rows stacked (and columns side by side).
    pub gap: String,
    /// Space round the whole screen, in every layout: "0", "1" or "2" rows (and columns), or
    /// `PADDING_TOP` (the default), a row above only.
    pub padding: String,
    /// How far unfocused panes fade toward their background: "off", "subtle", "30%", "60%" ("40%" from older configs counts as 30%).
    pub dim: String,
    /// The focused card's border: "accent", "bright" or "none".
    pub focus_border: String,
    /// One setting for every rounded shape (`Rounding`).
    pub rounded: Rounding,
    /// Floating: each pane's border in its own colour, by its place in the tab (ADR 0027).
    pub pane_colors: bool,
    /// What `fold_retired_layout` changed, to say once and write back: the retired look's name
    /// and the settings that stand for it now. Never read from or written to the file.
    #[serde(skip)]
    pub retired_layout: Option<RetiredLayout>,
    /// Show `· N agents` on a working session's row when the agent reports helpers.
    pub subagent_count: bool,
    /// The cursor's shape when the program in a pane doesn't ask for one: "terminal" (the
    /// terminal's own), "block", "underline" or "bar" (`CURSOR_SHAPES`).
    pub cursor: String,
    /// That cursor blinks.
    pub cursor_blink: bool,
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
        }
    }
}

/// How round the shapes are, all of them at once (`ui.rounded`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rounding {
    /// Round card corners, and half-circle ends on every chip, button, row and hover (Nerd Font).
    Full,
    /// Rounded squares: round card corners, square ends everywhere else. No Nerd Font needed.
    Corners,
    /// Everything square.
    Off,
}

impl Rounding {
    /// Settings' choices, in its order.
    pub const NAMES: &[&str] = &["full", "corners", "off"];

    /// Cards draw `╭╮╰╯`.
    pub fn corners(self) -> bool {
        self != Rounding::Off
    }

    /// Chips, buttons, rows and the hover end in half-circles.
    pub fn caps(self) -> bool {
        self == Rounding::Full
    }
}

/// Older files wrote `rounded = true` / `false`: on is `full`, off is `off`.
impl<'de> Deserialize<'de> for Rounding {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Switch(bool),
            Name(String),
        }
        match Written::deserialize(d)? {
            Written::Switch(on) => Ok(if on { Rounding::Full } else { Rounding::Off }),
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
pub const CURSOR_SHAPES: &[&str] = &["terminal", "block", "underline", "bar"];

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
    /// How Settings called it: "Tiled", "Grid" or "Compact".
    pub name: &'static str,
    pub kind: LayoutKind,
    /// `gap = 0` stands for it (Tiled).
    pub gap: bool,
    /// `pane_colors = true` stands for it (Grid).
    pub pane_colors: bool,
}

impl RetiredLayout {
    /// What the app says once when it starts with it.
    pub fn note(&self) -> String {
        let now = match (self.kind, self.gap, self.pane_colors) {
            (LayoutKind::Hairline, ..) => "Hairline, lines only",
            (_, true, _) => "Floating with no gap",
            (_, _, true) => "Floating with Pane colours on",
            _ => "Floating",
        };
        format!(
            "The {} layout is now {now}. Change it in Settings › Appearance",
            self.name
        )
    }
}

/// The two looks over the one session layout (ADR 0027).
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
    /// The look chosen; anything but "hairline" is Floating.
    pub fn layout_kind(&self) -> LayoutKind {
        if self.layout == "hairline" {
            LayoutKind::Hairline
        } else {
            LayoutKind::Floating
        }
    }

    /// An older file's look, kept as near as the two looks allow (ADR 0027): Tiled (and Cards
    /// with `panes = "tiled"`) is Floating with no gaps, Grid is Floating with pane colours,
    /// and Compact (once "flat") is Hairline. The file itself is not rewritten.
    pub fn fold_retired_layout(&mut self) {
        let tiled = self.layout == "tiled"
            || (matches!(self.layout.as_str(), "" | "cards") && self.panes == "tiled");
        let retired = if tiled {
            self.gap = "0".into();
            Some("Tiled")
        } else if self.layout == "grid" {
            self.pane_colors = true;
            Some("Grid")
        } else if matches!(self.layout.as_str(), "compact" | "flat") {
            Some("Compact")
        } else {
            None
        };
        let kind = if retired == Some("Compact") {
            LayoutKind::Hairline
        } else {
            self.layout_kind()
        };
        self.layout = kind.name().into();
        self.retired_layout = retired.map(|name| RetiredLayout {
            name,
            kind,
            gap: tiled,
            pane_colors: name == "Grid",
        });
    }

    /// The rounding drawn: `rounded`, except that Hairline is always square (ADR 0026).
    pub fn rounding(&self) -> Rounding {
        if self.layout_kind() == LayoutKind::Hairline {
            Rounding::Off
        } else {
            self.rounded
        }
    }
}

/// The reference frames' working spinner: three Braille bars, two columns wide. The ASCII glyph set
/// swaps it for `BARS_ASCII` (see `client::render::status_icon`).
pub const BARS_FRAMES: &[&str] = &["⣰⡄", "⣼⡀", "⣶⡆", "⣧⡇", "⣆⡆", "⣤⡄", "⣸⡆", "⣴⡇", "⣇⡄", "⣦⡀"];

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
            gap: "1".into(),
            padding: PADDING_TOP.into(),
            dim: "30%".into(),
            focus_border: "accent".into(),
            rounded: Rounding::Off,
            pane_colors: false,
            retired_layout: None,
            subagent_count: true,
            cursor: "terminal".into(),
            cursor_blink: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CURSOR_SHAPES, HELPERS_AT_ONCE, HELPERS_MAY, LayoutKind, Mcp, NEW_AGENTS, NEW_WHERE,
        Restore, Rounding, Sessions, Ui, Worktree, validate_cursor, validate_sessions,
    };

    #[test]
    fn ui_defaults_keep_the_existing_appearance_and_cursor_settings() {
        let ui = Ui::default();

        assert_eq!(ui.rounded, Rounding::Off);
        assert_eq!(ui.background, "solid");
        assert_eq!(ui.cursor, "terminal");
        assert_eq!(ui.padding, "top");
        assert_eq!(ui.spinner.first().map(String::as_str), Some("⣾"));
    }

    #[test]
    fn rounding_reads_legacy_booleans_and_named_values() {
        for (written, expected) in [
            ("true", Rounding::Full),
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
    fn old_layout_names_fold_into_the_supported_looks() {
        let mut ui: Ui = toml::from_str("layout = 'tiled'").unwrap();
        ui.fold_retired_layout();
        assert_eq!(ui.layout_kind(), LayoutKind::Floating);
        assert_eq!(ui.gap, "0");
        assert_eq!(ui.retired_layout.map(|layout| layout.name), Some("Tiled"));

        let mut ui: Ui = toml::from_str("layout = 'compact'").unwrap();
        ui.fold_retired_layout();
        assert_eq!(ui.layout_kind(), LayoutKind::Hairline);
        assert_eq!(ui.retired_layout.map(|layout| layout.name), Some("Compact"));
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
    fn persistence_worktree_and_mcp_settings_keep_their_defaults() {
        let restore = Restore::default();
        assert!(restore.enabled && restore.agents && restore.commands && restore.output);

        let worktree = Worktree::default();
        assert_eq!(worktree.dir, "{repo_parent}/{repo}-worktrees/{branch}");
        assert!(worktree.per_agent && worktree.delete_with_last);
        assert!(worktree.command.is_empty() && worktree.prewarm.is_empty());

        let mcp = Mcp::default();
        assert_eq!(mcp.approve, "never");
        assert_eq!(mcp.scope, "project");
        assert_eq!(mcp.grants, ["read", "write", "start"]);
        assert!(mcp.safe.iter().any(|command| command == "cargo test"));
    }
}
