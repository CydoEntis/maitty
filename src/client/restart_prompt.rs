//! What a new seshi shows when the server already running is another version: a full-screen
//! choice instead of an error, so one click replaces the old server.

use super::design::{BtnKind, button, fill, put, seg, segs_width};
use crate::config::Config;
use crate::theme::Theme;
use anyhow::Result;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::crossterm::execute;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Restart,
    Quit,
}

/// Rows the text block takes: title, gap, two lines, gap, buttons.
const BLOCK_HEIGHT: u16 = 6;
const BUTTON_GAP: u16 = 2;

pub struct Prompt {
    daemon: u32,
    /// Sessions that come back, and how many of them are mid-turn, when known.
    resume: Option<(usize, Option<usize>)>,
    theme: Theme,
    round: bool,
    hover: Option<Choice>,
    hits: Vec<(Rect, Choice)>,
}

impl Prompt {
    pub fn new(daemon: u32, cfg: &Config) -> Prompt {
        Prompt { daemon, resume: None, theme: cfg.theme(), round: cfg.ui.pill_caps, hover: None, hits: Vec::new() }
    }

    pub fn resuming(mut self, sessions: usize, busy: Option<usize>) -> Prompt {
        self.resume = Some((sessions, busy));
        self
    }

    /// What the restart costs, in the words of the screen's third line.
    fn cost(&self) -> String {
        let sessions = |n: usize| if n == 1 { "1 session resumes".to_string() } else { format!("{n} sessions resume") };
        match self.resume {
            Some((n, Some(busy))) if busy > 0 => format!("{} with their conversation; {busy} mid-turn now.", sessions(n)),
            Some((n, _)) => format!("{} with their conversation.", sessions(n)),
            None => "Running agents stop and resume when it restarts.".to_string(),
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) -> Option<Choice> {
        match k.code {
            KeyCode::Enter => Some(Choice::Restart),
            KeyCode::Esc => Some(Choice::Quit),
            // Raw mode turns Ctrl+C into a key, so it has to be answered here.
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => Some(Choice::Quit),
            _ => None,
        }
    }

    pub fn on_mouse(&mut self, m: MouseEvent) -> Option<Choice> {
        let at = Position::new(m.column, m.row);
        let under = self.hits.iter().find(|(r, _)| r.contains(at)).map(|(_, c)| *c);
        match m.kind {
            MouseEventKind::Moved => {
                self.hover = under;
                None
            }
            MouseEventKind::Down(MouseButton::Left) => under,
            _ => None,
        }
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let t = &self.theme;
        super::screen::set_round(self.round);
        let area = f.area();
        let buf = f.buffer_mut();
        fill(buf, area, t.bg);
        self.hits.clear();
        let ground = Style::default().bg(t.bg);
        let this = crate::protocol::PROTOCOL_VERSION;
        let age = if self.daemon < this { "older" } else { "newer" };
        let cost = self.cost();
        let lines = [
            (0, "Seshi".to_string(), ground.fg(t.strong).add_modifier(Modifier::BOLD)),
            (2, format!("The running Seshi server is {age} (protocol v{}; this one is v{this}).", self.daemon), ground.fg(t.text)),
            (3, cost, ground.fg(t.muted)),
        ];
        let top = area.y + area.height.saturating_sub(BLOCK_HEIGHT) / 2;
        for (row, text, style) in lines {
            let x = area.x + area.width.saturating_sub(text.chars().count() as u16) / 2;
            put(buf, x, top + row, &[seg(text, style)], area.right());
        }
        let y = top + BLOCK_HEIGHT - 1;
        let buttons = [
            (Choice::Restart, button(t, "Restart and open", "Enter", BtnKind::Primary)),
            (Choice::Quit, button(t, "Quit", "Esc", BtnKind::Normal)),
        ];
        let total: u16 = buttons.iter().map(|(_, b)| segs_width(b)).sum::<u16>() + BUTTON_GAP;
        let mut x = area.x + area.width.saturating_sub(total) / 2;
        for (choice, segs) in buttons {
            let w = segs_width(&segs);
            put(buf, x, y, &segs, area.right());
            let r = Rect { x, y, width: w, height: 1 }.intersection(area);
            if self.hover == Some(choice) {
                super::render::tint(buf, r, t);
            }
            if r.width > 0 && r.height > 0 {
                self.hits.push((r, choice));
            }
            x += w + BUTTON_GAP;
        }
    }
}

/// The protocol of the server this prompt is about, when it should be shown: an interactive
/// terminal, a local server, and a version mismatch. Anything else keeps the plain error.
pub fn wanted(err: &anyhow::Error, terminal: bool, remote: bool) -> Option<u32> {
    if !terminal || remote {
        return None;
    }
    crate::ipc::other_version(err)
}

/// Show the prompt on the terminal and wait for the choice.
pub fn offer(daemon: u32, sessions: usize, busy: Option<usize>) -> Result<Choice> {
    let (cfg, _) = Config::load_or_default();
    let mut prompt = Prompt::new(daemon, &cfg).resuming(sessions, busy);
    let mut terminal = ratatui::init();
    if cfg.ui.mouse {
        let _ = execute!(std::io::stdout(), event::EnableMouseCapture);
    }
    let choice = ask(&mut terminal, &mut prompt);
    super::restore_terminal();
    choice
}

fn ask(terminal: &mut ratatui::DefaultTerminal, prompt: &mut Prompt) -> Result<Choice> {
    loop {
        terminal.draw(|f| prompt.draw(f))?;
        let picked = match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => prompt.on_key(k),
            Event::Mouse(m) => prompt.on_mouse(m),
            _ => None,
        };
        if let Some(choice) = picked {
            return Ok(choice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::OtherVersion;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};

    const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];
    /// (ui.layout, ui.panes): Cards floating, Cards tiled, Flat.
    const LAYOUTS: [(&str, &str); 3] = [("cards", "floating"), ("cards", "tiled"), ("flat", "tiled")];
    const OLD_PROTOCOL: u32 = 42;

    fn cfg(layout: (&str, &str)) -> Config {
        let mut cfg = Config::default();
        (cfg.ui.layout, cfg.ui.panes) = (layout.0.into(), layout.1.into());
        cfg
    }

    fn drawn(prompt: &mut Prompt, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| prompt.draw(f)).unwrap();
        let buf = term.backend().buffer().clone();
        let out: String = (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect();
        if std::env::var("SESHI_SHOW").is_ok() {
            println!("{out}");
        }
        out
    }

    fn press(prompt: &mut Prompt, code: KeyCode) -> Option<Choice> {
        prompt.on_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn mouse(prompt: &mut Prompt, kind: MouseEventKind, r: Rect) -> Option<Choice> {
        prompt.on_mouse(MouseEvent { kind, column: r.x + r.width / 2, row: r.y, modifiers: KeyModifiers::NONE })
    }

    fn button_at(prompt: &Prompt, which: Choice) -> Rect {
        prompt.hits.iter().find_map(|(r, c)| (*c == which).then_some(*r)).expect("the button was drawn")
    }

    #[test]
    fn it_names_the_running_version_and_what_restarting_does_in_every_layout_and_size() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut prompt = Prompt::new(OLD_PROTOCOL, &cfg(layout));
                let out = drawn(&mut prompt, w, h);
                let this = crate::protocol::PROTOCOL_VERSION;
                for want in [format!("protocol v{OLD_PROTOCOL}"), format!("v{this}"), "Running agents stop and resume".to_string(), "Restart and open".to_string(), "Quit".to_string()] {
                    assert!(out.contains(&want), "{layout:?} {w}x{h} lacks {want:?}\n{out}");
                }
                for which in [Choice::Restart, Choice::Quit] {
                    let r = button_at(&prompt, which);
                    assert!(r.width > 0 && r.right() <= w && r.bottom() <= h, "{layout:?} {w}x{h}: {which:?} fits {r:?}");
                }
            }
        }
    }

    #[test]
    fn it_counts_the_sessions_that_resume_and_the_ones_mid_turn() {
        let mut prompt = Prompt::new(OLD_PROTOCOL, &cfg(LAYOUTS[0])).resuming(4, Some(2));
        let out = drawn(&mut prompt, 100, 30);
        assert!(out.contains("4 sessions resume with their conversation; 2 mid-turn now."), "{out}");
        let mut prompt = Prompt::new(OLD_PROTOCOL, &cfg(LAYOUTS[0])).resuming(1, None);
        assert!(drawn(&mut prompt, 100, 30).contains("1 session resumes with"));
    }

    #[test]
    fn a_newer_server_is_called_newer() {
        let mut prompt = Prompt::new(crate::protocol::PROTOCOL_VERSION + 1, &cfg(LAYOUTS[0]));
        assert!(drawn(&mut prompt, 100, 30).contains("newer"));
        let mut prompt = Prompt::new(OLD_PROTOCOL, &cfg(LAYOUTS[0]));
        assert!(drawn(&mut prompt, 100, 30).contains("older"));
    }

    #[test]
    fn enter_restarts_and_escape_quits() {
        let mut prompt = Prompt::new(OLD_PROTOCOL, &cfg(LAYOUTS[0]));
        assert_eq!(press(&mut prompt, KeyCode::Enter), Some(Choice::Restart));
        assert_eq!(press(&mut prompt, KeyCode::Esc), Some(Choice::Quit));
        assert_eq!(prompt.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), Some(Choice::Quit), "raw mode leaves Ctrl+C to us");
        assert_eq!(press(&mut prompt, KeyCode::Char('x')), None, "other keys do nothing");
    }

    #[test]
    fn the_buttons_work_by_mouse() {
        for layout in LAYOUTS {
            let mut prompt = Prompt::new(OLD_PROTOCOL, &cfg(layout));
            drawn(&mut prompt, 100, 30);
            let (restart, quit) = (button_at(&prompt, Choice::Restart), button_at(&prompt, Choice::Quit));
            assert_eq!(mouse(&mut prompt, MouseEventKind::Down(MouseButton::Left), restart), Some(Choice::Restart), "{layout:?}");
            assert_eq!(mouse(&mut prompt, MouseEventKind::Down(MouseButton::Left), quit), Some(Choice::Quit), "{layout:?}");
            assert_eq!(mouse(&mut prompt, MouseEventKind::Down(MouseButton::Right), restart), None, "only the left button clicks");
            assert_eq!(prompt.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 0, row: 0, modifiers: KeyModifiers::NONE }), None, "empty space does nothing");
        }
    }

    #[test]
    fn hovering_a_button_lights_it() {
        let mut prompt = Prompt::new(OLD_PROTOCOL, &cfg(LAYOUTS[0]));
        drawn(&mut prompt, 100, 30);
        let quit = button_at(&prompt, Choice::Quit);
        let bg = |prompt: &mut Prompt| {
            let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
            term.draw(|f| prompt.draw(f)).unwrap();
            term.backend().buffer()[(quit.x + 1, quit.y)].bg
        };
        let plain = bg(&mut prompt);
        assert_eq!(mouse(&mut prompt, MouseEventKind::Moved, quit), None);
        assert_eq!(prompt.hover, Some(Choice::Quit));
        assert_ne!(bg(&mut prompt), plain, "the button under the pointer is tinted");
        prompt.on_mouse(MouseEvent { kind: MouseEventKind::Moved, column: 0, row: 0, modifiers: KeyModifiers::NONE });
        assert_eq!(prompt.hover, None);
    }

    #[test]
    fn it_draws_in_a_tiny_window_without_panicking() {
        let mut prompt = Prompt::new(OLD_PROTOCOL, &cfg(LAYOUTS[0]));
        for (w, h) in [(1, 1), (10, 3), (30, 5)] {
            drawn(&mut prompt, w, h);
        }
    }

    #[test]
    fn it_shows_only_for_a_version_mismatch_on_a_local_terminal() {
        let mismatch = || anyhow::Error::new(OtherVersion { daemon: OLD_PROTOCOL });
        assert_eq!(wanted(&mismatch(), true, false), Some(OLD_PROTOCOL));
        assert_eq!(wanted(&mismatch().context("connecting"), true, false), Some(OLD_PROTOCOL), "found under added context");
        assert_eq!(wanted(&mismatch(), false, false), None, "scripts and pipes keep the error");
        assert_eq!(wanted(&mismatch(), true, true), None, "a remote server is never restarted from here");
        assert_eq!(wanted(&anyhow::anyhow!("connection refused"), true, false), None, "other failures keep their error");
    }
}
