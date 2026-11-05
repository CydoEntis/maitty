use super::*;
use serde_json::json;

// Fake secrets are built from fragments so that no real-looking secret sits in the source and no
// scanner trips on this file. Each has the shape and length of the real thing.
fn rep(unit: &str, count: usize) -> String {
    unit.repeat(count)
}
fn github_token() -> String {
    format!("{}{}", "ghp_", rep("aB3d", 9))
}
fn github_fine_grained() -> String {
    format!("{}{}", "github_pat_", rep("11AbC0_x", 4))
}
fn aws_access_key() -> String {
    format!("{}{}", "AKIA", "IOSFODNN7EXAMPLE")
}
fn aws_secret() -> String {
    format!("{}{}", "wJalrXUtnFEMI/K7MDENG", "/bPxRfiCYEXAMPLEKEY")
}
fn anthropic_key() -> String {
    format!("{}{}", "sk-ant-", "api03-Qw3rTy7UiOp9AsDf2GhJk")
}
fn openai_key() -> String {
    format!("{}{}", "sk-", rep("Ab1Cd2Ef", 6))
}
fn openai_project_key() -> String {
    format!("{}{}", "sk-proj-", "Zx9Cv8Bn7Mq6Wl5Er4Ty3Ui2")
}
fn slack_token() -> String {
    format!("{}{}", "xoxb-", "1234567890-0987654321-AbCdEfGh")
}
fn google_key() -> String {
    format!("{}{}", "AIza", "SyA1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q")
}
fn stripe_key() -> String {
    format!("{}{}", "sk_live_", "4eC39HqLyjWDarjtT1zdp7dc")
}
fn jwt() -> String {
    format!(
        "{}.{}.{}",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        "eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4ifQ",
        "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
    )
}
fn pem_block() -> String {
    let begin = format!("-----BEGIN {} KEY-----", "RSA PRIVATE");
    let end = format!("-----END {} KEY-----", "RSA PRIVATE");
    format!("{begin}\nMIIEowIBAAKCAQEA0Z3VS5JJcds3xfn\nabcdEFGH1234\n{end}")
}

fn masked(text: &str) -> String {
    redact_text(text).into_owned()
}

#[test]
fn well_known_token_shapes_are_masked_and_their_surroundings_kept() {
    let cases: Vec<(&str, String)> = vec![
        ("github classic", github_token()),
        ("github fine-grained", github_fine_grained()),
        ("aws access key", aws_access_key()),
        ("anthropic", anthropic_key()),
        ("openai", openai_key()),
        ("openai project", openai_project_key()),
        ("slack", slack_token()),
        ("google", google_key()),
        ("stripe", stripe_key()),
        ("jwt", jwt()),
    ];
    for (label, secret) in cases {
        let text = format!("before {secret} after");
        assert_eq!(masked(&text), "before [redacted] after", "{label}");
        let in_command = format!("curl -s -d 'k={secret}' https://example.test");
        assert!(
            !masked(&in_command).contains(&secret),
            "{label} survived inside a command"
        );
    }
}

#[test]
fn a_private_key_block_is_masked_through_its_end_line() {
    let text = format!("key:\n{}\nthen more text", pem_block());
    assert_eq!(masked(&text), "key:\n[redacted private key]\nthen more text");
}

#[test]
fn a_private_key_block_that_is_cut_or_still_streaming_is_masked_to_the_end() {
    let begin = format!("-----BEGIN {} KEY-----", "OPENSSH PRIVATE");
    let text = format!("echo '{begin}\nb3BlbnNzaC1rZXktdjEAAAAA\nAAAABG5vbmU");
    let out = masked(&text);
    assert!(out.starts_with("echo '[redacted private key]"));
    assert!(!out.contains("b3Blbn"));
}

