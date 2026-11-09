//! Built-in themes plus per-key overrides from config. Pane contents keep the program's own
//! colours; the theme only paints hydra's chrome (sidebar, title bars, status line).
//!
//! The roles: `surf` for chrome, `card`/`card2` for panels and
//! action rows, `btn`/`hov` for buttons and the hovered row, `acc` (teal in Dark
//! theme) for focus, `needs` (amber) only for "needs you".

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Theme {
    /// Ground, and the default background of terminal cells.
    pub bg: Color,
    pub fg: Color,
    pub muted: Color,
    /// Focus, primary worktree, primary buttons, keys.
    pub accent: Color,
    pub border: Color,
    pub border_active: Color,
    /// Sidebar, status line, unfocused title bars (`surf` in the design).
    pub sidebar_bg: Color,
    pub selection_bg: Color,
    pub tab_active_bg: Color,
    pub tab_active_fg: Color,
    pub working: Color,
    /// "Needs you". Nothing else uses this colour.
    pub blocked: Color,
    pub done: Color,
    pub idle: Color,
    /// Modal and menu panels.
    pub card: Color,
    /// Answer bar, action rows, modal input.
    pub card2: Color,
    /// Normal button ground.
    pub btn: Color,
    /// Hovered row or button, keyboard cursor row.
    pub hov: Color,
    /// Dividers and tree glyphs.
    pub line: Color,
    /// Secondary text.
    pub text: Color,
    /// Names and titles.
    pub strong: Color,
    /// Text on the accent colour.
    pub acc_ink: Color,
    /// Discard, failures.
    pub err: Color,
    /// Project colours (violet, sky, pink, teal), when the theme has its own.
    pub ws: Option<[Color; 4]>,
    /// Terminal colours for agent output (red, green, yellow, blue, magenta, cyan, gray),
    /// when the theme has its own; otherwise programs keep the terminal's palette.
    pub ansi: Option<[Color; 7]>,
    /// The splash's own colours (wordmark, its A and I, the swell, the deep the letters rise
    /// from), when the theme has them; otherwise `splash()` takes them from its roles.
    pub splash: Option<[Color; 4]>,
}

/// The colours the splash paints with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplashColors {
    pub wordmark: Color,
    pub wordmark_ai: Color,
    pub swell: Color,
    pub deep: Color,
}

/// Every field optional, as hex strings ("#89b4fa") or names ("red", "reset", "123").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeOverrides(pub BTreeMap<String, String>);

/// How much of the accent a selected row's ground carries (a share of 1): enough to be seen on a dark ground, little enough that
/// the text on it keeps its contrast.
const SELECTED_ACCENT: f32 = 0.24;

/// The theme when none is chosen.
pub const DEFAULT: &str = "night";
/// The light theme when none is chosen.
pub const DEFAULT_LIGHT: &str = "day";

/// Where a theme that is no longer built in went: each removed id, and the two Maitty long names, name the surviving theme
/// nearest to it on the same side (dark to a dark one, light to a light one, by the colour of the ground), so a config that
/// still says it keeps loading. `canonical` follows it once.
const ALIASES: &[(&str, &str)] = &[
    ("maitty-night", "night"),
    ("maitty-day", "day"),
    ("gruvbox-material", "gruvbox"),
    ("koda", "github-dark"),
    ("melange", "gruvbox"),
    ("apprentice", "gruvbox"),
    ("aquarium", "catppuccin-mocha"),
    ("arcade", "maitty-dark"),
    ("archery", "gruvbox"),
    ("ash", "catppuccin-latte"),
    ("ashen", "github-dark"),
    ("atelier-cave-dark", "tokyo-night"),
    ("atelier-cave-light", "maitty-light"),
    ("atelier-dune-dark", "tokyo-night"),
    ("atelier-dune-light", "solarized-light"),
    ("atelier-estuary-dark", "gruvbox"),
    ("atelier-estuary-light", "catppuccin-latte"),
    ("atelier-forest-dark", "tokyo-night"),
    ("atelier-forest-light", "maitty-light"),
    ("atelier-heath-dark", "tokyo-night"),
    ("atelier-heath-light", "catppuccin-latte"),
    ("atelier-lakeside-dark", "tokyo-night"),
    ("atelier-lakeside-light", "catppuccin-latte"),
    ("atelier-plateau-dark", "tokyo-night"),
    ("atelier-plateau-light", "maitty-light"),
    ("atelier-savanna-dark", "tokyo-night"),
    ("atelier-savanna-light", "day"),
    ("atelier-seaside-dark", "github-dark"),
    ("atelier-seaside-light", "one-light"),
    ("atelier-sulphurpool-dark", "one-dark"),
    ("atelier-sulphurpool-light", "one-light"),
    ("aurora", "github-dark"),
    ("ayu-dark", "github-dark"),
    ("ayu-light", "one-light"),
    ("ayu-mirage", "one-dark"),
    ("badwolf", "tokyo-night"),
    ("bamboo", "gruvbox"),
    ("bamboo-light", "solarized-light"),
    ("bamboo-multiplex", "gruvbox"),
    ("base16-default-dark", "tokyo-night"),
    ("brogrammer", "tokyo-night"),
    ("carbonfox", "github-dark"),
    ("catppuccin-frappe", "one-dark"),
    ("catppuccin-macchiato", "one-dark"),
    ("cendre", "github-dark"),
    ("cendre-medium", "tokyo-night"),
    ("cendre-soft", "tokyo-night"),
    ("cobalt2", "solarized-dark"),
    ("code-2026-dark", "github-dark"),
    ("code-2026-light", "github-light"),
    ("code-dark", "tokyo-night"),
    ("cyberdream", "tokyo-night"),
    ("cyberdream-light", "github-light"),
    ("cyberdream-muted", "tokyo-night"),
    ("dawnfox", "solarized-light"),
    ("dayfox", "catppuccin-latte"),
    ("dogrun", "catppuccin-mocha"),
    ("doom-one", "one-dark"),
    ("doom-one-light", "one-light"),
    ("dracula", "one-dark"),
    ("duskfox", "catppuccin-mocha"),
    ("edge", "one-dark"),
    ("eldar", "maitty-dark"),
    ("embark", "catppuccin-mocha"),
    ("ember", "github-dark"),
    ("everforest-dark", "one-dark"),
    ("everforest-light", "solarized-light"),
    ("falcon", "maitty-dark"),
    ("flexoki-dark", "github-dark"),
    ("flexoki-light", "one-light"),
    ("fluoromachine", "catppuccin-mocha"),
    ("github-dark-dimmed", "gruvbox"),
    ("goodwolf", "github-dark"),
    ("gotham", "github-dark"),
    ("gruber-darker", "tokyo-night"),
    ("gruvbox-baby", "tokyo-night"),
    ("gruvbox-material-dark", "gruvbox"),
    ("gruvbox-material-light", "gruvbox-light"),
    ("harbor", "night"),
    ("harbor-day", "catppuccin-latte"),
    ("horizon", "tokyo-night"),
    ("hybrid", "tokyo-night"),
    ("hybrid-light", "tokyo-night-day"),
    ("hybrid-reduced-contrast", "one-dark"),
    ("hydra", "github-dark"),
    ("iceberg", "tokyo-night"),
    ("ink", "github-dark"),
    ("jellybeans", "github-dark"),
    ("kanagawa-dragon", "github-dark"),
    ("kanagawa-lotus", "gruvbox-light"),
    ("kanagawa-paper", "catppuccin-mocha"),
    ("kanagawa-wave", "catppuccin-mocha"),
    ("koda-dark", "github-dark"),
    ("koda-glade", "one-light"),
    ("koda-light", "one-light"),
    ("koda-moss", "github-dark"),
    ("lackluster", "maitty-dark"),
    ("lucario", "one-dark"),
    ("luna", "maitty-dark"),
    ("melange-dark", "gruvbox"),
    ("melange-light", "catppuccin-latte"),
    ("mellifluous", "tokyo-night"),
    ("mellow", "github-dark"),
    ("mellow-light", "one-light"),
    ("miasma", "gruvbox"),
    ("miramare", "gruvbox"),
    ("modus-operandi", "github-light"),
    ("mono", "maitty-dark"),
    ("monokai", "gruvbox"),
    ("monokai-pro", "one-dark"),
    ("moonfly", "maitty-dark"),
    ("moonlit", "github-dark"),
    ("moss", "github-dark"),
    ("moss-day", "catppuccin-latte"),
    ("neovim-dark", "github-dark"),
    ("neovim-light", "tokyo-night-day"),
    ("night-owl", "night"),
    ("nightfly", "night"),
    ("nightfox", "catppuccin-mocha"),
    ("nord", "one-dark"),
    ("nordfox", "one-dark"),
    ("nordic", "one-dark"),
    ("oceanic-next", "one-dark"),
    ("one-half-dark", "one-dark"),
    ("one-half-light", "one-light"),
    ("onenord", "one-dark"),
    ("oxocarbon", "github-dark"),
    ("palenight", "one-dark"),
    ("panda", "gruvbox"),
    ("paper", "one-light"),
    ("papercolor-dark", "tokyo-night"),
    ("papercolor-light", "maitty-light"),
    ("rigel", "solarized-dark"),
    ("rose-pine", "tokyo-night"),
    ("rose-pine-dawn", "solarized-light"),
    ("rose-pine-moon", "catppuccin-mocha"),
    ("selenized-light", "solarized-light"),
    ("shades-of-purple", "catppuccin-mocha"),
    ("simple-dark", "maitty-dark"),
    ("solarized-osaka", "night"),
    ("solarized-osaka-light", "solarized-light"),
    ("sonokai", "one-dark"),
    ("spacecamp-lite", "tokyo-night"),
    ("synthwave-84", "catppuccin-mocha"),
    ("tango-dark", "one-dark"),
    ("tender", "gruvbox"),
    ("terafox", "solarized-dark"),
    ("text-to-colorscheme", "gruvbox"),
    ("tinkery", "github-dark"),
    ("tinkery-light", "maitty-light"),
    ("token", "gruvbox"),
    ("tokyo-night-moon", "catppuccin-mocha"),
    ("tokyo-night-storm", "one-dark"),
    ("tomorrow", "github-light"),
    ("tomorrow-night", "tokyo-night"),
    ("tomorrow-night-bright", "maitty-dark"),
    ("tomorrow-night-eighties", "one-dark"),
    ("turtles", "tokyo-night"),
    ("vague", "github-dark"),
    ("vim", "github-light"),
    ("vscode", "tokyo-night"),
    ("yui-dark", "catppuccin-mocha"),
    ("yui-light", "maitty-light"),
    ("zenbones-dark", "tokyo-night"),
    ("zenbones-light", "maitty-light"),
    ("zephyr", "one-dark"),
];

