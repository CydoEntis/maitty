//! The status bar along the bottom: the mode chip (` PANE ` while keys go to the pane, ` KEYS `
//! in key mode, or the name of whatever else has the keys), the session you are on, and at the
//! right the keys worth knowing, read from the keymap so a rebinding shows.

use super::*;
use crate::client::BAR_FLASH_FOR;
use crate::keys::{Action, KeySpec};

/// What key mode lists at the right, left to right; a narrow bar drops them from the right.
/// `esc back` always goes last.
fn key_mode_hints() -> [(Action, &'static str); 5] {
    [
        (Action::PickSession, "new"),
        (Action::SplitRight, "split"),
        (Action::ClosePane, "close"),
        (Action::FocusNext, "pane"),
        (Action::Help, "help"),
    ]
}

/// Cells between two hints.
const GAP: u16 = 2;

/// One thing on the bar, the cells before it, and what a click on it does.
#[derive(Clone)]
struct Piece {
    segs: Vec<Seg>,
    gap: u16,
    hit: Option<ScreenHit>,
}

/// How much of Update or Restart shows: all its words, the version, or the arrow.
#[derive(Clone, Copy, PartialEq)]
enum Release {
    Words,
    Version,
    Arrow,
}

/// Update when a newer release is out, else Restart once a newer one is installed but not
/// running, as much of it as `shown`. Only while the sidebar's foot is not showing its chip.
fn release_pill(app: &App, shown: Release, t: &Theme) -> Option<Piece> {
    let style = Style::default()
        .fg(t.accent)
        .bg(t.btn)
        .add_modifier(Modifier::BOLD);
    let (words, h) = if let Some(version) = app.update_available.as_ref() {
        let words = match shown {
            Release::Words => format!(" ↑ Update {version} "),
            Release::Version => format!(" ↑ {version} "),
            Release::Arrow => " ↑ ".to_string(),
        };
        (words, ScreenHit::Update)
    } else if app.restart_needed() {
        // Installed (here or from outside the app), not running yet: one click finishes it.
        let words = if shown == Release::Words { " ↻ Restart " } else { " ↻ " };
        (words.to_string(), ScreenHit::Restart)
    } else {
        return None;
    };
    Some(Piece {
        segs: vec![seg(words, style)],
        gap: GAP,
        hit: Some(h),
    })
}

/// A key bound after the leader to `a`, if any.
fn bound(app: &App, a: &Action) -> Option<KeySpec> {
    app.keymap
        .prefixed_order
        .iter()
        .find(|k| app.keymap.prefixed.get(k) == Some(a))
        .copied()
}

/// A key as the bar spells it: lower case, Ctrl as `^` (`^space`, `tab`, `esc`, `S`).
pub(in crate::client) fn short_key(k: &KeySpec) -> String {
    let long = crate::client::design::long_key(k).replace("Ctrl+", "^");
    if long.chars().count() > 1 {
        long.to_lowercase()
    } else {
        long
    }
}

/// A hint: its key bold in the accent, what it does muted.
fn hint(key: String, words: &str, hit: Option<ScreenHit>, t: &Theme) -> Piece {
    let base = Style::default().bg(t.bg);
    Piece {
        segs: vec![
            seg(key, base.fg(t.accent).add_modifier(Modifier::BOLD)),
            seg(format!(" {words}"), base.fg(t.muted)),
        ],
        gap: GAP,
        hit,
    }
}

/// The mode chip: its word on its own ground, dark ink, bold.
fn mode_chip(label: &str, ground: Color, ink: Color, hit: Option<ScreenHit>) -> Piece {
    Piece {
        segs: vec![seg(
            format!(" {label} "),
            Style::default().fg(ink).bg(ground).add_modifier(Modifier::BOLD),
        )],
        gap: 0,
        hit,
    }
}

/// Typing: every key here is a leader key, so each is shown with the leader in front (`^space j/k sessions`): the
/// bare `j/k` would be typed into the pane. The leader alone starts key mode (a click starts it), and `? help` lists
/// the keys.
fn pane_hints(app: &App, t: &Theme) -> Vec<Piece> {
    let lead = short_key(&app.keymap.prefix);
    let mut hints = Vec::new();
    hints.push(hint(lead.clone(), "keys", Some(ScreenHit::ShortcutMode), t));
    if let (Some(next), Some(prev)) = (bound(app, &Action::NextSession), bound(app, &Action::PrevSession)) {
        hints.push(hint(format!("{lead} {}/{}", short_key(&next), short_key(&prev)), "sessions", None, t));
    }
    if let Some(help) = bound(app, &Action::Help) {
        hints.push(hint(format!("{lead} {}", short_key(&help)), "help", None, t));
    }
    hints
}

/// Key mode: what its keys do, each a click that presses it, and `esc back` last.
fn key_hints(app: &mut App, t: &Theme) -> Vec<Piece> {
    let mut hints = Vec::new();
    for (a, words) in key_mode_hints() {
        if let Some(key) = bound(app, &a) {
            app.screen.bar_keys.push(key);
            let h = ScreenHit::BarKey(app.screen.bar_keys.len() - 1);
            hints.push(hint(short_key(&key), words, Some(h), t));
        }
    }
    let esc = KeySpec {
        code: crossterm::event::KeyCode::Esc,
        mods: crossterm::event::KeyModifiers::NONE,
    };
    app.screen.bar_keys.push(esc);
    let h = ScreenHit::BarKey(app.screen.bar_keys.len() - 1);
    hints.push(hint("esc".into(), "back", Some(h), t));
    hints
}

/// A line that just happened. The session's name and folder are not repeated here: the pane's
/// own title already shows them.
fn where_piece(app: &App, t: &Theme) -> Option<Piece> {
    let (text, at) = app.bar_flash.as_ref()?;
    (at.elapsed() < BAR_FLASH_FOR).then(|| Piece {
        segs: vec![seg(
            text.clone(),
            Style::default().bg(t.bg).fg(t.done).add_modifier(Modifier::BOLD),
        )],
        gap: 1,
        hit: None,
    })
}

/// A key and what it does; an empty key is words alone ("type").
type Hint = (&'static str, &'static str);

/// Whatever has the keys instead of the panes: its name, its main keys, and the way out, which
/// goes last. Every key here is one its own handler reads; none comes from the keymap, since
/// these screens don't take rebindings.
struct Holder {
    label: &'static str,
    hints: &'static [Hint],
    out: Hint,
}

const fn holder(label: &'static str, hints: &'static [Hint], out: Hint) -> Option<Holder> {
    Some(Holder { label, hints, out })
}

/// Who has the keys, if not the panes: a popup or modal, the sidebar, a view in the side
/// panel, or the key map's search. None while typing, and while shortcut mode is lit.
fn keys_holder(app: &App) -> Option<Holder> {
    use crate::client::branch::Step;
    match &app.mode {
        Mode::Normal => view_holder(app),
        Mode::Prefix { .. } => None,
        // A second step's letters pick its items, not shortcuts.
        Mode::KeyMap(km) if km.step.is_some() => holder(
            "KEY MAP",
            &[
                ("", "a letter picks"),
                ("↑↓", "choose"),
                ("Enter", "run"),
                ("Backspace", "back"),
            ],
            ("Esc", "close"),
        ),
        Mode::KeyMap(km) => match km.view {
            KeyView::Search => holder(
                "PALETTE",
                &[("", "type to search"), ("↑↓", "move"), ("Enter", "run")],
                ("Esc", "close"),
            ),
            KeyView::All => holder(
                "KEYS HELP",
                &[("", "type to search")],
                ("Esc", "close"),
            ),
            KeyView::Map => None,
        },
        Mode::Side => holder(
            "SIDEBAR",
            &[("↑↓", "move"), ("Enter", "open"), ("n", "new")],
            ("Esc", "back"),
        ),
        Mode::Secret { .. } => holder(
            "SECRET",
            &[("", "type masked"), ("Enter", "give it")],
            ("Esc", "decline"),
        ),
        // Waiting for a key: the next one, arrows too, becomes the binding.
        Mode::ScreenSettings(v) if v.capturing => {
            holder("SETTINGS", &[("", "press the new key")], ("Esc", "cancel"))
        }
        Mode::ScreenSettings(v) if v.editing.is_some() => holder(
            "SETTINGS",
            &[("", "type"), ("Enter", "save")],
            ("Esc", "cancel"),
        ),
        Mode::ScreenSettings(v)
            if crate::client::modal::Cat::ALL.get(v.cat)
                == Some(&crate::client::modal::Cat::Check) =>
        {
            check_page_holder(app)
        }
        Mode::ScreenSettings(_) => holder(
            "SETTINGS",
            &[("↑↓", "move"), ("Tab", "section"), ("Enter", "change")],
            ("Esc", "close"),
        ),
        // A merge or throw-away asked in a finished row: Enter does it, Esc only drops the question.
        Mode::GoTo { .. } => match app.screen.inbox_confirm {
            Some((_, true, false)) => holder("INBOX", &[("Enter", "merge")], ("Esc", "don't")),
            Some((_, false, false)) => {
                holder("INBOX", &[("Enter", "throw away")], ("Esc", "don't"))
            }
            _ if matches!(super::inbox::inbox_selected(app), Some(GoRow::Failed(_))) => holder(
                "INBOX",
                &[("↑↓", "select"), ("Enter", "open"), ("m", "send to agent")],
                ("Esc", "close"),
            ),
            _ if matches!(super::inbox::inbox_selected(app), Some(GoRow::Ask(_))) => holder(
                "INBOX",
                &[("↑↓", "select"), ("y", "yes"), ("a", "session"), ("n", "no"), ("Enter", "open")],
                ("Esc", "close"),
            ),
            _ => holder(
                "INBOX",
                &[("↑↓", "select"), ("Enter", "open"), ("c", "clear finished")],
                ("Esc", "close"),
            ),
        },
        Mode::Compose(_) => holder(
            "MESSAGE",
            &[("Enter", "send"), ("Shift+Enter", "new line")],
            ("Esc", "keep draft"),
        ),
        Mode::Confirm(c) if matches!(c.act, crate::client::menu::Act::OverrideCheck(..)) => holder(
            "OVERRIDE",
            &[("", "type a reason"), ("Enter", "override")],
            ("Esc", "cancel"),
        ),
        Mode::Confirm(_) => holder("CONFIRM", &[("y", "yes")], ("n", "no")),
        Mode::Prompt { kind, .. } if kind.is_confirm() => {
            holder("CONFIRM", &[("y", "yes")], ("n", "no"))
        }
        Mode::Prompt {
            kind: crate::client::PromptKind::AddReference(..),
            ..
        } => holder(
            "ADD REFERENCE",
            &[("", "type or paste a path"), ("Enter", "add")],
            ("Esc", "cancel"),
        ),
        Mode::Prompt {
            kind: crate::client::PromptKind::SendFile(_),
            ..
        } => holder(
            "SEND FILE",
            &[("", "type or drop a path"), ("Enter", "send")],
            ("Esc", "cancel"),
        ),
        Mode::Prompt { .. } | Mode::RenameTab(_) => holder(
            "NAME",
            &[("", "type"), ("Enter", "save")],
            ("Esc", "cancel"),
        ),
        Mode::Helpers { .. } => holder(
            "HELPERS",
            &[("1", "allow"), ("2", "always"), ("3", "no")],
            ("Esc", "back"),
        ),
        Mode::Approve(_) => holder(
            "RUN CHECK?",
            &[("1", "run once"), ("2", "always"), ("3", "never")],
            ("Esc", "not now"),
        ),
        Mode::ProjectCheck(d) if d.editing => holder(
            "CHECK SETTINGS",
            &[
                ("", "type a command"),
                ("Enter", "done"),
                ("Ctrl+S", "save"),
            ],
            ("Esc", "cancel"),
        ),
        Mode::ProjectCheck(d) if !super::check_settings::has_points(app, &d.project) => holder(
            "CHECK SETTINGS",
            &[
                ("", "type a command"),
                ("Enter", "edit"),
                ("Ctrl+S", "save"),
            ],
            ("Esc", "cancel"),
        ),
        Mode::ProjectCheck(_) => holder(
            "CHECK SETTINGS",
            &[("Ctrl+S", "save"), ("↑↓", "move"), ("r", "run now")],
            ("Esc", "cancel"),
        ),
        Mode::Finder(fd) if fd.for_setting.is_some() => holder(
            "OPEN FOLDER",
            &[("", "type a path"), ("Tab", "complete"), ("Enter", "open")],
            ("Esc", "back"),
        ),
        Mode::Finder(_) => holder(
            "OPEN FOLDER",
            &[("", "type a path"), ("Tab", "complete"), ("Enter", "open")],
            ("Esc", "close"),
        ),
        Mode::NewSession(_) => holder(
            "NEW SESSION",
            &[
                ("", "type a project or ~/path"),
                ("Tab", "complete"),
                ("Ctrl+O", "browse"),
                ("↑↓", "move"),
                ("Enter", "pick"),
            ],
            ("Esc", "close"),
        ),
        Mode::ScreenPane(_) => holder(
            "NEW PANE",
            &[("↑↓", "rows"), ("←→", "choose"), ("Enter", "start")],
            ("Esc", "close"),
        ),
        Mode::Themes { .. } => holder(
            "THEMES",
            &[("", "type to filter"), ("↑↓", "move"), ("Enter", "apply")],
            ("Esc", "back"),
        ),
        Mode::Spinners(_) => holder(
            "SPINNERS",
            &[("↑↓", "preview"), ("Enter", "apply")],
            ("Esc", "back"),
        ),
        Mode::ScreenMenu(_) => holder(
            "MENU",
            &[("↑↓", "move"), ("Enter", "pick")],
            ("Esc", "close"),
        ),
        Mode::Updates => holder(
            "UPDATES",
            &[("d", "download"), ("s", "skip"), ("r", "restart")],
            ("Esc", "close"),
        ),
        Mode::Actions { .. } => holder(
            "MENU",
            &[("↑↓", "move"), ("Enter", "run")],
            ("Esc", "close"),
        ),
        Mode::Find(_) => holder(
            "FIND",
            &[
                ("", "type to search"),
                ("Enter", "into the prompt"),
                ("Tab", "files / search"),
            ],
            ("Esc", "close"),
        ),
        Mode::Branch(v) => match v.step {
            Step::Pick => holder(
                "SWITCH BRANCH",
                &[
                    ("", "type to filter"),
                    ("↑↓", "choose"),
                    ("Enter", "switch"),
                ],
                ("Esc", "close"),
            ),
            Step::Changes(_) => holder("SWITCH BRANCH", &[("1-4", "choose")], ("Esc", "back")),
            Step::Message(_) => holder(
                "SWITCH BRANCH",
                &[("", "type a message"), ("Enter", "commit and switch")],
                ("Esc", "back"),
            ),
            Step::ConfirmDiscard => holder(
                "SWITCH BRANCH",
                &[("y", "discard")],
                ("", "any other key back"),
            ),
        },
        Mode::Usage => holder("USAGE", &[], ("Esc", "close")),
        Mode::History { .. } => holder(
            "HISTORY",
            &[("↑↓", "move"), ("Enter", "go to it"), ("c", "clear")],
            ("Esc", "close"),
        ),
        Mode::Machines(_) => holder(
            "ADD MACHINE",
            &[("Tab", "complete"), ("Enter", "connect")],
            ("Esc", "close"),
        ),
        Mode::Tidy(_) => holder(
            "TIDY",
            &[
                ("Space", "tick"),
                ("a", "all the safe ones"),
                ("Enter", "remove"),
            ],
            ("Esc", "close"),
        ),
        Mode::Worktrees { .. } => holder(
            "WORKTREES",
            &[("", "type a branch"), ("↑↓", "move"), ("Enter", "open")],
            ("Esc", "close"),
        ),
        Mode::Copy(c) if c.input.is_some() => holder(
            "COPY",
            &[("", "type to search"), ("Enter", "find")],
            ("Esc", "cancel"),
        ),
        // Esc drops a selection first; only the next one leaves.
        Mode::Copy(c) if c.anchor.is_some() => holder(
            "COPY",
            &[("y", "copy"), ("J", "copy as one line")],
            ("Esc", "clear"),
        ),
        Mode::Copy(_) => holder(
            "COPY",
            &[("v", "select"), ("y", "copy"), ("/", "search")],
            ("Esc", "leave"),
        ),
    }
}

/// Settings' Check page reads its own keys first (`on_check_page_key`): Enter edits only the
/// command row, ←→ only step the timeout or the checkpoint, and Esc or Tab drop the draft as
/// Settings closes or changes section.
fn check_page_holder(app: &App) -> Option<Holder> {
    use super::check_settings::{DraftRow, page_target};
    // No project open: every key but Tab and Esc only says so in a toast.
    let Some((draft, has_points)) = page_target(app) else {
        return holder("SETTINGS", &[("Tab", "section")], ("Esc", "close"));
    };
    if draft.is_some_and(|d| d.editing) {
        return holder(
            "SETTINGS",
            &[
                ("", "type a command"),
                ("Enter", "done"),
                ("Ctrl+S", "save"),
                ("Tab", "section"),
            ],
            ("Esc", "close"),
        );
    }
    // No checkpoint yet: only the command row, so a letter (r too) starts the command.
    if !has_points {
        return holder(
            "SETTINGS",
            &[
                ("", "type a command"),
                ("Enter", "edit"),
                ("Ctrl+S", "save"),
                ("Tab", "section"),
            ],
            ("Esc", "close"),
        );
    }
    let hints: &'static [Hint] = match draft.map_or(DraftRow::Command, |d| d.row) {
        DraftRow::Command => &[
            ("↑↓", "move"),
            ("Enter", "edit"),
            ("Ctrl+S", "save"),
            ("r", "run now"),
            ("Del", "remove"),
            ("Tab", "section"),
        ],
        DraftRow::Timeout => &[
            ("↑↓", "move"),
            ("←→", "change"),
            ("Ctrl+S", "save"),
            ("r", "run now"),
            ("Del", "remove"),
            ("Tab", "section"),
        ],
        DraftRow::Points => &[
            ("↑↓", "move"),
            ("←→", "checkpoint"),
            ("Ctrl+S", "save"),
            ("r", "run now"),
            ("Del", "remove"),
            ("Tab", "section"),
        ],
    };
    holder("SETTINGS", hints, ("Esc", "close"))
}

