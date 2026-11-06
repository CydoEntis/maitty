# Security

This doc covers how the project handles untrusted input, secrets,
authentication, authorization, and dependencies. `AGENTS.md` is the operating
contract; this doc holds the security practices in depth.

## Per-change checklist

Ask these questions for every change. If any answer is yes, add validation, a
least-privilege control, safe failure behaviour, and a targeted test. Then call
it out in the handoff.

- Does it accept input from a user, file, URL, network, IPC, webhook, or
  environment?
- Does it add or widen a permission, scope, role, CORS rule, CSP rule, or
  storage policy?
- Does it add an endpoint, a route, or a way to reach data?
- Does it read, store, or log credentials, tokens, or personal data?
- Does it render user-controlled content, or open or redirect to a URL?
- Does it run a command, build a file path, or build a query from input?
- Does it add a dependency or a build step that runs code?
- Does it read or act on something another machine's daemon sent (a `ServerMsg` from a
  `[[machines]]` entry)? Decide it in the remote-daemon table under *Project-specific: threat
  model*, clean what it carries, and test the ignored path.

## Input and output

- **Validate at the boundary** with a schema. Reject anything unknown. Don't
  just strip it and continue.
- **Injection.** Parameterize queries. Never build shell commands, file paths,
  SQL, or regexes from input by concatenation. Use allowlists for identifiers,
  such as sort fields or file names.
- **Path traversal.** Resolve paths and confirm they stay inside the intended
  directory.
- **XSS.** Rely on the framework's escaping. Never render raw HTML from input
  (`dangerouslySetInnerHTML`, `innerHTML`, `v-html`). If rich content is
  required, sanitize it with a maintained library.
- **Open redirects.** Redirect only to relative paths or an allowlist of
  hosts.
- **SSRF.** Server-side fetches of user-supplied URLs go through an allowlist,
  and never reach internal addresses.
- **Uploads.** Check size, type by content rather than extension, and
  filename. Store uploads outside the web root, or in object storage with
  private access by default.
- **Errors.** Return safe messages. Stack traces, SQL, and internal paths stay
  in server logs.

## Authentication

- Use a proven provider or library. Never write your own password hashing,
  token signing, or session handling.
- Tokens and session cookies are `HttpOnly`, `Secure`, and `SameSite`. Don't
  keep long-lived tokens in `localStorage`.
- Expire sessions, rotate refresh tokens, and invalidate sessions on logout and
  on password change.
- Rate-limit login, signup, password reset, and any endpoint that sends email
  or SMS.
- Keep responses generic, so they don't reveal whether an account exists.

## Authorization

- **Deny by default.** Every route, action, and query needs an explicit rule.
- Check authorization on the server, against the specific resource, on every
  request. Hidden UI is not access control.
- Never trust a user id, role, tenant id, or price sent by the client. Derive
  each one from the authenticated identity or from the server's own data.
- **IDOR.** Every lookup by id also checks ownership or permission.
- Test the denied path. Every protected operation has a test that shows an
  unauthorized caller is refused.

## Secrets

- Secrets live in the environment or a secret manager, never in source,
  fixtures, snapshots, docs, or examples. See [ENVIRONMENT](ENVIRONMENT.md).
- `.env.example` has keys and placeholder values only.
- Anything shipped to a browser or client bundle is public. That includes
  build-time public env vars. Never put a secret there.
- If a secret is committed, rotate it first, then remove it. Rewriting history
  alone doesn't revoke it.
- Never print secrets in logs, errors, or command output, including during
  debugging.

## Dependencies and supply chain

- Commit the lockfile. CI installs with a frozen lockfile.
- Before adding a package, check that it is maintained, check its download
  count and publisher, check for typosquats, and check any install scripts.
  Prefer fewer, well-known packages.
- Run the audit command in CI (see *Project-specific: tooling*). Fix or
  consciously accept high and critical findings, and record any accepted risk
  with a reason.
- Update dependencies deliberately and regularly, in their own commits.
- Pin CI actions and container base images to a version or digest.

## Data and privacy

- Collect the minimum data the feature needs.
- Never log personal data, tokens, passwords, or request bodies that contain
  them. Redact them in structured logs.
- Add no analytics, telemetry, or third-party scripts without an approved task
  and a documented privacy behaviour.

## Headers and transport

- Serve everything over HTTPS only.
- Set a Content Security Policy, and avoid `unsafe-inline` or `unsafe-eval`
  unless there is a recorded reason.
- Keep the CORS allowlist explicit. Never combine `*` with credentials.

## Project-specific: threat model

The Tauri webview is an untrusted client. Keep capabilities limited to the
application's own windows; validate command inputs in Rust; expose no generic
filesystem, process or shell bridge. Conversation authority and credentials
remain in the daemon and its authenticated IPC.

The native host may start only the bundled CLI sidecar with the fixed `daemon`
argument. Resolve its path from the application bundle; never accept a program
path or argument list from the webview. Remove inherited pane, structured
conversation and remote-route variables before starting it, while preserving
the user's local config, data and socket overrides.

Assets: the user's shells and agent sessions (which can run any command as the
user), their source code, and API keys in `config.local.toml`. Threats: another
local user or process driving the daemon's socket; a process inside one pane
forging status for another pane; command injection through names, branches or
paths that end up in shell commands; secrets leaking through config sync or logs;
an SSH remote being someone else's machine; what agents work on leaking through phone alerts
(ntfy, off by default: they name the agent and folder only, unless `phone_text` is on, and
the topic name is the only secret on the public server).

The agent launchers (`crates/maitty-core/src/shim.rs`) sit first on `PATH` in every plain shell pane. They are
files in maitty's own data folder, written by the daemon as the user; a process that can write
there can already run as the user through the shell startup files maitty writes beside them.
A launcher asks for a worktree with `maitty agent-dir`, which sends the pane's own
`MAITTY_TERM_ID` and `MAITTY_PANE_TOKEN` and nothing from its arguments; a wrong or missing token
makes the caller "from a pane" but not that pane, and the daemon answers with nothing (worktrees,
like status reports, are trusted only from the pane itself). A launcher passes the agent's
arguments on as an argument vector, never through a shell, and the paths it embeds are
single-quoted.

