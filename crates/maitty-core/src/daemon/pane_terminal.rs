//! A terminal pane as another client sees it: the screen, its history, typing and resizing,
//! keyed by the pane itself (the desktop app's terminal sessions have no conversation).

use super::{ClientId, Daemon, Grant};
use crate::protocol::{Status, TermId};
use anyhow::Result;

const MAX_INPUT_BYTES: usize = 16 * 1024;
const MIN_DIMENSION: u16 = 2;
const MAX_COLS: u16 = 500;
const MAX_ROWS: u16 = 300;

impl Daemon {
    /// The visible screen as ANSI text a terminal emulator can repaint from.
    pub(super) fn pane_screen(&self, term: TermId) -> Result<String> {
        let pane = self
            .terms
            .get(&term)
            .ok_or_else(|| anyhow::anyhow!("That terminal is not running"))?;
        let mut screen = b"\x1b[0m\x1b[2J\x1b[H".to_vec();
        screen.extend(pane.parser.screen().state_formatted());
        Ok(String::from_utf8_lossy(&screen).into_owned())
    }

    /// The pane's recent raw output, for the scrollback a new view starts with.
    pub(super) fn pane_replay(&self, term: TermId) -> Result<Vec<u8>> {
        Ok(self
            .terms
            .get(&term)
            .ok_or_else(|| anyhow::anyhow!("That terminal is not running"))?
            .replay())
    }

    /// Type into a pane: write needs the write grant, and a pane that is asking also needs respond.
    pub(super) fn pane_input(&mut self, client: ClientId, term: TermId, data: Vec<u8>) -> Result<()> {
        anyhow::ensure!(
            !data.is_empty() && data.len() <= MAX_INPUT_BYTES,
            "Terminal input must contain between 1 and {MAX_INPUT_BYTES} bytes"
        );
        let asking = self
            .terms
            .get(&term)
            .ok_or_else(|| anyhow::anyhow!("That terminal is not running"))?
            .status
            == Status::Blocked;
        self.may(client, Grant::Write, Some(term))?;
        if asking {
            self.may(client, Grant::Respond, Some(term))?;
        }
        if let Some(pane) = self.terms.get_mut(&term) {
            pane.input(&data);
        }
        Ok(())
    }

    pub(super) fn pane_resize(
        &mut self,
        client: ClientId,
        term: TermId,
        cols: u16,
        rows: u16,
    ) -> Result<()> {
        anyhow::ensure!(
            (MIN_DIMENSION..=MAX_COLS).contains(&cols) && (MIN_DIMENSION..=MAX_ROWS).contains(&rows),
            "Terminal size must be between 2x2 and 500x300"
        );
        self.may(client, Grant::Write, Some(term))?;
        self.terms
            .get_mut(&term)
            .ok_or_else(|| anyhow::anyhow!("That terminal is not running"))?
            .resize(cols, rows);
        self.dirty = true;
        Ok(())
    }
}
