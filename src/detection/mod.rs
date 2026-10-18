//! Pinned agent-owned screen evidence; native lifecycle reports remain separate.

mod regions;

use crate::protocol::Status;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Clone, Copy)]
pub(crate) struct DetectionInput<'a> {
    pub screen: &'a str,
    pub osc_title: &'a str,
    pub osc_progress: &'a str,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum State {
    Working,
    Blocked,
    Idle,
    Unknown,
}

#[derive(Default, Deserialize)]
struct Gate {
    #[serde(default)]
    all: Vec<Gate>,
    #[serde(default)]
    any: Vec<Gate>,
    #[serde(default, rename = "not")]
    not: Vec<Gate>,
    #[serde(default)]
    contains: Vec<String>,
    #[serde(default)]
    regex: Vec<String>,
    #[serde(default)]
    line_regex: Vec<String>,
}

#[derive(Deserialize)]
struct Rule {
    id: String,
    state: Option<State>,
    #[serde(default)]
    priority: i32,
    #[serde(default = "whole_recent")]
    region: String,
    #[serde(default)]
    visible_idle: bool,
    #[serde(default)]
    visible_blocker: bool,
    #[serde(default)]
    skip_state_update: bool,
    #[serde(flatten)]
    gate: Gate,
}

fn whole_recent() -> String {
    "whole_recent".into()
}

#[derive(Deserialize)]
struct Manifest {
    id: String,
    rules: Vec<Rule>,
}

struct CompiledGate {
    all: Vec<CompiledGate>,
    any: Vec<CompiledGate>,
    not: Vec<CompiledGate>,
    contains: Vec<String>,
    regex: Vec<Regex>,
    line_regex: Vec<Regex>,
}

impl CompiledGate {
    fn compile(gate: &Gate) -> Result<Self, regex::Error> {
        let nested = |gates: &[Gate]| {
            gates
                .iter()
                .map(Self::compile)
                .collect::<Result<Vec<_>, _>>()
        };
        let regexes = |patterns: &[String]| {
            patterns
                .iter()
                .map(|pattern| Regex::new(pattern))
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(Self {
            all: nested(&gate.all)?,
            any: nested(&gate.any)?,
            not: nested(&gate.not)?,
            contains: gate
                .contains
                .iter()
                .map(|text| text.to_lowercase())
                .collect(),
            regex: regexes(&gate.regex)?,
            line_regex: regexes(&gate.line_regex)?,
        })
    }

    fn matches(&self, text: &str, lower: &str) -> bool {
        self.contains.iter().all(|needle| lower.contains(needle))
            && self.regex.iter().all(|regex| regex.is_match(text))
            && self
                .line_regex
                .iter()
                .all(|regex| text.lines().any(|line| regex.is_match(line)))
            && self.all.iter().all(|gate| gate.matches(text, lower))
            && (self.any.is_empty() || self.any.iter().any(|gate| gate.matches(text, lower)))
            && !self.not.iter().any(|gate| gate.matches(text, lower))
    }
}

struct CompiledRule {
    rule: Rule,
    gate: CompiledGate,
}
struct CompiledManifest {
    id: String,
    rules: Vec<CompiledRule>,
}

macro_rules! manifests {
    ($($name:literal),+ $(,)?) => { &[$(include_str!(concat!("../../vendor/herdr-detection/", $name, ".toml"))),+] };
}

const SOURCES: &[&str] = manifests!(
    "amp",
    "antigravity",
    "claude",
    "cline",
    "codex",
    "cursor",
    "devin",
    "droid",
    "gemini",
    "github-copilot",
    "grok",
    "hermes",
    "kilo",
    "kimi",
    "kiro",
    "letta",
    "maki",
    "muse",
    "opencode",
    "pi",
    "qodercli",
    "qwen"
);

static MANIFESTS: LazyLock<Vec<CompiledManifest>> = LazyLock::new(|| {
    SOURCES
        .iter()
        .map(|source| {
            // These are checked-in source assets, never user files; the test gate validates them.
            let manifest: Manifest =
                toml::from_str(source).expect("bundled detection manifest must parse");
            let mut rules: Vec<_> = manifest
                .rules
                .into_iter()
                .map(|rule| {
                    let gate = CompiledGate::compile(&rule.gate)
                        .expect("bundled detection expressions must compile");
                    CompiledRule { rule, gate }
                })
                .collect();
            rules.sort_by_key(|rule| std::cmp::Reverse(rule.rule.priority));
            CompiledManifest {
                id: manifest.id,
                rules,
            }
        })
        .collect()
});

#[derive(Default)]
pub(crate) struct Signal<'a> {
    pub status: Option<Status>,
    pub skip: bool,
    pub visible_idle: bool,
    pub visible_blocker: bool,
    pub rule: &'a str,
}

