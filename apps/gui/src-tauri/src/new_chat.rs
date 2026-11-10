//! What the new chat page asks of this computer: which agent programs are installed, a folder's
//! branches and worktrees, and the two git actions it takes (switch a branch, add a worktree).
//! Git runs as a program with its arguments (never through a shell), off the UI thread, and is
//! stopped after `GIT_TIMEOUT`.

use serde::Serialize;
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

const GIT_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(20);
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The agents the page knows about: the program's name and where its official setup page is.
const AGENTS: &[(&str, &str)] = &[
    ("claude", "https://docs.claude.com/en/docs/claude-code/setup"),
    ("codex", "https://github.com/openai/codex"),
    ("gemini", "https://github.com/google-gemini/gemini-cli"),
    ("opencode", "https://opencode.ai"),
    ("cursor-agent", "https://cursor.com/cli"),
];

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Branch {
    name: String,
    current: bool,
    /// Unix seconds of the branch's last commit.
    when: Option<i64>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Worktree {
    path: String,
    branch: Option<String>,
    main: bool,
    when: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct BranchInfo {
    current: Option<String>,
    branches: Vec<Branch>,
    worktrees: Vec<Worktree>,
}

/// `git for-each-ref --format=%(HEAD)\t%(refname:short)\t%(committerdate:unix)` output.
fn parse_branches(text: &str) -> Vec<Branch> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let head = parts.next()?;
            let name = parts.next()?.trim();
            if name.is_empty() {
                return None;
            }
            Some(Branch {
                name: name.to_string(),
                current: head.trim() == "*",
                when: parts.next().and_then(|seconds| seconds.trim().parse().ok()),
            })
        })
        .collect()
}

/// `git worktree list --porcelain` output: blocks of `worktree <path>`, `HEAD <sha>` and
/// `branch refs/heads/<name>` (or `detached`). The first block is the main checkout.
fn parse_worktrees(text: &str, branches: &[Branch]) -> Vec<Worktree> {
    text.split("\n\n")
        .filter_map(|block| {
            let mut path = None;
            let mut branch = None;
            for line in block.lines() {
                if let Some(found) = line.strip_prefix("worktree ") {
                    path = Some(found.trim().to_string());
                } else if let Some(found) = line.strip_prefix("branch ") {
                    branch = Some(found.trim().trim_start_matches("refs/heads/").to_string());
                }
            }
            Some((path?, branch))
        })
        .enumerate()
        .map(|(at, (path, branch))| {
            let when = branch
                .as_deref()
                .and_then(|name| branches.iter().find(|b| b.name == name))
                .and_then(|b| b.when);
            Worktree { path, branch, main: at == 0, when }
        })
        .collect()
}

/// A branch name safe to hand to git as an argument: plain ref characters, and nothing git would
/// read as an option.
fn valid_branch(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && !name.starts_with(['-', '/', '.'])
        && !name.ends_with(['/', '.'])
        && !name.ends_with(".lock")
        && !name.contains("..")
        && !name.contains("//")
        && !name.contains("@{")
        && name != "@"
        && !name.chars().any(|c| c.is_whitespace() || c.is_control() || "~^:?*[\\".contains(c))
}

/// The sibling folder a branch's worktree goes in: `<repo>-<branch with / as ->`.
fn worktree_folder(repo: &Path, branch: &str) -> Option<PathBuf> {
    let name = repo.file_name()?.to_string_lossy().into_owned();
    Some(repo.parent()?.join(format!("{name}-{}", branch.replace('/', "-"))))
}

/// Whether `name` is a program in one of the folders of `path`; on Windows with any of `extensions`.
fn on_path(name: &str, path: &OsStr, extensions: &[&str]) -> bool {
    std::env::split_paths(path).any(|dir| {
        extensions
            .iter()
            .any(|extension| dir.join(format!("{name}{extension}")).is_file())
    })
}

fn path_extensions() -> &'static [&'static str] {
    if cfg!(windows) {
        &["", ".exe", ".cmd", ".bat", ".com"]
    } else {
        &[""]
    }
}

fn run_git(folder: &Path, args: &[&str]) -> Result<String, String> {
    run_program("git", folder, args, GIT_TIMEOUT)
}

