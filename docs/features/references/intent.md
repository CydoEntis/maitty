# Intent: references

Type: feature
Author: Cody · Status: accepted
Date: 2026-10-24 · Plan: [phase 28](../../ROADMAP.md#28--references--planned-2026-10-24)

## Problem

"A way you can add reference images to your app, so it knows what to pull for designs, and you
can open it and see, add and remove screenshots, files, etc." Today a screenshot has to be pasted
into each agent's prompt, every time.

## Proposed outcome

Each project has `.maitty/references/` for screenshots and design files. A References view in
the side panel lists them; you add one by pasting a clipboard image or picking a file, remove one
(asked first), and open an image in the system's viewer. Maitty's MCP server gains a tool that
lists a project's references with their paths, so agents doing design work can find them.

## Affected users and systems

Anyone giving agents design work. The client (the side panel, built in Phase 26), the MCP
server (`src/mcp.rs`), the repo's `.maitty/` folder.

## Constraints

- The folder is in the repo (decided 2026-10-24): agents read it like any file; you choose
  whether to commit it.
- Agents learn of it through an MCP tool (decided 2026-10-24); nothing is written into prompts
  or agent instruction files.
- Images open in the system viewer; the terminal shows none (terminal graphics cut in phase 7).
- Builds on Phase 26's side panel.

## Out

- Notes or tags per reference (the file name is its description).

## Open questions

Settled when Phase 28 is ticketed: whether `.maitty/references/` is gitignored by default.

## Tickets

(filled in when the phase is ticketed)
