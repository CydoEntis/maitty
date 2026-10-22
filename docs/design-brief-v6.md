# Seshi — design brief v6: checkpoints Seshi checks itself

Draft 2026-10-22, for the designer to work on alongside the first build. This brief **narrows**
[v5](design-brief-v5.md). The lead, the team view and personas from v5 are not part of it:
saved role profiles were cut (ROADMAP › Out), and the team view stays deferred. Same format as
before: an interactive cell-grid HTML prototype at 160×45 (plus how it degrades at 100×30 and
grows at 240×60), with the state machine as the spec.

## What Seshi is (one line)

A terminal app where one person runs many AI coding agents side by side, each in its own git
worktree, and only looks up when one needs them.

## The problem

An agent saying "done" turns its session done, even when the project's tests fail. You find out
later, or not at all. Seshi's one job is never to misreport an agent.

## The idea

A **checkpoint** is something Seshi checks itself, with evidence it observed. The agent cannot
tick it off by saying so. A session that has checkpoints is not shown as done until they pass.
Checkpoints never stop an agent working; they only change what Seshi will say about it.

Checkpoints are **opt-in per project**. With none set up, nothing here appears.

## What is settled, and what is open

**Settled (design these fully):**

- The first checkpoint is **verify**: a project names one check command. When an agent's turn
  ends, Seshi runs it in the session's worktree and records the result. Pass: the session reads
  done. Fail: it reads `failed: <check>`, shows in the Inbox, and stays visible.
- You can **override** a failed check. The override is recorded and visible.
- The result carries its evidence: the command, exit code, when it ran, how long it took, and the
  tail of its output.

**Open (design the shape, expect it to change; do not build screens around the details):**

- A list of several checkpoints per project, in an order the user chooses and names. Later kinds
  Seshi could check itself: **commits** (the branch has new commits), **approval** (you approve a
  plan), **review** (a fresh session reads the diff). Only verify is committed to; the rest wait on
  using verify in real work. Leave room in the layout for more than one, without drawing them.

## What already exists (design on top of it)

- The sidebar lists sessions under their folder, with state glyph and colour (working, needs
  you, done, idle, asleep `zzZ`) and the question under a session that needs an answer.
- The docked sheet on the right (72 columns; the whole pane column on narrow screens) holds the
  Inbox. `i` says why a session has the state it has; `m` sends a follow-up.
- The status line holds path, branch and PR (design v5 cleanup, ROADMAP phase 14).
- Cards and Flat are two drawing styles of one layout (ADR 0012). Every theme must keep working.

## What to design

For each: where it lives, what's on it, its states (empty, loading, error, many items), keyboard
and mouse, and how it looks at 100×30.

### 1. A session with a check

- **Where the result shows on a row.** Today a finished session is done. Show these without
  losing the state colour or the folder name: *checking* (running now), *passed*, *failed*,
  *failed, overridden*, *could not run* (command missing, timed out, cancelled).
- **Failed is not "needs you".** It should read differently from a question, but still be hard to
  miss. Say how.
- **Checking is not working.** The agent has stopped; Seshi is busy. Say how that differs from
  the spinner.

### 2. Evidence

- What you see when you open a result: command, exit code, time, duration, last lines of output.
- Long output, empty output, and a command that printed nothing.
- Where it opens: the `i` "why" popover, the sheet, or a view of its own.
- A result that is stale: the agent changed files after the check ran.

### 3. The Inbox

- How a failed check reads in the Inbox next to questions and finished work.
- Actions on it: open the session, read the output, **override**, re-run. Say what override asks
  you to confirm, since it hides a failure.

### 4. Turning it on

- A project's check command in settings. Empty state (nothing set, with one line saying what it
  does), a command that is set, a command that has never run, and a command that does not exist.
- Because a command from a cloned repo must never run without your approval, show the approval
  moment: what is about to run, in which folder.

### 5. Room for more

Without designing them, show that the row and the Inbox still work when a project has a list of
three or four checkpoints (for example *build ✓ · test ✓ · review …*), including how it narrows
at 100×30.

## Things we already know

- Seshi runs the command; the agent never reports a pass. A result is trusted because Seshi
  observed it.
- A project with no checks looks exactly as it does today.
- Nothing here may hide an agent that needs you. The Inbox stays the one place that lists them.
- Model, context and cost stay out of Seshi's own chrome (decided 2026-10-19).

## Constraints

- A terminal grid: one character per cell, 24-bit colour, bold/italic/underline/dim only. Nerd
  Font glyphs are used when the font has them, with plain fallbacks. Pass and fail must not
  depend on colour alone.
- Cards draw with flush edges or rounded corners, a setting; design for both, and for Flat.
- Everything works by keyboard (leader `Ctrl+Space` + a key; bare keys when the sidebar has
  focus; Alt+arrows move between panes) **and** by mouse (every chip clickable, hover states).
- Free keys: ADR 0009 (`docs/adr/0009-leader-keys-after-the-cut.md`) lists what's taken.
- Motion is short (90–140 ms) and can be turned off; nothing may depend on it.
- Windows Terminal, Ghostty, Alacritty, kitty, iTerm2. Windows, macOS and Linux.
- Every theme keeps working, and the contrast audit keeps passing.
