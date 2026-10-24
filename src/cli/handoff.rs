//! `<app> handoff`: the text brief a new agent starts with when another agent's work moves
//! to it. Text only: the new agent gets no conversation, memory or tool results.

use crate::config::Config;
use crate::protocol::{TermId, TermInfo};
use std::path::{Path, PathBuf};

/// Screen lines carried over, from the bottom.
pub const TAIL_LINES: usize = 20;
/// Changed files listed before the rest are counted.
pub const MAX_FILES: usize = 30;
/// The whole brief, which is passed on the command line as the agent's first prompt.
pub const MAX_BRIEF_CHARS: usize = 4000;

/// What the old pane was doing.
pub struct Facts {
    pub pane: TermId,
    pub agent: String,
    /// `TermInfo.name`: the /rename name, else the first prompt.
    pub task: String,
    /// `TermInfo.summary`.
    pub last_prompt: String,
    pub branch: Option<String>,
    pub checkout: PathBuf,
    pub changed: Result<Changed, String>,
    pub screen: String,
}

/// What a checkout changed, as far as git could tell.
#[derive(Debug, PartialEq)]
pub struct Changed {
    pub files: Vec<String>,
    /// Why the branch's committed changes could not be listed; `files` then holds only the
    /// uncommitted paths.
    pub committed_unknown: Option<String>,
}

pub fn brief(facts: &Facts) -> String {
    let head = head(facts);
    let label = "\nrecent output:\n";
    let room = MAX_BRIEF_CHARS.saturating_sub(head.chars().count() + label.chars().count());
    let tail = tail_within(&facts.screen, room);
    if tail.is_empty() {
        return cut(&head, MAX_BRIEF_CHARS);
    }
    format!("{head}{label}{tail}")
}

/// Everything but the recent output.
fn head(f: &Facts) -> String {
    let mut s = format!(
        concat!("Handoff from {} in ", env!("APP_COMMAND"), " pane {}. This is a text summary only: you do not have the previous agent's conversation, memory or tool results, so read the files yourself.\n\n"),
        f.agent, f.pane
    );
    let task = f.task.trim();
    s += &format!("task: {}\n", if task.is_empty() { "unknown" } else { task });
    let last = f.last_prompt.trim();
    if !last.is_empty() && last != task {
        s += &format!("last prompt: {last}\n");
    }
    s += &format!("branch: {}\ncheckout: {}\n", f.branch.as_deref().unwrap_or("none"), f.checkout.display());
    s += &changed_section(&f.changed);
    s
}

fn changed_section(changed: &Result<Changed, String>) -> String {
    let c = match changed {
        Err(why) => return format!("changed files: unknown ({why})\n"),
        Ok(c) => c,
    };
    let mut s = String::from(if c.files.is_empty() { "changed files: none\n" } else { "changed files:\n" });
    for f in c.files.iter().take(MAX_FILES) {
        s += &format!("- {f}\n");
    }
    if c.files.len() > MAX_FILES {
        s += &format!("… and {} more\n", c.files.len() - MAX_FILES);
    }
    if let Some(why) = &c.committed_unknown {
        s += &format!("committed changes: unknown ({why})\n");
    }
    s
}

/// The last non-blank screen lines (at most `TAIL_LINES`) that fit in `room` characters.
fn tail_within(screen: &str, room: usize) -> String {
    let lines: Vec<&str> = screen.lines().map(str::trim_end).filter(|l| !l.is_empty()).collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut used = 0;
    for line in lines.iter().rev().take(TAIL_LINES) {
        let cost = line.chars().count() + 1;
        if used + cost > room {
            break;
        }
        used += cost;
        kept.push(line);
    }
    kept.reverse();
    kept.iter().map(|l| format!("{l}\n")).collect()
}

fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

/// Whether `pane` can be handed to `to`: it exists, runs an agent, and `to` is an agent
/// the app can start. Gives the old agent's name.
pub fn check(cfg: &Config, pane: TermId, term: Option<&TermInfo>, to: &str) -> Result<String, String> {
    let term = term.ok_or_else(|| format!("no pane {pane}"))?;
    let agent = term.agent.clone().ok_or_else(|| format!("pane {pane} has no agent to hand off"))?;
    cfg.agent_command(Some(to), None, "")?;
    Ok(agent)
}

/// The pane in `now` that wasn't in `had`.
pub fn new_pane(had: &[TermId], now: impl IntoIterator<Item = TermId>) -> Option<TermId> {
    now.into_iter().find(|id| !had.contains(id))
}

