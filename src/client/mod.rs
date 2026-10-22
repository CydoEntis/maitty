//! The TUI client: renders the daemon's snapshot, keeps a terminal emulator per pane, and
//! turns keys into either pane input or commands.

mod copy;
mod marks;
mod design;
mod files;
mod find;
mod branch;
mod screen;
mod menu;
mod modal;
mod motion;
mod overlap;
mod pick;
mod recipes;
mod render;
mod tasks;
mod text_input;
mod views;
mod actions;
mod background;
mod input;
mod view_keys;
mod themes;
mod spinners;
mod shortcuts;
pub mod restart_prompt;

use crate::config::{Config, Keymap};
use crate::ipc;
use crate::keys::{self, Action, KeySpec};
use crate::protocol::*;
use crate::theme::Theme;
use anyhow::Result;
use futures_util::{FutureExt, StreamExt};
use ratatui::crossterm::event::{
    self, Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Position, Rect};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// "3 sessions stop and resume, 1 is working": what a daemon restart costs.
fn restart_summary(sessions: usize, working: usize) -> String {
    let stops = if sessions == 1 { "1 session stops and resumes".to_string() } else { format!("{sessions} sessions stop and resume") };
    match working {
        0 => stops,
        1 => format!("{stops}, 1 is working"),
        n => format!("{stops}, {n} are working"),
    }
}

/// How often the client looks at its timers (spinners, toasts, pending focus).
/// How often an open window asks again whether a newer seshi is out.
const UPDATE_CHECK_EVERY: Duration = Duration::from_secs(3 * 60 * 60);
const TICK: Duration = Duration::from_millis(50);
/// The shortest time between two frames (about 80 a second at most).
const MIN_FRAME: Duration = Duration::from_millis(12);
/// Keys of one paste come this close together (Windows); typing never does.
const PASTE_GAP: Duration = Duration::from_millis(8);
/// The longest a paste is waited on before what has come is handed over.
const PASTE_MAX_WAIT: Duration = Duration::from_secs(1);
/// How long a note stays in the bottom bar.
const NOTICE_FOR: Duration = Duration::from_secs(5);
/// Server messages taken in one go before the next frame.
const SERVER_BURST: usize = 512;
/// Losing the server twice within this long is the server going, not a hiccup: stop.
const REATTACH_AT_MOST_EVERY: Duration = Duration::from_secs(10);
/// The daemon sends Bye before exiting; wait past its graceful shutdown window before spawning it again.
const SERVER_SHUTDOWN_GRACE: Duration = Duration::from_millis(200);
/// Two clicks this close together are a double-click.
pub(super) const DOUBLE_CLICK: Duration = Duration::from_millis(400);

pub struct Options {
    /// Open (or switch to) a workspace for this directory on attach.
    pub open: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Normal,
    Prefix { since: Instant },
    /// Jump list (`commands: false`) or command palette (`commands: true`).
    Picker { query: String, sel: usize, commands: bool },
    Prompt { kind: PromptKind, input: String },
    Copy(Box<copy::Copy>),
    /// Pick an existing worktree of the repo, or type a branch to create one.
    Worktrees { ws: WsId, cmd: Option<String>, items: Option<Vec<WorktreeEntry>>, query: String, sel: usize },
    /// Seshi layout: open a folder as a project.
    Finder(Box<screen::Finder>),
    /// Seshi layout: new pane (project, worktree, what to run).
    ScreenPane(screen::NewPaneHy),
    /// Seshi layout: the settings overlay.
    ScreenSettings(Box<design::SettingsView>),
    Themes { query: String, sel: usize, saving: bool },
    Spinners(Box<spinners::Picker>),
    /// Seshi layout: keyboard cursor in the sidebar; bare keys act like leader keys.
    Side,
    /// A right-click menu (seshi layout).
    ScreenMenu(Box<menu::ScreenMenu>),
    /// Find a file / search the code.
    Find(Box<find::FindView>),
    /// Switch branch.
    Branch(Box<branch::BranchView>),
    /// Notification history (the selected row, newest first).
    History { sel: usize },
    /// A repo's worktrees and which to remove.
    Tidy(Box<screen::TidyView>),
    /// "Close …?" with confirm / cancel.
    Confirm(Box<menu::Confirm>),
    /// The switcher: projects and sessions, typed to filter.
    GoTo { query: String, sel: usize },
    /// Writing a quick follow-up to an agent (beside its row, or in its Inbox row).
    Compose(Box<screen::Compose>),
    /// Why a session has its status (beside its row); `side`: back to the sidebar after.
    Why { term: TermId, side: bool },
      /// Runtime-reported subagent activity, retained while its parent remains available.
      SubagentInfo { term: TermId, index: usize },
    /// The leader's key map (or its second step), typed to search.
    KeyMap(Box<screen::KeyMap>),
    Shortcuts { query: String, scroll: usize },
    /// Every command with its key (the sidebar's "a actions").
    Actions { sel: usize },
    /// A tab being renamed in its pill.
    RenameTab(Box<screen::TabName>),
}


/// A view that replaces the pane area.
pub(super) enum View {
    Changes(Box<views::ChangesView>),
    Files(Box<views::FilesTree>),
    /// Both diffs of a file two checkouts changed (in the sheet).
    Both(Box<screen::BothView>),
    /// A check's evidence (in the sheet).
    Check(Box<screen::CheckView>),
}

/// Clickable chips and buttons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Btn {
    Answer(TermId, char),
    CloseView,
    /// A key inside the current view ('\n' for Enter).
    ViewKey(char),
    /// A row of the current view's list.
    Row(usize),
}

