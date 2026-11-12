# Maitty 2g: change handoff

This is a **change list on top of the 2f handoff** (which sits on top of 2e). Anything not listed here is unchanged.

**How to use this handoff:** read this README from start to finish, then open `Maitty Desktop 2g.dc.html` in Chrome (keep the folder together) and check each item against it. The HTML is the source of truth for anything visual: sizes, colours, spacing, radii and motion. This README is the source of truth for behaviour. If they disagree, flag it.

## Files

- `Maitty Desktop 2g.dc.html`: the full mock with these changes.
- `support.js`, `sidebar-marks.json`, `assets/`: needed for the HTML to work. They aren't part of the change.

## Demo controls (not product UI)

The **Demo controls** pill at the top centre is only for reviewing the mock. **Don't build it.**
- **Style / Theme:** switch the window style and Midnight / Day.
- **Status:** Step and ▶ Play move one task through Idle → Working → Finished → Needs you.
- **Problem:** cycles through the three problem states in section 4.

---

## 1. App frame fix

The app fills the window at any size (`100vw × 100vh`). An old fixed 1440 × 900 frame was removed, because it clipped the header and the tray under the message box on shorter screens. The page itself never scrolls.

## 2. Sidebar task row

- **Layout:**
  - **Icon:** 26px, vertically centred.
  - **Top line:** the project's colour square and name on the left, then the branch on the right in mono, using `--branch`. The branch has an icon: ⑂ for main/dev/master, ⧉ for worktrees. It shortens with "…" first and takes at most 58% of the width.
  - **Second line:** the task name on the left, and the running time on the right. On hover, the time is replaced by the close ×.
- **Removed:** the unread dot.
- **Working badge:** a teal rounded square (3px radius) with a small spinning ring inside in `--on-accent`. The square itself doesn't move.
  - **Other badges, unchanged:** needs you is an amber square with !, stopped is a red square with ×, finished is a green dot with ✓, and idle is a grey ring.
- **Group headings:** never wrap. The hint text ends in "…". The **Needs you** heading shows a **⌘J** key hint.

## 3. ⌘J: next chat that needs you

**⌘J** (Ctrl+J) opens the next chat in the Needs you group (needs you or stopped), in sidebar order, and wraps around. It marks that chat as seen.

## 4. Problem states

A banner rises out of the top of the message box, in the same style as the needs-you tray. Each has a coloured tile, a title, one line of detail, and buttons.

| State | Where | Title / detail | Buttons | Also |
|---|---|---|---|---|
| Agent crashed | the crashed chat | "Claude stopped unexpectedly" / "The process exited (code 137, out of memory). Your branch and changes are kept." | View log, **Restart** | Sidebar badge turns red (stopped). The older "Stopped" banner is hidden while this one shows. |
| Machine offline | every chat on that machine | "devbox is offline" / "Lost the connection 40s ago. Trying again in 8s. The agent keeps running on devbox." | **Retry now** | That machine's rows fade to 50% and show "offline" instead of a time. The machine dot on its project headings turns red. |
| Context full | that chat | "Context is almost full · 96%" / "The agent will start forgetting the earliest messages soon." | New chat with summary, **Compact** | The Context ring reads 96% in amber. |

Colours: red (`--danger`) for crashed; amber (`--warn`) for offline and context full.

## 5. Message box and the tray underneath

- **Message box:** two rows.
  - **Top:** the text.
  - **Bottom:** **+** (attach) on the left, then the **model button** (agent icon, model name, four effort bars and a chevron) and send on the right.
- **Tray underneath, left to right:**
  1. **Project:** folder icon and name. The full path is in the tooltip.
  2. **Branch:** a button in `--branch` mono, with the worktree or branch icon and a chevron. It shortens with "…". It opens the **This chat works on** menu:
     - the current branch with ✓, the task's own branch, main and dev (worktrees are labelled "worktree")
     - a divider, then "Move to a new worktree…"
     - a note: "Switching changes where the agent's next edits go. Work already done stays on its branch."
     - The menu opens **upward** and sits above the message box.
  3. A divider, then **Permissions** (unchanged from 2f).
  4. A spacer, then three **usage rings** on the right.
