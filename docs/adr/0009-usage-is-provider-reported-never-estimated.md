# 0009. Usage is what the provider reported, never estimated

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

People want to see how much their agents cost. Guessing from tokens and a price list is wrong
for subscriptions and goes stale.

## Decision

- The daemon keeps one row per (UTC day, session): agent, folder name, input and output tokens,
  cost, turns and seconds working. Tokens and cost are only what Claude Code (status line) and
  Codex (session file) report; turns and working time are what the daemon observed. No prompts,
  output, paths or model names are stored.
- A figure no provider reported is `null`, never zero, and never computed from a price list. A
  subscription session shows no cost.
- Providers report running totals, so the store keeps a cursor per session and adds only the
  difference. Rows are kept 90 days.
- It lives in `usage-<socket>.json` in the data folder, private to the user, written atomically.
  An unreadable file is kept as `.bak` and the store starts empty. Clients ask the daemon for it.

## Alternatives considered

- **Estimate cost from tokens**: rejected, it is wrong and looks exact.

## Consequences

- The desktop app reads usage through the protocol, so it still links no daemon crate (0004).
