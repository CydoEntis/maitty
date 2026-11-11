//! Opening a web address from a chat in the person's own browser. The window never goes to an
//! address itself (that would replace the app with the page); a link that is clicked is handed to
//! the system, which opens it in the default browser. Only `http` and `https` addresses are
//! opened, as one argument to a fixed program, never through a shell.

use std::process::Command;

/// The longest address opened: a sign-in link with its parameters is a few hundred characters.
const MAX_URL_CHARS: usize = 4096;

/// The address, when it is one a browser should be given: http or https, with a host, and nothing
/// a command line or a terminal could read as more than an address.
fn web_address(url: &str) -> Result<&str, String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or("only web addresses (http and https) are opened")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let plain = |c: char| !c.is_whitespace() && !c.is_control();
    if host.is_empty() || host.starts_with('-') || url.chars().count() > MAX_URL_CHARS || !url.chars().all(plain) {
        return Err("that is not an address a browser can open".into());
    }
    Ok(url)
}

/// What to run to open `url` in the default browser: the system's own opener, with the address as
/// its one argument.
fn open_command(url: &str) -> Command {
    if cfg!(windows) {
        // Not `cmd /c start`, which would read `&` in an address as a second command.
        let mut command = Command::new("rundll32");
        command.arg("url.dll,FileProtocolHandler").arg(url);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        command
    } else {
        let mut command = maitty_desktop_client::host_command("xdg-open");
        command.arg(url);
        command
    }
}

/// Opens a web address in the default browser, for a link the person clicked.
#[tauri::command]
pub fn open_link(url: String) -> Result<(), String> {
    let url = web_address(&url)?;
    open_command(url)
        .spawn()
        .map(drop)
        .map_err(|error| format!("Could not open the link: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{open_command, web_address};

    #[test]
    fn only_a_web_address_with_a_host_is_opened() {
        let sign_in = "https://mcp.linear.app/authorize?response_type=code&client_id=https%3A%2F%2Fclaude.ai&state=a-b_c";
        assert_eq!(web_address(sign_in), Ok(sign_in));
        assert!(web_address("http://127.0.0.1:4747/").is_ok());
        for refused in ["file:///etc/passwd", "javascript:alert(1)", "ssh://host", "/usr/bin/x", "https://", "https:///path", "https://-flag", "https://a b.com", "https://a.com/\n--help", ""] {
            assert!(web_address(refused).is_err(), "{refused:?}");
        }
        assert!(web_address(&format!("https://a.com/{}", "x".repeat(5000))).is_err());
    }

    #[test]
    fn the_address_is_one_argument_to_the_systems_opener() {
        let command = open_command("https://example.com/?a=1&b=2");
        let program = command.get_program().to_string_lossy().into_owned();
        assert!(["rundll32", "open", "xdg-open"].contains(&program.as_str()), "{program}");
        assert_eq!(command.get_args().last().map(|arg| arg.to_string_lossy().into_owned()).as_deref(), Some("https://example.com/?a=1&b=2"));
    }
}
