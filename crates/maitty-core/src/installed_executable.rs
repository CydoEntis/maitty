use anyhow::Result;
use std::path::{Path, PathBuf};

/// Resolve the installed app if an update temporarily moved the running executable aside.
pub fn installed_executable() -> Result<PathBuf> {
    Ok(installed_path(&std::env::current_exe()?))
}

/// Updates may have moved the running image aside while replacing this path.
pub fn installed_path(exe: &Path) -> PathBuf {
    let name = exe.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if name.starts_with(concat!(env!("APP_COMMAND"), ".old")) && name.ends_with(".exe") {
        exe.with_file_name(concat!(env!("APP_COMMAND"), ".exe"))
    } else if let Some(path) = exe.to_str().and_then(|s| s.strip_suffix(" (deleted)")) {
        PathBuf::from(path)
    } else {
        exe.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::installed_path;
    use std::path::Path;

    #[test]
    fn finds_the_replacement_when_the_running_exe_was_moved() {
        assert_eq!(
            installed_path(Path::new(concat!(env!("APP_COMMAND"), ".old.exe"))),
            Path::new(concat!(env!("APP_COMMAND"), ".exe"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn removes_the_deleted_suffix_from_a_running_exe() {
        assert_eq!(
            installed_path(Path::new("/opt/maitty (deleted)")),
            Path::new("/opt/maitty")
        );
    }
}
