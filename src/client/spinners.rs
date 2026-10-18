//! Preview and persist working indicators without changing an agent's status.

use super::design::{put, seg};
use super::hydra::{HyHit, hints, hit, list_row, panel};
use super::{App, Bg, KeyCode, KeyEvent, Mode, modal, text_input};
use crate::config::{Config, SPINNERS, parse_spinner};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;

const PATH: &str = "ui.spinner";
const INPUT_LIMIT: usize = 1024;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Picker {
    pub sel: usize,
    editing: Option<String>,
    cursor: Option<usize>,
    pub saving: bool,
    error: Option<String>,
}

impl Picker {
    fn frames(&self, cfg: &Config) -> Vec<String> {
        if let Some((_, frames)) = SPINNERS.get(self.sel) {
            return frames.iter().map(|frame| (*frame).into()).collect();
        }
        self.editing
            .as_deref()
            .and_then(|text| parse_spinner(text).ok())
            .unwrap_or_else(|| cfg.ui.spinner.clone())
    }

    pub fn paste(&mut self, text: &str) {
        if self.saving {
            return;
        }
        let Some(editing) = &mut self.editing else {
            return;
        };
        if text.chars().any(|c| c.is_control() && !c.is_whitespace()) {
            self.error = Some("Frames cannot contain control characters.".into());
            return;
        }
        if editing.len() + text.len() > INPUT_LIMIT {
            self.error = Some("Too many characters; use at most 64 frames.".into());
            return;
        }
        text_input::insert(
            editing,
            &mut self.cursor,
            &text.split_whitespace().collect::<Vec<_>>().join(" "),
        );
        self.error = None;
    }
}

impl App {
    pub(super) fn open_spinners(&mut self) {
        let name = crate::config::spinner_name(&self.cfg.ui.spinner);
        let sel = SPINNERS
            .iter()
            .position(|(label, _)| *label == name)
            .unwrap_or(SPINNERS.len());
        self.mode = Mode::Spinners(Box::new(Picker {
            sel,
            editing: None,
            cursor: None,
            saving: false,
            error: None,
        }));
    }

    pub(super) fn spinner_pick(&mut self, index: usize) {
        let Mode::Spinners(mut view) = self.mode.clone() else {
            return;
        };
        if view.saving || index > SPINNERS.len() {
            return;
        }
        let again = view.sel == index;
        view.sel = index;
        view.editing = None;
        view.error = None;
        self.mode = Mode::Spinners(view);
        if again || index == SPINNERS.len() {
            self.on_spinner_key(&KeyEvent::new(KeyCode::Enter, super::KeyModifiers::NONE));
        }
    }

    pub(super) fn on_spinner_key(&mut self, key: &KeyEvent) {
        let Mode::Spinners(mut view) = self.mode.clone() else {
            return;
        };
        if view.saving {
            return;
        }
        if let Some(text) = &mut view.editing {
            match key.code {
                KeyCode::Esc => {
                    view.editing = None;
                    view.error = None;
                }
                KeyCode::Enter => match parse_spinner(text) {
                    Ok(frames) => {
                        self.save_spinner(*view, frames);
                        return;
                    }
                    Err(error) => view.error = Some(error.to_string()),
                },
                KeyCode::Char(c) if c.is_control() => {}
                KeyCode::Char(_)
                    if text.len() >= INPUT_LIMIT
                        && !key.modifiers.intersects(
                            super::KeyModifiers::CONTROL | super::KeyModifiers::ALT,
                        ) =>
                {
                    view.error = Some("Too many characters; use at most 64 frames.".into());
                }
                _ => {
                    text_input::key(text, &mut view.cursor, key);
                    view.error = None;
                }
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    self.back_to_setting(PATH);
                    return;
                }
                KeyCode::Enter if view.sel < SPINNERS.len() => {
                    let frames = view.frames(&self.cfg);
                    self.save_spinner(*view, frames);
                    return;
                }
                KeyCode::Enter => {
                    view.editing = Some(self.cfg.ui.spinner.join(" "));
                    view.cursor = None;
                }
                KeyCode::Down => view.sel = (view.sel + 1).min(SPINNERS.len()),
                KeyCode::Up => view.sel = view.sel.saturating_sub(1),
                KeyCode::Home | KeyCode::PageUp => view.sel = 0,
                KeyCode::End | KeyCode::PageDown => view.sel = SPINNERS.len(),
                _ => {}
            }
        }
        self.mode = Mode::Spinners(view);
    }

    fn save_spinner(&mut self, mut view: Picker, frames: Vec<String>) {
        view.saving = true;
        view.error = None;
        self.mode = Mode::Spinners(Box::new(view));
        self.spawn_bg(move || {
            let value = frames
                .iter()
                .map(String::as_str)
                .collect::<toml_edit::Array>();
            let result = modal::write(PATH, value.into()).and_then(|()| Config::load());
            Bg::Then(Box::new(move |app| match result {
                Ok(cfg) => {
                    app.apply_config(cfg);
                    app.back_to_setting(PATH);
                }
                Err(error) => {
                    if let Mode::Spinners(view) = &mut app.mode {
                        view.saving = false;
                        view.error = Some(format!("Could not save spinner: {error:#}"));
                    }
                }
            }))
        });
    }
}

