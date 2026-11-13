//! Giving a folder its own GitHub account and commit identity, when a person's client asks
//! (`Query::GitIdentityRule`): a preview of the exact config lines, writing exactly those lines,
//! or taking a rule away. A rule is one file in Maitty's own `git-identities` folder and one
//! `includeIf` entry in the global git config that points at it. Both are written with
//! `git config`, never by editing a config file as text, and a rule set outside Maitty is never
//! changed.
//!
//! git signs in to github.com inside the folder as the rule's login: the file names that login and
//! a credential helper that asks the machine's `gh` for that login's token. `gh auth git-credential`
//! cannot do this, as it only ever answers for the active account.

use super::git_identity::{FOLDER_KEY, LOGIN_KEY, Place, Problem};
use crate::protocol::{GIT_IDENTITY_TEXT_MAX_CHARS, GitConfigWrite, GitIdentity, GitRuleChange};
use maitty_shared::github::is_login;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The key that names a helper git asks for github.com's credentials.
const HELPER_KEY: &str = "credential.https://github.com.helper";
/// How a folder is matched: without regard to case where the file system does the same.
const CONDITION: &str = if cfg!(any(windows, target_os = "macos")) { "gitdir/i:" } else { "gitdir:" };
/// Characters git would read as a pattern in a folder, which a rule's folder may not have.
const PATTERN_CHARS: [char; 4] = ['*', '?', '[', '\\'];
/// How many hex digits of the folder's digest name its file.
const FILE_NAME_DIGITS: usize = 16;

/// Does `change`. The lines written (or, for a preview, that would be written, and for a removal
/// that were taken away), by file, or one line saying why nothing was done. `signed_in` says
/// whether a login is signed in to GitHub on this machine. Blocks while git runs.
pub(super) fn change(place: &Place, change: &GitRuleChange, signed_in: impl Fn(&str) -> bool) -> Result<Vec<GitConfigWrite>, String> {
    match change {
        GitRuleChange::Preview { folder, identity } => {
            let rule = Rule::new(place, folder, identity, &signed_in)?;
            rule.check_unclaimed(place)?;
            Ok(rule.lines(place))
        }
        GitRuleChange::Apply { folder, identity } => {
            let rule = Rule::new(place, folder, identity, &signed_in)?;
            rule.check_unclaimed(place)?;
            rule.write(place)?;
            Ok(rule.lines(place))
        }
        GitRuleChange::Remove { folder } => remove(place, folder),
    }
}

/// A rule ready to be written: everything in it checked.
struct Rule {
    /// The folder as git will match it: absolute, with `/` between its parts, no closing slash.
    folder: String,
    identity: GitIdentity,
    file: PathBuf,
}

impl Rule {
    fn new(place: &Place, folder: &str, identity: &GitIdentity, signed_in: &impl Fn(&str) -> bool) -> Result<Rule, String> {
        let folder = existing_folder(folder)?;
        let name = one_line(&identity.name, "name")?;
        let email = one_line(&identity.email, "email")?;
        if !is_login(&identity.login) {
            return Err("that is not a GitHub login".into());
        }
        if !signed_in(&identity.login) {
            return Err(format!("{} is not signed in to GitHub on this machine", identity.login));
        }
        let file = file_for(place, &folder);
        Ok(Rule { folder, identity: GitIdentity { name, email, login: identity.login.clone() }, file })
    }

    fn key(&self) -> String {
        include_key(CONDITION, &self.folder)
    }

    /// Refuses a folder that already has a rule Maitty did not write: it is changed only where it was set.
    fn check_unclaimed(&self, place: &Place) -> Result<(), String> {
        let ours = path_text(&self.file)?;
        let set = all_values(place, &self.key())?;
        if set.iter().any(|value| value != ours) {
            return Err(format!("{} has a rule set outside Maitty: change it in your git config", self.folder));
        }
        Ok(())
    }

