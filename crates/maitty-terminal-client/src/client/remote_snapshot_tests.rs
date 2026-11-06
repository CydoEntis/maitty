#[cfg(test)]
mod tests {
    use crate::protocol::{HumanQuestion, Snapshot};
    use maitty_machines::remote_snapshot::*;
    use serde_json::Value;

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
    fn a_repeat_count_is_held_to_a_range() {
        let mut snap = rich();
        let id = *snap.terms.keys().next().unwrap();
        snap.terms.get_mut(&id).unwrap().repeats = Some(u32::MAX);
        let safe = sanitize_snapshot(snap).unwrap();
        assert_eq!(safe.terms[&id].repeats, Some(REPEATS_MAX));
        let mut modest = rich();
        modest.terms.get_mut(&id).unwrap().repeats = Some(7);
        assert_eq!(sanitize_snapshot(modest).unwrap().terms[&id].repeats, Some(7));
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
