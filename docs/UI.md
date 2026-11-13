# UI

This doc covers how the user interface is structured and built.
`AGENTS.md` is the operating contract; this doc holds the UI practices in depth.

**Delete this doc if the project has no user interface,** and remove its row
from `AGENTS.md`. Game menus and HUDs count as a user interface.

## Component structure

```text
components/ui/           primitives: button, input, dialog (design-system level, no domain knowledge)
components/              shared composites used by 2+ features
features/<f>/components/ feature components that know the domain
app/ or routes/          pages/screens: compose features, own layout and routing
```

- Primitives know nothing about the domain. Feature components know nothing
  about routing.
- Keep components small and focused. When one component fetches, transforms,
  and renders, move the fetching and transforming into a hook or service and
  keep the component for rendering.
- Pass data down and events up. Avoid prop drilling deeper than about three
  levels. Composition (`children`, slots) usually solves it before context does.
- Keep business rules out of components. They call feature hooks and services.

## State

| Kind | Examples | Where it lives |
| --- | --- | --- |
| Server state | records from the API or database | a query and cache layer, not a global store |
| URL state | filters, tabs, selected id, pagination | the URL, so it survives reload and can be shared |
| Form state | field values, touched, errors | the form library |
| Local UI state | open/closed, hover, draft text | component state |
| Shared UI state | theme, sidebar, current modal | a small store, only when several distant components need it |

- Derive values during render. Don't copy them into state and sync them with
  effects.
- Put state in the lowest component that needs it.

## Async states

Every view that loads data deliberately renders each state that applies:

- **Loading:** a skeleton or spinner that holds the layout, with no content
  jumping when data arrives.
- **Empty:** says what would appear here and, where there is one, the action
  that creates it.
- **Error:** says what failed in plain language and offers a retry. It never
  shows a raw error message.
- **Success.**
- **Partial or stale**, where it applies: refetching and optimistic states.

A mutation shows that it is pending, blocks double submission, and reports the
outcome.

## Forms

- Validate with the same schema the API uses, where possible.
- Show errors next to the field they belong to, after the user interacts with
  it. Show a form-level error for server failures.
- Disable submit while the form is submitting. Keep the user's input when a
  submission fails.

## Accessibility checklist

- The feature works with the keyboard alone, and focus order follows visual
  order.
- Focus is always visible. Dialogs trap focus and return it to the trigger when
  they close.
- Use semantic elements: a `button` for actions and an `a` for navigation. A
  clickable `div` is a defect.
- Every control has an accessible name. Every input has a label.
- Color is never the only signal, and text contrast meets WCAG AA.
- Status and errors are announced without stealing focus.
- Honour `prefers-reduced-motion`.
- The layout works at 200% zoom and at the smallest supported width.

## Styling

- Use design tokens (color, spacing, radius, type scale) rather than hard-coded
  values. A hex color or pixel value in a component is a magic value.
- Build mobile-first, with the project's breakpoints.
- Support light and dark themes through tokens, if the project supports both.

## Performance

- Split code by route. Lazy-load heavy, rarely used views.
- Virtualize long lists.
- Memoize only when there is a measured problem or a clear structural reason.
- Give images explicit dimensions and use modern formats.

## Project-specific: every-surface check

A UI change is not done until it has been seen working on each place it shows. Tests
passing is not that. Before the handoff, go through this list on a running build of the
worktree (how to run one is in `AGENTS.md`, *verification gate*):

- **Every entry point.** Each way in: the key, the menu row, the click, the command
  search, the right-click menu, the CLI.
- **Both clients.** The TUI and the desktop app, where the feature exists in both.
- **Each agent.** Every agent it applies to (Claude Code, Codex, Gemini, OpenCode, a
  plain shell), not only the one it was built with.
- **The way back.** Each state it can reach has a way out, and the way out was used:
  Esc, close, cancel, undo, an error, an empty list.
- **This computer and another machine.** A local session and one on another machine,
  or the refusal a local-only feature shows there.

Leave out a line that does not apply, and say why. The handoff then carries the line
`Seen on screen: yes/no, where`: `yes` names the surfaces looked at, by a person or
through a rendered frame; `no` names what was not looked at and why. A surface nobody
looked at is reported as not seen, never as passing.

## Project-specific: design system

The optional desktop GUI is a separate React application in `apps/gui/`. Its
components, state and styling follow the TypeScript, React and shadcn/Tailwind
stack docs. It shares session and conversation contracts with the TUI, while
its visual design is owned by the GUI roadmap phases.