The daemon runs the user's own `$SHELL -ilc` once at start to learn the login PATH
(`crates/maitty-core/src/login_path.rs`; unix only). It is as safe as the user's shell
startup files already are, and bounded further: the shell is started by path as an argument
vector, never through a shell, and only when `$SHELL` is an absolute path to an existing file
(else `/bin/sh`); it has no terminal (its own session) and null stdin, its error output is
discarded, and the `MAITTY_*` variables (pane id and token, socket, routing) are removed from its
environment; a timeout kills its whole process group; its output is read only between two
marker words, capped in size, and used as PATH entries, never executed or logged. It runs in
the daemon, on the machine the daemon serves, never on a client.

A project's check is a command the daemon runs as the user when an agent's turn ends. It
comes from the user's own `config.toml` (`[[check]]`) or, for a project with none there, from
the repo's `.maitty/checks.toml` under the approval rules below. It runs through the user's shell in the
session's worktree with no stdin and without `MAITTY_PANE_TOKEN`, so it cannot report status
as the pane. On timeout its whole process tree is killed (process group on unix, job object
on Windows). If the daemon dies, a check dies with it on Windows (the job closes) and on
Linux (a parent-death signal kills the shell; a descendant the shell already started is
left to finish or to fail on its closed pipes). On macOS there is no such call, so a check
running when the daemon crashes lasts until it ends or writes to its closed pipes. Its output is a capped tail held by the daemon, saved beside the pane's output
files and served only by `Query::CheckOutput` under the read grant; it is never logged and
never in the state snapshot. It may contain secrets the command printed, like any pane's
output.

A structured conversation's provider (Codex app-server, Claude) is a process the daemon owns,
started in its own process group (a job object on Windows). `kill-server` stops every provider's
process tree before the daemon exits and waits up to five seconds for each to be stopped. If the
daemon dies without that, a provider dies with it on Windows (the job closes). On Linux and macOS
it and the processes it started keep running until the next daemon starts: the parent-death signal
that ends a check cannot be used, because a provider is started from a thread that does not live as
long as the daemon. Each provider's process group is recorded in `<socket id>.providers`, and a
daemon that holds the socket stops every recorded group that still has processes before it restores
anything. A group whose leader's start time differs from the recorded one is left alone, so an id
that has been given to another process is never signalled; ids 0 and 1 and negatives never are.

