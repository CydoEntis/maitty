# Testing

This doc covers what to test, at which level, and how to tell whether a test
proves anything. `AGENTS.md` is the operating contract; this doc holds the
testing practices in depth.

## Test levels

Use the narrowest test that proves the behaviour, then run the full suite.

| Level | Covers | Uses |
| --- | --- | --- |
| Unit | domain rules, pure functions, mappers, validators | plain inputs and outputs, no I/O |
| Service | use cases | fakes for ports (repositories, clock, mailer) |
| Integration | repositories, adapters, migrations, HTTP handlers | a real, isolated instance of the dependency |
| End-to-end | critical user journeys | the built app, driven as a user would |

Most tests should be unit and service tests. Keep end-to-end tests for the few
journeys that must never break.

## What to assert

- **Assert the outcome the user is left with**, not just that something
  changed. A test that checks "a value was written" passes while the value is
  wrong. Assert the exact output, the exact stored record, the exact response
  body, the exact error.
- Assert what was excluded as well as what was included, for filters,
  permissions, and search.
- Test behaviour through public contracts. Don't assert on private functions,
  internal call order, or implementation details that a refactor would change.
- Cover the edges: empty input, a single item, the boundary values, invalid
  input, duplicates, and concurrent or stale updates where they apply.
- A test that passes against broken code proves nothing. When you write a test
  for new behaviour, check that it fails without the change.

## Determinism

- Don't use sleeps, the real clock, randomness without a seed, live network
  calls, or shared mutable state.
- Inject the clock and the id generator. Use a fresh, isolated database or temp
  directory for each test or suite, and clean it up even when the test fails.
- Tests must pass in any order and in parallel.

## Fakes and mocks

- Prefer small in-memory fakes that implement a port over mocks that script
  call sequences.
- Mock only at the boundaries the project owns: its ports. Don't mock the code
  under test or third-party internals.
- Never let a fake drift from the real adapter's contract. Cover the real
  adapter with integration tests.

## Bugs

1. Reproduce the bug with a failing test first, whenever practical.
2. Fix it.
3. Keep the test as a regression guard, and name it after the behaviour, not the
   ticket.

## Rules

- Never delete, skip, or weaken a legitimate test to make a change pass. Change
  a test only when the intended behaviour changed, and state why.
- Keep test names as sentences describing behaviour, for example
  `rejects an expired token` rather than `test3`.
- Structure each test as arrange, act, assert. Put one behaviour in each test.
  Several assertions about that one behaviour are fine.
- Keep fixtures small and local. A shared fixture that every test tweaks is a
  hidden coupling.
- Never put real personal data or secrets in fixtures or snapshots.

## Project-specific: tools and commands

The optional desktop frontend uses the commands and tools recorded in its
stack docs. Keep those checks separate from the Rust verification gate so the
terminal-only build does not require Node or a webview.

The built-in test harness: `cargo test` runs everything (unit tests sit in
`#[cfg(test)] mod` blocks beside the code). UI tests render frames with
ratatui's `TestBackend` and assert on the text and cell styles;
`MAITTY_SHOW=1 cargo test <name> -- --nocapture` prints the frames. One test:
`cargo test <name>`.

## Project-specific: layout and fixtures

Tests live next to the code they test. Client UI tests build a fake snapshot
with `design_tests::render_with(w, h)` (projects, worktrees, agents in
each state) and drive it with `app.on_key` / `app.act`. Layout tests use
`client/reference_fixture.rs`: `world(stage)` is the reference sample world (affiliate-site,
pixls, maitty) with fix-footer's check at one of its stages, on a pinned clock
(`screen::FIXED_NOW`), and `frame(app, layout, w, h, glyphs)` draws it as text in Cards
floating, Cards tiled or Flat, to compare with the reference frames.

`client/reference_frames_tests.rs` holds one test per reference preset (01–17, A1–A5, B1–B2, C1–C2). Each
draws the whole screen in every layout at 160×45, 100×30 and 80×24 (and ASCII at Flat 80×24)
and compares it cell for cell with the reference in `client/reference_frames/<preset>.txt`. Its
header lists the masks (the pane's terminal text, the working spinner and ages the fixture
can't pin, the sample's checkpoint kinds) and every deliberate difference from the sample. A
failure prints each differing row as got/want. The reference frames
(`client/reference_frames/*.txt`, `client/screen/reference.txt`, `client/screen/check_modals_frames.txt`
and `client/screen/side_frames.txt`) are fixed fixtures. Update them by hand, in the same
change, only when the design itself changes; never to make a failing test pass.