    /// What the rule's own file holds, in order.
    fn entries(&self) -> Vec<(&'static str, String)> {
        let login = &self.identity.login;
        vec![
            ("user.name", self.identity.name.clone()),
            ("user.email", self.identity.email.clone()),
            (LOGIN_KEY, login.clone()),
            // An empty helper drops the ones set before it (a system manager, `gh auth setup-git`),
            // so only the next one answers for github.com in this folder.
            (HELPER_KEY, String::new()),
            (HELPER_KEY, format!("!f() {{ test \"$1\" = get && echo \"password=$(gh auth token --hostname github.com --user {login})\"; }}; f")),
            (FOLDER_KEY, self.folder.clone()),
        ]
    }

    fn lines(&self, place: &Place) -> Vec<GitConfigWrite> {
        let file = shown(&self.file);
        vec![
            GitConfigWrite { file: place.global_file(), lines: vec![format!("{} = {file}", self.key())] },
            GitConfigWrite { file, lines: self.entries().into_iter().map(|(key, value)| format!("{key} = {value}")).collect() },
        ]
    }

    fn write(&self, place: &Place) -> Result<(), String> {
        let file = path_text(&self.file)?;
        std::fs::create_dir_all(place.rules_folder()).map_err(|error| format!("couldn't make {}: {error}", shown(place.rules_folder())))?;
        // The file is Maitty's own and rewritten whole, so a changed rule keeps nothing of the old one.
        match std::fs::remove_file(&self.file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("couldn't replace {}: {error}", shown(&self.file))),
        }
        for (key, value) in self.entries() {
            run(place, &["--file", file, "--add", key, &value])?;
        }
        // `--replace-all` leaves one entry however many there were: they were all this file.
        run(place, &["--global", "--replace-all", &self.key(), file])
    }
}

/// Takes away the rule Maitty wrote for `folder`, as the report names it: its entry and its file,
/// nothing else. A folder whose only rules were set outside Maitty is refused.
fn remove(place: &Place, folder: &str) -> Result<Vec<GitConfigWrite>, String> {
    let folder = folder.trim_end_matches('/');
    if folder.is_empty() || folder.chars().any(char::is_control) {
        return Err("that is not a folder".into());
    }
    let file = file_for(place, folder);
    let ours = path_text(&file)?;
    let mut removed = Vec::new();
    let mut outside = false;
    for condition in ["gitdir:", "gitdir/i:"] {
        let key = include_key(condition, folder);
        let set = all_values(place, &key)?;
        outside |= set.iter().any(|value| value != ours);
        if set.iter().any(|value| value == ours) {
            run(place, &["--global", "--fixed-value", "--unset-all", &key, ours])?;
            removed.push(format!("{key} = {}", shown(&file)));
        }
    }
    if removed.is_empty() {
        return Err(if outside {
            format!("the rule for {folder} was set outside Maitty: change it in your git config")
        } else {
            format!("{folder} has no rule")
        });
    }
    let mut taken = vec![GitConfigWrite { file: place.global_file(), lines: removed }];
    let held = place.listed(&file);
    match std::fs::remove_file(&file) {
        Ok(()) => taken.push(GitConfigWrite { file: shown(&file), lines: held }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("the rule was taken out, but {} could not be removed: {error}", shown(&file))),
    }
    Ok(taken)
}

fn include_key(condition: &str, folder: &str) -> String {
    format!("includeIf.{condition}{folder}/.path")
}

/// The file a folder's rule lives in: named after a digest of the folder, so any folder has one
/// and no folder's name reaches the file system.
fn file_for(place: &Place, folder: &str) -> PathBuf {
    let digest = Sha256::digest(folder.as_bytes());
    let name: String = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>().chars().take(FILE_NAME_DIGITS).collect();
    place.rules_folder().join(format!("{name}.gitconfig"))
}

