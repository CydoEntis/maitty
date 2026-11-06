# Database

> **Not applicable:** Maitty has no database. Its state is small files: `config.toml`
> (settings), a JSON session snapshot written by the daemon (`daemon/persist.rs`), and
> the client's saved projects list. Add a pack if a database ever arrives.

This doc covers how data is stored, queried, migrated, and protected.
`AGENTS.md` is the operating contract; this doc holds the persistence practices
in depth.

## Principles

- The database is the durable source of truth. Caches and UI stores only
  reflect it.
- Only repositories and adapters touch the database. UI, handlers, and domain
  logic contain no queries.
- Repositories return domain types. Row shapes, ORM entities, and driver types
  stay inside the data layer.
- Validate or map records when they cross from the database into application
  code. Never assume stored data matches the current schema.
- Never silently drop or rewrite user data.

## Queries

- Parameterize every dynamic value. Never build SQL from input by string
  concatenation, including for identifiers such as sort columns. Use an
  allowlist for those.
- Wrap multi-step changes that must succeed or fail together in a single
  transaction, owned by the service or repository method that knows the
  invariant.
- Select only the columns you need. Avoid N+1 queries by batching or joining
  deliberately.
- Paginate or bound every list query. No query returns an unbounded result set.
- Map database errors, such as unique violations and foreign key failures, into
  typed domain errors at the repository.

## Schema design

- Every table has a primary key. Choose one ID strategy and use it everywhere.
- Every table has `created_at` and `updated_at`, stored in UTC.
- Enforce integrity in the database as well as in code: `NOT NULL`, foreign
  keys, unique constraints, and check constraints where they express a real
  rule.
- Store money as integer minor units or a fixed-point decimal, never as float.
- Soft-delete user-owned content by default, with a `deleted_at` column.
  Permanent deletion needs explicit product behaviour and a recovery plan.
- Justify each index by a real query. Index foreign keys used in joins.
- Don't create tables for features that are not being built.

## Migrations

- Migrations are committed, ordered, and deterministic. Once a migration has
  been applied anywhere shared, it is append-only.
- Never edit an applied migration. Write a new one.
- Every migration runs cleanly on a blank database and on the previous released
  schema.
- Destructive changes need a written plan and explicit approval before they are
  written. That covers dropping a column or table, narrowing a type, and
  rewriting data. Prefer expand-then-contract: add the new column, migrate
  reads and writes, backfill, then remove the old column in a later release.
- Long-running or locking migrations on large tables must say how they avoid
  downtime.

## Data safety

- Never run tests against a developer's real database or any shared
  environment.
- Never commit database files, dumps, or connection strings containing
  credentials.
- Stale writes must not overwrite newer data. Use a revision or version column,
  or an equivalent check, wherever concurrent edits are possible.
- Keep backups and a tested restore path in place before any destructive
  migration or sync feature ships.

## Testing

- Test repositories and migrations against a real, isolated instance of the same
  engine: a temp file, a container, or a per-test schema.
- Cover clean install, upgrade from the previous schema, constraint violations,
  transaction rollback, concurrency conflicts, and soft delete and restore where
  they exist.

## Project-specific: engine and access

`daemon/persist.rs` owns the session JSON snapshot. Conversation metadata and
pane associations share one private, synced temporary file renamed over the
snapshot; there is no separate registry file or database.

The client's own saved state is `maitty-ui.json` (`client/screen/mod.rs` `Saved`). Project
keys (`order`, `closed`, `names`, `open_branches`) and the top-level `session_order` and
`placed` describe this machine, so a file written before machines reads as it always did.
`machines` holds the same `session_order` and `placed` for each other machine by its
configured name, and a project on another machine is keyed `<machine>\u{1f}<folder>`. A machine
that is not connected, or no longer configured, keeps its entries; they are never pruned.
`machines` is left out of the file while it is empty.

## Project-specific: conventions

The top-level `conversations` list defaults empty for legacy snapshots. Optional
pane `conversation_id` links never establish identity from legacy hook metadata.
The registry validates nonzero IDs/generations, complete absolute bindings and
unique conversation/native-session identities before saving or restoring.


