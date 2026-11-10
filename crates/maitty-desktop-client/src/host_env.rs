//! Programs the desktop app starts.

use std::ffi::OsStr;
use std::process::Command;

/// A command for `program` that does not inherit an AppImage's bundle (`maitty_shared::host_env`).
pub fn host_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    for (key, value) in maitty_shared::host_env::inherited_bundle_changes() {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }
    command
}
