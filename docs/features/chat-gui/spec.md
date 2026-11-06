# Spec: standalone chat GUI with optional native terminal continuity

Intent: [intent.md](intent.md) · Status: accepted · Date: 2026-10-27
Evidence baseline: dev at c08354ce6afc7888774f309e9d7c4873ed721abd.

## Concerns

- Ownership: an idle native CLI can still write. End its runtime before transfer;
  cancellation alone does not release ownership.
- Trust: accept structured status only from a daemon-owned provider connection
  bound to the current conversation generation. Retain authenticated pane reports.
- History: provider-native session context and Maitty display history are distinct.
  Reconcile terminal-authored messages before enabling structured input.
- Permissions: preserve the effective permission/sandbox posture. Refuse a handoff
  when a provider cannot represent it; never broaden permissions to make it work.
- Privacy: extend current private atomic-file storage; no new database. With
  restore.output=false, remove/prevent Maitty chat caches. Native provider history
  is not deleted; it may hydrate memory for this attachment but is never recached.
- Compatibility: new IPC variants require a protocol bump under ADR 0002.
  Record verified Codex versions/capabilities before enabling the adapter.
- Scope: [current TUI design](../../DESIGN.md) remains authoritative for TUI controls.
  The GUI framework is selected in [ADR 0004](../../adr/0004-client-split-and-desktop-host.md).
  Claude uses the installed CLI's streaming protocol per
  ADR 0039; supported CLI versions,
  permission-control compatibility and authentication still require verification.

## Requirements

- R1. Native TUI workflows, ordinary panes, existing providers, checks and SSH still
  behave as before; existing tests plus isolated nested-server exercise.
- R2. A stable Maitty conversation identifies provider, exact native session,
  account/profile/config root, binary and working directory/worktree. Ambiguity
  refuses transfer; no most-recent fallback or implicit fork. Identity tests.
- R3. Structured processes belong to the daemon, survive client disconnect and
  produce ordered typed message/tool/status events. Fake-provider lifecycle tests.
- R4. Turn sends, approval answers and cancellation carry conversation generation;
  approval answers also identify the exact live request. Stale commands are
  rejected. Isolation and delayed-command tests.
- R5. At most one runtime writes a conversation. Busy or blocked handoff refuses
  visibly; the user finishes, resolves or cancels and retries. The old runtime
  must exit before the target starts. Failed stop leaves ownership unchanged.
  State-machine and surviving-child tests.
- R6. Terminal turns are reconciled before structured writes resume. Stable native
  identities suppress duplicates; missing, truncated or unsupported history is
  visibly incomplete and prevents takeover. Replay/pagination/failure tests.
- R7. Transfers and restart recovery preserve identity and never launch competing
  runtimes. Explicit recover/retry checks exact identity and confirmed old-runtime
  exit before resuming ownership; it never resends an uncertain turn. A persisted transfer is recovered as interrupted and requires explicit
  recovery; obsolete requests are invalidated. Fault-injection restore tests.
- R8. History remains local/private, bounded when paged and follows restore.output.
  No credentials or conversation text in diagnostic fixtures/logs. Storage tests.
- R9. The daemon owns structured Codex execution and retains exact conversation
  identity, ordered history, permission posture and recovery state for the GUI.
  Deterministic provider and persistence coverage is required.
- R10. Phase 34 provides a usable local Codex GUI: sessions, composer, streaming,
  tool activity, approvals, cancellation, errors/reconnect, diff access and
  close/reopen while daemon work continues. Validate normal new-conversation and
  chat flows against live Codex on Windows and Linux.
- R11. Phase 35 produces and exercises an approved GUI design against the working
  app, including keyboard, accessibility and empty/loading/failure states.
- R12. Before initial GUI release, Claude satisfies the standalone GUI contract,
  both providers pass live chat, recovery and permission checks on
  Windows/macOS/Linux, and optional GUI packaging leaves the TUI-only install
  usable without desktop/frontend tooling. GUI-to-TUI continuity is later work.

## Design

Keep provider-native history as executable agent context. Maitty holds a private
display projection and stable conversation binding, not a replacement agent loop.

Proposed feature locations: pure identity/state types in crates/maitty-shared/src/conversation.rs;
daemon runtime, provider adapter, history and transfer modules in
crates/maitty-core/src/daemon/conversation/; IPC in crates/maitty-shared/src/protocol.rs; CLI controls beside crates/maitty-terminal-client/src/cli.rs.
Tests live beside source. Names are planned locations, not existing modules.

Use existing serde/serde_json, tokio, local IPC, private-file helpers and
proc/platform launching conventions. Add an explicit provider protocol stack doc
before writing adapter code; use installed Codex app-server over stdio.
Do not require a Node bridge for the Codex foundation.

The wire contract carries conversation identity, runtime generation, request ID,
ordered history cursor and typed content/status. Mutations are validated at the
daemon boundary. Exact fields are owned by code and API_PATTERNS when implemented.

Structured status enters existing daemon arbitration/checkpoint behavior; an agent
claim never replaces a checkpoint result. Keep references available through existing
MCP rules, with no automatic attachment or prompt injection.

## Verification and coverage

Every implementation ticket runs the repository gate in order:
cargo build; cargo test; cargo clippy --all-targets -- -D warnings;
cargo check --target x86_64-unknown-linux-gnu;
cargo check --target aarch64-apple-darwin.
Report unavailable targets honestly; a compile check is not a platform live check.

Use isolated MAITTY_SOCKET servers and temporary repos/config roots. Never restart
the user's daemon or use private transcripts as fixtures. Paid/live provider runs
are explicit human acceptance work, separate from deterministic automated tests.

Phase 34 ticket coverage: R1 → [#206](https://github.com/CydoEntis/maitty/issues/206),[#207](https://github.com/CydoEntis/maitty/issues/207); R2 → [#198](https://github.com/CydoEntis/maitty/issues/198),[#204](https://github.com/CydoEntis/maitty/issues/204),[#205](https://github.com/CydoEntis/maitty/issues/205);
R3 → [#199](https://github.com/CydoEntis/maitty/issues/199),[#200](https://github.com/CydoEntis/maitty/issues/200),[#201](https://github.com/CydoEntis/maitty/issues/201); R4 → [#201](https://github.com/CydoEntis/maitty/issues/201),[#202](https://github.com/CydoEntis/maitty/issues/202),[#203](https://github.com/CydoEntis/maitty/issues/203); R5 → [#204](https://github.com/CydoEntis/maitty/issues/204),[#205](https://github.com/CydoEntis/maitty/issues/205); R6 → [#200](https://github.com/CydoEntis/maitty/issues/200),[#205](https://github.com/CydoEntis/maitty/issues/205);
R7 → [#198](https://github.com/CydoEntis/maitty/issues/198),[#204](https://github.com/CydoEntis/maitty/issues/204),[#205](https://github.com/CydoEntis/maitty/issues/205),[#206](https://github.com/CydoEntis/maitty/issues/206),[#207](https://github.com/CydoEntis/maitty/issues/207); R8 → [#198](https://github.com/CydoEntis/maitty/issues/198),[#200](https://github.com/CydoEntis/maitty/issues/200); R9 → [#198](https://github.com/CydoEntis/maitty/issues/198),[#199](https://github.com/CydoEntis/maitty/issues/199),[#200](https://github.com/CydoEntis/maitty/issues/200),[#201](https://github.com/CydoEntis/maitty/issues/201).
[#197](https://github.com/CydoEntis/maitty/issues/197) establishes capability evidence supporting R2–R7.
R10–R12 are later-phase outcomes; ticket coverage is required on phase activation.
