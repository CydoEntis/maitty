const STORAGE_KEY = 'maitty.gui.selection.v1'

export function readSelectedConversation(): string | null {
  const conversationId = localStorage.getItem(STORAGE_KEY)
  if (conversationId === null) return null
  if (conversationId.length > 0) return conversationId

  console.warn('Ignoring empty Maitty conversation selection')
  return null
}

export function writeSelectedConversation(conversationId: string): void {
  if (conversationId.length === 0) return
  localStorage.setItem(STORAGE_KEY, conversationId)
}
