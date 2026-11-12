import { describe, expect, it } from 'vitest'
import { lossOfRemoving, mergeBlock, overlapWords, standingWords } from './worktreeAdvice'

const clean = { dirty: 0, branch: 'task', merged: true, last_commit: '3 days ago', missing: false }

describe('what a worktree can lose', () => {
  it('says its state in words', () => {
    expect(standingWords(clean)).toBe('merged, nothing uncommitted · last commit 3 days ago')
    expect(standingWords({ ...clean, missing: true })).toBe('its folder is gone')
    expect(standingWords({ ...clean, merged: false })).toBe('not merged anywhere · last commit 3 days ago')
    expect(standingWords({ ...clean, dirty: 1, last_commit: '' })).toBe('1 uncommitted change')
  })

  it('asks before removing only what would be lost', () => {
    expect(lossOfRemoving(clean)).toBeNull()
    expect(lossOfRemoving({ ...clean, dirty: 2 })).toContain('2 uncommitted changes')
    expect(lossOfRemoving({ ...clean, merged: false })).toContain('no other branch')
  })

  it('offers a merge only for committed work that is nowhere else', () => {
    expect(mergeBlock({ ...clean, merged: false })).toBeNull()
    expect(mergeBlock({ ...clean, merged: false, dirty: 1 })).toBe('Commit its changes first.')
    expect(mergeBlock(clean)).toBe('Its work is already on another branch.')
    expect(mergeBlock({ ...clean, branch: null, merged: false })).toBe('It is on no branch.')
  })
})

describe('overlap words', () => {
  it('names the first shared file and counts the rest', () => {
    expect(overlapWords([])).toBe('')
    expect(overlapWords([{ file: 'src/checkout.ts', checkouts: ['orders', 'rate-limit'] }])).toBe('orders and rate-limit both changed checkout.ts')
    expect(overlapWords([{ file: 'a.ts', checkouts: ['x', 'y', 'z'] }, { file: 'b.ts', checkouts: ['x', 'y'] }])).toBe('3 checkouts changed a.ts (and 1 more file)')
  })
})