#[test]
fn authorization_headers_keep_the_scheme_and_lose_the_credential() {
    let token = "Zm9vOmJhcjEyMzQ1Njc4OQ";
    for (input, expected) in [
        (
            format!("Authorization: Bearer {token}"),
            "Authorization: Bearer [redacted]",
        ),
        (
            format!("-H \"authorization: Basic {token}\""),
            "-H \"authorization: Basic [redacted]\"",
        ),
        (
            format!("curl -H 'Authorization: token {}'", github_token()),
            "curl -H 'Authorization: token [redacted]'",
        ),
        (
            format!("sent bearer {}", rep("abcd1234", 4)),
            "sent bearer [redacted]",
        ),
    ] {
        assert_eq!(masked(&input), expected);
    }
}

#[test]
fn a_password_inside_a_url_is_masked_and_the_user_and_host_are_kept() {
    for (input, expected) in [
        (
            "postgres://app:hunter2pw@db.internal:5432/app",
            "postgres://app:[redacted]@db.internal:5432/app",
        ),
        (
            "git clone https://me:tok3n_value@example.test/r.git",
            "git clone https://me:[redacted]@example.test/r.git",
        ),
    ] {
        assert_eq!(masked(input), expected);
    }
}

#[test]
fn secret_named_assignments_lose_only_the_value() {
    let aws = aws_secret();
    let cases: Vec<(String, String)> = vec![
        ("API_KEY=abc123def456".into(), "API_KEY=[redacted]".into()),
        ("export GITHUB_TOKEN=plainvalue".into(), "export GITHUB_TOKEN=[redacted]".into()),
        ("DB_PASSWORD=correcthorse".into(), "DB_PASSWORD=[redacted]".into()),
        ("password: hunter2hunter2".into(), "password: [redacted]".into()),
        ("client_secret = \"s3cr3t value\"".into(), "client_secret = \"[redacted]\"".into()),
        ("{\"api_key\": \"abc123def456\"}".into(), "{\"api_key\": \"[redacted]\"}".into()),
        ("{\\\"password\\\":\\\"hunter2hunter2\\\"}".into(), "{\\\"password\\\":\\\"[redacted]\\\"}".into()),
        ("passwd='p4ss word'".into(), "passwd='[redacted]'".into()),
        (
            format!("aws_secret_access_key = {aws}"),
            "aws_secret_access_key = [redacted]".into(),
        ),
        ("PRIVATE_KEY=abc123def".into(), "PRIVATE_KEY=[redacted]".into()),
        ("AWS_CREDENTIAL=abcdef12".into(), "AWS_CREDENTIAL=[redacted]".into()),
        ("OAUTH_TOKEN=abc".repeat(1) + "defgh", "OAUTH_TOKEN=[redacted]".into()),
        ("curl 'https://x.test/a?id=1&token=abc123def'".into(), "curl 'https://x.test/a?id=1&token=[redacted]'".into()),
        ("githubToken: 'abc123xyz789'".into(), "githubToken: '[redacted]'".into()),
    ];
    for (input, expected) in cases {
        assert_eq!(masked(&input), expected, "input {input:?}");
    }
}

#[test]
fn masking_is_idempotent() {
    let text = format!(
        "{} {} Authorization: Bearer {} DB_PASSWORD=hunter2hunter2 postgres://a:b1b2b3b4@h/\n{}",
        github_token(),
        jwt(),
        rep("abcd1234", 4),
        pem_block()
    );
    let once = masked(&text);
    assert_eq!(masked(&once), once);
}