pub(super) fn draw(app: &mut App, frame: &mut Frame, area: Rect, theme: &Theme, view: &Picker) {
    let buf = frame.buffer_mut();
    super::hydra::dim_all(buf, area, theme);
    let r = panel(app, buf, area, 62, 22, "Working spinner", &[], theme);
    if r.height < 10 || r.width < 16 {
        return;
    }
    let rows = r.height.saturating_sub(9) as usize;
    let start = view.sel.saturating_sub(rows.saturating_sub(1));
    let tick = app.spinner_frame() as usize;
    let current = crate::config::spinner_name(&app.cfg.ui.spinner);
    for i in (start..=SPINNERS.len()).take(rows) {
        let row = Rect {
            x: r.x + 2,
            y: r.y + 2 + (i - start) as u16,
            width: r.width.saturating_sub(4),
            height: 1,
        };
        let style = list_row(app, buf, row, view.sel == i, theme);
        let (name, frames) = SPINNERS.get(i).copied().unwrap_or(("Custom…", &[]));
        let glyph = if i == SPINNERS.len() {
            app.cfg
                .ui
                .spinner
                .get(tick % app.cfg.ui.spinner.len().max(1))
                .map(String::as_str)
        } else {
            frames.get(tick % frames.len().max(1)).copied()
        }
        .unwrap_or(&app.cfg.icons.working);
        put(
            buf,
            row.x + 3,
            row.y,
            &[seg(format!("{glyph}  {name}"), style.fg(theme.working))],
            row.right().saturating_sub(11),
        );
        if current == name || (i == SPINNERS.len() && current == "Custom") {
            put(
                buf,
                row.right().saturating_sub(10),
                row.y,
                &[seg("● Applied", style.fg(theme.accent))],
                row.right(),
            );
        }
        hit(app, row, HyHit::SpinnerPick(i));
    }
    let frames = view.frames(&app.cfg);
    let glyph = frames
        .get(tick % frames.len().max(1))
        .unwrap_or(&app.cfg.icons.working);
    put(
        buf,
        r.x + 3,
        r.bottom() - 7,
        &[seg(
            format!("Preview: {glyph} working"),
            Style::default().fg(theme.working),
        )],
        r.right() - 3,
    );
    let text = if let Some(editing) = &view.editing {
        text_input::window(editing, view.cursor, r.width.saturating_sub(6) as usize)
    } else {
        "Choose a style; Custom lets you enter your own frames.".into()
    };
    put(
        buf,
        r.x + 3,
        r.bottom() - 5,
        &[seg(text, Style::default().fg(theme.strong))],
        r.right() - 3,
    );
    let detail = view.error.as_deref().unwrap_or(if view.saving {
        "Saving…"
    } else if view.editing.is_some() {
        "Separate frames with spaces; each uses one column."
    } else {
        "Preview only until you apply. Static stops the animation."
    });
    put(
        buf,
        r.x + 3,
        r.bottom() - 3,
        &[seg(
            detail,
            Style::default().fg(if view.error.is_some() {
                theme.err
            } else {
                theme.muted
            }),
        )],
        r.right() - 3,
    );
    let help = if view.editing.is_some() {
        hints(
            theme,
            &[
                ("←→", "cursor"),
                ("Ctrl+U", "clear"),
                ("Enter", "save"),
                ("Esc", "back"),
            ],
        )
    } else {
        hints(
            theme,
            &[("↑↓", "preview"), ("Enter", "apply"), ("Esc", "back")],
        )
    };
    put(buf, r.x + 3, r.bottom() - 2, &help, r.right() - 3);
}

