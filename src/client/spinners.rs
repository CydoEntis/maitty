//! Preview and persist working indicators without changing an agent's status.

use super::design::{put, seg};
use super::screen::{ScreenHit, hit, list_row, panel, panel_bar};
use super::{App, Bg, KeyCode, KeyEvent, Mode, modal};
use crate::config::{Config, SPINNERS};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;

const PATH: &str = "ui.spinner";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Picker {
    pub sel: usize,
    pub saving: bool,
    error: Option<String>,
}

impl Picker {
    fn frames(&self) -> Vec<String> {
        SPINNERS.get(self.sel).map(|(_, frames)| frames.iter().map(|frame| (*frame).into()).collect()).unwrap_or_default()
    }
}

/// The last row of the list.
fn last() -> usize {
    SPINNERS.len().saturating_sub(1)
}

impl App {
    pub(super) fn open_spinners(&mut self) {
        let name = crate::config::spinner_name(&self.cfg.ui.spinner);
        let sel = SPINNERS.iter().position(|(label, _)| *label == name).unwrap_or(0);
        self.mode = Mode::Spinners(Box::new(Picker { sel, saving: false, error: None }));
    }

    pub(super) fn spinner_pick(&mut self, index: usize) {
        let Mode::Spinners(mut view) = self.mode.clone() else {
            return;
        };
        if view.saving || index > last() {
            return;
        }
        let again = view.sel == index;
        view.sel = index;
        view.error = None;
        self.mode = Mode::Spinners(view);
        if again {
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
        match key.code {
            KeyCode::Esc => {
                self.back_to_setting(PATH);
                return;
            }
            KeyCode::Enter => {
                let frames = view.frames();
                self.save_spinner(*view, frames);
                return;
            }
            KeyCode::Down => view.sel = (view.sel + 1).min(last()),
            KeyCode::Up => view.sel = view.sel.saturating_sub(1),
            KeyCode::Home | KeyCode::PageUp => view.sel = 0,
            KeyCode::End | KeyCode::PageDown => view.sel = last(),
            _ => {}
        }
        self.mode = Mode::Spinners(view);
    }

    fn save_spinner(&mut self, mut view: Picker, frames: Vec<String>) {
        view.saving = true;
        view.error = None;
        self.mode = Mode::Spinners(Box::new(view));
        self.spawn_bg(move || {
            let value = frames.iter().map(String::as_str).collect::<toml_edit::Array>();
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
    super::screen::dim_all(app, buf, area, theme);
    let r = panel(app, buf, area, 62, 24, "Working spinner", &[], theme);
    if r.height < 10 || r.width < 16 {
        return;
    }
    let rows = r.height.saturating_sub(7) as usize;
    let start = view.sel.saturating_sub(rows.saturating_sub(1));
    let tick = app.spinner_frame() as usize;
    let current = crate::config::spinner_name(&app.cfg.ui.spinner);
    for (i, (name, frames)) in SPINNERS.iter().enumerate().skip(start).take(rows) {
        let row = Rect { x: r.x + 2, y: r.y + 2 + (i - start) as u16, width: r.width.saturating_sub(4), height: 1 };
        let style = list_row(app, buf, row, view.sel == i, theme);
        let glyph = frames.get(tick % frames.len().max(1)).copied().unwrap_or(&app.cfg.icons.working);
        put(buf, row.x + 3, row.y, &[seg(format!("{glyph}  {name}"), style.fg(theme.text))], row.right().saturating_sub(11));
        if current == *name {
            put(buf, row.right().saturating_sub(10), row.y, &[seg("● Applied", style.fg(theme.accent))], row.right());
        }
        hit(app, row, ScreenHit::SpinnerPick(i));
    }
    let frames = view.frames();
    let glyph = frames.get(tick % frames.len().max(1)).unwrap_or(&app.cfg.icons.working);
    put(buf, r.x + 3, r.bottom() - 5, &[seg(format!("Preview: {glyph} working"), Style::default().fg(theme.text))], r.right() - 3);
    let detail = view.error.as_deref().unwrap_or(if view.saving { "Saving…" } else { "Preview only until you apply." });
    let tone = if view.error.is_some() { theme.err } else { theme.muted };
    put(buf, r.x + 3, r.bottom() - 3, &[seg(detail, Style::default().fg(tone))], r.right() - 3);
    panel_bar(buf, r, theme, &[("↑↓", "preview"), ("Enter", "apply"), ("Esc", "back")]);
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
        terminal.draw(|f| super::super::render::draw(app, f)).unwrap();
        let buffer = terminal.backend().buffer();
        let text = (0..height).map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect::<String>()).collect::<Vec<_>>().join("\n");
        if std::env::var_os(concat!(env!("APP_ENV"), "_SHOW")).is_some() {
            println!("{text}");
        }
        text
    }

    #[test]
    fn spinner_picker_supports_mouse_keyboard_preview_and_cancel_at_small_sizes() {
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
                assert!(text.contains("make the window bigger"), "unsupported sizes retain the app's resize hint");
            }
            let original = app.cfg.ui.spinner.clone();
            let preset = 2;
            let hit = app.hits.iter().find(|(_, hit)| *hit == Hit::Screen(ScreenHit::SpinnerPick(preset))).map(|(rect, _)| *rect);
            if let Some(rect) = hit {
                app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: rect.x, row: rect.y, modifiers: KeyModifiers::NONE });
                assert!(matches!(&app.mode, Mode::Spinners(v) if v.sel == preset));
                assert_eq!(app.cfg.ui.spinner, original, "preview does not apply a choice");
            }
            app.on_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
            assert!(matches!(&app.mode, Mode::Spinners(v) if v.sel == last()), "End stops at the last spinner; there is no custom row");
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(
                matches!(&app.mode, Mode::ScreenSettings(v) if matches!(design::settings_rows(modal::Cat::Appearance).get(v.sel), Some(design::SRow::Setting(s)) if s.path == PATH))
            );
        }
    }

    #[test]
    fn the_list_has_the_snake_and_no_removed_styles() {
        let names: Vec<&str> = SPINNERS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"Snake"), "{names:?}");
        for gone in ["Circle", "Arrows", "Static"] {
            assert!(!names.contains(&gone), "{gone} is gone: {names:?}");
        }
        assert!(SPINNERS.iter().all(|(_, frames)| !frames.is_empty()), "every style animates");
    }

    #[test]
    fn changing_working_frames_leaves_other_status_icons_intact() {
        use crate::protocol::Status;
        let mut app = app();
        let statuses = [Status::Blocked, Status::Done, Status::Idle, Status::None];
        let before = statuses.map(|status| super::super::render::status_icon(&app, status));
        app.cfg.ui.spinner = vec!["a".into()];
        assert_eq!(super::super::render::status_icon(&app, Status::Working), "a");
        assert_eq!(statuses.map(|status| super::super::render::status_icon(&app, status)), before);
        app.cfg.ui.spinner.clear();
        assert_eq!(super::super::render::status_icon(&app, Status::Working), app.cfg.icons.working);
    }
}
