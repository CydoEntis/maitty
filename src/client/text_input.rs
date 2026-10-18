//! Cursor editing shared by profile fields, workflow names and task entry.
//! Positions count characters so editing never splits a UTF-8 code point.

use super::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthChar;

pub(super) fn insert(value: &mut String, cursor: &mut Option<usize>, text: &str) {
    let at = cursor
        .unwrap_or_else(|| value.chars().count())
        .min(value.chars().count());
    let byte = value
        .char_indices()
        .nth(at)
        .map_or(value.len(), |(byte, _)| byte);
    value.insert_str(byte, text);
    *cursor = Some(at + text.chars().count());
}

pub(super) fn key(value: &mut String, cursor: &mut Option<usize>, key: &KeyEvent) {
    let count = value.chars().count();
    let at = cursor.unwrap_or(count).min(count);
    let byte = |at| {
        value
            .char_indices()
            .nth(at)
            .map_or(value.len(), |(byte, _)| byte)
    };
    match key.code {
        KeyCode::Left => *cursor = Some(at.saturating_sub(1)),
        KeyCode::Right => *cursor = Some((at + 1).min(count)),
        KeyCode::Home => *cursor = Some(0),
        KeyCode::End => *cursor = Some(count),
        KeyCode::Up | KeyCode::Down => *cursor = Some(adjacent_line(value, at, key.code)),
        KeyCode::Backspace if at > 0 => {
            value.replace_range(byte(at - 1)..byte(at), "");
            *cursor = Some(at - 1);
        }
        KeyCode::Delete if at < count => {
            value.replace_range(byte(at)..byte(at + 1), "");
            *cursor = Some(at);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            value.clear();
            *cursor = Some(0);
        }
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            insert(value, cursor, &c.to_string())
        }
        _ => {}
    }
}

fn adjacent_line(text: &str, at: usize, direction: KeyCode) -> usize {
    let chars: Vec<_> = text.chars().collect();
    let start = chars[..at]
        .iter()
        .rposition(|c| *c == '\n')
        .map_or(0, |i| i + 1);
    let column = at - start;
    if direction == KeyCode::Up {
        if start == 0 {
            return at;
        }
        let end = start - 1;
        let previous = chars[..end]
            .iter()
            .rposition(|c| *c == '\n')
            .map_or(0, |i| i + 1);
        previous + column.min(end - previous)
    } else {
        let Some(end) = chars[at..].iter().position(|c| *c == '\n').map(|i| at + i) else {
            return at;
        };
        let next = end + 1;
        let length = chars[next..]
            .iter()
            .position(|c| *c == '\n')
            .unwrap_or(chars.len() - next);
        next + column.min(length)
    }
}

pub(super) fn caret(text: &str, cursor: Option<usize>) -> String {
    let at = cursor.unwrap_or_else(|| text.chars().count());
    let byte = text
        .char_indices()
        .nth(at)
        .map_or(text.len(), |(byte, _)| byte);
    format!("{}▏{}", &text[..byte], &text[byte..])
}

pub(super) fn window(text: &str, cursor: Option<usize>, width: usize) -> String {
    let at = cursor
        .unwrap_or_else(|| text.chars().count())
        .min(text.chars().count());
    let before: Vec<_> = text.chars().take(at).collect();
    let mut room = width.saturating_sub(1);
    let mut start = at;
    for c in before.iter().rev() {
        let cells = c.width().unwrap_or(0);
        if cells > room {
            break;
        }
        room -= cells;
        start -= 1;
    }
    let mut used = 0;
    caret(text, cursor)
        .chars()
        .skip(start)
        .take_while(|c| {
            used += c.width().unwrap_or(0);
            used <= width
        })
        .collect()
}

/// Preserve blank lines and indentation while keeping every input character reachable.
pub(super) fn lines(text: &str, width: usize) -> Vec<String> {
    const TAB_COLUMNS: usize = 4;
    let width = width.max(1);
    let mut result = Vec::new();
    for line in text.split('\n') {
        let mut row = String::new();
        let mut used = 0;
        for c in line
            .replace('\t', &" ".repeat(TAB_COLUMNS))
            .chars()
            .filter(|c| *c != '\r')
        {
            let cells = c.width().unwrap_or(0);
            if used + cells > width && !row.is_empty() {
                result.push(std::mem::take(&mut row));
                used = 0;
            }
            row.push(c);
            used += cells;
        }
        result.push(row);
    }
    result
}
