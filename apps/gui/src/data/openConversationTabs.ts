const STORAGE_KEY = 'maitty.gui.tabs.v4'

export type ClosedConversationTab = {
  tabIds: string[]
  selectedId: string | null
}

export function readOpenConversationTabs(): string[] {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === null) return []

  let value: unknown
  try {
    value = JSON.parse(stored)
  } catch (error) {
    console.warn('Ignoring invalid Maitty open tabs', error)
    return []
  }

  if (!Array.isArray(value) || !value.every((id) => typeof id === 'string')) {
    console.warn('Ignoring unsupported Maitty open tabs')
    return []
  }

  return uniqueIds(value)
}

export function writeOpenConversationTabs(tabIds: readonly string[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(uniqueIds(tabIds)))
}

export function openConversationTab(
  tabIds: readonly string[],
  conversationId: string,
  selectedId?: string,
): string[] {
  const uniqueTabIds = uniqueIds(tabIds)
  if (uniqueTabIds.includes(conversationId)) return uniqueTabIds

  const selectedIndex = selectedId ? uniqueTabIds.indexOf(selectedId) : -1
  const insertionIndex =
    selectedIndex < 0 ? uniqueTabIds.length : selectedIndex + 1
  return [
    ...uniqueTabIds.slice(0, insertionIndex),
    conversationId,
    ...uniqueTabIds.slice(insertionIndex),
  ]
}

export function reconcileConversationTabs(
  tabIds: readonly string[],
  availableIds: readonly string[],
  selectedId: string | undefined,
): string[] {
  const available = new Set(availableIds)
  const reconciled = uniqueIds(tabIds).filter((id) => available.has(id))
  return selectedId && available.has(selectedId)
    ? openConversationTab(reconciled, selectedId)
    : reconciled
}

export function closeConversationTab(
  tabIds: readonly string[],
  selectedId: string,
  closingId: string,
): ClosedConversationTab {
  if (tabIds.length <= 1 || !tabIds.includes(closingId)) {
    return { tabIds: [...tabIds], selectedId }
  }

  const closingIndex = tabIds.indexOf(closingId)
  const remaining = tabIds.filter((id) => id !== closingId)
  if (selectedId !== closingId) return { tabIds: remaining, selectedId }

  const nextIndex = Math.min(closingIndex, remaining.length - 1)
  return { tabIds: remaining, selectedId: remaining[nextIndex] ?? null }
}

function uniqueIds(ids: readonly string[]): string[] {
  return [...new Set(ids.filter((id) => id.length > 0))]
}
