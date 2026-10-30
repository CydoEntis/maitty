# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Optional cross-platform desktop shell: Tauri 2 with React, TypeScript, Vite,
shadcn/ui and Tailwind. Keep the Rust CLI/TUI install independent of it.

## Users

Developers who run several coding agents across projects and worktrees on
Windows, macOS or Linux. They need to notice when an agent needs them, move to
the right session, and keep work running when the interface closes.

## Product Purpose

Maitty is a terminal multiplexer for running many coding agents at once. It
shows which agents are working, need attention, are done or idle; makes it easy
to switch between them; and keeps their sessions alive when the UI closes.
Success means Maitty does not lose or misreport an agent's state.

## Positioning

The daemon is the single owner of sessions and agent processes. The TUI and the
optional chat GUI are views over the same daemon-owned work, so users can move
between an agent's native terminal and structured conversation without creating
a competing session owner.

## Operating Context

Users work across multiple local projects and worktrees and may run Claude Code,
Codex, Gemini, OpenCode and other supported agents. The terminal interface is
available without the desktop frontend. The desktop GUI is optional and starts
with Codex structured conversations; Claude continuity is planned before the
initial GUI release.

## Capabilities and Constraints

- The daemon owns sessions, PTYs, provider runtimes and persistence. Closing a
  client never kills a pane or conversation.
- Status comes only from the owning runtime and must distinguish working,
  needs you, done and idle.
- Preserve exact provider identity and permissions during handoff. Refuse
  ambiguous or unsafe transfers; never silently create a replacement session.
- The TUI and terminal-only install remain supported alongside the optional GUI.
- Windows, macOS and Linux are first-class targets.
- The GUI's required sessions, composer, streaming, tool activity, approvals,
  cancellation, diff access and interactive terminal are defined in
  `docs/features/chat-gui/spec.md`.

## Brand Commitments

Use the Maitty name and the tagline “All agents on deck.” Keep the current
navy/teal identity and amber-for-attention meaning across the desktop GUI.
Product copy is direct and calm; statuses use plain words.

## Evidence on Hand

The Rust daemon and TUI are implemented. The optional desktop GUI and real
Codex-to-native-terminal continuity are not yet verified. Do not invent user
quotes, adoption numbers or performance claims.

## Product Principles

- Keep the daemon as the one owner of agent work.
- Make every agent's state trustworthy and easy to scan.
- Preserve native agent workflows and terminal-only use.
- Make handoffs explicit, safe and recoverable.
- Keep conversation data local and private.

## Accessibility & Inclusion

The GUI must work with keyboard alone, show visible focus, use semantic and
named controls, keep status understandable without color, respect reduced
motion and meet WCAG AA text contrast.
