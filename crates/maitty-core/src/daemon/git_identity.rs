//! Which commit name, email and GitHub account git uses on this machine, and for which folders a
//! different one applies (`Query::GitIdentities`). It is read from the global git config with git
//! itself, never by reading the file as text, and nothing is written. A folder's rule is an
//! `includeIf "gitdir:<folder>/"` entry; one that Maitty wrote points at a file in Maitty's own
//! folder that names the folder it is for, and only those are read further. Any other is reported
//! by its folder alone.

use crate::proc::{RunError, run_within};
use crate::protocol::{
    GIT_IDENTITY_RULES_MAX, GIT_IDENTITY_TEXT_MAX_CHARS, GitIdentity, GitIdentityReport, GitIdentityRule,
};
use maitty_shared::github::is_login;
use maitty_shared::hidden::sanitize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// How long git has to answer one question about its config.
const ASK_LIMIT: Duration = Duration::from_secs(5);
/// The folder, inside Maitty's data folder, that holds the files its rules point at.
pub(super) const RULES_FOLDER: &str = "git-identities";
/// The key, in a file Maitty wrote, that names the folder the file is for.
pub(super) const FOLDER_KEY: &str = "maitty.folder";
/// The key that names the GitHub login git signs in as, in a file Maitty wrote.
pub(super) const LOGIN_KEY: &str = "credential.https://github.com.username";
/// The two conditions of an `includeIf` that are about a folder.
const FOLDER_CONDITIONS: [&str; 2] = ["gitdir:", "gitdir/i:"];

/// Where git's global config is and where Maitty keeps the files its rules point at.
pub(super) struct Place {
    /// The file to read in place of the person's global config, and the home folder to go with
    /// it. Tests name both, so a developer's own config is never read.
    stand_in: Option<(PathBuf, PathBuf)>,
    rules: PathBuf,
}

impl Place {
    /// This machine's own: the person's global config, and Maitty's data folder.
    pub(super) fn real() -> Place {
        Place { stand_in: None, rules: crate::config::data_dir().join(RULES_FOLDER) }
    }

    #[cfg(test)]
    pub(super) fn under(home: &Path) -> Place {
        Place { stand_in: Some((home.join("gitconfig"), home.to_path_buf())), rules: home.join(RULES_FOLDER) }
    }

    /// git, asked about its config, unable to ask anything back.
    fn git(&self) -> Command {
        let mut command = Command::new("git");
        command.env("LC_ALL", "C").env("GIT_TERMINAL_PROMPT", "0");
        if let Some((config, home)) = &self.stand_in {
            command
                .env("GIT_CONFIG_GLOBAL", config)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("HOME", home)
                .env("USERPROFILE", home)
                .current_dir(home);
        }
        command
    }

    /// What `git config <args>` printed. `None` when the key is not set or nothing matched, which
    /// git says with its exit status and no words.
    pub(super) fn config(&self, args: &[&str]) -> Result<Option<String>, Problem> {
        let mut command = self.git();
        command.arg("config").args(args);
        let ran = run_within(&mut command, ASK_LIMIT).map_err(|error| match error {
            RunError::Missing => Problem::NoGit,
            RunError::Start(why) => Problem::Failed(format!("couldn't run git: {why}")),
            RunError::TimedOut => Problem::Failed("git did not answer in time".into()),
        })?;
        if ran.success {
            return Ok(Some(ran.stdout));
        }
        match ran.stderr.lines().map(str::trim).find(|line| !line.is_empty()) {
            None => Ok(None),
            Some(said) => Err(Problem::Failed(said.to_owned())),
        }
    }

    /// One value of the global config, trimmed.
    fn global(&self, key: &str) -> Result<Option<String>, Problem> {
        Ok(self.config(&["--global", "--get", key])?.map(|value| value.trim().to_owned()))
    }

    /// One value of a file Maitty may have written, trimmed. A file that cannot be read has none.
    fn in_file(&self, file: &Path, key: &str) -> Option<String> {
        let file = file.to_str()?;
        self.config(&["--file", file, "--get", key]).ok().flatten().map(|value| value.trim().to_owned())
    }

    /// The folder Maitty keeps its rules' files in.
    pub(super) fn rules_folder(&self) -> &Path {
        &self.rules
    }

    /// The global config file, as a person would look for it.
    pub(super) fn global_file(&self) -> String {
        let file = match &self.stand_in {
            Some((config, _)) => config.clone(),
            None => match directories::BaseDirs::new() {
                Some(dirs) => dirs.home_dir().join(".gitconfig"),
                None => return "~/.gitconfig".into(),
            },
        };
        file.to_string_lossy().replace('\\', "/")
    }