An agent can start helpers only if you say so. Who is asking is proven the way status reports are
(the pane's `MAITTY_PANE_TOKEN` on the connection); the answer is `AnswerHelpers`, which the daemon
refuses from any pane and accepts only for a request it holds. "Always in this project" is kept in
`helper-projects.json` in Maitty's data folder, never in the repo, so an agent that can write the
repo cannot grant itself helpers. A helper's own permission prompts still come to you.

What a pull request's reviewers and bots wrote reaches the pane's agent as a queued message
(`sessions.pr_wake`, on by default), so it is untrusted text: it is cut short, flattened to one
line, withheld when it holds hidden characters, framed as other people's words and not
instructions, and capped at ten messages per commit. `gh` is run with prompts off and is given
only a branch's checkout or a validated GitHub pull request address.

An agent that needs a secret asks you for it (`RequestSecret`, proven the same way) instead of
asking in chat. Only you answer (`AnswerSecret`, refused from any pane). The daemon keeps what
you type in memory only (a delivered file is `0600` in a `0700` folder on unix; on Windows it
sits in your own app data, which only you and administrators can read, and is removed within a
day or when the daemon starts), and gives the agent a single-use reference that runs out after 24 hours
and works only for the pane that asked ([ADR 0006](adr/0006-secrets-never-enter-chat.md)). The
value is never in the snapshot or a log line (`SecretText` prints as `<hidden>`), and the label
and reason an agent supplies are checked for control and hidden characters before you read them.
A project may name several check commands. Each one runs under the same rules, one after
another in one background task, and each keeps its own output tail. A command the
resolver holds for approval or that you said never to is not started at all. To tell when
a pass goes stale, the daemon writes a tree of the worktree (tracked and untracked files,
`.gitignore` respected) when a run starts and again on the git poll, through a private
index file in the git directory, as snapshots do. It never touches the user's index; the
tree's blobs go into the repo's object store, where `git gc` drops them.

A repo-supplied command never runs until you approve it, so cloning a repo, or an agent
editing `.maitty/checks.toml`, cannot make Maitty run anything. The rules:

- Your own `[[check]]` for a project wins whole; the repo's file is then not read.
- Approval is per exact command: Always is kept with the command's sha256, and a changed
  command asks again. Never holds whatever the command becomes. Once lets one run of that
  pane's checkpoint go and is never saved.
- Answers live in `check-approvals.json` in Maitty's data folder, outside every repo, written
  atomically (synced, then renamed). The daemon reads the file once at start and goes by its
  own copy, so writing the file changes nothing while it runs. Every daemon shares the file,
  so an answer is written into the file as it is then, never as the daemon's whole copy, and
  a damaged file is left alone.
- Only you approve: `ApproveCheck` and `ForgetCheckApproval` are refused from every pane
  connection whatever its grants, and approve only a checkpoint waiting for approval with
  the command it waits with. `ApproveCheck` carries the sha256 of the command the modal
  showed, and the daemon refuses it for any other. No key press approves on its own; the
  client sends the choice.
- You approve what you read: a repo's `command` or `name` holding a line break, tab or other
  control character is refused with the file. The modal shows the whole command, wrapped,
  with invisible characters spelled out and a run of spaces shown by its length; until a
  frame has shown all of it, Once and Always are refused.
- A file that cannot be read or parsed runs nothing and reads as could-not-run with the reason.

Residual risk: a process running as the same OS user can edit `check-approvals.json`, which
counts after the next daemon restart, and a process that hides its pane identity (no
`MAITTY_TERM_ID`) is treated as you, as for grants. Both need code already running as you.

Overriding a failed check is a recorded, visible action: the time and the reason stay on the
result and its `Overridden` state shows wherever the check does. It needs the write grant on
the target pane, and a pane can never override or undo its own check, so an agent's claim
cannot turn its failure into done. An override covers one result; the next run replaces it.

`maitty handoff` copies the tail of a pane's screen, its prompts and its changed file names into
another agent's first prompt, so whatever the screen showed (secrets included) reaches that
agent. It adds no permission of its own: the screen read needs the read grant on the pane and
the new pane needs the start grant, both checked by the daemon as for `maitty read` and
`maitty split`. The brief is quoted with `Config::quote_for_shell` like any first prompt; for
PowerShell that escapes the curly single quotes as well as `'`, since PowerShell ends a
single-quoted string at either.

Text sent to an agent through `maitty send` (and `--wait`), MCP `maitty_send` and a
`maitty_start` task is refused when it holds characters that draw as nothing or reorder the text
(`hidden::is_invisible`: zero-width and direction marks, the BOM, Unicode tag characters), so
one agent cannot hide instructions to another from you. The error names each one and where it
is; nothing is sent. Your own typing and pastes in a pane are not checked.

References (`crates/maitty-terminal-client/src/references.rs`) are files the user puts in `.maitty/references/` at the repo's
main checkout; agents read them through MCP `maitty_references`, which returns paths and never
file contents. An agent's `path` must name an existing folder in its own project unless
`[mcp] scope = "all"`. Nothing outside that folder is listed or removed: a path with `..` is
refused, a link is listed only when it resolves inside the folder, and a references folder (or
`.maitty`) that is a link out of the project is refused; a link is never written through either,
since a new reference is made create-new. An agent's `path` is checked against its project by its
words before anything on disk is touched, so it cannot probe what exists elsewhere, and network
paths are refused outright (no SMB connection from a check). A paste into the References view is
added only when it is one line naming a file; anything else is not repeated in a toast, as it may
be a secret. The folder is kept out of git through `info/exclude`, since a reference is often a
screenshot not meant to be shared.

Saved pane output and check tails can hold secrets that were on screen. On unix their folders
are 0700 and their files 0600 (`persist::private_dir`, `create_private`); on Windows they sit
in your own profile. `[restore] output = false` stops saving pane output and deletes it.

A `[[machines]]` entry's `target` and `ssh_args` are user input that reaches `ssh`, so an option
smuggled into either runs code on this computer (`-oProxyCommand=...`). The config load refuses
a `target` that starts with `-` or holds whitespace or control characters, an empty or
control-character `ssh_args` entry, and a bad `remote_cmd`; `ssh_args` is a list and is never
split on spaces. The ssh command itself is built in one place (`ssh_command.rs` in `maitty-machines`, shared by the terminal app and the desktop app): ssh runs with
`-o BatchMode=yes` (no prompts) and `-o ForwardAgent=no` and `-a` (no agent forwarding), plus
`ConnectTimeout` and `ServerAlive*` defaults that `ssh_args` may override; `BatchMode` and
`ForwardAgent` come before `ssh_args` so ssh's first-value-wins rule keeps them. The host follows
`--`, and a host that starts with `-` is refused there too, since `--remote host` never passes
the config load. The remote command is one string for the remote user's login shell, so each
word is POSIX single-quoted whatever the client runs (a Windows client to a Linux remote
included); `remote_cmd` is one program path, a control character or newline in it is refused,
and only a leading `~/` stays unquoted. A remote machine is untrusted, so the pane id and token
a command carries (`Hello.from`) are sent only to this computer's daemon, and `MAITTY_REMOTE`,
`MAITTY_SSH` and `MAITTY_REMOTE_CMD` are removed from every pane's environment. The table is
read only from `config.local.toml` and refused in the synced
`config.toml`, so a shared config cannot point this computer at a host.

A machine's `via` is a program this app runs, so it is code: it is read only from
`config.local.toml` and a `via` in the synced `config.toml` is refused with the rest of the table, naming the
reason. It is an argv list and never a shell line: the program is `Command::new(via[0])` with the other
entries as arguments, no shell reads it, and the remote command (`remote_command`, the same quoting as for
ssh) is one argument, so a hostile `remote_cmd` stays inside it. The load refuses an empty list, a program
that is empty, starts with `-` or is `{command}`, a control character anywhere, and a `{command}` that is not
an entry of its own or appears twice. Nothing of ssh is added (no `-o`, `-a`, `--`): the transport is
trusted exactly as far as the user who wrote it. Its stdin and stdout are plain pipes (never a pty), stderr
is collected and cleaned before it is shown, and on unix it runs in its own session so it cannot read the
controlling terminal. It is refused on Windows, at load and at connect. Its arguments can hold tokens, so
only the program's name is shown (in `doctor`) and nothing of it is logged. Its failures are never read for
ssh's login words.

The Machines dialog adds two more ssh command lines, built beside the first in `ssh_command.rs` with the same
target check, hardening and `--`: a probe (`BatchMode`, `ConnectTimeout=8`, stdin closed, output read to
64 KiB and cleaned) and a first login (no `BatchMode`, so ssh can ask) that runs in a popup pane on this
computer. A person types or picks the host, so it is checked before anything runs: a target that starts with `-` or
holds whitespace or a control character is refused with the loader's message, and nothing is spawned or
written. A pane's shell reads the first-login line, so each word is a plain word (letters, digits and
`_./-~:=,+@`, with a Windows path's backslashes on Windows) or one single-quoted word; a word with a quote or a
control character, an unplain `remote_cmd` and a quoted program are refused rather than escaped, since each
shell escapes differently. The remote half of that line is never quoted for the remote shell. A target is a host name, so it
cannot become an option or a second command whichever shell reads the line. The dialog writes only to
`config.local.toml` (never the synced file), validates the entry with `validate_machines` first, and
changes nothing else; removing a machine does not touch the machine, only this computer's records of it.

Pasting an image into a pane of another machine (`client/remote_image.rs`, ADR 0007) is the same kind of
line: a fixed script (`maitty_core::remote_install::PASTE_SCRIPT`) run as `sh -c '<script>' sh <name> <bytes>`, the image
on stdin, over the hardened ssh line (`BatchMode`, no agent forwarding, the host after `--`). The file's name
is chosen by this computer from letters, digits, `.`, `_` and `-` and checked twice (before the line is
built and by the script); the byte count is checked by the script, which keeps nothing that arrives short.
The file is written under the app's folder in the far side's cache with `umask 077` and renamed into place,
and the script reads nothing of the far side's but its `HOME` and `XDG_CACHE_HOME` (to find the cache
folder, which it refuses unless the user running it owns it) and removes only its own stale files (pastes
older than a week, half-sent files older than a day). What comes back is untrusted: the path is taken only
from the line after the marker and only when it is absolute and free of control and invisible characters;
what ssh says when a send fails is cut to 200 characters and cleaned the same way. The image is
capped at 25 MB and the send at two minutes. A machine reached by a custom transport is not sent an image.

