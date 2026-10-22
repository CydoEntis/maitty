# Intent: a team list for the commander

Type: feature
Author: Cody · Status: accepted
Date: 2026-10-22 · Plan: [phase 17](../../ROADMAP.md#17--team-roles-for-the-commander--queued-2026-10-22)

## Problem

A commander ([phase 9](../fleet/intent.md)) starts helpers, but it does not know which model should
do which job. Plan with Opus, build with Sonnet, review with Opus: you say it again every time, and
switching from Claude to Codex means saying all of it again with different model names.

## Proposed outcome

A **Team** list in Settings. Each row is a job (planner, builder, reviewer, or one the user adds) with
a name and one model for each agent family, Claude and Codex. One global setting says which family is
active, and a project may override it. When a commander starts a helper for a job, it starts it on
that job's model for the active family. Switching families is one choice, not a rewrite.

The user's starting list:

| Job | Claude | Codex |
| --- | --- | --- |
| planner | opus | gpt-6.1-sol |
| builder | sonnet | gpt-6-luna |
| reviewer | opus | gpt-6.1-sol |

The model names are the user's, passed to the agent unchanged. Seshi sets the model only, not the
reasoning effort (none was given).

## Affected users and systems

Anyone who runs a commander over several jobs. The config file and its loader, the commander's brief
(`src/brief.md` and where the daemon composes it), launching a helper with a model flag, Settings (a new
Team tab), and `config.example.toml`.

## Constraints

- A commander stays an ordinary agent session. Seshi runs no pipeline, starts no stage on its own and
  moves no work from one job to the next (ROADMAP › Out, narrowed 2026-10-22).
- Handoffs are gated by [phase 15](../checkpoints/intent.md) checks: the brief tells the commander to
  wait for the build's check to pass before it starts the reviewer. Seshi reports the result; it blocks
  nothing and does not enforce the order.
- The list lives in the user's config file. Old `[[profiles]]` and `[[workflows]]` tables stay ignored.
- With no list, a commander works as it does today.
- Model names are not checked. A name the agent does not know shows as that agent's own error.
- Normal sessions are unchanged. Nothing sits in front of a session you talk to directly.
- Every Settings screen works in all three layouts (Cards floating, Cards tiled, Flat) at 80×24, 100×30
  and 160×45, reusing the Settings › Check tab's layout. No new theme roles.
- The three jobs above ship as an example in `config.example.toml`, not as built-in defaults, since
  model names go stale.

## Out of scope

Seshi starting or ordering stages itself (the workflow runner stays cut), a lead in front of ordinary
sessions, automatic retries of a failed check, reasoning-effort settings, and checking that a model
exists.

## Open questions

None.

## Tickets

Drafted, not filed. Phase 17 is queued, and only the active phase gets tickets. In order:

1. Config: parse and validate the team list and the family setting.
2. Resolve a job to an agent and model flag for the active family, with the per-project override.
3. Put the team list into the commander's brief.
4. Add the brief rule that a handoff waits for a passing check.
5. Settings › Team: list, edit, add, remove, and the family switch, in all three layouts.
6. Docs, `config.example.toml` and README.
7. A live trial: a commander runs a small ticket set on the user's models.
