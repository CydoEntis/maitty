//! Characters that draw as nothing or reorder the text around them. Text one agent sends
//! another can hide instructions from you in them, so sending refuses it and the approval
//! modal spells them out.

/// Draws as nothing, or reorders the text around it: soft hyphen, zero-width and direction
/// marks, invisible operators, the BOM, and Unicode tag characters (which can spell out a
/// whole hidden message).
pub fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00ad}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2069}' | '\u{feff}' | '\u{e0000}'..='\u{e007f}'
    )
}

/// Each invisible character in `text`, with its position (counted in characters).
pub fn find(text: &str) -> Vec<(usize, char)> {
    text.chars().enumerate().filter(|(_, c)| is_invisible(*c)).collect()
}

/// Why text with hidden characters is not sent, naming each one (a long run is cut short).
pub fn refusal(found: &[(usize, char)]) -> String {
    const NAMED_AT_MOST: usize = 8;
    let mut named: Vec<String> = found.iter().take(NAMED_AT_MOST).map(|(at, c)| format!("\\u{{{:x}}} at {at}", *c as u32)).collect();
    if found.len() > NAMED_AT_MOST {
        named.push(format!("and {} more", found.len() - NAMED_AT_MOST));
    }
    format!("refused: hidden characters {}; remove them and send again", named.join(", "))
}

/// `Err` with the refusal when `text` holds hidden characters.
pub fn check(text: &str) -> Result<(), String> {
    let found = find(text);
    if found.is_empty() { Ok(()) } else { Err(refusal(&found)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_accented_text_is_fine() {
        assert!(check("fix the login bug, then run cargo test").is_ok());
        assert!(check("café naïve Ünïcödé — 日本語 ✓").is_ok());
        assert!(check("line one\nline two\ttabbed").is_ok(), "line breaks and tabs are visible");
    }

    #[test]
    fn each_kind_of_hidden_character_is_found_where_it_is() {
        assert_eq!(find("a\u{200b}b"), [(1, '\u{200b}')], "zero-width space");
        assert_eq!(find("\u{202e}evil"), [(0, '\u{202e}')], "right-to-left override");
        assert_eq!(find("\u{feff}hi"), [(0, '\u{feff}')], "byte order mark");
        assert_eq!(find("ok\u{e0041}"), [(2, '\u{e0041}')], "tag character");
        assert_eq!(find("👨\u{200d}👩"), [(1, '\u{200d}')], "a joiner inside emoji is refused too");
    }

    #[test]
    fn the_refusal_names_each_character_and_its_place() {
        assert_eq!(
            check("go\u{200b}od \u{e0041}").unwrap_err(),
            "refused: hidden characters \\u{200b} at 2, \\u{e0041} at 6; remove them and send again"
        );
        let many = "\u{200b}".repeat(10);
        assert!(check(&many).unwrap_err().contains("and 2 more"));
    }
}
