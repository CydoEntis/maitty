import { describe, expect, it } from 'vitest'
import { shortcutModifier } from './shortcut-modifier'

describe('shortcutModifier', () => {
  it('uses the Command key on macOS', () => {
    expect(shortcutModifier('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)')).toBe('⌘')
  })

  it.each(['Mozilla/5.0 (Windows NT 10.0; Win64; x64)', 'Mozilla/5.0 (X11; Linux x86_64)'])('uses Control on %s', (userAgent) => {
    expect(shortcutModifier(userAgent)).toBe('Ctrl')
  })
})
