//! The PATH a login shell gives on the machine the daemon runs on. A daemon started by a
//! non-interactive ssh session (or a service manager) keeps a minimal PATH, and a plain pane
//! is interactive but not a login shell, so an agent installed through `.profile` or
//! `.zprofile` would be found by neither. The daemon asks `$SHELL -ilc` once at start. The
//! client never does: it has no use for the daemon host's PATH.
// The parsing and merging are pure and tested everywhere; only unix runs a shell.
#![cfg_attr(not(unix), allow(dead_code))]

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

/// Long enough for a heavy rc file (nvm, pyenv), short enough that a hanging one doesn't hold
/// up the daemon's first panes.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// A shell that prints more than this before the PATH is not one to wait for.
#[cfg(unix)]
const MAX_OUTPUT: usize = 1 << 20;
const START_MARKER: &str = concat!("__", env!("APP_ENV"), "_LOGIN_PATH_START__");
const END_MARKER: &str = concat!("__", env!("APP_ENV"), "_LOGIN_PATH_END__");
const FALLBACK_SHELL: &str = "/bin/sh";

type Reason = &'static str;
/// Variables set for the probe on top of the daemon's own; empty in production.
type ExtraEnv<'a> = &'a [(&'a str, &'a OsStr)];

/// Set once at daemon start. `None` inside: the probe failed and the current PATH stands.
static RESOLVED: OnceLock<Option<OsString>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Posix,
    Fish,
}

/// Run the probe once, off the event loop. A failure only logs: the daemon starts either way.
pub(crate) async fn resolve_at_start() {
    if !cfg!(unix) {
        return;
    }
    let done = tokio::task::spawn_blocking(|| {
        RESOLVED.get_or_init(|| {
            resolve(
                std::env::var_os("SHELL").as_deref(),
                std::env::var_os("PATH").as_deref(),
                PROBE_TIMEOUT,
                &[],
            )
        });
    })
    .await;
    if let Err(e) = done {
        tracing::warn!("login PATH probe did not finish: {e}");
    }
}

/// The login PATH merged with the daemon's own, when the probe worked.
pub(crate) fn resolved() -> Option<&'static OsStr> {
    RESOLVED.get()?.as_deref()
}

/// What to search for a program: the resolved PATH, else this process's own.
pub(crate) fn effective_path() -> Option<OsString> {
    resolved()
        .map(OsStr::to_os_string)
        .or_else(|| std::env::var_os("PATH"))
}

fn resolve(
    shell_env: Option<&OsStr>,
    current: Option<&OsStr>,
    timeout: Duration,
    env: ExtraEnv,
) -> Option<OsString> {
    let (shell, family) = pick_shell(shell_env);
    let merged = run_probe(&shell, family, timeout, env).and_then(|login| {
        std::env::join_paths(merge(login, current)).map_err(|_| "an entry holds the separator")
    });
    match merged {
        Ok(path) => Some(path),
        Err(reason) => {
            // Not the PATH: it can name the user's folders.
            tracing::warn!(
                shell = %shell.file_name().unwrap_or_default().to_string_lossy(),
                "login PATH not resolved ({reason}); keeping the current PATH"
            );
            None
        }
    }
}

/// `$SHELL` when it is an absolute path to a file and a shell the probe knows how to ask;
/// else `/bin/sh`, which every unix has.
fn pick_shell(shell_env: Option<&OsStr>) -> (PathBuf, Family) {
    let chosen = shell_env.map(PathBuf::from).and_then(|path| {
        let family = family(&path)?;
        (path.is_absolute() && path.is_file()).then_some((path, family))
    });
    chosen.unwrap_or_else(|| (PathBuf::from(FALLBACK_SHELL), Family::Posix))
}

fn family(shell: &Path) -> Option<Family> {
    match shell.file_name()?.to_str()? {
        "fish" => Some(Family::Fish),
        "bash" | "zsh" | "sh" | "dash" | "ksh" => Some(Family::Posix),
        _ => None,
    }
}

/// The argv after the shell: interactive and login, so both the profile and the rc files
/// run. The markers are separate words, so the command line never contains one joined to a
/// PATH. Fish's PATH is a list, hence `string join`.
fn probe_args(family: Family) -> [OsString; 2] {
    let script = match family {
        Family::Posix => format!("printf '%s%s%s\\n' {START_MARKER} \"$PATH\" {END_MARKER}"),
        Family::Fish => {
            format!("printf '%s%s%s\\n' {START_MARKER} (string join : $PATH) {END_MARKER}")
        }
    };
    ["-ilc".into(), script.into()]
}

