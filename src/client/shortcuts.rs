//! All configured bindings and the contextual reference, shared with the user docs.

use super::design::{keycaps, put, seg};
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

enum ShortcutLine {
    Text(String),
    Entry { keys: String, action: String },
}

fn lines(app: &App, query: &str, width: usize) -> Vec<ShortcutLine> {
    let words: Vec<_> = search_text(query)
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let mut out = Vec::new();
    let mut previous = String::new();
    for (scope, keys, action) in entries(app) {
        let hay = search_text(&format!("{scope} {keys} {action}"));
        if !words.iter().all(|word| hay.contains(word)) {
            continue;
        }
        if scope != previous {
            if !out.is_empty() {
                out.push(ShortcutLine::Text(String::new()));
            }
            out.push(ShortcutLine::Text(scope.to_uppercase()));
            previous = scope;
        }
        if keys == "Search by name" {
            out.push(ShortcutLine::Text(format!("{keys}  {action}")));
            continue;
        }
        let key_width = keys.chars().count().saturating_add(2);
        let action_width = width.saturating_sub(key_width.saturating_add(2)).max(8);
        let mut wrapped = super::views::wrap(&action, action_width);
        if let Some(first) = wrapped.first().cloned() {
            out.push(ShortcutLine::Entry { keys, action: first });
            let indent = " ".repeat(key_width + 2);
            out.extend(wrapped.drain(1..).map(|line| ShortcutLine::Text(format!("{indent}{line}"))));
        }
    }
    if out.is_empty() {
        out.push(ShortcutLine::Text("No shortcuts match. Ctrl+U clears the search.".into()));
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
    let rows = lines(app, query, body.width.max(1) as usize);
    let scroll = scroll.min(rows.len().saturating_sub(body.height as usize));
    if let Mode::Shortcuts { scroll: saved, .. } = &mut app.mode {
        *saved = scroll;
    }
    for (i, line) in rows
        .iter()
        .skip(scroll)
        .take(body.height as usize)
        .enumerate()
    {
        let y = body.y + i as u16;
        let segs = match line {
            ShortcutLine::Text(text) => vec![seg(text, style.fg(theme.text))],
            ShortcutLine::Entry { keys, action } => {
                let mut segs = keycaps(theme, keys, theme.card);
                segs.push(seg(format!("  {action}"), style.fg(theme.text)));
                segs
            }
        };
        put(
            buf,
            body.x,
            y,
            &segs,
            body.right(),
        );
    }
    let count = format!(
        "{}–{} / {} lines · C- = Ctrl, M- = Alt",
        scroll + 1,
        (scroll + body.height as usize).min(rows.len()),
        rows.len()
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
