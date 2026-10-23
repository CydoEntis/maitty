# 0012. Add a flat rendering style

- **Status:** Accepted
- **Date:** 2026-10-18

## Context

The Cards layout is comfortable for focused work, but its borders, gaps and rounded
surfaces do not suit every terminal workflow. The supplied flat-layout handoff
asks for a full-bleed presentation with square edges, while keeping the same
sessions and core navigation.

## Decision

Keep one shared terminal session layout and offer two render styles through
`ui.layout`: `cards` (the default) and `flat`. Flat removes card spacing and
card borders (pill ends follow `ui.pill_caps`, as in Cards; see ADR 0013), uses the theme surface for default terminal cells, exposes a
bottom status line. Tab, Shift+Tab and Esc go to the program in the pane, as in
Cards; the leader's "Focus the sidebar" moves focus to the sidebar. Suppress splits below 150
columns. Existing sessions, tabs, sheets, agent state and mouse access remain
shared. Applications' explicit terminal colors remain visible.

## Alternatives considered

- **Keep Cards only:** would not provide the requested low-chrome presentation.
- **Build a separate layout and session model:** duplicates behavior and makes
  status and navigation harder to keep consistent.

## Consequences

The renderer and UI tests must cover both styles, including narrow sizes and
keyboard focus. Cards remains the default, and `ui.panes` continues to control
floating or tiled spacing only within Cards.
