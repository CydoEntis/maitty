import { describe, expect, it } from 'vitest'
import type { Conversation } from '@/data/preview'
import {
  MAX_PANES,
  addPane,
  closePane,
  movePaneFocus,
  placeConversation,
  swapPanes,
  type Pane,
} from './panes'

const conversation = (id: string, status: Conversation['status'] = 'idle') =>
  ({ id, status }) as Conversation

const panes: Pane[] = [
  { id: 'p1', conversationId: 'a' },
  { id: 'p2', conversationId: 'b' },
]

describe('terminal panes', () => {
  it('swaps the sessions shown in two panes and keeps pane positions', () => {
    expect(swapPanes(panes, 'p1', 'p2')).toEqual([
      { id: 'p1', conversationId: 'b' },
      { id: 'p2', conversationId: 'a' },
    ])
  })

  it('loads a session into one pane without touching the others', () => {
    expect(placeConversation(panes, 'p2', 'c')).toEqual([
      { id: 'p1', conversationId: 'a' },
      { id: 'p2', conversationId: 'c' },
    ])
  })

  it('splits onto a working session that no pane shows yet', () => {
    const next = addPane(panes, [
      conversation('a'),
      conversation('b'),
      conversation('c'),
      conversation('d', 'working'),
    ])
    expect(next).toHaveLength(3)
    expect(next[2]?.conversationId).toBe('d')
  })

  it('does not split when every session is shown or four panes are open', () => {
    expect(addPane(panes, [conversation('a'), conversation('b')])).toBe(panes)
    const full = Array.from({ length: MAX_PANES }, (_, index) => ({
      id: `p${String(index)}`,
      conversationId: `c${String(index)}`,
    }))
    expect(addPane(full, [conversation('extra')])).toBe(full)
  })

  it('never closes the last pane', () => {
    expect(closePane(panes, 'p1')).toEqual([panes[1]])
    expect(closePane([panes[0] as Pane], 'p1')).toEqual([panes[0]])
  })

  it('moves focus between panes and wraps around', () => {
    expect(movePaneFocus(panes, 'p1', 1)).toBe('p2')
    expect(movePaneFocus(panes, 'p2', 1)).toBe('p1')
    expect(movePaneFocus(panes, 'p1', -1)).toBe('p2')
  })
})
