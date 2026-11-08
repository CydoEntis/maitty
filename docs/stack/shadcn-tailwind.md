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
- Use `cva` (class-variance-authority) for variants, as the primitives already
  do. Don't use conditional class soup.

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

Follow the approved `Maitty Desktop specification` handoff and the redesign handoff (five window styles, see [DESIGN](../DESIGN.md#desktop-app)) in
`docs/features/chat-gui/design-brief.md`. Use Geist Variable for interface text
and Geist Mono Variable for code, paths, shortcuts and diff numbers. Use the
4-pixel spacing scale, 6–8-pixel control and row radii, 9–10-pixel card radii,
12-pixel workspace radius and 14–16-pixel composer/dialog radius.

Midnight is the default; Paper is the light theme. Sea glass teal marks focus,
primary actions and working status. Amber is reserved for needs-you attention;
red is reserved for errors and recovery. Use the shared CSS variables for every
theme and keep shadcn primitives unchanged unless an accessibility or product
need is documented.

The GUI's design system rules (tokens only, popups from `components/ui/`, where
Tailwind and `workspace.css` each apply, and the colour lint) are owned by
[UI § GUI design system rules](../UI.md#project-specific-gui-design-system-rules).
