use std::io::{BufRead, Write};

fn main() {
    if std::env::args().any(|argument| argument == "--version") {
        println!("Claude Code 2.1.312");
        return;
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

    for frame in [
        r#"{"isReplay":true,"message":{"content":[{"text":"hello fake Claude","type":"text"}]},"type":"user"}"#,
        r#"{"event":{"message":{"id":"fake-message","model":"fake-model"},"type":"message_start"},"type":"stream_event"}"#,
        r#"{"event":{"delta":{"text":"fake reply","type":"text_delta"},"index":0,"type":"content_block_delta"},"type":"stream_event"}"#,
        r#"{"is_error":false,"result":"fake reply","session_id":"fixture-session","type":"result"}"#,
    ] {
        writeln!(output, "{frame}").unwrap();
    }
    output.flush().unwrap();
}
