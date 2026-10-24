//! A launcher per agent, first on the PATH of every the app shell pane. A shell function is
//! easy to shadow (an alias, a shim manager, a shell started inside the pane, `command
//! claude`); a program found through PATH is not. The launcher asks `the app agent-dir` for the
//! folder to start in, goes there, and runs the next program of that name on PATH.
//!
//! Who may ask is decided by the daemon, not here: `agent-dir` sends the pane's own
//! `<APP>_TERM_ID` and `<APP>_PANE_TOKEN`, and a pane that is not a plain shell in a repo's
//! main checkout gets nothing back (`Command::AgentWorktree`).

use std::ffi::{OsStr, OsString};
use crate::brand::{APP_COMMAND};
use std::path::{Path, PathBuf};

/// Set for the one command a shell function has already asked about, so the launcher the
/// function then reaches through PATH does not ask again.
pub const DONE_VAR: &str = concat!(env!("APP_ENV"), "_AGENT_DIR_DONE");

/// First lines of every launcher the app writes: how `install` knows a file is its own to
/// replace or remove.
const MARKER: &str = concat!(env!("APP_COMMAND"), "-agent-launcher");

/// Where the launchers live.
pub fn dir() -> PathBuf {
    crate::config::data_dir().join("shell").join("bin")
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The POSIX shell launcher for `agent`. `exe` is the app's path and `dir` the folder the
/// launchers live in (taken off PATH before the real agent is looked for, so it can never
/// run itself).
pub fn unix_script(agent: &str, exe: &str, dir: &str) -> String {
    format!(
        r#"#!/bin/sh
# {MARKER}: written by {APP_COMMAND}. Asks {APP_COMMAND} where {agent} should start (a new worktree, in a repo's
# main checkout), goes there, and runs the real {agent}.
agent={agent_q}
{APP_COMMAND}={exe_q}
shim_dir={dir_q}
old_ifs=$IFS
IFS=:
set -f
clean=
real=
for p in $PATH; do
  if [ "$p" = "$shim_dir" ] || [ "$p" = "$shim_dir/" ]; then
    continue
  fi
  clean=${{clean:+$clean:}}$p
  if [ -z "$real" ] && [ -n "$p" ] && [ -f "$p/$agent" ] && [ -x "$p/$agent" ]; then
    real=$p/$agent
  fi
done
IFS=$old_ifs
set +f
if [ -z "$real" ]; then
  printf '%s: command not found\n' "$agent" >&2
  exit 127
fi
if [ -z "${{{DONE_VAR}:-}}" ]; then
  d=$("${APP_COMMAND}" agent-dir "$agent" "$@" </dev/null) || d=
  if [ -n "$d" ]; then
    cd -- "$d" || printf '{APP_COMMAND}: could not enter %s; starting here\n' "$d" >&2
  fi
fi
unset {DONE_VAR}
PATH=$clean
export PATH
exec "$real" "$@"
"#,
        agent_q = sh_quote(agent),
        exe_q = sh_quote(exe),
        dir_q = sh_quote(dir),
    )
}

/// The Windows launcher for `agent`: a batch file that hands everything to `<app>
/// agent-launch`, which does what `unix_script` does. `%*` passes the arguments exactly as
/// they were typed, so the quoting the user wrote reaches the agent. None when the path
/// cannot be written in a batch file safely (cmd reads it in the OEM code page).
pub fn windows_script(agent: &str, exe: &str) -> Option<String> {
    let safe = exe.is_ascii() && !exe.contains(['"', '\r', '\n']);
    safe.then(|| format!("@echo off\r\nrem {MARKER}: written by {APP_COMMAND}.\r\n\"{}\" agent-launch {agent} %*\r\nexit /b %ERRORLEVEL%\r\n", exe.replace('%', "%%")))
}

fn launcher_name(agent: &str) -> String {
    if cfg!(windows) { format!("{agent}.cmd") } else { agent.to_string() }
}

fn launcher_text(agent: &str, exe: &str, dir: &Path) -> Option<String> {
    if cfg!(windows) { windows_script(agent, exe) } else { Some(unix_script(agent, exe, &dir.display().to_string())) }
}

fn write_launcher(path: &Path, text: &str) -> std::io::Result<()> {
    if std::fs::read_to_string(path).ok().as_deref() == Some(text) {
        return Ok(());
    }
    // Written aside and renamed in, so a pane never runs half a launcher.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, path)
}

/// Write a launcher in `dir` for each of `agents` and remove the ones the app wrote for agents
/// no longer listed. Files the app did not write are left alone.
pub fn install_in(dir: &Path, agents: &[String], exe: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let wanted: Vec<String> = agents.iter().map(|a| launcher_name(a)).collect();
    for entry in std::fs::read_dir(dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let ours = std::fs::read_to_string(entry.path()).is_ok_and(|t| t.contains(MARKER));
        if ours && !wanted.contains(&name) {
            std::fs::remove_file(entry.path())?;
        }
    }
    for agent in agents {
        if let Some(text) = launcher_text(agent, exe, dir) {
            write_launcher(&dir.join(launcher_name(agent)), &text)?;
        }
    }
    Ok(())
}

/// `install_in` for the app's own folder; the folder when there is something in it to run.
pub fn install(agents: &[String], exe: &str) -> std::io::Result<Option<PathBuf>> {
    let dir = dir();
    install_in(&dir, agents, exe)?;
    Ok((!agents.is_empty()).then_some(dir))
}

/// `current` (a PATH) with `dir` first and nowhere else.
pub fn path_with(dir: &Path, current: Option<&OsStr>) -> OsString {
    let rest = current.map(|p| std::env::split_paths(p).filter(|e| e != dir).collect::<Vec<_>>()).unwrap_or_default();
    std::env::join_paths(std::iter::once(dir.to_path_buf()).chain(rest)).unwrap_or_else(|_| dir.as_os_str().to_os_string())
}

/// `path` without `skip`.
pub fn path_without(skip: &Path, path: &OsStr) -> OsString {
    std::env::join_paths(std::env::split_paths(path).filter(|e| e != skip)).unwrap_or_else(|_| path.to_os_string())
}

/// The first file named `agent` (with one of `extensions` after it, in order, when it has
/// none; Windows' PATHEXT) in a PATH entry other than `skip`.
pub fn find_on_path(agent: &str, path: &OsStr, skip: &Path, extensions: &[String]) -> Option<PathBuf> {
    let names: Vec<String> = if Path::new(agent).extension().is_some() || extensions.is_empty() {
        vec![agent.to_string()]
    } else {
        extensions.iter().map(|e| format!("{agent}{e}")).collect()
    };
    std::env::split_paths(path).filter(|d| !d.as_os_str().is_empty() && d != skip).find_map(|d| names.iter().map(|n| d.join(n)).find(|p| p.is_file()))
}

/// Shell code for the rc of a pane's shell that keeps `dir` first on PATH, so a manager that
/// puts its own shims in front (mise, asdf, nvm) cannot get ahead of the launchers.
/// It runs just before each prompt (bash), each command (zsh) or each command line (fish).
pub fn keep_first(shell: &str, dir: &Path) -> String {
    let dir = dir.display().to_string();
    match shell {
        "fish" => {
            let q = format!("'{}'", dir.replace('\\', "\\\\").replace('\'', "\\'"));
            format!("function __{APP_COMMAND}_path --on-event fish_preexec; test \"$PATH[1]\" = {q}; or set -gx PATH {q} (string match -v -- {q} $PATH); end; __{APP_COMMAND}_path\n")
        }
        _ => {
            let hook = if shell == "zsh" { concat!("preexec_functions+=(__", env!("APP_COMMAND"), "_path)") } else { concat!("__", env!("APP_COMMAND"), "_nl=$'\\n'; PROMPT_COMMAND=\"${PROMPT_COMMAND:+$PROMPT_COMMAND$__", env!("APP_COMMAND"), "_nl}__", env!("APP_COMMAND"), "_path\"") };
            format!(
                "__{APP_COMMAND}_shim={}\n__{APP_COMMAND}_path() {{ case \":$PATH:\" in \":$__{APP_COMMAND}_shim:\"*) return ;; esac; local rest=\":$PATH:\"; rest=${{rest//\":$__{APP_COMMAND}_shim:\"/:}}; rest=${{rest#:}}; PATH=\"$__{APP_COMMAND}_shim:${{rest%:}}\"; }}\n__{APP_COMMAND}_path\n{hook}\n",
                sh_quote(&dir)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unix_launcher_cannot_run_itself() {
        let s = unix_script("claude", concat!("/opt/se shi/", env!("APP_COMMAND")), "/data/shell/bin");
        let strip = s.find("\"$p\" = \"$shim_dir\"").expect("the launcher's own folder is skipped");
        let look = s.find("-f \"$p/$agent\"").expect("the real agent is looked for");
        let run = s.find("exec \"$real\" \"$@\"").expect("and run with the arguments as typed");
        assert!(strip < look && look < run, "{s}");
        assert!(s.contains("PATH=$clean") && s.contains("export PATH"), "the agent gets a PATH without the launcher: {s}");
        assert!(s.contains("shim_dir='/data/shell/bin'") && s.contains(concat!(env!("APP_COMMAND"), "='/opt/se shi/", env!("APP_COMMAND"), "'")), "{s}");
        assert!(s.contains("agent-dir \"$agent\" \"$@\""), "the arguments reach agent-dir: {s}");
        assert!(s.contains("command not found") && s.contains("exit 127"), "a missing agent is reported like a shell would: {s}");
        assert!(s.contains("set -f"), "PATH entries are not globbed");
    }

    #[test]
    fn the_unix_launcher_quotes_what_it_embeds() {
        let s = unix_script("claude", concat!("/it's here/", env!("APP_COMMAND")), "/d'ir");
        assert!(s.contains(concat!(env!("APP_COMMAND"), r"='/it'\''s here/", env!("APP_COMMAND"), r"'")) && s.contains(r"shim_dir='/d'\''ir'"), "{s}");
    }

    #[test]
    fn the_windows_launcher_passes_arguments_through_untouched() {
        let s = windows_script("claude", concat!(r"C:\Users\a b\", env!("APP_COMMAND"), r".exe")).unwrap();
        assert!(s.contains(concat!("\"C:\\Users\\a b\\", env!("APP_COMMAND"), ".exe\" agent-launch claude %*\r\n")) && s.contains("exit /b %ERRORLEVEL%"), "{s}");
        assert!(s.starts_with("@echo off\r\n"));
        assert!(windows_script("claude", concat!(r"C:\100%\", env!("APP_COMMAND"), r".exe")).unwrap().contains(concat!(r"C:\100%%\", env!("APP_COMMAND"), r".exe")), "a percent sign is doubled in a batch file");
        assert!(windows_script("claude", concat!(r"C:\Users\é\", env!("APP_COMMAND"), r".exe")).is_none(), "cmd reads batch files in the OEM code page");
    }

    #[test]
    fn launchers_are_written_when_changed_and_only_our_stale_ones_removed() {
        let tmp = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-shim-{}"), std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        install_in(&tmp, &["claude".into(), "codex".into()], concat!("/x/", env!("APP_COMMAND"))).unwrap();
        std::fs::write(tmp.join("mine"), concat!("not ", env!("APP_COMMAND"), "'s")).unwrap();
        install_in(&tmp, &["claude".into()], concat!("/x/", env!("APP_COMMAND"))).unwrap();
        assert!(tmp.join(launcher_name("claude")).exists());
        assert!(!tmp.join(launcher_name("codex")).exists(), "an agent that is no longer listed loses its launcher");
        assert!(tmp.join("mine").exists(), concat!("files ", env!("APP_COMMAND"), " did not write stay"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(tmp.join("claude")).unwrap().permissions().mode() & 0o111, 0o111, "executable");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn path_helpers_put_the_folder_first_and_take_it_out() {
        let dir = Path::new(concat!("/", env!("APP_COMMAND"), "/bin"));
        let path = std::env::join_paths(["/usr/bin", concat!("/", env!("APP_COMMAND"), "/bin"), "/bin"]).unwrap();
        let first = path_with(dir, Some(&path));
        let parts: Vec<PathBuf> = std::env::split_paths(&first).collect();
        assert_eq!(parts, [PathBuf::from(concat!("/", env!("APP_COMMAND"), "/bin")), "/usr/bin".into(), "/bin".into()], "first, once");
        assert_eq!(std::env::split_paths(&path_without(dir, &first)).count(), 2);
        assert_eq!(std::env::split_paths(&path_with(dir, None)).count(), 1);
    }

    #[test]
    fn the_real_agent_is_found_past_the_launcher_folder_and_with_extensions() {
        let tmp = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-find-{}"), std::process::id()));
        let (shim, real, other) = (tmp.join("shim"), tmp.join("real"), tmp.join("other"));
        for d in [&shim, &real, &other] {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(shim.join("claude.cmd"), "").unwrap();
        std::fs::write(real.join("claude.cmd"), "").unwrap();
        std::fs::write(other.join("claude.exe"), "").unwrap();
        let path = std::env::join_paths([&shim, &real, &other]).unwrap();
        let exts = [".exe".to_string(), ".cmd".to_string()];
        assert_eq!(find_on_path("claude", &path, &shim, &exts), Some(real.join("claude.cmd")), "the first folder wins, then PATHEXT order inside it");
        assert_eq!(find_on_path("claude", &path, &shim, &[]), None, "no extensions: the exact name");
        assert_eq!(find_on_path("claude.cmd", &path, &shim, &exts), Some(real.join("claude.cmd")), "a name with an extension is not extended");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The launcher run for real under bash with a fake `<app>` and a fake agent.
    #[cfg(unix)]
    mod run {
        use super::*;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Output, Stdio};

        struct Rig {
            root: PathBuf,
            shim: PathBuf,
            real: PathBuf,
            work: PathBuf,
            target: PathBuf,
            bash: PathBuf,
        }

        fn script(path: &Path, body: &str) {
            std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        impl Rig {
            fn new(name: &str) -> Option<Rig> {
                let bash = find_on_path("bash", &std::env::var_os("PATH")?, Path::new(""), &[])?;
                let root = std::env::temp_dir().join(format!("{APP_COMMAND}-launch-{name}-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&root);
                let (shim, real, work, target) = (root.join("shim"), root.join("real"), root.join("work"), root.join("target"));
                for d in [&shim, &real, &work, &target] {
                    std::fs::create_dir_all(d).unwrap();
                }
                // `<app> agent-dir …` answers with $REPLY_DIR and records that it was asked.
                script(&root.join(env!("APP_COMMAND")), &format!("[ \"$1\" = agent-dir ] || exit 2\necho \"$@\" >> {}/asked\nprintf '%s' \"$REPLY_DIR\"", root.display()));
                script(&real.join("claude"), "echo \"in $PWD\"\nfor a in \"$@\"; do echo \"arg:$a\"; done\nread line\necho \"stdin:$line\"\nexit 7");
                install_in(&shim, &["claude".into()], &root.join(env!("APP_COMMAND")).display().to_string()).unwrap();
                Some(Rig { root, shim, real, work, target, bash })
            }

            fn path(&self) -> String {
                format!("{}:{}", self.shim.display(), self.real.display())
            }

            /// Run `line` under an interactive bash started in `work`, with stdin "typed".
            fn bash(&self, rc: &str, reply: &str, line: &str, extra: &[(&str, &str)]) -> Output {
                let rcfile = self.root.join("rc");
                std::fs::write(&rcfile, rc).unwrap();
                let mut c = Command::new(&self.bash);
                c.args(["--noprofile", "--rcfile"]).arg(&rcfile).args(["-i", "-c", line]).current_dir(&self.work).env_clear().env("PATH", self.path()).env("REPLY_DIR", reply);
                for (k, v) in extra {
                    c.env(k, v);
                }
                let mut child = c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
                std::io::Write::write_all(&mut child.stdin.take().unwrap(), b"typed\n").unwrap();
                child.wait_with_output().unwrap()
            }

            fn asked(&self) -> String {
                std::fs::read_to_string(self.root.join("asked")).unwrap_or_default()
            }
        }

        impl Drop for Rig {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.root);
            }
        }

        fn stdout(o: &Output) -> String {
            String::from_utf8_lossy(&o.stdout).into_owned()
        }

        #[test]
        fn the_agent_starts_in_the_folder_agent_dir_names_with_its_arguments_stdin_and_exit_code() {
            let Some(rig) = Rig::new("dir") else { return };
            let target = rig.target.canonicalize().unwrap();
            let o = rig.bash("", &target.display().to_string(), "claude --model 'a b' \"it's\" '' x*", &[]);
            let out = stdout(&o);
            assert!(out.contains(&format!("in {}", target.display())), "{out}");
            for want in ["arg:--model", "arg:a b", "arg:it's", "arg:", "arg:x*", "stdin:typed"] {
                assert!(out.lines().any(|l| l == want), "{want} in {out}");
            }
            assert_eq!(o.status.code(), Some(7), "the agent's exit code is the command's");
            assert!(rig.asked().contains("agent-dir claude --model a b it's  x*"), "agent-dir saw the arguments: {}", rig.asked());
        }

        #[test]
        fn an_empty_answer_starts_the_agent_where_it_was() {
            let Some(rig) = Rig::new("empty") else { return };
            let here = rig.work.canonicalize().unwrap();
            let out = stdout(&rig.bash("", "", "claude", &[]));
            assert!(out.contains(&format!("in {}", here.display())), "{out}");
        }

        #[test]
        fn a_users_alias_or_function_of_the_same_name_does_not_stop_it_firing_through_path() {
            let Some(rig) = Rig::new("shadow") else { return };
            let target = rig.target.canonicalize().unwrap();
            let rc = "claude() { echo shadowed by a function; }\nalias claude='echo shadowed by an alias'\n";
            let reply = target.display().to_string();
            for line in ["command claude", "exec claude"] {
                let out = stdout(&rig.bash(rc, &reply, line, &[]));
                assert!(out.contains(&format!("in {}", target.display())), "{line}: {out}");
            }
        }

        #[test]
        fn the_real_agent_is_found_past_the_launcher_and_never_run_twice() {
            let Some(rig) = Rig::new("past") else { return };
            // A second launcher folder entry and a trailing slash must not be mistaken for the agent.
            let path = format!("{}/:{}:{}", rig.shim.display(), rig.shim.display(), rig.real.display());
            let o = rig.bash("", "", "claude", &[("PATH", &path)]);
            assert!(stdout(&o).contains("stdin:typed"), "{}", stdout(&o));
            assert_eq!(rig.asked().lines().count(), 1, "asked once: {}", rig.asked());
            let only = rig.bash("", "", "claude", &[("PATH", &rig.shim.display().to_string())]);
            assert_eq!(only.status.code(), Some(127), "no real agent: not found, and no loop");
            assert!(String::from_utf8_lossy(&only.stderr).contains("claude: command not found"));
            assert_eq!(rig.asked().lines().count(), 1, "no worktree is made for an agent that is not installed");
        }

        #[test]
        fn a_manager_putting_its_shims_in_front_is_put_behind_the_launchers_again() {
            let Some(rig) = Rig::new("first") else { return };
            let snippet = keep_first("bash", &rig.shim);
            // The manager's hook runs at the prompt, before ours (the last in PROMPT_COMMAND).
            let line = format!("PATH=/mise/shims:$PATH\n{snippet}\nPROMPT_COMMAND='PATH=/mise/shims:$PATH'\n{snippet}\neval \"$PROMPT_COMMAND\"\necho \"$PATH\"");
            let out = Command::new(&rig.bash).arg("-c").arg(&line).env_clear().env("PATH", rig.path()).output().unwrap();
            let got = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = got.trim().split(':').collect();
            assert_eq!(parts[0], rig.shim.display().to_string(), "first: {got}");
            assert_eq!(parts.iter().filter(|p| **p == rig.shim.display().to_string()).count(), 1, "once: {got}");
            assert!(parts.contains(&"/mise/shims") && parts.contains(&rig.real.display().to_string().as_str()), "nothing else lost: {got}");
        }

        #[test]
        fn a_function_that_already_asked_is_not_asked_again() {
            let Some(rig) = Rig::new("done") else { return };
            let target = rig.target.canonicalize().unwrap();
            let o = rig.bash("", &target.display().to_string(), "claude", &[(DONE_VAR, "1")]);
            assert!(stdout(&o).contains(&format!("in {}", rig.work.canonicalize().unwrap().display())), "{}", stdout(&o));
            assert_eq!(rig.asked(), "", "agent-dir was not run");
        }

        #[test]
        fn the_agent_gets_a_path_without_the_launcher_folder() {
            let Some(rig) = Rig::new("env") else { return };
            script(&rig.real.join("claude"), concat!("echo \"path:$PATH\"\necho \"done:${", env!("APP_ENV"), "_AGENT_DIR_DONE-unset}\""));
            let out = stdout(&rig.bash("", "", "claude", &[(DONE_VAR, "1")]));
            assert!(out.contains(&format!("path:{}\n", rig.real.display())) && out.contains("done:unset"), "{out}");
        }
    }
}
