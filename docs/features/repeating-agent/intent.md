# Intent: a working agent that repeats itself says so

Type: feature · Author: Cody · Status: accepted · Date: 2026-11-06
Plan: phase 5 in [TICKETS-TO-FILE](../../TICKETS-TO-FILE.md)

## Problem

An agent can be busy and going nowhere: calling the same tool with the same input again and
again. It is not silent, so the quiet mark never shows, and its row reads working like any other.

## Goal

A working agent that has made the same call five times running, or has been taking turns between
the same two calls, says so in words on its row: `repeating 6x`. `ls`, `maitty_list` and `wait`
say it too. Its status stays working and nothing rings.

## Scope

Anyone running Claude, Qwen, Codex or Gemini in a pane with Maitty's hooks connected. The hook
report, the daemon, the protocol, the terminal app's sidebar row, `ls`, `wait` and the MCP tools.

## Decisions (the owner, 2026-11-06)

- Words on the row only. Not in the Inbox, no sound, no popup. An agent polling CI repeats itself
  on purpose, so this is a hint and never an alert.
- Status never changes. The mark sits beside quiet and follows its rules.
- Repeats only, in this version: five the same in a row, or two calls taking turns over eight.
- The daemon gets a fingerprint of each call and never the tool's name or input.
- No new hook events, so nothing has to be trusted again in Codex.
- Not in the desktop app, which does not show quiet either.

## Out of scope

Failed calls in a row (Later). Stopping, nudging or restarting the agent: Maitty remains the
session manager (ROADMAP, Out, 2026-10-18). Agents whose hooks pass no tool input (Kimi,
Mastracode, Kilo, OpenCode, Pi, OMP).

## Links

- Tickets: 58, 59, 60 in [TICKETS-TO-FILE](../../TICKETS-TO-FILE.md)
