//! Built-in themes plus per-key overrides from config. Pane contents keep the program's own
//! colours; the theme only paints hydra's chrome (sidebar, title bars, status line).
//!
//! The roles follow the design handoff: `surf` for chrome, `card`/`card2` for panels and
//! action rows, `btn`/`hov` for buttons and the hovered row, `acc` (teal in Seshi Dark
//! theme) for focus, `needs` (amber) only for "needs you".

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}

/// Every field optional, as hex strings ("#89b4fa") or names ("red", "reset", "123").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeOverrides(pub BTreeMap<String, String>);

/// The theme when none is chosen.
pub const DEFAULT: &str = "seshi-dark";

pub const BUILTIN: &[&str] = &[
    "seshi-dark",
    "seshi-light",
    "seshi-walnut",
    "seshi-night",
    "seshi-day",
    "harbor",
    "harbor-day",
    "moss",
    "moss-day",
    "ember",
    "ash",
    "ink",
    "paper",
    "arcade",
    "hydra",
    "tinkery",
    "tinkery-light",
    "aurora",
    "moonlit",
    "papercolor-dark",
    "tango-dark",
    "monokai",
    "tokyo-night",
    "catppuccin-mocha",
    "catppuccin-latte",
    "gruvbox",
    "nord",
    "dracula",
    "apprentice",
    "ayu-dark",
    "ayu-light",
    "ayu-mirage",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "cobalt2",
    "everforest-dark",
    "everforest-light",
    "gotham",
    "gruvbox-light",
    "iceberg",
    "jellybeans",
    "kanagawa-dragon",
    "kanagawa-lotus",
    "kanagawa-wave",
    "lucario",
    "moonfly",
    "night-owl",
    "nightfly",
    "one-dark",
    "one-half-dark",
    "one-half-light",
    "panda",
    "rose-pine",
    "rose-pine-dawn",
    "rose-pine-moon",
    "shades-of-purple",
    "solarized-dark",
    "solarized-light",
    "sonokai",
    "tokyo-night-day",
    "tokyo-night-moon",
    "tokyo-night-storm",
    "zenbones-dark",
    "zenbones-light",
    "nightfox",
    "duskfox",
    "carbonfox",
    "github-dark",
    "github-dark-dimmed",
    "github-light",
    "gruvbox-material",
    "gruvbox-baby",
    "oceanic-next",
    "tomorrow",
    "tomorrow-night",
    "tomorrow-night-eighties",
    "tomorrow-night-bright",
    "base16-default-dark",
    "palenight",
    "one-light",
    "monokai-pro",
    "oxocarbon",
    "cyberdream",
    "nordic",
    "onenord",
    "edge",
    "vscode",
    "code-dark",
    "melange",
    "falcon",
    "embark",
    "lackluster",
    "miasma",
    "bamboo",
    "mellifluous",
    "mellow",
    "kanagawa-paper",
    "zephyr",
    "koda",
    "aquarium",
    "fluoromachine",
    "synthwave-84",
    "horizon",
    "material",
    "bluloco",
    "modus-vivendi",
    "modus-operandi",
    "mono",
];

/// The themes shown by name in Settings, Seshi's own first.
pub const DESIGN: &[(&str, &str)] = &[
    ("seshi-dark", "Seshi Dark"),
    ("seshi-light", "Seshi Light"),
    ("seshi-walnut", "Seshi Walnut"),
    ("seshi-night", "Seshi Night"),
    ("seshi-day", "Seshi Day"),
    ("harbor", "Harbor"),
    ("harbor-day", "Harbor Day"),
    ("moss", "Moss"),
    ("moss-day", "Moss Day"),
    ("ember", "Ember"),
    ("ash", "Ash"),
    ("ink", "Ink"),
    ("paper", "Paper"),
    ("arcade", "Arcade"),
    ("hydra", "Hydra"),
    ("tinkery", "Tinkery"),
    ("tinkery-light", "Tinkery Light"),
    ("aurora", "Aurora"),
    ("moonlit", "Moonlit"),
    ("papercolor-dark", "PaperColor Dark"),
    ("tango-dark", "Tango Dark"),
    ("monokai", "Monokai"),
    ("tokyo-night", "Tokyo Night"),
    ("apprentice", "Apprentice"),
    ("ayu-dark", "Ayu Dark"),
    ("ayu-light", "Ayu Light"),
    ("ayu-mirage", "Ayu Mirage"),
    ("catppuccin-frappe", "Catppuccin Frappé"),
    ("catppuccin-latte", "Catppuccin Latte"),
    ("catppuccin-macchiato", "Catppuccin Macchiato"),
    ("catppuccin-mocha", "Catppuccin Mocha"),
    ("cobalt2", "Cobalt2"),
    ("dracula", "Dracula"),
    ("everforest-dark", "Everforest Dark"),
    ("everforest-light", "Everforest Light"),
    ("gotham", "Gotham"),
    ("gruvbox", "Gruvbox Dark"),
    ("gruvbox-light", "Gruvbox Light"),
    ("iceberg", "Iceberg"),
    ("jellybeans", "Jellybeans"),
    ("kanagawa-dragon", "Kanagawa Dragon"),
    ("kanagawa-lotus", "Kanagawa Lotus"),
    ("kanagawa-wave", "Kanagawa Wave"),
    ("lucario", "Lucario"),
    ("moonfly", "Moonfly"),
    ("night-owl", "Night Owl"),
    ("nightfly", "Nightfly"),
    ("nord", "Nord"),
    ("one-dark", "One Dark / Atom One"),
    ("one-half-dark", "One Half Dark"),
    ("one-half-light", "One Half Light"),
    ("panda", "Panda"),
    ("rose-pine", "Rosé Pine"),
    ("rose-pine-dawn", "Rosé Pine Dawn"),
    ("rose-pine-moon", "Rosé Pine Moon"),
    ("shades-of-purple", "Shades of Purple"),
    ("solarized-dark", "Solarized Dark"),
    ("solarized-light", "Solarized Light"),
    ("sonokai", "Sonokai"),
    ("tokyo-night-day", "Tokyo Night Day"),
    ("tokyo-night-moon", "Tokyo Night Moon"),
    ("tokyo-night-storm", "Tokyo Night Storm"),
    ("zenbones-dark", "Zenbones Dark"),
    ("zenbones-light", "Zenbones Light"),
    ("nightfox", "Nightfox"),
    ("duskfox", "Duskfox"),
    ("carbonfox", "Carbonfox"),
    ("github-dark", "GitHub Dark"),
    ("github-dark-dimmed", "GitHub Dark Dimmed"),
    ("github-light", "GitHub Light"),
    ("gruvbox-material", "Gruvbox Material"),
    ("gruvbox-baby", "Gruvbox Baby"),
    ("oceanic-next", "Oceanic Next"),
    ("tomorrow", "Tomorrow"),
    ("tomorrow-night", "Tomorrow Night"),
    ("tomorrow-night-eighties", "Tomorrow Night Eighties"),
    ("tomorrow-night-bright", "Tomorrow Night Bright"),
    ("base16-default-dark", "Base16 Default Dark"),
    ("palenight", "Palenight"),
    ("one-light", "One Light"),
    ("monokai-pro", "Monokai Pro"),
    ("oxocarbon", "Oxocarbon"),
    ("cyberdream", "Cyberdream"),
    ("nordic", "Nordic"),
    ("onenord", "OneNord"),
    ("edge", "Edge"),
    ("vscode", "VSCode Dark"),
    ("code-dark", "Code Dark"),
    ("melange", "Melange"),
    ("falcon", "Falcon"),
    ("embark", "Embark"),
    ("lackluster", "Lackluster"),
    ("miasma", "Miasma"),
    ("bamboo", "Bamboo"),
    ("mellifluous", "Mellifluous"),
    ("mellow", "Mellow"),
    ("kanagawa-paper", "Kanagawa Paper Ink"),
    ("zephyr", "Zephyr"),
    ("koda", "Koda"),
    ("aquarium", "Aquarium"),
    ("fluoromachine", "Fluoromachine"),
    ("synthwave-84", "SynthWave '84"),
    ("horizon", "Horizon"),
    ("material", "Material Darker"),
    ("bluloco", "Bluloco Dark"),
    ("modus-vivendi", "Modus Vivendi"),
    ("modus-operandi", "Modus Operandi"),
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
                if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
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
            let m = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * t.clamp(0.0, 1.0)).round() as u8;
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
        if surfaces.iter().all(|surface| contrast(candidate, *surface) >= minimum) {
            return candidate;
        }
    }
    ink
}

