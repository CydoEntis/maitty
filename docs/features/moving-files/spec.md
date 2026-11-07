# Spec: moving files between machines

Status: accepted · Intent: [intent.md](intent.md)

## Concerns (read first)

Data from a remote machine is written to this computer's disk, and ADR 0007 treats a remote
as untrusted. Each rule below is a requirement of the tickets.

1. **The remote decides nothing about where.** The local name comes from the last part of the
   remote path, cleaned: letters, digits and `. _ - space` only, no Windows reserved names
   (`CON`, `NUL`, `COM1`…), no trailing dot or space, at most 120 characters, and `file` when
   nothing is left. The destination folder is chosen here.
2. **Never overwrite.** A name that exists becomes `name-1`, `name-2`, created with
   create-new so two transfers cannot collide.
3. **Whole or not at all.** Bytes go to a hidden `.part` file in the destination and are
   renamed only when the byte count equals what the remote announced. Cancel, an error or the
   cap deletes the `.part`.
4. **The cap is enforced on bytes received**, not on what the remote claims: 1 GB. At the cap
   the stream stops and the file is deleted.
5. **The path you type is only an argument** to a fixed script, quoted once, never part of
   its text. The script reads one regular file and signals no process. Text before its marker
   line (a login banner) is ignored.
6. **Contents are never logged**, and messages carry names only. A pulled file can hold
   secrets; the guide says where it lands.
7. **Progress and Cancel** use the existing `Watch` (bytes so far, a cancel flag).

8. **Names from the machine are untrusted when listed.** An entry must start with the folder the
   machine says it listed, and its name must not contain `/`. A name with a control or invisible
   character is not offered; the count of those is shown. What is shown is exactly what is
   sent back as the path.
9. **Everything is capped.** A listing returns at most 2000 entries, an index at most 5000
   files, to a depth of 6, and the answer is cut at a byte limit. A cut answer says so.
10. **Read-only.** The list and index scripts write nothing, signal nothing, and do not follow
    symlinks when indexing.
11. **Each ask has a time limit** (30 seconds), and a failure leaves the typed-path box working.
12. **Browsing is not limited to the session's folder.** The person may go up to any folder their
    own account can read; only the search index is limited to the session's folder and below.

13. **A tar from a machine is unpacked only after its listing is checked, into a fresh temp folder.**
    It is refused whole, naming the first bad entry, when any entry is absolute, has a `..` part,
    contains a backslash, a colon, a control or invisible character, ends a part in a dot or space, is
    a Windows reserved name, is anything but a regular file or folder (no link, hard link, device,
    fifo or sparse file), lies outside the single top-level folder, or when there are more than
    50000 entries or a path deeper than 64 parts. Nothing is extracted with absolute-path options or
    with the machine's owners. The final folder name is made here and numbered; nothing is overwritten.
14. **The machine makes the archive with `tar -h`**, so symlinks arrive as the files and folders they
    point to and the archive holds no links. The cap is the same 1 GB, on bytes received: an
    uncompressed tar cannot unpack to more.
15. **A folder sent to a machine is archived here** with links stored as links, into a hidden temp
    file that is removed on every path, and lands in the machine's private cache folder.

## Requirements

- R1: a fixed script prints a marker line, the file's size, and then exactly that many bytes;
  it refuses a missing file, a non-file and an unreadable one with distinct exit codes, and a
  file over the cap.
- R2: the ssh line runs it with the path as one quoted argument and the cap as another.
- R3: a streaming reader writes the bytes into a `.part` file, enforces rules 1 to 4, and
  returns the final local path.
- R4: `maitty machine get <machine> <remote path> [local path]` saves into the current folder
  (or the given path) and prints where it landed. Exit 0 done, 2 usage, 1 failure.
- R5: the desktop app saves into `Downloads/maitty/<machine>/` and reports progress and
  completion as events; its dialog offers Cancel and Open folder.

- R6: a fixed script prints one folder's contents (sub-folders, then other entries) with its
  resolved path first. R7: a fixed script prints the regular files below a folder, skipping
  `.git`, `node_modules`, `target`, `dist`, `build`, `.cache`, `__pycache__` and `.venv`.
- R8: the Get a file dialog lists the session's folder, lets the person open folders and go
  up, and ranks index matches as they type (fuzzy: every typed letter in order, better for the
  file name, word starts and consecutive letters).

## Coverage

- R1, R2: MAITTY-227
- R3 and concerns 1 to 4, 7: MAITTY-228
- R4: MAITTY-229
- R5, host half: MAITTY-230; window half: MAITTY-231
- Text how-to (intent, phase 3): MAITTY-232
- R9, R10 (stream a folder): MAITTY-239; receiving and unpacking, concern 13 and 14: MAITTY-240
- R11 (send a folder), concern 15: MAITTY-241; command line: MAITTY-242; window: MAITTY-243
- R6: MAITTY-233; R7: MAITTY-234; concerns 8 to 11 and the parsing: MAITTY-235
- R8, host half: MAITTY-236; matching: MAITTY-237; window half: MAITTY-238
