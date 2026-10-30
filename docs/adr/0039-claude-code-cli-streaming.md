# 0039. Drive Claude Code through its streaming CLI protocol

- **Status:** Accepted
- **Date:** 2026-10-29
- **Amends:** [0041](0041-chat-gui-shared-conversations.md)

## Context

The optional chat GUI must support Claude Code alongside Codex before its initial
release. The user expects Maitty to follow PandaCode's Claude integration.
PandaCode launches the installed Claude Code CLI in headless `stream-json` mode
and speaks its bidirectional control protocol; it does not embed a visible
terminal or run the TypeScript SDK host. This is evidence for a provider-specific
adapter shape. Maitty relies on the user's authentication managed by the
installed Claude Code CLI, not on PandaCode's product-specific arrangements.

Maitty already has a daemon-owned Rust process supervisor and provider-specific
conversation lane. Claude Code's CLI supports streaming JSON input/output,
session IDs, resume and replaying submitted messages. PandaCode documents a
bidirectional stdio permission-control mode with
`--permission-prompt-tool stdio`. The CLI reference defines that flag for a
named MCP permission tool, while `--permission-prompts host` sends approval
requests to the Agent SDK host. The local 2.1.314 help documents the latter.
Use the documented host channel and verify its message flow against Maitty's
recovery, approval and cancellation requirements before enabling Claude.

## Decision

Use the installed Claude Code CLI through headless `stream-json` behind a
daemon-owned Claude adapter, following PandaCode's provider shape. Send
permission prompts to the Agent SDK host using `--permission-prompts host`.
Treat permission control as unavailable until its exact messages and native
choices are verified for a pinned version. Keep the GUI and Tauri host as
clients; they do not own Claude runtime state. Preserve exact provider session
IDs, approval options and cancellation semantics, and report only verified
provider capabilities. Keep the native TUI available as a separate user
workflow.

Use the authentication already configured in the installed Claude Code CLI.
Maitty does not offer a Claude login screen, handle OAuth credentials, proxy
tokens, or resell usage. Anthropic's current Help Center says third-party app
usage can draw on Claude plan limits, while the Agent SDK guide restricts
third-party products from offering their own claude.ai login without prior
approval. Maitty launches the official CLI and leaves sign-in to that CLI.
Verify the user's current regional terms and live CLI behavior before enabling
the provider. Supported Claude Code versions and Linux distributions remain
open until verified.

## Alternatives considered

- Claude Agent SDK: offers higher-level session, approval and streaming APIs,
  but requires a packaged Node worker and was not the user's expected integration
  shape. The worker also adds a second process and distribution lifecycle.
- Anthropic Messages API: would require Maitty to implement the agent harness,
  tools, context and permission behavior that Claude Code already supplies.
- Open Claude Desktop: does not meet the integrated GUI requirement or Linux
  support requirement.

## Consequences

The desktop package does not need a separate Node worker for Claude. The daemon
must discover a supported Claude CLI and own its process tree. Before advertising
Claude, validate exact-session recovery, approvals, cancellation, process cleanup
and Linux operation against the documented version range. If the CLI protocol
cannot meet the accepted conversation guarantees, keep Claude unavailable and
bring the gap back for a product decision instead of silently weakening them.
See [the Claude Code CLI stack rules](../stack/claude-code-cli.md).
