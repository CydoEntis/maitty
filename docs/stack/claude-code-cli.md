# Claude Code CLI

> **Use for:** daemon-owned Claude Code conversations in Maitty's optional chat GUI.
> **Requires:** the Claude Code CLI installed on the user's machine.
> **Fills:** ARCHITECTURE § planned structured conversations; API_PATTERNS § structured conversation IPC.

## Rules

- Drive the installed Claude Code executable through headless JSON streaming.
  Keep process ownership, session identity and durable conversation state in
  Maitty's daemon; the GUI and Tauri host are clients.
- Keep the Claude protocol in its own provider adapter. Do not emulate Codex
  JSON-RPC or expose provider frames to the GUI.
- Preserve and resume only the exact native session ID and its verified
  configuration root and working directory. Never continue the most recent
  session by inference.
- Pin and verify Claude Code versions before enabling structured chat. Validate
  each JSONL frame, cap frame and queue sizes, use argument arrays, and reject
  unknown or ambiguous control messages.
- Route permission prompts to the documented Agent SDK host channel with
  `--permission-prompts host`. Keep every offered choice and request ID exact;
  fail closed on unsupported input. Do not treat `--permission-prompt-tool
  stdio` as a documented host channel: the CLI documents that flag for a named
  MCP tool.
- Correlate each submitted operation to Claude's own message and lifecycle
  evidence. Persist an uncertain receipt before sending; never replay a prompt
  whose delivery is uncertain.
- Interrupt the exact active turn, then wait for provider completion and child
  tree cleanup before releasing conversation ownership. Client disconnects do
  not stop daemon-owned work.
- Let the installed Claude Code CLI own authentication. The user signs in with
  `claude auth login`; Maitty does not collect, copy, store or transmit Claude
  credentials. Do not put credentials in process arguments, logs, conversation
  records or GUI IPC.
- Keep the TUI-only build independent of desktop packaging. Test Windows,
  macOS, Linux and real Omarchy before advertising Claude support.

## Project-specific: Claude Code CLI

The selected protocol is Claude Code's `stream-json` input/output, following
PandaCode's installed-CLI provider shape. The user approved subscription sign-in
through the installed CLI. Anthropic's Help Center currently says third-party
app usage continues to draw from Claude subscription limits; Maitty does not
provide a separate Claude login or implement its own agent harness. The CLI's public reference says
`--permission-prompts host` routes approval prompts to the Agent SDK host. Keep
the adapter disabled until the host message flow is verified against each
pinned version. See [ADR 0039](../adr/0039-claude-code-cli-streaming.md) and
the [transport research](../research/claude-code-gui-transport.md).

<!-- FILL: Record the verified Claude Code version range and live protocol/platform results before enabling the Claude provider. The approved auth path is the user's existing local Claude Code sign-in; Maitty does not handle credentials. -->
