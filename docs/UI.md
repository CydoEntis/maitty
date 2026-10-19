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

The terminal client has one session layout: a sidebar of projects and their sessions (left or right), panes on
the rest (tabs and splits), and a one-line bottom bar. Settings > Appearance > Layout selects Cards or Flat;
both render the same sessions, tabs, splits and sheets. Cards is the default. Flat uses square, full-bleed
zones with no gaps, a persistent bottom status line, and Tab / Shift+Tab to switch focus between sidebar
and panes; Esc returns focus to the sidebar. Everything after the leader
key (`Ctrl+Space` by default; keys follow herdr's where they overlap). Views:
inbox (`j`) and settings (`,`) use the right-hand sheet. The command palette (`Space`),
keys (`?`), files, branches and new
agent use popups. Minimum supported size is about
80×24; popups clamp to the screen. Mouse: click, right-click menus, drag
dividers, scroll; Ctrl+click opens paths.

The high-fidelity flat-layout reference and its interaction spec are preserved in
[`docs/design/flat/README.md`](design/flat/README.md); keep the implementation aligned with it.

The sidebar has no footer; the status line along the bottom of both layouts holds the global hints
(`j inbox`, `a actions`, `,` settings, `?` keys). A **+ new** button on the sidebar's top row (cards),
a project's **+** and leader + `n` open a plain shell; every control stays reachable by mouse. Keep **Actions** available at the top right, including
without a sidebar or session. Offer clickable agent status setup (including Codex),
Settings and All commands; every palette row is clickable and wheel-scrollable.
Include configured commands, retaining existing confirmations.

When an update is available, show the accent dot beside **Actions** (top right and in the status line), its
**Settings** row and the **General** settings tab to guide users to the update.

Retain the working, needs-you, done or idle icon when a terminal rings. Terminal
bells do not add sidebar indicators or history entries.

Mark one selected sidebar row: the one under the keyboard while the sidebar has the keys,
else the open session. It gets a neutral tint (`sel_bg`: the background a step toward the text)
and a `▎` bar at its left edge, lime while the sidebar has the keys and dim when a pane or a sheet
does. The open session keeps a dim bar while another row is under the keyboard. Hover uses a
quieter tint (`hover_bg`). Preserve agent status icons and colors in each state.

Settings opens on General with one version line (`seshi 0.34.0 · up to date · daemon and window
match · u check now`; with an update, the version, what is out and **Update now**). Installed, window and
daemon versions are listed separately, with a labeled remote daemon and a restart-pending notice,
only when they differ or can't be read. General is in sections (INPUT, LAYOUT, SHELL, UPDATES), each a
dim caps heading with a rule; Layout and Background live in LAYOUT. Keep the update
dots until that mismatch is resolved. Installing an update leaves this window and
the daemon running. Never restart the daemon automatically, even on a protocol mismatch.
Status icons and pane status tags open the existing evidence popover, with a
plain-language explanation and actual report/observation ages.

**All keys** (`<leader> ?`, Actions or palette) opens a searchable, scrollable
reference. List every active configured leader/global binding separately,
including aliases and custom commands. Show palette-only commands without
inventing bindings. Keep the delayed leader overview compact. Contextual keys
come from [shortcuts.md](shortcuts.md), embedded in the app so both stay identical.

## Project-specific: background

Chrome (gaps, card insides, the sidebar, the pane area) sits on one ground. Settings >
Appearance > Background picks it: **terminal** (the default) hands that ground back to the
terminal's own background (SGR 49), so transparency or a background image looks the same under
all of it; **solid** paints it the theme's `bg`. Only the sheet, modals and the status line are a
painted, raised tone. The swap is one last pass over the frame (`use_terminal_background` in
`client/render.rs`), so drawing code keeps using `t.bg` and blending still works. The v5 handoff in
[`docs/design/v5/README.md`](design/v5/README.md) is the reference.

## Project-specific: each fact once

Show each fact in one place. The session name is the sidebar row and the pane title; the repo
follows it in the title (after what the agent is working on, which nothing else shows). The agent
is the sidebar row's meta, before the age, and is left out when the name is the agent. Where it
runs, its branch, PR, dev server and `⇆` tag belong to the status line. Model, context and cost
are the agent's own status line, so seshi's chrome never repeats them. The pane has no footer row.

## Project-specific: colour jobs

Each colour does one job. Lime (`accent`) is focus: the lit edge and the focused title. The
theme's `blocked` colour is needs-you and nothing else (the v5 design makes it amber; each
theme picks its own). Red (`err`) is failed, crashed or destructive, and a
machine load past its threshold. Green (`done`) is done. Sky is leader mode. **Working keeps its
orange, eased:** `Theme::status(Working)` is the theme's working colour mixed 35% toward the muted
text, so it sits quieter than needs-you. It shows on the status glyph and a helper's icon; names
shimmer in a neutral tone, not orange. A paused limit and ordinary CPU/RAM load are dim.

## Project-specific: one lit edge and the status line

Only what has the keyboard is lit: the focused pane's border (cards) or bar (flat), or, while a
sheet is open, the sheet. The pane then drops to a `line` border and fades; the sidebar's
selection bar goes dim. Unfocused panes fade by the Dim setting (30% by default; "40%" in an older
config counts as 30%).

The bottom row is the status line in both layouts, on the raised `card` tone. Left: in flat a zone
badge (`SIDEBAR` / `PANE`), then the selected session's path, `⎇ branch` and a `⇆ file` heads-up.
Right: `● n needs you`, then `j inbox   a actions   , settings   ? keys`, with an accent dot after
`actions` when an update waits. The right side keeps its room; the left fills the rest, dropping
the path first (it never shows below 140 columns) and then tags from the end. Below 140 columns the
right becomes `● n   ? keys`. The design also shows `#PR ✓/✕` and a dev-server tag; seshi has no
data for either yet.

## Project-specific: UI conventions

- Popups are centred `panel()`s with an accent title bar and `Esc close`; the
  screen behind is dimmed. Settings uses the right-hand sliding sheet, with the
  working panes visible beside it when the window allows.
  Settings keeps its General, Sessions, Appearance, Agents and Keys sections in
  the sheet; its update controls and keyboard and mouse editing stay available.
- Selection is a neutral tint (`sel_on` of the surface it sits on) with a lime `▎` bar;
  never the accent colour as a row fill.
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
Click rows to focus or resume a legacy sleeping session; retain keyboard browsing and wheel scrolling. Keep the
total blocked count visible even when its rows are off-screen. Provide a mouse
expand control and retain Actions. The rail is the collapsed state of the same
layout, not another layout.

Show active helper rows under their owning agent: an icon for what the helper is doing
(`✎` planning, `◎` researching, `⚙` building, `⚖` reviewing, `●` otherwise) in the eased working
orange, then its name in the plain text colour. Roles are told apart by icon, never by colour.

## Project-specific: frame presentation

Wrap each outer terminal frame in synchronized-update markers; end the update
even when drawing fails. An unfinished child update from the previous frame
must not delay switching to another session.

Keep pane padding and the card interior on the same surface as a terminal app
when at least three screen corners use one explicit background color. Preserve
the pane insets; isolated colored panels keep their existing theme treatment.
