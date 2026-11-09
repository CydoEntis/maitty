//! The in-order match the command search and the key map use.

/// The in-order, case-insensitive match of `query` in `text`: the char indices that matched.
pub(in crate::client) fn fuzzy_indices(query: &str, text: &str) -> Option<Vec<usize>> {
    let mut wanted = query.chars().flat_map(char::to_lowercase).peekable();
    let mut at = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let Some(&next) = wanted.peek() else { break };
        if c.to_lowercase().next() == Some(next) {
            at.push(i);
            wanted.next();
        }
    }
    wanted.peek().is_none().then_some(at)
}

#[cfg(test)]
mod tests {
    use super::fuzzy_indices;

    #[test]
    fn letters_match_in_order_and_ignoring_case() {
        assert_eq!(fuzzy_indices("spld", "Split down"), Some(vec![0, 1, 2, 6]));
        assert_eq!(fuzzy_indices("SD", "split down"), Some(vec![0, 6]));
        assert_eq!(fuzzy_indices("", "anything"), Some(vec![]));
    }

    #[test]
    fn a_letter_out_of_order_or_missing_does_not_match() {
        assert_eq!(fuzzy_indices("ds", "split down"), None);
        assert_eq!(fuzzy_indices("xyz", "split down"), None);
    }
}
