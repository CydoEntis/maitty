import type { Conversation } from '@/data/preview'

const STORAGE_KEY = 'maitty.gui.sidebar.v1'

export type SidebarPreferences = {
  pinnedIds: string[]
  archivedIds: string[]
  renamedTitles: Record<string, string>
}

const EMPTY_PREFERENCES: SidebarPreferences = {
  pinnedIds: [],
  archivedIds: [],
  renamedTitles: {},
}

export type PreferenceMigration = {
  archive: string[]
  rename: [id: string, title: string][]
  // What the browser should keep once the daemon has taken the rest.
  remaining: SidebarPreferences
}

// Names and closed sessions used to live only in this browser. Work out what to hand to the
// daemon (which shares them with every client) and what to forget. An entry for a session the
// daemon no longer lists is stale and dropped; a name the daemon already has wins.
export function planPreferenceMigration(
  preferences: SidebarPreferences,
  conversations: Pick<Conversation, 'id' | 'archived' | 'named'>[],
): PreferenceMigration {
  const known = new Map(conversations.map((conversation) => [conversation.id, conversation]))
  const archive = preferences.archivedIds.filter(
    (id) => known.has(id) && !known.get(id)?.archived,
  )
  const rename = Object.entries(preferences.renamedTitles).filter(
    ([id]) => known.has(id) && !known.get(id)?.named,
  )
  return {
    archive,
    rename,
    remaining: {
      pinnedIds: preferences.pinnedIds,
      archivedIds: [],
      renamedTitles: {},
    },
  }
}

export function readSidebarPreferences(): SidebarPreferences {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (!stored) return structuredClone(EMPTY_PREFERENCES)

  let value: unknown
  try {
    value = JSON.parse(stored)
  } catch (error) {
    console.warn('Ignoring invalid Maitty sidebar preferences', error)
    return structuredClone(EMPTY_PREFERENCES)
  }

  if (!isSidebarPreferences(value)) {
    console.warn('Ignoring unsupported Maitty sidebar preferences')
    return structuredClone(EMPTY_PREFERENCES)
  }

  return value
}

export function writeSidebarPreferences(preferences: SidebarPreferences): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences))
}

function isSidebarPreferences(value: unknown): value is SidebarPreferences {
  if (typeof value !== 'object' || value === null) return false
  const record = value as Record<string, unknown>
  return isStringArray(record.pinnedIds)
    && isStringArray(record.archivedIds)
    && isStringRecord(record.renamedTitles)
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
}

function isStringRecord(value: unknown): value is Record<string, string> {
  return typeof value === 'object'
    && value !== null
    && Object.values(value).every((item) => typeof item === 'string')
}
