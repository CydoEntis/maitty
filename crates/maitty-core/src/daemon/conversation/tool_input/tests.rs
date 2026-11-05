use super::*;
use serde_json::json;

#[test]
fn clean_text_removes_what_would_move_the_terminal_or_hide_text() {
    let cases = [
        ("plain text", "plain text"),
        ("keeps\nlines\tand tabs", "keeps\nlines\tand tabs"),
        ("\u{1b}[31mred\u{1b}[0m", "red"),
        ("\u{1b}]0;window title\u{7}after", "after"),
        ("\u{1b}]8;;http://x\u{1b}\\link\u{1b}]8;;\u{1b}\\", "link"),
        ("bell\u{7} and nul\u{0} and cr\r", "bell and nul and cr"),
        ("c1\u{9b}31m controls", "c131m controls"),
        ("zero\u{200b}width", "zero\\u{200b}width"),
        ("\u{202e}reversed", "\\u{202e}reversed"),
        ("café 日本語 ✓", "café 日本語 ✓"),
    ];
    for (input, expected) in cases {
        assert_eq!(clean_text(input), expected, "input {input:?}");
    }
}

#[test]
fn a_string_over_the_budget_is_cut_on_a_character_boundary_with_a_marker() {
    let text = "é".repeat(STRING_BUDGETS[0]);
    let bounded = bound_input(&json!({ "content": text })).unwrap();
    let kept = bounded["content"].as_str().unwrap();
    assert!(kept.ends_with(TRUNCATED));
    assert!(kept.len() <= STRING_BUDGETS[0] + TRUNCATED.len());
}

#[test]
fn input_that_is_too_big_is_cut_harder_until_it_fits_and_keeps_its_shape() {
    let edits: Vec<Value> = (0..40)
        .map(|n| json!({ "old_string": "a".repeat(8000), "new_string": format!("b{n}") }))
        .collect();
    let bounded = bound_input(&json!({ "file_path": "src/main.rs", "edits": edits })).unwrap();
    assert!(serde_json::to_vec(&bounded).unwrap().len() <= MAX_INPUT_BYTES);
    assert_eq!(bounded["file_path"], "src/main.rs");
    assert_eq!(bounded["edits"].as_array().unwrap().len(), 40);
    assert_eq!(bounded["edits"][39]["new_string"], "b39");
}

#[test]
fn input_that_cannot_keep_its_shape_is_left_out() {
    let too_many = json!({ "todos": vec![json!({"content": "x"}); MAX_ITEMS + 1] });
    let mut deep = json!("leaf");
    for _ in 0..=MAX_DEPTH {
        deep = json!({ "next": deep });
    }
    // 128 lists of 128 short strings stay within every per-level limit but not the byte cap.
    let wide = json!({ "rows": vec![vec![json!("z".repeat(300)); MAX_ITEMS]; MAX_ITEMS] });
    for input in [too_many, deep, wide] {
        assert_eq!(bound_input(&input), None, "{input}");
    }
}

#[test]
fn empty_input_is_not_kept() {
    assert_eq!(bound_input(&json!({})), None);
    assert_eq!(bound_input(&Value::Null), None);
}

#[test]
fn input_strings_and_keys_are_cleaned() {
    let bounded = bound_input(&json!({ "co\u{1b}[1mmmand": "ls\u{1b}[2J\u{200b}" })).unwrap();
    assert_eq!(bounded, json!({ "command": "ls\\u{200b}" }));
}

#[test]
fn each_tool_keeps_the_input_fields_the_chat_view_reads() {
    let cases = [
        json!({ "file_path": "a.rs", "old_string": "x", "new_string": "y" }),
        json!({ "file_path": "a.rs", "edits": [{ "old_string": "x", "new_string": "y" }] }),
        json!({ "file_path": "a.rs", "content": "fn main() {}\n" }),
        json!({ "todos": [{ "content": "ship", "status": "in_progress", "activeForm": "Shipping" }] }),
        json!({ "description": "Explore", "prompt": "look", "subagent_type": "Explore" }),
    ];
    for input in cases {
        assert_eq!(bound_input(&input), Some(input.clone()));
    }
}

#[test]
fn approval_commands_name_the_target() {
    let cases = [
        ("Bash", json!({ "command": "cargo test" }), "cargo test"),
        (
            "Edit",
            json!({ "file_path": "src/a.rs", "old_string": "x" }),
            "Edit: src/a.rs",
        ),
        (
            "Write",
            json!({ "file_path": "b.txt", "content": "c" }),
            "Write: b.txt",
        ),
        (
            "WebFetch",
            json!({ "url": "https://example.com/x" }),
            "WebFetch: https://example.com/x",
        ),
        (
            "Grep",
            json!({ "pattern": "todo", "path": "src" }),
            "Grep: src",
        ),
        ("Custom", json!({ "weird": 1 }), "Custom: {\"weird\":1}"),
    ];
    for (tool, input, expected) in cases {
        assert_eq!(approval_command(tool, &input), expected, "{tool}");
    }
}

#[test]
fn an_approval_summary_is_one_cleaned_cut_line() {
    let input = json!({ "prompt": format!("first\u{1b}[31m line\nsecond{}", "z".repeat(500)) });
    assert_eq!(input_summary(&input), "first line");
    let long = json!({ "description": "d".repeat(500) });
    assert_eq!(input_summary(&long).chars().count(), MAX_SUMMARY_CHARS);
}

#[test]
fn edit_tools_become_file_change_previews() {
    let cases = [
        (
            "Edit",
            json!({ "file_path": "a.rs", "old_string": "x\nz", "new_string": "y" }),
            json!([{ "path": "a.rs", "kind": "update", "diff": "-x\n-z\n+y" }]),
        ),
        (
            "MultiEdit",
            json!({ "file_path": "a.rs", "edits": [
                { "old_string": "1", "new_string": "2" },
                { "old_string": "", "new_string": "3" },
            ] }),
            json!([{ "path": "a.rs", "kind": "update", "diff": "-1\n+2\n+3" }]),
        ),
        (
            "Write",
            json!({ "file_path": "b.txt", "content": "one\ntwo" }),
            json!([{ "path": "b.txt", "kind": "write", "diff": "+one\n+two" }]),
        ),
    ];
    for (tool, input, expected) in cases {
        assert_eq!(
            file_change_previews(tool, &input),
            expected.as_array().cloned(),
            "{tool}"
        );
    }
}

#[test]
fn other_tools_and_malformed_edits_have_no_preview() {
    assert_eq!(
        file_change_previews("Bash", &json!({ "command": "ls" })),
        None
    );
    assert_eq!(
        file_change_previews("Edit", &json!({ "file_path": "a.rs" })),
        None
    );
    assert_eq!(
        file_change_previews("Write", &json!({ "content": "c" })),
        None
    );
}

#[test]
fn a_preview_diff_is_cut() {
    let input = json!({ "file_path": "big.txt", "content": "line\n".repeat(20_000) });
    let previews = file_change_previews("Write", &input).unwrap();
    let diff = previews[0]["diff"].as_str().unwrap();
    assert!(diff.len() <= MAX_PREVIEW_BYTES + TRUNCATED.len());
    assert!(diff.ends_with(TRUNCATED));
}