The website's screenshots are real frames: `client/site_frames.rs` draws made-up app states
(hero, grid, changes, files, inbox, the four looks, shortcut mode, the splash) in `night` and
`day` through the real draw code, on a pinned clock with motion off, and writes each as an HTML
`<pre>` (inline colours, no Nerd Font needed) plus `frames.json`. Its regular test checks every
frame draws; the export runs only on demand, from the repo root on `origin/dev` or later.
Glyphs the site's font may lack come wrapped in `.g` / `.g2` spans; the page needs the CSS in
`client/site_frames/html.rs` to hold them to their cells.

```sh
MAITTY_SITE_FRAMES=../maitty-site/site/frames cargo test export_site_frames -- --ignored
```

```powershell
$env:MAITTY_SITE_FRAMES='../maitty-site/site/frames'; cargo test export_site_frames -- --ignored
```

Live checks use a nested
server: `MAITTY_SOCKET=test maitty`, then `maitty send` / `maitty read` against it.
Tests never touch the user's real config (`MAITTY_CONFIG` or in-memory configs).

## Project-specific: testing over ssh (fake-ssh)

Anything that reaches another machine is tested through a stand-in for `ssh`, never a real
server. `apps/tui/examples/fake-ssh.rs` is a small program (no `sh`, except in `remote-sh` mode, so it runs the same on
Windows, Linux and macOS): point `MAITTY_SSH` at it and it ignores the host and every option,
then runs the real `maitty proxy` against the socket the test chose. `cargo test` builds it
with the tests (`apps/tui/tests/fake_ssh.rs` builds it itself when run with `--test`). The code that
builds and locates the shim (`fake_ssh()`) and runs a command with a time limit (`output_within()`)
is in `crates/maitty-test-support`, a dev-dependency any crate's tests can use, with `Sandbox`: a
private daemon's environment, the shim's per-machine arguments (`--mode`, `--maitty`, `--env`,
`--calls`, which let one process hold several machines), and killing the last connection. The
desktop host's `hub` tests use it.
The host also has an end-to-end test without a window (`hub/tests/end_to_end.rs`): a real daemon for
this computer and two behind the shim, the hub and the same `react` the window's glue runs; it checks
snapshots per machine, typing, a pane's replay, and a pane replayed again after one machine's link is
killed while the others are never touched.

| `MAITTY_FAKE_SSH_MODE` | The shim |
| --- | --- |
| `proxy` (default) | runs `$MAITTY_FAKE_SSH_MAITTY proxy`; the connection works |
| `noise` | prints login text on stdout, then behaves as `proxy` (which writes the stream marker, so the client skips the text) |
| `noise-only` | prints login text and exits, with no marker |
| `noise-flood` | prints more login text than the client skips (over 64 KiB), with no marker |
| `old` | behaves as a `proxy` from before the stream marker: the real proxy, minus the marker |
| `auth-fail` | prints `Permission denied (publickey).` on stderr, exits 255 |
| `hang` | says nothing and never exits |
| `mismatch` | answers the handshake with a protocol version one ahead |
| `exit` | exits 0 at once, before any frame |
| `unreachable` | prints `ssh: connect to host box port 22: Connection refused` on stderr, exits 255 |
| `not-installed` | prints `bash: maitty: command not found` on stderr, exits 127 |
| `remote-sh` | runs the remote command through the host's `sh -c` in a fake remote home (`--home=`); see below |
| `require-pipes` | behaves as `proxy`, but exits 3 if its stdin or stdout is a terminal (a custom transport must get plain pipes) |
| `capture-argv` | appends each argument it was called with, one per line then `--`, to `--argv-log=<file>`, then behaves as `proxy` |
| `capture-hello` | writes the client's first frame to the file `MAITTY_FAKE_SSH_HELLO` names, answers a Welcome, exits |

