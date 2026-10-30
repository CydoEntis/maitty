# T3 Code provider integrations

Checked 2026-10-29 against T3 Code's current `main` documentation/source and Anthropic's current first-party guidance. This records how T3 works; it does not establish that another app may reuse the same login method.

## Provider map

T3 separates orchestration from provider adapters. Provider instances own their account/configuration and session state; adapters normalize provider-specific events and expose truthful capabilities to the shared UI. T3 uses a native `ProviderAdapterV2` for providers with their own protocol and a shared ACP adapter for registry/local ACP agents. [Provider internals](https://github.com/pingdotgg/t3code/blob/main/docs/internals/providers.md) · [Adding a provider](https://github.com/pingdotgg/t3code/blob/main/docs/internals/adding-a-provider.md)

| Provider | T3 transport/adapter | Auth and setup | Session/config notes |
|---|---|---|---|
| Codex | Native `ProviderAdapterV2` using OpenAI Codex App Server, rather than a terminal transcript parser. | Connect ChatGPT account through T3's Codex sign-in, or use an existing authenticated Codex CLI login. | Provider instance owns account/session state; exposes Codex-native questions and permission modes. [T3 Codex guide](https://github.com/pingdotgg/t3code/blob/main/docs/user/providers-codex.md) · [Adapter source](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts) |
| Claude Code | Native adapter using Anthropic's Claude Agent SDK (`query()`), which runs Claude Code's agent runtime; not ACP and not direct Messages API calls. | T3 guide uses `claude auth login` and Claude Code config. Instances can set their own `CLAUDE_CONFIG_DIR`; API key/custom endpoint can be supplied through per-instance environment overrides. | SDK session/resume and tool approval events are normalized by the adapter. [T3 Claude guide](https://github.com/pingdotgg/t3code/blob/main/docs/user/providers-claude.md) · [Adapter source](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts) · [Anthropic SDK quickstart](https://code.claude.com/docs/en/agent-sdk/quickstart) |
| Cursor | Native adapter using Cursor's SDK/agent runtime. | User installs/signs into Cursor CLI/provider on the machine running T3; T3 configures executable and runtime options. | T3 maps SDK events, sandbox and model capabilities at the adapter boundary. [Adapter source](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/orchestration-v2/Adapters/CursorAdapterV2.ts) · [Provider setup](https://github.com/pingdotgg/t3code/blob/main/docs/user/providers-cursor.md) |
| OpenCode | Native adapter for OpenCode's server/runtime APIs, with version-specific behavior. | Uses installed OpenCode and its own auth/config. | 1.x uses a T3-managed chat server per thread; 2.x uses a server per provider instance, with thread-specific MCP registrations. [Provider internals](https://github.com/pingdotgg/t3code/blob/main/docs/internals/providers.md) · [Adapter source](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/orchestration-v2/Adapters/OpenCodeAdapterV2.ts) |
| Pi | Native adapter controlling Pi through RPC mode. | Uses the installed Pi agent and its own configured models/credentials. | Resume/rollback/fork use Pi's native session files; Pi is not treated as generic ACP. [Provider internals](https://github.com/pingdotgg/t3code/blob/main/docs/internals/providers.md) · [Adapter source](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/orchestration-v2/Adapters/PiAdapterV2.ts) |
| Muse Code | Native `ProviderAdapterV2`; provider-specific protocol. | T3's provider setup expects its installed/authenticated provider runtime. | T3 documents it among native providers. [Adding a provider](https://github.com/pingdotgg/t3code/blob/main/docs/internals/adding-a-provider.md) · [Provider setup](https://github.com/pingdotgg/t3code/blob/main/docs/user/providers-muse.md) |
| Grok Build | A small provider-specific flavor over T3's shared ACP adapter, not a separate protocol implementation. | Runs the installed/authenticated Grok Build ACP command. | ACP handles shared session/event behavior; flavor covers differences outside ACP. [Adding a provider](https://github.com/pingdotgg/t3code/blob/main/docs/internals/adding-a-provider.md) · [Adapter source](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/orchestration-v2/Adapters/GrokAdapterV2.ts) |
| Antigravity | A small flavor over shared ACP, with a T3-managed runtime/installer. | Google sign-in and per-instance account profiles. | T3 isolates profiles and manages an immutable install so accounts/configs don't bleed across instances. [Provider internals](https://github.com/pingdotgg/t3code/blob/main/docs/internals/providers.md) · [Adapter source](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/orchestration-v2/Adapters/AntigravityAdapterV2.ts) |
| ACP Registry/local agents | Shared ACP client; prefers ACP v2 preview and negotiates v1 for older agents. | Each ACP agent owns its own sign-in and credentials; T3 launches its configured command. | Capabilities, options and session import are only exposed when the agent advertises them. Permissions remain provider-controlled. [ACP guide](https://github.com/pingdotgg/t3code/blob/main/docs/user/providers-acp.md) |

The provider list is not a list of model APIs. T3's installer/setup documentation describes agents as installed and authenticated on the machine where they run. Native integrations are chosen when a provider protocol carries features T3 needs; ACP is the generic extension path. [Install and setup](https://github.com/pingdotgg/t3code/blob/main/docs/user/install.md) · [Provider architecture](https://github.com/pingdotgg/t3code/blob/main/docs/internals/adding-a-provider.md)

### Codex cross-check

T3's Codex guide confirms both an existing authenticated Codex CLI login and a
T3-managed ChatGPT connection are supported ways to configure its Codex provider.
Its native adapter talks to Codex App Server and passes provider-owned
permission/question choices through, while its provider guide requires truthful
capability reporting. Maitty's daemon-owned App Server adapter matches the
transport and adapter boundary. T3's managed ChatGPT sign-in, account shadow
homes and broader account UI are separate product features; they are not needed
to match the integration pattern and should not be assumed in Maitty's current
GUI scope. [T3 Codex guide](https://github.com/pingdotgg/t3code/blob/main/docs/user/providers-codex.md) Â· [Codex adapter](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts) Â· [Provider integration rules](https://github.com/pingdotgg/t3code/blob/main/docs/internals/adding-a-provider.md)

## Claude subscription caveat

T3's documentation says it uses Claude Code login/config and its FAQ says it drives Claude through the Agent SDK. That verifies T3's implementation, not permission for Maitty to offer the same login flow. [T3 Claude guide](https://github.com/pingdotgg/t3code/blob/main/docs/user/providers-claude.md) · [T3 FAQ](https://github.com/pingdotgg/t3code/blob/main/docs/user/faq.md)

Anthropic's current Help Center says subscription usage is designed for ordinary use of Anthropic's native apps, including Claude Code; recommends API-key or supported-cloud authentication for third-party tools; permits some third-party use by paid subscribers only at Anthropic's discretion (possibly billed to usage credits); and prohibits misrepresenting third-party traffic or routing it against subscription limits. It specifically tells developers building products to use API-key or supported-cloud authentication. So T3's approach is not evidence that Maitty may offer Claude subscription login. Use Anthropic API / supported cloud credentials unless Anthropic confirms another path for Maitty. [Anthropic: Log in to your Claude account](https://support.claude.com/en/articles/13189465-log-in-to-your-claude-account) · [Claude Agent SDK quickstart](https://code.claude.com/docs/en/agent-sdk/quickstart)

## PandaCode cross-check

PandaCode is also provider-adapter based, but it does not use one integration
mechanism for every harness. Its public provider guide says it launches installed
CLIs directly: Claude Code runs in headless `stream-json` mode with the CLI's
stdio control protocol; Codex uses App Server; Pi uses RPC; Muse uses MSP; and
compatible agents use ACP. It reads Claude's exact session ID and resumes that
session. PandaCode documents permission callbacks and streaming, but also warns
that Claude's low-level control protocol is an implementation interface that
must be rechecked when upgrading Claude Code. [PandaCode provider guide](https://github.com/aicodeking/pandacode/blob/main/docs/providers.md) Â· [PandaCode README](https://github.com/aicodeking/pandacode)

This is not a visible-terminal bridge: the CLI emits structured JSON events and
accepts control messages over stdio. It avoids a separate Node SDK worker, which
would simplify Maitty's optional desktop packaging. The tradeoff is relying on a
lower-level Claude control protocol; its session-history and durable submission
semantics still need to meet Maitty's stronger recovery contract. The official
Agent SDK exposes session-message retrieval and a supported callback API, but
requires a Node worker that the daemon must discover even when it was started by
the TUI. The current [Claude transport decision](../adr/0039-claude-code-cli-streaming.md)
follows PandaCode's installed-CLI control protocol, subject to a capability
spike against Maitty's stricter history and recovery requirements.

## What Maitty should take from this

- Keep provider-specific adapters for native protocols (Codex App Server is already the model for Codex); normalize them at one adapter boundary and keep orchestration/UI independent of provider quirks.
- Add a generic ACP route for compatible agents instead of writing one-off integrations for every CLI. Add dedicated flavors only when an agent's behavior exceeds ACP.
- Treat each provider account/config as an isolated instance. Keep its auth in the provider's supported credential store or explicit per-instance API/cloud credentials; do not copy T3's Claude subscription login without Anthropic approval for Maitty.
- Mirror the useful engineering pattern, not necessarily each exact transport: use the provider's supported protocol/auth path, preserve its native session and approval semantics, and advertise only capabilities actually supported.
- Use PandaCode as a second protocol reference: Claude headless streaming is a viable alternative to its SDK worker, but compare exact history, acknowledgement, permission and interruption behavior before changing Maitty's recorded SDK decision.

## Mapping to Maitty's agent catalog

Maitty's terminal agent catalog is broader than the providers in the current
chat-GUI release. This mapping is an integration direction, not a claim that
these agents already support structured GUI conversations:

| Maitty agent family | T3-informed direction | Evidence still needed before GUI support |
|---|---|---|
| Codex | Keep the existing native Codex App Server adapter. | Current release's live continuity and platform acceptance. |
| Claude Code | Drive the installed CLI through headless `stream-json`, subject to supported authentication and platform checks. | Anthropic auth approval/choice; exact session, history, acknowledgement, approval, cancellation and Linux behavior. |
| Cursor, OpenCode, Pi | Investigate each provider's supported native runtime first, as T3 does. | Supported public interfaces, licensing/auth constraints, session and approval semantics. |
| Gemini, Copilot, Amp, Qwen, Aider, Goose, Crush, Droid, Kiro, Grok, custom agents | Prefer shared ACP where the installed agent advertises the required capabilities; otherwise leave terminal-only until an explicit adapter case is justified. | Per-agent ACP availability/version, advertised features, lifecycle and auth behavior. |

Do not assume the T3 provider list and Maitty's catalog are identical, or that
ACP support exists just because an agent has a CLI. For each addition, record
the chosen transport, minimum supported version, auth ownership and truthful
capability set in its stack/architecture documentation before implementation.

## Source scope

T3's own provider docs and repository were the primary sources for implementation. Anthropic's first-party Help Center and Claude Code SDK docs were used for the Claude authentication caveat. No forum posts or third-party comparisons were used as authority.
