# 0010. Crews are daemon-owned, local first, and trust nothing that arrives from outside

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

Maitty runs one chat per ship (one session with one AI). We want custom agents with a role
(a researcher, a marketing lead), a captain that delegates to them, messages between crewmates, per-agent
privileges, and later teammates who join a crew with agents running on their own machines.

Research (OpenBot, Rakazo, 2026-11-06) shows the shape is proven: persistent role-based agents, a
lead, a mailbox, access levels. Neither product lets a teammate bring an agent that runs on the
teammate's own machine, and both have trust gaps (full-access agents; every member can edit every
agent; messages with no delivery state). Anthropic's terms allow each person to sign in to the
unmodified `claude` with their own plan, and forbid routing requests through someone else's plan
credentials. OpenAI's terms for Codex are unverified.

Existing rules that apply: the daemon owns every session (0001), status comes only from the owning
runtime (0003), provider adapters never widen permissions (0005), secrets never enter chat (0006),
one connection per machine (0007). `docs/API_PATTERNS.md` currently says saved role profiles and
fleet commanders are not part of the interface; this decision reverses that.

## Decision

1. **A crew lives in the daemon.** A crew is one shared chat with a set of members (people and crewmates). It is not a fleet and does not
   belong to one. The captain answers a message that names no one. A member is a crewmate: a saved
   profile (name, one-line "how others see me", instructions, provider and model, grants) bound to
   an ongoing session that is kept between tasks (with a reset option). One member is the captain. The daemon stores crews and decides every grant; clients only
   display and edit them.
2. **Local first.** The first release has no networking beyond the existing local socket. Crews work
   on one machine for one person.
3. **Messages are mailbox entries, not keystrokes.** A crewmate message is a typed, size-capped,
   persisted entry with a sender, a task id and a parent task, delivered by the daemon. It carries a hop
   count with a hard limit. Every exchange is logged read-only. A message is data to the receiver and
   never changes any pane's status (0003).
4. **Grants are explicit and re-checked on every call**, never inherited from the captain. Management
   powers (starting, editing or removing crewmates) work only in a run a human started, and the
   captain needs a human's approval before it starts a crewmate.
5. **Each person's agents run on their own machine under their own login.** No credential is ever
   sent between machines or stored by a relay.
6. **Remote crews are a later decision**, built behind a transport interface (an outbound-connection
   relay first; direct links possible later). Remote messages are untrusted input and need their own
   threat model before any code.

## Alternatives considered

- **Build the remote relay first**: the hardest part, with the least proven value; blocks the useful
  local feature.
- **Copy OpenBot's design directly**: its license (PolyForm Noncommercial) forbids copying code, and
  its default of full access with no approvals conflicts with 0005.
- **A server-hosted crew (Rakazo style)**: agents would not run on each person's machine, so it
  cannot use their own CLI logins.
- **Let crewmates talk by typing into each other's terminals**: no delivery state, no limits, no log,
  and it bypasses grants.

## Consequences

Easier: a captain and crewmates on one machine using the existing MCP tools and grants. Harder: a
protocol change (bump `PROTOCOL_VERSION`), a new saved-state file with a migration path, and more
tests for grant checks. We must maintain a hop limit, a mailbox and an audit log. Revisit when remote
crews start, or if a provider's terms change.
