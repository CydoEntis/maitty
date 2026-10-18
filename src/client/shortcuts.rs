//! All configured bindings and the contextual reference, shared with the user docs.

use super::design::{put, seg};
use super::*;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
};

const QUERY_MAX: usize = 100;
const PAGE: usize = 10;
const CONTEXTS: &str = include_str!("../../docs/shortcuts.md");

pub(super) fn append_query(query: &mut String, text: &str) {
    let room = QUERY_MAX.saturating_sub(query.chars().count());
    query.extend(text.chars().take(room));
}

/// Preserve aliases and custom bindings; the compact leader overview deliberately groups them.
pub(super) fn entries(app: &App) -> Vec<(String, String, String)> {
    let mut rows = vec![(
        "Start here".into(),
        app.keymap.prefix.to_string(),
        "Leader: press, release, then a binding below".into(),
    )];
    for key in &app.keymap.prefixed_order {
        if let Some(action) = app.keymap.prefixed.get(key).filter(|a| **a != Action::None) {
            rows.push(("After leader".into(), key.to_string(), action.describe()));
        }
    }
    let mut global: Vec<_> = app
        .keymap
        .global
        .iter()
        .filter(|(_, a)| **a != Action::None)
        .collect();
    global.sort_by_key(|(key, _)| key.to_config());
    rows.extend(
        global
            .into_iter()
            .map(|(key, action)| ("Without leader".into(), key.to_string(), action.describe())),
    );
    for action in app.palette_commands() {
        if !app
            .keymap
            .prefixed
            .values()
            .chain(app.keymap.global.values())
            .any(|bound| *bound == action)
        {
            rows.push((
                "Command palette (no binding)".into(),
                "Search by name".into(),
                action.describe(),
            ));
        }
    }
    let mut scope = "";
    for line in CONTEXTS.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            scope = heading;
        }
        if let Some(row) = line.strip_prefix("| ")
            && let Some((keys, action)) = row.split_once(" | ")
            && keys != "Keys"
            && !keys.starts_with('-')
        {
            rows.push((
                scope.into(),
                keys.replace('`', ""),
                action.trim_end_matches(" |").replace('`', ""),
            ));
        }
    }
    rows
}

fn search_text(text: &str) -> String {
    text.to_lowercase()
        .replace("ctrl+", "c-")
        .replace("alt+", "m-")
        .replace("shift+", "s-")
        .replace("pgup", "pageup")
        .replace("pgdn", "pagedown")
}

fn matching_entries(app: &App, query: &str) -> Vec<(String, String, String)> {
    let words: Vec<_> = search_text(query)
        .split_whitespace()
        .map(str::to_string)
        .collect();
    entries(app)
        .into_iter()
        .filter(|(scope, keys, action)| {
            let hay = search_text(&format!("{scope} {keys} {action}"));
            words.iter().all(|word| hay.contains(word))
        })
        .collect()
}

