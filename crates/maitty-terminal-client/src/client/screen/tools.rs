//! The tool rail on the right edge: its chips open Files, Changes, Branch and References in the
//! drawer (a second click closes).

use super::sheet::{DRAWER_TABS, SheetKind};
use super::*;

/// Rows between one chip of the rail and the next.
const CHIP_STEP: u16 = 2;

/// The rail: a box with the four tools' keys stacked down it. The open tool is lit in the accent,
/// solid while the drawer has the keys.
pub(in crate::client) fn draw_tools(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme) {
    if r.width == 0 || r.height < 3 {
        return;
    }
    let look = Look::of(&app.cfg.ui);
    panel_box(buf, r, look, Style::default().fg(t.line).bg(t.bg));
    hit(app, r, ScreenHit::Noop);
    let open = sheet::open_sheet(app);
    let keys = sheet::sheet_has_keys(app);
    for (i, (kind, _, _, action)) in DRAWER_TABS.iter().enumerate() {
        let y = r.y + 1 + i as u16 * CHIP_STEP;
        if y + 1 >= r.bottom() {
            break;
        }
        let key = app
            .keymap
            .prefixed_order
            .iter()
            .find(|k| app.keymap.prefixed.get(k) == Some(action))
            .map(|k| k.to_string())
            .unwrap_or_else(|| tool_key(*kind).to_string());
        let mine = open == Some(*kind);
        let style = if mine && keys {
            Style::default()
                .fg(t.acc_ink)
                .bg(t.accent)
                .add_modifier(Modifier::BOLD)
        } else if mine {
            Style::default()
                .fg(t.accent)
                .bg(t.btn)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text).bg(t.btn)
        };
        let chip = Rect {
            x: r.x + 1,
            y,
            width: r.width.saturating_sub(2),
            height: 1,
        };
        let label = format!("{key:^w$}", w = chip.width as usize);
        // With `ui.rounded = full` the chip is a pill: the key between two caps.
        let segs = if look.caps {
            round_ends(vec![seg(key.clone(), style)], t.bg)
        } else {
            vec![seg(label, style)]
        };
        put(buf, chip.x, y, &segs, chip.right());
        hit(app, chip, ScreenHit::Tool(i));
    }
}

/// The key that opens a tool when the keymap has none for it.
fn tool_key(kind: SheetKind) -> char {
    match kind {
        SheetKind::Files => 'f',
        SheetKind::Changes => 'c',
        SheetKind::Branch => 'b',
        _ => 'a',
    }
}
