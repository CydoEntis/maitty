//! A machine's `[[machines]]` entry in `config.local.toml`: the edits live in
//! `maitty-machines`. This is where the terminal app says which file they are made in.

use std::path::PathBuf;

pub use maitty_machines::machine_config::*;

/// Where this computer's own machines are kept: beside the config file, as the loader looks.
/// Tests get a folder of their own, so none of them can write to a real config.
pub fn local_config_path() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir()
            .join(format!(
                concat!(env!("APP_COMMAND"), "-test-{}"),
                std::process::id()
            ))
            .join("config.local.toml");
    }
    crate::config::config_path().with_file_name("config.local.toml")
}
