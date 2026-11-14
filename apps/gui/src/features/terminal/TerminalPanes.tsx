import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { Columns2, GripVertical, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { StatusMark } from '@/components/StatusMark'
import type { Conversation } from '@/data/preview'
import {
  addPane,
  closePane,
  movePaneFocus,
  placeConversation,
  swapPanes,
  MAX_PANES,
  type Pane,
} from '@/features/terminal/panes'

/**
 * Native terminals side by side. Closing a pane only stops watching it: the
 * daemon owns every session, so nothing here can stop an agent.
 */
export function TerminalPanes({
  conversations,
  selectedId,
  onSelect,
  renderTerminal,
  primaryModifier,
}: {
  conversations: readonly Conversation[]
  selectedId: string
  onSelect: (id: string) => void
  renderTerminal: (conversation: Conversation) => ReactNode
  primaryModifier: string
}) {
  const [panes, setPanes] = useState<Pane[]>(() => [
    { id: crypto.randomUUID(), conversationId: selectedId },
  ])
  const [focusedId, setFocusedId] = useState(() => panes[0]?.id ?? '')
  const [dragId, setDragId] = useState<string | null>(null)
  const [dropId, setDropId] = useState<string | null>(null)
  const stateRef = useRef({ panes, focusedId, conversations, selectedId })
  useEffect(() => {
    stateRef.current = { panes, focusedId, conversations, selectedId }
  })

  // Choosing a session elsewhere (tabs, sidebar) shows it here: focus its pane,
  // or load it into the focused pane.
  useEffect(() => {
    const { panes: current, focusedId: focused } = stateRef.current
    const shown = current.find((pane) => pane.conversationId === selectedId)
    if (shown) {
      if (shown.id !== focused) setFocusedId(shown.id)
      return
    }
    setPanes(placeConversation(current, focused, selectedId))
  }, [selectedId])

  const byId = new Map(conversations.map((item) => [item.id, item]))
  const visible = panes.filter((pane) => byId.has(pane.conversationId))
  const unused = conversations.filter(
    (item) => !panes.some((pane) => pane.conversationId === item.id),
  )

  const split = useCallback(() => {
    const { panes: current, conversations: all } = stateRef.current
    const next = addPane(current, all)
    if (next === current) return
    setPanes(next)
    const added = next[next.length - 1]
    if (added) setFocusedId(added.id)
  }, [])

  const focusPane = useCallback(
    (paneId: string) => {
      setFocusedId(paneId)
      const pane = stateRef.current.panes.find((item) => item.id === paneId)
      if (pane && pane.conversationId !== stateRef.current.selectedId)
        onSelect(pane.conversationId)
    },
    [onSelect],
  )

  useEffect(() => {
    // Captured on the window so xterm never sees the few keys Maitty keeps.
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.shiftKey || event.altKey)
        return
      const key = event.key.toLowerCase()
      if (key === 'd') {
        event.preventDefault()
        event.stopPropagation()
        split()
      } else if (key === '[' || key === ']') {
        event.preventDefault()
        event.stopPropagation()
        const { panes: current, focusedId: focused } = stateRef.current
        const target = movePaneFocus(current, focused, key === ']' ? 1 : -1)
        if (target) focusPane(target)
      }
    }
    window.addEventListener('keydown', onKeyDown, true)
    return () => {
      window.removeEventListener('keydown', onKeyDown, true)
    }
  }, [split, focusPane])

  function assign(paneId: string, conversationId: string) {
    const existing = panes.find(
      (pane) => pane.conversationId === conversationId,
    )
    setPanes(
      existing && existing.id !== paneId
        ? swapPanes(panes, paneId, existing.id)
        : placeConversation(panes, paneId, conversationId),
    )
    if (paneId === focusedId) onSelect(conversationId)
  }

  function close(paneId: string) {
    const next = closePane(panes, paneId)
    setPanes(next)
    if (paneId === focusedId && next[0]) focusPane(next[0].id)
  }

  return (
    <div className="terminal-panes">
      <div className="terminal-panes-bar">
        <span>
          Keys go straight to the focused agent. Closing a pane never stops it.
        </span>
        <Button
          variant="soft"
          size="sm"
          onClick={split}
          disabled={panes.length >= MAX_PANES || unused.length === 0}
          aria-label="Split terminal"
        >
          <Columns2 size={14} />
          Split <kbd>{primaryModifier} D</kbd>
        </Button>
      </div>
      <div className={`terminal-grid panes-${String(visible.length)}`}>
        {visible.map((pane) => {
          const conversation = byId.get(pane.conversationId)
          if (!conversation) return null
          const focused = visible.length > 1 && pane.id === focusedId
          return (
            <section
              key={pane.id}
              className={`terminal-pane ${focused ? 'is-focused' : ''} ${dropId === pane.id ? 'is-drop-target' : ''}`}
              aria-label={`Terminal for ${conversation.title}`}
              onMouseDownCapture={() => {
                focusPane(pane.id)
              }}
              onDragOver={(event) => {
                if (dragId && dragId !== pane.id) {
                  event.preventDefault()
                  setDropId(pane.id)
                }
              }}
              onDragLeave={() => {
                setDropId((current) => (current === pane.id ? null : current))
              }}
              onDrop={(event) => {
                event.preventDefault()
                if (dragId && dragId !== pane.id)
                  setPanes(swapPanes(panes, dragId, pane.id))
                setDragId(null)
                setDropId(null)
              }}
            >
              <header
                className="terminal-pane-header"
                draggable
                onDragStart={(event) => {
                  event.dataTransfer.setData('text/plain', pane.id)
                  event.dataTransfer.effectAllowed = 'move'
                  setDragId(pane.id)
                }}
                onDragEnd={() => {
                  setDragId(null)
                  setDropId(null)
                }}
              >
                <GripVertical size={14} aria-hidden="true" />
                <StatusMark status={conversation.status} />
                <select
                  aria-label="Session in this pane"
                  value={conversation.id}
                  onChange={(event) => {
                    assign(pane.id, event.target.value)
                  }}
                >
                  {conversations.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.title}
                    </option>
                  ))}
                </select>
                <code className="terminal-pane-path">
                  {conversation.folder}
                </code>
                {focused && (
                  <span className="terminal-pane-focus">Focused</span>
                )}
                <button
                  type="button"
                  className="terminal-pane-close"
                  aria-label={`Close pane for ${conversation.title}`}
                  disabled={panes.length <= 1}
                  onClick={() => {
                    close(pane.id)
                  }}
                >
                  <X size={13} />
                </button>
              </header>
              <div className="terminal-pane-body">
                {renderTerminal(conversation)}
              </div>
            </section>
          )
        })}
      </div>
    </div>
  )
}
