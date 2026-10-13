//! A sleeping agent's pane as it wakes: rain fills it, dissolves from the centre outwards
//! while the agent starts, then its screen comes back a line at a time. Only the inside of
//! the pane is drawn here; its border, title and footer stay as they are.
//!
//! Timings are in milliseconds from the wake, so the frame rate doesn't change them.

use super::card::{Look, pill};
use crate::client::design::{Seg, fill, put, seg, segs_width};
use crate::client::render::blend;
use crate::theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::time::Instant;

/// Every column rains at full speed until here.
const RAIN_MS: f32 = 900.0;
/// How long the dissolve takes when the agent is back in time.
const DISSOLVE_MS: f32 = 1500.0;
/// The dissolve ends here at the soonest.
const DISSOLVE_END_MS: f32 = 2600.0;
/// While the agent isn't back the dissolve holds here, and the rain goes on in what's left.
const HELD_AT: f32 = 0.85;
/// Once a held dissolve's agent is back, it finishes over this long.
const HELD_FINISH_MS: f32 = 300.0;
/// The rain falls at this share of its speed while dissolving.
const DISSOLVE_SPEED: f32 = 0.6;
/// One line of the screen comes back every this long.
const LINE_MS: f32 = 55.0;
/// The "awake" pill fades over this long.
const AWAKE_FADE_MS: f32 = 900.0;
/// The screen fades in over this long when there's no rain (`WakeStyle::Simple`).
const SIMPLE_FADE_MS: f32 = 300.0;
/// The progress bar: its width, and when it's full.
const BAR_CELLS: usize = 24;
const BAR_FULL_MS: f32 = 2400.0;
/// The step label says the agent is reattaching until here (or until it's back, if later).
const REATTACH_SAID_UNTIL_MS: f32 = 1800.0;
/// The spinner's frames, and how long each shows.
const SPINNER: [&str; 8] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
const SPINNER_MS: f32 = 80.0;
/// The rain's numbers are per frame at this rate; a slower window takes bigger steps.
const RAIN_FRAME_MS: f32 = 1000.0 / 60.0;
/// A window that stalled doesn't make the rain jump further than this many frames.
const RAIN_STEPS_MOST: f32 = 8.0;
/// A cell's brightness each frame, and the brightness under which it's blank.
const TRAIL_KEEPS: f32 = 0.955;
const TRAIL_GONE: f32 = 0.04;
/// Each frame a lit cell may change glyph.
const SHIMMER: f32 = 0.02;
/// A cell this close to dropping out is drawn dim, so it flickers out rather than pops.
const DYING_WITHIN: f32 = 0.08;
/// Half-width katakana, digits and marks: every one a single cell.
const GLYPHS: &str = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ0123456789:.=*+<>¦";

/// How much of the transition plays (Settings: Wake-up animation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::client) enum WakeStyle {
    /// Rain, dissolve, then the screen line by line.
    Full,
    /// No rain: the note while it starts, then the screen fades in.
    Simple,
    /// The note while it starts, then the screen.
    Off,
}

impl WakeStyle {
    /// From the setting; with motion off (or over SSH) the full one is the simple one.
    pub fn of(setting: &str, motion: bool) -> WakeStyle {
        match setting {
            "off" => WakeStyle::Off,
            "on" if motion => WakeStyle::Full,
            _ => WakeStyle::Simple,
        }
    }
}

/// A small generator of its own: the rain needs thousands of numbers a frame and nothing
/// about them matters but that they vary.
#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// 0 up to (not including) 1.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[derive(Debug, Clone)]
struct Stream {
    /// Its head's row (above the top while negative).
    y: f32,
    /// Rows per frame.
    speed: f32,
    len: f32,
}

#[derive(Debug, Clone, Copy)]
struct Drop {
    glyph: char,
    bright: f32,
    /// The stream's head entered it this frame.
    head: bool,
    /// How far through the dissolve it drops out: the centre first, the edges last.
    dies_at: f32,
}

