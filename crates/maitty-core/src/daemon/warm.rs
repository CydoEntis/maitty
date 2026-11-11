//! An agent started ahead of its first message. A window asks for one while a person is still
//! writing; it runs hidden, as a spare, and the session asked for next with the same command in
//! the same folder is that agent, already up.

use super::*;

/// How long a spare waits to be taken after it was last asked for. A window that still wants it
/// asks again; one that closed or crashed stops asking, and the agent is stopped.
pub(super) const WARM_KEEP: Duration = Duration::from_secs(10 * 60);

/// The agent kept warm: where, what was run, its pane, and when a window last asked for it.
pub(super) struct Warm {
    cwd: PathBuf,
    cmd: String,
    term: TermId,
    asked: Instant,
}

impl Daemon {
    /// Start `cmd` in `cwd` hidden, or keep the one already there for it. One is kept at a time:
    /// asking for another stops the first.
    pub(super) fn prewarm_agent(&mut self, cwd: PathBuf, cmd: String) -> Result<()> {
        let cwd = clean_path(cwd);
        anyhow::ensure!(cwd.is_dir(), "{} is not a folder", cwd.display());
        anyhow::ensure!(!cmd.trim().is_empty(), "there is no command to start");
        if let Some(warm) = self.warm.as_mut()
            && same_path(&warm.cwd, &cwd)
            && warm.cmd == cmd
            && self.terms.contains_key(&warm.term)
        {
            warm.asked = Instant::now();
            return Ok(());
        }
        self.drop_warm();
        let (cols, rows) = self.guess_size();
        let term = self.spawn(Some(&cmd), &cwd, cols, rows)?;
        if let Some(t) = self.terms.get_mut(&term) {
            t.spare = true;
        }
        self.warm = Some(Warm {
            cwd,
            cmd,
            term,
            asked: Instant::now(),
        });
        Ok(())
    }

    /// Stop the spare, if nothing took it.
    pub(super) fn drop_warm(&mut self) {
        if let Some(warm) = self.warm.take() {
            self.close_term(warm.term);
        }
    }

    /// The spare as the session being started, when it is exactly what was asked for: the same
    /// command in the same folder, still running. It is shown from now on, at the size given.
    pub(super) fn take_warm(
        &mut self,
        cwd: &std::path::Path,
        cmd: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Option<TermId> {
        let fits = self.warm.as_ref().is_some_and(|warm| {
            same_path(&warm.cwd, cwd)
                && cmd == Some(warm.cmd.as_str())
                && self.terms.contains_key(&warm.term)
        });
        if !fits {
            return None;
        }
        let term = self.warm.take()?.term;
        let t = self.terms.get_mut(&term)?;
        t.spare = false;
        t.resize(cols, rows);
        Some(term)
    }

    /// Stop a spare nobody asked for in `WARM_KEEP`, and forget one that ended by itself.
    pub(super) fn expire_warm(&mut self) {
        let Some(warm) = self.warm.as_ref() else {
            return;
        };
        if !self.terms.contains_key(&warm.term) {
            self.warm = None;
        } else if warm.asked.elapsed() >= WARM_KEEP {
            tracing::info!("pane {}: the spare agent was not taken, stopping it", warm.term);
            self.drop_warm();
        }
    }

    /// For tests: the spare's pane, and making it look last asked for `ago`.
    #[cfg(test)]
    pub(super) fn warm_term(&self) -> Option<TermId> {
        self.warm.as_ref().map(|warm| warm.term)
    }

    #[cfg(test)]
    pub(super) fn age_warm(&mut self, ago: Duration) {
        if let Some(warm) = self.warm.as_mut()
            && let Some(asked) = Instant::now().checked_sub(ago)
        {
            warm.asked = asked;
        }
    }
}
