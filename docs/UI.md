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
`client/design.rs` and `client/hydra/`: `panel()` (popup), `fill`, `put`/`seg`
(text runs), `keycap`/`keycaps`, `hints`, `button`, `dim_all`. Icons come from
config (`[icons]`), with Nerd Font glyphs where present.

## Project-specific: layout and navigation

<!-- local: user requested sidebar-bottom controls, tab-row update discovery and no global footer. -->

Seshi has one terminal interface: a sidebar of projects and sessions, with tabs and split panes in the remaining area. Settings > Appearance > Layout selects Cards or Flat; both use the same sessions, tabs, splits and sheets. Cards is the default. Flat uses square, full-bleed zones and Tab / Shift+Tab to move focus between the sidebar and panes; Esc returns focus to the sidebar. There is no application-wide footer bar. The tab row aligns with the sidebar. Keep **+ new** and **Actions** at the bottom of the sidebar in both layouts. The tab row shows **Update** only when a newer release is available.

The leader is `Ctrl+Space` by default. Inbox (`j`) uses the right-hand sheet. Settings (`,`) opens as a centred modal over the workspace so the pane layout does not change. The command palette (`Space`), shortcuts (`?`), files, branches and new agents use popups. The minimum supported size is about 80 by 24; popups clamp to the screen. Mouse users can reach every visible control; support click, right-click menus, drag dividers, scrolling and Ctrl+click paths. Prefer Seshi's retained history when it has lines; when an agent has no retained history, pass wheel and Page Up/Down input to the agent so its own transcript can scroll.

The high-fidelity flat-layout reference and its interaction spec are preserved in [`docs/design/flat/README.md`](design/flat/README.md); update it when the implementation changes the design contract.

The sidebar's **+ new** uses the same shell action as leader + `n`, including its configured key hint. **Actions** is in the sidebar footer and opens status setup, Settings and All commands. Every palette row is clickable and wheel-scrollable. Include configured commands and retain existing confirmations.

When an update is available, show **Update <version>** in the tab row and open the existing confirmation flow when clicked. Do not show the control when Seshi is current. Keep status icons and pane status tags clickable to open the evidence explanation. Terminal bells do not add sidebar indicators or history entries.

The evidence popover (`i`, or a click on a status) explains a session's status from its hook, screen and process reports. For a session with a check, it shows a timeline instead: when the agent's turn ended and what it said, when Seshi ran the command, and the result with exit code and duration, in local time (an age on Windows). Its keycap hints list only the keys that apply: `⏎` evidence and `r` re-run once there is a result, `O` override for a failed one, `Esc` close. A session without a check keeps the source rows.

Mark one selected sidebar row independently from hover: the keyboard selection stays selected when the mouse moves; hover is a separate, quieter tint. The open session keeps a dim selection bar while another row is under the keyboard. Preserve agent status icons and colors in each state.

Settings opens on General and reports installed, window and daemon versions separately, with a labeled remote daemon and restart-pending notice when they differ. General groups controls under INPUT, LAYOUT, SHELL and UPDATES. Show the update action only when an update is available; keep release notes and confirmation in the existing flow.

Show reported child agents under their parent, never as independent projects. Keep the sidebar list live: remove a child when its reported state is completed, failed, cancelled or unconfirmed, while retaining lifecycle evidence in daemon state. Make active child rows clickable to inspect reported task and stage details. Keep parent session status colors unchanged. Active subagents use muted neutral gray with no per-helper animation; their compact task or role label is muted, dim and non-bold, with no generic gear icon. Use the provider-reported task when available; otherwise show a reported current tool action, recognizable role stage, meaningful reported role name, or `Working` when none is available. Preserve standard needs-you and error colors for blocked or failed children. Do not show lifecycle group labels or counts beyond the parent heading total. Do not assign role colors that could be confused with done, working, blocked or idle.

Keep update indicators until the installed program, window and daemon versions agree. Installing an update leaves open sessions running. For a local daemon, when no panes are open, restart it automatically and reconnect the window so daemon fixes take effect without `kill-server`. For a remote daemon, explain that it must be updated on its host after its sessions are closed.

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

A session with a check result (`TermInfo.check`) swaps its status glyph for the
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
- ASCII glyphs are used when the `blocked`, `done` and `idle` icons are all ASCII.
- The line is dropped below `NARROW` columns, before the glyph; the folder name is
  never touched. A long command is shortened first so the result still reads.
- The collapsed rail shows the same glyph. The ring turns with the motion setting;
  the working spinner choice is unchanged.
- Colours are existing theme slots only; there is no check role. Code lives in
  `client/hydra/check.rs`.

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
