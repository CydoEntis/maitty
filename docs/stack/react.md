# React

> **Use for:** React frontends, and React UI shells around other systems such
> as a game engine.
> **Requires:** typescript
> **Fills:** ARCHITECTURE § stack (UI), patterns and conventions; UI § layout
> and navigation; TESTING § tools and commands (component tests)

## Rules

### Components

- Write function components only. Each component file holds one exported
  component; small private subcomponents in the same file are fine.
- Type props with a `type` named `<Component>Props`. Destructure them in the
  signature.
- Nothing side-effectful happens during render.
- Don't use effects for derived values or to respond to events. Compute
  during render, or do the work in the event handler. Effects are only for
  syncing with external systems.
- Use `key` on lists with stable ids, never array indexes for lists that can
  reorder.
- Keep components presentational where possible. Data access lives in feature
  hooks (`use<Thing>`), which call services.

### State

| Kind | Tool |
| --- | --- |
| Server state | TanStack Query. Query keys live in `features/<f>/keys.ts`. |
| URL state | the router's search params |
| Forms | react-hook-form with a Zod resolver |
| Shared client state | Zustand: small stores, one per concern, using selectors |
| Local | `useState` / `useReducer` |

- Never copy server data into Zustand.
- Mutations invalidate or update their query keys in the feature's mutation
  hook, not in components.
- Memoize (`useMemo`, `useCallback`, `memo`) only for a measured problem or for
  a stable identity that a dependency needs.

### Structure

```text
src/
  app/                 providers, router, layout shell
  routes/ | pages/     route components: compose features
  features/<f>/
    components/        feature UI
    hooks/             useX data + behaviour hooks
    api/ | services/   calls to backend/engine
    schemas.ts  types.ts  keys.ts  index.ts
  components/ui/       primitives (see shadcn-tailwind pack)
  components/          shared composites
  lib/                 utilities
```

### Errors

- Put an error boundary at the route level and around independent widgets.
- Query errors render the feature's error state with a retry. Mutation errors
  show a toast or inline message, whichever *UI § UI conventions* specifies.

## Naming

- Component files are `PascalCase.tsx`, or `kebab-case.tsx` if the project
  prefers. Choose one in *Project-specific: react* and use it everywhere.
- Hooks are `useCamelCase.ts`.

## Testing

- Use Vitest with React Testing Library. Query by role and accessible name,
  not by test id or class.
- Test what a user sees and does. Don't test hook internals or state shape.
- Mock the network at the fetch boundary with MSW, not by mocking hooks.

## Project-specific: react

Use Vite as a single-route desktop SPA; no router is needed until a second
route exists. Use TanStack Query for daemon state, local React state for
component state, and no Zustand store initially. Use Sonner for transient
toasts. Component files use `PascalCase.tsx`.
