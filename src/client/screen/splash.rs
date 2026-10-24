//! The splash screen.

use super::*;

// ---- splash --------------------------------------------------------------------------------

/// VIBECASSO in 7-row pixel letters; `#` is a lit pixel. One empty column sits between letters.
const FONT: [(char, [&str; 7]); 8] = [
    ('V', ["#...#", "#...#", "#...#", "#...#", ".#.#.", ".#.#.", "..#.."]),
    ('I', ["#", "#", "#", "#", "#", "#", "#"]),
    ('B', ["####.", "#...#", "#...#", "#####", "#...#", "#...#", "#####"]),
    ('E', ["#####", "#....", "#....", "####.", "#....", "#....", "#####"]),
    ('C', ["#####", "#....", "#....", "#....", "#....", "#....", "#####"]),
    ('A', ["#####", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
    ('S', ["#####", "#....", "#....", "#####", "....#", "....#", "#####"]),
    ('O', ["#####", "#...#", "#...#", "#...#", "#...#", "#...#", "#####"]),
];
const WORD: &str = "VIBECASSO";
const LOGO_ROWS: u16 = 7;

/// The wordmark as columns of lit rows: `None` is the gap between letters.
fn logo_columns() -> Vec<Option<[bool; 7]>> {
    let mut cols = Vec::new();
    for (i, ch) in WORD.chars().enumerate() {
        if i > 0 {
            cols.push(None);
        }
        let Some((_, glyph)) = FONT.iter().find(|(c, _)| *c == ch) else { continue };
        for x in 0..glyph[0].len() {
            let mut col = [false; 7];
            for (r, row) in glyph.iter().enumerate() {
                col[r] = row.as_bytes()[x] == b'#';
            }
            cols.push(Some(col));
        }
    }
    cols
}

/// Width of the wordmark: two columns per pixel from 120 columns up, one below.
fn logo_width(screen_w: u16) -> u16 {
    logo_columns().len() as u16 * if screen_w >= 120 { 2 } else { 1 }
}

/// The light band crosses the wordmark every 110 frames (5.5 s), after the stroke is done.
const BAND_EVERY: u64 = 110;
/// The band's first pass starts on this frame, once the stroke has crossed the word.
const BAND_FROM: u64 = 40;
/// Columns the brush crosses per frame (the word's 49 columns in about 1.6 s).
const STROKE_SPEED: f32 = 1.5;
/// How many columns behind the brush the paint is still wet (bright).
const WET: f32 = 3.0;
/// A frame count that is past every part of the intro: the finished wordmark.
pub(in crate::client) const SETTLED: u64 = 9999;

/// The wordmark at `(x, y)` as filled cell backgrounds (a `█` leaves seams between rows). `frame` is
/// the intro's frame: a brush paints the word left to right, its edge ragged like bristles and the
/// paint bright where it is still wet, a stroke under the word following it; then a light band
/// passes now and then. `SETTLED` draws it as it ends.
pub(in crate::client) fn draw_logo(buf: &mut Buffer, (x, y): (u16, u16), pw: u16, frame: u64, t: &Theme) {
    let cols = logo_columns();
    let sea = t.sky();
    let last = cols.len().saturating_sub(1).max(1) as f32;
    let brush = frame as f32 * STROKE_SPEED;
    let band = if frame > BAND_FROM { ((frame - BAND_FROM) % BAND_EVERY) as f32 * 0.8 - 6.0 } else { -99.0 };
    let white = Color::Rgb(255, 255, 255);
    for (c, col) in cols.iter().enumerate() {
        let Some(col) = col else { continue };
        let base = blend(t.accent, sea, c as f32 / last);
        for (r, on) in col.iter().enumerate() {
            // Each bristle reaches a pixel a little later than its neighbours.
            let behind = brush - c as f32 - ((c * 3 + r * 5) % 4) as f32 * 0.3;
            if !*on || behind <= 0.0 {
                continue;
            }
            let wet = (1.0 - behind / WET).max(0.0);
            let near = (c as f32 - band).abs();
            let shine = if wet == 0.0 && near < 1.6 { 0.45 * (1.0 - near / 1.6) } else { wet * 0.6 };
            let colour = blend(base, white, shine);
            let px = x + c as u16 * pw;
            for dx in 0..pw {
                if let Some(cell) = buf.cell_mut((px + dx, y + r as u16)) {
                    cell.set_symbol(" ").set_bg(colour);
                }
            }
        }
    }
    // The brush's stroke under the word: it follows the brush, then fades.
    let painted = brush.min(cols.len() as f32).max(0.0) as usize * pw as usize;
    let fade = 1.0 - ((frame as f32 - BAND_FROM as f32) / 10.0).clamp(0.0, 1.0);
    if painted > 0 && fade > 0.0 {
        let line = "▔".repeat(painted);
        put(buf, x, y + LOGO_ROWS, &[seg(line, Style::default().fg(blend(t.bg, sea, fade * 0.6)).bg(t.bg))], buf.area.right());
    }
}

/// The tagline under the wordmark, and its short form for narrow windows.
const TAG: &str = "every session, one calm place · your agents keep going when you leave";
const TAG_SHORT: &str = "every session, one calm place";
/// Narrower than this, the short tagline and no "while you were away" lead-in.
const NARROW: u16 = 100;
/// The widest a choice card gets.
const CARD_MAX: u16 = 64;

/// The splash's choices: (label, key shown, action key). Resume is only there when sessions are
/// still running; a first run has just New session and Open.
pub(in crate::client) fn splash_options(app: &App) -> Vec<(String, String, char)> {
    let mut v = Vec::new();
    if !app.snap.terms.is_empty() {
        v.push(("Resume where you left off".to_string(), "r".to_string(), 'r'));
    }
    v.push(("New session".to_string(), "n".to_string(), 'n'));
    v.push(("Open a project…".to_string(), "o".to_string(), 'o'));
    v
}

/// What a card says under its name: the first line's words (`true` for the strong one) and an
/// optional second line.
type CardText = (Vec<(String, bool)>, Option<String>);

fn card_text(app: &App, model: &[Proj], key: char) -> CardText {
    let dim = |s: String| (s, false);
    match key {
        'r' => {
            let last = app.focused().and_then(|fo| find(model, fo));
            let first = match last {
                Some((p, w, s)) => vec![(p.name.clone(), true), dim(format!(" › {} › {}", w.name, s.title))],
                None => vec![dim("the app as you left it".to_string())],
            };
            let n = app.snap.terms.len();
            (first, Some(format!("{n} session{} still running", if n == 1 { "" } else { "s" })))
        }
        'n' => {
            let agent = app.ns_default_agent();
            let place = match model.get(app.ns_default_project()) {
                Some(p) if app.ns_default_place() == super::newsession::WHERE_WORKTREE => format!("new worktree of {}", p.name),
                Some(p) => format!("in {}", p.name),
                None => "in this folder · you type the task there".to_string(),
            };
            (vec![dim(format!("{agent} · {place}"))], None)
        }
        _ => {
            let n = model.len();
            (vec![dim(if n == 0 { "pick a folder".to_string() } else { format!("{n} recent project{}", if n == 1 { "" } else { "s" }) })], None)
        }
    }
}

/// How much room the splash takes at each spacing: wordmark and its line, tagline, the "away"
/// line, the cards, the hints and, when it fits, the version.
#[derive(Clone, Copy)]
struct Spacing {
    gap: u16,
    between: bool,
    second: bool,
    version: bool,
}

const SPACINGS: [Spacing; 4] = [
    Spacing { gap: 2, between: true, second: true, version: true },
    Spacing { gap: 1, between: true, second: true, version: true },
    Spacing { gap: 1, between: true, second: false, version: false },
    Spacing { gap: 1, between: false, second: false, version: false },
];

fn block_height(o: Spacing, cards: &[CardText]) -> u16 {
    let items: u16 = cards.iter().map(|(_, d2)| 2 + u16::from(o.second && d2.is_some())).sum();
    let spacers = if o.between { cards.len().saturating_sub(1) as u16 } else { 0 };
    LOGO_ROWS + 1 + 1 + o.gap + 1 + o.gap + items + spacers + o.gap + 1 + if o.version { 2 } else { 0 }
}

/// Whether the intro plays: Settings › Appearance › Splash animation, motion on, and for
/// "cold start" a first start of this version or no session left running.
fn plays(app: &App) -> bool {
    app.motion_on()
        && match app.cfg.ui.splash_animation.as_str() {
            "always" => true,
            "never" => false,
            _ => app.splash_cold || app.snap.terms.is_empty(),
        }
}

/// Frames of the intro still to draw: the splash redraws on every tick until then and every
/// frame after for the passing band.
pub(in crate::client) fn splash_animating(app: &App) -> bool {
    app.splash && app.splash_play == Some(true)
}

fn cap(key: &str, on: bool, t: &Theme) -> Vec<Seg> {
    let st = if on { Style::default().bg(t.accent).fg(t.acc_ink) } else { Style::default().bg(t.btn).fg(t.strong) };
    chip(key, st.add_modifier(Modifier::BOLD)).into_iter().collect()
}

pub(in crate::client) fn draw_splash(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) {
    if app.splash_play.is_none() {
        app.splash_play = Some(plays(app));
    }
    let frame = if app.splash_play == Some(true) { app.started.elapsed().as_millis() as u64 / 50 } else { SETTLED };
    let model = app.screen_model();
    let all: Vec<&Session> = model.iter().flat_map(|p| p.sessions()).collect();
    let working = all.iter().filter(|s| s.status == Status::Working).count();
    let needs = all.iter().filter(|s| check::needs_you(s)).count();
    let finished = all.iter().filter(|s| check::finished(s)).count();
    let check_failed = all.iter().filter(|s| check::shown(s).is_some_and(check::failed)).count();
    let opts = splash_options(app);
    let cards: Vec<CardText> = opts.iter().map(|(_, _, c)| card_text(app, &model, *c)).collect();
    let o = SPACINGS.into_iter().find(|o| block_height(*o, &cards) <= area.height).unwrap_or(SPACINGS[3]);
    let mut y = area.y + area.height.saturating_sub(block_height(o, &cards)) / 2;
    let center = |w: u16| area.x + area.width.saturating_sub(w) / 2;
    fill(f.buffer_mut(), area, t.bg);

    let pw = if area.width >= 120 { 2 } else { 1 };
    draw_logo(f.buffer_mut(), (center(logo_width(area.width)), y), pw, frame, t);
    y += LOGO_ROWS + 1;
    let tag = if area.width < NARROW || app.snap.terms.is_empty() { TAG_SHORT } else { TAG };
    let buf = f.buffer_mut();
    put(buf, center(tag.width() as u16), y, &[seg(tag, Style::default().fg(t.muted).bg(t.bg).add_modifier(Modifier::ITALIC))], area.right());
    y += 1 + o.gap;

    let dim = Style::default().fg(t.muted).bg(t.bg);
    let away: Vec<Seg> = if app.snap.terms.is_empty() {
        vec![seg("No sessions yet. Start one and it keeps running after you close Vibecasso.", Style::default().fg(t.text).bg(t.bg))]
    } else if needs + working + finished + check_failed == 0 {
        vec![seg("nothing changed while you were away", dim)]
    } else {
        let gap = || seg("   ", Style::default().bg(t.bg));
        let mut counts = vec![
            seg(format!("{} {needs} need you", app.cfg.icons.blocked), Style::default().fg(t.blocked).bg(t.bg).add_modifier(Modifier::BOLD)),
            gap(),
            seg(format!("{} {working} still working", glyph(app, Status::Working)), Style::default().fg(t.text).bg(t.bg)),
            gap(),
            seg(format!("{} {finished} finished", app.cfg.icons.done), Style::default().fg(t.done).bg(t.bg)),
        ];
        if check_failed > 0 {
            let noun = if check_failed == 1 { "check" } else { "checks" };
            counts.push(gap());
            counts.push(seg(format!("{} {check_failed} {noun} failed", check::failed_mark(app)), Style::default().fg(t.err).bg(t.bg).add_modifier(Modifier::BOLD)));
        }
        // The lead-in gives way before a count is cut off.
        let mut v = vec![seg("while you were away   ", dim)];
        v.extend(counts.iter().cloned());
        if area.width < NARROW || segs_width(&v) > area.width {
            v = counts;
        }
        v
    };
    let buf = f.buffer_mut();
    put(buf, center(segs_width(&away)), y, &away, area.right());
    // The last day, summed, when there is room for a line of its own.
    if o.gap == 2 && let Some(line) = today_line(&app.snap.today, app.snap.spent_today) {
        let segs = vec![seg("today   ", dim), seg(line, Style::default().fg(t.text).bg(t.bg))];
        put(f.buffer_mut(), center(segs_width(&segs)), y + 1, &segs, area.right());
    }
    y += 1 + o.gap;

    // One card per choice; the chosen one is lifted with a bar in the accent.
    let mw = CARD_MAX.min(area.width.saturating_sub(6));
    let mx = center(mw);
    let sel = app.screen.splash_sel.min(opts.len().saturating_sub(1));
    for (i, ((label, key, c), (first, second))) in opts.iter().zip(&cards).enumerate() {
        let on = i == sel;
        let h = 2 + u16::from(o.second && second.is_some());
        let card = Rect { x: mx, y, width: mw, height: h };
        let hov = !on && hovered(app, card);
        let bg = if on { t.card2 } else if hov { t.hov } else { t.bg };
        let buf = f.buffer_mut();
        if on || hov {
            fill(buf, card, bg);
        }
        if on {
            for j in 0..h {
                put(buf, mx, y + j, &[seg("▌", Style::default().fg(t.accent).bg(bg))], mx + 1);
            }
        }
        let st = Style::default().bg(bg);
        put(buf, mx + 3, y, &[seg(label.clone(), st.fg(if on { t.strong } else { t.text }).add_modifier(Modifier::BOLD))], card.right().saturating_sub(2));
        let keys = cap(key, on, t);
        put(buf, card.right().saturating_sub(segs_width(&keys) + 2), y, &keys, card.right());
        let words: Vec<Seg> = first.iter().map(|(w, strong)| if *strong { seg(w.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD)) } else { seg(w.clone(), st.fg(t.muted)) }).collect();
        put(buf, mx + 3, y + 1, &words, card.right().saturating_sub(2));
        if o.second && let Some(d2) = second {
            put(buf, mx + 3, y + 2, &[seg(d2.clone(), st.fg(t.muted))], card.right().saturating_sub(2));
        }
        hit(app, card, ScreenHit::SplashKey(*c));
        y += h + u16::from(o.between);
    }
    y += o.gap - u16::from(o.between);

    let mut hints = Vec::new();
    for (i, (k, what)) in [("↑↓", "choose"), ("Enter", "open"), ("?", "keys")].into_iter().enumerate() {
        if i > 0 {
            hints.push(seg("     ", Style::default().bg(t.bg)));
        }
        hints.extend(crate::client::design::keycap(t, k));
        hints.push(seg(format!(" {what}"), dim));
    }
    put(f.buffer_mut(), center(segs_width(&hints)), y, &hints, area.right());
    hit(app, Rect { x: center(segs_width(&hints)), y, width: segs_width(&hints), height: 1 }, ScreenHit::SplashKey('?'));
    let mut foot = Vec::new();
    if o.version {
        foot.push(seg(format!("vibecasso {}", env!("CARGO_PKG_VERSION")), Style::default().fg(t.line).bg(t.bg)));
    }
    // An update is the one thing the splash adds beside the version: it is how you hear of one.
    let note = if let Some(v) = &app.update_available {
        Some(format!("↑ {v} is out: click Update at the top right"))
    } else if app.restart_needed() {
        Some("↻ a new version is installed: click Restart at the top right".to_string())
    } else {
        None
    };
    if let Some(n) = note {
        if !foot.is_empty() {
            foot.push(seg("   ", Style::default().bg(t.bg)));
        }
        foot.push(seg(n, Style::default().fg(t.accent).bg(t.bg).add_modifier(Modifier::BOLD)));
    }
    if !foot.is_empty() {
        put(f.buffer_mut(), center(segs_width(&foot)), (y + 2).min(area.bottom().saturating_sub(1)), &foot, area.right());
    }
}

/// True the first time this version runs (its number is not what the last start left in the data
/// folder), and leaves the number there. A cold start is the one that plays the intro.
pub(in crate::client) fn first_start_of_this_version() -> bool {
    let file = crate::config::data_dir().join("splash-version");
    let now = env!("CARGO_PKG_VERSION");
    if std::fs::read_to_string(&file).is_ok_and(|v| v.trim() == now) {
        return false;
    }
    if let Err(e) = std::fs::create_dir_all(file.parent().unwrap_or(&file)).and_then(|()| std::fs::write(&file, now)) {
        tracing::warn!("could not record the splash version: {e}");
    }
    true
}

/// "12 turns · 2h 10m working · $3.40" over the last day; None when nothing happened.
pub(in crate::client) fn today_line(day: &[crate::protocol::AgentDay], spent: f64) -> Option<String> {
    let turns: u32 = day.iter().map(|d| d.turns).sum();
    let secs: u64 = day.iter().map(|d| d.working_secs).sum();
    if turns == 0 && spent < 0.01 {
        return None;
    }
    let working = match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m", secs / 60),
        _ => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
    };
    let mut line = format!("{turns} turn{} · {working} working", if turns == 1 { "" } else { "s" });
    if spent >= 0.01 {
        line.push_str(&format!(" · ${spent:.2}"));
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::AgentDay;
    use ratatui::buffer::Buffer;

    fn lit(buf: &Buffer, bg: Color) -> Vec<(u16, u16, Color)> {
        let a = buf.area;
        (a.top()..a.bottom()).flat_map(|y| (a.left()..a.right()).map(move |x| (x, y))).filter(|&(x, y)| buf[(x, y)].bg != bg).map(|(x, y)| (x, y, buf[(x, y)].bg)).collect()
    }

    fn theme() -> Theme {
        crate::theme::Theme::named("vibecasso-dark")
    }

    #[test]
    fn the_settled_wordmark_is_every_pixel_as_a_filled_cell_in_a_gradient() {
        let t = theme();
        let glyph = |ch: char| FONT.iter().find(|(c, _)| *c == ch).expect("every letter of the word has a glyph").1;
        let pixels: usize = WORD.chars().map(|ch| glyph(ch).iter().map(|r| r.matches('#').count()).sum::<usize>()).sum();
        for pw in [1u16, 2] {
            let mut buf = Buffer::empty(Rect::new(0, 0, 120, 20));
            fill(&mut buf, Rect::new(0, 0, 120, 20), t.bg);
            draw_logo(&mut buf, (10, 4), pw, SETTLED, &t);
            let cells = lit(&buf, t.bg);
            let wordmark: Vec<_> = cells.iter().filter(|(_, y, _)| (4..4 + LOGO_ROWS).contains(y)).collect();
            assert_eq!(wordmark.len(), pixels * pw as usize, "pw {pw}: one filled cell per pixel (two wide), no seams");
            let (left, right) = (wordmark.iter().min_by_key(|(x, ..)| *x).unwrap().2, wordmark.iter().max_by_key(|(x, ..)| *x).unwrap().2);
            assert_ne!(left, right, "the colour runs from the accent toward the sea");
        }
    }

    #[test]
    fn the_word_spells_vibecasso_and_fits_eighty_columns() {
        assert_eq!(WORD, "VIBECASSO");
        assert!(WORD.chars().all(|ch| FONT.iter().any(|(c, _)| *c == ch)), "a glyph for every letter");
        assert!(logo_width(80) <= 78 && logo_width(160) <= 158, "{} / {}", logo_width(80), logo_width(160));
    }

    #[test]
    fn a_brush_paints_the_word_left_to_right() {
        let t = theme();
        let painted = |frame| {
            let mut buf = Buffer::empty(Rect::new(0, 0, 80, 30));
            fill(&mut buf, Rect::new(0, 0, 80, 30), t.bg);
            draw_logo(&mut buf, (10, 10), 1, frame, &t);
            lit(&buf, t.bg).into_iter().filter(|(_, y, _)| (10..10 + LOGO_ROWS).contains(y)).map(|(x, ..)| x).collect::<Vec<_>>()
        };
        let mid = painted(15);
        assert!(!mid.is_empty() && mid.len() < painted(SETTLED).len(), "part way across");
        let furthest = *mid.iter().max().unwrap();
        assert!(furthest < 10 + 49 - 5, "only the left of the word is painted yet: up to column {furthest}");
        assert!(painted(25).len() > mid.len(), "and it goes on to the right");
    }

    #[test]
    fn the_intro_starts_empty_paints_in_and_ends_where_it_is_settled() {
        let t = theme();
        let drawn = |frame| {
            let mut buf = Buffer::empty(Rect::new(0, 0, 80, 30));
            fill(&mut buf, Rect::new(0, 0, 80, 30), t.bg);
            draw_logo(&mut buf, (10, 10), 1, frame, &t);
            lit(&buf, t.bg).len()
        };
        assert_eq!(drawn(0), 0, "nothing at frame 0");
        assert!(drawn(8) > 0 && drawn(8) < drawn(SETTLED), "the brush is painting");
        let mut a = Buffer::empty(Rect::new(0, 0, 80, 30));
        let mut b = a.clone();
        for buf in [&mut a, &mut b] {
            fill(buf, Rect::new(0, 0, 80, 30), t.bg);
        }
        draw_logo(&mut a, (10, 10), 1, 60, &t);
        draw_logo(&mut b, (10, 10), 1, SETTLED, &t);
        let wordmark = |buf: &Buffer| lit(buf, t.bg).into_iter().filter(|(_, y, _)| (10..10 + LOGO_ROWS).contains(y)).map(|(x, y, _)| (x, y)).collect::<Vec<_>>();
        assert_eq!(wordmark(&a), wordmark(&b), "once landed it sits on the same cells (the band only lightens them)");
    }

    #[test]
    fn the_day_in_one_line() {
        let d = |turns, working_secs| AgentDay { agent: "claude".into(), place: "shop".into(), turns, working_secs, cost: 0.0 };
        assert_eq!(today_line(&[d(9, 5400), d(3, 2400)], 3.4).as_deref(), Some("12 turns · 2h 10m working · $3.40"));
        assert_eq!(today_line(&[d(1, 90)], 0.0).as_deref(), Some("1 turn · 1m working"));
        assert_eq!(today_line(&[], 0.0), None, "nothing happened: no line");
    }
}
