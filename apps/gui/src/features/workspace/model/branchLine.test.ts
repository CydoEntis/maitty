import { describe, expect, it } from 'vitest'
import { branchLineOf } from './branchLine'

const row = { branch: 'agent/fix-login', process: 'claude', name: null, title: 'agent/fix-login', machineName: null }

describe('the branch line under a session row', () => {
  it('is left out while the row already says the branch', () => {
    expect(branchLineOf(row)).toBeNull()
  })

  it('names the branch under a row that says something else', () => {
    expect(branchLineOf({ ...row, name: 'login work' })).toBe('agent/fix-login')
    expect(branchLineOf({ ...row, title: 'crates/maitty-core' })).toBe('agent/fix-login')
  })

  it('is never shown for a plain shell or a folder with no branch', () => {
    expect(branchLineOf({ ...row, name: 'x', process: 'shell' })).toBeNull()
    expect(branchLineOf({ ...row, name: 'x', branch: null })).toBeNull()
  })
})
