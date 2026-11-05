# 0053. A chat attachment is a file in a private cache, named by path in the message text

- **Status:** Proposed (the owner confirms the points under *To confirm*)
- **Date:** 2026-11-04
- **Relates to:** [0036](0036-secrets-never-enter-chat.md) (secrets and chat);
  [0049](0049-image-paste-to-another-machine.md) and
  [0050](0050-send-files-to-another-machine.md) (the image and file caps, the private cache, the
  transfer to another machine); [0052](0052-an-agent-in-a-pane-gets-a-read-only-conversation.md)
  (read-only chats have no composer)

## Context

The chat composer sends text. A person also needs to show the agent a screenshot, a design zip or
a log, as in Claude's own chat. Today the daemon's send carries text only
(`ConversationSend { id, generation, operation, text }`). Claude Code receives it as a
`stream-json` user message with one text block; Codex receives `turn/start` with
`input: [{ "type": "text", ... }]`. Both agents already read a file or an image given as a path
with their own tools (Claude's Read, Codex's image viewer), which is how a path typed into a
terminal pane works today (ADR 0049).

## Decision

1. **The message names the files by path, in its text.** An attachment is a file on the machine the
   agent runs on. `buildAttachmentMessage(text, attachments)` (the one place the message is made)
   appends, after a blank line, a fixed line `Attached files (kept by Maitty; read them with your
   file tools):` and one `- <absolute path>` line per file. The same module parses that block back
   so the sent message shows chips instead of the paths. The daemon, the wire protocol and the
   adapters are not changed, so `PROTOCOL_VERSION` stays 71.
2. **A structured field slots in later.** If a provider gains a better form (Codex `localImage`
   items, Claude inline image blocks), a `#[serde(default)] attachments` field on
   `ConversationSend` carries them; an old peer ignores it, so no bump is needed once a test
   decodes the old shape. Only `buildAttachmentMessage` and the daemon's send change; the
   composer does not.
3. **Caps and kinds.** At most 10 files per message. Images (PNG, JPEG, GIF, WebP, checked by
   signature) up to 25 MB; any other regular file up to 100 MB. A zip is a file: it is passed as
   a path and never unpacked by Maitty. A folder, a link to a folder, a device or an empty file is
   refused. The window checks the count; the host checks the rest, because the window's word is
   not trusted.
4. **Where files live.** The host copies each file into `attachments/` in the app's cache folder
   (owner-only: folder 700, file 600, written new, never over another file), named
   `attach-<ms>-<salt>-<cleaned name>`. The cleaned name keeps letters, digits, `.`, `_` and `-`
   and turns everything else (spaces, control, direction and invisible characters, path
   separators) into `_`, at most 64 characters, so a path in the message never needs quoting.
   Files older than a week are removed when a new one is added, as pastes are. A pasted image
   uses the existing paste cache (`save_pasted_image`).
5. **History keeps the path, never the bytes.** The conversation records the message text, so the
   cache path (and the cleaned name inside it) is what is stored. A file that has been swept
   shows as "no longer available". Sending a file is the person's own act; Maitty does not scan
   it. Secrets stay out of chat (0036): the guide says an attached file is read by the agent and
   kept in history by name.
6. **Thumbnails** are made in the window, from bytes it already holds (a pasted image), as an
   object URL revoked on remove or send. Nothing decodes an image on the host. A file picked or
   dropped by path shows a type icon chip, not a picture, because the window never reads it.
7. **Attachment only is allowed.** The text is then just the attachment block; both adapters send
   it as ordinary text.
8. **Another machine.** The composer's attachment intake takes a machine (null for this computer).
   For a machine, a picked or dropped file goes through `send_files_to_machine` and a pasted
   image (Ctrl+Shift+V, Cmd+V) through `send_pasted_image`, the paths of ADR 0049/0050 and nothing
   new on the host: the same checks (regular file, 100 MB; image signature, 25 MB; at most 10),
   the same far-side private folder and name (`paste-<ms>-<salt>-<cleaned name>`), the same
   one-send-at-a-time rule, and the same progress toast with Cancel. Each file is sent when it is
   added, so the chip appears when the machine has kept it, and the chip's path is the far-side
   path. That path is untrusted, so it is accepted only if it is absolute, at most 4096
   characters and free of control, invisible and line-separator characters (so it cannot add a
   line to the message block); it is written in the message as-is, a line of text and not a
   shell word, so no quoting applies. The `sent` event carries the file's size for the chip. A
   send that stops half way keeps the files that landed. No local copy is made. The far side
   keeps the file for a week like any send (0050); removing a chip, or never sending the
   message, does not delete it, because there is no remote delete. A sent message's chips for a
   far-side path cannot be checked with `attachments_exist`, which only knows this computer.
   A chat on another machine does not exist yet (0047, 0054), so nothing sets the chat link's
   `machine` today; when it does, the composer needs no further change.

## Alternatives considered

- **Inline image content blocks / `localImage` items now.** Needs a new daemon send shape and a
  per-provider form for each, and does nothing for zips and logs. Deferred (point 2).
- **A daemon file message.** A protocol bump for no gain over a path (same objection as 0049).
- **Referencing the original file in place.** The path may change, be a link, or be swept from a
  temp folder before the agent reads it, and a dropped path is a string from outside. A private
  copy is stable and checked.
- **Unpacking zips.** More surface (zip bombs, path escapes) for no gain; the agent can unzip.
- **Storing bytes in history.** Large, and a secret copy; the path is enough.

## To confirm

- Path-in-text for v1 (point 1) rather than a structured send now.
- The caps (10 files, 25 MB images, 100 MB files) and a one-week cache.
- Picked and dropped images show an icon chip, not a thumbnail (point 6).
- Remote attachments are sent when added, not when the message is sent (point 8), so an abandoned draft leaves files on the machine for the week; the alternative is to send at submit, which delays the send and loses the chip's early failure.