/// The PATH between the last start marker and the end marker after it; whatever the rc files
/// printed around it is ignored. `None` when the markers are missing or the PATH is empty.
fn parse_output(output: &str) -> Option<Vec<PathBuf>> {
    let from = output.rfind(START_MARKER)? + START_MARKER.len();
    let length = output[from..].find(END_MARKER)?;
    let entries = unique(output[from..from + length].split(':').map(PathBuf::from));
    (!entries.is_empty()).then_some(entries)
}

/// Login entries first, then the daemon's own that the login PATH lacks, so nothing the
/// daemon already found is lost.
fn merge(login: Vec<PathBuf>, current: Option<&OsStr>) -> Vec<PathBuf> {
    unique(
        login
            .into_iter()
            .chain(current.into_iter().flat_map(std::env::split_paths)),
    )
}

/// In order, without empty entries (which mean the working directory) or repeats.
fn unique(entries: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut seen: Vec<PathBuf> = Vec::new();
    for entry in entries {
        if !entry.as_os_str().is_empty() && !seen.contains(&entry) {
            seen.push(entry);
        }
    }
    seen
}

#[cfg(not(unix))]
fn run_probe(_: &Path, _: Family, _: Duration, _: ExtraEnv) -> Result<Vec<PathBuf>, Reason> {
    Err("login shells are not asked on this platform")
}

#[cfg(unix)]
fn run_probe(
    shell: &Path,
    family: Family,
    timeout: Duration,
    env: ExtraEnv,
) -> Result<Vec<PathBuf>, Reason> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let mut command = Command::new(shell);
    command
        .args(probe_args(family))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    scrub_app_env(&mut command);
    command.envs(env.iter().copied());
    // SAFETY: setsid is async-signal-safe and touches no memory the child shares. It leaves
    // the daemon's terminal behind (an interactive shell would fight over it) and makes the
    // shell a group leader, so a timeout can kill what its rc files started too.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(|_| "could not start the shell")?;
    let stdout = child.stdout.take().ok_or("no output pipe")?;
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::Builder::new()
        .name("login-path-read".into())
        .spawn(move || {
            let _ = sender.send(read_path(stdout));
        });
    let outcome = match reader {
        Ok(_) => receiver.recv_timeout(timeout).map_err(|_| "timed out"),
        Err(_) => Err("could not read the shell's output"),
    };
    // SAFETY: the shell is its own group leader (setsid above) and is not reaped until the
    // wait below, so the group number cannot belong to anything else.
    unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
    let _ = child.wait();
    outcome?.ok_or("no PATH in the shell's output")
}

/// Read until the PATH is complete, not until end of file: a daemon an rc file started would
/// hold the pipe open long after the shell has said what we need.
#[cfg(unix)]
fn read_path(mut stdout: impl std::io::Read) -> Option<Vec<PathBuf>> {
    let mut seen = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = stdout.read(&mut chunk).ok().filter(|n| *n > 0)?;
        seen.extend_from_slice(&chunk[..n]);
        if let Some(entries) = parse_output(&String::from_utf8_lossy(&seen)) {
            return Some(entries);
        }
        if seen.len() > MAX_OUTPUT {
            return None;
        }
    }
}

