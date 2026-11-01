import { beforeEach, describe, expect, it } from 'vitest'
import { readThemePreference, writeThemePreference } from './themePreference'

describe('theme preference', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('uses Midnight when no theme has been saved', () => {
    expect(readThemePreference()).toBe('night')
  })

  it('restores the saved Paper theme', () => {
    writeThemePreference('day')

    expect(readThemePreference()).toBe('day')
  })

  it('restores the saved Abyss theme', () => {
    writeThemePreference('abyss')

    expect(readThemePreference()).toBe('abyss')
  })

  it('ignores unsupported saved themes', () => {
    localStorage.setItem('maitty.gui.theme.v1', 'neon')

    expect(readThemePreference()).toBe('night')
  })
})
