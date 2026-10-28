//! State for the pane menu and settings screen. Drawing lives in render.rs;
//! the key handling that needs the whole app lives in mod.rs.

use crate::config::Config;
use anyhow::{Context, Result};

// ---- settings --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Bool,
    Choice(&'static [&'static str]),
    #[allow(dead_code)]
    Number { step: f64, min: f64, max: f64 },
    Int { step: i64, min: i64, max: i64 },
    Text,
    Key,
    /// A program: the ones of these installed here to pick from (and "default"); Enter
    /// types any other.
    Program(&'static [&'static str]),
    /// A folder: Enter opens the folder browser to pick one.
    Folder,
    Spinner,
}

/// Editors the app offers when they're installed.
pub const EDITORS: &[&str] = &["code", "cursor", "zed", "nvim", "vim", "hx", "micro", "nano", "emacs", "subl", "notepad++", "notepad"];
/// Shells the app offers when they're installed.
pub const SHELLS: &[&str] = &["pwsh", "powershell", "cmd", "bash", "zsh", "fish", "nu", "elvish", "xonsh"];

/// The choices for a program setting: "default" first, then the candidates installed here,
/// then the current value if it's something else. Looked up once per run (PATH doesn't
/// change under us).
pub fn program_options(cands: &'static [&'static str], current: &str) -> Vec<String> {
    use std::sync::{Mutex, OnceLock};
    static FOUND: OnceLock<Mutex<std::collections::HashMap<usize, Vec<String>>>> = OnceLock::new();
    let key = cands.as_ptr() as usize;
    let installed = FOUND
        .get_or_init(Default::default)
        .lock()
        .map(|mut m| m.entry(key).or_insert_with(|| cands.iter().filter(|c| crate::proc::on_path(c)).map(|c| c.to_string()).collect()).clone())
        .unwrap_or_default();
    let mut out = vec!["default".to_string()];
    out.extend(installed);
    if !current.is_empty() && !out.iter().any(|o| o == current) {
        out.push(current.to_string());
    }
    out
}

#[derive(Debug)]
pub struct Setting {
    pub path: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    /// Which page of the settings view it's on.
    pub cat: Cat,
    /// One plain sentence: what it changes.
    pub help: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cat {
    General,
    Appearance,
    Sessions,
    Alerts,
    Agents,
    Check,
    Keys,
}

impl Cat {
    pub const ALL: [Cat; 7] = [Cat::General, Cat::Appearance, Cat::Sessions, Cat::Alerts, Cat::Agents, Cat::Check, Cat::Keys];

    pub fn label(self) -> &'static str {
        match self {
            Cat::General => "General",
            Cat::Sessions => "Sessions",
            Cat::Appearance => "Appearance",
            Cat::Alerts => "Alerts",
            Cat::Agents => "Agents",
            Cat::Check => "Check",
            Cat::Keys => "Keys",
        }
    }

}

