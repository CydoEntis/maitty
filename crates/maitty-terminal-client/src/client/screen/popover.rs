//! The popover beside a sidebar row (the screen isn't dimmed): a quick follow-up to an agent
//! (`m`). A `◂` points at the row.

use super::*;
use crate::client::ids::{ClientTerm, Outgoing};
use crate::protocol::ClientMsg;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A follow-up being written: to `term`, from the sidebar (`side`: back to its cursor after)
/// or from an Inbox row (`inbox`: the Inbox's query and row, to go back to).
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct Compose {
    pub term: ClientTerm,
    pub text: String,
    pub side: bool,
    pub inbox: Option<(String, usize)>,
}

/// The Inbox row's text area grows to this many rows.
pub(in crate::client) const INBOX_ROWS: u16 = 6;

/// Word-wrapped to `width`, keeping blank lines.
pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut cur = String::new();
        for word in para.split(' ') {
            if !cur.is_empty() && cur.width() + 1 + word.width() > width {
                out.push(std::mem::take(&mut cur));
            } else if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
            while cur.width() > width {
                let head: String = cur.chars().take(width).collect();
                cur = cur.chars().skip(width).collect();
                out.push(head);
            }
        }
        out.push(cur);
    }
    out
}

/// `text` word-wrapped to `width` cells on at most `max` lines, so a long sentence drops to the
/// next line instead of being cut. What still does not fit ends the last line with `…`.
pub(super) fn wrap_capped(text: &str, width: usize, max: usize) -> Vec<String> {
    let max = max.max(1);
    let mut lines = wrap(text, width);
    if lines.len() > max {
        lines.truncate(max);
        if let Some(mut last) = lines.pop() {
            while last.width() + 1 > width && !last.is_empty() {
                last.pop();
            }
            last.push('…');
            lines.push(last);
        }
    }
    lines
}

/// The rows a text area `w` wide takes (its box and the keys under it).
pub(in crate::client) fn compose_rows(c: &Compose, w: u16, max: u16) -> u16 {
    (wrap(&c.text, w.saturating_sub(6) as usize).len() as u16).clamp(3, max) + 3
}

/// The text area at `at` (its top left and width), on `outer`: a rounded box of 3 to `max` rows that grows
/// with the text and scrolls past that (`↑ n more` in its top border), a word count top
/// right, and the keys under it. Returns the rows it took.
pub(in crate::client) fn compose_box(
    app: &mut App,
    buf: &mut Buffer,
    at: Rect,
    c: &Compose,
    max: u16,
    outer: Color,
    t: &Theme,
) -> u16 {
    let (x, y, w) = (at.x, at.y, at.width);
    let lines = wrap(&c.text, w.saturating_sub(6) as usize);
    let rows = (lines.len() as u16).clamp(3, max);
    let hidden = lines.len().saturating_sub(rows as usize);
    let more = if hidden > 0 {
        format!("↑ {hidden} more")
    } else {
        String::new()
    };
    let mut card_ = Card::new(t, &more);
    card_.border = t.accent;
    card_.bold = true;
    card_.title_fg = t.muted;
    card_.bg = t.card2;
    let r = Rect {
        x,
        y,
        width: w,
        height: rows + 2,
    };
    card(app, buf, r, &card_, t);
    let words = c.text.split_whitespace().count();
    if words > 0 {
        let wc = format!(" {words} word{} ", if words == 1 { "" } else { "s" });
        put(
            buf,
            r.right().saturating_sub(wc.width() as u16 + 2),
            y,
            &[seg(wc, Style::default().fg(t.muted).bg(outer))],
            r.right(),
        );
    }
    let s2 = Style::default().bg(t.card2);
    if c.text.is_empty() {
        let name = app
            .term_info(c.term)
            .map(|i| i.display_name())
            .unwrap_or_default();
        put(
            buf,
            x + 3,
            y + 1,
            &[
                seg(
                    format!("Next prompt for {name}…"),
                    s2.fg(t.muted).add_modifier(Modifier::ITALIC),
                ),
                seg(" █", s2.fg(t.accent)),
            ],
            r.right() - 2,
        );
    }
    let shown = &lines[hidden..];
    for (i, l) in shown.iter().enumerate().filter(|_| !c.text.is_empty()) {
        let mut segs = vec![seg(l.clone(), s2.fg(t.strong))];
        if i + 1 == shown.len() {
            segs.push(seg("█", s2.fg(t.accent)));
        }
        put(buf, x + 3, y + 1 + i as u16, &segs, r.right() - 2);
    }
    // To an agent that's working, Enter queues it for when the turn is over.
    let working = app
        .term_info(c.term)
        .is_some_and(|i| i.status == Status::Working);
    let keys = if working {
        cap_hints(
            t,
            outer,
            &[
                ("Enter", "send when it's done"),
                ("Ctrl+Enter", "now"),
                ("Shift+Enter", "new line"),
                ("Esc", "cancel"),
            ],
        )
    } else {
        cap_hints(
            t,
            outer,
            &[
                ("Enter", "send"),
                ("Shift+Enter", "new line"),
                ("Esc", "cancel"),
            ],
        )
    };
    put(buf, x + 1, y + rows + 2, &keys, x + w);
    rows + 3
}

