//! A subagent's own steps, read from the record its agent keeps (ADR 0010), in a modal opened from
//! its sidebar row. Asked again every so often while it works. Display only.

use super::*;
use crate::conversation::history::{Content, Entry, Role};
use crate::protocol::{ClientMsg, HelperRecord, Query};
use std::time::{Duration, Instant};

/// How often a working helper's record is asked for again.
const HELPER_EVERY: Duration = Duration::from_millis(1500);
/// The widest the modal grows.
const HELPER_W: u16 = 100;
/// The longest a tool's output line is shown.
const OUTPUT_LINE_MAX: usize = 160;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct HelperView {
    pub term: ClientTerm,
    pub id: String,
    pub name: String,
    /// Its lines, or why they could not be read; `None` until the first answer.
    pub lines: Option<Result<Vec<HelperLine>, String>>,
    pub start_missing: bool,
    asked: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) enum HelperLine {
    Asked(String),
    Said(String),
    Ran(String),
}

/// A record's entries as lines: what it was asked, what it said, what it ran.
pub(in crate::client) fn helper_lines(entries: &[Entry]) -> Vec<HelperLine> {
    entries
        .iter()
        .filter_map(|e| match &e.content {
            Content::Message { role: Role::User, text } => Some(HelperLine::Asked(text.clone())),
            Content::Message { role: Role::Assistant, text } => Some(HelperLine::Said(text.clone())),
            Content::Tool { kind, label, output, .. } => {
                let name = if label.is_empty() { kind } else { label };
                let first = output.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
                Some(HelperLine::Ran(truncate(&format!("{name} {first}"), OUTPUT_LINE_MAX)))
            }
            _ => None,
        })
        .collect()
}

impl App {
    /// Opens a subagent's steps. Another machine's, or an older daemon, still opens its parent's pane.
    pub(in crate::client) fn open_helper(&mut self, term: ClientTerm, id: String, name: String) {
        let ask = ClientMsg::Query(Query::HelperRead { term: term.term, helper: id.clone() });
        if term.machine != MachineId::LOCAL || ask.introduced_in() > self.local_protocol {
            self.screen_focus(term);
            return;
        }
        self.send(term.machine, ask);
        self.mode = Mode::Helper(Box::new(HelperView { term, id, name, lines: None, start_missing: false, asked: Instant::now() }));
    }

    /// The daemon's answer for the open helper.
    pub(in crate::client) fn helper_arrived(&mut self, record: HelperRecord) {
        if let Mode::Helper(v) = &mut self.mode
            && v.term.term == record.term
            && v.id == record.helper
        {
            v.lines = Some(Ok(helper_lines(&record.entries)));
            v.start_missing = record.start_missing;
        }
    }

    /// The daemon could not read the open helper's record. True when the error was for it.
    pub(in crate::client) fn helper_failed(&mut self, why: &str) -> bool {
        match &mut self.mode {
            Mode::Helper(v) if v.lines.is_none() => {
                v.lines = Some(Err(why.to_string()));
                true
            }
            _ => false,
        }
    }

    /// Asks again for a helper that is still working.
    pub(in crate::client) fn refresh_helper(&mut self) {
        let Mode::Helper(v) = &self.mode else {
            return;
        };
        let working = self
            .term_info(v.term)
            .and_then(|i| i.subagent_activity.iter().find(|a| a.id == v.id))
            .is_some_and(|a| a.state.active());
        if !working || v.asked.elapsed() < HELPER_EVERY || matches!(v.lines, Some(Err(_))) {
            return;
        }
        let (term, id) = (v.term, v.id.clone());
        self.send(term.machine, ClientMsg::Query(Query::HelperRead { term: term.term, helper: id }));
        if let Mode::Helper(v) = &mut self.mode {
            v.asked = Instant::now();
        }
    }

    pub(in crate::client) fn on_helper_key(&mut self, view: HelperView, k: &KeyEvent) {
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Normal,
            // Enter goes to the parent's pane, where the agent shows its subagents itself.
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                self.screen_focus(view.term);
            }
            _ => self.mode = Mode::Helper(Box::new(view)),
        }
    }
}

/// The modal: its newest lines that fit, oldest first.
pub(in crate::client) fn draw_helper(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, view: &HelperView) {
    let buf = f.buffer_mut();
    dim_all(app, buf, area, t);
    let h = area.height.saturating_sub(6).max(8);
    let r = panel(app, buf, area, HELPER_W, h, &format!("subagent {}", view.name), &[], t);
    let ground = Style::default().bg(t.card);
    let (x, end) = (r.x + 3, r.right().saturating_sub(3));
    let rows = r.height.saturating_sub(4) as usize;
    let say = |text: &str, style: Style| vec![seg(text.to_string(), style)];
    let lines: Vec<Vec<Seg>> = match &view.lines {
        None => vec![say("Reading what it did…", ground.fg(t.muted))],
        Some(Err(why)) => vec![say(&format!("Its record could not be read: {why}"), ground.fg(t.muted))],
        Some(Ok(lines)) if lines.is_empty() => vec![say("It has not done anything yet.", ground.fg(t.muted))],
        Some(Ok(lines)) => {
            let mut out: Vec<Vec<Seg>> = Vec::new();
            if view.start_missing {
                out.push(say("Its earlier steps are not shown.", ground.fg(t.muted)));
            }
            for line in lines {
                let (lead, text, style) = match line {
                    HelperLine::Asked(text) => ("asked ", text, ground.fg(t.strong)),
                    HelperLine::Said(text) => ("", text, ground.fg(t.text)),
                    HelperLine::Ran(text) => ("▸ ", text, ground.fg(t.muted)),
                };
                for (n, part) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
                    let lead = if n == 0 { lead } else { "" };
                    out.push(vec![seg(lead.to_string(), ground.fg(t.muted)), seg(part.to_string(), style)]);
                }
            }
            out
        }
    };
    let skip = lines.len().saturating_sub(rows);
    for (n, line) in lines.into_iter().skip(skip).enumerate() {
        put(buf, x, r.y + 2 + n as u16, &line, end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::ConversationId;
    use crate::conversation::history::{ItemState, SourceId};

    fn entry(content: Content) -> Entry {
        Entry {
            source: SourceId { conversation: ConversationId(0), turn: "t".into(), item: "i".into() },
            content,
            state: ItemState::Completed,
            started_at_ms: None,
            completed_at_ms: None,
        }
    }

    #[test]
    fn a_record_reads_as_what_it_was_asked_what_it_ran_and_what_it_said() {
        let lines = helper_lines(&[
            entry(Content::TurnStatus),
            entry(Content::Message { role: Role::User, text: "count the files".into() }),
            entry(Content::Tool {
                kind: "commandExecution".into(),
                output: "\na.txt\nb.txt".into(),
                label: "ls".into(),
                input: None,
                exit_code: None,
                duration_ms: None,
            }),
            entry(Content::Message { role: Role::Assistant, text: "There are 2.".into() }),
        ]);
        assert_eq!(
            lines,
            [
                HelperLine::Asked("count the files".into()),
                HelperLine::Ran("ls a.txt".into()),
                HelperLine::Said("There are 2.".into()),
            ]
        );
    }
}
