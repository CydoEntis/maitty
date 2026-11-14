//! The question a permission prompt asks, as the agent's own hook reported it (ADR 0003).
//!
//! It comes from the owning runtime's `PermissionRequest` hook payload, which reaches the daemon
//! through the same checked path as every status report (AGENTS.md non-negotiable 5), never from
//! the screen (ADR 0003). It is cleaned here before it is kept or sent to any client.

use super::conversation::mask_secrets;
use crate::protocol::{AskQuestion, PermissionAsk, Status};
use maitty_shared::hidden;

/// Longest tool name and subject kept for a permission prompt, in characters.
const MAX_ASK_TOOL_CHARS: usize = 64;
pub(super) const MAX_ASK_DETAIL_CHARS: usize = 300;
/// Most choices and longest choice kept for a question prompt; the menu itself holds four.
const MAX_ASK_CHOICES: usize = 4;
const MAX_ASK_CHOICE_CHARS: usize = 80;
/// Most questions kept for a prompt that asks several: the agent's own limit.
const MAX_ASK_QUESTIONS: usize = 4;

/// The tool Claude asks a multiple-choice question with. Its menu is not the allow-or-refuse menu,
/// so the key that allows a tool there would pick the first choice for the person.
const QUESTION_TOOL: &str = "AskUserQuestion";

/// The agents whose permission prompt the daemon knows how to answer with a key. Claude's menu is
/// the same everywhere; Codex's is trusted only while its screen still has the shape checked
/// below, so for Codex this is "may be answerable", settled by `codex_prompt_answerable`.
pub(super) fn knows_answer_keys(agent: &str) -> bool {
    matches!(agent, "claude" | "codex")
}

const CODEX_COMMAND_TITLE: &str = "Would you like to run the following command?";
const CODEX_EDIT_TITLE: &str = "Would you like to make the following edits?";
const CODEX_APPROVE_OPTION: &str = "1. Yes, proceed (y)";
const CODEX_ALWAYS_OPTION: &str = "2. Yes, and don't ask again";
const CODEX_DECLINE_OPTION: &str = "3. No, and tell Codex what to do differently (esc)";
const CODEX_MENU_FOOTER: &str = "Press enter to confirm or esc to cancel";
const CODEX_COMMAND_MARK: &str = "$ ";
const CODEX_DESTINATION_MARK: &str = "Destination:";
/// Most lines Codex puts between the title and the options (environment, reason, the command or
/// path, each wrapped) and in option 2, whose text holds the whole command and wraps with it.
const CODEX_MAX_BODY_LINES: usize = 12;
const CODEX_MAX_OPTION_LINES: usize = 8;

/// Lines of Codex's own history start with one of these; a menu's body never does, so one between
/// a title and the options means the title belongs to something already answered.
const CODEX_HISTORY_MARKS: [char; 4] = ['\u{2022}', '\u{2714}', '\u{2717}', '\u{25a0}'];

fn without_spaces(text: &str) -> String {
    text.split_whitespace().collect()
}

