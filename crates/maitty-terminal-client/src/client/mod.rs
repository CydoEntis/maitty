//! The TUI client: renders the daemon's snapshot, keeps a terminal emulator per pane, and
//! turns keys into either pane input or commands.

mod actions;
mod background;
mod branch;
pub use maitty_machines::connections;
mod copy;
mod design;
mod files;
mod find;
mod ids;
mod input;
pub use maitty_machines::link;
mod machine_config;
mod machine_flow;
mod machine_ids;
mod machine_links;
mod markdown;
mod marks;
mod menu;
mod modal;
mod motion;
mod overlap;
mod pick;
mod recipes;
use maitty_machines::remote_policy;
#[cfg(test)]
mod remote_policy_tests;
pub(crate) use maitty_machines::remote_snapshot;
mod remote_image;
#[cfg(test)]
mod remote_image_tests;
#[cfg(test)]
mod remote_snapshot_tests;
mod render;
pub mod restart_prompt;
mod routing;
mod screen;
mod send_file;
mod spinners;
pub mod ssh_hosts;
mod tasks;
mod themes;
mod view_keys;
mod views;

use crate::brand::APP_COMMAND;
use crate::config::{Config, Keymap};
use crate::ipc;
use crate::keys::{self, Action, KeySpec};
use crate::protocol::*;
use crate::theme::Theme;
use anyhow::Result;
use connections::{Burst, Connections, NotConnected, SERVER_BURST};
use futures_util::{FutureExt, StreamExt};
#[cfg(test)]
use ids::ClientSession;
use ids::{ClientTerm, MachineId, MachineSnap, Outgoing, Remote};
use ratatui::crossterm::event::{
    self, Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton,
    MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Position, Rect};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// "3 sessions stop and resume, 1 is working": what a daemon restart costs.
fn restart_summary(sessions: usize, working: usize, waiting: usize) -> String {
    let stops = if sessions == 1 {
        "1 session stops and resumes".to_string()
    } else {
        format!("{sessions} sessions stop and resume")
    };
    let working = match working {
        0 => None,
        1 => Some("1 is working".to_string()),
        n => Some(format!("{n} are working")),
    };
    let waiting = (waiting > 0).then(|| format!("{waiting} waiting for you"));
    [Some(stops), working, waiting]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ")
}

/// How often the client looks at its timers (spinners, toasts, pending focus).
/// How often an open window asks again whether a newer the app is out.
const UPDATE_CHECK_EVERY: Duration = Duration::from_secs(3 * 60 * 60);
const TICK: Duration = Duration::from_millis(50);
/// The shortest time between two frames (about 80 a second at most).
const MIN_FRAME: Duration = Duration::from_millis(12);
/// Keys of one paste come this close together (Windows); typing never does.
const PASTE_GAP: Duration = Duration::from_millis(8);
/// The longest a paste is waited on before what has come is handed over.
const PASTE_MAX_WAIT: Duration = Duration::from_secs(1);
/// How long a note stays in the bottom bar.
/// The most characters of the copied text a toast previews.
const COPY_PREVIEW_CHARS: usize = 72;
const NOTICE_FOR: Duration = Duration::from_secs(5);
/// How long the mode bar's flash line stays.
pub(in crate::client) const BAR_FLASH_FOR: Duration = Duration::from_millis(2500);
/// What a toast says beyond its title: a detail row and an optional preview row.
#[derive(Debug, Clone, PartialEq)]
struct NoticeCard {
    detail: String,
    preview: Option<String>,
}

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
    /// Shortcut mode: each key runs its leader action and the mode stays.
    /// `acted`: a shortcut has run, so the key map no longer opens on a pause.
    Prefix {
        since: Instant,
        acted: bool,
    },
    Prompt {
        kind: PromptKind,
        input: String,
    },
    Copy(Box<copy::Copy>),
    /// Pick an existing worktree of the repo, or type a branch to create one.
    Worktrees {
        ws: WsId,
        cmd: Option<String>,
        items: Option<Vec<WorktreeEntry>>,
        query: String,
        sel: usize,
    },
    /// The app layout: open a folder as a project.
    Finder(Box<screen::Finder>),
    /// The app layout: new pane (project, worktree, what to run).
    ScreenPane(screen::NewPaneHy),
    /// The app layout: the settings overlay.
    ScreenSettings(Box<design::SettingsView>),
    Themes {
        query: String,
        sel: usize,
        saving: bool,
    },
    Spinners(Box<spinners::Picker>),
    /// The app layout: keyboard cursor in the sidebar; bare keys act like leader keys.
    Side,
    /// A right-click menu (the app layout).
    ScreenMenu(Box<menu::ScreenMenu>),
    /// Find a file / search the code.
    Find(Box<find::FindView>),
    /// Switch branch.
    Branch(Box<branch::BranchView>),
    /// Notification history (the selected row, newest first).
    /// Usage and cost: what each agent reported using, and the plan limits.
    Usage,
    History {
        sel: usize,
    },
    /// A repo's worktrees and which to remove.
    Tidy(Box<screen::TidyView>),
    /// "Close …?" with confirm / cancel.
    Confirm(Box<menu::Confirm>),
    /// Run a repo's check command? Asked only when you open its waiting item.
    Approve(Box<screen::Approve>),
    /// A project's checkpoints (`PROJECT · <name>`), being edited.
    ProjectCheck(Box<screen::check_settings::CheckDraft>),
    /// The switcher: projects and sessions, typed to filter.
    GoTo {
        query: String,
        sel: usize,
    },
    /// Writing a quick follow-up to an agent (beside its row, or in its Inbox row).
    Compose(Box<screen::Compose>),
    /// A session asks to start helpers (beside its row); `side`: back to the sidebar after.
    Helpers {
        term: ClientTerm,
        side: bool,
    },
    Secret {
        term: ClientTerm,
        id: u64,
        side: bool,
    },
    /// The leader's key map (or its second step), typed to search.
    KeyMap(Box<screen::KeyMap>),
    /// Every command with its key (the sidebar's "menu").
    Actions {
        sel: usize,
    },
    /// The Updates box: what is new, download, restart.
    Updates,
    /// A tab being renamed in its pill.
    RenameTab(Box<screen::TabName>),
    /// The computers reached over ssh: how each stands, and adding or removing one.
    Machines(Box<screen::machines::MachinesView>),
}

/// A view that replaces the pane area.
pub(super) enum View {
    Changes(Box<views::ChangesView>),
    Files(Box<views::FilesTree>),
    /// Both diffs of a file two checkouts changed (in the sheet).
    Both(Box<screen::BothView>),
    /// A check's evidence (in the sheet).
    Check(Box<screen::CheckView>),
    /// Where a checkout stands (in the sheet).
    Branch(Box<screen::BranchSheet>),
    /// The project's reference images and design files (in the sheet).
    References(Box<screen::ReferencesSheet>),
}

/// Clickable chips and buttons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Btn {
    Answer(ClientTerm, char),
    /// A key inside the current view ('\n' for Enter).
    ViewKey(char),
    /// A key with its modifiers inside the current view (the editor's Ctrl+S, Esc, …).
    ViewChord(KeyCode, KeyModifiers),
    /// A row of the current view's list.
    Row(usize),
}

pub(super) enum Bg {
    /// A review, and what its marked files were like when it was read (`views::still_reviewed`).
    Changes(
        PathBuf,
        Result<Box<tasks::Review>, String>,
        HashMap<String, u64>,
    ),
    Tree(
        PathBuf,
        Vec<views::FileNode>,
        std::collections::HashMap<String, char>,
    ),
    TreeRecent(PathBuf, Vec<files::FileEntry>),
    /// A finished action: its message, and whether the open review should reload.
    Done(Result<String, String>, bool),
    /// A worktree's branch merged (or not): then the worktree and its branch go.
    Merged(Result<String, String>, PathBuf),
    /// A repo's worktrees, each looked at, for the clean-up list.
    Tidy(PathBuf, Vec<screen::TidyRow>),
    /// Each checkout's diff of a heads-up's file: (repo, [(checkout, diff)]).
    BothDiff(PathBuf, Vec<(String, String)>),
    /// Files more than one checkout of a repo changed (by the repo's folder).
    Overlaps(PathBuf, Vec<overlap::Overlap>),
    /// A checkout's uncommitted lines, added and removed.
    /// None: they could not be counted (git failed there), so the rows show none.
    LinesChanged(PathBuf, Option<(i64, i64)>),
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
    /// The finder's preview of one row: (folder, row, lines).
    FindPreview(PathBuf, find::Target, Vec<String>),
    /// The Files view's preview of one file: (the view's root, file, lines).
    FilesPreview(PathBuf, PathBuf, Vec<String>),
    /// A file's diff for the Changes view: (folder, which file, lines).
    Diff(PathBuf, usize, Vec<String>),
    /// Slow work done; finish it on the UI thread.
    Then(Box<dyn FnOnce(&mut App) + Send>),
}

