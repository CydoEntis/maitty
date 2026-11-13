//! What a GitHub login, repository name and time may look like. Everything that shows or uses one
//! that a program printed or another machine sent checks it here first, so text that is none of
//! these never reaches a screen or a command line.

/// The longest login GitHub gives out.
pub const LOGIN_MAX_CHARS: usize = 39;
/// The longest repository name GitHub allows.
pub const REPO_NAME_MAX_CHARS: usize = 100;
/// Longer than any time GitHub prints (`2026-01-02T03:04:05Z`).
const TIME_MAX_CHARS: usize = 32;

/// Whether `text` could be a GitHub login: letters, digits and dashes, with no dash at either end.
/// A login is put on `gh`'s command line, where a leading dash would be read as an option.
pub fn is_login(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= LOGIN_MAX_CHARS
        && !text.starts_with('-')
        && !text.ends_with('-')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Whether `text` could be a repository as GitHub writes it, `owner/name`.
pub fn is_repo(text: &str) -> bool {
    let Some((owner, name)) = text.split_once('/') else {
        return false;
    };
    is_login(owner)
        && !name.is_empty()
        && name.len() <= REPO_NAME_MAX_CHARS
        && name != "."
        && name != ".."
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The address `owner/name` is cloned from over https, or `None` when it is not a repository.
pub fn clone_address(repo: &str) -> Option<String> {
    is_repo(repo).then(|| format!("https://github.com/{repo}.git"))
}

/// Whether `text` could be a time as GitHub prints one.
pub fn is_time(text: &str) -> bool {
    !text.is_empty() && text.len() <= TIME_MAX_CHARS && text.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | ':' | 'T' | 'Z' | '.' | '+'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_login_is_letters_digits_and_dashes_up_to_githubs_length() {
        assert!(is_login("me") && is_login("work-me-2") && is_login(&"a".repeat(LOGIN_MAX_CHARS)));
        for not in ["", "a b", "<b>", "a/b", "@me", "--web", "-x", "x-", &"a".repeat(LOGIN_MAX_CHARS + 1)] {
            assert!(!is_login(not), "{not:?}");
        }
    }

    #[test]
    fn a_repository_is_an_owner_and_a_plain_name() {
        assert!(is_repo("me/widgets") && is_repo("my-org/.github") && is_repo("me/a_b.c-d"));
        for not in ["widgets", "me/", "/widgets", "me/a/b", "me/..", "me/.", "me/a b", "me/a;b", "-x/widgets", "me/--upload-pack=x"] {
            assert!(!is_repo(not), "{not:?}");
        }
    }

    #[test]
    fn the_clone_address_is_built_from_the_name_and_only_for_a_repository() {
        assert_eq!(clone_address("me/widgets").as_deref(), Some("https://github.com/me/widgets.git"));
        assert_eq!(clone_address("me/wid gets"), None);
    }

    #[test]
    fn a_time_is_only_what_a_time_is_written_with() {
        assert!(is_time("2026-01-02T03:04:05Z"));
        assert!(!is_time("") && !is_time("yesterday") && !is_time(&"1".repeat(TIME_MAX_CHARS + 1)));
    }
}
