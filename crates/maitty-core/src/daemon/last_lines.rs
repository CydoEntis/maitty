//! What an agent last said, read off its screen, for an agent whose hooks report no last
//! message. A terminal interface ends in its input line and a row of hints, so what it said is
//! the text above those.

/// How many rows of the screen's foot are looked at.
pub(super) const ROWS: u16 = 24;
/// The most lines kept.
const KEPT: usize = 3;
/// The most characters kept, so one endless line cannot fill every snapshot.
const MOST_CHARS: usize = 400;
/// What an agent's input line starts with, once the box round it is taken off.
const PROMPTS: [char; 3] = ['›', '❯', '>'];

/// A box-drawing or block character: the frame an interface draws round its text.
fn is_frame(c: char) -> bool {
    matches!(c, '\u{2500}'..='\u{259f}')
}

/// A row without the box round it. A row that is only frame comes out empty.
fn unboxed(row: &str) -> &str {
    row.trim_matches(|c: char| c.is_whitespace() || is_frame(c))
}

/// The last lines of text in `tail` (the screen's bottom rows) above the input line: the last
/// row that starts as a prompt does, or the end of the screen when none does.
pub(super) fn of_screen(tail: &str) -> String {
    let rows: Vec<&str> = tail.lines().map(unboxed).collect();
    let input = rows
        .iter()
        .rposition(|row| row.starts_with(PROMPTS))
        .unwrap_or(rows.len());
    let mut kept: Vec<&str> = rows[..input]
        .iter()
        .rev()
        .filter(|row| !row.is_empty())
        .take(KEPT)
        .copied()
        .collect();
    kept.reverse();
    kept.join("\n").chars().take(MOST_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lines_above_the_input_line_are_what_it_said() {
        let screen = "\
• Ran npm test -- Footer
  └ 3 passed

• Done. Footer updated and its tests pass.

Worked for 48s

› Ask Codex to do anything

  gpt-5 · 12% context left";
        // The `└` of a tool's result is frame, and goes with the borders.
        assert_eq!(
            of_screen(screen),
            "3 passed\n• Done. Footer updated and its tests pass.\nWorked for 48s"
        );
    }

    #[test]
    fn an_input_box_and_its_borders_are_not_what_it_said() {
        let screen = "\
All three migrations ran clean.
Want me to open the pull request?

╭──────────────────────────────────────╮
│ > Try \"fix the failing test\"         │
╰──────────────────────────────────────╯
  ? for shortcuts";
        assert_eq!(
            of_screen(screen),
            "All three migrations ran clean.\nWant me to open the pull request?"
        );
    }

    #[test]
    fn a_screen_with_no_input_line_gives_its_last_lines() {
        assert_eq!(of_screen("one\n\ntwo\nthree\n\nfour\n"), "two\nthree\nfour");
    }

    #[test]
    fn an_empty_or_frame_only_screen_says_nothing() {
        assert_eq!(of_screen(""), "");
        assert_eq!(of_screen("╭────╮\n│    │\n╰────╯\n"), "");
    }

    #[test]
    fn a_line_without_end_is_cut() {
        let long = "x".repeat(MOST_CHARS * 2);
        assert_eq!(of_screen(&long).chars().count(), MOST_CHARS);
    }
}