/// The cursor to show, as DECSCUSR's number (0 is the terminal's own): what the focused pane's
/// program asked for, else the user's `ui.cursor` and `ui.cursor_blink`. A program asking for 0
/// ("your default") gets the user's setting too.
fn cursor_code(program: Option<u8>, ui: &crate::config::Ui) -> u8 {
    if let Some(asked) = program.filter(|c| *c != 0) {
        return asked;
    }
    let blinking = match ui.cursor.as_str() {
        "block" => 1,
        "underline" => 3,
        "line" => 5,
        _ => return 0,
    };
    // DECSCUSR's odd numbers blink and the even ones after them hold still.
    if ui.cursor_blink {
        blinking
    } else {
        blinking + 1
    }
}

/// Say once that the config file named a retired look, and write what stands for it
/// into the file, so it is not said again.
fn settle_retired_layout(app: &mut App) {
    let Some(r) = app.cfg.ui.retired_layout.take() else {
        return;
    };
    let mut writes: Vec<(&str, toml_edit::Value)> = vec![("ui.layout", r.kind.name().into())];
    if r.gap {
        writes.push(("ui.gap", "0".into()));
    }
    for (path, value) in writes {
        if let Err(e) = modal::write(path, value) {
            app.notify(format!("{} (could not save it: {e:#})", r.note()), true);
            return;
        }
    }
    app.notify(r.note(), false);
}
#[cfg(test)]
mod cursor_tests {
    use super::cursor_code;
    use crate::config::Ui;

    fn ui(shape: &str, blink: bool) -> Ui {
        Ui {
            cursor: shape.into(),
            cursor_blink: blink,
            ..Default::default()
        }
    }

    #[test]
    fn your_cursor_shows_where_the_program_asks_for_none() {
        assert_eq!(cursor_code(None, &ui("block", true)), 1);
        assert_eq!(cursor_code(None, &ui("block", false)), 2);
        assert_eq!(cursor_code(None, &ui("underline", true)), 3);
        assert_eq!(cursor_code(None, &ui("underline", false)), 4);
        assert_eq!(cursor_code(None, &ui("line", true)), 5);
        assert_eq!(cursor_code(None, &ui("line", false)), 6);
        assert_eq!(
            cursor_code(Some(0), &ui("line", false)),
            6,
            "a program asking for the default gets yours"
        );
    }

    #[test]
    fn a_program_that_asks_for_its_own_cursor_gets_it() {
        assert_eq!(
            cursor_code(Some(5), &ui("block", false)),
            5,
            "vim's insert-mode bar wins over your block"
        );
        assert_eq!(cursor_code(Some(2), &ui("block", true)), 2);
    }
}

/// Save the clipboard image as a PNG under the data folder; old pastes (a week) are
/// cleared out. Returns (path, width, height).
fn clipboard_image_to_file() -> Result<(PathBuf, usize, usize), String> {
    let img = clipboard_image()?;
    let dir = crate::config::data_dir().join("pastes");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if e.metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| t.elapsed().is_ok_and(|a| a.as_secs() > 7 * 86_400))
            {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("paste-{ms}.png"));
    write_png(&path, img.width as u32, img.height as u32, &img.bytes)?;
    Ok((path, img.width, img.height))
}

/// The image on the clipboard. Slow for a big one: call it off the UI thread.
fn clipboard_image() -> Result<arboard::ImageData<'static>, String> {
    // Tests never touch the machine's clipboard: what is on it is not theirs to read.
    if cfg!(test) {
        return Err("there's no image on the clipboard".into());
    }
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("can't open the clipboard: {e}"))?;
    cb.get_image()
        .map_err(|_| "there's no image on the clipboard".to_string())
}

/// RGBA pixels to a PNG file.
fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    encode_png(file, w, h, rgba)
}