/// The follow-up popover's widest width: you are replying, not writing a document.
const POPOVER_W: u16 = 68;
/// Below this width the hints and the working line use their short words.
const POPOVER_NARROW_W: u16 = 60;


/// Quote lines and input rows the popover allows at a screen height: 160x45, 100x30, 80x24.
fn size_tier(screen_h: u16) -> (usize, usize) {
    if screen_h >= 45 {
        (4, 6)
    } else if screen_h >= 30 {
        (3, 4)
    } else {
        (2, 3)
    }
}

impl App {
    /// Start a follow-up to `term`; `inbox`: from that Inbox place (query, row). A draft kept
    /// with Esc comes back.
    pub(in crate::client) fn start_compose(
        &mut self,
        term: ClientTerm,
        side: bool,
        inbox: Option<(String, usize)>,
    ) {
        let agent = self.term_info(term).is_some_and(|t| t.agent.is_some());
        if !agent {
            self.notify("only an agent takes a follow-up".into(), true);
            return;
        }
        let text = self.screen.drafts.get(&term).cloned().unwrap_or_default();
        self.mode = Mode::Compose(Box::new(Compose {
            term,
            text,
            side,
            inbox,
        }));
    }

    /// Back where the follow-up was started from.
    fn compose_back(&mut self, c: &Compose) {
        self.mode = match &c.inbox {
            Some((query, sel)) => Mode::GoTo {
                query: query.clone(),
                sel: *sel,
            },
            None if c.side => Mode::Side,
            None => Mode::Normal,
        };
    }

