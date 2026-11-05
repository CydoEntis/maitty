//! The splash on a roomy terminal: the wordmark over two boxes side by side, START (the three
//! choices and the recent projects) and WHILE YOU WERE AWAY (the agents, most urgent first, and
//! the machines). A smaller terminal keeps the centred cards in `splash.rs`.

use super::splash::{SWELL_REACH, draw_logo, draw_swell, logo_width, pixel_width};
use super::*;
use crate::theme::mix;

const START_W: u16 = 64;
const AWAY_W: u16 = 72;
const BOX_GAP: u16 = 2;
const BOX_H: u16 = 22;
/// The wordmark, the swell and the tagline above the boxes, and the gap before them.
const ABOVE_BOXES: u16 = 11;
/// The terminal needs this much for both boxes and the wordmark above them.
const MIN_W: u16 = START_W + BOX_GAP + AWAY_W + 4;
const MIN_H: u16 = ABOVE_BOXES + BOX_H + 7;
/// The most the wordmark sits below the outer box's top edge.
const LOGO_DROP: u16 = 8;
/// Rows a menu choice's highlight takes, and the blank row between two.
const CHOICE_ROWS: u16 = 3;
/// Rows a session takes in the away list: its line, its activity and a blank row.
const AWAY_ROW: u16 = 3;

/// Whether `area` has room for both boxes.
pub(in crate::client) fn is_wide(area: Rect) -> bool {
    area.width >= MIN_W && area.height >= MIN_H
}

/// The projects the splash lists to open, this computer's first.
pub(in crate::client) fn recent_projects(model: &[Proj]) -> Vec<&Proj> {
    model.iter().take(RECENT_MAX).collect()
}

const RECENT_MAX: usize = 5;

/// Agents for the away list, the ones that need you first, then working, done and idle.
pub(in crate::client) fn away_sessions(model: &[Proj]) -> Vec<(&Session, &Proj)> {
    let mut all: Vec<(&Session, &Proj)> = model
        .iter()
        .flat_map(|p| p.sessions().map(move |s| (s, p)))
        .filter(|(s, _)| s.is_agent)
        .collect();
    all.sort_by_key(|(s, _)| urgency(s));
    all.truncate(AWAY_MAX);
    all
}

const AWAY_MAX: usize = 5;

fn urgency(s: &Session) -> u8 {
    if check::needs_you(s) || s.question.is_some() {
        0
    } else {
        match s.status {
            Status::Working => 1,
            Status::Done => 2,
            _ => 3,
        }
    }
}

/// The sub-text of a menu choice.
fn sub_text(c: char, away: &[(&Session, &Proj)]) -> String {
    match c {
        'r' => match away.iter().find(|(s, _)| urgency(s) == 0) {
            Some((s, _)) => format!("{} needs you", s.name),
            None => format!("{} sessions as left", away.len()),
        },
        'n' => "claude on this computer".to_string(),
        _ => "pick a folder on any machine".to_string(),
    }
}

fn badge_text(app: &App, machine: MachineId) -> String {
    if machine == MachineId::LOCAL {
        format!("{} Local", machine_group::LOCAL_ICON)
    } else {
        format!(
            "{} {}",
            machine_group::REMOTE_ICON,
            app.machine_label_of(machine)
        )
    }
}

fn badge(app: &App, machine: MachineId, up: bool, t: &Theme) -> Seg {
    let style = if up {
        Style::default().fg(t.strong).bg(t.btn)
    } else {
        Style::default().fg(t.muted).bg(t.card)
    };
    seg(
        format!(" {} ", badge_text(app, machine)),
        style.add_modifier(Modifier::BOLD),
    )
}

fn pill(app: &App, segs: Vec<Seg>, outer: Color) -> Vec<Seg> {
    if Look::of(&app.cfg.ui).caps {
        round_ends(segs, outer)
    } else {
        segs
    }
}