/// `folder` as git matches it, when it is a folder that is there.
fn existing_folder(folder: &str) -> Result<String, String> {
    let folder = folder.trim();
    if folder.is_empty() || folder.chars().any(char::is_control) {
        return Err("choose a folder".into());
    }
    let real = Path::new(folder).canonicalize().map_err(|_| format!("there is no folder {folder}"))?;
    if !real.is_dir() {
        return Err(format!("{folder} is not a folder"));
    }
    let text = real.to_str().ok_or("that folder's name cannot be written in git's config")?;
    // Windows writes the canonical form as `\\?\C:\…`; git wants `C:/…`.
    let text = text.strip_prefix(r"\\?\").unwrap_or(text).replace('\\', "/");
    let text = text.trim_end_matches('/');
    if text.starts_with("UNC/") || text.chars().any(|c| PATTERN_CHARS.contains(&c) || c == '"') {
        return Err(format!("{text} cannot have a rule: git would read its name as a pattern"));
    }
    if text.is_empty() {
        return Err("the root folder cannot have a rule".into());
    }
    Ok(text.to_owned())
}

fn one_line(text: &str, what: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(format!("write the {what} first"));
    }
    if text.chars().any(char::is_control) {
        return Err(format!("that {what} has a character that cannot be in one"));
    }
    if text.chars().count() > GIT_IDENTITY_TEXT_MAX_CHARS {
        return Err(format!("that {what} is too long"));
    }
    Ok(text.to_owned())
}

fn all_values(place: &Place, key: &str) -> Result<Vec<String>, String> {
    let listed = place.config(&["--global", "-z", "--get-all", key]).map_err(said)?.unwrap_or_default();
    Ok(listed.split('\0').filter(|value| !value.is_empty()).map(str::to_owned).collect())
}

fn run(place: &Place, args: &[&str]) -> Result<(), String> {
    place.config(args).map_err(said)?;
    Ok(())
}

fn said(problem: Problem) -> String {
    match problem {
        Problem::NoGit => "git is not installed on this machine".into(),
        Problem::Failed(why) => why,
    }
}

fn path_text(path: &Path) -> Result<&str, String> {
    path.to_str().ok_or_else(|| "Maitty's data folder cannot be written in git's config".to_owned())
}

fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::git_identity::{RULES_FOLDER, report};
    use crate::protocol::GitIdentityRule;
    use std::process::Command;

    struct Home {
        dir: crate::daemon::persist::ScratchDir,
        place: Place,
    }

    impl Home {
        fn new() -> Home {
            let dir = crate::daemon::persist::scratch_dir();
            std::fs::create_dir_all(dir.path()).unwrap();
            let place = Place::under(dir.path());
            Home { dir, place }
        }

        /// A folder under the home, made.
        fn folder(&self, name: &str) -> PathBuf {
            let folder = self.dir.path().join(name);
            std::fs::create_dir_all(&folder).unwrap();
            folder
        }

        fn global_text(&self) -> String {
            std::fs::read_to_string(self.dir.path().join("gitconfig")).unwrap_or_default()
        }

        /// git as the home's own, in `cwd`, with `bin` first on the PATH.
        fn git(&self, cwd: &Path, bin: Option<&Path>) -> Command {
            let mut command = Command::new("git");
            command
                .current_dir(cwd)
                .env("GIT_CONFIG_GLOBAL", self.dir.path().join("gitconfig"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("HOME", self.dir.path())
                .env("USERPROFILE", self.dir.path())
                .env("GIT_TERMINAL_PROMPT", "0")
                .env("LC_ALL", "C");
            if let Some(bin) = bin {
                let path = std::env::var_os("PATH").unwrap_or_default();
                let mut parts = vec![bin.to_path_buf()];
                parts.extend(std::env::split_paths(&path));
                command.env("PATH", std::env::join_paths(parts).unwrap());
            }
            command
        }

        fn email_in(&self, cwd: &Path) -> String {
            let out = self.git(cwd, None).args(["config", "user.email"]).output().unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        }
    }

    fn who(login: &str) -> GitIdentity {
        GitIdentity { name: format!("Casey as {login}"), email: format!("{login}@example.invalid"), login: login.into() }
    }

    fn apply(home: &Home, folder: &Path, login: &str) -> Result<Vec<GitConfigWrite>, String> {
        let change_it = GitRuleChange::Apply { folder: folder.to_str().unwrap().into(), identity: who(login) };
        change(&home.place, &change_it, |_| true)
    }

    fn folder_text(folder: &Path) -> String {
        existing_folder(folder.to_str().unwrap()).unwrap()
    }

    #[test]
    fn a_preview_names_the_files_and_lines_and_writes_nothing() {
        let home = Home::new();
        let work = home.folder("work");
        let preview = GitRuleChange::Preview { folder: work.to_str().unwrap().into(), identity: who("work-me") };
        let lines = change(&home.place, &preview, |_| true).expect("a preview");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].file, home.place.global_file());
        let file = shown(&file_for(&home.place, &folder_text(&work)));
        assert_eq!(lines[0].lines, [format!("includeIf.{CONDITION}{}/.path = {file}", folder_text(&work))]);
        assert_eq!(lines[1].file, file);
        assert!(lines[1].lines.contains(&"user.email = work-me@example.invalid".to_string()));
        assert!(lines[1].lines.iter().any(|line| line.contains("gh auth token --hostname github.com --user work-me")));
        assert_eq!(home.global_text(), "");
        assert!(!home.place.rules_folder().exists(), "nothing was made");
    }

    #[test]
    fn an_applied_rule_holds_inside_the_folder_only_and_applying_it_again_leaves_one_entry() {
        let home = Home::new();
        let work = home.folder("work");
        let inside = work.join("repo");
        std::fs::create_dir_all(&inside).unwrap();
        assert!(home.git(&inside, None).args(["init", "-q"]).status().unwrap().success());
        let outside = home.folder("elsewhere");
        assert!(home.git(&outside, None).args(["init", "-q"]).status().unwrap().success());
        assert!(home.git(home.dir.path(), None).args(["config", "--global", "user.email", "casey@example.invalid"]).status().unwrap().success());

        apply(&home, &work, "work-me").expect("applied");
        apply(&home, &work, "work-me").expect("applied again");
        assert_eq!(home.email_in(&inside), "work-me@example.invalid");
        assert_eq!(home.email_in(&outside), "casey@example.invalid");
        assert_eq!(home.global_text().matches("includeIf").count(), 1, "{}", home.global_text());

        let report = report(&home.place);
        assert_eq!(report.rules, [GitIdentityRule { folder: folder_text(&work), identity: Some(who("work-me")) }]);
    }

    /// A program called `gh` that, asked for a login's token, answers `token-for-<login>`.
    fn fake_gh(bin: &Path) {
        std::fs::create_dir_all(bin).unwrap();
        let program = bin.join("gh");
        let script = "#!/bin/sh\nuser=\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --user ]; then user=\"$2\"; fi\n  shift\ndone\necho \"token-for-$user\"\n";
        std::fs::write(&program, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn inside_the_folder_git_signs_in_to_github_as_the_rules_login() {
        let home = Home::new();
        let bin = home.dir.path().join("bin");
        fake_gh(&bin);
        let work = home.folder("work");
        let repo = work.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        assert!(home.git(&repo, None).args(["init", "-q"]).status().unwrap().success());
        apply(&home, &work, "work-me").expect("applied");

        let mut fill = home.git(&repo, Some(&bin));
        fill.args(["credential", "fill"]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
        let mut child = fill.spawn().unwrap();
        use std::io::Write;
        child.stdin.take().unwrap().write_all(b"protocol=https\nhost=github.com\n\n").unwrap();
        let out = child.wait_with_output().unwrap();
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(said.contains("username=work-me\n"), "{said}");
        assert!(said.contains("password=token-for-work-me\n"), "{said}");
    }

    #[test]
    fn removing_takes_out_that_entry_and_that_file_and_nothing_else() {
        let home = Home::new();
        let (work, oss) = (home.folder("work"), home.folder("oss"));
        apply(&home, &work, "work-me").expect("applied");
        apply(&home, &oss, "oss-me").expect("applied");
        assert!(home.git(home.dir.path(), None).args(["config", "--global", "user.name", "Casey"]).status().unwrap().success());
        let file = file_for(&home.place, &folder_text(&work));

        let taken = change(&home.place, &GitRuleChange::Remove { folder: folder_text(&work) }, |_| false).expect("removed");
        assert_eq!(taken[1].file, shown(&file));
        assert!(!file.exists());
        let report = report(&home.place);
        assert_eq!(report.rules, [GitIdentityRule { folder: folder_text(&oss), identity: Some(who("oss-me")) }]);
        assert_eq!(report.name.as_deref(), Some("Casey"));
        let again = change(&home.place, &GitRuleChange::Remove { folder: folder_text(&work) }, |_| false);
        assert_eq!(again, Err(format!("{} has no rule", folder_text(&work))));
    }

    #[test]
    fn a_rule_set_outside_maitty_is_neither_replaced_nor_removed() {
        let home = Home::new();
        let work = home.folder("work");
        let key = include_key(CONDITION, &folder_text(&work));
        assert!(home.git(home.dir.path(), None).args(["config", "--global", &key, "~/mine.gitconfig"]).status().unwrap().success());
        let before = home.global_text();

        let refused = apply(&home, &work, "work-me").expect_err("not ours");
        assert!(refused.contains("set outside Maitty"), "{refused}");
        let refused = change(&home.place, &GitRuleChange::Remove { folder: folder_text(&work) }, |_| true).expect_err("not ours");
        assert!(refused.contains("set outside Maitty"), "{refused}");
        assert_eq!(home.global_text(), before);
    }

    #[test]
    fn a_missing_folder_a_bad_name_or_email_and_a_login_not_signed_in_are_refused() {
        let home = Home::new();
        let work = home.folder("work");
        let gone = home.dir.path().join("gone");
        assert!(apply(&home, &gone, "work-me").is_err_and(|why| why.starts_with("there is no folder")));
        for (name, email) in [("Casey\nX", "a@example.invalid"), ("Casey", "a@example.invalid\u{1b}[31m"), ("", "a@example.invalid")] {
            let identity = GitIdentity { name: name.into(), email: email.into(), login: "work-me".into() };
            let asked = GitRuleChange::Apply { folder: work.to_str().unwrap().into(), identity };
            assert!(change(&home.place, &asked, |_| true).is_err(), "{name:?} {email:?}");
        }
        let asked = GitRuleChange::Apply { folder: work.to_str().unwrap().into(), identity: who("--upload-pack") };
        assert_eq!(change(&home.place, &asked, |_| true), Err("that is not a GitHub login".into()));
        let asked = GitRuleChange::Preview { folder: work.to_str().unwrap().into(), identity: who("stranger") };
        assert_eq!(change(&home.place, &asked, |_| false), Err("stranger is not signed in to GitHub on this machine".into()));
        let starred = home.folder("a[b]");
        assert!(apply(&home, &starred, "work-me").is_err_and(|why| why.contains("pattern")));
        assert_eq!(home.global_text(), "");
    }

    #[test]
    fn rules_live_in_maittys_own_folder() {
        let home = Home::new();
        assert!(file_for(&home.place, "/code/a").starts_with(home.dir.path().join(RULES_FOLDER)));
        assert_ne!(file_for(&home.place, "/code/a"), file_for(&home.place, "/code/b"));
    }
}
