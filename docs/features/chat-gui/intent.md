# Intent: standalone chat GUI, with native terminal continuity later

Type: feature
Author: Cody · Status: accepted · Date: 2026-10-27
Plan: [phases 34–37](../../ROADMAP.md#34--shared-conversations-and-codex-handoff--active)

## Problem

“I think a lot of people would benefit from Maitty as a GUI app as well.”
Chat should be the main GUI experience, with terminals available, while keeping
the existing TUI a great product. A GUI conversation must also be usable in the
TUI and vice versa.

## Proposed outcome

An optional desktop chat client works as a complete Codex and Claude experience,
while the existing TUI and terminal-only install remain supported. Native
conversation transfer between the GUI and TUI is optional follow-up work.

## Affected users and systems

Developers using local Maitty on Windows, macOS and Linux; the daemon, native
Codex and Claude sessions, local IPC, CLI, TUI and optional desktop client.

## Constraints

- Codex first; Claude must meet the same continuity contract before initial GUI release.
- Preserve native terminal agent interfaces, TUI layouts, keys and other providers.
- One execution owner per conversation. Any future handoff must be explicit and
  safe at idle.
- Daemon owns all processes. Closing either client leaves work running.
- Build a usable GUI before the dedicated GUI design pass.
- See ADR 0004.

## Out of scope

A replacement TUI chat renderer, a Maitty-built agent/tool loop, more structured
providers, a workflow runner, cloud sync, a browser service and new remote GUI
support. Existing SSH/TUI functionality remains supported. Arbitrary historical
chat browsing and attachments wait for separate scope.

## Open questions

None for phase 34. Later-phase decisions are recorded in the roadmap and do not
block foundation tickets.

## Tickets

Phase 34: [#197](https://github.com/CydoEntis/maitty/issues/197), [#198](https://github.com/CydoEntis/maitty/issues/198), [#199](https://github.com/CydoEntis/maitty/issues/199), [#200](https://github.com/CydoEntis/maitty/issues/200), [#201](https://github.com/CydoEntis/maitty/issues/201), [#202](https://github.com/CydoEntis/maitty/issues/202), [#203](https://github.com/CydoEntis/maitty/issues/203), [#204](https://github.com/CydoEntis/maitty/issues/204), [#205](https://github.com/CydoEntis/maitty/issues/205), [#206](https://github.com/CydoEntis/maitty/issues/206), [#207](https://github.com/CydoEntis/maitty/issues/207).
Later phases are not ticketed.
