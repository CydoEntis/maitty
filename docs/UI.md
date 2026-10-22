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

Seshi has one terminal interface: a sidebar of projects and sessions, with tabs and split panes in the remaining area. Settings > Appearance > Layout selects Cards or Flat; both use the same sessions, tabs, splits and sheets. Cards is the default. Flat uses square, full-bleed zones and Tab / Shift+Tab to move focus between the sidebar and panes; Esc returns focus to the sidebar. There is no application-wide footer bar. The tab row aligns with the sidebar. Keep **+ new** and **Actions** at the bottom of the sidebar in both layouts. The tab row shows **Update** only when a newer release is available.

The leader is `Ctrl+Space` by default. Inbox (`j`) uses the right-hand sheet. Settings (`,`) opens as a centred modal over the workspace so the pane layout does not change. The command palette (`Space`), shortcuts (`?`), files, branches and new agents use popups. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. Prefer Seshi's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll.

The high-fidelity flat-layout reference and its interaction spec are preserved in [`docs/design/flat/README.md`](design/flat/README.md); update it when the implementation changes the design contract.

The sidebar's **+ new** uses the same shell action as leader + `n`, including its configured key hint. **Actions** is in the sidebar footer and opens status setup, Settings and All commands. Every palette row is clickable and wheel-scrollable. Include configured commands and retain existing confirmations.

When an update is available, show **Update <version>** in the tab row and open the existing confirmation flow when clicked. Do not show the control when Seshi is current. Keep status icons and pane status tags clickable to open the evidence explanation. Terminal bells do not add sidebar indicators or history entries.

Mark one selected sidebar row independently from hover: the keyboard selection stays selected when the mouse moves; hover is a separate, quieter tint. The open session keeps a dim selection bar while another row is under the keyboard. Preserve agent status icons and colors in each state.

Settings opens on General and reports installed, window and daemon versions separately, with a labeled remote daemon and restart-pending notice when they differ. General groups controls under INPUT, LAYOUT, SHELL and UPDATES. Show the update action only when an update is available; keep release notes and confirmation in the existing flow.

Show reported child agents under their parent, never as independent projects. Keep the sidebar list live: remove a child when its reported state is completed, failed, cancelled or unconfirmed, while retaining lifecycle evidence in daemon state. Make active child rows clickable to inspect reported task and stage details. Keep parent session status colors unchanged. Active subagents use muted neutral gray with no per-helper animation; their compact task or role label is muted, dim and non-bold, with no generic gear icon. Use the provider-reported task when available; otherwise show a reported current tool action, recognizable role stage, meaningful reported role name, or `Working` when none is available. Preserve standard needs-you and error colors for blocked or failed children. Do not show lifecycle group labels or counts beyond the parent heading total. Do not assign role colors that could be confused with done, working, blocked or idle.

Keep update indicators until the installed program, window and daemon versions agree. Installing an update leaves open sessions running. For a local daemon, when no panes are open, restart it automatically and reconnect the window so daemon fixes take effect without `kill-server`. For a remote daemon, explain that it must be updated on its host after its sessions are closed.

When a local daemon is older than the installed seshi, Settings' restart-pending line carries a **Restart now** button (`R` in Settings), and the toast that follows an install shows the same button and stays longer than other toasts. Neither shows for a remote daemon, while a restart is under way, or when the daemon already matches. Choosing it opens a confirm (`menu::Confirm`, danger while an agent is working) that reads "3 sessions stop and resume, 1 is working" with Restart and Cancel; Cancel changes nothing, and with no sessions open it restarts without asking. It goes through the same stop-and-reconnect path as the automatic restart.

A new `seshi` that finds an older or newer server it cannot talk to (`ipc::OtherVersion`) shows a full-screen prompt (`client/restart_prompt.rs`) on an interactive terminal: the server's protocol next to this one's, "Running agents stop and resume when it restarts", and **Restart and open** (Enter) and **Quit** (Esc, Ctrl+C), each clickable. Restart stops the server by its process, as `seshi kill-server` does across versions, then opens normally; Quit prints the usual error and exits 1. The prompt uses the configured theme and pill style. Without a terminal on stdin and stdout, or with `--remote`, there is no prompt: the error text and exit code are unchanged, and a remote server is never restarted this way. The protocol number is all an older server reports, so the prompt cannot name its release.

**All keys** (`<leader> ?`, Actions or palette) opens a searchable, scrollable reference. List every active configured leader/global binding separately, including aliases and custom commands. Show palette-only commands without inventing bindings. Keep the delayed leader overview compact. Contextual keys come from [shortcuts.md](shortcuts.md), embedded in the app so both stay identical. Render each binding as a keycap, separate from its action text; use clear section headings and wrap long actions without making them look like additional keys. On wide screens, balance whole sections into three visibly separated columns; use one column when the reference would otherwise be cramped.

## Project-specific: pane focus