pub(crate) fn detect(agent: &str, input: DetectionInput<'_>) -> Signal<'static> {
    let Some(manifest) = MANIFESTS.iter().find(|manifest| manifest.id == agent) else {
        return Signal::default();
    };
    for entry in &manifest.rules {
        let text = regions::region(input, &entry.rule.region);
        if entry.gate.matches(text, &text.to_lowercase()) {
            return Signal {
                status: match entry.rule.state {
                    Some(State::Working) => Some(Status::Working),
                    Some(State::Blocked) => Some(Status::Blocked),
                    Some(State::Idle) => Some(Status::Idle),
                    _ => None,
                },
                skip: entry.rule.skip_state_update,
                visible_idle: entry.rule.visible_idle,
                visible_blocker: entry.rule.visible_blocker,
                rule: &entry.rule.id,
            };
        }
    }
    Signal::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_screen_agent_recognizes_its_native_chrome() {
        for (agent, screen, title, progress, expected) in [
            ("amp", "", "Plugin confirmation needed", "", Status::Blocked),
            ("agy", "navigate · tab amend", "", "", Status::Blocked),
            ("claude", "", "⠋ project", "", Status::Working),
            ("cline", "let cline use this tool", "", "", Status::Blocked),
            ("codex", "", "Action Required", "", Status::Blocked),
            (
                "cursor",
                "write to this file? proceed (y) · reject & propose changes",
                "",
                "",
                Status::Blocked,
            ),
            (
                "devin",
                "running tools · esc to interrupt",
                "",
                "",
                Status::Working,
            ),
            (
                "droid",
                "> yes, allow\n↑↓ to navigate · enter to select · esc to cancel",
                "",
                "",
                Status::Blocked,
            ),
            ("gemini", "│ Allow execution", "", "", Status::Blocked),
            (
                "copilot",
                "esc cancel · enter to select",
                "",
                "",
                Status::Blocked,
            ),
            ("grok", "", "Action Required", "", Status::Blocked),
            ("hermes", "", "⚠ question", "", Status::Blocked),
            ("kilo", "esc interrupt", "", "", Status::Working),
            (
                "kimi",
                "run this command? · choose approve · ↵ confirm",
                "",
                "",
                Status::Blocked,
            ),
            (
                "kiro",
                "> ask a question or describe a task",
                "",
                "",
                Status::Idle,
            ),
            ("letta", "", "", "4;3", Status::Blocked),
            (
                "maki",
                "permission required · y allow · n deny",
                "",
                "",
                Status::Blocked,
            ),
            (
                "muse",
                "Do you trust this workspace?\nTrust and continue",
                "",
                "",
                Status::Blocked,
            ),
            ("opencode", "△ Permission required", "", "", Status::Blocked),
            ("pi", "⠋ Working", "", "", Status::Working),
            ("qodercli", "(esc to cancel,", "", "", Status::Working),
            ("qwen", "", "◐ project", "", Status::Working),
        ] {
            assert_eq!(
                detect(
                    agent,
                    DetectionInput {
                        screen,
                        osc_title: title,
                        osc_progress: progress
                    }
                )
                .status,
                Some(expected),
                "{agent}"
            );
        }
    }

    #[test]
    fn every_bundled_manifest_compiles_and_has_a_known_region() {
        assert_eq!(MANIFESTS.len(), 22);
        for manifest in MANIFESTS.iter() {
            assert!(!manifest.rules.is_empty(), "{}", manifest.id);
            for entry in &manifest.rules {
                let region = &entry.rule.region;
                assert!(
                    matches!(
                        region.as_str(),
                        "whole_recent"
                            | "after_last_prompt_marker"
                            | "before_current_prompt_marker"
                            | "whole_recent_without_current_prompt_marker"
                            | "osc_title"
                            | "osc_progress"
                            | "prompt_box_body"
                            | "last_non_empty_above_prompt_box"
                            | "after_last_horizontal_rule"
                    ) || region.starts_with("bottom_non_empty_lines(")
                        || region.starts_with("top_non_empty_lines("),
                    "unknown region: {region}"
                );
            }
        }
    }

    #[test]
    fn native_chrome_wins_over_old_prose_and_viewers_keep_previous_state() {
        let status = |agent, screen| {
            detect(
                agent,
                DetectionInput {
                    screen,
                    osc_title: "",
                    osc_progress: "",
                },
            )
        };
        assert_eq!(status("pi", "⠋ Working").status, Some(Status::Working));
        assert_eq!(
            status("opencode", "esc to interrupt").status,
            Some(Status::Working)
        );
        assert_eq!(
            status("opencode", "△ Permission required").status,
            Some(Status::Blocked)
        );
        assert_eq!(
            status("muse", "Enter to select · Tab for an optional note").status,
            Some(Status::Blocked)
        );
        assert_eq!(
            status("muse", "We documented Enter to select earlier.").status,
            None
        );
        assert!(
            status(
                "codex",
                "↑/↓ to scroll\npgup/pgdn to page\nhome/end to jump\nq to quit\nesc to edit prev"
            )
            .skip
        );
        assert_eq!(status("codex", "Working (3m 12s • esc to interrupt) · 3 background terminals running · /ps to view · /stop to close\n› Ask Codex to do anything").status, Some(Status::Working));
    }
}