/// What the checkout changed: uncommitted paths, plus, off the base branch, the files
/// changed since it left it. Sorted, no repeats.
pub fn changed_files(top: &Path, root: Option<&Path>) -> Result<Changed, String> {
    let mut files = uncommitted(&crate::proc::git(top, &["status", "--porcelain", "-z", "--untracked-files=all"])?);
    let mut committed_unknown = None;
    if let Some(base) = base_branch(top, root) {
        // -z as for status, so a non-ASCII path is spelled the same way in both lists.
        match crate::proc::git(top, &["diff", "--name-only", "-z", &format!("{base}...HEAD")]) {
            Ok(committed) => files.extend(committed.split('\0').filter(|p| !p.is_empty()).map(String::from)),
            Err(why) => committed_unknown = Some(why),
        }
    }
    files.sort();
    files.dedup();
    Ok(Changed { files, committed_unknown })
}

/// The branch the checkout's work is measured against, never the one it is on: the branch
/// the main checkout is on, else the repo's default branch. A branch made in the main
/// checkout leaves the main checkout on that branch, hence the fallback.
fn base_branch(top: &Path, root: Option<&Path>) -> Option<String> {
    let here = crate::gitfs::head(top).map(|h| h.branch);
    let elsewhere = |b: &String| here.as_deref() != Some(b.as_str());
    crate::gitfs::main_branch(root.unwrap_or(top)).filter(elsewhere).or_else(|| default_branch(top).filter(elsewhere))
}

