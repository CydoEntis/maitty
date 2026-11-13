//! What this machine's GitHub command line (`gh`) says, for a client that wants to show it: who is
//! signed in (`Query::GitHubAccounts`), the repositories that account can clone
//! (`Query::GitHubRepos`) and whose they can be (`Query::GitHubOwners`). Only names are read. No
//! token is asked for, kept or sent: signing in and the credentials stay `gh`'s own.

use crate::proc::{Ran, RunError, run_within};
use crate::protocol::{
    GITHUB_ACCOUNTS_MAX, GITHUB_DESCRIPTION_MAX_CHARS, GITHUB_LIST_LIMIT_SECS, GITHUB_OWNERS_MAX, GITHUB_REPOS_MAX, GitHubAccount,
    GitHubAnswer, GitHubRepo, GitHubRepoList, GitHubReport,
};
use maitty_shared::github::{clone_address, is_login, is_time};
use maitty_shared::hidden::sanitize;
use serde_json::Value;
use std::ffi::OsStr;
use std::process::Command;
use std::time::Duration;

/// How long `gh` has to answer a question about its own sign-in.
const ASK_LIMIT: Duration = Duration::from_secs(15);
/// How long `gh` has to list repositories or organisations, which it asks GitHub for.
const LIST_LIMIT: Duration = Duration::from_secs(GITHUB_LIST_LIMIT_SECS);
/// The only host asked about.
const HOST: &str = "github.com";
/// The longest search a client may ask for.
const SEARCH_MAX_CHARS: usize = 100;

/// `gh` (or `program` in its place) set up so it can ask nothing and colours nothing.
pub(super) fn gh(program: &OsStr) -> Command {
    let mut command = Command::new(program);
    // The daemon may have been started without the PATH a login shell has, where `gh` often is.
    if let Some(path) = crate::login_path::resolved() {
        command.env("PATH", path);
    }
    command.env("GH_PROMPT_DISABLED", "1").env("NO_COLOR", "1");
    command
}

/// Who is signed in to GitHub on this machine. Blocks while `gh` runs.
pub(super) fn accounts() -> GitHubReport {
    accounts_from(OsStr::new("gh"), ASK_LIMIT)
}

/// The repositories `owner` (the signed-in account when none is named) has that the account can
/// clone, narrowed to those matching `search` when there is one. Blocks while `gh` runs.
pub(super) fn repos(owner: Option<&str>, search: Option<&str>) -> GitHubAnswer<GitHubRepoList> {
    repos_from(OsStr::new("gh"), owner, search, LIST_LIMIT)
}

/// Whose repositories can be listed: the signed-in account, then its organisations. Blocks while
/// `gh` runs.
pub(super) fn owners() -> GitHubAnswer<Vec<String>> {
    owners_from(OsStr::new("gh"), LIST_LIMIT)
}

fn accounts_from(program: &OsStr, limit: Duration) -> GitHubReport {
    let mut command = gh(program);
    command.args(["auth", "status", "--json", "hosts"]);
    let ran = match run_within(&mut command, limit) {
        Ok(ran) => ran,
        Err(RunError::Missing) => return GitHubReport::default(),
        Err(RunError::Start(why)) => return problem(format!("couldn't run gh: {why}")),
        Err(RunError::TimedOut) => return problem("gh did not answer in time".into()),
    };
    // With nobody signed in gh still prints the (empty) list but exits as a failure, so what it
    // printed is read first and its exit status only when that is not a list.
    match parse_status(&ran.stdout) {
        Some(accounts) => GitHubReport { installed: true, accounts, problem: None },
        // A gh too old to answer in JSON says so on its error stream.
        None => problem(first_line(&ran.stderr).unwrap_or("gh did not say who is signed in").to_owned()),
    }
}

fn problem(why: String) -> GitHubReport {
    GitHubReport { installed: true, accounts: Vec::new(), problem: Some(why) }
}

fn first_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

