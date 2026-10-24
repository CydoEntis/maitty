# 0020. The app is called Maitty

- **Status:** Accepted
- **Date:** 2026-10-23

## Context

The user chose a new name the same day as ADR 0018, once the name had moved into one place
(ADR 0019).

## Decision

The app is **Maitty** (command `maitty`), with the tagline "All agents on deck". The tagline
joins the name in `[package.metadata.brand]` and reaches the code as `APP_TAGLINE` (the
splash). The rename followed ADR 0019's steps: `Cargo.toml` (with `vibecasso` first in
`previous`), the installers' first lines, the regenerated frames, the docs and the repository
(CydoEntis/maitty). A vibecasso or seshi install carries over once.

## Alternatives considered

- **Keep Vibecasso:** the user disliked it.

## Consequences

Supersedes the name in ADR 0018, not its carry-over rules. GitHub redirects the old
repository URLs.