pub const SETTINGS: &[Setting] = &[
    // General
    Setting { path: "ui.update_check", label: "Check for updates", kind: Kind::Bool, cat: Cat::General, help: concat!("On start, and every few hours while open, look for a newer ", env!("APP_COMMAND"), ". The Update button installs it.") },
    Setting { path: "ui.splash", label: "Splash screen", kind: Kind::Bool, cat: Cat::General, help: concat!("Show the ", env!("APP_COMMAND"), " and what happened while you were away when you start.") },
    Setting { path: "ui.splash_animation", label: "Splash animation", kind: Kind::Choice(&["cold start", "always", "never"]), cat: Cat::General, help: "The wordmark falls into place. cold start: the first start of a version, or when nothing is running. Off with Motion off." },
    Setting { path: "prefix", label: "Leader key", kind: Kind::Key, cat: Cat::General, help: "Press it, let go, then a key. Enter, then press the new leader." },
    Setting { path: "ui.mouse", label: "Mouse", kind: Kind::Bool, cat: Cat::General, help: "Click, hover, scroll and drag. Hold Shift to select text with your terminal instead." },
    Setting { path: "ui.alt_keys", label: "Alt+letter shortcuts", kind: Kind::Bool, cat: Cat::General, help: "Alt+n shell, Alt+a session, Alt+x close, Alt+s / Alt+d split, without the leader. Change them in Settings › Keys." },
    Setting { path: "ui.alt_arrows", label: "Alt+arrows move between panes", kind: Kind::Bool, cat: Cat::General, help: "Without the leader. Turn it off if your shell jumps words with Alt+arrows." },
    Setting { path: "ui.which_key", label: "Leader waits for next key", kind: Kind::Bool, cat: Cat::General, help: "After the leader, list every key if you pause. Off: the Menu and ? list them." },
    Setting { path: "shell", label: "Shell", kind: Kind::Program(SHELLS), cat: Cat::General, help: "The shell new sessions run. Empty: pwsh / powershell on Windows, $SHELL elsewhere." },
    Setting { path: "editor", label: "Editor", kind: Kind::Program(EDITORS), cat: Cat::General, help: concat!("Opens files from ", env!("APP_COMMAND"), ". Empty: $VISUAL, $EDITOR, then code. nvim, hx, … open inside ", env!("APP_COMMAND"), ".") },
    Setting { path: "ui.start_dir", label: "Start folder", kind: Kind::Folder, cat: Cat::General, help: concat!("Where plain `", env!("APP_COMMAND"), "` opens, and the shell after you close everything. Empty: where you run it. ~ is home.") },
    Setting { path: "shell_integration", label: "PowerShell folder tracking", kind: Kind::Bool, cat: Cat::General, help: concat!("Lets ", env!("APP_COMMAND"), " see where PowerShell sessions cd to.") },
    // Appearance
    Setting { path: "theme", label: "Theme", kind: Kind::Choice(crate::theme::BUILTIN), cat: Cat::Appearance, help: "Enter to search and preview themes. Enter in the browser applies your choice." },
    Setting { path: "ui.layout", label: "Layout", kind: Kind::Choice(crate::config::LAYOUT_NAMES), cat: Cat::Appearance, help: "floating: a box per pane. hairline: no boxes, only lines." },
    Setting { path: "ui.sidebar_position", label: "Sidebar side", kind: Kind::Choice(&["left", "right"]), cat: Cat::Appearance, help: "Which edge the sidebar sits on." },
    Setting { path: "ui.padding", label: "Padding", kind: Kind::Choice(&["0", "1", "2"]), cat: Cat::Appearance, help: "Space round the whole screen, in rows and columns." },
    Setting { path: "ui.background", label: "Background", kind: Kind::Choice(&["terminal", "solid"]), cat: Cat::Appearance, help: concat!("terminal: your terminal's background shows through. solid: ", env!("APP_COMMAND"), " paints every cell.") },
    Setting { path: "ui.motion", label: "Motion", kind: Kind::Bool, cat: Cat::Appearance, help: "Slides, glides and fades (under 150 ms). Always off over SSH." },
    Setting { path: "ui.gap", label: "Gap", kind: Kind::Choice(&["0", "1", "2"]), cat: Cat::Appearance, help: "Space between floating panes, in rows stacked and columns side by side." },
    Setting { path: "ui.pane_colors", label: "Pane colours", kind: Kind::Bool, cat: Cat::Appearance, help: "Floating: each pane's border in its own colour, by its place in the tab." },
    Setting { path: "ui.rounded", label: "Rounded", kind: Kind::Choice(crate::config::Rounding::NAMES), cat: Cat::Appearance, help: "full: round corners and round ends everywhere (needs a Nerd Font). corners: rounded squares, square ends. off: all square." },
    Setting { path: "ui.dim", label: "Dim unfocused", kind: Kind::Choice(&["off", "subtle", "30%", "60%"]), cat: Cat::Appearance, help: "How far the panes you're not in fade. Their colours stay recognisable." },
    Setting { path: "ui.focus_border", label: "Focus border", kind: Kind::Choice(&["accent", "bright", "none"]), cat: Cat::Appearance, help: "How the pane you're in is outlined." },
    Setting { path: "ui.cursor", label: "Cursor", kind: Kind::Choice(crate::config::CURSOR_SHAPES), cat: Cat::Appearance, help: "The caret where a pane's program picks none: the terminal's own, block, underline or bar. vim keeps its own." },
    Setting { path: "ui.cursor_blink", label: "Cursor blink", kind: Kind::Bool, cat: Cat::Appearance, help: "Blink that caret. No effect with your terminal's own cursor." },
    Setting { path: "ui.glyphs", label: "Status glyphs", kind: Kind::Choice(&["auto", "dots", "unicode", "ascii", "nerd"]), cat: Cat::Appearance, help: "dots: a coloured dot and a word per session. unicode, ascii, nerd: a shape per state. auto: dots, or ascii if your icons are." },
    Setting { path: "ui.spinner", label: "Working spinner", kind: Kind::Spinner, cat: Cat::Appearance, help: "How a working agent's mark moves: Snake, Pulse, Braille and more. Other status icons keep their meaning." },
    // Sessions
    Setting { path: "sessions.agent", label: "Agent", kind: Kind::Choice(crate::config::NEW_AGENTS), cat: Cat::Sessions, help: concat!("Which agent leader s starts. last used is the last one you started from ", env!("APP_NAME"), ".") },
    Setting { path: "sessions.where", label: "Where", kind: Kind::Choice(crate::config::NEW_WHERE), cat: Cat::Sessions, help: "new worktree: its own branch, so sessions side by side don't collide." },
    Setting { path: "ui.sidebar_tabs", label: "Tabs in the sidebar", kind: Kind::Choice(crate::config::SIDEBAR_TABS), cat: Cat::Sessions, help: "A session's tabs as lines under its row. busy: only those working or needing you, and the one on screen." },
    Setting { path: "ui.attention_sort", label: "Sort sidebar by attention", kind: Kind::Bool, cat: Cat::Sessions, help: "Sessions that need you float to the top of their group. The rest keep their place." },
    Setting { path: "worktree.per_agent", label: "Own worktree per agent", kind: Kind::Bool, cat: Cat::Sessions, help: "An agent started in a repo's main folder gets its own branch and folder." },
    Setting { path: "worktree.command", label: "Start in new worktrees", kind: Kind::Text, cat: Cat::Sessions, help: "A command to run in every new worktree (e.g. claude). Empty: a shell." },
    Setting { path: "worktree.delete_with_last", label: "Remove worktree with last agent", kind: Kind::Bool, cat: Cat::Sessions, help: concat!("Closing the last thing in a worktree ", env!("APP_COMMAND"), " made removes its folder; the branch stays. Merged and clean goes; unmerged asks.") },
    Setting { path: "restore.enabled", label: "Bring sessions back after restart", kind: Kind::Bool, cat: Cat::Sessions, help: concat!("Agents keep going in the background; after a reboot ", env!("APP_COMMAND"), " rebuilds your sessions.") },
    Setting { path: "restore.agents", label: "Resume agents", kind: Kind::Bool, cat: Cat::Sessions, help: "Restart agents in their last conversation (claude --resume, codex resume)." },
    Setting { path: "restore.commands", label: "Re-run commands", kind: Kind::Bool, cat: Cat::Sessions, help: "Run again the commands sessions were started with (lazygit, a dev server, ...)." },
    Setting { path: "restore.output", label: "Keep each pane's recent output", kind: Kind::Bool, cat: Cat::Sessions, help: "Saved to disk so it's there after a restart. May include secrets shown on screen; off deletes it." },
    Setting { path: "scrollback", label: "Scrollback lines", kind: Kind::Int { step: 1000, min: 1000, max: 100_000 }, cat: Cat::Sessions, help: "How much history each session keeps for scrolling and search." },
    // Alerts
    Setting { path: "notify.sound", label: "Alert sounds", kind: Kind::Bool, cat: Cat::Alerts, help: "Two rising notes when an agent needs you, one soft note when a background session finishes." },
    Setting { path: "notify.phone_topic", label: "Phone alerts (ntfy topic)", kind: Kind::Text, cat: Cat::Alerts, help: concat!("Subscribe to a hard-to-guess topic in the free ntfy app and put it here. Empty is off. ", env!("APP_COMMAND"), " test-alert tries it.") },
    Setting { path: "notify.phone_after", label: "Phone: after waiting (s)", kind: Kind::Int { step: 30, min: 0, max: 3600 }, cat: Cat::Alerts, help: "Only once an agent has waited this long unanswered, so it stays quiet while you're at your desk." },
    Setting { path: "notify.phone_done", label: "Phone: when one finishes too", kind: Kind::Bool, cat: Cat::Alerts, help: "Also when an agent finishes and nobody has looked (after the same wait)." },
    Setting { path: "notify.phone_text", label: "Phone: say what it's doing", kind: Kind::Bool, cat: Cat::Alerts, help: "Put the agent's task in the alert. Off: just which agent and where. On ntfy.sh the topic is the only secret." },
    // Agents
    Setting { path: "detection.working_window_ms", label: "Working window (ms)", kind: Kind::Int { step: 250, min: 250, max: 10_000 }, cat: Cat::Agents, help: "For agents without hooks: output this recent counts as working." },
    Setting { path: "sessions.helpers", label: "Sessions may start helpers", kind: Kind::Choice(crate::config::HELPERS_MAY), cat: Cat::Agents, help: "ask: the first time, the row asks you once (1 allow · 2 always in this project · 3 no). always and never answer for you." },
    Setting { path: "sessions.helpers_at_once", label: "Helpers at once", kind: Kind::Choice(crate::config::HELPERS_AT_ONCE), cat: Cat::Agents, help: "Per session. Extra helpers wait their turn." },
    Setting { path: "ui.subagent_count", label: "Count on the session row", kind: Kind::Bool, cat: Cat::Agents, help: "Shows \"· 4 agents\" on a working row only when the agent reports helpers. Claude does; Codex doesn't." },
    Setting { path: "auto_continue", label: "Continue after a limit", kind: Kind::Bool, cat: Cat::Agents, help: "An agent stopped by its plan limit is told \"continue\" once the limit resets." },
    Setting { path: "mcp.approve", label: "Agents may approve prompts", kind: Kind::Choice(&["never", "safe", "always"]), cat: Cat::Agents, help: concat!("Through ", env!("APP_COMMAND"), " mcp. safe: only commands on [mcp] safe (tests, lint, git status…). Saying no is always allowed.") },
    Setting { path: "mcp.scope", label: "Agents can reach", kind: Kind::Choice(&["project", "all"]), cat: Cat::Agents, help: concat!("project: only sessions in the calling agent's own repo. Set up with: ", env!("APP_COMMAND"), " integrate mcp") },
];