A chat (a conversation the desktop app runs) is a row in the TUI sidebar like any
session. Its name and whether it is closed are the daemon's, shared with the desktop
app: **Rename** (the rename key, or Rename in its menu) names it for both, and
**Close** puts it away in both while it keeps running (closing a chat's terminal does
the same). A chat has no terminal, so Enter or → on its row says it opens in the desktop
app instead of showing an error; a terminal session opens in both apps. A session is a
chat or a terminal session from the moment it starts,
and nothing switches a live session between them. The desktop app shows terminal sessions
only: a new session there is a plain shell, closing a pane or tab detaches, and ending a
session is its own action (its look is in [DESIGN](DESIGN.md#desktop-app)).

The desktop app also shows other machines' terminal sessions (ADR 0007).
Its host holds one supervised connection per machine and the web view only renders: it knows a machine by
name, never by where it points. A session of another machine has the id `build/t12` (the machine's name, so a
saved layout survives a restart; this computer's stay `t12`). **Settings › Machines** retries, removes and
adds machines (the behaviour is the terminal app's [Machines dialog](#project-specific-machines-dialog),
with the host list read off the UI thread, the look at the machine ended by a 12 s limit or Cancel, and a
login that needs a person run in a session named `ssh <name>`); **New session** (⌘T, Ctrl+Shift+T, ⌘K, the sidebar's New agent chat, a machine's New agent chat here) opens, on this computer, the
**new agent chat page** (`NewChatPage`, layout 2f) over the panes, under the heading "What should we work on?" (the same for every agent, with no logo). It asks for a folder first: a search field over the machine's projects ("Recent projects", "Matching projects" while typing, "Open folder" for a typed path that is not in the list, and "No projects match. Paste a full path, or use Browse…"), with **Browse…** for the system's folder picker. No folder is chosen for the person: the message box cannot send until one is. Once a folder is picked, three buttons in one row replace the list: the **project** (a dropdown with a search, RECENT PROJECTS (the first three, the focused project first) and ALL ON THIS COMPUTER, then **Browse for a folder…**, a row for each other machine and **Add a machine…**), the **branch** (hidden outside a git repository; the folder's worktrees and branches with the age of their last commit (`git_branches`), and a typed name creates one) and **New worktree | This folder**, each with a line saying what it means. Choosing New worktree is a choice by itself: it takes a new branch under a suggested two-word name (`model/worktreeName.ts`) at once, which the branch button then shows, and a typed name or a branch with no worktree yet can replace it; This folder drops it. Under the buttons are three suggested tasks as plain rows; a click starts the chat with that task. The agent, its model and its effort are on the model button in the tray under the message box (see *Agent, model and effort*), where the agent switch shows because the chat has not started; an agent that is not installed says so under the box, with **set it up** opening the agent's official page (`open_agent_docs`, a fixed list, never a caller's address). The permission mode the chat starts in is beside it (`PermissionMenu`): Claude's modes, or for Codex **Read only**,
**Auto** (nothing is sent, so Codex's own settings stand) and **Full access** in red, which go on the launch line as
`--sandbox` and `--ask-for-approval`. While Yolo mode is on for Codex the page says Full access and offers no menu, since
that setting already starts it so. A running Codex chat has the same three modes on its permission button: a pick types
Codex's own `/permissions` command with a built-in profile (`CODEX_MODE_COMMAND`; the command is typed and its Enter pressed 400 ms later, `CODEX_SUBMIT_GAP_MS`, because Codex takes an Enter that arrives with the text for a line break and leaves the command unsent; checked against Codex 0.166), and is put back, with a toast, when
Codex's screen says it refused the command. Codex has no file that says its mode, so the button shows what the chat was
started in or last moved to, and "Permissions" where neither is known. It starts the agent in a terminal the daemon owns, shown as its chat (`startAgentSession`, ADR 0003), so the terminal | chat switch works for every agent; the picked model and effort go on the launch line (`launchFlags`: Claude `--model` and `--effort`; Codex `-m` and `-c model_reasoning_effort=`) and the first message waits under the session until the agent takes input. **The agent is started while the message is written** (`useWarmAgent`): once a folder is picked, and 400 ms after the last change of agent, model, effort or mode, the page asks the daemon to keep that agent warm (`prewarm_agent_session`, API_PATTERNS), asks again every four minutes while it stays open, and says none is wanted when it closes; the start then takes the warm agent, because it asks for the same words (`startWords`). It is a head start only: with a branch to switch to or a worktree to make, on a daemon from before it, or when it fails, the agent starts on send as before. On **This folder** a branch pick runs `git switch` (`git_switch_branch`); on **New worktree** the daemon makes a worktree (`add_branch_worktree`, `Command::BranchWorktree`: named by `worktree.dir`, trusted like its repo, the repo's setup run, listed for tidy-up) and the chat starts there. The branch is settled before the agent starts, and a failure stays on the page. **Resume** lists, under the suggested tasks, the conversations Claude or Codex saved for the folder (`saved_conversations`, API_PATTERNS; asked off the UI thread for the folder and agent picked, an answer for an earlier pick dropped, `useSavedConversations`): each its first message and how long ago it was active, newest first, at most 20 in a list that scrolls. A click starts the agent in that folder with that conversation (`resumeWords`: `claude --resume <id>`, `codex resume <id>`, the id held to the daemon's rule, letters, digits, `-` and `_`, at most 128, a letter or digit first so it is never read as a flag, and refused before any branch or worktree is made otherwise) and the picked model, effort, mode and any typed message; Own copy is left out for that start, which the line over the list says while it is on, and while the message box has text a line says "Your message is sent to the conversation you pick." With none saved the list is not shown; a failed read says "Could not read the saved conversations." and a daemon older than protocol 85 "Restart the Maitty service to see saved conversations.", one quiet line each, and a new chat starts as ever. It is not offered on another machine or for Gemini. There is no Subagents fact on the page: it counts the subagents a running chat started, which a chat that has not started has none of. **On another machine** the same page opens, titled for that machine (`machine` prop; the sidebar's New agent chat on a machine, and a focused remote session's machine, choose it; "Start on …" in the project menu switches it). It lists that machine's projects (`machineProjects`), takes a pasted path and has no folder picker or **Browse…** (this window cannot browse another machine's disk); it has no branch, worktree, attach or warm-agent controls, and the model button holds only the agent (`AgentPicker agentOnly`), because this computer's model lists and branches say nothing of the other machine. A worktree is that machine's daemon's `worktree.per_agent` setting. The window starts a shell there (`startSessionOnMachine`), holds the first message under the new session (`firstMessage.ts`) and types the agent's command into the shell; the line is built by `launchFlags` from the agent and a permission mode from the fixed list only, so no free text reaches the remote shell. For a terminal session the
message is also handed to the session's daemon (`send_when_agent_ready`), which types it in, so it is delivered when
the window closes first; the window keeps showing it until the history has it, and sends it itself only when the
daemon does not take it. There is no New session dialog. **Layout 2g changes the page above:** it opens on the most recent project with no folder step (the search list shows only when there is no project), and the "Work in" menu is replaced by an **Own copy** switch, on by default (the agent works in a worktree of its own, on a new branch under a suggested name, started from where the checkout is, so ↓ Pull first when the line below says it is behind; off, it edits the checkout). A line under the controls says how the checkout stands (`git_sync_state`, which fetches off the UI thread, never errors when offline): with Own copy on "Starts from x" (and "↓N behind origin/x" when it is); off and behind, "↓N behind origin/x" with **↓ Pull** (`git_pull`, `git pull --ff-only`, refused with "You have uncommitted changes. Commit or stash first." when tracked files are changed); else "Up to date with origin/x". Under it, up to four other recent projects are **Switch to** chips with their branch (`--branch`) and ↓N when behind. A chat running in the pane has a **problem banner** over its message box (`ProblemBanner`, one at a time: a lost machine with Retry now, a stopped agent with its exit code, View log (the terminal) and Restart, a context at 95% or more with Compact, which types `/compact`, or **New chat with summary**, which opens the new chat page on the same machine and folder with a plain-text summary (`summaryOf`: the task it began with, the latest asks and the agent's last reply, written from the chat, not by a model) as the first message to edit); the tray's branch button opens **This chat works on** upward (`BranchMenu`: switch the folder's branch, or **Move to a new worktree…**, which makes the worktree, types `/exit` to the idle Claude or Codex agent, waits up to 15 s for the pane to be a shell, then types `cd "<worktree>" && claude --resume <id>` (Codex: `codex resume <id> -C "<worktree>"`; `;` on Windows) into the same pane: the pane and daemon session stay, only the agent restarts, uncommitted edits stay in the old folder, and the row is off with the reason while the agent works or asks, has no saved conversation id (`TermInfo.session`, checked against a strict pattern) or is another agent; a folder name with quotes, `$`, `` ` `` or `%` is refused rather than typed; `useMoveToWorktree`), and usage reads as three rings, Context, 5h and Week, amber from 80%. A chat started from the new chat page begins on **Medium** effort (`START_REASONING`; launched with `--effort medium`, or Codex's `model_reasoning_effort=medium`), and the model button names the level beside the model; pressing the picked level again lets go and the agent keeps its own ("Agent's own"). With no level picked, the button shows the level the agent's own files say it runs at (`TermInfo.effort`: Codex always, Claude when its transcript names one) and the menu says "Medium · agent's own"; where they say none it reads "Agent's own". A chat is never claimed to be on a level its files do not show. A Markdown document the agent wrote (a finished Write of an `.md` file) is a row of its own in the chat, never folded into the run around it, and **Read** opens its text in place (the file may be outside the project, where no panel shows it). A machine's name is the one its owner gave it (**Rename…** on the machine heading, or **Rename machine…** on any of its rows) in every text that names it: the row's machine, the status bar, notices, toasts and dialogs (`machineLabel`, `data/machineNicknames.ts`); its own name stays the key everything is addressed by. A row's place line keeps the project name at least seven characters wide, and the branch and machine give way first. The working badge shows the spinner Settings picked (`StatusBadge`), in the working colour on the tile's own background. A session the daemon's list stops naming leaves the sidebar at once when this window closed it, its daemon gave a removal reason (`Snapshot.removed`, `Unknown` included; another machine's reasons count for its own sessions) or its machine was taken off the list in this window; with no reason it stays, shown as unknown, and the window logs one line with its id and the revisions of the list that named it and the one that did not, until a full list confirms it is gone (`trackMissing`). For this computer that is a read of the daemon's list (`list_sessions`, `confirmGone`), ten seconds after the miss and every ten seconds after; a failed read is logged once, and once more when one works. For another machine it is a list from that machine that comes at least ten seconds after the miss (`confirmGoneOnMachines`): every list the host sends of a machine is its whole list, and at each check the window asks each machine that is up and holds such a row for one (`ask_machine_list`). This computer's list never confirms another machine's session, and while a machine is down its rows stay as they are, unknown, until it is back. A pushed list, a whole `State` too, never confirms it: after a daemon restart the first `State` names no reasons. ⌘J (Ctrl+Shift+J elsewhere, since a bare Ctrl+J is a terminal key) jumps to the next session that needs you or stopped, and the Needs you heading shows the key. **New agent chat** (⌘⇧A,
Ctrl+Shift+A) opens the page above, and **New shell** (⌘⇧T, Ctrl+Alt+T, the sidebar's terminal button, a
machine's New shell here) starts a plain shell in the start folder with no dialog; another machine asks for a folder there first, so it opens the dialog. A screen reader is told when a machine goes down (`build disconnected: unreachable`) and when it is back, once each and not for every retry, through a polite live region; every state is a word as well as a shape and a colour, nothing about machines animates, and the Machines page works with the keyboard alone (Tab, Enter, Esc ends a look in progress without closing Settings). Features that read a pane's files (Inspect, worktreesFeatures that read a pane's files (Inspect, worktrees, opening a file from the output, dropping files, the screen text) are for this computer's panes only: each says `<Feature> is local-only in v1: the files are on <machine>, not on this computer.` and the Repository button shows disabled with that text.

**Recent sessions** (`RecentSessionsDialog`, `model/recentSessions.ts`, `useRecentSessions`; the clock button in the
sidebar's foot and the rail's, and **Recent sessions…** in the palette) lists the agent sessions that closed on this
computer, newest first (`closed_sessions`, API_PATTERNS; read when the list opens, off the UI thread): each its title
(else "Claude session"), the agent's mark and name, the folder, the branch and "closed 3h ago". Picking one closes the
list and starts that agent in that folder with its conversation, through the new chat's resume start (`startNewChat`
with `resume`; the id held to `resumeWords`' rule, model, effort and mode left to the agent); while another start is
under way the list stays open and a toast says "A session is already starting". A row whose folder is gone but was a
worktree with a known repository and branch reads "folder removed · closed 1d ago" and can be picked: the service makes
the worktree again and starts the agent itself (`reopen_closed_session`, API_PATTERNS; never `startNewChat`), while a
line over the list says "Making the folder again and starting the session…" and every row is `aria-disabled` until
it answers, so no second session starts meanwhile; an answer that comes after the list was closed changes nothing. Once it has, the
list closes; a refusal (a branch that is gone is named) shows as an alert line over the list, which stays usable, and a
service older than protocol 87 says "Restart the Maitty service to reopen it." A row cannot be picked
(`aria-disabled`, not `disabled`, so it can still be reached and read), and says why in place of the time, when its
folder is gone and was not such a worktree, which includes every record from before protocol 87 ("folder removed;
cannot be made again"), when its agent is not Claude or Codex ("cannot be reopened here"), or when its id fails the
rule ("no conversation to resume").
It says "Looking for closed sessions…" while it reads, "No closed sessions yet. A chat you close shows here once its
Undo has passed." when there are none, "Could not read the closed sessions." with **Try again** when the read fails,
and "Restart the Maitty service to see closed sessions." for a daemon older than protocol 86. Esc, Close or a click
outside closes it. Only this computer's daemon is asked: another machine's closed sessions are not listed.

**Desktop service states** (`ServiceStatus`, `model/serviceState.ts`). While maittyd does not answer, every status reads
Unknown, spinners stop and input waits. The first four failed tries show a thin bar under the pane header, "Reconnecting to
maittyd · your sessions are still running", with "Try N of 5 · next in Ns" (it ticks each second) and **Retry now**. After the
fifth a veil (the main card under `color-mix(--main 55%)` and a 3px blur) and one glass card take over: **Maitty service
stopped**, a line saying how many sessions are saved, the technical details under a Details disclosure, and **Start service**,
**Try again** and **Open log** (shows `daemon.log` selected in the data folder; the only record of why it stopped). The desktop's connection starts maittyd itself on every try (`open_or_spawn_for_user`), so both buttons try
again at once; there is no separate start-service command. A running service of another protocol gets the same veil and card,
**Restart Maitty to finish updating**, "The app updated, but the service behind it is still the older one.", a tray of what a restart
keeps and stops (sessions that reopen, agents that will offer Resume, shells that stop; a line with a count of 0 is left out)
and a **Restart and restore** button that asks for a second click (`restart_mismatched_daemon`, then "Service restarted · N
sessions back"), with `protocol 75 → 76` beside it (protocol numbers are never written as app versions). **Later** leaves a slim bar. The sidebar foot says "Service stopped" or "Restart
needed" in muted grey beside a red dot, never a red banner, and no needs-you toast shows while the service is not connected. A failed action whose only reason is that the window has no line to the service (`lib/serviceLine.ts`) shows no error toast either (`lib/toast.ts`): the bar, the card and the sidebar foot already say it. Agents that were mid-turn offer **Resume** in the restore banner; nothing resumes by itself.

A terminal UI (ratatui). Colours come only from the theme (`theme.rs`): semantic
slots `bg`, `card`, `card2`, `hov`, `accent`/`acc_ink`, `text`/`strong`/`muted`,
and status colours (`working`, `blocked` = needs you, `done`, `idle`, `err`).
A working row is one colour, `accent`, across its glyph and its name (shimmering with motion on). Needs you (`blocked`) is amber and failed (`err`) red in every theme, so they never read alike. `dark` sets its amber by hand; `mono` uses named yellow and red; every other theme keeps an amber of its own that stands apart from its other statuses, or gets one derived from its yellow (`with_distinct_needs` in `theme.rs`, darker on light themes). The audit in `theme.rs` asserts needs you looks unlike every other status, on the row and lit. A theme's amber `working` slot stays out of the sidebar. Chips on the check sheet, the Inbox's group heads and the approval tint use the same slots (`client/reference_frames_tests.rs` asserts the roles).
Every built-in theme passes a WCAG contrast audit test. Shared primitives in
`client/design.rs` and `client/screen/`: `panel()` (popup), `fill`, `put`/`seg`
(text runs), `keycap`/`keycaps`, `hints`, `button`, `dim_all`. Icons come from
config (`[icons]`), with Nerd Font glyphs where present.

## Project-specific: GUI design system rules

Rules for `apps/gui/`. Owner-approved 2026-11-04.

1. Every colour, radius and shadow comes from a theme token (a CSS variable),
   mapped into Tailwind by the `@theme inline` block in `apps/gui/src/index.css`.
   No raw hex, `rgb()` or `hsl()` in components.
2. Every popup (menu, dropdown, dialog, tooltip, popover, listbox) is a
   shadcn/Radix primitive from `src/components/ui/`. No hand-rolled focus, Esc
   or arrow-key handling.
   The same goes for the controls inside a page: a switch, a segmented or radio
   choice, a select or picker, a dialog. Each one exists once in `components/ui/`
   and every feature uses it ([shadcn-tailwind](stack/shadcn-tailwind.md#primitives),
   "One component per control").
3. New components use Tailwind utilities on those tokens. Heavy animation and
   per-style rules (the four window styles, `data-gpu`) stay in `material.css` and `workspace.css`.
4. Old CSS moves to these rules only when a ticket touches that area. There is
   no big-bang rewrite.

`pnpm lint` enforces rule 1 in TS and TSX: `eslint.config.ts` rejects a hex,
`rgb()` or `hsl()` literal in `apps/gui/src`. Exempt: test files, the theme
files in `features/workspace/theme/`, and `TerminalView.tsx` (the xterm.js
palette takes literal colours). Don't widen the exemptions to pass a change.

`pnpm lint` also enforces the one-component-per-control rule: a hand-built
`role="switch"`, `radiogroup`, `radio`, `dialog`, `alertdialog`, `menu`, `menuitem`,
`listbox` or `option`, or a `createPortal` import, outside `components/ui/` is an
error. There are no exempt files.

## Project-specific: desktop window sizes and sidebar order

Rules for `apps/gui/` (the look is in [DESIGN](DESIGN.md#desktop-app)).

1. The window is at least 720px wide. Under 1100px the sidebar is the 60px rail and
   expanding it opens the full sidebar as an overlay; under 800px panes stack in one
   column. The widths are constants in `features/workspace/model/windowSize.ts`; change
   them there and in DESIGN, nowhere else.
2. The sidebar lists **agents and shells only, grouped by machine** (this computer first). Within a machine the
   order is the **Session order** setting (`sidebarOrder`): `creation` (the default; the daemon's pane number) so a row
   never moves when its state changes, or `priority`, where a session that needs you goes to the top and the rest keep
   creation order (`model/sidebar.ts`, `bySidebarOrder`). The command palette still lists the most urgent first. A machine can
   be grouped by project and folded. There are no fleets, ships, tabs or tab bar: choosing a session shows the layout it
   is in (`model/layout.ts`), else one of its own. A session's **subagents** are the daemon's `TermInfo.subagent_activity`
   (`model/subagents.ts`), shown under their session (small icons when it is not selected, a row each when it is);
   clicking one shows its own chat in the session's pane until you pick a session or **Back to main session**. Rename is
   in place (double-click, F2 or the menu). Right-click menus share `MenuPopup` (focus, arrow keys, Esc, kept inside the
   window). Split layouts are this window's own state, up to four panes, mixed right and down with draggable dividers; a
   session closes with a 6 second Undo. Crewmates (`features/crew/`) is not mounted: the redesign leaves it undesigned.
3. A terminal sits on the workspace card's own ground (`--main`, xterm's background and ANSI black), so a pane reads as
   part of the card and not as a box inside it.
4. The window's state is put together once, in `features/workspace/useWorkspaceState.ts`, and `DesktopApp` hands it to
   the window's parts (`WorkspaceSidebars`, `WorkspaceMain`, `WorkspacePane`, `WorkspaceDialogs`) through
   `useWorkspace()` (`workspaceContext.ts`). A part of the window reads what it needs from there and is not passed the
   window's state as props; a component used outside the window (`Sidebar`, `PaneCard`, a dialog) still takes props.
   New window state goes in a hook of its own that `useWorkspaceState` calls, not in `DesktopApp`.

## Project-specific: usage and cost page (desktop)

The Usage and cost page, a section of Settings (`apps/gui/src/features/usage/`, look in [DESIGN](DESIGN.md#desktop-app)) shows only
what providers reported through the daemon's usage store ([ADR 0009](adr/0009-usage-is-provider-reported-never-estimated.md)).
A figure nobody reported is a dash in the UI, never zero and never computed from tokens. The monthly
budget is a client preference (`monthlyBudget`, validated in `preferences.ts`); the ring needs a budget, a
known cost and the Month window. The report is fetched with a query (no polling faster than 60 s), and
the page never blocks the UI thread.

## Project-specific: layout and navigation

<!-- local: user requested sidebar-bottom controls and no global footer; on 2026-10-24 they chose one exception, the mode bar along the bottom. -->

Maitty has one terminal interface, drawn to the TUI redesign: two separate frames with a one-column gap, the **sidebar** (each machine with its sessions; see *the sidebar*) and the **workspace** (one pane in a box with its title chip, or up to four boxes; see *the frame*), and the status bar along the bottom. `ui.gap` is the space between the frames. `ui.rounded` (`full`, `corners` or `off`) rounds the corners of every box (`╭╮╰╯`) with `full` or `corners` (the default); with `corners` chips, buttons and rows keep square ends, and with `full` they end in half-circle pills (a Nerd Font glyph). `ui.padding` is the blank space inside the terminal round both frames (`draw.rs` `padded`): `small` none, `medium` (the default) 1 row and 2 columns, `large` 2 rows and 4 columns; `framed` is that room less the mode bar's row. Older config values still load (`Ui::fold_retired_layout`, the file not rewritten): `tiled` (and `cards` with `panes = "tiled"`) as Floating with `gap = 0`, `grid` as Floating, and `compact`, `flat` and `hairline` as Floating. The first start with such a file says so once in a toast (`RetiredLayout::note`) and writes what stands for it into the file (`client/mod.rs` `settle_retired_layout`). Tab, Shift+Tab and Esc go to the program in the pane; the leader's Focus the sidebar moves focus to the sidebar. There is no application-wide footer bar but the mode bar (see *Mode bar*). The sidebar's foot holds the release chip and the **menu** chip, and the sidebar hides as *hiding the sidebar* says. A pane's title is separate chips on its top border: the status mark and name (the accent while it has the keys), then the folder, then the branch, a line cell between them (`workspace.rs` `title_segs`); the terminal sits inside `PANE_PAD_X` by `PANE_PAD_Y` of blank space. The first time it starts with Claude Code installed and its hooks missing, it asks once to connect them (`App::offer_claude_hooks`, remembered in `saved.hooks_offered`); `<APP>_NO_INTEGRATE=1`, a side server and a source-checkout build never ask. The mode bar along the bottom always shows the mode, then where the keys go (`keys go to claude`).

The prefix is `Ctrl+Space` by default; it starts key mode (`Ctrl+A` still works for one release unless it is bound to something or the prefix was set on purpose). Usage and cost (`u`, `screen/usage.rs`) is a modal of what each agent reported using (context, tokens, cost) and the plan limits, as the providers reported them (ADR 0009); it estimates nothing. Inbox (`Ctrl+J`, prefix `i`), Changes (`c`), Files (`f`), Branches (`g`) and References (`e`) use the right-hand sheet. Settings (`,`) opens as a centred modal over the workspace so the pane layout does not change. The key overlay (the prefix; Search commands is `Ctrl+K`, `?` lists all), the branch switcher and new agents use popups. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. A click on a toast (a finished session's note, the Restart now button) is the toast's even where it covers a pane whose program asked for the mouse; the program never sees it (`Screen::toast_rect`). A right-click menu answers each item's key while it is open and shows it only when it is a leader key (see *Menus*): the leader key bound to the same action in the live keymap (`menu.rs` `menu_keys`: split `s` / `S`, close `x`, Changes `c`), else a letter of its own, clear of those keys. A bound key the menu answers itself (`j` / `k` and the arrows, Enter, Esc, Delete, a check's `r` `o` `O` `u`, or the `x` that closes) goes to the menu, and that item takes a letter instead. Prefer Maitty's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll. Branch (`b`, `client/screen/branch_sheet.rs`) uses the right-hand sheet: the session's branch, ahead/behind its upstream and its base branch (the main checkout's, else `main`, else `master`), the last 10 commits, the session's last check and its snapshots, all read by git in a background task. Its first row, **Switch branch…**, opens the switcher popup; Enter on a snapshot starts the row menu's restore (asked first). A folder outside git says so instead of the lists; an upstream whose remote branch is gone reads `gone from the remote`, and a git error shows in the place of what it was reading. References (`a`, `client/screen/references_sheet.rs`, files through `crates/maitty-terminal-client/src/references.rs`) lists the project's screenshots and design files from `.maitty/references/` at the repo's main checkout (a folder outside git keeps its own), so agents in every worktree see one set: name, kind, size and age per row, newest first, the selected row a neutral tint with an accent `▎`. Enter (or `o`) opens one in the system viewer as a Ctrl+click path does, `p` (or Ctrl+V) adds the clipboard image as `screenshot-<UTC time>.png`, `n` asks for a file's path (typed or pasted; a drop's quotes come off) and copies it in, `d` (or Delete) removes the chosen one after a danger confirm, `y` copies its absolute path; `a` again or Esc closes it. A paste while it has the keys stays in it: one line naming a file (a file dropped on the terminal) is added, an empty paste adds the clipboard image, and anything else gets the toast "References has the keys: …", which never repeats what was pasted. A name starting with a dot is refused. Entries the listing left out (unreadable, or links out of the folder) are counted on the list's last row in `err`, with the first reason. Empty, it reads "No references yet · paste an image or add a file", where they are kept and that agents find them through MCP, and its action bar holds only the ways to add one. Every file step runs in `spawn_bg`; the first time Maitty makes the folder in a repo it adds `.maitty/references/` to the repo's `info/exclude`. Over `--remote` it is refused with a toast, like Files. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. Prefer Maitty's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll.

**Send a file** (`client/send_file.rs`, ADR 0007). In a pane on another machine the leader key `d` opens the prompt `Send a file to this machine: its path` (mode bar `SEND FILE`; a paste or a drop puts its first line in the box, a drop's quotes come off). Enter starts the upload in `spawn_bg`; a toast reads `sending <file> to <machine>… <n>%` and is refreshed once a second from the byte count, without adding to the notification history. `d` again, or a click on the toast's close mark, stops it (`stopping the send to <machine>…`, then `stopped sending the file to <machine>`); Esc only dismisses the toast and does not stop it, because Esc also belongs to the program in the pane. One file is sent at a time. On success the far-side path is typed into the pane like a local paste, and the toast says `sent <file> to <machine>`; every failure is an error toast (not a file, over 100 MB, no ssh to the machine, a login that needs a prompt, the machine could not keep it). On a local pane the key says there is nothing to send.

The sidebar's **+ New** chip and `n` start a shell at once (see *New session*). A project row's **+**, the empty project row's new action and leader + `n` are one action (`Action::ShellHere`): a plain shell where you are, whose tab opens with focus. You type your agent in it (see *Agent launchers* below). The Menu's `Session...` opens the same dialog; there is no instant agent session any more. **Menu** is in the sidebar footer. Every row of the key overlay and the Menu is clickable and wheel-scrollable. Include configured commands and retain existing confirmations.

When an update is available, show **Update <version>** at the right end of the mode bar and open the existing confirmation flow when clicked. Do not show the control when Maitty is current. Keep status icons and pane status tags clickable to open the evidence explanation. Terminal bells do not add sidebar indicators or history entries.

Mark one selected sidebar row independently from hover: the keyboard selection stays selected when the mouse moves; hover is a separate, quieter tint. The open session keeps a dim selection bar while another row is under the keyboard. Preserve agent status icons and colors in each state.

Settings opens on General and reports installed, window and daemon versions separately, with a labeled remote daemon and restart-pending notice when they differ. When a newer release is out, the first line under the section's title, in every section, says so in the accent with an **Update now** button (`dialogs.rs` `draw_settings`). The modal (`sheet.rs` `draw_settings_modal`, up to 110 by 32: the redesign's 96 columns and a little more for real values) has the sections down the left (`modal::Cat::ALL`: General, Appearance, Sessions, Alerts, Agents, Check and Keys; 22 columns, 14 under 90; the chosen one on `selection_bg` with an accent `▎`), the section's lowercase title at the right, its rows under it, and a footer: `↑↓ move  tab section  enter change  o open  esc close` and `saved to <the real config path>` (`config::config_path`); the title chip reads ` settings `; each groups its rows under headings in `design.rs` `GROUP_ORDER`, rows keeping their `SETTINGS` order within a group: General: UPDATES (the Version row first, then Check for updates), STARTUP, INPUT, SHELL; Appearance: THEME, LAYOUT, PANES, STATUS; Sessions: NEW SESSIONS, SIDEBAR, WORKTREES, RESTARTS, HISTORY; Alerts: SOUND (Alert sounds, then a switch for the needs-you sound and one for the finished sound, and Sound volume: `quiet`, `normal`, `loud`), PHONE; Agents: AGENT STATUS, HELPERS, LIMITS, AGENTS TALKING TO AGENTS; Keys: the leader's shortcuts by group, then WITHOUT THE LEADER (the Alt+letters, Alt+1–9, Alt+arrows; `design.rs` `direct_key_rows`). Enter on a one-key row takes the next key pressed and writes it to `[keys.prefix]`, or `[keys.global]` for a key without the leader. Show the update action only when an update is available; keep release notes and confirmation in the existing flow.

There is no Team tab; `[team]` in `config.toml` still loads and is no longer shown. The label column is as wide as the section's longest label (33 at most; no label is longer); the value follows it: a choice is its current value bold in the accent, `‹ ›` and where it stands among the choices (`2/3`), and Enter, Space or a click steps it; other rows show their own control. The selected row has the `hov` ground. The selected row's help (`Setting::help`) sits under a rule at the bottom of the page, word-wrapped over up to two lines, so a help text stays within two lines at 80 by 24.

**Settings › Sessions** opens on NEW SESSIONS (`[sessions]`: `Agent` last used, claude, codex or gemini, `Where` new worktree or this folder) and HELPERS (`Sessions may start helpers` ask, always or never; `Helpers at once` 2, 3 or 4), then the older alert, restart and history settings. Values outside those lists are refused when the config loads. `Agent` and `Where` are read only by the worktree and agent flows that remain (an old `plus_starts` key is read and ignored); the helpers settings are read by the daemon when an agent starts a session (see *helpers*). `[mcp] helpers_at_once` is not a setting any more: it is read only as a fallback for `[sessions] helpers_at_once`. **Settings › Agents** opens on AGENT STATUS: Claude Code reads `✓ hooks installed` (read once in the background when the page first shows, `checking…` until then, `hooks not installed` otherwise; the same test as `maitty doctor`), Codex reads `✓ read from the process and screen` until its lifecycle hooks are installed, then `hooks installed · trust them in Codex /hooks` until Codex has recorded trusting each, then `✓ hooks installed and trusted` (read once in the background; its help says Connect agents installs them and you review them once in Codex's `/hooks`), and `Connect agents…` opens the Connect agents list (`Action::AgentStatusSetup`), which offers exactly the agents `maitty integrate` sets up (`STATUS_AGENTS` in `crates/maitty-terminal-client/src/cli/integrations.rs`). An agent that started before its hooks were installed never reports through them (`TermInfo.hooked` stays false) until it restarts: the palette's **Reload agents that missed their hooks** (`Action::ReloadAgents`) and `maitty reload-agents` restart each one in place and resume its conversation (`Command::ReloadAgent`, `reload_agent` in `restore.rs`). One working or waiting for you is left alone; `maitty reload-agents --all` restarts it too, and `--dry-run` only lists (`cli/reload.rs`). SUBAGENTS has `Subagents in the sidebar` (`ui.subagent_count`, on by default): off leaves the subagent rows out of the sidebar. The `mcp.*` and worktree settings follow. General's `Leader waits for next key` (`ui.which_key`, off by default) shows every key if you pause after the leader.

**Subagents are rows, not chats.** The sidebar lists the subagents of an agent's current turn under its row (see *the sidebar*), from what the daemon tracks (`TermInfo::subagent_activity`), and nothing more: a click opens the parent's pane, where the agent shows them in its own terminal. There is no chat view in the TUI and no inspector. `ui.subagent_count` turns the rows off.

**Secret requests open by themselves** (`client/screen/secret_ask.rs` `auto_open_secret`, called after each state update). A request not yet offered opens its prompt beside the row when the mode is Normal or Side and no side view is open; otherwise it waits and the row's `asks for a secret` line stands. Each request id is offered once, so Esc is final. A prompt opened this way ignores keys and pastes for 1.5 s (Esc still declines), so a line being typed in a pane is not given away as the secret; one opened by Enter or a click on the row has no wait. Closing returns to the mode it opened from.

**Helpers are not a mode** (`client/screen/helpers_ask.rs`). Any session may start helpers (other agents it starts through `maitty mcp`); `[sessions] helpers` (ask, always, never) decides. When a session asks and the setting is `ask`, its row is **needs you** (amber, as for any question) and its second line reads `wants to start N helpers` (`wants N helpers` on a narrow sidebar). Enter on the row with the sidebar focused, Enter (or a digit) on its row in the Inbox, or a click on that line, opens a popover beside the row (from the Inbox, Esc or an answer goes back to the Inbox on that row): the screen dimmed except that row, a full accent border, a `◂` on the row, the title `START HELPERS? · <session>` and `Esc later` at the right. It says which agent wants to split the work into N helpers, lists the tasks (five, then `+ n more`), then "Their permission prompts come to your Inbox. Nothing merges." Buttons and keys: `1` Allow, `2` Always in <project> (`Always here` when narrow), `3` No, do it alone, each clickable; Esc decides later and the row keeps asking. Enter, `y` and Space do nothing there, and `1`/`2`/`3` act only inside the popover: no bare key answers from the sidebar or the collapsed strip. The answer is a command the daemon accepts only from you and only while it holds the request (`docs/API_PATTERNS.md`).

Keep update indicators until the installed program, window and daemon versions agree. Installing an update leaves open sessions running until **Restart now**. For a local daemon, when no panes are open, restart it automatically (see below). For a remote daemon, explain that it must be updated on its host after its sessions are closed. The one `↑ Update` pill sits at the sidebar's foot above its divider, or on the mode bar when the sidebar is collapsed, hidden or short, and becomes `↻ Restart` once a newer maitty is installed (here, or from outside maitty) and the server or this window still runs the old one; it clicks through to the same restart. The splash carries the matching line; there is no second notice in the working area.

**Restart now** is one step: it restarts the server onto the installed maitty **and** replaces this window with the installed one in the same terminal. It shows in Settings' restart-pending line (`R`), on the toast after an install (it stays 12 s), on the `↻ Restart` pill, and as the palette action *Restart maitty* (`Action::Restart`, `restart`); none show for a remote daemon or while a restart is under way. It asks only when a session is working or waiting for you (or when `restore.enabled` is off, since then sessions close for good): a confirm (`menu::Confirm`, danger) reading "4 sessions stop and resume, 2 are working, 1 waiting for you" with Restart (Enter) and Cancel. Otherwise it restarts at once. The server saves its session and every pane's output first (`KillServer`), then the window closes and the installed maitty opens in its place (`restart::relaunch`: exec on Unix, spawn-and-exit on Windows, same arguments and `MAITTY_SOCKET`), which starts the new server and restores the sessions. A window that is already current (only the server is old) reconnects in place; a current server with an old window swaps only the window and leaves every session running. An agent mid-turn is stopped when the server stops. A terminal agent cut mid-turn is not started again: its pane comes back as a shell with a Resume banner that offers its resume command (`TermInfo.interrupted`; see *After a restart*). A chat conversation resumes by itself on the same provider session (`restore.conversations`); the turn in flight is not replayed, and only with `restore.continue_turn` on is it sent `Continue where you left off.` once.

A new `maitty` that finds a server of another version replaces it on the way in. A server of an older release whose protocol this one speaks, and which holds no session, is stopped at once (`restart::replace_older_idle_server`, from the activity file the server keeps beside its pid file: `restart::Activity`). A server whose protocol this one cannot speak (`ipc::OtherVersion`) is judged by that file, or for a server too old to write one by the agents in the saved session (`restart::plan_for_mismatch`): when it holds no session, the terminal says `Updating maitty…`, the server is stopped by its process as `maitty kill-server` does across versions, and the window opens, with no prompt. When it holds sessions (a restart ends whatever runs in them: a build, a dev server, an agent's turn), the full-screen prompt (`client/restart_prompt.rs`) shows the protocols, how many sessions resume and how many are mid-turn, and **Restart service** (Enter, `r`) and **Not now** (Esc, Ctrl+C), each clickable; Not now prints the usual error and exits 1. A server newer than this maitty is never replaced by it, asked or not: that would be a downgrade. The prompt then offers **Update this app** (Enter, `u`) and **Not now**; the update installs the newest release over this maitty and starts it in this one's place, and the server and its sessions are not touched. When this maitty is already the newest release, it says a newer copy started the server and exits 1. Without a terminal on stdin and stdout, or with `--remote`, there is no automatic step or prompt: the error text and exit code are unchanged, and a remote server is never restarted this way. A same-protocol older server that holds sessions is left running; the `↻ Restart` pill offers the restart.

**One key overlay** replaces the command palette, the long shortcuts reference and the old leader overview (`client/screen/keyoverlay.rs`). It has three faces in one card with one typing row: Its card has no fill (its lifted rows are flattened to the ground), a red `×` close on the top border, and its key hints are chips on the bottom border.

- **Key map** (the prefix, or the pause after it). A banner, "Every key goes after the leader: `Ctrl+Space` then `s` = split", then Sessions, Panes, Sidebar, Tools and Maitty, each key a keycap followed by its name, from `key_groups()` and the real bindings. A pressed key runs its command; `Tab`, or an unbound letter, switches to the search. Three columns when the card is 100 or more wide, else two; each group's key column is as wide as its widest key. A key marked `›` opens a second step (Worktrees). Action bar: `key` run it, `Tab` search, `?` show all, `Esc` close.
- **Search commands** (`Ctrl+K`, `Space` after the prefix, or the Menu). A box with an amber border up to 110 by 26 (`draw_search`), an amber title chip and a pink `×`. `› type what you want to do`, then COMMANDS (RESULTS while typing) and SESSIONS. A command row is its label with the matched letters in the accent, a category (General, Panes, Tools, Find, Sidebar, Appearance) and its key as a keycap; a session row is its status dot, name and `project · machine`, and Enter goes to it. Matching is a case-blind in-order subsequence (`spld` finds Split down), names that begin with the text first. With nothing typed COMMANDS lists the last commands run and the first sessions. `↑ ↓ move`, `Enter run` and `Esc close` are chips on the bottom border. A set of keys (the arrows) is searched one key at a time. Every command the old palette offered stays reachable here.
- **Keys help** (`?`, the sidebar's menu → Keys; `keyoverlay.rs` `draw_keys_help`). An accent box on the panel ground, up to 92 columns: `keys` on its top border at the left and `esc close` at the right; three columns, `sessions` (new session, next / previous, jump to session, rename, filter, inbox), `panes` (split right, split down, focus next pane, zoom pane, close pane, move pane) and `window` (files, changes, usage and cost, settings, toggle sidebar, this help), each key as bound now in the accent (`help_key`: `j / k` for a pair, `1–9` for the jump keys); under them "Keys only work in key mode (Ctrl+Space). Everything else goes to the pane." A letter typed there starts a search; every other command is in Search commands.

Rules for keys and their labels: one key per action (no alias keys; a user's own `[keys.prefix]` entries still load), no default needing Shift, keys other apps use, sentence case, no bracketed explanations (the group gives the context). Every action has a leader key or is search only; nothing in the sidebar acts on a bare letter. `docs/shortcuts.md` documents the other screens' keys and is not embedded in the app.

**Project tools** (`client/screen/sheet.rs`, `menu.rs` `project_items`). A click on a pane's folder and branch chips on its bottom border, its right-click menu's Project…, or the key of each tool (leader `f` Files, `c` Changes, `g` Branches, `h` History, `e` References) open Files, Changes, Branches, History and References. A picked tool slides in as a panel from the pane's right edge, over its box including the pane's top line and its own close (`draw.rs` `grid`, `sheet.rs` `draw_panel_frame`): Files 96 columns and at most 80% of the panes, Changes 54 and at most 70%, the others 60 and at most 80%, until you size it (one saved width, `Saved::sheet_w["drawer"]`, drag its left edge or `[` `]`), so the panes are never resized by it; its title chip names the tool and ` ✕ ` closes it, and Tab / Shift+Tab step along the tools while it has the keys. Files shows the tree and the open file side by side from 60 columns, and by turns under that. Under the five tools the menu also keeps what the folder offers that the flat design has no place for: New / Open / Tidy worktrees, Rename and Close project, Snapshots and the check's output, re-run and override. The check sheets stay docked beside the panes (`PANES_KEEP` columns kept for them); the Inbox floats (see *the frame*). There is no tool rail. A draggable edge (the sidebar's, the panel's) lights in the accent while pointed at or dragged.

**Cursor.** The focused pane shows the cursor its program asked for (DECSCUSR, read from its output); where it asks for none, or for the default, Settings › Appearance › Cursor (`ui.cursor`: `block` by default, `underline` or `line`; an older `terminal` reads as `block` and `bar` as `line`) and Cursor blink (`ui.cursor_blink`) decide (`client/mod.rs` `cursor_code`). An unknown `ui.cursor` is refused when the config loads.

**Background.** Settings › Appearance › Background (`ui.background`): `solid`, the default, paints every cell the theme's `bg` and sets the terminal's own background to it (OSC 11, given back with OSC 111 on exit) so a plain terminal shows the whole theme; `terminal` leaves the chrome on the terminal's background (SGR 49) for transparent or image backgrounds (`client/render.rs` `draw`).

**Keycaps.** Every key the UI shows is a keycap from `design::keycap`: ` key ` on `btn`, `strong` and bold, with the ends round when `ui.rounded` is `full`. A combo splits on `+` into one cap per part joined by a dim `+`. The map, the search, hint rows, Settings key rows, dialogs and the copy hint all use it.

**Action bars.** A panel's key hints sit in their own one-row bar (`action_bar`, `panel_bar` in `popups.rs`) on a ground one step above the panel's: `card` for a sheet on the app background (Inbox, check evidence) and `card2` inside a modal (Settings, the key overlay, confirms, pickers). It spans the panel's width with one column of padding, shows keycaps and labels, and is always the panel's last row. When the hints do not fit they close their gaps, then give up the ones before `Esc`.

**Menu.** Sections a blank row apart, each under a `muted` title that says what its rows are for, labels capitalised, no letter lit: `NEW` (Shell here, Session..., Pane beside this); `TOOLS` (Files, Changes, Branches, History, References); `FIND` (Inbox, Search commands, All keys); `SIDEBAR` (Collapse sidebar); then Settings, untitled. The arrows move over rows and skip the headers; the selected row has an accent `▌`. In a short window the blank rows give way first, then the titles, so every row that does something stays (`leader.rs` `MenuRoom`). No row shows a key (a bare letter reads as one that works anywhere), but the leader key that does the same runs the row inside the Menu too; there is no footer. "Shell here" is leader `n` (a plain shell where you are) and "Session..." the New session dialog (Ctrl+T).

**Resume a conversation here** (Search commands only, `Action::ResumeConversation`, `resume-conversation`; `client/resume.rs`). It asks this computer's daemon for the conversations Claude and Codex saved for the selected pane's folder (`Query::SavedConversations`, one per agent; the answers arrive as messages). A menu by the pane opens at once with one row, `looking for saved conversations…` (picking it only closes the menu); once both answers are in, it shows the nine newest across both agents: `Claude · 3h ago · <the first message>`, cut to 44 columns. Answers that arrive after that menu was closed open nothing, and an error from this computer's daemon while it waits closes it under the error's toast. A daemon older than the request (`ClientMsg::introduced_in`, against the protocol it named when this window connected) is not asked: one error toast says `restart the daemon to list saved conversations`. Enter or a click starts that agent in a new pane in the folder with the agent's resume command from its `[[agents]]` definition (`claude --resume <id>`, `codex resume <id>`); an id that does not start with a letter or digit, or holds anything but letters, digits, `-` and `_`, is refused with an error toast. With none saved the menu closes and one toast line says `no saved Claude or Codex chats`. A pane on another machine gets the local-only toast (see *features that need a pane's files*).

**Agent launchers.** Typing an agent (`claude`, `codex`, `gemini`, `opencode`, a quick agent) in a Maitty shell in a git repo's main checkout starts it in a new worktree (`worktree.per_agent`); anywhere else, or in a folder that is not a git repo, it starts where you are, silently. It works from any shell because `<data>/shell/bin/` is first on the pane's `PATH`; see ARCHITECTURE › Agent launchers. No toast or warning is shown for main folders.

**New sessions** (`client/screen/newsession.rs`). Leader `n` starts a session with no form: agent `sessions.agent` (`last used` is the last agent started from Maitty, kept as `last_agent` in `maitty-ui.json`, else claude), project the one under the sidebar cursor or the one you are in, `sessions.where` (`new worktree`, which falls back to the folder in a project that is not a git repo, or `this folder`). The task is typed in the session, whose row reads `<agent> · new · type to begin` with the accent `○`. A new tab (leader `t`) takes only the shell that answers it: it waits ten seconds at most, takes only a pane that did not exist when the tab was asked for and that no agent started (another pane taking the focus leaves it waiting), and starting any other session before it arrives cancels the tab, so that session gets its own row and tab bar rather than becoming a tab of the one you were in.

## Project-specific: pane focus

Only the area with the keyboard is lit: the focused pane's whole border, or the open sheet. Unfocused panes fade by the Dim setting (off, light, normal or heavy; normal by default, and an older config's "subtle", "30%" and "60%" read as light, normal and heavy). Do not reserve a full-width bottom row for global hints; the mode bar is the one exception. The pane has no status line: its branch is in the sidebar's folder header.

## Project-specific: UI conventions

<!-- local: user requested distinct active, keyboard and hover sidebar states, mouse-first sidebar controls and parent-owned child-agent visibility. -->

- Popups are centred `panel()`s (`panel_at()` for a given rectangle) framed like a
  pane, in every layout: a box in the `accent` (a modal holds the keys) with corners by
  `ui.rounded`, on the panel ground (`flatten_modals`), the title as an accent chip on the top border two in, and a × chip (`btn`)
  a cell of border before the top right corner (the whole chip closes it); with `ui.rounded = full`
  both chips have caps. There is no accent bar across the top: the user chose the
  pane-like frame over an accent title bar (2026-10-24), and the frame
  tests apply that one difference (`as_bordered_header`). No shadow; the screen
  behind is left as it is, not dimmed: the modal fills its own area with the panel ground (`dim_all` only
  marks what is behind as taking no hover). Small explanations (why, a row's menu) are
  `popover_at()`s: a raised title row, no border, no shadow, beside the sidebar
  (`popover_rect()`). Settings is a
  centred modal that preserves the workspace layout and keeps its General,
  Sessions, Appearance, Agents and Keys sections, update controls, and keyboard
  and mouse editing available.
- The pointer's tint is one pass after drawing (`render.rs`): the topmost
  clickable region under it is tinted once, `mix(bg, text, 0.10)`, its ends round like every other shape when `ui.rounded` is `full` (`card.rs` `round_tinted`: an end cell holding only ground, or already a rounded end, becomes a cap or soft corner; an end with text stays square). Widgets do
  not tint themselves on hover. Only the top layer takes the pointer: `dim_all` marks every
  region drawn before it as under the overlay (`hits_under_overlay`), so behind a modal,
  popover or floating pane nothing tints and the pointer keeps its arrow, and an overlay's
  backdrop (its click-outside region) is never tinted. Inside the overlay hover works as
  anywhere.
- Shortcut mode ends on Esc, the leader again, a click, or an action that takes you into a
  pane to type (`n`, `s`, `t`, the splits, `spawn-*`, `o` once its
  dialog starts one, the worktree picker, opening a project). Each calls
  `leave_shortcut_mode`, which also sets an open side panel aside so the next key reaches the
  pane; `screen_new_session` and `screen_new_worktree` call it for you, and a new way to start
  a pane must too. `screen_start_pane` starts one without it, only for a pane nobody asked
  for (the home shell after the last session closes).
- Leader `x` closes what has the keys (`actions.rs` `ClosePane`): a side panel that has them
  (`view_has_keys`) closes with no question, so x never stops the program in the pane behind
  it; Files with unsaved edits stays open and says so. A click into a pane, or moving into the
  sidebar (`screen_side_set`), sets the panel aside: then x closes the focused pane, or in the
  sidebar the cursor's row, with its `Close pane` confirm. `f` or a click in the panel gives
  it the keys back.
- Selection is a neutral tint (`sel_on` of the surface it sits on) with a lime `▎` bar;
  never the accent colour as a row fill.
- Every popup row is clickable and keyboard-reachable; every key is shown as a
  keycap somewhere (hint bars, the key overlay, Settings).
- Short-lived notes are toasts above the pane content; destructive actions ask
  first (`Confirm`) and go last in menus.
- Anything filled that you click or read as a control (a button, a toggle, a tab, a tag,
  a key) is drawn with `button` / `btn`, `chip` or `pill`, so its ends follow the pill
  setting everywhere. Never a bare `seg` on a button colour.
- Copy is plain words ("Focus the sidebar", not "browse-tree").

## Project-specific: theme browser

Dark and light are two remembered slots and a switch. Appearance has three rows:
**Appearance** (`appearance`: `dark` or `light`, which slot shows), **Dark theme**
(`theme_dark`, `night` by default, shown as Maitty Night) and **Light theme** (`theme_light`, `day` by default, shown as Maitty Day). Maitty Dark (`maitty-dark`) and Maitty Light (`maitty-light`) are the neutral black and white pair.
`theme` stays "the theme showing now": `Config::settle_theme` sets it from the slot
`appearance` picks, so once a file has slots a `theme` written by hand is overwritten.
A file with only an older `theme` is not rewritten: its theme fills the slot its ground
belongs to and decides the appearance, one slot at a time (a light pick written beside an
old dark `theme` leaves that dark theme showing).
Enter or clicking a theme row's value opens a searchable browser with two tabs, **Dark** and
**Light** (`Tab` / `Shift+Tab`, or a click on the chip at the foot), each listing only its
themes, opened on the tab of the row you came from and on that slot's theme. Which tab a
theme is on is read from its background (`theme::is_light`), not from a list. The browser
shows palette swatches, an applied marker for that slot, a result count, and a preview.
Themes are listed alphabetically by display name, ignoring case (`BUILTIN` in
`theme.rs` is kept in that order, and every list of themes follows it); there are
no user-defined themes, only overrides of a built-in's colours. **Fifty themes are built in**, twenty-five dark and twenty-five light: Maitty's six (Night, Day, Dark, Light, and Retro Dark and Retro Light, which carry the Retro Pixel design's colours as a theme for any window style) and a dark and a light of each of GitHub, One, Catppuccin (Mocha, Latte), Solarized, Gruvbox, Tokyo Night, Dracula (with Alucard), Nord, Ayu, Everforest, Rosé Pine, Kanagawa, Nightfox (with Dayfox), Flexoki, Gruvbox Material, One Half, PaperColor, Tomorrow, Melange, Cyberdream, Doom One and Zenbones. The list is `crates/maitty-settings/themes.json`, the same file the desktop app reads, and a test holds `BUILTIN`, the display names, each theme's side and each family's ground and text equal to it. A theme id from an older build, one of the 160 or so removed in the cut, still loads: `ALIASES` in `theme.rs` maps it to the nearest survivor on the same side (by the colour of its ground), so a config that says `dracula` shows One Dark.
On a theme row in Settings, Left and Right step through that slot's themes and apply each at
once (`themes.rs` `step_theme`), so the whole screen shows the change; Enter or Space opens the
browser below for searching.
Typing filters by name or flavor; arrows, Home/End, page keys, and the mouse wheel
move the selection. Click a row to preview; click it again or press Enter to
save that tab's slot, which changes the screen only if that slot is the one showing. Esc or
the close button returns to Appearance without changing anything.
Saving runs in a background task; failure keeps the selection and shows a toast.
The key overlay's search has **Switch dark or light theme** (`toggle-appearance`, no default
key), which flips `appearance` and changes the whole screen at once.
The preview shows built-in defaults; configured overrides still apply after saving.
See [theme sources](theme-credits.md) for upstream palettes and local UI adaptations.

## Project-specific: working spinner picker

In Settings the Working spinner row shows the current spinner animated; ←→ step through the presets and apply each at once (like the theme rows), and Enter opens a mouse/keyboard picker with animated preset
previews and an applied marker: Braille, Dots, Line (ASCII), Snake, Pulse (each frame held
twice, so it breathes at half pace), Fill, Orbit, Bounce, Pair, Arc, Moon, Star (the last
seven from ratatui-spinner's flux frames), Blocks and Bars. There is no custom or static
row; frames set by hand in `ui.spinner` still load and work. Preview leaves the active config
unchanged. Enter or a repeated row click applies; Esc/close returns to the same settings row.
Save in a background task; disable duplicate saves and keep the applied choice on failure.
Choices affect only the working glyph; blocked/done/idle status colors and icons keep their
meaning.

The **Bars** preset is the 3-bar Braille spinner the reference frames draw. A frame is one column, or
two when every character is Braille; a two-column mark moves the sidebar row's name one
column right. Other choices are never rewritten. Working also shimmers its sidebar name in the working colour. With
motion off (`app.motion_on()`) the spinner rests on its third frame and the shimmer stands
still, so nothing depends on motion.

## Project-specific: sidebar check result

A session's row carries no check line. A failed check, or a command that ended in failure (`TermInfo.exit`), is the **problem** state: the red status glyph on its row, on its pane's border and in the collapsed column. The check's words, its checkpoints and its output stay where they were: the Inbox, the evidence sheet (`o` on a check in the Project menu, or Enter on a row that wants a look) and the Project menu's re-run and override items.

## Project-specific: check evidence sheet

The check evidence sheet is part of the frame (see *the frame*). Enter on a sidebar row whose check failed, could not run, was overridden, went stale or is never run for its repo opens it (a row waiting for approval opens the approval; every other row still focuses its session), as does Output in the Inbox. It docks right (`split_for_sheet`) and takes the pane column below `NARROW`. Its title is `CHECK · <session> › <checkpoint>`; content starts one column into the card's inside.

- A result chip (` <glyph> <word> ` bold on the state's colour: `failed` on `err`, `failed · overridden` on `text`, `could not run` and `not checked` on `violet()`, `passed` on `done`, `stale pass` on `btn`), then what it came to in `strong` bold (`exit 1`, `exit 0 on older code`, `never for this repo`; `command not found` in `violet()` for exit 127).
- Command, Folder (the run's folder, home as `~`), Ran (`14:22:05 · took 14.2s · after turn 3 ended 14:21:51`, local clock; `did not run` when it never started) and Seen by (`Maitty ran this itself. The agent said "…".`), or Source (`.maitty/checks.toml · <commit> · you chose never`) for a repo's own command. Rows are clipped at the sheet's edge, not wrapped.
- A banner on a tint for a result that needs explaining: overridden (by whom, when, `u undo`, the reason), stale (how many files changed and when, then one `M path` row per file that fits), never, could not run.
- `OUTPUT`, then `all N lines`, `last N of 1,284 lines` (numbered as the whole output, from `CheckInfo.line_count`; `f full output` when the daemon kept only the tail) or `empty`, on a darker block with 5-column line numbers; `✓` lines `done`, failure lines `err`, `▲` when there is more above. Empty output says why: `(no output)`, `(the command printed nothing)` / `exit 0 is what passed it.`, or `(nothing ran)`.
- Buttons on the third row from the bottom, each a click of its key: `Check settings ,` for never, else `Re-run r` (primary when stale, absent while it runs), `Override… O` (failed only), `Send to agent m` (failed only), `Undo override u` (overridden only), and `Open session o` when it fits. The last row is always `Esc back   r re-run   f full output   i why`.
- Keys: Esc closes; `r`, `O`, `u` act as the buttons; `m` sends a failure (below); `f` and the arrows open all of the output and scroll it; `o` goes to the session; `,` opens `PROJECT · <name>` for the session's project. With several checkpoints it opens on the headline one, and Tab / Shift+Tab move to the next / previous; the title names the one shown, its output is fetched by name, and `r`, `O` and `u` act on it.

**Sending a failure** (`client/screen/failure_note.rs`). `m` on a failed check, from its Inbox item or its evidence sheet, opens the agent's message box (the Inbox row's, or the popover beside the session's row from the sheet) with the failure written in: ``The test check failed: `npm test` exited with 1.``, then the last 40 lines of its output in a fenced block (`Its output:` when there are fewer; `It printed nothing.` when there are none). Colour codes and control characters are stripped, from the name and command too, and a line past 400 characters is cut. A draft the box held stays, above the failure. Nothing is sent until Enter, which sends it as any message is (several lines go in as one bracketed paste, queued or not). From the Inbox the output is asked for first (`Query::CheckOutput`) and the box opens when it arrives, unless an open sheet on that checkpoint holds it; a daemon error stops the wait. The sheet already holds it. The mode bar reads `m send to agent` while a failure is selected in the Inbox or shown in the sheet.

The row menu of a session with a check adds Check output (any result), Re-run check (not while it runs), Override check… (failed only) and Undo override (overridden only); letters come from `menu_keys` like every item. Output comes from `Query::CheckOutput` and is fetched again when a new run replaces the result. Command output is not trusted: colour codes and control characters are stripped before drawing.

## Project-specific: check modals and settings

Deciding is a modal; nothing here pops up on its own. Each matches its reference frames inside the modal frame, cell for cell, in every layout at 160×45, 100×30 and 80×24 (`client/screen/check_modals_tests.rs`). Colours are existing slots: `blocked` for "needs you" amber, `done`, `err`, `violet()`, `sky()`, `card2` for fields.

**Override** (`client/screen/check_actions.rs`): `OVERRIDE A FAILED CHECK`, 70 wide and 15 tall. It says what the session will read (`done (overridden)`) though which checkpoint failed and when, what it hides and where the failure stays visible (⊘ on the row, in the Inbox history and in the evidence, for this result only), and takes an optional reason (at most `OVERRIDE_REASON_MAX_CHARS`; typing, paste, Backspace and Ctrl+U edit it). Enter overrides; `O` overrides only while the reason is empty, so a typed reason can hold an O; Esc or Cancel does not. Opened from the Inbox, the Inbox stays under it and Esc or Cancel goes back to it, on the same item. The sheet, Inbox, popover and row menu all open this one modal. After it the evidence sheet shows that checkpoint and a toast says `Overridden. <session> reads done (overridden); the failure stays in its evidence.` A re-run says `Re-running <cmd> in <session>'s worktree…` in a toast, or `re-run queued` while the agent works; a daemon error (`ServerMsg::Error`) is a toast. While a sheet takes the pane column, toasts show over the sheet.

**Approving a repo's command** (`client/screen/check_approve.rs`): opens only from its waiting item, by Enter on the Inbox item or the sidebar row of a session whose checkpoint needs approval. Several waiting: one modal per checkpoint, the next opening after a choice. It shows the command (`$ npm test`, or `- old` / `+ new` when the repo changed one you had said Always to) in full: wrapped over as many `card2` rows as it needs, the modal growing to fit; `\n`, `\r`, `\t` and other control or invisible characters spelled out (`\u{001b}`), and a run of spaces shown as `[N spaces]`. Where it runs, the file and commit it came from (`added in a1b2c3d, 3 days ago`, `changed in <sha>, 2h ago`, or `changed, not committed`), when it runs, and "Read it. It can do anything you can in that folder." `1` and `2` answer only once the last frame showed the whole command, and only for that command: one too long for the window ends its last row in `…`, says `The whole command doesn't fit. Make the window bigger and read it first.`, and 1 and 2 only say so; a command that changes disarms `2` (`The command changed. Read it again before you choose.`). There is no default: `1` runs once, `2` arms Always (a strip spells out what it means and the button reads `2 again`) and a second `2` sends it, `3` is Never and opens the evidence sheet, Esc leaves the item waiting (back in the Inbox, on the same item, when opened there). Enter, `y` and Space send nothing and say so; other keys and pastes do nothing. Clicking Always once arms it like `2`. Below 80 columns the buttons use short labels; below 30 rows the modal loses the blank line after the command.

**Settings › Check** (`client/screen/check_settings.rs`) is one renderer with two hosts: the `PROJECT · <name>` modal (`,` from the evidence sheet or an Inbox row, for that session's project) and the Check page of Settings (the workspace you are in). Empty, it explains what a check is and takes a first command. Set, it shows the selected checkpoint's command, where it runs, its timeout (`‹ ›`), its latest result by clock time (`✓ passed on fix-footer 14:22 · 14.2s`, `… never run`, `? could not run … pnpm: not found (exit 127)` with what to do) and its source, then `CHECKPOINTS`: `1 of 1`, or `N of M · ‹ › switch` for several (←/→ on that row, or a click). Your checkpoints are `[[check]]` tables in config.toml; Ctrl+S writes the shown one by name in place, keeping comments, off the UI thread (a failed save keeps the draft and toasts; `Saved. Runs after the next turn ends.` for a first command, else `Saved.`). Del removes its table. A repo's checkpoint is read-only here: its source reads `.maitty/checks.toml · <commit> · always | never | asks each time` and Del (`Forget choice`) sends `ForgetCheckApproval`. Further checkpoints are added in config.toml or the repo's file; there is no add screen. Run now (`r`, or its button) re-runs that checkpoint in the session with the latest result, else the focused or first agent of the project; a working agent queues it. Esc drops the draft.

## Project-specific: Inbox check groups

The Inbox (leader `i`, or Ctrl+J) is part of the frame. Its title is `INBOX <n>`, n counting
everything not just finished or exited. It lists, in this order: NEEDS YOU, FAILED CHECKS, COULD NOT RUN,
FINISHED, EXITED, then HEADS UP, each a chip head on its colour (`blocked`, `err`, `violet()`, `done`, `err`)
with its count. A question is never below a check. EXITED lists a session whose last command ended in
failure (`TermInfo.exit`, set by the daemon) that asks for nothing and has no check of its own to show:
its line reads `exited with code N`, Enter or `o` opens it, and `c` or Delete clears it until another
command fails (`Saved`-free: `screen.exit_seen`, by when the command ended). A session that is both
finished and exited is listed as exited only.

- A group with no rows is not drawn; with nothing at all it says "Nothing is waiting on you.".
  A project with no check has the groups it always had, FINISHED renamed from JUST FINISHED.
- NEEDS YOU holds questions, then repo check commands waiting for approval
  (`approve check command: npm test`), then quiet working agents (`no output for 12m · may be
  stuck`, no sound; Enter opens the session, Delete hides it until it next goes quiet).
- FINISHED lists sessions with a check result first, then the newest first.
- A Done session is finished (`check::finished`) when it has no check, or its check passed, was
  overridden, passed on older code (stale) or you chose never to run it. It is not finished while
  checking, failed, could not run or waiting for approval. The home screen counts finished by the
  same rule. A session whose agent is working or asking is not listed for its old result (see
  `check::shown`).
- A failed check stays listed until a later run replaces it, even after you have looked at the
  session. A could-not-run check is listed only until you look (`Del` marks it seen).
- An item is the session glyph (`glyphs::Kind`), name (`strong` bold), ` · <folder>` and an age;
  at +2 the question or the result (`failed: test · exit 1 · 14:22`, `test could not run: pnpm
  not found`, `passed: test · 14.2s`, `failed, overridden: test`, `stale pass: test · 3 files
  changed since`, `not checked: never for this repo`); a failure adds its
  first failing output line. A plain finish, and an item that waits on you without having asked
  through the app, say what the agent last said (`check_inbox.rs` `last_said`: `TermInfo.said`,
  else `TermInfo.last_lines`, the last lines of its screen for an agent whose hooks report no
  message): one line cut with `…`, and on the selected item up to four wrapped lines over its
  **Reply** `m` and **Open** `o` buttons. With nothing said, a finish is its head line alone (its
  heading already says finished) and a waiting item reads `waiting on you`. Several checkpoints read as one word first, then
  `build ✓ · test ✕`, or the glyph strip when that doesn't fit.
- The selected item is tinted across the sheet with `▎` (`err` for a failure, else `accent`); a
  failure keeps its red `▎` unselected. It grows its buttons, each a click of its key: question
  `Reply m` `Open o`; approval `Read the command ⏎`; failed `Output ⏎` `Re-run r` `Override… O`
  `Send to agent m` (the last, dropped where it doesn't fit);
  could not run `Output ⏎` `Settings ,`; stale `Re-run r` `Output ⏎`. Enter runs the first
  button (Read the command opens the approval); an item with no buttons opens its session. `o` opens the session of any item; `r`, `O`, `u`, `,` act on an
  item's check where they apply and are typed otherwise. 1–9 answer a question; d, M, x and m
  act on a finished item; d, m and k on a heads-up; m on a failure sends it (see *Sending a
  failure*); Delete marks seen.
- Typing finds any session; the `› query` row shows only once you type. The last row is
  `↑↓ select   ⏎ open   r re-run   O override   Esc close`.
- A session on another machine ends its head line with ` · on <machine>`, and a machine whose line
  is down has no items (*machines in the sidebar*).
- Existing theme slots only. Code lives in `client/screen/inbox.rs` (groups, drawing) and
  `client/screen/check_inbox.rs` (what each item says and its buttons).

## Project-specific: the frame

The layout and checkpoints are summed up in [DESIGN](DESIGN.md). `client/screen/flat_tests.rs` holds the sidebar, the status glyphs and the workspace to the TUI redesign; `client/screen/frame_tests.rs` holds the modal and popover frames.

- **Grid** (`draw.rs` `grid`): inside `ui.padding` (`framed`). The sidebar is `SIDE_DEFAULT` (34) columns, resizable between `SIDE_MIN` (24) and `SIDE_MAX` (60) (a width you dragged wins) and never more than half the screen, the full height, then `ui.gap` columns, then the workspace. Hidden, it leaves `SHOW_CHIP_W` columns for its `»` chip.
- **Chips** (`chips.rs`): every label on a border is a chip, its text with a cell each side on `btn` (`#15253A` in Night): title (accent with `acc_ink` when its frame has the keys, else `strong` bold), action (accent text) and quiet (`muted`). The ends follow `ui.rounded`.
- **Pane** (`workspace.rs` `draw_pane`): every pane is its own box, round corners by default (`ui.rounded = "corners"`). On its top border, at the left, the title chip: the status mark and the name bold (`acc_ink` on the accent while it has the keys, `strong` on `btn` otherwise), and at the right ` × `, which closes the pane (`menu_act(Act::End)`: with an undo, asking again for a running agent). On its bottom border, at the left, its folder and then its branch as quiet chips a cell of border apart (`where_segs`), cut with `…` from the right; a click on them opens the pane's Project menu (`ScreenHit::PaneProject`). Files and Changes are not chips: they are in that menu and on their keys. The program starts on the row under the top border.
- **Splits** (`workspace.rs` `draw_main`): up to four panes (`MAX_PANES`), side by side and stacked as the daemon's `Split` says, each a box of its own one column or row apart (`apart`). Split commands (leader `_` up, `-` down, `\` left, `|` right) put the new pane before the focused one for up and left and after it for down and right. Drag the gap between two boxes to resize them; it lights in the accent while pointed at. Exactly one box has the accent: the focused pane's; a pane being closed lights its box red. Zoom pane (`zoom-pane`) fills the view with the focused pane until run again.
- **No pane open** (`draw.rs` `draw_no_sessions`): a dashed box, `No pane open`, an ` n new session ` chip (a shell at once) and `or pick a session on the left`. Closing the last pane leaves it; nothing starts a shell by itself.
- **Focus**: a click on a pane's title or body focuses it. While the sidebar or a side panel has the keys, no pane is lit.
- **Swapping panes** (`input.rs` `Drag::Pane`, `behaviour.rs` `swap_panes`): a press on a split pane's top border starts a drag; let go on another pane of the view and the two trade places. While dragging, the pane under the pointer lights its lines in `accent` with a centred `⇄ swap here` pill, and a `btn` pill with the dragged pane's name follows the pointer.
- **Status bar** (`mode_bar.rs`): the bottom row of the room inside `ui.padding` (`draw.rs` `mode_bar_row`; `framed` is the room above it while the bar is shown), on the app ground (`bg`, no panel of its own), one blank row above it (`BAR_GAP`). Popups and their dimming sit above it (`above_bar`), so it is never covered or dimmed. At the left the mode chip; the session's name and folder are not repeated there, the pane's own title shows them (a line that just happened, `+ name started`, takes the space for a moment). **Its two rows are always kept** (`bar_shown` is always true: the bar and its gap), so opening a menu or key mode never moves the frames. **Blank while keys go to the pane** (`bar_has_words`): key mode shows on the focused frame, which turns the KEYS yellow (`blocked`, `border_of`). The bar speaks for key mode, for anything that holds the keys, for a line it flashes, and while Update or Restart waits. Typing with the bar up (a waiting update): ` PANE ` (the accent, `acc_ink`, bold; a click starts key mode), and at the right `j/k sessions  ^space keys  ? help`, each key bold in the accent and its words `muted`, read from the keymap (`short_key`: `^space`, `tab`, `esc`), so a rebound key shows its new key and an unbound one is left out; a click on `^space keys` starts key mode too. Key mode: ` KEYS ` (`blocked` yellow with its ink; a click opens the key map) and `n new  s split  x close  tab pane  ? help  esc back`, each a click that presses its key, so it does what the key does: the mode stays, unless the key starts a pane. Who has the keys (anything but the panes, whether or not key mode is held: the sidebar, a popup or modal, a side-panel view with the keys, the key map's search; `keys_holder`): a calm chip (`strong` bold on `btn`: ` SIDEBAR `, ` SETTINGS `, ` FILES `, ` INBOX `, ` CHANGES `, ` BRANCH `, ` PALETTE `, ` MENU `, ` UPDATES `, ` USAGE `, ` CONFIRM `, ` NAME `, ` MESSAGE `, …), and at the right its main keys and the way out, keys in lower case: `↑↓ move  enter open  n new  esc back`, `y yes  n no`, `type  enter save  esc cancel`. The keys are the ones that thing's own handler reads in the state it is in (Files' tree, filter, preview or editor; Changes asking y/n; each step of Switch branch; Settings waiting for a key or editing a value; Settings' Check page by its row, while its command is typed, before the project has a checkpoint (a letter starts the command) and with no project open (only Tab and Esc), and the project check likewise; the Inbox asking to merge or throw away; copy mode's search and selection; the key map's second step; a folder picked for a setting, where Esc goes back); add a row there when a mode or view is added (the match is exhaustive). While key mode is held it adds `shortcut mode waits` before the way out. A narrow bar drops hints from the right, the way out last, then Update shortens and goes, then where the keys go; the chip stays. Under anything that has the keys the bar is only words: its chips take no click, and a click on its row does nothing (`only_says`), so it never closes the popup, runs a hint or drops typed text. **Update** or **Restart**, when waiting and the sidebar's foot is out of view, sits at the bar's right end. With `ui.rounded = "full"` each chip on the bar is capped on the bar's ground. In key mode a key with no action shows the toast `Shortcut mode — Esc to type` (`input.rs` `hint_dead_key`), marked with a sky `i` and bar since it is information, not a success: at most once every 6 s, never over a toast still showing, and not kept in the notification history; keys that act are unchanged.
- **Sheet** (`sheet.rs` `split_for_sheet`): the check sheets and both-diffs dock right from 140 columns, the layout's gap from the panes; below 140, or when the panes would keep fewer than 40 columns (`PANES_KEEP`), they take the pane column and Esc brings the panes back. They start at 72 columns.
- **Inbox** (`draw.rs` `grid`, `sheet.rs` `INBOX_W`): a panel 62 columns wide that floats over the workspace from the sidebar's side (the left edge of the pane column, or its right edge when the sidebar is on the right). The panes keep their column; the panel has a fixed width, so `[` `]` and its edge do nothing. Its cards are NEEDS YOU, FAILED CHECKS, COULD NOT RUN, FINISHED and EXITED.
- **Message popover** (`popover.rs` `draw_compose_pop`): the follow-up to an agent, opened from the sidebar row or the leader (`M`). The screen is dimmed except the popover and its row. A full `accent` border (corners by `ui.rounded`), `card` ground, the input on `bg`, no shadow.
- **Toast** (`draw_toast`): one component for every Maitty notice, bottom right, its bottom edge `TOAST_ABOVE_INPUT` (5) rows above the screen's so it sits over the agent's input area. It draws a `line` outline on `bg` with round corners whatever `ui.rounded` says, its left side a heavy `┃` in the notice's colour (error `err`, about a session that needs you `blocked`, about a finished one `done`, otherwise `accent`).
- **Selecting and copying**: a mouse drag or `v`/`V` in copy mode tints the selection sky (`blend(bg, sky, 0.38)`) with `strong` text; `y` copies and closes copy mode; the copied cells then glow accent for `copy::FLASH_FOR` while the toast reads `Copied to clipboard`.

## Project-specific: home screen check counters

The home screen's "while you were away" line counts a session as finished by the Inbox's rule
(`check::finished`: see Inbox check groups; a stale pass or a check never run here counts as
finished), and a check waiting for approval as needing you.
A `✕ N check failed` (`N checks failed`) count follows, in `err`, only when N is above zero.
When the line does not fit the width, the lead-in "while you were away" is dropped so no count
is cut; below 100 columns the lead-in goes too. A project with no check draws the line it always did.
Code lives in `client/screen/splash.rs`.

## Project-specific: splash

Centred, no bottom bar, in the
theme's colours (`Theme::splash()`: Night has its own wordmark, A-and-I, swell and deep colours;
other themes take them from their strongest text and accent). Top to bottom: the wordmark in 5-row
block letters as filled cell backgrounds (never `█`, which leaves seams), two cells per pixel from
110 columns, the first "AI" in the name in the second colour; a blank row; the swell, one rolling
`▁▂▃▄▅▄▃▂` line 4 cells wider than the wordmark on each side, fading into the ground over 6 cells
at each end; a blank row; the tagline from `Cargo.toml` in italic; the "while you were away" counts
when anything happened; the choice cards, 56 columns or the width less 8; the keycap hints; the
version faint at the bottom right, or in its place an available update or a pending restart.

Cards: **Resume where you left off** `r` (only while sessions run; its second row is project ›
session), **New session** `n` and **Open a project…** `o`. The chosen card is `card2` with an
accent `▌` down its edge, a bold strong label and an accent keycap. Arrows or `j` `k`, and a card's
key, choose; Enter opens the chosen one; a click chooses and a click on the chosen card opens it.
Below 30 rows Resume loses its second row and the gaps between and above the cards close.

**The splash** (`client/screen/splash_wide.rs`) is drawn on the plain ground, with no box round
the screen, no chips on its edge, no row of key hints and no version line. The wordmark, swell
and tagline as above; under them two boxes side by side, **START** (64 columns: the three choices
as 3-row highlights with the sub-text and a key chip on the middle row, then a `RECENT PROJECTS`
rule and up to five projects with a machine badge and `⎇ branch`) and **WHILE YOU WERE AWAY**
(72 columns: `●n ✻n ✓n` chips on its top edge, up to five agents, the ones that need you first,
each two rows: glyph, name, machine badge, project, `● you` or its age, then what it asks or does;
a `MACHINES` rule with each machine's badge and dot). The box that has the keys has the accent
border and title chip. Tab switches box, the arrows walk the one that has them, Enter opens: a
choice, a project (the splash goes, the project is unfolded and selected in the sidebar) or a
session. A waiting release is an accent chip at the right of START's top edge at every size (`u`
or a click acts).

It shows less as the terminal gets smaller, and is the splash at every size that holds the START
box (66 columns, 22 rows). Too narrow for both boxes (under 140 columns): START alone, and what
the away box would say is the counts line over it (`● 1 need you   ✻ 2 still working   ✓ 1
finished`), so who needs you is never dropped. Too short for the wordmark over the boxes: the
boxes alone; the wordmark gives way before the counts do. Only a terminal that cannot hold the
START box keeps the cards above.

The intro, at 50 ms a frame: letter k rises from the deep colour to its own from frame 3k over 8
frames (eased out), nothing moving; the swell fades in over 14 frames and rolls a step every 3; the
tagline fades in over frames 10–20. Cards and hints are drawn from the first frame and keys act at
once. `ui.splash_animation` (Settings › Appearance): `cold start` (default: the first start of a
version, or nothing left running), `always`, `never`. Motion off, or a remote session, never plays
it and holds the swell still.

## Project-specific: hiding the sidebar

The sidebar shows or collapses (`App::sidebar`) to a narrow column (`side.rs` `draw_rail`, `RAIL_W` columns): the ` » ` chip on top and one status mark per session, each a click that opens it. Leader `b` (`Action::ToggleSidebar`; Hide sidebar is the same), the `maitty` or `«` chip collapse it; ` » ` brings it back. It slides as it goes and comes.

## Project-specific: the sidebar

(`client/screen/side.rs`.) **Each machine with its sessions under it.** A box `SIDE_DEFAULT` (34) columns wide on the app ground (`bg`), resizable between 24 and 60. Blank rows separate the sessions and follow each machine heading. The daemon's workspaces are not shown: a new shell still starts in one (`screen/fleets.rs`: the pane you are on's workspace, else the machine's first, else a new `Scratch`).

- **Top border**: the ` maitty ` title chip at the left and ` « ` (hide the sidebar) at the right; either hides it. There is no filter field: `/` (Filter sessions, also in the menu) opens a `Filter sessions` modal; it matches sessions by name, project, process, machine and branch, Enter keeps the match open and clears the filter, Esc cancels. The top border also holds the group-by chip (`icons.folder` and `▾`): it opens a `group by` menu with `Priority` and `Folder`, the current one ticked (`saved.group_folders`; Folder lists each machine's sessions under a folder heading and drops the folder from a session's second line; each folder gets a bar down its left side in one of the theme's four pane colours, taken in turn so neighbours differ, the same colour on its folder icon, and a line of `─` before every folder but a machine's first). Add machine… is an item of the sidebar's `menu` chip. The Machines dialog has a button on each row (update or install, reconnect, edit, remove).
- **Machine heading**: a title chip with the machine's name (`This computer` for this one) and its number of sessions, muted, inside it; an accent ` + ` chip that starts a shell on that machine at once (`App::new_ship_on`; a warning, and nothing started, while it is offline), or in its place ` install ` (the app is not there) or ` update ` (another version is) while the machine `needs update`, which runs the install in a popup (see *Machines dialog*); a green `●` at the right while it is up, a dim `┆` while its line is down. A click on a remote heading opens Machines on it; a right-click opens its menu (*Machines dialog*). A blank row parts two machines. A draft chat waiting for its first message is the last row of its machine, drawn as a twin of a session row (an icon square with three bouncing dots, bars for the name and project); the real row takes its place.
- **Folding.** A machine heading and (when grouped by folder) a folder heading each start with an arrow, `▾` open and `▸` folded; a click on the arrow or the name folds it and a second click opens it again. A folded machine or folder hides its sessions (the cursor skips them) and keeps its count; a filter shows every match whatever is folded. The folds are kept in the saved state (`closed`, `m:<machine>` and `f:<machine>/<folder>`). A machine heading's right-click menu stays on the name.
- **New `+` chip.** In the sidebar's top border, left of the group-by chip: a menu with `New session` and `Add remote machine…` (the Machines dialog). There is no `Remote machines` heading: a machine reached over ssh is a heading like `This computer`, and with none added the `+` menu is the way to add one. A blank row parts two machines; none follows a heading.
- **Nesting.** Each level sits two columns in from the one above: a folder under its machine, and a session under its folder (or flush under its machine when not grouped).
- **Splits.** Every session a pane of the current view shows has a bar in the row's left edge: the focused one in the accent, the others quieter, so the panes of a split are all marked in the list.
- **Session row** (one line under a folder heading, two lines when sessions are not grouped; agents first, then shells, each in creation order, so a row never moves when its status changes): the status mark, then the name (your name for it, else the process; a chat goes by its title), bold in `strong` for an agent that is working, needs you, done or failed, plain `text` otherwise; when there is a second line it holds the project, muted; the branch is not shown (the open pane's title has it). No status words anywhere, but one: a session that hit its plan limit (`TermInfo.resume_at` still ahead) says `limited 16:30`, the local time the daemon continues it (`limited Sun 16:30`, with the short weekday, when that is not today), in `blocked` amber after the project (before the pull request), or at the right of a one-line row when the name leaves room, where it takes the pull request's place. Its mark and its place in the list do not change, and a time that has passed shows nothing. The collapsed column does not show it. The session open in the workspace has the `hov` ground and an accent `▎` on both lines; the keyboard's row `sel`. A session whose machine is down is drawn in `idle`.
- **Subagents** (`side::turn_subagents`, from `TermInfo::subagent_activity`, the daemon's record of what the agent's own hooks reported): only those of the turn the agent is in, so none once it is done or idle; a finished one clears 15 seconds after it reports (`FINISHED_LINGER_SECS`), so a long turn does not pile up rows that look stuck. One whose stop never reached the daemon clears when the agent's turn ends and its own list of background work (Claude's `background_tasks`, `Hook::background_reported`) names no helper still running: the agent's list outranks the daemon's count, so a lost stop neither leaves a row nor holds the session as working. Under any other session they are one row, `└` and a mark each; under the open session one row each, `├` / `└`, the mark and the name. A click on one opens its parent's pane: the agent shows its subagents in its own terminal, and the TUI draws no chat of its own.
- **Foot** (the bottom border): at the left the release chip, ` ↑ v0.15.0 ` in the accent when a release is out, ` ↻ restart ` once one is installed and not running, else the version this is, muted; it opens a menu upward (`menu.rs` `release_menu`: Update to …, or Restart onto the new version; Running …; Release notes). At the right ` menu `, the Menu.
- **Row menu** (right-click on a session, or `.` with the cursor on one): see *Menus*. Anywhere else `.` is the Menu.
- **Keys.** ↑ ↓ walk the sessions; → or Enter opens one; `1`–`9` open that row.

**Status glyphs** (`screen/status_mark.rs`): working is the chosen spinner in the accent (Dots, the design's Braille, held 80 ms a frame, Snake 90 ms, Bars 85 ms, other spinners 90 ms; Reduce motion shows the first frame); needs you is a `●` in `blocked` amber, done in `done` green, problem (a failed command, a failed check) in `err` red, idle in `idle` with the name dimmed; `┆` for a machine that cannot be reached. By default the states differ by colour only, as the product owner chose. Settings › Appearance › **Status glyphs** `Unicode` (`ui.glyphs = "unicode"`) gives each its own shape for colour-blind users: `●` needs you, `✓` done, `✕` problem, `○` idle.

## Project-specific: New session

There is no New session dialog. `Action::PickSession` (leader `n`, palette `New session`, the splash's `n`, the empty workspace's ` n new session ` chip) does what **Shell here** does (`Action::ShellHere`: leader `t`, `Alt+N`, `Ctrl+T`): one plain shell at once in the folder you are in, or in the workspace's own when you are not in it, on the machine of the session you are on. A machine's ` + ` chip on its sidebar heading starts one on that machine. Every session this starts is a **shell**: agents start inside it. The Menu has no `Session...` row.

## Project-specific: tray item (desktop)

The desktop app keeps one item in the Windows tray or the macOS menu bar (`useTrayItem.ts`,
`model/traySessions.ts`, the host's `tray.rs`). Its menu lists every session that needs you, finished or exited: **Needs you (N)**, then **Finished (N)**, each session as
`agent: name`, or **Nothing needs you**; then **Show Maitty** and **Quit Maitty**. Choosing a
session brings the window forward on it. Hovering says how many need you. The menu is the
system's, so it takes no theme. Closing the window still closes the app; the item goes with it.

## Project-specific: closing, service and update states

- **Closing a session** (`client/screen/closing.rs`): closing always asks first, by whatever route: the pane's `×`, the pane
  menu, the row menu, leader `x` or Alt+`x`, a middle click on a tab, or putting a chat away. The question is one dialog
  (`menu.rs` `menu_act`; a working agent's says closing stops it). Accept it with Enter, `y`, `x` or Delete, or click
  Close; decline with Esc, `n` or `q`, or click Cancel. Only then does a single session send `Command::CloseSession`; the daemon hides it
  and keeps its process for six seconds (`UNDO_WINDOW`), `Command::UndoClose` brings it back. A banner on the first row of the
  focused pane reads `Closed <name> · Ctrl+Space then Ctrl+Z undo 6s`, counting down; leader `Ctrl+Z` undoes. Closing several
  panes at once (a tab's `✕`) asks in its own dialog and has no undo.
- **Menus** (`menu.rs` `draw_menu`): a box in the accent on the panel ground, no title, one row per item
  with its leader key at the right in the accent when it has one (letters a menu gives items of its own run
  them while it is open but are not shown), the chosen row on `hov`, and the item that ends something last,
  in red, after a line. It opens under its point, or above it with no room below (the sidebar foot's chips).
  **Session menu** (`.` or right-click on a session row): Open and Open in split (not for the session on
  screen), Rename `r`, Copy path and, when they apply, Changes, Snapshots, the check items and queued
  follow-ups; **Stop session** (an agent) or **Close shell** last. **Pane menu**: Rename `r`, Split right `s`,
  Split down `S`, Find, the right-click toggle, Project… (worktrees, snapshots, the check), **Close pane** `x`
  last. **Sidebar menu** (the ` menu ` chip): Filter sessions `/`, Inbox `i`, Usage and cost `u`, Settings `,`,
  Keys `?`; the full Menu is the leader's `.`.
- **Reconnecting** (`client/screen/service.rs`): when the window loses its own daemon without being told to
  quit, it stays. An amber bar on the first row of the pane says
  `┆ Reconnecting to maittyd · sessions still running · try N of 5 · next in Ns · r retry`; tries wait 1, 2, 4, 8 and 15 s
  (`BACKOFF`). Typing goes nowhere; `r` tries now and `Esc` leaves. The status glyphs do not change to `┆` yet.
- **Service down.** After five tries a red box, ` MAITTYD ISN'T RESPONDING `, says sessions are still running and
  gives three things to try: `r` connect again, `s` start the service (`ipc::open_or_spawn_installed`), `maitty doctor`.
  The window still tries by itself every 15 s (`DOWN_RETRY`) and the box says so, so a service that another window or
  an update brought back is found without a key.
- **Service came back as another version** (another copy of the app updated and restarted it): no try can work, so the
  first try that hears the other version ends them (`service_changed_version`). When a newer maitty is installed here it
  opens in this window's place; otherwise the window closes into the *Service update needed* screen below. A connect
  waits at most 3 s for the daemon's pipe (`ipc::CONNECT_WITHIN`), so a try never holds the keys.
- **Service update needed** is `restart_prompt.rs`, shown before the window opens when the running daemon is another
  protocol version: an amber title, SURVIVES (shells and their scrollback), STOPS (agents), `r` restart service or `Esc`
  not now (which leaves the window; a client and daemon that cannot read each other do not talk, so there is no
  read-only mode).
- **Links in the terminal view.** Cmd/Ctrl-click on a file name opens it in the Inspect panel (`findPathLinks`).
  Cmd/Ctrl-click on a web address opens it in the person's default browser through the host's `open_link`
  (`findWebLinks`): an address with its scheme, or a bare site name with a common ending (`app.hubspot.com`), which
  is opened as `https://` and is no longer offered as a file.
- **Service a release behind (desktop)** (`features/updates/ServiceBehindNotice.tsx`, shown by the workspace's
  dialogs when the service is on the app's protocol): an update leaves a running daemon alone while it holds
  sessions, so the window can be newer than the service it talks to, and none of the release's fixes to the service
  apply until it restarts. When the service can be restarted with nothing lost and says no agent is working or
  waiting, and **Install updates when idle** is on, the window restarts it at once and says `Maitty service updated`
  with how many sessions came back (idle agents are resumed by the restore). Otherwise a toast that stays says
  `The Maitty service is still on <version>` with `Restart…`, which opens a dialog saying how many sessions come
  back and how many agent turns stop before anything is restarted. A service that cannot say whether agents are
  working is never restarted unasked, and a restart that fails falls back to the toast. With sessions that are not
  saved the toast says so and offers no restart.
- **After a restart** a line reads `maittyd restarted · N agents stopped · Enter on its row resumes` for 30 seconds
  while any session has `TermInfo.interrupted`; Enter on that row in the sidebar types its resume command.
  Resuming is never automatic.
- **No sessions**: `No sessions running`, `Sessions keep running when you close the app.`, `Ctrl+T new session`.
- **Updates box** (`client/screen/updates.rs`; opened from the release chip's menu, it has no leader key: leader `u` is Usage): above the sidebar footer. Available: `<current> → <version>`, the
  notes under NEW, `d Download · s Skip · l Later`; Downloading: a spinner, the step (`Downloading`, `Checking the download`, `Unpacking`, `Installing`) and a bar of the steps done, `step 2 of 4`
  (the installer reports no byte count, so the bar moves in whole steps and never shows a percentage; `cli::update::step`); Ready: `r Restart now` (the same one step as everywhere else: the server restarts onto the installed version and this window reopens; it asks first when a session is working or waiting for you); Failed: the error and
  `r Retry`. It reads the existing state (`update_available`, `updating`, `restart_needed`) and uses `start_update` and `ask_restart`.

## Project-specific: Machines dialog

Code: `client/screen/machines.rs` (the view, its keys and its drawing), `client/machine_flow.rs` (what a
choice does), `client/machine_config.rs` (where `config.local.toml` is; the `[[machines]]` entry edits are in `maitty-machines`),
`maitty-machines/src/machine_probe.rs` (the look at a machine). Opened by leader `w` or the command palette
(`Machines`); `Mode::Machines`.

- **One input, typed like scp.** `ADD MACHINE`: `Where's the machine? Type it like scp or ssh.`, the
  input (`› user@host`, on `card2`) and under it a parse line, `user` and `host` each a faint label and a
  bold value, a faint `—` while unknown. A host that is a `Host` of `~/.ssh/config` fills its user and host
  name. Under it, `FROM ~/.ssh/config` lists the hosts (read off the UI thread; `loading hosts from
  ~/.ssh/config…`, then `alias  user@hostname`), filtered by the host part of the input; a host that is
  already a machine is not offered. The keys start in the input (the cursor `█`); `↓` (or `Tab`) walks into the list, where the chosen
  host is marked with `⏎ connect`, and `↑` from its first row (or typing) is back in the input.
  Keys: typing, `↑↓`, `Enter` connects (to the typed text, or the chosen host), `Esc` cancels. A
  `:path` after the host is refused for now (opening a folder after connecting is not built).
- **Your machines.** With a machine configured, `YOUR MACHINES` lists them above the input, by name: ` ⇄ build  ci@build` and at the right how its connection stands (`✕ needs update`, in the state's colour). `↑` from the input walks onto the list and `↓` from its last row is back in the input; a click on a remote machine's heading in the sidebar opens the dialog with that machine chosen. A machine whose app is missing has an accent ` u install ` chip left of its state, one on another version ` u update ` (`MachineRow::fix`). Under the chosen one is what to do about it, in full (the fix text, `<name>` filled in). On it: `r` (or Enter) reconnects, `u` installs or updates (the bar says which), `e` edits where it is, `x` (or Delete) removes; `n` and `s` turn toasts and alert sounds from that machine on or off (both are off until you do; under the chosen machine a line says where they stand, "Alerts from build: toasts off · sounds off", and the change is written to its `config.local.toml` entry). A machine that connects with toasts off says so once per run, "Toasts from build are off. Open Machines, pick build and press n to turn them on.", at the connection and never when its agent needs you, so a machine cannot put a toast on the screen unasked. The title is `MACHINES` while the keys are there.
- **Windows.** The dialog says, in a dim line at its top and on Windows only, `Windows: use ssh keys or an
  agent. Password prompts work in the popup but are not remembered.`
- **Adding is one step for a key-based host.** Enter on a host row (or on the typed text) first refuses a
  target ssh would read as an option or that holds whitespace (the loader's own words, shown under the lists;
  nothing runs and nothing is written). The name is the alias, made valid and unique (`build`, then
  `build-2`); when nothing usable is left of the target, a name step asks for one and shows the loader's
  reason inline. The entry is `name` and `target` only, and `target` is the alias, so ssh's own config for
  it (user, port, key, jump host) still applies. Then the machine is looked at, off the UI thread, with `ssh
  <target> maitty --version` that never asks anything (`BatchMode`, a short `ConnectTimeout`, stdin closed;
  `Checking…`, `Esc` ends ssh and adds nothing, and a limit of 12 s ends it whatever ssh does).
  - *It answers:* the entry is appended to `config.local.toml` (comments and the rest of the file kept,
    written atomically, never `config.toml`), the machine's supervisor starts, and its group is in the
    sidebar as `connecting` at once.
  - *ssh needs a person* (a password, a code, an unknown host key): the entry is written, and ssh is run
    **once**, interactively, in a popup pane on this computer's daemon (`Command::Popup`), so the prompts can
    be answered. The popup line is the probe's without `BatchMode`, with the same hardening (`ForwardAgent=no`,
    `-a`, `ConnectTimeout`, `--` before the target). When the popup pane closes the machine is dialled with
    `BatchMode` as every connection is; whether the login worked is the connection's to say, and if it did
    not the machine shows `needs login` with the fix. A popup has the keys only in plain typing mode, so
    the dialog closes when it asks for the popup and a toast says `ssh needs you to log in: use the popup`;
    the keys then go to ssh. A popup that is never seen (it ended between two reports of the daemon) is
    taken as closed after 20 s. A `remote_cmd` that needs quoting, or an `ssh_args` word that holds a quote, cannot
    be put on a pane's command line safely: the machine is added and the dialog says to run `ssh` once in a
    terminal.
  - *The app is missing, old or not running there:* the machine is added; its state shows `needs update`
    or `unreachable` with the exact fix text. A `needs update` toast names the click rather than the command
    (`build has no Maitty: click install on its sidebar heading, or press u on it in Machines`; `runs another
    version of Maitty` and `update` for an old one; `AppFix::notice`).
  - *ssh cannot reach it* (a bad host): ssh's last line, cleaned of control characters and cut, is shown
    under the lists, and nothing is added. Nothing hangs: every look ends at its limit, on `Esc`, or when
    ssh does.
  A Windows remote is not supported in v1; the dialog does not guess one from the probe.
- **Checklist.** After Enter the dialog shows the steps connecting takes: `reach host`, `verify host
  key`, `sign in as user`, `maitty-daemon`, with `✓` done, a spinner while running, `●` (amber) waiting on you,
  `○` pending and `✕` failed. While ssh is being asked, the first step runs. When the machine is written the
  title becomes `MACHINE ADDED` and the steps show how it stands: all `✓` with the daemon's version; `●` at
  the sign-in with `log in in the popup`; or `✕` at `maitty-daemon` with `the connection shows the fix`.
  `Enter` or `Esc` closes. The host key, password and daemon install are still asked by ssh in its popup,
  not inline.
- **A machine heading's right-click menu** (`App::menu_for_machine`): **Install Maitty** or **Update Maitty** first while the app
  there is missing or on another version, **New shell here**, and for a remote machine **Reconnect** and **Machines…** (the dialog with it
  chosen, where Edit and Remove are). The sidebar's ` menu ` chip lists `Install Maitty on <name>` / `Update Maitty on <name>` first for
  each machine that needs it. Reconnect dials it now and clears a refusal; a machine
  with no supervisor gets one. Install and Update (the menu items, the heading's chip, `u` in the dialog) run
  `maitty machine update <name> --hold` in a popup pane on this computer, with no `--install` or `--yes`: it says what it found and asks
  `Install …?` or `Replace …?` there, and that answer is the consent, so nobody types a flag. A newer version there is refused, never
  downgraded, and a Windows machine is refused before anything is sent. `--hold` (hidden from `--help`) prints how it ended (installed,
  already up to date, not confirmed, refused, could not reach it) and waits for Enter, because the popup closes when the command ends. The
  toast says `Installing Maitty on <name>` or `Updating Maitty on <name>`, and the machine is dialled again when the popup closes, so its
  heading shows how it stands. Edit asks `Where is build? Type it like scp or ssh.` with the address it has now in the field: Enter writes the new `target` into its entry in `config.local.toml` (its name, its other keys and the rest of the file kept), lets the old connection go, forgets the daemon id recorded for the name and dials the new address at once; a target the loader refuses is said under the field and nothing is written. The name is not edited: its saved layout is kept under it. Remove asks first (`Remove machine`, with the address and "Its sessions keep running on it, and you can add it again"; Enter removes, Esc keeps it; in the Machines dialog, `x` asks the same on its own step), then says `Removes build from this list. Its sessions keep running on
  build.` in a toast. It deletes the entry from `config.local.toml`, stops the supervisor and the connection, takes the machine out
  of the sidebar and the window's records, and forgets the daemon id recorded for the name. The machine, its
  daemon, its sessions and its saved layout (kept under its name) are not touched.
- **A refused machine** (a different daemon answers at its name) shows `refused` with the reason and the
  fix: remove it and add it again, or fix the host name or DNS and reconnect.
- Adding and removing take effect in the running window. Editing the file by hand still takes effect at the
  next start.

## Project-specific: Hairline

Not applicable: the Hairline look was removed; Floating is the only look.

## Project-specific: features that need a pane's files

Files, Changes, Find, Branches, References, connecting an agent, worktree
actions and the TUI's Resume a conversation here read or change the files of the pane they apply to. (Pasting an image is not on this list in
the desktop app: it works for panes on other machines, see below. The terminal app's own list is
unchanged.) For a pane on another machine they do
nothing but show one error toast, always in this form: "<Feature> is local-only in v1: the files
are on <machine>, not on this computer." The check is per pane: moving the focus or the sidebar
selection between a local and a remote pane turns the feature off and on. Menu entries that would
run git in a remote pane's folder (Changes, Snapshots, Make it a git repo) are left out of that
pane's menu instead.

**Dropping files on a pane** (`features/terminal/dropRoute.ts`, `useFileSends.tsx`, host `file_send.rs`,
ADR 0007). On a pane of this computer the dropped paths are typed into it, quoted for its shell. On a
pane of another machine a path here names nothing there, so the files are sent to that machine over its ssh
line (regular files only, 100 MB each, at most 10 at a time; more are refused whole with one error toast,
never cut) and the path each one is kept at is typed into the pane as it lands. A machine that is not
`connected` is refused with one error toast. The send runs on the host's background worker, one at a time. A
progress toast (`Sending a.log to build (2 of 3)`, a named progress bar with the percentage, and a
**Cancel** button) stays until the host reports the end; it has no close button, because hiding it would not
stop the send. It ends as `Sent 3 files to build`, an error toast with what ssh said (the files after the
failed one are not tried), or `Stopped sending to build`. Cancel cuts the file now going and tries no other.
What the machine reports is untrusted: the window types a path only if it is absolute, at most 4096
characters and free of control and invisible characters, and a path that is not typed is named in the final
toast instead.

**Pasting an image into a pane** (`features/terminal/clipboardImage.ts`, `useImagePaste.ts`, `api/imagePaste.ts`,
host `image_paste.rs`, ADR 0007 and the amendment to 0055). Ctrl+Shift+V on Windows and Linux, or Cmd+V on macOS, pastes through the
webview's paste event, whose clipboard holds text, an image or both. **Text comes first**: any text and the
terminal pastes it as it always has (bracketed paste included); only an image with no text becomes an image
paste. On Linux the webview hands a pasted image over as an empty paste, so an empty paste asks the host for the
clipboard's picture ([tauri-clipboard](stack/tauri-clipboard.md)). Plain Ctrl+V on Windows and Linux is never taken: like every plain Ctrl+letter there, it goes to the program
in the pane as a raw `^V` (so Claude Code's own clipboard handling and vim's `^V` work). The chat composer takes any
paste chord, since its paste event is not behind a terminal. PNG, JPEG, GIF and WebP are taken (the host checks the
first bytes, not the name the webview gives); another image type is refused with one error toast. At most 25 MB,
refused before the image is read. On a pane of this computer the host keeps the image in its private cache
(`pastes` in the app's cache folder, mode 600, cleared after a week) and the path is typed into the pane, quoted
for the shell. On a pane of another machine the image is sent as a dropped file is, by the same runner and events:
the same progress toast (`Sending the image to build`, progress bar, **Cancel**), the same error toasts, and the
far-side path typed only if it is absolute, at most 4096 characters and free of control and invisible characters.
A machine that is not `connected`, one reached by a custom transport (`via`), or an unknown one is refused with one
error toast, in the words used for dropped files. The image never goes through an event and is never logged: it is
the raw body of the command.

## Project-specific: frame presentation

Wrap each outer terminal frame in synchronized-update markers; end the update
even when drawing fails. An unfinished child update from the previous frame
must not delay switching to another session.

Keep pane padding and the card interior on the same surface as a terminal app
when at least three screen corners use one explicit background color. Preserve
the pane insets; isolated colored panels keep their existing theme treatment.
Preserve colors applications paint explicitly.

## Project-specific: desktop chat view

The desktop app's chat view is a second view of a Claude Code or Codex session, never a second session
([chat view plan](features/chat-gui/chat-view-plan.md), ADR 0006). Rules:

- **Source.** It renders only the daemon's structured events, through `features/chat/` (the projection).
  It never reads terminal output. A session has a chat only when `conversationsFromSnapshot` found a daemon
  conversation whose native terminal is that pane (`Conversation.chat`); `model/chatView.ts` decides
  availability: `ready`, `no-events`, `remote` (local-only in v1), `unsupported` (another agent) or `shell`.
  The daemon gives a Claude, Codex or Gemini CLI agent started in a pane a **read-only** conversation that follows the agent's
  own transcript (Gemini's `~/.gemini/tmp/<project>/chats/session-….jsonl`, named by the hook's `transcript_path`; it has no structured runtime, so "New agent chat" starts it in a terminal shown as its chat) ([ADR 0003](adr/0003-status-and-conversation-trust.md)); its link has
  `readOnly: true`, and the conversation is not listed as a session of its own (the pane is the session).
- **A chat session is the other kind of session** ([ADR 0003](adr/0003-status-and-conversation-trust.md)):
  a structured conversation the daemon started (`Conversation.execution` is `Structured`, `chat` is set without
  `readOnly`, `native_term` is none; `model/chatSession.ts` `isChatSession`). It is a sidebar row, tab and pane of its
  own, grouped by its machine like any session. It has **no terminal**: the pane is the chat
  always, there is no Terminal | Chat switch, `chat-toggle` does nothing, the terminal slot holds a one-line note
  (`NoTerminalNote`), and nothing that needs a pane number (find, restart, paste, broadcast, typed answers, the
  worktree menu, Inspect) is offered for it. Its composer is on (send, queue with Remove, Stop, attachments); an
  approval is answered by request id (y / a / n) and a user-input request by a form (`UserInputForm`), so neither
  points to a terminal.
  **A new session** (the new chat page) starts the chosen
  agent (Settings › Chat › Chat view › **Default agent**, Claude, Codex or Gemini, unless the page picks another) in a new
  shell the daemon owns and types the agent's command into it, and opens the pane in the view **New sessions open as**
  names (Chat by default). It is
  one session either way: **the switch between Terminal and Chat never restarts the agent**
  ([ADR 0003](adr/0003-status-and-conversation-trust.md), v18 B.3). The chat reads the
  agent's own transcript (ADR 0003), and its composer **types into the agent's terminal** as one bracketed paste and
  Enter, only while the pane's trusted status says the agent is idle or done; while it works the composer is off and
  **Stop** sends Esc, and while it asks something in the terminal the chat shows a card, built from the agent's own
  `PermissionRequest` hook and never the screen: "<agent> wants to use <tool>" and one line of what it acts on. For
  Claude the card has **Approve once** (sends `1`) and **Deny** (sends Esc); for an agent whose answer keys are not
  verified (Codex), or a menu the chat has no buttons for (the Claude autocompact size, a setting to confirm), it shows the question as the agent's terminal draws it (`TerminalPrompt`, read again every second and after each key) and a row of **↑ ↓ ← → Enter Esc**, which type those keys into the agent's terminal, so the person answers from the chat as they would there. The keys are the person's own and change no status (ADR 0003); on another machine the screen cannot be read, so the card says the question is in the terminal and keeps the keys. The daemon sends a screen as what a terminal needs to draw it, so the text is read back as plain lines first (`linesOfScreen`): no colour or cursor code is ever shown. What is shown is the menu under the rule that opens it, looked for under the last two rules only (a rule Claude draws inside its question menu, above the last option, does not start it), or the last 14 lines under the last rule, each cut at 140 characters, when no menu is found. A menu is one with a selected option: a `❯` line that is numbered or has another choice right beside it, so a message the person sent, which Claude also marks `❯`, is not one. The options start at the unbroken block that holds the selected one, so a numbered list in the question stays with the question. Of a menu, the last 12 lines above the options and the first 60 lines of options are shown, uncut in length. What is asked stays in view, itself no taller than 220px; the options scroll under it at 220px, with the selected one marked and kept in sight. The card is the message box's question tray
  ("Yes, once" sends `1`, "No" sends Esc). A question Claude asks (`AskUserQuestion`) with one answer has a button
  for each choice, which sends its number. A prompt of several such questions (`PermissionAsk.questions`) shows each
  question with its choices to pick from and one **Send answers** button, off until every question has an answer;
  it then types the number of each pick in the prompt's order, a quarter second apart, and `1`, which is "Submit
  answers" on the page that reviews them (checked against Claude Code 2.1.322 in a terminal). A question that takes
  several answers, or the prompt's own "Type something", is still answered in the terminal. The box stays off while
  the tray shows, because typed text would answer that prompt. A message the daemon types for an agent (the
  first message of a new chat, or a queued one) is its text and then Enter a tick later; when the agent was still
  starting or the machine was busy the two can arrive together, and the agent reads the Enter as a new line and leaves
  the message unsent in its box. An agent with hooks reports a prompt the moment it is submitted, so when no report
  comes within three seconds the daemon presses Enter once more (`Term::enter_again_due`), never when something was
  typed into the pane since. Attachments are off for it. **New terminal** (the sidebar button, Ctrl/Cmd+Shift+T,
  a project's **New terminal here**, the palette) is a plain shell with no chat view. A chat session of ADR 0003, with
  no terminal, still starts from a shell's empty chat card (**Start a Claude chat** / **Start a Codex chat**) and
  stops the agent when closed (`closeChatSession`: end, then put away; if the stop fails it stays open).
- **Choosing the agent for one session.** A small chevron beside the sidebar's + opens **New Claude chat** / **New Codex chat**
  (the default is marked); the + still starts the default from Settings. It starts the chosen agent in the start folder and
  opens in the view Settings names. Beside a session on another machine it opens the new chat page on that machine, as + does.
- **Agent, model and effort.** A button names the agent's mark and the model (the one picked, else the one the agent's
  own file says, else just the agent) with a four-bar meter for the effort, and opens one small menu (`AgentPicker`, 248px,
  one column): the agent switch, only before a chat starts (after that its agent is already running); the models, a row
  each with its description as the tooltip, a tick on the picked one and a dim "default" on the model the agent's settings
  name; then **Effort** as four rising bars (Low, Medium, High, Max) with the level's name beside them. Earlier models wait
  under a **More models** row, so the newest lead; the picked one shows wherever it is. Picking leaves the menu open; Esc
  or a click outside closes it. The default model (the one the settings name, else the one Claude Code says it
  would pick) stands for "nothing picked": there is no separate Default row then, and picking it sends no model. Where
  no default is known (and for Codex), a Default row stays, because which model it means cannot be read. Effort starts as "Agent's own" with no bar filled, since the level the
  agent uses cannot be read; pressing the picked bar again goes back to it. A level a model does not take is dimmed with
  "not on this model": the levels come from the agent's own list for each model (for Codex's Default row, whose model cannot be read, Max
  is left out). Codex's default is the top-level `model` of its `config.toml`, where its list has that model.
  Picking a model that does not take the picked level drops to Medium, or to the agent's own where it takes none.
  Claude's models are asked of Claude Code itself (`agent_models`, `claude_models.rs`): the app starts it once, reads the
  model list from its start-up reply and stops it, so every model the account can run is there under Claude Code's own
  names. Where it cannot be asked, its files are read instead (the account's extra models, the ones it has settings for
  and the one its settings name) and added to the fixed aliases. The list is this computer's, also for a chat on
  another machine. The choice is kept per conversation in this window's storage
  (`agentChoice.ts`) and goes to the daemon with every message (`send_conversation`'s `model` and `effort`); what was
  not picked sends nothing, so the chat keeps what it started with and the button never claims a model the chat is not
  using.
  Claude offers Sonnet, Opus and Haiku (`KNOWN_MODELS`, a fixed list because the daemon does not report models);
  Codex offers only its account default, because its models depend on the account and version. The agent never changes.
- **Switching changes the window, not the session.** The choice is per session (`usePaneViews`), the terminal stays
  mounted and its pane keeps its size, and no daemon command is sent. The shortcut is `chat-toggle`
  (Ctrl+Shift+E, Cmd+Shift+E) on the focused pane.
- **Keyboard.** While the chat covers a pane its terminal is not focused (`TerminalView focused` is false) and is
  hidden from Tab and screen readers. The chat adds no layer to the Esc ladder.
- **Selecting.** In a terminal pane, releasing the mouse on a selection copies it and a "Copied" toast says so
  (xterm's own selection, `TerminalView.tsx`). In the chat nothing is copied by itself: a small bar appears where the
  mouse let go (`SelectionBar`, `selectionActions.ts`; never for the message box, and only for a selection that
  started in that chat) with **Copy**, which copies with the same toast, and **Quote**, which adds the text to the
  message box as a Markdown quote under what is already written and puts the caret after it. The bar goes when the
  selection does, on Esc, and when the chat scrolls. A program that handles the mouse itself (a full-screen agent) copies what was
  selected in it with OSC 52; the daemon passes that on as `Clipboard` and the window puts it on the clipboard with the
  same toast (`useMachines.ts`), for this computer and for a machine that was allowed to.
- **A program's "bright black" background** is drawn as the hovered row's ground (`terminalSlab`), not the palette's
  bright black, which is dim text and as a background is a glaring bar. xterm has one colour per slot, so
  `brightBlackGround.ts` rewrites `100` and `48;5;8` to a true colour on the way in. The TUI does the same when it draws.
- **History.** `useChatFeed` follows one conversation while its chat is open (`chatFeed.ts` is the pure core). The
  first read pages the whole history; later reads happen when the daemon says the conversation changed
  (`conversation-changed`, at least 100 ms apart) and on a poll (300 ms while the agent works or waits on a person,
  1 s once the daemon has announced a change for this chat, 3 s otherwise, none while the window is hidden). Each
  reads from the daemon's `resume` cursor, which returns only the entries added or
  changed since, so a 1,000-entry conversation costs one small read per poll. A page that says `reset` (the history was
  rebuilt, or the conversation started over) replaces what the feed held. A failed read, or a daemon that predates
  `resume`, falls back to the old way: ask again for the last page read, or read from the start when that cursor is
  stale. Entries already applied unchanged are skipped, so nothing is shown twice and unchanged events stay the same
  objects. A new generation shows a fresh projection. Pending approvals come with every page and go to `applyApprovals`. The last
  feed per conversation is kept (8 at most), so reopening a chat shows it at once.
- **List.** `VirtualList` draws only the rows near the view (`transform` placement, measured heights, stable event
  ids as keys), follows the newest row until the reader moves up (any move up, however small, and a wheel move up at once),
  then offers "Jump to the latest message"; it follows again when the reader scrolls back down to within 48 px of the
  bottom (`followAfterScroll`). Rows
  are `buildRows(events)`: one agent message per turn (adjacent agent messages of a turn are joined; a step between
  two messages keeps their order), user bubbles, step cards, approvals, turn dividers, and the working line last.
  Two or more steps that follow each other (edits, commands, tools; never the to-do list) fold into one **work row**
  (`workSummary.ts`, `WorkCard`): a line such as "Read 2 files, edited 3, ran 1 command" with the daemon's own time for
  the run, which opens to the steps. Under the line is a stack of the run's latest commands and edits, three at most
  (`workStack.ts`), with "+N more" on the line when it leaves steps out; reads and searches get no card. The stack is a
  second place to click, not a second Tab stop. A run with a failed step starts open.
  Settings › Chat › Chat view › **Hide successful tool calls** (`hideSuccessfulTools`, off by default; a setting saved
  before it reads as off) leaves out a run, or a lone step, made only of reads and searches that completed
  (`isQuietRun`: Claude's Read, Grep, Glob, LS, NotebookRead, WebSearch, WebFetch and ToolSearch, matched on the whole
  name; Codex reads and searches through commands). A command, any other tool (every MCP tool), an edit, a
  written document, a step running, failed or stopped, and the to-do list always show; so do the turn's divider and
  changes row. A run shows while it runs and goes when it finishes. A message the hidden run stood between reads as
  one, and an approval about a hidden step links to no row. The summary a new chat starts from is built as if
  nothing were hidden.
  Under the divider of a turn that edited files is a **changes row**: "3 changed files +111 −0" (each file once; the
  lines only when every diff was kept), which opens the side panel on Changes with what that turn changed: the diff
  from the last snapshot taken by the turn's start to the last one taken by its end, a snapshot counting up to 3 s
  late (`inspect/turnRange.ts`; the snapshot list is read again for each turn). The range covers the checkout, not
  the agent: hand edits, or another agent in the same checkout, during the turn are in it. "This turn's changes"
  heads the files with **All changes** to go back to the working tree, and no commit is offered beside it. With no
  snapshot from before the turn, none newer than that by its end (a row only shows for a turn that edited files, so
  that one is missing, not "nothing changed"), or a turn without the daemon's times, it shows the working tree as
  before under "Whole working tree · <why>". The row focuses its own pane and shows that pane's session (a helper agent's chat open there stays open); moving
  to another session, closing the panel, and the other ways in show the working tree. A session on another machine
  gets the local-only refusal, as before.
  A message the person sent has a **⋯ menu** beside its bubble (shown on hover or focus) with **Edit from here**
  (`MessageMenu`, `useEditFromHere.ts`). It finds the snapshot taken before that message's turn
  (`snapshotBefore` in `inspect/turnRange.ts`, by the daemon's turn times), asks the daemon how many files a restore
  would change against the files as they are now (`RepoRequest::RestoreCount`), and asks first: "N files change back
  … What is there now is kept as a "before a restore" snapshot", Cancel (which has the focus, so an Enter typed as it opens does nothing) or **Restore N files** (danger). Nothing is
  restored without that yes. A turn without the daemon's times, no snapshot from before it, or a count that cannot be
  read gives a toast saying why and offers no restore. After the restore the message goes in the box, ahead of
  anything already typed there (a draft is kept, under it, and the confirm says so); nothing is sent. The next
  message sent from that chat starts with one line telling the agent the files were restored to before that message
  (`restoreNote`; the quoted words cut by character, with no control or format characters); the agent's own conversation is not rewound. A line under the box says the note will go, with **Leave it out**. A bubble, a later Edit from here and the quote leave that note out (`stripRestoreNote`).
  The note goes when the box is emptied or the chat is left; a slash command or `!` line goes out without it and
  leaves it for the next message. The item is greyed, with the reason in the menu, while the agent works (the daemon
  refuses then too, for a terminal agent and for a chat working in the same checkout) or the daemon is offline; a session on another machine gets the local-only refusal.
  It is also greyed, "Another session is using this folder", while another open session on the same machine runs
  anything but a plain shell in the same checkout, idle or not, since a restore would undo its files too
  (`sharedCheckoutRefusal` in `workspace/model/sharedCheckout.ts`). Same checkout means the same top folder (the
  daemon's `top`, the pane's main checkout or linked worktree), compared with either separator and without a `\\?\` prefix, and without case only for a
  Windows drive path; a
  session in a subfolder shares it, another linked worktree of the same repository does not. A chat the daemon gives no
  checkout for counts when its folder is inside this one. The session's own terminals and closed sessions do not
  count. The check is the window's; the daemon holds no lock. A refusal that arrives while the confirm is open makes
  the confirm restore nothing and say why.
- **Cards** (`features/chat/components/`). Every fact the history carries is shown, and a missing one is said to be
  unavailable (a diff the history did not keep, a checklist without items, a command time or exit code the provider
  does not report, a turn without the daemon's times shows "Turn finished", not an invented duration). Only a step
  that is running animates (its spinner).
  - *Step row* (`cardParts.tsx` `StepRow`): the mark, a verb, what the step acted on, and how it ended. Every step
    is one; a step with detail is a button that opens it.
  - *Tool step:* the verb and target are read from the tool's own input (`toolFacts.ts`): "Read `src/a.ts` lines
    40–88", "Search `pattern` in src", "Fetch example.com/docs", "Skill kitchen:kitchen" and its arguments, an extension's server
    and tool. Nothing the input does not hold (hit counts, a status code) is shown. Click opens the input and the
    result, bounded.
  - *Edit step:* the file as a badge, Added / Modified / Deleted where the history says, `+N` `−N`; it opens to the
    diff, folded past 12 lines with "Show all N lines" / "Show less", and "Open in side panel" (the Inspect panel's
    Changes view). A diff too large, one the history did not keep and a secret file each say so. A finished Write of a
    Markdown file whose text the history kept is a document card instead (`writtenDocument.ts`).
  - *Command step:* the command's first line, then the live clock or `exit N · time`; it opens to the whole command
    and the last 3 lines of output with "Show all N lines". The elapsed time is the provider's duration, else the
    daemon's start and completion times; the exit code exists for Codex only, so for Claude it reads "exit code not
    reported". One that failed starts open.
  - *Approval card:* see Approvals. *Helper agents:* consecutive helper calls are one card, "3 agents, 2 working",
    a row each (title, type, what it was asked, its time); a row opens what it was asked and its report. *Plan:*
    the agent's own to-do items, "N of M done" and a progress bar. *Working line:* "claude is working · m:ss"; no token count,
    because the daemon's history carries none.
  - *Markdown* (`markdown.ts`, `components/Markdown.tsx`): an agent message is read into a data tree and drawn as
    elements. There is no HTML anywhere (no `dangerouslySetInnerHTML`), raw tags are text, images are never loaded
    (their alt text shows), and a link is kept only for an `http(s)` address. A link is a button, never an anchor, so it
    never navigates the webview: a click hands the address to the host (`open_link`), which opens it in the person's
    default browser, and the toast that says so offers **Copy link**. Outside the desktop app a click copies it.
  - *Saving a block* (`exportBlock.ts`, `saveBlock.ts`): a code block has **Copy** and **Save**; a table has **Copy**
    (as Markdown) and **Markdown**, **CSV** and **HTML**. A save goes to the export folder of Settings › General
    (Downloads until one is picked with the system's folder picker) under "what date time.ext", and a toast says where.
    A code block saves as `.md`, `.html`, `.csv` or `.json` when that is its language and as `.txt` otherwise. A CSV
    cell that a spreadsheet would run as a formula is written with a leading `'`; an HTML table's cells are escaped text.
    A table (a header line, a `| --- |` delimiter row of the same width, then rows, with `:` for alignment) is a
    real `<table>` drawn as a hairline grid in existing tokens, inside its own named, focusable, sideways-scrolling
    region, so a wide table never widens the message; cells hold inline markdown, and a right-aligned column stays on
    one line.
    A reply still arriving (`running`) is read in its finished form (`streamingText`): an opened bold, italic or code
    span, or a link whose address is still arriving, reads as if it closed; a mark with nothing after it yet and a
    last line that is only a list, quote or heading marker are left out; a table waits for its delimiter row; an open
    code fence is untouched. A finished reply is read as written.
    Supported: paragraphs, headings, fenced code, lists, quotes, rules, inline code, bold, italic.
  - *Rows* are memoised on their events (`ChatItem`), and the virtual list measures heights, so a card that expands
    resizes its own row only.
- **While it waits.** The chat never looks stuck. First read of a history: `ChatSkeleton`, the shape of a conversation,
  static (an endless animation costs a repaint every frame, PERFORMANCE). A new session's chat is `StartingChat`, there
  at once with its message box; what is typed then is held per session (`firstMessage.ts`) and sent when the real chat can
  send, and a half-typed draft carries over. The model, effort and mode picked on the new-chat page are held the same way
  (`holdStartChoice`): the starting chat draws them in the same chips the real chat has (locked), and the real chat takes them as
  its conversation's choice, so nothing in the box changes when it takes over. One message is held at a time. **Connect <agent>** on its notice installs the hooks and then restarts that session in place (`reloadAgentOnDaemon`, `Command::ReloadAgent`), so the agent loads them. A message sent into the terminal is drawn at once as an `echo` row, with the
  working line under it, until the history has those words (`countUserMessages` against the count at send time); it is
  dropped after 20 s rather than left standing as sent. The **working line** says what the agent is doing from the newest
  step still running (`workPhase.ts`): "reading src/app.ts", "writing hello.txt", "running npm test", "thinking". It says
  only what the history says.
- **Mode and model.** The settings tray under the message box says how Claude is set up, from its own transcript and
  never a guess: the permission mode (`Ask before edits`, `Accept edits`, `Plan mode`, `Auto`; `Bypass permissions` in the
  danger colour, like Full access), "Permissions" until the file has said one. A Claude chat that follows a terminal agent
  changes both from the tray, by typing the agent's own commands into its terminal: the mode chip is
  a dropdown that presses Shift+Tab until the agent's screen shows the pick (`modeFromScreen`), and is off while the
  agent works; the model and reasoning picker sends `/model` and `/effort`, and a pick made while the agent works is
  typed in at once, as it would be in the terminal, for the agent to take as soon as it can; only a question the agent is asking holds the picker, since typed text would answer it. The agent
  itself is never changed in a chat that has started. Other agents show them read-only. A Codex chat shows none yet (`chatMeta.ts`,
  `SessionMeta` in the read result). Bypass permissions is pickable only when Claude started allowing it: **Settings >
  General > Yolo mode** has one switch per agent, off by default, that adds the agent's own launch flag to sessions started
  afterwards (`yoloFlags`).
- **Composer.** `useChatComposer` sends through the conversation's own send (`send_conversation`) with an operation
  number (the receipt's plus one; a failed send is retried with the same number so the daemon can tell a retry from a
  new message). Enter sends, Shift+Enter adds a line (not while an input method is composing), Esc or Stop interrupts
  the running turn (`cancel_conversation`). It is off, with the reason as its placeholder, while an approval is
  pending ("Answer the request above first") or the daemon is offline. While the daemon reports the turn working,
  Enter still sends: the daemon holds the message (`ReadResult.queued`) and delivers it when the turn ends, in order
  and once each, so it is shown as a dashed pending bubble with remove and Edit (both `dequeue_conversation`; Edit puts the words back in the box only after the daemon has let go of them, so a message sent a moment ago is never sent twice) for as long as the
  daemon holds it, and closing the window loses nothing. The queue is the daemon's memory: a daemon restart drops it,
  and a message queued for a conversation that has since started over is dropped, not sent into the new one. The
  boundary is the end of the turn, the one the daemon can observe for both providers; a message is never delivered
  into a running turn. A send that the daemon refuses as `stale or reused` (another window sent in between) is read
  and sent again under a new number; a retry of any other failure keeps its number.
- **Naming a file with @** (`fileMention.ts`, `useFileMention.ts`, `FilePicker.tsx`). An `@` at the start of the box
  or after a space, with the name typed after it at the end of the text and the caret there with nothing selected,
  opens a list over the box (the command list's
  look) of the session's files whose path holds the name. An `@` inside a word (an email address) opens nothing. The
  daemon finds and ranks them (`RepoRequest::FindFiles`, through `repo_query` and the same pane Inspect asks,
  `repoPaneOf`), asked 120 ms after the typing stops for at most 50; an answer for a name typed earlier is dropped,
  and the client does not re-rank. Only the answer for the name typed now is listed: until it comes the list says
  "Looking for files…" and ↑ ↓, Enter and Tab do nothing (Enter does not send). Moving the caret off the end closes
  the list and gives the keys back to the box. ↑ ↓ move, Enter or Tab replaces the `@` and the name with the path and a space,
  Esc puts the list away until the text changes, a click picks. The path is relative to the session's folder
  (`pathFromFolder`: the daemon answers from the checkout's top, so a session in a subfolder gets `src/x.rs` or
  `../other/y.rs`), and one with whitespace or a `"` is written in double quotes, its own quotes escaped as `\"`. Folders are matched
  without case on a Windows drive path. The message stays plain text. A list of 50 says
  only the best 50 are listed; a daemon that stopped its walk early says some files may be missing. A failed search,
  a daemon older than protocol 89 among them, is one quiet line in the list. For a session on another machine the
  picker reads that machine's folder index instead (`index_remote_files`, the host's ssh index the Get a file dialog
  uses; no daemon question, so no protocol floor): the session's folder is indexed once as the list opens, again at the
  next `@`, and the box ranks the index itself (`fuzzyFind`, the dialog's ranking); the paths are already relative to
  the folder. Until the index comes the list says "Looking for files…" with the keys held as above; a failed index is
  the same quiet line; a machine shown as down says "Machine offline" and asks nothing. It is not offered in the
  browser build or for a session of this computer with no pane to ask.
- **Attachments** (ticket 20). A paperclip left of the text box (tooltip and `aria-label` "Attach files",
  or "Attach files: <reason>" while the composer is off; `aria-disabled`, not `disabled`, so the reason can be read),
  a drop on the composer, or a bare image pasted into the box adds files to the draft; a paste that holds any text
  stays a text paste. The picker is the dialog plugin's `open` (`api/attachments.chooseFilesToAttach`); a drop
  reaches the composer through `chat/composerDrop.ts`, which `DesktopApp`'s one file-drop handler asks before it
  routes the drop to a pane. `useAttachmentIntake` is the only code that adds or removes a draft file: at most 10 per
  message (a drop over the limit is refused whole), a pasted image over 25 MB is refused before it is read, the
  host (`attach_local_file`, `attachment.rs`) checks everything else and returns the cleaned name that is shown.
  Draft files live in `composerStore` beside the text, so switching views loses nothing. Each is a chip (kind icon,
  name, size, a Remove button) or, for a pasted image, a thumbnail made from the bytes the window already holds
  (an object URL, revoked on remove or send). Sending builds the text with `buildAttachmentMessage` (the text, then a
  fixed block naming each kept file by path); a message may be only files. A failed send gives text and files back.
  A sent message is shown by `parseAttachmentMessage` as its text and chips (name, and a thumbnail for an image; "no longer available"
  when `attachments_exist` says the kept file was swept). An image thumbnail, in a draft or a sent message, opens the picture
  full size in a viewer (`KeptImage.tsx`; Escape or a click outside closes it); the host returns a kept image's bytes
  (`attachment_image`, only from its own folders and only for an image) and the window shows them as an object URL. The composer takes a `machine` (the chat link's):
  for another machine the same picker, drop and paste send the file there (`sendAttachments` over `send_files_to_machine` /
  `send_pasted_image`, with the terminal's progress toast and Cancel) and the chip holds the far-side path, checked
  before it is used. No chat sets a machine yet. A read-only chat shows no paperclip, unless it types into the agent's terminal (a chat that follows a terminal agent): the files go in as the block of paths, pasted with the message.
  The new-chat page and a chat that is still starting take files too, keyed in `composerStore` under a negative key
  (`-1` for the page, `draftKeyFor(session)` for a starting chat) since no conversation exists yet; the page puts them in the
  first message, the starting chat holds them with its held message and shows the echo as text and chips.
- **Read-only chat.** For a link with `readOnly` the composer is off with "Read-only: talk to the agent in the
  terminal" as its placeholder, and offers no Stop or queue wording; the daemon refuses send, approve and cancel
  for such a conversation. The history and status still follow the agent.
- **Approvals.** A pending request is the message box's question tray (`QuestionTray.tsx`), not a row of the list; it
  answers through `approve_conversation` by request id, and the request then folds into one line in the list ("Allowed", "Allowed for this session", "Denied"),
  with the buttons gone. While it is pending, `y` (once), `a` (this session) and `n` (no) answer it from anywhere in the
  chat but a field, the same letters as typing into the terminal, which stays a working fallback. The daemon's `answered_approvals` are
  read with every page, so a request answered in the terminal or in another window shows as answered with the real
  choice, even when this chat was opened afterwards. The card names the tool step it is about (`item_id`) and its
  "For: ..." link scrolls the list to that step. A request that asks the person a question (`UserInput`) is answered
  in the terminal.
- **Screen readers.** The history is a labelled region, not a live region. A separate polite status line says only
  that the agent is working, is waiting for an answer, or finished.
- **Styling.** Tailwind utilities on theme tokens only; the keyframes for the view change live in `workspace.css`.

Settings spacing: the sections column is one row each; the rows on the right get a blank row between them when the whole list fits that way, else they sit tight. The sidebar leaves one blank row between a machine heading and its first session, and between machines; sessions sit tight against each other.

Host terminal tab: the client writes the focused session's state mark (the working spinner while it works) and name to the terminal's tab title (`ESC ] 0`, `screen::window_title`, `App::sync_window_title`) after each frame, only when it changed, so the tab says what you are on and whether it needs you.

A line starting with `!` in a Claude or Gemini chat that follows a terminal is typed into that terminal, not pasted: the `!` first, then the command and Enter after a short gap (`typeMessage`, `isBangLine`), so the agent switches to its own shell mode, runs it and keeps the output in its context. The box says it runs in the terminal, where anything it asks (a login code) is answered. A chat with no terminal, and Codex, send it as an ordinary message. A row whose status is unknown (its machine is out of reach, or a list just missed it) fades to half and says `offline` in place of the time.