In `proxy`, `noise` and `old` modes a call whose remote command ends in ` --version` (the Machines
dialog's probe and first login) answers `maitty 0.0.0-fake` and exits 0; the other modes fail or hang
for it as they do for a proxy.

The shim is also a machine's custom transport: put its path first in `via`, with `--mode=` and the rest in
the later entries, and the remote command arrives as its last argument. The tests that run it
(`custom_transport` in `apps/tui/tests/fake_ssh.rs`: end to end through a local pipe, noise, no marker,
`Permission denied` with 255, exit 127, a hang dropped by a timeout with its process gone, plain pipes, the
argv the transport is given) are `cfg(unix)`: a transport is unsupported on Windows, so there they only
compile. Validation, argv construction, classification and the Windows refusal are pure tests that run on
every host. The daemon's login-PATH probe (`login_path.rs`) is split the same way: its parsing and merging
run everywhere, and the tests that start a shell are `cfg(unix)`.

`MAITTY_FAKE_SSH_LOG` names a file that receives the arguments the shim was called with, one
per line, to assert how the client builds its ssh command.

Several machines in one process put each machine's settings in the shim's arguments, which win
over the environment: `--mode=`, `--maitty=`, `--env=KEY=value` (repeatable), and `--calls=<file>`,
which appends a line `<shim pid> <proxy pid>` per call (the proxy's is 0 when none ran). A test
counts dials from it, and kills both pids to cut a live line the way a dead link does. Like ssh,
the shim passes `--attach-only` on to `maitty proxy` when the remote command ends with it.
`--argv-log=<file>` also records the arguments of each `proxy` call, so a test can tell a dial that
may start the daemon from an attach-only one. On Windows a daemon that a proxy starts keeps the shim's
pipe open, so a test that cuts the line starts the daemon apart first.

`remote-sh` is for the install script: the shim runs the remote command (the last argument) through the
host's POSIX `sh -c` with `HOME` set to the folder `--home=<dir>` names (its `bin` first on `PATH`, where a
test puts a fake `uname` and fake installed apps) and stdin passed through, so the real script runs against
a temporary filesystem. `--argv-log=<file>` appends each remote command (newlines as `\n`), so a test can
assert what was or was not sent (the install script is the only thing that runs `mktemp`).
`client/machine_update_tests.rs` drives `machine update` with it in-process: missing, too old, matching and
newer, the prompt, a short upload, a file that will not run, a destination that cannot be made, a
destination with spaces and a quote, a Windows-like and an unsupported `uname`, and a bad checksum on a
fetched release (a fake `Releases`, so no test touches the network). They need a POSIX `sh` with `wc mv
chmod mktemp cat dirname tr cut` on `PATH` (any Linux or macOS host; Git for Windows' `sh` on Windows) and
print `skipped:` and pass when there is none. The rest of the machine-update logic (the `uname` parser,
version compare, the decision table, the exit-code table, archive listing checks, checksums, asset names)
is pure and runs everywhere. The one unix-only assertion (the `0755` mode) is `#[cfg(unix)]`.
`client/remote_image_tests.rs` sends a pasted image the same way (ADR 0049): the real paste script
runs in the fake home, and the tests check that the file arrives whole and private, that two pastes are two
files, that an image over 25 MB never starts ssh, that a machine reached by a custom transport is told it
needs ssh, and that a failed login says so and keeps nothing. The script itself is also run directly
(`maitty-machines`' `remote_paste` tests: a short upload is not kept, a bad name writes nothing, old pastes are
cleared, only pastes are cleared). They need `sh` with `wc mv mktemp cat tr printf rm mkdir find` and print
`skipped:` and pass when there is none.

`client/cli_machine_tests.rs` runs the one-shot commands in-process against a real daemon behind the shim, the
way `--on build` reaches it: a `[[machines]]` entry whose `ssh_args` carry `--mode=`, `--maitty=`, `--env=`
and `--calls=` is resolved with `resolve_machine`, and the shim program is passed in. It covers new, split,
focus, popup, close, send, `wait --regex`, read, the `ls` text and JSON tags, `ls --all` with a machine that
answers and one that is down or locked, the one-line fixes (`needs a login`, `has no app`, `is not running`),
`kill-server`, and the rule that a named machine needs an explicit pane (the shim's call count proves nothing
was dialled). A configured machine cannot be named through the binary on Windows, because the config
directory there is not taken from the environment and `MAITTY_CONFIG` ignores `config.local.toml`; the
binary-level tests in `fake_ssh.rs` cover what needs no machine (an unknown name is exit 2 with the
`[[machines]]` hint, `--on` with `--remote`, `--on` on a command that ignores it) and that `--remote ls`
is unchanged. The pane-token rule is `a_local_pane_token_never_reaches_another_machine_s_daemon`. Tests never
dial this computer's own daemon and never set `MAITTY_TERM_ID` on the shared test process.

`client/doctor_machine_tests.rs` runs `doctor`'s machine checks (`cli/doctor_machines.rs`) against the shim: a good
machine gets five passing lines (the shim answers `--version` as `0.0.0-fake` and the login-PATH script as
`/fake/bin/maitty`; the test passes `0.0.0` as this computer's version), `mismatch` fails the protocol line with
the update hint, `auth-fail` gives one failed line, the skip and the options, and `hang` finishes within a small
budget passed to `check_all` with the shim process gone afterwards. Three hanging machines checked in parallel
must finish in well under the sum of their budgets. The line texts and the skip rule are pure tests in the module.

`client/mcp_machine_tests.rs` runs the MCP tools (`Server::call`) against the same kind of daemon: list, read,
send and wait on a machine's panes with `machine/id` ids, ids round-tripping, a machine that is not opened
(`machine_not_enabled`, call count unchanged), an unknown machine, start/answer/interrupt
(`unsupported_for_remote`), and `machine = "all"` with one machine down. This computer's daemon is not dialled:
`Reach::new` takes the stand-in that answers the calling pane's grant probes, so a read-only caller is tested
without touching a real daemon (the call count proves nothing was dialled), and the probes' answers are pinned
against the real grant check in `daemon/logic_tests.rs`. The routing table and the cleaning are pure tests in
`cli/mcp/machines.rs`.

The `supervised` tests in `fake_ssh.rs` drive `link::Links` and a `Connections` registry the
way the event loop does, against real daemons: a killed line shows reconnecting then connected
and the pane replays with the same pane id; one machine dying leaves another answering; the
`auth-fail`, `mismatch` and `hang` shims give needs-login, needs-update and an unreachable
machine that is dialled again with its hung ssh killed; no daemon means unreachable and no daemon
started; a refused machine stays parked until a retry. They use tiny backoff and no sleeps for
correctness; a timeout only turns a hang into a failure. The watchdog and the state machine are
unit tests (`connections.rs`, `link/state.rs` in `maitty-machines`) with durations of tens of
milliseconds and no clock.

`client/remote_status_tests.rs` is the same pattern from inside the client: a real daemon behind the
shim, a pane that reports through `maitty hook`, and the window's model of that machine checked
against the daemon's status for the first report and the next. It builds the `maitty` binary and
the shim itself, as `fake_ssh.rs` does.

`client/machine_flow/tests.rs` drives the Machines dialog with its keys against the shim and real
daemons: adding from the picker (one `[[machines]]` entry in a sandbox's `config.local.toml`, the synced
file byte for byte as it was, a duplicate name numbered, a hostile target refused before anything runs),
the probe (a bad host shows ssh's error, a hung ssh is ended by the limit and by `Esc` and the child is
gone, a host that logs in is added with no popup), the first login (`Command::Popup` to this computer with
the interactive line, the machine dialled when the popup pane closes, `needs login` with the fix when the
login still fails), removing (config, supervisor, sidebar, daemon id, saved layout lane, and the remote
daemon's pane still alive over a second connection) and retrying a refusal. The frames of the dialog are
`client/screen/machines_tests.rs` (`MAITTY_SHOW=1 cargo test machines_tests -- --nocapture` prints them
at 80x24, 100x30 and 160x45).

### The Machines dialog by hand

No test drives a real terminal, so look once with a nested server and the shim:

1. `cargo build --bin maitty --example fake-ssh`. Make a folder `lookat` and put a copy of
   `target/debug/examples/fake-ssh` (`.exe` on Windows) in it named `ssh` (`ssh.exe`).
2. In a scratch shell, point everything at a private daemon and put the shim first on the path:
   `MAITTY_SOCKET=lookat`, `MAITTY_DATA_DIR=<lookat>/data`, `MAITTY_CONFIG=<lookat>/config.toml`,
   `MAITTY_FAKE_SSH_MAITTY=<path to target/debug/maitty>`, `PATH=<lookat>:$PATH` (PowerShell:
   `$env:NAME = ...`). A machine added in the dialog is reached with plain `ssh`, which is now the
   shim, and the shim runs `maitty proxy` against that same private daemon.
3. Run `target/debug/maitty`, press the leader, then `w`. Type `box` and Enter: the dialog says
   `Checking box…`, then the machine appears in *Yours* as `connecting` and then `connected`, and its group
   is in the sidebar. `<lookat>/config.local.toml` holds one `[[machines]]` entry (`name = "box"`,
   `target = "box"`) and `config.toml` is untouched.
4. `Tab`, `x`, Enter removes it: its group leaves the sidebar and the entry leaves the file, and the
   daemon's sessions are still there.
5. Popup path: quit, set `MAITTY_FAKE_SSH_MODE=auth-fail`, start `maitty`, add `box` again. The probe
   fails with a login, the dialog closes with the toast `ssh needs you to log in: use the popup`, and a popup runs the interactive line
   (the shim prints `Permission denied` and the popup closes). The machine then shows `needs login`
   with its fix. Set `MAITTY_FAKE_SSH_MODE=unreachable` instead and nothing is added: the dialog shows
   ssh's `Connection refused` line. `hang` shows `Checking box…` until `Esc` (or 12 s).
6. With a real host (no shim): leader `w`, pick a host from your `~/.ssh/config`, answer ssh's
   prompts in the popup if it opens one.

`Sandbox` in `fake_ssh.rs` gives each test its own socket, data folder and config, removes
`MAITTY_TERM_ID` and `MAITTY_PANE_TOKEN` (so a suite run inside a pane does not speak as that
pane), stops its daemon on drop, and captures output to files, not pipes: a daemon started by
the command keeps a pipe open on Windows. Copy that pattern for a new remote test.

## Project-specific: required coverage

- Status detection (`daemon/scan.rs`, hook handling): every agent's
  working / needs-you / done transitions.
- Protocol: round-trip of every message through msgpack.
- Checks: exit code, timeout and spawn failure map to one `CheckState` (table test in
  `daemon/check.rs`); the output cap cuts on a character boundary; a saved pane without the
  check fields still loads.
  The runner (`run_check`) is tested with real shells: pass, fail, timeout (and no child left
  running), missing command, a scrubbed environment; the daemon hook-in in `daemon/logic_tests.rs`.
  Several checkpoints run there too, with a stand-in resolver (`Daemon::resolver`) for the
  approval and never gates. Stale passes use a real temporary git repo and call the git
  poll's probe directly (`stale_probes` then `check_stale::probe`), with no sleeps.
- Key handling: every default binding resolves to an action, and new keys don't
  collide.
- Every popup: a render test that it opens, fits at 100×30, and closes on Esc.
- Themes: the contrast audit test covers every built-in theme.
- Daemon logic: `crates/maitty-core/src/daemon/logic_tests.rs` drives a real `Daemon` (`Daemon::new`) with real
  shell panes; new daemon logic comes with a test there.

## Project-specific: interaction performance

Native Pi/OMP/OpenCode/Kilo adapters also run without an installed agent, network, or real
status socket: `node --experimental-vm-modules --test crates/maitty-terminal-client/src/cli/integrations.test.mjs`.
The VM supplies the process-launch boundary and executes the bundled adapters.
Cover nested dialogs, event ordering, helper versus parent completion, unrelated
sessions, reporting failures/recovery and inert behavior outside Maitty. CI runs
these adapter tests on Windows and Linux; Node's VM flag is test-only.
Cover OpenCode TUI V1/V2 selection, initial idle, shared-backend isolation and
disposal. Exercise OMP retry/continuation timers deterministically. Rust tests
validate Kimi/Mastracode config shapes, OpenCode JSONC retention/migration, all
29 process identities, all 22 bundled screen manifests and Codex's wrapped footer
with/without completion hooks. Use the exact screenshot footer as a regression.

Run `cargo test --release session_interactions_under_heavy_output -- --ignored --nocapture`
for the opt-in interaction benchmark. It feeds eight panes with output while
interleaving acknowledged session switches, scrollback and resize/rewrap. It
checks focus, pane bounds, scrollback and retained output. Ordinary CI excludes
this timing measurement; there is no machine-dependent pass threshold.

Windows TestBackend measurement on 2026-10-15: 960 output messages, 4,300,800 bytes.

| Operation | p50 | p95 | Maximum |
| --- | --- | --- | --- |
| Session switch and draw | 3.896 ms | 7.881 ms | 8.405 ms |
| Scroll and draw | 0.212 ms | 0.529 ms | 0.547 ms |
| Resize, rewrap and draw | 3.799 ms | 6.644 ms | 7.404 ms |
| Eight-pane output burst | 0.484 ms | 0.733 ms | 1.027 ms |

These are optimized app/TestBackend timings with simulated daemon acknowledgements,
not native terminal presentation, IPC latency or live-agent measurements. Check
subjective smoothness separately in the normal terminal with real sessions.
