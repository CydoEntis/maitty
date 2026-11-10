import type { Dispatch, SetStateAction } from 'react'
import type { QueryClient } from '@tanstack/react-query'
import type { SidebarPreferences } from '@/data/sidebarPreferences'
import { toast } from '@/lib/toast'
import { archiveConversationOnDaemon, endConversation as endDaemonConversation, renameConversationOnDaemon } from '@/api/daemon'
import type { Conversation } from '@/data/preview'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { errorMessage } from '@/lib/error-message'

/** What the sidebar does to a conversation: pin, rename, close, end. The daemon keeps each change; only the browser preview keeps it itself. */
export function useConversationActions({ setSidebarPreferences, settingsPreferences, setSidebarCollapsed, setSettingsPreferences, desktop, queryClient, conversations, openTabIds, closeConversation, setEndTarget, endTarget }: {
  setSidebarPreferences: Dispatch<SetStateAction<SidebarPreferences>>
  settingsPreferences: SettingsPreferences
  setSidebarCollapsed: Dispatch<SetStateAction<boolean>>
  setSettingsPreferences: Dispatch<SetStateAction<SettingsPreferences>>
  desktop: boolean
  queryClient: QueryClient
  conversations: Conversation[]
  openTabIds: string[]
  closeConversation: (conversationId: string) => void
  setEndTarget: Dispatch<SetStateAction<Conversation | null>>
  endTarget: Conversation | null
}) {


  function togglePin(conversationId: string) {
    setSidebarPreferences((current) => ({
      ...current,
      pinnedIds: current.pinnedIds.includes(conversationId)
        ? current.pinnedIds.filter((id) => id !== conversationId)
        : [...current.pinnedIds, conversationId],
    }))
  }

  function updateSettingsPreferences(preferences: SettingsPreferences) {
    if (preferences.compactSidebar !== settingsPreferences.compactSidebar) {
      setSidebarCollapsed(preferences.compactSidebar)
    }
    setSettingsPreferences(preferences)
  }

  // Close or reopen sessions for every client: the daemon keeps the flag. Only the browser
  // preview, which has no daemon, remembers it itself.
  function putAway(ids: string[], archived: boolean) {
    if (!desktop) {
      setSidebarPreferences((current) => ({
        ...current,
        archivedIds: archived
          ? [...new Set([...current.archivedIds, ...ids])]
          : current.archivedIds.filter((id) => !ids.includes(id)),
      }))
      return
    }
    void Promise.all(
      ids.map((id) => archiveConversationOnDaemon({ id }, archived)),
    )
      .catch((error: unknown) => {
        toast.error(
          error instanceof Error
            ? error.message
            : 'Could not change this session',
        )
      })
      .finally(() => {
        void queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
  }

  // Closing a session ends it: a chat asks first (its agent stops), a terminal session closes
  // its pane. There is no archive; a closed session leaves the list.
  function setArchived(conversationId: string, archived: boolean) {
    if (!archived) return
    const conversation = conversations.find(
      (item) => item.id === conversationId,
    )
    if (conversation?.kind !== 'terminal') {
      requestEndSession(conversationId)
      return
    }
    putAway([conversationId], true)
    if (openTabIds.includes(conversationId) && openTabIds.length > 1)
      closeConversation(conversationId)
    toast.message('Terminal session closed', {
      description:
        'Its program was stopped, as when it is closed in the terminal app.',
    })
  }

  function requestEndSession(conversationId: string) {
    const conversation = conversations.find(
      (item) => item.id === conversationId,
    )
    if (conversation) setEndTarget(conversation)
  }

  function confirmEndSession() {
    const target = endTarget
    if (!target) return
    setEndTarget(null)
    if (!desktop) {
      putAway([target.id], true)
      if (openTabIds.includes(target.id) && openTabIds.length > 1)
        closeConversation(target.id)
      return
    }
    void endDaemonConversation(target)
      .then(() => {
        // The agent is stopped; move the session out of the way, like an archived one.
        putAway([target.id], true)
        if (openTabIds.includes(target.id) && openTabIds.length > 1)
          closeConversation(target.id)
        void queryClient.invalidateQueries({ queryKey: ['conversations'] })
        toast.message('Session ended', {
          description:
            'The agent is stopped. Its history stays under Archived. Start a new session to continue with different permissions.',
        })
      })
      .catch((error: unknown) => {
        toast.error(errorMessage(error, 'Could not end this session'))
      })
  }

  function renameConversation(conversationId: string, title: string) {
    const cleanTitle = title.trim()
    if (!cleanTitle) {
      toast.error('Conversation name cannot be empty')
      return
    }
    if (desktop) {
      void renameConversationOnDaemon({ id: conversationId }, cleanTitle)
        .catch((error: unknown) => {
          toast.error(
            error instanceof Error ? error.message : 'Could not rename it',
          )
        })
        .finally(() => {
          void queryClient.invalidateQueries({ queryKey: ['conversations'] })
        })
      return
    }
    setSidebarPreferences((current) => ({
      ...current,
      renamedTitles: { ...current.renamedTitles, [conversationId]: cleanTitle },
    }))
  }
  return { updateSettingsPreferences, togglePin, setArchived, renameConversation, requestEndSession, confirmEndSession }
}
