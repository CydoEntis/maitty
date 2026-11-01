import { beforeEach, describe, expect, it } from 'vitest'
import { readSidebarPreferences, writeSidebarPreferences } from './sidebarPreferences'

describe('sidebar preferences', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('loads empty preferences by default', () => {
    expect(readSidebarPreferences()).toEqual({ pinnedIds: [], archivedIds: [], renamedTitles: {} })
  })

  it('keeps pinned, archived and renamed conversation choices', () => {
    const preferences = {
      pinnedIds: ['42'],
      archivedIds: ['17'],
      renamedTitles: { '42': 'Review layout' },
    }
    writeSidebarPreferences(preferences)

    expect(readSidebarPreferences()).toEqual(preferences)
  })

  it('ignores corrupt preference data', () => {
    localStorage.setItem('maitty.gui.sidebar.v1', '{broken')

    expect(readSidebarPreferences()).toEqual({ pinnedIds: [], archivedIds: [], renamedTitles: {} })
  })
})