/// RGBA pixels as a PNG into a file already open.
fn encode_png(file: std::fs::File, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
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
    if out.status.success() {
        Ok(format!("removed {}", dir.display()))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// A row in the worktree picker.
#[derive(Debug, Clone)]
enum WtRow {
    Existing(WorktreeEntry),
    Create(String),
}

#[derive(Debug, Clone, PartialEq)]
enum PromptKind {
    RenamePane(ClientTerm),
    /// A chat's name, shared with every client (empty: automatic).
    RenameConversation(crate::conversation::ConversationId),
    RenameProject(String),
    RenameTab(WsId, TabId),
    NewWorkspace,
    ConfirmCloseWorkspace(WsId),
    ConfirmKillServer,
    ConfirmRemoveWorktree(WsId),
    /// A file to copy into the project's references: (project root, folder a relative path starts from).
    AddReference(PathBuf, PathBuf),
    /// A file on this computer to send to the machine this pane is on (ADR 0007).
    SendFile(ClientTerm),
}

impl PromptKind {
    fn label(&self) -> &'static str {
        match self {
            PromptKind::RenamePane(_) => "Rename pane (empty: automatic)",
            PromptKind::RenameConversation(_) => "Rename chat (empty: automatic)",
            PromptKind::RenameProject(_) => "Rename project (empty: its folder name)",
            PromptKind::RenameTab(..) => "Rename session",
            PromptKind::NewWorkspace => "New fleet: name it (empty = its folder)",
            PromptKind::ConfirmCloseWorkspace(_) => "Close workspace and all its panes? (y/n)",
            PromptKind::ConfirmKillServer => "Kill the server and every pane? (y/n)",
            PromptKind::ConfirmRemoveWorktree(_) => "Remove this worktree? (y / f = force / n)",
            PromptKind::AddReference(..) => "Add a reference: the file's path",
            PromptKind::SendFile(_) => "Send a file to this machine: its path",
        }
    }
    fn is_confirm(&self) -> bool {
        matches!(
            self,
            PromptKind::ConfirmCloseWorkspace(_)
                | PromptKind::ConfirmKillServer
                | PromptKind::ConfirmRemoveWorktree(_)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Pane(ClientTerm),
    Button(Btn),
    Screen(screen::ScreenHit),
}

pub struct App {
    /// A newer release, found when the app opened or since (the Update button shows).
    update_available: Option<String>,
    installed_version: Option<Result<String, String>>,
    /// Whether Claude's hooks are installed, read in the background when Settings > Agents first shows (None until then).
    claude_hooks: Option<bool>,
    claude_hooks_asked: bool,
    /// The same for Codex's lifecycle hooks: installed, and whether Codex has been told to trust them.
    codex_hooks: Option<maitty_provider_codex::HooksState>,
    codex_hooks_asked: bool,
    /// The theme browser's tab: the Light themes when true, else the Dark ones.
    theme_tab_light: bool,
    /// What changed in it (and any releases between), for the update dialog.
    update_notes: Vec<String>,
    /// When the app last asked whether a newer release is out.
    update_checked: Option<Instant>,
    /// The title last written to the host terminal's tab, so it is only written when it changes.
    window_title: Option<String>,
    /// An update is downloading.
    updating: bool,
    /// Why the last install failed, until it is tried again or the box is read.
    update_failed: Option<String>,
    /// The version the Updates box was told to skip this run.
    update_skipped: Option<String>,
    /// The installed daemon needs a restart after the panes are closed.
    update_installed_this_run: bool,
    daemon_restart_requested: bool,
    daemon_restart_reconnect: bool,
    /// This window is on an older the app than the one installed: it closes and the installed
    /// one opens in its place (`cli::restart::relaunch`).
    relaunch: bool,
    /// Asking, because you said to, whether a newer the app is out.
    update_checking: bool,
    /// When this window last had to attach to the server again.
    reattached: Option<Instant>,
    cfg: Config,
    theme: Theme,
    keymap: Keymap,
    notice: Option<(String, Instant, bool)>,
    /// The session the note is about (an agent finished or needs you): clicking it goes there.
    notice_term: Option<ClientTerm>,
    /// The toast offers Restart now (the update just installed, the daemon still old).
    notice_restart: bool,
    /// The toast is a hint about keys, not news: it is marked `i`, not ✓.
    notice_hint: bool,
    /// The file being sent to another machine, if any (ADR 0007).
    sending: Option<send_file::Sending>,
    /// The toast's second and third rows, when the notice has them.
    notice_card: Option<NoticeCard>,
    /// The selection just copied, glowing for a moment after copy mode closed.
    copy_flash: Option<copy::Flash>,
    /// A line the mode bar shows in place of its hint for `BAR_FLASH_FOR` (`+ name started`).
    bar_flash: Option<(String, Instant)>,
    /// The undo offer after a session is closed, and a first press waiting for its second.
    closing: screen::Closing,
    /// Whether the window has its daemon, and where the tries to get it back stand.
    service: screen::Service,
    /// `r` asked to try now; `s` asked to start the service. Taken by the next tick.
    service_retry_now: bool,
    service_start_now: bool,
    /// When the daemon's last restart was first seen to have stopped agents.
    stopped_since: Option<Instant>,
    /// What happened lately, newest last: (unix secs, pane, kind, text). Kinds: '!' needs
    /// you, '✓' finished, 'i' a message, 'x' an error.
    pub(super) history: std::collections::VecDeque<(u64, Option<ClientTerm>, char, String)>,
    /// When the right button last went down (menus open on press or release, once).
    right_down: Option<Instant>,
    /// This machine's daemon.
    snap: Snapshot,
    /// The other machines' daemons.
    remotes: std::collections::BTreeMap<MachineId, Remote>,
    /// The daemon each machine name was first seen with.
    machine_ids: machine_ids::MachineIds,
    /// How to reach the supervisors of the configured machines.
    links: link::LinkControl,
    /// What the Machines dialog has under way: a look at a machine, a login in a popup.
    machine_flow: machine_flow::Flow,
    /// Typed input that found its machine disconnected, by machine: told once it is back.
    /// Input is dropped, never kept for later: replaying keystrokes into a pane that moved on
    /// could run commands the person no longer meant.
    unsent_input: std::collections::BTreeMap<MachineId, u32>,
    /// Which machine holds the focus, and which have a request out waiting for an answer.
    route: routing::Routing,
    /// Machines turned away because a different daemon answers at their name, with why.
    refused: std::collections::BTreeMap<MachineId, String>,
    /// The machine whose daemon this window shows.
    pub(super) machine: crate::ipc::Machine,
    got_state: bool,
    parsers: HashMap<ClientTerm, vt100::Parser>,
    /// The mouse pointer shape last asked of the terminal (OSC 22).
    pointer: &'static str,
    /// The pane copy mode was on when it yanked (for joining with `copy_set`).
    copy_term: Option<ClientTerm>,
    /// Selections kept from other panes while copying across a split (Tab in copy mode):
    /// (pane, text), copied together with the current one.
    copy_set: Vec<(ClientTerm, String)>,
    /// Where commands started in each pane's history (see `marks`).
    marks: HashMap<ClientTerm, marks::Marks>,
    sizes: HashMap<ClientTerm, (u16, u16)>,
    mode: Mode,
    /// The sidebar shows; hidden, a `»` chip brings it back.
    sidebar: bool,
    /// The last alert per agent (its status, when) and for any agent: the same news again
    /// soon doesn't ring twice.
    alerted: HashMap<ClientTerm, (Status, Instant)>,
    last_alert: Option<(crate::alert::Kind, Instant)>,
    /// Only ring if the pending status still matches the latest snapshot at its deadline.
    settling: HashMap<ClientTerm, (Status, Instant)>,
    /// The machines whose first held-back alert this run has been explained (toasts are off for a machine until you turn them on).
    alerts_off_told: HashSet<MachineId>,
    /// What's moving, and how far the sidebar and the sheet are in this frame (0 to 1).
    motion: motion::Motion,
    side_frac: f32,
    sheet_frac: f32,
    hits: Vec<(Rect, Hit)>,
    /// How many of `hits` lie under the topmost dimmed overlay (`dim_all`): the pointer
    /// neither tints them nor takes their shape, since they are not what a look says is live.
    hits_under_overlay: usize,
    /// Inner rects of the panes drawn last frame.
    panes: Vec<(ClientTerm, Rect)>,
    /// Outer rects of the panes drawn last frame (for neighbour search).
    pane_frames: Vec<(ClientTerm, Rect)>,
    /// When the wheel last paged an agent's full-screen view (see the wheel in input.rs).
    wheel_page: Option<Instant>,
    /// Pages the wheel has sent up to an agent that scrolls itself, less those sent down:
    /// The app can't see where it is, so this is how far "Back to bottom" has to page down.
    paged_up: std::collections::HashMap<ClientTerm, u32>,
    /// When a remote machine's refused copy was last told, so it is told once a minute.
    copy_refusal_told: Option<Instant>,
    /// The panes drawn at the last `sync_sizes`, to see which have just come into view.
    shown_panes: std::collections::HashSet<ClientTerm>,
    out: mpsc::UnboundedSender<Outgoing>,
    quit: Option<String>,
    dirty: bool,
    started: Instant,
    /// The first start of this version: its splash plays the intro.
    splash_cold: bool,
    /// Whether this run's splash plays the intro, decided when it is first drawn.
    splash_play: Option<bool>,
    open: Option<PathBuf>,
    /// Where a left-button press started, for drag-to-select.
    drag: Option<(ClientTerm, Position)>,
    bg: mpsc::UnboundedSender<Bg>,
    #[cfg(test)]
    run_background_jobs_inline: bool,
    view: Option<View>,
    /// Where the mouse is, for hover highlights.
    hover: Option<Position>,
    /// The status line's message can be undone with `u`.
    undo_hint: bool,
    /// The last click, for double-click detection.
    last_click: Option<(Hit, Instant)>,
    /// Shortcut mode was on when the open popup took the keys; it comes back when that closes.
    shortcut_held: bool,
    /// When shortcut mode last said a key did nothing, so mashing keys shows it once.
    dead_key_hint: Option<Instant>,
    /// The open view a click into a pane set aside: while that view stays open, keys go to the
    /// pane. A click back in the side sheet, or another view opening, hands them back.
    view_set_aside: Option<std::mem::Discriminant<View>>,
    /// Bumped each time a view opens, to tell an action that opened one (even of the same kind).
    view_opened: u64,
    /// The welcome screen is up; which of its panes is selected.
    splash: bool,
    /// The app layout's own state.
    screen: screen::Screen,
    /// The terminal window has focus (for alerts about the session you're looking at).
    window_focused: bool,
    /// The background we told the terminal to use (OSC 11), so its padding matches.
    osc_bg: Option<ratatui::style::Color>,
    /// Panes in the middle of a synchronized update (mode 2026): since when. Not drawn
    /// until it ends (or 100 ms pass), so redraws don't flicker.
    sync_hold: HashMap<ClientTerm, Instant>,
    /// Panes whose program asked to hear about focus (mode 1004).
    focus_report: HashSet<ClientTerm>,
    /// The focus we last told programs about: (pane, window focused).
    focus_sent: Option<(ClientTerm, bool)>,
    /// The last left-click in a pane, for double-click.
    pane_click: Option<(Position, Instant)>,
    /// Recent raw output per pane, to re-wrap the screen when its width changes.
    raw: HashMap<ClientTerm, std::collections::VecDeque<u8>>,
    /// Panes whose width changed during a drag: re-wrapped from `raw` once it ends.
    rewrap: std::collections::HashSet<ClientTerm>,
    /// The cursor style each pane's program asked for (DECSCUSR 0-6), and the one we set.
    cursor_style: HashMap<ClientTerm, u8>,
    cursor_sent: u8,
}

pub fn run(opts: Options) -> Result<()> {
    crate::proc::ahead_of_the_panes();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(run_async(opts))
}

async fn run_async(opts: Options) -> Result<()> {
    let mut connections = Connections::default();
    connections.add(MachineId::LOCAL, ipc::open_or_spawn(true).await?);
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Outgoing>();

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
    settle_retired_layout(&mut app);
    let mut links = link::Links::start(
        link::configured_machines(&app.cfg.machines, ipc::SSH_PROGRAM),
        link::Timing::DEFAULT,
        || ipc::terminal_dial(true),
    );
    app.links = links.control();

    let mut terminal = ratatui::init();
    // A panic must leave the shell as it found it: ratatui's own hook leaves the alternate
    // screen and raw mode; this one also turns off what the app turned on.
    let earlier = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        earlier(info);
    }));
    if app.cfg.ui.mouse {
        let _ = execute!(std::io::stdout(), event::EnableMouseCapture);
    }
    let _ = execute!(
        std::io::stdout(),
        event::EnableBracketedPaste,
        event::EnableFocusChange
    );
    enhance_keyboard();

    let result = app
        .event_loop(
            &mut terminal,
            &mut connections,
            &mut links,
            &mut out_rx,
            &mut bg_rx,
        )
        .await;

    restore_terminal();
    result?;
    if app.relaunch {
        crate::cli::restart::request_relaunch();
    } else if let Some(reason) = app.quit {
        println!("[{reason}]");
    }
    Ok(())
}