/// Runs `program` in `folder` with its arguments (never through a shell) and answers what it
/// printed, or the last line of what it complained about. It is stopped after `limit`.
pub(crate) fn run_program(program: &str, folder: &Path, args: &[&str], limit: Duration) -> Result<String, String> {
    if !folder.is_dir() {
        return Err("that folder is gone".into());
    }
    let mut command = maitty_desktop_client::host_command(program);
    command
        .args(args)
        .current_dir(folder)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|error| format!("could not run {program}: {error}"))?;
    // The pipes are drained on their own threads so a large answer cannot block git while we wait.
    let mut out = child.stdout.take().ok_or_else(|| format!("{program} gave no output"))?;
    let mut err = child.stderr.take().ok_or_else(|| format!("{program} gave no output"))?;
    let out = std::thread::spawn(move || {
        let mut text = String::new();
        out.read_to_string(&mut text).map(|_| text)
    });
    let err = std::thread::spawn(move || {
        let mut text = String::new();
        err.read_to_string(&mut text).map(|_| text)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| format!("{program}: {error}"))? {
            break status;
        }
        if started.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{program} took too long"));
        }
        std::thread::sleep(POLL);
    };
    let text = out.join().map_err(|_| format!("{program} output failed"))?.map_err(|error| format!("{program}: {error}"))?;
    if status.success() {
        return Ok(text);
    }
    let said = err.join().ok().and_then(Result::ok).unwrap_or_default();
    Err(complaint(&said))
}

/// The one line of what a program complained about that says what went wrong: git's `fatal:` or
/// `error:` line when it has one (what follows it is advice), else the last line.
fn complaint(said: &str) -> String {
    let lines = || said.lines().map(str::trim).filter(|line| !line.is_empty());
    lines()
        .find(|line| line.starts_with("fatal:") || line.starts_with("error:"))
        .or_else(|| lines().next_back())
        .unwrap_or("it failed")
        .to_string()
}

fn branch_info(folder: &Path) -> Result<Option<BranchInfo>, String> {
    if run_git(folder, &["rev-parse", "--is-inside-work-tree"]).is_err() {
        return Ok(None);
    }
    let branches = parse_branches(&run_git(
        folder,
        &["for-each-ref", "--sort=-committerdate", "--format=%(HEAD)\t%(refname:short)\t%(committerdate:unix)", "refs/heads"],
    )?);
    let worktrees = parse_worktrees(&run_git(folder, &["worktree", "list", "--porcelain"])?, &branches);
    let current = branches.iter().find(|branch| branch.current).map(|branch| branch.name.clone());
    Ok(Some(BranchInfo { current, branches, worktrees }))
}

fn switch_branch(folder: &Path, branch: &str, create: bool) -> Result<(), String> {
    if !valid_branch(branch) {
        return Err("that is not a branch name".into());
    }
    let args: &[&str] = if create { &["switch", "-c", branch] } else { &["switch", branch] };
    run_git(folder, args).map(drop)
}

fn add_worktree(folder: &Path, branch: &str, create: bool) -> Result<String, String> {
    if !valid_branch(branch) {
        return Err("that is not a branch name".into());
    }
    let root = PathBuf::from(run_git(folder, &["rev-parse", "--show-toplevel"])?.trim());
    let target = worktree_folder(&root, branch).ok_or("could not place the worktree")?;
    if target.exists() {
        return Err(format!("{} already exists", target.display()));
    }
    let target_text = target.to_string_lossy().into_owned();
    let args: Vec<&str> = if create {
        vec!["worktree", "add", "-b", branch, &target_text]
    } else {
        vec!["worktree", "add", &target_text, branch]
    };
    run_git(&root, &args)?;
    Ok(target_text)
}

/// The names of the agent programs found on this computer's PATH.
#[tauri::command]
pub async fn agents_installed() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let path = std::env::var_os("PATH").unwrap_or_default();
        AGENTS
            .iter()
            .filter(|(name, _)| on_path(name, &path, path_extensions()))
            .map(|(name, _)| (*name).to_string())
            .collect()
    })
    .await
    .map_err(|error| error.to_string())
}

/// A folder's branches and worktrees, or nothing when it is not in a git repository.
#[tauri::command]
pub async fn git_branches(folder: String) -> Result<Option<BranchInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || branch_info(Path::new(&folder)))
        .await
        .map_err(|error| error.to_string())?
}

/// Switches the folder's checkout to `branch` (made first when `create`).
#[tauri::command]
pub async fn git_switch_branch(folder: String, branch: String, create: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || switch_branch(Path::new(&folder), &branch, create))
        .await
        .map_err(|error| error.to_string())?
}

/// Adds a worktree of the folder's repository for `branch` (made first when `create`) beside it,
/// and answers its path.
#[tauri::command]
pub async fn git_add_worktree(folder: String, branch: String, create: bool) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || add_worktree(Path::new(&folder), &branch, create))
        .await
        .map_err(|error| error.to_string())?
}