    /// Every `key=value` line of a file, in order; none when it cannot be read.
    pub(super) fn listed(&self, file: &Path) -> Vec<String> {
        let Some(file) = file.to_str() else {
            return Vec::new();
        };
        let listed = self.config(&["--file", file, "--list"]).ok().flatten().unwrap_or_default();
        listed.lines().map(|entry| line(&entry.replacen('=', " = ", 1))).collect()
    }

    /// Whether `file` is directly inside the folder Maitty keeps its rules' files in.
    fn holds(&self, file: &Path) -> bool {
        let (Some(parent), Ok(ours)) = (file.parent(), self.rules.canonicalize()) else {
            return false;
        };
        parent.canonicalize().is_ok_and(|parent| parent == ours)
    }
}

pub(super) enum Problem {
    NoGit,
    Failed(String),
}

/// What git's global config says about who commits, and where. Blocks while git runs.
pub(super) fn report(place: &Place) -> GitIdentityReport {
    match read(place) {
        Ok(report) => report,
        Err(Problem::NoGit) => GitIdentityReport::default(),
        Err(Problem::Failed(why)) => GitIdentityReport {
            installed: true,
            problem: Some(line(&why)),
            ..GitIdentityReport::default()
        },
    }
}

fn read(place: &Place) -> Result<GitIdentityReport, Problem> {
    let name = place.global("user.name")?.map(|name| line(&name));
    let email = place.global("user.email")?.map(|email| line(&email));
    // `-z` ends each entry with a NUL and puts a line break between its key and its value, so a
    // folder with a space or a dot in its name is read whole.
    let listed = place.config(&["--global", "-z", "--get-regexp", r"^includeif\..*\.path$"])?.unwrap_or_default();
    let mut rules: Vec<GitIdentityRule> = Vec::new();
    for entry in listed.split('\0') {
        let Some((key, file)) = entry.split_once('\n') else {
            continue;
        };
        let Some(folder) = folder_of(key) else {
            continue;
        };
        let identity = written_by_us(place, &folder, Path::new(file));
        // git applies every entry for a folder in order, so a later one is what holds.
        rules.retain(|rule| rule.folder != folder);
        rules.push(GitIdentityRule { folder, identity });
    }
    rules.truncate(GIT_IDENTITY_RULES_MAX);
    Ok(GitIdentityReport { installed: true, name, email, rules, problem: None })
}

/// The folder an `includeif.<condition>.path` key is about, as it is written there without the
/// closing slash. `None` for a condition that is not about a folder (a branch, a remote).
fn folder_of(key: &str) -> Option<String> {
    // git lower-cases the section's name and keeps the condition as it was written.
    let condition = key.get("includeif.".len()..)?.strip_suffix(".path")?;
    let pattern = FOLDER_CONDITIONS.iter().find_map(|kind| condition.strip_prefix(kind))?;
    let folder = line(pattern.trim_end_matches('/'));
    (!folder.is_empty()).then_some(folder)
}

/// The identity `file` gives `folder`, when Maitty wrote the file for that folder.
fn written_by_us(place: &Place, folder: &str, file: &Path) -> Option<GitIdentity> {
    if !place.holds(file) || place.in_file(file, FOLDER_KEY).as_deref() != Some(folder) {
        return None;
    }
    Some(GitIdentity {
        name: line(&place.in_file(file, "user.name")?),
        email: line(&place.in_file(file, "user.email")?),
        login: place.in_file(file, LOGIN_KEY).filter(|login| is_login(login))?,
    })
}