The desktop app's image paste (`image_paste.rs`, ticket 19) hands the host the bytes the webview read from the
clipboard, so the host trusts nothing about them: the type comes from the first bytes (PNG, JPEG, GIF, WebP;
anything else is refused), the size is capped at 25 MB before a byte is written, and the machine must be a known
ssh machine that is connected, all before anything is staged. The staged copy and the copy kept for a local pane
are created `create_new` and mode 600 in an owner-only folder of the app's cache, and nothing but this app's own
`paste-` and `staged-` files is ever swept. The far-side name is `paste-<ms>-<8 hex>-image.<ext>` with the
extension from the checked type, never from the webview. The image is the raw body of a command, not a logged
event, and is never written to a log. What the far side prints is checked as for any file.

A chat attachment (`attachment.rs`, ticket 20) is a path from a drop or the file picker, so it is a string
from outside: the host follows it only to a regular file (not a folder, device or pipe), refuses an empty file, caps an
image (by its first bytes, not its name) at 25 MB and any other file at 100 MB while copying (so a file that grows is
cut off and removed), and copies it `create_new`, mode 600, into an owner-only `attachments` folder of the app's
cache. The copy's name keeps only letters, digits, `.`, `_` and `-` (directory parts, spaces, control, direction and
invisible characters become `_`), so the path the agent is told never needs quoting and the name shown is the name
on disk. A zip is never unpacked. Nothing but `attach-` files of that folder is swept (after a week).
`attachments_exist` answers only for files directly inside the attachments or paste folder, so the window cannot use it
to probe the disk. The message stores the path and name, never the bytes; sending a file to the agent is the person's
own act and is not scanned (secrets stay out of chat, ADR 0006).

Sending any file to another machine (`client/send_file.rs`, ADR 0007) is the same line and the same script;
only the name, the cap and the trigger differ. A file is read from this computer only when the person
names it in the send prompt (a terminal cannot tell a drop from a paste, so a paste of text never
uploads a file, even when the text is a path); it must be a regular file, at most 100 MB, checked before ssh
starts and again by the script's byte count. The far-side name is `paste-<ms>-<8 hex>-<original>` with the
original reduced to letters, digits, `.`, `_` and `-` (anything else becomes `_`), leading dots dropped and
cut to 48 characters, so it cannot be a path, an option or shell syntax and two files never overwrite each
other; the line builder checks it again (at most 96 characters). Progress is the count of bytes written to
ssh; stopping ends the ssh process, and the script keeps nothing that arrives short. What comes back is
refused unless it is the checked path shape, exactly as for images. A sent file stays on the far machine
for a week, mode 600 in a folder only its owner can use; the guide tells people so. Directories are not sent.

`maitty machine update <name>` (`cli/machine_update.rs`) runs one fixed script on a machine and writes a
binary there, so it follows the same rules as every other ssh line, plus these. The script is a constant
in the app (`maitty_machines::INSTALL_SCRIPT`, re-exported by `remote_install`, passed to the remote shell as `sh -c '<script>' sh <args>`);
the destination and the byte count are positional arguments, each quoted once by `ssh_command`, and never
part of the script's text, so nothing a person or a machine supplies can become shell code (tests assert
the argv, and the script has no `{`/`}` and no process-signalling command). The binary travels on stdin,
not in a command. The script writes beside the destination with `umask 077`, checks the byte count, runs
`--version` (owner-only mode until it has run), makes it 755, and renames it over the old file; on any
failure it removes the temp file and leaves the destination as it was. It never signals a process: a
running daemon keeps its sessions. A release fetched for another platform is verified against
`sha256sums.txt` (exact full sha256) before it is unpacked, the archive listing is checked before
extraction (no absolute path, no `..`, no link, only the release folder; only the one binary is taken),
and the archive and the binary are capped at 200 MB. Nothing fetched is ever run on this computer. The
machine never needs network access to GitHub. The platform answer (`uname -sm`) and the version line are
read as data and cleaned before they are shown. Asking is never silent: a first install needs `--install`
or a `y`, a replacement `--yes` or a `y`, a downgrade `--force`. A Windows machine is refused as a remote
in v1.

