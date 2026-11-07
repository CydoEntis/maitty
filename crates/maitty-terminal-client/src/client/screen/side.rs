//! The sidebar: one flat list of sessions in creation order, two lines each. The daemon's
//! panes are the sessions; the project and machine are badges on a row, never groups, and
//! a row never moves when its status changes (the order is the id, which only counts up).

use super::chips::{Chip, chip, right_chips};
use super::status_mark::State;
use super::*;
use crate::client::ids::ClientSession;

/// How long a new row's accent bar shows.
const FLASH: std::time::Duration = std::time::Duration::from_millis(500);
/// Rows a session takes.
const ROW_H: u16 = 2;
/// The glyph and the space after it.
const GLYPH_W: u16 = 2;
/// The `/ filter` row's text when nothing is typed.
const FILTER_HINT: &str = "/ filter";
/// The mark after a name for a pinned session.
const PIN: &str = "⌖";
/// The mark after a name for a session with output you have not looked at.
const UNREAD: &str = "•";
/// What a session stopped by a restart offers in place of its diff counts.
const RESUME: &str = "⏎ resume";
/// Placed before a remote machine's name, and before a reconnecting one's.
const REMOTE_MARK: &str = "⇄";
const RECONNECTING_MARK: &str = "┆";
const LOCAL: &str = "local";
/// Below this many rows tall the footer keeps only its buttons.
const FOOTER_FULL_FROM: u16 = 10;
/// A machine's name keeps at most this share of a row's second line.
const MACHINE_SHARE: usize = 2;

/// Where a session runs, as the second line says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Where {
    Local,
    Remote(String),
    /// Its machine's line is down; the row is the last report.
    Reconnecting(String),
}

impl Where {
    /// The text, with its mark.
    fn text(&self) -> String {
        match self {
            Where::Local => LOCAL.to_string(),
            Where::Remote(name) => format!("{REMOTE_MARK} {name}"),
            Where::Reconnecting(name) => format!("{RECONNECTING_MARK} {name}"),
        }
    }
}

/// A session as the sidebar lists it.
#[derive(Debug, Clone)]
pub(super) struct FlatRow {
    pub key: ClientSession,
    pub term: Option<ClientTerm>,
    /// Your name for it, else what runs in it.
    pub name: String,
    /// The last segment of its project's folder.
    pub project: String,
    pub place: Where,
    pub state: State,
    pub unread: bool,
    pub pinned: bool,
    pub added: u32,
    pub removed: u32,
    /// A restart stopped its agent: Enter resumes it.
    pub resume: bool,
    pub process: String,
    /// Creation order: ids only count up.
    order: (u64, MachineId),
}

impl FlatRow {
    fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        q.is_empty()
            || [&self.name, &self.project, &self.process, &self.place.text()]
                .iter()
                .any(|field| field.to_lowercase().contains(&q))
    }
}

/// What a session is called: your name for it, else what runs in it, else `fallback`.
pub(in crate::client) fn display_name(info: Option<&TermInfo>, fallback: &str) -> String {
    let custom = info.map_or("", |i| i.label.trim());
    let process = info.map_or("", |i| i.process.trim());
    if !custom.is_empty() {
        custom.to_string()
    } else if !process.is_empty() {
        process.to_string()
    } else {
        fallback.to_string()
    }
}

/// The last folder of a path, as the project's name.
fn last_segment(p: &Proj) -> String {
    p.name.clone()
}