/// A title chip on a border at `(x, y)`: the accent when its box holds the keys.
fn title(
    app: &App,
    buf: &mut Buffer,
    (x, y): (u16, u16),
    label: &str,
    lit: bool,
    t: &Theme,
) -> u16 {
    let (bg, ink) = if lit {
        (t.accent, t.acc_ink)
    } else {
        (t.btn, t.strong)
    };
    let chip = pill(
        app,
        vec![seg(
            format!(" {label} "),
            Style::default().fg(ink).bg(bg).add_modifier(Modifier::BOLD),
        )],
        t.bg,
    );
    let w = segs_width(&chip);
    put(buf, x, y, &chip, buf.area.right());
    w
}

/// `├────┤` across `r`'s width at its row, in the box's edge.
fn rule(buf: &mut Buffer, (x, y): (u16, u16), w: u16, edge: Style) {
    for i in 0..w {
        let c = if i == 0 {
            "├"
        } else if i == w - 1 {
            "┤"
        } else {
            "─"
        };
        buf[(x + i, y)].set_symbol(c).set_style(edge);
    }
}

/// A key on a row: the accent when the row is chosen in the box that holds the keys.
fn key_chip(app: &App, key: &str, lit: bool, ground: Color, t: &Theme) -> Vec<Seg> {
    let st = if lit {
        Style::default().bg(t.accent).fg(t.acc_ink)
    } else {
        Style::default().bg(t.btn).fg(t.strong)
    };
    pill(
        app,
        vec![seg(format!(" {key} "), st.add_modifier(Modifier::BOLD))],
        ground,
    )
}

/// Mark a chosen row: the tint across it and, in the box with the keys, the bar.
fn mark_row(buf: &mut Buffer, r: Rect, lit: bool, t: &Theme) {
    fill(buf, r, t.card2);
    if lit {
        for y in r.y..r.bottom() {
            put(
                buf,
                r.x,
                y,
                &[seg("▌", Style::default().fg(t.accent).bg(t.card2))],
                r.x + 1,
            );
        }
    }
}

fn status_colour(s: &Session, t: &Theme) -> Color {
    match urgency(s) {
        0 => t.blocked,
        1 => t.accent,
        2 => t.done,
        _ => t.muted,
    }
}