Reconnecting never starts a daemon: after an established line ended, a configured machine's proxy
runs `proxy --attach-only` and exits when none is running, so a dead link cannot make this
computer launch processes on a server the user stopped. The first connection, and a retry a
person asks for, run plain `proxy`, which starts the daemon: it is the same command `--remote
host` already runs, and the person added that machine to use it. The CLI's `--on`, MCP and
`doctor` stay attach-only. What ssh
or the far side prints (its last stderr line) is untrusted: it is stripped of control characters,
so no escape sequence reaches the terminal, and cut to 200 characters before it is shown. The
failure is classified from ssh's exit status and the stream, not its wording, apart from three
fixed tokens that mark a login ssh could not do without a person (`link_failure::classify`). Typing
for a machine that is down is dropped, never queued and replayed.

A remote daemon is someone else's computer, so everything it sends is untrusted: it can forge
status, clipboard writes, toasts and prompts. Two rules hold for every message from a machine
that is not this one (`remote_policy.rs`, `remote_snapshot.rs` in `maitty-machines`):

- **Clean on ingest.** A `State`, and the panes of a `TermsChanged`, are made safe before they are kept: every string loses control
  characters (escape sequences included), hidden and direction-changing characters and line
  breaks (`hidden::sanitize`; only text blocks such as `said` and `asking` keep `\n`) and is
  cut (one-line fields 512 characters, paths 4096, text blocks 8192); every list and map is
  capped at 10,000 entries and a snapshot of more than 250,000 values in all, or with a number
  that is not finite, is refused whole and the last good one stays; pane sizes are held to
  1-1000 columns and rows, `cpu` to 0-100. The walk is over the serialized snapshot, so a string
  field added to the protocol is cleaned without being listed. A `TermsChanged` is cleaned with the
  same walk, refused whole when it names more than 10,000 panes to change or drop, and then
  merged into that machine's held snapshot, which goes through the same checks as a `State`
  (daemon id, caps) before it is kept. A path in a remote snapshot is a
  label: it is never used to touch this computer's disk (`App::local_cwd` is `None` for a pane on
  another machine, and `guard_local` refuses every feature that reads files or runs git there;
  see API_PATTERNS "client routing"). A toast it causes leads with
  `[machine]` and is one clean line of at most 300 characters. Nothing from a remote is logged
  above debug, and then only a fixed reason, never its text.
- **Decide every message.** `remote_policy::allow` names every `ServerMsg` variant (no
  wildcard; verdicts are apply, answer, or ignore), so a new one cannot compile until someone
  decides it, and its test table fails if the decision changes unnoticed. The default for a variant nobody can justify is ignore. This
  computer's own daemon is not subject to it.

| Message from another machine | Verdict | Why |
| --- | --- | --- |
| `Welcome`, `Bye` | apply | no effect beyond that machine's own connection |
| `State` | apply, cleaned | updates only that machine's snapshot; first-state work, restart handling and saved-change alerts are this computer's own and never run for it |
| `TermsChanged` | apply, cleaned | the panes that changed, cleaned like a `State`'s, merged into only that machine's snapshot and then judged as a whole `State` (daemon-id check, caps); an unclean or oversized delta is dropped and the last good snapshot stays |
| `Replay`, `Output` | apply | feed that machine's own pane screens; a screen's size is held to 1-1000 |
| `Attention` | apply | queues that machine's alert; the sound plays only with `sounds`, the toast shows only with `notices` |
| `Clipboard` | gated by `clipboard` (on by default; `clipboard = false` refuses) | writes this computer's clipboard; cut to 64 KiB; the toast says which machine, and a refused copy says so |
| `Notice` | gated by `notices` | a toast, cleaned and led by `[machine]`; it does not touch this computer's undo hint |
| `Raise` | gated by `notices` | brings the window to the front, the same "may get my attention" grant as a toast (there is no separate `raise` setting) |
| `Error`, `Reply` | answer, cleaned | applied only while this window has a command or query out to that machine that nothing has answered yet (client-side bookkeeping, see API_PATTERNS "client routing"; the wire is unchanged); otherwise ignored. An error's toast leads with `[machine]`; reply text is cleaned and cut to 256 KiB. A `Worktrees` reply never fills this computer's picker, and a check's output is taken only when the open sheet or the failure waiting for it is on that machine |
| `AskRemoveWorktree` | ignore | names a folder on this computer's disk to delete; remote worktree management is out of scope (ADR 0007) |
| `AutoWorkspace` | ignore | announces a workspace and arms this computer's undo |

The opt-ins are read from the machine's `[[machines]]` entry by name; a machine no entry names
has all of them off. `AskRemoveWorktree` from this computer's own daemon still shows its path
as the daemon sent it; the folder is removed by that daemon when you answer.

**The desktop app and other machines** ([ADR 0007](adr/0007-multiple-machines.md)). The web view
never connects to a machine and never starts `ssh`: the Rust host holds every connection, runs the same
`remote_policy` and `remote_snapshot` checks as the terminal app on everything that arrives (`hub.rs`,
before anything is kept or shown), and hands the web view cleaned data only. The Tauri capabilities did not
grow for machines (`capabilities/default.json` still holds window controls, the folder dialog, notifications,
the updater and restart; no shell, file system or network plugin). What the web view can ask of a machine
is a fixed set of host commands that take a machine's name or number, never a program or a path to run:

- **Names only.** The window knows a machine by name; where it points (`target`, `ssh_args`, `remote_cmd`,
  `via`) stays in the host.
- **Adding** a machine runs the loader's checks on the target and name first (an option-like or
  whitespace target is refused before anything runs), then looks at it with the shared probe: no prompt
  (`BatchMode`, stdin closed), ended by a 12 s limit or Cancel, and the entry is written only when that held
  up. The one time ssh may ask a person is the first login, which runs as a session on this computer, with
  the probe's hardening and a command line that refuses a word it cannot quote.
- **Grants.** `clipboard` is on for a machine until its entry in `config.local.toml` says `false`, and
  `notices` is off until it says `true`. The Machines page writes either through the shared editor before
  the host changes what it lets through. A notice is one clean line led by `[machine]` and a copy is cut to 64 KiB. `sounds` is read but
  never acted on: the desktop app plays none.
