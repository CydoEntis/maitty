# GUI chat and native terminal conversation continuity

Research date: 2026-10-27. Source inspection only; the three applications were not run.
This is a proposal, not a change to Maitty's terminal-only policy or roadmap.

## Requirement

A conversation started in the GUI must continue in Maitty's existing terminal
workflow, and a conversation started there must continue in GUI chat. Here,
terminal workflow means the agent's own interactive CLI inside a Maitty pane.
A shell panel beside a separate chat does not satisfy this requirement.

Two different capabilities matter: attaching two views to one live process, and
handing an idle conversation between separate processes using the same native
session ID. The inspected code establishes useful examples of the latter; this
research does not establish concurrent native CLI and GUI control of one agent.

## Comparison

| Project | Chat execution | Native conversation continuity | Relevant lesson |
| --- | --- | --- | --- |
| T3 Code | Codex app-server and Claude Agent SDK adapters; server event persistence | Native-session import and exact provider resume IDs; seamless round-trip live synchronization not verified | Separate provider events, app history and provider runtime identity |
| PandaCode | Codex app-server, Claude stream-json, ACP and provider-specific transports | Imports Claude/Codex conversations; launches native harness consoles with resume IDs; prevents chat/console ownership overlap | Closest concrete example of controlled GUI/native CLI handoff |
| Terax AI | Its own AI SDK model/tool loop with app-owned messages | Terminal context and injection are present; native Claude/Codex session continuity not established | Useful UI/tool integration reference, a different conversation architecture |

## T3 Code

Inspected commit: `5b70d81b0d77bdd5883a835173735d55992009af`.

