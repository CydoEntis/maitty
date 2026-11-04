# 0049. A pasted image reaches another machine as a file sent over its own ssh line

- **Status:** Proposed
- **Date:** 2026-11-04
- **Relates to:** [0047](0047-multiple-machines.md) (pasting an image was one of the features that
  were local-only in v1)

## Context

Pasting an image into a pane saves the clipboard image as a file and types its path into the pane;
Claude Code and Codex attach an image given by path. For a pane on another machine that cannot
work as it is: the clipboard is on this computer and the pane's programs read the other
machine's files. ADR 0047 made it local-only and said so in a toast ("Pasting an image is
local-only in v1"), and listed a remote file API as a later piece (MAITTY-140).

The connection to another machine is a protocol stream to its daemon, and the daemon has no message
that carries a file. A message that did would change `PROTOCOL_VERSION`, and every machine would
have to update before it worked.

## Decision

The image is sent over the same hardened ssh line the installer uses (`ssh_command`: no prompts,
no agent forwarding, the host after `--`), as a separate short command, not through the daemon:

- A fixed script (`PASTE_SCRIPT`, one of the fixed remote scripts in `maitty-machines`, with the checks in `remote_paste` in `maitty-core`) runs as `sh -c <script> sh <name> <bytes>`
  with the image on stdin. It is a constant of the app. What varies is the file's name and its size,
  each passed as a quoted word, never as part of the script. The name is checked twice (letters,
  digits, `.`, `_`, `-`; not starting with a dot; at most 64) before the line is built.
- The file is kept in the app's folder of the far side's cache (`$XDG_CACHE_HOME`, else
  `~/.cache`), readable by its owner alone, written beside its place and renamed into it, so a
  half-sent image is never there. Pastes older than a week are cleared, and so are half-sent
  files a killed upload left behind (older than a day).
- The script prints a marker line and then the file's absolute path. The window takes the path
  only from the line after the marker, and only when it is an absolute path with no control or
  invisible character, because the far side is someone else's computer. What ssh says when the
  send fails is shown as one short line without control or invisible characters.
- The image is capped at 25 MB (checked before ssh is started and again by the script's byte
  count). The whole send is capped at two minutes.
- The path is typed into the pane exactly as a local paste types a local path.

Only machines reached by ssh can be sent an image. One reached by a custom transport (`via`) says
so. The machine must run Linux or macOS, as for everything else (0047).

## Alternatives considered

- **A daemon message that carries the image.** Cleaner, but it changes the wire protocol, so every
  machine would have to update first. It would also make the daemon a writer of files for whoever
  can talk to it. Rejected for now; the ssh route can be replaced by it later without changing what
  the person sees.
- **A shared folder or `scp`/`sftp`.** Needs a second protocol and a second set of options to
  harden; the installer already shows that a fixed script over the hardened line is enough.
- **Leaving it local-only.** The error was the reason for this record: a person on this computer
  with a pane on a build machine could not show an agent a screenshot.

## Consequences

- Pasting an image works for a pane on another machine in the terminal app. The desktop app can
  send the same way later, with the same script and checks.
- One more script runs on another machine. It writes only under the app's own cache folder and
  reads only its `HOME` and `XDG_CACHE_HOME`, and refuses a cache folder the user does not own.
- Each paste is a new ssh connection, so it takes a moment and needs the same login that the
  machine's connection already has (no prompt is possible).
- Revisit when a remote file API exists (MAITTY-139), or if the cache folder should be elsewhere.
