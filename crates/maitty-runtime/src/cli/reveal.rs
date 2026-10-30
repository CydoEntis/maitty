use anyhow::{Context, Result, bail};

/// `<app> reveal <link or pane>`: focus the session and bring its window forward.
pub(crate) fn run(target: &str) -> Result<()> {
    let Some((term, socket)) = crate::reveal::parse(target) else {
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
