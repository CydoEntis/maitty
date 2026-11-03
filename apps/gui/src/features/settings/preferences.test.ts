import { beforeEach, describe, expect, it } from 'vitest'
import { readSettingsPreferences, writeSettingsPreferences, type SettingsPreferences } from './preferences'

const DEFAULTS: SettingsPreferences = {
  accent: 'teal',
  spinner: 'braille',
  restoreTabs: true,
  reduceMotion: false,
  animateSidebarActivity: true,
  compactSidebar: false,
  toastPosition: 'bottom-right',
  notifyNeedsYou: true,
  notifyFinished: true,
  notifyError: true,
  notificationSound: false,
  terminalFontSize: 13,
  startFolder: '',
  groupBy: 'repo',
  theme: 'Midnight',
  cursorStyle: 'block',
  scrollback: 10000,
  effectWorking: false,
  effectNeedsYou: false,
  effectDone: false, density: 'comfortable', sidebarTextSize: 13.5, tabTextSize: 13.5, sidebarWidth: 264, iconSize: 'medium', trayStyle: 'names', leadMode: 'focused', style: 'minimal', projectColors: {},
}

describe('settings preferences', () => {
  beforeEach(() => { localStorage.clear() })

  it('uses the defaults until preferences are saved', () => {
    expect(readSettingsPreferences()).toEqual(DEFAULTS)
  })

  it('restores every saved choice', () => {
    const saved: SettingsPreferences = { ...DEFAULTS, accent: 'violet', spinner: 'snake', restoreTabs: false, reduceMotion: true, toastPosition: 'top-left', notifyNeedsYou: false, notificationSound: true, terminalFontSize: 15, theme: 'Nord', cursorStyle: 'underline', scrollback: 50000, effectNeedsYou: false }
    writeSettingsPreferences(saved)

    expect(readSettingsPreferences()).toEqual(saved)
  })

  it('falls back safely when saved preferences are malformed', () => {
    localStorage.setItem('maitty.gui.settings.v1', '{')

    expect(readSettingsPreferences()).toEqual(DEFAULTS)
  })

  it('reads preferences saved before the terminal settings existed', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ accent: 'blue', spinner: 'bars', restoreTabs: true, reduceMotion: false, animateSidebarActivity: true, compactSidebar: true, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: false, notificationSound: false, terminalFontSize: 10, startFolder: '~/code', groupBy: 'folder' }))

    expect(readSettingsPreferences()).toEqual({ ...DEFAULTS, style: 'expressive', effectWorking: true, effectNeedsYou: true, effectDone: true, accent: 'blue', spinner: 'bars', compactSidebar: true, notifyFinished: false, terminalFontSize: 12, startFolder: '~/code', groupBy: 'folder' })
  })

  it('starts a fresh install in Minimal with the sweeps off, and keeps Expressive for older settings', () => {
    expect(readSettingsPreferences()).toMatchObject({ style: 'minimal', effectWorking: false, effectNeedsYou: false, effectDone: false })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ accent: 'blue' }))
    expect(readSettingsPreferences()).toMatchObject({ style: 'expressive', effectWorking: true, effectNeedsYou: true, effectDone: true })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'minimal', effectWorking: true }))
    expect(readSettingsPreferences()).toMatchObject({ style: 'minimal', effectWorking: true, effectDone: false })
  })

  it('keeps the interface sizes in range and falls back from nonsense', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ density: 'spacious', sidebarTextSize: 99, tabTextSize: 'big', sidebarWidth: 5 }))

    expect(readSettingsPreferences()).toMatchObject({ density: 'spacious', sidebarTextSize: 17, tabTextSize: 13.5, sidebarWidth: 200 })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ density: 'huge', sidebarWidth: 9999 }))
    expect(readSettingsPreferences()).toMatchObject({ density: 'comfortable', sidebarWidth: 420 })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ trayStyle: 'stacked', leadMode: 'attention' }))
    expect(readSettingsPreferences()).toMatchObject({ trayStyle: 'stacked', leadMode: 'attention' })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ trayStyle: 'wide', leadMode: 'random' }))
    expect(readSettingsPreferences()).toMatchObject({ trayStyle: 'names', leadMode: 'focused', projectColors: {} })
  })

  it('carries over the theme chosen before the theme browser', () => {
    localStorage.setItem('maitty.gui.theme.v1', 'day')

    expect(readSettingsPreferences().theme).toBe('Paper')
  })
})
