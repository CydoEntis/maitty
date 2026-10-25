//! Sending a failed check to its agent: `m` on a failed check (its Inbox row or its evidence
//! sheet) opens the agent's message box with the check's command, exit code and the tail of
//! its output written in. Nothing is sent until Enter.
use super::*;
use crate::client::View;
use crate::output_line::clean;
use crate::protocol::{CheckState, ClientMsg, Query};
use std::time::{Duration, Instant};

/// How much of the output's tail goes into the message.
const TAIL_LINES: usize = 40;
/// A line longer than this (a minified bundle, a base64 blob) is cut, so a message stays a
/// size an agent's prompt takes.
const LINE_MAX_CHARS: usize = 400;
/// An output asked for longer ago than this is not waited for any more: the daemon had none.
const OUTPUT_WAIT: Duration = Duration::from_secs(3);

/// A failure waiting for its output before the message box opens.
#[derive(Debug, Clone)]
pub(in crate::client) struct FailureAsk {
    term: TermId,
    name: String,
    inbox: Option<(String, usize)>,
    asked: Instant,
}

/// The message about a failed check: what ran, how it ended, and the last `TAIL_LINES` lines
/// it printed, colour codes and control characters stripped.
pub(super) fn failure_note(info: &CheckInfo, output: &str) -> String {
    // An escape in a repo's command could end the paste it goes in. A carriage return is kept
    // as a space, not read as the output's overprint, so no part of the command is hidden.
    let plain = |s: &str| clean(&s.replace(['\r', '\n'], " "));
    let (name, command) = (plain(&info.name), plain(&info.command));
    let check = if name.is_empty() { "The check".to_string() } else { format!("The {name} check") };
    let ended = match info.exit_code {
        Some(code) => format!("exited with {code}"),
        None => "did not finish".to_string(),
    };
    let mut lines: Vec<String> = output.lines().map(clean).collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    let lines = &lines[lines.len().saturating_sub(TAIL_LINES)..];
    let mut note = format!("{check} failed: `{command}` {ended}.");
    if lines.is_empty() {
        note.push_str(" It printed nothing.");
        return note;
    }
    let shown = if lines.len() == TAIL_LINES { format!("The last {TAIL_LINES} lines of its output:") } else { "Its output:".to_string() };
    note.push_str(&format!(" {shown}\n\n```\n"));
    for line in lines {
        note.push_str(&truncate(line, LINE_MAX_CHARS));
        note.push('\n');
    }
    note.push_str("```");
    note
}

impl App {
    /// `m` on a failed check: the message box to its agent with the failure in it. The sheet
    /// already holds the output; from the Inbox it is asked for first, unless a sheet open on
    /// the same checkpoint holds it.
    pub(in crate::client) fn send_failure(&mut self, term: TermId, name: String, output: Option<&str>, inbox: Option<(String, usize)>) {
        let held = match &self.view {
            Some(View::Check(v)) if v.term == term && v.name == name => v.output.clone(),
            _ => None,
        };
        match output.map(str::to_string).or(held) {
            Some(output) => self.open_failure_note(term, &name, &output, inbox),
            None => {
                if inbox.is_some() {
                    let asked = (!name.is_empty()).then(|| name.clone());
                    self.send(ClientMsg::Query(Query::CheckOutput { term, name: asked }));
                }
                self.screen.failure_ask = Some(FailureAsk { term, name, inbox, asked: Instant::now() });
            }
        }
    }

    /// A check's output arrived: a failure waiting for it opens its message, and a sheet on
    /// the same checkpoint takes it too. Returns whether the sheet should be left alone.
    pub(in crate::client) fn failure_output_arrived(&mut self, text: &str) -> bool {
        let Some(ask) = self.screen.failure_ask.take().filter(|a| a.asked.elapsed() < OUTPUT_WAIT) else { return false };
        let still_there = match &ask.inbox {
            Some(_) => matches!(self.mode, Mode::GoTo { .. }),
            None => self.mode == Mode::Normal && matches!(&self.view, Some(View::Check(v)) if v.term == ask.term),
        };
        if still_there {
            self.open_failure_note(ask.term, &ask.name, text, ask.inbox.clone());
        }
        !matches!(&self.view, Some(View::Check(v)) if v.term == ask.term && v.name == ask.name)
    }