Only the area with the keyboard is lit: the focused pane border (Cards), focus bar (Flat), or open sheet. Unfocused panes fade by the Dim setting (30% by default; "40%" in an older config counts as 30%). Do not reserve a full-width bottom row for global hints; session paths and branch details remain in the pane chrome.

## Project-specific: UI conventions

<!-- local: user requested distinct active, keyboard and hover sidebar states, mouse-first sidebar controls and parent-owned child-agent visibility. -->

- Popups are centred `panel()`s with an accent title bar and `Esc close`; the
  screen behind is dimmed. Settings is a centred modal that preserves the
  workspace layout and keeps its General, Sessions, Appearance, Agents and Keys
  sections, update controls, and keyboard and mouse editing available.
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

A session with a check result (`TermInfo::check()`, the headline checkpoint) swaps its status glyph for the
check's and adds one line under the row. A session with no result, or one whose
agent is working or asking, draws exactly as without checks.

| Result | Glyph (Unicode / ASCII) | Colour slot | Line under the row |
| --- | --- | --- | --- |
| checking | `◜◝◞◟` / `- \ \| /` | `sky()` | `checking <cmd> · <age>` |
| passed | `✓` / `+` | `done` | `<cmd> ✓ · <took> · <age> ago` |
| failed | `✕` bold / `x` | `err` | `failed: <cmd> · exit N`, bold |
| overridden | `⊘` / `/` | `text`, never green | `<cmd> ✕ overridden by you` |
| could not run | `?` / `?` | `muted`, not a failure | `<cmd> could not run · not found` |

- Shape and word carry the meaning; colour only backs them. Failed (`✕`, "failed")
  and needs-you (`●`, the question) differ in glyph, word and colour.
- A failed row keeps a red `▎` edge bar on both lines even when not selected.
  Could not run has no bar.
- Glyphs come from the set `[ui] glyphs` picks (`client/screen/glyphs.rs`: unicode, ascii,
  nerd); `auto` uses ASCII when the `blocked`, `done` and `idle` icons are all ASCII.
- The line is dropped below `NARROW` columns, before the glyph; the folder name is
  never touched. A long command is shortened first so the result still reads.
- The collapsed rail shows the same glyph. The ring turns with the motion setting;
  the working spinner choice is unchanged.