pub(super) enum Bg {
    Changes(PathBuf, Result<Box<tasks::Review>, String>),
    Tree(PathBuf, Vec<views::FileNode>, std::collections::HashMap<String, char>),
    TreeRecent(PathBuf, Vec<files::FileEntry>),
    /// A finished action: its message, and whether the open review should reload.
    Done(Result<String, String>, bool),
    /// A worktree's branch merged (or not): then the worktree and its branch go.
    Merged(Result<String, String>, PathBuf),
    /// A repo's worktrees, each looked at, for the clean-up list.
    Tidy(PathBuf, Vec<screen::TidyRow>),
    /// Each checkout's diff of a heads-up's file: (repo, [(checkout, diff)]).
    BothDiff(PathBuf, Vec<(String, String)>),
    /// What a checkout changed: (its folder, the status time it's for, "+42 −7 · 3 files").
    ChangeSize(PathBuf, u64, String),
    /// Files more than one checkout of a repo changed (by the repo's folder).
    Overlaps(PathBuf, Vec<overlap::Overlap>),
    /// Pulled a shared setup from another machine.
    Synced(bool),
    /// Every file under a folder (Find).
    FindFiles(PathBuf, Vec<String>),
    /// A checkout's branches: (folder, current, branches, files with changes).
    Branches(PathBuf, String, Vec<branch::Branch>, usize),
    /// A branch switch finished.
    Switched(Result<String, String>),
    /// A code search's results: (folder, which search, hits).
    Grep(PathBuf, u64, Result<Vec<find::GrepHit>, String>),
    /// A file's diff for the Changes view: (folder, which file, lines).
    Diff(PathBuf, usize, Vec<String>),
    /// Slow work done; finish it on the UI thread.
    Then(Box<dyn FnOnce(&mut App) + Send>),
}


/// Save the clipboard image as a PNG under the data folder; old pastes (a week) are
/// cleared out. Returns (path, width, height).
fn clipboard_image_to_file() -> Result<(PathBuf, usize, usize), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("can't open the clipboard: {e}"))?;
    let img = cb.get_image().map_err(|_| "there's no image on the clipboard".to_string())?;
    let dir = crate::config::data_dir().join("pastes");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t.elapsed().is_ok_and(|a| a.as_secs() > 7 * 86_400)) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let path = dir.join(format!("paste-{ms}.png"));
    write_png(&path, img.width as u32, img.height as u32, &img.bytes)?;
    Ok((path, img.width, img.height))
}