#[cfg(test)]
mod tests {
    use super::super::{Hit, KeyModifiers, design};
    use super::*;
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};

    fn app() -> App {
        let (tx, _) = super::super::mpsc::unbounded_channel();
        let (bg, _) = super::super::mpsc::unbounded_channel();
        let mut app = App::new(Config::default(), tx, None, bg);
        app.splash = false;
        app
    }

    fn draw(app: &mut App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| super::super::render::draw(app, f))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        if std::env::var_os("SESHI_SHOW").is_some() {
            println!("{text}");
        }
        text
    }

    #[test]
    fn spinner_picker_supports_mouse_keyboard_preview_cancel_and_custom_input_at_small_sizes() {
        for (w, h) in [(80, 24), (100, 30), (160, 45), (32, 12)] {
            let mut app = app();
            app.back_to_setting(PATH);
            let settings = draw(&mut app, w, h);
            if w >= 80 {
                assert!(settings.contains("Working spinner"));
            }
            app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            assert!(matches!(app.mode, Mode::Spinners(_)));
            let text = draw(&mut app, w, h);
            if w >= 80 {
                assert!(text.contains("Working spinner") && text.contains("Preview:"));
            } else {
                assert!(
                    text.contains("make the window bigger"),
                    "unsupported sizes retain the app's resize hint"
                );
            }
            let original = app.cfg.ui.spinner.clone();
            let preset = 2;
            let hit = app
                .hits
                .iter()
                .find(|(_, hit)| *hit == Hit::Hy(HyHit::SpinnerPick(preset)))
                .map(|(rect, _)| *rect);
            if let Some(rect) = hit {
                app.on_mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: rect.x,
                    row: rect.y,
                    modifiers: KeyModifiers::NONE,
                });
                assert!(matches!(&app.mode, Mode::Spinners(v) if v.sel == preset));
                assert_eq!(
                    app.cfg.ui.spinner, original,
                    "preview does not apply a choice"
                );
            }
            app.on_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
            app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
            app.on_paste("🌈".into());
            app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            assert!(
                matches!(&app.mode, Mode::Spinners(v) if !v.saving && v.error.is_some() && v.editing.as_deref() == Some("🌈"))
            );
            assert_eq!(app.cfg.ui.spinner, original);
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(
                matches!(&app.mode, Mode::HySettings(v) if matches!(design::settings_rows(modal::Cat::Appearance).get(v.sel), Some(design::SRow::Setting(s)) if s.path == PATH))
            );
        }
    }

    #[test]
    fn changing_working_frames_leaves_other_status_icons_intact() {
        use crate::protocol::Status;
        let mut app = app();
        let statuses = [Status::Blocked, Status::Done, Status::Idle, Status::None];
        let before = statuses.map(|status| super::super::render::status_icon(&app, status));
        app.cfg.ui.spinner = vec!["a".into()];
        assert_eq!(
            super::super::render::status_icon(&app, Status::Working),
            "a"
        );
        assert_eq!(
            statuses.map(|status| super::super::render::status_icon(&app, status)),
            before
        );
        app.cfg.ui.spinner.clear();
        assert_eq!(
            super::super::render::status_icon(&app, Status::Working),
            app.cfg.icons.working
        );
    }
}
