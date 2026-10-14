# Agent profiles

Open **Agent profiles** from the command palette, or press **Ctrl+P** in
**New**. Create a role, or choose one of six editable
Planner / Worker / Reviewer starters for Claude Code and Codex. The starters
use medium effort; Claude uses `opus` / `sonnet` aliases and Codex uses its
configured model. Enter an exact model identifier to pin a version.

## Profile contract

Store personal roles as `[[profiles]]` in Seshi's `config.toml`. If
`config.local.toml` supplies the profiles array, the editor updates that file
instead. Store project roles in `.seshi.toml`. A project's complete definition
replaces the personal role with the same name; other personal roles remain
available. A worktree's `.seshi.toml` takes precedence over the main checkout's.

```toml
[[profiles]]
name = "planner"
agent = "claude"
model = "opus"
effort = "medium"
instructions = "Investigate the request and produce a plan. Do not implement it."
skills = ["ponytail", "show-me"]
```

- `name`: starts with a letter; letters, digits, `-` and `_`; at most 64 bytes.
- `agent`: `claude` or `codex`.
- `model`: blank for the agent's configured default, otherwise its model name.
- `effort`: blank, `low`, `medium`, `high`, `xhigh` or `max`. The selected model
  and installed agent version must support the requested level.
- `instructions`: role instructions, at most 16 KiB. Project instructions apply.
- `skills`: installed skill names, separated by commas in the editor. Claude
  preloads the named skills through its agent definition. Codex receives an
  explicit instruction to use those skills and report missing ones. This is
  a request, not an exclusive skill allowlist; existing skills remain available.

Profiles do not install skills, plugins, MCP servers, or models. The agent's
existing setup, authentication, permission policy and MCP configuration still
apply. A Planner or Reviewer starter's instruction to avoid edits is behavioral,
not a separate permission boundary.

## Launch and workflow use

Enter on a profile opens **New** with that role selected. It appears with a `◇`
in the RUN row. Opening profiles from an existing worktree keeps that checkout
selected for the new agent, so a reviewer can inspect the worker's actual changes.
Choose a task and checkout, then launch. The preview resolves the selected
checkout's overrides. PgUp/PgDn scroll the role instructions. Every launch starts a
new session; Seshi does not resume or fork a previous agent conversation. Pass
requirements, plans and the target checkout explicitly when requesting a review.
Tasks are limited to 8 KiB. The daemon owns the pane and all its processes.

Scripts can use:

```sh
seshi profile list
seshi profile run planner -- "Plan the requested change"
seshi profile export planner
seshi profile export planner --project
seshi profile import path/to/agent.toml --project
```

**Export** writes `.claude/agents/<name>.md` or `.codex/agents/<name>.toml`, in
the user's home folder for personal roles or the project folder for overrides.
Personal Codex exports honor `CODEX_HOME`. Native workflows can delegate to that
role using their own agent mechanism. Export again after editing a profile;
native files are snapshots, not automatically synchronized. Seshi only replaces
exports carrying its ownership marker and refuses collisions with other files.

**Import** copies supported fields into an editable draft without changing the
source. Save the draft to select its scope. Codex imports accept `name`,
`description`, `model`, `model_reasoning_effort` and `developer_instructions`.
Claude imports accept scalar `name`, `description`, `model`, `effort`, a JSON
`skills` array, and the Markdown prompt. Multiline YAML metadata and unsupported
native settings are rejected instead of silently discarded. The CLI saves an
import immediately and rejects a duplicate name in the selected scope.

Claude and Codex have different inheritance rules. In particular, native Codex
subagents inherit permissions and MCP servers from their parent. Exporting a role
does not create an independently sandboxed subagent.

Profile setup is local; it is unavailable over SSH. Profiles are not offered in
the race chooser. Workflow sequencing, approval stages and automatic review/fix
loops are outside this version.
