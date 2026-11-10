//! The environment a program outside the desktop app's bundle should see.
//!
//! An AppImage runs the app through `AppRun`, which points the loader, Python, Perl, GTK and
//! GStreamer at the libraries bundled under its mount (`/tmp/.mount_…`). Those settings are for
//! the app alone. A daemon started with them hands them to every session, where the system's
//! Python then fails to start ("No module named 'encodings'") and system programs load the
//! bundle's libraries instead of their own.

/// Where the bundle is mounted, as AppRun says it.
const MOUNT_VARIABLE: &str = "APPDIR";
/// What AppRun sets about the bundle itself, which means nothing to another program.
const BUNDLE_ONLY: &[&str] = &["APPIMAGE", "ARGV0", "OWD"];
/// How every AppImage mount is named, so a mount of an earlier run is recognised too.
const MOUNT_MARK: &str = "/.mount_";
const LIST_SEPARATOR: char = ':';

/// What to change in this process's own environment; nothing off Linux, where there is no AppImage.
pub fn inherited_bundle_changes() -> Vec<(String, Option<String>)> {
    if !cfg!(target_os = "linux") {
        return Vec::new();
    }
    bundle_changes(
        std::env::vars_os()
            .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?))),
    )
}

/// What to change in `vars` for a program outside the bundle: each variable with its new value,
/// or `None` to drop it. Path lists lose the entries inside a mount and keep the rest, in order.
/// Nothing when no variable mentions a mount, so an installed build changes nothing.
pub fn bundle_changes(
    vars: impl IntoIterator<Item = (String, String)>,
) -> Vec<(String, Option<String>)> {
    let vars: Vec<(String, String)> = vars.into_iter().collect();
    let mount = vars
        .iter()
        .find(|(key, _)| key == MOUNT_VARIABLE)
        .map(|(_, value)| value.trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty());
    let bundled = |entry: &str| {
        entry.contains(MOUNT_MARK) || mount.as_deref().is_some_and(|mount| entry.starts_with(mount))
    };
    let mut changes: Vec<(String, Option<String>)> = Vec::new();
    for (key, value) in &vars {
        if !value.split(LIST_SEPARATOR).any(bundled) {
            continue;
        }
        let kept: Vec<&str> = value
            .split(LIST_SEPARATOR)
            .filter(|entry| !entry.is_empty() && !bundled(entry))
            .collect();
        let rest = (!kept.is_empty()).then(|| kept.join(&LIST_SEPARATOR.to_string()));
        changes.push((key.clone(), rest));
    }
    if changes.is_empty() {
        return changes;
    }
    for key in BUNDLE_ONLY {
        if vars.iter().any(|(name, _)| name == key) && !changes.iter().any(|(name, _)| name == key)
        {
            changes.push(((*key).to_owned(), None));
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::bundle_changes;

    fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    fn change<'a>(changes: &'a [(String, Option<String>)], key: &str) -> Option<&'a Option<String>> {
        changes
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    #[test]
    fn an_installed_build_changes_nothing() {
        let changes = bundle_changes(vars(&[
            ("PATH", "/usr/local/bin:/usr/bin"),
            ("PYTHONPATH", "/home/casey/lib"),
            ("ARGV0", "zsh"),
        ]));
        assert!(changes.is_empty());
    }

    #[test]
    fn python_is_no_longer_pointed_at_the_bundle() {
        let changes = bundle_changes(vars(&[
            ("APPDIR", "/tmp/.mount_MaittyAbc"),
            ("PYTHONHOME", "/tmp/.mount_MaittyAbc/usr/"),
            ("PYTHONPATH", "/tmp/.mount_MaittyAbc/usr/share/pyshared/:"),
        ]));
        assert_eq!(change(&changes, "PYTHONHOME"), Some(&None));
        assert_eq!(change(&changes, "PYTHONPATH"), Some(&None));
        assert_eq!(change(&changes, "APPDIR"), Some(&None));
    }

    #[test]
    fn a_path_list_keeps_what_is_the_users_own_in_order() {
        let changes = bundle_changes(vars(&[
            ("APPDIR", "/tmp/.mount_MaittyAbc"),
            (
                "PATH",
                "/tmp/.mount_MaittyAbc/usr/bin/:/home/casey/.local/bin:/usr/bin",
            ),
            ("LD_LIBRARY_PATH", "/tmp/.mount_MaittyAbc/usr/lib/:/opt/cuda/lib"),
            ("HOME", "/home/casey"),
        ]));
        assert_eq!(
            change(&changes, "PATH"),
            Some(&Some("/home/casey/.local/bin:/usr/bin".to_owned()))
        );
        assert_eq!(
            change(&changes, "LD_LIBRARY_PATH"),
            Some(&Some("/opt/cuda/lib".to_owned()))
        );
        assert_eq!(change(&changes, "HOME"), None);
    }

    #[test]
    fn a_mount_left_by_an_earlier_run_goes_too() {
        let changes = bundle_changes(vars(&[
            ("APPDIR", "/tmp/.mount_MaittyNew"),
            (
                "PYTHONPATH",
                "/tmp/.mount_MaittyNew/usr/share/pyshared/:/tmp/.mount_MaittyOld/usr/share/pyshared/:/home/casey/lib",
            ),
        ]));
        assert_eq!(
            change(&changes, "PYTHONPATH"),
            Some(&Some("/home/casey/lib".to_owned()))
        );
    }

    #[test]
    fn what_only_describes_the_bundle_is_dropped_with_it() {
        let changes = bundle_changes(vars(&[
            ("APPDIR", "/tmp/.mount_MaittyAbc"),
            ("APPIMAGE", "/home/casey/Apps/Maitty.AppImage"),
            ("ARGV0", "./Maitty.AppImage"),
            ("OWD", "/home/casey"),
        ]));
        for key in ["APPIMAGE", "ARGV0", "OWD"] {
            assert_eq!(change(&changes, key), Some(&None), "{key}");
        }
    }
}