#[test]
fn ordinary_text_is_not_masked() {
    let git_sha = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3";
    let uuid = "550e8400-e29b-41d4-a716-446655440000";
    let base64 = "TWFueSBoYW5kcyBtYWtlIGxpZ2h0IHdvcmsuIFRoaXMgaXMgbm90IGEgc2VjcmV0Lg==";
    let must_not_mask = [
        "Here is a normal sentence about tokens and passwords.",
        "The token budget is 4096 and max_tokens=4096 per request.",
        "password: required",
        "token: the next one",
        "password = user.password",
        "token = refresh_token",
        "password=$DB_PASSWORD",
        "TOKEN=${GITHUB_TOKEN}",
        "api_key: <your-api-key>",
        "secret=********",
        "auth_url=https://example.test/login",
        "token_file=/etc/service/token",
        "author: Jane Doe-Smith",
        "authorization: true",
        "const token = getToken();",
        "let secret = process.env.SECRET_VALUE;",
        "Visit https://example.test:8080/path for the docs.",
        "ssh://git@github.com/org/repo.git",
        "the sk-learn-compatible-estimators page",
        "task-force-ghp_short",
        "bearer of bad news and other things",
        "commit a94a8fe5 fixed it",
        "- key: value",
        "12:34:56 started",
        git_sha,
        uuid,
        base64,
        "src/main.rs:42:7: error",
        "eyJ is the start of base64 JSON",
        "{\"tokens\": 120, \"tokenizer\": \"bpe\", \"author\": \"someone\"}",
    ];
    for text in must_not_mask {
        assert_eq!(masked(text), text, "{text:?} must stay as written");
    }
}

#[test]
fn a_string_leaf_in_a_tool_input_is_masked_and_a_secret_named_key_loses_its_value() {
    let mut input = json!({
        "command": format!("echo {} > out.txt", github_token()),
        "env": { "API_KEY": "abc123def456", "MODE": "fast", "retries": 3, "TOKEN": true },
        "list": [format!("x {}", aws_access_key())],
    });
    assert!(redact_input(&mut input));
    assert_eq!(input["command"], "echo [redacted] > out.txt");
    assert_eq!(input["env"]["API_KEY"], MASK);
    assert_eq!(input["env"]["MODE"], "fast");
    assert_eq!(input["env"]["retries"], 3);
    assert_eq!(input["env"]["TOKEN"], true);
    assert_eq!(input["list"][0], "x [redacted]");
    assert!(!redact_input(&mut input), "a second pass changes nothing");
}

#[test]
fn secret_paths_are_recognised() {
    for path in [
        ".env",
        "app/.env",
        "/home/me/project/.env.local",
        ".env.production",
        "C:\\work\\app\\.ENV",
        "certs/server.pem",
        "tls/private.key",
        "id_rsa",
        "/home/me/.ssh/id_ed25519",
        "store.p12",
        "store.pfx",
        "~/.npmrc",
        ".pypirc",
        "/home/me/.netrc",
        "creds/credentials",
        "/home/me/.git-credentials",
        "config/secrets.yml",
        "secrets.json",
        "vault.kdbx",
        "/home/me/.aws/config",
        "C:\\Users\\me\\.ssh\\config",
        "/home/me/.gnupg/pubring.kbx",
    ] {
        assert!(is_secret_path(path), "{path}");
    }
    for path in [
        "",
        ".env.example",
        "app/.env.sample",
        ".env.template",
        "src/credentials.rs",
        "src/secrets_manager.rs",
        "README.md",
        "src/environment.ts",
        "keys.md",
        "monkey",
        ".envrc",
        "docs/aws/overview.md",
        "key",
        ".key",
        "src/main.rs",
    ] {
        assert!(!is_secret_path(path), "{path:?}");
    }
}

#[test]
fn a_write_to_a_secret_file_keeps_its_path_and_loses_its_text() {
    let mut input = json!({ "file_path": "app/.env", "content": "DB_PASSWORD=abc\nMODE=dev\n" });
    assert!(omit_secret_file_content(&mut input));
    assert_eq!(
        input,
        json!({ "file_path": "app/.env", "content": OMITTED, "omitted_bytes": 25 })
    );
    assert!(!omit_secret_file_content(&mut input), "idempotent");
    assert_eq!(input["omitted_bytes"], 25);
}

