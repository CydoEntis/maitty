//! Search and preview built-in palettes before saving a choice.

use super::design::{fill, put, seg, theme_label};
use super::screen::{ScreenHit, hit, list_row, panel_bar, query_list};
use super::{App, Bg, KeyCode, KeyEvent, KeyModifiers, Mode, modal};
use crate::theme::{BUILTIN, DEFAULT, Theme};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

const QUERY_LIMIT: usize = 80;
const PREVIEW_ROWS: u16 = 7;
const PAGE_ROWS: usize = 10;

pub(super) fn append_query(query: &mut String, text: &str) {
    let remaining = QUERY_LIMIT.saturating_sub(query.chars().count());
    query.extend(text.chars().filter(|c| !c.is_control()).take(remaining));
}

/// The themes of one tab (light or dark, read from each theme's ground) that match `query`.
pub(super) fn matching(query: &str, light: bool) -> Vec<&'static str> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    BUILTIN
        .iter()
        .copied()
        .filter(|name| crate::theme::is_light(name) == light)
        .filter(|name| {
            let label = theme_label(name).to_lowercase();
            words
                .iter()
                .all(|word| label.contains(word) || name.contains(word))
        })
        .collect()
}

/// The setting a tab's pick is saved in.
fn slot_path(light: bool) -> &'static str {
    if light { "theme_light" } else { "theme_dark" }
}

impl App {
    /// The theme a tab has chosen.
    fn slot_theme(&self, light: bool) -> &str {
        let name = if light {
            &self.cfg.theme_light
        } else {
            &self.cfg.theme_dark
        };
        match name.as_str() {
            "" => DEFAULT,
            "drover" => "hydra",
            name => crate::theme::canonical(name),
        }
    }

    /// Open the browser on the Light (true) or Dark (false) tab, on that tab's chosen theme.
    pub(super) fn open_themes_on(&mut self, light: bool) {
        self.theme_tab_light = light;
        self.mode = Mode::Themes {
            query: String::new(),
            sel: self.slot_row(light),
            saving: false,
        };
    }

    /// Where a tab's chosen theme sits in that tab's list (the top when it is not there).
    fn slot_row(&self, light: bool) -> usize {
        let current = self.slot_theme(light);
        matching("", light)
            .iter()
            .position(|name| *name == current)
            .unwrap_or(0)
    }

    /// Flip between the dark theme and the light one, each as chosen in Settings. The whole
    /// screen changes at once; the choice is remembered.
    pub(super) fn toggle_appearance(&mut self) {
        let next = if self.cfg.appearance == "light" {
            "dark"
        } else {
            "light"
        };
        self.spawn_bg(move || {
            let result = modal::write("appearance", next.into())
                .and_then(|()| crate::config::Config::load());
            Bg::Then(Box::new(move |app| match result {
                Ok(cfg) => {
                    app.apply_config(cfg);
                    let name = theme_label(crate::theme::canonical(&app.cfg.theme)).to_string();
                    let which = if next == "light" { "Light" } else { "Dark" };
                    app.notify(format!("{which} theme: {name}"), false);
                }
                Err(error) => app.notify(format!("couldn't switch theme: {error:#}"), true),
            }))
        });
    }

    /// Switch tabs, landing on the theme that tab has chosen.
    pub(super) fn theme_tab(&mut self, light: bool) {
        if matches!(self.mode, Mode::Themes { saving: false, .. }) {
            self.open_themes_on(light);
        }
    }

    pub(super) fn on_theme_key(
        &mut self,
        mut query: String,
        mut sel: usize,
        saving: bool,
        key: &KeyEvent,
    ) {
        if saving {
            return;
        }
        let light = self.theme_tab_light;
        let names = matching(&query, light);
        let last = names.len().saturating_sub(1);
        sel = sel.min(last);
        match key.code {
            KeyCode::Esc => {
                self.back_to_setting(slot_path(light));
                return;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.theme_tab(!light);
                return;
            }
            KeyCode::Enter => {
                let Some(&name) = names.get(sel) else { return };
                self.mode = Mode::Themes {
                    query,
                    sel,
                    saving: true,
                };
                self.spawn_bg(move || {
                    let result = modal::write(slot_path(light), name.into())
                        .and_then(|()| crate::config::Config::load());
                    Bg::Then(Box::new(move |app| match result {
                        Ok(cfg) => {
                            app.apply_config(cfg);
                            app.back_to_setting(slot_path(light));
                        }
                        Err(error) => {
                            if let Mode::Themes { saving, .. } = &mut app.mode {
                                *saving = false;
                            }
                            app.notify(format!("couldn't save theme: {error:#}"), true);
                        }
                    }))
                });
                return;
            }
            KeyCode::Down => sel = (sel + 1).min(last),
            KeyCode::Up => sel = sel.saturating_sub(1),
            KeyCode::PageDown => sel = (sel + PAGE_ROWS).min(last),
            KeyCode::PageUp => sel = sel.saturating_sub(PAGE_ROWS),
            KeyCode::Home => sel = 0,
            KeyCode::End => sel = last,
            KeyCode::Backspace => {
                query.pop();
                sel = 0;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                query.clear();
                sel = 0;
            }
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                append_query(&mut query, &ch.to_string());
                sel = 0;
            }
            _ => {}
        }
        self.mode = Mode::Themes {
            query,
            sel,
            saving: false,
        };
    }
}

