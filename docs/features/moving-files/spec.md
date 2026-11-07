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

## Coverage

- R1, R2: MAITTY-227
- R3 and concerns 1 to 4, 7: MAITTY-228
- R4: MAITTY-229
- R5, host half: MAITTY-230; window half: MAITTY-231
- Text how-to (intent, phase 3): MAITTY-232