/// The shell is the user's, not a pane: it must not see which pane or conversation or
/// daemon it belongs to.
#[cfg(unix)]
fn scrub_app_env(command: &mut std::process::Command) {
    let prefix = concat!(env!("APP_ENV"), "_");
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name.starts_with(prefix) || name == env!("APP_ENV") {
            command.env_remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(entries: &[&str]) -> Vec<PathBuf> {
        entries.iter().map(PathBuf::from).collect()
    }

    fn output(path: &str) -> String {
        format!("{START_MARKER}{path}{END_MARKER}\n")
    }

    fn script(family: Family) -> String {
        probe_args(family)[1].to_string_lossy().into_owned()
    }

    #[test]
    fn every_shell_family_is_asked_interactive_and_login() {
        for family in [Family::Posix, Family::Fish] {
            assert_eq!(probe_args(family)[0], "-ilc");
        }
    }

    #[test]
    fn posix_shells_print_path_between_the_markers() {
        assert_eq!(
            script(Family::Posix),
            format!("printf '%s%s%s\\n' {START_MARKER} \"$PATH\" {END_MARKER}")
        );
    }

    #[test]
    fn fish_joins_its_path_list_with_colons() {
        assert_eq!(
            script(Family::Fish),
            format!("printf '%s%s%s\\n' {START_MARKER} (string join : $PATH) {END_MARKER}")
        );
    }

    #[test]
    fn shells_are_told_apart_by_file_name() {
        for (shell, expected) in [
            ("/bin/bash", Some(Family::Posix)),
            ("/usr/bin/zsh", Some(Family::Posix)),
            ("/bin/sh", Some(Family::Posix)),
            ("/opt/homebrew/bin/fish", Some(Family::Fish)),
            ("/usr/bin/nu", None),
        ] {
            assert_eq!(family(Path::new(shell)), expected, "{shell}");
        }
    }

    #[test]
    fn text_before_and_after_the_markers_is_ignored() {
        let printed = format!(
            "Welcome back!\nlast login: today\n{}motd: bye\n",
            output("/usr/bin:/opt/tools")
        );
        assert_eq!(
            parse_output(&printed),
            Some(paths(&["/usr/bin", "/opt/tools"]))
        );
    }

    #[test]
    fn banner_without_a_final_newline_shares_the_marker_line() {
        let printed = format!("banner{}", output("/usr/bin"));
        assert_eq!(parse_output(&printed), Some(paths(&["/usr/bin"])));
    }

    #[test]
    fn entries_with_spaces_dollars_and_quotes_stay_literal() {
        let weird = "/opt/my tools/bin:/home/a$b/\"q\":/it's/'x'";
        assert_eq!(
            parse_output(&output(weird)),
            Some(paths(&[
                "/opt/my tools/bin",
                "/home/a$b/\"q\"",
                "/it's/'x'"
            ]))
        );
    }

    #[test]
    fn duplicate_and_empty_entries_are_dropped_in_order() {
        assert_eq!(
            parse_output(&output(":/b::/a:/b:/a:")),
            Some(paths(&["/b", "/a"]))
        );
    }

    #[test]
    fn output_without_markers_is_a_failure() {
        assert_eq!(parse_output("just a banner\n/usr/bin:/bin\n"), None);
        assert_eq!(parse_output(&format!("{START_MARKER}/usr/bin")), None);
    }

    #[test]
    fn an_empty_path_is_a_failure() {
        assert_eq!(parse_output(&output("")), None);
        assert_eq!(parse_output(&output("::")), None);
    }

    #[test]
    fn the_last_start_marker_wins() {
        let printed = format!("{START_MARKER}/trace {END_MARKER}\n{}", output("/real"));
        assert_eq!(parse_output(&printed), Some(paths(&["/real"])));
    }

    #[test]
    fn merge_puts_login_entries_first_then_current_ones_it_lacks() {
        let current = std::env::join_paths(["/bin", "/login", "/usr/local/bin"]).unwrap();
        assert_eq!(
            merge(paths(&["/login", "/opt/my tools"]), Some(&current)),
            paths(&["/login", "/opt/my tools", "/bin", "/usr/local/bin"])
        );
    }

    #[test]
    fn merge_without_a_current_path_is_the_login_path() {
        assert_eq!(merge(paths(&["/a", "/b"]), None), paths(&["/a", "/b"]));
    }

    #[test]
    fn a_pane_path_keeps_the_launchers_first_and_the_login_path_after() {
        let launchers = Path::new("/data/shell/bin");
        let current = std::env::join_paths(["/usr/bin", "/data/shell/bin"]).unwrap();
        let login =
            std::env::join_paths(merge(paths(&["/opt/my tools/bin"]), Some(&current))).unwrap();
        let pane = crate::shim::path_with(launchers, Some(&login));
        assert_eq!(
            std::env::split_paths(&pane).collect::<Vec<_>>(),
            paths(&["/data/shell/bin", "/opt/my tools/bin", "/usr/bin"])
        );
    }

    #[test]
    fn nothing_is_resolved_until_the_daemon_starts() {
        assert!(resolved().is_none());
        assert_eq!(effective_path(), std::env::var_os("PATH"));
    }

    #[test]
    fn an_unknown_or_relative_shell_falls_back_to_sh() {
        for shell in [
            None,
            Some("nu"),
            Some("bash"),
            Some("/definitely/not/a/shell"),
        ] {
            let (path, family) = pick_shell(shell.map(OsStr::new));
            assert_eq!(path, Path::new(FALLBACK_SHELL), "{shell:?}");
            assert_eq!(family, Family::Posix);
        }
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    const TIMEOUT: Duration = Duration::from_secs(20);

    struct Home(PathBuf);

    impl Home {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "login-path-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, name: &str, text: &str) {
            let file = self.0.join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }

        /// A folder with a space in its name that holds an executable `tool`.
        fn tool_dir(&self) -> PathBuf {
            let dir = self.0.join("my tools").join("bin");
            std::fs::create_dir_all(&dir).unwrap();
            let tool = dir.join("tool");
            std::fs::write(&tool, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
            dir
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn shell_on_path(name: &str) -> Option<PathBuf> {
        let found = crate::shim::find_on_path(name, &std::env::var_os("PATH")?, Path::new(""), &[]);
        if found.is_none() {
            eprintln!("skipped: {name} is not installed");
        }
        found
    }

    fn entries(path: Option<OsString>) -> Vec<PathBuf> {
        std::env::split_paths(&path.expect("a PATH was resolved")).collect()
    }

    fn resolve_in(
        home: &Home,
        shell: Option<&Path>,
        current: &str,
        timeout: Duration,
    ) -> Option<OsString> {
        let root = home.0.as_os_str();
        resolve(
            shell.map(Path::as_os_str),
            Some(OsStr::new(current)),
            timeout,
            &[("HOME", root), ("ZDOTDIR", root), ("XDG_CONFIG_HOME", root)],
        )
    }

    fn assert_profile_dir_leads(resolved: Option<OsString>, dir: &Path) {
        let all = entries(resolved);
        assert_eq!(all.first().map(PathBuf::as_path), Some(dir), "{all:?}");
        assert!(all.contains(&PathBuf::from("/usr/bin")), "{all:?}");
    }

    fn profile_adding(dir: &Path) -> String {
        format!(
            "echo 'welcome to the profile'\nprintf 'no newline'\nexport PATH=\"{}:$PATH\"\n",
            dir.display()
        )
    }

    #[test]
    fn bash_login_profile_that_prints_text_adds_a_path_with_spaces() {
        let Some(bash) = shell_on_path("bash") else {
            return;
        };
        let home = Home::new();
        let dir = home.tool_dir();
        home.write(".bash_profile", &profile_adding(&dir));
        assert_profile_dir_leads(resolve_in(&home, Some(&bash), "/usr/bin", TIMEOUT), &dir);
    }

    #[test]
    fn zsh_reads_zprofile_and_zshrc() {
        let Some(zsh) = shell_on_path("zsh") else {
            return;
        };
        let home = Home::new();
        let dir = home.tool_dir();
        home.write(".zprofile", "echo 'zprofile banner'\n");
        home.write(
            ".zshrc",
            &format!(
                "echo 'zshrc banner'\nexport PATH=\"{}:$PATH\"\n",
                dir.display()
            ),
        );
        assert_profile_dir_leads(resolve_in(&home, Some(&zsh), "/usr/bin", TIMEOUT), &dir);
    }

    #[test]
    fn fish_config_sets_its_path_list() {
        let Some(fish) = shell_on_path("fish") else {
            return;
        };
        let home = Home::new();
        let dir = home.tool_dir();
        home.write(
            "fish/config.fish",
            &format!(
                "echo 'fish banner'\nset -gx PATH \"{}\" $PATH\n",
                dir.display()
            ),
        );
        assert_profile_dir_leads(resolve_in(&home, Some(&fish), "/usr/bin", TIMEOUT), &dir);
    }

    #[test]
    fn sh_is_used_when_the_shell_variable_is_unset_or_invalid() {
        if !Path::new(FALLBACK_SHELL).is_file() {
            eprintln!("skipped: no {FALLBACK_SHELL}");
            return;
        }
        let home = Home::new();
        let dir = home.tool_dir();
        home.write(".profile", &profile_adding(&dir));
        for shell in [None, Some(Path::new("relative-shell"))] {
            let resolved = resolve_in(&home, shell, "/usr/bin", TIMEOUT);
            assert_profile_dir_leads(resolved, &dir);
        }
    }

    #[test]
    fn a_login_only_agent_is_found_by_the_executable_lookup() {
        let Some(bash) = shell_on_path("bash") else {
            return;
        };
        let home = Home::new();
        let dir = home.tool_dir();
        home.write(".bash_profile", &profile_adding(&dir));
        let path = resolve_in(&home, Some(&bash), "/usr/bin", TIMEOUT);
        let expected = std::fs::canonicalize(dir.join("tool")).unwrap();
        let without =
            crate::proc::resolve_executable_in(Path::new("tool"), Some(OsStr::new("/usr/bin")));
        assert_ne!(without.ok(), Some(expected.clone()));
        let found = crate::proc::resolve_executable_in(Path::new("tool"), path.as_deref()).unwrap();
        assert_eq!(found, expected);
    }

    #[test]
    fn a_hanging_rc_file_times_out_and_the_current_path_stands() {
        let Some(bash) = shell_on_path("bash") else {
            return;
        };
        let home = Home::new();
        home.write(".bash_profile", "sleep 3600\n");
        let started = std::time::Instant::now();
        let resolved = resolve_in(&home, Some(&bash), "/usr/bin", Duration::from_millis(300));
        assert_eq!(resolved, None);
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