/// Every session in creation order.
pub(super) fn flat_rows(app: &App, model: &[Proj]) -> Vec<FlatRow> {
    let mut rows: Vec<FlatRow> = model
        .iter()
        .flat_map(|p| p.sessions().map(move |s| (p, s)))
        .map(|(p, s)| {
            let info = s.native_term.and_then(|t| app.term_info(t));
            let process = info.map_or(String::new(), |i| i.process.clone());
            // A chat goes by its title, even once it has a terminal.
            let name = if matches!(s.key.key, SessionKey::Conversation(_)) {
                s.name.clone()
            } else {
                display_name(info, &s.name)
            };
            let failed = super::status_mark::failed(
                info.is_some_and(|i| i.exit.is_some()),
                s.status,
                s.check.as_ref(),
            );
            let place = if s.key.machine == MachineId::LOCAL {
                Where::Local
            } else if s.down {
                Where::Reconnecting(app.machine_label_of(s.key.machine))
            } else {
                Where::Remote(app.machine_label_of(s.key.machine))
            };
            FlatRow {
                key: s.key,
                term: s.native_term,
                name,
                project: last_segment(p),
                place,
                state: State::of(s.status, failed, s.down),
                unread: info.is_some_and(|i| i.bell),
                pinned: info.is_some_and(|i| i.pinned),
                added: info.map_or(0, |i| i.diff_added),
                removed: info.map_or(0, |i| i.diff_removed),
                resume: info.is_some_and(|i| i.interrupted.is_some()),
                process,
                order: (
                    s.native_term.map_or(u64::MAX, |t| u64::from(t.term)),
                    s.key.machine,
                ),
            }
        })
        .collect();
    rows.sort_by_key(|r| (r.order, r.key));
    rows
}

/// The rows the filter lets through.
pub(super) fn shown_rows(app: &App, model: &[Proj]) -> Vec<FlatRow> {
    let query = &app.screen.filter;
    flat_rows(app, model)
        .into_iter()
        .filter(|r| r.matches(query))
        .collect()
}

/// The sessions as the sidebar lists them, top to bottom.
pub(in crate::client) fn side_keys(app: &App, model: &[Proj]) -> Vec<ClientSession> {
    shown_rows(app, model).into_iter().map(|r| r.key).collect()
}

/// The sessions open in the workspace: the panes of the tab on screen, the focused one among
/// them.
fn open_terms(app: &App) -> Vec<ClientTerm> {
    let focus = app.focused();
    let mut open: Vec<ClientTerm> = app
        .screen
        .tabs
        .get(app.screen.tab)
        .filter(|tab| focus.is_some_and(|f| tab.contains(f)))
        .map(ScreenTab::leaves)
        .unwrap_or_default();
    if let Some(f) = focus
        && !open.contains(&f)
    {
        open.push(f);
    }
    open
}

/// `▲` at the list's top when rows are scrolled off above, `▼` at its bottom when more are
/// below, in column `x`.
fn scroll_marks(
    buf: &mut Buffer,
    x: u16,
    top: u16,
    (scroll, shown, len): (usize, usize, usize),
    rows_h: u16,
    st: Style,
) {
    if scroll > 0 {
        put(buf, x, top, &[seg("▲", st)], x + 1);
    }
    if scroll + shown < len {
        put(
            buf,
            x,
            top + rows_h.saturating_sub(1),
            &[seg("▼", st)],
            x + 1,
        );
    }
}

/// The first row shown: the cursor's (or the open session's) kept in view when it changes,
/// otherwise the wheel rules.
fn first_shown(app: &mut App, keys: &[ClientSession], shown: usize) -> usize {
    let mut scroll = app.screen.side_scroll as usize;
    if app.screen.follow {
        let want = app
            .screen
            .cursor
            .or(app.focused().map(|id| app.session_key(id)))
            .and_then(|k| keys.iter().position(|x| *x == k));
        if let Some(i) = want {
            if i < scroll {
                scroll = i;
            } else if i >= scroll + shown {
                scroll = i + 1 - shown;
            }
        }
    }
    scroll = scroll.min(keys.len().saturating_sub(shown));
    app.screen.side_scroll = scroll as u16;
    app.screen.follow = false;
    scroll
}

