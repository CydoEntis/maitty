import { describe, expect, it } from 'vitest'
import { commitTextFor } from './commitText'

const added = (path: string) => ({ path, patch: 'diff --git a/x b/x\nnew file mode 100644\n@@ -0,0 +1 @@\n+x' })
const edited = (path: string) => ({ path, patch: 'diff --git a/x b/x\n@@ -1 +1 @@\n-a\n+b' })
const removed = (path: string) => ({ path, patch: 'diff --git a/x b/x\ndeleted file mode 100644\n@@ -1 +0,0 @@\n-x' })

describe('a suggested commit message', () => {
  it('is empty with nothing changed', () => {
    expect(commitTextFor([])).toBe('')
  })
  it('names a few files by what happened to them', () => {
    expect(commitTextFor([added('src/a.ts')])).toBe('Add a.ts')
    expect(commitTextFor([edited('src/a.ts'), edited('b.ts')])).toBe('Update a.ts, b.ts')
    expect(commitTextFor([removed('old.ts')])).toBe('Remove old.ts')
  })
  it('says Update when the changes are mixed', () => {
    expect(commitTextFor([added('a.ts'), removed('b.ts')])).toBe('Update a.ts, b.ts')
  })
  it('counts many files and names the folder they share', () => {
    const many = ['a', 'b', 'c', 'd'].map((name) => edited(`src/ui/${name}.ts`))
    expect(commitTextFor(many)).toBe('Update 4 files in src/ui')
    expect(commitTextFor([...many.slice(0, 3), edited('top.ts')])).toBe('Update 4 files')
  })
})