- Colours are existing theme slots only; there is no check role. `Theme::violet()` (the
  theme's first project colour, made readable) and `sky()` are derived, not roles. Code lives in
  `client/screen/check.rs`; the evidence sheet is `client/screen/check_sheet.rs`.

## Project-specific: check evidence sheet

The check evidence sheet (Enter on a sidebar row whose check failed, could not run or was overridden; every other row still focuses its session) docks right like the Inbox and takes the pane column below `NARROW`. It shows the result chip with its exit code, a banner for an override (when, the reason, `u undo`) or a command that could not run ("Not a test failure. The work hasn't been checked."), the Command / Folder / Ran / Seen by rows with relative ages, and the output on a darker block with line numbers (`✓` lines in `done`, error lines in `err`). Output longer than the block shows the tail that fits ("last N of M lines"); `f` or the arrows open all of it and scroll. Empty output reads "(the command printed nothing)", with "exit 0 is what passed it." for a pass. Re-run (`r`, not while it runs), Override… (`O`, failed only) and Undo override (`u`, overridden only) are buttons and keys; the hint row lists the ones that apply. A re-run queued behind a working agent reads `re-run queued` beside the result chip.

The row menu of a session with a check adds Check output (any result), Re-run check (not while it runs) and Override check… (failed only); letters come from `menu_keys` like every item. Output comes from `Query::CheckOutput` and is fetched again when a new run replaces the result. Command output is not trusted: colour codes and control characters are stripped before drawing.

## Project-specific: why popover

The evidence popover (`i`, or a click on a status) explains a session's status from its hook, screen and process reports. For a session with a check, it shows a timeline instead: when the agent's turn ended and what it said, when Seshi ran the command, and the result with exit code and duration, each with its age (`2m ago`). Its keycap hints list only the keys that apply, and each one acts: `⏎` evidence and `r` re-run once there is a result, `O` override for a failed one, `Esc` close. A key the popover does not show closes it and is not passed to the pane. A session without a check keeps the source rows.

## Project-specific: check modals and settings

Deciding is a modal; nothing here pops up on its own. Each matches the v7 handoff (`checks7.js` `drawModal`) inside the modal frame, cell for cell, in every layout at 160×45, 100×30 and 80×24 (`client/screen/check_modals_tests.rs`). Colours are existing slots: `blocked` for "needs you" amber, `done`, `err`, `violet()`, `sky()`, `card2` for fields.

**Override** (`client/screen/check_actions.rs`): `OVERRIDE A FAILED CHECK`, 70 wide and 15 tall. It says what the session will read (`done (overridden)`) though which checkpoint failed and when, what it hides and where the failure stays visible (⊘ on the row, in the Inbox history and in the evidence, for this result only), and takes an optional reason (at most `OVERRIDE_REASON_MAX_CHARS`; typing, paste, Backspace and Ctrl+U edit it). Enter overrides; `O` overrides only while the reason is empty, so a typed reason can hold an O; Esc or Cancel does not. The sheet, Inbox, popover and row menu all open this one modal. After it: `Overridden. <session> reads done (overridden); the failure stays in its evidence.` A re-run says `re-running <cmd>…` in a toast, or `re-run queued` while the agent works; a daemon error (`ServerMsg::Error`) is a toast. While a sheet takes the pane column, toasts show over the sheet.

**Approving a repo's command** (`client/screen/check_approve.rs`): opens only from its waiting item, by Enter on the Inbox item or the sidebar row of a session whose checkpoint needs approval. Several waiting: one modal per checkpoint, the next opening after a choice. It shows the command (`$ npm test`, or `- old` / `+ new` when the repo changed one you had said Always to), where it runs, the file and commit it came from (`added in a1b2c3d, 3 days ago`, `changed in <sha>, 2h ago`, or `changed, not committed`), when it runs, and "Read it. It can do anything you can in that folder." There is no default: `1` runs once, `2` arms Always (a strip spells out what it means and the button reads `2 again`) and a second `2` sends it, `3` is Never and opens the evidence sheet, Esc leaves the item waiting. Enter, `y` and Space send nothing and say so; other keys and pastes do nothing. Clicking Always once arms it like `2`. Below 80 columns the buttons use short labels; below 30 rows the modal loses the blank line after the command.

**Settings › Check** (`client/screen/check_settings.rs`) is one renderer with two hosts: the `PROJECT · <name>` modal (`,` from the evidence sheet or an Inbox row, for that session's project) and the Check page of Settings (the workspace you are in). Empty, it explains what a check is and takes a first command. Set, it shows the selected checkpoint's command, where it runs, its timeout (`‹ ›`), its latest result by clock time (`✓ passed on fix-footer 14:22 · 14.2s`, `… never run`, `? could not run … pnpm: not found (exit 127)` with what to do) and its source, then `CHECKPOINTS`: `1 of 1`, or `N of M · ‹ › switch` for several (←/→ on that row, or a click). Your checkpoints are `[[check]]` tables in config.toml; Ctrl+S writes the shown one by name in place, keeping comments, off the UI thread (a failed save keeps the draft and toasts; `Saved. Runs after the next turn ends.` for a first command, else `Saved.`). Del removes its table. A repo's checkpoint is read-only here: its source reads `.seshi/checks.toml · <commit> · always | never | asks each time` and Del (`Forget choice`) sends `ForgetCheckApproval`. Further checkpoints are added in config.toml or the repo's file; there is no add screen. Run now (`r`, or its button) re-runs that checkpoint in the session with the latest result, else the focused or first agent of the project; a working agent queues it. Esc drops the draft.

## Project-specific: Inbox check groups

The Inbox (`j`) lists, in this order: NEEDS YOU, FAILED CHECKS, COULD NOT RUN, JUST FINISHED,
then HEADS UP, each heading with its count. A question is never below a check.

- A group with no rows is not drawn, so a project with no check shows the Inbox it always had.
- A Done session is finished (`check::finished`) only when it has no check or its check passed
  or was overridden. It is not finished while checking, failed or could not run, so it is not
  under JUST FINISHED. The home screen counts finished by the same rule. A session whose agent
  is working or asking is not listed for its old result (see `check::shown`).
- A failed check stays listed until a later run replaces it, even after you have looked at the
  session. A could-not-run check is listed only until you look (`Del` marks it seen).
- A row is the result glyph (the sidebar's), name, `agent · folder` and a relative age; line 2 is
  the sidebar's result words; a failed row adds the first failing output line in `muted`.
  Enter focuses the session, as on every other row. The selected failed row grows a line of buttons: Output `o`, Re-run `r`, Override… `O` (a confirm, as in the sheet). While a failed row is selected, those three letters act instead of starting a search.
- Existing theme slots only. Code lives in `client/screen/check_inbox.rs`.

## Project-specific: v7 frame

The v7 layout and checkpoints follow [`docs/design/v7/`](design/v7/README.md) (`checks7.js` is the spec).

## Project-specific: home screen check counters

The home screen's "while you were away" line counts a session as finished by the Inbox's rule
(`check::finished`: see Inbox check groups).
A `✕ N check failed` (`N checks failed`) count follows, in `err`, only when N is above zero.
When the line does not fit the width, the lead-in "while you were away" is dropped so no count
is cut. A project with no check draws the line it always did. Code lives in `client/screen/splash.rs`.

## Project-specific: collapsed sidebar

Hiding the full sidebar leaves a seven-column rail on the configured side.
Show each session's status and first two non-space title characters, including
sessions in folded groups. Reuse the working spinner and blocked/done colors.
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
In Flat layout, use the theme surface for default terminal cells so the pane
canvas does not become a separate black block. Preserve colors applications
paint explicitly.
