# Herdr status compatibility audit

Research date: 2026-10-16. Upstream reference: Herdr commit
[`143bf5fd4781d164260c63fe28e7dc08054ad32f`](https://github.com/herdrdev/herdr/tree/143bf5fd4781d164260c63fe28e7dc08054ad32f).
This is a source audit, not evidence that every real agent/provider combination
has been run. Seshi observations below describe the local worktree at the start
of this audit, including its unreleased Pi/OpenCode adapters. Implementation
changes and their verification belong in the task handoff.

## Implemented and verified locally

The final worktree recognizes all 24 Herdr identities plus Seshi's existing five
additional agents. It bundles all 22 unmodified pinned screen manifests and uses
their regions, gates, priorities, OSC evidence and viewer-preservation rules.
Native setup now includes Pi, OMP, OpenCode, Kilo, Kimi and Mastracode.
OpenCode includes V1/V2 per-TUI ownership and comment-preserving JSONC setup.
Codex's exact screenshot footer, soft wrapping, queued/expanded questions and
automatic continuation after an accepted completion have regression coverage.
Recognized process exit/replacement releases hook identity rather than retaining
stale working status. Focused questions ring after 300 ms still blocked; red
status is immediate. Unseen completion settles for one second. Client and
headless daemon both recheck the queued status and cancel obsolete alerts;
a new working turn clears same-news deduplication.

Verification: the ordered Windows build, full tests (307 passed, four ignored),
strict all-target clippy, Linux x64 and macOS ARM cross-checks passed. Ten Node VM
adapter tests passed, including nested asks, retries, failures, TUI ownership,
initial idle and disposal. Config tests cover native schema shapes, idempotency,
uninstall, invalid config preflight, JSONC comments and migration deferral.
Rendered frames were inspected. A named isolated Windows server ran a synthetic
Codex-shaped Node process through working → blocked → working → idle → process
exit. The daemon's reported states matched the displayed indicators. The
completion report crossed the real authenticated hook socket. The owned test
process tree was stopped and reveal registration was unchanged.

This establishes the listed coverage, not universal runtime parity. No real
provider job was launched. Remaining differences include Herdr's full wrapper/
foreground-process parser and Letta helper exclusions, sequenced lifecycle
reports across separate hook invocations, per-client completion acknowledgements,
100 ms/three-confirmation/700 ms ambiguous-idle debounce, local/remote manifest
updates, session-only metadata installers and notification delivery settings.
Seshi retains its five-second ambiguous quiet hold and four-second global/
two-minute same-news sound throttles. Shared OpenCode behavior is exercised with
V1/V2 API fixtures, not two real attached clients. Kimi/Mastracode configuration
is validated against the source schema, not a real authenticated agent launch.
These are verification/compatibility limits, not claims that every agent/provider
or CLI version has been tested live.

Implementation adds pinned `jsonc-parser` 0.33.1 to preserve native JSONC
comments/settings and redistributes Apache-2.0 source with LICENSE/NOTICE.
It introduces no wire shape, database migration, permission or decision-marker
change. The baseline comparison below is retained for traceability.

## Finding

Seshi did not have exact Herdr parity at audit start. Identity recognition,
screen evidence, native integration availability, source arbitration and
notification delivery are separate capabilities. Adding an executable name
alone does not establish reliable working/blocked/completed detection.

Herdr registers 24 identities and 22 screen manifests. OMP and Mastracode use
lifecycle integrations without bundled screen manifests. Seshi initially
registered 18 identities, sharing 13 with Herdr; eleven Herdr identities were
missing. Seshi additionally recognized Aider, Goose, Crush, Auggie and the
separate DeepSeek harness. The inventory comes from the respective
[Herdr identity registry](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/mod.rs#L43)
and [Seshi built-ins](../src/config.rs).

## Agent coverage at audit start

In the table, **screen** means a bundled Herdr screen manifest; **lifecycle**
means one of its six explicitly hook-authoritative integrations. **Identity**
means a session metadata integration, which must not be advertised as complete
lifecycle reporting. Herdr also installs several other hooks that are not
classified as full lifecycle authorities. The authoritative source for those
distinctions is its
[source classification](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/mod.rs#L329),
not simply the presence of an install command. Installable targets are listed in
the [integration schema](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/api/schema/integrations.rs#L32).

| Herdr identity | Herdr status sources | Seshi recognition | Seshi installable status integration |
| --- | --- | --- | --- |
| Pi | Screen + lifecycle | Yes, current and old packages | Pi extension |
| Claude Code | Screen + hooks | Yes | Claude hooks |
| Codex | Screen/OSC + hooks | Yes | End-of-turn notify; screen supplies working/blocked |
| Gemini | Screen | Yes | Gemini hooks |
| Cursor | Screen + hooks | Yes | None |
| Devin | Screen + session hooks | Missing | None |
| Antigravity (`agy`) | Screen + identity | Missing | None |
| Cline | Screen | Missing | None |
| OMP | Lifecycle | Missing | None |
| Mastracode | Lifecycle | Missing | None |
| OpenCode | Screen + lifecycle; TUI ownership adapter | Yes | Server plugin only |
| GitHub Copilot | Screen + session hooks | Yes | None |
| Kimi | Screen + lifecycle | Yes | None |
| Kiro | Screen | Yes | None |
| Droid | Screen + session hooks | Yes | None |
| Amp | Screen | Yes | None |
| Grok | Screen + hooks | Yes | None |
| Hermes | Screen/OSC + identity | Missing | None |
| Kilo | Screen + lifecycle | Missing | None |
| Qoder CLI | Screen + session hooks | Missing | None |
| Qwen | Screen + identity | Yes | Qwen hooks |
| Letta | Screen/OSC + experimental identity integration | Missing | None |
| Maki | Screen | Missing | None |
| Muse | Screen | Missing | None |

Sources: Herdr's
[22 manifests](https://github.com/herdrdev/herdr/tree/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests),
[integration event registrations](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/mod.rs),
and Seshi's [integration dispatcher](../src/cli.rs) and
[Pi/OpenCode installers](../src/cli/integrations.rs). Letta is explicitly outside
Herdr's frozen integration target enum and exposed as experimental.

The user's z.ai and DeepSeek sessions run through Pi/OpenCode. Those providers
do not add terminal-app identities: lifecycle events belong to the host app.
Provider failures, retries, permissions and questions still need correct host
handling. This follows directly from the provider-independent event handlers in
the [Pi adapter](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/assets/pi/herdr-agent-state.ts)
and [OpenCode adapters](https://github.com/herdrdev/herdr/tree/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/assets/opencode).

## Identity and screen detection

Herdr inspects the pane's foreground job, executable and executable/script
arguments. Its parser recognizes wrappers and package entrypoints; it avoids
classifying arbitrary `node -e`, Python snippets or incidental source mentions
as an interactive agent. Notable aliases include `opencode2`, `amp-local`,
`grok-build`, `qoderclicn`, `qoder`, `qodercn`, and versioned
`muse-bin-<digit...>` executables. Pi and OMP package paths are distinct.
An interactive Letta invocation is distinguished from helper commands. See the
[process parser and regressions](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/mod.rs#L252).

Herdr's manifests define prioritized rules, matching regions, positive/negative
gates and confidence metadata. Regions include live composer boundaries,
bottom nonempty lines, OSC title and OSC progress. A transcript/history viewer
can explicitly preserve the preceding state. The engine is more expressive
than Seshi's initial per-agent working/blocked regex lists. Its local/remote
manifest selection and diagnostic explanation are also separate features.
See [manifest engine](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifest.rs).

Examples that require more than matching a single transcript phrase:

- Codex: current approval/question controls outrank activity; history viewers
  preserve state; title activity and action-required signals are evidence.
  [Manifest](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests/codex.toml)
- Claude: live form and composer boundaries distinguish current dialogs from
  history; background agents/MCP activity have specific rules.
  [Manifest](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests/claude.toml)
- OpenCode/Kilo: permission and question controls occur together; matching
  `esc dismiss` alone is insufficient.
  [OpenCode](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests/opencode.toml),
  [Kilo](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests/kilo.toml)
- Antigravity: dialogs and working turns share cancellation wording, so the
  footer shape and navigation controls matter.
  [Manifest](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests/antigravity.toml)
- Letta: stale spinners remain in history; OSC signals and current running
  rows are used instead of a generic spinner match.
  [Manifest](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests/letta.toml)
- Muse: permission answers and question controls are paired to avoid classifying
  assistant prose as a blocker. Maki uses its bottom status bar.
  [Muse](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests/muse.toml),
  [Maki](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/detect/manifests/maki.toml)

## Lifecycle ownership and timing

Herdr treats Pi, OMP, Mastracode, OpenCode, Kilo and Kimi as full lifecycle
authorities only while their expected process is present. Sequenced reports,
session ownership, process exit and stale-session suppression prevent an old
report from reviving another session. Their live hook state is not overridden
by screen matching. For other hook sources, a newer strong visible blocker for
the same detected agent can override a nonblocked hook. See
[arbitration](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/terminal/state.rs#L2032).

For screen fallback, Herdr delays an uncertain working-to-idle transition:
recheck every 100 ms, accept after three confirmations, cap the hold at 700 ms.
Explicit live idle evidence, blockers, process exit and identity changes bypass
that hold. Stable visible blockers refresh at 800 ms. Startup grace is three
seconds. Seshi initially used a five-second quiet-output hold for screen-driven
completion; it has different dismissal/turn-end rules. These timings are not
equivalent. [Herdr detection transitions](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/pane/agent_detection.rs),
[Seshi status state machine](../src/daemon/status.rs).

Pi's upstream adapter gates the root session on `ctx.mode === "tui"` because
RPC can also report `hasUI`. Reload derives running state from `ctx.isIdle()`;
`agent_settled` finishes only after that reports idle. Blocking dialogs use a
count, so one resolution cannot clear another. Seshi's initial adapter already
used settlement and prompt counts, but reset to idle on reload and did not gate
non-TUI sessions. [Pi source](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/assets/pi/herdr-agent-state.ts).

OpenCode's server events can be shared by several terminals. Herdr has a
separate TUI adapter for the session selected in each pane, with V1/V2 loading,
route changes, ancestry resolution, initial hydration of statuses/asks,
retries and disposal. Its server plugin refuses ownership for normal shared
server/attached TUI lifecycles. Seshi's initial server-only adapter inferred its
active root from busy events; this cannot establish exact ownership for every
attached/shared-server pane. [Server adapter](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/assets/opencode/herdr-agent-state.js),
[TUI adapter](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/assets/opencode/herdr-tui-session.js),
[V2 entrypoint](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/assets/opencode/tui.js).

OMP additionally excludes nested `OMPCODE=1` invocations, tracks approval/ask
lifecycles, ignores scheduled continuation and duplicate end events, debounces
idle by 250 ms and holds retryable failures working for 2.5 seconds before
blocking. Mastracode installs eleven event mappings in `~/.mastracode/hooks.json`.
These cannot be replaced with an unqualified process-output fallback.
[OMP adapter](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/assets/omp/herdr-agent-state.ts),
[Mastracode mappings](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/integration/mod.rs#L298).

## Notifications and presentation

Herdr distinguishes backend idle from an unseen completion. A client projects
`Done` using completion evidence and acknowledges it only after a current pane
surface is shown while the outer terminal is focused. Restart/connection
generation and state sequence prevent stale surfaces from marking new work
seen. Seshi initially marks focused completion idle in the daemon; this is
similar in ordinary use but not the same multiple-client presentation model.
[Herdr completion projection](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/client/shell/endpoint_agent_state.rs),
[Seshi hook handling](../src/daemon/status.rs).

Herdr requests a needs-attention sound when entering blocked even on the active
tab. Completion requires a working/blocked-to-idle transition; initial discovery
is not completion. Active focused-tab completion sounds/toasts are suppressed.
Delivery supports app, terminal, system or off, configurable delay, an eight-item
queue and per-pane replacement. Delayed notifications are checked against the
current state; completion can wait for a snapshot for up to one second in
50 ms increments. Toast duration is eight seconds for attention and five for
completion. [Transition policy](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/app/actions.rs#L27),
[delivery policy](https://github.com/herdrdev/herdr/blob/143bf5fd4781d164260c63fe28e7dc08054ad32f/src/client/shell/notification_policy.rs).

Seshi initially suppresses both sounds and notices for the focused pane in a
focused window. Unseen completion alerts wait twenty seconds; same-status
sound deduplication is 120 seconds and global sound spacing is four seconds.
Its daemon shows desktop alerts when no viewer is attached; phone alerts are
a separate existing option. This policy differs from Herdr and should not be
described as exact notification parity.
[Client policy](../src/client/background.rs), [daemon policy](../src/daemon/status.rs),
[alert timing](../src/alert.rs).

Retain explicit Seshi requirements: unanswered questions use the blocked dot
and red blocked text; terminal bells never create a musical-note agent status;
and reports must authenticate through the pane's verified process tree or
`SESHI_PANE_TOKEN`. These are user/project contracts, not permission to copy
another application's transport or change its trust boundary.
[Status contract](agent-status.md), [security contract](SECURITY.md).

## Verification needed before claiming parity

- Cover all 24 identities with process-entrypoint fixtures, including negative
  helper/eval cases, npm/native wrappers, Windows paths and alternate aliases.
- Exercise every bundled manifest and boundary region against current working,
  idle, blocker, history, narrow/wrapped and stale-text fixtures.
- Prove lifecycle ordering, reload, retries, nested helpers, multiple simultaneous
  asks, cancellation, process exit and transport failure for each native adapter.
- Test OpenCode session switching and two attached clients against one server;
  event ancestry alone does not establish the pane's selected session.
- Test focused/background/unfocused-window/detached notifications, stale
  completion and duplicate delivery independently from icon/color rendering.
- Run the repository's Windows build/test/clippy and Linux/macOS cross-check
  gate, rendered UI checks and isolated live integration probes. Passing mocks
  does not verify every paid provider or CLI version.

Upstream was read only; no Herdr scripts or live provider jobs were executed
for this audit. This report introduces no dependency, config, permission or
wire change and fills no documentation decision markers.
