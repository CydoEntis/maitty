# Maitty Desktop (layout 2e): design handoff

This is the new desktop layout for Maitty. It replaces the current workspace with a simpler three-part app: a task list, the chat, and a slide-out side panel for changes, files and terminals.

**How to use this handoff:** read this README from start to finish, then open `Maitty Desktop.dc.html` in Chrome and check every section against it. The HTML is the source of truth for anything visual: sizes, colours, spacing, radii, shadows, motion and copy. This README explains the structure, the behaviour and the rules. If the two disagree, trust the HTML for looks and this README for behaviour, and flag the mismatch.

## Files

- `Maitty Desktop.dc.html`: an interactive mock of the whole window. Keep the folder together.
- `support.js`: the runtime the HTML needs. It isn't part of the design.
- `sidebar-marks.json`: the Claude and Codex marks the mock loads.
- `assets/`: the app logo and the captain and crew avatars used on crew tasks.

## Using the mock

- **Demo bar:** the bar at the top centre is a review harness, not product UI.
  - **Style buttons:** Glass, Frosted, Mono Type, Console and Retro Pixel switch the window style.
  - **Midnight / Day:** switch the theme.
  - **Demo:** **Step** and **▶ Play** take the task "Add rate limiting to the API" through Idle → Working → Finished → Needs you, so you can watch status changes animate.
- **Everything else is clickable:** rows, menus, right-click, splits, drags, the side panel, terminals and Settings.

Check at a few window sizes, for example 1600×1000 and 1200×760. The layout adapts below about 1200 wide and 760 tall.

## The model

- **Task:** one agent chat working on one branch in one folder. A task is what the sidebar lists, and it's named after the work ("Add a refunds endpoint"), not after the agent.
- **Subagent:** helpers a task starts. They show as a tree under their task.
- **Crew task:** a task run by a captain with crewmates. It uses the avatars in `assets/` and has a Crew view.
- **Terminal:** a shell that belongs to a task, in that task's folder. Terminals are not sidebar rows.

## Layout

From left to right:
1. **Sidebar:** the task list.
2. **Chat area:** one to three chat panes, each optionally with its own terminal pane underneath.
3. **Side panel:** slides out over the chat when opened.

The window has its own title bar with the logo and the window controls. The title bar has no background of its own.

## Sidebar

- **Top:** **New chat** (⌘N) and **Search** (⌘K).
- **Filters:** **All projects ▾** and **Group by ▾**, as quiet ghost buttons.
- **Group by Status (default):** sections in this order, each with a hint and a fold chevron:
  - **Needs you:** waiting on your answer
  - **Working:** running now
  - **Finished:** ready to review
  - **Idle:** agent stopped, nothing to do
- **Group by Project:** one heading per project, one level deep.
  - **Remote projects:** the heading gets the machine and an online dot, e.g. **docs-site · devbox ●**.
  - **A project on two machines:** it gets one heading per machine.
- **Group by Machine:** groups tasks by the machine they run on.
- **Task row:** agent icon with a status badge in its corner, task name, a project chip on the line below, and a time on the right.
  - **Selected row:** raised.
  - **Subagents:** listed under the row with their own status.
- **Status badges:**
  - **Working:** spinning teal ring
  - **Needs you:** amber square with **!**, pulsing
  - **Finished:** green dot with ✓
  - **Stopped / failed:** red square with ×
  - **Idle:** grey hollow ring
- **Hover card:** hovering a row shows a card to the right of the sidebar, top-aligned with the row and kept inside the window.
- **Footer:** the version / update chip and a Settings cog (⌘,).
- **Right-click a task:** Open, Open in split, Show changes, Rename…, Copy folder path, Mark as done, Stop agent, Close chat….
  - **Close chat…:** asks first. The agent stops, but its branch and changes are kept.
- **New chat:** adds a "New chat" row at the top of the list and opens it.
- **Motion:**
  - **Status change:** the row slides into its new section with a brief teal flash.
  - **Removed row:** the rows below slide up.

## Chat pane

- **Header, left:** the agent icon with its status badge, the task name in bold, and the folder path and branch (⑂, purple) on the line below. A long name ends in "…" before anything on the right is squeezed.
- **Header, right (icons only, no counts):**
  - **Split:** opens another chat pane.
  - **Chat / terminal switch:** shows the agent as a chat or as its own terminal screen.
  - **Side panel:** opens Changes, Files and Terminal. Has a small green dot when files have changed, and the tooltip gives the details.
  - **Bottom drawer:** only shown when terminals are pinned under the chat.
- **Chat body:**
  - **Your messages:** bubbles on the right.
  - **Agent work:** collapsed into one summary line ("Read 2 files, edited 3, ran 1 command · 3m 40s ›") that opens to show the steps.
  - **Changed files:** a card ("3 changed files +111 −0 ›") that opens the side panel on Changes.
  - **Running work:** a teal spinner line.
