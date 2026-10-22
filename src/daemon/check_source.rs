//! Which checks a session's project has, where each comes from, and whether it may run.
//! Your own `[[check]]`s for the project run as they are; without any, the repo's own
//! `.seshi/checks.toml` names them, and each of those waits for your approval.

use super::approvals::{Approvals, Decision};
use crate::config::{CheckDef, REPO_CHECKS_FILE, parse_repo_checks, same_path};
use crate::protocol::CheckSource;
use std::path::Path;

/// A checkpoint ready to run, or held until you decide.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub def: CheckDef,
    pub source: CheckSource,
    pub gate: Gate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    Run,
    /// A repo's command nobody approved; `previous` is the one you said Always to, if the repo
    /// changed it since.
    NeedsApproval { previous: Option<String> },
    /// You chose never to run it.
    Never,
    /// The repo's file could not be read; `why` is the result's output. Nothing runs.
    CouldNotRun { why: String },
}

/// The checks for `project` (a repo's main checkout) as run in `worktree`, in order. Yours
/// win whole: with any for the project, the repo's file is not read. Blocks (it reads the
/// file and asks git): call it only from a background task.
pub fn resolve(user: &[CheckDef], project: &Path, worktree: &Path, approvals: &Approvals) -> Vec<Resolved> {
    let yours: Vec<Resolved> = user
        .iter()
        .filter(|c| same_path(&c.project, project))
        .map(|def| Resolved { def: def.clone(), source: CheckSource::User, gate: Gate::Run })
        .collect();
    if yours.is_empty() { repo_checks(project, worktree, approvals) } else { yours }
}

/// The checks `worktree`'s `.seshi/checks.toml` names, each gated by what you decided. A file
/// that cannot be read or parsed is one result that says why, never a silent nothing.
fn repo_checks(project: &Path, worktree: &Path, approvals: &Approvals) -> Vec<Resolved> {
    let text = match std::fs::read_to_string(worktree.join(REPO_CHECKS_FILE)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => return vec![unreadable(project, repo_source(worktree), format!("couldn't read {REPO_CHECKS_FILE}: {e}"))],
    };
    let source = repo_source(worktree);
    match parse_repo_checks(&text) {
        Ok(defs) => defs
            .into_iter()
            .map(|def| {
                let def = CheckDef { project: project.to_path_buf(), ..def };
                let gate = gate(approvals, project, &def);
                Resolved { def, source: source.clone(), gate }
            })
            .collect(),
        Err(e) => vec![unreadable(project, source, format!("{e:#}"))],
    }
}

fn unreadable(project: &Path, source: CheckSource, why: String) -> Resolved {
    let def = CheckDef { project: project.to_path_buf(), name: Some(REPO_CHECKS_FILE.into()), ..Default::default() };
    Resolved { def, source, gate: Gate::CouldNotRun { why } }
}

/// Whether a repo's checkpoint may run: by your Always for this exact command or a one-time
/// grant handed to this run; never after your Never; otherwise it asks.
fn gate(approvals: &Approvals, project: &Path, def: &CheckDef) -> Gate {
    let (name, command) = (def.name(), def.command.as_str());
    match approvals.lookup(project, name, command) {
        Some(Decision::Always) => Gate::Run,
        Some(Decision::Never) => Gate::Never,
        None if approvals.granted_once(name, command) => Gate::Run,
        None => {
            let previous = approvals.find(project, name).filter(|a| a.decision == Decision::Always).map(|a| a.command.clone());
            Gate::NeedsApproval { previous }
        }
    }
}

