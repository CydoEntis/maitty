# shadcn/ui + Tailwind

> **Use for:** React UIs styled with Tailwind CSS and shadcn/ui components.
> **Requires:** react
> **Fills:** UI § design system, UI conventions

## Rules

### Primitives

- shadcn components are added into `src/components/ui/` and become project
  code. Add them with the CLI; don't copy them from the website by hand.
- Keep `components/ui/` domain-free. Build feature-specific variants in the
  feature folder by composing primitives, not by adding domain props to them.
- Editing a primitive is allowed when the change applies everywhere, such as a
  design token or an accessibility fix. Note the change in the commit body, so
  it's clear the primitive now diverges from the upstream shadcn version.
- **One component per control (hard rule).** A switch, segmented or radio
  choice, select or picker, dialog, alert dialog, menu, context menu, popover
  or tooltip exists once, in `src/components/ui/`, on its Radix primitive. A
  feature composes it and never builds a second one: no `role="switch"`,
  `radiogroup`, `dialog`, `menu`, `listbox` or hand-opened `createPortal`
  outside `components/ui/`. A different look comes from a token or a prop on the
  shared component. If the control you need is missing, add it to `components/ui/`
  first. ESLint enforces this (`HAND_BUILT_CONTROL` in `apps/gui/eslint.config.ts`)
  with no exempt files outside `components/ui/`. Never add an exemption to pass a change.
- Use `cva` (class-variance-authority) for variants, as the primitives already
  do. Don't use conditional class soup.
- `button.tsx` diverges from upstream: its variants are the design's kinds (`primary`, `needs`, `soft`, `ghost`,
  `danger`, `link`) and sizes (`sm`, `md`, `icon`), drawn from the role tokens ([DESIGN](../DESIGN.md#desktop-app),
  Primitives).

### Styling

- Style with Tailwind utility classes. Don't add CSS files unless a utility
  can't express the style.
- Merge classes with `cn()` from `lib/utils` whenever a component accepts
  `className`.
- Use theme tokens only (`bg-background`, `text-muted-foreground`,
  `border-border`, `bg-primary`). Hard-coded colors like `bg-[#1a1a1a]` or
  `text-gray-500` bypass theming and count as magic values.
- Define tokens once, as CSS variables in the global stylesheet (the Tailwind
  v4 `@theme` block). Dark mode works by switching variables, not through
  per-component `dark:` overrides for colors.
- Use arbitrary values (`w-[337px]`) only with a comment explaining why no
  scale value works.
- Order classes automatically with `prettier-plugin-tailwindcss`.

### Behaviour

- Use the Radix-based primitives' built-in accessibility. Don't rebuild
  dialogs, menus, or popovers.
- Build forms with shadcn's `Form` wrappers around react-hook-form, so labels,
  descriptions, and errors are wired together.
- Use toasts from the project's toast primitive (Sonner by default) for
  transient feedback, and inline errors for field or form problems.
- Use `lucide-react` for icons. An icon-only button needs an `aria-label`.

## Commands

```bash
pnpm dlx shadcn@latest init
pnpm dlx shadcn@latest add button dialog form
```

## Project-specific: shadcn-tailwind

Follow the chat handoff (`design_handoff_maitty_chat`, see [DESIGN](../DESIGN.md#desktop-app)) and
`docs/features/chat-gui/design-brief.md`. Use Inter for interface text and JetBrains Mono for code,
paths, shortcuts and diff numbers. Use the spacing tokens `--s1`..`--s9` and the radius tokens
`--r-control` (6px), `--r-panel` (10px, panels, dialogs and menus) and `--r-badge` (3px), which are 0 under
Square corners (`apps/gui/src/tokens.css`). A surface takes its fill from its role in `material.css` (panel,
float or field), and a control inside it from the `--m-*` and `--c-*` tokens there; the look is flat, with no
gradient or blur ([DESIGN](../DESIGN.md#desktop-app)).

Maitty Night is the default dark theme and Maitty Day the default light one (sea-glass and brass,
their `palette` in `crates/maitty-settings/themes.json`). A primary button is the theme's accent; teal means only
working; `--needs` (the theme's warn) means only needs-you; red is reserved for errors and recovery. Use the shared CSS variables for every
theme and keep shadcn primitives unchanged unless an accessibility or product
need is documented.

The GUI's design system rules (tokens only, popups from `components/ui/`, where
Tailwind and `workspace.css` each apply, and the colour lint) are owned by
[UI § GUI design system rules](../UI.md#project-specific-gui-design-system-rules).