/// The accounts in the answer to `gh auth status --json hosts` whose sign-in still works, the
/// active one first. `None` when the text is not that answer.
fn parse_status(json: &str) -> Option<Vec<GitHubAccount>> {
    let value: Value = serde_json::from_str(json).ok()?;
    let hosts = value.get("hosts")?.as_object()?;
    let mut accounts: Vec<GitHubAccount> = hosts
        .get(HOST)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entry| entry.get("state").and_then(Value::as_str) == Some("success"))
        .filter_map(|entry| {
            let login = entry.get("login").and_then(Value::as_str).filter(|login| is_login(login))?;
            Some(GitHubAccount {
                login: login.to_owned(),
                active: entry.get("active").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect();
    accounts.sort_by_key(|account| !account.active);
    accounts.truncate(GITHUB_ACCOUNTS_MAX);
    Some(accounts)
}

/// The login `gh` acts as, or the answer that says why there is none.
fn active_login<T>(program: &OsStr) -> Result<String, GitHubAnswer<T>> {
    let report = accounts_from(program, ASK_LIMIT);
    if !report.installed {
        return Err(GitHubAnswer::NotInstalled);
    }
    if let Some(why) = report.problem {
        return Err(GitHubAnswer::Failed(why));
    }
    report.accounts.into_iter().next().map(|account| account.login).ok_or(GitHubAnswer::NotSignedIn)
}

/// What `gh` printed when it ran to its end and said it worked, or the answer that says why not.
fn listed<T>(command: &mut Command, limit: Duration, what: &str) -> Result<Ran, GitHubAnswer<T>> {
    let ran = run_within(command, limit).map_err(|error| match error {
        RunError::Missing => GitHubAnswer::NotInstalled,
        RunError::Start(why) => GitHubAnswer::Failed(format!("couldn't run gh: {why}")),
        RunError::TimedOut => GitHubAnswer::Failed("gh did not answer in time".into()),
    })?;
    if ran.success {
        return Ok(ran);
    }
    let why = first_line(&ran.stderr).map_or_else(|| format!("gh could not list the {what}"), |line| sanitize(line, false));
    Err(GitHubAnswer::Failed(why))
}

/// Whether a client's search is words only: no `:` that GitHub would read as a qualifier, and
/// nothing a command line could read as anything but text.
fn is_search(text: &str) -> bool {
    text.chars().count() <= SEARCH_MAX_CHARS && text.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-'))
}

fn repos_from(program: &OsStr, owner: Option<&str>, search: Option<&str>, limit: Duration) -> GitHubAnswer<GitHubRepoList> {
    let named = |text: Option<&str>| text.map(str::trim).filter(|text| !text.is_empty()).map(str::to_owned);
    let (owner, search) = (named(owner), named(search));
    if owner.as_deref().is_some_and(|owner| !is_login(owner)) {
        return GitHubAnswer::Failed("That is not a GitHub account or organisation".into());
    }
    if search.as_deref().is_some_and(|search| !is_search(search)) {
        return GitHubAnswer::Failed("Search with letters, digits, spaces, dots and dashes".into());
    }
    let login = match active_login(program) {
        Ok(login) => login,
        Err(answer) => return answer,
    };
    let owner = owner.unwrap_or(login);
    // One more than is given, to know whether there are more.
    let asked = (GITHUB_REPOS_MAX + 1).to_string();
    let mut command = gh(program);
    let name_field = match &search {
        None => {
            command.args(["repo", "list"]).arg(&owner).args(["--json", "nameWithOwner,description,isPrivate,pushedAt", "--limit", &asked]);
            "nameWithOwner"
        }
        Some(search) => {
            // After `--` so that a search starting with a dash is still a search.
            command.args(["search", "repos", "--owner"]).arg(&owner).args(["--json", "fullName,description,isPrivate,pushedAt", "--limit", &asked, "--"]).arg(search);
            "fullName"
        }
    };
    let ran = match listed(&mut command, limit, "repositories") {
        Ok(ran) => ran,
        Err(answer) => return answer,
    };
    match parse_repos(&ran.stdout, name_field) {
        Some(list) => GitHubAnswer::Listed(list),
        None => GitHubAnswer::Failed("gh did not list the repositories".into()),
    }
}

/// The repositories in a `gh … --json` answer, each named by `name_field`. One whose name is not a
/// repository's is left out. `None` when the text is not a list.
fn parse_repos(json: &str, name_field: &str) -> Option<GitHubRepoList> {
    let value: Value = serde_json::from_str(json).ok()?;
    let mut repos: Vec<GitHubRepo> = value
        .as_array()?
        .iter()
        .filter_map(|entry| {
            let name = entry.get(name_field).and_then(Value::as_str)?;
            let clone_url = clone_address(name)?;
            let text = |field: &str| entry.get(field).and_then(Value::as_str).unwrap_or_default();
            Some(GitHubRepo {
                name: name.to_owned(),
                description: sanitize(text("description"), false).trim().chars().take(GITHUB_DESCRIPTION_MAX_CHARS).collect(),
                private: entry.get("isPrivate").and_then(Value::as_bool).unwrap_or(false),
                pushed_at: Some(text("pushedAt")).filter(|time| is_time(time)).unwrap_or_default().to_owned(),
                clone_url,
            })
        })
        .collect();
    let more = repos.len() > GITHUB_REPOS_MAX;
    repos.truncate(GITHUB_REPOS_MAX);
    Some(GitHubRepoList { repos, more })
}

fn owners_from(program: &OsStr, limit: Duration) -> GitHubAnswer<Vec<String>> {
    let login = match active_login(program) {
        Ok(login) => login,
        Err(answer) => return answer,
    };
    let mut command = gh(program);
    command.args(["org", "list", "--limit", &GITHUB_OWNERS_MAX.to_string()]);
    let ran = match listed(&mut command, limit, "organisations") {
        Ok(ran) => ran,
        Err(answer) => return answer,
    };
    // One organisation a line; a line that is not a login (a heading) is not one.
    let organisations = ran.stdout.lines().map(str::trim).filter(|line| is_login(line) && *line != login).map(str::to_owned);
    let mut owners: Vec<String> = std::iter::once(login.clone()).chain(organisations).collect();
    owners.truncate(GITHUB_OWNERS_MAX);
    GitHubAnswer::Listed(owners)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    const TWO_ACCOUNTS: &str = r#"{"hosts":{"github.com":[
        {"state":"success","active":false,"host":"github.com","login":"work-me","tokenSource":"keyring","scopes":"repo","gitProtocol":"https"},
        {"state":"success","active":true,"host":"github.com","login":"me","tokenSource":"keyring","scopes":"repo","gitProtocol":"https"},
        {"state":"error","active":false,"host":"github.com","login":"expired","error":"bad credentials"}
    ],"ghe.example.com":[{"state":"success","active":true,"host":"ghe.example.com","login":"elsewhere"}]}}"#;
    const NOBODY: &str = r#"{"hosts":{}}"#;

    fn account(login: &str, active: bool) -> GitHubAccount {
        GitHubAccount { login: login.into(), active }
    }

    #[test]
    fn the_accounts_whose_sign_in_works_are_listed_with_the_active_one_first() {
        assert_eq!(parse_status(TWO_ACCOUNTS), Some(vec![account("me", true), account("work-me", false)]));
    }

    #[test]
    fn nobody_signed_in_is_an_empty_list_and_other_text_is_not_an_answer() {
        assert_eq!(parse_status(NOBODY), Some(Vec::new()));
        assert_eq!(parse_status("You are not logged into any GitHub hosts."), None);
        assert_eq!(parse_status(r#"{"other":1}"#), None);
    }

    #[test]
    fn a_login_that_is_not_one_is_left_out() {
        let json = r#"{"hosts":{"github.com":[
            {"state":"success","active":true,"login":"ok-1"},
            {"state":"success","active":false,"login":"<b>x</b>"},
            {"state":"success","active":false,"login":""},
            {"state":"success","active":false,"login":"a b"}
        ]}}"#;
        assert_eq!(parse_status(json), Some(vec![account("ok-1", true)]));
    }

    /// What a pretend `gh` prints and the status it ends with.
    #[derive(Clone, Copy)]
    struct Says<'a>(&'a str, i32);

    /// A program called `gh`, in a folder of its own. Asked `auth …` it says `auth`; asked anything
    /// else it writes what it was asked to `args.txt` beside it and says `rest`.
    fn fake_gh(root: &Path, auth: Says, rest: Says) -> PathBuf {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("auth.txt"), auth.0).unwrap();
        std::fs::write(root.join("rest.txt"), rest.0).unwrap();
        if cfg!(windows) {
            let program = root.join("gh.cmd");
            let script = format!(
                "@echo off\r\nif \"%1\"==\"auth\" (\r\n  type \"%~dp0auth.txt\"\r\n  exit /b {}\r\n)\r\n>\"%~dp0args.txt\" echo %*\r\ntype \"%~dp0rest.txt\"\r\nexit /b {}\r\n",
                auth.1, rest.1
            );
            std::fs::write(&program, script).unwrap();
            program
        } else {
            let program = root.join("gh");
            let script = format!(
                "#!/bin/sh\nhere=\"$(dirname \"$0\")\"\nif [ \"$1\" = auth ]; then cat \"$here/auth.txt\"; exit {}; fi\necho \"$@\" > \"$here/args.txt\"\ncat \"$here/rest.txt\"\nexit {}\n",
                auth.1, rest.1
            );
            std::fs::write(&program, script).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            program
        }
    }

    /// What the pretend `gh` was last asked, apart from its sign-in.
    fn asked(root: &Path) -> String {
        std::fs::read_to_string(root.join("args.txt")).unwrap_or_default().replace('"', "").trim().to_owned()
    }

    #[test]
    fn the_report_is_what_gh_says_and_holds_no_token() {
        let root = crate::daemon::persist::scratch_dir();
        let said = TWO_ACCOUNTS.replace("\"tokenSource\":\"keyring\"", "\"token\":\"gho_secretsecretsecret\"");
        let program = fake_gh(root.path(), Says(&said, 0), Says("", 0));

        let report = accounts_from(program.as_os_str(), ASK_LIMIT);

        assert_eq!(report, GitHubReport { installed: true, accounts: vec![account("me", true), account("work-me", false)], problem: None });
        assert!(!format!("{report:?}").contains("gho_"));
    }

    #[test]
    fn nobody_signed_in_is_not_a_problem_though_gh_ends_as_a_failure() {
        let root = crate::daemon::persist::scratch_dir();
        let program = fake_gh(root.path(), Says(NOBODY, 1), Says("", 0));

        assert_eq!(accounts_from(program.as_os_str(), ASK_LIMIT), GitHubReport { installed: true, accounts: Vec::new(), problem: None });
    }

    #[test]
    fn a_machine_without_gh_says_it_is_not_installed() {
        let root = crate::daemon::persist::scratch_dir();
        let missing = root.path().join("no-such-program");

        assert_eq!(accounts_from(missing.as_os_str(), ASK_LIMIT), GitHubReport { installed: false, accounts: Vec::new(), problem: None });
        assert_eq!(repos_from(missing.as_os_str(), None, None, LIST_LIMIT), GitHubAnswer::NotInstalled);
        assert_eq!(owners_from(missing.as_os_str(), LIST_LIMIT), GitHubAnswer::NotInstalled);
    }

    #[test]
    fn nobody_signed_in_is_its_own_answer_and_gh_is_asked_for_no_list() {
        let root = crate::daemon::persist::scratch_dir();
        let program = fake_gh(root.path(), Says(NOBODY, 1), Says("[]", 0));

        assert_eq!(repos_from(program.as_os_str(), None, None, LIST_LIMIT), GitHubAnswer::NotSignedIn);
        assert_eq!(owners_from(program.as_os_str(), LIST_LIMIT), GitHubAnswer::NotSignedIn);
        assert_eq!(asked(root.path()), "");
    }

    const REPOS: &str = r#"[
        {"nameWithOwner":"me/widgets","description":"Small\u001b[31m parts\nfor things","isPrivate":true,"pushedAt":"2026-01-02T03:04:05Z"},
        {"nameWithOwner":"me/plain","description":null,"isPrivate":false,"pushedAt":"<script>"},
        {"nameWithOwner":"me/--upload-pack=x","description":"","isPrivate":false,"pushedAt":""},
        {"nameWithOwner":"not a repository","description":"","isPrivate":false,"pushedAt":""}
    ]"#;

    #[test]
    fn the_signed_in_accounts_repositories_are_listed_clean_with_their_clone_address() {
        let root = crate::daemon::persist::scratch_dir();
        let program = fake_gh(root.path(), Says(TWO_ACCOUNTS, 0), Says(REPOS, 0));

        let GitHubAnswer::Listed(list) = repos_from(program.as_os_str(), None, None, LIST_LIMIT) else {
            panic!("a signed-in machine lists its repositories");
        };

        assert!(!list.more);
        assert_eq!(
            list.repos,
            vec![
                GitHubRepo {
                    name: "me/widgets".into(),
                    description: "Small[31m parts for things".into(),
                    private: true,
                    pushed_at: "2026-01-02T03:04:05Z".into(),
                    clone_url: "https://github.com/me/widgets.git".into(),
                },
                GitHubRepo {
                    name: "me/plain".into(),
                    description: String::new(),
                    private: false,
                    pushed_at: String::new(),
                    clone_url: "https://github.com/me/plain.git".into(),
                },
            ]
        );
        assert_eq!(asked(root.path()), "repo list me --json nameWithOwner,description,isPrivate,pushedAt --limit 201");
    }

    #[test]
    fn an_owner_and_a_search_are_passed_on_as_plain_words() {
        let root = crate::daemon::persist::scratch_dir();
        let found = r#"[{"fullName":"my-org/widgets","description":"","isPrivate":false,"pushedAt":"2026-01-02T03:04:05Z"}]"#;
        let program = fake_gh(root.path(), Says(TWO_ACCOUNTS, 0), Says(found, 0));

        let GitHubAnswer::Listed(list) = repos_from(program.as_os_str(), Some(" my-org "), Some("-wid gets"), LIST_LIMIT) else {
            panic!("a search is answered");
        };

        assert_eq!(list.repos.iter().map(|repo| repo.name.as_str()).collect::<Vec<_>>(), ["my-org/widgets"]);
        assert_eq!(asked(root.path()), "search repos --owner my-org --json fullName,description,isPrivate,pushedAt --limit 201 -- -wid gets");
    }

    #[test]
    fn an_owner_or_a_search_that_is_more_than_words_is_refused_before_gh_runs() {
        let root = crate::daemon::persist::scratch_dir();
        let program = fake_gh(root.path(), Says(TWO_ACCOUNTS, 0), Says("[]", 0));

        for (owner, search) in [(Some("--web"), None), (Some("a/b"), None), (None, Some("user:someone")), (None, Some("a\"b")), (None, Some("x;y"))] {
            let answer = repos_from(program.as_os_str(), owner, search, LIST_LIMIT);
            assert!(matches!(answer, GitHubAnswer::Failed(_)), "{owner:?} {search:?} gave {answer:?}");
        }
        assert!(matches!(repos_from(program.as_os_str(), None, Some(&"a".repeat(SEARCH_MAX_CHARS + 1)), LIST_LIMIT), GitHubAnswer::Failed(_)));
        assert_eq!(asked(root.path()), "");
    }

    #[test]
    fn at_most_two_hundred_repositories_are_given_and_it_says_there_are_more() {
        let many: Vec<String> = (0..=GITHUB_REPOS_MAX).map(|n| format!(r#"{{"nameWithOwner":"me/r{n}","description":"{}"}}"#, "d".repeat(400))).collect();

        let list = parse_repos(&format!("[{}]", many.join(",")), "nameWithOwner").unwrap();

        assert_eq!(list.repos.len(), GITHUB_REPOS_MAX);
        assert!(list.more);
        assert_eq!(list.repos[0].description.chars().count(), GITHUB_DESCRIPTION_MAX_CHARS);
        assert_eq!(parse_repos("not a list", "nameWithOwner"), None);
    }

    #[test]
    fn what_gh_says_went_wrong_is_the_answer() {
        let root = crate::daemon::persist::scratch_dir();
        let program = fake_gh(root.path(), Says(TWO_ACCOUNTS, 0), Says("", 1));

        assert_eq!(
            repos_from(program.as_os_str(), None, None, LIST_LIMIT),
            GitHubAnswer::Failed("gh could not list the repositories".into())
        );
    }

    #[test]
    fn the_owners_are_the_signed_in_account_then_its_organisations() {
        let root = crate::daemon::persist::scratch_dir();
        let program = fake_gh(root.path(), Says(TWO_ACCOUNTS, 0), Says("Showing 2 of 2 organizations\n\nmy-org\nme\nother-org\n<b>\n", 0));

        assert_eq!(
            owners_from(program.as_os_str(), LIST_LIMIT),
            GitHubAnswer::Listed(vec!["me".into(), "my-org".into(), "other-org".into()])
        );
        assert_eq!(asked(root.path()), "org list --limit 100");
    }
}
