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

## Project-specific: design system

A terminal UI (ratatui). Colours come only from the theme (`theme.rs`): semantic
slots `bg`, `card`, `card2`, `hov`, `accent`/`acc_ink`, `text`/`strong`/`muted`,
and status colours (`working`, `blocked` = needs you, `done`, `idle`, `err`).
Every built-in theme passes a WCAG contrast audit test. Shared primitives in
`client/design.rs` and `client/screen/`: `panel()` (popup), `fill`, `put`/`seg`
(text runs), `keycap`/`keycaps`, `hints`, `button`, `dim_all`. Icons come from
config (`[icons]`), with Nerd Font glyphs where present.

## Project-specific: layout and navigation

<!-- local: user requested sidebar-bottom controls, tab-row update discovery and no global footer. -->

Seshi has one terminal interface: a sidebar of projects and sessions, with tabs and split panes in the remaining area. Settings > Appearance > Layout selects Cards or Flat; both use the same sessions, tabs, splits and sheets. Cards is the default. Flat uses square, full-bleed zones and Tab / Shift+Tab to move focus between the sidebar and panes; Esc returns focus to the sidebar. There is no application-wide footer bar and no separate tab row: the tabs are chips in the top-left pane's title (see *v7 frame*). Keep **+ new** and **Actions** at the bottom of the sidebar in both layouts. The top right of row 0 shows **Update** only when a newer release is available.

The leader is `Ctrl+Space` by default. Inbox (`j`) uses the right-hand sheet. Settings (`,`) opens as a centred modal over the workspace so the pane layout does not change. The command palette (`Space`), shortcuts (`?`), files, branches and new agents use popups. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. Prefer Seshi's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll.

The high-fidelity flat-layout reference and its interaction spec are preserved in [`docs/design/flat/README.md`](design/flat/README.md); update it when the implementation changes the design contract.

The sidebar's **+ new** uses the same shell action as leader + `n`, including its configured key hint. **Actions** is in the sidebar footer and opens status setup, Settings and All commands. Every palette row is clickable and wheel-scrollable. Include configured commands and retain existing confirmations.

When an update is available, show **Update <version>** at the top right of row 0 and open the existing confirmation flow when clicked. Do not show the control when Seshi is current. Keep status icons and pane status tags clickable to open the evidence explanation. Terminal bells do not add sidebar indicators or history entries.

Mark one selected sidebar row independently from hover: the keyboard selection stays selected when the mouse moves; hover is a separate, quieter tint. The open session keeps a dim selection bar while another row is under the keyboard. Preserve agent status icons and colors in each state.

Settings opens on General and reports installed, window and daemon versions separately, with a labeled remote daemon and restart-pending notice when they differ. General groups controls under INPUT, LAYOUT, SHELL and UPDATES. Show the update action only when an update is available; keep release notes and confirmation in the existing flow.

Settings > Team, after Check, uses a compact layout. Under TEAM, Family (claude or codex, ←→) is the global `team.family`; This project steps global / claude / codex for the project of the workspace you are in, writing or removing its `[[team.projects]]` entry in place, keeping comments, off the UI thread; a failed save keeps the old value and shows a toast. Under JOBS each job reads name · claude model · codex model, `default` for an empty model, with the model of the family active here in bold; its help line says which agent and model a commander starts it on in this project now. With no jobs the page says "No team set." above the **+ add job** row. Enter on a job edits it in three steps in the inline editor the Check command uses (name, Claude model, Codex model); `a`, Enter on the add row or a click on it starts the same editor on a new job, and Esc cancels with nothing written. Each step is checked with `validate_team` (name characters, a duplicate name ignoring case, a space in a model); a refused value stays in the field with the reason in a toast. The last step saves the job's `[[team.jobs]]` table in place, matched by name, keeping comments and the order of jobs, off the UI thread; a failed save puts the draft back. Del on a job asks in a confirm, then removes it; Cancel returns to the page. Jobs are not reordered here; edit the file for that.