    pub(in crate::client) fn on_compose_key(&mut self, mut c: Compose, k: &KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let newline = k
            .modifiers
            .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT);
        match k.code {
            KeyCode::Esc => {
                if c.text.trim().is_empty() {
                    self.screen.drafts.remove(&c.term);
                } else {
                    self.screen.drafts.insert(c.term, c.text.clone());
                }
                return self.compose_back(&c);
            }
            KeyCode::Enter if newline => c.text.push('\n'),
            KeyCode::Enter => {
                let text = c.text.trim_end().to_string();
                if !text.trim().is_empty() {
                    self.screen.drafts.remove(&c.term);
                    // Typed into a working agent it would interrupt, or be lost: it waits its turn.
                    let working = self
                        .term_info(c.term)
                        .is_some_and(|t| t.status == Status::Working);
                    if working && !ctrl {
                        self.queue_follow_up(c.term, &text);
                    } else {
                        self.type_follow_up(c.term, &text);
                    }
                }
                return self.compose_back(&c);
            }
            KeyCode::Backspace => {
                c.text.pop();
            }
            KeyCode::Char('u') if ctrl => c.text.clear(),
            KeyCode::Char(ch) if !ctrl => c.text.push(ch),
            _ => {}
        }
        self.mode = Mode::Compose(Box::new(c));
    }

    /// Leave `text` with the server for the agent: it's typed in when this turn is over.
    pub(in crate::client) fn queue_follow_up(&mut self, term: ClientTerm, text: &str) {
        self.cmd_on(
            term.machine,
            Command::QueueFollowUp {
                term: term.term,
                text: text.to_string(),
            },
        );
    }

    /// Type `text` into the agent's prompt and press Enter (a moment later, once it has taken
    /// the text); several lines go in as one paste.
    fn type_follow_up(&mut self, term: ClientTerm, text: &str) {
        let to = term;
        let bracketed = self
            .parsers
            .get(&to)
            .is_some_and(|p| p.screen().bracketed_paste());
        let data = if text.contains('\n') && bracketed {
            format!("\x1b[200~{}\x1b[201~", text.replace('\n', "\r"))
        } else {
            text.replace('\n', " ")
        };
        self.send_input(to, data.into_bytes());
        let out = self.out.clone();
        std::thread::spawn(move || {
            std::thread::sleep(ENTER_AFTER);
            let _ = out.send(Outgoing {
                machine: to.machine,
                msg: ClientMsg::Input {
                    term: to.term,
                    data: b"\r".to_vec(),
                },
            });
        });
    }

}

/// How long after the text the follow-up's Enter goes, so the agent has taken the text.
const ENTER_AFTER: std::time::Duration = std::time::Duration::from_millis(60);

/// What a blocked agent asks, when it has said nothing else the popover can quote.
fn quote_fallback(app: &App, term: ClientTerm) -> String {
    let model = app.screen_model();
    find(&model, term)
        .and_then(|(_, _, s)| s.question.clone())
        .unwrap_or_default()
}