pub(super) fn draw_themes(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    query: &str,
    sel: usize,
    saving: bool,
) {
    let light = app.theme_tab_light;
    let names = matching(query, light);
    let sel = sel.min(names.len().saturating_sub(1));
    let current = app.slot_theme(light).to_string();
    let buf = frame.buffer_mut();
    let (panel, mut list, _) = query_list(
        app,
        buf,
        area,
        theme,
        if light { "Light themes" } else { "Dark themes" },
        80,
        names.len().clamp(6, 14) + PREVIEW_ROWS as usize,
        query,
        "type a theme or flavor",
        sel,
    );
    if list.height == 0 {
        list.y = panel.y + 3;
        list.height = 1;
    }
    let has_preview = list.height > PREVIEW_ROWS;
    if has_preview {
        list.height = list.height.saturating_sub(PREVIEW_ROWS);
    }
    let start = sel.saturating_sub(list.height.saturating_sub(1) as usize);
    for (i, name) in names
        .iter()
        .enumerate()
        .skip(start)
        .take(list.height as usize)
    {
        let row = Rect {
            y: list.y + (i - start) as u16,
            height: 1,
            ..list
        };
        let st = list_row(app, buf, row, i == sel, theme);
        let active = *name == current;
        let label = format!("{} {}", if active { "●" } else { "○" }, theme_label(name));
        let swatch_x = row.right().saturating_sub(24);
        put(
            buf,
            row.x + 3,
            row.y,
            &[seg(
                label,
                st.fg(if i == sel { theme.strong } else { theme.text }),
            )],
            swatch_x,
        );
        let palette = Theme::named(name);
        let ansi = palette.ansi.unwrap_or([
            palette.err,
            palette.done,
            palette.working,
            palette.accent,
            palette.sky(),
            palette.accent,
            palette.muted,
        ]);
        for (j, color) in [
            palette.bg,
            palette.accent,
            ansi[0],
            ansi[1],
            ansi[2],
            ansi[3],
            ansi[4],
        ]
        .into_iter()
        .enumerate()
        {
            let x = swatch_x + j as u16 * 3;
            if x >= row.x + 3 && x + 2 < row.right() {
                fill(
                    buf,
                    Rect {
                        x,
                        y: row.y,
                        width: 2,
                        height: 1,
                    },
                    color,
                );
            }
        }
        hit(app, row, ScreenHit::ThemePick(i));
    }
    if names.is_empty() {
        put(
            buf,
            list.x + 3,
            list.y,
            &[seg(
                "No themes match. Backspace or Ctrl+U to clear.",
                Style::default().bg(theme.card).fg(theme.muted),
            )],
            list.right(),
        );
    }
    if has_preview && let Some(name) = names.get(sel) {
        draw_preview(
            buf,
            Rect {
                y: list.bottom() + 1,
                height: PREVIEW_ROWS - 1,
                ..list
            },
            name,
        );
    }
    let count = if saving {
        "Saving…".to_string()
    } else {
        format!(
            "{} / {} themes · ● applied",
            names.len(),
            matching("", light).len()
        )
    };
    let st = Style::default().bg(theme.card).fg(theme.muted);
    if panel.height > 8 {
        let y = panel.bottom().saturating_sub(3);
        // The two tabs, each a chip (the open one lit); a click switches, as Tab does.
        let mut x = panel.x + 3;
        for (tab_light, label) in [(false, "Dark"), (true, "Light")] {
            let open = tab_light == light;
            let chip = super::screen::chip(
                &format!(" {label} "),
                if open {
                    Style::default()
                        .bg(theme.accent)
                        .fg(theme.acc_ink)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().bg(theme.btn).fg(theme.strong)
                },
            );
            let w = super::design::segs_width(&chip);
            put(buf, x, y, &chip, panel.right().saturating_sub(2));
            hit(
                app,
                Rect {
                    x,
                    y,
                    width: w,
                    height: 1,
                },
                ScreenHit::ThemeTab(tab_light),
            );
            x += w + 1;
        }
        put(
            buf,
            x + 1,
            y,
            &[seg(count, st)],
            panel.right().saturating_sub(2),
        );
    }
    // The key hints sit where the one list row would on a tiny screen: the list wins.
    if panel.height >= THEMES_BAR_MIN_H {
        panel_bar(
            buf,
            panel,
            theme,
            &[
                ("↑↓", "move"),
                ("Tab", "dark/light"),
                ("Enter", "apply"),
                ("Esc", "back"),
            ],
        );
    }
}