Show reported child agents under their parent, never as independent projects. Keep the sidebar list live: remove a child when its reported state is completed, failed, cancelled or unconfirmed, while retaining lifecycle evidence in daemon state. Make active child rows clickable to inspect reported task and stage details. Keep parent session status colors unchanged. Active subagents use muted neutral gray with no per-helper animation; their compact task or role label is muted, dim and non-bold, with no generic gear icon. Use the provider-reported task when available; otherwise show a reported current tool action, recognizable role stage, meaningful reported role name, or `Working` when none is available. Preserve standard needs-you and error colors for blocked or failed children. Do not show lifecycle group labels or counts beyond the parent heading total. Do not assign role colors that could be confused with done, working, blocked or idle.

Keep update indicators until the installed program, window and daemon versions agree. Installing an update leaves open sessions running. For a local daemon, when no panes are open, restart it automatically and reconnect the window so daemon fixes take effect without `kill-server`. For a remote daemon, explain that it must be updated on its host after its sessions are closed.

When a local daemon is older than the installed seshi, Settings' restart-pending line carries a **Restart now** button (`R` in Settings), and the toast that follows an install shows the same button and stays longer than other toasts. Neither shows for a remote daemon, while a restart is under way, or when the daemon already matches. Choosing it opens a confirm (`menu::Confirm`, danger while an agent is working) that reads "3 sessions stop and resume, 1 is working" with Restart and Cancel; Cancel changes nothing, and with no sessions open it restarts without asking. It goes through the same stop-and-reconnect path as the automatic restart.

A new `seshi` that finds an older or newer server it cannot talk to (`ipc::OtherVersion`) shows a full-screen prompt (`client/restart_prompt.rs`) on an interactive terminal: the server's protocol next to this one's, "Running agents stop and resume when it restarts", and **Restart and open** (Enter) and **Quit** (Esc, Ctrl+C), each clickable. Restart stops the server by its process, as `seshi kill-server` does across versions, then opens normally; Quit prints the usual error and exits 1. The prompt uses the configured theme and pill style. Without a terminal on stdin and stdout, or with `--remote`, there is no prompt: the error text and exit code are unchanged, and a remote server is never restarted this way. The protocol number is all an older server reports, so the prompt cannot name its release.

**All keys** (`<leader> ?`, Actions or palette) opens a searchable, scrollable reference. List every active configured leader/global binding separately, including aliases and custom commands. Show palette-only commands without inventing bindings. Keep the delayed leader overview compact. Contextual keys come from [shortcuts.md](shortcuts.md), embedded in the app so both stay identical. Render each binding as a keycap, separate from its action text; use clear section headings and wrap long actions without making them look like additional keys. On wide screens, balance whole sections into three visibly separated columns; use one column when the reference would otherwise be cramped.

## Project-specific: pane focus

Only the area with the keyboard is lit: the focused pane's whole border (Cards), its title chip and left `│` rule (Flat), or the open sheet. Unfocused panes fade by the Dim setting (30% by default; "40%" in an older config counts as 30%). Do not reserve a full-width bottom row for global hints. The pane has no status line: its branch is in the sidebar's folder header.

## Project-specific: UI conventions

<!-- local: user requested distinct active, keyboard and hover sidebar states, mouse-first sidebar controls and parent-owned child-agent visibility. -->

