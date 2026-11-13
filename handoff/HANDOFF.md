# Handoff: Daily loop run (2026-11-12 22:15 to 2026-11-13 afternoon)

This note is for the next session that picks up the Daily loop tickets on another machine.
It is not part of the repo's docs. Delete this branch once it has been read.

## How the run worked (repeat this)

For each ticket:
1. `plan-pull brief MAITTY-N`, then `plan-pull start MAITTY-N`.
2. `git worktree add <dir>/dl-N -b <type>/<slug> origin/dev`. Branch from `origin/dev`, not `main`.
3. A builder agent implements the ticket test-first, runs the gate and commits once, without pushing.
   Its brief is `builder-brief.md` next to this file.
4. Push, then `gh pr create --base dev`.
5. A blind reviewer (diff only: `git diff origin/dev...HEAD`) reviews it. The findings go back to the same
   builder, then the same reviewer takes a second look.
6. Merge `origin/dev` into the branch, check, push. Wait for CI, then `gh pr merge N --merge`.
7. `plan-pull done MAITTY-N --gate "..."` only when every criterion is met. If a desktop
   "Seen on screen" criterion is open, leave the ticket In Progress and post a comment.
8. Remove the worktree folder after the merge: the disk filled up once.

Known noise: `approved_commands_are_listed_and_taken_back_by_a_person_only` is flaky on Windows.
Rust tests that wait on a shell or git can time out under load. The cross-checks need
`cargo check --workspace --exclude maitty-gui --target ...` on Windows.

## Protocol

`PROTOCOL_VERSION` on dev is 92 (310 merged). The next protocol change takes 93.

## Open pull requests left by this run

See the "State at handoff" section at the end; it is filled in at the last minute.

## Waiting on Cody (nothing to build until he answers)

- About 24 merged tickets are In Progress only for the desktop "Seen on screen" check. Each ticket's
  comment says what to look at.
- MAITTY-278 (Codex Approve/Decline): Codex keys can be rebound. Needs someone to capture both prompt
  screens and `/keymap debug`, and two decisions: what to do about rebound keys, and whether
  the daemon may read the screen (ADR 0003).
- MAITTY-307 (design spike: ask a side question about a session). Cody had no view yet; it is
  close to the `/btw` bug below, so settle the bug first and bring 307 back to him with that.

## Decided 2026-11-13, now free to build

- MAITTY-282, PR #317: option (c). First make an unreadable history cache non-fatal (an older
  daemon that meets an entry kind it can't decode skips or rebuilds the cache and logs it,
  instead of failing to start), then merge #317. Then 283, 284; 317 and 318 after 283.
- MAITTY-296: option A. Persist the main checkout and branch on `Conversation`; on send, recreate
  a missing worktree with `reopen.rs`'s recreate logic.
- MAITTY-298 to 300 (Codex app-server `turn/steer`, then Send now for structured chats): no key
  guessing involved. First check that the installed Codex app-server really has `turn/steer`.

## Free to build next

- MAITTY-311 (snooze UI), once 310 (#359) has merged.
- MAITTY-314 and MAITTY-315 (guide docs), last.
- Bug, no ticket needed: `/btw` in the desktop chat is queued while the agent works, instead of
  being typed into its terminal. In `apps/gui/src/features/chat/useChatComposer.ts` (about lines
  204 to 212), `holding` sends every message to `paneQueue.add`. A slash command meant to run
  mid-turn (`/btw`, listed in `terminalCommands.ts`) should be typed straight in, as
  `sendQueuedNow` does. Check how Codex's `/side` behaves too (`codexCommands.ts`).

## Follow-ups to propose (not filed)

- ArchivePane lets an agent pane put away other panes; SnoozePane now refuses that.
- Sidebar answer keys differ from the chat's: the sidebar sends bare `n`/`a` to Claude's menu,
  which ignores them while showing "Sent n" (bug). The Codex sidebar keys are unverified.
- A put-away stopped chat doesn't protect its worktree; it needs a persisted closed-for-good marker.
- Refuse a restore while a conversation is Blocked mid-turn, not only while it is Working.
- Palette output search: if the search call never answers, search stays off until the palette
  reopens. The daemon copies every pane's whole output on its main loop per search; search the
  recent tail off the loop, and strip DCS/APC/PM payloads in `plain_lines`.
- The session list is about 352 KB for 50 sessions. The desktop worktree manager opens one
  connection per row. Recent sessions look at each folder once per open.
- Flaky tests: the pid-keyed temp folder in machines_tests; `image_paste` tests collide when runs overlap.
- AGENTS.md exception: an enum with a `#[serde(other)]` variant needs no version bump.
- Stale doc notes: FEATURES.md:48, UI.md:248.

## Kitchen

Write-up: https://claude.ai/artifact/2siA5AVp7RZUfyuwMC4rwa. Recommendation: keep with changes,
then retry it on three tickets.

## State at handoff

- Merged today after the summary above: 297 (#357), 306 (#356), 309 (#358, Done), 310 (#359, Done).
- PR #360 (MAITTY-312, put away a session whose PR merged): built, gate passed locally, pushed.
  Not reviewed yet: run the blind review, then merge. It is In Progress in Plane.
- MAITTY-322 (desktop app sends operation ids): the builder was stopped before it committed.
  Its unfinished edits are in `322-unfinished.patch` beside this note (apply onto `origin/dev`
  with `git apply`). The brief it worked to: use a random nonzero nonce rather than
  `nextOperation`, keep u64 values safe in JS, and send `Some` only to daemons at protocol 91+.
  Its checks had not run. Review the patch before trusting it, or redo the ticket.
  It is In Progress in Plane.
- The daemon search, `ArchivePane` and the other follow-ups above are not filed.
