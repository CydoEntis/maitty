//! The app's command and environment prefix, from the root `Cargo.toml` through `build.rs`.

/// What people type, and the root of the app's files, folders and tags.
pub const APP_COMMAND: &str = env!("APP_COMMAND");
/// The prefix of the app's environment variables.
pub const APP_ENV: &str = env!("APP_ENV");
