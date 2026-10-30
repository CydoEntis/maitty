//! Metadata for agents with native status integrations.

/// An agent that Maitty can connect to native status reporting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusAgent {
    pub id: &'static str,
    pub name: &'static str,
    pub adapter: StatusAdapter,
}

/// Native integration implementation selected by each catalog entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusAdapter {
    Claude,
    Codex,
    Gemini,
    Qwen,
    OpenCode,
    Pi,
    Omp,
    Kilo,
    Kimi,
    MastraCode,
}

/// Agents offered by both the CLI and the TUI's Connect menu.
pub const STATUS_AGENTS: &[StatusAgent] = &[
    StatusAgent {
        id: "claude",
        name: "Claude Code",
        adapter: StatusAdapter::Claude,
    },
    StatusAgent {
        id: "codex",
        name: "Codex",
        adapter: StatusAdapter::Codex,
    },
    StatusAgent {
        id: "gemini",
        name: "Gemini CLI",
        adapter: StatusAdapter::Gemini,
    },
    StatusAgent {
        id: "qwen",
        name: "Qwen Code",
        adapter: StatusAdapter::Qwen,
    },
    StatusAgent {
        id: "opencode",
        name: "OpenCode",
        adapter: StatusAdapter::OpenCode,
    },
    StatusAgent {
        id: "pi",
        name: "Pi",
        adapter: StatusAdapter::Pi,
    },
    StatusAgent {
        id: "omp",
        name: "OMP",
        adapter: StatusAdapter::Omp,
    },
    StatusAgent {
        id: "kilo",
        name: "Kilo",
        adapter: StatusAdapter::Kilo,
    },
    StatusAgent {
        id: "kimi",
        name: "Kimi",
        adapter: StatusAdapter::Kimi,
    },
    StatusAgent {
        id: "mastracode",
        name: "Mastra Code",
        adapter: StatusAdapter::MastraCode,
    },
];

#[cfg(test)]
mod tests {
    use super::STATUS_AGENTS;
    use std::collections::HashSet;

    #[test]
    fn status_agent_ids_are_unique_and_display_names_are_present() {
        let mut ids = HashSet::new();

        for agent in STATUS_AGENTS {
            assert!(
                ids.insert(agent.id),
                "duplicate integration id: {}",
                agent.id
            );
            assert!(!agent.name.trim().is_empty());
        }
    }
}