/// The follow-up popover, beside its sidebar row with the rest of the screen dimmed.
pub(in crate::client) fn draw_compose_pop(
    app: &mut App,
    buf: &mut Buffer,
    screen: Rect,
    side: Rect,
    c: &Compose,
    t: &Theme,
) {
    let Some(info) = app.term_info(c.term).cloned() else {
        return;
    };
    let ascii = app.cfg.ui.glyphs == "ascii";
    let (quote_max, input_max) = size_tier(screen.height);
    // Beside the sidebar; with the sidebar hidden, inside the padding round the screen, which
    // stays empty.
    let x0 = (side.right() + 1).max(framed(app, screen).x + 1);
    let w = POPOVER_W.min(screen.right().saturating_sub(x0 + 1));
    if w < 20 {
        return;
    }
    let (iw, x) = (w as usize - 4, x0 + 2);
    let narrow = w < POPOVER_NARROW_W;

    let agent = info.agent.clone().unwrap_or_else(|| "agent".into());
    let blocked = info.status == Status::Blocked;
    let working = info.status == Status::Working;
    let said = if info.said.trim().is_empty() {
        quote_fallback(app, c.term)
    } else {
        info.said.clone()
    };
    let quote: Vec<String> = wrap(said.trim(), iw - 2)
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .collect();
    let shown = &quote[quote.len().saturating_sub(quote_max)..];
    let cut = quote.len() > shown.len();
    let input = wrap(&c.text, iw - 2);
    let more = if input.len() > input_max {
        input.len() - (input_max - 1)
    } else {
        0
    };
    let visible = if more > 0 {
        &input[input.len() - (input_max - 1)..]
    } else {
        &input[..]
    };
    let input_rows = if more > 0 { input_max } else { input.len() } as u16;
    let quote_rows = if shown.is_empty() {
        0
    } else {
        shown.len() as u16 + 1
    };
    let h = 2 + quote_rows + 1 + input_rows + u16::from(working) + 1;

    let (r, _) = frame_beside_row(app, buf, screen, side, c.term, (x0, w, h), t);

    let name = app.session_name(c.term);
    let title = vec![
        seg(
            format!(" MESSAGE {name} "),
            Style::default()
                .fg(t.acc_ink)
                .bg(t.accent)
                .add_modifier(Modifier::BOLD),
        ),
        seg(
            format!("· {agent} "),
            Style::default().fg(t.muted).bg(t.card),
        ),
    ];
    put(buf, r.x + 2, r.y, &plain(title.clone()), r.right() - 2);
    let bold = Modifier::BOLD;
    let state = if blocked {
        vec![seg(
            format!(" {} needs you ", if ascii { "!" } else { "●" }),
            Style::default().fg(t.blocked).bg(t.card).add_modifier(bold),
        )]
    } else if working {
        vec![seg(
            format!(" {}working ", glyph(app, Status::Working)),
            Style::default().fg(t.accent).bg(t.card).add_modifier(bold),
        )]
    } else {
        vec![seg(
            format!(" {} done ", if ascii { "o" } else { "○" }),
            Style::default().fg(t.muted).bg(t.card),
        )]
    };
    let state_x = (r.right() - 2).saturating_sub(segs_width(&state));
    if state_x > r.x + 2 + segs_width(&title) {
        put(buf, state_x, r.y, &state, r.right() - 2);
    }

    let ground = Style::default().bg(t.card);
    let mut y = r.y + 1;
    if !shown.is_empty() {
        let verb = if blocked { "asked" } else { "said" };
        let tail = if cut {
            format!(" · last {} of {} lines", shown.len(), quote.len())
        } else {
            String::new()
        };
        put(
            buf,
            x,
            y,
            &[
                seg(
                    format!("{agent} {verb}"),
                    ground.fg(t.muted).add_modifier(bold),
                ),
                seg(tail, ground.fg(t.muted)),
            ],
            x + iw as u16,
        );
        y += 1;
        for l in shown {
            put(
                buf,
                x,
                y,
                &[
                    seg(if ascii { "| " } else { "▏ " }, ground.fg(t.muted)),
                    seg(l.clone(), ground.fg(t.text)),
                ],
                x + iw as u16,
            );
            y += 1;
        }
    }
    for xx in r.x + 1..r.right() - 1 {
        buf[(xx, y)].set_symbol("─").set_style(ground.fg(t.line));
    }
    y += 1;
    let well = Style::default().bg(t.bg);
    fill(
        buf,
        Rect {
            x: x - 1,
            y,
            width: iw as u16 + 2,
            height: input_rows,
        },
        t.bg,
    );
    if more > 0 {
        let up = if ascii { "^" } else { "↑" };
        put(
            buf,
            x,
            y,
            &[seg(
                format!("{up} {more} more line{}", if more == 1 { "" } else { "s" }),
                well.fg(t.muted),
            )],
            x + iw as u16,
        );
        y += 1;
    }
    for (i, l) in visible.iter().enumerate() {
        let first = more == 0 && i == 0;
        let mut segs = vec![seg(
            if first {
                if ascii { "> " } else { "› " }
            } else {
                "  "
            },
            well.fg(t.accent).add_modifier(bold),
        )];
        if c.text.is_empty() {
            segs.push(seg("█", well.fg(t.accent)));
            segs.push(seg(format!(" Reply to {agent}…"), well.fg(t.muted)));
        } else {
            segs.push(seg(l.clone(), well.fg(t.strong)));
            if i + 1 == visible.len() {
                segs.push(seg("█", well.fg(t.accent)));
            }
        }
        put(buf, x, y, &segs, x + iw as u16);
        y += 1;
    }
    if working {
        let rest = if narrow {
            "Enter queues it."
        } else {
            "Enter queues this until the turn ends."
        };
        put(
            buf,
            x,
            y,
            &[
                seg(format!("{agent} is mid-turn. "), ground.fg(t.text)),
                seg(rest, ground.fg(t.muted)),
            ],
            x + iw as u16,
        );
        y += 1;
    }
    let keys = if working {
        cap_hints_gap(
            t,
            t.card,
            if narrow { "  " } else { "   " },
            &[
                ("Enter", "queue"),
                ("Ctrl+Enter", if narrow { "now" } else { "send now" }),
                ("Esc", "cancel"),
            ],
        )
    } else {
        cap_hints_gap(
            t,
            t.card,
            if narrow { "  " } else { "   " },
            &[
                ("Enter", "send"),
                ("Shift+Enter", if narrow { "line" } else { "new line" }),
                ("Esc", "cancel"),
            ],
        )
    };
    let keys_end = put(buf, x, y, &keys, x + iw as u16);
    let words = c.text.split_whitespace().count();
    if words > 0 {
        let wc = format!("{words} word{}", if words == 1 { "" } else { "s" });
        let wx = (x + iw as u16).saturating_sub(wc.width() as u16);
        if wx > keys_end + 1 {
            put(buf, wx, y, &[seg(wc, ground.fg(t.muted))], x + iw as u16);
        }
    }
}