/// Whether the app asked the terminal for the kitty keyboard protocol (so it gives it back).
static KEYBOARD_ENHANCED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Without the kitty keyboard protocol a terminal sends Shift+Enter as a plain Enter, which an
/// agent reads as "send": ask for it where the terminal has it. (Windows reports modifiers
/// on its own.)
fn enhance_keyboard() {
    #[cfg(not(windows))]
    if crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false)
        && execute!(
            std::io::stdout(),
            event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            )
        )
        .is_ok()
    {
        KEYBOARD_ENHANCED.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Undo everything the app turned on in the terminal: mouse, paste and focus reporting, the
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
    let _ = execute!(
        std::io::stdout(),
        crossterm::cursor::SetCursorStyle::DefaultUserShape
    );
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
    fn new(
        cfg: Config,
        out: mpsc::UnboundedSender<Outgoing>,
        open: Option<PathBuf>,
        bg_tx: mpsc::UnboundedSender<Bg>,
    ) -> App {
        let keymap = cfg.keymap();
        let mut app = App {
            update_available: None,
            installed_version: None,
            claude_hooks: None,
            claude_hooks_asked: false,
            codex_hooks: None,
            codex_hooks_asked: false,
            theme_tab_light: false,
            update_notes: Vec::new(),
            update_checked: None,
            window_title: None,
            updating: false,
            update_failed: None,
            update_skipped: None,
            update_installed_this_run: false,
            daemon_restart_requested: false,
            daemon_restart_reconnect: false,
            relaunch: false,
            update_checking: false,
            reattached: None,
            theme: cfg.theme(),
            sidebar: cfg.ui.sidebar,
            alerted: HashMap::new(),
            last_alert: None,
            settling: HashMap::new(),
            alerts_off_told: HashSet::new(),
            motion: motion::Motion::default(),
            side_frac: 1.0,
            sheet_frac: 1.0,
            keymap,
            cfg,
            notice: None,
            notice_term: None,
            notice_restart: false,
            notice_hint: false,
            sending: None,
            notice_card: None,
            copy_flash: None,
            bar_flash: None,
            closing: Default::default(),
            service: Default::default(),
            service_retry_now: false,
            service_start_now: false,
            stopped_since: None,
            history: Default::default(),
            right_down: None,
            snap: Snapshot::default(),
            remotes: Default::default(),
            machine_ids: machine_ids::load(),
            links: Default::default(),
            machine_flow: Default::default(),
            unsent_input: Default::default(),
            route: Default::default(),
            refused: Default::default(),
            machine: crate::ipc::Machine::current(),
            got_state: false,
            parsers: HashMap::new(),
            marks: HashMap::new(),
            copy_set: Vec::new(),
            copy_term: None,
            pointer: "default",
            sizes: HashMap::new(),
            mode: Mode::Normal,
            hits: Vec::new(),
            hits_under_overlay: 0,
            panes: Vec::new(),
            pane_frames: Vec::new(),
            wheel_page: None,
            paged_up: std::collections::HashMap::new(),
            copy_refusal_told: None,
            shown_panes: std::collections::HashSet::new(),
            out,
            quit: None,
            dirty: true,
            started: Instant::now(),
            splash_cold: false,
            splash_play: None,
            open,
            drag: None,
            bg: bg_tx,
            #[cfg(test)]
            run_background_jobs_inline: false,
            view: None,
            hover: None,
            undo_hint: false,
            last_click: None,
            shortcut_held: false,
            dead_key_hint: None,
            view_set_aside: None,
            view_opened: 0,
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
        app.splash_cold = !cfg!(test) && app.splash && screen::first_start_of_this_version();
        app.offer_claude_hooks();
        if !crate::theme::BUILTIN.contains(&crate::theme::canonical(&app.cfg.theme)) {
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
        connections: &mut Connections,
        links: &mut link::Links,
        out_rx: &mut mpsc::UnboundedReceiver<Outgoing>,
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
            let next_draw =
                tokio::time::Instant::from_std((last_draw + frame).max(hold.unwrap_or(last_draw)));
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
                        self.on_burst(burst);
                    }
                    Some(Ok(ev)) => self.on_event(ev),
                    Some(Err(e)) => return Err(e.into()),
                    None => return Ok(()),
                },
                Some(b) = bg_rx.recv() => {
                    self.on_bg(b);
                    self.dirty = true;
                }
                Some(out) = out_rx.recv() => self.deliver(out, connections),
                event = links.next(), if links.any() => {
                    self.on_link_event(event, connections);
                    self.dirty = true;
                }
                // Take everything a machine has already sent before drawing again. One
                // message per frame can't keep up once a frame takes longer than messages
                // arrive (many agents printing), and the server lets go of a window that
                // falls too far behind.
                burst = connections.recv_burst(SERVER_BURST) => {
                    self.on_server_burst(burst, connections).await;
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
                        self.show_send_progress();
                        self.dirty = true;
                    }
                    let frame = self.spinner_frame();
                    self.settle_alerts();
                    if frame != last_frame && ((self.motion_on() && (self.any_working() || self.any_checking() || self.any_calling())) || matches!(self.mode, Mode::Spinners(_))) {
                        last_frame = frame;
                        self.dirty = true;
                    }
                    if matches!(self.mode, Mode::Prefix { .. }) || screen::splash_animating(self) {
                        self.dirty = true;
                    }
                    if self.notice.as_ref().is_some_and(|(_, at, _)| at.elapsed() > NOTICE_FOR) {
                        self.notice = None;
                        self.dirty = true;
                    }
                    self.expire_unseen_logins();
                    if self.closing.expire() {
                        self.dirty = true;
                    }
                    self.service_tick(connections).await;
                    if self.bar_flash.as_ref().is_some_and(|(_, at)| at.elapsed() >= BAR_FLASH_FOR) {
                        self.bar_flash = None;
                        self.dirty = true;
                    }
                    if self.copy_flash.as_ref().is_some_and(|f| !f.live()) {
                        self.copy_flash = None;
                        self.dirty = true;
                    }
                }
            }
            if self.quit.is_some() {
                return Ok(());
            }
            for machine in self.take_dropped_machines() {
                connections.remove(machine);
            }
            self.report_focus();
            self.sync_cursor_style();
            self.request_diff();
            self.request_files_preview();
            if self.dirty
                && last_draw.elapsed() >= frame
                && self.sync_hold_until().is_none_or(|h| Instant::now() >= h)
            {
                self.dirty = false;
                last_draw = Instant::now();
                // Commit the whole replacement frame together, including pane switches.
                execute!(
                    terminal.backend_mut(),
                    ratatui::crossterm::terminal::BeginSynchronizedUpdate
                )?;
                let drawn = terminal.draw(|f| render::draw(self, f)).map(|_| ());
                let ended = execute!(
                    terminal.backend_mut(),
                    ratatui::crossterm::terminal::EndSynchronizedUpdate
                );
                drawn?;
                ended?;
                self.sync_sizes();
                self.sync_window_title();
                // Something's moving: the next frame too.
                if self.motion.moving() || self.toast_moving() {
                    self.dirty = true;
                }
            }
        }
    }

    async fn on_server_burst(&mut self, burst: Burst, connections: &mut Connections) {
        let Burst {
            machine,
            msgs,
            lost,
            stalled,
        } = burst;
        for msg in msgs {
            if self.quit.is_some() {
                break;
            }
            self.on_machine_server(machine, msg);
        }
        if self.refused.contains_key(&machine) {
            connections.mark_lost(machine);
        }
        if lost && self.quit.is_none() {
            self.on_connection_lost(machine, stalled, connections).await;
        }
    }

    /// Only losing this machine's own daemon can end the window; another machine is just
    /// reported as lost, by its supervisor when it has one.
    async fn on_connection_lost(
        &mut self,
        machine: MachineId,
        stalled: bool,
        connections: &mut Connections,
    ) {
        if machine != MachineId::LOCAL {
            if !self.links.line_ended(machine, stalled) {
                let name = self.machine_name(machine).to_owned();
                self.notify(format!("Lost the connection to {name}"), true);
            }
        } else if !self.reattach(connections).await {
            // The daemon keeps every session; the window stays and keeps asking for it.
            self.service_lost();
        }
    }

    /// The connection to the server ended while the server is still there (it lets go of a
    /// window that falls too far behind): attach again, for a fresh copy of everything.
    /// False when it can't: the server is gone, or this keeps happening.
    async fn reattach(&mut self, connections: &mut Connections) -> bool {
        if !self.daemon_restart_reconnect
            && self
                .reattached
                .is_some_and(|at| at.elapsed() < REATTACH_AT_MOST_EVERY)
        {
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
        let Ok(connection) = connection else {
            return false;
        };
        connections.add(MachineId::LOCAL, connection);
        self.reattached = Some(Instant::now());
        if !restarting {
            self.notify(
                "Lost the server for a moment; connected again".into(),
                false,
            );
        }
        true
    }

    /// Installing must not interrupt agents, including across protocol changes.
    fn start_update(&mut self) {
        if self.updating {
            return;
        }
        self.updating = true;
        self.notify(
            "Installing update… the daemon will restart when no sessions are open.".into(),
            false,
        );
        self.spawn_bg(|| {
            let r = crate::cli::update::install_latest().map(|(v, exe)| {
                (v, crate::cli::update::integrate_agents(&exe))
            });
            Bg::Then(Box::new(move |app: &mut App| match r {
                Ok((v, Ok(()))) => app.update_installed(v),
                Ok((v, Err(e))) => {
                    app.update_installed(v);
                    app.notify(format!("Updated, but connecting agents failed: {e}. Run `{APP_COMMAND} integrate` to retry."), true);
                }
                Err(e) => {
                    app.updating = false;
                    app.update_failed = Some(format!("{e:#}"));
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
        let message = if self.machine.is_remote() {
            format!(
                "Installed {APP_COMMAND} {version}. Update the remote daemon on its host after its sessions are closed."
            )
        } else {
            format!(
                "Installed {APP_COMMAND} {version}. The daemon will restart automatically when no sessions are open."
            )
        };
        self.notify(message, false);
        self.notice_restart = self.restart_needed();
        self.maybe_restart_daemon(false);
    }

    fn maybe_restart_daemon(&mut self, last_session_closed: bool) {
        if self.machine.is_remote()
            || !self.got_state
            || !self.snap.terms.is_empty()
            || !self.snap.conversations.is_empty()
            || self.daemon_restart_requested
            || self.daemon_restart_reconnect
        {
            return;
        }
        let Some(Ok(installed)) = &self.installed_version else {
            return;
        };
        if self.snap.daemon_version.is_empty() || installed == &self.snap.daemon_version {
            return;
        }
        if !self.update_installed_this_run
            && !last_session_closed
            && self.snap.workspaces.is_empty()
        {
            return;
        }

        self.request_daemon_restart(
            "No sessions are open. Restarting the daemon to finish the update.",
        );
    }

    fn notify(&mut self, msg: String, error: bool) {
        self.remember(None, if error { 'x' } else { 'i' }, msg.clone());
        self.notice = Some((msg, Instant::now(), error));
        self.notice_term = None;
        self.notice_restart = false;
        self.notice_hint = false;
        self.notice_card = None;
        self.dirty = true;
    }

    /// A toast with a detail row and, optionally, a preview row under its title.
    fn notify_card(&mut self, title: String, detail: String, preview: Option<String>, error: bool) {
        self.notify(format!("{title}: {detail}"), error);
        self.notice = self.notice.take().map(|(_, at, e)| (title, at, e));
        self.notice_card = Some(NoticeCard { detail, preview });
    }

    /// Esc dismisses the toast; the key still goes on to the pane.
    fn dismiss_notice(&mut self) {
        if self.notice.take().is_some() {
            self.notice_card = None;
            self.dirty = true;
        }
    }

    /// Add to the notification history (Ctrl+Space N).
    pub(super) fn remember(&mut self, term: Option<ClientTerm>, kind: char, text: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // The same thing twice in a row is one entry.
        if self
            .history
            .back()
            .is_some_and(|(_, t, k, x)| *t == term && *k == kind && *x == text)
        {
            return;
        }
        self.history.push_back((now, term, kind, text));
        while self.history.len() > 300 {
            self.history.pop_front();
        }
    }

    fn send(&self, machine: MachineId, msg: ClientMsg) {
        if machine != MachineId::LOCAL && matches!(msg, ClientMsg::Command(_) | ClientMsg::Query(_))
        {
            self.route.asked(machine);
        }
        let _ = self.out.send(Outgoing { machine, msg });
    }

    fn send_input(&self, to: ClientTerm, data: Vec<u8>) {
        self.send(
            to.machine,
            ClientMsg::Input {
                term: to.term,
                data,
            },
        );
    }

    /// Ask (in the background) whether a newer the app is out: when the window opens, then now
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
            let installed = crate::cli::update::installed_version();
            let latest = crate::cli::update::latest_tag();
            let current = installed.as_deref().unwrap_or(env!("APP_VERSION"));
            let newer = latest
                .as_ref()
                .ok()
                .filter(|tag| crate::version::newer_than(tag, current))
                .map(|tag| tag.trim_start_matches('v').to_string());
            // Only asked for when there is something new: one more request, now and then.
            let notes = if newer.is_some() {
                crate::cli::update::changes_since_this()
            } else {
                Vec::new()
            };
            Bg::Then(Box::new(move |app: &mut App| {
                app.installed_version = Some(installed);
                app.update_checked_now(asked, latest.map(|_| newer), notes);
            }))
        });
    }

    fn refresh_installed_version(&self) {
        self.spawn_bg(|| {
            let installed = crate::cli::update::installed_version();
            Bg::Then(Box::new(move |app| {
                app.installed_version = Some(installed);
                app.maybe_restart_daemon(false);
            }))
        });
    }

    /// A local daemon older than the installed the app, which a restart would replace.
    fn daemon_restartable(&self) -> bool {
        !self.machine.is_remote()
            && !self.daemon_restart_requested
            && !self.daemon_restart_reconnect
            && !self.snap.daemon_version.is_empty()
            && self
                .installed_version
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .is_some_and(|v| *v != self.snap.daemon_version)
    }

    /// The toast offers Restart now: it stays long enough to be read and clicked.
    fn toast_restart(&self) -> bool {
        self.notice_restart && self.restart_needed()
    }

    /// This window runs an older the app than the one installed here (a remote server's
    /// version is its own business).
    fn window_stale(&self) -> bool {
        !self.machine.is_remote()
            && self
                .installed_version
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .is_some_and(|v| crate::version::newer_than(v, env!("APP_VERSION")))
    }

    /// There is something Restart would replace: the server, or this window.
    fn restart_needed(&self) -> bool {
        !self.daemon_restart_requested
            && !self.daemon_restart_reconnect
            && !self.relaunch
            && (self.daemon_restartable() || self.window_stale())
    }

    /// Once, offer to connect Claude Code's status hooks when it is installed and they are not:
    /// without them its sessions never report working, needs you or done. Reads files, so the
    /// look is off the UI thread; it asks only if nothing else is open by then.
    fn offer_claude_hooks(&mut self) {
        if cfg!(test)
            || self.machine.is_remote()
            || self.screen.saved.hooks_offered
            || crate::cli::update::integrate_opted_out()
            || !crate::cli::hooks_belong_to_this_build()
        {
            return;
        }
        self.spawn_bg(|| {
            let installed = maitty_agent_integrations::STATUS_AGENTS
                .iter()
                .find(|a| a.id == "claude")
                .is_some_and(crate::cli::integrations::agent_installed);
            let missing = installed && !crate::cli::claude_status_hooks_installed();
            Bg::Then(Box::new(move |app: &mut App| {
                if !missing || !matches!(app.mode, Mode::Normal) {
                    return;
                }
                app.screen.saved.hooks_offered = true;
                app.screen.save();
                app.mode = Mode::Confirm(Box::new(menu::Confirm {
                    title: "Connect Claude Code".into(),
                    sub: String::new(),
                    what: "Show when Claude is working, needs you or is done".into(),
                    detail: String::new(),
                    note: concat!(
                        "Adds ",
                        env!("APP_NAME"),
                        "'s status hooks to ~/.claude/settings.json. Restart Claude to load them. ",
                        "Asked once; Settings > Agents connects it later."
                    )
                    .into(),
                    list: Vec::new(),
                    yes: "Connect".into(),
                    key: '\n',
                    danger: false,
                    act: menu::Act::InstallStatus("claude".into()),
                }));
            }))
        });
    }

    /// Restart onto the installed version: the server (sessions stop and resume) and this
    /// window, in one step. It asks only when a session is working or waiting for you, or
    /// when sessions would be closed for good because restoring is off.
    fn ask_restart(&mut self) {
        if !self.restart_needed() {
            return;
        }
        if !self.daemon_restartable() {
            // The server is already the installed one: only this window is replaced.
            return self.relaunch_window();
        }
        let sessions = self.snap.terms.len();
        let working = self
            .snap
            .terms
            .values()
            .filter(|t| t.status == Status::Working)
            .count();
        let waiting = self
            .snap
            .terms
            .values()
            .filter(|t| t.status == Status::Blocked)
            .count();
        let keeps = self.cfg.restore.enabled;
        if sessions == 0 || (working + waiting == 0 && keeps) {
            return self.restart_daemon_now();
        }
        self.mode = Mode::Confirm(Box::new(menu::Confirm {
            title: concat!("Restart ", env!("APP_COMMAND")).into(),
            sub: String::new(),
            what: restart_summary(sessions, working, waiting),
            detail: String::new(),
            note: if keeps {
                "They come back where they left off, with their conversation.".into()
            } else {
                "Restoring sessions is off (restore.enabled), so they close for good.".into()
            },
            list: Vec::new(),
            yes: "Restart".into(),
            key: '\n',
            danger: working + waiting > 0 || !keeps,
            act: menu::Act::RestartDaemon,
        }));
    }

    /// The palette's Restart: say so when there is nothing to replace.
    /// Restart the agents that never loaded their hooks, so they report their own status. One in
    /// the middle of a turn is left alone: the command line's `--all` restarts it too.
    fn reload_agents_from_menu(&mut self) {
        let plan = crate::cli::reload::plan(&self.snap, false);
        for t in &plan.restart {
            self.cmd(Command::ReloadAgent { term: t.id });
        }
        let message = match (plan.restart.len(), plan.skipped.len()) {
            (0, 0) => "No running agent needs a restart".to_string(),
            (0, busy) => format!("{busy} agent(s) need one, but are working or waiting for you"),
            (n, 0) => format!("Restarting {n} agent(s) so they load their hooks"),
            (n, busy) => format!("Restarting {n} agent(s); {busy} working or waiting were left alone"),
        };
        self.notify(message, false);
    }

    fn restart_from_menu(&mut self) {
        if self.restart_needed() {
            self.ask_restart();
        } else {
            self.notify(
                concat!(
                    env!("APP_NAME"),
                    " is already running the installed version"
                )
                .into(),
                false,
            );
        }
    }

    /// Close this window and open the installed the app in its place; the server keeps every
    /// session meanwhile.
    fn relaunch_window(&mut self) {
        self.relaunch = true;
        self.quit = Some(concat!("restarting ", env!("APP_COMMAND")).into());
    }

    fn restart_daemon_now(&mut self) {
        if self.restart_needed() && self.daemon_restartable() {
            self.request_daemon_restart("Restarting the daemon…");
        }
    }

    fn request_daemon_restart(&mut self, message: &str) {
        self.daemon_restart_requested = true;
        self.notify(message.into(), false);
        self.cmd(Command::KillServer { forget: false });
    }

    fn restart_pending(&self) -> bool {
        self.installed_version
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .is_some_and(|v| {
                v != env!("APP_VERSION")
                    || (!self.snap.daemon_version.is_empty() && *v != self.snap.daemon_version)
            })
    }

    /// What a check for a newer the app found: the newer version, none, or why it couldn't
    /// look. One you asked for always says.
    fn update_checked_now(
        &mut self,
        asked: bool,
        found: Result<Option<String>, String>,
        notes: Vec<String>,
    ) {
        if asked {
            self.update_checking = false;
        }
        match found {
            Ok(Some(v)) if self.update_available.as_ref() != Some(&v) => {
                self.notify(
                    format!("{APP_COMMAND} {v} is out: Update now is in Settings"),
                    false,
                );
                self.update_available = Some(v);
                self.update_notes = notes;
            }
            Ok(Some(v)) if asked => self.notify(
                format!("{APP_COMMAND} {v} is out: Update now is in Settings"),
                false,
            ),
            Ok(None) => {
                self.update_available = None;
                self.update_notes.clear();
                if asked {
                    let version = self
                        .installed_version
                        .as_ref()
                        .and_then(|r| r.as_ref().ok())
                        .map(String::as_str)
                        .unwrap_or(env!("APP_VERSION"));
                    self.notify(
                        format!(
                            "Installed {APP_COMMAND} {version} is the latest{}",
                            if self.restart_pending() {
                                "; restart pending"
                            } else {
                                ""
                            }
                        ),
                        false,
                    );
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
        let new = self
            .update_available
            .clone()
            .unwrap_or_else(|| "the latest".into());
        self.mode = Mode::Confirm(Box::new(menu::Confirm {
            title: concat!("Update ", env!("APP_COMMAND")).into(),
            sub: String::new(),
            what: format!("{} → {new}", env!("APP_VERSION")),
            detail: String::new(),
            note: if self.machine.is_remote() {
                "Open sessions keep running. Update the remote daemon on its host after they are closed.".into()
            } else {
                "Open sessions keep running. Restart now (offered afterwards) switches to the new version in one step; agents resume.".into()
            },
            list: self.update_notes.clone(),
            yes: "Update now".into(),
            key: 'u',
            danger: false,
            act: menu::Act::Update,
        }));
    }

    fn cmd(&self, c: Command) {
        self.cmd_on(MachineId::LOCAL, c);
    }

    /// A command for the daemon that owns the pane it names.
    fn cmd_on(&self, machine: MachineId, c: Command) {
        if routing::moves_focus(&c) {
            self.route.set_focus(machine);
        }
        self.send(machine, ClientMsg::Command(c));
    }

    /// Slides and glides: on in Settings, and not over SSH (every frame goes down the link).
    /// Tests look at single frames, so there things are where they end.
    fn motion_on(&self) -> bool {
        self.cfg.ui.motion && !self.machine.is_remote() && !cfg!(test)
    }

    /// The toast is sliding in, settling from its flash, or fading out.
    fn toast_moving(&self) -> bool {
        let Some((_, at, err)) = &self.notice else {
            return false;
        };
        let hold = screen::toast_hold(*err, self.notice_term.is_some(), self.toast_restart());
        let e = at.elapsed();
        self.motion_on()
            && (e < motion::TOAST_IN.max(motion::TOAST_FLASH)
                || (e + motion::TOAST_FADE >= hold && e < hold))
    }

    fn spinner_frame(&self) -> u64 {
        (self.started.elapsed().as_millis() / 90) as u64
    }

    /// A check's ring turns while it runs.
    fn any_checking(&self) -> bool {
        self.all_terms().iter().any(|(_, term)| {
            term.check()
                .is_some_and(|c| c.state == crate::protocol::CheckState::Checking)
        })
    }

    /// A session that needs you or finished unseen: its name pulses or shimmers.
    fn any_calling(&self) -> bool {
        self.all_terms()
            .iter()
            .any(|(_, term)| matches!(term.status, Status::Blocked | Status::Done))
    }

    fn any_working(&self) -> bool {
        self.all_terms()
            .iter()
            .any(|(_, term)| term.status == Status::Working)
    }

    /// The topmost clickable region under `at` on the top layer: nothing under an overlay.
    fn hover_hit(&self, at: Position) -> Option<&(Rect, Hit)> {
        self.hits
            .get(self.hits_under_overlay..)
            .unwrap_or_default()
            .iter()
            .rev()
            .find(|(r, _)| r.contains(at))
    }

    // ---- model helpers -------------------------------------------------------------

    /// Every machine's daemon state, this machine first.
    fn machines(&self) -> Vec<MachineSnap<'_>> {
        let here = MachineSnap {
            id: MachineId::LOCAL,
            name: "",
            snap: &self.snap,
        };
        let others = self.remotes.iter().map(|(id, r)| MachineSnap {
            id: *id,
            name: &r.name,
            snap: &r.snap,
        });
        std::iter::once(here).chain(others).collect()
    }

    /// Every pane on every machine.
    fn all_terms(&self) -> Vec<(ClientTerm, &TermInfo)> {
        self.machines()
            .into_iter()
            .flat_map(|m| {
                m.snap
                    .terms
                    .iter()
                    .map(move |(id, t)| (ClientTerm::on(m.id, *id), t))
            })
            .collect()
    }

    fn snap_of(&self, machine: MachineId) -> Option<&Snapshot> {
        if machine == MachineId::LOCAL {
            return Some(&self.snap);
        }
        self.remotes.get(&machine).map(|r| &r.snap)
    }

    fn machine_name(&self, machine: MachineId) -> &str {
        self.remotes.get(&machine).map_or("", |r| &r.name)
    }

    fn term_info(&self, term: ClientTerm) -> Option<&TermInfo> {
        self.snap_of(term.machine)?.terms.get(&term.term)
    }

    /// The workspace on screen: the one on the machine that holds the focus.
    fn active_ws(&self) -> Option<&WorkspaceInfo> {
        self.snap_of(self.focus_machine())?.active()
    }

    fn active_tab(&self) -> Option<&TabInfo> {
        self.active_ws().and_then(|w| w.tab())
    }

    fn focused(&self) -> Option<ClientTerm> {
        self.active_tab()
            .map(|t| ClientTerm::on(self.focus_machine(), t.focus))
    }

    /// A floating pane (`<app> popup`) open now: it has the keys until it closes.
    /// Only on the machine that holds the focus: another machine's popup must not take the
    /// keys meant for the pane you are on.
    fn popup(&self) -> Option<ClientTerm> {
        let machine = self.focus_machine();
        self.snap_of(machine)?
            .terms
            .values()
            .filter(|t| t.popup)
            .map(|t| t.id)
            .max()
            .map(|id| ClientTerm::on(machine, id))
    }

    /// Where typing goes: an open popup, else the pane you're on.
    fn typing_to(&self) -> Option<ClientTerm> {
        self.popup().or_else(|| {
            // An unbound sidebar selection must not fall through to an unrelated shell.
            if self.screen.cursor.is_some() && self.selected_term().is_none() {
                return None;
            }
            self.focused()
        })
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
        // A pane that was out of sight (another tab, a hidden session) and is shown again has its
        // size told to the daemon afresh: another window on the same daemon may have set it to
        // its own in the meantime, and the program then draws for a width this screen is not.
        let now: std::collections::HashSet<ClientTerm> = panes.iter().map(|(t, _)| *t).collect();
        for term in now.difference(&self.shown_panes) {
            self.sizes.remove(term);
        }
        self.shown_panes = now;
        for (term, r) in panes {
            let want = (r.width.max(2), r.height.max(2));
            // An agent's screen is drawn by cursor moves and erases that are only right at the
            // width they were made for: replaying them into a screen of another width scrambles
            // it. The program redraws itself when it hears of the new size, so such a pane only
            // changes size.
            let replays = !self.draws_itself(term);
            if !dragging && self.rewrap.remove(&term) && replays {
                self.rewrap_pane(term, want);
            }
            if self.sizes.get(&term) == Some(&want) {
                continue;
            }
            let old = self.sizes.insert(term, want);
            if replays
                && old.is_some_and(|o| o.0 != want.0)
                && self.raw.get(&term).is_some_and(|r| !r.is_empty())
            {
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
            self.send(
                term.machine,
                ClientMsg::Resize {
                    term: term.term,
                    cols: want.0,
                    rows: want.1,
                },
            );
        }
    }

    /// Whether the program in `term` is an agent, which paints its own screen with cursor moves.
    fn draws_itself(&self, term: ClientTerm) -> bool {
        self.term_info(term).is_some_and(|i| i.agent.is_some())
    }

    /// A fresh screen for `term` at `(cols, rows)`, from the output kept for it.
    fn rewrap_pane(&mut self, term: ClientTerm, (cols, rows): (u16, u16)) {
        let mut p = self.new_parser(rows, cols);
        if let Some(raw) = self.raw.get_mut(&term) {
            p.process(raw.make_contiguous());
        }
        self.parsers.insert(term, p);
    }

    // ---- server messages -----------------------------------------------------------

    /// What the sidebar calls a session: what you renamed it to, else its worktree or folder.
    /// Notes about a session use this, so they say the name you'd look for.
    pub(super) fn session_name(&self, term: ClientTerm) -> String {
        let model = self.screen_model();
        if let Some((_, _, s)) = screen::find(&model, term) {
            return s.name.clone();
        }
        self.term_info(term)
            .map(|t| {
                if t.label.trim().is_empty() {
                    t.display_name()
                } else {
                    t.label.trim().to_string()
                }
            })
            .unwrap_or_default()
    }

    fn describe_term(&self, term: ClientTerm) -> String {
        let name = self.session_name(term);
        if let Some(t) = self.term_info(term) {
            let place = crate::alert::place_label(
                t.root.as_deref(),
                t.top.as_deref().unwrap_or(&t.cwd),
                t.branch.as_deref(),
            );
            if !place.is_empty() {
                return format!("{name} ({place})");
            }
        }
        match self
            .snap_of(term.machine)
            .and_then(|snap| snap.locate(term.term))
        {
            Some((w, t)) => {
                let idx = w.tabs.iter().position(|x| x.id == t.id).unwrap_or(0) + 1;
                format!("{name} ({} › {idx})", w.name)
            }
            None => name,
        }
    }

    // ---- input ---------------------------------------------------------------------

    fn yank(&mut self, text: String) {
        let src = match &self.mode {
            Mode::Copy(c) => Some(c.term),
            _ => self.copy_term,
        };
        if let Mode::Copy(c) = &self.mode {
            let spans = c.selected_spans();
            if !spans.is_empty() {
                self.copy_flash = Some(copy::Flash {
                    term: c.term,
                    spans,
                    at: Instant::now(),
                });
            }
        }
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
            let named: Vec<(String, String)> = parts
                .into_iter()
                .map(|(t, s)| {
                    (
                        self.term_info(t)
                            .map(|i| i.display_name())
                            .unwrap_or_else(|| format!("pane {}", t.term)),
                        s,
                    )
                })
                .collect();
            let _ = copy::to_clipboard(&copy::join_pieces(&named));
            self.notify_card(
                "Copied to clipboard".into(),
                format!("{n} panes, each under its name"),
                None,
                false,
            );
            return;
        };
        if text.is_empty() {
            return;
        }
        let native = copy::to_clipboard(&text);
        let from = src.map(|t| self.session_name(t)).filter(|n| !n.is_empty());
        self.announce_copy(&text, native, from);
    }

    /// The one toast for a copy, whether you selected the text here or a program asked for it:
    /// what was copied, how much, from where, and a line of it.
    pub(super) fn announce_copy(&mut self, text: &str, native: bool, from: Option<String>) {
        let chars = text.chars().count();
        let lines = text.lines().count().max(1);
        let mut detail = format!(
            "{chars} character{} · {lines} line{}",
            if chars == 1 { "" } else { "s" },
            if lines == 1 { "" } else { "s" }
        );
        if let Some(n) = from {
            detail.push_str(&format!(" from {n}"));
        }
        if !native {
            detail.push_str(" (via terminal)");
        }
        let first = text
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("");
        let preview = (!first.is_empty())
            .then(|| format!("\"{}\"", render::truncate(first, COPY_PREVIEW_CHARS)));
        self.notify_card("Copied to clipboard".into(), detail, preview, false);
    }

    fn enter_copy(&mut self, term: ClientTerm) -> bool {
        let Some(p) = self.parsers.get_mut(&term) else {
            return false;
        };
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
    fn pane_info(&mut self, term: ClientTerm) -> Option<String> {
        let agent = self.term_info(term).is_some_and(|t| t.agent.is_some());
        let p = self.parsers.get_mut(&term)?;
        let at = p.screen().scrollback();
        p.screen_mut().set_scrollback(usize::MAX);
        let history = p.screen().scrollback();
        p.screen_mut().set_scrollback(at);
        let s = p.screen();
        let wheel = if s.alternate_screen()
            || (history == 0
                && (agent || s.mouse_protocol_mode() != vt100::MouseProtocolMode::None))
        {
            "the program"
        } else {
            env!("APP_COMMAND")
        };
        let full = if s.alternate_screen() {
            "full-screen (it scrolls itself)"
        } else {
            "normal screen"
        };
        let raw = self.raw.get(&term).map(|r| r.len() / 1024).unwrap_or(0);
        Some(format!("{full} · the wheel goes to {wheel} · {history} lines of history, {at} up · {raw} KB kept · {:?}", self.mode).chars().take(200).collect())
    }

    fn keep_raw(&mut self, term: ClientTerm, data: &[u8], replace: bool) {
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
        let want = cursor_code(
            self.focused()
                .and_then(|t| self.cursor_style.get(&t).copied()),
            &self.cfg.ui,
        );
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

    /// The clipboard's image, saved as a PNG; its path is pasted into the typing target
    /// (Claude Code, Codex and friends attach an image given by path).
    pub(super) fn paste_image(&mut self) {
        let Some(term) = self.typing_to() else { return };
        if remote_image::is_remote(term) {
            self.paste_image_to_machine(term);
            return;
        }
        // Reading the clipboard and encoding the PNG can take a while for a big image.
        self.spawn_bg(move || {
            let made = clipboard_image_to_file();
            Bg::Then(Box::new(move |app: &mut App| match made {
                Ok((path, w, h)) => {
                    let text = files::quote_path(&path);
                    let bracketed = app
                        .parsers
                        .get(&term)
                        .is_some_and(|p| p.screen().bracketed_paste());
                    let data = if bracketed {
                        format!("\x1b[200~{text} \x1b[201~")
                    } else {
                        format!("{text} ")
                    };
                    app.send_input(term, data.into_bytes());
                    app.notify(
                        format!("pasted the clipboard image ({w}×{h}) as a file"),
                        false,
                    );
                }
                Err(e) => app.notify(e, true),
            }))
        });
    }

    /// The clipboard's image, sent to the machine the pane is on and kept there as a file; its
    /// path there is typed into the pane. The upload is ssh, run off the UI thread.
    fn paste_image_to_machine(&mut self, term: ClientTerm) {
        let Some((machine, name)) = self.machine_for(term) else {
            return;
        };
        self.notify(format!("sending the clipboard image to {name}…"), false);
        self.spawn_bg(move || {
            let sent = remote_image::send(&machine, &name);
            Bg::Then(Box::new(move |app: &mut App| match sent {
                Ok(sent) => {
                    app.type_path_into(term, &sent.path);
                    app.notify(
                        format!(
                            "pasted the clipboard image ({}×{}) to {name}",
                            sent.width, sent.height
                        ),
                        false,
                    );
                }
                Err(e) => app.notify(e, true),
            }))
        });
    }

    /// When to draw a pane that's mid synchronized update: its start + 100 ms, if that's
    /// still ahead.
    fn sync_hold_until(&self) -> Option<Instant> {
        // The previous frame's panes must not hold up navigation to a different session.
        if !matches!(self.mode, Mode::Normal)
            || self.view.is_some()
            || self
                .focused()
                .is_some_and(|focused| !self.panes.iter().any(|(id, _)| *id == focused))
        {
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

    /// Tell the daemon the pane in front has been looked at, so its "done" clears. The daemon does
    /// not guess this from which pane is active: a window in the background is not looking.
    pub(super) fn mark_looked_pane_seen(&mut self) {
        if !self.window_focused {
            return;
        }
        let Some(front) = self.focused() else {
            return;
        };
        let done = self
            .snap_of(front.machine)
            .and_then(|snap| snap.terms.get(&front.term))
            .is_some_and(|t| t.status == Status::Done);
        if done {
            self.cmd_on(front.machine, Command::MarkSeen { term: front.term });
        }
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
            self.send_input(t, b"\x1b[O".to_vec());
        }
        if let Some((t, true)) = now
            && self.focus_report.contains(&t)
        {
            self.send_input(t, b"\x1b[I".to_vec());
        }
        self.focus_sent = now;
    }

    /// (offset from the bottom, lines of history) for a pane.
    pub(super) fn history(&mut self, term: ClientTerm) -> (usize, usize) {
        let Some(p) = self.parsers.get_mut(&term) else {
            return (0, 0);
        };
        let cur = p.screen().scrollback();
        p.screen_mut().set_scrollback(usize::MAX);
        let total = p.screen().scrollback();
        p.screen_mut().set_scrollback(cur);
        (cur, total)
    }

    /// Back to the newest output: the app's own history to its end, and an agent that scrolls
    /// itself paged down as far as the wheel paged it up.
    pub(super) fn back_to_bottom(&mut self, term: ClientTerm) {
        self.scroll_to(term, 0);
        if let Some(n) = self.paged_up.remove(&term) {
            let down: Vec<u8> = b"\x1b[6~".repeat(n as usize);
            self.send_input(term, down);
        }
    }

    pub(super) fn scroll_to(&mut self, term: ClientTerm, offset: usize) {
        let cur = self.history(term).0 as i32;
        self.scroll_by(term, offset as i32 - cur);
    }

    // ---- actions -------------------------------------------------------------------

    /// Where the user is: the focused pane's folder, else the workspace's. On another machine
    /// it is that machine's path, taken as it is: this computer's disk has no say in it.
    fn here_dir(&self) -> PathBuf {
        if let Some(there) = self.focused().filter(|t| !self.is_local_pane(*t)) {
            return self
                .term_info(there)
                .map(|t| t.cwd.clone())
                .or_else(|| self.active_ws().map(|w| w.cwd.clone()))
                .unwrap_or_default();
        }
        self.focused()
            .and_then(|id| self.local_cwd(id))
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
mod reference_fixture;
#[cfg(test)]
mod site_frames;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod conversation_tests;

#[cfg(test)]
mod cli_machine_tests;
#[cfg(test)]
mod doctor_machine_tests;
#[cfg(test)]
mod machine_update_tests;
#[cfg(test)]
mod mcp_machine_tests;
#[cfg(test)]
mod remote_status_tests;
#[cfg(test)]
mod routing_tests;