#[derive(Debug, Clone)]
struct Rain {
    w: usize,
    h: usize,
    streams: Vec<Stream>,
    drops: Vec<Drop>,
    glyphs: Vec<char>,
    rng: Rng,
}

impl Rain {
    fn new(w: usize, h: usize, seed: u64) -> Rain {
        let mut rng = Rng(seed | 1);
        let glyphs: Vec<char> = GLYPHS.chars().collect();
        let streams = (0..w).map(|_| Stream { y: -(rng.below(h.max(1)) as f32), speed: stream_speed(&mut rng), len: 6.0 + rng.below(18) as f32 }).collect();
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let drops = (0..w * h)
            .map(|i| {
                let (dx, dy) = (((i % w) as f32 - cx) / cx.max(1.0), ((i / w) as f32 - cy) / cy.max(1.0));
                let dist = (dx * dx + dy * dy).sqrt().min(1.0);
                Drop { glyph: glyphs[rng.below(glyphs.len())], bright: 0.0, head: false, dies_at: 0.15 + dist * 0.55 + rng.unit() * 0.3 }
            })
            .collect();
        Rain { w, h, streams, drops, glyphs, rng }
    }

    /// Move everything on by `frames` (at 60 a second), the streams at `speed` of their own.
    fn step(&mut self, frames: f32, speed: f32) {
        let keeps = TRAIL_KEEPS.powf(frames);
        let shimmer = 1.0 - (1.0 - SHIMMER).powf(frames);
        for d in &mut self.drops {
            d.bright *= keeps;
            d.head = false;
            if d.bright >= TRAIL_GONE && self.rng.unit() < shimmer {
                d.glyph = self.glyphs[self.rng.below(self.glyphs.len())];
            }
        }
        for x in 0..self.w {
            let s = &mut self.streams[x];
            let from = s.y.floor() as i64;
            s.y += s.speed * frames * speed;
            let to = s.y.floor() as i64;
            // Every row the head passed through this frame is lit; the one it's in is the head.
            for row in (from + 1).max(0)..=to.min(self.h as i64 - 1) {
                let d = &mut self.drops[row as usize * self.w + x];
                (d.glyph, d.bright, d.head) = (self.glyphs[self.rng.below(self.glyphs.len())], 1.0, row == to);
            }
            if s.y - s.len > self.h as f32 {
                (s.y, s.speed) = (-(self.rng.below(8) as f32), stream_speed(&mut self.rng));
            }
        }
    }
}

fn stream_speed(rng: &mut Rng) -> f32 {
    0.35 + rng.unit() * 0.9
}

/// One pane's wake, from the moment its new pane appeared.
#[derive(Debug, Clone)]
pub(in crate::client) struct Wake {
    pub started: Instant,
    /// When its agent was back (its hooks reported, or it drew its screen).
    pub ready: Option<Instant>,
    kind: WakeStyle,
    seed: u64,
    rain: Option<Rain>,
    stepped: Instant,
    /// How many of the screen's rows have something on them: what there is to bring back.
    rows: u16,
}

/// What of the transition is on screen at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// The agent isn't back and there's no rain: just the note.
    Waiting,
    /// Rain, dissolved this far (0: not yet), over `ms` so far.
    Rain { dissolved: f32, ms: f32 },
    /// The screen coming back: this many ms in.
    Restore { ms: f32 },
    /// The screen fading in: this far (0 to 1).
    Fade(f32),
    Done,
}

impl Wake {
    pub fn new(now: Instant, kind: WakeStyle, seed: u64) -> Wake {
        Wake { started: now, ready: None, kind, seed, rain: None, stepped: now, rows: 0 }
    }

    fn ms(&self, at: Instant) -> f32 {
        at.saturating_duration_since(self.started).as_secs_f32() * 1000.0
    }