/// The remote's default branch when it exists locally, else a local `main` or `master`.
fn default_branch(top: &Path) -> Option<String> {
    // No origin, or no origin/HEAD, is normal: the local names are the fallback.
    let remote = crate::proc::git(top, &["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"])
        .ok()
        .and_then(|r| r.trim().strip_prefix("origin/").map(String::from));
    let local = |b: &str| crate::proc::git(top, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{b}")]).is_ok();
    remote.filter(|b| local(b)).or_else(|| ["main", "master"].into_iter().find(|b| local(b)).map(String::from))
}

/// The paths in `git status --porcelain -z`. A rename or copy is followed by its old path,
/// which is skipped.
fn uncommitted(porcelain: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut entries = porcelain.split('\0');
    while let Some(e) = entries.next() {
        let Some((xy, path)) = e.split_at_checked(3).filter(|(_, p)| !p.is_empty()) else { continue };
        out.push(path.to_string());
        if xy.contains('R') || xy.contains('C') {
            entries.next();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changed(files: &[&str]) -> Changed {
        Changed { files: files.iter().map(|f| f.to_string()).collect(), committed_unknown: None }
    }

    fn facts() -> Facts {
        Facts {
            pane: 3,
            agent: "claude".into(),
            task: "fix the flaky checkout test".into(),
            last_prompt: "now run it again".into(),
            branch: Some("fix-checkout".into()),
            checkout: PathBuf::from("/work/app"),
            changed: Ok(changed(&["src/a.rs", "src/b.rs"])),
            screen: "one\n\n  \ntwo   \nthree\n".into(),
        }
    }

    #[test]
    fn the_brief_opens_by_saying_it_is_text_only() {
        let b = brief(&facts());
        let first = b.lines().next().unwrap();
        assert!(first.starts_with(concat!("Handoff from claude in ", env!("APP_COMMAND"), " pane 3.")), "{first}");
        for words in ["text summary only", "conversation, memory or tool results", "read the files yourself"] {
            assert!(first.contains(words), "{words}: {first}");
        }
    }

    #[test]
    fn the_sections_come_in_order() {
        let b = brief(&facts());
        let at = |s: &str| b.find(s).unwrap_or_else(|| panic!("{s} missing:\n{b}"));
        let order = [at("task: fix the flaky checkout test\n"), at("last prompt: now run it again\n"), at("branch: fix-checkout\ncheckout: /work/app\n"), at("changed files:\n- src/a.rs\n- src/b.rs\n"), at("recent output:\none\ntwo\nthree\n")];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{b}");
    }

    #[test]
    fn a_last_prompt_that_says_nothing_new_is_left_out() {
        for last in ["", "  ", "fix the flaky checkout test"] {
            let b = brief(&Facts { last_prompt: last.into(), ..facts() });
            assert!(!b.contains("last prompt:"), "{last:?}: {b}");
        }
    }

    #[test]
    fn changed_files_that_could_not_be_read_or_are_none_say_so() {
        let b = brief(&Facts { changed: Err("over ssh".into()), ..facts() });
        assert!(b.contains("changed files: unknown (over ssh)\n"), "{b}");
        let b = brief(&Facts { changed: Ok(changed(&[])), ..facts() });
        assert!(b.contains("changed files: none\n"), "{b}");
    }

    #[test]
    fn committed_changes_that_could_not_be_read_keep_the_uncommitted_list() {
        let c = Changed { committed_unknown: Some("bad revision".into()), ..changed(&["src/a.rs"]) };
        let b = brief(&Facts { changed: Ok(c), ..facts() });
        assert!(b.contains("changed files:\n- src/a.rs\ncommitted changes: unknown (bad revision)\n"), "{b}");
    }

    #[test]
    fn files_past_the_limit_are_counted() {
        let files = |n: usize| Ok(Changed { files: (0..n).map(|i| format!("f{i:02}")).collect(), committed_unknown: None });
        let b = brief(&Facts { changed: files(MAX_FILES), ..facts() });
        assert!(b.contains(&format!("- f{:02}\n", MAX_FILES - 1)) && !b.contains("more"), "{b}");
        let b = brief(&Facts { changed: files(MAX_FILES + 4), ..facts() });
        assert!(!b.contains(&format!("- f{MAX_FILES:02}\n")) && b.contains("… and 4 more\n"), "{b}");
    }

    #[test]
    fn only_the_last_screen_lines_are_kept() {
        let screen = |n: usize| (1..=n).map(|i| format!("line {i}  \n\n")).collect::<String>();
        let b = brief(&Facts { screen: screen(TAIL_LINES), ..facts() });
        assert!(b.ends_with(&format!("recent output:\n{}", (1..=TAIL_LINES).map(|i| format!("line {i}\n")).collect::<String>())), "{b}");
        let b = brief(&Facts { screen: screen(TAIL_LINES + 5), ..facts() });
        assert!(!b.contains("line 5\n") && b.contains("line 6\n") && b.ends_with(&format!("line {}\n", TAIL_LINES + 5)), "{b}");
    }

    #[test]
    fn a_long_brief_loses_old_output_first_and_never_its_header() {
        let wide = |n: usize| (1..=TAIL_LINES).map(|i| format!("{i:02}{}\n", "x".repeat(n))).collect::<String>();
        let fits = brief(&Facts { screen: wide(100), ..facts() });
        assert!(fits.chars().count() <= MAX_BRIEF_CHARS && fits.contains("\n01x"), "all output fits");
        let b = brief(&Facts { screen: wide(400), ..facts() });
        assert!(b.chars().count() <= MAX_BRIEF_CHARS, "{}", b.chars().count());
        assert!(!b.contains("\n01x") && b.contains(&format!("\n{TAIL_LINES:02}x")), "the oldest lines go, the newest stay");
        assert!(b.starts_with("Handoff from claude") && b.contains("changed files:\n- src/a.rs\n"));
        let b = brief(&Facts { task: "t".repeat(MAX_BRIEF_CHARS * 2), ..facts() });
        assert_eq!(b.chars().count(), MAX_BRIEF_CHARS);
        assert!(b.starts_with(concat!("Handoff from claude in ", env!("APP_COMMAND"), " pane 3.")) && b.ends_with('…'));
    }

    #[test]
    fn a_curly_quote_on_screen_stays_inside_the_powershell_prompt() {
        let cfg = Config { shell: Some("pwsh".into()), ..Config::default() };
        let b = brief(&Facts { screen: "don\u{2019}t; Write-Output INJECTED; #\n".into(), ..facts() });
        let cmd = cfg.agent_command(Some("claude"), None, &b).unwrap();
        assert!(cmd.ends_with(&format!("'{}'", b.replace('\'', "''").replace('\u{2019}', "\u{2019}\u{2019}"))), "{cmd}");
        assert!(cmd.contains("don\u{2019}\u{2019}t; Write-Output INJECTED"), "{cmd}");
    }

    #[test]
    fn porcelain_keeps_new_paths_and_skips_old_ones() {
        let z = " M src/a.rs\0R  new name.rs\0old name.rs\0?? docs/x.md\0A  b.rs\0";
        assert_eq!(uncommitted(z), ["src/a.rs", "new name.rs", "docs/x.md", "b.rs"]);
        assert_eq!(uncommitted("R  new.c\0a.c\0 M x.rs\0"), ["new.c", "x.rs"], "a short old path is still skipped alone");
        assert!(uncommitted("").is_empty());
    }

    fn git(dir: &Path, args: &[&str]) {
        crate::proc::git(dir, args).unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    }

    /// A repo in `<root>/repo` on `main` with one commit. Signing and hooks are switched off
    /// so the user's global git config can't fail or stall the test.
    fn repo(root: &Path) -> PathBuf {
        let _ = std::fs::remove_dir_all(root);
        let repo = root.join("repo");
        let hooks = root.join("no-hooks");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(&hooks).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        for (k, v) in [("user.email", "t@t"), ("user.name", "t"), ("commit.gpgsign", "false"), ("core.hooksPath", hooks.to_str().unwrap())] {
            git(&repo, &["config", k, v]);
        }
        for f in ["kept.rs", "edited.rs"] {
            std::fs::write(repo.join(f), "a").unwrap();
        }
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-qm", "one"]);
        repo
    }

    /// Commits a new `committed.rs` in `dir`, then edits `edited.rs` without committing it.
    fn commit_one_edit_another(dir: &Path) {
        std::fs::write(dir.join("committed.rs"), "b").unwrap();
        git(dir, &["add", "."]);
        git(dir, &["commit", "-qm", "two"]);
        std::fs::write(dir.join("edited.rs"), "c").unwrap();
    }

    #[test]
    fn a_worktree_lists_what_it_committed_and_what_it_has_not() {
        let root = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-handoff-{}"), std::process::id()));
        let repo = repo(&root);
        let wt = root.join("orders");
        git(&repo, &["worktree", "add", "-q", "-b", "orders", wt.to_str().unwrap()]);
        commit_one_edit_another(&wt);
        assert_eq!(changed_files(&wt, Some(&repo)).unwrap(), changed(&["committed.rs", "edited.rs"]));
        assert_eq!(changed_files(&repo, Some(&repo)).unwrap(), changed(&[]), "the main checkout: only what isn't committed");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_branch_in_the_main_checkout_lists_what_it_committed_and_what_it_has_not() {
        let root = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-handoff-branch-{}"), std::process::id()));
        let repo = repo(&root);
        git(&repo, &["checkout", "-q", "-b", "feat"]);
        commit_one_edit_another(&repo);
        assert_eq!(changed_files(&repo, Some(&repo)).unwrap(), changed(&["committed.rs", "edited.rs"]));
        git(&repo, &["checkout", "-q", "-f", "main"]);
        std::fs::write(repo.join("edited.rs"), "d").unwrap();
        assert_eq!(changed_files(&repo, Some(&repo)).unwrap(), changed(&["edited.rs"]), "on main: only what isn't committed");
        let _ = std::fs::remove_dir_all(&root);
    }

    fn pane(agent: Option<&str>) -> TermInfo {
        let mut t: TermInfo = serde_json::from_value(serde_json::json!({
            "id": 3, "cols": 80, "rows": 24, "title": "", "process": "node", "agent": null,
            "status": "Done", "cwd": "/work/app", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 0, "asleep": false, "win32_input": false,
        }))
        .unwrap();
        t.agent = agent.map(String::from);
        t
    }

    #[test]
    fn a_handoff_needs_a_pane_with_an_agent_and_a_known_agent_to_go_to() {
        let cfg = Config::default();
        assert_eq!(check(&cfg, 3, Some(&pane(Some("claude"))), "codex"), Ok("claude".into()));
        assert_eq!(check(&cfg, 9, None, "codex"), Err("no pane 9".into()));
        assert_eq!(check(&cfg, 3, Some(&pane(None)), "codex"), Err("pane 3 has no agent to hand off".into()));
        let err = check(&cfg, 3, Some(&pane(Some("claude"))), "rm -rf;x").unwrap_err();
        assert!(err.starts_with("unknown agent \"rm -rf;x\"; use one of:"), "{err}");
    }

    #[test]
    fn the_new_pane_is_the_one_that_was_not_there() {
        assert_eq!(new_pane(&[1, 3], [1, 3, 7]), Some(7));
        assert_eq!(new_pane(&[1, 3], [1, 3]), None);
        assert_eq!(new_pane(&[1, 3], [3]), None, "a pane closing is not a new one");
    }
}
