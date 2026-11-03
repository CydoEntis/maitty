import { describe, expect, it } from 'vitest'
import { branchNameProblem } from './branchName'

describe('branchNameProblem', () => {
  it.each(['feat/inspect-panel', 'fix_123', 'release-1.2', 'a'])('accepts %s', (name) => {
    expect(branchNameProblem(name)).toBeNull()
  })

  it.each([
    ['', 'Name the branch'],
    ['-rf', 'dash'],
    ['/x', 'slash'],
    ['x/', 'slash'],
    ['x.', 'dot'],
    ['x.lock', 'dot'],
    ['a..b', '..'],
    ['a b', 'spaces'],
    ['a~1', 'spaces'],
    ['a\u0007b', 'spaces'],
  ])('refuses %j', (name, hint) => {
    expect(branchNameProblem(name)).toContain(hint)
  })
})