- Popups are centred `panel()`s (`panel_at()` for a given rectangle) with an
  accent title row and `Esc close`, `│` sides and a `─` bottom in `line`, and a
  hard shadow a cell right and down; the screen behind is dimmed. Small
  explanations (why, a row's menu) are `popover_at()`s: a raised title row, no
  border, the same shadow, beside the sidebar (`popover_rect()`). Settings is a
  centred modal that preserves the workspace layout and keeps its General,
  Sessions, Appearance, Agents and Keys sections, update controls, and keyboard
  and mouse editing available.
- The pointer's tint is one pass after drawing (`render.rs`): the topmost
  clickable region under it is tinted once, `mix(bg, text, 0.10)`. Widgets do
  not tint themselves on hover.
- Selection is a neutral tint (`sel_on` of the surface it sits on) with a lime `▎` bar;
  never the accent colour as a row fill.
- Every popup row is clickable and keyboard-reachable; every key is shown as a
  keycap somewhere (hints line, keys screen, palette).
- Short-lived notes are toasts above the pane content; destructive actions ask
  first (`Confirm`) and go last in menus.
- Anything filled that you click or read as a control (a button, a toggle, a tab, a tag,
  a key) is drawn with `button` / `btn`, `chip` or `pill`, so its ends follow the pill
  setting everywhere. Never a bare `seg` on a button colour.
- Copy is plain words ("Focus the sidebar", not "browse-tree").

## Project-specific: theme browser

Appearance has one Theme control. Enter or clicking its value opens a searchable
browser with palette swatches, an applied marker, a result count, and a preview.
Typing filters by name or flavor; arrows, Home/End, page keys, and the mouse wheel
move the selection. Click a row to preview; click it again or press Enter to
save. Esc or the close button returns to Appearance without changing the theme.
Saving runs in a background task; failure keeps the selection and shows a toast.
The preview shows built-in defaults; configured overrides still apply after saving.
See [theme sources](theme-sources.md) for upstream palettes and local UI adaptations.

## Project-specific: working spinner picker

Appearance's Working spinner opens a mouse/keyboard picker with animated preset
previews, an applied marker, Static and Custom. Preview leaves the active config
unchanged. Enter or a repeated row click applies; Esc/close returns to the same
settings row. Custom accepts space-separated frames, cursor editing, paste and
Ctrl+U. Retain invalid drafts and show inline errors. Save in a background task;
disable duplicate saves and retain the draft on failure. Choices affect only the
working glyph; blocked/done/idle status colors and icons keep their meaning.

## Project-specific: sidebar check result

The sidebar is the v7 handoff's `drawSide` (`docs/design/v7/checks7.js`); its rows are
tested cell for cell against the reference frames (`client/screen/side_frames.txt`).

- **Title:** `SESSIONS` bold and the count, then right-aligned `●n` (`blocked`, bold:
  asking or waiting for check approval) and `✕n` (`err`, bold: failed checks), each only
  above zero.
- **Folder:** fold glyph (`accent`), name (`strong`, bold, never cut before the meta),
  meta right-aligned in `muted`: `⎇ main` plus ` · check: <name>` or ` · N checkpoints`,
  or `no git`. The meta shows only with 6 or more columns free, cut with `…`. A blank row
  follows each folder.
- **Row line 1:** the session's mark (`client/screen/glyphs.rs` `Kind`; bold for failed,
  needs you and approve), the name (`strong` when selected or failed, else `text`, bold;
  working keeps its shimmer), the age right-aligned in `muted`.
- **Row line 2:** the headline check's result (`TermInfo::check()`) by checkpoint name, or
  the question, or what the session is doing. While the agent works or asks, its last
  result is about an older turn and is not shown.
- **Row line 3** (140 columns and up): a failed check's first failing line (`muted`), or
  every checkpoint by name: `build ✓ · test ✕` (a failed name in `err`).

| State | Glyph (Unicode / ASCII) | Colour slot | Line 2 |
| --- | --- | --- | --- |
| working | the spinner you chose | `accent` | `claude · working` |
| needs you | `●` / `!` | `blocked` | `asks: <question>` |
| done, not seen / seen | `✓` / `+`; `○` / `o` | `done`; `muted` | `finished · not seen` / `finished` |
| asleep | `z` | `muted` | `asleep zzZ` |
| checking | `◜◝◞◟` / `- \ \| /` | `sky()` | `checking test · 6s` |
| passed | `✓` / `+` | `done` | `passed: test` (bold) ` · 14.2s · 14:22` (`muted`, local clock) |
| failed | `✕` / `x` | `err` | `failed: test · exit 1` (bold) |
| overridden | `⊘` / `/` | `text`, never green | `failed, overridden: test` (bold) |
| could not run | `?` / `?` | `violet()`, not a failure | `test could not run · not found` |
| stale | `◷` / `~` | `text`, never green | `stale pass: test` (bold) ` · 3 files changed` (`muted`) |
| never for this repo | `?` / `?` | `violet()` | `not checked: never for this repo` |
| waits for approval | `●` / `!` | `blocked` | `approve check: <command>` |

