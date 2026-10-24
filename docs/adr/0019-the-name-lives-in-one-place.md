# 0019. The app's name lives in one place

- **Status:** Accepted
- **Date:** 2026-10-23

## Context

Renaming Seshi to Vibecasso (ADR 0018) touched 2,094 places: the name was written out in paths,
environment variables, hook tags, MCP names, theme names, adapters, UI text, comments and tests.
The user expects to rename the app again and wants a rename to mean changing the name, not the
code.

## Decision

`src/brand.rs` holds the name: `NAME` (shown to people, "Vibecasso") and `COMMAND` (typed, the
crate's name from `Cargo.toml`, `vibecasso`). Everything that carries the name is built from
them: config and data folders, `.<command>.toml` and `.<command>/checks.toml`, environment
variables (`brand::env("TERM_ID")` → `VIBECASSO_TERM_ID`), socket and pipe names, the MCP server
and its `<command>_*` tools, hook tags, adapter file names (the adapters are templates filled in
when installed), UI text, and the splash wordmark (a full A–Z font). Themes and code are named
by what they are, not by the product (Walnut, Night, Day; `is_our_daemon`). Comments say "the
app" or name the part. Release and install scripts read the name from one variable at their
top. A previous name goes in `brand::PREVIOUS`; the carry-over in ADR 0008 and 0018 reads
that list. `tests/naming.rs` fails if the current name, or a previous one outside the carry-over
code, is written anywhere else in `src`.

To rename: change `NAME` and the package name in `Cargo.toml`, add the old command to
`brand::PREVIOUS`, set the scripts' variable, replace the name in the docs, rename the repository.

## Alternatives considered

- **Permanent neutral internal names** (a fixed prefix for env vars, folders and tags): no
  carry-over needed for those, but a second name to choose now and to explain forever.
- **Keep writing the name out:** every rename is another sweep through the code.

## Consequences

A few lines read a little less directly (`brand::env("SOCKET")` for an env var). Docs still name
the app and change with it. Supersedes the naming parts of ADR 0008 and ADR 0018, not their
carry-over.
