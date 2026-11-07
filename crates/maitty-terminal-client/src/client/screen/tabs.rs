//! The tab row: the top border of the workspace. One tab per ship of the fleet you are in, in
//! creation order, then ` + ` for a new shell, and ` Files ` and ` Changes ` at the right.

use super::chips::{Chip, chip};
use super::status_mark::State;
use super::workspace::{pane_state, pane_title};
use super::*;

/// The most a tab's title takes before it is cut short.
const TITLE_MAX: usize = 24;
/// Columns a tab takes besides its title: the cell either side, the status glyph, the gaps
/// and the `×`.
const TAB_CHROME: u16 = 6;
/// The overflow chip's text is ` …N `; this much room is kept for it when tabs are hidden.
const OVERFLOW_W: u16 = 6;
const CLOSE: &str = "×";

/// A tab: one ship of the current fleet.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Tab {
    pub ship: ClientTerm,
    pub title: String,
    pub state: State,
    /// How many panes its view holds.
    pub panes: usize,
    pub active: bool,
}

impl App {
    /// The tabs of the fleet you are in. A pane that sits in another ship's split is part of
    /// that ship's tab, not a tab of its own.
    pub(super) fn fleet_tabs(&self, model: &[Proj]) -> Vec<Tab> {
        let Some(fleet) = self.current_fleet() else {
            return Vec::new();
        };
        let ships = self
            .fleet_groups()
            .into_iter()
            .flat_map(|g| g.fleets)
            .find(|f| f.id == fleet)
            .map(|f| f.ships)
            .unwrap_or_default();
        let members: HashSet<ClientTerm> = self
            .screen
            .tabs
            .iter()
            .filter(|tab| tab.layout.leaves().len() > 1)
            .flat_map(|tab| tab.leaves().into_iter().filter(|t| *t != tab.owner))
            .collect();
        let on_screen = self.focused().map(|f| {
            self.screen
                .tabs
                .iter()
                .find(|tab| tab.contains(f))
                .map_or(f, |tab| tab.owner)
        });
        ships
            .into_iter()
            .filter(|t| !members.contains(t))
            .filter_map(|ship| {
                let info = self.term_info(ship)?;
                Some(Tab {
                    ship,
                    title: pane_title(self, model, ship),
                    state: pane_state(self, info, ship),
                    panes: self.pane_count(ship),
                    active: on_screen == Some(ship),
                })
            })
            .collect()
    }

    /// The panes in a ship's view: the tab it owns that you were last on, else one.
    pub(super) fn pane_count(&self, ship: ClientTerm) -> usize {
        self.screen
            .tabs
            .iter()
            .filter(|tab| tab.owner == ship)
            .max_by_key(|tab| tab.used)
            .map_or(1, |tab| tab.layout.leaves().len())
    }

    /// Every pane a ship's tab shows, which closing the tab ends.
    pub(super) fn tab_panes(&self, ship: ClientTerm) -> Vec<ClientTerm> {
        self.screen
            .tabs
            .iter()
            .filter(|tab| tab.owner == ship)
            .max_by_key(|tab| tab.used)
            .map_or_else(|| vec![ship], ScreenTab::leaves)
    }

    /// Go to the next or previous tab of the fleet, round the ends.
    pub(super) fn step_tab(&mut self, forward: bool) {
        let model = self.screen_model();
        let tabs = self.fleet_tabs(&model);
        let n = tabs.len();
        if n < 2 {
            return;
        }
        let at = tabs.iter().position(|t| t.active).unwrap_or(0);
        let to = if forward { (at + 1) % n } else { (at + n - 1) % n };
        self.screen_focus(tabs[to].ship);
    }

    /// Go to the `n`th tab of the fleet (counting from 1).
    pub(super) fn select_tab(&mut self, n: usize) {
        let model = self.screen_model();
        if let Some(tab) = n.checked_sub(1).and_then(|i| self.fleet_tabs(&model).into_iter().nth(i)) {
            self.screen_focus(tab.ship);
        }
    }
}

/// One tab as segments: ` ● title 3 × `. The active tab of a focused workspace is the accent.
fn tab_segs(app: &App, tab: &Tab, title: &str, focused: bool, t: &Theme) -> Vec<Seg> {
    let mark = app.state_mark(tab.state, t);
    let (ground, ink) = if tab.active && focused {
        (t.accent, t.acc_ink)
    } else if tab.active {
        (t.btn, t.strong)
    } else {
        (t.btn, t.text)
    };
    // A session that cannot be reached is shown dimmed, its mark `┆`.
    let ink = if tab.state == State::Unknown && !(tab.active && focused) {
        t.idle
    } else {
        ink
    };
    // A glyph in the accent colour would vanish on the accent tab.
    let glyph_ink = if tab.active && focused && mark.color == t.accent {
        ink
    } else {
        mark.color
    };
    let base = Style::default().bg(ground);
    let mut segs = vec![
        seg(" ", base),
        seg(mark.glyph, base.fg(glyph_ink)),
        seg(" ", base),
        seg(title.to_string(), base.fg(ink).add_modifier(Modifier::BOLD)),
    ];
    if tab.panes >= 2 {
        segs.push(seg(format!(" {}", tab.panes), base.fg(ink)));
    }
    segs.push(seg(" ", base));
    segs.push(seg(CLOSE, base.fg(ink)));
    segs.push(seg(" ", base));
    segs
}