- **Message box, three layers:**
  1. **Needs-you tray:** rises out of the top of the box when the agent asks something. Amber, with keyed answers (**Yes y · Yes, don't ask again a · No n**).
  2. **Message box:** with a teal send button.
  3. **Settings tray:** a narrower tray underneath with the model, permission mode (**Ask before edits**), reasoning level and branch.
- **New chat:** shows "What should Claude work on?", a sentence line ("on This computer · in acme-store · using Claude · on main") and three suggested tasks. Picking one starts the agent, renames the row and moves it to Working.
- **Terminal view (from the switch):** the same conversation drawn the way the agent's CLI draws it, with ● steps, results underneath and the numbered approval box.

## Splits

- **Opening:** split opens the next task beside the current one, up to three panes.
- **Focus:** click a pane to focus it. It gets a teal ring, and clicking a task in the sidebar loads it into the focused pane.
- **Resizing:** drag a pane's left edge (minimum 320px). Adding or closing a pane resets the widths.
- **Closing:** remaining panes refill the space with an animation.
- **Reordering:** drag a pane by its header onto another pane to swap them. The dragged pane fades, the target shows a dashed "Drop to swap", and both panes glide to their new places over about 0.3s.
- **Only the rightmost pane shows the side-panel button.**
- **Each pane is independent:** its own view switch, its own terminal pane and its own needs-you tray.
- **Right-click a pane:** Split right, Changes and files, Pin terminals under chat / Move terminals to side panel, Rename…, Close pane.

## Terminals

Terminals live in the side panel by default.

- **Side panel Terminal tab:** a tab for each terminal, **+** to add one, × to close one, and full-height output.
- **Pin under chat:** moves the task's terminals into their own pane under the chat pane.
  - **Look:** a separate glass card with the same edge and finish as the chat, with a gap between them.
  - **Header:** Terminals ▾, the terminal tabs, **+**, the folder path, a button to move them back into the side panel, and ×.
  - **Resizing:** drag the handle on top. The terminal pane is 120–640px tall and at most 60% of the height, and the chat card shrinks to match.
  - **Folding:** folds down to its 42px header. On short windows it starts folded.
- **New agent ▾ → New terminal:** adds a tab to the current task's terminals.

## Side panel

- **Opening and closing:** it slides in from the right over a dimmed strip of the chat, at about three-quarters of the window width and never under the sidebar. Close it with ×, Esc, or a click on the strip.
- **Header:** only the tabs **Changes 3 · Files · Terminal** and ×.
- **Changes:**
  - **Left column:** the branch at the top ("⑂ feat/refunds from main"), then the changed files with A/M and +/−. At the bottom: a commit message suggested by the agent, **Commit N files** and **Create PR**.
  - **Right side:** the full-size diff with line numbers, green for added lines and red for removed.
  - **File bar above the code:** the path (folder dim, name bright), an Added/Modified pill, the +/− counts and "Click a line to comment" on wide windows. The path keeps priority and ends in "…" before shrinking.
- **Comments:** click a diff line to open a comment box under it, with **Send to Claude** (or Codex).
  - **After sending:** the comment stays pinned to the line ("applying…") and appears in the chat as your message.
- **Files:**
  - **Left column:** "Changed by the agent" at the top, then the full tree. Folders open and close, files have type tags (TS, TSX, { }, MD), and changed files get A/M with a dot on their parent folders.
  - **Right side:** clicking a file opens it with syntax highlighting. A changed file opens as its diff.
  - **On open:** it shows the first changed file, or the README.
- **Agent questions don't appear in the side panel.** They stay in the chat and the sidebar.

## Settings

Open with the cog or ⌘,. The sections are General, Appearance, Layout, Agents and Shortcuts.

- **Layout:**
  - **Terminals open in:** Side panel or Under the chat.
  - **Group tasks by:** Status, Project or Machine.
- **Appearance:**
  - **Theme:** Midnight or Day.
  - **Window style:** Glass, Frosted, Mono, Console or Retro Pixel.

## Themes and window styles

All colours come from tokens. The Midnight and Day sets are in the HTML's logic (`DV_DARK`, `DV_LIGHT`), and each window style overrides some of them (`DV_STYLES`). Build every component once and have it read the tokens, so all five styles and both themes work by swapping tokens alone.

- **Glass (default):** the chat pane and terminal pane are see-through frosted cards (about 70% of the main colour plus a backdrop blur) with a soft top highlight. A faint accent glow sits behind the window so the frost has something to show.
- **Mono and Retro Pixel:** square corners everywhere. Pixel also has thicker borders.
- **Accent rules:**
  - **Teal:** working and focus.
  - **Amber:** only for needs you.
  - **Green:** done and added lines.
  - **Red:** stopped and removed lines.

## Motion

Keep everything short and ease-out.

- **Side panel:** slides in over a fading backdrop.
- **Menus:** pop open.
- **Needs-you trays:** rise.
- **New panes:** fade in.
- **Spinners:** spin.
- **Status changes and swaps:** rows and panes glide to their new positions.
- **Reduce motion:** should turn all of these off.

## Not built yet

- **Message box tray:** the model, permission, reasoning and branch items are labels only; they don't open dropdowns yet. Reuse the pickers from the current desktop app.
- **Hard-coded icon colours:** a few, such as the Codex mark and some small tool icons, don't follow the theme.
- **Settings:** only Layout and Appearance change the app. The other sections show their options but don't do anything.
- **Sample text:** comments and suggested commit messages use sample text.

## Verification checklist

- [ ] Sidebar sections, hints and badges match the descriptions above, in Status, Project and Machine grouping.
- [ ] Hover card opens beside the hovered row and stays inside the window.
- [ ] Right-click menus on tasks and panes have the items listed above.
- [ ] Split to three panes, resize, close one, and swap two by dragging. Each move animates.
- [ ] Side panel opens and closes with ×, Esc and the dimmed strip. Changes, Files and Terminal all work.
- [ ] Clicking a diff line opens a comment box, and sending puts it in the chat.
- [ ] Pin under chat gives a separate terminal pane under the chat that resizes and folds, and it moves back into the side panel.
- [ ] Demo ▶ Play moves the rate-limiting task through all four states.
- [ ] All five window styles and both themes look right, with no hard-coded colours in new code.
- [ ] At 1200×760 nothing overlaps and the header icons stay visible.
