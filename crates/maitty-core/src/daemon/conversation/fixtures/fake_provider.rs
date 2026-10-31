use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};

fn main() {
    let fixture = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let trace_dir = std::env::var_os("CODEX_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| fixture.clone());
    std::fs::write(
        trace_dir.join(format!("args-{}.txt", std::process::id())),
        args.join("\n"),
    )
    .unwrap();
    if args.first().map(String::as_str) == Some("--descendant") {
        loop {
            std::thread::park();
        }
    }
    if args.first().map(String::as_str) == Some("resume") {
        use std::io::Write;
        let mut log = std::fs::File::create(trace_dir.join("native-resume.args")).unwrap();
        for arg in &args {
            writeln!(log, "{arg}").unwrap();
        }
        writeln!(log, "CODEX_HOME={}", std::env::var("CODEX_HOME").unwrap()).unwrap();
        let mode = std::env::var("TEST_MODE")
            .ok()
            .or_else(|| std::fs::read_to_string(fixture.join("mode")).ok());
        if mode.as_deref() != Some("conversation-empty-thread") {
            std::fs::write(trace_dir.join("native-history-written"), "true").unwrap();
        }
        loop {
            std::thread::park();
        }
    }
    if args == ["--version"] {
        println!("codex-cli 0.164.0");
        return;
    }
    if args.get(1).map(String::as_str) == Some("generate-json-schema") {
        let output = std::path::Path::new(&args[3]);
        std::fs::create_dir_all(output).unwrap();
        for name in ["ClientRequest.json", "ServerRequest.json"] {
            std::fs::copy(
                std::path::PathBuf::from(
                    std::env::var_os("TEST_SCHEMA_DIR")
                        .unwrap_or_else(|| fixture.clone().into_os_string()),
                )
                .join(name),
                output.join(name),
            )
            .unwrap();
        }
        return;
    }
    assert_eq!(args, ["app-server", "--listen", "stdio://"]);
    // Start before reading initialize: this exercises the process-creation race.
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--descendant")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::fs::write(
        std::env::var_os("TEST_PID_FILE").unwrap_or_else(|| fixture.join("pids").into_os_string()),
        format!("{} {}", std::process::id(), child.id()),
    )
    .unwrap();
    io::stderr().write_all(&vec![b'x'; 256 * 1024]).unwrap();
    let mut lines = io::stdin().lock().lines();
    let initialize = lines.next().unwrap().unwrap();
    assert!(initialize.contains("\"method\":\"initialize\""));
    assert!(initialize.contains("\"id\":1"));
    let mode = std::env::var("TEST_MODE")
        .unwrap_or_else(|_| std::fs::read_to_string(fixture.join("mode")).unwrap());
    match mode.as_str() {
        "timeout" => loop {
            std::thread::park();
        },
        "malformed" => {
            println!("not-json PRIVATE_SENTINEL");
            io::stdout().flush().unwrap();
        }
        "oversize" => {
            println!("{}", "x".repeat(1024 * 1024 + 1));
            io::stdout().flush().unwrap();
        }
        "closed" => return,
        _ => {
            let home = std::path::PathBuf::from(std::env::var_os("CODEX_HOME").unwrap());
            let home = if mode == "wrong-home" {
                home.join("different")
            } else {
                home
            };
            let home = home
                .to_str()
                .unwrap()
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            let id = if mode == "wrong-id" { "\"1\"" } else { "1" };
            println!(
                "{{\"id\":{id},\"result\":{{\"codexHome\":\"{home}\",\"platformFamily\":\"test\",\"platformOs\":\"test\",\"userAgent\":\"fake\"}}}}"
            );
            io::stdout().flush().unwrap();
        }
    }
    let initialized = lines.next().unwrap().unwrap();
    assert_eq!(initialized, "{\"method\":\"initialized\"}");
    if mode != "unexpected-response" {
        println!(
            "{{\"method\":\"test/ready\",\"params\":{{\"accountEnv\":\"{}\"}}}}",
            std::env::var("TEST_ACCOUNT_VALUE").unwrap_or_else(|_| "synthetic-account".into())
        );
    }
    io::stdout().flush().unwrap();
    let mut turn_number = 0;
    for line in lines {
        let line = line.unwrap();
        if (mode == "conversation-approval"
            || mode == "conversation-file-approval"
            || mode == "conversation-file-approval-no-patch"
            || mode == "conversation-file-approval-malformed"
            || mode == "conversation-permission-approval"
            || mode == "conversation-user-input")
            && line.contains("\"result\":")
        {
            use std::fs::OpenOptions;
            writeln!(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(fixture.join("responses"))
                    .unwrap(),
                "{line}"
            )
            .unwrap();
            println!(
                "{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":\"exact-thread\",\"turn\":{{\"id\":\"turn-1\",\"status\":\"completed\"}}}}}}"
            );
            io::stdout().flush().unwrap();
            continue;
        }
        let request_id = line
            .split("\"id\":")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap();
        if line.contains("\"method\":\"config/read\"") {
            println!("{{\"id\":{request_id},\"result\":{{\"config\":{{}}}}}}");
            if mode == "unexpected-response" {
                println!("{{\"id\":99,\"result\":null}}");
            }
            io::stdout().flush().unwrap();
            continue;
        }
        if !mode.starts_with("conversation") {
            continue;
        }
        if line.contains("\"method\":\"thread/start\"")
            || line.contains("\"method\":\"thread/resume\"")
        {
            if line.contains("missing-thread") {
                println!(
                    "{{\"id\":{request_id},\"error\":{{\"code\":-32001,\"message\":\"Exact thread missing\"}}}}"
                );
            } else {
                let cwd = quoted(&std::env::current_dir().unwrap().to_string_lossy());
                println!(
                    "{{\"id\":{request_id},\"result\":{{\"thread\":{{\"id\":\"exact-thread\",\"sessionId\":\"different-shared-root\"}},\"cwd\":{cwd},\"model\":\"synthetic-model\",\"modelProvider\":\"synthetic-provider\",\"approvalPolicy\":\"on-request\",\"approvalsReviewer\":\"user\",\"sandbox\":{{\"type\":\"readOnly\",\"networkAccess\":false}}}}}}"
                );
            }
        } else if line.contains("\"method\":\"thread/turns/list\"") {
            if fixture.join("native-history-written").is_file() {
                println!(
                    "{{\"id\":{request_id},\"result\":{{\"data\":[{{\"id\":\"native-turn-1\",\"status\":\"completed\",\"itemsView\":\"full\",\"items\":[{{\"id\":\"native-user-1\",\"type\":\"userMessage\",\"content\":[{{\"type\":\"text\",\"text\":\"from native terminal\"}}]}},{{\"id\":\"native-answer-1\",\"type\":\"agentMessage\",\"text\":\"native reply preserved\"}}]}}],\"nextCursor\":null}}}}"
                );
            } else if mode == "conversation-empty-thread" && turn_number == 0 {
                println!("{{\"id\":{request_id},\"error\":{{\"code\":-32600,\"message\":\"thread is not materialized yet; thread/turns/list is unavailable before first user message\"}}}}");
            } else if turn_number == 0 {
                println!("{{\"id\":{request_id},\"result\":{{\"data\":[],\"nextCursor\":null}}}}");
            } else {
                println!(
                    "{{\"id\":{request_id},\"result\":{{\"data\":[{{\"id\":\"turn-{turn_number}\",\"status\":\"completed\",\"itemsView\":\"full\",\"items\":[{{\"id\":\"answer\",\"type\":\"agentMessage\",\"text\":\"Synthetic stream complete\"}}]}}],\"nextCursor\":null}}}}"
                );
            }
        } else if line.contains("\"method\":\"turn/start\"") {
            turn_number += 1;
            if let Some(path) = std::env::var_os("TEST_REQUEST_LOG")
                .or_else(|| Some(fixture.join("requests").into_os_string()))
            {
                use std::fs::OpenOptions;
                writeln!(
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .unwrap(),
                    "turn-{turn_number}"
                )
                .unwrap();
            }
            if mode == "conversation-disconnect" {
                return;
            }
            let turn = format!("turn-{turn_number}");
            println!(
                "{{\"id\":{request_id},\"result\":{{\"turn\":{{\"id\":\"{turn}\",\"items\":[],\"status\":\"inProgress\"}}}}}}"
            );
            if mode == "conversation-approval" {
                println!(
                    "{{\"id\":\"approval-private\",\"method\":\"item/commandExecution/requestApproval\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"cmd-1\",\"command\":\"echo synthetic\\u200b\",\"reason\":\"Synthetic approval\",\"availableDecisions\":[\"accept\",\"decline\"]}}}}"
                );
            } else if mode == "conversation-file-approval" {
                println!(
                    "{{\"method\":\"item/fileChange/patchUpdated\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"file-change-1\",\"changes\":[{{\"path\":\"src/synthetic.rs\",\"kind\":\"update\",\"diff\":\"@@ -1 +1 @@\\n-old\\n+new\\n\"}}]}}}}"
                );
                println!(
                    "{{\"id\":\"file-approval-private\",\"method\":\"item/fileChange/requestApproval\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"file-change-1\",\"grantRoot\":\"C:/synthetic/project\",\"reason\":\"Synthetic file change\"}}}}"
                );
            } else if mode == "conversation-file-approval-no-patch" {
                println!(
                    "{{\"id\":\"file-approval-private\",\"method\":\"item/fileChange/requestApproval\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"file-change-1\",\"grantRoot\":\"C:/synthetic/project\",\"reason\":\"Synthetic file change\"}}}}"
                );
            } else if mode == "conversation-file-approval-malformed" {
                println!(
                    "{{\"method\":\"item/fileChange/patchUpdated\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"file-change-1\",\"changes\":[{{\"path\":\"src/synthetic.rs\",\"kind\":\"update\",\"diff\":\"+valid but incomplete\\n\"}},{{\"path\":\"missing fields\"}}]}}}}"
                );
                println!(
                    "{{\"id\":\"file-approval-private\",\"method\":\"item/fileChange/requestApproval\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"file-change-1\",\"grantRoot\":\"C:/synthetic/project\",\"reason\":\"Synthetic file change\"}}}}"
                );
            } else if mode == "conversation-user-input" {
                println!(
                    "{{\"id\":\"user-input-private\",\"method\":\"item/tool/requestUserInput\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"question-1\",\"isBlocking\":true,\"questions\":[{{\"id\":\"choice\",\"header\":\"Choice\",\"question\":\"Which option?\",\"isOther\":true,\"isSecret\":false,\"options\":[{{\"label\":\"A\",\"description\":\"Choose A\"}},{{\"label\":\"B\",\"description\":\"Choose B\"}}]}},{{\"id\":\"secret\",\"header\":\"Secret\",\"question\":\"Enter private text\",\"isOther\":false,\"isSecret\":true,\"options\":[]}}]}}}}"
                );
            } else if mode == "conversation-permission-approval" {
                println!(
                    "{{\"id\":\"permission-approval-private\",\"method\":\"item/permissions/requestApproval\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"permissions-1\",\"cwd\":\"C:/synthetic/project\",\"reason\":\"Synthetic extra access\",\"permissions\":{{\"network\":{{\"enabled\":true}},\"fileSystem\":{{\"write\":[\"C:/synthetic/project/output\"]}}}}}}}}"
                );
            } else if mode == "conversation-error" {
                println!(
                    "{{\"method\":\"error\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"willRetry\":false,\"error\":{{\"message\":\"Synthetic provider failure\"}}}}}}"
                );
            } else if mode == "conversation-failed" || mode == "conversation-interrupted" {
                let status = if mode == "conversation-failed" {
                    "failed"
                } else {
                    "interrupted"
                };
                println!(
                    "{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":\"exact-thread\",\"turn\":{{\"id\":\"{turn}\",\"status\":\"{status}\",\"error\":{{\"message\":\"Synthetic {status} turn\"}}}}}}}}"
                );
            } else if mode == "conversation-blocked" {
                println!(
                    "{{\"id\":\"approval-private\",\"method\":\"future/unknownApproval\",\"params\":{{\"threadId\":\"exact-thread\"}}}}"
                );
            } else if mode == "conversation-cancel" {
                println!(
                    "{{\"method\":\"turn/started\",\"params\":{{\"threadId\":\"exact-thread\",\"turn\":{{\"id\":\"{turn}\",\"status\":\"inProgress\"}}}}}}"
                );
            } else {
                println!(
                    "{{\"method\":\"turn/started\",\"params\":{{\"threadId\":\"exact-thread\",\"turn\":{{\"id\":\"{turn}\",\"status\":\"inProgress\"}}}}}}"
                );
                println!(
                    "{{\"method\":\"item/agentMessage/delta\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"itemId\":\"answer\",\"delta\":\"Synthetic stream\"}}}}"
                );
                println!(
                    "{{\"method\":\"item/completed\",\"params\":{{\"threadId\":\"exact-thread\",\"turnId\":\"{turn}\",\"item\":{{\"id\":\"answer\",\"type\":\"agentMessage\",\"text\":\"Synthetic stream complete\"}}}}}}"
                );
                println!(
                    "{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":\"exact-thread\",\"turn\":{{\"id\":\"{turn}\",\"status\":\"completed\"}}}}}}"
                );
            }
        } else if line.contains("\"method\":\"turn/interrupt\"") {
            use std::fs::OpenOptions;
            writeln!(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(fixture.join("interrupts"))
                    .unwrap(),
                "{line}"
            )
            .unwrap();
            println!("{{\"id\":{request_id},\"result\":{{}}}}");
            println!(
                "{{\"method\":\"turn/completed\",\"params\":{{\"threadId\":\"exact-thread\",\"turn\":{{\"id\":\"turn-1\",\"status\":\"interrupted\"}}}}}}"
            );
        } else {
            panic!("unexpected synthetic conversation request");
        }
        io::stdout().flush().unwrap();
    }
}

fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
