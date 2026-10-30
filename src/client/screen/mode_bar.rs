//! The mode bar along the bottom of the screen (ADR 0024): which mode the keys are in, always in
//! view: in shortcut mode what the keys do, read from the keymap so a rebinding shows, and when
//! something else has the keys, what it is and how to use and leave it.

use super::*;
use crate::keys::{Action, KeySpec};

/// What shortcut mode suggests after its tag, left to right; a narrow bar drops them from the
/// right. All keys and Esc come after them and go last.
fn suggestions() -> [(Action, &'static str); 7] {
    [
        (Action::ShellHere, "shell"),
        (Action::StartSession, "session"),
        (Action::Changes, "changes"),
        (Action::Files, "files"),
        (Action::Jump, "inbox"),
        (Action::Settings, "settings"),
        (Action::References, "references"),
    ]
}

/// Cells between two suggestions.
const GAP: u16 = 2;

/// One thing on the bar, the cells before it, and what a click on it does.
#[derive(Clone)]
struct Piece {
    segs: Vec<Seg>,
    gap: u16,
    hit: Option<ScreenHit>,
}

/// How much of Update or Restart shows: all its words, the version, the arrow, or nothing.
#[derive(Clone, Copy, PartialEq)]
enum Release {
    Words,
    Version,
    Arrow,
    Hidden,
}

/// Update when a newer release is out, else Restart once a newer one is installed but not
/// running, as much of it as `shown`.
fn release_pill(app: &App, shown: Release, t: &Theme) -> Option<Piece> {
    let style = Style::default().fg(t.accent).bg(t.btn).add_modifier(Modifier::BOLD);
    let (words, h) = if let Some(version) = app.update_available.as_ref() {
        let words = match shown {
            Release::Words => format!(" ↑ Update {version} "),
            Release::Version => format!(" ↑ {version} "),
            _ => " ↑ ".to_string(),
        };
        (words, ScreenHit::Update)
    } else if app.restart_needed() {
        // Installed (here or from outside the app), not running yet: one click finishes it.
        ((if shown == Release::Words { " ↻ Restart " } else { " ↻ " }).to_string(), ScreenHit::Restart)
    } else {
        return None;
    };
    (shown != Release::Hidden).then(|| Piece { segs: vec![seg(words, style)], gap: 0, hit: Some(h) })
}

/// The bar's ground: sky while shortcut mode is lit, so the whole row says so, else the screen's.
fn ground(app: &App, t: &Theme) -> Color {
    if armed(app) { t.sky() } else { t.bg }
}

/// A key bound after the leader to `a`, if any.
fn bound(app: &App, a: &Action) -> Option<KeySpec> {
    app.keymap.prefixed_order.iter().find(|k| app.keymap.prefixed.get(k) == Some(a)).copied()
}

/// A suggestion: its key in bold, then what it does, on the lit ground. A click presses the key.
fn suggestion(app: &mut App, key: KeySpec, label: &str, t: &Theme) -> Piece {
    let ink = Style::default().fg(t.ink_on(t.sky())).bg(t.sky());
    app.screen.bar_keys.push(key);
    let segs = vec![seg(crate::client::design::long_key(&key), ink.add_modifier(Modifier::BOLD)), seg(format!(" {label}"), ink)];
    Piece { segs, gap: GAP, hit: Some(ScreenHit::BarKey(app.screen.bar_keys.len() - 1)) }
}

/// Lit: the SHORTCUT tag (a click opens the key map), the suggestions bound to a key, then All
/// keys and Esc, which go last.
fn lit(app: &mut App, t: &Theme) -> (Piece, Vec<Piece>, Vec<Piece>) {
    let tag = Style::default().fg(t.sky()).bg(t.bg).add_modifier(Modifier::BOLD);
    let tag = Piece { segs: vec![seg(" SHORTCUT ", tag)], gap: 0, hit: Some(ScreenHit::Leader) };
    let mut middle = Vec::new();
    for (a, label) in suggestions() {
        if let Some(key) = bound(app, &a) {
            middle.push(suggestion(app, key, label, t));
        }
    }
    let mut tail: Vec<Piece> = bound(app, &Action::Help).map(|key| suggestion(app, key, "all keys", t)).into_iter().collect();
    let esc = KeySpec { code: crossterm::event::KeyCode::Esc, mods: crossterm::event::KeyModifiers::NONE };
    tail.push(suggestion(app, esc, "type", t));
    (tag, middle, tail)
}

/// Typing: quiet, and how to get to the shortcuts. A click there starts shortcut mode, as the
/// leader does.
fn typing(app: &App, t: &Theme) -> (Piece, Vec<Piece>) {
    let hit = Some(ScreenHit::ShortcutMode);
    let lead = crate::client::design::long_key(&app.keymap.prefix);
    if Look::of(&app.cfg.ui).hairline {
        // Hairline has no chips: the mode is a bold word in the accent.
        let tag = Piece { segs: vec![seg("TYPING", Style::default().fg(t.accent).bg(t.bg).add_modifier(Modifier::BOLD))], gap: 0, hit };
        let hint = Piece { segs: vec![seg(format!("{lead} for shortcuts"), Style::default().fg(t.muted).bg(t.bg))], gap: GAP, hit };
        return (tag, vec![hint]);
    }
    let tag = Piece { segs: vec![seg(" TYPING ", Style::default().fg(t.muted).bg(t.btn))], gap: 0, hit };
    let hint = Piece { segs: vec![seg(format!("· {lead} for shortcuts"), Style::default().fg(t.muted).bg(t.bg))], gap: 1, hit };
    (tag, vec![hint])
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
        Mode::KeyMap(km) if km.step.is_some() => holder("KEY MAP", &[("", "a letter picks"), ("↑↓", "choose"), ("Enter", "run"), ("Backspace", "back")], ("Esc", "close")),
        Mode::KeyMap(km) => match km.view {
            KeyView::Search => holder("KEY MAP", &[("", "type to search"), ("↑↓", "pick"), ("Enter", "run")], ("Esc", "close")),
            KeyView::All => holder("KEY MAP", &[("", "type to search"), ("↑↓", "scroll")], ("Esc", "close")),
            KeyView::Map => None,
        },
        Mode::Side => holder("SIDEBAR", &[("↑↓", "move"), ("Enter", "open")], ("Esc", "back")),
        // Waiting for a key: the next one, arrows too, becomes the binding.
        Mode::ScreenSettings(v) if v.capturing => holder("SETTINGS", &[("", "press the new key")], ("Esc", "cancel")),
        Mode::ScreenSettings(v) if v.editing.is_some() => holder("SETTINGS", &[("", "type"), ("Enter", "save")], ("Esc", "cancel")),
        Mode::ScreenSettings(v) if crate::client::modal::Cat::ALL.get(v.cat) == Some(&crate::client::modal::Cat::Check) => check_page_holder(app),
        Mode::ScreenSettings(_) => holder("SETTINGS", &[("↑↓", "move"), ("←→", "change"), ("Tab", "section")], ("Esc", "close")),
        // A merge or throw-away asked in a finished row: Enter does it, Esc only drops the question.
        Mode::GoTo { .. } => match app.screen.inbox_confirm {
            Some((_, true, false)) => holder("INBOX", &[("Enter", "merge")], ("Esc", "don't")),
            Some((_, false, false)) => holder("INBOX", &[("Enter", "throw away")], ("Esc", "don't")),
            _ if matches!(super::inbox::inbox_selected(app), Some(GoRow::Failed(_))) => holder("INBOX", &[("↑↓", "select"), ("Enter", "open"), ("m", "send to agent")], ("Esc", "close")),
            _ => holder("INBOX", &[("↑↓", "select"), ("Enter", "open")], ("Esc", "close")),
        },
        Mode::Compose(_) => holder("MESSAGE", &[("Enter", "send"), ("Shift+Enter", "new line")], ("Esc", "keep draft")),
        Mode::Confirm(c) if matches!(c.act, crate::client::menu::Act::OverrideCheck(..)) => holder("OVERRIDE", &[("", "type a reason"), ("Enter", "override")], ("Esc", "cancel")),
        Mode::Confirm(_) => holder("CONFIRM", &[("y", "yes")], ("n", "no")),
        Mode::Prompt { kind, .. } if kind.is_confirm() => holder("CONFIRM", &[("y", "yes")], ("n", "no")),
        Mode::Prompt { kind: crate::client::PromptKind::AddReference(..), .. } => holder("ADD REFERENCE", &[("", "type or paste a path"), ("Enter", "add")], ("Esc", "cancel")),
        Mode::Prompt { .. } | Mode::RenameTab(_) => holder("NAME", &[("", "type"), ("Enter", "save")], ("Esc", "cancel")),
        Mode::Why { .. } => holder("WHY", &[], ("Esc", "close")),
        Mode::Helpers { .. } => holder("HELPERS", &[("1", "allow"), ("2", "always"), ("3", "no")], ("Esc", "back")),
        Mode::Secret { .. } => holder("SECRET", &[("", "type or paste"), ("Enter", "give")], ("Esc", "decline")),
        Mode::Approve(_) => holder("RUN CHECK?", &[("1", "run once"), ("2", "always"), ("3", "never")], ("Esc", "not now")),
        Mode::ProjectCheck(d) if d.editing => holder("CHECK SETTINGS", &[("", "type a command"), ("Enter", "done"), ("Ctrl+S", "save")], ("Esc", "cancel")),
        Mode::ProjectCheck(d) if !super::check_settings::has_points(app, &d.project) => holder("CHECK SETTINGS", &[("", "type a command"), ("Enter", "edit"), ("Ctrl+S", "save")], ("Esc", "cancel")),
        Mode::ProjectCheck(_) => holder("CHECK SETTINGS", &[("Ctrl+S", "save"), ("↑↓", "move"), ("r", "run now")], ("Esc", "cancel")),
        Mode::Finder(fd) if fd.for_setting.is_some() => holder("OPEN FOLDER", &[("", "type a path"), ("Tab", "complete"), ("Enter", "open")], ("Esc", "back")),
        Mode::Finder(_) => holder("OPEN FOLDER", &[("", "type a path"), ("Tab", "complete"), ("Enter", "open")], ("Esc", "close")),
        Mode::ScreenPane(_) => holder("NEW PANE", &[("↑↓", "rows"), ("←→", "choose"), ("Enter", "start")], ("Esc", "close")),
        Mode::Themes { .. } => holder("THEMES", &[("", "type to filter"), ("↑↓", "move"), ("Enter", "apply")], ("Esc", "back")),
        Mode::Spinners(_) => holder("SPINNERS", &[("↑↓", "preview"), ("Enter", "apply")], ("Esc", "back")),
        Mode::ScreenMenu(_) => holder("MENU", &[("↑↓", "move"), ("Enter", "pick")], ("Esc", "close")),
        Mode::Actions { .. } => holder("COMMANDS", &[("↑↓", "move"), ("Enter", "run")], ("Esc", "close")),
        Mode::Find(_) => holder("FIND", &[("", "type to search"), ("Enter", "into the prompt"), ("Tab", "files / search")], ("Esc", "close")),
        Mode::Branch(v) => match v.step {
            Step::Pick => holder("SWITCH BRANCH", &[("", "type to filter"), ("↑↓", "choose"), ("Enter", "switch")], ("Esc", "close")),
            Step::Changes(_) => holder("SWITCH BRANCH", &[("1-4", "choose")], ("Esc", "back")),
            Step::Message(_) => holder("SWITCH BRANCH", &[("", "type a message"), ("Enter", "commit and switch")], ("Esc", "back")),
            Step::ConfirmDiscard => holder("SWITCH BRANCH", &[("y", "discard")], ("", "any other key back")),
        },
        Mode::History { .. } => holder("HISTORY", &[("↑↓", "move"), ("Enter", "go to it"), ("c", "clear")], ("Esc", "close")),
        Mode::Tidy(_) => holder("TIDY", &[("Space", "tick"), ("a", "all the safe ones"), ("Enter", "remove")], ("Esc", "close")),
        Mode::Worktrees { .. } => holder("WORKTREES", &[("", "type a branch"), ("↑↓", "move"), ("Enter", "open")], ("Esc", "close")),
        Mode::Copy(c) if c.input.is_some() => holder("COPY", &[("", "type to search"), ("Enter", "find")], ("Esc", "cancel")),
        // Esc drops a selection first; only the next one leaves.
        Mode::Copy(c) if c.anchor.is_some() => holder("COPY", &[("y", "copy")], ("Esc", "clear")),
        Mode::Copy(_) => holder("COPY", &[("v", "select"), ("y", "copy"), ("/", "search")], ("Esc", "leave")),
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
        return holder("SETTINGS", &[("", "type a command"), ("Enter", "done"), ("Ctrl+S", "save"), ("Tab", "section")], ("Esc", "close"));
    }
    // No checkpoint yet: only the command row, so a letter (r too) starts the command.
    if !has_points {
        return holder("SETTINGS", &[("", "type a command"), ("Enter", "edit"), ("Ctrl+S", "save"), ("Tab", "section")], ("Esc", "close"));
    }
    let hints: &'static [Hint] = match draft.map_or(DraftRow::Command, |d| d.row) {
        DraftRow::Command => &[("↑↓", "move"), ("Enter", "edit"), ("Ctrl+S", "save"), ("r", "run now"), ("Del", "remove"), ("Tab", "section")],
        DraftRow::Timeout => &[("↑↓", "move"), ("←→", "change"), ("Ctrl+S", "save"), ("r", "run now"), ("Del", "remove"), ("Tab", "section")],
        DraftRow::Points => &[("↑↓", "move"), ("←→", "checkpoint"), ("Ctrl+S", "save"), ("r", "run now"), ("Del", "remove"), ("Tab", "section")],
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
        View::Files(v) if v.edit.is_some() => holder("EDITOR", &[("Ctrl+S", "save"), ("Ctrl+Z", "undo")], ("Esc", "done")),
        View::Files(v) if v.filtering => holder("FILES", &[("", "type to filter"), ("Enter", "keep")], ("Esc", "clear")),
        View::Files(v) if v.in_preview => holder("FILES", &[("↑↓", "read"), ("i", "edit")], ("Esc", "back")),
        View::Files(_) => holder("FILES", &[("/", "filter"), ("i", "edit"), ("Enter", "insert path")], ("Esc", "close")),
        View::Changes(v) if v.confirm.is_some() => holder("CHANGES", &[("y", "yes")], ("n", "no")),
        View::Changes(_) => holder("CHANGES", &[("↑↓", "file"), ("c", "commit"), ("x", "reviewed"), ("e", "editor")], ("Esc", "close")),
        View::Branch(_) => holder("BRANCH", &[("↑↓", "choose"), ("Enter", "open")], ("Esc", "close")),
        View::Check(v) if super::check_sheet::sheet_failed(app, v) => holder("CHECK", &[("r", "re-run"), ("m", "send to agent"), ("f", "full output"), ("i", "why")], ("Esc", "back")),
        View::References(_) => holder("REFERENCES", &[("Enter", "open"), ("p", "paste image"), ("n", "add file"), ("d", "remove")], ("Esc", "close")),
        View::Check(_) => holder("CHECK", &[("r", "re-run"), ("f", "full output"), ("i", "why")], ("Esc", "back")),
        View::Both(_) => holder("BOTH DIFFS", &[("↑↓", "scroll"), ("k", "dismiss")], ("Esc", "close")),
    }
}