/// RGBA pixels to a PNG file.
fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgba).map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove a worktree that isn't open as a workspace.
fn remove_worktree_dir(dir: &std::path::Path) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["worktree", "remove", "--force", "."])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() { Ok(format!("removed {}", dir.display())) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// Used where a key handler replaces the view itself and must not restore the old one.
fn true_and_replace() -> bool {
    false
}


/// A row in the worktree picker.
#[derive(Debug, Clone)]
enum WtRow {
    Existing(WorktreeEntry),
    Create(String),
}

#[derive(Debug, Clone, PartialEq)]
enum PromptKind {
    RenamePane(TermId),
    RenameProject(String),
    RenameTab(WsId, TabId),
    NewWorkspace,
    ConfirmCloseWorkspace(WsId),
    ConfirmKillServer,
    ConfirmRemoveWorktree(WsId),
}

impl PromptKind {
    fn label(&self) -> &'static str {
        match self {
            PromptKind::RenamePane(_) => "Rename pane (empty: automatic)",
            PromptKind::RenameProject(_) => "Rename project (empty: its folder name)",
            PromptKind::RenameTab(..) => "Rename tab",
            PromptKind::NewWorkspace => "New pane: name it (empty = its folder)",
            PromptKind::ConfirmCloseWorkspace(_) => "Close workspace and all its panes? (y/n)",
            PromptKind::ConfirmKillServer => "Kill the server and every pane? (y/n)",
            PromptKind::ConfirmRemoveWorktree(_) => "Remove this worktree? (y / f = force / n)",
        }
    }
    fn is_confirm(&self) -> bool {
        matches!(
            self,
            PromptKind::ConfirmCloseWorkspace(_) | PromptKind::ConfirmKillServer | PromptKind::ConfirmRemoveWorktree(_)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Pane(TermId),
    Button(Btn),
    Screen(screen::ScreenHit),
}

/// A row in the jump picker.
#[derive(Debug, Clone)]
struct PickItem {
    label: String,
    detail: String,
    status: Status,
    /// Shortcut shown on the right (commands only).
    key: String,
    target: PickTarget,
}

#[derive(Debug, Clone)]
enum PickTarget {
    Workspace(WsId),
    Pane(TermId),
    Command(Action),
}

pub struct App {
    /// A newer release, found when seshi opened or since (the Update button shows).
    update_available: Option<String>,
    installed_version: Option<Result<String, String>>,
    /// What changed in it (and any releases between), for the update dialog.
    update_notes: Vec<String>,
    /// When seshi last asked whether a newer release is out.
    update_checked: Option<Instant>,
    /// An update is downloading.
    updating: bool,
    /// The installed daemon needs a restart after the panes are closed.
    update_installed_this_run: bool,
    daemon_restart_requested: bool,
    daemon_restart_reconnect: bool,
    /// Asking, because you said to, whether a newer seshi is out.
    update_checking: bool,
    /// When this window last had to attach to the server again.
    reattached: Option<Instant>,
    cfg: Config,
    theme: Theme,
    keymap: Keymap,
    notice: Option<(String, Instant, bool)>,
    /// The session the note is about (an agent finished or needs you): clicking it goes there.
    notice_term: Option<TermId>,
    /// The toast offers Restart now (the update just installed, the daemon still old).
    notice_restart: bool,
    /// What happened lately, newest last: (unix secs, pane, kind, text). Kinds: '!' needs
    /// you, '✓' finished, 'i' a message, 'x' an error.
    pub(super) history: std::collections::VecDeque<(u64, Option<TermId>, char, String)>,
    /// When the right button last went down (menus open on press or release, once).
    right_down: Option<Instant>,
    snap: Snapshot,
    got_state: bool,
    parsers: HashMap<TermId, vt100::Parser>,
    /// The mouse pointer shape last asked of the terminal (OSC 22).
    pointer: &'static str,
    /// The pane copy mode was on when it yanked (for joining with `copy_set`).
    copy_term: Option<TermId>,
    /// Selections kept from other panes while copying across a split (Tab in copy mode):
    /// (pane, text), copied together with the current one.
    copy_set: Vec<(TermId, String)>,
    /// Where commands started in each pane's history (see `marks`).
    marks: HashMap<TermId, marks::Marks>,
    sizes: HashMap<TermId, (u16, u16)>,
    mode: Mode,
    sidebar: bool,
    /// The last alert per agent (its status, when) and for any agent: the same news again
    /// soon doesn't ring twice.
    alerted: HashMap<TermId, (Status, Instant)>,
    last_alert: Option<(crate::alert::Kind, Instant)>,
    /// Only ring if the pending status still matches the latest snapshot at its deadline.
    settling: HashMap<TermId, (Status, Instant)>,
    /// What's moving, and how far the sidebar and the sheet are in this frame (0 to 1).
    motion: motion::Motion,
    side_frac: f32,
    sheet_frac: f32,
    hits: Vec<(Rect, Hit)>,
    /// Inner rects of the panes drawn last frame.
    panes: Vec<(TermId, Rect)>,
    /// Outer rects of the panes drawn last frame (for neighbour search).
    pane_frames: Vec<(TermId, Rect)>,
    /// When the wheel last paged an agent's full-screen view (see the wheel in input.rs).
    wheel_page: Option<Instant>,
    out: mpsc::UnboundedSender<ClientMsg>,
    quit: Option<String>,
    dirty: bool,
    started: Instant,
    open: Option<PathBuf>,
    /// Where a left-button press started, for drag-to-select.
    drag: Option<(TermId, Position)>,
    bg: mpsc::UnboundedSender<Bg>,
    view: Option<View>,
    /// Where the mouse is, for hover highlights.
    hover: Option<Position>,
    /// The status line's message can be undone with `u`.
    undo_hint: bool,
    /// The last click, for double-click detection.
    last_click: Option<(Hit, Instant)>,
    /// The welcome screen is up; which of its panes is selected.
    splash: bool,
    /// The seshi layout's own state.
    screen: screen::Screen,
    /// The terminal window has focus (for alerts about the session you're looking at).
    window_focused: bool,
    /// The background we told the terminal to use (OSC 11), so its padding matches.
    osc_bg: Option<ratatui::style::Color>,
    /// Panes in the middle of a synchronized update (mode 2026): since when. Not drawn
    /// until it ends (or 100 ms pass), so redraws don't flicker.
    sync_hold: HashMap<TermId, Instant>,
    /// Panes whose program asked to hear about focus (mode 1004).
    focus_report: HashSet<TermId>,
    /// The focus we last told programs about: (pane, window focused).
    focus_sent: Option<(TermId, bool)>,
    /// The last left-click in a pane, for double-click.
    pane_click: Option<(Position, Instant)>,
    /// Recent raw output per pane, to re-wrap the screen when its width changes.
    raw: HashMap<TermId, std::collections::VecDeque<u8>>,
    /// Panes whose width changed during a drag: re-wrapped from `raw` once it ends.
    rewrap: std::collections::HashSet<TermId>,
    /// The cursor style each pane's program asked for (DECSCUSR 0-6), and the one we set.
    cursor_style: HashMap<TermId, u8>,
    cursor_sent: u8,
}

pub fn run(opts: Options) -> Result<()> {
    crate::proc::ahead_of_the_panes();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;
    rt.block_on(run_async(opts))
}

async fn run_async(opts: Options) -> Result<()> {
    let (mut reader, mut writer) = ipc::open_or_spawn(true).await?;
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<ClientMsg>();
    tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ipc::send(&mut writer, &msg).await.is_err() {
                break;
            }
        }
    });

    let (cfg, err) = Config::load_or_default();
    let (bg_tx, mut bg_rx) = mpsc::unbounded_channel::<Bg>();
    let mut app = App::new(cfg, out_tx, opts.open, bg_tx);
    if crate::sync::enabled() {
        app.spawn_bg(|| Bg::Synced(crate::sync::pull().unwrap_or(false)));
    }
    app.check_for_update(false);
    app.refresh_installed_version();
    if let Some(e) = err {
        app.notify(format!("config error: {e}"), true);
    }

    let mut terminal = ratatui::init();
    // A panic must leave the shell as it found it: ratatui's own hook leaves the alternate
    // screen and raw mode; this one also turns off what seshi turned on.
    let earlier = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        earlier(info);
    }));
    if app.cfg.ui.mouse {
        let _ = execute!(std::io::stdout(), event::EnableMouseCapture);
    }
    let _ = execute!(std::io::stdout(), event::EnableBracketedPaste, event::EnableFocusChange);
    enhance_keyboard();

    let result = app.event_loop(&mut terminal, &mut reader, &mut bg_rx).await;

    restore_terminal();
    result?;
    if let Some(reason) = app.quit {
        println!("[{reason}]");
    }
    Ok(())
}