/// The sidebar: the frame with its chips, the filter, the rows and the footer.
pub(in crate::client) fn draw_side(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    model: &[Proj],
    t: &Theme,
) {
    let focused = side_focused(app);
    let mut c = Card::new(t, "");
    if focused {
        c = c.lit(t.accent);
    }
    card(app, buf, r, &c, t);
    app.screen.side_rect = r;
    hit(app, r, ScreenHit::SideFocus);
    if r.width < 12 || r.height < 5 {
        return;
    }
    let border = if focused { t.accent } else { t.line };

    // The top border: SESSIONS on the left; inbox and « on the right.
    let title = chip(app, "SESSIONS", Chip::Title { focused }, t.bg, t);
    let title_end = r.x + 2 + segs_width(&title);
    put(buf, r.x + 2, r.y, &title, title_end);
    let needing = model
        .iter()
        .flat_map(|p| p.sessions())
        .filter(|s| check::needs_you(s))
        .count();
    let inbox_n = super::inbox::inbox_count(app);
    let inbox = chip(
        app,
        &format!("inbox {inbox_n}"),
        if needing > 0 {
            Chip::Alert
        } else {
            Chip::Quiet
        },
        t.bg,
        t,
    );
    let collapse = chip(app, "«", Chip::Action, t.bg, t);
    right_chips(
        app,
        buf,
        r.y,
        (title_end + 1, r.right().saturating_sub(2)),
        vec![
            (inbox, Some(ScreenHit::OpenInbox)),
            (collapse, Some(ScreenHit::CollapseSidebar)),
        ],
        border,
    );

    // The filter row.
    let (x0, end) = (r.x + 2, r.right().saturating_sub(2));
    let filter_row = Rect {
        x: r.x + 1,
        y: r.y + 1,
        width: r.width - 2,
        height: 1,
    };
    let typed = &app.screen.filter;
    let filter_segs = if app.screen.filtering {
        vec![
            seg("/ ", Style::default().fg(t.accent).bg(t.bg)),
            seg(typed.clone(), Style::default().fg(t.strong).bg(t.bg)),
            seg("▏", Style::default().fg(t.accent).bg(t.bg)),
        ]
    } else if typed.is_empty() {
        vec![seg(FILTER_HINT, Style::default().fg(t.muted).bg(t.bg))]
    } else {
        vec![
            seg("/ ", Style::default().fg(t.muted).bg(t.bg)),
            seg(typed.clone(), Style::default().fg(t.strong).bg(t.bg)),
        ]
    };
    put(buf, x0, filter_row.y, &filter_segs, end);
    hit(app, filter_row, ScreenHit::SideFilter);

    // The footer: version or update, a rule, and the buttons.
    let full_foot = r.height >= FOOTER_FULL_FROM;
    let buttons_y = r.bottom().saturating_sub(2);
    draw_buttons(app, buf, r, buttons_y, t);
    let foot_rows = if full_foot { 3 } else { 1 };
    let list_bottom = r.bottom().saturating_sub(1 + foot_rows);
    if full_foot {
        draw_version(app, buf, r, buttons_y - 2, t);
    }

    // The rows.
    let rows = shown_rows(app, model);
    let keys: Vec<ClientSession> = rows.iter().map(|x| x.key).collect();
    app.screen.visible = keys.clone();
    app.screen.side_items = keys.iter().map(|k| SideItem::Sess(*k)).collect();
    app.screen.row_y.clear();
    let top = r.y + 2;
    let list_h = list_bottom.saturating_sub(top);
    let fits = (list_h / ROW_H) as usize;
    let scroll = first_shown(app, &keys, fits.max(1));
    let cursor = focused.then_some(app.screen.cursor).flatten();
    let open = open_terms(app);
    let current = app.focused();
    note_new_rows(app, &keys);
    if rows.is_empty() && !app.screen.filter.is_empty() {
        put(
            buf,
            x0,
            top,
            &[seg("no match", Style::default().fg(t.muted).bg(t.bg))],
            end,
        );
    }
    for (i, row) in rows.iter().enumerate().skip(scroll).take(fits) {
        let y = top + (i - scroll) as u16 * ROW_H;
        let picked = cursor == Some(row.key);
        let is_open = row.term.is_some_and(|t| open.contains(&t));
        let ground = if picked {
            t.sel_bg()
        } else if is_open {
            t.card
        } else {
            t.bg
        };
        let lit = Rect {
            x: r.x + 1,
            y,
            width: r.width - 2,
            height: ROW_H,
        };
        fill(buf, lit, ground);
        if app
            .screen
            .flash
            .get(&row.key)
            .is_some_and(|at| at.elapsed() < FLASH)
        {
            for dy in 0..ROW_H {
                put(
                    buf,
                    lit.x,
                    y + dy,
                    &[seg("▌", Style::default().fg(t.accent).bg(ground))],
                    lit.x + 1,
                );
            }
        }
        draw_row(app, buf, row, (x0, end, y), ground, t);
        if let Some(term) = row.term {
            app.screen.row_y.insert(term, y);
        }
        hit(app, lit, ScreenHit::Session(row.key));
        // The project on the second line filters to that project.
        let project_w = row.project.width() as u16;
        hit(
            app,
            Rect {
                x: x0 + GLYPH_W,
                y: y + 1,
                width: project_w.min(end.saturating_sub(x0 + GLYPH_W)),
                height: 1,
            },
            ScreenHit::SideProject(row.key),
        );
        let _ = current;
    }
    scroll_marks(
        buf,
        r.right().saturating_sub(2),
        top,
        (scroll, fits, rows.len()),
        fits as u16 * ROW_H,
        Style::default().bg(t.bg).fg(t.muted),
    );
}