/// The splash, wide: `intro` and `clock` are the frames the wordmark and swell are drawn by.
pub(in crate::client) fn draw_wide(
    app: &mut App,
    f: &mut Frame,
    area: Rect,
    model: &[Proj],
    (intro, clock): (u64, u64),
    t: &Theme,
) {
    app.screen.splash_wide = true;
    let look = Look::of(&app.cfg.ui);
    fill(f.buffer_mut(), area, t.bg);
    let outer = Rect {
        height: area.height.saturating_sub(1),
        ..area
    };
    let line = Style::default().fg(t.line).bg(t.bg);
    panel_box(f.buffer_mut(), outer, look, line);
    let version = format!(" {} ", env!("APP_VERSION"));
    let name = format!(" {} ", crate::brand::APP_NAME.to_uppercase());
    let nw = title(
        app,
        f.buffer_mut(),
        (outer.x + 2, outer.y),
        name.trim(),
        true,
        t,
    );
    put(
        f.buffer_mut(),
        outer.x + 2 + nw + 1,
        outer.y,
        &[seg(version, Style::default().fg(t.muted).bg(t.bg))],
        outer.right(),
    );
    // A release waiting: a chip on the top edge, where it can't be missed; a click or `u` acts.
    if let Some((news, what, hit_as)) = super::splash::release_news(app) {
        let chip = pill(
            app,
            vec![seg(
                format!(" {news}  u {what} "),
                Style::default()
                    .fg(t.acc_ink)
                    .bg(t.accent)
                    .add_modifier(Modifier::BOLD),
            )],
            t.bg,
        );
        let cw = segs_width(&chip);
        let x = outer.x + 2 + nw + 12;
        put(f.buffer_mut(), x, outer.y, &chip, outer.right());
        hit(
            app,
            Rect {
                x,
                y: outer.y,
                width: cw,
                height: 1,
            },
            hit_as,
        );
    }
    let here = pill(
        app,
        vec![seg(
            format!(" {} Local ", machine_group::LOCAL_ICON),
            Style::default()
                .fg(t.strong)
                .bg(t.btn)
                .add_modifier(Modifier::BOLD),
        )],
        t.bg,
    );
    let hw = segs_width(&here);
    put(
        f.buffer_mut(),
        outer.right().saturating_sub(hw + 2),
        outer.y,
        &here,
        outer.right(),
    );

    let centre = |w: u16| area.x + area.width.saturating_sub(w) / 2;
    let free = outer.height.saturating_sub(2 + ABOVE_BOXES + BOX_H);
    let top = outer.y + 1 + (free / 2).min(LOGO_DROP);
    let lw = logo_width(area.width);
    draw_logo(
        f.buffer_mut(),
        (centre(lw), top),
        pixel_width(area.width),
        intro,
        t,
    );
    let swell_w = (lw + SWELL_REACH * 2).min(area.width);
    draw_swell(
        f.buffer_mut(),
        (centre(swell_w), top + 6),
        swell_w,
        clock,
        intro,
        t,
    );
    let tag = concat!(env!("APP_TAGLINE"), ".");
    let shown = ((intro as f32 - 10.0) / 10.0).clamp(0.0, 1.0);
    put(
        f.buffer_mut(),
        centre(tag.width() as u16),
        top + 8,
        &[seg(
            tag,
            Style::default()
                .fg(mix(t.bg, t.muted, shown))
                .bg(t.bg)
                .add_modifier(Modifier::ITALIC),
        )],
        area.right(),
    );

    // The keys, and the version barely there, on the row under the box.
    let hints = crate::client::design::cap_hints_gap(
        t,
        t.bg,
        "   ",
        &[
            ("↑↓", "choose"),
            ("Enter", "open"),
            ("Tab", "switch box"),
            ("r n o", "jump"),
            ("?", "keys"),
        ],
    );
    let (hx, hy) = (area.x + 2, area.bottom().saturating_sub(1));
    put(f.buffer_mut(), hx, hy, &hints, area.right());
    hit(
        app,
        Rect {
            x: hx,
            y: hy,
            width: segs_width(&hints),
            height: 1,
        },
        ScreenHit::SplashKey('?'),
    );
    let foot = seg(
        format!(concat!(env!("APP_COMMAND"), " {}"), env!("APP_VERSION")),
        Style::default().fg(t.line).bg(t.bg),
    );
    let fw = segs_width(std::slice::from_ref(&foot));
    put(
        f.buffer_mut(),
        area.right().saturating_sub(fw + 2),
        hy,
        &[foot],
        area.right(),
    );

    let by = top + ABOVE_BOXES;
    let gx = centre(START_W + BOX_GAP + AWAY_W);
    let away = away_sessions(model);
    let recent = recent_projects(model);
    draw_start(
        app,
        f.buffer_mut(),
        Rect {
            x: gx,
            y: by,
            width: START_W,
            height: BOX_H,
        },
        (&away, &recent),
        t,
    );
    draw_away(
        app,
        f.buffer_mut(),
        Rect {
            x: gx + START_W + BOX_GAP,
            y: by,
            width: AWAY_W,
            height: BOX_H,
        },
        &away,
        t,
    );
}

