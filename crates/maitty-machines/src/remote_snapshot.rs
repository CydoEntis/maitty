//! A snapshot from another machine's daemon is whatever that daemon chose to send, so it is
//! made safe before it is kept: every string loses its control and hidden characters and is cut
//! to a length, every list is capped, and sizes that drive layout are held to a range.
//!
//! The walk is over the snapshot's serialized form, so a string field added to the protocol is
//! cleaned without anyone remembering to list it here.

use maitty_shared::hidden::sanitize;
use maitty_shared::protocol::{Snapshot, TermInfo};
use serde_json::Value;

/// Longest name, title, branch, command or similar one-line string, in characters.
pub const LINE_MAX: usize = 512;
/// Longest path, in characters.
pub const PATH_MAX: usize = 4096;
/// Longest multi-line text (what an agent said, what it asks), in characters.
pub const TEXT_MAX: usize = 8192;
/// Most entries kept in any one list or map.
pub const ITEMS_MAX: usize = 10_000;
/// Most values a snapshot may hold in all; a snapshot past it is refused whole.
pub const NODES_MAX: usize = 250_000;
/// Largest pane width or height taken from a remote daemon. A terminal's grid is allocated
/// from it.
pub const PANE_DIM_MAX: u16 = 1000;
/// Largest repeat count taken from a remote daemon. It is drawn as digits on a row.
pub const REPEATS_MAX: u32 = 99_999;
/// Highest workspace colour index taken from a remote daemon. It names a place in a palette
/// this window does not hold, so it is held to a range any palette covers.
pub const WORKSPACE_COLOR_MAX: u8 = 15;

#[derive(Clone, Copy)]
enum Class {
    Line,
    Path,
    Text,
}

impl Class {
    /// Fields named here are text blocks or paths; every other string is one line.
    fn of_key(key: &str) -> Option<Class> {
        match key {
            "said" | "asking" | "text" | "result" | "latest_activity" | "task" => Some(Class::Text),
            "cwd" | "root" | "top" | "path" => Some(Class::Path),
            _ => None,
        }
    }

    fn max(self) -> usize {
        match self {
            Class::Line => LINE_MAX,
            Class::Path => PATH_MAX,
            Class::Text => TEXT_MAX,
        }
    }
}

/// `snap` with every string cleaned, every list capped and layout sizes clamped. `Err` with
/// the reason when it cannot be made safe (too many values, or a number that is not finite).
pub fn sanitize_snapshot(snap: Snapshot) -> Result<Snapshot, &'static str> {
    let mut value = serde_json::to_value(&snap).map_err(|_| "it could not be inspected")?;
    drop(snap);
    let mut nodes = 0;
    clean(&mut value, Class::Line, &mut nodes)?;
    let mut snap: Snapshot =
        serde_json::from_value(value).map_err(|_| "it was not a snapshot once cleaned")?;
    clamp_numbers(&mut snap);
    Ok(snap)
}

/// `terms` cleaned like the panes of a snapshot. `Err` when there are more than a snapshot may
/// hold or one cannot be made safe.
pub fn sanitize_terms(terms: Vec<TermInfo>) -> Result<Vec<TermInfo>, &'static str> {
    if terms.len() > ITEMS_MAX {
        return Err("it names too many panes");
    }
    let mut value = serde_json::to_value(&terms).map_err(|_| "it could not be inspected")?;
    drop(terms);
    let mut nodes = 0;
    clean(&mut value, Class::Line, &mut nodes)?;
    let mut terms: Vec<TermInfo> =
        serde_json::from_value(value).map_err(|_| "it was not panes once cleaned")?;
    terms.iter_mut().for_each(clamp_term);
    Ok(terms)
}

fn clean(value: &mut Value, class: Class, nodes: &mut usize) -> Result<(), &'static str> {
    *nodes += 1;
    if *nodes > NODES_MAX {
        return Err("it holds too many values");
    }
    match value {
        Value::String(s) => {
            let head: String = s.chars().take(class.max()).collect();
            *s = sanitize(&head, matches!(class, Class::Text));
        }
        Value::Array(items) => {
            items.truncate(ITEMS_MAX);
            for item in items {
                clean(item, class, nodes)?;
            }
        }
        Value::Object(map) => {
            let extra: Vec<String> = map.keys().skip(ITEMS_MAX).cloned().collect();
            for key in extra {
                map.remove(&key);
            }
            for (key, item) in map.iter_mut() {
                clean(item, Class::of_key(key).unwrap_or(class), nodes)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn clamp_term(t: &mut TermInfo) {
    t.cols = t.cols.clamp(1, PANE_DIM_MAX);
    t.rows = t.rows.clamp(1, PANE_DIM_MAX);
    t.cpu = t.cpu.clamp(0.0, 100.0);
    t.repeats = t.repeats.map(|n| n.min(REPEATS_MAX));
}

fn clamp_numbers(snap: &mut Snapshot) {
    snap.terms.values_mut().for_each(clamp_term);
    for w in &mut snap.workspaces {
        w.color = w.color.min(WORKSPACE_COLOR_MAX);
    }
    snap.spent_today = snap.spent_today.max(0.0);
}
