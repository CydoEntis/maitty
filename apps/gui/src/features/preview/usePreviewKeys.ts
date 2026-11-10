import { type Dispatch, type SetStateAction, useEffect, useRef } from 'react'
import { toast } from '@/lib/toast'
import type { Conversation, Message } from '@/data/preview'
import type { PanelTab } from './shared'

/** The preview app's keyboard shortcuts. */
export function usePreviewKeys({ setCommandDialogOpen, openTabIds, selected, selectConversation, closeConversation, setSettingsDialogOpen, openNewSession, activePanel, setActivePanel, sidebarDrawerOpen, setSidebarDrawerOpen, isCompactSidebar, setSidebarCollapsed, setKeyboardDialogOpen, pendingApproval, daemonDisconnected, desktop, toggleTerminal, cancelTurn, activeConversations }: {
  setCommandDialogOpen: Dispatch<SetStateAction<boolean>>
  openTabIds: string[]
  selected: Conversation | undefined
  selectConversation: (conversationId: string) => void
  closeConversation: (conversationId: string) => void
  setSettingsDialogOpen: Dispatch<SetStateAction<boolean>>
  openNewSession: () => void
  activePanel: PanelTab | null
  setActivePanel: Dispatch<SetStateAction<PanelTab | null>>
  sidebarDrawerOpen: boolean
  setSidebarDrawerOpen: Dispatch<SetStateAction<boolean>>
  isCompactSidebar: boolean
  setSidebarCollapsed: Dispatch<SetStateAction<boolean>>
  setKeyboardDialogOpen: Dispatch<SetStateAction<boolean>>
  pendingApproval: Message | undefined
  daemonDisconnected: boolean
  desktop: boolean
  toggleTerminal: () => void
  cancelTurn: () => void
  activeConversations: Conversation[]
}) {
  // The window listens once; what a key does is read from the latest render.
  const handler = useRef<(event: globalThis.KeyboardEvent) => void>(() => undefined)
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      const hasModifier = event.ctrlKey || event.metaKey
      const key = event.key.toLowerCase()
      const target = event.target instanceof HTMLElement ? event.target : null
      const inTerminal = Boolean(target?.closest('.terminal-screen'))
      const inDialog = Boolean(target?.closest('[role="dialog"]'))
      const inTextEntry =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        Boolean(target?.isContentEditable)

      if (hasModifier && key === 'k') {
        event.preventDefault()
        setCommandDialogOpen(true)
        return
      }
      if (inDialog) return
      if (inTerminal && !(hasModifier && key === '`')) return
      if (hasModifier && event.key === 'Tab' && openTabIds.length > 1) {
        event.preventDefault()
        const currentIndex = openTabIds.indexOf(selected?.id ?? '')
        const direction = event.shiftKey ? -1 : 1
        const nextId =
          openTabIds[
            (currentIndex + direction + openTabIds.length) % openTabIds.length
          ]
        if (nextId) selectConversation(nextId)
        return
      }
      if (hasModifier && /^\d$/.test(event.key)) {
        const tabId = openTabIds[Number(event.key) - 1]
        if (tabId) {
          event.preventDefault()
          selectConversation(tabId)
        }
        return
      }
      if (hasModifier && key === 'w' && selected) {
        event.preventDefault()
        closeConversation(selected.id)
        return
      }
      if (hasModifier && event.key === ',') {
        event.preventDefault()
        setSettingsDialogOpen(true)
        return
      }
      if (hasModifier && key === 'o') {
        event.preventDefault()
        openNewSession()
        return
      }
      if (event.key === 'Escape' && !inTerminal) {
        if (activePanel) {
          event.preventDefault()
          setActivePanel(null)
        } else if (sidebarDrawerOpen) {
          event.preventDefault()
          setSidebarDrawerOpen(false)
        }
        return
      }
      if (hasModifier && key === '\\') {
        event.preventDefault()
        if (isCompactSidebar) setSidebarDrawerOpen((open) => !open)
        else setSidebarCollapsed((collapsed) => !collapsed)
        return
      }
      if (
        event.key === '?' &&
        !(target instanceof HTMLInputElement) &&
        !(target instanceof HTMLTextAreaElement) &&
        !target?.isContentEditable
      ) {
        event.preventDefault()
        setKeyboardDialogOpen(true)
        return
      }
      if (!hasModifier && !inTextEntry && !inTerminal) {
        const useAltArrow = event.altKey && !event.shiftKey
        const useJk = !event.altKey && !event.shiftKey
        const direction =
          (useAltArrow && event.key === 'ArrowDown') || (useJk && key === 'j')
            ? 1
            : (useAltArrow && event.key === 'ArrowUp') || (useJk && key === 'k')
              ? -1
              : 0
        if (direction !== 0) {
          const rows = Array.from(
            document.querySelectorAll<HTMLButtonElement>(
              '.session-list-scroll button.session-row',
            ),
          )
          if (rows.length === 0) return
          const currentIndex = rows.findIndex(
            (row) => row.getAttribute('aria-current') === 'page',
          )
          const nextIndex =
            currentIndex < 0
              ? direction > 0
                ? 0
                : rows.length - 1
              : (currentIndex + direction + rows.length) % rows.length
          const nextRow = rows[nextIndex]
          if (!nextRow) return
          event.preventDefault()
          nextRow.click()
          return
        }
        if (
          pendingApproval &&
          !daemonDisconnected &&
          pendingApproval.approvalKind !== 'UserInput'
        ) {
          const choices =
            pendingApproval.approvalChoices ??
            (desktop ? [] : ['Decline', 'Accept', 'AcceptForSession', 'Cancel'])
          const decision =
            event.shiftKey && key === 'n'
              ? 'Cancel'
              : !event.shiftKey && (key === 'y' || key === 'a')
                ? 'Accept'
                : !event.shiftKey && key === 'n'
                  ? 'Decline'
                  : null
          if (decision && choices.includes(decision)) {
            const action = document.querySelector<HTMLButtonElement>(
              `.approval-dock [data-approval-choice="${decision}"]`,
            )
            if (action && !action.disabled) {
              event.preventDefault()
              action.click()
              return
            }
          }
        }
      }
      if (!hasModifier) return

      if (key === 'n') {
        if (daemonDisconnected) return
        event.preventDefault()
        openNewSession()
      } else if (event.shiftKey && key === 'd') {
        event.preventDefault()
        setActivePanel((panel) => (panel === 'diff' ? null : 'diff'))
      } else if (key === 'i') {
        event.preventDefault()
        setActivePanel((panel) => (panel === 'details' ? null : 'details'))
      } else if (key === '`') {
        event.preventDefault()
        toggleTerminal()
      } else if (
        key === '.' &&
        selected &&
        (selected.status === 'working' || selected.status === 'needs-you')
      ) {
        event.preventDefault()
        cancelTurn()
      } else if (event.shiftKey && key === 'a' && !daemonDisconnected) {
        const blocked = activeConversations.filter(
          (conversation) =>
            conversation.status === 'needs-you' ||
            conversation.status === 'recovery',
        )
        if (blocked.length === 0) {
          toast.message('No conversations need your attention')
          return
        }
        const firstBlocked = blocked[0]
        if (!firstBlocked) return
        event.preventDefault()
        const selectedIndex = blocked.findIndex(
          (conversation) => conversation.id === selected?.id,
        )
        selectConversation(
          blocked[(selectedIndex + 1) % blocked.length]?.id ?? firstBlocked.id,
        )
      }
    }
    handler.current = onKeyDown
  })
  useEffect(() => {
    const listen = (event: globalThis.KeyboardEvent) => { handler.current(event) }
    window.addEventListener('keydown', listen)
    return () => { window.removeEventListener('keydown', listen) }
  }, [])
}