/// One hint after a `·`: its key bold, what it does plain, both quiet on the screen's ground.
fn hint_piece((key, words): Hint, t: &Theme) -> Piece {
    let quiet = Style::default().fg(t.muted).bg(t.bg);
    let mut segs = vec![seg("· ", quiet)];
    if !key.is_empty() {
        segs.push(seg(format!("{key} "), quiet.fg(t.strong).add_modifier(Modifier::BOLD)));
    }
    segs.push(seg(words, quiet));
    Piece { segs, gap: 1, hit: None }
}

/// Something else has the keys: its name in a calm chip (sky is for shortcut mode lit), its
/// keys, and the way out last. While shortcut mode is held it says so, first to go when narrow,
/// so nobody wonders where the mode went. Only words: the keys are that thing's to take.
fn holding(app: &App, h: &Holder, t: &Theme) -> (Piece, Vec<Piece>, Vec<Piece>) {
    let tag = if Look::of(&app.cfg.ui).hairline {
        seg(h.label, Style::default().fg(t.strong).bg(t.bg).add_modifier(Modifier::BOLD))
    } else {
        seg(format!(" {} ", h.label), Style::default().fg(t.strong).bg(t.btn).add_modifier(Modifier::BOLD))
    };
    let tag = Piece { segs: vec![tag], gap: 0, hit: None };
    let mut middle: Vec<Piece> = h.hints.iter().map(|&hint| hint_piece(hint, t)).collect();
    if shortcut_waits(app) {
        middle.push(hint_piece(("", "shortcut mode waits"), t));
    }
    (tag, middle, vec![hint_piece(h.out, t)])
}