    /// When the dissolve is over, once the agent is back: on time, or a moment after a late
    /// agent arrives.
    fn dissolve_end(&self) -> Option<f32> {
        let ready = self.ms(self.ready?);
        Some(if ready <= RAIN_MS + HELD_AT * DISSOLVE_MS { DISSOLVE_END_MS } else { ready + HELD_FINISH_MS })
    }

    fn phase(&self, now: Instant) -> Phase {
        let ms = self.ms(now);
        let since_ready = self.ready.map(|r| now.saturating_duration_since(r).as_secs_f32() * 1000.0);
        match (self.kind, since_ready) {
            (WakeStyle::Off, Some(_)) => Phase::Done,
            (WakeStyle::Simple, Some(since)) if since >= SIMPLE_FADE_MS => Phase::Done,
            (WakeStyle::Simple, Some(since)) => Phase::Fade(since / SIMPLE_FADE_MS),
            (WakeStyle::Off | WakeStyle::Simple, None) => Phase::Waiting,
            (WakeStyle::Full, _) => match self.dissolve_end() {
                Some(end) if ms >= end => {
                    let into = ms - end;
                    if into >= (f32::from(self.rows) * LINE_MS).max(AWAKE_FADE_MS) { Phase::Done } else { Phase::Restore { ms: into } }
                }
                end => {
                    let on_time = ((ms - RAIN_MS) / DISSOLVE_MS).clamp(0.0, 1.0);
                    let dissolved = match (end, since_ready) {
                        // A late agent: held, then the rest of the way once it's here.
                        (Some(end), Some(since)) if end > DISSOLVE_END_MS => HELD_AT + (1.0 - HELD_AT) * (since / HELD_FINISH_MS).clamp(0.0, 1.0),
                        (Some(_), _) => on_time,
                        (None, _) => on_time.min(HELD_AT),
                    };
                    Phase::Rain { dissolved, ms }
                }
            },
        }
    }

    /// Over: the pane is an ordinary pane again.
    pub fn done(&self, now: Instant) -> bool {
        self.phase(now) == Phase::Done
    }

    /// Something moves every frame (rather than only the spinner turning).
    pub fn animating(&self, now: Instant) -> bool {
        !matches!(self.phase(now), Phase::Waiting | Phase::Done)
    }
}

/// What the transition draws with, from the theme.
pub(in crate::client) struct Ink<'a> {
    pub theme: &'a Theme,
    pub look: Look,
    /// The pane's own ground.
    pub ground: Color,
    /// Who is coming back, for the step label ("opus 4.5", or the agent's name).
    pub who: &'a str,
}

