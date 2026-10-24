//! The app's name (ADR 0019). It is set in `Cargo.toml` and nowhere else: the package name is the
//! command, `[package.metadata.brand]` holds the name people see and the names it had before.
//! Code writes it with `env!("APP_COMMAND")`, `env!("APP_NAME")` and `env!("APP_ENV")` inside
//! `concat!`, so a file, variable or message stays a constant; a format string with arguments
//! of its own names these constants instead (`format!("{APP_NAME} saw {n}")`).

/// What people see: titles, the splash, messages.
pub const APP_NAME: &str = env!("APP_NAME");
/// What people type, and the root of the app's files, folders and tags.
pub const APP_COMMAND: &str = env!("APP_COMMAND");
/// The prefix of the app's environment variables.
pub const APP_ENV: &str = env!("APP_ENV");

/// The commands the app had before, newest first: what an old install left is carried over.
pub fn previous() -> impl Iterator<Item = &'static str> {
    env!("APP_PREVIOUS").split(',').filter(|p| !p.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_comes_from_cargo_toml() {
        assert_eq!(APP_COMMAND, env!("CARGO_PKG_NAME"));
        assert!(!APP_NAME.is_empty() && APP_NAME.chars().all(|c| c.is_ascii_alphanumeric() || c == ' '), "{APP_NAME}");
        assert_eq!(APP_ENV, APP_COMMAND.to_ascii_uppercase().replace('-', "_"));
        assert!(previous().all(|p| p != APP_COMMAND), "a previous name is not the current one");
    }
}
