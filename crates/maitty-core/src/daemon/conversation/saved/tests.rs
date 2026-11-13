use super::*;
use std::time::Duration;

fn scratch(label: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "{}-saved-{label}-{}",
        env!("APP_COMMAND"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn project() -> PathBuf {
    PathBuf::from(if cfg!(windows) { r"C:\code\app" } else { "/code/app" })
}

fn write(file: &Path, lines: &[Value], age_secs: u64) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let body: String = lines.iter().map(|line| format!("{line}\n")).collect();
    std::fs::write(file, body).unwrap();
    let when = SystemTime::now() - Duration::from_secs(age_secs);
    std::fs::File::options().write(true).open(file).unwrap().set_modified(when).unwrap();
}

fn claude_user(cwd: &Path, content: Value) -> Value {
    serde_json::json!({"type": "user", "cwd": cwd, "isSidechain": false, "message": {"role": "user", "content": content}})
}

fn listed(found: SavedConversations) -> Vec<(String, String)> {
    let SavedConversations::Listed(found) = found else {
        panic!("expected a list, got {found:?}");
    };
    found.into_iter().map(|saved| (saved.id, saved.first_message)).collect()
}

fn stores(claude: &Path, codex: &Path) -> Stores {
    Stores { claude: Some(claude.to_path_buf()), codex: Some(codex.to_path_buf()) }
}

#[test]
fn claude_lists_the_folders_transcripts_newest_first_with_what_the_person_typed_first() {
    let root = scratch("claude");
    let app = project();
    let dir = root.join(claude_folder_name(&app));
    write(
        &dir.join("older-session.jsonl"),
        &[
            serde_json::json!({"type": "permission-mode", "permissionMode": "default"}),
            serde_json::json!({"type": "user", "cwd": app, "isMeta": true, "message": {"content": "<local-command-caveat>x</local-command-caveat>"}}),
            claude_user(&app, serde_json::json!("<command-name>/clear</command-name>")),
            claude_user(&app, serde_json::json!("<system-reminder>be brief</system-reminder>")),
            claude_user(&app, serde_json::json!([{"type": "tool_result", "content": "ok"}])),
            claude_user(&app, serde_json::json!([{"type": "text", "text": "Add a dark\nmode   toggle"}])),
            claude_user(&app, serde_json::json!("A later message")),
        ],
        60,
    );
    write(&dir.join("newer-session.jsonl"), &[claude_user(&app, serde_json::json!("Fix the \u{1b}[31mlogin\u{7} bug"))], 5);
    // Same folder name on disk, another working folder.
    let twin = PathBuf::from(if cfg!(windows) { r"C:\code-app" } else { "/code-app" });
    write(&dir.join("twin-session.jsonl"), &[claude_user(&twin, serde_json::json!("Not this one"))], 1);
    write(&dir.join("only-commands.jsonl"), &[claude_user(&app, serde_json::json!("<command-name>/model</command-name>"))], 2);
    write(&dir.join("bad id!.jsonl"), &[claude_user(&app, serde_json::json!("Odd name"))], 3);

    let found = listed(list("claude", &app, &stores(&root, &root.join("none"))));

    assert_eq!(
        found,
        [
            ("newer-session".to_string(), "Fix the login bug".to_string()),
            ("older-session".to_string(), "Add a dark mode toggle".to_string()),
        ]
    );
}

fn codex_meta(id: &str, cwd: &Path, source: &str) -> Value {
    serde_json::json!({"type": "session_meta", "payload": {"id": id, "cwd": cwd, "thread_source": source, "base_instructions": {"text": "long"}}})
}

fn codex_user(text: &str) -> Value {
    serde_json::json!({"type": "response_item", "payload": {"type": "message", "role": "user", "content": [{"type": "input_text", "text": text}]}})
}

#[test]
fn codex_lists_the_folders_sessions_skipping_injected_context_other_folders_and_subagents() {
    let root = scratch("codex");
    let app = project();
    let day = |date: &str| root.join(date);
    write(
        &day("2026/11/12").join("rollout-2026-11-12T09-00-00-aaa.jsonl"),
        &[
            codex_meta("aaa", &app, "user"),
            serde_json::json!({"type": "response_item", "payload": {"type": "message", "role": "developer", "content": [{"type": "input_text", "text": "You are Codex"}]}}),
            codex_user("# AGENTS.md instructions for app"),
            codex_user("<environment_context>\n<cwd>x</cwd>"),
            codex_user("Write the release notes"),
        ],
        10,
    );
    write(
        &day("2026/10/30").join("rollout-2026-10-30T09-00-00-bbb.jsonl"),
        &[
            codex_meta("bbb", &app, "user"),
            serde_json::json!({"type": "event_msg", "payload": {"type": "user_message", "message": "Port the parser"}}),
        ],
        100,
    );
    write(&day("2026/11/12").join("rollout-2026-11-12T10-00-00-ccc.jsonl"), &[codex_meta("ccc", &app, "subagent"), codex_user("Helper task")], 1);
    let elsewhere = PathBuf::from(if cfg!(windows) { r"C:\code\other" } else { "/code/other" });
    write(&day("2026/11/12").join("rollout-2026-11-12T11-00-00-ddd.jsonl"), &[codex_meta("ddd", &elsewhere, "user"), codex_user("Elsewhere")], 1);

    let found = listed(list("codex", &app, &stores(&root.join("none"), &root)));

    assert_eq!(
        found,
        [
            ("aaa".to_string(), "Write the release notes".to_string()),
            ("bbb".to_string(), "Port the parser".to_string()),
        ]
    );
}

#[test]
fn at_most_twenty_are_listed_and_a_long_first_message_is_cut() {
    let root = scratch("many");
    let app = project();
    let dir = root.join(claude_folder_name(&app));
    for n in 0..25 {
        write(&dir.join(format!("s{n:02}.jsonl")), &[claude_user(&app, serde_json::json!("word ".repeat(100)))], n);
    }

    let found = listed(list("claude", &app, &stores(&root, &root)));

    assert_eq!(found.len(), SAVED_CONVERSATIONS_MAX);
    assert_eq!(found[0].0, "s00", "newest first");
    assert!(found.iter().all(|(_, said)| said.chars().count() == SAVED_FIRST_MESSAGE_MAX_CHARS));
}

#[test]
fn a_folder_with_none_is_an_empty_list_and_another_agent_is_not_supported() {
    let root = scratch("none");
    let app = project();
    assert_eq!(list("claude", &app, &stores(&root, &root)), SavedConversations::Listed(Vec::new()));
    assert_eq!(list("Codex", &app, &stores(&root.join("gone"), &root.join("gone"))), SavedConversations::Listed(Vec::new()));
    assert_eq!(list("claude", &app, &Stores { claude: None, codex: None }), SavedConversations::Listed(Vec::new()));
    for agent in ["gemini", "opencode", ""] {
        assert_eq!(list(agent, &app, &stores(&root, &root)), SavedConversations::NotSupported);
    }
}

#[test]
fn a_file_that_cannot_be_read_is_skipped() {
    let root = scratch("unreadable");
    assert_eq!(read_or_log(&root.join("missing.jsonl"), HEAD_MAX_BYTES), None);
}

#[test]
fn claude_names_a_working_folder_as_it_does_on_disk() {
    assert_eq!(claude_folder_name(Path::new(r"C:\dev\my_app.v2\")), "C--dev-my-app-v2");
    assert_eq!(claude_folder_name(Path::new("/home/me/code")), "-home-me-code");
}