/// A popover's frame right of the sidebar, at `(x, width, height)` as far as the screen allows,
/// beside `term`'s row: the screen dimmed except that row, a full accent border on `card`, and
/// a `◂` on the row. Returns the frame and the row's rect.
pub(in crate::client) fn frame_beside_row(
    app: &mut App,
    buf: &mut Buffer,
    screen: Rect,
    side: Rect,
    term: ClientTerm,
    (x0, w, h): (u16, u16, u16),
    t: &Theme,
) -> (Rect, Rect) {
    let ascii = app.cfg.ui.glyphs == "ascii";
    let hit_row = app.hits.iter().find_map(|(r, hit)| match hit {
        crate::client::Hit::Screen(ScreenHit::Session(t))
            if *t == app.session_key(term) && r.right() <= x0 =>
        {
            Some(*r)
        }
        _ => None,
    });
    let anchor = hit_row.unwrap_or(Rect {
        x: side.x,
        y: screen.y + 1,
        width: side.width,
        height: 1,
    });
    // Never in the padding round the screen, which stays empty.
    let top = place(screen, anchor, h).max(framed(app, screen).y);
    let r = Rect {
        x: x0,
        y: top,
        width: w,
        height: h,
    };

    let kept = hit_row.map(|row| {
        (
            row,
            (row.y..row.bottom())
                .flat_map(|y| (row.x..row.right()).map(move |x| (x, y)))
                .map(|p| (p, buf[p].clone()))
                .collect::<Vec<_>>(),
        )
    });
    dim_all(app, buf, screen, t);
    if let Some((_, cells)) = kept {
        for (p, cell) in cells {
            buf[p] = cell;
        }
    }

    let look = Look::of(&app.cfg.ui);
    fill(buf, r, t.card);
    // Hairline frames it in `line` like every other popup; the `◂` keeps the accent.
    let edge = Style::default()
        .fg(if look.hairline { t.line } else { t.accent })
        .bg(t.card);
    let pointer = Style::default().fg(t.accent).bg(t.card);
    let [tl, tr, bl, br] = corners(look);
    for xx in r.x + 1..r.right() - 1 {
        buf[(xx, r.y)].set_symbol("─").set_style(edge);
        buf[(xx, r.bottom() - 1)].set_symbol("─").set_style(edge);
    }
    for yy in r.y + 1..r.bottom() - 1 {
        buf[(r.x, yy)].set_symbol("│").set_style(edge);
        buf[(r.right() - 1, yy)].set_symbol("│").set_style(edge);
    }
    for (xx, yy, s) in [
        (r.x, r.y, tl),
        (r.right() - 1, r.y, tr),
        (r.x, r.bottom() - 1, bl),
        (r.right() - 1, r.bottom() - 1, br),
    ] {
        buf[(xx, yy)].set_symbol(s).set_style(edge);
    }
    hit(app, r, ScreenHit::Noop);
    if anchor.y > r.y && anchor.y < r.bottom() - 1 {
        put(
            buf,
            r.x,
            anchor.y,
            &[seg(
                if ascii { "<" } else { "◂" },
                pointer.add_modifier(Modifier::BOLD),
            )],
            r.x + 1,
        );
    }

    (r, anchor)
}

