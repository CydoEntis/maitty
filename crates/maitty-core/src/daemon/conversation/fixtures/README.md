# Synthetic daemon provider fixtures

The fake Codex and Claude executables used by daemon integration tests are
sanitized, local fixtures. Codex compatibility schemas live with the
`maitty-provider-codex` package that validates them.

## Recorded transcript replays

`claude_transcript.jsonl` is a Claude Code transcript with the shape of a real one and none of
its content: a fictional task ("add a greeting function"), made-up ids, paths under
`/home/user/demo` and fixed 2026 timestamps. It holds a summary line, a snapshot line, a side-chain
line (skipped by the follower), a thinking block, two tool calls, an edit and an `AskUserQuestion`
call. `follow_transcript/tests.rs` replays it through the transcript follower and asserts every
history item in order. Keep the file LF; the test reads it with `lines()`, so a CRLF checkout on
Windows replays the same.

Record a new one by running the agent on a throwaway task in a scratch folder, then reading its
file under `~/.claude/projects/<folder>/<session>.jsonl`. Only learn the line shapes from it. Do
not copy a recorded line into the repo and edit it; write each line fresh from the shape, so a
forgotten field cannot leak. That recording holds your real prompts and paths: keep it local and
never paste it into an issue, a chat or a model. Clean-up checklist for a new fixture:

1. Every text, command, path, file name and tool output is invented. No real project, user, host,
   email or repo name.
2. `cwd` and every path sit under `/home/user/...`.
3. `uuid`, `parentUuid`, `sessionId`, message and tool ids are made up (for example
   `00000000-0000-4000-8000-...`), and `signature` is a placeholder.
4. Timestamps are a fixed made-up day, a few seconds apart.
5. No token, key or secret of any kind (`sk-`, `ghp_`, `Bearer`, long random strings). Grep the
   file for your home path, login name, machine name and those prefixes before committing.
6. Keep it small (about 15 lines) and keep one line per kind of block the test needs.

### Codex

`codex_rollout.jsonl` is a Codex rollout (session file) built the same way: a fictional task, made-up
ids, paths under `/home/user/demo` and fixed 2026 timestamps. It holds `session_meta`,
`turn_context`, the injected environment message (skipped), the person's message, one reasoning
item with a summary and one with an empty summary and only `encrypted_content` (no history item), a
shell command with its output, an `exec_approval_request` event, a `token_count` event, an
`apply_patch` call with its output, the closing message and `task_complete`.
`a_recorded_codex_rollout_replays_to_its_history_in_order` replays it and asserts the items in order.
The follower makes no history item from an approval event; the test says so. No rollout on the
machine this was written on held an approval event, so that line's shape comes from the Codex
protocol, not from a recording.

Record a new one by running Codex on a throwaway task in a scratch folder, then reading the newest
file under `~/.codex/sessions/<year>/<month>/<day>/rollout-*.jsonl`. Learn only the line shapes (print
field names and types, never values), and write each line fresh. The recording holds your real prompts,
paths, account ids and your instructions; keep it local and never paste it into an issue, a chat or a
model. Apply the checklist above, and also: `base_instructions`, `encrypted_content`, account, creator
and thread ids, `repository_url` and every `commit_hash` are placeholders.
