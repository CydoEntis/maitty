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

The terminal client has one layout: a sidebar of projects and their sessions (left or right), panes on
the rest (tabs and splits), and a one-line bottom bar. Everything after the leader
key (`Ctrl+Space` by default; keys follow herdr's where they overlap). Views:
inbox (`j`) and settings (`,`) use the right-hand sheet; Profiles and Workflows
are there too. The command palette (`Space`), keys (`?`), files, branches and new
agent use popups. Minimum supported size is about
80×24; popups clamp to the screen. Mouse: click, right-click menus, drag
dividers, scroll; Ctrl+click opens paths.

The sidebar's **+ new** uses the same shell action as leader + `n`, including
its configured key hint. Keep **Actions** available at the top right, including
without a sidebar or session. Offer clickable Profiles, Workflows, Runs, agent
status setup, Settings and All commands; every palette row is clickable and
wheel-scrollable. Include configured commands, retaining existing confirmations.

When an update is available, show the accent dot beside **Actions**, its
**Settings** row and the **General** settings tab to guide users to the update.

Retain the working, needs-you, done or idle icon when a terminal rings. Terminal
bells do not add sidebar indicators or history entries.

Keep the active sidebar session filled with `hov` and marked `▸`, independently
of mouse hover or keyboard browsing. Hover uses a quieter fill; the keyboard
target uses `›`. Preserve agent status icons and colors in each state.

Settings shows installed, window and daemon versions separately, with a labeled
remote daemon and a restart-pending notice when versions differ. Keep the update
dots until that mismatch is resolved. Installing an update leaves this window and
the daemon running. Never restart the daemon automatically, even on a protocol mismatch.
Status icons and pane status tags open the existing evidence popover, with a
plain-language explanation and actual report/observation ages.

**All keys** (`<leader> ?`, Actions or palette) opens a searchable, scrollable
reference. List every active configured leader/global binding separately,
including aliases and custom commands. Show palette-only commands without
inventing bindings. Keep the delayed leader overview compact. Contextual keys
come from [shortcuts.md](shortcuts.md), embedded in the app so both stay identical.

## Project-specific: UI conventions

<!-- local: user requested right-hand Profiles/Workflows sheets and distinct active, keyboard and hover sidebar states. -->

- Popups are centred `panel()`s with an accent title bar and `Esc close`; the
  screen behind is dimmed. Settings, Profiles and Workflows use the right-hand
  sliding sheet, with the working panes visible beside it when the window allows.
  Settings keeps its General, Sessions, Appearance, Agents and Keys sections in
  the sheet; its update controls and keyboard and mouse editing stay available.
- Selection uses the theme's selection fill and a marker; never the accent
  colour as a row fill. The sidebar keeps active `▸` and keyboard `›` distinct.
- Every popup row is clickable and keyboard-reachable; every key is shown as a
  keycap somewhere (hints line, keys screen, palette).
- Short-lived notes are toasts above the bottom bar; destructive actions ask
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

## Project-specific: collapsed sidebar

Hiding the full sidebar leaves a seven-column rail on the configured side.
Show each session's status and first two non-space title characters, including
sessions in folded groups. Reuse the working spinner and blocked/done colors.
Keep active selection distinct from hover; reveal project and full title on hover.
Click rows to focus/wake; retain keyboard browsing and wheel scrolling. Keep the
total blocked count visible even when its rows are off-screen. Provide a mouse
expand control and retain Actions. The rail is the collapsed state of the same
layout, not another layout.

## Project-specific: workflow sheet

The workflow sheet has profile selectors, task entry, reports, human plan
approval, retry and cancellation; see [usage](features/workflows/usage.md).
Make section tabs clickable and keyboard-reachable; preserve drafts/selections
between sections. Enter/double-click opens editing or run details. Keep Launch,
Run and Open pane separate from opening an item. Role selectors open a profile
picker; show complete run reports with explicit approval and Back controls.
Keep errors above action rows, including at 80×24. Text cursors must stay visible.
Setup previews and copies repository instructions without editing files. Show
workflow stage/round labels beside associated sessions. Pending plan approvals
also appear in Inbox and open the exact run for review before human approval.

## Project-specific: frame presentation

Wrap each outer terminal frame in synchronized-update markers; end the update
even when drawing fails. An unfinished child update from the previous frame
must not delay switching to another session.

Keep pane padding and the card interior on the same surface as a terminal app
when at least three screen corners use one explicit background color. Preserve
the pane insets; isolated colored panels keep their existing theme treatment.