/// The popover's top row for a session row at `anchor`: its top border a line above the row,
/// else (opening upward) its bottom border just under the row, else pinned to the bottom of
/// the screen. Under the row, not on its last line, so the `◂` stays on the row.
fn place(screen: Rect, anchor: Rect, h: u16) -> u16 {
    let mut top = anchor.y.saturating_sub(1);
    if top + h > screen.bottom() {
        top = (anchor.bottom() + 1).saturating_sub(h);
        if top + h > screen.bottom() {
            top = screen.bottom().saturating_sub(h);
        }
    }
    top.max(screen.y)
}

#[cfg(test)]
mod tests {
    use super::{wrap, wrap_capped};

    const LONG: &str = "Files is local-only in v1: the files are on kingdom, not on this computer.";

    #[test]
    fn text_that_fits_stays_on_one_line() {
        assert_eq!(
            wrap_capped("Connected to kingdom", 40, 3),
            ["Connected to kingdom"]
        );
    }

    #[test]
    fn long_text_drops_to_the_next_line_and_loses_nothing() {
        let lines = wrap_capped(LONG, 44, 3);
        assert!(lines.len() >= 2, "it wraps: {lines:?}");
        assert!(
            lines.iter().all(|l| l.chars().count() <= 44),
            "each line fits: {lines:?}"
        );
        assert_eq!(lines.join(" "), LONG, "every word is kept, in order");
    }

    #[test]
    fn text_past_the_line_limit_ends_the_last_line_with_an_ellipsis() {
        let lines = wrap_capped(LONG, 14, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'), "{lines:?}");
        assert!(lines.iter().all(|l| l.chars().count() <= 14), "{lines:?}");
    }

    #[test]
    fn a_narrower_width_takes_more_lines_for_the_same_words() {
        let wide = wrap_capped(LONG, 60, 9).len();
        let narrow = wrap_capped(LONG, 20, 9).len();
        assert!(narrow > wide, "{narrow} lines at 20 vs {wide} at 60");
    }

    #[test]
    fn wide_characters_count_by_their_cells() {
        let lines = wrap_capped("キーボード キーボード", 10, 3);
        assert_eq!(lines, ["キーボード", "キーボード"]);
    }

    #[test]
    fn the_text_area_wraps_on_words_and_keeps_blank_lines() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("a\n\nb", 10), vec!["a", "", "b"]);
        assert_eq!(
            wrap("abcdefghij", 4),
            vec!["abcd", "efgh", "ij"],
            "a word longer than the line breaks"
        );
    }
}

#[cfg(test)]
mod message_tests {
    use super::*;
    use crate::client::reference_fixture::{LAYOUTS, SIZES, Stage, frame, world};

    const QUESTION: &str = "Both lockfiles are in the repo.\nCI runs pnpm install, but the README says npm.\nTwo scripts call npx directly.\nUse pnpm or npm for the workspace?\nTell me which.";

    fn open(term: ClientTerm, text: &str) -> App {
        let mut app = world(Stage::Failed);
        app.snap.terms.get_mut(&term.term).unwrap().said = QUESTION.into();
        app.start_compose(term, true, None);
        if let Mode::Compose(c) = &mut app.mode {
            c.text = text.into();
        }
        app
    }