/// The side panel's view, while it has the keys: what its own key handler reads in the state
/// it is in (Files' editor, filter or preview; Changes asking y/n).
fn view_holder(app: &App) -> Option<Holder> {
    use crate::client::View;
    if !app.view_has_keys() {
        return None;
    }
    match app.view.as_ref()? {
        View::Files(v) if v.edit.is_some() => holder(
            "EDITOR",
            &[("Ctrl+S", "save"), ("Ctrl+Z", "undo")],
            ("Esc", "done"),
        ),
        View::Files(v) if v.filtering => holder(
            "FILES",
            &[("", "type to filter"), ("Enter", "keep")],
            ("Esc", "clear"),
        ),
        View::Files(v) if v.in_preview => {
            holder("FILES", &[("↑↓", "read"), ("i", "edit")], ("Esc", "back"))
        }
        View::Files(_) => holder(
            "FILES",
            &[("/", "filter"), ("i", "edit"), ("Enter", "insert path")],
            ("Esc", "close"),
        ),
        View::Changes(v) if v.confirm.is_some() => holder("CHANGES", &[("y", "yes")], ("n", "no")),
        View::Changes(_) => holder(
            "CHANGES",
            &[
                ("↑↓", "file"),
                ("c", "commit"),
                ("x", "reviewed"),
                ("e", "editor"),
            ],
            ("Esc", "close"),
        ),
        View::Branch(_) => holder(
            "BRANCH",
            &[("↑↓", "choose"), ("Enter", "open")],
            ("Esc", "close"),
        ),
        View::Check(v) if super::check_sheet::sheet_failed(app, v) => holder(
            "CHECK",
            &[
                ("r", "re-run"),
                ("m", "send to agent"),
                ("f", "full output"),
                ("i", "why"),
            ],
            ("Esc", "back"),
        ),
        View::References(_) => holder(
            "REFERENCES",
            &[
                ("Enter", "open"),
                ("p", "paste image"),
                ("n", "add file"),
                ("d", "remove"),
                ("y", "copy path"),
            ],
            ("Esc", "close"),
        ),
        View::Check(_) => holder(
            "CHECK",
            &[("r", "re-run"), ("f", "full output"), ("i", "why")],
            ("Esc", "back"),
        ),
        View::Both(_) => holder(
            "BOTH DIFFS",
            &[("↑↓", "scroll"), ("k", "dismiss")],
            ("Esc", "close"),
        ),
    }
}

