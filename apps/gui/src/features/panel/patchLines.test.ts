import { describe, expect, it } from 'vitest'
import { parsePatch, splitPath } from './patchLines'

// A patch as git writes it: the file header, then hunks.
const PATCH = [
  'diff --git a/src/a.ts b/src/a.ts',
  'index 111..222 100644',
  '--- a/src/a.ts',
  '+++ b/src/a.ts',
  '@@ -3,4 +3,5 @@ function a() {',
  ' keep',
  '-old one',
  '-old two',
  '+new one',
  '+new two',
  '+new three',
  ' tail',
  '@@ -20 +21 @@',
  '-gone',
  '+here',
  '\\ No newline at end of file',
].join('\n')

describe('parsePatch', () => {
  it('numbers each line from the hunk headers, old and new', () => {
    const lines = parsePatch(PATCH)
    const row = (index: number) => {
      const line = lines[index]
      return line === undefined || line.kind === 'hunk' ? line?.kind : [line.kind, line.oldNo, line.newNo, line.text]
    }

    expect(lines.map((line) => line.kind)).toEqual(['hunk', 'context', 'remove', 'remove', 'add', 'add', 'add', 'context', 'hunk', 'remove', 'add'])
    expect(row(1)).toEqual(['context', 3, 3, 'keep'])
    expect(row(2)).toEqual(['remove', 4, null, 'old one'])
    expect(row(4)).toEqual(['add', null, 4, 'new one'])
    expect(row(6)).toEqual(['add', null, 6, 'new three'])
    expect(row(7)).toEqual(['context', 6, 7, 'tail'])
    expect(row(9)).toEqual(['remove', 20, null, 'gone'])
    expect(row(10)).toEqual(['add', null, 21, 'here'])
  })

  it('leaves out the file header and the no-newline marker', () => {
    const text = parsePatch(PATCH).flatMap((line) => (line.kind === 'hunk' ? [] : [line.text]))

    expect(text).not.toContain('index 111..222 100644')
    expect(text.some((line) => line.startsWith('\\'))).toBe(false)
  })

  it('reads a removed line that looks like a file header as a removed line', () => {
    const [, removed] = parsePatch('@@ -1,2 +1,1 @@\n--- a rule\n+kept')

    expect(removed).toMatchObject({ kind: 'remove', text: '-- a rule' })
  })

  it.each(['', 'Binary files a/x.png and b/x.png differ', 'not a diff at all'])('gives no lines for %j', (patch) => {
    expect(parsePatch(patch)).toEqual([])
  })
})

describe('splitPath', () => {
  it.each([
    ['src/app/a.ts', 'src/app/', 'a.ts'],
    ['a.ts', '', 'a.ts'],
  ])('%s', (path, dir, name) => {
    expect(splitPath(path)).toEqual({ dir, name })
  })
})