- **Several checkpoints:** line 2 is the headline (worst) checkpoint's word, then one bold
  glyph per checkpoint (pass `✓` `done`, fail `✕` `err`, ring `sky()`, could not run or
  never `?` `violet()`, stale `◷` `muted`, overridden `⊘` `text`, approval `●` `blocked`,
  pending `…` `muted`). Words: `failed: <name>` (`err`), `could not run: <name>`,
  `needs you: <name>` (`blocked`), `checking: <name>` (`sky()`), `next: <name>` (`muted`),
  `stale pass: <name>`, `not checked: <name>`, `overridden: <name>`, `all passed` (`done`).
  The glyphs are clipped, never the word.
- **What gives way:** the folder meta first (by its room), then the age (below 100
  columns), then line 3 (below 140), then line 2 of rows with no check result, question
  or plan-limit wait. Queued follow-ups and subagents keep their lines after these.
- **Selection:** the keyboard's row (while the sidebar has the keys) or the open session
  is tinted (`Theme::sel_bg()`) from X to one past RX on all its lines, with `▎` at X:
  `accent` while the sidebar has the keys, else `muted`; while the keyboard browses, the
  open row keeps a `muted` bar. A failed row carries `▎` in `err` on every line, selected
  or not. X is 2 columns in (1 in Flat), RX 3 from the right (2 in Flat).
- **Foot:** `+ new` at the left and `a actions` right-aligned, on the last row inside.
- Shape and word carry the meaning; colour only backs them, so every state reads in the
  ASCII set and without colour. Glyphs come from `[ui] glyphs` (`unicode`, `ascii`,
  `nerd`, or `auto`: ASCII when the `blocked`, `done` and `idle` icons are all ASCII);
  a Nerd icon is clipped to one cell. Take marks from `check::marks`, never a literal.
