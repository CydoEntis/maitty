# 0050. Any file can be sent to another machine, over the same ssh line as an image

- **Status:** Proposed
- **Date:** 2026-11-04
- **Relates to:** [0047](0047-multiple-machines.md) (a remote file API was out of scope),
  [0049](0049-image-paste-to-another-machine.md) (the mechanism this widens)

## Context

0049 lets a clipboard image reach a pane on another machine: a fixed script over the machine's own
hardened ssh line keeps it in a private cache folder and prints its path, which is typed into the
pane. People also need to hand an agent a log, a spec, a zip or a binary that lives on this
computer. The same mechanism carries any file; what changes is what may be sent, how big, and what
the person sees while it goes.

## Decision

One mechanism, widened. No daemon message, so **no `PROTOCOL_VERSION` bump** (as in 0049).

- **What may be sent.** A regular file the person names (drops it, or types or pastes its path).
  Directories, devices, pipes and missing paths are refused with a plain message; a symlink is
  followed because the person named it. The terminal app sends one file per request; the desktop
  app, which gets a drop as a list, sends them one after another, at most 10. Nothing is sent without that act: a paste of text never uploads a file, even when the
  text happens to be a path.
- **Size.** 100 MB per file for files, 25 MB for images (0049, unchanged). Checked before ssh starts
  and again by the script's byte count. A file over the cap is refused, never truncated.
- **Where it lands.** The same private folder as images: `$XDG_CACHE_HOME` else `~/.cache`, under
  the app's `pastes` folder, which the script refuses unless the user running it owns it; the
  folder is mode 700, files 600, written beside their place and renamed into it. Files older than a
  week are cleared with the images; half-sent leftovers after a day. Nothing is ever written
  elsewhere.
- **Names.** This computer picks the name: `paste-<ms>-<8 hex>-<original>`. The original is
  reduced to letters, digits, `.`, `_` and `-` (anything else becomes `_`), leading dots dropped,
  cut to 48 characters keeping its extension, `file` when nothing is left. The time and the salt
  make two sends of the same name two files, so nothing is overwritten. The name is a positional
  argument checked twice (client and script), never part of the script text.
- **What the far side says.** Untrusted, exactly as in 0049: only the line after the marker, only
  an absolute path of at most 4096 characters with no control or invisible character; what ssh
  says on failure is cut to 200 characters and cleaned. The path is typed into the pane quoted as
  a local path is.
- **Custom transports.** A machine reached by `via` is refused with the same message as images
  (it needs ssh).
- **Progress and cancel.** The send runs off the UI thread (`spawn_bg`). Bytes written to ssh are
  counted; the person sees `sending <name> to <machine>… 40%`, and a failure as an error toast.
  Cancel ends the ssh process; the script then sees a short file, keeps nothing, and the stale
  sweep removes any leftover. The time limit is 15 minutes for a file (2 minutes for an image).
- **Windows.** The client may be Windows: the local path is read with ordinary file calls and
  streamed on stdin to the Windows OpenSSH client; no local shell is involved. The far side stays
  Linux or macOS (0047). A Windows path with spaces or quotes, as a terminal writes it on a drop,
  has its quotes taken off and is otherwise used as typed.
- **How the person asks.** In the terminal app, a leader key opens a prompt for the path (a
  dropped file pastes its path into it); the same key while a send runs cancels it. A terminal
  cannot tell a drop from a paste, so a drop straight onto a pane is not taken as a send: it stays
  the text it is. The desktop app gets real drop events and sends on a drop.

### Threat model (docs/SECURITY.md)

- The local file is read only because the person named it; the read is a regular file, the size
  is capped, and the bytes go only to the machine they chose, over its own login.
- Everything the person or the far side supplies reaches the remote shell only as one quoted
  positional word from a restricted alphabet. The script is a constant.
- The far side can lie about the path; the client accepts only the checked shape, and it is typed
  into the pane, never run.
- Files are mode 600 in a folder the user owns; another user on the far machine cannot read them
  or plant a folder (the ownership check from 0049). A sent file stays up to a week: a person
  sending a secret should know that, and the guide says so.

## Alternatives considered

- **A daemon file message.** Same objection as 0049: a protocol bump and every machine updated
  first.
- **Treating any pasted existing path as a send.** A path pasted for the remote machine's own file
  (`/etc/hosts`) would silently upload a local one. Rejected.
- **Pasted text too large for a normal paste, sent as a file.** It would hand the agent a path
  where the person pasted text. Not done; revisit if large pastes stall a remote pane.
- **Directories (as an archive).** More surface (symlinks, size walks). Refused for now.
- **A clipboard file list.** The clipboard crate here gives images and text only; not done.

## Decisions to confirm with the owner

- 100 MB per file, and 10 files per action.
- Directories refused.
- A terminal drop is not a send (a prompt is); only the desktop app sends on a drop.
- Large text is not sent as a file.

## Consequences

- One script and one folder serve images and files; the script text is unchanged, so no far-side
  update is needed.
- The terminal app needs a progress and cancel state; the desktop app needs the ssh runner, which
  today lives in the terminal client, moved into `maitty-machines`.

## Amendment: the desktop app

The desktop app sends a drop of up to 10 files in turn, over the runner that now lives in
`maitty-machines` (`remote_send`, `ssh_run`). Decisions that follow from the rest of this ADR:

- Everything is checked before the first byte is sent (each path absolute, a regular file, within
  100 MB; the machine a known ssh machine), so a bad third file does not leave two sent. More than 10
  files are refused whole, never cut.
- One send runs at a time. It stops at the first file that fails, and the files after it are not tried;
  the ones already kept stay typed into the pane.
- The machine must be `connected`, since the path is typed through its line.
- The window types a far-side path only if it is absolute, at most 4096 characters and free of control
  and invisible characters, on top of the host's own check of the same shape.

### The desktop app's image paste

A clipboard image pasted in the desktop app (0049's feature, ticket 19) reuses this send. The window hands the
host the bytes as the raw body of a command, never as base64 in an event. The host checks them (PNG, JPEG, GIF
or WebP signature; 25 MB), stages them as a private file in the app's cache and runs the same job, events and
cancel with the image cap and the two-minute limit; the staged copy is removed when the send ends. The
far-side name is `…-image.<ext>` from the checked type, so a JPEG is not named `.png`. For a pane of this
computer the host keeps the image in the same cache folder and the window types its path. The paste key
is Ctrl+Shift+V on Windows and Linux and Cmd+V on macOS. Plain Ctrl+V on Windows and Linux still sends `^V` to the
program, as every plain Ctrl+letter there does (0029 keeps Alt+V for Claude Code).