- **Usage rings:** each reads **name, %, ring**, in 11px with tabular numbers. The ring is 11px. Each has a tooltip.
  - **Context:** this chat's memory used. The ring is green, and turns amber (ring and number) at 80%. Tooltip: "How much of this chat's memory is used: N%".
  - **5h:** the 5-hour limit. Tooltip: "5-hour limit: 62% used · resets in 2h 14m".
  - **Week:** the weekly limit. Tooltip: "Weekly limit: 49% used · resets in 1d 23h".
- **The sidebar footer** is just the version chip and the settings cog. The usage meters are no longer there.

## 6. Chat area

- **Fade under the header:** the chat scroll area fades to transparent over its top 36px (a CSS mask), so messages dissolve under the header instead of being cut off. The header itself isn't faded.
- **Helpers:** the "2 helpers" card is now one line: ↳ **2 helpers** · 1 working · 1 done ›. The helpers themselves are listed under the task in the sidebar.

## 7. Crew task

The crew task "Onboarding polish" is back in the task list.
- **Sidebar:** a normal row with the captain avatar, and its crewmates in the subagent tree underneath.
- **Side panel:** has a **Crew** tab first, before Changes, Files and Terminal.

## 8. Faster new chat

This replaces the two-step flow from 2f.

- **New chat opens ready to type:** it starts on the most recent project, on that project's current branch, with **Own copy** on. There's no folder step. Step 1 (the search and recent list) only appears if there are no recent projects.
- **Heading:** "What should we work on?" (from 2f).
- **Under the heading, three controls:**
  1. **Project ▾:** the project dropdown from 2f, with search, recents and Browse.
  2. **Branch ▾:** the branch menu from 2f.
  3. **⧉ Own copy:** an on/off **switch** that replaces 2f's worktree menu.
     - **On (default):** the agent works in a new worktree.
     - **Off:** the agent edits your checkout directly.
- **Sync line under the controls:**
  - **Own copy on:** a green dot and "Starts from latest origin/<branch> · fetched just now". Maitty fetches before creating the worktree.
  - **Own copy off, checkout up to date:** a green dot and "Up to date with origin/<branch>". After a pull it reads "Pulled · up to date…".
  - **Own copy off, checkout behind:** amber **↓N behind origin/<branch>**, and a **↓ Pull** button that runs `git pull --ff-only`.
  - **Uncommitted changes:** Pull is disabled at 50% opacity, with the tooltip "You have uncommitted changes. Commit or stash first."
- **Switch to:** chips for up to four other recent projects.
  - **Each chip:** colour square, name, "· machine" for remote projects, the branch in `--branch` mono, and **↓N** if that checkout is behind.
  - **Tooltip:** the path · machine.
  - **Clicking a chip:** switches project.
- **Suggested tasks:** unchanged, below the chips.

**Mock data to check against:**
- **acme-store / main on This computer:** 3 behind.
- **billing-api:** has uncommitted changes, so Pull is disabled.
- **Remote copies:** never show a behind count in the mock.

## 9. Tokens

- **New token `--branch`:** for every branch name and icon. **#A78BFA** in Midnight, **#6B46C1** in Day (readable on light backgrounds). There are no hard-coded purples left.

---

## Not done yet

- **New chat keyboard:** ↑↓, Enter and Tab on the new chat screen are not in the mock yet. Planned: ↑↓ to move through recents, Enter to pick, Tab to jump to the branch.
- **Day theme:** not reviewed across the new pieces (problem banners, branch menu, rings, Own copy switch).
- **Buttons with no action:** View log, Retry now, Compact and "Move to a new worktree…" just dismiss in the mock. Hook them to the real actions.

## Checklist

- [ ] Window fills the screen at 1200×760 and 1600×1000 with nothing clipped.
- [ ] Sidebar row layout and badges match section 2. Headings never wrap.
- [ ] ⌘J moves through the Needs you chats in order and wraps around.
- [ ] All three problem states show the right banner and sidebar changes, and clear when acted on.
- [ ] The branch menu in the tray opens above the message box and switches branch.
- [ ] Usage rings read name, %, ring, with tooltips. Context turns amber at 80%.
- [ ] The chat fades under the header. Helpers is one line.
- [ ] The crew task appears, with its tree and a Crew tab.
- [ ] New chat opens on the last project. Own copy toggles the sync line. Pull works and is disabled when there are uncommitted changes. Switch-to chips show the machine and behind counts.
- [ ] Everything is checked in Midnight and Day and in all window styles, with colours only from tokens.
