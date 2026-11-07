# Crews and crewmates: plan

Status: ACCEPTED, 2026-11-06. Filed in Plane as the module "Crews and crewmates" (MAITTY-204 to
MAITTY-222). Decision record: [ADR 0010](../../adr/0010-crews-are-daemon-owned-and-local-first.md) (Accepted).

## Words

- **Ship**: one session, one chat with one AI (already built).
- **Fleet**: a named group of ships on one machine (already planned).
- **Crewmate**: a custom agent with a role, bound to a session.
- **Captain**: the crewmate that leads a crew and may start and message the others.
- **Crew**: one shared chat that people and crewmates are all in. A crewmate speaks in it, is spoken
  to by name, and talks to other crewmates in it. Decided 2026-11-06. Locally the only person is you;
  teammates join in Phase B.

## What exists and is reused

- MCP tools (`maitty_send`, read, wait, start, cancel) with per-pane read/write/start grants.
- Trusted status from the owning runtime; ships and fleets in the daemon.
- Provider adapters for Claude and Codex, running the user's own logins.

## What is new

1. **Crewmate profile**: name, one-line "how others see me" (the routing hint other agents read),
   instructions, provider and model, grants. Free text for roles; no fixed role types.
2. **Crew record**: members, captain, settings. Saved by the daemon with the session snapshot.
3. **Mailbox**: persisted messages between crewmates (typed, 8,000-character cap, sender, task id,
   parent task, hop count, intent such as `ask` / `reply` / `fyi`). Hop limit 6; a human message resets it.
4. **Task tree**: a captain's delegation makes a child task; cancelling a parent cancels children, and
   a parent never waits on a cancelled child.
5. **Grants per crewmate**: levels none / view / chat / manage on each other crewmate, plus scoped
   start and send. Checked by the daemon on every call. Management powers only in human-started runs.
   Subagents and mailbox-started runs get no management tools.
6. **Audit log**: a read-only record of every message and grant change.
7. **Client surfaces**: crew list, create/edit crewmate, a crew view with the message log. TUI first
   (the desktop Crewmates mode follows the same daemon data).

## Safety rules (from the research)

- A message never changes pane status and is treated as untrusted text by the receiver.
- Cross-crew messaging is off by default.
- Sharing a crewmate means its profile only: never credentials, files, memory or history.
- No credential ever leaves a machine. Each person signs in to their own `claude` / `codex`.
- Idempotent, schema-validated tools so a retry cannot double-start a crewmate.

## Phases

**Phase A: local crews (first release)**
- A1 ADR 0010 accepted; reconcile `API_PATTERNS.md` and `FLAT-SESSIONS-PLAN.md` §8 wording.
- A2 Protocol: crew and crewmate types, commands, snapshot fields; `PROTOCOL_VERSION` bump; old-shape test.
- A3 Daemon: crew store (saved with sessions), crewmate sessions launched with profile instructions.
- A4 Grants: per-crewmate levels, re-check on each call, human-started-run rule.
- A5 Mailbox, hop limit, task tree, cancel cascade, audit log.
- A6 MCP tools: list crewmates, message, delegate, wait, report (crew-scoped).
- A7 Captain: a crew-aware brief; starting a crewmate asks the human first.
- A7b Crew chat: one shared chat per crew, @mention routing, replies attributed to a crewmate or person.
- A8 TUI: crew list, edit crewmate, crew log view.
- A9 Desktop: Crewmates mode on the same data.
- A10 Docs (API_PATTERNS, SECURITY, ARCHITECTURE, DATABASE), tests, Windows cross-checks.

**Phase B: remote crews (separate review, after A ships)**
- B1 Threat model and ADR: identity, invites, revocation, relay trust.
- B2 Transport interface; B3 outbound-connection relay; B4 member keys and signed member list;
  B5 one-time expiring invites; B6 remote messages as untrusted input; B7 revocation.
- Check OpenAI/Codex terms and the full Anthropic consumer terms before B1.

## Open questions for review

Decided 2026-11-06: a crew is one shared chat (people and crewmates together) and is not a fleet;
crewmates are persistent (with a reset option); the captain needs human approval before starting a
crewmate; the captain answers an unaddressed message; each crewmate keeps its own private working
session beside the shared chat.

1. Hop limit and message cap values (6 and 8,000 are borrowed from Rakazo).
2. Should a crewmate's usage count against a crew budget? (Provider-reported usage only, ADR 0009.)

## Tickets

Filed in Plane as the module "Crews and crewmates": Phase A is MAITTY-204 to MAITTY-214, Phase B is
MAITTY-215 to MAITTY-222 (Backlog). Order and "blocked by" are in [`tickets.md`](tickets.md); the
Plane API on our server has no relations endpoint, so the links are text in each ticket.
