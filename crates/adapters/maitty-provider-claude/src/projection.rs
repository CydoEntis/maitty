use crate::{OutputFrame, bounded_content_text};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, PartialEq)]
pub enum ProjectedEvent {
    Notification { method: String, params: Value },
    Acknowledged { result: Value },
    ControlRequest { request_id: String, request: Value },
}

struct ProjectedTool {
    id: String,
    name: String,
    initial_input: Value,
    /// The complete input, known once the tool block ends; kept for the completion event.
    input: Value,
    partial_input: String,
    command: String,
}

pub struct TurnProjector {
    thread_id: String,
    turn_id: String,
    submitted_text: String,
    user_item_id: String,
    /// When this turn began (Unix ms). Claude reports no times, so the daemon-side clock stands in.
    started_at_ms: u64,
    acknowledged: bool,
    active_message_id: Option<String>,
    assistant_text: BTreeMap<String, String>,
    started_messages: BTreeSet<String>,
    tools_by_block: BTreeMap<u64, ProjectedTool>,
    tools_by_id: BTreeMap<String, ProjectedTool>,
}

impl TurnProjector {
    pub fn new(
        thread_id: String,
        turn_id: String,
        submitted_text: String,
        user_item_id: String,
    ) -> Self {
        Self {
            thread_id,
            turn_id,
            submitted_text,
            user_item_id,
            started_at_ms: now_ms(),
            acknowledged: false,
            active_message_id: None,
            assistant_text: BTreeMap::new(),
            started_messages: BTreeSet::new(),
            tools_by_block: BTreeMap::new(),
            tools_by_id: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub fn project(&mut self, frame: OutputFrame) -> Result<Vec<ProjectedEvent>> {
        let mut events = Vec::new();
        match frame {
            OutputFrame::UserReplay { text } if !self.acknowledged => {
                anyhow::ensure!(
                    text == self.submitted_text,
                    "Claude replayed a different user message"
                );
                events.push(self.notification(
                    "turn/started",
                    serde_json::json!({"turn":{"id":self.turn_id,"status":"inProgress"}}),
                ));
                let user = serde_json::json!({
                    "id": self.user_item_id,
                    "type": "userMessage",
                    "content": [{"type":"text","text":text}]
                });
                events.push(self.notification("item/started", serde_json::json!({"item":user})));
                events.push(self.notification("item/completed", serde_json::json!({"item":user})));
                events.push(ProjectedEvent::Acknowledged {
                    result: serde_json::json!({"turn":{"id":self.turn_id,"status":"inProgress","items":[]}}),
                });
                self.acknowledged = true;
            }
            OutputFrame::UserReplay { .. } => {}
            OutputFrame::MessageStarted { message_id, .. } => {
                self.active_message_id = Some(message_id);
            }
            OutputFrame::TextDelta { text, .. } => {
                let message_id = self
                    .active_message_id
                    .clone()
                    .context("Claude text arrived outside a message")?;
                if self.started_messages.insert(message_id.clone()) {
                    events.push(self.notification(
                        "item/started",
                        serde_json::json!({"item":{"id":message_id,"type":"agentMessage","text":""}}),
                    ));
                }
                self.assistant_text
                    .entry(message_id.clone())
                    .or_default()
                    .push_str(&text);
                events.push(self.notification(
                    "item/agentMessage/delta",
                    serde_json::json!({"itemId":message_id,"delta":text}),
                ));
            }
            OutputFrame::ToolUse {
                block_index,
                id,
                name,
                input,
            } => {
                if self.tools_by_block.len() >= MAX_PENDING_TOOLS {
                    bail!("Claude emitted too many unfinished tool calls");
                }
                self.tools_by_block.insert(
                    block_index,
                    ProjectedTool {
                        id,
                        name,
                        initial_input: input,
                        input: Value::Null,
                        partial_input: String::new(),
                        command: String::new(),
                    },
                );
            }
            OutputFrame::ToolInputDelta {
                block_index,
                partial_json,
            } => {
                let tool = self
                    .tools_by_block
                    .get_mut(&block_index)
                    .context("Claude tool input delta has no matching tool call")?;
                let size = tool
                    .partial_input
                    .len()
                    .checked_add(partial_json.len())
                    .filter(|size| *size <= MAX_TOOL_INPUT_BYTES)
                    .context("Claude tool input exceeds size limit")?;
                tool.partial_input.reserve(size - tool.partial_input.len());
                tool.partial_input.push_str(&partial_json);
            }
            OutputFrame::ContentBlockEnded { block_index } => {
                if let Some(mut tool) = self.tools_by_block.remove(&block_index) {
                    let input = if tool.partial_input.is_empty() {
                        tool.initial_input.clone()
                    } else {
                        serde_json::from_str(&tool.partial_input)
                            .context("Claude tool input is invalid JSON")?
                    };
                    tool.input = input.clone();
                    tool.command = input
                        .get("command")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            format!("{} {}", tool.name, bounded_content_text(Some(&input)))
                        });
                    let item = serde_json::json!({
                        "id":tool.id,
                        "type":"commandExecution",
                        "command":tool.command,
                        "input":tool.input,
                        "status":"inProgress",
                        "aggregatedOutput":""
                    });
                    events
                        .push(self.notification("item/started", serde_json::json!({"item":item})));
                    self.tools_by_id.insert(tool.id.clone(), tool);
                }
            }
            OutputFrame::ToolResults(results) => {
                for result in results {
                    let tool = self
                        .tools_by_id
                        .remove(&result.id)
                        .context("Claude returned a result for an unknown tool call")?;
                    let item = serde_json::json!({
                        "id":result.id,
                        "type":"commandExecution",
                        "command":tool.command,
                        "input":tool.input,
                        "status":if result.is_error {"failed"} else {"completed"},
                        "aggregatedOutput":result.output
                    });
                    events.push(
                        self.notification("item/completed", serde_json::json!({"item":item})),
                    );
                }
            }
            OutputFrame::ControlRequest {
                request_id,
                request,
            } => {
                events.push(ProjectedEvent::ControlRequest {
                    request_id,
                    request,
                });
            }
            OutputFrame::Result {
                session_id,
                is_error,
                result,
            } => {
                anyhow::ensure!(
                    session_id == self.thread_id,
                    "Claude returned a different session identity"
                );
                for (message_id, streamed_text) in std::mem::take(&mut self.assistant_text) {
                    let text = result
                        .as_ref()
                        .filter(|text| !text.is_empty())
                        .unwrap_or(&streamed_text);
                    events.push(self.notification(
                        "item/completed",
                        serde_json::json!({"item":{"id":message_id,"type":"agentMessage","text":text}}),
                    ));
                }
                let status = if is_error { "failed" } else { "completed" };
                events.push(self.notification(
                    "turn/completed",
                    serde_json::json!({"turn":{"id":self.turn_id,"status":status}}),
                ));
            }
            OutputFrame::Initialized { .. }
            | OutputFrame::ControlResponse { .. }
            | OutputFrame::Other => {}
        }
        Ok(events)
    }

