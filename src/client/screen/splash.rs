//! The splash screen: the wordmark rising from deep water, a swell under it, the tagline, and
//! the choices (see docs/UI.md, splash).

use super::*;
use crate::theme::mix;

/// 5-row block letters for the app's name, whatever it is (ADR 0019): A–Z, 0–9 and a space;
/// `#` is a lit pixel. One empty column sits between letters; anything else is left out.
const FONT: [(char, [&str; 5]); 37] = [
    ('A', [".##.", "#..#", "####", "#..#", "#..#"]),
    ('B', ["###.", "#..#", "###.", "#..#", "###."]),
    ('C', [".###", "#...", "#...", "#...", ".###"]),
    ('D', ["###.", "#..#", "#..#", "#..#", "###."]),
    ('E', ["####", "#...", "###.", "#...", "####"]),
    ('F', ["####", "#...", "###.", "#...", "#..."]),
    ('G', [".###", "#...", "#.##", "#..#", ".###"]),
    ('H', ["#..#", "#..#", "####", "#..#", "#..#"]),
    ('I', ["###", ".#.", ".#.", ".#.", "###"]),
    ('J', ["..##", "...#", "...#", "#..#", ".##."]),
    ('K', ["#..#", "#.#.", "##..", "#.#.", "#..#"]),
    ('L', ["#...", "#...", "#...", "#...", "####"]),
    ('M', ["#...#", "##.##", "#.#.#", "#...#", "#...#"]),
    ('N', ["#...#", "##..#", "#.#.#", "#..##", "#...#"]),
    ('O', [".##.", "#..#", "#..#", "#..#", ".##."]),
    ('P', ["###.", "#..#", "###.", "#...", "#..."]),
    ('Q', [".##.", "#..#", "#..#", "#.#.", ".#.#"]),
    ('R', ["###.", "#..#", "###.", "#.#.", "#..#"]),
    ('S', [".###", "#...", ".##.", "...#", "###."]),
    ('T', ["#####", "..#..", "..#..", "..#..", "..#.."]),
    ('U', ["#..#", "#..#", "#..#", "#..#", ".##."]),
    ('V', ["#...#", "#...#", "#...#", ".#.#.", "..#.."]),
    ('W', ["#...#", "#...#", "#.#.#", "##.##", "#...#"]),
    ('X', ["#...#", ".#.#.", "..#..", ".#.#.", "#...#"]),
    ('Y', ["#...#", ".#.#.", "..#..", "..#..", "..#.."]),
    ('Z', ["####", "...#", ".##.", "#...", "####"]),
    ('0', [".##.", "#.##", "##.#", "#..#", ".##."]),
    ('1', [".#.", "##.", ".#.", ".#.", "###"]),
    ('2', ["###.", "...#", ".##.", "#...", "####"]),
    ('3', ["###.", "...#", ".##.", "...#", "###."]),
    ('4', ["#..#", "#..#", "####", "...#", "...#"]),
    ('5', ["####", "#...", "###.", "...#", "###."]),
    ('6', [".##.", "#...", "###.", "#..#", ".##."]),
    ('7', ["####", "...#", "..#.", ".#..", ".#.."]),
    ('8', [".##.", "#..#", ".##.", "#..#", ".##."]),
    ('9', [".##.", "#..#", ".###", "...#", ".##."]),
    (' ', ["..", "..", "..", "..", ".."]),
];
const LOGO_ROWS: u16 = 5;
/// From this width each font pixel is two cells wide.
const WIDE_PIXELS_FROM: u16 = 110;
/// Below this height the splash tightens: Resume loses its second line, cards lose their gaps.
const COMPACT_BELOW: u16 = 30;
/// Narrower than this, the "while you were away" lead-in gives way to the counts alone.
const NARROW: u16 = 100;
/// The widest a choice card gets.
const CARD_MAX: u16 = 56;
/// The swell reaches this far past the wordmark on each side, and fades out over `SWELL_FADE`
/// cells at each end.
const SWELL_REACH: u16 = 4;
const SWELL_FADE: f32 = 6.0;
const SWELL: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▄", "▃", "▂"];
/// The swell moves one step every this many frames (a frame is 50 ms), and fades in over
/// `SWELL_FADE_IN` frames.
const SWELL_STEP: u64 = 3;
const SWELL_FADE_IN: f32 = 14.0;
/// Letter k starts rising on frame `RISE_STAGGER * k` and takes `RISE_FRAMES`.
const RISE_STAGGER: f32 = 3.0;
const RISE_FRAMES: f32 = 8.0;
/// The tagline fades in over these frames.
const TAG_FROM: f32 = 10.0;
const TAG_FRAMES: f32 = 10.0;
/// A frame count that is past every part of the intro: the finished splash.
pub(in crate::client) const SETTLED: u64 = 9999;

