import { describe, expect, it } from 'vitest'
import { modeFromScreen, modeLabel, modelLabel } from './chatMeta'

describe('the mode and model in plain words', () => {
  it('names Claude\'s permission modes', () => {
    expect(modeLabel('auto')).toBe('Auto')
    expect(modeLabel('acceptEdits')).toBe('Accept edits')
    expect(modeLabel('bypassPermissions')).toBe('Bypass permissions')
    expect(modeLabel('plan')).toBe('Plan mode')
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

describe('reading the permission mode from the agent\'s screen', () => {
  const screen = (footer: string) => `> hello\n\n  Hey! What are we working on?\n\n────────\n❯ \n────────\n  dev · Sonnet 5.5\n  ${footer}\n`

  it('names each mode the footer shows', () => {
    expect(modeFromScreen(screen('⏵⏵ auto mode on (shift+tab to cycle)'))).toBe('auto')
    expect(modeFromScreen(screen('⏵⏵ accept edits on (shift+tab to cycle)'))).toBe('acceptEdits')
    expect(modeFromScreen(screen('⏸ plan mode on (shift+tab to cycle)'))).toBe('plan')
    expect(modeFromScreen(screen('⏵⏵ bypass permissions on (shift+tab to cycle)'))).toBe('bypassPermissions')
  })

  it('is the default mode when the footer shows none, and unknown on an empty screen', () => {
    expect(modeFromScreen(screen('? for shortcuts'))).toBe('default')
    expect(modeFromScreen('  \n')).toBeNull()
  })
})
