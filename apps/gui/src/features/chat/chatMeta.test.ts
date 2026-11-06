import { describe, expect, it } from 'vitest'
import { modeLabel, modelLabel } from './chatMeta'

describe('the mode and model in plain words', () => {
  it('names Claude\'s permission modes', () => {
    expect(modeLabel('auto')).toBe('Auto')
    expect(modeLabel('acceptEdits')).toBe('Accept edits')
    expect(modeLabel('bypassPermissions')).toBe('Bypass permissions')
    expect(modeLabel('plan')).toBe('Plan')
  })

  it('shows a mode it does not know as it was written, and nothing for none', () => {
    expect(modeLabel('futureMode')).toBe('futureMode')
    expect(modeLabel(null)).toBeNull()
  })

  it('names a model by family and version, with or without a date', () => {
    expect(modelLabel('claude-sonnet-5-5')).toBe('sonnet 5.5')
    expect(modelLabel('claude-opus-4-1-20250805')).toBe('opus 4.1')
  })

  it('keeps a model name it cannot read, minus the vendor prefix', () => {
    expect(modelLabel('claude-haiku-latest')).toBe('haiku-latest')
    expect(modelLabel('gpt-6-luna')).toBe('gpt-6-luna')
    expect(modelLabel(null)).toBeNull()
  })
})