/// The built-in theme a configured name means: itself, or the theme its old name became.
pub fn canonical(name: &str) -> &str {
    ALIASES
        .iter()
        .find(|(old, _)| *old == name)
        .map_or(name, |(_, new)| new)
}

/// Whether the built-in theme `name` is a light one. A name that is no theme reads as dark.
pub fn is_light(name: &str) -> bool {
    BUILTIN.contains(&canonical(name)) && Theme::named(name).is_light()
}

/// Every built-in theme, kept in alphabetical order of its display name: the theme browser,
/// the Theme setting's cycling and the unknown-theme warning all list it as written.
pub const BUILTIN: &[&str] = &[
    "catppuccin-latte",
    "catppuccin-mocha",
    "github-dark",
    "github-light",
    "gruvbox",
    "gruvbox-light",
    "maitty-dark",
    "day",
    "maitty-light",
    "night",
    "one-dark",
    "one-light",
    "solarized-dark",
    "solarized-light",
    "tokyo-night",
    "tokyo-night-day",
];

/// The display names Settings shows; a theme missing here shows its config name in title case.
pub const DESIGN: &[(&str, &str)] = &[
    ("catppuccin-latte", "Catppuccin Latte"),
    ("catppuccin-mocha", "Catppuccin Mocha"),
    ("github-dark", "GitHub Dark"),
    ("github-light", "GitHub Light"),
    ("gruvbox", "Gruvbox Dark"),
    ("gruvbox-light", "Gruvbox Light"),
    ("maitty-dark", "Maitty Dark"),
    ("day", "Maitty Day"),
    ("maitty-light", "Maitty Light"),
    ("night", "Maitty Night"),
    ("one-dark", "One Dark"),
    ("one-light", "One Light"),
    ("solarized-dark", "Solarized Dark"),
    ("solarized-light", "Solarized Light"),
    ("tokyo-night", "Tokyo Night"),
    ("tokyo-night-day", "Tokyo Night Day"),
];

/// Needs-you ambers: gold-orange on dark grounds, a deeper brown-orange on light ones, and a
/// redder fallback of each for when the first sits next to another status.
const AMBER_ON_DARK: Color = Color::Rgb(0xf4, 0xb0, 0x3c);
const DEEP_AMBER_ON_DARK: Color = Color::Rgb(0xe8, 0x8a, 0x1e);
const AMBER_ON_LIGHT: Color = Color::Rgb(0xa8, 0x62, 0x00);
const GOLD: Color = Color::Rgb(0xf0, 0xc8, 0x2c);
const RED_ORANGE: Color = Color::Rgb(0xf0, 0x78, 0x28);
const MID_ORANGE: Color = Color::Rgb(0xff, 0x9f, 0x2e);
const PALE_AMBER: Color = Color::Rgb(0xff, 0xd0, 0x70);
const DEEP_AMBER_ON_LIGHT: Color = Color::Rgb(0x8f, 0x4a, 0x00);

/// The Maitty colour tokens, parsed once from `maitty-tokens.json`.
fn maitty_tokens() -> &'static serde_json::Value {
    static TOKENS: OnceLock<serde_json::Value> = OnceLock::new();
    TOKENS.get_or_init(|| {
        serde_json::from_str(include_str!("../maitty-tokens.json")).expect("maitty-tokens.json is valid JSON")
    })
}

/// One token of the Maitty file, e.g. `maitty_token("night", "bg.page")`.
pub fn maitty_token(side: &str, path: &str) -> String {
    let mut node = &maitty_tokens()[side];
    for key in path.split('.') {
        node = &node[key];
    }
    node.as_str().unwrap_or_else(|| panic!("maitty-tokens.json has no {side}.{path}")).to_string()
}

fn hex(s: &str) -> Color {
    parse_color(s).unwrap_or(Color::Reset)
}

pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('#') {
        if h.len() == 6 {
            let v = u32::from_str_radix(h, 16).ok()?;
            return Some(Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8));
        }
        return None;
    }
    if let Ok(n) = s.parse::<u8>() {
        return Some(Color::Indexed(n));
    }
    s.parse::<Color>().ok()
}

/// How well a state's colour must read on the lit row (the audit's bar for coloured text
/// on the sidebar), and how it gets there: a step toward the strongest text at a time.
const LIT_READS: f64 = 3.0;
const LIT_STEP: f32 = 0.15;
const LIT_STEPS: usize = 12;

/// WCAG contrast between two colours (1 to 21).
pub fn contrast(a: Color, b: Color) -> f64 {
    let l = |c: Color| match c {
        Color::Rgb(r, g, b) => {
            let ch = |v: u8| {
                let s = v as f64 / 255.0;
                if s <= 0.03928 {
                    s / 12.92
                } else {
                    ((s + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
        }
        _ => 0.5,
    };
    let (x, y) = (l(a), l(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Mix `b` into `a` by `t`; colours that aren't RGB can't be mixed and fall back.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let m =
                |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * t.clamp(0.0, 1.0)).round() as u8;
            Color::Rgb(m(r1, r2), m(g1, g2), m(b1, b2))
        }
        _ if t >= 0.5 => b,
        _ => a,
    }
}

fn readable_on(color: Color, ink: Color, surfaces: &[Color], minimum: f64) -> Color {
    const STEPS: usize = 100;
    for step in 0..=STEPS {
        let candidate = mix(color, ink, step as f32 / STEPS as f32);
        if surfaces
            .iter()
            .all(|surface| contrast(candidate, *surface) >= minimum)
        {
            return candidate;
        }
    }
    ink
}

fn colors_alike(a: Color, b: Color) -> bool {
    const MIN_COLOR_DISTANCE: i32 = 90;
    const MIN_COLOR_CONTRAST: f64 = 1.15;
    let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) else {
        return a == b;
    };
    let distance = (r1 as i32 - r2 as i32).abs()
        + (g1 as i32 - g2 as i32).abs()
        + (b1 as i32 - b2 as i32).abs();
    a == b || contrast(a, b) < MIN_COLOR_CONTRAST && distance < MIN_COLOR_DISTANCE
}

impl Theme {
    /// Return the foreground and background as RGB values for daemon snapshots.
    pub fn terminal_foreground_background_rgb(&self) -> ((u8, u8, u8), (u8, u8, u8)) {
        let rgb = |color| match color {
            Color::Rgb(red, green, blue) => (red, green, blue),
            _ => (0xc9, 0xd1, 0xd9),
        };
        let background = match self.bg {
            Color::Rgb(red, green, blue) => (red, green, blue),
            _ => (0x07, 0x0b, 0x10),
        };
        (rgb(self.fg), background)
    }

    /// Keep terminal colors faithful to upstream, while adapting UI roles for readable chrome.
    fn palette(base: [&str; 4], ansi: [&str; 7]) -> Theme {
        let [bg, fg, muted, accent] = base.map(hex);
        let ansi = ansi.map(hex);
        let ink = if contrast(bg, Color::Rgb(0, 0, 0)) > contrast(bg, Color::Rgb(255, 255, 255)) {
            Color::Rgb(0, 0, 0)
        } else {
            Color::Rgb(255, 255, 255)
        };
        let sidebar = mix(bg, fg, 0.025);
        let card = mix(bg, fg, 0.04);
        let card2 = mix(bg, fg, 0.08);
        let btn = mix(bg, fg, 0.10);
        let hov = mix(bg, fg, 0.16);
        let accent = readable_on(accent, ink, &[btn], 3.0);
        let open = mix(sidebar, accent, 0.16);
        let hover = mix(sidebar, ink, 0.10);
        let strong = readable_on(fg, ink, &[bg, open, hover], 7.0);
        let text = readable_on(mix(fg, muted, 0.20), ink, &[bg, card, card2], 4.5);
        let muted = readable_on(muted, ink, &[bg, sidebar, open], 3.0);
        let black = Color::Rgb(0, 0, 0);
        let white = Color::Rgb(255, 255, 255);
        let acc_ink = if contrast(black, accent) > contrast(white, accent) {
            black
        } else {
            white
        };
        let done = readable_on(ansi[1], ink, &[sidebar, hov], 3.0);
        let blocked = readable_on(ansi[0], ink, &[sidebar, hov], 3.0);
        let mut working = readable_on(ansi[2], ink, &[sidebar, hov], 4.5);
        // Muted terminal yellows can look like green or red. Status must remain distinguishable.
        if colors_alike(working, done) || colors_alike(working, blocked) {
            working = readable_on(
                mix(ansi[2], Color::Rgb(255, 96, 0), 0.85),
                ink,
                &[sidebar, hov],
                4.5,
            );
        }
        let line = mix(bg, fg, 0.22);
        Theme {
            bg,
            fg,
            muted,
            accent,
            border: line,
            border_active: accent,
            sidebar_bg: sidebar,
            selection_bg: hov,
            tab_active_bg: accent,
            tab_active_fg: acc_ink,
            working,
            blocked,
            done,
            idle: muted,
            card,
            card2,
            btn,
            hov,
            line,
            text,
            strong,
            acc_ink,
            err: readable_on(ansi[0], ink, &[card], 3.0),
            ws: Some([ansi[4], ansi[3], ansi[0], ansi[5]]),
            ansi: Some(ansi),
            splash: None,
        }
    }

    /// The fill of a destructive button (Close): the error red, a little deeper when its
    /// text wouldn't read on it otherwise.
    pub fn danger_fill(&self) -> Color {
        let mut c = self.err;
        for _ in 0..6 {
            if contrast(self.ink_on(c), c) >= 4.5 {
                break;
            }
            c = mix(c, Color::Rgb(0, 0, 0), 0.08);
        }
        c
    }

    /// The selected row's ground: the background tinted with the theme's accent, so a selection is
    /// the accent's colour and is seen on any ground, dark or light.
    /// Tree lines and line numbers: half way from the quiet text to the ground.
    pub fn faint(&self) -> Color {
        mix(self.muted, self.bg, 0.5)
    }

    pub fn sel_bg(&self) -> Color {
        mix(self.bg, self.accent, SELECTED_ACCENT)
    }

    /// The selected row's ground on a raised surface (a sheet, a popup): `ground` tinted with the accent.
    pub fn sel_on(&self, ground: Color) -> Color {
        mix(ground, self.accent, SELECTED_ACCENT)
    }

    /// A pointed-at row on a raised surface.
    pub fn hover_on(&self, ground: Color) -> Color {
        mix(ground, self.text, 0.06)
    }

    /// A state's colour as drawn on the lit row: itself where it reads on `hov`, else moved
    /// toward the theme's strongest text just far enough to. A row you select keeps saying
    /// its state, and no theme needs colours of its own for that (some put a red on a
    /// violet highlight, where it can't be read as it is).
    pub fn on_lit(&self, state: Color) -> Color {
        let mut c = state;
        for _ in 0..LIT_STEPS {
            if contrast(c, self.sel_bg()) >= LIT_READS && contrast(c, self.hov) >= LIT_READS {
                break;
            }
            c = mix(c, self.strong, LIT_STEP);
        }
        c
    }

    /// Text to put on a filled `c`: the theme's background or its strongest text, whichever
    /// reads better (light themes need dark ink on orange, dark themes light).
    pub fn ink_on(&self, c: Color) -> Color {
        // WCAG relative luminance and contrast.
        let l = |c: Color| match c {
            Color::Rgb(r, g, b) => {
                let ch = |v: u8| {
                    let s = v as f64 / 255.0;
                    if s <= 0.03928 {
                        s / 12.92
                    } else {
                        ((s + 0.055) / 1.055).powf(2.4)
                    }
                };
                0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
            }
            _ => 0.5,
        };
        let contrast = |a: f64, b: f64| (a.max(b) + 0.05) / (a.min(b) + 0.05);
        let fill = l(c);
        if contrast(l(self.bg), fill) >= contrast(l(self.strong), fill) {
            self.bg
        } else {
            self.strong
        }
    }

    /// A theme from its 14 classic colours; the newer roles are derived from them.
    fn classic(c: [&str; 14]) -> Theme {
        let (bg, fg, muted, border) = (hex(c[0]), hex(c[1]), hex(c[2]), hex(c[4]));
        Theme {
            bg,
            fg,
            muted,
            accent: hex(c[3]),
            border,
            border_active: hex(c[5]),
            sidebar_bg: hex(c[6]),
            selection_bg: hex(c[7]),
            tab_active_bg: hex(c[8]),
            tab_active_fg: hex(c[9]),
            // Working yellow, needs you red, done green, idle dim.
            working: hex(c[10]),
            blocked: hex(c[11]),
            done: hex(c[13]),
            idle: muted,
            card: mix(bg, fg, 0.05),
            card2: mix(bg, fg, 0.11),
            btn: mix(bg, fg, 0.14),
            hov: hex(c[7]),
            line: border,
            text: mix(muted, fg, 0.5),
            strong: fg,
            acc_ink: bg,
            err: hex(c[11]),
            ws: None,
            ansi: None,
            splash: None,
        }
    }

    /// A theme from the full set of design roles.
    /// `c`: bg fg surf card card2 line dim text strong acc accInk needs ok err btn hov.
    fn design(c: [&str; 16], ws: [&str; 4], a: [&str; 7]) -> Theme {
        let h = |i: usize| hex(c[i]);
        Theme {
            bg: h(0),
            fg: h(1),
            muted: h(6),
            accent: h(9),
            border: h(5),
            border_active: h(9),
            sidebar_bg: h(2),
            selection_bg: h(15),
            tab_active_bg: h(9),
            tab_active_fg: h(10),
            // Working yellow, needs you red, done green, idle dim.
            working: h(11),
            blocked: h(13),
            done: h(12),
            idle: h(6),
            card: h(3),
            card2: h(4),
            btn: h(14),
            hov: h(15),
            line: h(5),
            text: h(7),
            strong: h(8),
            acc_ink: h(10),
            err: h(13),
            ws: Some(ws.map(hex)),
            ansi: Some(a.map(hex)),
            splash: None,
        }
    }

    /// The Maitty theme for `side` (`night`, `day`, `dark` or `light`), made from the tokens file so every Maitty theme follows it.
    /// An error red is the one colour the file does not carry: it must not read as the coral of needs-you.
    fn maitty(side: &str) -> Theme {
        // The two light sides (the sea-glass day and the neutral light) read the same way: darker brass text, card ink.
        let day = side == "day" || side == "light";
        let t = |path: &str| maitty_token(side, path);
        // Text on the ground uses the file's `*Text` tokens (the fill and its text colour differ by day).
        let accent_text = t("accent.brassText");
        // The accent is the brass; on the day ground the brass fill is too pale to read as text or to carry an
        // ink, so the day accent is the file's darker `brassText` with the card colour as its ink.
        let accent = if day { accent_text.clone() } else { t("accent.brass") };
        let accent_ink = if day { t("bg.card") } else { t("accent.brassOn") };
        let err = if day { "#c4003c" } else { "#e0245e" };
        // Done is the file's green text; by day it is a shade too near the teal of working to tell them apart by
        // colour alone, so the TUI lifts it a third of the way toward the file's green fill.
        let done = if day {
            let lifted = mix(hex(&t("state.doneText")), hex(&t("state.done")), 0.35);
            match lifted {
                Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
                _ => t("state.doneText"),
            }
        } else {
            t("state.doneText")
        };
        let card2 = if day { t("bg.depth1") } else { t("bg.depth2") };
        let selection = match mix(hex(&t("bg.card")), hex(&t("accent.brass")), 0.30) {
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => t("bg.selected"),
        };
        let mut theme = Theme::design(
            [
                &t("bg.page"),
                &t("text.body"),
                &t("bg.surface"),
                &t("bg.card"),
                &card2,
                &t("line.default"),
                &t("text.dim"),
                &t("text.body"),
                &t("text.strong"),
                &accent,
                &accent_ink,
                &t("state.workingText"),
                &done,
                &t("state.needsText"),
                &t("bg.selected"),
                &t("bg.depth2"),
            ],
            [
                if day { "#7e57c2" } else { "#c4a7ff" },
                if day { "#2f6fbf" } else { "#6cb6ff" },
                &accent_text,
                &t("state.workingText"),
            ],
            [
                err,
                &done,
                &accent_text,
                if day { "#2f6fbf" } else { "#6cb6ff" },
                if day { "#7e57c2" } else { "#c4a7ff" },
                &t("state.workingText"),
                &t("text.dim"),
            ],
        )
        .with_err(err)
        .with_selection(&selection)
        .with_splash([
            &t("text.strong"),
            &accent_text,
            &t("bg.depth1"),
            &t(if day { "bg.depth2" } else { "bg.depth3" }),
        ]);
        theme.border_active = hex(&t("focus"));
        theme
    }

    /// Whether this theme is drawn on a light ground: its background is nearer white than black.
    /// A ground that is not a fixed colour (Mono takes the terminal's own) reads as dark.
    pub fn is_light(&self) -> bool {
        matches!(self.bg, Color::Rgb(..))
            && contrast(self.bg, Color::Rgb(255, 255, 255)) < contrast(self.bg, Color::Rgb(0, 0, 0))
    }

    pub fn named(name: &str) -> Theme {
        let name = canonical(name);
        let mut theme = Theme::base(name);
        // The row the keys are on is the accent tinted into the ground, in every theme: a theme's own hover colour was often a step
        // from the card too small to see on a dark ground.
        theme.hov = theme.sel_on(theme.card);
        if matches!(name, "night" | "day" | "maitty-dark" | "maitty-light") {
            return theme;
        }
        theme.with_distinct_needs()
    }

    fn base(name: &str) -> Theme {
        match name {
            // The app's own pair, Maitty Sea-glass and Brass, built from `maitty-tokens.json`: teal only
            // means working, coral only means needs you, brass is the accent, the AI in the wordmark is gold.
            "night" => Theme::maitty("night"),
            "day" => Theme::maitty("day"),
            "maitty-dark" => Theme::maitty("dark"),
            "maitty-light" => Theme::maitty("light"),
            "tokyo-night" => Theme::design(
                [
                    "#1a1b26", "#c0caf5", "#16161e", "#1f2335", "#292e42", "#292e42", "#7a82ad",
                    "#a9b1d6", "#e0e6ff", "#7aa2f7", "#1a1b26", "#e0af68", "#9ece6a", "#f7768e",
                    "#292e42", "#3b4261",
                ],
                ["#bb9af7", "#7dcfff", "#f7768e", "#73daca"],
                [
                    "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#565f89",
                ],
            ),
            "catppuccin-mocha" => Theme::classic([
                "#1e1e2e", "#cdd6f4", "#6c7086", "#89b4fa", "#45475a", "#89b4fa", "#181825",
                "#313244", "#89b4fa", "#1e1e2e", "#f9e2af", "#f38ba8", "#89dceb", "#a6e3a1",
            ]),
            "catppuccin-latte" => {
                let mut theme = Theme::classic([
                    "#eff1f5", "#4c4f69", "#6c6f85", "#7a2fd8", "#bcc0cc", "#7a2fd8", "#e6e9ef",
                    "#ccd0da", "#7a2fd8", "#eff1f5", "#8a5a00", "#d20f39", "#1e5ad8", "#2f8a1f",
                ]);
                // The text is Latte's own, as the shared list has it; names and titles stay the deeper
                // blue-grey so they keep 7:1 on a lit row.
                theme.strong = hex("#303446");
                theme
            }
            "gruvbox" => Theme::classic([
                "#282828", "#ebdbb2", "#928374", "#fabd2f", "#504945", "#fabd2f", "#1d2021",
                "#3c3836", "#fabd2f", "#282828", "#fe8019", "#fb4934", "#83a598", "#b8bb26",
            ]),
            "github-dark" => Theme::palette(
                ["#0d1117", "#c9d1d9", "#8b949e", "#2f81f7"],
                [
                    "#ff7b72", "#3fb950", "#d29922", "#58a6ff", "#bc8cff", "#76e3ea", "#8b949e",
                ],
            ),
            "github-light" => Theme::palette(
                ["#ffffff", "#24292f", "#57606a", "#0969da"],
                [
                    "#cf222e", "#116329", "#4d2d00", "#0969da", "#8250df", "#1b7c83", "#57606a",
                ],
            ),
            "gruvbox-light" => Theme::palette(
                ["#fbf1c7", "#3c3836", "#928374", "#076678"],
                [
                    "#9d0006", "#79740e", "#b57614", "#076678", "#8f3f71", "#427b58", "#3c3836",
                ],
            ),
            "one-dark" => Theme::palette(
                ["#282c34", "#abb2bf", "#3e4452", "#61afef"],
                [
                    "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#5c6370",
                ],
            ),
            "one-light" => Theme::palette(
                ["#fafafa", "#383a42", "#a0a1a7", "#4078f2"],
                [
                    "#e45649", "#50a14f", "#c18401", "#4078f2", "#a626a4", "#0184bc", "#a0a1a7",
                ],
            ),
            "solarized-dark" => Theme::palette(
                ["#002b36", "#93a1a1", "#586e75", "#268bd2"],
                [
                    "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5",
                ],
            ),
            "solarized-light" => Theme::palette(
                ["#fdf6e3", "#586e75", "#93a1a1", "#268bd2"],
                [
                    "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5",
                ],
            ),
            "tokyo-night-day" => Theme::palette(
                ["#e1e2e7", "#3760bf", "#a1a6c5", "#2e7de9"],
                [
                    "#f52a65", "#587539", "#8c6c3e", "#2e7de9", "#9854f1", "#007197", "#6172b0",
                ],
            ),
            _ => Theme::base(DEFAULT),
        }
    }

    /// Destructive red apart from what needs you (the design palettes share one colour).
    fn with_splash(mut self, c: [&str; 4]) -> Theme {
        self.splash = Some(c.map(hex));
        self
    }

    /// The splash's colours: the theme's own, or the strongest text for the wordmark, the
    /// accent for its A and I, and a swell and a deep drawn from the accent toward the ground.
    pub fn splash(&self) -> SplashColors {
        match self.splash {
            Some([wordmark, wordmark_ai, swell, deep]) => SplashColors {
                wordmark,
                wordmark_ai,
                swell,
                deep,
            },
            None => SplashColors {
                wordmark: self.strong,
                wordmark_ai: self.accent,
                swell: mix(self.bg, self.accent, 0.55),
                deep: mix(self.bg, self.accent, 0.2),
            },
        }
    }

    fn with_err(mut self, c: &str) -> Theme {
        self.err = hex(c);
        self
    }

    fn with_selection(mut self, c: &str) -> Theme {
        self.selection_bg = hex(c);
        self
    }

    /// Needs you in an amber of its own, so it never reads as failed. A theme that already has
    /// one apart from every other status keeps it; otherwise the amber comes from the theme's
    /// own yellow nudged toward orange, else a tuned amber, moved toward the text colour just
    /// far enough to read on every surface it is drawn on. Working falls back to the accent
    /// when it would sit next to the amber, as in `with_amber_needs`.
    fn with_distinct_needs(mut self) -> Theme {
        let ink = if contrast(self.bg, Color::Rgb(0, 0, 0))
            > contrast(self.bg, Color::Rgb(255, 255, 255))
        {
            Color::Rgb(0, 0, 0)
        } else {
            Color::Rgb(255, 255, 255)
        };
        let light = ink == Color::Rgb(0, 0, 0);
        let tuned = if light { AMBER_ON_LIGHT } else { AMBER_ON_DARK };
        let deep = if light {
            DEEP_AMBER_ON_LIGHT
        } else {
            DEEP_AMBER_ON_DARK
        };
        let flat_terminal = mix(self.bg, Color::Rgb(0, 0, 0), 0.4);
        let surfaces = [
            self.bg,
            self.sidebar_bg,
            self.card,
            self.card2,
            self.hov,
            self.sel_bg(),
            flat_terminal,
        ];
        let others = [self.err, self.done, self.accent, self.sky(), self.violet()];
        let kept = surfaces
            .iter()
            .all(|s| contrast(self.blocked, *s) >= LIT_READS);
        let working_ok = |t: &Theme| {
            !colors_alike(t.working, t.blocked)
                && !colors_alike(t.on_lit(t.working), t.on_lit(t.blocked))
        };
        let clear = |t: &Theme| {
            others.iter().all(|o| !colors_alike(t.blocked, *o))
                && !colors_alike(t.on_lit(t.blocked), t.on_lit(t.err))
                && !colors_alike(t.on_lit(t.blocked), t.on_lit(t.done))
        };
        if kept && clear(&self) && working_ok(&self) {
            return self;
        }
        let mut candidates = vec![tuned, deep, GOLD, RED_ORANGE, MID_ORANGE, PALE_AMBER];
        if let Some(a) = &self.ansi {
            candidates.insert(0, mix(a[2], tuned, 0.5));
        }
        let original_working = self.working;
        let mut chosen = None;
        for c in candidates {
            let mut t = self.clone();
            t.blocked = readable_on(c, ink, &surfaces, LIT_READS);
            if !clear(&t) {
                continue;
            }
            if !working_ok(&t) {
                t.working = readable_on(t.accent, ink, &[t.sidebar_bg, t.hov, flat_terminal], 4.5);
                if !working_ok(&t) || colors_alike(t.working, t.done) {
                    t.working = original_working;
                    continue;
                }
            }
            chosen = Some(t);
            break;
        }
        match chosen {
            Some(t) => t,
            None => {
                self.blocked = readable_on(tuned, ink, &surfaces, LIT_READS);
                self
            }
        }
    }

    pub fn apply(&mut self, o: &ThemeOverrides) {
        for (k, v) in &o.0 {
            let Some(c) = parse_color(v) else { continue };
            let slot = match k.replace('-', "_").as_str() {
                "bg" => &mut self.bg,
                "fg" => &mut self.fg,
                "muted" | "dim" => &mut self.muted,
                "accent" | "acc" => &mut self.accent,
                "border" => &mut self.border,
                "border_active" => &mut self.border_active,
                "sidebar_bg" | "surf" => &mut self.sidebar_bg,
                "selection_bg" => &mut self.selection_bg,
                "tab_active_bg" => &mut self.tab_active_bg,
                "tab_active_fg" => &mut self.tab_active_fg,
                "working" => &mut self.working,
                "blocked" | "needs" => &mut self.blocked,
                "done" | "ok" => &mut self.done,
                "idle" => &mut self.idle,
                "card" => &mut self.card,
                "card2" => &mut self.card2,
                "btn" => &mut self.btn,
                "hov" => &mut self.hov,
                "line" => &mut self.line,
                "text" => &mut self.text,
                "strong" => &mut self.strong,
                "acc_ink" => &mut self.acc_ink,
                "err" => &mut self.err,
                _ => continue,
            };
            *slot = c;
        }
    }

    /// A project's colour by its index: the theme's four, then yellow and dim.
    pub fn project(&self, i: usize, fallback: &[Color]) -> Color {
        match (&self.ws, &self.ansi) {
            (Some(ws), Some(a)) => [ws[0], ws[1], ws[2], ws[3], a[2], self.muted][i % 6],
            _ if !fallback.is_empty() => fallback[i % fallback.len()],
            _ => self.accent,
        }
    }

    /// Leader mode's colour (sky), and nothing else's: the armed pill, its border, the key map.
    pub fn sky(&self) -> Color {
        self.ws
            .map(|w| w[1])
            .unwrap_or(Color::Rgb(0x5a, 0xa9, 0xff))
    }

    /// The workspace palette's cyan.
    pub fn cyan(&self) -> Color {
        self.ws
            .map(|w| w[3])
            .unwrap_or(Color::Rgb(0x4e, 0xe0, 0xfa))
    }

    /// Grid's pane colours, in pane order (green, pink, violet, cyan in the design), from roles
    /// every theme has. Identity, not state: they repeat from the fifth pane. A theme whose
    /// roles share a colour (an accent that is also its violet) moves on to the next role, so
    /// no two panes in a row look alike.
    pub fn grid_colors(&self) -> [Color; 4] {
        let pink = self.ws.map(|w| w[2]);
        let roles = [
            Some(self.done),
            Some(self.accent),
            Some(self.violet()),
            Some(self.cyan()),
            pink,
            Some(self.sky()),
            Some(self.working),
        ];
        let terminal = self.ansi.into_iter().flatten().map(Some);
        let mut out: Vec<Color> = Vec::with_capacity(4);
        for c in roles
            .into_iter()
            .chain(terminal)
            .chain([Some(self.blocked)])
            .flatten()
        {
            if out.len() < 4 && !out.iter().any(|o| colors_alike(*o, c)) {
                out.push(c);
            }
        }
        while out.len() < 4 {
            out.push(self.strong);
        }
        [out[0], out[1], out[2], out[3]]
    }

    /// A check that could not run, or that you chose never to run: not a failure, so not red.
    /// The theme's violet, moved toward its strongest text where it wouldn't read on the
    /// ground, a card or the selected row (some themes' violet is a dim terminal blue).
    pub fn violet(&self) -> Color {
        let base = self
            .ws
            .map(|w| w[0])
            .unwrap_or(Color::Rgb(0xb5, 0x8c, 0xff));
        readable_on(
            base,
            self.strong,
            &[self.bg, self.card, self.sel_bg()],
            LIT_READS,
        )
    }

    pub fn status(&self, s: crate::protocol::Status) -> Color {
        use crate::protocol::Status::*;
        match s {
            Working => self.working,
            Blocked => self.blocked,
            Done => self.done,
            Idle => self.idle,
            None => self.muted,
        }
    }
}

#[cfg(test)]
mod grid {
    use super::{BUILTIN, Theme};

    #[test]
    fn every_theme_gives_grid_four_different_pane_colours() {
        for name in BUILTIN {
            let c = Theme::named(name).grid_colors();
            for i in 0..c.len() {
                for j in i + 1..c.len() {
                    assert_ne!(c[i], c[j], "{name}: Grid colours {i} and {j} are the same");
                }
            }
        }
    }
}

#[cfg(test)]
mod audit {
    use super::Theme;
    use ratatui::style::Color;

    #[test]
    fn builtins_are_unique_and_resolve_without_falling_back() {
        let mut names = std::collections::HashSet::new();
        for &name in super::BUILTIN {
            assert!(names.insert(name), "duplicate theme {name}");
            let theme = Theme::named(name);
            if name != super::DEFAULT {
                assert_ne!(
                    theme.bg,
                    Theme::named(super::DEFAULT).bg,
                    "{name} fell back to the default"
                );
            }
            if name != "mono" {
                assert!(
                    matches!(theme.bg, Color::Rgb(..)),
                    "invalid background for {name}"
                );
            }
        }
    }

    #[test]
    fn daemon_snapshot_colors_keep_the_rgb_defaults() {
        for &name in super::BUILTIN {
            let theme = Theme::named(name);
            let foreground = match theme.fg {
                Color::Rgb(red, green, blue) => (red, green, blue),
                _ => (0xc9, 0xd1, 0xd9),
            };
            let background = match theme.bg {
                Color::Rgb(red, green, blue) => (red, green, blue),
                _ => (0x07, 0x0b, 0x10),
            };
            assert_eq!(
                theme.terminal_foreground_background_rgb(),
                (foreground, background),
                "{name}"
            );
        }
    }

    #[test]
    fn old_theme_names_load_the_theme_they_became() {
        for (old, new) in super::ALIASES {
            assert!(
                super::BUILTIN.contains(new),
                "{old} points at {new}, which is not built in"
            );
            assert!(
                !super::BUILTIN.contains(old),
                "{old} is both an alias and a theme"
            );
            assert_eq!(
                Theme::named(old).bg,
                Theme::named(new).bg,
                "{old} loads {new}"
            );
        }
    }

    fn lum(c: Color) -> Option<f64> {
        let Color::Rgb(r, g, b) = c else { return None };
        let ch = |v: u8| {
            let s = v as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        Some(0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b))
    }

    pub fn contrast(a: Color, b: Color) -> f64 {
        match (lum(a), lum(b)) {
            (Some(x), Some(y)) => (x.max(y) + 0.05) / (x.min(y) + 0.05),
            _ => 21.0,
        }
    }

    fn mix(a: Color, b: Color, t: f32) -> Color {
        super::mix(a, b, t)
    }

    /// Dark or light is read from the ground, so the browser needs no list kept by hand: a
    /// theme named for its light variant is light, one named dark is dark, and the defaults
    /// sit on the sides the config expects.
    #[test]
    fn a_theme_is_light_when_its_ground_is() {
        let mut bad = Vec::new();
        for &n in super::BUILTIN {
            let light = super::is_light(n);
            if n.ends_with("-light") && !light {
                bad.push(format!("{n} is named light but drawn dark"));
            }
            if n.ends_with("-dark") && light {
                bad.push(format!("{n} is named dark but drawn light"));
            }
        }
        assert!(
            bad.is_empty(),
            "{}",
            bad.join(
                "
"
            )
        );
        assert!(super::is_light("day") && !super::is_light(super::DEFAULT));
        assert!(
            !super::is_light("mono"),
            "a removed theme that has no ground of its own reads as dark"
        );
        // Nothing sits near the line: a ground is clearly light or clearly dark, so the tab a
        // theme lands on is never a judgement call.
        let mut near = Vec::new();
        for &n in super::BUILTIN {
            if let Color::Rgb(r, g, b) = Theme::named(n).bg {
                let lum =
                    (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0;
                if (0.25..0.6).contains(&lum) {
                    near.push(format!("{n}: {lum:.2}"));
                }
            }
        }
        assert!(near.is_empty(), "grounds near the line: {near:?}");
        assert!(!super::is_light("not-a-theme"));
        let light = super::BUILTIN.iter().filter(|n| super::is_light(n)).count();
        assert_eq!(light, 8, "eight light themes");
        assert_eq!(super::BUILTIN.len() - light, 8, "and eight dark ones");
    }

    /// Every theme gives each status its own colour, as the design's roles do: working,
    /// needs you (amber), failed, done, checking and could not run.
    #[test]
    fn every_theme_gives_each_status_its_own_colour() {
        let mut bad = Vec::new();
        for &n in super::BUILTIN {
            let t = Theme::named(n);
            let roles = [
                ("working", t.accent),
                ("needs you", t.blocked),
                ("failed", t.err),
                ("done", t.done),
                ("checking", t.sky()),
                ("could not run", t.violet()),
            ];
            for (i, (a, ca)) in roles.iter().enumerate() {
                for (b, cb) in &roles[i + 1..] {
                    // Needs you must differ from every status; the older pairs among the
                    // rest are held by the other audit or belong to the theme's own palette.
                    if (*a == "needs you" || *b == "needs you") && super::colors_alike(*ca, *cb) {
                        bad.push(format!("{n}: {a} and {b} look alike"));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// Every theme: text readable on every surface it's drawn on, and the colours that mean
    /// different things look different.
    #[test]
    fn every_theme_reads_well() {
        let names = super::BUILTIN;
        let mut bad = Vec::new();
        for &n in names {
            let t = Theme::named(n);
            let active = mix(t.sidebar_bg, t.accent, 0.16);
            let hover = mix(t.sidebar_bg, t.text, 0.10);
            let flat_terminal = mix(t.bg, Color::Black, 0.4);
            let checks: Vec<(&str, Color, Color, f64)> = vec![
                ("text on bg", t.text, t.bg, 4.5),
                ("strong on bg", t.strong, t.bg, 7.0),
                ("muted on bg", t.muted, t.bg, 3.0),
                ("muted on sidebar", t.muted, t.sidebar_bg, 3.0),
                ("text on card", t.text, t.card, 4.5),
                ("text on card2", t.text, t.card2, 4.0),
                ("strong on flat terminal", t.strong, flat_terminal, 4.5),
                ("text on flat terminal", t.text, flat_terminal, 4.5),
                ("muted on flat terminal", t.muted, flat_terminal, 3.0),
                ("blocked on flat terminal", t.blocked, flat_terminal, 3.0),
                ("done on flat terminal", t.done, flat_terminal, 3.0),
                ("working on flat terminal", t.working, flat_terminal, 4.5),
                ("strong on btn", t.strong, t.btn, 4.5),
                ("accent on btn (keys)", t.accent, t.btn, 3.0),
                ("ink on accent", t.acc_ink, t.accent, 4.5),
                ("strong on open row", t.strong, active, 7.0),
                ("muted on open row", t.muted, active, 2.6),
                ("strong on hover", t.strong, hover, 7.0),
                ("strong on hov (menus)", t.strong, t.hov, 4.5),
                ("blocked on sidebar", t.blocked, t.sidebar_bg, 3.0),
                ("done on sidebar", t.done, t.sidebar_bg, 3.0),
                ("working on sidebar", t.working, t.sidebar_bg, 4.5),
                // The selected row is lit with `hov`, and its name keeps its state's colour.
                ("blocked on the lit row", t.on_lit(t.blocked), t.hov, 3.0),
                ("done on the lit row", t.on_lit(t.done), t.hov, 3.0),
                ("working on the lit row", t.on_lit(t.working), t.hov, 3.0),
                ("err on card", t.err, t.card, 3.0),
                ("violet on bg", t.violet(), t.bg, 3.0),
                ("violet on card", t.violet(), t.card, 3.0),
                ("violet on the selected row", t.violet(), t.sel_bg(), 3.0),
                (
                    "ink on the red Close button",
                    t.ink_on(t.danger_fill()),
                    t.danger_fill(),
                    4.5,
                ),
            ];
            for (what, fg, bg, min) in checks {
                let c = contrast(fg, bg);
                if c < min {
                    bad.push(format!("{n:18} {what:24} {c:.2} (wants {min})"));
                }
            }
            // Different meanings, different colours.
            for (what, a, b) in [
                ("accent vs blocked", t.accent, t.blocked),
                ("blocked vs err", t.blocked, t.err),
                ("done vs blocked", t.done, t.blocked),
                // Needs you is amber, apart from failed red; working and done must differ.
                ("working vs done", t.working, t.done),
                ("working vs blocked", t.working, t.blocked),
                // And still on the lit row, where each may have moved toward white.
                (
                    "lit done vs lit blocked",
                    t.on_lit(t.done),
                    t.on_lit(t.blocked),
                ),
                (
                    "lit err vs lit blocked",
                    t.on_lit(t.err),
                    t.on_lit(t.blocked),
                ),
                (
                    "lit working vs lit done",
                    t.on_lit(t.working),
                    t.on_lit(t.done),
                ),
                (
                    "lit working vs lit blocked",
                    t.on_lit(t.working),
                    t.on_lit(t.blocked),
                ),
            ] {
                if super::colors_alike(a, b) {
                    bad.push(format!("{n:18} {what:24} too alike"));
                }
            }
            // Open and hover rows must differ from the sidebar and each other.
            if contrast(active, t.sidebar_bg) < 1.12 || contrast(hover, t.sidebar_bg) < 1.08 {
                bad.push(format!(
                    "{n:18} open/hover rows barely show ({:.2} / {:.2})",
                    contrast(active, t.sidebar_bg),
                    contrast(hover, t.sidebar_bg)
                ));
            }
        }
        for b in &bad {
            eprintln!("{b}");
        }
        assert!(bad.is_empty(), "{} theme problems", bad.len());
    }
}

#[cfg(test)]
mod maitty {
    use super::{Theme, contrast, hex, maitty_token};

    const AA: f64 = 4.5;

    /// Text pairs the tokens file promises meet WCAG AA: its `*Text` colours and body text on its grounds.
    fn pairs(side: &str) -> Vec<(String, String)> {
        let grounds = ["bg.page", "bg.card", "bg.surface", "bg.terminal"];
        let texts = ["text.strong", "text.body", "text.dim", "state.workingText", "state.needsText", "state.doneText", "accent.brassText"];
        let mut out = Vec::new();
        for text in texts {
            for ground in grounds {
                out.push((text.to_string(), ground.to_string()));
            }
        }
        // The fill colours carry their own ink: brass, teal, coral and green fills each take their `*On`.
        for (fill, on) in [("accent.brass", "accent.brassOn"), ("state.working", "state.workingOn"), ("state.needs", "state.needsOn"), ("state.done", "state.doneOn")] {
            out.push((on.to_string(), fill.to_string()));
        }
        let _ = side;
        out
    }

    #[test]
    fn every_text_pair_the_tokens_file_names_meets_aa_in_all_four_maitty_themes() {
        let mut short = Vec::new();
        for side in ["night", "day", "dark", "light"] {
            for (fg, bg) in pairs(side) {
                let ratio = contrast(hex(&maitty_token(side, &fg)), hex(&maitty_token(side, &bg)));
                if ratio < AA {
                    short.push(format!("{side}: {fg} on {bg} is {ratio:.2}"));
                }
            }
        }
        assert!(short.is_empty(), "{}", short.join("\n"));
    }

    #[test]
    fn the_tui_night_and_day_are_the_tokens_files_colours() {
        for (name, side) in [("night", "night"), ("day", "day"), ("maitty-dark", "dark"), ("maitty-light", "light")] {
            let theme = Theme::named(name);
            assert_eq!(theme.bg, hex(&maitty_token(side, "bg.page")), "{name} ground");
            assert_eq!(theme.fg, hex(&maitty_token(side, "text.body")), "{name} text");
            assert_eq!(theme.card, hex(&maitty_token(side, "bg.card")), "{name} card");
            assert_eq!(theme.strong, hex(&maitty_token(side, "text.strong")), "{name} strong");
            assert_eq!(theme.working, hex(&maitty_token(side, "state.workingText")), "{name}: teal is working");
            assert_eq!(theme.blocked, hex(&maitty_token(side, "state.needsText")), "{name}: coral is needs you");
        }
        // The accent is the brass on the dark ground, and its darker text colour on the light one.
        assert_eq!(Theme::named("night").accent, hex(&maitty_token("night", "accent.brass")));
        assert_eq!(Theme::named("day").accent, hex(&maitty_token("day", "accent.brassText")));
        assert_eq!(Theme::named("maitty-dark").accent, hex(&maitty_token("dark", "accent.brass")));
        assert_eq!(Theme::named("maitty-light").accent, hex(&maitty_token("light", "accent.brassText")));
    }

    #[test]
    fn working_and_needs_you_are_two_different_colours_and_neither_is_the_error_red() {
        for name in ["night", "day", "maitty-dark", "maitty-light"] {
            let t = Theme::named(name);
            assert_ne!(t.working, t.blocked, "{name}");
            assert_ne!(t.err, t.blocked, "{name}: an error must not read as needs-you");
            assert_ne!(t.err, t.working, "{name}");
        }
    }

    #[test]
    fn the_settings_list_names_them_maitty_night_and_maitty_day_and_the_old_ids_still_load() {
        let label = |id: &str| super::DESIGN.iter().find(|(name, _)| *name == id).map(|(_, shown)| *shown);
        assert_eq!(label("night"), Some("Maitty Night"));
        assert_eq!(label("day"), Some("Maitty Day"));
        assert_eq!(label("maitty-dark"), Some("Maitty Dark"));
        assert_eq!(label("maitty-light"), Some("Maitty Light"));
        assert_eq!(super::canonical("maitty-night"), "night");
        assert_eq!(super::canonical("maitty-day"), "day");
    }

    #[test]
    fn maitty_dark_is_a_dark_ground_and_maitty_light_a_light_one_and_the_defaults_are_unchanged() {
        assert!(!super::is_light("maitty-dark") && !Theme::named("maitty-dark").is_light());
        assert!(super::is_light("maitty-light") && Theme::named("maitty-light").is_light());
        assert_eq!(super::DEFAULT, "night");
        assert_eq!(super::DEFAULT_LIGHT, "day");
        for id in ["maitty-dark", "maitty-light"] {
            assert!(super::BUILTIN.contains(&id), "{id} is a built-in theme");
        }
    }

    #[test]
    fn a_selected_row_is_seen_on_every_ground_and_keeps_its_text_readable() {
        for id in ["night", "day", "maitty-dark", "maitty-light", "dracula", "gruvbox-light"] {
            let t = Theme::named(id);
            for ground in [t.bg, t.card] {
                let on = t.sel_on(ground);
                assert!(contrast(on, ground) >= 1.25, "{id}: the selection is seen against its ground");
                assert!(contrast(t.strong, on) >= 4.5, "{id}: strong text reads on the selection");
            }
        }
    }
}

#[cfg(test)]
mod manifest {
    use super::*;

    /// `themes.json`, the list of themes both apps ship; the GUI builds its themes from it too.
    fn themes() -> Vec<serde_json::Value> {
        let file: serde_json::Value = serde_json::from_str(include_str!("../themes.json")).expect("themes.json is JSON");
        file["themes"].as_array().expect("a themes list").clone()
    }

    fn text<'a>(theme: &'a serde_json::Value, key: &str) -> &'a str {
        theme[key].as_str().unwrap_or_else(|| panic!("{key} is a string"))
    }

    #[test]
    fn the_built_in_themes_are_exactly_the_manifests_sixteen() {
        let manifest = themes();
        let mut ids: Vec<&str> = manifest.iter().map(|t| text(t, "id")).collect();
        let mut built: Vec<&str> = BUILTIN.to_vec();
        ids.sort_unstable();
        built.sort_unstable();
        assert_eq!(ids.len(), 16);
        assert_eq!(built, ids, "BUILTIN and themes.json list the same themes");
    }

    #[test]
    fn each_theme_has_the_manifests_display_name_and_the_manifests_side() {
        for theme in themes() {
            let id = text(&theme, "id");
            let name = DESIGN.iter().find(|(i, _)| *i == id).map(|(_, n)| *n);
            assert_eq!(name, Some(text(&theme, "name")), "{id}: display name");
            assert_eq!(is_light(id), theme["light"].as_bool().expect("a light flag"), "{id}: light or dark");
        }
    }

    #[test]
    fn a_family_themes_ground_and_text_are_the_manifests() {
        for theme in themes().iter().filter(|t| t.get("bg").is_some()) {
            let id = text(theme, "id");
            let tui = Theme::named(id);
            assert_eq!(tui.bg, hex(text(theme, "bg")), "{id}: ground");
            assert_eq!(tui.fg, hex(text(theme, "fg")), "{id}: text");
        }
    }

    #[test]
    fn a_removed_theme_loads_a_survivor_on_the_same_side_it_had() {
        for (old, new) in ALIASES {
            assert!(BUILTIN.contains(new), "{old} -> {new} is built in");
        }
        // The side a removed theme was drawn on is the side its survivor is drawn on.
        for (old, light) in [("dracula", false), ("nord", false), ("mono", false), ("papercolor-light", true), ("modus-operandi", true), ("rose-pine-dawn", true)] {
            assert_eq!(is_light(old), light, "{old} keeps its side");
        }
        assert_eq!(canonical("dracula"), "one-dark");
        assert_eq!(canonical("gruvbox-material"), canonical("gruvbox-material-dark"));
        assert_eq!(canonical("maitty-night"), "night");
        assert_eq!(canonical("github-dark"), "github-dark");
    }
}