/// Draw the tab row on the top border `row` of the workspace `area`.
pub(super) fn draw_tab_row(
    app: &mut App,
    buf: &mut Buffer,
    area: Rect,
    model: &[Proj],
    focused: bool,
    t: &Theme,
) {
    if area.width < 16 {
        return;
    }
    // A panel open over the workspace's right side keeps the tool chips beside it, lit.
    let sheet = app.screen.sheet_rect;
    let edge = if sheet.width > 0 && sheet.x > area.x {
        sheet.x.min(area.right())
    } else {
        area.right()
    };
    let (left, right_end) = (area.x + 2, edge.saturating_sub(2));
    let files_open = open_sheet(app) == Some(SheetKind::Files);
    let changes_open = open_sheet(app) == Some(SheetKind::Changes);
    let tool = |app: &App, label: &str, open: bool| {
        chip(
            app,
            label,
            if open {
                Chip::Title { focused: true }
            } else {
                Chip::Quiet
            },
            t.bg,
            t,
        )
    };
    let files = tool(app, "Files", files_open);
    let changes = tool(app, "Changes", changes_open);
    let tools_w = segs_width(&files) + segs_width(&changes) + 2;
    let plus = chip(app, "+", Chip::Action, t.bg, t);
    let plus_w = segs_width(&plus);

    let tabs = app.fleet_tabs(model);
    let room = right_end
        .saturating_sub(left)
        .saturating_sub(tools_w + plus_w + 2);
    // Each tab at its natural width, its title cut at TITLE_MAX.
    let titled: Vec<(String, u16)> = tabs
        .iter()
        .map(|tab| {
            let title = truncate(&tab.title, TITLE_MAX);
            let w = title.width() as u16
                + TAB_CHROME
                + if tab.panes >= 2 {
                    format!(" {}", tab.panes).width() as u16
                } else {
                    0
                };
            (title, w)
        })
        .collect();
    let active = tabs.iter().position(|t| t.active).unwrap_or(0);
    let widths: Vec<u16> = titled.iter().map(|(_, w)| w + 1).collect();
    let (first, shown) = if widths.iter().sum::<u16>() <= room {
        (0, titled.len())
    } else {
        // Not all fit: keep room for the overflow chip and scroll until the active tab shows.
        let budget = room.saturating_sub(OVERFLOW_W);
        super::side::window(&widths, 0, Some(active), budget)
    };
    let hidden = titled.len() - shown;

    let mut x = left;
    let mut closes: Vec<(Rect, ClientTerm)> = Vec::new();
    for (i, tab) in tabs.iter().enumerate().skip(first).take(shown) {
        let (title, w) = &titled[i];
        let segs = tab_segs(app, tab, title, focused, t);
        put(buf, x, area.y, &segs, x + w);
        hit(
            app,
            Rect {
                x,
                y: area.y,
                width: *w,
                height: 1,
            },
            ScreenHit::Tab(tab.ship),
        );
        // The × is the cell before the closing space.
        closes.push((
            Rect {
                x: x + w - 3,
                y: area.y,
                width: 3,
                height: 1,
            },
            tab.ship,
        ));
        // A cell of the border stays between tabs, in the colour the frame gave it.
        x += w + 1;
    }
    for (r, ship) in closes {
        hit(app, r, ScreenHit::TabClose(ship));
    }
    if hidden > 0 {
        let more = chip(app, &format!("…{hidden}"), Chip::Quiet, t.bg, t);
        let w = segs_width(&more);
        put(buf, x, area.y, &more, x + w);
        x += w + 1;
    }
    put(buf, x, area.y, &plus, x + plus_w);
    hit(
        app,
        Rect {
            x,
            y: area.y,
            width: plus_w,
            height: 1,
        },
        ScreenHit::NewShell,
    );
    // The tool chips at the right edge, a border cell between them.
    let mut at = right_end;
    for (segs, h) in [(changes, ScreenHit::ChangesChip), (files, ScreenHit::FilesChip)] {
        let w = segs_width(&segs);
        if at < x + plus_w + w + 1 {
            break;
        }
        at -= w;
        put(buf, at, area.y, &segs, at + w);
        hit(
            app,
            Rect {
                x: at,
                y: area.y,
                width: w,
                height: 1,
            },
            h,
        );
        at -= 1;
    }
}
