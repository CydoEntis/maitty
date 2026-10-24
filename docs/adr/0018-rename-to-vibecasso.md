# 0018. The app is called Vibecasso

- **Status:** Accepted
- **Date:** 2026-10-23

## Context

The user chose a new name for the app, Vibecasso, with a clean break: the old `seshi`
command does not keep working. What an existing install left behind (config, saved sessions,
hooks and MCP entries in agents' own settings, per-repo files) must still carry over, or
sessions and agent status would be lost on the first start.

## Decision

The app, its command, binary and crate are `vibecasso`. Config and data move to `vibecasso`
folders, environment variables to `VIBECASSO_*`, the per-repo files to `.vibecasso.toml` and
`.vibecasso/checks.toml`, the MCP server and its tools to `vibecasso` / `vibecasso_*`, hook
tags to `_vibecasso`, adapter files to `vibecasso.js`, release archives to
`vibecasso-<target>`, the default themes to Vibecasso Walnut, Night and Day, and the repository
to `CydoEntis/vibecasso`. The splash wordmark spells VIBECASSO, filled in by a paint-stroke
sweep.

Carried over, as ADR 0008 did for hydra: a seshi install's config and data are copied once
on first start (`config::migrate_old_names`, its themes renamed); hooks, status lines and MCP
entries naming seshi are recognised and rewritten; `.seshi.toml` and `.seshi/checks.toml` are
still read when the new file is absent. `seshi` is added to the retired names
`tests/naming.rs` checks. Test fixtures keep their sample project called "seshi": it is sample
data, not the product.

## Alternatives considered

- **Keep `seshi` as a second command:** the user chose a clean break.
- **Rename only what users see:** the command you type would differ from the app's name.

## Consequences

Users reinstall once (`vibecasso` does not replace `seshi.exe`), and a running seshi daemon is
not taken over: its sessions come back when Vibecasso first starts from the saved session.
Adapters for Pi, OpenCode and the others are rewritten by `vibecasso integrate <agent>`;
`vibecasso doctor` says when one still names seshi. GitHub redirects the old repository URLs.
Supersedes the name in ADR 0008, not its migration rules.
