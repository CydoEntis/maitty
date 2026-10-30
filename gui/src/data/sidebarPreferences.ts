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
