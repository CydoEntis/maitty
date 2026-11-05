import { describe, expect, it } from 'vitest'
import { parseUnifiedDiff, replacementHunk, splitLines, wholeFileDiff } from './unifiedDiff'

describe('parseUnifiedDiff', () => {
  it('counts added and removed lines and keeps context', () => {
    const diff = parseUnifiedDiff('@@ -1,3 +1,3 @@\n keep\n-old\n+new\n tail\n')
    expect(diff).toEqual({
      status: 'available',
      added: 1,
      removed: 1,
      truncated: false,
      hunks: [
        {
          header: '@@ -1,3 +1,3 @@',
          lines: [
            { kind: 'context', text: 'keep' },
            { kind: 'remove', text: 'old' },
            { kind: 'add', text: 'new' },
            { kind: 'context', text: 'tail' },
          ],
        },
      ],
    })
  })

  it('skips file headers before the first hunk but keeps a changed line that starts with dashes inside one', () => {
    const diff = parseUnifiedDiff('--- a/x\n+++ b/x\n@@ -1 +1 @@\n--- not a header\n+new\n')
    expect(diff).toMatchObject({ added: 1, removed: 1 })
  })

  it('reads several hunks and ignores the no-newline marker', () => {
    const diff = parseUnifiedDiff('@@ -1 +1 @@\n-a\n+b\n\\ No newline at end of file\n@@ -9 +9 @@\n-c\n+d\n')
    expect(diff).toMatchObject({ added: 2, removed: 2, hunks: [{ lines: expect.any(Array) as unknown }, { header: '@@ -9 +9 @@' }] })
  })

  it('keeps changes in a diff with no hunk header', () => {
    expect(parseUnifiedDiff('+one\n+two\n')).toMatchObject({ added: 2, hunks: [{ header: '' }] })
  })

  it('returns no hunks for empty text', () => {
    expect(parseUnifiedDiff('')).toMatchObject({ status: 'available', hunks: [], added: 0, removed: 0 })
  })
})

describe('building diffs from plain text', () => {
  it('splits lines without inventing an empty last line', () => {
    expect(splitLines('a\nb\n')).toEqual(['a', 'b'])
    expect(splitLines('')).toEqual([])
    expect(splitLines('\n')).toEqual([''])
  })

  it('shows a created file as all added and a deleted one as all removed', () => {
    expect(wholeFileDiff('a\nb', 'add')).toMatchObject({ added: 2, removed: 0 })
    expect(wholeFileDiff('a\nb', 'remove')).toMatchObject({ added: 0, removed: 2 })
  })

  it('shows a replacement as the old lines removed then the new lines added', () => {
    expect(replacementHunk('x\ny', 'z').lines.map((line) => `${line.kind}:${line.text}`)).toEqual(['remove:x', 'remove:y', 'add:z'])
  })
})
