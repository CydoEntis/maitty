const STORAGE_KEY = 'maitty.gui.drafts.v1'

export type ConversationDrafts = Record<string, string>

export function readConversationDrafts(): ConversationDrafts {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (!stored) return {}

  let value: unknown
  try {
    value = JSON.parse(stored)
  } catch (error) {
    console.warn('Ignoring invalid Maitty conversation drafts', error)
    return {}
  }

  if (!isConversationDrafts(value)) {
    console.warn('Ignoring unsupported Maitty conversation drafts')
    return {}
  }

  return value
}

export function writeConversationDrafts(drafts: ConversationDrafts): void {
  const savedDrafts = Object.fromEntries(Object.entries(drafts).filter(([, draft]) => draft.length > 0))
  localStorage.setItem(STORAGE_KEY, JSON.stringify(savedDrafts))
}

function isConversationDrafts(value: unknown): value is ConversationDrafts {
  return typeof value === 'object'
    && value !== null
    && !Array.isArray(value)
    && Object.values(value).every((draft) => typeof draft === 'string')
}
