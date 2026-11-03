//! Reading the question an agent is stopped on, off its screen.

const OPTION_LOOKBACK_LINES: usize = 20;
const QUESTION_LINES: usize = 6;
const ASKING_LINES: usize = 12;
const BOX_CHARS: &str = "│┃ ";
const BORDER_CHARS: &str = "─━╭╮╰╯┌┐└┘";

/// The numbered choices on a screen and the question above them.
pub fn prompt_on(screen: &str) -> (String, Vec<(u32, String)>) {
    let lines: Vec<&str> = screen.lines().collect();
    let (opts, first) = options_in(&lines);
    let question = lines[first.saturating_sub(QUESTION_LINES)..first].join(" ");
    (question, opts)
}

/// What a stopped agent is asking, for a card that has to show the exact command and not just the
/// last prompt: the lines above its numbered choices (the command, the file), then the choices.
/// Empty when the screen shows no numbered choices.
pub fn asking(screen: &str) -> String {
    let lines: Vec<&str> = screen.lines().collect();
    let (opts, first) = options_in(&lines);
    if opts.is_empty() {
        return String::new();
    }
    let above = lines[first.saturating_sub(ASKING_LINES)..first]
        .iter()
        .map(|line| {
            line.trim()
                .trim_matches(|c: char| BOX_CHARS.contains(c))
                .trim_end()
        })
        .map(|line| {
            if line.chars().all(|c| BORDER_CHARS.contains(c)) {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>();
    // Blank lines at the top and bottom are the box's padding.
    let start = above
        .iter()
        .position(|line| !line.is_empty())
        .unwrap_or(above.len());
    let end = above
        .iter()
        .rposition(|line| !line.is_empty())
        .map_or(start, |last| last + 1);
    let mut text = above[start..end].join("\n");
    for (number, label) in &opts {
        text.push_str(&format!("\n{number}. {label}"));
    }
    text
}

fn options_in(lines: &[&str]) -> (Vec<(u32, String)>, usize) {
    let mut opts = Vec::new();
    let mut first = lines.len();
    for (i, l) in lines.iter().enumerate().rev().take(OPTION_LOOKBACK_LINES) {
        let t = l
            .trim()
            .trim_matches(|c: char| BOX_CHARS.contains(c))
            .trim_start_matches(['❯', '>', ' '])
            .trim();
        let mut ch = t.chars();
        if let (Some(d), Some('.')) = (ch.next(), ch.next())
            && let Some(n) = d.to_digit(10)
        {
            opts.push((n, ch.as_str().trim().to_string()));
            first = i;
        }
    }
    opts.reverse();
    (opts, first)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLAUDE_BASH_PROMPT: &str = "\
╭──────────────────────────────────────╮
│ Bash command                         │
│                                      │
│   rm -rf target/debug                │
│   Delete the build folder            │
│                                      │
│ Do you want to proceed?              │
│ ❯ 1. Yes                             │
│   2. Yes, and don't ask again        │
│   3. No, and tell Claude what to do  │
╰──────────────────────────────────────╯";

    #[test]
    fn the_choices_and_the_question_above_them_are_read() {
        let (question, options) = prompt_on(CLAUDE_BASH_PROMPT);
        assert!(question.contains("Do you want to proceed?"));
        assert_eq!(options.len(), 3);
        assert_eq!(options[0], (1, "Yes".to_string()));
    }

    #[test]
    fn asking_keeps_the_exact_command_and_the_choices_without_the_box() {
        let text = asking(CLAUDE_BASH_PROMPT);
        assert!(text.contains("rm -rf target/debug"), "{text}");
        assert!(text.contains("3. No, and tell Claude what to do"), "{text}");
        assert!(!text.contains('│'), "{text}");
    }

    #[test]
    fn a_screen_with_no_numbered_choices_is_asking_nothing() {
        assert_eq!(asking("$ cargo build\nFinished\n"), "");
    }
}
