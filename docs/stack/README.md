# Stack docs

This folder holds the rules for each technology this project uses. Most are
applied from packs shipped with the `starter` skill. Some are written for this
project alone (source `project`).

Precedence: stack docs override the generic base. A doc's `Project-specific:`
sections override the stack docs.

## Starter

- Version: 1.0.0
- Preset: Rust CLI/TUI with optional TypeScript desktop frontend
- Applied: 2026-10-28

`/starter update` compares this version with the installed skill and brings the
starter-owned sections forward. It never touches `Project-specific:` sections.

## Applied packs

| Pack | Source | Version |
| --- | --- | --- |
| rust | project | — |
| [typescript](typescript.md) | starter | 1.0.0 |
| [react](react.md) | starter | 1.0.0 |
| [shadcn-tailwind](shadcn-tailwind.md) | starter | 1.0.0 |
| [tauri](tauri.md) | project | 2 |
| [tauri-clipboard](tauri-clipboard.md) | project | 2.5.0 |
| [tauri-dialog](tauri-dialog.md) | project | 2.8.4 |
| [tauri-notification](tauri-notification.md) | project | 2.8.0 |
| [tauri-updater](tauri-updater.md) | project | 2 |
| [syntect](syntect.md) | project | 5.3 |
| [xterm](xterm.md) | project | 6.0.0 |
| [claude-code-cli](claude-code-cli.md) | project | Claude Code CLI `2.1.314` observed locally; support range open |
| [codex-app-server](codex-app-server.md) | project | codex-cli 0.164.0, 0.165.0 |

## Not applied

No additional packs are applied to the current TUI and daemon.

## Adding, changing, or removing a pack

Follow **Stack packs** in `AGENTS.md`, or run `/starter add <tech>` or
`/starter remove <pack>`.