/// One of a holder's hints: its key bold in the accent, what it does muted; a hint with no key
/// is words alone.
fn hint_piece((key, words): Hint, t: &Theme) -> Piece {
    if key.is_empty() {
        return Piece {
            segs: vec![seg(words, Style::default().fg(t.muted).bg(t.bg))],
            gap: GAP,
            hit: None,
        };
    }
    hint(key.to_lowercase(), words, None, t)
}

/// Something else has the keys: its name in a calm chip, then its keys at the right with the
/// way out last. While key mode is held it says so, first to go when narrow, so nobody
/// wonders where the mode went. Only words: the keys are that thing's to take.
fn holding(app: &App, h: &Holder, t: &Theme) -> (Piece, Vec<Piece>) {
    let tag = mode_chip(h.label, t.btn, t.strong, None);
    let mut right: Vec<Piece> = h.hints.iter().map(|&hint| hint_piece(hint, t)).collect();
    if shortcut_waits(app) {
        right.push(hint_piece(("", "shortcut mode waits"), t));
    }
    right.push(hint_piece(h.out, t));
    (tag, right)
}

fn width(pieces: &[Piece]) -> u16 {
    pieces.iter().map(|p| p.gap + segs_width(&p.segs)).sum()
}

/// Under anything that has the keys (a popup, a modal, a view shortcut mode opened, the key map
/// with its search), the bar is only words: the keys and the way out are that thing's, and
/// what was typed in it must survive. It still takes the click, which would otherwise land
/// outside the popup and close it.
fn only_says(app: &App) -> bool {
    shortcut_waits(app) || !matches!(app.mode, Mode::Normal | Mode::Side | Mode::Prefix { .. })
}

