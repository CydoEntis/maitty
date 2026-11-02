import { beforeEach, describe, expect, it } from 'vitest'
import { readSettingsPreferences, writeSettingsPreferences } from './preferences'

describe('settings preferences', () => {
  beforeEach(() => { localStorage.clear() })

  it('uses notification defaults until preferences are saved', () => {
    expect(readSettingsPreferences()).toEqual({ accent: 'teal', spinner: 'braille', restoreTabs: true, reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notificationSound: false })
  })

  it('restores saved toast and notification choices', () => {
    writeSettingsPreferences({ accent: 'violet', spinner: 'snake', restoreTabs: false, reduceMotion: true, animateSidebarActivity: false, compactSidebar: true, toastPosition: 'top-left', notifyNeedsYou: false, notifyFinished: true, notificationSound: true })

    expect(readSettingsPreferences()).toEqual({ accent: 'violet', spinner: 'snake', restoreTabs: false, reduceMotion: true, animateSidebarActivity: false, compactSidebar: true, toastPosition: 'top-left', notifyNeedsYou: false, notifyFinished: true, notificationSound: true })
  })

  it('falls back safely when saved preferences are malformed', () => {
    localStorage.setItem('maitty.gui.settings.v1', '{')

    expect(readSettingsPreferences()).toEqual({ accent: 'teal', spinner: 'braille', restoreTabs: true, reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notificationSound: false })
  })
})
