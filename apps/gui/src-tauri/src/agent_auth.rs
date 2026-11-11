//! Whether an agent's own account is signed in, and signing it out, for Settings › Agents. The
//! agent's own command line is asked (`claude auth status`, `codex login status`) and told
//! (`claude auth logout`, `codex logout`): the host keeps no credentials and reads none. Each
//! program is the one found on PATH by the host, its arguments are constants, and the webview
//! supplies only the agent's name. Signing in is the agent's own interactive command, run in a
//! terminal session like any other (`start_agent_session` with the words `auth login`).

use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

/// The agent answers at once; this covers a slow start (its start-up checks).
const ASK_TIMEOUT: Duration = Duration::from_secs(15);
/// The most of an account's name or a status line that is shown.
const MAX_WORDS_CHARS: usize = 120;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// What an agent says about its account. `signed_in` is `None` when it could not be asked.
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct AuthStatus {
    pub installed: bool,
    #[serde(rename = "signedIn")]
    pub signed_in: Option<bool>,
    /// Who is signed in, as the agent says it (an email, or how it signed in); empty when it does not say.
    pub account: String,
    /// The plan, when the agent names one; empty otherwise.
    pub plan: String,
}

/// One line of text for the window: no control characters, no line breaks, cut to a length.
fn one_line(raw: &str) -> String {
    let words: Vec<&str> = raw.split(|c: char| c.is_whitespace() || c.is_control()).filter(|word| !word.is_empty()).collect();
    words.join(" ").chars().take(MAX_WORDS_CHARS).collect()
}

/// Claude Code's `auth status --json`.
fn claude_status(output: &str) -> AuthStatus {
    let Ok(reply) = serde_json::from_str::<serde_json::Value>(output.trim()) else {
        return AuthStatus { installed: true, ..Default::default() };
    };
    let text = |key: &str| one_line(reply.get(key).and_then(serde_json::Value::as_str).unwrap_or_default());
    AuthStatus {
        installed: true,
        signed_in: reply.get("loggedIn").and_then(serde_json::Value::as_bool),
        account: text("email"),
        plan: text("subscriptionType"),
    }
}

/// Codex's `login status`: one line saying how it is logged in, and a failing exit when it is not.
fn codex_status(succeeded: bool, output: &str) -> AuthStatus {
    let line = one_line(output.lines().find(|line| !line.trim().is_empty()).unwrap_or_default());
    let signed_in = succeeded && line.to_lowercase().starts_with("logged in");
    AuthStatus { installed: true, signed_in: Some(signed_in), account: if signed_in { line } else { String::new() }, plan: String::new() }
}

/// An agent's program and the words that ask about, and end, its sign-in.
struct Account {
    executable: anyhow::Result<PathBuf>,
    ask: &'static [&'static str],
    end: &'static [&'static str],
}

fn program(agent: &str) -> Result<Account, String> {
    match agent {
        "claude" => Ok(Account { executable: maitty_desktop_client::claude_executable(), ask: &["auth", "status", "--json"], end: &["auth", "logout"] }),
        "codex" => Ok(Account { executable: maitty_desktop_client::codex_app_server_executable(), ask: &["login", "status"], end: &["logout"] }),
        _ => Err(format!("{agent} has no account to manage from here")),
    }
}

/// Runs the agent with fixed words and gives back whether it succeeded and what it printed.
async fn run(executable: PathBuf, words: &'static [&'static str]) -> Result<(bool, String), String> {
    let mut command = tokio::process::Command::new(executable);
    command
        .args(words)
        // Claude Code will not start inside another Claude Code session, and this app may have been started from one.
        .env_remove("CLAUDECODE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = tokio::time::timeout(ASK_TIMEOUT, command.output())
        .await
        .map_err(|_| "The agent did not answer in time".to_string())?
        .map_err(|error| format!("Could not run the agent: {error}"))?;
    let printed = if output.stdout.is_empty() { &output.stderr } else { &output.stdout };
    Ok((output.status.success(), String::from_utf8_lossy(printed).into_owned()))
}

/// Whether the agent is installed and signed in, as the agent itself says.
#[tauri::command]
pub async fn agent_auth_status(agent: String) -> Result<AuthStatus, String> {
    let account = program(&agent)?;
    let Ok(executable) = account.executable else {
        return Ok(AuthStatus::default());
    };
    let (succeeded, printed) = run(executable, account.ask).await?;
    Ok(if agent == "claude" { claude_status(&printed) } else { codex_status(succeeded, &printed) })
}

/// Signs the agent's account out, with the agent's own command. Sessions already running keep
/// what they loaded until they are restarted.
#[tauri::command]
pub async fn agent_sign_out(agent: String) -> Result<(), String> {
    let account = program(&agent)?;
    let executable = account.executable.map_err(|error| format!("{error:#}"))?;
    let (succeeded, printed) = run(executable, account.end).await?;
    if succeeded { Ok(()) } else { Err(format!("The agent did not sign out: {}", one_line(&printed))) }
}

#[cfg(test)]
mod tests {
    use super::{claude_status, codex_status, program, AuthStatus};

    #[test]
    fn claude_says_who_is_signed_in_and_on_which_plan() {
        let status = claude_status(r#"{"loggedIn":true,"email":"me@example.com","subscriptionType":"max","orgId":"x"}"#);
        assert_eq!(status, AuthStatus { installed: true, signed_in: Some(true), account: "me@example.com".into(), plan: "max".into() });
        assert_eq!(claude_status(r#"{"loggedIn":false}"#).signed_in, Some(false));
    }

    #[test]
    fn an_answer_that_cannot_be_read_says_nothing_about_the_account() {
        assert_eq!(claude_status("not json"), AuthStatus { installed: true, ..Default::default() });
    }

    #[test]
    fn codex_is_signed_in_when_it_says_so_and_exits_well() {
        let status = codex_status(true, "\nLogged in using ChatGPT\n");
        assert_eq!((status.signed_in, status.account.as_str()), (Some(true), "Logged in using ChatGPT"));
        assert_eq!(codex_status(false, "Not logged in").signed_in, Some(false));
        assert_eq!(codex_status(true, "something else").signed_in, Some(false));
    }

    #[test]
    fn only_an_agent_with_an_account_command_is_asked() {
        assert!(program("claude").is_ok() && program("codex").is_ok());
        assert!(program("gemini").is_err());
        assert!(program("claude; rm -rf /").is_err());
    }
}