fn draw_start(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    (away, recent): (&[(&Session, &Proj)], &[&Proj]),
    t: &Theme,
) {
    let look = Look::of(&app.cfg.ui);
    let lit = !app.screen.splash_away;
    let edge = Style::default()
        .fg(if lit { t.accent } else { t.line })
        .bg(t.bg);
    panel_box(buf, r, look, edge);
    title(app, buf, (r.x + 2, r.y), "START", lit, t);
    hit(app, r, ScreenHit::SplashFocus(false));
    let opts = splash_options(app);
    let sel = app.screen.splash_sel.min(opts.len() + recent.len() - 1);
    let (left, right) = (r.x + 3, r.right().saturating_sub(4));
    for (i, (label, key, c)) in opts.iter().enumerate() {
        let y = r.y + 1 + i as u16 * (CHOICE_ROWS + 1);
        let row = Rect {
            x: r.x + 1,
            y,
            width: r.width - 2,
            height: CHOICE_ROWS,
        };
        let on = sel == i;
        if on {
            mark_row(buf, row, lit, t);
        }
        let ground = if on { t.card2 } else { t.bg };
        let mid = y + 1;
        let label_style = if on {
            Style::default().fg(t.strong).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text)
        };
        put(
            buf,
            left,
            mid,
            &[seg(label.clone(), label_style.bg(ground))],
            right,
        );
        let sub = sub_text(*c, away);
        let needs = *c == 'r' && away.iter().any(|(s, _)| urgency(s) == 0);
        let keys = key_chip(app, key, on && lit, ground, t);
        let key_x = right.saturating_sub(segs_width(&keys) - 1);
        put(
            buf,
            left + 26,
            mid,
            &[seg(
                crate::client::render::truncate(&sub, key_x.saturating_sub(left + 27) as usize),
                Style::default()
                    .fg(if needs { t.blocked } else { t.muted })
                    .bg(ground),
            )],
            key_x,
        );
        put(buf, key_x, mid, &keys, r.right());
        hit(app, row, ScreenHit::SplashCard(i));
    }
    let dy = r.y + 2 + opts.len() as u16 * (CHOICE_ROWS + 1);
    rule(buf, (r.x, dy), r.width, edge);
    title(app, buf, (r.x + 2, dy), "RECENT PROJECTS", false, t);
    for (j, p) in recent.iter().enumerate() {
        let y = dy + 2 + j as u16;
        if y + 1 >= r.bottom() {
            break;
        }
        let row = Rect {
            x: r.x + 1,
            y,
            width: r.width - 2,
            height: 1,
        };
        let on = sel == opts.len() + j;
        if on {
            mark_row(buf, row, lit, t);
        }
        let ground = if on { t.card2 } else { t.bg };
        let branch = p
            .wts
            .iter()
            .find(|w| w.main)
            .or(p.wts.first())
            .map(|w| format!("⎇ {}", crate::client::render::truncate(&w.branch, 16)))
            .unwrap_or_default();
        let bw = branch.width() as u16;
        put(
            buf,
            right.saturating_sub(bw),
            y,
            &[seg(branch, Style::default().fg(t.muted).bg(ground))],
            right + 1,
        );
        let mut x = left;
        let name = crate::client::render::truncate(&p.name, 24);
        x = put(
            buf,
            x,
            y,
            &[
                seg(
                    "▸ ",
                    Style::default()
                        .fg(if on { t.accent } else { t.muted })
                        .bg(ground),
                ),
                seg(
                    name,
                    if on {
                        Style::default()
                            .fg(t.strong)
                            .bg(ground)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(t.text).bg(ground)
                    },
                ),
                seg("  ", Style::default().bg(ground)),
            ],
            right,
        );
        let b = pill(app, vec![badge(app, p.machine, true, t)], ground);
        put(buf, x, y, &b, right.saturating_sub(bw + 1));
        hit(app, row, ScreenHit::SplashProject(j));
    }
}

