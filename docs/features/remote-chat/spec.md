# Spec: remote chat

Intent: [intent.md](intent.md). Decision: [ADR 0007](../../adr/0007-multiple-machines.md)
(*Remote chat*).

## How a chat works today

The chat view reads the daemon's conversation for the session, never the terminal's text
(ADR 0003). The window learns which conversation a pane has from the daemon's snapshot
(`ConversationSummary`, with `native_term` and `generation`), and then asks the daemon for its
history and sends to it with `Query::ConversationRead`, `Command` messages for send, approve,
answer and cancel. Those go on a short connection the host opens to **this computer's** daemon
(`maitty_desktop_client::request`, which keeps up to four idle connections).

For another machine the window already holds one long connection (the hub, ADR 0007), which
carries terminal output and pane commands. Two things keep a remote pane out of the chat view:

1. `conversationsFromSnapshot` drops the conversations of a machine's snapshot, so no pane
   there has a chat link.
2. The chat commands in the host have no `machine` argument, so they can only reach this
   computer's daemon.

The remote daemon is the same daemon, and its snapshot already carries `conversations`. Nothing
on the remote side has to change.

## Decision

**Route the existing conversation messages by machine. No new message, no `PROTOCOL_VERSION`
change.**

- **Request lines.** The host keeps, per machine, a small pool of request connections beside the
  hub's long one, opened through the same hardened ssh line, **attach only** (never starts a
  daemon), with the same Hello. They carry the conversation queries and commands exactly as the
  local short connection does, and are kept idle and reused the same way (the pool in
  `maitty-desktop-client` becomes keyed by machine; the local key is unchanged). A line that
  fails is dropped and the request is retried once on a fresh one, as the local pool does.
- **Why not the hub's line.** Its replies to `Query::Conversation*` carry no tag, so sharing it
  would need a tag on every conversation reply: a message-shape change, a `PROTOCOL_VERSION` bump
  and a daemon change on every machine. The extra connection costs one ssh session per idle
  line and needs nothing from the remote.
- **Why not read the transcript over ssh.** The daemon is the only reader of a provider's
  transcript and verifies the file belongs to the session (ADR 0003, points 4 and 5); a script
  that tails a file would be a second, unverified reader.
- **Snapshots.** `conversationsFromSnapshot` keeps a remote machine's conversations and gives a
  pane a chat link carrying `machine` (the field already exists for attachments). A conversation
  that belongs to a pane stays that pane's chat, as on this computer; a remote conversation with
  no pane is not listed as its own session in v1.
- **Status.** Unchanged. The remote daemon decides it and the window displays it (ADR 0003).
  Chat state reads the same snapshot status; a remote conversation cannot set a status that its
  own daemon did not.

## Starting a chat on a remote machine

The new chat page starts what a terminal start already does on this computer: a terminal session
with the agent in it, followed by the first message held until the agent is ready
(`send_when_agent_ready`, which already takes a machine). On another machine:

- The session starts as the modal does today (`NewWorkspace`, then the agent's command line is
  typed into the shell). The command line is built in one place from a closed set: the agent, a
  permission mode from a fixed list, and a model only from a fixed list. No free text goes into
  it; it is quoted for a POSIX shell (a Windows remote is out).
- **Model and reasoning:** the page reads this computer's agent caches, which say nothing of the
  remote. On a remote machine it offers the agent's own default and the permission mode, and says
  so. A remote model list is out of scope.
- **Worktree:** decided by the remote daemon's own `worktree.per_agent`. The page does not claim a
  worktree it cannot promise.
- **The old dialog** (`NewSessionModal`, `startFromModal`) is removed. A plain shell on a machine
  (palette, New shell) starts in that machine's home folder with no dialog.

## Trust

A remote daemon is untrusted (ADR 0007). Everything a remote conversation supplies is data:

- Strings (titles, tool names, file names, event text) pass through `maitty_shared::hidden::sanitize`
  in the host before they reach the webview. The chat renders text as it does for a local
  conversation, with no raw HTML.
- A link in a remote conversation opens only after the person confirms the address. An image or
  path in it is never fetched or opened on its own.
- Clipboard writes, notices and sounds from a remote conversation stay off unless that machine
  opts in, as for its terminal.
- A remote conversation's `generation` and ids are used only for requests to that machine's
  lines; they never address another machine's conversations. Ids are kept by machine and
  conversation together, as panes are.
- An answer to a prompt, an approval or a send goes to the machine that holds the conversation
  and to no other.
- Request lines never carry a pane's id or secret to another machine (`hello` drops them).

## Known limits

- A request line does not re-check the daemon id the long connection recorded (ADR 0007). If a
  machine's name starts answering as a different computer, the long connection notices and refuses
  it, and requests stop with it (a refused machine takes nothing); in the moment between, a request
  is told apart only by the conversation id and its generation, which the daemon checks.
- A command (send, approve, answer, cancel) always opens a fresh line; only questions reuse one,
  as on this computer, because a command asked twice could act twice.

## Older and newer machines

A machine whose daemon is outside the client's compatibility window is already refused with a
reason, so a connected machine speaks the conversation messages this client sends: no new refusal
is needed. If a remote answers a conversation request with an error, the chat card shows it and
the pane's terminal works as usual; it never misparses a reply (a reply of the wrong kind is an
error, as locally).

## What stays local-only

Changes, Files, Find, opening a path, and pasting or dropping an image into the composer read or
write this computer's disk or the remote's, and stay gated by `refuseOtherMachine`, saying which
machine. Plane MAITTY-139 is the remote file API.

## Platforms

The request lines use the ssh line that already runs from Windows, macOS and Linux clients
(ADR 0008). A Windows remote stays out of scope.

## Build split

- MAITTY-263: `maitty-desktop-client` keyed request pool and the machine's request line;
  host commands take a `machine`; snapshot keeps remote conversations; sanitising.
- MAITTY-264: the chat view and new chat page on a machine; the old dialog removed; docs.
