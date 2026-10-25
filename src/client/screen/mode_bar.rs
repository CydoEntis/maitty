//! The mode bar along the bottom of the screen (ADR 0024): which mode the keys are in, always in
//! view, and in shortcut mode what the keys do, read from the keymap so a rebinding shows.

use super::*;
use crate::keys::{Action, KeySpec};

/// What shortcut mode suggests after its tag, left to right; a narrow bar drops them from the
/// right. All keys and Esc come after them and go last.
fn suggestions() -> [(Action, &'static str); 6] {
    [
        (Action::ShellHere, "shell"),
        (Action::StartSession, "session"),
        (Action::Changes, "changes"),
        (Action::Files, "files"),
        (Action::Jump, "inbox"),
        (Action::Settings, "settings"),
    ]
}

/// Cells between two suggestions.
const GAP: u16 = 2;

/// One thing on the bar, the cells before it, and what a click on it does.
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
/// leader does; typing under a popup it is only words, so a click there still reaches the popup.
fn typing(app: &App, t: &Theme) -> (Piece, Vec<Piece>) {
    let clickable = matches!(app.mode, Mode::Normal | Mode::Side);
    let hit = clickable.then_some(ScreenHit::ShortcutMode);
    let tag = Piece { segs: vec![seg(" TYPING ", Style::default().fg(t.muted).bg(t.btn))], gap: 0, hit };
    let lead = crate::client::design::long_key(&app.keymap.prefix);
    let hint = Piece { segs: vec![seg(format!("· {lead} for shortcuts"), Style::default().fg(t.muted).bg(t.bg))], gap: 1, hit };
    (tag, vec![hint])
}

/// Held while a popup or view has the keys: dimmed, and nothing to suggest, since the popup
/// shows its own keys.
fn paused(t: &Theme) -> Piece {
    Piece { segs: vec![seg(" SHORTCUT · paused ", Style::default().fg(t.muted).bg(t.btn))], gap: 0, hit: None }
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

/// Draw the mode bar on `row`: lit, paused or typing, with Update or Restart at its right end.
pub(in crate::client) fn draw_mode_bar(app: &mut App, buf: &mut Buffer, row: Rect, t: &Theme) {
    app.screen.bar_keys.clear();
    if row.height == 0 || row.width < 4 {
        return;
    }
    let ground = ground(app, t);
    fill(buf, row, ground);
    let (tag, middle, tail) = if armed(app) {
        lit(app, t)
    } else if shortcut_paused(app) {
        (paused(t), Vec::new(), Vec::new())
    } else {
        let (tag, hint) = typing(app, t);
        (tag, hint, Vec::new())
    };
    let releases: Vec<Piece> = [Release::Words, Release::Version, Release::Arrow].into_iter().filter_map(|r| release_pill(app, r, t)).collect();
    let round = Look::of(&app.cfg.ui).round_tabs;
    let caps = |p: &Piece| if round && p.segs.first().is_some_and(|(_, s)| s.bg != Some(ground)) { round_ends(p.segs.clone(), ground) } else { p.segs.clone() };
    let room = row.width.saturating_sub(2);
    let (left, release) = fit(room, &tag, &middle, &tail, &releases);
    let paused = shortcut_paused(app);
    let mut x = row.x + 1;
    for p in left {
        x += p.gap;
        let segs = caps(p);
        let end = put(buf, x, row.y, &segs, row.right());
        if let (false, Some(h)) = (paused, p.hit) {
            hit(app, Rect { x, y: row.y, width: end - x, height: 1 }, h);
        }
        x = end;
    }
    if let Some(p) = release {
        let segs = caps(p);
        let x = row.right().saturating_sub(1 + segs_width(&segs));
        put(buf, x, row.y, &segs, row.right());
        if let (false, Some(h)) = (paused, p.hit) {
            hit(app, Rect { x, y: row.y, width: segs_width(&segs), height: 1 }, h);
        }
    }
    // Paused, it only says so: the popup has the keys and its own way out. The bar still takes
    // the click, which would otherwise land outside the popup and close it.
    if paused {
        hit(app, row, ScreenHit::Noop);
    }
}