fn width(pieces: &[&Piece]) -> u16 {
    pieces.iter().map(|p| p.gap + segs_width(&p.segs)).sum()
}

/// The fullest bar that fits `room`: the suggestions go from the right, then Update shortens and
/// goes, then All keys, then Esc. The tag always stays.
fn fit<'a>(room: u16, tag: &'a Piece, middle: &'a [Piece], tail: &'a [Piece], releases: &'a [Piece]) -> (Vec<&'a Piece>, Option<&'a Piece>) {
    let (mut mid, mut rel, mut keep) = (middle.len(), 0, tail.len());
    loop {
        let left: Vec<&Piece> = std::iter::once(tag).chain(&middle[..mid]).chain(&tail[tail.len() - keep..]).collect();
        let release = releases.get(rel);
        let release_w = release.map_or(0, |r| segs_width(&r.segs) + GAP);
        if width(&left) + release_w <= room {
            return (left, release);
        }
        if mid > 0 {
            mid -= 1;
        } else if rel < releases.len() {
            rel += 1;
        } else if keep > 0 {
            keep -= 1;
        } else {
            return (left, None);
        }
    }
}

/// Under anything that has the keys (a popup, a modal, a view shortcut mode opened, the key map
/// with its search), the bar is only words: the keys and the way out are that thing's, and
/// what was typed in it must survive. It still takes the click, which would otherwise land
/// outside the popup and close it.
fn only_says(app: &App) -> bool {
    shortcut_waits(app) || !matches!(app.mode, Mode::Normal | Mode::Side | Mode::Prefix { .. })
}

