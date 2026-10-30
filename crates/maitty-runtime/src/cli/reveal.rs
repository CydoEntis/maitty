use crate::protocol::TermId;
use anyhow::{Context, Result, bail};

const SCHEME: &str = env!("APP_COMMAND");

fn parse(target: &str) -> Option<(TermId, Option<String>)> {
    let target = target.trim();
    if let Ok(term) = target.parse() {
        return Some((term, None));
    }
    let rest = target.strip_prefix(&format!("{SCHEME}://session/"))?;
    let (term, query) = rest.split_once('?').unwrap_or((rest, ""));
    let term = term.trim_end_matches('/').parse().ok()?;
    let socket = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("socket="))
        .map(decode)
        .filter(|socket| !socket.is_empty());
    Some((term, socket))
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = (bytes[index] == b'%' && index + 2 < bytes.len())
            .then(|| {
                std::str::from_utf8(&bytes[index + 1..index + 3])
                    .ok()
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            })
            .flatten();
        match hex {
            Some(byte) => {
                decoded.push(byte);
                index += 3;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// `<app> reveal <link or pane>`: focus the session and bring its window forward.
pub(crate) fn run(target: &str) -> Result<()> {
    let Some((term, socket)) = parse(target) else {
        bail!(
            "not a {} link or pane number: {target}",
            env!("APP_COMMAND")
        )
    };
    if let Some(socket) = socket {
        // SAFETY: set once at startup, before any thread is started.
        unsafe { std::env::set_var(concat!(env!("APP_ENV"), "_SOCKET"), socket) };
    }
    // Clicking the notification gave this process the right to bring a window forward;
    // pass it on to the app's window, which does it when the server asks.
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(
            windows_sys::Win32::UI::WindowsAndMessaging::ASFW_ANY,
        );
    }
    crate::cli::reveal(term).context("couldn't reach that session")
}

#[cfg(test)]
mod tests {
    use super::{SCHEME, decode, parse};

    #[test]
    fn reveal_links_accept_encoded_sockets_and_bare_pane_numbers() {
        let link = format!("{SCHEME}://session/12?socket=my%20demo");
        assert_eq!(parse(&link), Some((12, Some("my demo".into()))));
        assert_eq!(parse(&format!("{SCHEME}://session/7/")), Some((7, None)));
        assert_eq!(parse("7"), Some((7, None)));
        assert_eq!(parse(&format!("{SCHEME}://nope/7")), None);
        assert_eq!(parse("https://example.com/session/7"), None);
        assert_eq!(decode("100%"), "100%");
    }
}
