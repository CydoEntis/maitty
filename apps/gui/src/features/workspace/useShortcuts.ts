import { type Dispatch, type SetStateAction, useCallback, useEffect, useRef } from 'react'
import { matchShortcut, NEW_CHAT_PAGE_ACTIONS, type Platform, type ShortcutAction } from './model/shortcuts'
import type { ChatAgent, ChatAvailability, PaneView } from './model/chatView'
import { toast } from '@/lib/toast'
import { defaultSessionFolder } from '@/api/daemon'
import { LOCAL_MACHINE } from '@/api/machines'
import type { DesktopSettingsSection } from '@/features/settings/components/DesktopSettings'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { guardLocal } from '@/lib/localOnly'
import { isChatSession } from './model/chatSession'
import { usePaneViews } from './usePaneViews'
import { activeArrangement, cycleFocus, type Layout, type SplitDirection, toggleBroadcast } from './model/layout'
import type { SessionView } from './model/session'
import { type EscLayer, topmostOpen } from './model/escLadder'
import { jumpToast, nextNeedsYou, NOTHING_NEEDS_YOU } from '@/features/needs-you/model/needsYou'
import { resolveStartFolder } from './model/startFolder'

/** Listens for the keyboard: a shortcut runs its action, and Esc closes the topmost layer. */
export function useShortcutKeys({ runShortcut, canUndoClose, dialogOpen, onNewChatPage, overlayOpen, paletteOpen, inboxOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen, setPaletteOpen, setInboxOpen, setSettingsSection, setNewChatOpen, setDrawerOpen, setFocusMode, platform }: {
  runShortcut: (action: ShortcutAction) => void
  canUndoClose: boolean
  dialogOpen: boolean
  onNewChatPage: boolean
  overlayOpen: boolean
  paletteOpen: boolean
  inboxOpen: boolean
  focusMode: boolean
  drawerShown: boolean
  welcomeOpen: boolean
  shortcutsOpen: boolean
  setPaletteOpen: Dispatch<SetStateAction<boolean>>
  setInboxOpen: Dispatch<SetStateAction<boolean>>
  setSettingsSection: Dispatch<SetStateAction<DesktopSettingsSection | null>>
  setNewChatOpen: Dispatch<SetStateAction<boolean>>
  setDrawerOpen: Dispatch<SetStateAction<boolean>>
  setFocusMode: Dispatch<SetStateAction<boolean>>
  platform: Platform
}) {
  const shortcutState = useRef({ runShortcut, canUndoClose: () => canUndoClose, settingsOpen: dialogOpen, onNewChatPage, overlayOpen, paletteOpen, inboxOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen })
  useEffect(() => { shortcutState.current = { runShortcut, canUndoClose: () => canUndoClose, settingsOpen: dialogOpen, onNewChatPage, overlayOpen, paletteOpen, inboxOpen, focusMode, drawerShown, welcomeOpen, shortcutsOpen } })
  useEffect(() => {
    // How each layer of the Esc ladder closes. A new dialog adds its closer here and its id to ESC_LADDER.
    const closers: Partial<Record<EscLayer, () => void>> = {
      palette: () => { setPaletteOpen(false) },
      inbox: () => { setInboxOpen(false) },
      // The new chat page counts as a dialog here, so this layer is the one that closes it too.
      settings: () => { setSettingsSection(null); setNewChatOpen(false) },
      drawer: () => { setDrawerOpen(false) },
      'focus-mode': () => { setFocusMode(false) },
    }
    const onKeyDown = (event: KeyboardEvent) => {
      const state = shortcutState.current
      if (state.welcomeOpen) return
      const match = matchShortcut(event, platform)
      if (state.shortcutsOpen) {
        // Only its own key closes it; Esc is the dialog's.
        if (match?.action === 'shortcuts') {
          event.preventDefault()
          event.stopPropagation()
          state.runShortcut('shortcuts')
        }
        return
      }
      if (event.key === 'Escape') {
        // A menu or list that is open (the spinner picker in Settings) takes Esc itself.
        if (event.target instanceof Element && event.target.closest('[role="menu"]')) return
        const top = topmostOpen({ palette: state.paletteOpen, inbox: state.inboxOpen, settings: state.settingsOpen, drawer: state.drawerShown, 'focus-mode': state.focusMode })
        if (top === null) return
        // What Esc closes is not also sent on to the terminal, where it would interrupt an agent.
        event.preventDefault()
        event.stopPropagation()
        closers[top]?.()
        return
      }
      if (state.overlayOpen) {
        if (match?.action === 'palette' || match?.action === 'inbox') {
          event.preventDefault()
          event.stopPropagation()
          state.runShortcut(match.action)
        }
        return
      }
      if (state.settingsOpen && !(state.onNewChatPage && match !== null && NEW_CHAT_PAGE_ACTIONS.has(match.action))) return
      if (!match) return
      // Undo is the field's own inside a text field, and the terminal's own when nothing was just closed.
      if (match.action === 'undo-close' && (!state.canUndoClose() || (event.target instanceof Element && event.target.closest('input, textarea, [contenteditable="true"]')))) return
      event.preventDefault()
      event.stopPropagation()
      state.runShortcut(match.action)
    }
    window.addEventListener('keydown', onKeyDown, true)
    return () => { window.removeEventListener('keydown', onKeyDown, true) }
  }, [platform, setDrawerOpen, setFocusMode, setInboxOpen, setNewChatOpen, setPaletteOpen, setSettingsSection])
}

