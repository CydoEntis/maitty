import { describe, expect, it } from 'vitest'
import { lossOfRemoving, mergeBlock, standingWords } from './worktreeAdvice'

const clean = { dirty: 0, branch: 'task', merged: true, last_commit: '3 days ago' }

describe('what a worktree can lose', () => {
  it('says its state in words', () => {
    expect(standingWords(clean)).toBe('merged, nothing uncommitted · last commit 3 days ago')
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