    fn type_keys(app: &mut App, text: &str) {
        for ch in text.chars() {
            app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
    }

    #[test]
    fn it_names_the_session_quotes_the_tail_and_points_at_the_row_in_every_layout_and_size() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut app = open(ClientTerm::local(3), "");
                let o = frame(&mut app, layout, w, h, "unicode");
                let (quote, _) = size_tier(h);
                assert!(
                    o.contains("MESSAGE warm-finch · codex"),
                    "{layout:?} {w}x{h}\n{o}"
                );
                assert!(
                    o.contains(&format!("codex asked · last {quote} of 5 lines")) || quote >= 5,
                    "{layout:?} {w}x{h}\n{o}"
                );
                assert!(
                    o.contains("▏ Tell me which.") && !o.contains("Both lockfiles"),
                    "the tail, not the head\n{o}"
                );
                assert!(
                    o.contains("◂") && o.contains("Reply to codex…") && o.contains("● needs you"),
                    "{layout:?} {w}x{h}\n{o}"
                );
                let corner = o.lines().filter(|l| l.contains("MESSAGE")).any(|l| {
                    l.contains("┌─ MESSAGE")
                        || l.contains("╭─ MESSAGE")
                        || l.contains("┌ MESSAGE")
                        || l.contains(" MESSAGE")
                });
                assert!(corner);
                let top = o.lines().find(|l| l.contains("MESSAGE")).unwrap();
                assert!(
                    top.contains('┐') || top.contains('╮'),
                    "a full accent border in {layout:?}: {top}"
                );
            }
        }
    }

    #[test]
    fn it_grows_a_row_a_line_then_scrolls() {
        let rows_of = |lines: usize, w: u16, h: u16| {
            let mut app = open(
                ClientTerm::local(3),
                &(1..=lines)
                    .map(|i| format!("line {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            let o = frame(
                &mut app,
                crate::client::reference_fixture::Layout::Floating,
                w,
                h,
                "unicode",
            );
            (
                o.lines()
                    .filter(|l| l.contains("│ › line") || l.contains("│   line "))
                    .count(),
                o,
            )
        };
        assert_eq!(rows_of(1, 160, 45).0, 1);
        assert_eq!(rows_of(3, 160, 45).0, 3);
        let (n, o) = rows_of(9, 160, 45);
        assert!(
            n == 5 && o.contains("↑ 4 more lines") && o.contains("line 9█"),
            "{o}"
        );
        let (n, o) = rows_of(9, 100, 30);
        assert!(n == 3 && o.contains("↑ 6 more lines"), "{o}");
        let (n, o) = rows_of(9, 80, 24);
        assert!(n == 2 && o.contains("↑ 7 more lines"), "{o}");
    }

    #[test]
    fn it_opens_upward_then_pins_when_neither_side_has_room() {
        let screen = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        };
        assert_eq!(
            place(
                screen,
                Rect {
                    x: 0,
                    y: 5,
                    width: 30,
                    height: 2
                },
                9
            ),
            4,
            "a line above the row"
        );
        let top = place(
            screen,
            Rect {
                x: 0,
                y: 26,
                width: 30,
                height: 2,
            },
            9,
        );
        assert_eq!(top + 9, 29, "its bottom border just under the row: {top}");
        assert_eq!(
            place(
                screen,
                Rect {
                    x: 0,
                    y: 28,
                    width: 30,
                    height: 1
                },
                40
            ),
            0,
            "taller than the screen: pinned"
        );
        assert_eq!(
            place(
                Rect {
                    height: 20,
                    ..screen
                },
                Rect {
                    x: 0,
                    y: 19,
                    width: 30,
                    height: 1
                },
                12
            ),
            8
        );
    }

    #[test]
    fn esc_keeps_the_draft_for_that_session_and_an_empty_enter_only_closes() {
        let mut app = open(ClientTerm::local(3), "");
        type_keys(&mut app, "half a thought");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.mode == Mode::Side);
        app.start_compose(ClientTerm::local(1), true, None);
        assert!(
            matches!(&app.mode, Mode::Compose(c) if c.text.is_empty()),
            "another session's box is empty"
        );
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.start_compose(ClientTerm::local(3), true, None);
        assert!(
            matches!(&app.mode, Mode::Compose(c) if c.text == "half a thought"),
            "its own draft is back"
        );
        for _ in 0.."half a thought".len() {
            app.on_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        }
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(
            app.mode == Mode::Side,
            "nothing sent"
        );
    }
}
