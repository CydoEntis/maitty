//! The marks a session and its checks are drawn with: glyph, colour and word for each state.
//! In the full sets (Unicode, ASCII, Nerd) shape and word carry the meaning and colour
//! only backs them, so a state reads in plain ASCII and without colour too. The `dots` set
//! (the default) marks a session with a coloured dot and leaves the meaning to the colour and
//! the word beside it.
use crate::config::Icons;
use crate::protocol::{CheckInfo, CheckState, Status, reported_check};
use crate::theme::Theme;
use ratatui::style::Color;

/// One set of marks. Each is one terminal cell, except `branch` in ASCII (`git:`).
#[derive(Debug, PartialEq, Eq)]
pub(in crate::client) struct Glyphs {
    /// The checking ring, one frame per step.
    pub ring: [&'static str; 4],
    pub needs: &'static str,
    pub done: &'static str,
    pub failed: &'static str,
    pub over: &'static str,
    pub couldnt: &'static str,
    pub stale: &'static str,
    pub idle: &'static str,
    pub asleep: &'static str,
    pub pend: &'static str,
    /// The edge bar of a selected or failed row.
    pub bar: &'static str,
    pub branch: &'static str,
    pub close: &'static str,
    pub fold: &'static str,
    pub warn: &'static str,
    pub arrow: &'static str,
    /// Mark every session by a dot (`●`, or `○` at rest) rather than by a glyph of its own. A
    /// check's own marks (`✓ ✕` beside its name) keep their shapes either way.
    pub dots: bool,
}

pub(in crate::client) const UNICODE: Glyphs = Glyphs {
    ring: ["◜", "◝", "◞", "◟"],
    needs: "●",
    done: "✓",
    failed: "✕",
    over: "⊘",
    couldnt: "?",
    stale: "◷",
    idle: "○",
    asleep: "z",
    pend: "…",
    bar: "▎",
    branch: "⎇",
    close: "✕",
    fold: "▾",
    warn: "⚠",
    arrow: "›",
    dots: false,
};

/// Unicode, with a session marked by a coloured dot: `●` for every state that wants you or says
/// something, `○` for idle and asleep; working and checking keep their moving marks.
pub(in crate::client) const DOTS: Glyphs = Glyphs { dots: true, ..UNICODE };

pub(in crate::client) const ASCII: Glyphs = Glyphs {
    ring: ["-", "\\", "|", "/"],
    needs: "!",
    done: "+",
    failed: "x",
    over: "/",
    couldnt: "?",
    stale: "~",
    idle: "o",
    asleep: "z",
    pend: ".",
    bar: "|",
    branch: "git:",
    close: "x",
    fold: "v",
    warn: "!",
    arrow: ">",
    dots: false,
};

/// Unicode with Nerd Font icons where the reference has one; each is drawn in one cell.
pub(in crate::client) const NERD: Glyphs = Glyphs {
    needs: "\u{f0f3}",
    done: "\u{f00c}",
    failed: "\u{f00d}",
    over: "\u{f05e}",
    couldnt: "\u{f128}",
    stale: "\u{f017}",
    idle: "\u{f10c}",
    branch: "\u{e0a0}",
    close: "\u{f00d}",
    warn: "\u{f071}",
    ..UNICODE
};

/// The set `[ui] glyphs` names. `auto` (or anything unknown): ASCII when the blocked, done and
/// idle icons are all ASCII, else dots.
pub(in crate::client) fn chosen(setting: &str, icons: &Icons) -> &'static Glyphs {
    match setting {
        "dots" => &DOTS,
        "unicode" => &UNICODE,
        "ascii" => &ASCII,
        "nerd" => &NERD,
        _ if [&icons.blocked, &icons.done, &icons.idle].iter().all(|icon| icon.is_ascii()) => &ASCII,
        _ => &DOTS,
    }
}

/// What a session reads as, from its status and its headline check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum Kind {
    Working,
    Checking,
    Needs,
    /// A repo's check command waits for your approval.
    Approve,
    Done,
    Passed,
    Stale,
    Failed,
    Over,
    Couldnt,
    Never,
    Idle,
    Asleep,
}

/// How a session is marked: its glyph, the colour that backs it, and the word that says it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::client) struct Mark<'a> {
    pub glyph: &'a str,
    pub color: Color,
    pub word: &'static str,
}

impl Kind {
    pub fn mark<'a>(self, g: &'static Glyphs, t: &Theme, ring_step: usize, working: &'a str) -> Mark<'a> {
        Mark { glyph: self.glyph(g, ring_step, working), color: self.color(t), word: self.word() }
    }