pub fn in_cat(cat: Cat) -> Vec<&'static Setting> {
    SETTINGS.iter().filter(|s| s.cat == cat).collect()
}

/// The effective value of a setting (defaults included), as TOML.
pub fn current(cfg: &Config, path: &str) -> Option<toml::Value> {
    let mut v = toml::Value::try_from(cfg).ok()?;
    for part in path.split('.') {
        v = v.get(part)?.clone();
    }
    Some(v)
}

pub fn display(cfg: &Config, s: &Setting) -> String {
    if s.kind == Kind::Spinner {
        return crate::config::spinner_name(&cfg.ui.spinner).into();
    }
    match (s.kind, current(cfg, s.path)) {
        (Kind::Key, Some(toml::Value::String(k))) => {
            k.parse::<crate::keys::KeySpec>().map(|k| k.to_string()).unwrap_or(k)
        }
        (Kind::Bool, Some(toml::Value::Boolean(b))) => (if b { "on" } else { "off" }).into(),
        (Kind::Number { .. }, Some(toml::Value::Float(f))) => format!("{f:.2}"),
        (Kind::Text, Some(toml::Value::String(t))) if t.is_empty() => "(shell)".into(),
        (_, Some(toml::Value::String(t))) => t,
        (_, Some(other)) => other.to_string(),
        (_, None) => String::new(),
    }
}

