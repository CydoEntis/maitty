import { describe, expect, it } from 'vitest'
import { formatTokens, usageParts } from './usage'

describe('formatTokens', () => {
  it.each([
    [812, '812'],
    [1000, '1.0k'],
    [12_400, '12k'],
    [9_950, '10k'],
    [1_300_000, '1.3M'],
  ])('writes %i as %s', (tokens, text) => {
    expect(formatTokens(tokens)).toBe(text)
  })
})

describe('usageParts', () => {
  it('says only what the provider reported', () => {
    expect(usageParts({ tokens: 12_400, context: 41.6, cost: 1.5 })).toEqual(['12k tokens', '42% context', '$1.50'])
    expect(usageParts({ tokens: null, context: 7, cost: null })).toEqual(['7% context'])
  })

  it('has nothing to show when nothing was reported, so no pill is drawn', () => {
    expect(usageParts({ tokens: null, context: null, cost: null })).toEqual([])
  })
})
