# 0051. The daemon keeps provider-reported usage per session per day

- **Status:** Accepted
- **Date:** 2026-11-04
- **Relates to:** [0045](0045-each-session-belongs-to-one-app.md) (the daemon owns every
  session), AGENTS.md non-negotiables 1 and 5

## Context

The Usage and cost page (design handoff C.7) needs tokens in and out, cost, turns and time working
per session, by agent and folder, for a day, week or month. The daemon only kept one rolling day
of "cost" and a by-agent-and-folder summary in memory, so a restart lost it and nothing covered a
week. The owner decided (2026-11-04) that the daemon may store provider-reported usage per
session per day, and that it must never estimate.

## Decision

- **What is kept.** One row per (UTC day, session): agent (`claude` or `codex`), folder name,
  input tokens, output tokens, cost, turns and seconds working. Tokens and cost are only what
  Claude Code (status line) and Codex (session file) report; turns and working time are what the
  daemon observed itself. No prompts, no output, no paths, no model names.
- **Never estimate.** A figure no provider reported is `null`, not zero, and is never computed
  from tokens or a price list. A session whose reports carry plan limits is a subscription: its
  cost is dropped, so a subscription session shows no cost. Codex reports no cost at all.
- **Running totals.** Providers report totals so far. The store keeps the last total seen per
  session (a cursor) and adds only the difference to today's row. A total that falls is a new
  session counting from zero. Cursors are saved with the rows, or a restart would count a whole
  session again.
- **Retention.** 90 days, today included. Older rows and cursors are pruned when the file is
  loaded and each time it is saved.
- **Where.** `usage-<socket>.json` in the data folder (a nested daemon has its own), private to
  the user, versioned, written atomically in the background and when the daemon exits. An
  unreadable file or one from a newer build is kept as `.bak` and the store starts empty, with a
  logged reason. The daemon never stops over it.
- **Why in the daemon.** It is the only process that outlives clients and sees every provider's
  reports. Clients ask `Query::UsageReport { period }` and get `Reply::UsageReport` (protocol 70,
  human clients only). The desktop app reaches it through the `usage_report` command and
  `maitty-desktop-client`, so it still links no daemon crate.

## Consequences

- Days are UTC; a person far from UTC sees a day change at an odd hour. Periods are rolling
  windows ending today (1, 7, 30 days), not calendar weeks or months.
- Structured chat conversations report no usage yet, so they are absent until their provider
  events are mapped.
- A new fact (for example a per-turn breakdown) adds a defaulted field to the row; a change that
  cannot be read that way raises the file's `version`.