/// The value one step left (`dir` -1) or right (+1) of the current one.
pub fn step(cfg: &Config, s: &Setting, dir: i64) -> Option<toml_edit::Value> {
    let cur = current(cfg, s.path)?;
    Some(match s.kind {
        Kind::Bool => (!cur.as_bool()?).into(),
        Kind::Choice(opts) => {
            let i = opts.iter().position(|o| Some(*o) == cur.as_str()).unwrap_or(0) as i64;
            let n = opts.len() as i64;
            opts[((i + dir).rem_euclid(n)) as usize].into()
        }
        Kind::Number { step, min, max } => {
            let v = cur.as_float().unwrap_or(0.0) + step * dir as f64;
            ((v.clamp(min, max) * 100.0).round() / 100.0).into()
        }
        Kind::Int { step, min, max } => (cur.as_integer().unwrap_or(0) + step * dir).clamp(min, max).into(),
        Kind::Program(cands) => {
            let now = cur.as_str().unwrap_or_default().to_string();
            let opts = program_options(cands, &now);
            let i = opts.iter().position(|o| *o == now).unwrap_or(0) as i64;
            let next = &opts[((i + dir).rem_euclid(opts.len() as i64)) as usize];
            (if next == "default" { "" } else { next.as_str() }).into()
        }
        Kind::Text | Kind::Key | Kind::Folder | Kind::Spinner => return None,
    })
}

