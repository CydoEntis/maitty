//! Key specs ("ctrl+b", "alt+h", "%", "shift+tab"), bindable actions and the default keymap.

use anyhow::{Result, anyhow, bail};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventState, KeyModifiers};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeySpec {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl KeySpec {
    /// Normalise an incoming event so it compares equal to a parsed spec: for characters the
    /// shift state is already in the character itself ('N', '%'), so SHIFT is dropped.
    pub fn from_event(ev: &KeyEvent) -> Self {
        let mut mods =
            ev.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT);
        let mut code = ev.code;
        if let KeyCode::Char(c) = code {
            // Windows reports Caps Lock as an uppercase character without Shift; terminals
            // with enhanced keyboard reporting provide the lock state explicitly.
            if c.is_ascii_uppercase()
                && !mods.contains(KeyModifiers::SHIFT)
                && (cfg!(windows) || ev.state.contains(KeyEventState::CAPS_LOCK))
            {
                code = KeyCode::Char(c.to_ascii_lowercase());
            }
            mods.remove(KeyModifiers::SHIFT);
            if mods.contains(KeyModifiers::CONTROL) && c.is_ascii_uppercase() {
                code = KeyCode::Char(c.to_ascii_lowercase());
            }
            // Ctrl+Space, Ctrl+@ and Ctrl+2 are the same byte (NUL) on the wire; when it
            // arrives that way (ConPTY, SSH, nested sessions) Windows reports '@' or '2'.
            if mods.contains(KeyModifiers::CONTROL) && matches!(c, '@' | '2' | '\0') {
                code = KeyCode::Char(' ');
            }
        }
        if code == KeyCode::BackTab {
            mods.remove(KeyModifiers::SHIFT);
        }
        KeySpec { code, mods }
    }
}

impl KeySpec {
    /// The config-file spelling ("ctrl+b", "alt+space", "%").
    pub fn to_config(self) -> String {
        let mut s = String::new();
        if self.mods.contains(KeyModifiers::CONTROL) {
            s.push_str("ctrl+");
        }
        if self.mods.contains(KeyModifiers::ALT) {
            s.push_str("alt+");
        }
        if self.mods.contains(KeyModifiers::SHIFT) {
            s.push_str("shift+");
        }
        match self.code {
            KeyCode::Char(' ') => s.push_str("space"),
            KeyCode::Char(c) => s.push(c),
            KeyCode::F(n) => s.push_str(&format!("f{n}")),
            KeyCode::Enter => s.push_str("enter"),
            KeyCode::Tab => s.push_str("tab"),
            KeyCode::BackTab => s.push_str("shift+tab"),
            KeyCode::Backspace => s.push_str("backspace"),
            KeyCode::Left => s.push_str("left"),
            KeyCode::Right => s.push_str("right"),
            KeyCode::Up => s.push_str("up"),
            KeyCode::Down => s.push_str("down"),
            KeyCode::Home => s.push_str("home"),
            KeyCode::End => s.push_str("end"),
            KeyCode::PageUp => s.push_str("pageup"),
            KeyCode::PageDown => s.push_str("pagedown"),
            KeyCode::Insert => s.push_str("insert"),
            KeyCode::Delete => s.push_str("delete"),
            _ => s.push_str("ctrl+space"),
        }
        s
    }
}

impl FromStr for KeySpec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        // A lone "+" or a trailing "+" (e.g. "ctrl++") is the plus key itself.
        let (mod_part, key) = match s.strip_suffix("++") {
            Some(m) => (m, "+"),
            None if s == "+" => ("", "+"),
            None => match s.rsplit_once('+') {
                Some((m, k)) => (m, k),
                None => ("", s),
            },
        };
        let mut mods = KeyModifiers::NONE;
        for m in mod_part.split('+').filter(|m| !m.is_empty()) {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "c" | "control" => mods |= KeyModifiers::CONTROL,
                "alt" | "a" | "meta" | "m" | "opt" | "option" => mods |= KeyModifiers::ALT,
                "shift" | "s" => mods |= KeyModifiers::SHIFT,
                other => bail!("unknown modifier `{other}` in `{s}`"),
            }
        }
        let lower = key.to_ascii_lowercase();
        let code = match lower.as_str() {
            "enter" | "return" | "cr" => KeyCode::Enter,
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "esc" | "escape" => KeyCode::Esc,
            "space" => KeyCode::Char(' '),
            "backspace" | "bs" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "insert" | "ins" => KeyCode::Insert,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" => KeyCode::PageDown,
            f if f.len() > 1 && f.starts_with('f') && f[1..].parse::<u8>().is_ok() => {
                KeyCode::F(f[1..].parse()?)
            }
            _ => {
                let mut chars = key.chars();
                let c = chars.next().ok_or_else(|| anyhow!("empty key in `{s}`"))?;
                if chars.next().is_some() {
                    bail!("unknown key `{key}` in `{s}`");
                }
                KeyCode::Char(c)
            }
        };
        let mut spec = KeySpec { code, mods };
        // Express shift on characters through the character, matching `from_event`.
        if let KeyCode::Char(c) = spec.code {
            if spec.mods.contains(KeyModifiers::SHIFT) {
                spec.mods.remove(KeyModifiers::SHIFT);
                spec.code = KeyCode::Char(c.to_ascii_uppercase());
            }
            if spec.mods.contains(KeyModifiers::CONTROL) {
                spec.code = KeyCode::Char(c.to_ascii_lowercase());
            }
        }
        if spec.code == KeyCode::Tab && spec.mods.contains(KeyModifiers::SHIFT) {
            spec = KeySpec {
                code: KeyCode::BackTab,
                mods: spec.mods - KeyModifiers::SHIFT,
            };
        }
        Ok(spec)
    }
}

