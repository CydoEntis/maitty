# Shortcuts

Every default shortcut, then the keys each screen has of its own. You can change any of them
in Settings › Keys or under `[keys.prefix]` in your config.

## Default shortcuts

Press `Ctrl+Space` for shortcut mode, then the key. Only `?` needs Shift, and each key does
one thing (ADR 0017, 0022).

### Sessions

| Key | Action |
|---|---|
| `n` | New shell where you are |
| `s` | New session, started straight away with your default agent |
| `o` | New session with options (agent, where, project) |
| `Enter` | Message the agent |
| `k` | Stop the agent |
| `i` | Inbox: what needs you, then every session |
| `y` | Why this status |

### Panes

| Key | Action |
|---|---|
| `←` `↓` `↑` `→` | Move to the pane that way (left past the edge: the sidebar) |
| `v` | Split side by side |
| `h` | Split stacked |
| `x` | Close the pane |
| `Ctrl`+arrows | Resize the pane |
| `PgUp` / `PgDn` | Scroll half a pane |
| `z` | Collapse the sidebar (again brings it back) |

### Tabs

| Key | Action |
|---|---|
| `t` | New tab |
| `Ctrl+W` | Close the tab |
| `r` | Rename the tab |
| `]` / `[` | Next / previous tab |
| `1`–`9` | Go to that tab (`Alt+1`–`9` works without the leader) |

### Project

| Key | Action |
|---|---|
| `f` | Files |
| `e` | The sidebar |
| `p` | Find a file |
| `/` | Search the code |
| `c` | Changes |
| `g` | Worktrees › |
| `b` | Branch (in the side panel; its first row switches branch) |

### Maitty

| Key | Action |
|---|---|
| `Space` | Search commands |
| `m` | Menu |
| `,` | Settings |
| `?` | All keys |
| `l` | What happened (history) |
| `Ctrl+V` | Paste the clipboard's image |
| `q` | Step away (agents keep running) |
| `Ctrl+R` | Reload the config |

**Search only** (`Space`, then type): hide the sidebar completely, previous / next command in
history, open the worktree picker, rename a session, send `Ctrl+Space` itself to the program.

## How shortcut mode works

The leader is `Ctrl+Space`. Press it and you are in **shortcut mode**: each key runs its
action, and the mode stays for the next, so a run of actions is one key each. `Esc`, the
leader again or a click go back to **input mode**, where keys type into the pane; nothing
else does. A new shell, session, tab or split leaves you in shortcut mode, so press `Esc` to
type into it. An action that opens a popup or sheet hands it the keys, and closing it returns
to shortcut mode. The top right always says which mode you are in: `TYPING`, or a sky
`SHORTCUT · Esc to type` while shortcut mode is on (`SHORTCUT · paused` while a popup has
the keys). In shortcut mode the whole window is also framed in sky blue and the focused
pane's border is sky blue, and a key that does nothing shows "Shortcut mode — Esc to type".
To send `Ctrl+Space` itself to the pane, search
commands for "Send the leader key" or bind `send-prefix` to a key. One overlay holds every key
reference; it is not a second palette:

- **Key map** (`Ctrl+Space`, then a pause, with Settings › General › **Leader waits for next
  key** on; it is off by default): the keys by group, Sessions, Panes, Tabs,
  Project and Maitty, built from your real bindings. Pressing a key runs it.
- **Search commands** (`Space` after the leader, `Tab` from the map, or typing a letter
  that is not a key): every command, including ones with no key. With nothing typed it
  lists the ones you ran last.
- **All keys** (`?` after the leader, or Menu → All keys): the map in full, the keys that
  need no leader, and the commands that only search finds.