/// Whether the screen is Codex's command or file-change approval menu, in the shape checked
/// against codex-cli 0.166.0 by driving a real Codex in a terminal (`y` ran the command or applied
/// the patch, Esc did neither; the chat's keys are `CODEX_ANSWER_KEYS` in `apps/gui`), and shows
/// what the hook said was asked (`detail`). A newer Codex may reword a menu, which is why the screen
/// is compared rather than the keys assumed.
///
/// The shape: the footer is the last line; right above it the decline (esc) option, above that
/// option 2 (it holds the whole command or paths, so it may wrap over several lines) and "Yes,
/// proceed (y)" first; above those, close and with no history line between, the title, then its
/// body: a `$ <command>` for a command, `Destination:` and the path for an edit. A command must be
/// the one the hook reported (spaces ignored, either one allowed to be the shorter, since the
/// daemon cuts a long detail and Codex may cut the line); an edit must name the hook's file. Any
/// other screen (a network or tool approval, a menu from a newer Codex, an answered prompt, another
/// command than the one reported) is not, and the chat falls back to the terminal's own menu.
pub(super) fn codex_prompt_answerable(screen: &str, detail: &str) -> bool {
    let lines: Vec<&str> = screen
        .lines()
        .map(|line| line.trim().trim_start_matches('\u{203a}').trim())
        .filter(|line| !line.is_empty())
        .collect();
    let Some(footer_at) = lines.len().checked_sub(1) else { return false };
    if lines[footer_at] != CODEX_MENU_FOOTER || footer_at == 0 || lines[footer_at - 1] != CODEX_DECLINE_OPTION {
        return false;
    }
    let decline_at = footer_at - 1;
    let Some(approve_at) = (decline_at.saturating_sub(CODEX_MAX_OPTION_LINES + 1)..decline_at)
        .rev()
        .find(|at| lines[*at] == CODEX_APPROVE_OPTION)
    else {
        return false;
    };
    let always = lines[approve_at + 1..decline_at].join(" ");
    if !always.starts_with(CODEX_ALWAYS_OPTION) || !(always.ends_with("(p)") || always.ends_with("(a)")) {
        return false;
    }
    let Some(title_at) = (approve_at.saturating_sub(CODEX_MAX_BODY_LINES + 1)..approve_at)
        .rev()
        .find(|at| lines[*at] == CODEX_COMMAND_TITLE || lines[*at] == CODEX_EDIT_TITLE)
    else {
        return false;
    };
    let body = &lines[title_at + 1..approve_at];
    if body.iter().any(|line| line.starts_with(CODEX_HISTORY_MARKS) || *line == CODEX_MENU_FOOTER) {
        return false;
    }
    let asked = without_spaces(detail);
    if asked.is_empty() {
        return false;
    }
    if lines[title_at] == CODEX_COMMAND_TITLE {
        let Some(start) = body.iter().position(|line| line.starts_with(CODEX_COMMAND_MARK)) else { return false };
        let shown = without_spaces(&body[start..].join(" ")[CODEX_COMMAND_MARK.len()..]);
        let shown = shown.trim_end_matches('\u{2026}');
        !shown.is_empty() && (shown.starts_with(&asked) || asked.starts_with(shown))
    } else {
        let Some(start) = body.iter().position(|line| line.starts_with(CODEX_DESTINATION_MARK)) else { return false };
        let path = without_spaces(&body[start + 1..].join(" "));
        let name = path.rsplit(['/', '\\']).next().unwrap_or_default();
        !name.is_empty() && asked.contains(name)
    }
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
    // Only a question has choices, and only for Claude, whose number keys are known.
    let choices = if tool == QUESTION_TOOL && agent == "claude" {
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
    // Several questions are answered by a key each, in order: all of them clean, or none, since a
    // choice dropped from one would put every number after it on the wrong line.
    let clean = |text: &str, most: usize| -> String {
        mask_secrets(&hidden::sanitize(text, false)).trim().chars().take(most).collect()
    };
    let questions: Vec<AskQuestion> = if tool == QUESTION_TOOL && agent == "claude" && ask.questions.len() <= MAX_ASK_QUESTIONS {
        ask.questions
            .iter()
            .map(|asked| AskQuestion {
                question: clean(&asked.question, MAX_ASK_DETAIL_CHARS),
                choices: asked.choices.iter().map(|choice| clean(choice, MAX_ASK_CHOICE_CHARS)).collect(),
            })
            .collect()
    } else {
        Vec::new()
    };
    let whole = questions.len() >= 2
        && questions.iter().all(|asked| {
            !asked.question.is_empty()
                && (2..=MAX_ASK_CHOICES).contains(&asked.choices.len())
                && asked.choices.iter().all(|choice| !choice.is_empty())
        });
    Some(PermissionAsk {
        tool,
        detail,
        answerable,
        choices,
        questions: if whole { questions } else { Vec::new() },
    })
}

impl super::term::Term {
    /// The permission prompt the pane is stopped on, if its agent said what it asks and the pane
    /// still needs you.
    pub(super) fn ask_now(&self) -> Option<PermissionAsk> {
        let mut ask = self.ask.clone().filter(|_| self.status == Status::Blocked)?;
        // Codex's keys were checked against one menu shape: read the screen now, not when the hook fired.
        if self.ask_agent.as_deref() == Some("codex") {
            ask.answerable &= codex_prompt_answerable(&self.parser.screen().contents(), &ask.detail);
        }
        Some(ask)
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
            questions: Vec::new(),
        }
    }

    #[test]
    fn a_prompt_is_shown_by_its_tool_and_subject_and_the_daemon_decides_if_it_can_be_answered() {
        let cleaned = clean_ask(ask("Bash", "cargo test"), "claude").unwrap();
        assert_eq!(
            (cleaned.tool.as_str(), cleaned.detail.as_str(), cleaned.answerable),
            ("Bash", "cargo test", true)
        );
        let gemini = clean_ask(ask("Bash", "cargo test"), "gemini").unwrap();
        assert!(!gemini.answerable, "a hook cannot claim a key the daemon does not know");
        assert!(clean_ask(ask("shell", "ls"), "codex").unwrap().answerable, "Codex is settled by its screen, in ask_now");
        assert!(clean_ask(ask("  ", "x"), "claude").is_none());
        let question = clean_ask(ask("AskUserQuestion", "Which? 1 A · 2 B"), "claude").unwrap();
        assert!(!question.answerable, "the allow key would pick the first choice of a question");
        let mut asked = ask("AskUserQuestion", "Which?");
        asked.choices = vec!["A".into(), " B\u{1b}".into(), "".into()];
        let cleaned = clean_ask(asked.clone(), "claude").unwrap();
        assert_eq!(cleaned.choices, ["A", "B"], "cleaned, and an empty one dropped");
        assert!(clean_ask(asked, "codex").unwrap().choices.is_empty(), "no choice keys known, no buttons");
    }

    #[test]
    fn several_questions_are_kept_whole_or_not_at_all() {
        let asked = |choices: &[&str]| AskQuestion {
            question: " Which?\u{1b} ".into(),
            choices: choices.iter().map(|choice| choice.to_string()).collect(),
        };
        let mut prompt = ask("AskUserQuestion", "Which? / Which?");
        prompt.questions = vec![asked(&["A", "B"]), asked(&["C", " D ", "E"])];
        let cleaned = clean_ask(prompt.clone(), "claude").unwrap();
        assert_eq!(cleaned.questions.len(), 2);
        assert_eq!((cleaned.questions[0].question.as_str(), cleaned.questions[1].choices.clone()), ("Which?", vec!["C".to_string(), "D".into(), "E".into()]));
        assert!(clean_ask(prompt.clone(), "codex").unwrap().questions.is_empty(), "no choice keys known");
        // A choice that cleans to nothing would shift every number after it.
        prompt.questions[1].choices[1] = "\u{1b}".into();
        assert!(clean_ask(prompt.clone(), "claude").unwrap().questions.is_empty());
        // One question is the single-choice prompt's, not this one's.
        prompt.questions = vec![asked(&["A", "B"])];
        assert!(clean_ask(prompt, "claude").unwrap().questions.is_empty());
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

    /// Captured from codex-cli 0.166.0 in a terminal (see `codex_prompt_answerable`); the folder and
    /// file names are placeholders.
    const CODEX_COMMAND_SCREEN: &str = "\
  Would you like to run the following command?
  Environment: local
  Reason: The sandbox is read-only. Allow running this command outside the sandbox to create probe.txt?
  $ echo probe > probe.txt
\u{203a} 1. Yes, proceed (y)
  2. Yes, and don't ask again for commands that start with `echo probe > probe.txt` (p)
  3. No, and tell Codex what to do differently (esc)

  Press enter to confirm or esc to cancel
";
    const CODEX_EDIT_SCREEN: &str = "\
\u{2022} Added note.txt (+1 -0)
    1 +hi
    + Show details
  Would you like to make the following edits?
  Description: Apply proposed file edits
  Destination:
  /work/project/note.txt
\u{203a} 1. Yes, proceed (y)
  2. Yes, and don't ask again for these files (a)
  3. No, and tell Codex what to do differently (esc)

  Press enter to confirm or esc to cancel
";

    const COMMAND: &str = "echo probe > probe.txt";

    #[test]
    fn codex_command_and_file_change_menus_are_answerable() {
        assert!(codex_prompt_answerable(CODEX_COMMAND_SCREEN, COMMAND));
        assert!(codex_prompt_answerable(CODEX_EDIT_SCREEN, "*** Begin Patch *** Add File: note.txt +hi *** End Patch"));
    }

    #[test]
    fn a_codex_screen_off_the_verified_shape_is_not_answerable() {
        assert!(!codex_prompt_answerable("", COMMAND));
        assert!(!codex_prompt_answerable("\u{203a} Ask Codex to do anything", COMMAND));
        // The menu already answered: the options are gone.
        let answered = CODEX_COMMAND_SCREEN.replace("Press enter to confirm or esc to cancel", "Working (4s)");
        assert!(!codex_prompt_answerable(&answered, COMMAND));
        // A reworded approve key must not be answered with `y`.
        let edit = "Add File: note.txt";
        assert!(!codex_prompt_answerable(&CODEX_EDIT_SCREEN.replace("1. Yes, proceed (y)", "1. Yes, proceed (enter)"), edit));
        // A reworded decline.
        assert!(!codex_prompt_answerable(&CODEX_EDIT_SCREEN.replace("(esc)", "(n)"), edit));
        // Another menu with the same footer (a different approval) has no known title.
        let other = CODEX_COMMAND_SCREEN.replace(CODEX_COMMAND_TITLE, "Allow the tool to run?");
        assert!(!codex_prompt_answerable(&other, COMMAND));
        // Extra options mean the keys' meaning has moved.
        let more = CODEX_COMMAND_SCREEN.replace("  3. No,", "  3. Maybe\n  4. No,");
        assert!(!codex_prompt_answerable(&more, COMMAND));
    }

    #[test]
    fn the_menu_must_show_what_the_hook_reported() {
        assert!(!codex_prompt_answerable(CODEX_COMMAND_SCREEN, "rm -rf build"), "another command");
        assert!(!codex_prompt_answerable(CODEX_COMMAND_SCREEN, ""), "nothing to compare");
        assert!(!codex_prompt_answerable(CODEX_EDIT_SCREEN, "*** Add File: other.txt"), "another file");
        assert!(!codex_prompt_answerable(CODEX_EDIT_SCREEN, ""), "nothing to compare");
        // The hook's line is whitespace-collapsed and may be cut short; the screen's is drawn as is.
        assert!(codex_prompt_answerable(CODEX_COMMAND_SCREEN, "echo   probe\n >  probe.txt"));
        assert!(codex_prompt_answerable(CODEX_COMMAND_SCREEN, "echo probe"));
    }

    const LONG: &str = "cargo test --workspace --exclude maitty-gui --all-targets -- --nocapture --test-threads=1";

    fn narrow_command_screen() -> String {
        String::from(
            "\
  Would you like to run the following command?
  Environment: local
  Reason: Run the whole suite outside the read-only sandbox?
  $ cargo test --workspace --exclude maitty-gui --all-targets --
  --nocapture --test-threads=1
\u{203a} 1. Yes, proceed (y)
  2. Yes, and don't ask again for commands that start with `cargo test
  --workspace --exclude maitty-gui --all-targets -- --nocapture
  --test-threads=1` (p)
  3. No, and tell Codex what to do differently (esc)

  Press enter to confirm or esc to cancel
"
        )
    }

    #[test]
    fn a_wrapped_option_and_command_are_still_the_checked_menu() {
        assert!(codex_prompt_answerable(&narrow_command_screen(), LONG));
        // The same width with another command is still refused.
        assert!(!codex_prompt_answerable(&narrow_command_screen(), "cargo publish"));
        // Wrapped text that is not an option 2 of the menu is a different shape.
        let other = narrow_command_screen().replace("2. Yes, and don't ask again for commands", "2. Maybe later for commands");
        assert!(!codex_prompt_answerable(&other, LONG));
    }

    #[test]
    fn the_title_must_sit_over_this_menu_not_in_the_scrollback() {
        let stale = format!(
            "  Would you like to run the following command?\n  $ {COMMAND}\n• Ran {COMMAND}\n• Working on another thing\n  Allow network access?\n› 1. Yes, proceed (y)\n  2. Yes, and don't ask again for this host (p)\n  3. No, and tell Codex what to do differently (esc)\n\n  Press enter to confirm or esc to cancel\n"
        );
        assert!(!codex_prompt_answerable(&stale, COMMAND));
        // Far above is not over it either, whatever lies between.
        let far = CODEX_COMMAND_SCREEN.replace("  Environment: local\n", &"  text\n".repeat(30));
        assert!(!codex_prompt_answerable(&far, COMMAND));
    }
}