- **Files stay on their machine.** Features that read a pane's files or repository are refused for another
  machine's pane by the window and again by the host's commands, in the terminal app's words, so a pane
  number from another machine can never reach a pane of this computer.
- **No install or restart.** A machine that needs the app shows the command to run; the desktop app never
  installs, updates or restarts anything on it.
- **A different daemon at a known name** is refused (the shared daemon-id store) and says so; Remove then
  adding the machine again is the way out.

## Project-specific: auth and permissions

No user accounts. Access to the daemon is access to its local socket: a named
pipe on Windows, a Unix socket in the temp directory elsewhere, named per user
(`maitty-<user>-<label>.sock`). Socket access must stay limited to the user who
started the daemon. Status reports (`maitty hook`) are accepted only from a
process inside the pane (traced through its process tree) or carrying that
pane's `MAITTY_PANE_TOKEN`, a random secret set only in that pane's environment.
Remote work goes over the user's own `ssh`, so SSH's authentication applies.
The one-shot commands (`--on build ...`, `--pane build/4`) treat the machine they reach the same way.
What it answers is untrusted: a list (`ls`, `wait`) is cleaned like a window's snapshot
(`remote_snapshot::sanitize_snapshot`), an error or notice and a line a `wait --regex` matched lose
control characters and hidden characters (`hidden::sanitize`), and a failure to connect is shown as
one cleaned line with its fix (`link_failure::classify`). `read` prints another machine's screen
without escape sequences or other control characters (newlines stay); `read --raw` prints it exactly
as sent, which is the person's choice to make and can move the cursor or retitle the terminal. The
pane a command runs in belongs to this computer's daemon, so `$MAITTY_TERM_ID` is never the default
pane on another machine and the Hello to it carries no pane id or token. `kill-server --on` stops all
of that machine's sessions and needs `--yes`.

**MCP across machines.** An MCP tool call is made by an agent, a model in a pane here, and the
machine it names never learns which pane that is (the Hello to it carries no pane id), so that
machine's daemon would take the caller for you and enforce no grant. The rules that close this,
in `cli/mcp/machines.rs`: (1) a machine is reachable from MCP only if its `[[machines]]` entry says
`mcp = true`; any other call naming it (a `machine` argument or a `machine/id` id) returns
`machine_not_enabled` and is never dialled, and one naming no entry returns `unknown_machine`;
(2) before dialling, this computer's daemon decides whether the calling pane may do it, with the
table it uses for the same tool here: `read` to list, read and wait, `write` to send (and `read` too
with `wait`). The server asks by reading, and writing to, a pane that does not exist
(`Reach::caller_may`; the daemon test `the_grant_probes_the_mcp_server_uses_are_decided_by_the_panes_grants`
pins the answers) and takes anything but the expected answer, a daemon that does not answer included,
as a refusal. A refusal is `permission_denied` with the daemon's own message and the machine's name;
(3) a call to a machine never carries the pane's id or token; (4) whatever the machine returns is
untrusted text that can try to steer the model: a list is cleaned like a window's snapshot, a screen,
a reply and an error lose control and hidden characters (`hidden::sanitize`), a reply is cut at 32,768
characters, and every reply and error from a machine begins `[machine: name]` so the model can tell
where text came from; (5) answering a prompt, interrupting and starting agents or worktrees are
`unsupported_for_remote`, so a remote pane's prompt is never answered by an agent here and no
worktree or folder is chosen from a machine's paths. `[mcp] scope` limits the sessions of this
computer only; opening a machine opens all of its sessions to every pane here that holds the grant.

`maitty doctor` checks each configured machine with the same ssh lines as everything else
(`ssh_command.rs`: `BatchMode`, no agent forwarding, `--` before the host), plus one fixed script,
`command -v maitty` (`maitty_machines::WHERE_SCRIPT`), which carries no user data. Every step has a time
limit and the ssh it started is ended when the limit passes. What a machine answers (the version line, the
program's path, ssh's last line) is cleaned of control and hidden characters and cut short before it is
printed, and only the program's path is shown from the login shell, never its PATH, which can name users.
The `ssh_args` line shows your own arguments without control characters and only the file name of a key
given with `-i`.
The daemon id (`Snapshot.daemon_id`) detects that a different machine now answers at a saved
machine name, so its saved layout is not reused and the window says what happened. It is not
authentication and not a secret: anyone who can talk to a daemon can read its id, so it never
grants access. A daemon with no id answering at a name that has a recorded id is refused too.
Only an 8-character prefix of an id is shown or logged.
Machine-only settings and secrets belong in `config.local.toml`, which config sync
never uploads.

## Project-specific: tooling

Bundled Herdr detection rules and adapted integration code are pinned to a
reviewed source commit; provenance and Apache terms live in
`vendor/herdr-detection/NOTICE.md` and LICENSE. No remote rules are executed or
downloaded. Native JavaScript adapters invoke the installed Maitty executable with
an argument array and inherited pane authentication; they do not forward prompt
or answer text. Setup rejects non-regular owned-file targets and preserves
unowned extensions/hooks. OpenCode's JSONC editor uses pinned jsonc-parser 0.33.1
to retain native comments/settings, with invalid-config preflight tests.

The installers (`install.sh`, `install.ps1`) and `maitty update` run `maitty integrate`, which
edits the config of every agent installed on the machine: Claude Code's `settings.json` and
`~/.claude.json` (through `claude mcp add`), Codex's `config.toml`, Gemini CLI's and Qwen Code's
`settings.json`, Kimi's `config.toml`, Mastra Code's `hooks.json`, and adapter files for OpenCode,
Pi, OMP and Kilo. Only Maitty's own entries change; a changed config file is first copied to
`<file>.maitty-bak` (once). Agents that aren't installed get no folder or file. A failure
warns and leaves the install in place. `MAITTY_NO_INTEGRATE=1` skips the step.