/// After TYPING: which tab of its session this is and how many splits it shows, dim. When
/// narrow the hint after it goes first, then this.
fn layout_piece(app: &App, t: &Theme) -> Option<Piece> {
    let n = app.screen.leaf_rects.len();
    let tabs = app.session_tabs();
    let at = tabs.iter().position(|&i| i == app.screen.tab)?;
    (n > 0 && app.focused().is_some()).then(|| {
        let words = format!("tab {}/{} · {n} split{}", at + 1, tabs.len(), if n == 1 { "" } else { "s" });
        Piece { segs: vec![seg(words, Style::default().fg(t.muted).bg(t.bg))], gap: GAP, hit: None }
    })
}

/// The right end's first piece: how many sessions need you in amber (the session's name when
/// it is one), shortest last, or `all clear`.
fn alerts(app: &App, needs: usize, t: &Theme) -> Vec<Piece> {
    let amber = Style::default().fg(t.blocked).bg(t.bg).add_modifier(Modifier::BOLD);
    let words = |text: String, st: Style| Piece { segs: vec![seg(text, st)], gap: 0, hit: None };
    if needs == 0 {
        return vec![words("all clear".into(), Style::default().fg(t.muted).bg(t.bg))];
    }
    let lone = (needs == 1).then(|| app.snap.terms.values().find(|i| check::needs(i.status, i.check())).map(|i| session_name(app, &app.screen_model(), i.id))).flatten();
    let full = match lone {
        Some(name) => format!("● {name} needs you"),
        None => format!("● {needs} needs you"),
    };
    vec![words(full, amber), words(format!("● {needs} needs you"), amber), words(format!("● {needs}"), amber)]
}