/// `text` as one clean line of a length a report carries.
fn line(text: &str) -> String {
    sanitize(text.trim(), false).chars().take(GIT_IDENTITY_TEXT_MAX_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A home folder with a global git config, and the place that reads it.
    struct Home {
        dir: crate::daemon::persist::ScratchDir,
        place: Place,
    }

    impl Home {
        fn new() -> Home {
            let dir = crate::daemon::persist::scratch_dir();
            std::fs::create_dir_all(dir.path().join(RULES_FOLDER)).unwrap();
            let place = Place::under(dir.path());
            Home { dir, place }
        }

        fn set(&self, args: &[&str]) {
            assert!(self.place.config(args).is_ok_and(|said| said.is_some()), "git config {args:?}");
        }

        /// A rule for `folder` as Maitty writes one: a file of its own, and the entry that points at it.
        fn rule(&self, folder: &str, name: &str, email: &str, login: &str) -> PathBuf {
            let file = self.dir.path().join(RULES_FOLDER).join(format!("{login}.gitconfig"));
            let path = file.to_str().unwrap();
            for (key, value) in [("user.name", name), ("user.email", email), (LOGIN_KEY, login), (FOLDER_KEY, folder)] {
                self.set(&["--file", path, key, value]);
            }
            self.set(&["--global", &format!("includeIf.gitdir:{folder}/.path"), path]);
            file
        }

        fn global_text(&self) -> String {
            std::fs::read_to_string(self.dir.path().join("gitconfig")).unwrap_or_default()
        }
    }

    fn ours(folder: &str, name: &str, email: &str, login: &str) -> GitIdentityRule {
        let identity = GitIdentity { name: name.into(), email: email.into(), login: login.into() };
        GitIdentityRule { folder: folder.into(), identity: Some(identity) }
    }

    fn outside(folder: &str) -> GitIdentityRule {
        GitIdentityRule { folder: folder.into(), identity: None }
    }

    #[test]
    fn the_default_identity_and_a_rule_maitty_wrote_are_reported() {
        let home = Home::new();
        home.set(&["--global", "user.name", "Casey Example"]);
        home.set(&["--global", "user.email", "casey@example.invalid"]);
        home.rule("/home/casey/work projects", "Casey At Work", "casey@work.invalid", "casey-work");

        let report = report(&home.place);
        assert_eq!(
            report,
            GitIdentityReport {
                installed: true,
                name: Some("Casey Example".into()),
                email: Some("casey@example.invalid".into()),
                rules: vec![ours("/home/casey/work projects", "Casey At Work", "casey@work.invalid", "casey-work")],
                problem: None,
            }
        );
    }

    #[test]
    fn a_rule_set_outside_maitty_is_reported_by_its_folder_alone() {
        let home = Home::new();
        let theirs = home.dir.path().join("their.gitconfig");
        home.set(&["--file", theirs.to_str().unwrap(), "user.email", "secret@elsewhere.invalid"]);
        home.set(&["--global", "includeIf.gitdir:~/oss/.path", theirs.to_str().unwrap()]);
        home.set(&["--global", "includeIf.gitdir/i:C:/Clients/.path", "~/never-written.gitconfig"]);
        home.set(&["--global", "includeIf.onbranch:main.path", theirs.to_str().unwrap()]);

        let report = report(&home.place);
        assert_eq!(report.rules, [outside("~/oss"), outside("C:/Clients")]);
        assert_eq!((report.name, report.email, report.problem), (None, None, None));
    }

    #[test]
    fn a_file_in_maittys_folder_counts_only_when_it_names_that_folder_and_a_real_login() {
        let home = Home::new();
        let file = home.rule("/code/a", "A", "a@example.invalid", "login-a");
        let path = file.to_str().unwrap();
        // Someone pointed a second folder at the same file: the file is not that folder's.
        home.set(&["--global", "includeIf.gitdir:/code/b/.path", path]);
        assert_eq!(report(&home.place).rules, [ours("/code/a", "A", "a@example.invalid", "login-a"), outside("/code/b")]);

        home.set(&["--file", path, LOGIN_KEY, "not a login"]);
        assert_eq!(report(&home.place).rules, [outside("/code/a"), outside("/code/b")]);
    }

    #[test]
    fn the_last_entry_for_a_folder_is_the_one_reported_and_text_is_made_safe() {
        let home = Home::new();
        home.rule("/code/a", "A\u{1b}[31m", "a@example.invalid", "login-a");
        home.set(&["--global", "user.name", "Casey\u{7} Example"]);
        let first = report(&home.place);
        assert_eq!(first.rules, [ours("/code/a", "A[31m", "a@example.invalid", "login-a")]);
        assert_eq!(first.name.as_deref(), Some("Casey Example"));

        let theirs = home.dir.path().join("their.gitconfig");
        home.set(&["--file", theirs.to_str().unwrap(), "user.name", "x"]);
        home.set(&["--global", "--add", "includeIf.gitdir:/code/a/.path", theirs.to_str().unwrap()]);
        assert_eq!(report(&home.place).rules, [outside("/code/a")]);
    }

    #[test]
    fn nothing_is_written_and_an_empty_config_is_an_empty_report() {
        let home = Home::new();
        let empty = report(&home.place);
        assert_eq!(empty, GitIdentityReport { installed: true, ..GitIdentityReport::default() });
        assert!(!home.dir.path().join("gitconfig").exists(), "reading made no config file");

        home.rule("/code/a", "A", "a@example.invalid", "login-a");
        let before = home.global_text();
        report(&home.place);
        assert_eq!(home.global_text(), before);
    }
}