fn draw_away(app: &mut App, buf: &mut Buffer, r: Rect, away: &[(&Session, &Proj)], t: &Theme) {
    let look = Look::of(&app.cfg.ui);
    let lit = app.screen.splash_away;
    let edge = Style::default()
        .fg(if lit { t.accent } else { t.line })
        .bg(t.bg);
    panel_box(buf, r, look, edge);
    title(app, buf, (r.x + 2, r.y), "WHILE YOU WERE AWAY", lit, t);
    hit(app, r, ScreenHit::SplashFocus(true));

    let count = |f: &dyn Fn(&Session) -> bool| away.iter().filter(|(s, _)| f(s)).count();
    let (n_need, n_run, n_done) = (
        count(&|s| urgency(s) == 0),
        count(&|s| urgency(s) == 1),
        count(&|s| urgency(s) == 2),
    );
    let chip = |glyph: &str, n: usize, fg: Color| {
        seg(
            format!(" {glyph} {n} "),
            Style::default()
                .fg(fg)
                .bg(t.btn)
                .add_modifier(Modifier::BOLD),
        )
    };
    let counts = vec![
        chip("●", n_need, if n_need > 0 { t.blocked } else { t.muted }),
        chip("✻", n_run, t.muted),
        chip("✓", n_done, t.muted),
    ];
    let cw = segs_width(&counts);
    put(
        buf,
        r.right().saturating_sub(cw + 2),
        r.y,
        &counts,
        r.right(),
    );

    let (left, right) = (r.x + 3, r.right().saturating_sub(4));
    let sel = app.screen.splash_away_sel.min(away.len().saturating_sub(1));
    let machines_y = r.bottom().saturating_sub(5);
    for (i, (s, p)) in away.iter().enumerate() {
        let y = r.y + 2 + i as u16 * AWAY_ROW;
        if y + 2 > machines_y {
            break;
        }
        let row = Rect {
            x: r.x + 1,
            y,
            width: r.width - 2,
            height: 2,
        };
        let on = lit && sel == i;
        if on {
            mark_row(buf, row, true, t);
        }
        let ground = if on { t.card2 } else { t.bg };
        let colour = status_colour(s, t);
        let mut x = left;
        x = put(
            buf,
            x,
            y,
            &[
                seg(
                    format!("{} ", glyph(app, s.status)),
                    Style::default().fg(colour).bg(ground),
                ),
                seg(
                    s.name.clone(),
                    if on {
                        Style::default()
                            .fg(t.strong)
                            .bg(ground)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(t.text).bg(ground)
                    },
                ),
                seg("  ", Style::default().bg(ground)),
            ],
            right,
        );
        let b = pill(app, vec![badge(app, p.machine, true, t)], ground);
        x = put(buf, x, y, &b, right);
        put(
            buf,
            x + 1,
            y,
            &[seg(
                crate::client::render::truncate(&p.name, 20),
                Style::default().fg(t.muted).bg(ground),
            )],
            right.saturating_sub(10),
        );
        let (when, when_style) = if urgency(s) == 0 {
            (
                "● you".to_string(),
                Style::default().fg(t.blocked).add_modifier(Modifier::BOLD),
            )
        } else if urgency(s) == 2 {
            (
                format!("done {}", age(s.since)),
                Style::default().fg(t.done),
            )
        } else {
            (age(s.since), Style::default().fg(t.muted))
        };
        let ww = when.width() as u16;
        put(
            buf,
            right.saturating_sub(ww) + 1,
            y,
            &[seg(when, when_style.bg(ground))],
            right + 1,
        );
        let said = s
            .question
            .clone()
            .map(|q| format!("asks: {q}"))
            .unwrap_or_else(|| s.title.clone());
        put(
            buf,
            left + 2,
            y + 1,
            &[seg(
                crate::client::render::truncate(&said, (right.saturating_sub(left + 2)) as usize),
                Style::default()
                    .fg(if urgency(s) == 0 { t.blocked } else { t.muted })
                    .bg(ground),
            )],
            right + 1,
        );
        hit(app, row, ScreenHit::SplashAway(i));
    }

    rule(buf, (r.x, machines_y), r.width, edge);
    title(app, buf, (r.x + 2, machines_y), "MACHINES", false, t);
    let mut x = left;
    let ids: Vec<MachineId> = app.machines().iter().map(|m| m.id).collect();
    for id in ids {
        let (dot, colour) = if id == MachineId::LOCAL {
            ("●", t.done)
        } else {
            app.machine_dot(id, t)
        };
        let word = app
            .machine_mark(id, t)
            .filter(|m| m.color != t.done)
            .map(|m| format!(" {}", m.word))
            .unwrap_or_default();
        let up = word.is_empty();
        let b = pill(app, vec![badge(app, id, up, t)], t.bg);
        let bw = segs_width(&b);
        if x + bw + 4 > right {
            break;
        }
        put(buf, x, machines_y + 2, &b, right);
        let state = vec![seg(
            format!(" {dot}{word}"),
            Style::default().fg(colour).bg(t.bg),
        )];
        let sw = segs_width(&state);
        put(buf, x + bw, machines_y + 2, &state, right);
        x += bw + sw + 3;
    }
}
