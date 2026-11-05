import { describe, expect, it } from 'vitest'
import { mapClaudeTool } from './claudeTools'
import { parseCodexFileChanges } from './codexFileChange'
import { OMITTED_MARKER, REDACTED_MARKER, isOmitted, redactionNote } from './redaction'

describe('what the daemon hides from the history', () => {
  it.each([
    ['a Write', 'Write', { file_path: 'app/.env', content: OMITTED_MARKER, omitted_bytes: 25 }],
    ['an Edit', 'Edit', { file_path: 'app/.env', old_string: OMITTED_MARKER, new_string: OMITTED_MARKER }],
    ['a MultiEdit', 'MultiEdit', { file_path: 'app/.env', edits: [{ old_string: OMITTED_MARKER, new_string: OMITTED_MARKER }] }],
  ])('maps %s on a secret file to an omitted diff that keeps the file', (_name, tool, input) => {
    expect(mapClaudeTool(tool, input)).toMatchObject({ kind: 'edit', file: 'app/.env', diff: { status: 'omitted' } })
  })

  it('still maps an edit of an ordinary file to a diff', () => {
    expect(mapClaudeTool('Write', { file_path: 'a.ts', content: 'x' })).toMatchObject({ diff: { status: 'available' } })
  })

  it('maps a Codex change to a secret file to an omitted diff and keeps the other file', () => {
    const output = `app/.env (add)\n${OMITTED_MARKER}\nsrc/a.rs (update)\n@@ -1 +1 @@\n-old\n+new`
    const changes = parseCodexFileChanges('Edited 2 files', output)

    expect(changes.map((change) => [change.file, change.diff.status])).toEqual([
      ['app/.env', 'omitted'],
      ['src/a.rs', 'available'],
    ])
  })

  it('tells masked text from omitted text', () => {
    expect(isOmitted(OMITTED_MARKER)).toBe(true)
    expect(isOmitted(`x ${OMITTED_MARKER}`)).toBe(false)
    expect(redactionNote(`token is ${REDACTED_MARKER}`)).toMatch(/hid it/)
    expect(redactionNote(OMITTED_MARKER)).toMatch(/not kept/)
    expect(redactionNote('plain', null)).toBeNull()
  })
})
