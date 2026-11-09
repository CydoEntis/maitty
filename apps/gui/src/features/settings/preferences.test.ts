import { beforeEach, describe, expect, it, vi } from 'vitest'
import { hasSavedSettings, readSettingsPreferences, writeSettingsPreferences, type SettingsPreferences } from './preferences'

const DEFAULTS: SettingsPreferences = {
  accent: 'teal',
  spinner: 'braille',
  restoreLayout: true,
  reduceMotion: false,
  animateSidebarActivity: true,
  compactSidebar: false,
  toastPosition: 'top-right',
  notifyNeedsYou: true,
  notifyFinished: true,
  notifyError: true,
  notificationSound: false,
  terminalFontSize: 13,
  terminalFontFamily: '',
  terminalFontWeight: 400,
  terminalFontWeightBold: 700,
  startFolder: '',
  themeMode: 'system',
  darkTheme: 'Maitty Night',
  lightTheme: 'Maitty Day',
  cursorStyle: 'block',
  scrollback: 10000, scrollSpeed: 1,
  
  
  defaultAgent: 'claude', newSessionView: 'chat',
  density: 'comfortable', sidebarTextSize: 13.5, tabTextSize: 13.5, sidebarWidth: 264, sidebarSide: 'left', sidebarOrder: 'creation', terminalScreenReader: false, autoDownloadUpdates: true, restartWhenIdle: true, monthlyBudget: null, iconSize: 'medium', iconColor: 'color', statusBar: false, chatViewDefault: { claude: 'chat', codex: 'chat', gemini: 'chat' }, chatFullAccessDefault: false, yoloClaude: false, yoloCodex: false, yoloGemini: false, perfOverlay: false, terminalPlainDrawing: false, terminalSeeThrough: true, backgroundGlow: false, cursorBlink: false, style: 'glass',
}

