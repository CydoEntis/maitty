# Claude Code chat transport research

Checked 2026-10-29 against Anthropic's current Claude Code documentation and
first-party SDK repository. The locally installed CLI reports `2.1.314`; the
first-party TypeScript SDK changelog currently lists `0.3.312`, aligned with
Claude Code `2.1.312`. Maitty's adapter only accepts `2.1.312`, so this local
CLI is intentionally rejected until a version is verified.

## Verified facts

- Claude Code CLI supports non-interactive JSONL streaming with
  `--print --input-format stream-json --output-format stream-json`. It accepts
  multiple text user messages over one process, and its output can include
  partial text and tool events. CLI stream input currently supports text user
  messages only. See the [CLI reference](https://code.claude.com/docs/en/cli-usage)
  and [streaming input guide](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode).
- The CLI supports exact session IDs: `--session-id` when starting a UUID-backed
  session and `--resume <id>` to continue it. It also documents JSON and
  `stream-json` output and `--replay-user-messages` for echoing submitted
  messages as acknowledgement. Its CLI reference says
  `--permission-prompt-tool` takes a named MCP tool and `--permission-prompts
  host` routes prompts to the Agent SDK host. The installed `2.1.314` help
  documents `--permission-prompts`; the adapter will use that documented host
  mode, and `claude --print --input-format stream-json --output-format
  stream-json --verbose --permission-prompts host --version` exits successfully.
  This is argument parsing only. PandaCode's current source sends
  `--permission-prompt-tool stdio`, an implementation detail beyond the public
  flag contract. The host protocol still needs a live Maitty roundtrip before
  enabling approvals. See the
  [CLI reference](https://code.claude.com/docs/en/cli-usage) and
  [PandaCode provider guide](https://github.com/aicodeking/pandacode/blob/main/docs/providers.md).
- The Agent SDK's persistent streaming mode documents multi-turn input, live
  response/tool events, interruptions and session management. Its
  `canUseTool` callback can pause for a user decision, and can report tool
  approvals or Claude's `AskUserQuestion` requests. Sessions can be resumed by
  exact ID and the SDK can read/list saved sessions. See the [streaming input](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode),
  [approval handling](https://code.claude.com/docs/en/agent-sdk/user-input),
  [streaming output](https://code.claude.com/docs/en/agent-sdk/streaming-output),
  and [session](https://code.claude.com/docs/en/agent-sdk/sessions) guides.
- The CLI reference describes `claude -p` as print mode and directs programmatic
  use to the Agent SDK. The SDK quickstart instructs developers to use an API
  key and says third-party products need prior approval before offering their
  own claude.ai login or subscription rate limits. Maitty does not offer a
  separate login; the user signs into the official local CLI. See the
  [CLI reference](https://code.claude.com/docs/en/cli-usage) and
  [SDK quickstart](https://code.claude.com/docs/en/agent-sdk/quickstart).
- Anthropic's Help Center article [Use the Claude Agent SDK with your Claude
  plan](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan),
  updated June 15 and October 7, 2026, says third-party app usage, the Agent
  SDK and `claude -p` still draw from Claude subscription limits when signed in
  with a Claude plan. It also documents separate API credits for linked Console
  keys. This supports Maitty delegating to the installed CLI and its existing
  sign-in; Maitty does not collect OAuth tokens, proxy requests or resell usage.
- Anthropic's [Consumer Terms of Service, section 3](https://www.anthropic.com/legal/consumer-terms)
  allows automated access through an API key or where Anthropic explicitly
  permits it. The cited consumer-terms page is for EEA and Swiss consumers;
  applicability depends on the user's account and region. The current Help
  Center article is the explicit support for subscription-backed third-party
  app use. Do not generalize the Agent SDK restriction on offering login to a
  blanket ban on orchestrating the user's installed official CLI.
- The current first-party SDK changelog records a paginated
  `getSessionMessages()` API, per-message `command_lifecycle` states, and
  `user_message_uuid` / `user_message_uuids` on assistant and result events.
  These give a concrete basis for a capability spike: page exact-session
  history, then correlate a submission through its supplied UUID and lifecycle
  events. They do not by themselves prove exhaustive history or a durable
  provider acknowledgement for Maitty. The changelog also records fixes to
  transcript omissions and turn correlation, so the worker must pin and test an
  exact SDK/CLI pair. See the [first-party SDK changelog](https://github.com/anthropics/claude-agent-sdk-typescript/blob/main/CHANGELOG.md).
- Anthropic's TypeScript Agent SDK is a first-party package. Its docs say it
  bundles a native Claude Code binary on supported installs, with platform
  optional dependencies. The SDK quickstart instructs developers to use an
  Anthropic API key; it explicitly says third-party products may not offer
  claude.ai login or its rate limits without prior approval. See the [SDK repository](https://github.com/anthropics/claude-agent-sdk-typescript)
  and [SDK quickstart](https://code.claude.com/docs/en/agent-sdk/quickstart).
- PandaCode's current provider guide says it launches installed CLI providers
  directly and lets each CLI own its authentication. That confirms the
  provider-native process shape the user requested. See the
  [PandaCode provider guide](https://github.com/aicodeking/pandacode/blob/main/docs/providers.md).
- T3 Code documents a different Claude path: its FAQ says it uses Anthropic's
  Agent SDK and supports Claude account login, and it describes working with
  Anthropic on that arrangement. That product-specific statement does not grant
  the same permission to Maitty. PandaCode's installed-CLI approach and T3's
  Agent-SDK approach are not the same integration. See the
  [T3 Code FAQ](https://github.com/pingdotgg/t3code/blob/main/docs/user/faq.md).
- Official Claude Code system requirements list macOS, Windows, Ubuntu, Debian
  and Alpine Linux. Native Windows supports PowerShell; Git for Windows enables
  Bash but is optional. The official list does not name Arch Linux, so Omarchy
  needs a live compatibility check. See [system requirements and setup](https://code.claude.com/docs/en/getting-started).
- Claude Code stores sessions locally. The official docs describe credentials
  in macOS Keychain or per-user credential files on Linux and Windows. In
  non-interactive mode, an inherited `ANTHROPIC_API_KEY` can select API-key
  authentication even when a subscription login exists. See [authentication](https://code.claude.com/docs/en/authentication)
  and [session storage](https://code.claude.com/docs/en/agent-sdk/sessions).

## Options and tradeoffs

| Option | Fits Maitty | Main tradeoff |
| --- | --- | --- |
| Call the installed CLI using JSON streaming | Uses the user's installed executable and its existing sign-in; Rust can own its process directly, expose text/tool events, and preserve exact session IDs | Lower-level control frames and interactive permissions need version-specific verification. History completeness and operation acknowledgement need evidence. |
| Run a small TypeScript worker with the Agent SDK | First-party structured stream, multi-turn input, tool events, approval callbacks, paginated session history, per-message turn correlation and interruption support | Adds a Node/SDK runtime and a packaged worker. The available APIs still need a live capability spike. The SDK guide says third-party products must not offer their own claude.ai login without approval. |
| Open Claude Desktop | Reuses Anthropic's own GUI | It is not available on Linux and would take the user out of Maitty. |

## Selected direction

Treat Claude as a daemon-owned structured provider, like Codex, and keep its
native TUI available separately. Follow PandaCode's installed CLI route:
headless bidirectional `stream-json`, exact session IDs and its permission
control protocol. Pin the CLI version and prove history completeness,
operation-correlated acknowledgement, approval mapping, interruption and full
process-tree cleanup before enabling the provider. The Rust daemon owns the CLI
and its full process tree. The SDK remains a higher-level alternative if the
CLI protocol cannot meet Maitty's contract.

The approved product path delegates authentication and execution to the user's
installed Claude Code CLI. Maitty has no Claude login UI and never handles
credentials. Keep the provider disabled until the installed CLI's protocol,
history, permission and cancellation behavior are verified end to end.

For Linux, build and exercise the Claude worker on Arch/Omarchy even though
Anthropic's published OS list names Ubuntu, Debian and Alpine rather than Arch.
Keep the terminal-only Maitty build independent of Node and the GUI worker.

## Not established by documentation

- Whether account- or region-specific terms impose additional requirements on
  a local GUI that launches the official Claude Code CLI.
- Whether the CLI's message replay and local transcript allow complete,
  ordered history recovery, and whether a replayed user message proves durable
  operation-specific acceptance. Prove permission control, cancellation, child
  cleanup and exact-session resume before claiming parity.
- Whether Claude Code is supported on Omarchy. Verify on the user's real Arch
  desktop; a successful Ubuntu build is not enough.