Keys are drawn as keycaps; `Ctrl+Space` is two caps joined by a dim `+`. On Windows,
Caps Lock does not change shortcut meaning, and no default but `?` needs Shift (ADR 0017,
0022). The
[README](../README.md#shortcuts) lists the defaults.

Mouse entry: **Menu** has a chip in the collapsed strip (row 0); with the sessions hidden
(search commands for "Hide sidebar"), use the leader. Click a row to run it; the wheel moves the selection. Close buttons
dismiss overlays.

Keys below act inside the named screen, without the leader. Ordinary typing edits the
focused text field; Backspace removes a character. Keys sent to Claude, Codex, your shell
or another program follow that program's own help.

## Key overlay

| Keys | Action |
| --- | --- |
| A bound key (key map) | Run its command and close |
| Tab | Switch between the key map and the search |
| Any unbound letter (key map or All keys) | Start a search with it |
| Type, Backspace, Ctrl+U (search) | Edit the words; Backspace on an empty box returns to the map |
| Up, Down, Ctrl+P, Ctrl+N, PgUp, PgDn (search) | Choose a result |
| Enter (search) | Run the selected command |
| Up, Down, PgUp, PgDn, Home, End (All keys) | Scroll the list |
| Esc | Close; no command runs while reading |

Every key is a leader key: there is no letter that acts by itself in the sidebar.
The defaults are one key each, none but `?` needing Shift, chosen from what other apps use
(ADR 0017, 0022): `v` / `h` split side by side / stacked, `c` Changes, `i` Inbox, `y` why, `k` stop
the agent, `Enter` message it, `o` session with options, `p` find a file, `g` worktrees, `b`
Branch, `l` what happened, `?` all keys, `Ctrl+W` close tab, `Ctrl+V` paste an image,
`Ctrl`+arrows resize, `1`–`9` go to a tab. Your own `[keys.prefix]` entries still load.

## Menu

| Keys | Action |
| --- | --- |
| The key shown beside a row | Run it (the same leader key works anywhere): `n` Shell here, `s` Session, `o` Session with options…, `v` Pane beside this, `t` Tab, `i` Inbox, `Space` Search commands, `?` All keys, `z` Collapse sidebar, `,` Settings |
| Up, Down, Enter | Choose and run a row |
| Esc | Close the Menu |

"Shell here" starts a plain shell where you are (the same as `n`); "Session" starts an agent
straight away (leader `s`). You can also type `claude` in the shell yourself: it starts in its own
worktree in a git project.

## Worktrees second step

| Keys | Action |
| --- | --- |
| n, s, m, d | New worktree, switch via Inbox, open Changes to merge, remove worktree |
| Backspace, Esc | Return to the key map, or close |

## Sidebar keyboard focus

The sidebar takes navigation only. No letter acts from it: every action is the leader (`Ctrl+Space`) and its key, such as `n` new shell, `s` new session, `x` close, `m` Menu, `Enter` message the agent, `y` why, `k` stop the agent. Any other key leaves the sidebar and types into the pane. The row's right-click menu runs its items by letter while it is open.

| Keys | Action |
| --- | --- |
| Up / k, Down / j, Home, End | Browse rows, first row, last row; active session stays selected |
| Right, Left | Expand project or enter session; fold project or go to session's project |
| Enter | Toggle project or open selected session; opening a sleeping session wakes it. On a session whose check failed, could not run, was overridden, went stale or is never run for its repo, open its evidence sheet instead; on one whose repo command waits for approval, open the approval |
| Esc | Return keyboard focus to the active pane |

## Text prompts and worktree picker

| Keys | Action |
| --- | --- |
| Type, Backspace | Edit text |
| Up / Shift+Tab / Ctrl+P, Down / Tab / Ctrl+N | Move through worktree-picker results |
| Ctrl+U in text prompts | Clear prompt text |
| Enter, Esc | Open the selected result / submit prompt, or cancel |
| y / Y / Enter in close-group or remove-worktree prompt | Confirm; f / F forces worktree removal when offered; other keys cancel |

## New session (`o`)

`s` starts a session at once with no dialog (Settings › Sessions sets the agent and where); `o`
opens this dialog. `n` and the `+ new` chip start a plain shell, not a session.

| Keys | Action |
| --- | --- |
| Down / Tab, Up / Shift+Tab | Next or previous row (Agent, Where, Project) |
| Left, Right | Pick the row's option; on Project, the previous or next project |
| Enter, Esc | Start, or cancel |

## A session wants to start helpers

Enter on its sidebar row (or a click on the `wants to start N helpers` line) opens the question.
No bare key answers from the sidebar.

| Keys | Action |
| --- | --- |
| 1 | Allow |
| 2 | Always in this project |
| 3 | No, do it alone |
| Esc | Later: the row keeps asking |

## Side panel

The panel docked right (Inbox, Changes, Files, checks, Branch). Its width is kept for each kind of view.
Files, Changes and Branch show a tab strip at its top, `Files f · Changes c · Branch b` with
your own leader keys; the Inbox and checks have none.

| Keys | Action |
| --- | --- |
| [ , ] | Narrow or widen it by 4 columns (in the Inbox, while its search is empty; in Files, not while filtering or editing) |
| = | Back to its default width |
| Drag its left edge | Resize it |
| Click a pane | Type into that pane; the view stays open, unlit |
| Click the panel | Give the view the keys again |
| Tab, Shift+Tab | Files, Changes, Branch: the next or previous tab, for the same checkout (not while Files filters or edits) |
| Click a tab | Switch to it |

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
| Enter | Send follow-up, or queue while working; on an empty box it only closes |
| Ctrl+Enter | Send immediately, including while agent is working |
| Shift+Enter, Alt+Enter | Insert a newline |
| Ctrl+U | Clear the message |
| Esc | Close the box and keep the draft for that session until it is sent or cleared |
| i, Esc in evidence | Close status explanation; other keys close it and perform their normal action |
| Enter, r, O in a check's evidence | Open its evidence sheet, run it again, override it (asks first); only the keys the popover shows act |

## Files tree and preview

| Keys | Action |
| --- | --- |
| Up / k, Down / j, Home, End | Choose file |
| Right / l, Left / h, Enter on directory | Expand, collapse / parent, toggle directory |
| Enter on file | Insert quoted path into agent prompt and close Files |
| r, / | Toggle recent/tree, filter paths; Enter ends filtering, Esc clears filter |
| e, i, o, y, d | External editor, edit in preview, open outside, copy path, Changes |
| m on a markdown file | Show it as text or drawn (drawn is the default) |
| PgUp, PgDn, Shift+Up, Shift+Down | Scroll preview pages or individual lines |
| Right / l on file | Focus preview; arrows / j / k scroll, Space pages down, g / G or Home / End go to ends |
| Left / h / Esc in preview, or click ‹ | Return to tree (on a narrow panel the file takes the whole panel) |
| Drag or double-click over the file, then Ctrl+C | Select its text (a word on double-click) and copy it; `y` still copies the path |
| Click the selected file, or double-click a file, on a narrow panel | Open it in the panel |
| f, Esc in tree | Close Files |

## Built-in file editor

| Keys | Action |
| --- | --- |
| Arrows, Home, End, PgUp, PgDn | Move cursor or page |
| Shift + those keys, Ctrl+A | Select as the cursor moves; select the whole file |
| Drag, click, double-click | Select with the mouse; move the cursor (Shift+click extends); select a word |
| Enter, Tab, Delete, Backspace | Newline, four spaces, delete forward, delete backward; typing or deleting replaces a selection |
| Ctrl+C, Ctrl+X | Copy, cut the selection (with nothing selected: the cursor's line) |
| Ctrl+V, the terminal's own paste | Paste text at the cursor, replacing a selection; never an image, never into the pane |
| Ctrl+Z, Ctrl+Y | Undo (a run of typing or deleting is one step), redo |
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
| Up / k, Down / j | Previous or next changed file |
| PgUp, PgDn, Space | Read diff; Space pages down |
| x, e, r | Mark reviewed, open editor, return to agent |
| c, p | Ask to commit every changed file; ask to push and open a pull request (y or Enter says yes) |
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

## Branch view

Leader + `b` opens it for the session you're on; `b` there used to open the branch switcher,
which is now its first row. In `[keys.prefix]` its action is `branch` (`switch-branch` still
loads). It has the keys while open; closing it returns to shortcut mode.

| Keys | Action |
| --- | --- |
| Up / k, Down / j | Choose a row: Switch branch…, or a snapshot |
| Enter on Switch branch… | Open the branch switcher |
| Enter on a snapshot | Restore it (asks first; refused while the agent works) |
| b, Esc | Close it |

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
| o | Open configuration file outside Maitty |
| u | Check for updates now |
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

No default key: search commands (leader, then `Space`) opens it as **Select text**.

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

Scrolled up, a pane shows **↓ Back to bottom** at its bottom right (with how many lines up,
in Maitty's own history); a click goes back to the newest output. An agent that scrolls itself
(a full-screen one, which the wheel pages) shows it too, and the click pages it back down as
far as the wheel paged it up; typing to the agent clears it, since the agent scrolls to its
prompt itself.

Leader `z` collapses the sessions into a strip of chips on the top row (click a chip to
focus it, `»` or `+n more` to expand); with the strip focused, `←` `→` and `1`-`9` switch
sessions. "Hide sidebar" (search commands) hides the sessions completely; `z` brings them back.

| Keys | Action |
| --- | --- |
| PageUp, PageDown at a prompt | Scroll history; full-screen programs keep these keys |
| Shift+PageUp, Shift+PageDown, Ctrl+Shift+Up, Ctrl+Shift+Down | Scroll half-pages or individual lines outside full-screen mode |
| Shift with mouse | Keep selection/scrolling in Maitty when program captures the mouse |
| Ctrl+click | Open a path or link |
| Shift+right-click | Maitty menu even when program handles right-click |
| Double-click, drag | Copy word/path/link, select and copy text |

## Menus, history, splash and confirmations

| Keys | Action |
| --- | --- |
| Up / k, Down / j, Enter, shown letters | Choose or run a context-menu item; Delete asks to close where offered |
| 1–9 in numbered menu | Choose numbered option, such as an agent or snapshot |
| Up, Down, Enter, c, Esc in history | Choose event, go to its session, clear events, close history |
| Up / Left / k / Shift+Tab, Down / Right / j / Tab in splash | Choose splash action |
| r, n, o, comma, ? in splash | Resume if available, new session, open a project, Settings, All keys |
| Enter / y / shown confirm key | Confirm action; closing panes also accepts x or Delete |
| Esc / n / q in standard confirmation | Cancel action |
| Tab in folder picker | Complete current directory; arrows choose; Enter opens; Esc goes back |
