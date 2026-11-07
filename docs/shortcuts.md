# Shortcuts

Every default shortcut, then the keys each screen has of its own. You can change any of them
in Settings › Keys or under `[keys.prefix]` in your config.

## Default shortcuts

**Inside a pane only these keys are taken; everything else goes to the program**, including
`Ctrl+C`, `Ctrl+D`, `Ctrl+Z`, `Ctrl+L`, `Ctrl+R`, the arrows, `Tab` and `Esc`.

| Key | Action |
|---|---|
| `Ctrl+Space` | Key mode: the keys below until `Esc` (configurable as `prefix`) |
| `Ctrl+T` | New shell where you are, at once (the prefix then `t` where the terminal cannot send it) |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous session of the sidebar |
| `Ctrl+K` | Search commands |
| `Ctrl+J` | Inbox |
| `Ctrl+Shift+F` | Find in the pane |
| `Alt+Down` / `Alt+Up` | Next / previous session |
| `F2` | Rename the selected session |
| `Alt+1`-`9` | Go to that session of the sidebar |

**`Ctrl+A` still opens key mode for one release**, so a muscle memory or a script keeps working.
It stops working when the prefix is set to another key on purpose or `Ctrl+A` is bound to something
else under `[keys.global]`. The keys follow the TUI redesign: `n` is the New session dialog (`t` a
shell at once), `j` / `k` step through the sessions (`n` / `p` stepped through tabs, and there are no
tabs), `s` / `S` split right and down (they were `|` and `-`), `z` zooms the pane (undo close is
`Ctrl+Z`), `/` filters the sidebar (find in the pane is `Ctrl+Shift+F`), `u` is usage and cost
(Updates is in the release chip's menu), `k` (stop the agent) is `Ctrl+C`.

Press the prefix for key mode, then the key. Only `?`, `S` and `_` need Shift, and each key does one thing.

### Sessions

| Key | Action |
|---|---|
| `n` | New session (the dialog) |
| `t` | New shell where you are, at once |
| `j` / `k` | Next / previous session; key mode stays, so `j` `j` walks the list |
| `1`–`9` | Open that session of the sidebar |
| `r` | Rename the session |
| `/` | Filter the sidebar |
| `i` | Inbox: what needs you, then every session |
| `Enter` | Message the agent |
| `Ctrl+C` | Stop the agent |
| `Ctrl+Z` | Undo the last close (six seconds) |

### Panes

| Key | Action |
|---|---|
| `s` / `S` | Split right / down |
| `_` `\` | Split up / left (the new pane goes before the focused one) |
| `Tab` | Focus the next pane |
| `←` `↓` `↑` `→` | Move to the pane that way (left past the edge: the sidebar) |
| `z` | Zoom the pane: it fills the view until pressed again |
| `x` | Close what has the keys: the side panel (Files, Changes, Branch, a check) when it has them, else the focused pane, with an undo (a running agent asks for the same press again) |
| `m` | Move the pane: trade places with the next one of the view |
| `Ctrl`+arrows | Resize the pane |
| `Alt`+arrows | Swap the pane with the one that way |
| `PgUp` / `PgDn` | Scroll half a pane |

### Sidebar

| Key | Action |
|---|---|
| `b` | Hide the sidebar (again brings it back; so does its `»` chip) |
| `[` / `]` | Narrow / widen the sidebar |

### Tools

| Key | Action |
|---|---|
| `f` | Files |
| `c` | Changes |
| `g` | Branches (in the side panel; its first row switches branch) |
| `h` | History: what happened |
| `e` | References (in the side panel) |
| `o` | Find a file |

### Maitty

| Key | Action |
|---|---|
| `Space` | Search commands |
| `.` | Menu |
| `u` | Usage and cost: what each agent reported using, and the plan limits |
| `,` | Settings |
| `?` | Keys help |
| `w` | Machines: the computers reached over ssh; add, retry or remove one |
| `Ctrl+V` | Paste the clipboard's image |
| `d` | In a pane on another machine: send a file from this computer (type or drop its path; again while it sends stops it) |
| `q` | Step away (agents keep running) |
| `Ctrl+R` | Reload the config |

**Search only** (`Ctrl+K`, then type): worktrees, new tab, Updates, find in the pane, previous / next
command in history, send the prefix itself to the program.

## How key mode works

The prefix is `Ctrl+Space`. Press it and you are in **key mode**: each key runs its action, and the
mode stays for the next, so a run of actions is one key each. `Esc`, the prefix again or a click go
back to the pane, where keys type into the program. So does an action that starts a pane for you to
type into: `t` a shell, `s` / `S` a split, and the dialog once it starts the session; the next key goes
to the new pane. An action that opens a popup or panel hands it the keys, and closing it returns to key
mode; `Enter` (message the agent) is one of those. The status bar along the bottom always says which
mode you are in: ` PANE ` (accent), the session and its folder, then `j/k sessions  ^space keys  ? help`;
in key mode ` KEYS ` (yellow) and `n new  s split  x close  tab pane  ? help  esc back`, each a click
that presses it, and a click on ` PANE ` starts key mode. While something other than the panes has the
keys (the sidebar, a popup, Files, …) the chip names it and its keys sit at the right with the way
out last, such as ` SIDEBAR ` … `↑↓ move  enter open  n new  esc back`, adding `shortcut mode waits`
when key mode comes back after. In key mode the focused pane's border is sky blue, and a key that does
nothing shows "Shortcut mode — Esc to type".
To send `Ctrl+Space` itself to the pane, search commands for "Send the leader key" or bind
`send-prefix` to a key. One overlay holds every key reference; it is not a second palette:

- **Key map** (the prefix, then a pause, with Settings › General › **Leader waits for next
  key** on; it is off by default): the keys by group, built from your real bindings. Pressing a key
  runs it.
- **Search commands** (`Ctrl+K`, `Space` after the prefix, `Tab` from the map, or typing a letter
  that is not a key): every command, including ones with no key, and your sessions. Matching is
  an in-order subsequence (`spld` finds Split down). With nothing typed it lists the commands you ran last.
- **Keys help** (`?` after the prefix, or the sidebar's menu → Keys): three columns, sessions, panes
  and window, each key as it is bound now; under them "Keys only work in key mode (Ctrl+Space).
  Everything else goes to the pane." `esc close` on its top border closes it.

Keys are drawn as keycaps in the key map and dialogs; the status bar and the keys help spell them
short (`^space`, `tab`, `esc`). On Windows, Caps Lock does not change shortcut meaning, and no default
but `?`, `S` and `_` needs Shift. The [README](../README.md#shortcuts) lists the defaults.

Mouse entry: the sidebar's ` menu ` chip; with the sidebar hidden, its ` » ` chip brings it back. Click
a row to run it; the wheel moves the selection. Close buttons dismiss overlays.

Keys below act inside the named screen, without the leader. Ordinary typing edits the
focused text field; Backspace removes a character. Keys sent to Claude, Codex, your shell
or another program follow that program's own help.

## Key overlay

| Keys | Action |
| --- | --- |
| A bound key (key map) | Run its command and close |
| Tab | Switch between the key map and the search |
| Any unbound letter (key map or keys help) | Start a search with it |
| Type, Backspace, Ctrl+U (search) | Edit the words; Backspace on an empty box returns to the map |
| Up, Down, Ctrl+P, Ctrl+N, PgUp, PgDn (search) | Choose a result |
| Enter (search) | Run the selected command |
| Esc | Close; no command runs while reading |

Every key is a leader key: there is no letter that acts by itself in the sidebar. `Ctrl+V` pastes an
image, `d` sends a file to the pane's machine, `Ctrl`+arrows resize, `Alt`+arrows swap panes. Your own
`[keys.prefix]` entries still load.

## Menu

| Keys | Action |
| --- | --- |
| The key shown beside a row | Run it (the same leader key works anywhere) |
| Up, Down, Enter | Choose and run a row |
| Esc | Close the Menu |

"Shell here" starts a shell at once where you are (the same as `t`); "Session..." opens the New
session dialog, for another folder or machine. Start an agent by typing `claude` (or any other) in the shell: it starts in its own worktree
in a git project. Section headers (NEW, TOOLS, FIND, SIDEBAR) are not rows: the arrows skip them.

## Worktrees second step

| Keys | Action |
| --- | --- |
| n, s, m, d | New worktree, switch via Inbox, open Changes to merge, remove worktree |
| Backspace, Esc | Return to the key map, or close |

## Sidebar keyboard focus

The sidebar takes navigation and `/`. No other letter acts from it: every action is the prefix
(`Ctrl+Space`) and its key, such as `t` new shell, `n` new session, `x` close, `.` Menu, `Enter`
message the agent. Any other key leaves the sidebar and types into the pane. The row's right-click
menu runs its items by letter while it is open.

| Keys | Action |
| --- | --- |
| Up / k, Down / j, Home, End | Browse the sessions, first, last; the active session stays selected |
| Right | Enter the selected session |
| `1`–`9` | Open that session |
| `/` | Filter the list: type to narrow it by name, project, process, machine or branch; Enter opens the match under the cursor, Esc clears the filter |
| Enter | Open the selected session; opening a sleeping session wakes it. On a session whose check failed, could not run, was overridden, went stale or is never run for its repo, open its evidence sheet instead; on one whose repo command waits for approval, open the approval |
| Menu key (`.` after the prefix), right-click | The session's menu |
| `+` on a machine heading | A shell on that machine at once |
| Enter on a row of an agent a restart stopped | Types its resume command, then opens the session |
| Esc | Return keyboard focus to the active pane |

## Text prompts and worktree picker

| Keys | Action |
| --- | --- |
| Type, Backspace | Edit text |
| Up / Shift+Tab / Ctrl+P, Down / Tab / Ctrl+N | Move through worktree-picker results |
| Ctrl+U in text prompts | Clear prompt text |
| Enter, Esc | Open the selected result / submit prompt, or cancel |
| y / Y / Enter in close-group or remove-worktree prompt | Confirm; f / F forces worktree removal when offered; other keys cancel |

## New session dialog

Leader `n`, the Menu's `Session...` and the ` n new session ` chip of an empty workspace open it (`Ctrl+T`, leader `t` and a machine's `+` start a shell at once, with
no dialog). It starts a shell; step 1 is the project, step 2 the machine.

| Keys | Action |
| --- | --- |
| Type, Backspace, Ctrl or Alt+Backspace | Edit the search or path; the last also deletes a path segment |
| `~`, `/` or `.` first | A folder path on the chosen machine, completed from the real folders |
| Tab, Right | Accept the dim completion; with none, pick the project and go to step 2 |
| Ctrl+O | Browse: `Use <folder>`, `..` and the folders inside |
| Up, Down | Move through the list (the machines in step 2; offline ones are skipped) |
| Enter | Pick the project (starts the shell at once when this computer is the only machine); in step 2, start the shell on that machine. Blocked while the folder is missing |
| Esc | Back (step 2, the picker), then close |

The details are in [UI](UI.md#project-specific-new-session-dialog). Ctrl+T and leader `t` are not this dialog: they make a shell where you are at once.

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

The panel on the right. The checks dock beside the panes; Files, Changes, Branches and
References open from a pane's ` files ` and ` changes ` chips (or their leader keys) as a panel that slides in from the pane's right edge
inside its box; the panes are not narrowed by it, and ` » ` or Esc closes it. The Inbox is a panel 62 columns wide
over the workspace from the sidebar's side; it has a fixed width.

| Keys | Action |
| --- | --- |
| [ , ] | Narrow or widen it by 4 columns (not for the Inbox, which is fixed; in Files, not while filtering or editing) |
| = | Back to its default width |
| Drag its left edge | Resize it (not the Inbox) |
| Click a pane | Type into that pane; the view stays open, unlit |
| Click the panel | Give the view the keys again |
| Tab, Shift+Tab | Files, Changes, Branch, References: the next or previous tab, for the same checkout (not while Files filters or edits) |
| Click a tab | Switch to it |

## Inbox

| Keys | Action |
| --- | --- |
| Type, Backspace | Filter by project or session (the search row shows once you type) |
| Up, Down, PgUp, PgDn | Choose a row |
| Enter | The selected item's first button: Reply (a question), Read the command (a check waiting for approval), Output (a failed or could-not-run check), Re-run (a stale pass); an item with no buttons opens its session, as does a search result |
| o | Open the selected item's session; empty search |
| 1–9 | Answer selected agent question when search is empty |
| y, a, n | On a question: yes once, yes for the session, no, sent to that session's terminal as typed; empty search |
| c | Clear the finished and exited cards (each session stays); empty search |
| m | Follow-up to selected asking/finished agent when search is empty; on a failed check, the message with the failure written in (sent only on Enter) |
| d, M, x | Finished worktree: diff, ask to merge, ask to discard; empty search |
| d, m, k | Shared-file warning: diff, message its agent, dismiss; empty search |
| r, O, u, , | The selected item's check: run it again, override it (asks first), undo an override, its settings; only where they apply, else typed; empty search |
| Click a button | The key it shows |
| Delete | Mark selected finished session, exited session or unrun check, seen |
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
| e, i, o, y | External editor, edit in preview, open outside, copy path |
| m on a markdown file | Show it as text or drawn (drawn is the default) |
| PgUp, PgDn, Shift+Up, Shift+Down | Scroll preview pages or individual lines |
| Right / l on file | Focus preview; arrows / j / k scroll, Space pages down, g / G or Home / End go to ends |
| Left / h / Esc in preview, or click ‹ | Return to tree (on a narrow panel the file takes the whole panel) |
| Paste (terminal) | While filtering: its first line into the filter; otherwise kept from the pane (Esc closes Files, or click a pane to paste there) |
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
| Click Save, Copy, Paste, Undo, Done | The action row's buttons while editing: Ctrl+S, Ctrl+C, Ctrl+V, Ctrl+Z, Esc |
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
| m | A failed check: the agent's message box with its command, exit code and last 40 lines written in; sent only on Enter |
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

## References view

Leader + `a` opens the project's references (`.maitty/references/` at the repo's main checkout).
In `[keys.prefix]` its action is `references`. It has the keys while open.

| Keys | Action |
| --- | --- |
| Up / k, Down / j, PgUp, PgDn, Home, End | Choose a reference |
| Enter, o | Open it in the system viewer |
| p, Ctrl+V | Add the clipboard image as a screenshot |
| Drop a file on the terminal | Add it (the terminal pastes its path) |
| n | Add a file: type or paste its path, then Enter |
| d, Delete | Remove it (asks first) |
| y | Copy its absolute path |
| a, Esc | Close it |

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

Leader `z` collapses the sessions into a 10-column column of status glyphs and initials (click a row
to focus it, `»` or the `…` row to expand; under 110 columns it is collapsed by itself, and `z` then
opens the full sidebar over the workspace); with the column focused, `↑` `↓`, `←` `→` and `1`-`9` switch
sessions. "Hide sidebar" (search commands) hides the sessions completely; `z` brings them back.

| Keys | Action |
| --- | --- |
| PageUp, PageDown at a prompt | Scroll history; full-screen programs keep these keys |
| Shift+PageUp, Shift+PageDown, Ctrl+Shift+Up, Ctrl+Shift+Down | Scroll half-pages or individual lines outside full-screen mode |
| Shift with mouse | Keep selection/scrolling in Maitty when program captures the mouse |
| Ctrl+click | Open a path or link |
| Shift+right-click | Maitty menu even when program handles right-click |
| Double-click, drag | Copy word/path/link, select and copy text |
| Drag a pane's title onto another pane | Swap the two (Esc, or letting go outside the panes, cancels) |

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
