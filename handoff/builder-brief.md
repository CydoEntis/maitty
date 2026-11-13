# Builder brief: Maitty daily-loop tickets

You implement ONE Plane ticket in ONE git worktree. The ticket text and the worktree path are
in the message that sent you here. The user is asleep and cannot answer questions.

## Rules

- Work only inside your worktree. It is already on its own branch, cut from `origin/dev`.
  Never touch `C:\dev\maitty` or any other worktree.
- Read `AGENTS.md` first, then the docs its documentation map names for the areas you touch.
  Read `docs/features/daily-loop/intent.md`. Follow them; they outrank this brief where they
  are stricter.
- The ticket's line numbers and file names were true when it was written. Check them against
  the code before relying on them.
- Test first where the ticket asks for a failing test: run it red before the fix, and do not
  edit it between red and green.
- Stay inside the ticket. No refactors, renames or clean-ups it does not ask for. Respect its
  Non-goals.
- Never weaken a type, lint rule or test to make a check pass. Never mark a test ignored.
- Update the doc that owns a contract you change (`docs/UI.md`, `docs/API_PATTERNS.md`,
  `docs/FEATURES.md`, ...), in the same commit. A protocol change follows AGENTS.md's
  `PROTOCOL_VERSION` rule, with a test that decodes the old shape.
- Do not start other agents. Do not use the Plane tracker. Do not push. Do not open a pull
  request.
- If the ticket cannot be done without a product decision that is not a small, obvious
  default, STOP: commit nothing half-done, and report `escalate` with the decision and its
  options. For a small default, pick the conservative option and name it in your report.
- If the ticket turns out to be already done on `dev`, or wrong about the code, say so with
  evidence and report `escalate`; do not invent work.

## Checks (run all, report each with its real result)

When `apps/gui` is touched, in `apps/gui`:
`pnpm install --frozen-lockfile`, then `pnpm test`, `pnpm typecheck`, `pnpm lint`.

Always, from the worktree root, in order:

```
cargo build            (a fresh worktree needs `pnpm bundle:daemon:dev` in apps/gui first)
cargo test
cargo clippy --all-targets -- -D warnings
bash scripts/check-gui-deps.sh
cargo check --workspace --exclude maitty-gui --target x86_64-unknown-linux-gnu
cargo check --workspace --exclude maitty-gui --target aarch64-apple-darwin
```

Known: `approved_commands_are_listed_and_taken_back_by_a_person_only` in `maitty-core` fails by
chance on Windows ("fixture directory survived cleanup"). If only that test fails, rerun
`cargo test` once and report both runs. Any other failure is yours to fix or report.

Other builders run cargo on this machine at the same time, so builds are slow. Run long
commands in the foreground with a long timeout and wait for them; do not hand back until the
checks have finished.

A "Seen on screen" criterion needs a person at the desktop app or TUI. For the TUI, a
`TestBackend` frame test counts as evidence; say that is what you used. For the desktop app,
report it as NOT DONE. Never report it as passed on the strength of unit tests.

## Commit

One commit for the ticket (two only if the ticket clearly holds two logical changes), staged
by file name, never `git add -A`. Message per `docs/COMMIT-STANDARDS.md`: use the ticket's
title as the subject where it fits, a short body saying why, no ticket ids, no AI or tool
attribution of any kind.

## Report (your final message)

Line 1: `pass`, `fail` or `escalate`. Then:
- what changed, by file, in plain language;
- each acceptance criterion: met (with the test or evidence) or not met (why);
- each check command and its real result;
- defaults you chose, docs you updated, anything a reviewer should look at first;
- the commit hash.

Also known: with several builds running on this machine, Rust tests that wait on a shell, git or a pane can time out ("timed out waiting for the pane's shell", "git took too long"). If a full `cargo test` fails only on timeouts or the known flake, do not loop on it: run each failing crate alone once (`cargo test -p <crate>`), report every run, and move on. At most two full `cargo test` runs.