    /// While the agent works or asks, its last check is about an older turn and says nothing.
    pub fn of(status: Status, check: Option<&CheckInfo>, asleep: bool) -> Kind {
        if asleep {
            return Kind::Asleep;
        }
        let shown = reported_check(status, check).map(|c| match c.state {
            CheckState::Checking | CheckState::Pending => Kind::Checking,
            CheckState::NeedsApproval => Kind::Approve,
            CheckState::Passed => Kind::Passed,
            CheckState::Stale => Kind::Stale,
            CheckState::Failed => Kind::Failed,
            CheckState::Overridden => Kind::Over,
            CheckState::CouldNotRun => Kind::Couldnt,
            CheckState::Never => Kind::Never,
        });
        shown.unwrap_or(match status {
            Status::Working => Kind::Working,
            Status::Blocked => Kind::Needs,
            Status::Done => Kind::Done,
            Status::Idle | Status::None => Kind::Idle,
        })
    }

    pub fn word(self) -> &'static str {
        match self {
            Kind::Working => "working",
            Kind::Checking => "checking",
            Kind::Needs | Kind::Approve => "needs you",
            Kind::Done | Kind::Passed => "done",
            Kind::Stale => "done · stale",
            Kind::Failed => "failed",
            Kind::Over => "done · overridden",
            Kind::Couldnt | Kind::Never => "not checked",
            Kind::Idle => "idle",
            Kind::Asleep => "asleep",
        }
    }

    /// Its colour: a stale pass and an override are never green, and not running a check is
    /// not a failure, so neither borrows those colours.
    pub fn color(self, t: &Theme) -> Color {
        match self {
            Kind::Working => t.accent,
            Kind::Checking => t.sky(),
            Kind::Needs | Kind::Approve => t.blocked,
            Kind::Done | Kind::Passed => t.done,
            Kind::Stale | Kind::Over => t.text,
            Kind::Failed => t.err,
            Kind::Couldnt | Kind::Never => t.violet(),
            Kind::Idle | Kind::Asleep => t.muted,
        }
    }

    /// Its glyph. Working shows the user's spinner frame and checking the ring at `ring_step`.
    pub fn glyph<'a>(self, g: &'static Glyphs, ring_step: usize, working: &'a str) -> &'a str {
        if g.dots && !matches!(self, Kind::Working | Kind::Checking) {
            return if matches!(self, Kind::Idle | Kind::Asleep) { g.idle } else { g.needs };
        }
        match self {
            Kind::Working => working,
            Kind::Checking => g.ring[ring_step % g.ring.len()],
            Kind::Needs | Kind::Approve => g.needs,
            Kind::Done | Kind::Passed => g.done,
            Kind::Stale => g.stale,
            Kind::Failed => g.failed,
            Kind::Over => g.over,
            Kind::Couldnt | Kind::Never => g.couldnt,
            Kind::Idle => g.idle,
            Kind::Asleep => g.asleep,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    const SETS: [(&str, &Glyphs); 3] = [("unicode", &UNICODE), ("ascii", &ASCII), ("nerd", &NERD)];

    fn info(state: CheckState) -> CheckInfo {
        CheckInfo { state, ..Default::default() }
    }

    /// Every state the reference names: what it is made of and how it must read in each set.
    #[test]
    fn every_state_reads_by_glyph_colour_and_word_in_every_set() {
        let t = Theme::named(crate::theme::DEFAULT);
        let cases = [
            (Status::Working, None, false, Kind::Working, ["⠹", "⠹", "⠹"], t.accent, "working"),
            (Status::Done, Some(CheckState::Checking), false, Kind::Checking, ["◜", "-", "◜"], t.sky(), "checking"),
            (Status::Blocked, None, false, Kind::Needs, ["●", "!", "\u{f0f3}"], t.blocked, "needs you"),
            (Status::Done, Some(CheckState::NeedsApproval), false, Kind::Approve, ["●", "!", "\u{f0f3}"], t.blocked, "needs you"),
            (Status::Done, None, false, Kind::Done, ["✓", "+", "\u{f00c}"], t.done, "done"),
            (Status::Done, Some(CheckState::Passed), false, Kind::Passed, ["✓", "+", "\u{f00c}"], t.done, "done"),
            (Status::Done, Some(CheckState::Stale), false, Kind::Stale, ["◷", "~", "\u{f017}"], t.text, "done · stale"),
            (Status::Done, Some(CheckState::Failed), false, Kind::Failed, ["✕", "x", "\u{f00d}"], t.err, "failed"),
            (Status::Done, Some(CheckState::Overridden), false, Kind::Over, ["⊘", "/", "\u{f05e}"], t.text, "done · overridden"),
            (Status::Done, Some(CheckState::CouldNotRun), false, Kind::Couldnt, ["?", "?", "\u{f128}"], t.violet(), "not checked"),
            (Status::Done, Some(CheckState::Never), false, Kind::Never, ["?", "?", "\u{f128}"], t.violet(), "not checked"),
            (Status::Idle, None, false, Kind::Idle, ["○", "o", "\u{f10c}"], t.muted, "idle"),
            (Status::Idle, None, true, Kind::Asleep, ["z", "z", "z"], t.muted, "asleep"),
        ];
        for (status, state, asleep, kind, glyphs, color, word) in cases {
            let check = state.map(info);
            assert_eq!(Kind::of(status, check.as_ref(), asleep), kind, "{status:?} {state:?}");
            for ((name, set), glyph) in SETS.iter().zip(glyphs) {
                assert_eq!(kind.mark(set, &t, 0, "⠹"), Mark { glyph, color, word }, "{kind:?} in {name}");
            }
        }
    }

    /// The dots set marks a session by colour and word: one dot for every state that has
    /// something to say, a ring at rest, and the moving marks left alone.
    #[test]
    fn dots_mark_every_session_by_a_coloured_dot_and_its_word() {
        let t = Theme::named(crate::theme::DEFAULT);
        let said = [
            (Kind::Needs, t.blocked, "needs you"),
            (Kind::Approve, t.blocked, "needs you"),
            (Kind::Done, t.done, "done"),
            (Kind::Passed, t.done, "done"),
            (Kind::Failed, t.err, "failed"),
            (Kind::Stale, t.text, "done · stale"),
            (Kind::Over, t.text, "done · overridden"),
            (Kind::Couldnt, t.violet(), "not checked"),
            (Kind::Never, t.violet(), "not checked"),
        ];
        for (kind, color, word) in said {
            assert_eq!(kind.mark(&DOTS, &t, 0, "⠹"), Mark { glyph: "●", color, word }, "{kind:?}");
        }
        for (kind, word) in [(Kind::Idle, "idle"), (Kind::Asleep, "asleep")] {
            assert_eq!(kind.mark(&DOTS, &t, 0, "⠹"), Mark { glyph: "○", color: t.muted, word }, "{kind:?}");
        }
        assert_eq!(Kind::Working.mark(&DOTS, &t, 0, "⠹").glyph, "⠹", "working keeps your spinner");
        assert_eq!(Kind::Checking.mark(&DOTS, &t, 1, "").glyph, DOTS.ring[1], "checking keeps its ring");
        assert_eq!(DOTS.done, UNICODE.done, "a check's own ✓ and ✕ keep their shapes");
        assert_eq!(DOTS.failed, UNICODE.failed);
    }

    #[test]
    fn a_check_says_nothing_while_the_agent_works_or_asks() {
        let failed = info(CheckState::Failed);
        assert_eq!(Kind::of(Status::Working, Some(&failed), false), Kind::Working);
        assert_eq!(Kind::of(Status::Blocked, Some(&failed), false), Kind::Needs);
        assert_eq!(Kind::of(Status::Idle, Some(&failed), false), Kind::Failed, "seen, it still failed");
        assert_eq!(Kind::of(Status::Done, Some(&info(CheckState::Pending)), false), Kind::Checking, "a run is still going");
    }

    #[test]
    fn the_ring_turns_through_its_four_frames() {
        for (_, set) in SETS {
            let frames: Vec<&str> = (0..5).map(|i| Kind::Checking.glyph(set, i, "")).collect();
            assert_eq!(frames, [set.ring[0], set.ring[1], set.ring[2], set.ring[3], set.ring[0]]);
        }
    }

    #[test]
    fn every_glyph_is_one_cell_but_the_ascii_branch() {
        for (name, g) in SETS {
            let all = [g.needs, g.done, g.failed, g.over, g.couldnt, g.stale, g.idle, g.asleep, g.pend, g.bar, g.close, g.fold, g.warn, g.arrow];
            for s in all.iter().chain(&g.ring).chain((name != "ascii").then_some(&g.branch)) {
                assert_eq!((s.chars().count(), s.width()), (1, 1), "{s:?} in {name}");
            }
        }
        assert_eq!(ASCII.branch, "git:");
        assert!(ASCII.ring.iter().chain([&ASCII.needs, &ASCII.done, &ASCII.stale]).all(|s| s.is_ascii()));
    }

    #[test]
    fn auto_is_dots_unless_the_icons_are_plain_and_a_setting_wins() {
        let fancy = Icons::default();
        let plain = Icons { blocked: "!".into(), done: "*".into(), idle: "-".into(), ..Icons::default() };
        assert_eq!(chosen("auto", &fancy), &DOTS);
        assert_eq!(chosen("dots", &plain), &DOTS);
        assert_eq!(chosen("auto", &plain), &ASCII);
        assert_eq!(chosen("unicode", &plain), &UNICODE);
        assert_eq!(chosen("ascii", &fancy), &ASCII);
        assert_eq!(chosen("nerd", &fancy), &NERD);
        assert_eq!(chosen("nonsense", &fancy), &DOTS, "unknown reads as auto");
    }
}