/// Draw `wake` over `inner`, which already holds the agent's screen as it is now.
pub(in crate::client) fn draw(buf: &mut Buffer, inner: Rect, wake: &mut Wake, now: Instant, ink: &Ink) {
    let inner = inner.intersection(buf.area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    wake.rows = rows_used(buf, inner);
    let t = ink.theme;
    let mid = inner.y + (inner.height / 2).saturating_sub(1);
    match wake.phase(now) {
        Phase::Done => {}
        Phase::Waiting => {
            fill(buf, inner, ink.ground);
            centred(buf, inner, mid, &waking_pill(wake.ms(now), ink));
        }
        Phase::Fade(shown) => fade(buf, inner, ink.ground, 1.0 - shown),
        Phase::Rain { dissolved, ms } => {
            fill(buf, inner, ink.ground);
            let (w, h) = (usize::from(inner.width), usize::from(inner.height));
            let rain = match &mut wake.rain {
                Some(r) if (r.w, r.h) == (w, h) => r,
                // A pane that changed size starts its rain over: the cells are other cells.
                slot => slot.insert(Rain::new(w, h, wake.seed)),
            };
            let frames = (now.saturating_duration_since(wake.stepped).as_secs_f32() * 1000.0 / RAIN_FRAME_MS).min(RAIN_STEPS_MOST);
            wake.stepped = now;
            rain.step(frames, if ms < RAIN_MS { 1.0 } else { DISSOLVE_SPEED });
            let head = blend(t.accent, Color::Rgb(255, 255, 255), 0.85);
            let dying = blend(t.accent, ink.ground, 0.7);
            let reach = dissolved * 1.15;
            for (i, d) in rain.drops.iter().enumerate() {
                if d.bright < TRAIL_GONE || (dissolved > 0.0 && reach > d.dies_at) {
                    continue;
                }
                let style = if d.head {
                    Style::default().fg(head).add_modifier(Modifier::BOLD)
                } else if dissolved > 0.0 && reach > d.dies_at - DYING_WITHIN {
                    Style::default().fg(dying)
                } else {
                    Style::default().fg(blend(ink.ground, t.accent, (d.bright * 1.1).min(1.0)))
                };
                let (x, y) = (inner.x + (i % w) as u16, inner.y + (i / w) as u16);
                buf[(x, y)].set_char(d.glyph).set_style(style.bg(ink.ground));
            }
            centred(buf, inner, mid, &waking_pill(ms, ink));
            // The bar: how far along, and what it's doing. It never says done before the
            // agent is back.
            let full = (ms / BAR_FULL_MS).clamp(0.0, 1.0);
            let filled = ((if wake.ready.is_some() { full } else { full.min(HELD_AT) }) * BAR_CELLS as f32).round() as usize;
            let step = if ms < RAIN_MS {
                "restoring session".to_string()
            } else if ms < REATTACH_SAID_UNTIL_MS || wake.ready.is_none() {
                format!("reattaching {}", ink.who)
            } else {
                "replaying scrollback".to_string()
            };
            let on = Style::default().bg(ink.ground);
            let bar = [seg("━".repeat(filled), on.fg(t.accent)), seg("━".repeat(BAR_CELLS - filled), on.fg(t.line)), seg(format!("  {step}"), on.fg(t.muted))];
            // Its own ground under it, so the rain doesn't run through the words.
            let bw = segs_width(&bar);
            let bx = inner.x + inner.width.saturating_sub(bw) / 2;
            if mid + 2 < inner.bottom() {
                fill(buf, Rect { x: bx.saturating_sub(1), y: mid + 2, width: (bw + 2).min(inner.right().saturating_sub(bx.saturating_sub(1))), height: 1 }, ink.ground);
                put(buf, bx, mid + 2, &bar, inner.right());
            }
        }
        Phase::Restore { ms } => {
            // The screen from the top, a line at a time; the rest waits its turn.
            let shown = (ms / LINE_MS) as u16;
            if shown < inner.height {
                fill(buf, Rect { y: inner.y + shown, height: inner.height - shown, ..inner }, ink.ground);
            }
            if ms < AWAKE_FADE_MS {
                let gone = ms / AWAKE_FADE_MS;
                let toward = |c: Color| blend(c, ink.ground, gone);
                let on = Style::default();
                let said = vec![
                    seg("✓ ", on.fg(toward(t.done))),
                    seg("awake", on.fg(toward(t.accent)).add_modifier(Modifier::BOLD)),
                    seg(" · back where it left off", on.fg(toward(t.text))),
                ];
                centred(buf, inner, mid, &pill(ink.look, said, toward(t.card2), ink.ground));
            }
        }
    }
}

/// "⣾ waking up · picking up where it left off", on its pill.
fn waking_pill(ms: f32, ink: &Ink) -> Vec<Seg> {
    let t = ink.theme;
    let on = Style::default();
    let spin = SPINNER[(ms / SPINNER_MS) as usize % SPINNER.len()];
    let said = vec![
        seg(format!("{spin} "), on.fg(t.working)),
        seg("waking up", on.fg(t.accent).add_modifier(Modifier::BOLD)),
        seg(" · picking up where it left off", on.fg(t.text)),
    ];
    pill(ink.look, said, t.card2, ink.ground)
}

fn centred(buf: &mut Buffer, inner: Rect, y: u16, segs: &[Seg]) {
    if y >= inner.y && y < inner.bottom() {
        put(buf, inner.x + inner.width.saturating_sub(segs_width(segs)) / 2, y, segs, inner.right());
    }
}

/// How many rows from the top have something on them.
fn rows_used(buf: &Buffer, inner: Rect) -> u16 {
    (inner.top()..inner.bottom()).rev().find(|y| (inner.left()..inner.right()).any(|x| buf[(x, *y)].symbol().trim() != "")).map_or(0, |y| y - inner.y + 1)
}

/// Every cell's ink `gone` of the way to the ground (1: nothing to see).
fn fade(buf: &mut Buffer, inner: Rect, ground: Color, gone: f32) {
    for y in inner.top()..inner.bottom() {
        for x in inner.left()..inner.right() {
            let cell = &mut buf[(x, y)];
            let fg = if cell.fg == Color::Reset { Color::Rgb(200, 200, 200) } else { cell.fg };
            cell.set_fg(blend(fg, ground, gone));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(wake: &Wake, ms: u64) -> Instant {
        wake.started + Duration::from_millis(ms)
    }

    fn text(buf: &Buffer) -> String {
        let a = buf.area;
        (a.top()..a.bottom()).map(|y| (a.left()..a.right()).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    /// A pane with the agent's screen on it: three lines of output.
    fn screen() -> (Buffer, Rect) {
        let inner = Rect { x: 0, y: 0, width: 90, height: 24 };
        let mut buf = Buffer::empty(inner);
        for (y, line) in ["● Read(src/ui/AgentList.tsx)", "  └ 188 lines", "> "].iter().enumerate() {
            buf.set_string(0, y as u16, line, Style::default());
        }
        (buf, inner)
    }

    fn ink(theme: &Theme) -> Ink<'_> {
        Ink { theme, look: Look::of(&crate::config::Config::default().ui), ground: theme.bg, who: "opus 4.5" }
    }

    #[test]
    fn rain_then_a_dissolve_from_the_centre_then_the_screen_line_by_line() {
        let theme = Theme::named("seshi-night");
        let start = Instant::now();
        let mut wake = Wake::new(start, WakeStyle::Full, 7);
        wake.ready = Some(at(&wake, 400));
        let lit = |buf: &Buffer, inner: Rect, middle: bool| {
            let (cx, cy) = (inner.width / 2, inner.height / 2);
            (inner.top()..inner.bottom())
                .flat_map(|y| (inner.left()..inner.right()).map(move |x| (x, y)))
                .filter(|(x, y)| (x.abs_diff(cx) < 12 && y.abs_diff(cy) < 4) == middle)
                .filter(|(x, y)| GLYPHS.contains(buf[(*x, *y)].symbol()) && !buf[(*x, *y)].symbol().trim().is_empty() && buf[(*x, *y)].fg != Color::Reset)
                .count()
        };
        // Rain: the agent's screen is covered, the pill and the bar say what's happening.
        let frame = |wake: &mut Wake, ms: u64| {
            let (mut buf, inner) = screen();
            let now = at(wake, ms);
            draw(&mut buf, inner, wake, now, &ink(&theme));
            (buf, inner)
        };
        let mut last = frame(&mut wake, 0);
        for ms in (16..=880).step_by(16) {
            last = frame(&mut wake, ms);
        }
        let said = text(&last.0);
        assert!(said.contains("waking up · picking up where it left off") && said.contains("restoring session"), "{said}");
        assert!(!said.contains("AgentList"), "the screen is covered while it rains");
        assert!(lit(&last.0, last.1, false) > 150, "the pane is full of rain: {}", lit(&last.0, last.1, false));
        assert!(wake.animating(at(&wake, 880)));
        // Dissolving: the middle has cleared before the edges have.
        for ms in (900..=1900).step_by(16) {
            last = frame(&mut wake, ms);
        }
        let (middle, edges) = (lit(&last.0, last.1, true), lit(&last.0, last.1, false));
        assert!(middle == 0 && edges > 0, "the centre goes first: {middle} in the middle, {edges} around it");
        assert!(text(&last.0).contains("replaying scrollback"));
        // Restore: the screen from the top, a line at a time, under a fading "awake".
        let (buf, _) = frame(&mut wake, 2600 + 60);
        let said = text(&buf);
        assert!(said.contains("AgentList") && !said.contains("188 lines") && said.contains("✓ awake · back where it left off"), "one line in: {said}");
        let (buf, _) = frame(&mut wake, 2600 + 170);
        assert!(text(&buf).contains("188 lines"));
        // And then it's an ordinary pane.
        assert!(!wake.done(at(&wake, 2600 + 800)) && wake.done(at(&wake, 2600 + 901)));
        let (buf, _) = frame(&mut wake, 4000);
        assert_eq!(text(&buf), text(&screen().0), "nothing left over it");
    }

    #[test]
    fn a_slow_agent_holds_the_dissolve_until_it_is_back() {
        let theme = Theme::named("seshi-night");
        let mut wake = Wake::new(Instant::now(), WakeStyle::Full, 3);
        let (mut buf, inner) = screen();
        for ms in (0..=6000).step_by(33) {
            let now = at(&wake, ms);
            (buf, _) = screen();
            draw(&mut buf, inner, &mut wake, now, &ink(&theme));
        }
        // Six seconds on and no agent: still waking, the bar short of full, saying who.
        let said = text(&buf);
        assert!(matches!(wake.phase(at(&wake, 6000)), Phase::Rain { dissolved, .. } if dissolved == HELD_AT));
        assert!(said.contains("waking up") && said.contains("reattaching opus 4.5") && !said.contains("AgentList"), "{said}");
        let filled = said.lines().find(|l| l.contains("reattaching")).map(|l| l.matches('━').count()).unwrap();
        assert_eq!(filled, BAR_CELLS, "the bar is drawn whole (filled and empty cells)");
        assert!(!wake.done(at(&wake, 60_000)), "and it waits as long as it takes");
        // It answers: the dissolve finishes, and the screen comes back.
        wake.ready = Some(at(&wake, 6000));
        assert!(matches!(wake.phase(at(&wake, 6150)), Phase::Rain { dissolved, .. } if dissolved > HELD_AT));
        assert!(matches!(wake.phase(at(&wake, 6000 + 301)), Phase::Restore { .. }));
        assert!(wake.done(at(&wake, 6000 + 300 + 901)));
    }

    #[test]
    fn with_motion_off_there_is_no_rain() {
        let theme = Theme::named("seshi-night");
        assert_eq!((WakeStyle::of("on", true), WakeStyle::of("on", false), WakeStyle::of("simple", true), WakeStyle::of("off", true)), (WakeStyle::Full, WakeStyle::Simple, WakeStyle::Simple, WakeStyle::Off));
        for kind in [WakeStyle::Simple, WakeStyle::Off] {
            let mut wake = Wake::new(Instant::now(), kind, 1);
            let (mut buf, inner) = screen();
            let now = at(&wake, 1500);
            draw(&mut buf, inner, &mut wake, now, &ink(&theme));
            let said = text(&buf);
            assert!(said.contains("waking up · picking up where it left off") && !said.contains("AgentList") && !said.contains('ｱ'), "{kind:?}: just the note");
            assert!(!wake.animating(now), "only its spinner turns");
            wake.ready = Some(at(&wake, 2000));
            let after = if kind == WakeStyle::Simple { 2000 + 301 } else { 2000 };
            assert!(wake.done(at(&wake, after)), "{kind:?}");
        }
        // Simple: the screen fades in rather than appearing.
        let mut wake = Wake::new(Instant::now(), WakeStyle::Simple, 1);
        wake.ready = Some(at(&wake, 1000));
        let (mut buf, inner) = screen();
        let now = at(&wake, 1100);
        draw(&mut buf, inner, &mut wake, now, &ink(&theme));
        assert!(text(&buf).contains("AgentList") && buf[(0, 0)].fg != Color::Reset, "there, and part of the way to its colour");
    }
}
