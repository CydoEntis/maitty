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
const SHORTCUT_COLUMNS: usize = 3;
const THREE_COLUMN_MIN_WIDTH: usize = 78;
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

fn wrapped_keycaps(keys: &str, width: usize, theme: &Theme) -> Vec<Vec<super::design::Seg>> {
    let style = Style::default().bg(theme.card);
    let parts = key_parts(keys);
    if parts.is_empty() {
        return vec![vec![seg(keys, style.fg(theme.muted))]];
    }

    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0;
    for part in parts {
        let cap = super::design::keycap(theme, &part);
        let cap_width = super::design::segs_width(&cap) as usize;
        let separator_width = usize::from(!current.is_empty()) * 3;
        if !current.is_empty() && current_width + separator_width + cap_width > width {
            lines.push(current);
            current = Vec::new();
            current_width = 0;
        }
        if !current.is_empty() {
            current.push(seg(" / ", style.fg(theme.muted)));
            current_width += 3;
        }
        current.extend(cap);
        current_width += cap_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
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
        let mut lines = wrapped_keycaps(keys, width.saturating_sub(2).max(1), theme);
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

fn sections(app: &App, query: &str) -> Vec<(String, Vec<(String, String)>)> {
    let matches = matching_entries(app, query);
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for (scope, keys, action) in &matches {
        if out.last().is_none_or(|(current, _)| current != scope) {
            out.push((scope.clone(), Vec::new()));
        }
        if let Some((_, entries)) = out.last_mut() {
            entries.push((keys.clone(), action.clone()));
        }
    }
    out
}

fn section_lines(
    title: &str,
    entries: &[(String, String)],
    width: usize,
    theme: &Theme,
) -> Vec<Vec<super::design::Seg>> {
    let mut out = vec![vec![seg(
        title.to_uppercase(),
        Style::default()
            .bg(theme.card)
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )]];
    for (keys, action) in entries {
        out.extend(entry_lines(width.saturating_sub(1), keys, action, theme));
    }
    out.push(Vec::new());
    out
}

fn single_column_lines(
    sections: &[(String, Vec<(String, String)>)],
    width: usize,
    theme: &Theme,
) -> Vec<Vec<super::design::Seg>> {
    let mut out = Vec::new();
    for (title, entries) in sections {
        out.extend(section_lines(title, entries, width, theme));
    }
    if out.last().is_some_and(Vec::is_empty) {
        out.pop();
    }
    out
}

fn three_column_lines(
    sections: &[(String, Vec<(String, String)>)],
    width: usize,
    theme: &Theme,
) -> Vec<Vec<Vec<super::design::Seg>>> {
    let gutter = 3;
    let column_width = width.saturating_sub(gutter * (SHORTCUT_COLUMNS - 1)) / SHORTCUT_COLUMNS;
    let mut columns = vec![Vec::new(); SHORTCUT_COLUMNS];
    for (title, entries) in sections {
        let column = columns
            .iter()
            .enumerate()
            .min_by_key(|(_, lines)| lines.len())
            .map(|(index, _)| index)
            .unwrap_or(0);
        columns[column].extend(section_lines(title, entries, column_width, theme));
    }
    columns
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
    let width = body.width.max(1) as usize;
    let sections = sections(app, query);
    let total_lines;
    let scroll = if sections.is_empty() {
        total_lines = 1;
        put(
            buf,
            body.x,
            body.y,
            &[seg(
                "No shortcuts match. Ctrl+U clears the search.",
                style.fg(theme.muted),
            )],
            body.right(),
        );
        0
    } else if width >= THREE_COLUMN_MIN_WIDTH {
        let columns = three_column_lines(&sections, width, theme);
        total_lines = columns.iter().map(Vec::len).max().unwrap_or(0);
        let scroll = scroll.min(total_lines.saturating_sub(body.height as usize));
        let gutter = 3;
        let column_width = width.saturating_sub(gutter * (SHORTCUT_COLUMNS - 1)) / SHORTCUT_COLUMNS;
        for (index, column) in columns.iter().enumerate() {
            let column_scroll = scroll.min(column.len().saturating_sub(body.height as usize));
            let x = body.x + (index * (column_width + gutter)) as u16;
            for (row, line) in column
                .iter()
                .skip(column_scroll)
                .take(body.height as usize)
                .enumerate()
            {
                put(buf, x, body.y + row as u16, line, x + column_width as u16);
            }
            if index > 0 {
                let divider_x = x.saturating_sub(2);
                for y in body.y..body.bottom() {
                    put(
                        buf,
                        divider_x,
                        y,
                        &[seg("│", Style::default().fg(theme.muted).bg(theme.card))],
                        divider_x + 1,
                    );
                }
            }
        }
        scroll
    } else {
        let text = single_column_lines(&sections, width, theme);
        total_lines = text.len();
        let scroll = scroll.min(text.len().saturating_sub(body.height as usize));
        for (row, line) in text
            .iter()
            .skip(scroll)
            .take(body.height as usize)
            .enumerate()
        {
            put(buf, body.x, body.y + row as u16, line, body.right());
        }
        scroll
    };
    if let Mode::Shortcuts { scroll: saved, .. } = &mut app.mode {
        *saved = scroll;
    }
    let count = format!(
        "{}–{} / {} lines · C- = Ctrl, M- = Alt",
        scroll + 1,
        (scroll + body.height as usize).min(total_lines),
        total_lines
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
