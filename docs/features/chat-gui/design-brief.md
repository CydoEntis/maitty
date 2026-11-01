# Maitty desktop GUI design brief

## Approved design handoff

The user approved the v4 desktop handoff. Its README, prototype and supporting
references are preserved in [`design-handoff/v4/`](design-handoff/v4/). Follow
the README for measurements and behavior; the HTML prototype is the visual
source of truth. The v2 handoff remains in [`design-handoff/`](design-handoff/)
for history. Prototype conversations, agent output, files, diffs, settings and
terminal are sample data only. The application must display daemon-backed data
or an honest unavailable/empty state instead of copying those samples as real
work.

The handoff sets a 264px sidebar, an inset dark workspace card, and a responsive
right panel: docked at 1280px and wider, overlay below 1280px, with a 52px status
rail and sidebar drawer below 900px. It defines Midnight as default, Paper as
the light theme, Geist/Geist Mono typography, sea-glass teal for working and
primary actions, amber only for attention, and red for failure/recovery. Keep
status shapes and words so meaning does not depend on color.

Design a polished Windows, macOS and Linux desktop app for Maitty, a terminal
multiplexer for people running several coding agents at once. This is a real
developer tool, not a marketing page or generic chatbot. Users need to see which
agents need attention, open the right conversation quickly, inspect agent work,
and continue in the agent's native terminal when needed.

## Product rules

- The Rust daemon owns sessions and conversations. Closing the GUI never stops
  an agent.
- The existing TUI stays supported and shares sessions with the GUI. Switching
  between chat and native terminal never loses or duplicates conversation
  history.
- Show status plainly: working, needs you, done and idle, plus disconnected or
  recovery-needed states. Never imply work continued when state is uncertain.
- Preserve provider permission and sandbox choices. Approval requests show the
  exact action and only valid choices.
- Codex and Claude are supported when their local CLIs are available. Show each
  provider's actual readiness and never imply that a missing provider is ready.

## Main workspace

Design these areas:

1. A project and conversation sidebar with status marks, attention cues, search
   and a clear new conversation action. Make dozens of sessions scannable
   without making every row equally loud.
2. A conversation view with readable user and agent messages, visible streaming,
   and compact tool activity that expands to useful details without exposing raw
   logs by default.
3. A multiline composer with send and cancel controls, keyboard operation, and
   a clear running or approval-waiting state.
4. Review and context panels for changed files and diffs, the interactive native
   terminal, project/worktree, provider and permission context.
5. Recovery behavior that preserves drafts and selection across reconnects.
   Clearly explain connection loss, failed sends, uncertain delivery and
   recovery-needed states. Never silently retry a prompt that may have arrived.

## Screens and states to design

- First launch and no projects/conversations
- Several projects and conversations with mixed agent states
- Streaming response with tool activity
- Agent waiting for user input or approval
- Diff review and native terminal
- Empty conversation, loading, provider unavailable, daemon disconnected and
  reconnecting, send failure, recovery required
- Compact window and keyboard-only navigation

## Visual direction

Implement the approved handoff directly. Treat shadcn/ui as primitives; keep
Maitty's focused developer-workspace character and do not add unrelated project
management or cloud-agent features. Honor reduced motion, visible keyboard
focus, screen-reader status announcements and WCAG AA text contrast. Keep the
final layout and compact states usable at 1440×900, 1024×700 and 800×600.

## References

Study these for selected patterns, without copying their branding, exact layouts
or unrelated features:

- [Codex app](https://openai.com/index/introducing-the-codex-app/): project/thread
  organization, parallel work and review surfaces.
- [Zed Agent Panel](https://zed.dev/docs/ai/agent-panel): separation between
  thread navigation, agent conversation, terminal and review.
- [Linear triage](https://linear.app/changelog/2021-06-29-linear-release-and-issue-triage):
  keyboard-friendly navigation and scannable work lists.

Do not assume Maitty needs cloud agents, team collaboration or project
management features.

## Deliverables

The screen map, visual mockup, interaction notes, tokens, keyboard guidance,
accessibility requirements and first-release scope are in the approved handoff.
Use its "Must" list as the first usable Codex GUI scope; its "Can follow" list
is later work. Verify the running app at 1440×900, 1024×700 and 800×600.
