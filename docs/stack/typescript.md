# TypeScript

> **Use for:** any TypeScript codebase: frontend, backend, library, or game.
> **Requires:** none
> **Fills:** ARCHITECTURE § stack (language); CODE-STANDARDS § language and
> framework rules, naming, formatting and linting; TESTING § tools and commands

## Rules

### Compiler

- Enable `strict: true`, plus `noUncheckedIndexedAccess`,
  `exactOptionalPropertyTypes`, `noImplicitOverride`, and
  `noFallthroughCasesInSwitch`.
- Never loosen `tsconfig` to make code compile.
- Use ES modules only. Use `import type` for type-only imports
  (`verbatimModuleSyntax`).
- Put no plain `.js` source in the repo. Config files are `.ts` wherever the
  tool supports it.

### Types

- Use `unknown` at every boundary and parse it with **Zod**. Derive TypeScript
  types from schemas (`z.infer`) rather than declaring them twice.
- Use `type` for unions and data shapes, and `interface` for contracts that
  classes implement or that consumers extend. Be consistent within a feature.
- Model state as discriminated unions with a `kind` or `status` tag. Check
  switches for exhaustiveness with `satisfies never` or an `assertNever(x)`
  helper.
- Use branded types for ids that must not be mixed up:
  `type UserId = string & { readonly __brand: "UserId" }`.
- Return types are explicit on exported functions and inferred on local ones.
- For closed sets, prefer `as const` objects or string-literal unions over
  `enum`.
- Use `readonly` on data that is never meant to be mutated, such as arrays and
  props passed across modules.

### Errors

- Expected failures return `Result<T, E>`:
  `{ ok: true; value: T } | { ok: false; error: E }`. Error types are
  discriminated unions with a `code`.
- Throw `Error` subclasses only for bugs and unrecoverable states, and always
  set `cause` when you wrap an error.
- In `catch (error)`, `error` is `unknown`. Narrow it before you use it.

### Modules

- Use named exports. Use default exports only where a framework requires them.
- Give each feature one public entry file (`index.ts`). Other features import
  from that entry, never from its internal files.
- Use no barrel files beyond that feature entry.
- Use path aliases (`@/…`) for cross-directory imports and relative imports
  within a feature.

### Runtime

- Use Node 24 or later. It runs `.ts` files directly (type stripping), so a
  single-file script or tool needs no build step. Use `tsx` or a bundler only
  when you need non-erasable syntax or bundling.
- Use `pnpm` as the package manager. Commit `pnpm-lock.yaml`, and pin
  `packageManager` in `package.json`.

## Naming

- Files are `kebab-case.ts`. React component files follow the `react` pack.
- Types, interfaces, and classes are `PascalCase`. Functions and variables are
  `camelCase`. Module-level constants are `UPPER_SNAKE_CASE`.
- Tests are named `<file>.test.ts` and sit next to the file they test.

## Tooling

| Concern | Tool |
| --- | --- |
| Tests | Vitest |
| Lint | ESLint (flat config) with `typescript-eslint` strict-type-checked |
| Format | Prettier |
| Validation | Zod |

Enable these lint rules: `no-explicit-any`, `no-floating-promises`,
`no-misused-promises`, `switch-exhaustiveness-check`, `no-non-null-assertion`,
and `no-magic-numbers` (ignoring 0, 1, and -1, and ignoring tests).

## Commands

```bash
pnpm typecheck   # tsc --noEmit
pnpm lint        # eslint .
pnpm format      # prettier --write .
pnpm test        # vitest run
```

## Project-specific: typescript

The Rust repository contains one optional frontend package in `gui/`. Frontend
code runs in the Tauri webview; Node is tooling only. Use Node 24 or later and
pnpm with a committed `pnpm-lock.yaml`. Keep strict compiler settings and the
lint rules above; do not add plain JavaScript source.