/// The tab's checkout branch, dim: `⎇ bold-raven`.
fn branch_piece(app: &App, t: &Theme) -> Option<Piece> {
    let branch = app.focused().and_then(|f| app.snap.terms.get(&f)).and_then(|i| i.branch.clone())?;
    let words = format!("{} {branch}", check::marks(app).branch);
    Some(Piece { segs: vec![seg(words, Style::default().fg(t.muted).bg(t.bg))], gap: GAP, hit: None })
}

/// The focused pane's pull request: `#7 2 failing`, red when there is something to act on.
fn pr_piece(app: &App, t: &Theme) -> Option<Piece> {
    let pr = app.focused().and_then(|f| app.snap.terms.get(&f)).and_then(|i| i.pr.as_ref())?;
    let colour = match pr.tone() {
        crate::protocol::PrTone::Bad => t.err,
        crate::protocol::PrTone::Good => t.done,
        crate::protocol::PrTone::Waiting | crate::protocol::PrTone::Plain => t.muted,
    };
    Some(Piece { segs: vec![seg(pr.headline(), Style::default().fg(colour).bg(t.bg))], gap: GAP, hit: None })
}

/// What the tab's panes take of the machine, dim: `▲ 3% CPU · 1.2 GB`.
fn load_piece(app: &App, t: &Theme) -> Option<Piece> {
    let leaves = app.screen.tabs.get(app.screen.tab)?.layout.leaves();
    let (cpu, mem) = leaves.iter().filter_map(|id| app.snap.terms.get(id)).fold((0.0f32, 0u64), |(c, m), i| (c + i.cpu, m + i.mem));
    let gb = mem as f64 / f64::from(1u32 << 30);
    Some(Piece { segs: vec![seg(format!("▲ {cpu:.0}% CPU · {gb:.1} GB"), Style::default().fg(t.muted).bg(t.bg))], gap: GAP, hit: None })
}