/// Write one setting into config.toml, keeping the rest of the file (comments included).
pub fn write(path: &str, value: toml_edit::Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();
    write_at(&parts, value)
}

/// Write a value at a path of keys (a key may contain dots, like the `.` shortcut).
pub fn write_at(parts: &[&str], mut value: toml_edit::Value) -> Result<()> {
    edit_config(|doc| {
        let (leaf, tables) = parts.split_last().context("empty setting path")?;
        let mut table = doc.as_table_mut();
        for part in tables {
            if !table.contains_key(part) {
                // Implicit: `[keys.prefix]` without an empty `[keys]` above it.
                let mut new = toml_edit::Table::new();
                new.set_implicit(true);
                table.insert(part, toml_edit::Item::Table(new));
            }
            table = table[*part].as_table_mut().with_context(|| format!("`{part}` in config.toml is not a table"))?;
        }
        if let Some(old) = table.get_mut(leaf).and_then(toml_edit::Item::as_value_mut) {
            // Replacing the whole entry discards comments attached to its key and value.
            *value.decor_mut() = old.decor().clone();
            *old = value;
        } else {
            table.insert(leaf, toml_edit::value(value));
        }
        Ok(())
    })
}

/// Change config.toml through `edit`, keeping the rest of the file (comments included).
pub fn edit_config(edit: impl FnOnce(&mut toml_edit::DocumentMut) -> Result<()>) -> Result<()> {
    let file = crate::config::config_path();
    let text = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context("reading config"),
    };
    let mut doc: toml_edit::DocumentMut = text.parse().context("config.toml has a syntax error; fix it first")?;
    edit(&mut doc)?;
    crate::config::write_atomic(&file, doc.to_string()).context("writing config")?;
    crate::sync::push_soon();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn settings_step_and_display() {
        let cfg = Config::default();
        assert!(SETTINGS.iter().all(|s| s.path != "sleep_after"), "automatic agent sleep is no longer configurable");
        assert!(SETTINGS.iter().all(|s| s.path != "ui.wake_animation"), "wake animations are no longer configurable");
        for s in SETTINGS.iter().filter(|s| s.path != "shell") {
            // `shell` is unset by default (the app picks one), so it has no value yet.
            assert!(current(&cfg, s.path).is_some(), "missing setting {}", s.path);
        }
        let lines = SETTINGS.iter().find(|s| s.path == "scrollback").unwrap();
        assert_eq!(step(&cfg, lines, 1).unwrap().as_integer(), Some(cfg.scrollback as i64 + 1000));
        let side = SETTINGS.iter().find(|s| s.path == "ui.sidebar_position").unwrap();
        assert_eq!(step(&cfg, side, 1).unwrap().as_str(), Some("right"));
        assert_eq!(step(&cfg, side, -1).unwrap().as_str(), Some("right"));
        let leader = SETTINGS.iter().find(|s| s.path == "prefix").unwrap();
        assert_eq!(display(&cfg, leader), "C-Space");
    }

    #[test]
    fn write_keeps_comments() {
        let dir = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-settings-{}"), std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("config.toml");
        std::fs::write(&file, "# my notes\ntheme = \"nord\"\n\n[ui]\n# my ui notes\nsidebar = true\n").unwrap();
        // SAFETY: tests touching <APP>_CONFIG run in this one test only.
        unsafe { std::env::set_var(concat!(env!("APP_ENV"), "_CONFIG"), &file) };
        write("ui.which_key_delay_ms", 500.into()).unwrap();
        write("worktree.command", "claude".into()).unwrap();
        let (tx, _) = super::super::mpsc::unbounded_channel();
        let (bg_tx, mut bg_rx) = super::super::mpsc::unbounded_channel();
        let mut app = super::super::App::new(Config::load().unwrap(), tx, None, bg_tx);
        app.splash = false;
        app.open_themes();
        app.on_paste("zenbones light".into());
        app.on_key(super::super::KeyEvent::new(super::super::KeyCode::Enter, super::super::KeyModifiers::NONE));
        assert!(matches!(app.mode, super::super::Mode::Themes { saving: true, .. }));
        app.on_bg(bg_rx.blocking_recv().unwrap());
        assert_eq!(app.cfg.theme, "zenbones-light");
        assert_eq!(app.theme.bg, crate::theme::Theme::named("zenbones-light").bg);
        assert!(matches!(app.mode, super::super::Mode::ScreenSettings(_)));
        let out = std::fs::read_to_string(&file).unwrap();
        assert!(out.contains("theme = \"zenbones-light\""));
        // A directory cannot be a config file: failed saves retain the browser and applied theme.
        unsafe { std::env::set_var(concat!(env!("APP_ENV"), "_CONFIG"), &dir) };
        app.open_themes();
        app.on_paste("solarized dark".into());
        app.on_key(super::super::KeyEvent::new(super::super::KeyCode::Enter, super::super::KeyModifiers::NONE));
        app.on_bg(bg_rx.blocking_recv().unwrap());
        assert_eq!(app.cfg.theme, "zenbones-light");
        assert!(matches!(&app.mode, super::super::Mode::Themes { query, saving: false, .. } if query == "solarized dark"));
        // SAFETY: this existing config-save test owns <APP>_CONFIG; all prior jobs have completed.
        unsafe { std::env::set_var(concat!(env!("APP_ENV"), "_CONFIG"), &file) };
        app.open_spinners();
        app.spinner_pick(2);
        app.on_key(super::super::KeyEvent::new(super::super::KeyCode::Enter, super::super::KeyModifiers::NONE));
        assert!(matches!(&app.mode, super::super::Mode::Spinners(v) if v.saving));
        app.on_bg(bg_rx.blocking_recv().unwrap());
        assert_eq!(app.cfg.ui.spinner, ["|", "/", "-", "\\"]);
        assert_eq!(Config::load().unwrap().ui.spinner, app.cfg.ui.spinner);
        let saved_spinner = app.cfg.ui.spinner.clone();
        // SAFETY: the spinner save job completed before changing this test's isolated path.
        unsafe { std::env::set_var(concat!(env!("APP_ENV"), "_CONFIG"), &dir) };
        app.open_spinners();
        app.on_key(super::super::KeyEvent::new(super::super::KeyCode::End, super::super::KeyModifiers::NONE));
        app.on_key(super::super::KeyEvent::new(super::super::KeyCode::Enter, super::super::KeyModifiers::NONE));
        app.on_bg(bg_rx.blocking_recv().unwrap());
        assert_eq!(app.cfg.ui.spinner, saved_spinner, "a failed save keeps the applied choice");
        assert!(matches!(&app.mode, super::super::Mode::Spinners(v) if !v.saving));
        // SAFETY: the failed save job completed before restoring the isolated test file.
        unsafe { std::env::set_var(concat!(env!("APP_ENV"), "_CONFIG"), &file) };
        app.on_key(super::super::KeyEvent::new(super::super::KeyCode::Enter, super::super::KeyModifiers::NONE));
        app.on_bg(bg_rx.blocking_recv().unwrap());
        assert_eq!(app.cfg.ui.spinner, crate::config::BARS_FRAMES, "the picked style survives a failed save and applies on retry");
        let legacy_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
        write("ui.spinner", legacy_frames.into_iter().collect::<toml_edit::Array>().into()).unwrap();
        assert_eq!(Config::load().unwrap().ui.spinner, legacy_frames, "custom frames must not be silently migrated");
        unsafe { std::env::remove_var(concat!(env!("APP_ENV"), "_CONFIG")) };
        assert!(out.contains("# my notes") && out.contains("# my ui notes"), "{out}");
        assert!(out.contains("which_key_delay_ms = 500"), "{out}");
        assert!(out.contains("[worktree]") && out.contains("command = \"claude\""), "{out}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