/// Draw the status bar on `row`: the mode chip and where the keys go at the left, the hints and
/// (with the sidebar's foot out of view) Update or Restart at the right.
pub(in crate::client) fn draw_mode_bar(app: &mut App, buf: &mut Buffer, row: Rect, t: &Theme) {
    app.screen.bar_keys.clear();
    if row.height == 0 || row.width < 4 {
        return;
    }
    fill(buf, row, t.bg);
    let holder = keys_holder(app);
    let (tag, mut right, place) = if let Some(h) = &holder {
        let (tag, right) = holding(app, h, t);
        (tag, right, None)
    } else if armed(app) {
        let tag = mode_chip("KEYS", t.blocked, t.ink_on(t.blocked), Some(ScreenHit::Leader));
        (tag, key_hints(app, t), where_piece(app, t))
    } else {
        let tag = mode_chip("PANE", t.accent, t.acc_ink, Some(ScreenHit::ShortcutMode));
        (tag, pane_hints(app, t), where_piece(app, t))
    };
    let mut releases: Vec<Piece> = if app.screen.release_in_side {
        Vec::new()
    } else {
        [Release::Words, Release::Version, Release::Arrow]
            .into_iter()
            .filter_map(|r| release_pill(app, r, t))
            .collect()
    };
    let room = row.width.saturating_sub(2);
    let tag_w = width(std::slice::from_ref(&tag));
    let mut place = place;
    let used = |place: &Option<Piece>, right: &[Piece], releases: &[Piece]| {
        tag_w
            + place.as_ref().map_or(0, |p| width(std::slice::from_ref(p)))
            + width(right)
            + releases.first().map_or(0, |p| width(std::slice::from_ref(p)))
    };
    // The hints give way from the right, the way out last; then Update shortens and goes;
    // then where the keys go.
    while used(&place, &right, &releases) > room {
        if right.len() > 1 {
            right.remove(right.len() - 2);
        } else if !releases.is_empty() {
            releases.remove(0);
        } else if place.is_some() {
            place = None;
        } else {
            right.clear();
            break;
        }
    }
    right.extend(releases.into_iter().take(1));
    let words_only = only_says(app);
    // With `ui.rounded = "full"` a chip on the bar is capped on the bar's ground.
    let round = Look::of(&app.cfg.ui).round_tabs;
    let ground = t.bg;
    let put_piece = |app: &mut App, buf: &mut Buffer, x: u16, p: &Piece| -> u16 {
        let chip = p.segs.first().is_some_and(|(_, s)| s.bg != Some(ground));
        let segs = if round && chip {
            round_ends(p.segs.clone(), ground)
        } else {
            p.segs.clone()
        };
        let end = put(buf, x, row.y, &segs, row.right());
        if let (false, Some(h)) = (words_only, p.hit) {
            hit(app, Rect { x, y: row.y, width: end - x, height: 1 }, h);
        }
        end
    };
    let mut x = put_piece(app, buf, row.x + 1, &tag);
    if let Some(p) = &place {
        x = put_piece(app, buf, x + p.gap, p);
    }
    let _ = x;
    let mut x = row.right().saturating_sub(1 + width(&right));
    for p in &right {
        x = put_piece(app, buf, x + p.gap, p);
    }
    if words_only {
        hit(app, row, ScreenHit::Noop);
    }
}
