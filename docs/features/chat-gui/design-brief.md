# Maitty desktop GUI design brief

## Approved design handoff

The user approved the v5 desktop handoff. Its README, prototype and runtime are
preserved in [`design-handoff/v5/`](design-handoff/v5/). Follow the README for
measurements and behavior; the HTML prototype is the visual source of truth. The
v4 handoff in [`design-handoff/v4/`](design-handoff/v4/) stays as the reference for
the New session dialog, toast stacking, Files editing and References, which v5
points back to. Prototype conversations, agent output, files, diffs, settings and
terminal are sample data only. The application must display daemon-backed data or
an honest unavailable/empty state instead of copying those samples as real work.

v5 has two planes: the window (the sidebar sits directly on it) and one inset main
card. The sidebar is 256px with a brand row, an icon-only New conversation button,
search and a footer holding the daemon status and Settings. A 46px tab strip holds
the open sessions, a Chat | Terminal switch and one Review button; Changes and
Context open from Review, docked at 1280px and wider and as a 480px overlay with a
scrim below that. Themes are Midnight (default), Abyss and Paper. Teal means active
work and primary actions, amber means needs you, red means failure, and done is a
muted green. Each status mark has its own shape, so meaning never depends on color.

Where the app differs from the prototype on purpose:

- Claude stays a working provider when its CLI is installed; the prototype's
  "Not supported yet" row applies only when the CLI is missing.
- Unsent-prompt delivery keeps the daemon-backed Check delivery flow instead of
  the prototype's "It arrived / Send anyway" buttons, so nothing is ever resent
  on a guess.
- Permissions are chosen per session in the New conversation dialog (Read only,
  Workspace write as the default, or Full access with no sandbox and no prompts;
  see [ADR 0043](../../adr/0043-full-access-conversations.md)). There is no global
  default in Settings. To change a running session's permissions, End session (which
stops the agent and keeps its history) and start a new one.
- The "Working · 1:42" timer shows only for a turn the GUI watched start. The
  daemon reports no start time, so a turn already running when the GUI opens
  shows "Working" without a clock rather than a wrong one.
- Open project (Ctrl/Cmd+O) is a palette for a typed path or a recent project;
  choosing one continues into the New conversation dialog, because a project
  exists in the sidebar only once it has a conversation.
- Review → Files reads the project folder through Tauri commands that serve only
  folders owned by a daemon session, and refuse paths that leave them.
- Syntax colors are a small built-in tokenizer (keywords, strings, numbers,
  comments), not a full grammar; unknown file types stay plain.

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
