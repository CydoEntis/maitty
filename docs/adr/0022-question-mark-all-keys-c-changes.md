# 0022. `?` lists all keys, `c` opens Changes, copy mode has no key

- **Status:** Accepted
- **Date:** 2026-10-24

## Context

After trying v0.43.1 the user asked for three key changes. `F1` for all keys was hard to
reach and not where they looked for help; Changes on `d` did not say what it opened; copy
mode on `c` was used rarely, and a mouse drag already selects text.

## Decision

At the user's request, `?` after the leader opens All keys, though it needs Shift on most
keyboards. `c` opens Changes. Copy mode keeps working with no default key: search commands
(leader, then `Space`) finds it as "Select text". `F1` and `d` have no default. This amends
ADR 0017: its rules still hold for every other default, and its test allows `?` alone.

## Alternatives considered

- **Keep `F1`:** the user found it awkward.
- **`h` for help:** `h` is split side by side (ADR 0017).
- **Keep copy mode on another letter:** the user chose no key.

## Consequences

`F1`, `d` and `c` habits break. A user's own `[keys.prefix]` entries, by action name, still
load and win, so `d = "changes"` or `f1 = "help"` brings the old key back.
