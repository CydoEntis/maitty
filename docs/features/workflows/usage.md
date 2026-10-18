# Using workflows

Click **Actions → Workflows**, open **Workflows** in the command palette
(Ctrl+Space, then Space), or open
**Agent profiles** and select **Workflows**. **Runs** shows progress and reports.
The sheet slides in from the right and obeys Settings → Motion.

Click a section tab, or use Tab/Shift+Tab or Left/Right from a list. Ctrl+Tab and
Ctrl+Shift+Tab switch sections from editors and reports too. Each section retains
its selection, unsaved draft and report position while the drawer stays open.
Enter or double-click opens a profile/workflow for editing, or a run's details.
Launching is separate: **Launch** (`l`) for a profile, **Run** (`s`) for a workflow.
Esc backs out of an editor or detail view before it closes the drawer.

Stage and round labels appear beside associated sessions. Pending plans also
appear in **Inbox**; **Review plan** opens the exact run in **Runs** for approval.

## First setup

Choose **Claude starter** or **Codex starter** to save a `feature` workflow and
any missing Planner, Worker and Reviewer profiles. Existing profiles stay intact.
Choose **Edit** to select your own profile for each stage, or **New** to create a
different workflow. Tab moves fields, arrows select profiles/scope, Ctrl+S saves.
Click a role field or press Enter/Space on it to open the profile picker. Choose
with arrows and Enter, or click the profile. Name and task fields support cursor
movement, Home/End, Delete and Backspace; pastes insert at the cursor. Profile
instructions use a multiline editor. Saving failures keep the draft visible.
Choose **Setup** (`u`) to preview selected roles and repository instructions.
**Copy instructions** (`c`) copies the reviewed snippet for `AGENTS.md` or
`CLAUDE.md`; no instruction file is changed automatically. Page Up/Down scrolls
the preview and Esc returns to the list.
New definitions can be personal or project-specific. A project definition replaces
a personal definition with the same name. The equivalent CLI setup is:

```sh
seshi workflow init --agent codex
seshi workflow list
seshi workflow show feature
```

Add `--project` to `init` to save in `.seshi.toml`. No skills, models or native
agent CLIs are installed. Roles keep native permissions; instructions not to edit
are behavioral, not a separate read-only sandbox.

## Let normal prompts select it

Add this instruction to your project's `AGENTS.md` or `CLAUDE.md`:

> For implementation tasks, use Seshi's `feature` workflow. Unless already
> executing an assigned workflow stage, run `seshi workflow show feature` and
> start it with a self-contained task using `seshi workflow run feature -- "<task>"`.
> Monitor the returned id with `seshi workflow status <id>` and tell me when the
> plan needs approval. Do not approve it yourself or start duplicate runs while
> waiting. If assigned a stage, follow that stage instead of starting a workflow.

Then just prompt **Build password reset.** You need not repeat the workflow name.
The session must be able to run Seshi or use its MCP connection. Tell an existing
session to read newly edited repo instructions once; changing the file does not
guarantee that a running turn reloads it.

MCP equivalents: `seshi_workflow_list`, `seshi_workflow_show`,
`seshi_workflow_start`, `seshi_workflow_status`. The saved workflow names the exact
profiles, including your own custom roles; the coordinator does not guess them.

## Execution and handoffs

1. Seshi validates and freezes all profiles, then creates one task worktree from
   the selected checkout's committed HEAD. Uncommitted source changes are not
   copied. An approved `on_create` hook finishes before agents start.
2. The planner submits a report and finishes its turn. Read the plan in **Runs**
   and choose **Approve plan** before building starts.
3. The worker receives the goal and plan, implements, verifies, and reports.
4. The reviewer checks the actual checkout and evidence. A passing review ends
   the run. Findings return to the worker; after three build/review rounds,
   unresolved findings end the run as failed for you to inspect.

In Runs, Enter opens the selected run. Goal, Plan, Build and Review buttons
show complete reports; Tab/Left/Right or 1–4 changes the report. Page keys and
the wheel scroll; Home/End reaches the bounds. Opening and reading never approve
a plan. **Open pane** (`o`) is a separate action. Controls stay tied to the open
run even if newer runs arrive; Esc returns to the list.

Stages are fresh native sessions in the same task checkout. They read `goal.md`,
`plan.md`, `worker.md` and `review.md` from their handoff directory. Profiles may
mix Claude and Codex. The stage must submit a nonempty UTF-8 Markdown report with
`seshi workflow finish <id> --report <file> --verdict complete`, then finish its
turn. Review uses `pass` or `changes`. Goals allow 4 KiB and reports 64 KiB.
Only the assigned pane may report. A prompt, quiet screen or missing report never
advances the run. Questions stay in the stage pane for the person to answer.

## Recovery and controls

Closing the client keeps runs alive. A daemon restart marks in-flight stages
interrupted and keeps reports; it never replays a task. Close the previous stage
pane, then choose **Retry stage** to start a fresh session for that stage against
existing files. Setup failures without a checkout require a corrected new run.

**Cancel run** stops sequencing and keeps panes/files; interrupt a working stage
in its pane if you also want the process stopped. Workflows do not commit, merge,
push or delete checkouts. Retained runs protect their checkouts from automatic
cleanup. History keeps up to 32 runs; new starts evict the oldest ended record
while keeping handoff files. At most 32 runs may remain unfinished.

Human CLI controls are `seshi workflow approve|retry|cancel <id>`, run outside
Seshi's agent panes. Agents cannot use them to bypass approval. Setup is local;
run Seshi and the commands on the same host.
Use `seshi workflow status [<id>] --json` for structured run data in scripts.
