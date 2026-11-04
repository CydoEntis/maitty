//! Reading and writing the small state files the machine code keeps.

use crate::brand::APP_COMMAND;

/// Write a file so a reader (or a crash) never sees half of it: write a temporary file
/// beside it, then swap it in.
pub fn write_atomic(path: &std::path::Path, data: impl AsRef<[u8]>) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{APP_COMMAND}-tmp"));
    std::fs::write(&tmp, data)?;
    keep_mode(path, &tmp)
        .and_then(|()| std::fs::rename(&tmp, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
}

/// Gives the new file the old one's permissions, so swapping it in never widens a private
/// file (a 0600 agent config) to the default mode.
#[cfg(unix)]
fn keep_mode(old: &std::path::Path, new: &std::path::Path) -> std::io::Result<()> {
    match std::fs::metadata(old) {
        Ok(meta) => std::fs::set_permissions(new, meta.permissions()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
fn keep_mode(_old: &std::path::Path, _new: &std::path::Path) -> std::io::Result<()> {
    Ok(())
}

/// Read a JSON state file; one that's there but won't parse is kept as `<name>.bak` (so a
/// bad write or a hand edit loses nothing) and the defaults are used.
pub fn read_state<T: serde::de::DeserializeOwned + Default>(path: &std::path::Path) -> T {
    let Ok(text) = std::fs::read_to_string(path) else {
        return T::default();
    };
    match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let bak = path.with_file_name(format!("{name}.bak"));
            tracing::warn!(
                "{} doesn't parse ({e}); kept it as {}",
                path.display(),
                bak.display()
            );
            let _ = std::fs::copy(path, &bak);
            T::default()
        }
    }
}
