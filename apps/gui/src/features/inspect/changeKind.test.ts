import { describe, expect, it } from 'vitest'
import { changeKind } from './changeKind'

describe('what happened to a changed file', () => {
  it('reads added and deleted from the header git writes', () => {
    expect(changeKind('diff --git a/x b/x\nnew file mode 100644\n@@ -0,0 +1 @@\n+a\n').word).toBe('Added')
    expect(changeKind('diff --git a/x b/x\ndeleted file mode 100644\n@@ -1 +0,0 @@\n-a\n').word).toBe('Deleted')
  })

  it('calls anything else modified, whatever its lines say', () => {
    expect(changeKind('diff --git a/x b/x\n@@ -1 +1 @@\n-new file mode 100644\n+deleted file mode 100644\n').word).toBe('Modified')
    expect(changeKind('').letter).toBe('M')
  })
})