/// Whether seshi asked the terminal for the kitty keyboard protocol (so it gives it back).
static KEYBOARD_ENHANCED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Without the kitty keyboard protocol a terminal sends Shift+Enter as a plain Enter, which an
/// agent reads as "send": ask for it where the terminal has it. (Windows reports modifiers
/// on its own.)
fn enhance_keyboard() {
    #[cfg(not(windows))]
    if crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false)
        && execute!(std::io::stdout(), event::PushKeyboardEnhancementFlags(event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)).is_ok()
    {
        KEYBOARD_ENHANCED.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Undo everything seshi turned on in the terminal: mouse, paste and focus reporting, the
/// keyboard protocol, its cursor shape and background colour, the alternate screen and raw
/// mode. Safe to run twice.
fn restore_terminal() {
    let _ = execute!(
        std::io::stdout(),
        event::DisableMouseCapture,
        event::DisableBracketedPaste,
        event::DisableFocusChange
    );
    if KEYBOARD_ENHANCED.swap(false, std::sync::atomic::Ordering::Relaxed) {
        let _ = execute!(std::io::stdout(), event::PopKeyboardEnhancementFlags);
    }
    // Give the terminal its own background, cursor and mouse pointer back.
    let _ = execute!(std::io::stdout(), crossterm::cursor::SetCursorStyle::DefaultUserShape);
    {
        use std::io::Write;
        let _ = write!(std::io::stdout(), "\x1b]22;default\x1b\\");
    }
    {
        use std::io::Write;
        let _ = write!(std::io::stdout(), "]111");
    }
    ratatui::restore();
}

impl App {
    fn new(cfg: Config, out: mpsc::UnboundedSender<ClientMsg>, open: Option<PathBuf>, bg_tx: mpsc::UnboundedSender<Bg>) -> App {
        let keymap = cfg.keymap();
        let mut app = App {
            update_available: None,
            installed_version: None,
            update_notes: Vec::new(),
            update_checked: None,
            updating: false,
            update_installed_this_run: false,
            daemon_restart_requested: false,
            daemon_restart_reconnect: false,
            update_checking: false,
            reattached: None,
            theme: cfg.theme(),
            sidebar: cfg.ui.sidebar,
            alerted: HashMap::new(),
            last_alert: None,
            settling: HashMap::new(),
            motion: motion::Motion::default(),
            side_frac: 1.0,
            sheet_frac: 1.0,
            keymap,
            cfg,
            notice: None,
            notice_term: None,
            notice_restart: false,
            history: Default::default(),
            right_down: None,
            snap: Snapshot::default(),
            got_state: false,
            parsers: HashMap::new(),
            marks: HashMap::new(),
            copy_set: Vec::new(),
            copy_term: None,
            pointer: "default",
            sizes: HashMap::new(),
            mode: Mode::Normal,
            hits: Vec::new(),
            panes: Vec::new(),
            pane_frames: Vec::new(),
            wheel_page: None,
            out,
            quit: None,
            dirty: true,
            started: Instant::now(),
            open,
            drag: None,
            bg: bg_tx,
            view: None,
            hover: None,
            undo_hint: false,
            last_click: None,
            splash: false,
            screen: screen::Screen::load(),
            window_focused: true,
            osc_bg: None,
            sync_hold: HashMap::new(),
            focus_report: HashSet::new(),
            focus_sent: None,
            pane_click: None,
            raw: HashMap::new(),
            rewrap: Default::default(),
            cursor_style: HashMap::new(),
            cursor_sent: 0,
        };
        app.splash = app.cfg.ui.splash;
        if !crate::theme::BUILTIN.contains(&app.cfg.theme.as_str()) {
            app.keymap.warnings.push(format!(
                "unknown theme `{}` (have: {})",
                app.cfg.theme,
                crate::theme::BUILTIN.join(", ")
            ));
        }
        if !app.keymap.warnings.is_empty() {
            app.notify(format!("config: {}", app.keymap.warnings.join("; ")), true);
        }
        app
    }

    async fn event_loop(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        reader: &mut ipc::Reader,
        bg_rx: &mut mpsc::UnboundedReceiver<Bg>,
    ) -> Result<()> {
        let mut events = EventStream::new();
        let mut tick = tokio::time::interval(TICK);
        let mut last_draw = crate::clock::ago(Duration::from_secs(1));
        let mut last_frame = 0u64;
        let mut last_fresh = Instant::now();
        // At most one frame per 12 ms, but never wait longer than that to show new output.
        let frame = MIN_FRAME;
        loop {
            let hold = self.sync_hold_until();
            let next_draw = tokio::time::Instant::from_std((last_draw + frame).max(hold.unwrap_or(last_draw)));
            tokio::select! {
                _ = tokio::time::sleep_until(next_draw), if self.dirty => {}
                ev = events.next() => match ev {
                    // Windows hands a paste over as keystrokes: take the keys already waiting
                    // with this one, so a pasted line break isn't an Enter that sends.
                    Some(Ok(ev @ Event::Key(_))) if cfg!(windows) => {
                        let mut burst = vec![ev];
                        while let Some(Some(next)) = events.next().now_or_never() {
                            burst.push(next?);
                        }
                        // Two keys down at once is a paste starting, and the console hands the
                        // rest over in pieces: wait them out, or each piece is typed on its own.
                        if input::presses(&burst) >= 2 {
                            let started = Instant::now();
                            while started.elapsed() < PASTE_MAX_WAIT {
                                match tokio::time::timeout(PASTE_GAP, events.next()).await {
                                    Ok(Some(next)) => burst.push(next?),
                                    _ => break,
                                }
                            }
                        }
                        // The mouse moving during a paste isn't part of it.
                        let (keys, rest): (Vec<Event>, Vec<Event>) = burst.iter().cloned().partition(|ev| matches!(ev, Event::Key(_)));
                        match input::paste_from_burst(&keys) {
                            Some(text) => {
                                self.dirty = true;
                                self.on_paste(text);
                                rest.into_iter().for_each(|ev| self.on_event(ev));
                            }
                            None => burst.into_iter().for_each(|ev| self.on_event(ev)),
                        }
                    }
                    Some(Ok(ev)) => self.on_event(ev),
                    Some(Err(e)) => return Err(e.into()),
                    None => return Ok(()),
                },
                Some(b) = bg_rx.recv() => {
                    self.on_bg(b);
                    self.dirty = true;
                }
                msg = ipc::recv_server(reader) => {
                    // Take everything the server has already sent before drawing again. One
                    // message per frame can't keep up once a frame takes longer than messages
                    // arrive (many agents printing), and the server lets go of a window that
                    // falls too far behind.
                    let mut next = Some(msg);
                    let mut taken = 0;
                    let mut lost = false;
                    while let Some(msg) = next.take() {
                        match msg {
                            Ok(Some(m)) => self.on_server(m),
                            // One message that didn't decode: skip it rather than quit.
                            Err(e) if ipc::is_decode_error(&e) => tracing::warn!("skipped a message from the server: {e:#}"),
                            _ => {
                                lost = true;
                                break;
                            }
                        }
                        taken += 1;
                        if taken < SERVER_BURST && self.quit.is_none() {
                            next = ipc::recv_server(reader).now_or_never();
                        }
                    }
                    if lost && self.quit.is_none() && !self.reattach(reader).await {
                        self.quit.get_or_insert_with(|| "server exited".into());
                    }
                },
                _ = tick.tick() => {
                    if self.update_checked.is_some_and(|at| at.elapsed() >= UPDATE_CHECK_EVERY) {
                        self.check_for_update(false);
                    }
                    // What agents ask (read off their screens) can change without a state
                    // message; look again now and then.
                    if last_fresh.elapsed() >= Duration::from_secs(1) {
                        last_fresh = Instant::now();
                        self.screen_fresh();
                        self.dirty = true;
                    }
                    let frame = self.spinner_frame();
                    self.settle_alerts();
                    if frame != last_frame && (self.any_working() || self.any_checking() || matches!(self.mode, Mode::Spinners(_))) {
                        last_frame = frame;
                        self.dirty = true;
                    }
                    if matches!(self.mode, Mode::Prefix { .. }) {
                        self.dirty = true;
                    }
                    if self.notice.as_ref().is_some_and(|(_, at, _)| at.elapsed() > NOTICE_FOR) {
                        self.notice = None;
                        self.dirty = true;
                    }
                }
            }
            if self.quit.is_some() {
                return Ok(());
            }
            self.report_focus();
            self.sync_cursor_style();
            self.request_diff();
            if self.dirty && last_draw.elapsed() >= frame && self.sync_hold_until().is_none_or(|h| Instant::now() >= h) {
                self.dirty = false;
                last_draw = Instant::now();
                // Commit the whole replacement frame together, including pane switches.
                execute!(terminal.backend_mut(), ratatui::crossterm::terminal::BeginSynchronizedUpdate)?;
                let drawn = terminal.draw(|f| render::draw(self, f)).map(|_| ());
                let ended = execute!(terminal.backend_mut(), ratatui::crossterm::terminal::EndSynchronizedUpdate);
                drawn?;
                ended?;
                self.sync_sizes();
                // Something's moving: the next frame too.
                if self.motion.moving() || self.toast_moving() {
                    self.dirty = true;
                }
            }
        }
    }

    /// The connection to the server ended while the server is still there (it lets go of a
    /// window that falls too far behind): attach again, for a fresh copy of everything.
    /// False when it can't: the server is gone, or this keeps happening.
    async fn reattach(&mut self, reader: &mut ipc::Reader) -> bool {
        if !self.daemon_restart_reconnect && self.reattached.is_some_and(|at| at.elapsed() < REATTACH_AT_MOST_EVERY) {
            return false;
        }
        let restarting = self.daemon_restart_reconnect;
        let connection = if restarting {
            tokio::time::sleep(SERVER_SHUTDOWN_GRACE).await;
            ipc::open_or_spawn_installed(true).await
        } else {
            // Connect only: an unexpectedly stopped server must not be started again by this.
            ipc::open(true).await
        };
        let Ok((r, mut writer)) = connection else { return false };
        *reader = r;
        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<ClientMsg>();
        tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if ipc::send(&mut writer, &msg).await.is_err() {
                    break;
                }
            }
        });
        self.out = out_tx;
        self.reattached = Some(Instant::now());
        if !restarting {
            self.notify("Lost the server for a moment; connected again".into(), false);
        }
        true
    }

    /// Installing must not interrupt agents, including across protocol changes.
    fn start_update(&mut self) {
        if self.updating {
            return;
        }
        self.updating = true;
        self.notify("Installing update… the daemon will restart when no sessions are open.".into(), false);
        self.spawn_bg(|| {
            let r = crate::update::install_latest();
            Bg::Then(Box::new(move |app: &mut App| match r {
                Ok((v, _)) => app.update_installed(v),
                Err(e) => {
                    app.updating = false;
                    app.notify(format!("Update failed: {e:#}"), true);
                }
            }))
        });
    }

    fn update_installed(&mut self, version: String) {
        self.updating = false;
        self.update_installed_this_run = true;
        self.installed_version = Some(Ok(version.clone()));
        self.update_available = None;
        self.update_notes.clear();
        let message = if crate::ipc::remote().is_some() {
            format!("Installed seshi {version}. Update the remote daemon on its host after its sessions are closed.")
        } else {
            format!("Installed seshi {version}. The daemon will restart automatically when no sessions are open.")
        };
        self.notify(message, false);
        self.notice_restart = self.daemon_restartable();
        self.maybe_restart_daemon(false);
    }

    fn maybe_restart_daemon(&mut self, last_session_closed: bool) {
        if crate::ipc::remote().is_some()
            || !self.got_state
            || !self.snap.terms.is_empty()
            || self.daemon_restart_requested
            || self.daemon_restart_reconnect
        {
            return;
        }
        let Some(Ok(installed)) = &self.installed_version else { return };
        if self.snap.daemon_version.is_empty() || installed == &self.snap.daemon_version {
            return;
        }
        if !self.update_installed_this_run && !last_session_closed && self.snap.workspaces.is_empty() {
            return;
        }

        self.request_daemon_restart("No sessions are open. Restarting the daemon to finish the update.");
    }

    fn notify(&mut self, msg: String, error: bool) {
        self.remember(None, if error { 'x' } else { 'i' }, msg.clone());
        self.notice = Some((msg, Instant::now(), error));
        self.notice_term = None;
        self.notice_restart = false;
        self.dirty = true;
    }

    /// Add to the notification history (Ctrl+Space N).
    pub(super) fn remember(&mut self, term: Option<TermId>, kind: char, text: String) {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        // The same thing twice in a row is one entry.
        if self.history.back().is_some_and(|(_, t, k, x)| *t == term && *k == kind && *x == text) {
            return;
        }
        self.history.push_back((now, term, kind, text));
        while self.history.len() > 300 {
            self.history.pop_front();
        }
    }

    fn send(&self, msg: ClientMsg) {
        let _ = self.out.send(msg);
    }

    /// Ask (in the background) whether a newer seshi is out: when the window opens, then now
    /// and then while it stays open, and when you ask (`asked`: that one says what it found
    /// either way, and isn't turned off by the setting). A new one brings up the Update
    /// button.
    fn check_for_update(&mut self, asked: bool) {
        if (!asked && !self.cfg.ui.update_check) || self.update_checking {
            return;
        }
        self.update_checked = Some(Instant::now());
        self.update_checking = asked;
        if asked {
            self.notify("Checking for updates…".into(), false);
        }
        self.spawn_bg(move || {
            let installed = crate::update::installed_version();
            let latest = crate::update::latest_tag();
            let current = installed.as_deref().unwrap_or(env!("CARGO_PKG_VERSION"));
            let newer = latest.as_ref().ok().filter(|tag| crate::update::newer_than(tag, current)).map(|tag| tag.trim_start_matches('v').to_string());
            // Only asked for when there is something new: one more request, now and then.
            let notes = if newer.is_some() { crate::update::changes_since_this() } else { Vec::new() };
            Bg::Then(Box::new(move |app: &mut App| {
                app.installed_version = Some(installed);
                app.update_checked_now(asked, latest.map(|_| newer), notes);
            }))
        });
    }

    fn refresh_installed_version(&self) {
        self.spawn_bg(|| {
            let installed = crate::update::installed_version();
            Bg::Then(Box::new(move |app| {
                app.installed_version = Some(installed);
                app.maybe_restart_daemon(false);
            }))
        });
    }

    /// A local daemon older than the installed seshi, which a restart would replace.
    fn daemon_restartable(&self) -> bool {
        crate::ipc::remote().is_none()
            && !self.daemon_restart_requested
            && !self.daemon_restart_reconnect
            && !self.snap.daemon_version.is_empty()
            && self.installed_version.as_ref().and_then(|r| r.as_ref().ok()).is_some_and(|v| *v != self.snap.daemon_version)
    }

    /// The toast offers Restart now: it stays long enough to be read and clicked.
    fn toast_restart(&self) -> bool {
        self.notice_restart && self.daemon_restartable()
    }

    /// Restart the daemon onto the installed version. Agents stop and resume, so it asks
    /// first unless nothing is open.
    fn ask_restart(&mut self) {
        if !self.daemon_restartable() {
            return;
        }
        let sessions = self.snap.terms.len();
        if sessions == 0 {
            return self.restart_daemon_now();
        }
        let working = self.snap.terms.values().filter(|t| t.status == Status::Working).count();
        self.mode = Mode::Confirm(Box::new(menu::Confirm {
            title: "Restart daemon".into(),
            sub: String::new(),
            what: restart_summary(sessions, working),
            detail: String::new(),
            note: "They come back where they left off once the new daemon is up.".into(),
            list: Vec::new(),
            yes: "Restart".into(),
            key: '\n',
            danger: working > 0,
            act: menu::Act::RestartDaemon,
        }));
    }

    fn restart_daemon_now(&mut self) {
        if self.daemon_restartable() {
            self.request_daemon_restart("Restarting the daemon…");
        }
    }

    fn request_daemon_restart(&mut self, message: &str) {
        self.daemon_restart_requested = true;
        self.notify(message.into(), false);
        self.cmd(Command::KillServer { forget: false });
    }

    fn restart_pending(&self) -> bool {
        self.installed_version.as_ref().and_then(|r| r.as_ref().ok()).is_some_and(|v| {
            v != env!("CARGO_PKG_VERSION") || (!self.snap.daemon_version.is_empty() && *v != self.snap.daemon_version)
        })
    }

    /// What a check for a newer seshi found: the newer version, none, or why it couldn't
    /// look. One you asked for always says.
    fn update_checked_now(&mut self, asked: bool, found: Result<Option<String>, String>, notes: Vec<String>) {
        if asked {
            self.update_checking = false;
        }
        match found {
            Ok(Some(v)) if self.update_available.as_ref() != Some(&v) => {
                self.notify(format!("seshi {v} is out: Update now is in Settings"), false);
                self.update_available = Some(v);
                self.update_notes = notes;
            }
            Ok(Some(v)) if asked => self.notify(format!("seshi {v} is out: Update now is in Settings"), false),
            Ok(None) => {
                self.update_available = None;
                self.update_notes.clear();
                if asked {
                    let version = self.installed_version.as_ref().and_then(|r| r.as_ref().ok()).map(String::as_str).unwrap_or(env!("CARGO_PKG_VERSION"));
                    self.notify(format!("Installed seshi {version} is the latest{}", if self.restart_pending() { "; restart pending" } else { "" }), false);
                }
            }
            Err(e) if asked => self.notify(format!("Couldn't check for updates: {e}"), true),
            _ => {}
        }
    }

    /// Ask before updating: this version, the new one, and what changed.
    fn ask_update(&mut self) {
        if self.updating {
            return;
        }
        let new = self.update_available.clone().unwrap_or_else(|| "the latest".into());
        self.mode = Mode::Confirm(Box::new(menu::Confirm {
            title: "Update seshi".into(),
            sub: String::new(),
            what: format!("{} → {new}", env!("CARGO_PKG_VERSION")),
            detail: String::new(),
            note: if crate::ipc::remote().is_some() {
                "Open sessions keep running. Update the remote daemon on its host after they are closed.".into()
            } else {
                "Open sessions keep running. The daemon restarts automatically when all sessions are closed.".into()
            },
            list: self.update_notes.clone(),
            yes: "Update now".into(),
            key: 'u',
            danger: false,
            act: menu::Act::Update,
        }));
    }

    fn cmd(&self, c: Command) {
        self.send(ClientMsg::Command(c));
    }

    /// Slides and glides: on in Settings, and not over SSH (every frame goes down the link).
    /// Tests look at single frames, so there things are where they end.
    fn motion_on(&self) -> bool {
        self.cfg.ui.motion && crate::ipc::remote().is_none() && !cfg!(test)
    }

    /// The toast is sliding in, settling from its flash, or fading out.
    fn toast_moving(&self) -> bool {
        let Some((_, at, err)) = &self.notice else { return false };
        let hold = screen::toast_hold(*err, self.notice_term.is_some(), self.toast_restart());
        let e = at.elapsed();
        self.motion_on() && (e < motion::TOAST_IN.max(motion::TOAST_FLASH) || (e + motion::TOAST_FADE >= hold && e < hold))
    }

    fn spinner_frame(&self) -> u64 {
        (self.started.elapsed().as_millis() / 90) as u64
    }

    /// A check's ring turns while it runs.
    fn any_checking(&self) -> bool {
        self.snap.terms.values().any(|term| term.check.as_ref().is_some_and(|c| c.state == crate::protocol::CheckState::Checking))
    }

    fn any_working(&self) -> bool {
        self.snap.terms.values().any(|term| term.status == Status::Working)
    }

    // ---- model helpers -------------------------------------------------------------

    fn active_ws(&self) -> Option<&WorkspaceInfo> {
        self.snap.active()
    }

    fn active_tab(&self) -> Option<&TabInfo> {
        self.active_ws().and_then(|w| w.tab())
    }

    fn focused(&self) -> Option<TermId> {
        self.active_tab().map(|t| t.focus)
    }

    /// A floating pane (`seshi popup`) open now: it has the keys until it closes.
    fn popup(&self) -> Option<TermId> {
        self.snap.terms.values().filter(|t| t.popup).map(|t| t.id).max()
    }

    /// Where typing goes: an open popup, else the pane you're on.
    fn typing_to(&self) -> Option<TermId> {
        self.popup().or_else(|| self.focused())
    }

    fn new_parser(&self, rows: u16, cols: u16) -> vt100::Parser {
        vt100::Parser::new(rows.max(1), cols.max(1), self.cfg.scrollback)
    }

    /// Resize panes whose drawn size differs from what the daemon last heard.
    fn sync_sizes(&mut self) {
        // Mid-slide the panes are only drawn at the passing sizes; they're resized once it stops.
        if self.motion.layout_moving() {
            return;
        }
        let panes = self.panes.clone();
        let dragging = self.screen.drag.is_some();
        for (term, r) in panes {
            let want = (r.width.max(2), r.height.max(2));
            if !dragging && self.rewrap.remove(&term) {
                self.rewrap_pane(term, want);
            }
            if self.sizes.get(&term) == Some(&want) {
                continue;
            }
            let old = self.sizes.insert(term, want);
            if old.is_some_and(|o| o.0 != want.0) && self.raw.get(&term).is_some_and(|r| !r.is_empty()) {
                // A new width: replay recent output into a fresh screen so text re-wraps
                // instead of being cut (history included). Mid-drag that would be every
                // step: once the drag ends instead.
                if dragging {
                    self.rewrap.insert(term);
                    if let Some(p) = self.parsers.get_mut(&term) {
                        p.screen_mut().set_size(want.1, want.0);
                    }
                } else {
                    self.rewrap_pane(term, want);
                }
            } else if let Some(p) = self.parsers.get_mut(&term) {
                p.screen_mut().set_size(want.1, want.0);
            }
            self.send(ClientMsg::Resize { term, cols: want.0, rows: want.1 });
        }
    }

    /// A fresh screen for `term` at `(cols, rows)`, from the output kept for it.
    fn rewrap_pane(&mut self, term: TermId, (cols, rows): (u16, u16)) {
        let mut p = self.new_parser(rows, cols);
        if let Some(raw) = self.raw.get_mut(&term) {
            p.process(raw.make_contiguous());
        }
        self.parsers.insert(term, p);
    }

    // ---- server messages -----------------------------------------------------------

    /// What the sidebar calls a session: what you renamed it to, else its worktree or folder.
    /// Notes about a session use this, so they say the name you'd look for.
    pub(super) fn session_name(&self, term: TermId) -> String {
        let model = self.screen_model();
        if let Some((_, _, s)) = screen::find(&model, term) {
            return s.name.clone();
        }
        self.snap.terms.get(&term).map(|t| if t.label.trim().is_empty() { t.display_name() } else { t.label.trim().to_string() }).unwrap_or_default()
    }

    fn describe_term(&self, term: TermId) -> String {
        let name = self.session_name(term);
        match self.snap.locate(term) {
            Some((w, t)) => {
                let idx = w.tabs.iter().position(|x| x.id == t.id).unwrap_or(0) + 1;
                format!("{name} ({} › {idx})", w.name)
            }
            None => name,
        }
    }

    // ---- input ---------------------------------------------------------------------

    /// The colour of a workspace, from the configured palette.
    fn ws_color(&self, w: &WorkspaceInfo) -> ratatui::style::Color {
        let pal = &self.cfg.ui.workspace_colors;
        if pal.is_empty() {
            return self.theme.accent;
        }
        crate::theme::parse_color(&pal[w.color as usize % pal.len()]).unwrap_or(self.theme.accent)
    }

    fn yank(&mut self, text: String) {
        self.mode = Mode::Normal;
        // Copying across panes: each one's piece under its name, in the order taken.
        let text = if self.copy_set.is_empty() {
            text
        } else {
            let last = self.copy_term.take();
            let mut parts = std::mem::take(&mut self.copy_set);
            if let Some(t) = last.filter(|_| !text.is_empty()) {
                parts.retain(|(p, _)| *p != t);
                parts.push((t, text));
            }
            let n = parts.len();
            let named: Vec<(String, String)> =
                parts.into_iter().map(|(t, s)| (self.snap.terms.get(&t).map(|i| i.display_name()).unwrap_or_else(|| format!("pane {t}")), s)).collect();
            let _ = copy::to_clipboard(&copy::join_pieces(&named));
            self.notify(format!("Copied from {n} panes"), false);
            return;
        };
        if text.is_empty() {
            return;
        }
        let lines = text.lines().count().max(1);
        let native = copy::to_clipboard(&text);
        let how = if native { "" } else { " (via terminal)" };
        let _ = lines;
        self.notify(format!("Copied{how}"), false);
    }

    fn enter_copy(&mut self, term: TermId) -> bool {
        let Some(p) = self.parsers.get_mut(&term) else { return false };
        let mut c = copy::Copy::new(term, p);
        if let Some((_, r)) = self.panes.iter().find(|(t, _)| *t == term) {
            c.height = r.height as usize;
            c.width = r.width as usize;
        }
        self.mode = Mode::Copy(Box::new(c));
        true
    }

    /// Keep the last ~768 KiB a pane printed (for re-wrapping on resize).
    /// How a pane takes the wheel and how much history it has, in one line (to say what's
    /// going on when it won't scroll).
    fn pane_info(&mut self, term: TermId) -> Option<String> {
        let p = self.parsers.get_mut(&term)?;
        let at = p.screen().scrollback();
        p.screen_mut().set_scrollback(usize::MAX);
        let history = p.screen().scrollback();
        p.screen_mut().set_scrollback(at);
        let s = p.screen();
        let agent = self.snap.terms.get(&term).is_some_and(|t| t.agent.is_some());
        let wheel = if s.alternate_screen()
            || (history == 0 && (agent || s.mouse_protocol_mode() != vt100::MouseProtocolMode::None))
        {
            "the program"
        } else {
            "seshi"
        };
        let full = if s.alternate_screen() { "full-screen (it scrolls itself)" } else { "normal screen" };
        let raw = self.raw.get(&term).map(|r| r.len() / 1024).unwrap_or(0);
        Some(format!("{full} · the wheel goes to {wheel} · {history} lines of history, {at} up · {raw} KB kept · {:?}", self.mode).chars().take(200).collect())
    }

    fn keep_raw(&mut self, term: TermId, data: &[u8], replace: bool) {
        // Replayed into a fresh screen when a pane's width changes (so text re-wraps): what's
        // here is all the history it has after. An agent redraws a lot while it works, so
        // this is generous.
        const CAP: usize = 8 * 1024 * 1024;
        let ring = self.raw.entry(term).or_default();
        if replace {
            ring.clear();
        }
        ring.extend(data.iter().copied());
        if ring.len() > CAP {
            let extra = ring.len() - CAP;
            ring.drain(..extra);
        }
    }

    /// Show the focused program's cursor style (bar, block, underline) in the real terminal.
    fn sync_cursor_style(&mut self) {
        use crossterm::cursor::SetCursorStyle as S;
        let want = self.focused().and_then(|t| self.cursor_style.get(&t).copied()).unwrap_or(0);
        if want == self.cursor_sent {
            return;
        }
        self.cursor_sent = want;
        let style = match want {
            1 => S::BlinkingBlock,
            2 => S::SteadyBlock,
            3 => S::BlinkingUnderScore,
            4 => S::SteadyUnderScore,
            5 => S::BlinkingBar,
            6 => S::SteadyBar,
            _ => S::DefaultUserShape,
        };
        if !cfg!(test) {
            let _ = execute!(std::io::stdout(), style);
        }
    }

    /// The clipboard's image, saved as a PNG; its path is pasted into the focused pane
    /// (Claude Code, Codex and friends attach an image given by path).
    pub(super) fn paste_image(&mut self) {
        let Some(term) = self.focused() else { return };
        // Reading the clipboard and encoding the PNG can take a while for a big image.
        self.spawn_bg(move || {
            let made = clipboard_image_to_file();
            Bg::Then(Box::new(move |app: &mut App| match made {
                Ok((path, w, h)) => {
                    let text = files::quote_path(&path);
                    let bracketed = app.parsers.get(&term).is_some_and(|p| p.screen().bracketed_paste());
                    let data = if bracketed { format!("\x1b[200~{text} \x1b[201~") } else { format!("{text} ") };
                    app.send(ClientMsg::Input { term, data: data.into_bytes() });
                    app.notify(format!("pasted the clipboard image ({w}×{h}) as a file"), false);
                }
                Err(e) => app.notify(e, true),
            }))
        });
    }

    /// When to draw a pane that's mid synchronized update: its start + 100 ms, if that's
    /// still ahead.
    fn sync_hold_until(&self) -> Option<Instant> {
        // The previous frame's panes must not hold up navigation to a different session.
        if !matches!(self.mode, Mode::Normal) || self.view.is_some()
            || self.focused().is_some_and(|focused| !self.panes.iter().any(|(id, _)| *id == focused)) {
            return None;
        }
        let limit = Duration::from_millis(100);
        self.panes
            .iter()
            .filter_map(|(t, _)| self.sync_hold.get(t))
            .map(|at| *at + limit)
            .filter(|until| *until > Instant::now())
            .max()
    }

    /// Programs that asked (mode 1004) hear when their pane or the window gains or loses
    /// focus: `ESC [ I` / `ESC [ O`.
    fn report_focus(&mut self) {
        let now = self.focused().map(|t| (t, self.window_focused));
        if now == self.focus_sent {
            return;
        }
        if let Some((t, true)) = self.focus_sent
            && self.focus_report.contains(&t)
            && now != Some((t, true))
        {
            self.send(ClientMsg::Input { term: t, data: b"\x1b[O".to_vec() });
        }
        if let Some((t, true)) = now
            && self.focus_report.contains(&t)
        {
            self.send(ClientMsg::Input { term: t, data: b"\x1b[I".to_vec() });
        }
        self.focus_sent = now;
    }

    /// (offset from the bottom, lines of history) for a pane.
    pub(super) fn history(&mut self, term: TermId) -> (usize, usize) {
        let Some(p) = self.parsers.get_mut(&term) else { return (0, 0) };
        let cur = p.screen().scrollback();
        p.screen_mut().set_scrollback(usize::MAX);
        let total = p.screen().scrollback();
        p.screen_mut().set_scrollback(cur);
        (cur, total)
    }

    pub(super) fn scroll_to(&mut self, term: TermId, offset: usize) {
        let cur = self.history(term).0 as i32;
        self.scroll_by(term, offset as i32 - cur);
    }

    // ---- actions -------------------------------------------------------------------

    /// Where the user is: the focused pane's folder, else the workspace's.
    fn here_dir(&self) -> PathBuf {
        self.focused()
            .and_then(|id| self.snap.terms.get(&id))
            .map(|t| t.cwd.clone())
            .filter(|p| p.is_dir())
            .or_else(|| self.active_ws().map(|w| w.cwd.clone()))
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default()
    }

}

fn same_dir(a: &std::path::Path, b: &std::path::Path) -> bool {
    let a = a.canonicalize().unwrap_or_else(|_| a.to_path_buf());
    let b = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());
    a == b
}

#[cfg(test)]
mod tests;