describe('settings preferences', () => {
  beforeEach(() => { localStorage.clear() })

  it('opens a new session as a chat with Claude until told otherwise, and ignores a value it does not know', () => {
    expect(readSettingsPreferences()).toMatchObject({ defaultAgent: 'claude', newSessionView: 'chat' })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ defaultAgent: 'codex', newSessionView: 'terminal' }))
    expect(readSettingsPreferences()).toMatchObject({ defaultAgent: 'codex', newSessionView: 'terminal' })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ defaultAgent: 'gemini', newSessionView: 'sideways' }))
    expect(readSettingsPreferences()).toMatchObject({ defaultAgent: 'claude', newSessionView: 'chat' })
  })

  it('starts with a flat ground and a steady cursor, and keeps a stored choice', () => {
    expect(readSettingsPreferences()).toMatchObject({ backgroundGlow: false, cursorBlink: false })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'glass', backgroundGlow: true, cursorBlink: true }))
    expect(readSettingsPreferences()).toMatchObject({ backgroundGlow: true, cursorBlink: true })
  })

  it('installs updates when idle by default, and a stored off stays off', () => {
    expect(readSettingsPreferences().restartWhenIdle).toBe(true)
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'glass' }))
    expect(readSettingsPreferences().restartWhenIdle).toBe(true)
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'glass', restartWhenIdle: false }))
    expect(readSettingsPreferences().restartWhenIdle).toBe(false)
  })

  it('uses the defaults until preferences are saved', () => {
    expect(readSettingsPreferences()).toEqual(DEFAULTS)
  })

  it('restores every saved choice', () => {
    const saved: SettingsPreferences = { ...DEFAULTS, accent: 'violet', spinner: 'snake', restoreLayout: false, reduceMotion: true, toastPosition: 'top-left', notifyNeedsYou: false, notificationSound: true, terminalFontSize: 15, themeMode: 'dark', darkTheme: 'Tokyo Night', cursorStyle: 'underline', scrollback: 50000, }
    writeSettingsPreferences(saved)

    expect(readSettingsPreferences()).toEqual(saved)
  })

  it('keeps the choice to draw terminals without the GPU, and ignores a value that is not a switch', () => {
    writeSettingsPreferences({ ...DEFAULTS, terminalPlainDrawing: true })
    expect(readSettingsPreferences().terminalPlainDrawing).toBe(true)

    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ terminalPlainDrawing: 'yes' }))
    expect(readSettingsPreferences().terminalPlainDrawing).toBe(false)
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
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ accent: 'blue', spinner: 'bars', restoreLayout: true, reduceMotion: false, animateSidebarActivity: true, compactSidebar: true, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: false, notificationSound: false, terminalFontSize: 10, startFolder: '~/code' }))

    expect(readSettingsPreferences()).toEqual({ ...DEFAULTS, accent: 'blue', spinner: 'bars', compactSidebar: true, toastPosition: 'bottom-right', notifyFinished: false, terminalFontSize: 12, startFolder: '~/code', })
  })

  it('reads the layout choice saved as restoreTabs before fleets, and prefers the new name', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ restoreTabs: false }))
    expect(readSettingsPreferences().restoreLayout).toBe(false)
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ restoreTabs: false, restoreLayout: true }))
    expect(readSettingsPreferences().restoreLayout).toBe(true)
  })

  it('starts a fresh install in Glass and keeps any saved window style', () => {
    expect(readSettingsPreferences()).toMatchObject({ style: 'glass' })
    for (const style of ['glass', 'mono', 'console', 'pixel']) {
      localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style }))
      expect(readSettingsPreferences()).toMatchObject({ style })
    }
  })

  it('reads the old style names, and a saved Flat or Tiled, as Console, and falls back from nonsense', () => {
    const style = (value: string) => {
      localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: value }))
      return readSettingsPreferences().style
    }
    expect([style('minimal'), style('expressive'), style('tiled'), style('flat'), style('glass'), style('frosted')]).toEqual(['console', 'glass', 'console', 'console', 'glass', 'glass'])
    expect(style('neon')).toBe('glass')
  })

  it('hides the status bar unless it was turned on', () => {
    expect(readSettingsPreferences().statusBar).toBe(false)
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: true }))
    expect(readSettingsPreferences().statusBar).toBe(true)
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ statusBar: 'no' }))
    expect(readSettingsPreferences().statusBar).toBe(false)
  })

  it('starts new agent sessions as chats with full access off, and ignores nonsense', () => {
    expect(readSettingsPreferences()).toMatchObject({ chatFullAccessDefault: false })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ chatFullAccessDefault: true }))
    expect(readSettingsPreferences()).toMatchObject({ chatFullAccessDefault: true })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ chatFullAccessDefault: 'yes' }))
    expect(readSettingsPreferences()).toMatchObject({ chatFullAccessDefault: false })
  })

  it('opens agent sessions in the chat unless a view was chosen, and ignores nonsense', () => {
    expect(readSettingsPreferences().chatViewDefault).toEqual({ claude: 'chat', codex: 'chat', gemini: 'chat' })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ chatViewDefault: { claude: 'terminal', codex: 'sideways' } }))
    expect(readSettingsPreferences().chatViewDefault).toEqual({ claude: 'terminal', codex: 'chat', gemini: 'chat' })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ chatViewDefault: 'terminal' }))
    expect(readSettingsPreferences().chatViewDefault).toEqual({ claude: 'chat', codex: 'chat', gemini: 'chat' })
  })

  it('keeps icons in colour unless grayscale was chosen, in any style', () => {
    expect(readSettingsPreferences().iconColor).toBe('color')
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'mono', iconColor: 'grayscale' }))
    expect(readSettingsPreferences()).toMatchObject({ style: 'mono', iconColor: 'grayscale' })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'mono', iconColor: 'sepia' }))
    expect(readSettingsPreferences().iconColor).toBe('color')
  })

  it('keeps the interface sizes in range and falls back from nonsense', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ density: 'spacious', sidebarTextSize: 99, tabTextSize: 'big', sidebarWidth: 5 }))

    expect(readSettingsPreferences()).toMatchObject({ density: 'spacious', sidebarTextSize: 17, tabTextSize: 13.5, sidebarWidth: 208 })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ density: 'huge', sidebarWidth: 9999 }))
    expect(readSettingsPreferences()).toMatchObject({ density: 'comfortable', sidebarWidth: 440 })
    // The old grouping and tray settings are gone: a saved copy is ignored, not carried.
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ }))
    expect(readSettingsPreferences()).not.toHaveProperty('trayStyle')
    expect(readSettingsPreferences()).not.toHaveProperty('groupBy')
  })

  it('keeps a valid terminal font and falls back to the defaults for nonsense', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ terminalFontFamily: ' Fira Code ', terminalFontWeight: 500, terminalFontWeightBold: 800 }))
    expect(readSettingsPreferences()).toMatchObject({ terminalFontFamily: 'Fira Code', terminalFontWeight: 500, terminalFontWeightBold: 800 })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ terminalFontFamily: 'x"; background: red', terminalFontWeight: 450, terminalFontWeightBold: 'bold' }))
    expect(readSettingsPreferences()).toMatchObject({ terminalFontFamily: '', terminalFontWeight: 400, terminalFontWeightBold: 700 })
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ terminalFontFamily: 7, terminalFontWeight: null }))
    expect(readSettingsPreferences()).toMatchObject({ terminalFontFamily: '', terminalFontWeight: 400 })
  })

  it('ignores per-project colours saved by an older version, without an error', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ density: 'spacious', projectColors: { '/code/maitty': 'violet' } }))

    const read = readSettingsPreferences()

    expect(read.density).toBe('spacious')
    expect(read).not.toHaveProperty('projectColors')
  })

  it('carries over the theme chosen before the theme browser', () => {
    localStorage.setItem('maitty.gui.theme.v1', 'day')

    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'light', lightTheme: 'Maitty Day', darkTheme: 'Maitty Night' })
  })

  it('moves a single saved theme into its dark or light slot and keeps using it', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ theme: 'Tokyo Night' }))
    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'dark', darkTheme: 'Tokyo Night', lightTheme: 'Maitty Day' })

    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ theme: 'Gruvbox Light' }))
    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'light', lightTheme: 'Gruvbox Light', darkTheme: 'Maitty Night' })
  })

  it('keeps each slot to its own kind of theme and the mode to its three values', () => {
    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ themeMode: 'dark', darkTheme: 'Maitty Day', lightTheme: 'Tokyo Night' }))
    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'dark', darkTheme: 'Maitty Night', lightTheme: 'Maitty Day' })

    localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ themeMode: 'auto', darkTheme: 'Maitty Dark' }))
    expect(readSettingsPreferences()).toMatchObject({ themeMode: 'system', darkTheme: 'Maitty Dark' })
  })

  it('has no monthly budget by default and keeps only a positive amount in range', () => {
    expect(readSettingsPreferences().monthlyBudget).toBeNull()
    for (const [stored, expected] of [[50, 50], [12.5, 12.5], [0, null], [-5, null], ['50', null], [2_000_000, null], [null, null]] as const) {
      localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'glass', monthlyBudget: stored }))
      expect(readSettingsPreferences().monthlyBudget).toBe(expected)
    }
  })
})
