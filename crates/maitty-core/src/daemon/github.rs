//! What this machine's GitHub command line (`gh`) says about who is signed in, for a client that
//! wants to show it (`Query::GitHubAccounts`). Only logins are read. No token is asked for, kept
//! or sent: signing in and the credentials stay `gh`'s own.

use crate::proc::{RunError, run_within};
use crate::protocol::{GITHUB_ACCOUNTS_MAX, GitHubAccount, GitHubReport};
use serde_json::Value;
use std::ffi::OsStr;
use std::process::Command;
use std::time::Duration;

/// How long `gh` has to answer a question about its own sign-in.
const ASK_LIMIT: Duration = Duration::from_secs(15);
/// The only host asked about.
const HOST: &str = "github.com";
/// The longest login GitHub gives out.
const LOGIN_MAX_CHARS: usize = 39;

/// `gh` (or `program` in its place) set up so it can ask nothing and colours nothing.
fn gh(program: &OsStr) -> Command {
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

/// Whether `text` could be a GitHub login: letters, digits and dashes. Anything else is not shown.
fn is_login(text: &str) -> bool {
    !text.is_empty() && text.len() <= LOGIN_MAX_CHARS && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
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

    fn account(login: &str, active: bool) -> GitHubAccount {
        GitHubAccount { login: login.into(), active }
    }

    #[test]
    fn the_accounts_whose_sign_in_works_are_listed_with_the_active_one_first() {
        assert_eq!(parse_status(TWO_ACCOUNTS), Some(vec![account("me", true), account("work-me", false)]));
    }

    #[test]
    fn nobody_signed_in_is_an_empty_list_and_other_text_is_not_an_answer() {
        assert_eq!(parse_status(r#"{"hosts":{}}"#), Some(Vec::new()));
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

    /// A program called `gh` that prints `said` and ends with `code`, in a folder of its own.
    fn fake_gh(root: &Path, said: &str, code: i32) -> PathBuf {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("said.txt"), said).unwrap();
        if cfg!(windows) {
            let program = root.join("gh.cmd");
            std::fs::write(&program, format!("@echo off\r\ntype \"%~dp0said.txt\"\r\nexit /b {code}\r\n")).unwrap();
            program
        } else {
            let program = root.join("gh");
            std::fs::write(&program, format!("#!/bin/sh\ncat \"$(dirname \"$0\")/said.txt\"\nexit {code}\n")).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            program
        }
    }

    #[test]
    fn the_report_is_what_gh_says_and_holds_no_token() {
        let root = crate::daemon::persist::scratch_dir();
        let said = TWO_ACCOUNTS.replace("\"tokenSource\":\"keyring\"", "\"token\":\"gho_secretsecretsecret\"");
        let program = fake_gh(root.path(), &said, 0);

        let report = accounts_from(program.as_os_str(), ASK_LIMIT);

        assert_eq!(report, GitHubReport { installed: true, accounts: vec![account("me", true), account("work-me", false)], problem: None });
        assert!(!format!("{report:?}").contains("gho_"));
    }

    #[test]
    fn nobody_signed_in_is_not_a_problem_though_gh_ends_as_a_failure() {
        let root = crate::daemon::persist::scratch_dir();
        let program = fake_gh(root.path(), r#"{"hosts":{}}"#, 1);

        assert_eq!(accounts_from(program.as_os_str(), ASK_LIMIT), GitHubReport { installed: true, accounts: Vec::new(), problem: None });
    }

    #[test]
    fn a_machine_without_gh_says_it_is_not_installed() {
        let root = crate::daemon::persist::scratch_dir();
        let missing = root.path().join("no-such-program");

        assert_eq!(accounts_from(missing.as_os_str(), ASK_LIMIT), GitHubReport { installed: false, accounts: Vec::new(), problem: None });
    }
}
