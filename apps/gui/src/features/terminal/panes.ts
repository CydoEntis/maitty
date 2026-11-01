import type { Conversation } from '@/data/preview'

export const MAX_PANES = 4

export type Pane = { id: string; conversationId: string }

/** Show `conversationId` in `paneId`, leaving every other pane as it is. */
export function placeConversation(
  panes: readonly Pane[],
  paneId: string,
  conversationId: string,
): Pane[] {
  return panes.map((pane) =>
    pane.id === paneId ? { ...pane, conversationId } : pane,
  )
}

/** Exchange the sessions shown in two panes; each pane keeps its position. */
export function swapPanes(
  panes: readonly Pane[],
  firstId: string,
  secondId: string,
): Pane[] {
  const first = panes.find((pane) => pane.id === firstId)
  const second = panes.find((pane) => pane.id === secondId)
  if (!first || !second) return [...panes]
  return panes.map((pane) =>
    pane.id === firstId
      ? { ...pane, conversationId: second.conversationId }
      : pane.id === secondId
        ? { ...pane, conversationId: first.conversationId }
        : pane,
  )
}

/**
 * Add a pane showing a session no pane shows yet, preferring one that is
 * working. Returns the same array when nothing can be added.
 */
export function addPane(
  panes: readonly Pane[],
  conversations: readonly Conversation[],
): Pane[] {
  if (panes.length >= MAX_PANES) return panes as Pane[]
  const shown = new Set(panes.map((pane) => pane.conversationId))
  const candidates = conversations.filter((item) => !shown.has(item.id))
  const choice =
    candidates.find((item) => item.status === 'working') ?? candidates[0]
  if (!choice) return panes as Pane[]
  return [...panes, { id: crypto.randomUUID(), conversationId: choice.id }]
}

/** Remove a pane, never the last one. */
export function closePane(panes: readonly Pane[], paneId: string): Pane[] {
  if (panes.length <= 1) return [...panes]
  return panes.filter((pane) => pane.id !== paneId)
}

/** The pane `step` positions from the focused one, wrapping around. */
export function movePaneFocus(
  panes: readonly Pane[],
  focusedId: string,
  step: 1 | -1,
): string | null {
  if (panes.length === 0) return null
  const index = panes.findIndex((pane) => pane.id === focusedId)
  const next = panes[(Math.max(index, 0) + step + panes.length) % panes.length]
  return next?.id ?? null
}
