//! Find across sessions: a text search over what each pane has printed recently.
//! Everything here blocks; callers run it on a blocking thread.

use crate::protocol::{SearchHit, TermId};

pub const MAX_QUERY_CHARS: usize = 200;
const MAX_HITS: usize = 100;
const MAX_HITS_PER_PANE: usize = 20;
const MAX_LINE_CHARS: usize = 200;

/// Case-insensitive matches of `query` in each pane's output, newest lines first within a pane.
pub fn search(panes: &[(TermId, Vec<u8>)], query: &str) -> Vec<SearchHit> {
    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    for (term, output) in panes {
        let lines = plain_lines(output);
        let found = lines
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, text)| text.to_lowercase().contains(&needle))
            .take(MAX_HITS_PER_PANE)
            .map(|(line, text)| SearchHit {
                term: *term,
                line: line + 1,
                text: text.chars().take(MAX_LINE_CHARS).collect(),
            });
        hits.extend(found);
        if hits.len() >= MAX_HITS {
            hits.truncate(MAX_HITS);
            break;
        }
    }
    hits
}

/// Output as plain lines: escape sequences dropped, a carriage return starting the line over.
fn plain_lines(output: &[u8]) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Text,
        Escape,
        Csi,
        Osc,
        OscEscape,
    }
    let mut lines = vec![String::new()];
    let mut state = State::Text;
    let mut pending = Vec::new();
    // A carriage return only matters if text follows it before the line ends: CRLF is a line end.
    let mut returned = false;
    let flush = |pending: &mut Vec<u8>, line: &mut String| {
        if !pending.is_empty() {
            line.push_str(&String::from_utf8_lossy(pending));
            pending.clear();
        }
    };
    for &byte in output {
        state = match (state, byte) {
            (State::Text, 0x1b) => State::Escape,
            (State::Text, b'\n') => {
                returned = false;
                if let Some(line) = lines.last_mut() {
                    flush(&mut pending, line);
                }
                lines.push(String::new());
                State::Text
            }
            (State::Text, b'\r') => {
                returned = true;
                State::Text
            }
            (State::Text, byte) if byte >= 0x20 || byte == b'\t' => {
                if returned {
                    returned = false;
                    pending.clear();
                    if let Some(line) = lines.last_mut() {
                        line.clear();
                    }
                }
                pending.push(byte);
                State::Text
            }
            (State::Text, _) => State::Text,
            (State::Escape, b'[') => State::Csi,
            (State::Escape, b']') => State::Osc,
            (State::Escape, _) => State::Text,
            (State::Csi, 0x40..=0x7e) => State::Text,
            (State::Csi, _) => State::Csi,
            (State::Osc, 0x07) => State::Text,
            (State::Osc, 0x1b) => State::OscEscape,
            (State::Osc, _) => State::Osc,
            (State::OscEscape, _) => State::Text,
        };
    }
    if let Some(line) = lines.last_mut() {
        flush(&mut pending, line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_codes_and_title_sequences_are_not_part_of_the_text() {
        let output = b"\x1b[1;32mgreen\x1b[0m word\r\n\x1b]0;a title\x07second line\n";
        assert_eq!(plain_lines(output), ["green word", "second line", ""]);
    }

    #[test]
    fn a_carriage_return_starts_the_line_over_like_a_progress_bar() {
        assert_eq!(plain_lines(b"10%\r50%\r100% done\n")[0], "100% done");
    }

    #[test]
    fn search_ignores_case_numbers_lines_and_finds_the_newest_first() {
        let panes = vec![(7, b"first Error here\nnothing\nanother ERROR\n".to_vec())];
        let hits = search(&panes, "error");
        assert_eq!(
            hits.iter()
                .map(|h| (h.term, h.line, h.text.as_str()))
                .collect::<Vec<_>>(),
            [(7, 3, "another ERROR"), (7, 1, "first Error here")]
        );
    }

    #[test]
    fn one_noisy_pane_cannot_crowd_out_the_others() {
        let noisy = "match\n".repeat(500).into_bytes();
        let panes = vec![(1, noisy), (2, b"match once\n".to_vec())];
        let hits = search(&panes, "match");
        assert_eq!(
            hits.iter().filter(|h| h.term == 1).count(),
            MAX_HITS_PER_PANE
        );
        assert_eq!(hits.iter().filter(|h| h.term == 2).count(), 1);
    }
}