`cargo audit` (install with `cargo install cargo-audit`) before releases; GitHub
push protection for secrets. Not yet in CI (there is no CI yet).

## Project-specific: usage store

The usage store holds token counts, cost, turn counts and working seconds, with the agent name,
the folder's name and the agent's session id. It holds no prompts, output, paths or secrets. The
file is private to the user (0600 in a 0700 folder on Unix). `Query::UsageReport` is for human
clients only: an agent pane or a structured runtime cannot read what other sessions cost.

A hook report never carries what a tool was asked to do. A tool-start report carries `call`, a
fingerprint (the first 8 bytes of SHA-256 over the tool's name and input, as 16 hex characters),
which cannot be turned back into the input and is never logged. The name and input stay in the
agent.

## Project-specific: followed transcripts

The daemon reads an agent's own transcript for a pane it follows ([ADR 0003](adr/0003-status-and-conversation-trust.md)).
The path comes from a hook report, so it is input, not trust:

- It is read only if its real path (links followed) is a regular `.jsonl` file inside the user's home folder
  and its name carries the session id the hook reported (Claude `<session>.jsonl`; Codex
  `rollout-…<session>.jsonl`). Anything else is refused with a reason that names no content.
- Only a pane that is hook-verified, owned by the desktop app and running `claude` or `codex` is followed; a
  pane cannot make the daemon read a file for another session.
- Reads are incremental and bounded (4 MiB per poll, a 16 MiB tail first, lines over 1 MiB dropped), run off the
  event loop, never write, and are cleaned and bounded like structured history (`tool_input.rs`). The text is
  held in daemon memory for the conversation's life, never cached, logged or put in a fixture.
- A followed conversation is read-only, and reading never changes a status.

## Project-specific: chat sessions started from the desktop

A chat session is a Claude or Codex run the daemon starts itself ([ADR 0003](adr/0003-status-and-conversation-trust.md)).
The agent runs with the user's permissions, so the window's start path is narrow:

- **Posture.** `start_chat_session` takes one optional boolean, `full_access`, and no other permission argument. By
  default the daemon starts the provider with workspace write and approval on request (Claude `--permission-mode
  default` with the stdio permission prompt; Codex `workspace-write`, `on-request`). With `full_access` it starts
  the provider's own skip-permissions mode (ADR 0005: Claude `bypassPermissions` with
  `--allow-dangerously-skip-permissions`; Codex `danger-full-access` with approval `never`). It is the user's
  explicit choice per chat (a labelled checkbox with a warning; the Settings default is off), is never selected
  silently, and is fixed for the session: Maitty never widens a running session, and a chat started with it shows
  a visible Full access tag. In the default posture every tool use the provider does not allow by itself becomes an approval
  that waits for a person's answer, and there is no auto-approve. There is no way to change a running session's
  posture, and no read-only-to-writable switch, in the window. Answers are human-only, bound to the current generation and the exact request id (unchanged
  from the structured runtime rules above).
- **Where it runs.** The folder is a project the app already shows; a branch start also needs a git
  repository and a valid branch name (`branch_agent::branch_problem`), and the worktree is made by the daemon
  (`create_trusted_worktree`: the trust copy and the repo's allowed setup hook only). The window never builds a
  path or runs git. A start for another machine is refused in the window.
- **Status** comes only from the daemon's events for the current generation; a superseded generation's command
  changes nothing.
- **User-input answers** (including a question marked secret) are relayed once, typed hidden, and never recorded
  or logged.
- **Closing** a chat session stops its agent before it is put away, so a session the user closed is never left
  running out of sight; if the stop fails nothing is hidden.
- Secret masking in history is the daemon's (ticket 18), at the point where history is stored and served; the
  window adds no copy of a message.

## Project-specific: structured history cache boundary

The structured-history cache reuses private-directory/file helpers:
0700 directories and 0600 files on Unix, the user's private data location on
Windows. Binding fingerprints and numeric Maitty paths keep native identifiers
out of filenames; pages remain sensitive conversation content, never diagnostics.
Only current daemon-owned provider events may project into attached history.
Unsupported raw provider objects are not retained. Cache disable serializes with
writes and deletes only Maitty's socket-scoped history root, never native history.
Without persistence, hydrated content belongs to an attachment and disappears on
its disconnect. No credentials, real transcripts or model requests enter fixtures.
Structured start/send/read/approval/cancel actions use the existing socket
authentication without a conversation-based self exemption. Start requires the
start grant, send and cancel require write, read requires read, and approval
answers require respond. Structured runtimes cannot answer provider approval
requests. A human approval is bound to its exact provider request ID,
conversation generation and daemon-owned connection; unknown request kinds stay
unanswered. Invisible characters are displayed as Unicode escapes. Only the
command and reason needed for the decision leave the daemon;
raw provider params are not retained in the conversation history or metadata.
A tool call's `input` is kept in history (it can hold file contents, commands and
environment), so it is held to the same boundary as the rest of the history: it
leaves only in an explicit authorized read and in the private cache. It is cleaned
of escape and control characters, invisible characters are spelled out as Unicode
escapes, every string is cut and the whole value is capped at 64 KiB, and an input
that cannot be bounded in shape is dropped (`tool_input.rs`). Secrets are
masked on the way in; see *Conversation history and secrets* below. Answered approvals
keep the request's command and diff in daemon memory only, never in the cache, and
the answers to user-input requests are not kept at all.
A message queued while a turn runs is held in daemon memory only, never in the session file or the history
cache, and goes through the same admission checks as a direct send (invisible characters, size, write grant)
when it is queued and the same durable-receipt path when it is delivered. Removing one needs a human client.
Prompts are checked for invisible characters at daemon admission. Launch inputs
are validated in background and explicit sandbox/approval posture is checked in
the provider response. Native profile-v2 and ambiguous identity are refused.
Command execution, file-change and user-input requests may be answered only by a
human through the exact live request ID. File-change approval stays disabled
until the matching diff is visible. User-input answers are validated against
the displayed questions and choices, reject hidden Unicode, and are not stored
in history or metadata. Permission requests stay blocked and unanswered until
their controls exist; no unsupported request can approve itself. Turn cancellation is human-only and carries the acknowledged active
turn ID through the owning provider connection; uncertain sends are never
guessed. Generation and connection identity fence every event; checkpoint
approvals retain separate typed owners.