/** What each shortcut does. The palette and the keyboard both go through it. */
export function useShortcutRunner({ newSession, focusedMachineNumber, newTerminal, setSettingsSection, startAgentSession, preferences, home, split, setLayout, layout, askToClose, endSessions, toggleSidebar, setPaletteOpen, setInboxOpen, sessions, focusedSession, flash, showSessionById, undoLastClose, chatOf, paneViews, setFocusMode, setShortcutsOpen, setTextSize, toggleInspect, setFindOpen, online, setWorktreeFrom }: {
  newSession: (machine?: number) => void
  focusedMachineNumber: number
  newTerminal: (machine?: number) => void
  setSettingsSection: Dispatch<SetStateAction<DesktopSettingsSection | null>>
  startAgentSession: (folder: () => Promise<string>, agent: ChatAgent, view: PaneView, firstMessage?: string, flags?: string, afterPlace?: (id: string) => void) => Promise<boolean>
  preferences: SettingsPreferences
  home: string
  split: (paneId: string | null, direction: SplitDirection) => void
  setLayout: Dispatch<SetStateAction<Layout>>
  layout: Layout
  askToClose: (ids: string[], close: () => void) => void
  endSessions: (ids: string[], what: string) => void
  toggleSidebar: () => void
  setPaletteOpen: Dispatch<SetStateAction<boolean>>
  setInboxOpen: Dispatch<SetStateAction<boolean>>
  sessions: SessionView[]
  focusedSession: SessionView | undefined
  flash: (text: string) => void
  showSessionById: (sessionId: string) => void
  undoLastClose: () => void
  chatOf: (session: SessionView) => { availability: ChatAvailability; view: PaneView }
  paneViews: ReturnType<typeof usePaneViews>
  setFocusMode: Dispatch<SetStateAction<boolean>>
  setShortcutsOpen: Dispatch<SetStateAction<boolean>>
  setTextSize: (size: number) => void
  toggleInspect: () => void
  setFindOpen: Dispatch<SetStateAction<boolean>>
  online: boolean
  setWorktreeFrom: Dispatch<SetStateAction<SessionView | null>>
}) {
  const runShortcut = useCallback((action: ShortcutAction) => {
    switch (action) {
      case 'new-session':
        newSession(focusedMachineNumber)
        return
      case 'new-terminal':
        newTerminal(focusedMachineNumber)
        return
      case 'new-agent-chat':
        // A chat with the default agent in the start folder, without the dialog. On another machine the dialog asks for its folder.
        setSettingsSection(null)
        if (focusedMachineNumber !== LOCAL_MACHINE) newSession(focusedMachineNumber)
        else void startAgentSession(async () => resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), preferences.defaultAgent, 'chat')
        return
      case 'split-right':
        split(null, 'right')
        return
      case 'split-down':
        split(null, 'down')
        return
      case 'focus-previous':
        setLayout((current) => cycleFocus(current, -1))
        return
      case 'focus-next':
        setLayout((current) => cycleFocus(current, 1))
        return
      case 'close': {
        const tab = activeArrangement(layout)
        if (!tab) return
        const single = tab.panes.length > 1
        const targets = single ? tab.panes.filter((pane) => pane.id === tab.focus) : tab.panes
        const ids = targets.map((pane) => pane.sessionId)
        askToClose(ids, () => { endSessions(ids, single ? 'Pane' : 'Session') })
        return
      }
      case 'toggle-sidebar':
        toggleSidebar()
        return
      case 'settings':
        setSettingsSection((current) => current ?? 'appearance')
        return
      case 'palette':
        setPaletteOpen((open) => !open)
        return
      case 'inbox':
        setInboxOpen((open) => !open)
        return
      case 'jump-needs': {
        const target = nextNeedsYou(sessions, focusedSession?.id ?? null)
        if (!target) flash(NOTHING_NEEDS_YOU)
        else {
          showSessionById(target.session.id)
          flash(jumpToast(target))
        }
        return
      }
      case 'performance':
        setSettingsSection('performance')
        return
      case 'undo-close':
        undoLastClose()
        return
      case 'usage':
        setSettingsSection('usage')
        return
      case 'chat-toggle': {
        // A chat session has only its chat; there is nothing to switch to.
        if (!focusedSession || isChatSession(focusedSession.conversation)) return
        const next: PaneView = chatOf(focusedSession).view === 'chat' ? 'terminal' : 'chat'
        paneViews.choose(focusedSession.id, next)
        return
      }
      case 'focus-mode':
        setFocusMode((on) => !on)
        return
      case 'shortcuts':
        setShortcutsOpen((open) => !open)
        return
      case 'text-larger':
        setTextSize(preferences.terminalFontSize + 1)
        return
      case 'text-smaller':
        setTextSize(preferences.terminalFontSize - 1)
        return
      case 'inspect':
        toggleInspect()
        return
      case 'find':
        setFindOpen((open) => !open)
        return
      case 'new-worktree': {
        const refusal = guardLocal(focusedSession, 'worktrees')
        if (refusal !== null) toast.message(refusal)
        else if (focusedSession && online) setWorktreeFrom(focusedSession)
        return
      }
      case 'broadcast':
        if (layout.activeId) {
          const turningOn = !activeArrangement(layout)?.broadcast
          setLayout((current) => toggleBroadcast(current, current.activeId ?? ''))
          flash(turningOn ? 'Broadcast on · typing goes to every idle pane in this tab' : 'Broadcast off')
        }
        return
    }
  }, [newSession, focusedMachineNumber, newTerminal, setSettingsSection, startAgentSession, preferences.defaultAgent, preferences.terminalFontSize, preferences.startFolder, split, setLayout, toggleSidebar, setPaletteOpen, setInboxOpen, undoLastClose, setFocusMode, setShortcutsOpen, setTextSize, toggleInspect, setFindOpen, layout, home, askToClose, endSessions, sessions, focusedSession, flash, showSessionById, chatOf, paneViews, online, setWorktreeFrom])
  return { runShortcut }
}