    fn open_failure_note(&mut self, term: TermId, name: &str, output: &str, inbox: Option<(String, usize)>) {
        let Some(info) = self.snap.terms.get(&term).and_then(|t| t.checks.iter().find(|c| c.name == name)).filter(|c| c.state == CheckState::Failed).cloned() else {
            return;
        };
        self.start_compose(term, false, inbox);
        if let Mode::Compose(c) = &mut self.mode {
            let note = failure_note(&info, output);
            c.text = if c.text.trim().is_empty() { note } else { format!("{}\n\n{note}", c.text.trim_end()) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::reference_fixture::{Stage, world};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};
    use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

    /// fix-footer, whose test failed in `Stage::Failed`.
    const FAILED: TermId = 1;

    /// A hundred coloured lines, as a test runner prints them.
    fn sample_output() -> String {
        (1..=100).map(|i| format!("\x1b[31mline {i}\x1b[0m\n")).collect()
    }

    fn listen(app: &mut App) -> UnboundedReceiver<ClientMsg> {
        let (tx, rx) = unbounded_channel();
        app.out = tx;
        rx
    }

    fn drain(rx: &mut UnboundedReceiver<ClientMsg>) -> Vec<ClientMsg> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    /// Whether anything was typed or queued for the agent.
    fn delivered(sent: &[ClientMsg]) -> bool {
        sent.iter().any(|m| matches!(m, ClientMsg::Input { term, .. } | ClientMsg::Command(crate::protocol::Command::QueueFollowUp { term, .. }) if *term == FAILED))
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn inbox_on_failure(app: &mut App) {
        app.screen_fresh();
        let sel = goto_rows(&app.screen_model(), "", 0).iter().position(|r| *r == GoRow::Failed(FAILED)).expect("the failed item");
        app.mode = Mode::GoTo { query: String::new(), sel };
    }

    fn written(app: &App) -> &Compose {
        match &app.mode {
            Mode::Compose(c) => c,
            other => panic!("no message box: {other:?}"),
        }
    }

    fn the_failure_is_written_in(app: &App) {
        let c = written(app);
        let exit = app.snap.terms[&FAILED].check().and_then(|i| i.exit_code).expect("an exit code");
        assert_eq!(c.term, FAILED);
        assert!(c.text.contains(&format!("exited with {exit}.")) && c.text.contains("The last 40 lines"), "{}", c.text);
        assert!(c.text.contains("```\nline 61\n") && c.text.contains("line 100\n```") && !c.text.contains('\x1b'), "{}", c.text);
    }

    fn mode_bar(app: &mut App) -> String {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| crate::client::render::draw(app, f)).unwrap();
        let buf = term.backend().buffer();
        let row = mode_bar_row(app, buf.area);
        (row.x..row.right()).map(|x| buf[(x, row.y)].symbol()).collect()
    }

    #[test]
    fn m_on_a_failed_inbox_item_writes_the_failure_into_the_message_and_sends_nothing() {
        let mut app = world(Stage::Failed);
        let mut rx = listen(&mut app);
        inbox_on_failure(&mut app);
        press(&mut app, KeyCode::Char('m'));
        let sent = drain(&mut rx);
        assert!(sent.iter().any(|m| matches!(m, ClientMsg::Query(Query::CheckOutput { term: FAILED, .. }))), "its output is asked for: {sent:?}");
        assert!(matches!(app.mode, Mode::GoTo { .. }), "the Inbox waits for it");
        app.check_output_arrived(sample_output());
        the_failure_is_written_in(&app);
        assert!(written(&app).inbox.is_some(), "in the Inbox row");
        assert!(!delivered(&drain(&mut rx)), "nothing goes before Enter");
        press(&mut app, KeyCode::Enter);
        assert!(delivered(&drain(&mut rx)), "Enter sends it");
    }

    #[test]
    fn an_error_for_the_output_stops_waiting_for_it() {
        use crate::protocol::{Reply, ServerMsg};
        let mut app = world(Stage::Failed);
        inbox_on_failure(&mut app);
        press(&mut app, KeyCode::Char('m'));
        app.on_server(ServerMsg::Error("no check output for pane 1".into()));
        app.on_server(ServerMsg::Reply(Reply::Text(sample_output())));
        assert!(matches!(app.mode, Mode::GoTo { .. }), "a later reply is not taken for it: {:?}", app.mode);
    }

    #[test]
    fn m_in_the_inbox_uses_the_output_a_sheet_on_that_check_holds() {
        let mut app = world(Stage::Failed);
        app.open_check_sheet(FAILED);
        app.check_output_arrived(sample_output());
        let mut rx = listen(&mut app);
        inbox_on_failure(&mut app);
        press(&mut app, KeyCode::Char('m'));
        the_failure_is_written_in(&app);
        assert!(!drain(&mut rx).iter().any(|m| matches!(m, ClientMsg::Query(Query::CheckOutput { .. }))), "not asked for again");
    }

    #[test]
    fn m_on_a_failed_check_s_sheet_writes_the_failure_into_the_message() {
        let mut app = world(Stage::Failed);
        app.open_check_sheet(FAILED);
        app.check_output_arrived(sample_output());
        let mut rx = listen(&mut app);
        press(&mut app, KeyCode::Char('m'));
        the_failure_is_written_in(&app);
        assert!(!delivered(&drain(&mut rx)), "nothing goes before Enter");
        press(&mut app, KeyCode::Esc);
        assert!(app.screen.drafts[&FAILED].contains("line 100"), "Esc keeps it as a draft");
        assert!(matches!(&app.view, Some(View::Check(_))), "the sheet stays");
    }

    #[test]
    fn m_before_the_sheet_s_output_arrives_waits_for_it() {
        let mut app = world(Stage::Failed);
        app.open_check_sheet(FAILED);
        press(&mut app, KeyCode::Char('m'));
        assert_eq!(app.mode, Mode::Normal);
        app.check_output_arrived(sample_output());
        the_failure_is_written_in(&app);
        assert!(matches!(&app.view, Some(View::Check(v)) if v.output.is_some()), "the sheet has it too");
    }

    #[test]
    fn m_on_a_check_that_did_not_fail_does_nothing() {
        let mut app = world(Stage::Passed);
        app.open_check_sheet(FAILED);
        app.check_output_arrived(sample_output());
        press(&mut app, KeyCode::Char('m'));
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn the_mode_bar_offers_m_on_a_failure() {
        let mut app = world(Stage::Failed);
        inbox_on_failure(&mut app);
        assert!(mode_bar(&mut app).contains("· m send to agent"), "the Inbox");
        app.mode = Mode::Normal;
        app.open_check_sheet(FAILED);
        assert!(mode_bar(&mut app).contains("· m send to agent"), "the sheet");
        let mut app = world(Stage::Passed);
        app.open_check_sheet(FAILED);
        assert!(!mode_bar(&mut app).contains("send to agent"), "not on a pass");
    }

    fn failed(exit: Option<i32>) -> CheckInfo {
        CheckInfo { name: "test".into(), command: "npm test".into(), state: CheckState::Failed, exit_code: exit, ..Default::default() }
    }

    #[test]
    fn a_note_names_the_check_its_command_and_its_exit_code() {
        let note = failure_note(&failed(Some(1)), "FAIL footer\n");
        assert!(note.starts_with("The test check failed: `npm test` exited with 1. Its output:\n\n```\nFAIL footer\n```"), "{note}");
        assert!(failure_note(&failed(None), "x").contains("`npm test` did not finish."));
    }

    #[test]
    fn a_note_keeps_the_last_forty_lines_without_colour_codes() {
        let output: String = (1..=100).map(|i| format!("\x1b[31mline {i}\x1b[0m\n")).collect();
        let note = failure_note(&failed(Some(2)), &output);
        assert!(note.contains("The last 40 lines of its output:"), "{note}");
        assert!(!note.contains("line 60\n") && note.contains("```\nline 61\n") && note.contains("line 100\n```"), "{note}");
        assert!(!note.contains('\x1b'), "colour codes stripped");
    }

    #[test]
    fn escapes_in_the_check_s_name_and_command_are_stripped() {
        let info = CheckInfo { name: "te\x1b[201~st".into(), command: "npm test\x1b[201~\rrm -rf x".into(), ..failed(Some(1)) };
        let note = failure_note(&info, "FAIL\n");
        assert!(!note.contains('\x1b') && !note.contains('\r'), "{note:?}");
        assert!(note.starts_with("The test check failed: `npm test rm -rf x` exited with 1."), "nothing hidden: {note:?}");
    }

    #[test]
    fn a_very_long_line_is_cut() {
        let note = failure_note(&failed(Some(1)), &"x".repeat(10_000));
        assert!(note.len() < 1_000 && note.contains('…'), "{}", note.len());
    }

    #[test]
    fn a_failure_that_printed_nothing_says_so() {
        assert_eq!(failure_note(&failed(Some(1)), "\n \n"), "The test check failed: `npm test` exited with 1. It printed nothing.");
    }
}