#[test]
fn edits_to_a_secret_file_lose_every_text_field_and_keep_their_count() {
    let mut input = json!({
        "file_path": "/home/me/.ssh/config",
        "old_string": "Host a",
        "new_string": "Host b",
        "replace_all": true,
        "edits": [
            { "old_string": "x", "new_string": "y" },
            { "old": "p", "new": "q" },
        ],
    });
    assert!(omit_secret_file_content(&mut input));
    assert_eq!(input["file_path"], "/home/me/.ssh/config");
    assert_eq!(input["old_string"], OMITTED);
    assert_eq!(input["new_string"], OMITTED);
    assert_eq!(input["replace_all"], true);
    assert_eq!(input["edits"].as_array().unwrap().len(), 2);
    assert_eq!(input["edits"][1]["new"], OMITTED);
}

#[test]
fn an_edit_of_an_ordinary_file_keeps_its_text() {
    let mut input = json!({ "file_path": "src/main.rs", "content": "fn main() {}" });
    assert!(!omit_secret_file_content(&mut input));
    assert_eq!(input["content"], "fn main() {}");
    assert!(input.get("omitted_bytes").is_none());
}

#[test]
fn a_diff_of_a_secret_file_is_omitted_and_any_other_diff_is_masked() {
    assert_eq!(redact_diff("a/.env", "+KEY=1"), OMITTED);
    let diff = format!("+let k = \"{}\";", github_token());
    assert_eq!(redact_diff("src/a.rs", &diff), "+let k = \"[redacted]\";");
}

fn tool(command: &str, output: &str) -> Entry {
    use super::super::history::{ItemState, SourceId};
    Entry {
        source: SourceId {
            conversation: crate::conversation::ConversationId(1),
            turn: "t".into(),
            item: "i".into(),
        },
        content: Content::Tool {
            kind: "commandExecution".into(),
            output: output.into(),
            label: command.into(),
            input: Some(json!({ "command": command })),
            exit_code: None,
            duration_ms: None,
        },
        state: ItemState::Completed,
        started_at_ms: None,
        completed_at_ms: None,
    }
}

#[test]
fn command_output_that_echoes_a_token_is_masked_in_an_entry() {
    let mut entry = tool("echo $T", &format!("token is {}", github_token()));
    redact_entry(&mut entry);
    let Content::Tool { output, .. } = &entry.content else {
        unreachable!()
    };
    assert_eq!(output, "token is [redacted]");
}

#[test]
fn reading_a_secret_file_with_a_shell_command_omits_the_output_only() {
    for command in ["cat .env", "tail -n 5 ~/.ssh/id_rsa", "grep KEY 'app/.env.local'"] {
        let mut entry = tool(command, "KEY=value\nOTHER=x");
        redact_entry(&mut entry);
        let Content::Tool { output, label, .. } = &entry.content else {
            unreachable!()
        };
        assert_eq!(output, OMITTED, "{command}");
        assert_eq!(label, command);
    }
    for command in ["git add .env", "ls ~/.ssh", "cat README.md", "echo .env"] {
        let mut entry = tool(command, "plain output");
        redact_entry(&mut entry);
        let Content::Tool { output, .. } = &entry.content else {
            unreachable!()
        };
        assert_eq!(output, "plain output", "{command}");
    }
}

#[test]
fn a_message_is_masked() {
    use super::super::history::{ItemState, Role, SourceId};
    let mut entry = Entry {
        source: SourceId {
            conversation: crate::conversation::ConversationId(1),
            turn: "t".into(),
            item: "i".into(),
        },
        content: Content::Message {
            role: Role::Assistant,
            text: format!("use {} here", stripe_key()),
        },
        state: ItemState::Completed,
        started_at_ms: None,
        completed_at_ms: None,
    };
    redact_entry(&mut entry);
    assert_eq!(
        entry.content,
        Content::Message {
            role: Role::Assistant,
            text: "use [redacted] here".into()
        }
    );
}
