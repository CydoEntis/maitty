import { describe, expect, it } from 'vitest'
import { formatTokens } from './usage'

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