/// The word the splash spells: the app's name in capitals.
fn word() -> String {
    crate::brand::APP_NAME.to_uppercase()
}

/// The letters drawn in the second colour: the first "AI" the name hides, if it hides one.
fn ai_letters() -> std::ops::Range<usize> {
    let w: Vec<char> = word().chars().collect();
    w.windows(2).position(|p| p == ['A', 'I']).map_or(0..0, |i| i..i + 2)
}

fn letter(ch: char) -> Option<&'static [&'static str; 5]> {
    FONT.iter().find(|(c, _)| *c == ch).map(|(_, g)| g)
}

/// Font columns the wordmark spans, gaps included.
fn logo_columns() -> u16 {
    let widths: Vec<u16> = word().chars().filter_map(letter).map(|g| g[0].len() as u16).collect();
    widths.iter().sum::<u16>() + widths.len().saturating_sub(1) as u16
}

/// Cells per font pixel at this width.
fn pixel_width(screen_w: u16) -> u16 {
    if screen_w >= WIDE_PIXELS_FROM && logo_columns() * 2 + 2 <= screen_w { 2 } else { 1 }
}

fn logo_width(screen_w: u16) -> u16 {
    logo_columns() * pixel_width(screen_w)
}

/// Whether the wordmark fits at all; a name too long for the window is written out instead.
fn logo_fits(screen_w: u16) -> bool {
    logo_width(screen_w) + 2 <= screen_w
}

/// 0 → 1 over `frames` frames from `from`, eased out (fast, then settling).
fn ease_out(t: f32, from: f32, frames: f32) -> f32 {
    let a = ((t - from) / frames).clamp(0.0, 1.0);
    1.0 - (1.0 - a).powi(3)
}

/// The wordmark at `(x, y)` as filled cell backgrounds (a `█` leaves seams between rows). Each
/// letter rises from the deep to its colour, left to right; nothing moves. `t` is the intro's
/// frame, `SETTLED` its end.
pub(in crate::client) fn draw_logo(buf: &mut Buffer, (mut x, y): (u16, u16), pw: u16, t: u64, th: &Theme) {
    let c = th.splash();
    let ai = ai_letters();
    for (k, ch) in word().chars().enumerate() {
        let Some(g) = letter(ch) else { continue };
        let lit = ease_out(t as f32, k as f32 * RISE_STAGGER, RISE_FRAMES);
        let colour = mix(c.deep, if ai.contains(&k) { c.wordmark_ai } else { c.wordmark }, lit);
        for (r, row) in g.iter().enumerate() {
            for (col, b) in row.bytes().enumerate() {
                if b != b'#' {
                    continue;
                }
                for dx in 0..pw {
                    if let Some(cell) = buf.cell_mut((x + col as u16 * pw + dx, y + r as u16)) {
                        cell.set_symbol(" ").set_bg(colour);
                    }
                }
            }
        }
        x += (g[0].len() as u16 + 1) * pw;
    }
}

/// The swell: one rolling line `w` wide, fading into the ground at both ends. `roll` is the
/// frame it rolls by (0 holds it still), `t` the intro's frame.
fn draw_swell(buf: &mut Buffer, (x, y): (u16, u16), w: u16, roll: u64, t: u64, th: &Theme) {
    let shown = (t as f32 / SWELL_FADE_IN).min(1.0);
    for k in 0..w {
        let edge = (k.min(w - 1 - k) as f32 / SWELL_FADE).min(1.0);
        let step = (k as u64 + roll / SWELL_STEP) as usize % SWELL.len();
        let style = Style::default().fg(mix(th.bg, th.splash().swell, edge * shown)).bg(th.bg);
        put(buf, x + k, y, &[seg(SWELL[step], style)], x + k + 1);
    }
}

/// The splash's choices: (label, key, action key). Resume is only there when sessions are still
/// running; a first run has just New session and Open.
pub(in crate::client) fn splash_options(app: &App) -> Vec<(String, String, char)> {
    let mut v = Vec::new();
    if !app.snap.terms.is_empty() {
        v.push(("Resume where you left off".to_string(), "r".to_string(), 'r'));
    }
    v.push(("New session".to_string(), "n".to_string(), 'n'));
    v.push(("Open a project…".to_string(), "o".to_string(), 'o'));
    v
}