fn colors_alike(a: Color, b: Color) -> bool {
    const MIN_COLOR_DISTANCE: i32 = 90;
    const MIN_COLOR_CONTRAST: f64 = 1.15;
    let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) else { return a == b };
    let distance = (r1 as i32 - r2 as i32).abs() + (g1 as i32 - g2 as i32).abs() + (b1 as i32 - b2 as i32).abs();
    a == b || contrast(a, b) < MIN_COLOR_CONTRAST && distance < MIN_COLOR_DISTANCE
}

impl Theme {
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
        let acc_ink = if contrast(black, accent) > contrast(white, accent) { black } else { white };
        let done = readable_on(ansi[1], ink, &[sidebar, hov], 3.0);
        let blocked = readable_on(ansi[0], ink, &[sidebar, hov], 3.0);
        let mut working = readable_on(ansi[2], ink, &[sidebar, hov], 4.5);
        // Muted terminal yellows can look like green or red. Status must remain distinguishable.
        if colors_alike(working, done) || colors_alike(working, blocked) {
            working = readable_on(mix(ansi[2], Color::Rgb(255, 96, 0), 0.85), ink, &[sidebar, hov], 4.5);
        }
        let line = mix(bg, fg, 0.22);
        Theme {
            bg, fg, muted, accent,
            border: line,
            border_active: accent,
            sidebar_bg: sidebar,
            selection_bg: hov,
            tab_active_bg: accent,
            tab_active_fg: acc_ink,
            working, blocked, done,
            idle: muted,
            card, card2, btn, hov, line, text, strong, acc_ink,
            err: readable_on(ansi[0], ink, &[card], 3.0),
            ws: Some([ansi[4], ansi[3], ansi[0], ansi[5]]),
            ansi: Some(ansi),
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

    /// The selected row's ground: the background a step toward the text, so it reads on any
    /// theme without a colour of its own.
    pub fn sel_bg(&self) -> Color {
        mix(self.bg, self.text, 0.12)
    }

    /// The selected row's ground on a raised surface (a sheet, a popup): `ground` a step toward
    /// the text.
    pub fn sel_on(&self, ground: Color) -> Color {
        mix(ground, self.text, 0.12)
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
                    if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
                };
                0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
            }
            _ => 0.5,
        };
        let contrast = |a: f64, b: f64| (a.max(b) + 0.05) / (a.min(b) + 0.05);
        let fill = l(c);
        if contrast(l(self.bg), fill) >= contrast(l(self.strong), fill) { self.bg } else { self.strong }
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
        }
    }

    /// A theme from the design handoff's roles.
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
        }
    }

    pub fn named(name: &str) -> Theme {
        let theme = Theme::base(name);
        if name == "seshi-dark" || name == "mono" {
            return theme;
        }
        theme.with_distinct_needs()
    }

    fn base(name: &str) -> Theme {
        match name {
            "seshi-dark" => Theme::design(
                [
                    "#10141b", "#dce4ee", "#141a23", "#1a222d", "#202a37", "#344252", "#8695a8", "#c1ccda",
                    "#f4f7fb", "#43c7c4", "#0c2022", "#f1ad4f", "#92d477", "#ff6678", "#273443", "#314456",
                ],
                ["#b393f5", "#5aa9ff", "#e581b5", "#4dc8c1"],
                ["#ff6678", "#92d477", "#f1ad4f", "#65b6e8", "#bb93f5", "#4dc8c1", "#8695a8"],
            )
            .with_err("#ff6678")
            .with_amber_needs("#f4c24d"),
            "seshi-light" => Theme::design(
                [
                    "#f5f7fa", "#243244", "#e9eef4", "#ffffff", "#edf2f7", "#c8d2de", "#68788b", "#43546a",
                    "#172536", "#087f86", "#f5ffff", "#915300", "#287844", "#c33343", "#dfe7ef", "#d4e0e9",
                ],
                ["#6753b7", "#087f86", "#af3973", "#2c6e9d"],
                ["#c33343", "#287844", "#915300", "#2c6e9d", "#7350a8", "#087f86", "#68788b"],
            )
            .with_err("#b3263a"),
            // A walnut-cased keyboard with chocolate keys: brown grounds, turquoise for focus,
            // the red, orange and lemon-lime run for needs you, working and done, cream text,
            // lemon (ws[1]) for leader mode.
            "seshi-walnut" => Theme::design(
                [
                    "#1f140d", "#f1e4d4", "#281a12", "#2f2017", "#3b291e", "#4a3326", "#a68b77", "#e8d8c4",
                    "#fff4e6", "#3fc6cc", "#0f2427", "#f58a32", "#c6d64c", "#ff5a3c", "#4a3326", "#533a2c",
                ],
                ["#3fc6cc", "#ece65a", "#f58a32", "#ff5a3c"],
                ["#ff5a3c", "#b9d64a", "#ece65a", "#5cb8ea", "#e57fb8", "#3fc6cc", "#a68b77"],
            )
            .with_err("#ff4f6e"),
            // Golden hour after the session: warm driftwood dark, sunset peach for focus, sun
            // for what needs you, sea glass (ws[1]) for leader mode.
            "seshi-night" => Theme::design(
                [
                    "#191411", "#e6d8ca", "#221b17", "#261e19", "#30261f", "#33291f", "#9b8a7c", "#d9cbbd",
                    "#fff6ec", "#ff9e7a", "#2a1208", "#d9cbbd", "#8bd17c", "#ffd25e", "#372b23", "#43352b",
                ],
                ["#c3a6ff", "#6fd3c4", "#ff9fb8", "#7fd8c0"],
                ["#f2777a", "#9fd27f", "#f0c66b", "#7fb4e6", "#d4a5f5", "#6fd3c4", "#9b8a7c"],
            )
            .with_err("#ff5d73"),
            // The same beach in daylight: sand, deep coral for focus, gold for what needs you.
            "seshi-day" => Theme::design(
                [
                    "#fbf3eb", "#4a3c33", "#f4e7da", "#fffaf5", "#f6ebdf", "#e2cfbd", "#8f7b6c", "#5a4a3f",
                    "#2a1f18", "#b8492c", "#fff7f0", "#5a4a3f", "#3f8f45", "#a06a00", "#efdfcf", "#e7d2bf",
                ],
                ["#7b5cd6", "#13867c", "#c2477a", "#1d8f86"],
                ["#c0392b", "#3f8f45", "#a06a00", "#2f6fb0", "#8e4dbf", "#13867c", "#8f7b6c"],
            )
            .with_err("#c73a55"),
            // Seshi's other pairs, a dark one and its day, and one loud one.
            // A harbour at night: deep navy, brass for focus.
            "harbor" => Theme::design(
                [
                    "#0b1220", "#c9d3e0", "#101a2c", "#142036", "#1b2a44", "#263a5c", "#7f8ea6", "#b9c5d6",
                    "#f1f5fb", "#e0b25c", "#1a1203", "#f0a35a", "#6fcf97", "#ff6b7a", "#223452", "#2b4068",
                ],
                ["#e0b25c", "#5fb8d9", "#f0a35a", "#8fd3c1"],
                ["#ff6b7a", "#6fcf97", "#e8c56a", "#5fa8e8", "#b79cf0", "#5fc9d9", "#7f8ea6"],
            ),
            // The same harbour in fog: pale grey-blue, rope amber, harbour blue for focus.
            "harbor-day" => Theme::design(
                [
                    "#eef2f5", "#33424f", "#e3e9ee", "#f7f9fb", "#e9eef2", "#c5d0da", "#6f7f8d", "#3f4f5d",
                    "#14202b", "#1f5f8b", "#f5fafd", "#925400", "#1f7a4d", "#b3262f", "#d8e1e8", "#cfdae3",
                ],
                ["#1f5f8b", "#8a6a12", "#b0522a", "#1f7a6a"],
                ["#b3262f", "#1f7a4d", "#8a6a00", "#1f5f8b", "#7a4fb0", "#13807c", "#6f7f8d"],
            ),
            // A forest floor: greens on near-black, lime for focus.
            "moss" => Theme::design(
                [
                    "#0d120e", "#c8d3c4", "#121a13", "#172118", "#1f2c20", "#2c3d2d", "#7d8f7a", "#b7c4b2",
                    "#f0f6ec", "#9bd16c", "#0f1a08", "#e8b04a", "#5fd0a3", "#ff6f61", "#263626", "#2f4330",
                ],
                ["#9bd16c", "#e8b04a", "#6fc3d6", "#d9a0e0"],
                ["#ff6f61", "#8fd16c", "#e8c15a", "#6fb3e0", "#c79ae0", "#5fd0c0", "#7d8f7a"],
            ),
            // The same forest by day: paper and sage.
            "moss-day" => Theme::design(
                [
                    "#f3f4ec", "#3d4739", "#e8ebdf", "#fafbf5", "#edf0e4", "#c9d1bd", "#717b69", "#48523f",
                    "#1b2416", "#3f7a2e", "#f6fbf2", "#8f5a00", "#1d7d6a", "#b3312b", "#dde2d1", "#d3dac5",
                ],
                ["#3f7a2e", "#8f5a00", "#2f6f8f", "#8a4f9a"],
                ["#b3312b", "#3f7a2e", "#8a6a00", "#2f6fa0", "#8a4f9a", "#1d7d6a", "#717b69"],
            ),
            // Charcoal with a fire in it: red-orange for focus.
            "ember" => Theme::design(
                [
                    "#120d0b", "#d6c9c2", "#1a1310", "#211815", "#2b1f1b", "#3d2c26", "#93807a", "#c6b7af",
                    "#fbf1ea", "#ff6a3d", "#1f0a02", "#f2c14e", "#7fd18a", "#ff7ab8", "#36261f", "#452f27",
                ],
                ["#ff6a3d", "#f2c14e", "#6fb6d9", "#c79ae0"],
                ["#ff7ab8", "#7fd18a", "#f2c14e", "#6fa8e8", "#d28ae0", "#5fc9c0", "#93807a"],
            ),
            // What the fire leaves: warm grey-white, burnt orange for focus.
            "ash" => Theme::design(
                [
                    "#f4f1ee", "#45403b", "#eae5e0", "#fbf9f7", "#efeae5", "#cfc7bf", "#7a726a", "#4f4943",
                    "#1e1a17", "#c2410c", "#fff7f2", "#8a5b00", "#2f7d4f", "#b01e4a", "#e0d9d2", "#d6cdc4",
                ],
                ["#c2410c", "#8a5b00", "#2f6f9a", "#7a4fa0"],
                ["#b01e4a", "#2f7d4f", "#8a6a00", "#2f6fa0", "#7a4fa0", "#1d7d7a", "#7a726a"],
            ),
            // Nearly no colour: greys, one blue for focus, and the states.
            "ink" => Theme::design(
                [
                    "#0e0e10", "#cfcfd4", "#141417", "#1a1a1e", "#222227", "#303037", "#85858f", "#bdbdc6",
                    "#f5f5f7", "#7aa2ff", "#08122b", "#d9a441", "#74c98a", "#f0707a", "#2a2a31", "#34343d",
                ],
                ["#7aa2ff", "#b9b9c4", "#9a9aa6", "#dcdce4"],
                ["#f0707a", "#74c98a", "#d9a441", "#7aa2ff", "#b08ae0", "#6fc3c9", "#85858f"],
            ),
            // Ink's page: off-white and greys, one blue for focus.
            "paper" => Theme::design(
                [
                    "#f7f7f5", "#3a3a3a", "#ededea", "#ffffff", "#f1f1ee", "#cfcfca", "#74746f", "#454545",
                    "#141414", "#2f5fd0", "#f7faff", "#8a5a00", "#2a7a45", "#b3262f", "#e2e2de", "#d8d8d3",
                ],
                ["#2f5fd0", "#5a5a5a", "#7a7a7a", "#3a3a3a"],
                ["#b3262f", "#2a7a45", "#8a6a00", "#2f5fd0", "#7a4fb0", "#1d7d7a", "#74746f"],
            ),
            // The loud one: neon on black, magenta for focus.
            "arcade" => Theme::design(
                [
                    "#07060d", "#d9d6f0", "#0d0b18", "#131022", "#1b1730", "#2c2550", "#8a84b0", "#c8c4e6",
                    "#ffffff", "#ff3df2", "#1a0218", "#ffe14a", "#3dffa0", "#ff4f5e", "#251f44", "#332a5e",
                ],
                ["#ff3df2", "#3df2ff", "#ffe14a", "#3dffa0"],
                ["#ff4f5e", "#3dffa0", "#ffe14a", "#4f9dff", "#ff3df2", "#3df2ff", "#8a84b0"],
            ),
            // The design handoff's palette, the default before the rename ("drover" is the
            // app's first name).
            "hydra" | "drover" => Theme::design(
                [
                    "#070b10", "#c9d1d9", "#0c131b", "#0f1821", "#18242f", "#1f2c3a", "#71808f", "#a7b4c2",
                    "#f2f6f8", "#c3f53c", "#0a1204", "#ffb547", "#7fd962", "#ff6b6b", "#1d2a37", "#2a3a4c",
                ],
                ["#a593ff", "#5aa9ff", "#ff7ab6", "#3dd6c0"],
                ["#ff6b6b", "#7fd962", "#e8c565", "#5aa9ff", "#a593ff", "#3dd6c0", "#71808f"],
            ),
            // The four that follow are Tinkery's (the editor's), in its colours. Where one
            // missed the audit below, that tone is a shade off Tinkery's: the dim text in the
            // last three, and tinkery-light's amber and red.
            // Tinkery's own: a forge at night, ember orange on char.
            "tinkery" => Theme::design(
                [
                    "#110f0e", "#dcd2c7", "#171413", "#1f1b19", "#292320", "#40362f", "#7d7067", "#b5a89d",
                    "#f8f0e6", "#ff8a3d", "#1f0d02", "#f2c14e", "#8fc7a2", "#ff5d4d", "#332a25", "#3a302a",
                ],
                ["#ff8a3d", "#ffd27a", "#8fb4d6", "#8fc7a2"],
                ["#ff5d4d", "#8fc7a2", "#f2c14e", "#8fb4d6", "#ff8a3d", "#ffd27a", "#7d7067"],
            ),
            // The same forge by day.
            "tinkery-light" => Theme::design(
                [
                    "#f5f0e9", "#3a322c", "#ede6dc", "#e4dcd0", "#d9cfc2", "#c0b3a3", "#847768", "#5b5047",
                    "#1c1714", "#b84a08", "#fff6ec", "#835900", "#3f7d5a", "#a3123f", "#dcd2c5", "#d3c7b8",
                ],
                ["#b84a08", "#b07d0c", "#3c6a94", "#3f7d5a"],
                ["#a3123f", "#3f7d5a", "#835900", "#3c6a94", "#b84a08", "#b07d0c", "#847768"],
            ),
            // Tinkery's Aurora: violet on near-black, with a teal second light.
            "aurora" => Theme::design(
                [
                    "#0b0a10", "#d2cfe2", "#0f0e16", "#16141f", "#1e1b2a", "#2e2a40", "#6d6884", "#a6a2bb",
                    "#f4f2fb", "#b69cff", "#120a24", "#f0cf7a", "#7fe3c9", "#ff6f91", "#2e2a40", "#221c35",
                ],
                ["#b69cff", "#6ee7d8", "#8fd0ff", "#7fe3c9"],
                ["#ff6f91", "#7fe3c9", "#f0cf7a", "#8fd0ff", "#b69cff", "#6ee7d8", "#6d6884"],
            ),
            // Tinkery's Moonlit: pale blue on blue-black, gold beside it.
            "moonlit" => Theme::design(
                [
                    "#090b10", "#d0d6e4", "#0d1017", "#141822", "#1b2030", "#2a3246", "#657089", "#a2abbf",
                    "#f2f5fb", "#a9c6ff", "#0b1426", "#f0cf7a", "#7fe3c9", "#ff6f91", "#2a3246", "#1c2538",
                ],
                ["#a9c6ff", "#f0cf7a", "#a9c6ff", "#7fe3c9"],
                ["#ff6f91", "#7fe3c9", "#f0cf7a", "#a9c6ff", "#a9c6ff", "#f0cf7a", "#657089"],
            ),
            "papercolor-dark" => Theme::design(
                [
                    "#1c1c1c", "#d0d0d0", "#262626", "#303030", "#3a3a3a", "#444444", "#808080", "#b2b2b2",
                    "#eeeeee", "#00afaf", "#1c1c1c", "#ffaf00", "#5faf00", "#ff5f87", "#3a3a3a", "#5f5faf",
                ],
                ["#af87d7", "#5fafd7", "#ff5faf", "#5f8787"],
                ["#ff5f87", "#5faf00", "#d7af5f", "#5fafd7", "#af87d7", "#00afaf", "#808080"],
            ),
            "tango-dark" => Theme::design(
                [
                    "#2e3436", "#eeeeec", "#252a2b", "#363c3e", "#41474a", "#555753", "#9a9c97", "#d3d7cf",
                    "#eeeeec", "#8ae234", "#2e3436", "#fcaf3e", "#73d216", "#ff5c5c", "#4a5052", "#204a87",
                ],
                ["#ad7fa8", "#729fcf", "#e9b96e", "#34e2e2"],
                ["#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8", "#34e2e2", "#888a85"],
            ),
            "monokai" => Theme::design(
                [
                    "#272822", "#f8f8f2", "#1e1f1c", "#2f302a", "#3e3d32", "#49483e", "#8f8a72", "#cfcfc2",
                    "#f8f8f2", "#a6e22e", "#272822", "#fd971f", "#a6e22e", "#f92672", "#3e3d32", "#55544a",
                ],
                ["#ae81ff", "#66d9ef", "#f92672", "#a1efe4"],
                ["#f92672", "#a6e22e", "#e6db74", "#66d9ef", "#ae81ff", "#a1efe4", "#75715e"],
            ),
            "tokyo-night" => Theme::design(
                [
                    "#1a1b26", "#c0caf5", "#16161e", "#1f2335", "#292e42", "#292e42", "#7a82ad", "#a9b1d6",
                    "#e0e6ff", "#7aa2f7", "#1a1b26", "#e0af68", "#9ece6a", "#f7768e", "#292e42", "#3b4261",
                ],
                ["#bb9af7", "#7dcfff", "#f7768e", "#73daca"],
                ["#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#565f89"],
            ),
            "catppuccin-mocha" => Theme::classic([
                "#1e1e2e", "#cdd6f4", "#6c7086", "#89b4fa", "#45475a", "#89b4fa", "#181825",
                "#313244", "#89b4fa", "#1e1e2e", "#f9e2af", "#f38ba8", "#89dceb", "#a6e3a1",
            ]),
            "catppuccin-latte" => Theme::classic([
                "#eff1f5", "#303446", "#6c6f85", "#7a2fd8", "#bcc0cc", "#7a2fd8", "#e6e9ef",
                "#ccd0da", "#7a2fd8", "#eff1f5", "#8a5a00", "#d20f39", "#1e5ad8", "#2f8a1f",
            ]),
            "gruvbox" => Theme::classic([
                "#282828", "#ebdbb2", "#928374", "#fabd2f", "#504945", "#fabd2f", "#1d2021",
                "#3c3836", "#fabd2f", "#282828", "#fe8019", "#fb4934", "#83a598", "#b8bb26",
            ]),
            "nord" => Theme::classic([
                "#2e3440", "#eceff4", "#8390a8", "#a3be8c", "#434c5e", "#a3be8c", "#272c36",
                "#3b4252", "#a3be8c", "#2e3440", "#ebcb8b", "#e0707a", "#81a1c1", "#a3be8c",
            ]),
            "dracula" => Theme::classic([
                "#282a36", "#f8f8f2", "#7a8ac0", "#bd93f9", "#44475a", "#bd93f9", "#21222c",
                "#44475a", "#bd93f9", "#282a36", "#f1fa8c", "#ff5555", "#8be9fd", "#50fa7b",
            ]),
            "apprentice" => Theme::palette(
                ["#262626", "#bcbcbc", "#6c6c6c", "#5f87af"],
                ["#af5f5f", "#5f875f", "#87875f", "#5f87af", "#5f5f87", "#5f8787", "#6c6c6c"],
            ),
            "ayu-dark" => Theme::palette(
                ["#0f1419", "#e6e1cf", "#5c6773", "#36a3d9"],
                ["#f07178", "#b8cc52", "#f29718", "#36a3d9", "#ffee99", "#95e6cb", "#ffffff"],
            ),
            "ayu-light" => Theme::palette(
                ["#fafafa", "#5c6773", "#abb0b6", "#36a3d9"],
                ["#f07178", "#86b300", "#ff6a00", "#36a3d9", "#a37acc", "#4cbf99", "#ffffff"],
            ),
            "ayu-mirage" => Theme::palette(
                ["#212733", "#d9d7ce", "#5c6773", "#5ccfe6"],
                ["#f07178", "#bbe67e", "#ffcc66", "#5ccfe6", "#d4bfff", "#95e6cb", "#ffffff"],
            ),
            "catppuccin-frappe" => Theme::palette(
                ["#303446", "#c6d0f5", "#626880", "#8caaee"],
                ["#e78284", "#a6d189", "#e5c890", "#8caaee", "#f4b8e4", "#81c8be", "#b5bfe2"],
            ),
            "catppuccin-macchiato" => Theme::palette(
                ["#24273a", "#cad3f5", "#5b6078", "#8aadf4"],
                ["#ed8796", "#a6da95", "#eed49f", "#8aadf4", "#f5bde6", "#8bd5ca", "#b8c0e0"],
            ),
            "cobalt2" => Theme::palette(
                ["#122738", "#ffffff", "#aaaaaa", "#ffc600"],
                ["#ff628c", "#3ad900", "#ffc600", "#0088ff", "#fb94ff", "#80fcff", "#ffffff"],
            ),
            "everforest-dark" => Theme::palette(
                ["#2d353b", "#d3c6aa", "#859289", "#7fbbb3"],
                ["#e67e80", "#a7c080", "#dbbc7f", "#7fbbb3", "#d699b6", "#83c092", "#d3c6aa"],
            ),
            "everforest-light" => Theme::palette(
                ["#fdf6e3", "#5c6a72", "#939f91", "#3a94c5"],
                ["#f85552", "#8da101", "#dfa000", "#3a94c5", "#df69ba", "#35a77c", "#5c6a72"],
            ),
            "gotham" => Theme::palette(
                ["#0c1014", "#99d1ce", "#195466", "#195466"],
                ["#c23127", "#2aa889", "#edb443", "#195466", "#4e5166", "#33859e", "#99d1ce"],
            ),
            "gruvbox-light" => Theme::palette(
                ["#fbf1c7", "#3c3836", "#928374", "#076678"],
                ["#9d0006", "#79740e", "#b57614", "#076678", "#8f3f71", "#427b58", "#3c3836"],
            ),
            "iceberg" => Theme::palette(
                ["#161821", "#c6c8d1", "#6b7089", "#84a0c6"],
                ["#e27878", "#b4be82", "#e2a478", "#84a0c6", "#a093c7", "#89b8c2", "#c6c8d1"],
            ),
            "jellybeans" => Theme::palette(
                ["#151515", "#e8e8d3", "#888888", "#597bc5"],
                ["#cf6a4c", "#99ad6a", "#d8ad4c", "#597bc5", "#a037b0", "#71b9f8", "#adadad"],
            ),
            "kanagawa-dragon" => Theme::palette(
                ["#181616", "#c5c9c5", "#a6a69c", "#8ba4b0"],
                ["#c4746e", "#8a9a7b", "#c4b28a", "#8ba4b0", "#a292a3", "#8ea4a2", "#c8c093"],
            ),
            "kanagawa-lotus" => Theme::palette(
                ["#f2ecbc", "#545464", "#8a8980", "#4d699b"],
                ["#c84053", "#6f894e", "#77713f", "#4d699b", "#b35b79", "#597b75", "#545464"],
            ),
            "kanagawa-wave" => Theme::palette(
                ["#1f1f28", "#dcd7ba", "#727169", "#7e9cd8"],
                ["#c34043", "#76946a", "#c0a36e", "#7e9cd8", "#957fb8", "#6a9589", "#c8c093"],
            ),
            "lucario" => Theme::palette(
                ["#2b3e50", "#f8f8f2", "#2f3943", "#5c98cd"],
                ["#e94b35", "#199c4b", "#f0cc04", "#5c98cd", "#ca94ff", "#8be0fd", "#f8f8f2"],
            ),
            "moonfly" => Theme::palette(
                ["#080808", "#bdbdbd", "#949494", "#80a0ff"],
                ["#ff5d5d", "#8cc85f", "#e3c78a", "#80a0ff", "#cf87e8", "#79dac8", "#c6c6c6"],
            ),
            "night-owl" => Theme::palette(
                ["#011627", "#d6deeb", "#575656", "#82aaff"],
                ["#ef5350", "#22da6e", "#c5e478", "#82aaff", "#c792ea", "#21c7a8", "#ffffff"],
            ),
            "nightfly" => Theme::palette(
                ["#011627", "#bdc1c6", "#7c8f8f", "#82aaff"],
                ["#fc514e", "#a1cd5e", "#e3d18a", "#82aaff", "#c792ea", "#7fdbca", "#a1aab8"],
            ),
            "one-dark" => Theme::palette(
                ["#282c34", "#abb2bf", "#3e4452", "#61afef"],
                ["#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#5c6370"],
            ),
            "one-half-dark" => Theme::palette(
                ["#282c34", "#dcdfe4", "#5d677a", "#61afef"],
                ["#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#dcdfe4"],
            ),
            "one-half-light" => Theme::palette(
                ["#fafafa", "#383a42", "#383a42", "#0184bc"],
                ["#e45649", "#40a14f", "#c18401", "#0184bc", "#a626a4", "#0997b3", "#fafafa"],
            ),
            "panda" => Theme::palette(
                ["#292a2b", "#e6e6e6", "#757575", "#ff75b5"],
                ["#ff2c6d", "#19f9d8", "#ffb86c", "#45a9f9", "#ff75b5", "#b084eb", "#cdcdcd"],
            ),
            "rose-pine" => Theme::palette(
                ["#191724", "#e0def4", "#6e6a86", "#c4a7e7"],
                ["#eb6f92", "#31748f", "#f6c177", "#9ccfd8", "#c4a7e7", "#ebbcba", "#e0def4"],
            ),
            "rose-pine-dawn" => Theme::palette(
                ["#faf4ed", "#575279", "#9893a5", "#907aa9"],
                ["#b4637a", "#286983", "#ea9d34", "#56949f", "#907aa9", "#d7827e", "#575279"],
            ),
            "rose-pine-moon" => Theme::palette(
                ["#232136", "#e0def4", "#6e6a86", "#c4a7e7"],
                ["#eb6f92", "#3e8fb0", "#f6c177", "#9ccfd8", "#c4a7e7", "#ea9a97", "#e0def4"],
            ),
            "shades-of-purple" => Theme::palette(
                ["#1e1e3f", "#ffffff", "#5c5c61", "#7857fe"],
                ["#e43937", "#3ad900", "#fad000", "#7857fe", "#ff2c70", "#80fcff", "#ffffff"],
            ),
            "solarized-dark" => Theme::palette(
                ["#002b36", "#839496", "#586e75", "#268bd2"],
                ["#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5"],
            ),
            "solarized-light" => Theme::palette(
                ["#fdf6e3", "#657b83", "#93a1a1", "#268bd2"],
                ["#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5"],
            ),
            "sonokai" => Theme::palette(
                ["#2c2e34", "#e2e2e3", "#7f8490", "#76cce0"],
                ["#fc5d7c", "#9ed072", "#e7c664", "#76cce0", "#b39df3", "#f39660", "#e2e2e3"],
            ),
            "tokyo-night-day" => Theme::palette(
                ["#e1e2e7", "#3760bf", "#a1a6c5", "#2e7de9"],
                ["#f52a65", "#587539", "#8c6c3e", "#2e7de9", "#9854f1", "#007197", "#6172b0"],
            ),
            "tokyo-night-moon" => Theme::palette(
                ["#222436", "#c8d3f5", "#444a73", "#82aaff"],
                ["#ff757f", "#c3e88d", "#ffc777", "#82aaff", "#c099ff", "#86e1fc", "#828bb8"],
            ),
            "tokyo-night-storm" => Theme::palette(
                ["#24283b", "#c0caf5", "#414868", "#7aa2f7"],
                ["#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6"],
            ),
            "zenbones-dark" => Theme::palette(
                ["#1c1917", "#b4bdc3", "#403833", "#6099c0"],
                ["#de6e7c", "#819b69", "#b77e64", "#6099c0", "#b279a7", "#66a5ad", "#b4bdc3"],
            ),
            "zenbones-light" => Theme::palette(
                ["#f0edec", "#2c363c", "#cfc1ba", "#286486"],
                ["#a8334c", "#4f6c31", "#944927", "#286486", "#88507d", "#3b8992", "#2c363c"],
            ),
            "nightfox" => Theme::palette(
                ["#192330", "#cdcecf", "#738091", "#719cd6"],
                ["#c94f6d", "#81b29a", "#dbc074", "#719cd6", "#9d79d6", "#63cdcf", "#575860"],
            ),
            "duskfox" => Theme::palette(
                ["#232136", "#e0def4", "#817c9c", "#569fba"],
                ["#eb6f92", "#a3be8c", "#f6c177", "#569fba", "#c4a7e7", "#9ccfd8", "#47407d"],
            ),
            "carbonfox" => Theme::palette(
                ["#161616", "#f2f4f8", "#6e6f70", "#78a9ff"],
                ["#ee5396", "#25be6a", "#08bdba", "#78a9ff", "#be95ff", "#33b1ff", "#484848"],
            ),
            "github-dark" => Theme::palette(
                ["#0d1117", "#e6edf3", "#8b949e", "#2f81f7"],
                ["#ff7b72", "#3fb950", "#d29922", "#58a6ff", "#bc8cff", "#76e3ea", "#8b949e"],
            ),
            "github-dark-dimmed" => Theme::palette(
                ["#22272e", "#adbac7", "#768390", "#539bf5"],
                ["#f47067", "#57ab5a", "#c69026", "#539bf5", "#b083f0", "#76e3ea", "#768390"],
            ),
            "github-light" => Theme::palette(
                ["#ffffff", "#1f2328", "#57606a", "#0969da"],
                ["#cf222e", "#116329", "#4d2d00", "#0969da", "#8250df", "#1b7c83", "#57606a"],
            ),
            "gruvbox-material" => Theme::palette(
                ["#282828", "#d4be98", "#928374", "#7daea3"],
                ["#ea6962", "#a9b665", "#d8a657", "#7daea3", "#d3869b", "#89b482", "#5a524c"],
            ),
            "gruvbox-baby" => Theme::palette(
                ["#1d2021", "#ebdbb2", "#665c54", "#7fa2ac"],
                ["#cc241d", "#98971a", "#fabd2f", "#458588", "#b16286", "#83a598", "#665c54"],
            ),
            "oceanic-next" => Theme::palette(
                ["#1b2b34", "#d8dee9", "#65737e", "#6699cc"],
                ["#ec5f67", "#99c794", "#fac863", "#6699cc", "#c594c5", "#62b3b2", "#65737e"],
            ),
            "tomorrow" => Theme::palette(
                ["#ffffff", "#4d4d4c", "#8e908c", "#4271ae"],
                ["#c82829", "#718c00", "#eab700", "#4271ae", "#8959a8", "#3e999f", "#666666"],
            ),
            "tomorrow-night" => Theme::palette(
                ["#1d1f21", "#c5c8c6", "#969896", "#81a2be"],
                ["#cc6666", "#b5bd68", "#f0c674", "#81a2be", "#b294bb", "#8abeb7", "#666666"],
            ),
            "tomorrow-night-eighties" => Theme::palette(
                ["#2d2d2d", "#cccccc", "#999999", "#6699cc"],
                ["#f2777a", "#99cc99", "#ffcc66", "#6699cc", "#cc99cc", "#66cccc", "#666666"],
            ),
            "tomorrow-night-bright" => Theme::palette(
                ["#000000", "#eaeaea", "#969896", "#7aa6da"],
                ["#d54e53", "#b9ca4a", "#e7c547", "#7aa6da", "#c397d8", "#70c0b1", "#666666"],
            ),
            "base16-default-dark" => Theme::palette(
                ["#181818", "#d8d8d8", "#585858", "#7cafc2"],
                ["#ab4642", "#a1b56c", "#f7ca88", "#7cafc2", "#ba8baf", "#86c1b9", "#585858"],
            ),
            "palenight" => Theme::palette(
                ["#292d3e", "#bfc7d5", "#697098", "#82b1ff"],
                ["#ff5370", "#c3e88d", "#ffcb6b", "#82b1ff", "#c792ea", "#89ddff", "#697098"],
            ),
            "one-light" => Theme::palette(
                ["#fafafa", "#494b53", "#a0a1a7", "#4078f2"],
                ["#e45649", "#50a14f", "#c18401", "#4078f2", "#a626a4", "#0184bc", "#a0a1a7"],
            ),
            "monokai-pro" => Theme::palette(
                ["#2d2a2e", "#fcfcfa", "#727072", "#ffd866"],
                ["#ff6188", "#a9dc76", "#ffd866", "#fc9867", "#ab9df2", "#78dce8", "#727072"],
            ),
            "oxocarbon" => Theme::palette(
                ["#161616", "#d0d0d0", "#525252", "#78a9ff"],
                ["#ee5396", "#42be65", "#be95ff", "#78a9ff", "#ff7eb6", "#3ddbd9", "#525252"],
            ),
            "cyberdream" => Theme::palette(
                ["#16181a", "#ffffff", "#7b8496", "#5ea1ff"],
                ["#ff6e5e", "#5eff6c", "#f1ff5e", "#5ea1ff", "#bd5eff", "#5ef1ff", "#3c4048"],
            ),
            "nordic" => Theme::palette(
                ["#242933", "#c0c8d8", "#4c566a", "#5e81ac"],
                ["#bf616a", "#a3be8c", "#ebcb8b", "#5e81ac", "#b48ead", "#8fbcbb", "#4c566a"],
            ),
            "onenord" => Theme::palette(
                ["#2e3440", "#e5e9f0", "#6c7a96", "#81a1c1"],
                ["#e06c75", "#9ec183", "#ebcb8b", "#81a1c1", "#b988b0", "#88c0d0", "#4c566a"],
            ),
            "edge" => Theme::palette(
                ["#2c2e34", "#c5cdd9", "#758094", "#6cb6eb"],
                ["#ec7279", "#a0c980", "#deb974", "#6cb6eb", "#d38aea", "#5dbbc1", "#758094"],
            ),
            "vscode" => Theme::palette(
                ["#1f1f1f", "#d4d4d4", "#808080", "#569cd6"],
                ["#f44747", "#608b4e", "#dcdcaa", "#569cd6", "#c678dd", "#56b6c2", "#808080"],
            ),
            "code-dark" => Theme::palette(
                ["#1e1e1e", "#d4d4d4", "#808080", "#569cd6"],
                ["#f44747", "#6a9955", "#dcdcaa", "#569cd6", "#c586c0", "#9cdcfe", "#808080"],
            ),
            "melange" => Theme::palette(
                ["#292522", "#ece1d7", "#867462", "#7f91b2"],
                ["#bd8183", "#78997a", "#e49b5d", "#7f91b2", "#b380b0", "#7b9695", "#867462"],
            ),
            "falcon" => Theme::palette(
                ["#020221", "#b4b4b9", "#787882", "#ff761a"],
                ["#ff3600", "#859933", "#ffc552", "#635196", "#ff761a", "#34bfa4", "#787882"],
            ),
            "embark" => Theme::palette(
                ["#1e1c31", "#cbe3e7", "#8a889d", "#91ddff"],
                ["#f48fb1", "#a1efd3", "#ffe6b3", "#91ddff", "#d4bfff", "#abf8f7", "#585273"],
            ),
            "lackluster" => Theme::palette(
                ["#0a0a0a", "#deeeed", "#444444", "#7788aa"],
                ["#d70000", "#789978", "#ffaa88", "#7788aa", "#d7007d", "#708090", "#444444"],
            ),
            "miasma" => Theme::palette(
                ["#222222", "#c2c2b0", "#666666", "#78824b"],
                ["#685742", "#5f875f", "#b36d43", "#78824b", "#bb7744", "#c9a554", "#666666"],
            ),
            "bamboo" => Theme::palette(
                ["#252623", "#f1e9d2", "#838781", "#57a5e5"],
                ["#e75a7c", "#8fb573", "#dbb651", "#57a5e5", "#aaaaff", "#70c2be", "#5b5e5a"],
            ),
            "mellifluous" => Theme::palette(
                ["#1a1a1a", "#dadada", "#737246", "#a8a1be"],
                ["#d59192", "#b3b393", "#bfaf8e", "#a8a1be", "#b99bb5", "#a8a1be", "#737246"],
            ),
            "mellow" => Theme::palette(
                ["#161617", "#c9c7cd", "#757581", "#aca1cf"],
                ["#f5a191", "#90b99f", "#e6b99d", "#aca1cf", "#e29eca", "#ea83a5", "#757581"],
            ),
            "kanagawa-paper" => Theme::palette(
                ["#1f1f28", "#dcd7ba", "#727169", "#658594"],
                ["#c4746e", "#699469", "#c4b28a", "#435965", "#a292a3", "#8ea49e", "#aca9a4"],
            ),
            "zephyr" => Theme::palette(
                ["#262a33", "#bbc2cf", "#73797e", "#61afef"],
                ["#e95678", "#afd700", "#f7bb3b", "#61afef", "#cba6f7", "#36d0e0", "#c78665"],
            ),
            "koda" => Theme::palette(
                ["#101010", "#ffffff", "#50585d", "#d9ba73"],
                ["#ff7676", "#a3d6a3", "#ffffff", "#b3b3b3", "#f4b8e4", "#fafafa", "#50585d"],
            ),
            "aquarium" => Theme::palette(
                ["#20202a", "#63718b", "#44495e", "#cddbf9"],
                ["#ebb9b9", "#b1dba4", "#e6dfb8", "#cddbf9", "#f6bbe7", "#b8deeb", "#44495e"],
            ),
            "fluoromachine" => Theme::palette(
                ["#262335", "#8ba7a7", "#495495", "#fc199a"],
                ["#fe4450", "#72f1b8", "#ffcc00", "#af6df9", "#fc199a", "#61e2ff", "#495495"],
            ),
            "synthwave-84" => Theme::palette(
                ["#262335", "#ffffff", "#848bbd", "#f97e72"],
                ["#fe4450", "#72f1b8", "#f3e70f", "#03edf9", "#ff7edb", "#03edf9", "#848bbd"],
            ),
            "horizon" => Theme::palette(
                ["#1c1e26", "#d5d8da", "#4c4d53", "#e95378"],
                ["#e95678", "#29d398", "#fab795", "#26bbd9", "#ee64ac", "#59e1e3", "#4c4d53"],
            ),
            "material" => Theme::palette(
                ["#212121", "#b0bec5", "#515151", "#82aaff"],
                ["#dc6068", "#abcf76", "#e6b455", "#6e98eb", "#b480d6", "#71c6e7", "#474747"],
            ),
            "bluloco" => Theme::palette(
                ["#282c34", "#b9c0cb", "#636d83", "#3691ff"],
                ["#fc2f52", "#25a45c", "#ff936a", "#3476ff", "#7a82da", "#4483aa", "#8f9aae"],
            ),
            "modus-vivendi" => Theme::palette(
                ["#000000", "#ffffff", "#989898", "#00bcff"],
                ["#ff5f59", "#44bc44", "#d0bc00", "#2fafff", "#feacd0", "#00d3d0", "#595959"],
            ),
            "modus-operandi" => Theme::palette(
                ["#ffffff", "#000000", "#595959", "#0031a9"],
                ["#a60000", "#006800", "#6f5500", "#0031a9", "#721045", "#005e8b", "#595959"],
            ),
            "mono" => Theme {
                bg: Color::Reset,
                fg: Color::Reset,
                muted: Color::DarkGray,
                accent: Color::White,
                border: Color::DarkGray,
                border_active: Color::White,
                sidebar_bg: Color::Reset,
                selection_bg: Color::DarkGray,
                tab_active_bg: Color::White,
                tab_active_fg: Color::Black,
                // Named colours only: working white (the accent), needs you yellow, failed red.
                working: Color::White,
                blocked: Color::Yellow,
                done: Color::Green,
                idle: Color::DarkGray,
                card: Color::Black,
                card2: Color::DarkGray,
                btn: Color::DarkGray,
                hov: Color::DarkGray,
                line: Color::DarkGray,
                text: Color::Gray,
                strong: Color::White,
                acc_ink: Color::Black,
                err: Color::Red,
                ws: None,
                ansi: None,
            },
            _ => Theme::base(DEFAULT),
        }
    }

    /// Destructive red apart from what needs you (the design palettes share one colour).
    fn with_err(mut self, c: &str) -> Theme {
        self.err = hex(c);
        self
    }

    /// Needs you in amber, apart from failed red (the handoff's `needs` and `err` roles; most
    /// themes share one red for both), and working as the accent so amber is only needs you.
    fn with_amber_needs(mut self, needs: &str) -> Theme {
        self.blocked = hex(needs);
        self.working = self.accent;
        self
    }

    /// Needs you in an amber of its own, so it never reads as failed. A theme that already has
    /// one apart from every other status keeps it; otherwise the amber comes from the theme's
    /// own yellow nudged toward orange, else a tuned amber, moved toward the text colour just
    /// far enough to read on every surface it is drawn on. Working falls back to the accent
    /// when it would sit next to the amber, as in `with_amber_needs`.
    fn with_distinct_needs(mut self) -> Theme {
        let ink = if contrast(self.bg, Color::Rgb(0, 0, 0)) > contrast(self.bg, Color::Rgb(255, 255, 255)) {
            Color::Rgb(0, 0, 0)
        } else {
            Color::Rgb(255, 255, 255)
        };
        let light = ink == Color::Rgb(0, 0, 0);
        let tuned = if light { AMBER_ON_LIGHT } else { AMBER_ON_DARK };
        let deep = if light { DEEP_AMBER_ON_LIGHT } else { DEEP_AMBER_ON_DARK };
        let flat_terminal = mix(self.bg, Color::Rgb(0, 0, 0), 0.4);
        let surfaces = [self.bg, self.sidebar_bg, self.card, self.card2, self.hov, self.sel_bg(), flat_terminal];
        let others = [self.err, self.done, self.accent, self.sky(), self.violet()];
        let kept = surfaces.iter().all(|s| contrast(self.blocked, *s) >= LIT_READS);
        let working_ok = |t: &Theme| !colors_alike(t.working, t.blocked) && !colors_alike(t.on_lit(t.working), t.on_lit(t.blocked));
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
        self.ws.map(|w| w[1]).unwrap_or(Color::Rgb(0x5a, 0xa9, 0xff))
    }

    /// The workspace palette's cyan.
    pub fn cyan(&self) -> Color {
        self.ws.map(|w| w[3]).unwrap_or(Color::Rgb(0x4e, 0xe0, 0xfa))
    }

    /// Grid's pane colours, in pane order (green, pink, violet, cyan in the design), from roles
    /// every theme has. Identity, not state: they repeat from the fifth pane. A theme whose
    /// roles share a colour (an accent that is also its violet) moves on to the next role, so
    /// no two panes in a row look alike.
    pub fn grid_colors(&self) -> [Color; 4] {
        let pink = self.ws.map(|w| w[2]);
        let roles = [Some(self.done), Some(self.accent), Some(self.violet()), Some(self.cyan()), pink, Some(self.sky()), Some(self.working)];
        let terminal = self.ansi.into_iter().flatten().map(Some);
        let mut out: Vec<Color> = Vec::with_capacity(4);
        for c in roles.into_iter().chain(terminal).chain([Some(self.blocked)]).flatten() {
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
        let base = self.ws.map(|w| w[0]).unwrap_or(Color::Rgb(0xb5, 0x8c, 0xff));
        readable_on(base, self.strong, &[self.bg, self.card, self.sel_bg()], LIT_READS)
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
                assert_ne!(theme.bg, Theme::named(super::DEFAULT).bg, "{name} fell back to the default");
            }
            if name != "mono" {
                assert!(matches!(theme.bg, Color::Rgb(..)), "invalid background for {name}");
            }
        }
    }

    fn lum(c: Color) -> Option<f64> {
        let Color::Rgb(r, g, b) = c else { return None };
        let ch = |v: u8| {
            let s = v as f64 / 255.0;
            if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
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

    /// Every theme gives each status its own colour, as the handoff's roles do: working,
    /// needs you (amber), failed, done, checking and could not run.
    #[test]
    fn every_theme_gives_each_status_its_own_colour() {
        let mut bad = Vec::new();
        for &n in super::BUILTIN {
            let t = Theme::named(n);
            let roles = [("working", t.accent), ("needs you", t.blocked), ("failed", t.err), ("done", t.done), ("checking", t.sky()), ("could not run", t.violet())];
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
                ("ink on the red Close button", t.ink_on(t.danger_fill()), t.danger_fill(), 4.5),
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
                ("lit done vs lit blocked", t.on_lit(t.done), t.on_lit(t.blocked)),
                ("lit err vs lit blocked", t.on_lit(t.err), t.on_lit(t.blocked)),
                ("lit working vs lit done", t.on_lit(t.working), t.on_lit(t.done)),
                ("lit working vs lit blocked", t.on_lit(t.working), t.on_lit(t.blocked)),
            ] {
                if super::colors_alike(a, b) {
                    bad.push(format!("{n:18} {what:24} too alike"));
                }
            }
            // Open and hover rows must differ from the sidebar and each other.
            if contrast(active, t.sidebar_bg) < 1.12 || contrast(hover, t.sidebar_bg) < 1.08 {
                bad.push(format!("{n:18} open/hover rows barely show ({:.2} / {:.2})", contrast(active, t.sidebar_bg), contrast(hover, t.sidebar_bg)));
            }
        }
        for b in &bad {
            eprintln!("{b}");
        }
        assert!(bad.is_empty(), "{} theme problems", bad.len());
    }

}
