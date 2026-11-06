//! The question a permission prompt asks, as the agent's own hook reported it (ADR 0003).
//!
//! It comes from the owning runtime's `PermissionRequest` hook payload, which reaches the daemon
//! through the same checked path as every status report (AGENTS.md non-negotiable 5), never from
//! the screen (ADR 0003). It is cleaned here before it is kept or sent to any client.

use super::conversation::mask_secrets;
use crate::protocol::{PermissionAsk, Status};
use maitty_shared::hidden;

/// Longest tool name and subject kept for a permission prompt, in characters.
const MAX_ASK_TOOL_CHARS: usize = 64;
pub(super) const MAX_ASK_DETAIL_CHARS: usize = 300;
/// Most choices and longest choice kept for a question prompt; the menu itself holds four.
const MAX_ASK_CHOICES: usize = 4;
const MAX_ASK_CHOICE_CHARS: usize = 80;

/// The tool Claude asks a multiple-choice question with. Its menu is not the allow-or-refuse menu,
/// so the key that allows a tool there would pick the first choice for the person.
const QUESTION_TOOL: &str = "AskUserQuestion";

/// The agents whose permission prompt the daemon knows how to answer with a key.
pub(super) fn knows_answer_keys(agent: &str) -> bool {
    agent == "claude"
}

/// A hook's report of a permission prompt, made safe to keep and show: hidden and control
/// characters out, secret-shaped text masked, both parts cut short. `None` for an empty one.
/// Whether the prompt can be answered from the chat is the daemon's call, never the hook's.
pub(super) fn clean_ask(ask: PermissionAsk, agent: &str) -> Option<PermissionAsk> {
    let tool: String = hidden::sanitize(&ask.tool, false)
        .trim()
        .chars()
        .take(MAX_ASK_TOOL_CHARS)
        .collect();
    if tool.is_empty() {
        return None;
    }
    // Mask first, so a secret cut in half by the length limit is not left half showing.
    let detail = mask_secrets(&hidden::sanitize(&ask.detail, false));
    let detail = detail.trim().chars().take(MAX_ASK_DETAIL_CHARS).collect();
    let answerable = knows_answer_keys(agent) && tool != QUESTION_TOOL;
    // Only a question has choices, and only for an agent whose number keys are known.
    let choices = if tool == QUESTION_TOOL && knows_answer_keys(agent) {
        ask.choices
            .iter()
            .take(MAX_ASK_CHOICES)
            .map(|choice| {
                let clean = mask_secrets(&hidden::sanitize(choice, false));
                clean.trim().chars().take(MAX_ASK_CHOICE_CHARS).collect::<String>()
            })
            .filter(|choice| !choice.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    Some(PermissionAsk {
        tool,
        detail,
        answerable,
        choices,
    })
}

impl super::term::Term {
    /// The permission prompt the pane is stopped on, if its agent said what it asks and the pane
    /// still needs you.
    pub(super) fn ask_now(&self) -> Option<PermissionAsk> {
        self.ask.clone().filter(|_| self.status == Status::Blocked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(tool: &str, detail: &str) -> PermissionAsk {
        PermissionAsk {
            tool: tool.into(),
            detail: detail.into(),
            answerable: true,
            choices: Vec::new(),
        }
    }

    #[test]
    fn a_prompt_is_shown_by_its_tool_and_subject_and_the_daemon_decides_if_it_can_be_answered() {
        let cleaned = clean_ask(ask("Bash", "cargo test"), "claude").unwrap();
        assert_eq!(
            (cleaned.tool.as_str(), cleaned.detail.as_str(), cleaned.answerable),
            ("Bash", "cargo test", true)
        );
        let codex = clean_ask(ask("Bash", "cargo test"), "codex").unwrap();
        assert!(!codex.answerable, "a hook cannot claim a key the daemon does not know");
        assert!(clean_ask(ask("  ", "x"), "claude").is_none());
        let question = clean_ask(ask("AskUserQuestion", "Which? 1 A · 2 B"), "claude").unwrap();
        assert!(!question.answerable, "the allow key would pick the first choice of a question");
        let mut asked = ask("AskUserQuestion", "Which?");
        asked.choices = vec!["A".into(), " B\u{1b}".into(), "".into()];
        let cleaned = clean_ask(asked.clone(), "claude").unwrap();
        assert_eq!(cleaned.choices, ["A", "B"], "cleaned, and an empty one dropped");
        assert!(clean_ask(asked, "codex").unwrap().choices.is_empty(), "no keys known, no buttons");
    }

    #[test]
    fn a_prompt_loses_control_characters_secrets_and_length() {
        let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let cleaned = clean_ask(
            ask(
                "Ba\u{1b}sh\u{200b}",
                &format!("curl -H \"Authorization: Bearer {secret}\"\nrm -rf x\u{1b}[2J"),
            ),
            "claude",
        )
        .unwrap();
        assert_eq!(cleaned.tool, "Bash");
        assert!(!cleaned.detail.contains(secret), "{}", cleaned.detail);
        assert!(!cleaned.detail.contains('\u{1b}') && !cleaned.detail.contains('\n'));
        let long = clean_ask(ask("Bash", &"a".repeat(5000)), "claude").unwrap();
        assert_eq!(long.detail.chars().count(), MAX_ASK_DETAIL_CHARS);
    }
}
