//! A snapshot from another machine's daemon is whatever that daemon chose to send, so it is
//! made safe before it is kept: every string loses its control and hidden characters and is cut
//! to a length, every list is capped, and sizes that drive layout are held to a range.
//!
//! The walk is over the snapshot's serialized form, so a string field added to the protocol is
//! cleaned without anyone remembering to list it here.

use crate::hidden::sanitize;
use crate::protocol::{Snapshot, TermInfo};
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
}

fn clamp_numbers(snap: &mut Snapshot) {
    snap.terms.values_mut().for_each(clamp_term);
    for w in &mut snap.workspaces {
        w.color = w.color.min(WORKSPACE_COLOR_MAX);
    }
    snap.spent_today = snap.spent_today.max(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::HumanQuestion;

    const HOSTILE: &str = "\u{1b}[2J\u{1b}]52;c;AAAA\u{7}\u{202e}\0\u{200b}\u{2028}";

    fn clean_text(s: &str) -> bool {
        !s.chars()
            .any(|c| c.is_control() || crate::hidden::is_invisible(c) || c == '\u{2028}')
    }

    fn strings<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
        match v {
            Value::String(s) => out.push(s),
            Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
            Value::Object(m) => m.values().for_each(|x| strings(x, out)),
            _ => {}
        }
    }

    /// Every string in `snap` turned hostile, as far as it still reads as a snapshot (an enum's
    /// variant name is a string too, and cannot change). Returns how many were turned.
    fn make_hostile(snap: &Snapshot) -> (Snapshot, usize) {
        let mut value = serde_json::to_value(snap).unwrap();
        let mut turned = 0;
        let mut at = 0;
        loop {
            let mut seen = 0;
            let mut found = false;
            let mut trial = value.clone();
            visit(&mut trial, &mut |s| {
                if seen == at {
                    *s = format!("{HOSTILE}{s}");
                    found = true;
                }
                seen += 1;
            });
            if !found {
                break;
            }
            if serde_json::from_value::<Snapshot>(trial.clone()).is_ok() {
                value = trial;
                turned += 1;
            }
            at += 1;
        }
        (serde_json::from_value(value).unwrap(), turned)
    }

    fn visit(v: &mut Value, f: &mut impl FnMut(&mut String)) {
        match v {
            Value::String(s) => f(s),
            Value::Array(a) => a.iter_mut().for_each(|x| visit(x, f)),
            Value::Object(m) => m.values_mut().for_each(|x| visit(x, f)),
            _ => {}
        }
    }

    fn rich() -> Snapshot {
        let mut app = crate::client::reference_fixture::world(
            crate::client::reference_fixture::Stage::Failed,
        );
        let mut snap = std::mem::take(&mut app.snap);
        snap.questions.push(HumanQuestion {
            id: 1,
            term: 1,
            text: "use pnpm or npm?".into(),
            options: vec!["pnpm".into(), "npm".into()],
        });
        snap
    }

    #[test]
    fn no_string_anywhere_in_a_hostile_snapshot_keeps_a_control_or_hidden_character() {
        let (hostile, turned) = make_hostile(&rich());
        assert!(
            turned > 30,
            "the fixture should exercise many fields: {turned}"
        );
        let before = serde_json::to_value(&hostile).unwrap();
        let mut found = Vec::new();
        strings(&before, &mut found);
        assert!(found.iter().any(|s| !clean_text(s)), "the input is hostile");

        let safe = sanitize_snapshot(hostile).unwrap();
        let after = serde_json::to_value(&safe).unwrap();
        let mut found = Vec::new();
        strings(&after, &mut found);
        let bad: Vec<_> = found.iter().filter(|s| !clean_text(s)).collect();
        assert!(bad.is_empty(), "survived: {bad:?}");
    }

    #[test]
    fn a_clean_snapshot_comes_back_unchanged() {
        let snap = rich();
        let same = sanitize_snapshot(snap.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&snap).unwrap(),
            serde_json::to_value(&same).unwrap()
        );
        assert!(!same.terms.is_empty());
    }

    #[test]
    fn cleaning_twice_changes_nothing() {
        let (hostile, _) = make_hostile(&rich());
        let once = sanitize_snapshot(hostile).unwrap();
        let twice = sanitize_snapshot(once.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&once).unwrap(),
            serde_json::to_value(&twice).unwrap()
        );
    }

    #[test]
    fn huge_strings_are_cut_by_what_they_are() {
        let mut snap = rich();
        let id = *snap.terms.keys().next().unwrap();
        let t = snap.terms.get_mut(&id).unwrap();
        t.title = "t".repeat(10 * 1024 * 1024);
        t.said = "line\n".repeat(100_000);
        t.cwd = std::path::PathBuf::from("/".to_owned() + &"d".repeat(1_000_000));
        let t = sanitize_snapshot(snap).unwrap().terms.remove(&id).unwrap();
        assert_eq!(t.title.chars().count(), LINE_MAX);
        assert_eq!(t.said.chars().count(), TEXT_MAX);
        assert!(t.said.contains('\n'), "a text block keeps its lines");
        assert_eq!(t.cwd.to_string_lossy().chars().count(), PATH_MAX);
    }

    #[test]
    fn a_line_field_is_one_line() {
        let mut snap = rich();
        let id = *snap.terms.keys().next().unwrap();
        snap.terms.get_mut(&id).unwrap().name = "two\nlines".into();
        let t = sanitize_snapshot(snap).unwrap().terms.remove(&id).unwrap();
        assert_eq!(t.name, "two lines");
    }

    #[test]
    fn bad_paths_stay_text_and_lose_only_hostile_characters() {
        let mut snap = rich();
        let id = *snap.terms.keys().next().unwrap();
        let t = snap.terms.get_mut(&id).unwrap();
        t.cwd = r"..\..\x".into();
        t.root = Some("/etc/passwd\u{1b}[31m".into());
        t.top = Some(r"\\server\share\..\..\x".into());
        let t = sanitize_snapshot(snap).unwrap().terms.remove(&id).unwrap();
        assert_eq!(t.cwd.to_string_lossy(), r"..\..\x");
        assert_eq!(t.root.unwrap().to_string_lossy(), "/etc/passwd[31m");
        assert_eq!(t.top.unwrap().to_string_lossy(), r"\\server\share\..\..\x");
    }

    #[test]
    fn lists_are_capped() {
        let snap = Snapshot {
            questions: (0..ITEMS_MAX as u64 + 50)
                .map(|id| HumanQuestion {
                    id,
                    term: 1,
                    text: String::new(),
                    options: Vec::new(),
                })
                .collect(),
            ..Snapshot::default()
        };
        let safe = sanitize_snapshot(snap).unwrap();
        assert_eq!(safe.questions.len(), ITEMS_MAX);
    }

    #[test]
    fn a_snapshot_with_too_many_values_in_all_is_refused() {
        let snap = Snapshot {
            questions: (0..ITEMS_MAX as u64)
                .map(|id| HumanQuestion {
                    id,
                    term: 1,
                    text: String::new(),
                    options: vec![String::new(); 30],
                })
                .collect(),
            ..Snapshot::default()
        };
        assert!(sanitize_snapshot(snap).is_err());
    }

    #[test]
    fn sizes_that_drive_layout_are_held_to_a_range() {
        let mut snap = rich();
        let id = *snap.terms.keys().next().unwrap();
        let t = snap.terms.get_mut(&id).unwrap();
        (t.cols, t.rows, t.cpu) = (u16::MAX, 0, 9e9);
        snap.spent_today = -5.0;
        let safe = sanitize_snapshot(snap).unwrap();
        let t = &safe.terms[&id];
        assert_eq!((t.cols, t.rows, t.cpu), (PANE_DIM_MAX, 1, 100.0));
        assert_eq!(safe.spent_today, 0.0);
    }

    #[test]
    fn a_workspace_colour_past_any_palette_is_held_to_a_range() {
        let mut snap = rich();
        snap.workspaces[0].color = u8::MAX;
        snap.workspaces[1].color = 3;
        let safe = sanitize_snapshot(snap).unwrap();
        assert_eq!(safe.workspaces[0].color, WORKSPACE_COLOR_MAX);
        assert_eq!(safe.workspaces[1].color, 3);
    }

    #[test]
    fn a_number_that_is_not_finite_refuses_the_snapshot() {
        let mut snap = rich();
        let id = *snap.terms.keys().next().unwrap();
        snap.terms.get_mut(&id).unwrap().cpu = f32::NAN;
        assert!(sanitize_snapshot(snap).is_err());
    }
}
