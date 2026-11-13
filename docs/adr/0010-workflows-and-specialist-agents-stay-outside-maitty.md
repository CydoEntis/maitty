# 0010. Workflows and specialist agents stay outside Maitty; Maitty shows them

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

Users want two ways of working with specialist agents (a reviewer, a builder, a security
checker), and the same result from both:

- **While they work:** they chat with one agent, and it calls a specialist when the task fits.
- **While they are away:** a workflow such as Kitchen (a Claude Code plugin in its own repository)
  takes tickets through fixed steps, each done by a specialist, until each is a reviewed pull request.

Claude Code and Codex both ship this natively. Specialists are agent files (`.claude/agents/`,
`.codex/agents/`) chosen by their description or called by name, and both report subagent start
and stop through hooks. Maitty already receives those reports for both providers and lists a
session's subagents, but it shows only their task and result, not their work. It has no view of a
workflow's steps, and it does not know which ticket a subagent is working on.

A Maitty-run workflow runner and saved role profiles were removed on 2026-10-18
([FEATURES](../FEATURES.md)): a runner inside Maitty copies what the providers do, ties Maitty to
one way of working, and makes every stalled stage Maitty's fault. Status must stay right
([0003](0003-status-and-conversation-trust.md)).

## Decision

1. **Maitty does not run workflows or define agents.** It never starts, orders, retries or stops a
   workflow step, and never creates or edits a specialist's definition. Specialists live with the
   provider (agent files, skills, plugins). Workflows such as Kitchen live in their own projects
   and run inside an agent session like any other work. Neither is built into Maitty.
2. **Maitty shows specialists working, live, for every provider that reports them.** Their start
   and stop come only from the owning runtime (hooks verified as in 0003, or the daemon-owned
   connection). Their work comes from the provider's own record of that subagent, read only when
   the daemon has tied it to the pane's session: Claude's per-subagent transcript beside the
   session's transcript, Codex's child thread named by its parent thread, and Codex app-server
   collab items for structured conversations. This is history, so it is display only and never
   changes status.
3. **A workflow reports its progress to Maitty through one provider-neutral command:**
   `maitty workflow report`. Any process in the pane may run it, so it works the same under
   Claude Code, Codex or a plain script. The daemon accepts it as it accepts a hook, verified by the
   pane's process tree or `MAITTY_PANE_TOKEN`, so every report belongs to a known session. A report
   names one run of the workflow: the workflow's name, a run id, an optional ticket (key, title, link),
   the ordered steps, and for each step its state (waiting, working, passed, failed, sent back,
   needs you, skipped) and an optional short note. The daemon validates and bounds every field,
   keeps the latest report per run with the pane, and sends it to clients. It is a new message, so
   it gets its own `introduced_in` arm and a `PROTOCOL_VERSION` bump.
4. **Workflow progress is display only.** It never changes a pane's status and never raises an
   alert of its own. A workflow that needs the user stops its agent and asks, and the agent's
   trusted status says "needs you"; the workflow view explains why.
5. **Maitty never reads a workflow's own files** (for example `.kitchen/jobs/`). A file in a
   repository is not tied to a session and holds one tool's format. Maitty knows a workflow only
   through its reports.
6. **No plugin system or marketplace for now.** The report command is the whole contract a third
   party needs. Maitty lists a project's specialist files read-only only if a later ticket asks.

## Alternatives considered

- **Build Kitchen into Maitty**: it ties Maitty to one workflow that runs only under Claude Code,
  ships every workflow change as a Maitty release, and repeats the runner cut on 2026-10-18.
- **Maitty reads each workflow's state files from the repository**: the file is not tied to a
  session, so tickets could not be placed under the session working them. Each tool's format would
  need its own reader, and 0003 forbids reading unverified files.
- **A plugin system and extension marketplace**: installing, sandboxing, updating and supporting
  third-party code on three platforms is a product of its own. Today there is one workflow, Kitchen.
  The report command does not rule this out later.
- **Maitty's own agent editor**: Claude Code's `/agents` and Codex's files already do this, and the
  two formats differ and keep changing.

## Consequences

- Specialists and workflows change without a Maitty release. A Codex version of a workflow needs
  only Codex agent files and a call to the same command.
- Maitty maintains one small message, the report command, and two readers of provider subagent
  records. A change to the report's shape follows the protocol rules (0002).
- A workflow that never calls the command still runs. Its subagents show live, but its steps do not.
- What a step shows is only as true as the workflow's report. Maitty guarantees it came from the
  pane, not that it is accurate.
- Revisit point 6 when several independent workflows report to Maitty and users ask to install them
  from inside it.

## To confirm

1. How a step names the subagent doing it, so the view can open that subagent's work. The step
   could carry the subagent's id where the workflow knows it, or the daemon could link a subagent
   that starts after the step's report in the same pane. Settle this from a live run (tracked in Plane).
