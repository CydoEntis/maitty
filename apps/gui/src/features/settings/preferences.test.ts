import { beforeEach, describe, expect, it, vi } from 'vitest'
import { hasSavedSettings, readSettingsPreferences, writeSettingsPreferences, type SettingsPreferences } from './preferences'

const DEFAULTS: SettingsPreferences = {
  accent: 'teal',
  spinner: 'braille',
  restoreTabs: true,
  reduceMotion: false,
  animateSidebarActivity: true,
  compactSidebar: false,
  toastPosition: 'top-right',
  notifyNeedsYou: true,
  notifyFinished: true,
  notifyError: true,
  notificationSound: false,
  terminalFontSize: 13,
  startFolder: '',
  groupBy: 'repo',
  themeMode: 'system',
  darkTheme: 'Midnight',
  lightTheme: 'Paper',
  cursorStyle: 'block',
  scrollback: 10000,
  effectWorking: true,
  effectNeedsYou: true,
  effectDone: true, density: 'comfortable', sidebarTextSize: 13.5, tabTextSize: 13.5, sidebarWidth: 264, sidebarSide: 'left', terminalScreenReader: false, autoDownloadUpdates: true, restartWhenIdle: false, iconSize: 'medium', trayStyle: 'stacked', leadMode: 'focused', iconColor: 'color', statusBar: true, style: 'glass', projectColors: {},
}

describe('settings preferences', () => {
  beforeEach(() => { localStorage.clear() })

  it('uses the defaults until preferences are saved', () => {
    expect(readSettingsPreferences()).toEqual(DEFAULTS)
  })

  it('restores every saved choice', () => {
    const saved: SettingsPreferences = { ...DEFAULTS, accent: 'violet', spinner: 'snake', restoreTabs: false, reduceMotion: true, toastPosition: 'top-left', notifyNeedsYou: false, notificationSound: true, terminalFontSize: 15, themeMode: 'dark', darkTheme: 'Nord', cursorStyle: 'underline', scrollback: 50000, effectNeedsYou: false }
    writeSettingsPreferences(saved)

    expect(readSettingsPreferences()).toEqual(saved)
  })

  it('falls back safely when saved preferences are malformed', () => {
    localStorage.setItem('maitty.gui.settings.v1', '{')

    expect(readSettingsPreferences()).toEqual(DEFAULTS)
  })

  it('starts with the defaults when browser storage is blocked', () => {
    const blocked = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('denied') })
    try {
      expect(readSettingsPreferences()).toEqual(DEFAULTS)
      expect(hasSavedSettings()).toBe(false)
    } finally {
      blocked.mockRestore()
    }
  })

  it('reads preferences saved before the terminal settings existed', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ accent: 'blue', spinner: 'bars', restoreTabs: true, reduceMotion: false, animateSidebarActivity: true, compactSidebar: true, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: false, notificationSound: false, terminalFontSize: 10, startFolder: '~/code', groupBy: 'folder' }))

    expect(readSettingsPreferences()).toEqual({ ...DEFAULTS, style: 'glass', effectWorking: true, effectNeedsYou: true, effectDone: true, accent: 'blue', spinner: 'bars', compactSidebar: true, toastPosition: 'bottom-right', notifyFinished: false, terminalFontSize: 12, startFolder: '~/code', groupBy: 'folder' })
  })

  it('starts a fresh install in Glass with the sweeps on, and Flat or Tiled with them off', () => {
    expect(readSettingsPreferences()).toMatchObject({ style: 'glass', effectWorking: true, effectNeedsYou: true, effectDone: true })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'flat', effectWorking: false, effectNeedsYou: false, effectDone: false }))
    expect(readSettingsPreferences()).toMatchObject({ style: 'flat', effectWorking: false, effectNeedsYou: false, effectDone: false })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'flat', effectWorking: true }))
    expect(readSettingsPreferences()).toMatchObject({ style: 'flat', effectWorking: true, effectDone: true })
  })

  it('reads the old style names as Flat and Glass, keeps Tiled, and falls back from nonsense', () => {
    const style = (value: string) => {
      localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: value }))
      return readSettingsPreferences().style
    }
    expect([style('minimal'), style('expressive'), style('tiled'), style('flat'), style('glass')]).toEqual(['flat', 'glass', 'tiled', 'flat', 'glass'])
    expect(style('neon')).toBe('glass')
  })

  it('shows the status bar unless it was turned off', () => {
    expect(readSettingsPreferences().statusBar).toBe(true)
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: false }))
    expect(readSettingsPreferences().statusBar).toBe(false)
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: 'no' }))
    expect(readSettingsPreferences().statusBar).toBe(true)
  })

  it('keeps icons in colour unless grayscale was chosen, in either style', () => {
    expect(readSettingsPreferences().iconColor).toBe('color')
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'flat', iconColor: 'grayscale' }))
    expect(readSettingsPreferences()).toMatchObject({ style: 'flat', iconColor: 'grayscale' })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'flat', iconColor: 'sepia' }))
    expect(readSettingsPreferences().iconColor).toBe('color')
  })

  it('keeps the interface sizes in range and falls back from nonsense', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ density: 'spacious', sidebarTextSize: 99, tabTextSize: 'big', sidebarWidth: 5 }))

    expect(readSettingsPreferences()).toMatchObject({ density: 'spacious', sidebarTextSize: 17, tabTextSize: 13.5, sidebarWidth: 200 })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ density: 'huge', sidebarWidth: 9999 }))
    expect(readSettingsPreferences()).toMatchObject({ density: 'comfortable', sidebarWidth: 420 })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ trayStyle: 'stacked', leadMode: 'attention' }))
    expect(readSettingsPreferences()).toMatchObject({ trayStyle: 'stacked', leadMode: 'attention' })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ trayStyle: 'wide', leadMode: 'random' }))
    expect(readSettingsPreferences()).toMatchObject({ trayStyle: 'stacked', leadMode: 'focused', iconColor: 'color', projectColors: {} })
  })

  it('carries over the theme chosen before the theme browser', () => {
    localStorage.setItem('maitty.gui.theme.v1', 'day')

    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'light', lightTheme: 'Paper', darkTheme: 'Midnight' })
  })

  it('moves a single saved theme into its dark or light slot and keeps using it', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ theme: 'Nord' }))
    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'dark', darkTheme: 'Nord', lightTheme: 'Paper' })

    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ theme: 'Gruvbox Light' }))
    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'light', lightTheme: 'Gruvbox Light', darkTheme: 'Midnight' })
  })

  it('keeps each slot to its own kind of theme and the mode to its three values', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ themeMode: 'dark', darkTheme: 'Paper', lightTheme: 'Nord' }))
    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'dark', darkTheme: 'Midnight', lightTheme: 'Paper' })

    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ themeMode: 'auto', darkTheme: 'High Contrast' }))
    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'system', darkTheme: 'High Contrast' })
  })
})