/// Remember which rows exist; a row new since the last frame flashes its bar. The first
/// frame with sessions flashes nothing: they were not just made.
fn note_new_rows(app: &mut App, keys: &[ClientSession]) {
    let moving = app.motion_on();
    let known = app
        .screen
        .rows_known
        .get_or_insert_with(|| keys.iter().copied().collect());
    let fresh: Vec<ClientSession> = keys.iter().filter(|k| known.insert(**k)).copied().collect();
    if moving {
        for k in fresh {
            app.screen.flash.insert(k, Instant::now());
        }
    }
    app.screen.flash.retain(|_, at| at.elapsed() < FLASH);
}

/// One session's two lines.
fn draw_row(
    app: &App,
    buf: &mut Buffer,
    row: &FlatRow,
    (x0, end, y): (u16, u16, u16),
    ground: Color,
    t: &Theme,
) {
    let unknown = row.state == State::Unknown;
    let mark = app.state_mark(row.state, t);
    let dimmed = |c: Color| if unknown { t.idle } else { c };
    let base = Style::default().bg(ground);
    let mark_style = base
        .fg(mark.color)
        .add_modifier(if row.state == State::Working {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    put(buf, x0, y, &[seg(mark.glyph, mark_style)], x0 + 1);

    // Line 1: the name, its marks, and the diff counts or the resume offer.
    let nx = x0 + GLYPH_W;
    let right: Vec<Seg> = if row.resume {
        vec![seg(RESUME, base.fg(dimmed(t.accent)))]
    } else if row.added > 0 || row.removed > 0 {
        vec![
            seg(format!("+{}", row.added), base.fg(dimmed(t.done))),
            seg(" ", base),
            seg(format!("−{}", row.removed), base.fg(dimmed(t.err))),
        ]
    } else {
        Vec::new()
    };
    let rw = segs_width(&right);
    let marks_w = u16::from(row.unread) * 2 + u16::from(row.pinned) * 2;
    let gap = u16::from(rw > 0);
    let name_room = end.saturating_sub(nx).saturating_sub(rw + gap + marks_w) as usize;
    let name = truncate(&row.name, name_room.max(1));
    let name_color = if row.state == State::Idle {
        t.muted
    } else {
        t.strong
    };
    let name_style = base.fg(dimmed(name_color)).add_modifier(Modifier::BOLD);
    let after = put(buf, nx, y, &[seg(name, name_style)], end);
    let mut x = after;
    if row.unread {
        x = put(
            buf,
            x,
            y,
            &[seg(format!(" {UNREAD}"), base.fg(dimmed(t.accent)))],
            end,
        );
    }
    if row.pinned {
        put(
            buf,
            x,
            y,
            &[seg(format!(" {PIN}"), base.fg(dimmed(t.muted)))],
            end,
        );
    }
    if rw > 0 {
        put(buf, end.saturating_sub(rw), y, &right, end);
    }

    // Line 2: the project, and where it runs at the right.
    let y2 = y + 1;
    let machine = row.place.text();
    let machine_w = machine
        .width()
        .min((end.saturating_sub(nx) as usize) / MACHINE_SHARE) as u16;
    let machine_text = truncate(&machine, machine_w as usize);
    let machine_style = match row.place {
        Where::Reconnecting(_) => base.fg(t.idle),
        _ => base.fg(dimmed(t.muted)),
    };
    let project_room = end.saturating_sub(nx).saturating_sub(machine_w + 1) as usize;
    let project = truncate(&row.project, project_room.max(1));
    put(buf, nx, y2, &[seg(project, base.fg(dimmed(t.muted)))], end);
    put(
        buf,
        end.saturating_sub(machine_w),
        y2,
        &[seg(machine_text, machine_style)],
        end,
    );
}

/// The footer's version line: the update marker when one is on offer, else the version, dim.
fn draw_version(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, t: &Theme) {
    let (x, end) = (r.x + 2, r.right().saturating_sub(2));
    let room = end.saturating_sub(x);
    let marker = [mode_bar::Release::Words, mode_bar::Release::Version]
        .into_iter()
        .find_map(|shown| {
            let (segs, h) = mode_bar::release_segs(app, shown, t)?;
            (segs_width(&segs) <= room).then_some((segs, h))
        });
    app.screen.release_in_side = marker.is_some();
    match marker {
        Some((segs, h)) => {
            let w = segs_width(&segs);
            put(buf, x, y, &segs, end);
            hit(
                app,
                Rect {
                    x,
                    y,
                    width: w,
                    height: 1,
                },
                h,
            );
        }
        None => {
            put(
                buf,
                x,
                y,
                &[seg(
                    format!("{} {}", crate::brand::APP_NAME, crate::brand::APP_VERSION),
                    Style::default().fg(t.idle).bg(t.bg),
                )],
                end,
            );
        }
    }
}

/// The rule over the buttons, and ` + New ` and ` Menu `.
fn draw_buttons(app: &mut App, buf: &mut Buffer, r: Rect, y: u16, t: &Theme) {
    let (x, end) = (r.x + 2, r.right().saturating_sub(2));
    if r.height >= 6 {
        for xx in r.x + 1..r.right() - 1 {
            put(
                buf,
                xx,
                y - 1,
                &[seg("─", Style::default().fg(t.line).bg(t.bg))],
                xx + 1,
            );
        }
    }
    let new = chip(app, "+ New", Chip::Action, t.bg, t);
    let menu = chip(app, "Menu", Chip::Quiet, t.bg, t);
    let (nw, mw) = (segs_width(&new), segs_width(&menu));
    if x + nw > end {
        return;
    }
    put(buf, x, y, &new, end);
    hit(
        app,
        Rect {
            x,
            y,
            width: nw,
            height: 1,
        },
        ScreenHit::NewShell,
    );
    let mx = end.saturating_sub(mw);
    if mx > x + nw {
        put(buf, mx, y, &menu, end);
        hit(
            app,
            Rect {
                x: mx,
                y,
                width: mw,
                height: 1,
            },
            ScreenHit::Actions,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_matches_name_project_process_and_machine() {
        let row = FlatRow {
            key: ClientSession::local(SessionKey::Pane(1)),
            term: None,
            name: "Growth Tests".into(),
            project: "AeVoxGrowth".into(),
            place: Where::Remote("devbox".into()),
            state: State::Idle,
            unread: false,
            pinned: false,
            added: 0,
            removed: 0,
            resume: false,
            process: "claude".into(),
            order: (1, MachineId::LOCAL),
        };
        for q in ["", "growth t", "AEVOX", "cla", "devbox", "  devbox "] {
            assert!(row.matches(q), "{q:?}");
        }
        assert!(!row.matches("pacman"));
    }

    #[test]
    fn where_reads_as_the_design_spells_it() {
        assert_eq!(Where::Local.text(), "local");
        assert_eq!(Where::Remote("devbox".into()).text(), "⇄ devbox");
        assert_eq!(Where::Reconnecting("gpu-01".into()).text(), "┆ gpu-01");
    }
}
