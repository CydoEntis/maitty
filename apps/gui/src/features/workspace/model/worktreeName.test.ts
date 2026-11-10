import { describe, expect, it } from 'vitest'
import { suggestBranchName } from './worktreeName'

describe('a suggested branch name', () => {
  it('is two short words', () => {
    expect(suggestBranchName(new Set(), () => 0)).toBe('bold-badger')
    expect(suggestBranchName(new Set(), () => 0.999)).toBe('young-yak')
  })

  it('is never a name already in use', () => {
    const rolls = [0, 0, 0.5, 0.5]
    let at = 0
    const random = () => rolls[at++] ?? 0.5
    expect(suggestBranchName(new Set(['bold-badger']), random)).toBe('quiet-marten')
  })

  it('takes a number when it keeps landing on names in use', () => {
    expect(suggestBranchName(new Set(['bold-badger', 'bold-badger-2']), () => 0)).toBe('bold-badger-3')
  })
})