/// The fewest rows the Themes panel needs to show its key hints under a list row.
const THEMES_BAR_MIN_H: u16 = 7;

fn draw_preview(buf: &mut ratatui::buffer::Buffer, area: Rect, name: &str) {
    let theme = Theme::named(name);
    fill(buf, area, theme.bg);
    let st = Style::default().bg(theme.bg);
    put(
        buf,
        area.x + 2,
        area.y,
        &[seg(
            format!("PREVIEW · {}", theme_label(name)),
            st.fg(theme.strong).add_modifier(Modifier::BOLD),
        )],
        area.right(),
    );
    put(
        buf,
        area.x + 2,
        area.y + 1,
        &[
            super::screen::plain(vec![seg(
                " Focused pane ",
                st.bg(theme.accent).fg(theme.acc_ink),
            )]),
            vec![
                seg("  Regular text  ", st.fg(theme.text)),
                seg("dimmed", st.fg(theme.muted)),
            ],
        ]
        .concat(),
        area.right(),
    );
    put(
        buf,
        area.x + 2,
        area.y + 3,
        &[
            seg("● working  ", st.fg(theme.text)),
            seg("! needs you  ", st.fg(theme.blocked)),
            seg("✓ done  ", st.fg(theme.done)),
            seg("○ idle", st.fg(theme.idle)),
        ],
        area.right(),
    );
    put(
        buf,
        area.x + 2,
        area.y + 4,
        &[seg(
            concat!("UI sample; Enter applies across ", env!("APP_NAME"), "."),
            st.fg(theme.muted),
        )],
        area.right(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn app() -> App {
        let (tx, _) = super::super::mpsc::unbounded_channel();
        let (bg, _) = super::super::mpsc::unbounded_channel();
        let mut app = App::new(crate::config::Config::default(), tx, None, bg);
        app.splash = false;
        app
    }

    fn draw(app: &mut App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| super::super::render::draw(app, frame))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            for y in 0..height {
                println!(
                    "{}",
                    (0..width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                );
            }
        }
        buffer
    }

    #[test]
    fn theme_browser_opens_on_the_theme_an_old_name_became() {
        let mut app = app();
        app.cfg.theme_dark = "melange".into();
        app.open_themes_on(false);
        assert!(
            matches!(app.mode, Mode::Themes { sel, .. } if matching("", false)[sel] == "melange-dark")
        );
    }

    #[test]
    fn theme_browser_filters_previews_and_returns_without_changing_the_theme() {
        let mut app = app();
        app.cfg.theme = "dracula".into();
        app.cfg.theme_dark = "dracula".into();
        app.theme = Theme::named("dracula");
        app.open_themes_on(false);
        assert!(
            matches!(app.mode, Mode::Themes { sel, .. } if matching("", false)[sel] == "dracula")
        );
        app.on_paste("CATPPUCCIN mocha\nignored".into());
        assert_eq!(
            matching("CATPPUCCIN mocha", false),
            vec!["catppuccin-mocha"]
        );
        let buffer = draw(&mut app, 100, 30);
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("PREVIEW · Catppuccin Mocha"));
        assert!(text.contains("1 / "));
        assert!(
            app.hits
                .iter()
                .any(|(_, hit)| matches!(hit, super::super::Hit::Screen(ScreenHit::ThemePick(0))))
        );
        assert!(
            buffer
                .content()
                .iter()
                .any(|cell| cell.bg == Theme::named("catppuccin-mocha").bg)
        );
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.cfg.theme, "dracula");
        assert_eq!(app.theme.bg, Theme::named("dracula").bg);
        assert!(
            matches!(&app.mode, Mode::ScreenSettings(view) if view.cat == 1 && matches!(super::super::design::settings_rows(modal::Cat::Appearance).get(view.sel), Some(super::super::design::SRow::Setting(s)) if s.path == "theme_dark"))
        );
    }

    #[test]
    fn theme_browser_pages_to_the_end_and_handles_empty_searches_and_small_screens() {
        let mut app = app();
        app.open_themes_on(false);
        let dark = matching("", false);
        let default = dark.iter().position(|name| *name == DEFAULT).unwrap();
        app.on_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Themes { sel, .. } if sel == default + PAGE_ROWS));
        app.on_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        // At 40 columns the swatches clip the label to its first few letters.
        let last: String = theme_label(dark.last().unwrap()).chars().take(6).collect();
        for (width, height) in [(100, 30), (80, 24), (40, 10)] {
            let buffer = draw(&mut app, width, height);
            let text = buffer
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(
                text.contains(&last),
                "the last theme remains visible at {width}x{height}"
            );
        }
        app.on_paste("no-such-theme".into());
        assert!(matching("no-such-theme", false).is_empty());
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Themes { saving: false, .. }));
        let buffer = draw(&mut app, 100, 30);
        assert!(
            buffer
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("No themes match")
        );
        app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(matches!(app.mode, Mode::Themes { ref query, sel: 0, .. } if query.is_empty()));
        app.on_paste("x".repeat(QUERY_LIMIT + 1));
        assert!(matches!(app.mode, Mode::Themes { ref query, .. } if query.len() == QUERY_LIMIT));
    }

    #[test]
    fn the_appearance_switch_changes_the_whole_screen_and_each_slot_keeps_its_pick() {
        let mut app = app();
        let mut cfg = app.cfg.clone();
        cfg.theme_dark = "dracula".into();
        cfg.theme_light = "vim".into();
        cfg.appearance = "dark".into();
        cfg.settle_theme([true; 3]);
        app.apply_config(cfg.clone());
        assert_eq!(app.theme.bg, Theme::named("dracula").bg);
        cfg.appearance = "light".into();
        cfg.settle_theme([true; 3]);
        app.apply_config(cfg.clone());
        assert_eq!(
            app.theme.bg,
            Theme::named("vim").bg,
            "the light slot's theme shows"
        );
        assert!(app.theme.is_light());
        // Back again: the dark slot still holds the dark pick.
        cfg.appearance = "dark".into();
        cfg.settle_theme([true; 3]);
        app.apply_config(cfg);
        assert_eq!(app.theme.bg, Theme::named("dracula").bg);
    }

    #[test]
    fn a_tab_click_switches_tabs_and_lands_on_the_theme_that_tab_chose() {
        let mut app = app();
        app.cfg.theme_light = "vim".into();
        app.open_themes_on(false);
        let before = draw(&mut app, 100, 30);
        assert!(
            app.hits.iter().any(|(_, hit)| matches!(
                hit,
                super::super::Hit::Screen(ScreenHit::ThemeTab(true))
            )),
            "the Light tab is a click"
        );
        drop(before);
        app.on_hy_hit(ScreenHit::ThemeTab(true), false);
        assert!(app.theme_tab_light);
        assert!(
            matches!(app.mode, Mode::Themes { sel, .. } if matching("", true)[sel] == "vim"),
            "the Light tab opens on its own pick"
        );
        app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
        assert!(!app.theme_tab_light, "Shift+Tab goes back");
    }

    #[test]
    fn theme_browser_lists_themes_alphabetically_by_display_name() {
        let mut total = 0;
        for light in [false, true] {
            let labels: Vec<String> = matching("", light)
                .iter()
                .map(|name| theme_label(name).to_lowercase())
                .collect();
            total += labels.len();
            assert!(!labels.is_empty() && labels.is_sorted(), "{labels:?}");
        }
        assert_eq!(total, BUILTIN.len(), "every theme is on exactly one tab");
    }

    #[test]
    fn theme_browser_clicks_preview_and_wheel_moves_the_selection() {
        let mut app = app();
        let applied = app.cfg.theme.clone();
        app.open_themes_on(false);
        let selected = matching("", false)
            .iter()
            .position(|name| *name == "kanagawa-wave")
            .unwrap();
        app.on_hy_hit(ScreenHit::ThemePick(selected), false);
        assert!(matches!(app.mode, Mode::Themes { sel, saving: false, .. } if sel == selected));
        assert_eq!(app.cfg.theme, applied);
        let buffer = draw(&mut app, 100, 30);
        assert!(
            buffer
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("PREVIEW · Kanagawa Wave")
        );
        app.on_mouse(super::super::MouseEvent {
            kind: super::super::MouseEventKind::ScrollDown,
            column: 50,
            row: 10,
            modifiers: KeyModifiers::NONE,
        });
        assert!(matches!(app.mode, Mode::Themes { sel, .. } if sel == selected + 1));
        app.on_hy_hit(ScreenHit::Close, false);
        assert!(matches!(app.mode, Mode::ScreenSettings(_)));
        assert_eq!(app.cfg.theme, applied);
        assert_eq!(matching("frappe", false), vec!["catppuccin-frappe"]);
        assert_eq!(matching("rosé dawn", true), vec!["rose-pine-dawn"]);
    }
}
