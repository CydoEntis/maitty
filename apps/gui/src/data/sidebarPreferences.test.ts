import { beforeEach, describe, expect, it } from 'vitest'
import { readSidebarPreferences, writeSidebarPreferences, planPreferenceMigration } from './sidebarPreferences'

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

describe('moving saved names and closed sessions to the daemon', () => {
  const saved = {
    pinnedIds: ['1'],
    archivedIds: ['1', '2', '9'],
    renamedTitles: { '1': 'Mine', '2': 'Old name', '9': 'Gone' },
  }

  it('hands over what the daemon lacks, drops what it already has or no longer lists, and keeps pins', () => {
    const plan = planPreferenceMigration(saved, [
      { id: '1', archived: false, named: false },
      { id: '2', archived: true, named: true },
    ])

    expect(plan.archive).toEqual(['1'])
    expect(plan.rename).toEqual([['1', 'Mine']])
    expect(plan.remaining).toEqual({ pinnedIds: ['1'], archivedIds: [], renamedTitles: {} })
  })
})
