import { describe, expect, it } from 'vitest'
import { commentMessage, patchLines } from './patchLines'

const PATCH = [
  'diff --git a/src/a.ts b/src/a.ts',
  '--- a/src/a.ts',
  '+++ b/src/a.ts',
  '@@ -10,3 +10,4 @@ function a() {',
  ' const one = 1',
  '--- an old comment',
  '+const two = 2',
  '+const three = 3',
  ' return one',
  '\\ No newline at end of file',
].join('\n')

describe('a diff’s lines', () => {
  it('numbers added and kept lines by the new file and removed lines by the old one', () => {
    const lines = patchLines(PATCH)
    expect(lines.map((line) => [line.kind, line.line])).toEqual([
      ['meta', null], ['meta', null], ['meta', null], ['hunk', null],
      ['context', 10], ['remove', 11], ['add', 11], ['add', 12], ['context', 13], ['meta', null],
    ])
  })

  it('reads a removed line that looks like a file header as code', () => {
    expect(patchLines(PATCH)[5]).toEqual({ kind: 'remove', text: '--- an old comment', line: 11, oldLine: 11, newLine: null })
  })

  it('starts numbering again at each hunk', () => {
    const lines = patchLines('@@ -1 +1 @@\n-a\n+b\n@@ -40,2 +40,2 @@\n x\n-y\n+z')
    expect(lines.filter((line) => line.line !== null).map((line) => line.line)).toEqual([1, 1, 40, 41, 41])
  })
})

describe('a comment on a line', () => {
  it('tells the agent the file, the line and what it says, then the comment', () => {
    const added = patchLines(PATCH)[6]
    if (!added) throw new Error('no line')
    expect(commentMessage('src/a.ts', added, '  use a constant  ')).toBe('About src/a.ts, line 11:\n> const two = 2\n\nuse a constant')
  })

  it('says when the line was removed', () => {
    const removed = patchLines(PATCH)[5]
    if (!removed) throw new Error('no line')
    expect(commentMessage('src/a.ts', removed, 'keep this')).toContain('line 11 (removed)')
  })
})
