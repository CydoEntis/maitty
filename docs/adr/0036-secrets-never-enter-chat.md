# 0036. Secrets reach agents by file or wrapper, never in chat

- **Status:** Accepted
- **Date:** 2026-10-29

## Context

An agent that needs an API key asks for it in chat, so the key is typed into the transcript.
Maitty can ask the user instead, through a masked prompt, but the agent is a separate process:
nothing can put a variable into the environment of a process that is already running, and a
value printed in a tool result is back in the transcript. The daemon already authenticates a
pane by its `MAITTY_PANE_TOKEN` and asks the user on the caller's row for helpers
(`src/daemon/helpers.rs`).

## Decision

- An agent requests a secret through the MCP server. Only the user can answer, and never from
  inside a pane.
- The agent receives a path or a ref, never the value. Two ways to use it: a `0600` file in a
  `0700` folder, and `maitty secret run <ref> -- <command>`, which puts the value in that one
  command's environment.
- The daemon holds the value in memory only. A ref works once and expires after 24 hours. A
  delivery file is deleted when it is used or expires. A daemon restart forgets every secret.

## Alternatives considered

- **Return the value in the tool result:** it lands in the transcript, which is the problem.
- **Save secrets on disk:** a secret at rest needs OS-level protection on Windows, macOS and
  Linux, and a leak outlives the session.
- **A file only:** tools that read only environment variables would need a shell workaround.
- **A wrapper only:** tools that insist on a file would not work.

## Consequences

The agent needs two small habits (read a file, or prefix a command), which the tool's reply
explains. The user retypes a secret after a daemon restart. Revisit if users want secrets to
persist, or once Maitty has a credential store to put them in.