- Colours are existing theme slots only; there is no check role. `Theme::violet()` and
  `sky()` are derived, not roles. Code: `client/screen/side.rs` (the card),
  `client/screen/check.rs` (a result's words and marks).

## Project-specific: check evidence sheet

The check evidence sheet follows v7 (`drawEvidence` in [`docs/design/v7/checks7.js`](design/v7/checks7.js)). Enter on a sidebar row whose check failed, could not run, was overridden, went stale or is never run for its repo opens it (a row waiting for approval opens the approval; every other row still focuses its session), as do Output in the Inbox and `⏎` in the why popover. It docks right like the Inbox and takes the pane column below `NARROW`. Its title is `CHECK · <session> › <checkpoint>`; content starts one column into the card's inside, and in Flat the sheet keeps the Cards box (the Inbox too): only the frame changes.

- A result chip (` <glyph> <word> ` bold on the state's colour: `failed` on `err`, `failed · overridden` on `text`, `could not run` and `not checked` on `violet()`, `passed` on `done`, `stale pass` on `btn`), then what it came to in `strong` bold (`exit 1`, `exit 0 on older code`, `never for this repo`; `command not found` in `violet()` for exit 127).
- Command, Folder (the run's folder, home as `~`), Ran (`14:22:05 · took 14.2s · after turn 3 ended 14:21:51`, local clock; `did not run` when it never started) and Seen by (`Seshi ran this itself. The agent said "…".`), or Source (`.seshi/checks.toml · <commit> · you chose never`) for a repo's own command. Rows are clipped at the sheet's edge, not wrapped.
- A banner on a tint for a result that needs explaining: overridden (by whom, when, `u undo`, the reason), stale (how many files changed and when, then one `M path` row per file that fits), never, could not run.
- `OUTPUT`, then `all N lines`, `last N of 1,284 lines` (numbered as the whole output, from `CheckInfo.line_count`; `f full output` when the daemon kept only the tail) or `empty`, on a darker block with 5-column line numbers; `✓` lines `done`, failure lines `err`, `▲` when there is more above. Empty output says why: `(no output)`, `(the command printed nothing)` / `exit 0 is what passed it.`, or `(nothing ran)`.
- Buttons on the third row from the bottom, each a click of its key: `Check settings ,` for never, else `Re-run r` (primary when stale, absent while it runs), `Override… O` (failed only), `Undo override u` (overridden only), and `Open session o` when it fits. The last row is always `Esc back   r re-run   f full output   i why`.
- Keys: Esc closes; `r`, `O`, `u` act as the buttons; `f` and the arrows open all of the output and scroll it; `o` goes to the session; `i` opens the why popover; `,` opens `PROJECT · <name>` for the session's project. With several checkpoints it opens on the headline one, and Tab / Shift+Tab move to the next / previous; the title names the one shown, its output is fetched by name, and `r`, `O` and `u` act on it.

The row menu of a session with a check adds Check output (any result), Re-run check (not while it runs), Override check… (failed only) and Undo override (overridden only); letters come from `menu_keys` like every item. Output comes from `Query::CheckOutput` and is fetched again when a new run replaces the result. Command output is not trusted: colour codes and control characters are stripped before drawing.

## Project-specific: why popover

The evidence popover (`i`, or a click on a status) explains a session's status from its hook, screen and process reports. For a session with a check it follows v7 (`drawWhy`): the title ` WHY · <session> reads <word> ` (the session's word: `failed`, `done · stale`, `not checked`, …), then one line each with its local clock time: `agent's turn ended · it said "<said>"`, `Seshi ran <command> in its worktree`, and what came of it in the state's colour, bold: `✕ exit 1 after 14.2s` for a pass or failure, the row's words for the rest (`⊘ failed, overridden: test`, `◷ stale pass: test · 3 files changed`, `? not checked: never for this repo`, `● approve check: npm test`). A line whose time is unknown leaves the time blank. Its hints list only the keys that act, in v7's style: `⏎ evidence` always, `r re-run` once there is a result, `O override` for a failed one, `Esc close`. A key the popover does not show closes it and is not passed to the pane. A session without a check keeps the source rows. The frame and placement are the v7 frame's.

## Project-specific: check modals and settings

Deciding is a modal; nothing here pops up on its own. Each matches the v7 handoff (`checks7.js` `drawModal`) inside the modal frame, cell for cell, in every layout at 160×45, 100×30 and 80×24 (`client/screen/check_modals_tests.rs`). Colours are existing slots: `blocked` for "needs you" amber, `done`, `err`, `violet()`, `sky()`, `card2` for fields.

**Override** (`client/screen/check_actions.rs`): `OVERRIDE A FAILED CHECK`, 70 wide and 15 tall. It says what the session will read (`done (overridden)`) though which checkpoint failed and when, what it hides and where the failure stays visible (⊘ on the row, in the Inbox history and in the evidence, for this result only), and takes an optional reason (at most `OVERRIDE_REASON_MAX_CHARS`; typing, paste, Backspace and Ctrl+U edit it). Enter overrides; `O` overrides only while the reason is empty, so a typed reason can hold an O; Esc or Cancel does not. Opened from the Inbox, the Inbox stays under it and Esc or Cancel goes back to it, on the same item. The sheet, Inbox, popover and row menu all open this one modal. After it the evidence sheet shows that checkpoint and a toast says `Overridden. <session> reads done (overridden); the failure stays in its evidence.` A re-run says `Re-running <cmd> in <session>'s worktree…` in a toast, or `re-run queued` while the agent works; a daemon error (`ServerMsg::Error`) is a toast. While a sheet takes the pane column, toasts show over the sheet.

**Approving a repo's command** (`client/screen/check_approve.rs`): opens only from its waiting item, by Enter on the Inbox item or the sidebar row of a session whose checkpoint needs approval. Several waiting: one modal per checkpoint, the next opening after a choice. It shows the command (`$ npm test`, or `- old` / `+ new` when the repo changed one you had said Always to) in full: wrapped over as many `card2` rows as it needs, the modal growing to fit; `\n`, `\r`, `\t` and other control or invisible characters spelled out (`\u{001b}`), and a run of spaces shown as `[N spaces]`. Where it runs, the file and commit it came from (`added in a1b2c3d, 3 days ago`, `changed in <sha>, 2h ago`, or `changed, not committed`), when it runs, and "Read it. It can do anything you can in that folder." `1` and `2` answer only once the last frame showed the whole command, and only for that command: one too long for the window ends its last row in `…`, says `The whole command doesn't fit. Make the window bigger and read it first.`, and 1 and 2 only say so; a command that changes disarms `2` (`The command changed. Read it again before you choose.`). There is no default: `1` runs once, `2` arms Always (a strip spells out what it means and the button reads `2 again`) and a second `2` sends it, `3` is Never and opens the evidence sheet, Esc leaves the item waiting (back in the Inbox, on the same item, when opened there). Enter, `y` and Space send nothing and say so; other keys and pastes do nothing. Clicking Always once arms it like `2`. Below 80 columns the buttons use short labels; below 30 rows the modal loses the blank line after the command.

**Settings › Check** (`client/screen/check_settings.rs`) is one renderer with two hosts: the `PROJECT · <name>` modal (`,` from the evidence sheet or an Inbox row, for that session's project) and the Check page of Settings (the workspace you are in). Empty, it explains what a check is and takes a first command. Set, it shows the selected checkpoint's command, where it runs, its timeout (`‹ ›`), its latest result by clock time (`✓ passed on fix-footer 14:22 · 14.2s`, `… never run`, `? could not run … pnpm: not found (exit 127)` with what to do) and its source, then `CHECKPOINTS`: `1 of 1`, or `N of M · ‹ › switch` for several (←/→ on that row, or a click). Your checkpoints are `[[check]]` tables in config.toml; Ctrl+S writes the shown one by name in place, keeping comments, off the UI thread (a failed save keeps the draft and toasts; `Saved. Runs after the next turn ends.` for a first command, else `Saved.`). Del removes its table. A repo's checkpoint is read-only here: its source reads `.seshi/checks.toml · <commit> · always | never | asks each time` and Del (`Forget choice`) sends `ForgetCheckApproval`. Further checkpoints are added in config.toml or the repo's file; there is no add screen. Run now (`r`, or its button) re-runs that checkpoint in the session with the latest result, else the focused or first agent of the project; a working agent queues it. Esc drops the draft.

## Project-specific: Inbox check groups

The Inbox (`j`) follows v7 (`inboxItems`, `drawInbox`). Its title is `INBOX <n>`, n counting
everything not just finished. It lists, in this order: NEEDS YOU, FAILED CHECKS, COULD NOT RUN,
FINISHED, then HEADS UP, each a chip head on its colour (`blocked`, `err`, `violet()`, `done`)
with its count. A question is never below a check.

- A group with no rows is not drawn; with nothing at all it says "Nothing is waiting on you.".
  A project with no check has the groups it always had, FINISHED renamed from JUST FINISHED.
- NEEDS YOU holds questions, then repo check commands waiting for approval
  (`approve check command: npm test`).
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
  changed since`, `not checked: never for this repo`, `finished · not seen`); a failure adds its
  first failing output line. Several checkpoints read as one word first, then
  `build ✓ · test ✕`, or the glyph strip when that doesn't fit.
- The selected item is tinted across the sheet with `▎` (`err` for a failure, else `accent`); a
  failure keeps its red `▎` unselected. It grows its buttons, each a click of its key: question
  `Reply m` `Open o`; approval `Read the command ⏎`; failed `Output ⏎` `Re-run r` `Override… O`;
  could not run `Output ⏎` `Settings ,`; stale `Re-run r` `Output ⏎`. Enter runs the first
  button (Read the command opens the approval); an item with no buttons opens its session. `o` opens the session of any item; `r`, `O`, `u`, `,` act on an
  item's check where they apply and are typed otherwise. 1–9 answer a question; d, M, x and m
  act on a finished item; d, m and k on a heads-up; Delete marks seen.
- Typing finds any session; the `› query` row shows only once you type. The last row is
  `↑↓ select   ⏎ open   r re-run   O override   Esc close`.
- Existing theme slots only. Code lives in `client/screen/inbox.rs` (groups, drawing) and
  `client/screen/check_inbox.rs` (what each item says and its buttons).

## Project-specific: v7 frame

The v7 layout and checkpoints follow [`docs/design/v7/`](design/v7/README.md) (`checks7.js` is the spec). The frame is [ADR 0013](adr/0013-v7-frame.md); `client/screen/v7_frame_tests.rs` holds it to the reference frames, and `client/v7_frames_tests.rs` holds every preset's whole screen to them (see [TESTING](TESTING.md)).

- **Grid** (`draw.rs` `grid`): no outer margin. The sidebar is 40 columns from 200, 34 from 140, else 26 (a width you dragged wins), the full height. Floating leaves `ui.gap` between cards, tiled none (borders side by side, never merged), Flat starts the panes on the sidebar's last column.
- **Cards** (`card.rs` `card`): `┌┐└┘` or `╭╮╰╯` (`ui.corners`), `line`, the focused card's whole border in the accent. The title chip sits on the top border two in; the right-hand segments end a padding space before the corner. **Flat**: no box; row 0 is a header strip of `card`, the chip one in, the right-hand segments a column from the edge; a zone not at the screen's edge starts with a `│` rule.
- **Pane title** (`draw.rs` `draw_session`): the top-left pane of the tab carries the session's tabs (a number chip and a name chip each, the current one in the accent; those that don't fit drop from the right); other panes show their session's name. On the right, the session's mark (glyph and word; it opens why) and a ✕ that closes the pane. The leader pill (armed) or **Update** sits at the top right of row 0, a column in.
- **Sheet** (`sheet.rs` `split_for_sheet`): 72 columns docked right from 140 columns, the layout's gap from the panes; below 140 it takes the pane column and Esc brings the panes back.
- **Differences kept on purpose.** The app draws its own data where the handoff has samples, and these on purpose: Settings › Check marks the row the keys are on by its label in bold `accent` (the handoff draws every label dim and has no row cursor); evidence output colours only `✓` and failure lines (the handoff also dims `>` command echoes, `❯ file:line` and `Duration`, and colours an assertion's continuation as a failure); the why popover's last line leaves out the handoff's `· 1 of 59 tests failed` (output is not parsed for counts) and the evidence Source row leaves out when you chose never (`you chose never 14:31`; the time of a choice is not shown); and the several-checkpoints words add `could not run: <name>` and `checking: <name>` to the handoff's four (`failed`, `needs you`, `next`, `all passed`), because a checkpoint that could not run is not a failure and checking must read apart from next. A re-run keeps the sheet or Inbox it was started from open (the handoff closes it), and its result is told by the daemon's notices (`<place>: check failed`) and the row, not by the handoff's `test passed (exit 0, 13.9s). fix-footer reads done.` / `test failed (exit 1). fix-footer is in the Inbox.` toasts.
- **Toast**: bottom right on the second row from the bottom, ending three columns from the edge, raised (`btn`) in `strong` with its mark in its colour.

## Project-specific: home screen check counters

The home screen's "while you were away" line counts a session as finished by the Inbox's rule
(`check::finished`: see Inbox check groups; a stale pass or a check never run here counts as
finished), and a check waiting for approval as needing you.
A `✕ N check failed` (`N checks failed`) count follows, in `err`, only when N is above zero.
When the line does not fit the width, the lead-in "while you were away" is dropped so no count
is cut. A project with no check draws the line it always did. Code lives in `client/screen/splash.rs`.

## Project-specific: collapsed sidebar

Hiding the full sidebar leaves a seven-column rail on the configured side.
Show each session's mark (the same glyph and colour as its sidebar row, from the
`[ui] glyphs` set, so stale, never-run and waiting-for-approval checks read there too)
and first two non-space title characters, including sessions in folded groups. A plain
terminal keeps its `›`.
Keep active selection distinct from hover; reveal project and full title on hover.
Click rows to focus or resume a legacy sleeping session; retain keyboard browsing and wheel scrolling. Keep the
total blocked count visible even when its rows are off-screen. Provide a mouse
expand control and retain Actions. The rail is the collapsed state of the same
layout, not another layout.

Show active helper rows under their owning agent with the reported task as the label. When no task is reported, show a reported current tool action if available, then use a recognizable stage, a meaningful role name, or `Working` as the fallback. Keep helper rows compact, dim and neutral gray, without a stage icon or working spinner, so the parent session remains visually primary. Keep the rows visible in narrow sidebars and truncate labels to the available width. Preserve distinct status colors for helpers that need attention, have completed, or failed.

## Project-specific: frame presentation

Wrap each outer terminal frame in synchronized-update markers; end the update
even when drawing fails. An unfinished child update from the previous frame
must not delay switching to another session.

Keep pane padding and the card interior on the same surface as a terminal app
when at least three screen corners use one explicit background color. Preserve
the pane insets; isolated colored panels keep their existing theme treatment.
In Flat layout, a zone's body (the sidebar's, a pane's padding and its default
terminal cells) is the theme's `bg`, as in Cards; only row 0 is the `card` header
strip. Preserve colors applications paint explicitly.
