# 0006. Secrets never enter chat

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

An agent sometimes needs a secret. Anything typed into a conversation is stored in history,
transcripts and logs.

## Decision

- An agent requests a secret through the MCP server. Only the user can answer, never from inside
  a pane.
- The agent receives a path or a ref, never the value. Two uses: a `0600` file in a `0700` folder,
  and `maitty secret run <ref> -- <command>`, which puts the value in that one command's environment.
- The daemon holds the value in memory only. A ref works once and expires after 24 hours. A
  delivery file is deleted when used or expired. A daemon restart forgets every secret.

## Alternatives considered

- **Pass the value in the message**: rejected, it lands in history and logs.

## Consequences

- No code path writes a secret value to a log, snapshot, fixture or history entry
  (`docs/SECURITY.md` has the checklist).
