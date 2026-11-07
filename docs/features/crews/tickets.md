# Crews: tickets in Plane (MAITTY)

Filed 2026-11-06 as MAITTY-204 to MAITTY-222, module
"Crews and crewmates". "Blocked by" links are real; Plane holds them as text.

## Phase A: local crews

| Key | Title | Labels | Blocked by |
| --- | --- | --- | --- |
| A1 | Accept ADR 0010; update API_PATTERNS role-profile wording and FLAT-SESSIONS-PLAN §8 | adr, docs | none |
| A2 | Protocol: crew, crewmate, crew-chat message types and commands; bump PROTOCOL_VERSION; old-shape decode test | daemon, feature | A1 |
| A3 | Daemon crew store saved with sessions; crewmate profile; ongoing crewmate sessions started with profile instructions | daemon, feature | A2 |
| A4 | Per-crewmate grants (none/view/chat/manage), re-checked per call; management only in human-started runs | daemon, feature | A3 |
| A5 | Mailbox: persisted messages, 8,000-char cap, hop limit 6, task tree with cancel cascade, read-only audit log | daemon, feature | A3 |
| A6 | Crew chat: one shared chat per crew, @mention routing, attributed replies, messages never change pane status | daemon, feature | A5 |
| A7 | MCP tools: list crewmates, message, delegate, wait, report (crew-scoped, idempotent) | daemon, feature | A4, A5 |
| A8 | Captain: crew-aware brief; human approval before starting a crewmate | daemon, feature | A4, A7 |
| A9 | TUI: crew list, create/edit crewmate, crew chat, message log | client, feature | A6, A8 |
| A10 | Desktop: Crewmates mode on the same daemon data | client, feature | A9 |
| A11 | Docs (API_PATTERNS, SECURITY, ARCHITECTURE, DATABASE), Windows/macOS cross-checks, verification gate | docs, windows | A9 |

## Phase B: remote crews (Backlog only, review after A ships)

| Key | Title | Labels | Blocked by |
| --- | --- | --- | --- |
| B0 | Check OpenAI/Codex terms and Anthropic consumer terms for teammate use | spike, docs | none |
| B1 | Threat model + ADR: identity, invites, revocation, relay trust | adr, spike | A11, B0 |
| B2 | Transport interface for crew traffic (relay first, direct later) | daemon, feature | B1 |
| B3 | Outbound-connection relay service | tooling, feature | B2 |
| B4 | Member keypairs and owner-signed member list | daemon, feature | B1 |
| B5 | One-time expiring invite links | daemon, feature | B4 |
| B6 | Remote messages as untrusted input; cleaning; ignored-path tests | daemon, feature | B2 |
| B7 | Revocation and removing a member | daemon, feature | B4 |
