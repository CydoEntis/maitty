use std::io::{BufRead, Write};

fn main() {
    if std::env::args().any(|argument| argument == "--version") {
        println!("Claude Code 2.1.312");
        return;
    }

    // Lets a test see which arguments the last message was started with.
    if let Ok(executable) = std::env::current_exe() {
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        let _ = std::fs::write(executable.with_extension("args"), arguments.join("\n"));
    }

    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut output = std::io::stdout().lock();
    let mut line = String::new();
    let scenario = std::env::var("FAKE_CLAUDE_SCENARIO").unwrap_or_default();

    input.read_line(&mut line).unwrap();
    assert_eq!(
        line.trim_end(),
        r#"{"request":{"hooks":null,"subtype":"initialize"},"request_id":"claude-initialize","type":"control_request"}"#
    );
    let session_id = std::env::args()
        .skip_while(|argument| argument != "--session-id")
        .nth(1)
        .or_else(|| {
            std::env::args()
                .find_map(|argument| argument.strip_prefix("--resume=").map(str::to_owned))
        })
        .expect("Claude session identity argument");
    writeln!(
        output,
        "{{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"{session_id}\"}}"
    )
    .unwrap();
    writeln!(
        output,
        "{}",
        r#"{"response":{"request_id":"claude-initialize","subtype":"success"},"type":"control_response"}"#
    )
    .unwrap();
    output.flush().unwrap();

    line.clear();
    input.read_line(&mut line).unwrap();
    assert_eq!(
        line.trim_end(),
        r#"{"message":{"content":[{"text":"hello fake Claude","type":"text"}],"role":"user"},"type":"user"}"#
    );

    if scenario == "approval" || scenario == "approval-decline" {
        for frame in [
            r#"{"isReplay":true,"message":{"content":[{"text":"hello fake Claude","type":"text"}]},"type":"user"}"#,
            r#"{"event":{"message":{"id":"approval-message","model":"fake-model"},"type":"message_start"},"type":"stream_event"}"#,
            r#"{"request_id":"tool-request","request":{"input":{"command":"echo hello"},"subtype":"can_use_tool","tool_name":"Bash"},"type":"control_request"}"#,
        ] {
            writeln!(output, "{frame}").unwrap();
        }
        output.flush().unwrap();

        line.clear();
        input.read_line(&mut line).unwrap();
        let expected_response = match scenario.as_str() {
            "approval" => {
                r#"{"response":{"request_id":"tool-request","response":{"behavior":"allow","updatedInput":{"command":"echo hello"}},"subtype":"success"},"type":"control_response"}"#
            }
            "approval-decline" => {
                r#"{"response":{"request_id":"tool-request","response":{"behavior":"deny","message":"The user declined this action.","updatedInput":{"command":"echo hello"}},"subtype":"success"},"type":"control_response"}"#
            }
            _ => unreachable!(),
        };
        assert_eq!(line.trim_end(), expected_response);
        writeln!(output, "{}", r#"{"is_error":false,"result":"tool decision processed","session_id":"fixture-session","type":"result"}"#).unwrap();
        output.flush().unwrap();
        return;
    }

    // An Edit call as Claude streams it: the tool block with its input, then the permission
    // request naming the same tool use, then the result.
    if scenario == "tool-edit" {
        for frame in [
            r#"{"isReplay":true,"message":{"content":[{"text":"hello fake Claude","type":"text"}]},"type":"user"}"#,
            r#"{"event":{"message":{"id":"edit-message","model":"fake-model"},"type":"message_start"},"type":"stream_event"}"#,
            r#"{"event":{"content_block":{"id":"toolu_edit","input":{},"name":"Edit","type":"tool_use"},"index":1,"type":"content_block_start"},"type":"stream_event"}"#,
            r#"{"event":{"delta":{"partial_json":"{\"file_path\":\"src/a.rs\",\"old_string\":\"old\",\"new_string\":\"new\"}","type":"input_json_delta"},"index":1,"type":"content_block_delta"},"type":"stream_event"}"#,
            r#"{"event":{"index":1,"type":"content_block_stop"},"type":"stream_event"}"#,
            r#"{"request_id":"edit-request","request":{"input":{"file_path":"src/a.rs","new_string":"new","old_string":"old"},"subtype":"can_use_tool","tool_name":"Edit","tool_use_id":"toolu_edit"},"type":"control_request"}"#,
        ] {
            writeln!(output, "{frame}").unwrap();
        }
        output.flush().unwrap();

        line.clear();
        input.read_line(&mut line).unwrap();
        assert_eq!(
            line.trim_end(),
            r#"{"response":{"request_id":"edit-request","response":{"behavior":"allow","updatedInput":{"file_path":"src/a.rs","new_string":"new","old_string":"old"}},"subtype":"success"},"type":"control_response"}"#
        );
        for frame in [
            r#"{"message":{"content":[{"content":"edited","tool_use_id":"toolu_edit","type":"tool_result"}]},"type":"user"}"#,
            r#"{"is_error":false,"result":"done","session_id":"fixture-session","type":"result"}"#,
        ] {
            writeln!(output, "{frame}").unwrap();
        }
        output.flush().unwrap();
        return;
    }

    // A Write of a .env and a command that echoes a token. The agent must get its own input back
    // untouched (`updatedInput`); only Maitty's stored copy is masked. The token is built from
    // pieces so no secret-shaped literal sits in the source.
    if scenario == "tool-secret" {
        let token = format!("{}{}", "ghp_", "aB3d".repeat(9));
        let write_input = r#"{"content":"DB_PASSWORD=hunter2hunter2\n","file_path":"app/.env"}"#;
        for frame in [
            r#"{"isReplay":true,"message":{"content":[{"text":"hello fake Claude","type":"text"}]},"type":"user"}"#.to_owned(),
            r#"{"event":{"message":{"id":"secret-message","model":"fake-model"},"type":"message_start"},"type":"stream_event"}"#.to_owned(),
            r#"{"event":{"content_block":{"id":"toolu_write","input":{},"name":"Write","type":"tool_use"},"index":1,"type":"content_block_start"},"type":"stream_event"}"#.to_owned(),
            r#"{"event":{"delta":{"partial_json":"{\"file_path\":\"app/.env\",\"content\":\"DB_PASSWORD=hunter2hunter2\\n\"}","type":"input_json_delta"},"index":1,"type":"content_block_delta"},"type":"stream_event"}"#.to_owned(),
            r#"{"event":{"index":1,"type":"content_block_stop"},"type":"stream_event"}"#.to_owned(),
            format!(r#"{{"request_id":"write-request","request":{{"input":{write_input},"subtype":"can_use_tool","tool_name":"Write","tool_use_id":"toolu_write"}},"type":"control_request"}}"#),
        ] {
            writeln!(output, "{frame}").unwrap();
        }
        output.flush().unwrap();

        line.clear();
        input.read_line(&mut line).unwrap();
        assert!(
            line.contains("hunter2hunter2"),
            "the agent gets its own input back unchanged: {line}"
        );
        for frame in [
            r#"{"message":{"content":[{"content":"created","tool_use_id":"toolu_write","type":"tool_result"}]},"type":"user"}"#.to_owned(),
            r#"{"event":{"content_block":{"id":"toolu_echo","input":{},"name":"Bash","type":"tool_use"},"index":2,"type":"content_block_start"},"type":"stream_event"}"#.to_owned(),
            r#"{"event":{"delta":{"partial_json":"{\"command\":\"echo $TOKEN\"}","type":"input_json_delta"},"index":2,"type":"content_block_delta"},"type":"stream_event"}"#.to_owned(),
            r#"{"event":{"index":2,"type":"content_block_stop"},"type":"stream_event"}"#.to_owned(),
            format!(r#"{{"message":{{"content":[{{"content":"token is {token}","tool_use_id":"toolu_echo","type":"tool_result"}}]}},"type":"user"}}"#),
            r#"{"is_error":false,"result":"done","session_id":"fixture-session","type":"result"}"#.to_owned(),
        ] {
            writeln!(output, "{frame}").unwrap();
        }
        output.flush().unwrap();
        return;
    }

    if scenario == "interrupt" {
        writeln!(output, "{}", r#"{"isReplay":true,"message":{"content":[{"text":"hello fake Claude","type":"text"}]},"type":"user"}"#).unwrap();
        output.flush().unwrap();

        line.clear();
        input.read_line(&mut line).unwrap();
        assert_eq!(
            line.trim_end(),
            r#"{"request":{"subtype":"interrupt"},"request_id":"interrupt-4","type":"control_request"}"#
        );
        writeln!(output, "{}", r#"{"response":{"request_id":"interrupt-4","subtype":"success"},"type":"control_response"}"#).unwrap();
        writeln!(output, "{}", r#"{"is_error":false,"result":"interrupted","session_id":"fixture-session","type":"result"}"#).unwrap();
        output.flush().unwrap();
        return;
    }

    // The result names the session the daemon asked for, so a conversation it created itself (with its
    // own session id) reads back as well as the fixed one.
    let result = format!(
        r#"{{"is_error":false,"result":"fake reply","session_id":"{session_id}","type":"result"}}"#
    );
    for frame in [
        r#"{"isReplay":true,"message":{"content":[{"text":"hello fake Claude","type":"text"}]},"type":"user"}"#,
        r#"{"event":{"message":{"id":"fake-message","model":"fake-model"},"type":"message_start"},"type":"stream_event"}"#,
        r#"{"event":{"delta":{"text":"fake reply","type":"text_delta"},"index":0,"type":"content_block_delta"},"type":"stream_event"}"#,
        result.as_str(),
    ] {
        writeln!(output, "{frame}").unwrap();
    }
    output.flush().unwrap();
}