    fn notification(&self, method: &str, mut params: Value) -> ProjectedEvent {
        params["threadId"] = Value::String(self.thread_id.clone());
        match method {
            "turn/started" | "turn/completed" => {
                // Codex reports a turn's times in whole seconds; keep the same shape.
                params["turn"]["startedAt"] = (self.started_at_ms / 1000).into();
                if method == "turn/completed" {
                    params["turn"]["completedAt"] = (now_ms() / 1000).into();
                }
            }
            "item/started" => {
                params["turnId"] = Value::String(self.turn_id.clone());
                params["startedAtMs"] = now_ms().into();
            }
            "item/completed" => {
                params["turnId"] = Value::String(self.turn_id.clone());
                params["completedAtMs"] = now_ms().into();
            }
            _ => params["turnId"] = Value::String(self.turn_id.clone()),
        }
        ProjectedEvent::Notification {
            method: method.into(),
            params,
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
        .unwrap_or_default()
}

pub const MAX_PENDING_TOOLS: usize = 64;
const MAX_TOOL_INPUT_BYTES: usize = 64 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn projector() -> TurnProjector {
        TurnProjector::new(
            "session-1".into(),
            "turn-1".into(),
            "show status".into(),
            "user-1".into(),
        )
    }

    #[test]
    fn acknowledges_only_the_exact_submitted_message_once() {
        let mut projector = projector();
        assert!(
            projector
                .project(OutputFrame::UserReplay {
                    text: "different prompt".into()
                })
                .is_err()
        );
        assert!(!projector.acknowledged());

        let events = projector
            .project(OutputFrame::UserReplay {
                text: "show status".into(),
            })
            .unwrap();
        assert!(
            matches!(events.first(), Some(ProjectedEvent::Notification { method, .. }) if method == "turn/started")
        );
        assert!(
            matches!(events.last(), Some(ProjectedEvent::Acknowledged { result }) if result["turn"]["id"] == "turn-1")
        );
        assert!(projector.acknowledged());
        assert!(
            projector
                .project(OutputFrame::UserReplay {
                    text: "show status".into()
                })
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn projects_streamed_assistant_text_into_history_notifications() {
        let mut projector = projector();
        projector
            .project(OutputFrame::MessageStarted {
                message_id: "message-1".into(),
                model: "claude-sonnet".into(),
            })
            .unwrap();
        let started = projector
            .project(OutputFrame::TextDelta {
                block_index: 0,
                text: "hello".into(),
            })
            .unwrap();
        assert!(
            matches!(started.first(), Some(ProjectedEvent::Notification { method, .. }) if method == "item/started")
        );
        assert!(
            matches!(started.last(), Some(ProjectedEvent::Notification { method, params }) if method == "item/agentMessage/delta" && params["delta"] == "hello")
        );

        let completed = projector
            .project(OutputFrame::Result {
                session_id: "session-1".into(),
                is_error: false,
                result: Some("hello".into()),
            })
            .unwrap();
        assert!(
            matches!(completed.first(), Some(ProjectedEvent::Notification { method, params }) if method == "item/completed" && params["item"]["text"] == "hello")
        );
        assert!(
            matches!(completed.last(), Some(ProjectedEvent::Notification { method, params }) if method == "turn/completed" && params["turn"]["status"] == "completed")
        );
    }

    #[test]
    fn projects_tool_input_only_after_the_complete_json_arrives() {
        let mut projector = projector();
        projector
            .project(OutputFrame::ToolUse {
                block_index: 2,
                id: "tool-1".into(),
                name: "Bash".into(),
                input: json!({}),
            })
            .unwrap();
        assert!(
            projector
                .project(OutputFrame::ToolInputDelta {
                    block_index: 2,
                    partial_json: "{\"command\":\"pwd\"}".into(),
                })
                .unwrap()
                .is_empty()
        );
        let started = projector
            .project(OutputFrame::ContentBlockEnded { block_index: 2 })
            .unwrap();
        assert!(
            matches!(started.as_slice(), [ProjectedEvent::Notification { method, params }] if method == "item/started" && params["item"]["command"] == "pwd")
        );

        let completed = projector
            .project(OutputFrame::ToolResults(vec![crate::ToolResult {
                id: "tool-1".into(),
                output: "done".into(),
                is_error: false,
            }]))
            .unwrap();
        assert!(
            matches!(completed.as_slice(), [ProjectedEvent::Notification { method, params }] if method == "item/completed" && params["item"]["aggregatedOutput"] == "done")
        );
    }

    #[test]
    fn a_tool_step_carries_its_full_input_and_times_through_start_and_completion() {
        let mut projector = projector();
        projector
            .project(OutputFrame::ToolUse {
                block_index: 0,
                id: "tool-1".into(),
                name: "TodoWrite".into(),
                input: json!({}),
            })
            .unwrap();
        projector
            .project(OutputFrame::ToolInputDelta {
                block_index: 0,
                partial_json: r#"{"todos":[{"content":"ship","status":"pending"}]}"#.into(),
            })
            .unwrap();
        let todos = json!({"todos":[{"content":"ship","status":"pending"}]});
        let started = projector
            .project(OutputFrame::ContentBlockEnded { block_index: 0 })
            .unwrap();
        let completed = projector
            .project(OutputFrame::ToolResults(vec![crate::ToolResult {
                id: "tool-1".into(),
                output: "ok".into(),
                is_error: false,
            }]))
            .unwrap();
        let [ProjectedEvent::Notification { params: begin, .. }] = started.as_slice() else {
            panic!("one start event");
        };
        let [ProjectedEvent::Notification { params: end, .. }] = completed.as_slice() else {
            panic!("one completion event");
        };
        assert_eq!(begin["item"]["input"], todos);
        assert_eq!(end["item"]["input"], todos);
        assert!(begin["startedAtMs"].as_u64().unwrap() > 0);
        assert!(end["completedAtMs"].as_u64().unwrap() >= begin["startedAtMs"].as_u64().unwrap());
    }

    #[test]
    fn turn_events_carry_the_turn_times_in_seconds() {
        let mut projector = projector();
        let started = projector
            .project(OutputFrame::UserReplay {
                text: "show status".into(),
            })
            .unwrap();
        let ProjectedEvent::Notification { params, .. } = &started[0] else {
            panic!("turn start");
        };
        assert!(params["turn"]["startedAt"].as_u64().unwrap() > 0);
        let finished = projector
            .project(OutputFrame::Result {
                session_id: "session-1".into(),
                is_error: false,
                result: None,
            })
            .unwrap();
        let Some(ProjectedEvent::Notification { params, .. }) = finished.last() else {
            panic!("turn completion");
        };
        assert!(params["turn"]["completedAt"].as_u64().unwrap() >= params["turn"]["startedAt"].as_u64().unwrap());
    }
}