/// Draw the mode bar on `row`: lit, who has the keys, or typing, with Update or Restart at its
/// right end. Hairline adds the look and pane count after TYPING and, unless shortcut mode
/// lights the row, how many need you just before Update or Restart (or at the end without them).
pub(in crate::client) fn draw_mode_bar(app: &mut App, buf: &mut Buffer, row: Rect, t: &Theme) {
    app.screen.bar_keys.clear();
    if row.height == 0 || row.width < 4 {
        return;
    }
    let holder = keys_holder(app);
    let ground = if holder.is_some() { t.bg } else { ground(app, t) };
    fill(buf, row, ground);
    let (tag, mut middle, tail) = if let Some(h) = &holder {
        holding(app, h, t)
    } else if armed(app) {
        lit(app, t)
    } else {
        let (tag, hint) = typing(app, t);
        (tag, hint, Vec::new())
    };
    let calm = ground == t.bg;
    if calm && holder.is_none() && let Some(p) = layout_piece(app, t) {
        middle.insert(0, p);
    }
    let releases: Vec<Piece> = [Release::Words, Release::Version, Release::Arrow].into_iter().filter_map(|r| release_pill(app, r, t)).collect();
    let round = Look::of(&app.cfg.ui).round_tabs;
    let caps = |p: &Piece| if round && p.segs.first().is_some_and(|(_, s)| s.bg != Some(ground)) { round_ends(p.segs.clone(), ground) } else { p.segs.clone() };
    let room = row.width.saturating_sub(2);
    // The right end: needs you keeps its place before the mode's hints and Update do; the
    // branch and the load are there only while the whole left side still fits beside them.
    let release_room = |release: Option<&Piece>| release.map_or(0, |r| segs_width(&r.segs) + GAP);
    let mut right: Vec<Piece> = Vec::new();
    let mut left_and_release = None;
    if calm {
        let tag_w = width(&[&tag]);
        let needing = alerts(app, super::hairline::needing(app), t);
        if let Some(alert) = needing.into_iter().find(|a| tag_w + GAP + segs_width(&a.segs) <= room) {
            let alert_w = GAP + segs_width(&alert.segs);
            let (left, release) = fit(room.saturating_sub(alert_w), &tag, &middle, &tail, &releases);
            let used = width(&left) + release_room(release) + alert_w;
            right.push(alert);
            for extra in branch_piece(app, t).into_iter().chain(pr_piece(app, t)).chain(load_piece(app, t)) {
                let more = extra.gap + segs_width(&extra.segs);
                if used + right[1..].iter().map(|p| p.gap + segs_width(&p.segs)).sum::<u16>() + more <= room {
                    right.push(extra);
                }
            }
            left_and_release = Some((left, release));
        }
    }
    let right_w: u16 = right.iter().map(|p| p.gap + segs_width(&p.segs)).sum();
    let (left, release) = left_and_release.unwrap_or_else(|| fit(room, &tag, &middle, &tail, &releases));
    if !right.is_empty() {
        let mut x = row.right().saturating_sub(1 + release_room(release) + right_w);
        for p in &right {
            x += p.gap;
            x = put(buf, x, row.y, &p.segs, row.right());
        }
    }
    let words_only = only_says(app);
    let mut x = row.x + 1;
    for p in left {
        x += p.gap;
        let segs = caps(p);
        let end = put(buf, x, row.y, &segs, row.right());
        if let (false, Some(h)) = (words_only, p.hit) {
            hit(app, Rect { x, y: row.y, width: end - x, height: 1 }, h);
        }
        x = end;
    }
    if let Some(p) = release {
        let segs = caps(p);
        let x = row.right().saturating_sub(1 + segs_width(&segs));
        put(buf, x, row.y, &segs, row.right());
        if let (false, Some(h)) = (words_only, p.hit) {
            hit(app, Rect { x, y: row.y, width: segs_width(&segs), height: 1 }, h);
        }
    }
    if words_only {
        hit(app, row, ScreenHit::Noop);
    }
}