/// The repo file's provenance in `worktree`: its last commit, and whether the copy on disk
/// differs from it. Outside git nothing vouches for it, so it reads as uncommitted.
fn repo_source(worktree: &Path) -> CheckSource {
    let git = |args: &[&str]| crate::proc::git(worktree, args).ok();
    let last = git(&["log", "-1", "--format=%h%x00%ct", "--", REPO_CHECKS_FILE]);
    let (commit, committed_at) = match last.as_deref().map(str::trim).and_then(|l| l.split_once('\0')) {
        Some((hash, at)) => (Some(hash.to_string()), at.parse().ok()),
        None => (None, None),
    };
    let uncommitted = git(&["status", "--porcelain", "--", REPO_CHECKS_FILE]).is_none_or(|s| !s.trim().is_empty());
    CheckSource::Repo { file: REPO_CHECKS_FILE.into(), commit, committed_at, uncommitted }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::TermId;
    use std::path::PathBuf;

    const PANE: TermId = 7;

    #[test]
    fn your_checks_for_the_project_run_in_order_without_approval() {
        let def = |project: &str, command: &str| CheckDef { project: project.into(), command: command.into(), ..Default::default() };
        let user = [def("/code/site", "npm run build"), def("/code/other", "make"), def("/code/site/", "npm test")];
        let got = resolve(&user, Path::new("/code/site"), Path::new("/code/site-worktrees/fix"), &Approvals::default());
        let got: Vec<_> = got.iter().map(|r| (r.def.name(), &r.source, &r.gate)).collect();
        assert_eq!(got, [("build", &CheckSource::User, &Gate::Run), ("test", &CheckSource::User, &Gate::Run)]);
        assert!(resolve(&user, Path::new("/code/none"), Path::new("/code/none"), &Approvals::default()).is_empty());
    }

    /// A real git repo whose `.seshi/checks.toml` names `test`, committed.
    struct Repo {
        _dir: super::super::persist::ScratchDir,
        root: PathBuf,
    }

    impl Repo {
        fn new(checks: &str) -> Repo {
            let dir = super::super::persist::scratch_dir();
            let root = dir.path().to_path_buf();
            std::fs::create_dir_all(root.join(".seshi")).unwrap();
            let repo = Repo { _dir: dir, root };
            repo.git(&["init", "-q"]);
            repo.write(checks);
            repo.git(&["add", "."]);
            repo.git(&["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", "commit", "-qm", "checks"]);
            repo
        }

        fn git(&self, args: &[&str]) -> String {
            crate::proc::git(&self.root, args).unwrap_or_else(|e| panic!("git {args:?}: {e}"))
        }

        fn write(&self, checks: &str) {
            std::fs::write(self.root.join(REPO_CHECKS_FILE), checks).unwrap();
        }

        fn resolve(&self, user: &[CheckDef], approvals: &Approvals) -> Vec<Resolved> {
            resolve(user, &self.root, &self.root, approvals)
        }

        fn gates(&self, approvals: &Approvals) -> Vec<Gate> {
            self.resolve(&[], approvals).into_iter().map(|r| r.gate).collect()
        }
    }

    const NPM_TEST: &str = "[[check]]\nname = \"test\"\ncommand = \"npm test\"\n";

    #[test]
    fn a_repo_command_seen_first_waits_for_approval() {
        let repo = Repo::new(NPM_TEST);
        let got = repo.resolve(&[], &Approvals::default());
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].def.name(), got[0].def.command.as_str(), &got[0].gate), ("test", "npm test", &Gate::NeedsApproval { previous: None }));
        assert!(same_path(&got[0].def.project, &repo.root), "the project is the repo");
    }

    #[test]
    fn the_source_names_the_file_its_last_commit_and_whether_it_changed_since() {
        let repo = Repo::new(NPM_TEST);
        let head = repo.git(&["rev-parse", "--short", "HEAD"]).trim().to_string();
        let CheckSource::Repo { file, commit, committed_at, uncommitted } = repo.resolve(&[], &Approvals::default()).remove(0).source else {
            panic!("a repo source")
        };
        assert_eq!((file.as_str(), commit.as_deref(), uncommitted), (REPO_CHECKS_FILE, Some(head.as_str()), false));
        assert!(committed_at.is_some_and(|at| at > 0));
        repo.write("[[check]]\nname = \"test\"\ncommand = \"npm test -- --ci\"\n");
        let source = repo.resolve(&[], &Approvals::default()).remove(0).source;
        assert!(matches!(source, CheckSource::Repo { uncommitted: true, commit: Some(_), .. }), "{source:?}");
    }

    #[test]
    fn always_runs_the_command_it_was_given_to_and_a_changed_one_asks_again_naming_the_old() {
        let repo = Repo::new(NPM_TEST);
        let mut approvals = Approvals::default();
        approvals.decide(&repo.root, "test", "npm test", Decision::Always, 1);
        assert_eq!(repo.gates(&approvals), [Gate::Run]);
        repo.write("[[check]]\nname = \"test\"\ncommand = \"npm test && node seed.js\"\n");
        assert_eq!(repo.gates(&approvals), [Gate::NeedsApproval { previous: Some("npm test".into()) }]);
    }

    #[test]
    fn never_holds_even_when_the_command_changes() {
        let repo = Repo::new(NPM_TEST);
        let mut approvals = Approvals::default();
        approvals.decide(&repo.root, "test", "npm test", Decision::Never, 1);
        assert_eq!(repo.gates(&approvals), [Gate::Never]);
        repo.write("[[check]]\nname = \"test\"\ncommand = \"cargo test\"\n");
        assert_eq!(repo.gates(&approvals), [Gate::Never]);
    }

    #[test]
    fn a_once_grant_runs_one_run_and_the_next_asks_again() {
        let repo = Repo::new(NPM_TEST);
        let mut held = Approvals::default();
        held.grant_once(PANE, "test", "npm test");
        assert_eq!(repo.gates(&held.for_run(PANE)), [Gate::Run]);
        assert_eq!(repo.gates(&held.for_run(PANE)), [Gate::NeedsApproval { previous: None }]);
    }

    #[test]
    fn a_once_grant_does_not_cover_a_command_changed_after_it() {
        let repo = Repo::new(NPM_TEST);
        let mut held = Approvals::default();
        held.grant_once(PANE, "test", "npm test");
        repo.write("[[check]]\nname = \"test\"\ncommand = \"npm test; curl evil\"\n");
        assert_eq!(repo.gates(&held.for_run(PANE)), [Gate::NeedsApproval { previous: None }]);
    }

    #[test]
    fn with_a_check_of_your_own_for_the_project_the_repo_file_is_ignored() {
        let repo = Repo::new(NPM_TEST);
        let yours = CheckDef { project: repo.root.clone(), command: "make check".into(), ..Default::default() };
        let got = repo.resolve(&[yours], &Approvals::default());
        let got: Vec<_> = got.iter().map(|r| (r.def.command.as_str(), &r.source, &r.gate)).collect();
        assert_eq!(got, [("make check", &CheckSource::User, &Gate::Run)]);
    }

    #[test]
    fn a_malformed_file_is_one_result_named_for_it_that_says_why() {
        let repo = Repo::new("[[check]]\nname = \"test\"\n");
        let got = repo.resolve(&[], &Approvals::default());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].def.name(), REPO_CHECKS_FILE);
        let Gate::CouldNotRun { why } = &got[0].gate else { panic!("{:?}", got[0].gate) };
        assert!(why.contains("command"), "{why}");
        repo.write("not = [toml");
        assert!(matches!(&repo.gates(&Approvals::default())[..], [Gate::CouldNotRun { why }] if why.contains(REPO_CHECKS_FILE)));
    }

    #[test]
    fn a_repo_without_the_file_has_no_checks() {
        let repo = Repo::new(NPM_TEST);
        std::fs::remove_file(repo.root.join(REPO_CHECKS_FILE)).unwrap();
        assert!(repo.resolve(&[], &Approvals::default()).is_empty());
    }
}