fn key_parts(keys: &str) -> Vec<String> {
    keys.split([',', '/'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn entry_lines(
    width: usize,
    keys: &str,
    action: &str,
    theme: &Theme,
) -> Vec<Vec<super::design::Seg>> {
    let style = Style::default().bg(theme.card);
    let mut key_runs = Vec::new();
    let parts = key_parts(keys);
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            key_runs.push(seg("/", style.fg(theme.muted)));
        }
        key_runs.extend(super::design::keycap(theme, part));
    }
    if parts.is_empty() {
        key_runs.push(seg(keys, style.fg(theme.muted)));
    }

    let key_width = super::design::segs_width(&key_runs) as usize + 2;
    let action_style = style.fg(theme.text);
    if key_width > 28 || key_width >= width.saturating_sub(8) {
        let mut lines = vec![key_runs];
        lines.extend(
            super::views::wrap(action, width.saturating_sub(2).max(1))
                .into_iter()
                .map(|line| vec![seg("  ", style), seg(line, action_style)]),
        );
        return lines;
    }

    let wrapped = super::views::wrap(action, width.saturating_sub(key_width).max(1));
    let mut lines = Vec::with_capacity(wrapped.len().max(1));
    if let Some(first) = wrapped.first() {
        let mut line = key_runs;
        line.push(seg("  ", style));
        line.push(seg(first.clone(), action_style));
        lines.push(line);
    }
    lines.extend(
        wrapped
            .into_iter()
            .skip(1)
            .map(|text| vec![seg(" ".repeat(key_width), style), seg(text, action_style)]),
    );
    lines
}

fn lines(app: &App, query: &str, width: usize, theme: &Theme) -> Vec<Vec<super::design::Seg>> {
    let matches = matching_entries(app, query);
    let mut out = Vec::new();
    let mut previous = String::new();
    for (scope, keys, action) in &matches {
        if *scope != previous {
            if !previous.is_empty() {
                out.push(Vec::new());
            }
            out.push(vec![seg(
                scope.to_uppercase(),
                Style::default()
                    .bg(theme.card)
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]);
            previous = scope.clone();
        }
        out.extend(entry_lines(width.saturating_sub(1), keys, action, theme));
    }
    if matches.is_empty() {
        return vec![vec![seg(
            "No shortcuts match. Ctrl+U clears the search.",
            Style::default().bg(theme.card).fg(theme.muted),
        )]];
    }
    out
}

impl App {
    pub(super) fn open_shortcuts(&mut self) {
        self.mode = Mode::Shortcuts {
            query: String::new(),
            scroll: 0,
        };
    }

    pub(super) fn on_shortcuts_key(
        &mut self,
        mut query: String,
        mut scroll: usize,
        key: &KeyEvent,
    ) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Up => scroll = scroll.saturating_sub(1),
            KeyCode::Down => scroll = scroll.saturating_add(1),
            KeyCode::PageUp => scroll = scroll.saturating_sub(PAGE),
            KeyCode::PageDown => scroll = scroll.saturating_add(PAGE),
            KeyCode::Home => scroll = 0,
            KeyCode::End => scroll = usize::MAX,
            KeyCode::Backspace => {
                query.pop();
                scroll = 0;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                query.clear();
                scroll = 0;
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                append_query(&mut query, &c.to_string());
                scroll = 0;
            }
            _ => {}
        }
        self.mode = Mode::Shortcuts { query, scroll };
    }
}

pub(super) fn draw_shortcuts(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    query: &str,
    scroll: usize,
) {
    let buf = frame.buffer_mut();
    hydra::dim_all(buf, area, theme);
    let size = hydra::tool_rect(area);
    let r = hydra::panel(
        app,
        buf,
        area,
        size.width.min(110),
        size.height,
        "All shortcuts",
        &[],
        theme,
    );
    let style = Style::default().bg(theme.card);
    put(
        buf,
        r.x + 2,
        r.y + 2,
        &[seg(format!("Find: {query}▏"), style.fg(theme.strong))],
        r.right().saturating_sub(2),
    );
    let body = Rect {
        x: r.x + 2,
        y: r.y + 4,
        width: r.width.saturating_sub(4),
        height: r.height.saturating_sub(7),
    };
    let text = lines(app, query, body.width.max(1) as usize, theme);
    let scroll = scroll.min(text.len().saturating_sub(body.height as usize));
    if let Mode::Shortcuts { scroll: saved, .. } = &mut app.mode {
        *saved = scroll;
    }
    for (row, line) in text
        .iter()
        .skip(scroll)
        .take(body.height as usize)
        .enumerate()
    {
        put(buf, body.x, body.y + row as u16, line, body.right());
    }
    let count = format!(
        "{}–{} / {} lines · C- = Ctrl, M- = Alt",
        scroll + 1,
        (scroll + body.height as usize).min(text.len()),
        text.len()
    );
    put(
        buf,
        r.x + 2,
        r.bottom().saturating_sub(3),
        &[seg(
            count,
            style.fg(theme.muted).add_modifier(Modifier::ITALIC),
        )],
        r.right().saturating_sub(2),
    );
    put(
        buf,
        r.x + 2,
        r.bottom().saturating_sub(2),
        &hydra::hints(
            theme,
            &[
                ("↑↓/PgUp/Dn", "scroll"),
                ("Ctrl+U", "clear"),
                ("Esc", "close"),
            ],
        ),
        r.right().saturating_sub(2),
    );
}
