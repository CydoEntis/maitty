use anyhow::{Context, Result, bail};
use std::time::Duration;

const SERVER_GONE_WITHIN: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(100);

pub(crate) fn wait_until_server_gone() -> Result<()> {
    let start = std::time::Instant::now();
    while super::server_running() {
        if start.elapsed() > SERVER_GONE_WITHIN {
            bail!(concat!(
                "the old server is still running; try `",
                env!("APP_COMMAND"),
                " kill-server`"
            ));
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}

pub(crate) fn relaunch() -> Result<()> {
    let exe = crate::installed_executable::installed_executable()?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(cmd.exec()).with_context(|| format!("starting {}", exe.display()))
    }
    #[cfg(not(unix))]
    {
        let status = cmd
            .status()
            .with_context(|| format!("starting {}", exe.display()))?;
        std::process::exit(status.code().unwrap_or(0));
    }
}

static RELAUNCH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn request_relaunch() {
    RELAUNCH.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub(crate) fn relaunch_requested() -> bool {
    RELAUNCH.load(std::sync::atomic::Ordering::SeqCst)
}
