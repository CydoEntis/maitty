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

Assets: the user's shells and agent sessions (which can run any command as the
user), their source code, and API keys in `config.local.toml`. Threats: another
local user or process driving the daemon's socket; a process inside one pane
forging status for another pane; command injection through names, branches or
paths that end up in shell commands; secrets leaking through config sync or logs;
an SSH remote being someone else's machine; what agents work on leaking through phone alerts
(ntfy, off by default: they name the agent and folder only, unless `phone_text` is on, and
the topic name is the only secret on the public server).

A project's check is a command the daemon runs as the user when an agent's turn ends. It
comes from the user's own `config.toml` (`[[check]]`) or, for a project with none there, from
the repo's `.seshi/checks.toml` under the approval rules below. It runs through the user's shell in the
session's worktree with no stdin and without `SESHI_PANE_TOKEN`, so it cannot report status
as the pane. On timeout its whole process tree is killed (process group on unix, job object
on Windows). If the daemon dies, a check dies with it on Windows (the job closes) and on
Linux (a parent-death signal kills the shell; a descendant the shell already started is
left to finish or to fail on its closed pipes). On macOS there is no such call, so a check
running when the daemon crashes lasts until it ends or writes to its closed pipes. Its output is a capped tail held by the daemon, saved beside the pane's output
files and served only by `Query::CheckOutput` under the read grant; it is never logged and
never in the state snapshot. It may contain secrets the command printed, like any pane's
output.

A repo-supplied command never runs until you approve it, so cloning a repo, or an agent
editing `.seshi/checks.toml`, cannot make Seshi run anything. The rules:

- Your own `[[check]]` for a project wins whole; the repo's file is then not read.
- Approval is per exact command: Always is kept with the command's sha256, and a changed
  command asks again. Never holds whatever the command becomes. Once lets one run of that
  pane's checkpoint go and is never saved.
- Answers live in `check-approvals.json` in Seshi's data folder, outside every repo, written
  atomically (synced, then renamed). The daemon reads the file once at start and goes by its
  own copy, so writing the file changes nothing while it runs.
- Only you approve: `ApproveCheck` and `ForgetCheckApproval` are refused from every pane
  connection whatever its grants, and approve only a checkpoint waiting for approval with
  the command it waits with. No key press approves on its own; the client sends the choice.
- A file that cannot be read or parsed runs nothing and reads as could-not-run with the reason.

Residual risk: a process running as the same OS user can edit `check-approvals.json`, which
counts after the next daemon restart, and a process that hides its pane identity (no
`SESHI_TERM_ID`) is treated as you, as for grants. Both need code already running as you.

Overriding a failed check is a recorded, visible action: the time and the reason stay on the
result and its `Overridden` state shows wherever the check does. It needs the write grant on
the target pane, and a pane can never override or undo its own check, so an agent's claim
cannot turn its failure into done. An override covers one result; the next run replaces it.

## Project-specific: auth and permissions

No user accounts. Access to the daemon is access to its local socket: a named
pipe on Windows, a Unix socket in the temp directory elsewhere, named per user
(`seshi-<user>-<label>.sock`). Socket access must stay limited to the user who
started the daemon. Status reports (`seshi hook`) are accepted only from a
process inside the pane (traced through its process tree) or carrying that
pane's `SESHI_PANE_TOKEN`, a random secret set only in that pane's environment.
Remote work goes over the user's own `ssh`, so SSH's authentication applies.
Machine-only settings and secrets belong in `config.local.toml`, which config sync
never uploads.

## Project-specific: tooling

Bundled Herdr detection rules and adapted integration code are pinned to a
reviewed source commit; provenance and Apache terms live in
`vendor/herdr-detection/NOTICE.md` and LICENSE. No remote rules are executed or
downloaded. Native JavaScript adapters invoke the installed Seshi executable with
an argument array and inherited pane authentication; they do not forward prompt
or answer text. Setup rejects non-regular owned-file targets and preserves
unowned extensions/hooks. OpenCode's JSONC editor uses pinned jsonc-parser 0.33.1
to retain native comments/settings, with invalid-config preflight tests.

`cargo audit` (install with `cargo install cargo-audit`) before releases; GitHub
push protection for secrets. Not yet in CI (there is no CI yet).