Persist metadata intent before provider creation/submission and durable native
turn acknowledgement before returning success. A crash or transport ambiguity
never retries a possibly delivered prompt. Prompt text and effective environment
never enter the metadata ledger or diagnostics. Structured history appears only
in explicit authorized read replies and the optional private cache.

Outgoing structured tools authenticate with a daemon-minted runtime secret bound
to conversation, generation and connection. Strip inherited pane/structured
credentials before launch. Do not persist this secret, copy a pane token, accept
a partial environment claim, or treat rejected/revoked claims as human callers.
Every request uses configured MCP grants, with no logical-conversation self
exemption; human-only grant/check/helper answers remain human-only. Structured
descendant starts are refused pending conversation-owned helper consent.

Codex filters MCP child environments, so validated runtime overrides append the
four reserved identity variable names to existing stdio MCP allowlists. They
contain no secret values. Preserve the effective default/all shell policy and
refuse unsupported custom filtering, environment-set or profile policy and
reserved MCP environment conflicts before a thread starts. No credential
exposure switch is broadened. Explicit MCP references revalidate current read
authority before filesystem work. An in-memory native acknowledgement remains
uncertain in every read response until its persistence barrier succeeds.

### Conversation history and secrets

Everything the daemon stores and serves as conversation history passes one function,
`redact_entry` (`daemon/conversation/redact.rs`), when it enters the store (`History::apply`,
`hydrate`) and again when a cache is loaded. That covers the Claude and Codex adapters, both
followed-transcript parsers, message text, tool input, command output and approvals. It changes only
Maitty's copy: the agent still sees exactly what it sent and received, and nothing is blocked.

**Masked** (the secret part becomes `[redacted]`, the text around it stays):

- GitHub tokens (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`, `github_pat_`), AWS access key ids (`AKIA`,
  `ASIA`), Anthropic (`sk-ant-`) and OpenAI (`sk-`, `sk-proj-`) keys, Slack (`xox*`, `xapp-`), Google
  (`AIza`), Stripe live (`sk_live_`, `rk_live_`), JSON Web Tokens.
- PEM private key blocks, from the `BEGIN ... PRIVATE KEY` line to the `END` line (or to the end of the
  text if it is cut or still streaming); the whole block becomes `[redacted private key]`.
- `Authorization: Bearer|Basic|Token ...` headers and bare `Bearer <token>`.
- The password in `scheme://user:password@host`.
- `NAME=value`, `name: value` and `"name": "value"` where the name has a secret, token, password,
  passwd, api key, private key, credential or auth part (`githubToken`, `AWS_SECRET_ACCESS_KEY`), and
  every value under such a key in a tool's input JSON. Placeholders (`$VAR`, `<your-key>`, `****`),
  URLs, paths, flags, short values and code references (`token = refresh_token`) are left alone.

**Omitted** (the text becomes `[content of a secret file omitted]`; the path, tool, kind and an
`omitted_bytes` count stay, so the chat still says "wrote .env"): the content, old/new text and diffs of
a tool input, an approval's `changes` and a Codex file change whose path is `.env` or `.env.*` (not
`.env.example`, `.sample`, `.template`), `*.pem`, `*.key`, `*.p12`, `*.pfx`, `*.kdbx`, `id_rsa`,
`id_ed25519` (and `id_ecdsa`, `id_dsa`), `.npmrc`, `.pypirc`, `.netrc`, `credentials`,
`.git-credentials`, `secrets` / `secrets.*`, or anything under `.aws`, `.ssh` or `.gnupg`. The output of
a shell command that prints such a file (`cat`, `head`, `tail`, `grep`, `sed`, ...) is omitted too.

**Limits.** Pattern masking is best-effort and is not a guarantee. It misses a secret with no known
shape and no telling name (a bare 40-character key, a password in prose, a custom token format), a
secret split by the provider across separate tool results, a file that holds secrets under an ordinary
name (`config.json`), and a command that reads a secret file in a way the command list does not know.
It can also hide something that was not a secret. The patterns lean towards missing a secret over
wrecking ordinary text: prose, git hashes, UUIDs and plain base64 are not touched. Treat history as
sensitive anyway: it still reaches any client with a read grant and the private cache. A person who
approves a command reads the masked text, so a command's masked part cannot be checked from the chat.
The functions never log or return what a pattern matched.

## Project-specific: terminal output socket

The desktop host listens on `127.0.0.1` (a port the system picks) and sends terminal output to its own
window as WebSocket messages ([ADR 0004](adr/0004-client-split-and-desktop-host.md)). Output
is what the panes show, so the rules are those of a credential:

- **Loopback only.** The listener binds the loopback address and nothing else.
- **A one-time token.** The window asks the host for a 256-bit random token (`open_terminal_socket`),
  valid for 60 seconds and one use, sends it as the first message, and gets nothing until the host
  says `ready`. At most 8 tokens are outstanding. The comparison does not stop at the first
  difference. The token is never in the address and never logged.
- **Origin allow-list.** The upgrade is refused unless the `Origin` header is the app's own
  (`tauri://localhost`, `http://tauri.localhost`, `https://tauri.localhost`), plus the dev server's
  origin in a debug build. A web page in the user's browser cannot connect.
- **Bounded.** At most 4 connections may be waiting to prove a token, each has 5 seconds, and at most
  4,096 frames wait for a window that has stopped reading, after which the socket is dropped and the
  channel takes over.
- **One window.** A new connection replaces the old one.
- **CSP.** `connect-src` allows `ws://127.0.0.1:*` and no other socket.