## Project-specific: migrations workflow


## Project-specific: entities

A conversation stores provider, exact native session ID, optional account label
and profile, config root, executable, cwd, optional worktree, execution kind,
generation and recovery state. It stores no conversation content, credentials,
provider connection or approval authority, including when restore.output=false.
Running/transferring records restore with a checked generation increment and
recovery-required state; linked panes never use legacy most-recent resume.
Recovered metadata remains due for saving until an atomic write succeeds.
A retained registry prevents empty-daemon auto-exit from deleting its snapshot.
Profile persistence does not establish verified native/app-server launch parity;
see [the Codex contract](stack/codex-app-server.md).


Two small files in the data folder name machines. `daemon-id` holds the daemon's own random
128-bit id as hex, made on first start and replaced (with a warning) if unreadable, so a bad
file never stops the daemon. `machine-ids.json` is the window's `{"ids": {name: daemon id}}`,
per user and never synced, written atomically; unknown fields are ignored and a file that does
not parse is kept as `.bak` and starts empty. Neither has a version: a new field is added with a
default.

## Project-specific: usage store

`daemon/usage_store.rs` keeps provider-reported usage per session per day in
`usage-<socket>.json` in the data folder ([ADR 0009](adr/0009-usage-is-provider-reported-never-estimated.md)).
It is a small versioned JSON file (`version`, `rows`, `cursors`), not a database. A row is
(UTC day, session) with agent, folder name, input and output tokens, cost, turns and seconds
working; a cursor is the last running total seen for a session, so only the growth is added and a
restart does not count a session twice. Only what Claude Code and Codex report is stored; a figure
nobody reported is absent, never estimated, and a subscription session stores no cost.

Rows and cursors older than 90 days are pruned on load and on every save. The file is written
private (0600 in a 0700 folder on Unix), synced, then renamed over the old one, off the loop at
most every 30 seconds while something changed and once more when the daemon exits. A file that
does not parse, or whose `version` is newer than the build reads, is renamed to `<file>.bak` and
the store starts empty with a logged warning; it never stops the daemon. Unknown fields are
ignored and missing ones default, so a field can be added without a version bump.

## Project-specific: structured history cache foundation

`daemon/conversation/history_storage.rs` is a private structured display cache,
not a database or a replacement for provider-native context. `Cache::for_session`
derives `<data>/session-<socket>.history/` from the existing session snapshot path.
Conversation directories and immutable page filenames use numeric Maitty IDs;
native IDs never become paths. Pages hold at most 128 entries / 256 KiB.

A synced, atomically replaced `manifest.json` commits the revision, a SHA-256
binding fingerprint, completeness and the exact page list with lengths/digests.
Readers consult only that manifest; missing/corrupt pages fail explicitly and
require native hydration. Temporary/orphan files never prove completeness.
Streaming drafts are excluded from durable pages. Metadata snapshots remain
content-free.

One cache owner per daemon/socket serializes writes and policy changes. Prepared
writes carry a policy epoch; disabled or superseded writes cannot recreate
content. Older queued revisions cannot replace newer committed revisions.
`restore.output = false` or `restore.enabled = false` deletes/prevents this cache;
cleanup failure is explicit. Startup and reload apply these policies in production.

`pending_starts` defaults empty and records identity creation intent before the
provider can create a thread. An interrupted intent remains unresolved and never
automatically starts another thread. Complete bindings include verified structured
launch settings; the generic Codex `session_id` means exact `Thread.id`, never
the shared tree's `Thread.sessionId`.

`conversation_operations` defaults empty. It stores one monotonic operation
watermark, prompt digest and uncertain/acknowledged turn receipt per conversation,
including when output retention is disabled. It stores no prompt body or general
request ledger. Metadata restoration must be enabled for structured mutations.
All full snapshots use a capture-time ticket and one resolved-path writer lock;
an older completion cannot overwrite a newer committed intent or receipt.