impl fmt::Display for KeySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mods.contains(KeyModifiers::CONTROL) {
            write!(f, "C-")?;
        }
        if self.mods.contains(KeyModifiers::ALT) {
            write!(f, "M-")?;
        }
        if self.mods.contains(KeyModifiers::SHIFT) {
            write!(f, "S-")?;
        }
        match self.code {
            KeyCode::Char(' ') => write!(f, "Space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::F(n) => write!(f, "F{n}"),
            KeyCode::BackTab => write!(f, "S-Tab"),
            other => write!(f, "{other:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    SplitRight,
    SplitDown,
    SplitLeft,
    SplitUp,
    /// Split and run a command in the new pane: `spawn-right:claude`.
    Spawn(crate::layout::Dir, String),
    /// New tab running a command: `spawn-tab:lazygit`.
    SpawnTab(String),
    ClosePane,
    Focus(crate::layout::Dir),
    FocusNext,
    FocusPrev,
    Resize(crate::layout::Dir),
    /// Trade places with the neighbouring pane that way.
    Swap(crate::layout::Dir),
    NewTab,
    NextTab,
    PrevTab,
    CloseTab,
    RenameTab,
    SelectTab(usize),
    NewWorkspace,
    CloseWorkspace,
    RenameWorkspace,
    ToggleSidebar,
    /// Hide the sessions completely (Shift+Z); the same key brings the sidebar back.
    HideSidebar,
    ScrollUp,
    ScrollDown,
    CopyMode,
    Search,
    /// Prompt for a branch, create a worktree, open it; optionally run a command there.
    NewWorktree(Option<String>),
    RemoveWorktree,
    /// Search every command, workspace, agent and pane.
    Palette,
    Settings,
    /// Move through the sidebar tree with the arrow keys.
    BrowseTree,
    /// Recent and project files; Enter puts the path in the prompt.
    Files,
    /// A new session straight away with the defaults in Settings › Sessions (`n`, `+`).
    StartSession,
    /// The New Session picker with nothing typed: open projects and folders on every machine.
    PickSession,
    /// The selected worktree's changes (review and ship).
    Changes,
    /// Interrupt the selected worktree's agent (Ctrl+C).
    StopAgent,
    /// A quick follow-up to the agent you're on: its next prompt, without going there.
    Message,
    /// Sessions that need you, then finished ones, across every project.
    Jump,
    /// A new agent session in the sidebar cursor's worktree.
    NewSession,
    /// Close the split (both sessions keep running).
    CloseSplit,
    /// Save the clipboard's image to a file and paste its path (agents attach it).
    PasteImage,
    /// Send a file from this computer to the machine the pane is on, and paste its path there;
    /// pressed again while a file is being sent, stop it (ADR 0055).
    SendFile,
    /// Find a file (0) or search the code (1).
    Find(u8),
    AgentStatusSetup,
    /// Switch the branch of the checkout you're in.
    Branches,
    /// The project's reference images and design files, in the side panel.
    References,
    /// A shell in the folder of the session you're on, straight away.
    ShellHere,
    /// Scroll to the command before (or after) the one at the top of the view.
    PrevPrompt,
    NextPrompt,
    /// What happened lately: who finished, who asked, messages.
    History,
    /// The computers reached over ssh: how each stands, and adding or removing one.
    Machines,
    /// Move the sidebar cursor (-1 up, 1 down); bare keys then work on the sidebar.
    SideMove(i8),
    Detach,
    Help,
    ReloadConfig,
    /// Say how the pane you're on takes the mouse and how much history it has.
    PaneInfo,
    SendPrefix,
    KillServer,
    /// Install the newest the app without interrupting running sessions.
    Update,
    /// Ask now whether a newer the app is out, without waiting for the next check.
    CheckUpdate,
    /// Replace the running server and this window with the installed the app.
    Restart,
    /// Switch between the dark theme and the light one.
    ToggleAppearance,
    /// The clean-up list for the worktrees of the project you're in.
    TidyWorktrees,
    /// The menu: the common commands as a list (the sidebar's "menu").
    Menu,
    /// The worktrees step of the key map: new, switch, merge, delete.
    Worktrees,
    /// Explicitly unbind a default.
    None,
}

impl Action {
    pub fn describe(&self) -> String {
        use crate::layout::Dir::*;
        let dir = |d: &crate::layout::Dir| match d {
            Left => "left",
            Right => "right",
            Up => "up",
            Down => "down",
        };
        match self {
            Action::SplitRight => "Split side by side".into(),
            Action::SplitDown => "Split stacked".into(),
            Action::SplitLeft => "Split left".into(),
            Action::SplitUp => "Split up".into(),
            Action::Spawn(d, c) => format!("Run `{c}` {}", dir(d)),
            Action::SpawnTab(c) => format!("Run `{c}` in a new tab"),
            Action::ClosePane => "Close pane".into(),
            Action::Focus(d) => format!("Focus the pane {}", dir(d)),
            Action::FocusNext => "Next pane".into(),
            Action::FocusPrev => "Previous pane".into(),
            Action::Resize(d) => format!("Resize the pane {}", dir(d)),
            Action::Swap(d) => format!("Swap with the pane {}", dir(d)),
            Action::NewTab => "New tab".into(),
            Action::NextTab => "Next tab".into(),
            Action::PrevTab => "Previous tab".into(),
            Action::CloseTab => "Close tab".into(),
            Action::RenameTab => "Rename session".into(),
            Action::SelectTab(n) => format!("Go to tab {n}"),
            Action::NewWorkspace => "New group".into(),
            Action::CloseWorkspace => "Close session and its tabs".into(),
            Action::RenameWorkspace => "Rename selected session".into(),
            Action::ToggleSidebar => "Collapse sidebar".into(),
            Action::HideSidebar => "Hide sidebar".into(),
            Action::ScrollUp => "Scroll up".into(),
            Action::ScrollDown => "Scroll down".into(),
            Action::CopyMode => "Select text with the keyboard".into(),
            Action::Search => "Search history".into(),
            Action::NewWorktree(None) => "New worktree".into(),
            Action::NewWorktree(Some(c)) => format!("Worktrees running `{c}`"),
            Action::RemoveWorktree => "Remove worktree".into(),
            Action::Palette => "Search commands".into(),
            Action::Settings => "Settings".into(),
            Action::BrowseTree => "Focus the sidebar".into(),
            Action::Files => "Files".into(),
            Action::StartSession => "New session".into(),
            Action::PickSession => "New session anywhere".into(),
            Action::Changes => "Changes".into(),
            Action::StopAgent => "Stop agent".into(),
            Action::Message => "Message this session".into(),
            Action::Jump => "Inbox".into(),
            Action::NewSession => "New session beside this one".into(),
            Action::CloseSplit => "Close the split".into(),
            Action::PasteImage => "Paste the clipboard image".into(),
            Action::SendFile => "Send a file to the pane's machine".into(),
            Action::AgentStatusSetup => "Connect agents".into(),
            Action::Branches => "Branch".into(),
            Action::References => "References".into(),
            Action::ShellHere => "New shell here".into(),
            Action::PrevPrompt => "Previous command in history".into(),
            Action::NextPrompt => "Next command in history".into(),
            Action::History => "What happened".into(),
            Action::Machines => "Machines".into(),
            Action::Find(0) => "Find a file".into(),
            Action::Find(_) => "Search the code".into(),
            Action::SideMove(d) if *d < 0 => "Sidebar up".into(),
            Action::SideMove(_) => "Sidebar down".into(),
            Action::Detach => "Quit, agents keep running".into(),
            Action::Help => "All keys".into(),
            Action::ReloadConfig => "Reload config".into(),
            Action::PaneInfo => "Pane info".into(),
            Action::SendPrefix => "Send the leader key to the pane".into(),
            Action::KillServer => "Kill server".into(),
            Action::Update => concat!("Update ", env!("APP_COMMAND")).into(),
            Action::CheckUpdate => "Check for updates".into(),
            Action::Restart => concat!("Restart ", env!("APP_COMMAND")).into(),
            Action::ToggleAppearance => "Switch dark or light theme".into(),
            Action::TidyWorktrees => "Clean up worktrees".into(),
            Action::Menu => "Menu".into(),
            Action::Worktrees => "Worktrees…".into(),
            Action::None => "Unbound".into(),
        }
    }
}

impl Action {
    /// The config-file spelling of an action (`"split-right"`), the inverse of `parse`.
    pub fn to_config(&self) -> Option<String> {
        use crate::layout::Dir::*;
        let d = |d: &crate::layout::Dir| match d {
            Left => "left",
            Right => "right",
            Up => "up",
            Down => "down",
        };
        Some(match self {
            Action::SplitRight => "split-right".into(),
            Action::SplitDown => "split-down".into(),
            Action::SplitLeft => "split-left".into(),
            Action::SplitUp => "split-up".into(),
            Action::Spawn(dir, c) => format!("spawn-{}:{c}", d(dir)),
            Action::SpawnTab(c) => format!("spawn-tab:{c}"),
            Action::ClosePane => "close-pane".into(),
            Action::Focus(dir) => format!("focus-{}", d(dir)),
            Action::FocusNext => "focus-next".into(),
            Action::FocusPrev => "focus-prev".into(),
            Action::Resize(dir) => format!("resize-{}", d(dir)),
            Action::Swap(dir) => format!("swap-{}", d(dir)),
            Action::NewTab => "new-tab".into(),
            Action::NextTab => "next-tab".into(),
            Action::PrevTab => "prev-tab".into(),
            Action::CloseTab => "close-tab".into(),
            Action::RenameTab => "rename-tab".into(),
            Action::SelectTab(n) => format!("select-tab-{n}"),
            Action::NewWorkspace => "new-workspace".into(),
            Action::CloseWorkspace => "close-workspace".into(),
            Action::RenameWorkspace => "rename-workspace".into(),
            Action::ToggleSidebar => "toggle-sidebar".into(),
            Action::HideSidebar => "hide-sidebar".into(),
            Action::ScrollUp => "scroll-up".into(),
            Action::ScrollDown => "scroll-down".into(),
            Action::CopyMode => "copy-mode".into(),
            Action::Search => "search".into(),
            Action::NewWorktree(None) => "new-worktree".into(),
            Action::NewWorktree(Some(c)) => format!("new-worktree:{c}"),
            Action::RemoveWorktree => "remove-worktree".into(),
            Action::Palette => "palette".into(),
            Action::Settings => "settings".into(),
            Action::BrowseTree => "browse-tree".into(),
            Action::Files => "files".into(),
            Action::StartSession => "start-session".into(),
            Action::PickSession => "pick-session".into(),
            Action::Jump => "jump".into(),
            Action::NewSession => "new-beside".into(),
            Action::CloseSplit => "close-split".into(),
            Action::PasteImage => "paste-image".into(),
            Action::SendFile => "send-file".into(),
            Action::AgentStatusSetup => "agent-status-setup".into(),
            Action::Branches => "branch".into(),
            Action::References => "references".into(),
            Action::ShellHere => "shell-here".into(),
            Action::PrevPrompt => "prev-prompt".into(),
            Action::NextPrompt => "next-prompt".into(),
            Action::History => "history".into(),
            Action::Machines => "machines".into(),
            Action::Find(0) => "find-file".into(),
            Action::Find(_) => "search-code".into(),
            Action::SideMove(d) if *d < 0 => "side-up".into(),
            Action::SideMove(_) => "side-down".into(),
            Action::Changes => "changes".into(),
            Action::StopAgent => "stop-agent".into(),
            Action::Message => "message".into(),
            Action::Detach => "detach".into(),
            Action::Help => "help".into(),
            Action::ReloadConfig => "reload-config".into(),
            Action::PaneInfo => "pane-info".into(),
            Action::SendPrefix => "send-prefix".into(),
            Action::KillServer => "kill-server".into(),
            Action::Update => "update".into(),
            Action::CheckUpdate => "check-update".into(),
            Action::Restart => "restart".into(),
            Action::ToggleAppearance => "toggle-appearance".into(),
            Action::TidyWorktrees => "tidy-worktrees".into(),
            Action::Menu => "menu".into(),
            Action::Worktrees => "worktrees".into(),
            Action::None => return None,
        })
    }
}

impl FromStr for Action {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        use crate::layout::Dir::*;
        if let Some((verb, arg)) = s.split_once(':') {
            let arg = arg.trim().to_string();
            return Ok(match verb {
                "spawn-right" => Action::Spawn(Right, arg),
                "spawn-down" => Action::Spawn(Down, arg),
                "spawn-left" => Action::Spawn(Left, arg),
                "spawn-up" => Action::Spawn(Up, arg),
                "spawn-tab" => Action::SpawnTab(arg),
                "new-worktree" => Action::NewWorktree(Some(arg).filter(|a| !a.is_empty())),
                "select-tab" => Action::SelectTab(arg.parse()?),
                _ => bail!("unknown action `{s}`"),
            });
        }
        if let Some(n) = s.strip_prefix("select-tab-") {
            return Ok(Action::SelectTab(n.parse()?));
        }
        Ok(match s {
            "split-right" => Action::SplitRight,
            "split-down" => Action::SplitDown,
            "split-left" => Action::SplitLeft,
            "split-up" => Action::SplitUp,
            "close-pane" => Action::ClosePane,
            "focus-left" => Action::Focus(Left),
            "focus-right" => Action::Focus(Right),
            "focus-up" => Action::Focus(Up),
            "focus-down" => Action::Focus(Down),
            "focus-next" => Action::FocusNext,
            "focus-prev" => Action::FocusPrev,
            "resize-left" => Action::Resize(Left),
            "resize-right" => Action::Resize(Right),
            "resize-up" => Action::Resize(Up),
            "resize-down" => Action::Resize(Down),
            "swap-left" => Action::Swap(Left),
            "swap-right" => Action::Swap(Right),
            "swap-up" => Action::Swap(Up),
            "swap-down" => Action::Swap(Down),
            "new-tab" => Action::NewTab,
            "next-tab" => Action::NextTab,
            "prev-tab" => Action::PrevTab,
            "close-tab" => Action::CloseTab,
            "rename-tab" => Action::RenameTab,
            "new-workspace" => Action::NewWorkspace,
            "close-workspace" => Action::CloseWorkspace,
            "rename-workspace" => Action::RenameWorkspace,
            "toggle-sidebar" => Action::ToggleSidebar,
            "hide-sidebar" => Action::HideSidebar,
            "scroll-up" => Action::ScrollUp,
            "scroll-down" => Action::ScrollDown,
            "copy-mode" => Action::CopyMode,
            "search" => Action::Search,
            "new-worktree" => Action::NewWorktree(None),
            "remove-worktree" => Action::RemoveWorktree,
            "palette" | "command-palette" => Action::Palette,
            "settings" => Action::Settings,
            "browse-tree" | "browse" => Action::BrowseTree,
            "files" => Action::Files,
            // The options dialog these named is gone; a saved binding stays loadable and does nothing.
            "new-pane" | "new" => Action::None,
            "start-session" => Action::StartSession,
            "pick-session" => Action::PickSession,
            "jump" => Action::Jump,
            "new-session" | "new-beside" => Action::NewSession,
            "close-split" => Action::CloseSplit,
            "paste-image" => Action::PasteImage,
            "send-file" => Action::SendFile,
            // Keep old saved bindings loadable after the removed workflow sheets; send
            // them to the command palette so stale config never prevents startup.
            "profiles" | "workflows" | "workflow-runs" => Action::Palette,
            "agent-status-setup" => Action::AgentStatusSetup,
            "branch" | "switch-branch" => Action::Branches,
            "references" => Action::References,
            "shell-here" => Action::ShellHere,
            "prev-prompt" => Action::PrevPrompt,
            "next-prompt" => Action::NextPrompt,
            "history" => Action::History,
            "machines" => Action::Machines,
            "find-file" => Action::Find(0),
            "search-code" => Action::Find(1),
            "side-up" => Action::SideMove(-1),
            "side-down" => Action::SideMove(1),
            "changes" => Action::Changes,
            "stop-agent" => Action::StopAgent,
            "message" | "follow-up" => Action::Message,
            "detach" => Action::Detach,
            "help" => Action::Help,
            "reload-config" => Action::ReloadConfig,
            "pane-info" => Action::PaneInfo,
            "send-prefix" => Action::SendPrefix,
            "kill-server" => Action::KillServer,
            "update" => Action::Update,
            "check-update" => Action::CheckUpdate,
            "restart" => Action::Restart,
            "toggle-appearance" => Action::ToggleAppearance,
            "tidy-worktrees" => Action::TidyWorktrees,
            "menu" | "actions" => Action::Menu,
            "worktrees" => Action::Worktrees,
            // The fleet and the why popover are gone; a binding written for one stays
            // harmless, not a config error.
            "none" | "unbind" | "start-fleet" | "why" => Action::None,
            _ => bail!("unknown action `{s}`"),
        })
    }
}

/// Bindings in shortcut mode (after the leader), ADR 0017: one key each, none needing Shift
/// but `?` for all keys (ADR 0022), chosen from what other apps use. Copy mode, hide sidebar,
/// prompt jumps, new worktree, rename workspace and send-prefix have no default key; search
/// commands finds them.
pub const DEFAULT_PREFIX_KEYS: &[(&str, &str)] = &[
    // Sessions
    ("n", "shell-here"),
    ("s", "start-session"),
    ("o", "pick-session"),
    ("enter", "message"),
    ("k", "stop-agent"),
    ("i", "jump"),
    // Panes
    ("left", "focus-left"),
    ("down", "focus-down"),
    ("up", "focus-up"),
    ("right", "focus-right"),
    ("v", "split-down"),
    ("h", "split-right"),
    ("x", "close-pane"),
    ("ctrl+left", "resize-left"),
    ("ctrl+down", "resize-down"),
    ("ctrl+up", "resize-up"),
    ("ctrl+right", "resize-right"),
    ("alt+left", "swap-left"),
    ("alt+down", "swap-down"),
    ("alt+up", "swap-up"),
    ("alt+right", "swap-right"),
    ("pageup", "scroll-up"),
    ("pagedown", "scroll-down"),
    ("z", "toggle-sidebar"),
    // Tabs
    ("t", "new-tab"),
    ("ctrl+w", "close-tab"),
    ("r", "rename-tab"),
    ("]", "next-tab"),
    ("[", "prev-tab"),
    ("1", "select-tab-1"),
    ("2", "select-tab-2"),
    ("3", "select-tab-3"),
    ("4", "select-tab-4"),
    ("5", "select-tab-5"),
    ("6", "select-tab-6"),
    ("7", "select-tab-7"),
    ("8", "select-tab-8"),
    ("9", "select-tab-9"),
    // Project
    ("f", "files"),
    ("e", "browse-tree"),
    ("p", "find-file"),
    ("/", "search-code"),
    ("c", "changes"),
    ("g", "worktrees"),
    ("b", "branch"),
    ("a", "references"),
    // The app
    ("space", "palette"),
    ("m", "menu"),
    (",", "settings"),
    ("?", "help"),
    ("l", "history"),
    ("w", "machines"),
    ("ctrl+v", "paste-image"),
    ("d", "send-file"),
    ("q", "detach"),
    ("ctrl+r", "reload-config"),
];

/// Bindings active without the prefix: Alt+1–9 go to a tab. With `ALT_ARROW_KEYS` and
/// `ALT_LETTER_KEYS` (each with its own switch) these are all that is taken from programs.
pub const DEFAULT_GLOBAL_KEYS: &[(&str, &str)] = &[
    ("alt+1", "select-tab-1"),
    ("alt+2", "select-tab-2"),
    ("alt+3", "select-tab-3"),
    ("alt+4", "select-tab-4"),
    ("alt+5", "select-tab-5"),
    ("alt+6", "select-tab-6"),
    ("alt+7", "select-tab-7"),
    ("alt+8", "select-tab-8"),
    ("alt+9", "select-tab-9"),
];

/// The everyday actions without the leader (`ui.alt_keys`), on no key an agent uses: Claude
/// Code takes Alt+v (paste an image, on Windows) and Alt+t (thinking), so the splits are Alt+s
/// (side by side) and Alt+d (down), a new session (an agent) is Alt+a, and a
/// new tab has none. No Alt+[ or Alt+]: terminals send those as the start of an escape sequence.
pub const ALT_LETTER_KEYS: &[(&str, &str)] = &[
    ("alt+n", "shell-here"),
    ("alt+a", "start-session"),
    ("alt+x", "close-pane"),
    ("alt+s", "split-right"),
    ("alt+d", "split-down"),
];

/// Moving between panes without the leader (`ui.alt_arrows`).
pub const ALT_ARROW_KEYS: &[(&str, &str)] = &[
    ("alt+left", "focus-left"),
    ("alt+down", "focus-down"),
    ("alt+up", "focus-up"),
    ("alt+right", "focus-right"),
];

/// Keys plain VT can't express exactly (Ctrl+Shift+letter, Ctrl+Enter, Ctrl+Tab, Ctrl with
/// symbols) — sent as Windows key records when ConPTY asked for them. Everything else keeps
/// its usual encoding (so Claude's Shift+Enter, Ctrl+C and friends behave as before).
pub fn wants_win32(ev: &KeyEvent) -> bool {
    let m = ev.modifiers;
    let (ctrl, alt, shift) = (
        m.contains(KeyModifiers::CONTROL),
        m.contains(KeyModifiers::ALT),
        m.contains(KeyModifiers::SHIFT),
    );
    match ev.code {
        // Ctrl+Alt+symbol is AltGr typing a character: leave it.
        KeyCode::Char(c) if ctrl && alt && !c.is_ascii_alphabetic() => false,
        KeyCode::Char(' ') => false,
        KeyCode::Char(c) => ctrl && (shift || !c.is_ascii_alphabetic()),
        KeyCode::Enter | KeyCode::Tab | KeyCode::Esc => ctrl,
        KeyCode::Backspace => ctrl && shift,
        _ => false,
    }
}

/// A key as Windows' win32-input-mode records (`CSI Vk;Sc;Uc;Kd;Cs;Rc _`), down then up.
pub fn encode_win32(ev: &KeyEvent) -> Option<Vec<u8>> {
    let m = ev.modifiers;
    let (ctrl, alt, shift) = (
        m.contains(KeyModifiers::CONTROL),
        m.contains(KeyModifiers::ALT),
        m.contains(KeyModifiers::SHIFT),
    );
    let (vk, uc, enhanced): (u16, u32, bool) = match ev.code {
        KeyCode::Char(c) => {
            let up = c.to_ascii_uppercase();
            let vk = if up.is_ascii_alphanumeric() {
                up as u16
            } else {
                match c {
                    ' ' => 0x20,
                    '-' | '_' => 0xBD,
                    '=' | '+' => 0xBB,
                    '[' | '{' => 0xDB,
                    ']' | '}' => 0xDD,
                    '\\' | '|' => 0xDC,
                    ';' | ':' => 0xBA,
                    '\'' | '"' => 0xDE,
                    ',' | '<' => 0xBC,
                    '.' | '>' => 0xBE,
                    '/' | '?' => 0xBF,
                    '`' | '~' => 0xC0,
                    '!' => 0x31,
                    '@' => 0x32,
                    '#' => 0x33,
                    '$' => 0x34,
                    '%' => 0x35,
                    '^' => 0x36,
                    '&' => 0x37,
                    '*' => 0x38,
                    '(' => 0x39,
                    ')' => 0x30,
                    _ => return None,
                }
            };
            let uc = if ctrl && up.is_ascii_alphabetic() {
                up as u32 - 0x40
            } else {
                c as u32
            };
            (vk, uc, false)
        }
        KeyCode::Enter => (0x0D, 13, false),
        KeyCode::Tab | KeyCode::BackTab => (0x09, 9, false),
        KeyCode::Backspace => (0x08, if ctrl { 0x7f } else { 8 }, false),
        KeyCode::Esc => (0x1B, 27, false),
        KeyCode::Up => (0x26, 0, true),
        KeyCode::Down => (0x28, 0, true),
        KeyCode::Left => (0x25, 0, true),
        KeyCode::Right => (0x27, 0, true),
        KeyCode::Home => (0x24, 0, true),
        KeyCode::End => (0x23, 0, true),
        KeyCode::PageUp => (0x21, 0, true),
        KeyCode::PageDown => (0x22, 0, true),
        KeyCode::Insert => (0x2D, 0, true),
        KeyCode::Delete => (0x2E, 0, true),
        KeyCode::F(n @ 1..=12) => (0x6F + n as u16, 0, false),
        _ => return None,
    };
    let mut cs = 0u32;
    if shift || ev.code == KeyCode::BackTab {
        cs |= 0x10; // SHIFT_PRESSED
    }
    if ctrl {
        cs |= 0x08; // LEFT_CTRL_PRESSED
    }
    if alt {
        cs |= 0x02; // LEFT_ALT_PRESSED
    }
    if enhanced {
        cs |= 0x100; // ENHANCED_KEY
    }
    let rec = |down: u8| format!("\x1b[{vk};0;{uc};{down};{cs};1_");
    Some(format!("{}{}", rec(1), rec(0)).into_bytes())
}

/// The last cursor style a program asked for (DECSCUSR `CSI n SP q`, 0-6) in `data`.
pub fn last_cursor_style(data: &[u8]) -> Option<u8> {
    let mut found = None;
    let mut i = 0;
    while i + 3 < data.len() + 1 {
        if data[i] == 0x1b && data.get(i + 1) == Some(&b'[') {
            let mut j = i + 2;
            let mut n: Option<u8> = None;
            if let Some(d) = data.get(j).filter(|d| d.is_ascii_digit()) {
                n = Some(d - b'0');
                j += 1;
            }
            if data.get(j) == Some(&b' ') && data.get(j + 1) == Some(&b'q') {
                found = Some(n.unwrap_or(0).min(6));
                i = j + 2;
                continue;
            }
        }
        i += 1;
    }
    found
}

/// Encode a key press as the bytes a terminal program expects.
pub fn encode(ev: &KeyEvent, app_cursor: bool) -> Vec<u8> {
    let m = ev.modifiers;
    let ctrl = m.contains(KeyModifiers::CONTROL);
    let alt = m.contains(KeyModifiers::ALT);
    let shift = m.contains(KeyModifiers::SHIFT);
    // xterm modifier parameter: 1 + shift + 2*alt + 4*ctrl
    let param = 1 + shift as u8 + 2 * alt as u8 + 4 * ctrl as u8;
    let csi_mod = |base: &str, fin: char| -> Vec<u8> {
        if param > 1 {
            format!("\x1b[1;{param}{fin}").into_bytes()
        } else {
            base.as_bytes().to_vec()
        }
    };
    let tilde = |n: u8| -> Vec<u8> {
        if param > 1 {
            format!("\x1b[{n};{param}~").into_bytes()
        } else {
            format!("\x1b[{n}~").into_bytes()
        }
    };
    let mut out = match ev.code {
        KeyCode::Char(c) => {
            // AltGr arrives as Ctrl+Alt on Windows; treat non-letters as plain text.
            if ctrl && alt && !c.is_ascii_alphabetic() {
                let mut b = [0; 4];
                return c.encode_utf8(&mut b).as_bytes().to_vec();
            }
            if ctrl {
                let b = match c.to_ascii_lowercase() {
                    c @ 'a'..='z' => c as u8 - b'a' + 1,
                    ' ' | '@' | '2' => 0,
                    '[' | '3' => 0x1b,
                    '\\' | '4' => 0x1c,
                    ']' | '5' => 0x1d,
                    '^' | '6' => 0x1e,
                    '_' | '-' | '7' => 0x1f,
                    '?' | '8' => 0x7f,
                    other => {
                        let mut b = [0; 4];
                        return other.encode_utf8(&mut b).as_bytes().to_vec();
                    }
                };
                vec![b]
            } else {
                let mut b = [0; 4];
                c.encode_utf8(&mut b).as_bytes().to_vec()
            }
        }
        // Shift+Enter as Meta+Enter: what agent CLIs (Claude Code, Codex) read as "newline".
        KeyCode::Enter if shift => return b"\x1b\r".to_vec(),
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => return b"\x1b[Z".to_vec(),
        KeyCode::Backspace if ctrl => vec![0x08],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => {
            return if app_cursor && param == 1 {
                b"\x1bOA".to_vec()
            } else {
                csi_mod("\x1b[A", 'A')
            };
        }
        KeyCode::Down => {
            return if app_cursor && param == 1 {
                b"\x1bOB".to_vec()
            } else {
                csi_mod("\x1b[B", 'B')
            };
        }
        KeyCode::Right => {
            return if app_cursor && param == 1 {
                b"\x1bOC".to_vec()
            } else {
                csi_mod("\x1b[C", 'C')
            };
        }
        KeyCode::Left => {
            return if app_cursor && param == 1 {
                b"\x1bOD".to_vec()
            } else {
                csi_mod("\x1b[D", 'D')
            };
        }
        KeyCode::Home => {
            return if app_cursor && param == 1 {
                b"\x1bOH".to_vec()
            } else {
                csi_mod("\x1b[H", 'H')
            };
        }
        KeyCode::End => {
            return if app_cursor && param == 1 {
                b"\x1bOF".to_vec()
            } else {
                csi_mod("\x1b[F", 'F')
            };
        }
        KeyCode::Insert => return tilde(2),
        KeyCode::Delete => return tilde(3),
        KeyCode::PageUp => return tilde(5),
        KeyCode::PageDown => return tilde(6),
        KeyCode::F(n @ 1..=4) => {
            let fin = (b'P' + n - 1) as char;
            return if param > 1 {
                format!("\x1b[1;{param}{fin}").into_bytes()
            } else {
                format!("\x1bO{fin}").into_bytes()
            };
        }
        KeyCode::F(n) => {
            let code = match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                _ => return Vec::new(),
            };
            return tilde(code);
        }
        _ => return Vec::new(),
    };
    if alt {
        out.insert(0, 0x1b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyEventKind, KeyEventState};

    /// Characters a US keyboard types only with Shift.
    const SHIFTED: &str = "~!@#$%^&*()_+{}|:\"<>?";

    /// The one default that needs Shift, at the user's request (ADR 0022).
    const SHIFTED_ALLOWED: &[(&str, &str)] = &[("?", "help")];

    #[test]
    fn default_shortcuts_are_one_key_each_and_none_but_all_keys_needs_shift() {
        let mut keys = std::collections::HashSet::new();
        let mut actions = std::collections::HashSet::new();
        for (key, action) in DEFAULT_PREFIX_KEYS {
            let spec: KeySpec = key.parse().unwrap();
            assert!(keys.insert(spec), "{key} is bound twice");
            assert!(actions.insert(*action), "{action} has two keys");
            assert!(
                !spec.mods.contains(KeyModifiers::SHIFT),
                "{key} needs Shift"
            );
            if let KeyCode::Char(c) = spec.code
                && !SHIFTED_ALLOWED.contains(&(*key, *action))
            {
                assert!(
                    !c.is_uppercase() && !SHIFTED.contains(c),
                    "{key} needs Shift"
                );
            }
        }
    }

    #[test]
    fn question_mark_matches_with_or_without_shift_reported() {
        let spec: KeySpec = "?".parse().unwrap();
        assert_eq!(
            spec,
            KeySpec::from_event(&ev(KeyCode::Char('?'), KeyModifiers::SHIFT)),
            "Windows and most terminals report Shift"
        );
        assert_eq!(
            spec,
            KeySpec::from_event(&ev(KeyCode::Char('?'), KeyModifiers::NONE))
        );
    }

    fn ev(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn parse_and_match() {
        let spec: KeySpec = "ctrl+b".parse().unwrap();
        assert_eq!(
            spec,
            KeySpec::from_event(&ev(KeyCode::Char('b'), KeyModifiers::CONTROL))
        );
        let spec: KeySpec = "%".parse().unwrap();
        assert_eq!(
            spec,
            KeySpec::from_event(&ev(KeyCode::Char('%'), KeyModifiers::SHIFT))
        );
        let spec: KeySpec = "shift+n".parse().unwrap();
        assert_eq!(
            spec,
            KeySpec::from_event(&ev(KeyCode::Char('N'), KeyModifiers::SHIFT))
        );
        let spec: KeySpec = "N".parse().unwrap();
        assert_eq!(
            spec,
            KeySpec::from_event(&ev(KeyCode::Char('N'), KeyModifiers::SHIFT))
        );
        let caps_lock_n = KeyEvent::new_with_kind_and_state(
            KeyCode::Char('N'),
            KeyModifiers::NONE,
            KeyEventKind::Press,
            KeyEventState::CAPS_LOCK,
        );
        assert_eq!(KeySpec::from_event(&caps_lock_n), "n".parse().unwrap());
        #[cfg(windows)]
        assert_eq!(
            KeySpec::from_event(&ev(KeyCode::Char('N'), KeyModifiers::NONE)),
            "n".parse().unwrap()
        );
        let space: KeySpec = "ctrl+space".parse().unwrap();
        assert_eq!(
            space,
            KeySpec::from_event(&ev(KeyCode::Char(' '), KeyModifiers::CONTROL))
        );
        assert_eq!(
            space,
            KeySpec::from_event(&ev(
                KeyCode::Char('@'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            ))
        );
        assert_eq!(
            space,
            KeySpec::from_event(&ev(KeyCode::Char('2'), KeyModifiers::CONTROL))
        );
        let spec: KeySpec = "alt+left".parse().unwrap();
        assert_eq!(
            spec,
            KeySpec::from_event(&ev(KeyCode::Left, KeyModifiers::ALT))
        );
        assert_eq!(
            "ctrl++".parse::<KeySpec>().unwrap().code,
            KeyCode::Char('+')
        );
        // Windows key records for what VT can't say; the rest unchanged.
        let k = |code, m| KeyEvent::new(code, m);
        let cs = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
        assert!(wants_win32(&k(KeyCode::Char('A'), cs)));
        assert!(wants_win32(&k(KeyCode::Enter, KeyModifiers::CONTROL)));
        assert!(
            !wants_win32(&k(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            "Ctrl+C stays ^C"
        );
        assert!(
            !wants_win32(&k(KeyCode::Enter, KeyModifiers::SHIFT)),
            "Shift+Enter stays the agents' newline"
        );
        assert_eq!(
            encode(&k(KeyCode::Enter, KeyModifiers::SHIFT), false),
            b"\x1b\r".to_vec(),
            "Shift+Enter: a newline, not send"
        );
        assert_eq!(
            encode(&k(KeyCode::Enter, KeyModifiers::NONE), false),
            b"\r".to_vec()
        );
        assert!(
            !wants_win32(&k(
                KeyCode::Char('@'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            "AltGr typing"
        );
        assert_eq!(
            encode_win32(&k(KeyCode::Char('A'), cs)).unwrap(),
            b"\x1b[65;0;1;1;24;1_\x1b[65;0;1;0;24;1_".to_vec()
        );
        assert_eq!(
            encode_win32(&k(KeyCode::Up, KeyModifiers::NONE)).unwrap(),
            b"\x1b[38;0;0;1;256;1_\x1b[38;0;0;0;256;1_".to_vec()
        );
        // Cursor styles.
        assert_eq!(last_cursor_style(b"x\x1b[2 qy\x1b[6 q"), Some(6));
        assert_eq!(last_cursor_style(b"\x1b[ q"), Some(0));
        assert_eq!(last_cursor_style(b"\x1b[2J"), None);
        for (_, a) in DEFAULT_PREFIX_KEYS {
            let act: Action = a.parse().unwrap();
            assert_eq!(
                act.to_config().unwrap().parse::<Action>().unwrap(),
                act,
                "{a}"
            );
        }
        for s in [
            "ctrl+b",
            "ctrl+space",
            "alt+a",
            "%",
            "N",
            "f5",
            "shift+tab",
            "ctrl+left",
        ] {
            let k: KeySpec = s.parse().unwrap();
            assert_eq!(k.to_config().parse::<KeySpec>().unwrap(), k, "{s}");
        }
        for (k, a) in DEFAULT_PREFIX_KEYS {
            k.parse::<KeySpec>().unwrap();
            a.parse::<Action>().unwrap();
        }
    }

    #[test]
    fn encoding() {
        assert_eq!(
            encode(&ev(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            vec![3]
        );
        assert_eq!(
            encode(&ev(KeyCode::Char('x'), KeyModifiers::ALT), false),
            b"\x1bx"
        );
        assert_eq!(
            encode(&ev(KeyCode::Up, KeyModifiers::NONE), true),
            b"\x1bOA"
        );
        assert_eq!(
            encode(&ev(KeyCode::Up, KeyModifiers::CONTROL), true),
            b"\x1b[1;5A"
        );
        assert_eq!(
            encode(
                &ev(
                    KeyCode::Char('@'),
                    KeyModifiers::CONTROL | KeyModifiers::ALT
                ),
                false
            ),
            b"@"
        );
    }
}