- [CodexAdapterV2](https://github.com/pingdotgg/t3code/blob/5b70d81b0d77bdd5883a835173735d55992009af/apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts)
  uses `effect-codex-app-server`; its resume paths call `thread/resume`.
- [ClaudeAdapterV2](https://github.com/pingdotgg/t3code/blob/5b70d81b0d77bdd5883a835173735d55992009af/apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts)
  imports `@anthropic-ai/claude-agent-sdk`, carries `sessionId` or `resume`
  query options, and maps provider activity into application events.
- [AgentSessionImporter](https://github.com/pingdotgg/t3code/blob/5b70d81b0d77bdd5883a835173735d55992009af/apps/server/src/project/AgentSessionImporter.ts)
  uses native-session scanning, retains provider identity, and emits imported
  messages as app events. It defines an explicit error for overwriting an
  imported thread that already has non-imported activity. Import is therefore
  not evidence of unrestricted ongoing two-way transcript merging.
- [OrchestrationEventStore](https://github.com/pingdotgg/t3code/blob/5b70d81b0d77bdd5883a835173735d55992009af/apps/server/src/persistence/OrchestrationEventStore.ts)
  uses SQLite-backed event storage and publishes after the event/projection
  transaction commits. The app's conversation model is more than a terminal log.

Conclusion: strong reference for structured chat adapters and durable history.
Native import/resume is real; a complete GUI → native terminal → same GUI thread
history refresh was not traced or exercised in this investigation.

## PandaCode

Inspected commit: `25d3466e4b1cbc51108f57cfcdd73c6a18d01317`.

- [providers.rs](https://github.com/aicodeking/pandacode/blob/25d3466e4b1cbc51108f57cfcdd73c6a18d01317/src-tauri/src/providers.rs)
  configures Codex `app-server`, Claude `-p --input-format stream-json
  --output-format stream-json`, and ACP for several other harnesses. Claude
  receives an explicit resume ID and explicit permission mode. Its
  `session/load` handling notes that replayed history is already owned by the UI.
- [conversation_import.rs](https://github.com/aicodeking/pandacode/blob/25d3466e4b1cbc51108f57cfcdd73c6a18d01317/src-tauri/src/conversation_import.rs)
  imports native Claude logs and Codex conversation messages into tasks while
  retaining native session IDs. Existing imported tasks are returned/skipped;
  this import path is not an automatic incremental refresh of existing history.
- [terminal.rs](https://github.com/aicodeking/pandacode/blob/25d3466e4b1cbc51108f57cfcdd73c6a18d01317/src-tauri/src/terminal.rs#L287)
  constructs literal native resume arguments: Codex `resume <id>`, Claude
  `--resume <id>`, and provider-specific variants. It explicitly refuses
  unsupported combinations, including resuming a Copilot ACP task in its console.
- [task_controls.rs](https://github.com/aicodeking/pandacode/blob/25d3466e4b1cbc51108f57cfcdd73c6a18d01317/src-tauri/src/task_controls.rs#L618)
  reserves ownership through a task lease while opening the harness console,
  revalidates provider/session/worktree/model/mode settings, and refuses active
  Codex goals. [Chat admission](https://github.com/aicodeking/pandacode/blob/25d3466e4b1cbc51108f57cfcdd73c6a18d01317/src-tauri/src/lib.rs#L1382)
  rejects a task already running in an agent or harness console.
- [terminal.ts](https://github.com/aicodeking/pandacode/blob/25d3466e4b1cbc51108f57cfcdd73c6a18d01317/src/terminal.ts)
  renders terminal output and forwards input/resizes. Its inspected exit path
  does not establish a native-conversation history refresh into chat.

Conclusion: the clearest handoff reference. It demonstrates actual reuse of
native sessions, not just a shell beside chat. However, retaining the provider's
context and updating every message visible in the GUI are separate obligations.
Automatic reconciliation after terminal turns remains unverified; the importer
alone does not provide it.

## Terax AI

Inspected commit: `640e8b2e6b9a9904af3d1b1161ad936864eb4a9a`.

- [chatRuntime.ts](https://github.com/crynta/terax-ai/blob/640e8b2e6b9a9904af3d1b1161ad936864eb4a9a/src/modules/ai/store/chatRuntime.ts)
  constructs an AI SDK `Chat`, supplies selected model and credentials, and
  exposes terminal context, PTY injection, file context and managed-agent tools.
- [agent.ts](https://github.com/crynta/terax-ai/blob/640e8b2e6b9a9904af3d1b1161ad936864eb4a9a/src/modules/ai/lib/agent.ts)
  uses `streamText`, converts UI messages into model messages, and builds its
  own tool collection. This is an application-owned agent loop.
- [sessions.ts](https://github.com/crynta/terax-ai/blob/640e8b2e6b9a9904af3d1b1161ad936864eb4a9a/src/modules/ai/lib/sessions.ts)
  persists app `UIMessage` histories and session metadata with serialized storage
  operations. These IDs were not found mapped to native Claude/Codex resume IDs
  in the inspected chat paths.

Conclusion: terminal-aware chat is supported, but that does not establish that a
native CLI can resume the same conversation. Adopting this model would require
Maitty to own an agent runtime or add a separate native-session bridge.

## Recommendation for Maitty

This section is an inference and proposed design, not existing functionality.

Keep the daemon as the owner of every process. Add structured provider adapters
there, and let GUI chat consume daemon messages. Preserve the existing PTY
terminal path for the TUI and for providers without verified structured support.

Use a stable Maitty conversation identity that records provider, exact native
session ID, account/profile, working directory/worktree, execution owner and a
history cursor. Never use a most-recent-session fallback for a deliberate
handoff: it can silently continue a different conversation.

Start with idle handoff under daemon control:

1. GUI starts a provider session through its structured adapter.
2. Before native terminal takeover, reach a safe idle state and relinquish the
   structured runtime's ownership. Reuse the same exact provider session.
3. Launch the native interactive CLI inside a daemon-owned PTY; the existing
   Maitty TUI attaches to that pane.
4. Before returning to GUI control, relinquish terminal runtime ownership and
   read provider history since the saved cursor. Reconcile messages and tool
   results without duplicates, then resume structured execution.
5. Support the reverse starting point by discovering a terminal-created native
   session through trusted pane evidence and binding it to a Maitty conversation.

Do not promise simultaneous input to a native CLI and structured runtime. Both
frontends can observe daemon state, but one runtime must own conversation writes
unless a provider explicitly supports a stronger sharing model.

Prove one provider on Windows first, then Linux and macOS: GUI prompt, terminal
follow-up, GUI follow-up, complete history in both directions, unchanged native
session ID/account/worktree, correct permissions, cancellation and reconnects.
Closing either client must leave daemon-owned work alive. Test an active turn,
pending permission, missing session and provider-version mismatch explicitly.

Local context inspected by the parent investigation: `src/protocol.rs` currently
transports snapshots and PTY bytes, not structured chat turns;
`src/daemon/persist.rs` stores agent session/account/cwd; and
`src/daemon/restore.rs` already resumes configured native session IDs. These are
useful foundations, not proof of a bidirectional chat bridge.

## Reuse and verification limits

- T3 Code's [license](https://github.com/pingdotgg/t3code/blob/5b70d81b0d77bdd5883a835173735d55992009af/LICENSE)
  is MIT.
- PandaCode's [license](https://github.com/aicodeking/pandacode/blob/25d3466e4b1cbc51108f57cfcdd73c6a18d01317/LICENSE)
  adds a public/commercial fork attribution condition to MIT-style terms. Treat
  it as a reference unless those terms have been assessed for the proposed reuse.
- Terax AI's [license](https://github.com/crynta/terax-ai/blob/640e8b2e6b9a9904af3d1b1161ad936864eb4a9a/LICENSE)
  is Apache-2.0.

No upstream app, model call or cross-interface live test was run. Sources were
read from shallow clones pinned above. T3's full Windows checkout encountered a
vendored-path limit; relevant files were inspected directly from Git objects.
No dependency, schema, configuration, policy or roadmap was changed, and no
Maitty verification gate was run for this research-only document.
