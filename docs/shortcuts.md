# Contextual shortcuts

Open **All keys** with `Ctrl+Space`, release, then `?` (or Actions → All keys).
The in-app reference lists every configured leader and global binding, including
aliases and custom commands. Type to filter by key, action or screen name; arrows,
page keys, Home/End and the mouse wheel scroll. Ctrl+U clears; Esc closes.
`C-` means Ctrl, `M-` means Alt. The [README](../README.md#use) lists the defaults.
On Windows, Caps Lock does not change shortcut meaning; hold Shift for uppercase
bindings such as `N` (What happened).

Mouse entry: **Actions** stays at the top right even with the sidebar hidden.
It opens agent status setup, Settings and All commands.
Click a command to run it; the wheel scrolls the palette. Configured custom
bindings also appear as clickable commands. Close buttons dismiss overlays.

These tables are also embedded in the app's All keys reference. Keys below act
inside the named screen, without the leader. Ordinary typing edits the focused
text field; Backspace removes a character. Keys sent to Claude, Codex, your shell
or another program follow that program's own help.

## All shortcuts

| Keys | Action |
| --- | --- |
| Type, Backspace, Ctrl+U | Filter by key, action or screen; clear search |
| Up, Down, PgUp, PgDn, Home, End, wheel | Scroll reference, page, go to beginning/end |
| Esc | Close reference; no command runs while reading it |

## Leader overview and Actions

| Keys | Action |
| --- | --- |
| Bound leader key | Run its command from the overview; Actions accepts keys for its listed commands |
| Tab | Toggle overview search; type words to search commands |
| Up, Down, Enter | Choose and run a search result, worktree step or Actions row |
| Backspace | Clear the overview query one character at a time, then return from a step |
| Esc | Cancel the overview or Actions |

## Worktrees second step

| Keys | Action |
| --- | --- |
| n, s, m, d | New worktree, switch via Inbox, open Changes to merge, remove worktree |
| Backspace, Esc | Return to overview, or close |

## Sidebar keyboard focus

| Keys | Action |
| --- | --- |
| Up / k, Down / j, Home, End | Browse rows, first row, last row; active session stays selected |
| Right, Left | Expand project or enter session; fold project or go to session's project |
| Enter | Toggle project or open selected session; opening a sleeping session wakes it. On a session whose check failed, could not run, was overridden, went stale or is never run for its repo, open its evidence sheet instead; on one whose repo command waits for approval, open the approval |
| Space | Open New for the selected project |
| m, i, s, a | Follow-up to selected agent, why it reads as it does, inspect its subagent activity, Actions |
| r, O, u, o | On a session with a check: re-run it (once it has a result), override it (failed only, asked first), undo the override (overridden only), its output. A key the result doesn't take says why |
| Up, Down in the activity inspector | Inspect the previous or next child record |
| Delete | Ask to close the selected session or project |
| Menu letters | Use the letters shown in the selected row's right-click menu; r renames, x closes. A session with a check also lists Check output (o), Re-run check (r), Override check… (O, failed) and Undo override (u, overridden); there Rename takes e |
| Esc | Return keyboard focus to the active pane |

## Command palette and text prompts

| Keys | Action |
| --- | --- |
| Type, Backspace | Search or edit text |
| Up / Shift+Tab / Ctrl+P, Down / Tab / Ctrl+N | Move through palette or worktree-picker results |
| Ctrl+U in text prompts | Clear prompt text |
| Enter, Esc | Run selected result / submit prompt, or cancel |
| y / Y / Enter in close-group or remove-worktree prompt | Confirm; f / F forces worktree removal when offered; other keys cancel |

## New agent or fleet

| Keys | Action |
| --- | --- |
| Tab / Down, Shift+Tab / Up | Next or previous field |
| Left, Right | Change the selected option |
| Ctrl+U | Clear task text |
| b | Toggle beside when outside the task text field |
| Enter, Esc | Start, or cancel |

## Inbox

| Keys | Action |
| --- | --- |
| Type, Backspace | Filter by project or session (the search row shows once you type) |
| Up, Down, PgUp, PgDn | Choose a row |
| Enter | The selected item's first button: Reply (a question), Read the command (a check waiting for approval), Output (a failed or could-not-run check), Re-run (a stale pass); an item with no buttons opens its session, as does a search result |
| o | Open the selected item's session; empty search |
| 1–9 | Answer selected agent question when search is empty |
| m | Follow-up to selected asking/finished agent when search is empty |
| d, M, x | Finished worktree: diff, ask to merge, ask to discard; empty search |
| d, m, k | Shared-file warning: diff, message its agent, dismiss; empty search |
| r, O, u, , | The selected item's check: run it again, override it (asks first), undo an override, its settings; only where they apply, else typed; empty search |
| Click a button | The key it shows |
| Delete | Mark selected finished session, or unrun check, seen |
| Enter, Esc in confirmation | Confirm action, or cancel |
| Esc | Close Inbox |

## Follow-up and status evidence

| Keys | Action |
| --- | --- |
| Enter | Send follow-up, or queue while working; empty message means keep going |
| Ctrl+Enter | Send immediately, including while agent is working |
| Shift+Enter, Alt+Enter | Insert a newline |
| Ctrl+U, Esc | Clear message, or cancel |
| i, Esc in evidence | Close status explanation; other keys close it and perform their normal action |
| Enter, r, O in a check's evidence | Open its evidence sheet, run it again, override it (asks first); only the keys the popover shows act |

## Files tree and preview

| Keys | Action |
| --- | --- |
| Up / k, Down / j, Home, End | Choose file |
| Right / l, Left / h, Enter on directory | Expand, collapse / parent, toggle directory |
| Enter on file | Insert quoted path into agent prompt and close Files |
| Tab, / | Toggle recent/tree, filter paths; Enter ends filtering, Esc clears filter |
| e, i, o, y, d | External editor, edit in preview, open outside, copy path, Changes |
| PgUp, PgDn, Shift+Up, Shift+Down | Scroll preview pages or individual lines |
| Right on file | Focus preview; arrows / j / k scroll, Space pages down, g / G or Home / End go to ends |
| Left / h / Esc in preview | Return to tree |
| Esc in tree | Close Files |

## Built-in file editor

| Keys | Action |
| --- | --- |
| Arrows, Home, End, PgUp, PgDn | Move cursor or page |
| Enter, Tab, Delete, Backspace | Newline, four spaces, delete forward, delete backward |
| Ctrl+S | Save; repeat to overwrite a file that changed on disk after warning |
| Esc | Exit editor; repeat to discard unsaved changes after warning |

## Find file and search code

| Keys | Action |
| --- | --- |
| Type, Backspace, Tab, Shift+Tab | Edit search; switch Files / Search code tab |
| Up, Down, PgUp, PgDn | Choose result |
| Enter, Ctrl+E, Ctrl+Y | Put path/line into prompt, open in editor, copy path |
| Esc | Close search |

## Changes and shared-file diffs

| Keys | Action |
| --- | --- |
| Up / k / Shift+Tab, Down / j / Tab | Previous or next changed file |
| PgUp, PgDn, Space | Read diff; Space pages down |
| x, e, r | Mark reviewed, open editor, return to agent |
| c, p | Commit; ask to push and open a pull request |
| m, d | Linked worktree: ask to merge, ask to discard |
| Enter / y in confirmation | Confirm the requested action; other keys cancel |
| Up, Down, PgUp, PgDn in shared-file diffs | Scroll both checkout diffs together |
| m, M, k in shared-file diffs | Message first agent, message second agent, dismiss warning |
| Esc | Close Changes or shared-file diffs |

## Check evidence sheet

| Keys | Action |
| --- | --- |
| Up, Down, PgUp, PgDn | Scroll the output; the first press leaves the tail that fits |
| f | Show all the output, or return to the tail |
| r | Run the check again (queued while the agent is working) |
| O | Override a failed check: opens a confirm with an optional reason |
| u | Undo an override: the check reads as failed again |
| o | Go to the session |
| i | Why it reads as it does (the evidence popover) |
| , | The project's check (`PROJECT · <name>`) |
| Tab, Shift+Tab | Several checkpoints: the next or previous one |
| Esc | Close the sheet |

## Check modals

| Keys | Action |
| --- | --- |
| Type, paste, Backspace, Ctrl+U in Override | Write the reason, delete, clear it |
| Enter in Override | Override; O also overrides while no reason is typed (after that it is a letter) |
| Esc in Override | Cancel |
| 1 in the approval | Run the repo's command once; it asks again next turn |
| 2, then 2 again, in the approval | Always for this exact command; the first 2 (or click) only arms it |
| 3 in the approval | Never for this repo; the evidence sheet opens |
| Esc in the approval | Not now: the item stays in the Inbox |
| Enter, y, Space in the approval | Nothing: a toast says to choose 1, 2 or 3 |
| Up, Down in `PROJECT · <name>` | Choose the Command, Timeout or CHECKPOINTS row |
| Enter or typing on Command | Edit the command (your checkpoints only); Enter ends editing |
| Left, Right on Timeout or CHECKPOINTS | Step the timeout; switch checkpoint |
| Ctrl+S | Save the shown checkpoint |
| r | Run it now (queued while the agent is working) |
| Delete | Remove your checkpoint; forget your choice for a repo's |
| Esc | Close and drop the draft |

## Branches and worktree cleanup

| Keys | Action |
| --- | --- |
| Type, Backspace, Up, Down, PgUp, PgDn, Enter | Branches: filter, choose branch/create option, continue |
| Up, Down, 1–4, Enter | Choose how branch switching handles uncommitted changes |
| y in discard confirmation | Confirm discarding changes; other keys go back |
| Enter in commit text | Commit changes and switch branch |
| Up / k, Down / j, Space | Cleanup: choose row, toggle removal |
| a, n, Enter in cleanup | Select safe unused worktrees, clear selection, review removal confirmation |
| Esc | Go back from branch step, or close branches/cleanup |

## Settings and themes

| Keys | Action |
| --- | --- |
| Tab, Shift+Tab, Up / k, Down / j | Change settings category; choose row |
| Left / h, Right / l, Enter, Space | Change value; Enter edits text or captures binding; theme opens browser |
| o | Open configuration file outside Seshi |
| R | Restart the daemon after an update (when the Restart now button shows) |
| Keys on the Check page | As in `PROJECT · <name>` (see Check modals); Tab, Esc and o stay Settings' |
| Enter, Esc when editing | Save field / cancel; when capturing press the new key or Esc |
| Type, Backspace, Ctrl+U in themes | Filter themes, clear filter |
| Up, Down, PgUp, PgDn, Home, End in themes | Preview themes |
| Enter, Esc in themes | Save theme, or return without saving |
| Up, Down, wheel, Home / PgUp, End / PgDn in working spinners | Preview styles; go to first/last choice |
| Enter, Esc in working spinners | Apply preset / edit Custom; return without applying |
| Left, Right, Home, End, Backspace, Delete, Ctrl+U in custom frames | Edit cursor position, delete, or clear |
| Paste, Enter, Esc in custom frames | Insert frames, save, or cancel editing |
| Esc in settings | Close Settings |

## Copy mode

| Keys | Action |
| --- | --- |
| Arrows / h j k l, w, b | Move cursor, next word, previous word |
| Home / 0, ^, End / $ | Line start, first nonblank character, line end |
| g, G, H, M, L | History start, history end, viewport top, middle, bottom |
| Ctrl+U / Ctrl+D, Ctrl+B / Ctrl+F, PgUp / PgDn | Half-page or full-page scroll |
| Ctrl+Y, Ctrl+E | Scroll one line up or down |
| v / Space, V | Toggle character selection, toggle line selection |
| y / Enter with selection | Copy selection and leave |
| y / Y without selection | Copy current line; Enter without selection leaves without copying |
| /, ?, n, N | Search forward, backward, next match, previous match |
| Enter, Esc in search | Apply search, or cancel search entry |
| Tab, Shift+Tab | Keep selection and move to next/previous split pane; y copies collected selections |
| Esc, q, Ctrl+C | Esc clears selection first, then exits; q or Ctrl+C exits |

## Terminal scrolling and mouse

Hiding the sidebar retains a compact session rail: click initials to focus,
hover for the full title, wheel to scroll, and click its chevron to expand.
Status glyphs/colors and the total blocked count remain visible. The existing
sidebar toggle switches between the full sidebar and rail.

| Keys | Action |
| --- | --- |
| PageUp, PageDown at a prompt | Scroll history; full-screen programs keep these keys |
| Shift+PageUp, Shift+PageDown, Ctrl+Shift+Up, Ctrl+Shift+Down | Scroll half-pages or individual lines outside full-screen mode |
| Shift with mouse | Keep selection/scrolling in Seshi when program captures the mouse |
| Ctrl+click | Open a path or link |
| Shift+right-click | Seshi menu even when program handles right-click |
| Double-click, drag | Copy word/path/link, select and copy text |

## Menus, history, splash and confirmations

| Keys | Action |
| --- | --- |
| Up / k, Down / j, Enter, shown letters | Choose or run a context-menu item; Delete asks to close where offered |
| 1–9 in numbered menu | Choose numbered option, such as an agent or snapshot |
| Up, Down, Enter, c, Esc in history | Choose event, go to its session, clear events, close history |
| Up / Left / k / Shift+Tab, Down / Right / j / Tab in splash | Choose splash action |
| r, n, comma, ? in splash | Resume if available, new shell, Settings, All keys |
| Enter / y / shown confirm key | Confirm action; closing panes also accepts x or Delete |
| Esc / n / q in standard confirmation | Cancel action |
| Tab in folder picker | Complete current directory; arrows choose; Enter opens; Esc goes back |