/// Resume's second line: where you were.
fn last_place(app: &App, model: &[Proj]) -> String {
    match app.focused().and_then(|fo| find(model, fo)) {
        Some((p, _, s)) => format!("{} › {}", p.name, s.title),
        None => "the app as you left it".to_string(),
    }
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

/// The splash redraws every tick while motion is on: the intro, then the swell rolling.
pub(in crate::client) fn splash_animating(app: &App) -> bool {
    app.splash && app.motion_on()
}

/// What happened while you were away ("2 need you   3 still working   1 finished"), or
/// nothing when nothing did.
fn away_line(app: &App, model: &[Proj], t: &Theme, width: u16) -> Vec<Seg> {
    let all: Vec<&Session> = model.iter().flat_map(|p| p.sessions()).collect();
    let working = all.iter().filter(|s| s.status == Status::Working).count();
    let needs = all.iter().filter(|s| check::needs_you(s)).count();
    let finished = all.iter().filter(|s| check::finished(s)).count();
    let failed = all.iter().filter(|s| check::shown(s).is_some_and(check::failed)).count();
    if needs + working + finished + failed == 0 {
        return Vec::new();
    }
    let gap = || seg("   ", Style::default().bg(t.bg));
    let mut counts = vec![
        seg(format!("{} {needs} need you", app.cfg.icons.blocked), Style::default().fg(t.blocked).bg(t.bg).add_modifier(Modifier::BOLD)),
        gap(),
        seg(format!("{} {working} still working", glyph(app, Status::Working)), Style::default().fg(t.text).bg(t.bg)),
        gap(),
        seg(format!("{} {finished} finished", app.cfg.icons.done), Style::default().fg(t.done).bg(t.bg)),
    ];
    if failed > 0 {
        let noun = if failed == 1 { "check" } else { "checks" };
        counts.push(gap());
        counts.push(seg(format!("{} {failed} {noun} failed", check::failed_mark(app)), Style::default().fg(t.err).bg(t.bg).add_modifier(Modifier::BOLD)));
    }
    // The lead-in gives way before a count is cut off.
    let mut v = vec![seg("while you were away   ", Style::default().fg(t.muted).bg(t.bg))];
    v.extend(counts.iter().cloned());
    if width < NARROW || segs_width(&v) > width { counts } else { v }
}

/// A key on a card: the accent when the card is chosen.
fn cap(key: &str, on: bool, t: &Theme) -> Vec<Seg> {
    let st = if on { Style::default().bg(t.accent).fg(t.acc_ink) } else { Style::default().bg(t.btn).fg(t.strong) };
    chip(key, st.add_modifier(Modifier::BOLD))
}

/// The version, barely there.
fn foot(t: &Theme) -> Seg {
    seg(format!(concat!(env!("APP_COMMAND"), " {}"), env!("CARGO_PKG_VERSION")), Style::default().fg(t.line).bg(t.bg))
}

/// A newer release, or one installed and waiting for a restart: what the splash's banner says,
/// and what `u` and a click on it do.
fn release_news(app: &App) -> Option<(String, &'static str, ScreenHit)> {
    if let Some(v) = &app.update_available {
        Some((format!(concat!("↑ ", env!("APP_COMMAND"), " {} is out"), v), "update", ScreenHit::Update))
    } else if app.restart_needed() {
        Some(("↻ a new version is installed".to_string(), "restart", ScreenHit::Restart))
    } else {
        None
    }
}

/// Rows the release banner takes: its box and a blank row under it.
const BANNER_ROWS: u16 = 4;

/// The release banner at `r`: boxed as the look boxes a pane (ADR 0028), the news in the accent
/// and `u` with what it does at the right; a click on it does the same.
fn draw_banner(app: &mut App, buf: &mut Buffer, r: Rect, (news, what, hit_as): (String, &str, ScreenHit), t: &Theme) {
    let look = Look::of(&app.cfg.ui);
    let ground = t.card;
    fill(buf, r, ground);
    panel_box(buf, r, look, Style::default().fg(t.accent).bg(ground));
    let y = r.y + 1;
    let mut key = crate::client::design::keycap(t, "u");
    key.push(seg(format!(" {what}"), Style::default().fg(t.text).bg(ground)));
    let key_x = r.right().saturating_sub(segs_width(&key) + 2);
    put(buf, r.x + 2, y, &[seg(news, Style::default().fg(t.accent).bg(ground).add_modifier(Modifier::BOLD))], key_x.saturating_sub(1));
    put(buf, key_x, y, &key, r.right().saturating_sub(1));
    hit(app, r, hit_as);
}

pub(in crate::client) fn draw_splash(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) {
    if app.splash_play.is_none() {
        app.splash_play = Some(plays(app));
    }
    let clock = if app.motion_on() { app.started.elapsed().as_millis() as u64 / 50 } else { 0 };
    let intro = if app.splash_play == Some(true) { clock } else { SETTLED };
    let model = app.screen_model();
    let opts = splash_options(app);
    let compact = area.height < COMPACT_BELOW;
    let away = away_line(app, &model, t, area.width);
    let resume_rows = if compact { 1 } else { 2 };
    let rows_of = |c: char| if c == 'r' { resume_rows } else { 1 };
    let between = u16::from(!compact);
    let before_cards = if compact { 1 } else { 2 };
    let away_rows = if away.is_empty() { 0 } else { 2 };
    let news = release_news(app);
    let banner_rows = if news.is_some() { BANNER_ROWS } else { 0 };
    let cards: u16 = opts.iter().map(|o| rows_of(o.2)).sum::<u16>() + between * opts.len().saturating_sub(1) as u16;
    let height = LOGO_ROWS + 1 + 2 + 1 + away_rows + before_cards + banner_rows + cards + 1 + 1;
    let mut y = area.y + (area.height.saturating_sub(height) / 2).saturating_sub(between).max(1);
    let center = |w: u16| area.x + area.width.saturating_sub(w) / 2;
    fill(f.buffer_mut(), area, t.bg);

    if logo_fits(area.width) {
        draw_logo(f.buffer_mut(), (center(logo_width(area.width)), y), pixel_width(area.width), intro, t);
    } else {
        let name = crate::brand::APP_NAME;
        let style = Style::default().fg(t.splash().wordmark).bg(t.bg).add_modifier(Modifier::BOLD);
        put(f.buffer_mut(), center(name.width() as u16), y + LOGO_ROWS / 2, &[seg(name, style)], area.right());
    }
    y += LOGO_ROWS + 1;
    let swell_w = (logo_width(area.width) + SWELL_REACH * 2).min(area.width);
    draw_swell(f.buffer_mut(), (center(swell_w), y), swell_w, clock, intro, t);
    y += 2;
    let tag = concat!(env!("APP_TAGLINE"), ".");
    let shown = ((intro as f32 - TAG_FROM) / TAG_FRAMES).clamp(0.0, 1.0);
    let style = Style::default().fg(mix(t.bg, t.text, shown)).bg(t.bg).add_modifier(Modifier::ITALIC);
    put(f.buffer_mut(), center(tag.width() as u16), y, &[seg(tag, style)], area.right());
    y += 1;
    if !away.is_empty() {
        put(f.buffer_mut(), center(segs_width(&away)), y + 1, &away, area.right());
        y += away_rows;
    }
    y += before_cards;
    // A release waiting: over the cards, where it can't be missed.
    if let Some(news) = news {
        let mw = CARD_MAX.min(area.width.saturating_sub(8));
        draw_banner(app, f.buffer_mut(), Rect { x: center(mw), y, width: mw, height: 3 }, news, t);
        y += banner_rows;
    }

    // One card per choice; the chosen one is lifted, with a bar in the accent down its edge.
    let mw = CARD_MAX.min(area.width.saturating_sub(8));
    let mx = center(mw);
    let sel = app.screen.splash_sel.min(opts.len().saturating_sub(1));
    for (i, (label, key, c)) in opts.iter().enumerate() {
        let on = i == sel;
        let rows = rows_of(*c);
        let card = Rect { x: mx, y, width: mw, height: rows };
        let bg = if on { t.card2 } else { t.bg };
        let buf = f.buffer_mut();
        fill(buf, card, bg);
        if on {
            for j in 0..rows {
                put(buf, mx, y + j, &[seg("▌", Style::default().fg(t.accent).bg(bg))], mx + 1);
            }
        }
        let label_style = if on { Style::default().fg(t.strong).bg(bg).add_modifier(Modifier::BOLD) } else { Style::default().fg(t.text).bg(bg) };
        let keys = cap(key, on, t);
        let key_x = card.right().saturating_sub(segs_width(&keys) + 1);
        put(buf, mx + 2, y, &[seg(label.clone(), label_style)], key_x.saturating_sub(1));
        put(buf, key_x, y, &keys, card.right());
        if rows == 2 {
            put(buf, mx + 2, y + 1, &[seg(last_place(app, &model), Style::default().fg(t.muted).bg(bg))], card.right().saturating_sub(2));
        }
        hit(app, card, ScreenHit::SplashCard(i));
        y += rows + between;
    }
    y += 1 - between;

    let mut hints = Vec::new();
    for (i, (k, what)) in [("↑↓", "choose"), ("Enter", "open"), ("?", "keys")].into_iter().enumerate() {
        if i > 0 {
            hints.push(seg("   ", Style::default().bg(t.bg)));
        }
        hints.extend(crate::client::design::keycap(t, k));
        hints.push(seg(format!(" {what}"), Style::default().fg(t.muted).bg(t.bg)));
    }
    let hx = center(segs_width(&hints));
    put(f.buffer_mut(), hx, y, &hints, area.right());
    hit(app, Rect { x: hx, y, width: segs_width(&hints), height: 1 }, ScreenHit::SplashKey('?'));

    let foot = foot(t);
    let fw = segs_width(std::slice::from_ref(&foot));
    put(f.buffer_mut(), area.right().saturating_sub(fw + 2), area.bottom().saturating_sub(1), &[foot], area.right());
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    fn lit(buf: &Buffer, bg: Color) -> Vec<(u16, u16, Color)> {
        let a = buf.area;
        (a.top()..a.bottom()).flat_map(|y| (a.left()..a.right()).map(move |x| (x, y))).filter(|&(x, y)| buf[(x, y)].bg != bg).map(|(x, y)| (x, y, buf[(x, y)].bg)).collect()
    }

    fn theme() -> Theme {
        crate::theme::Theme::named("night")
    }

    fn wordmark(frame: u64, pw: u16) -> Vec<(u16, u16, Color)> {
        let t = theme();
        let area = Rect::new(0, 0, logo_columns() * 2 + 20, 12);
        let mut buf = Buffer::empty(area);
        fill(&mut buf, area, t.bg);
        draw_logo(&mut buf, (10, 4), pw, frame, &t);
        lit(&buf, t.bg)
    }

    #[test]
    fn the_settled_wordmark_is_every_pixel_as_a_filled_cell() {
        let pixels: usize = word().chars().filter_map(letter).map(|g| g.iter().map(|r| r.matches('#').count()).sum::<usize>()).sum();
        for pw in [1u16, 2] {
            let cells = wordmark(SETTLED, pw);
            assert_eq!(cells.len(), pixels * pw as usize, "pw {pw}: one filled cell per pixel, no seams");
            assert!(cells.iter().all(|(_, y, _)| (4..4 + LOGO_ROWS).contains(y)), "five rows tall");
        }
    }

    #[test]
    fn the_ai_the_name_hides_is_in_its_own_colour() {
        let c = theme().splash();
        let colours: Vec<Color> = wordmark(SETTLED, 1).into_iter().map(|(.., col)| col).collect();
        assert!(colours.iter().all(|col| [c.wordmark, c.wordmark_ai].contains(col)));
        assert_eq!(colours.contains(&c.wordmark_ai), !ai_letters().is_empty(), "the A and I stand out when the name has them");
    }

    #[test]
    fn the_letters_rise_from_the_deep_left_to_right_without_moving() {
        let deep = theme().splash().deep;
        let at = |frame| wordmark(frame, 1);
        let place = |cells: Vec<(u16, u16, Color)>| cells.into_iter().map(|(x, y, _)| (x, y)).collect::<Vec<_>>();
        assert_eq!(place(at(0)), place(at(SETTLED)), "every pixel is in place from the first frame");
        assert!(at(0).iter().all(|(.., c)| *c == deep), "frame 0: all deep water");
        let mid = at(5);
        let first = mid.iter().min_by_key(|(x, ..)| *x).unwrap().2;
        let last = mid.iter().max_by_key(|(x, ..)| *x).unwrap().2;
        assert!(first != deep && last == deep, "the first letter has risen, the last not yet");
    }

    #[test]
    fn the_font_spells_any_name_and_the_wordmark_never_overflows() {
        for ch in ('A'..='Z').chain('0'..='9') {
            let g = letter(ch).unwrap_or_else(|| panic!("no glyph for {ch}"));
            assert!(g.iter().all(|row| row.len() == g[0].len()), "{ch}: every row the same width");
        }
        for w in [60, 80, 109, 110, 160] {
            assert!(!logo_fits(w) || logo_width(w) + 2 <= w, "{w} columns: the wordmark fits with a margin");
        }
        assert_eq!(pixel_width(109), 1);
        assert_eq!(pixel_width(110), 2, "two-cell pixels from 110 columns");
    }

    #[test]
    fn the_swell_fades_at_both_ends() {
        let t = theme();
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
        draw_swell(&mut buf, (0, 0), 40, 0, SETTLED, &t);
        assert_eq!(buf[(0, 0)].fg, t.bg, "the ends sink into the ground");
        assert_eq!(buf[(20, 0)].fg, t.splash().swell, "the middle is the sea");
        assert_eq!(buf[(0, 0)].symbol(), "▁");
    }
}
