//! The tray (Windows) or menu-bar (macOS) item: the sessions that need you or finished, one click
//! from anywhere. The window says which sessions those are, by the names its sidebar shows; this
//! only lists them in the system's own menu and tells the window which one was chosen. It decides
//! no status and sends nothing to a session.

use serde::Deserialize;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const APP: &str = "Maitty";
const TRAY_ID: &str = "main";
const MAIN_WINDOW: &str = "main";
/// The window is told the chosen session by this event; its payload is the session's id.
pub const OPEN_SESSION_EVENT: &str = "tray-open-session";
const SHOW_ID: &str = "show";
const QUIT_ID: &str = "quit";
const SESSION_PREFIX: &str = "session:";
/// A menu is a glance, not a list to scroll: more sessions than this are counted, not listed.
const MOST_LISTED: usize = 10;
const LONGEST_LABEL: usize = 60;
const LONGEST_ID: usize = 64;

/// A session as the window names it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TraySession {
    id: String,
    label: String,
}

/// One line of the menu.
#[derive(Debug, PartialEq)]
enum Row {
    /// A line that says something and does nothing.
    Says(String),
    Session { id: String, label: String },
    Separator,
    Show,
    Quit,
}

/// A session's id as the window makes them (`t4`, `12`, `box.lan/t7`); anything else is not listed.
fn is_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= LONGEST_ID
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
}

/// A name fit for a menu line: one line, no control characters, not too long.
fn menu_label(label: &str) -> String {
    let clean: String = label
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= LONGEST_LABEL {
        return clean;
    }
    let mut cut: String = clean.chars().take(LONGEST_LABEL - 1).collect();
    cut.push('…');
    cut
}

fn listed(sessions: &[TraySession]) -> impl Iterator<Item = Row> + '_ {
    sessions
        .iter()
        .filter(|session| is_session_id(&session.id))
        .take(MOST_LISTED)
        .map(|session| Row::Session {
            id: session.id.clone(),
            label: menu_label(&session.label),
        })
}

fn group(rows: &mut Vec<Row>, title: &str, sessions: &[TraySession]) {
    if sessions.is_empty() {
        return;
    }
    rows.push(Row::Says(format!("{title} ({})", sessions.len())));
    rows.extend(listed(sessions));
    if sessions.len() > MOST_LISTED {
        rows.push(Row::Says(format!("and {} more", sessions.len() - MOST_LISTED)));
    }
    rows.push(Row::Separator);
}

/// The menu, top to bottom.
fn rows(needs: &[TraySession], finished: &[TraySession]) -> Vec<Row> {
    let mut rows = Vec::new();
    group(&mut rows, "Needs you", needs);
    group(&mut rows, "Finished", finished);
    if rows.is_empty() {
        rows.push(Row::Says("Nothing needs you".into()));
        rows.push(Row::Separator);
    }
    rows.push(Row::Show);
    rows.push(Row::Quit);
    rows
}

/// What hovering the item says.
fn tooltip(needs: usize) -> String {
    match needs {
        0 => APP.into(),
        1 => format!("{APP}: 1 session needs you"),
        n => format!("{APP}: {n} sessions need you"),
    }
}

fn menu(app: &AppHandle, rows: &[Row]) -> tauri::Result<Menu<tauri::Wry>> {
    let mut builder = MenuBuilder::new(app);
    for (at, row) in rows.iter().enumerate() {
        builder = match row {
            Row::Says(text) => builder.item(
                &MenuItemBuilder::with_id(format!("says:{at}"), text)
                    .enabled(false)
                    .build(app)?,
            ),
            Row::Session { id, label } => builder.item(
                &MenuItemBuilder::with_id(format!("{SESSION_PREFIX}{id}"), label).build(app)?,
            ),
            Row::Separator => builder.separator(),
            Row::Show => builder.item(&MenuItemBuilder::with_id(SHOW_ID, format!("Show {APP}")).build(app)?),
            Row::Quit => builder.item(&MenuItemBuilder::with_id(QUIT_ID, format!("Quit {APP}")).build(app)?),
        };
    }
    builder.build()
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        // Each is best effort: a window that is already up refuses none of them in a way worth telling.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Puts the item in the tray, empty until the window says what to list.
pub fn start(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tooltip(0))
        .menu(&menu(app, &rows(&[], &[]))?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            SHOW_ID => show_window(app),
            QUIT_ID => app.exit(0),
            id => {
                if let Some(session) = id.strip_prefix(SESSION_PREFIX) {
                    show_window(app);
                    if let Err(error) = app.emit(OPEN_SESSION_EVENT, session) {
                        eprintln!("telling the window which session the tray chose: {error}");
                    }
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// The window's list of the sessions that need you and the ones that finished, newest state.
#[tauri::command]
pub fn set_tray_sessions(
    app: AppHandle,
    needs: Vec<TraySession>,
    finished: Vec<TraySession>,
) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let menu = menu(&app, &rows(&needs, &finished)).map_err(|error| error.to_string())?;
    tray.set_menu(Some(menu)).map_err(|error| error.to_string())?;
    let asking = needs.iter().filter(|session| is_session_id(&session.id)).count();
    tray.set_tooltip(Some(tooltip(asking)))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, label: &str) -> TraySession {
        TraySession {
            id: id.into(),
            label: label.into(),
        }
    }

    #[test]
    fn with_nothing_to_say_the_menu_says_so_above_show_and_quit() {
        assert_eq!(
            rows(&[], &[]),
            [
                Row::Says("Nothing needs you".into()),
                Row::Separator,
                Row::Show,
                Row::Quit
            ]
        );
        assert!(!tooltip(0).contains("need"));
    }

    #[test]
    fn the_sessions_that_need_you_come_first_then_the_finished_ones() {
        let menu = rows(&[session("t4", "api")], &[session("box.lan/t7", "web")]);
        assert_eq!(
            menu,
            [
                Row::Says("Needs you (1)".into()),
                Row::Session { id: "t4".into(), label: "api".into() },
                Row::Separator,
                Row::Says("Finished (1)".into()),
                Row::Session { id: "box.lan/t7".into(), label: "web".into() },
                Row::Separator,
                Row::Show,
                Row::Quit,
            ]
        );
        assert!(tooltip(1).ends_with("1 session needs you"));
        assert!(tooltip(3).ends_with("3 sessions need you"));
    }

    #[test]
    fn a_long_list_is_cut_and_the_rest_counted() {
        let many: Vec<TraySession> = (0..MOST_LISTED + 3)
            .map(|n| session(&format!("t{n}"), "x"))
            .collect();
        let menu = rows(&many, &[]);
        let listed = menu.iter().filter(|row| matches!(row, Row::Session { .. })).count();
        assert_eq!(listed, MOST_LISTED);
        assert!(menu.contains(&Row::Says("and 3 more".into())));
    }

    #[test]
    fn a_name_is_made_one_short_line_and_an_id_that_is_not_one_is_left_out() {
        assert_eq!(menu_label("fix\tthe\r\nlogin \u{1b}[31m page"), "fix the login [31m page");
        let long = menu_label(&"a".repeat(200));
        assert_eq!(long.chars().count(), LONGEST_LABEL);
        assert!(long.ends_with('…'));
        let menu = rows(&[session("t1; rm", "bad"), session("", "empty"), session("t2", "good")], &[]);
        let ids: Vec<&str> = menu
            .iter()
            .filter_map(|row| match row {
                Row::Session { id, .. } => Some(id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(ids, ["t2"]);
    }
}