/// Opens an agent's official setup page. Only the pages in `AGENTS` can be opened: the caller names
/// the agent, never the address.
#[tauri::command]
pub fn open_agent_docs(agent: String) -> Result<(), String> {
    let (_, url) = AGENTS
        .iter()
        .find(|(name, _)| *name == agent)
        .ok_or("unknown agent")?;
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    maitty_desktop_client::host_command(program)
        .arg(url)
        .spawn()
        .map(drop)
        .map_err(|error| format!("could not open the page: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branches_read_name_current_and_last_commit() {
        let found = parse_branches("*\tmain\t1700000100\n \tfix/x\t1700000000\n \tbare\t\n");
        assert_eq!(
            found,
            [
                Branch { name: "main".into(), current: true, when: Some(1_700_000_100) },
                Branch { name: "fix/x".into(), current: false, when: Some(1_700_000_000) },
                Branch { name: "bare".into(), current: false, when: None },
            ]
        );
    }

    #[test]
    fn worktrees_read_path_branch_and_mark_the_first_as_main() {
        let branches = parse_branches("*\tmain\t10\n \tagent/grow-1\t20\n");
        let text = "worktree /dev/growth\nHEAD abc\nbranch refs/heads/main\n\nworktree /dev/growth-agent-grow-1\nHEAD def\nbranch refs/heads/agent/grow-1\n\nworktree /dev/loose\nHEAD 123\ndetached\n";
        assert_eq!(
            parse_worktrees(text, &branches),
            [
                Worktree { path: "/dev/growth".into(), branch: Some("main".into()), main: true, when: Some(10) },
                Worktree { path: "/dev/growth-agent-grow-1".into(), branch: Some("agent/grow-1".into()), main: false, when: Some(20) },
                Worktree { path: "/dev/loose".into(), branch: None, main: false, when: None },
            ]
        );
    }

    #[test]
    fn a_branch_name_git_would_read_as_an_option_or_a_range_is_refused() {
        for bad in ["", "-f", "--orphan", "a b", "a..b", ".hidden", "x/", "x.lock", "a~1", "a:b", "a\\b", "@", "a@{1}"] {
            assert!(!valid_branch(bad), "{bad:?}");
        }
        for good in ["main", "fix/limiter-burst", "release/0.15", "agent/grow-028"] {
            assert!(valid_branch(good), "{good:?}");
        }
    }

    #[test]
    fn a_worktree_goes_beside_the_repository_named_for_its_branch() {
        assert_eq!(
            worktree_folder(Path::new("/dev/growth"), "agent/grow-028"),
            Some(PathBuf::from("/dev/growth-agent-grow-028"))
        );
    }

    #[test]
    fn a_program_is_found_on_the_path_by_any_extension() {
        let dir = std::env::temp_dir().join(format!("maitty-newchat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp folder");
        std::fs::write(dir.join("codex.cmd"), "").expect("shim");
        let path = std::env::join_paths([&dir]).expect("path");
        assert!(on_path("codex", &path, &["", ".exe", ".cmd"]));
        assert!(!on_path("codex", &path, &["", ".exe"]));
        assert!(!on_path("claude", &path, &["", ".exe", ".cmd"]));
        std::fs::remove_dir_all(&dir).expect("clean up");
    }

    #[test]
    fn a_real_repository_lists_switches_and_gets_a_worktree() {
        let root = std::env::temp_dir().join(format!("maitty-newchat-git-{}", std::process::id()));
        let repo = root.join("growth");
        std::fs::create_dir_all(&repo).expect("temp repo folder");
        let git = |args: &[&str]| run_git(&repo, args);
        if git(&["init", "-b", "main"]).is_err() {
            return; // no git on this machine: the parsers above still cover the reading
        }
        git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "--allow-empty", "-m", "first"]).expect("commit");

        let info = branch_info(&repo).expect("reads").expect("a repository");
        assert_eq!(info.current.as_deref(), Some("main"));
        assert!(branch_info(&root).expect("a plain folder is fine").is_none());

        switch_branch(&repo, "feature/x", true).expect("creates and switches");
        let info = branch_info(&repo).expect("reads").expect("a repository");
        assert_eq!(info.current.as_deref(), Some("feature/x"));
        assert!(switch_branch(&repo, "-f", false).is_err());

        let made = add_worktree(&repo, "agent/grow-1", true).expect("adds a worktree");
        assert!(Path::new(&made).ends_with("growth-agent-grow-1"));
        let info = branch_info(&repo).expect("reads").expect("a repository");
        assert!(info.worktrees.iter().any(|tree| tree.branch.as_deref() == Some("agent/grow-1")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn every_agent_has_a_setup_page_on_https() {
        assert!(AGENTS.iter().all(|(_, url)| url.starts_with("https://")));
    }
}
