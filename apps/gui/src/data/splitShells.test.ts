import { describe, expect, it } from 'vitest'
import { withSplitShell } from './splitShells'

describe('split terminals', () => {
  it('adds the new one and forgets sessions that are gone', () => {
    const next = withSplitShell(new Set(['a', 'gone']), 'b', new Set(['a', 'b']))
    expect([...next].sort()).toEqual(['a', 'b'])
  })
})
