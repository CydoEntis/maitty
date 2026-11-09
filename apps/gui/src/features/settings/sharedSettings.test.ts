import { describe, expect, it } from 'vitest'
import type { SharedSettings } from '@/api/sharedSettings'
import { framesOfTuiSpinner } from '@/lib/spinner-clock'
import { readSettingsPreferences } from './preferences'
import { applyShared, changesToSend, sharedFrom } from './sharedSettings'

const SHARED: SharedSettings = {
  default_agent: 'codex',
  new_session_view: 'terminal',
  sidebar_side: 'right',
  theme_dark: 'Nord',
  theme_light: 'Maitty Day',
  appearance: 'light',
  spinner: [...framesOfTuiSpinner('snake')],
  start_dir: '/srv',
  reduce_motion: true,
  window_style: 'glass',
}

describe('the settings shared with the terminal app', () => {
  it('lays what the daemon says over the preferences', () => {
    const next = applyShared({ ...readSettingsPreferences(), themeMode: 'dark' }, SHARED)
    expect(next).toMatchObject({
      defaultAgent: 'codex', newSessionView: 'terminal', sidebarSide: 'right', darkTheme: 'Nord', lightTheme: 'Maitty Day',
      themeMode: 'light', spinner: 'snake', startFolder: '/srv', reduceMotion: true, style: 'glass',
    })
  })

  it('keeps what this window can hold and the shared config cannot', () => {
    const own = { ...readSettingsPreferences(), themeMode: 'system' as const, spinner: 'circle' as const, defaultAgent: 'claude' as const }
    const next = applyShared(own, { ...SHARED, default_agent: 'last used', theme_dark: 'No such theme', spinner: ['x', 'y'] })
    expect(next).toMatchObject({ themeMode: 'system', spinner: 'circle', defaultAgent: 'claude', darkTheme: own.darkTheme })
  })

  it('does not name a key it cannot write: System mode and a web spinner stay home', () => {
    const shared = sharedFrom({ ...readSettingsPreferences(), themeMode: 'system', spinner: 'ping' })
    expect(shared).not.toHaveProperty('appearance')
    expect(shared).not.toHaveProperty('spinner')
    expect(sharedFrom({ ...readSettingsPreferences(), themeMode: 'dark', spinner: 'bars' })).toMatchObject({ appearance: 'dark', spinner: [...framesOfTuiSpinner('bars')] })
  })

  it('sends only what the user changed and the daemon does not already have', () => {
    const before = applyShared(readSettingsPreferences(), SHARED)
    expect(changesToSend(before, before, SHARED)).toEqual([])
    expect(changesToSend(before, { ...before, sidebarSide: 'left', reduceMotion: false }, SHARED)).toEqual([['sidebar_side', 'left'], ['reduce_motion', false]])
    // A change the daemon already reported (it came from the other app) is not sent back.
    expect(changesToSend(before, { ...before, sidebarSide: 'right', style: 'glass' }, SHARED)).toEqual([])
    // A stand-in for the shared "last used" is not an edit, so it never overwrites it.
    expect(changesToSend(before, { ...before }, { ...SHARED, default_agent: 'last used' })).toEqual([])
  })
})
