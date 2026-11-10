import type { Dispatch, RefObject, SetStateAction } from 'react'
import type { QueryClient } from '@tanstack/react-query'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { toast } from '@/lib/toast'
import { createConversation as createDaemonConversation, defaultSessionFolder, loadConversations, type SessionPermissions, startShellSession, startTerminalSession } from '@/api/daemon'
import type { Conversation, Message } from '@/data/preview'
import { readConversationPermissions, rememberConversationPermissions } from '@/data/conversationPermissions'
import { errorMessage } from '@/lib/error-message'
import type { PanelTab } from './shared'

/** Starts a session: a shell at once in the desktop app, through the dialog in the browser preview. */
export function useConversationStarts({ desktop, setNewDialogOpen, daemonDisconnected, conversationCreationPendingRef, setConversationCreationPending, conversations, settingsPreferences, queryClient, selectConversation, newProject, newKind, newProvider, setStartingSessions, newPermissions, setStartedWith, setConversations, setMessagesById, setActivePanel }: {
  desktop: boolean
  setNewDialogOpen: Dispatch<SetStateAction<boolean>>
  daemonDisconnected: boolean
  conversationCreationPendingRef: RefObject<boolean>
  setConversationCreationPending: Dispatch<SetStateAction<boolean>>
  conversations: Conversation[]
  settingsPreferences: SettingsPreferences
  queryClient: QueryClient
  selectConversation: (conversationId: string) => void
  newProject: string
  newKind: "terminal" | "chat"
  newProvider: "Codex" | "Claude"
  setStartingSessions: Dispatch<SetStateAction<{ key: string; label: string; startedAt: number }[]>>
  newPermissions: SessionPermissions
  setStartedWith: Dispatch<SetStateAction<Record<string, SessionPermissions>>>
  setConversations: Dispatch<SetStateAction<Conversation[]>>
  setMessagesById: Dispatch<SetStateAction<Record<string, Message[]>>>
  setActivePanel: Dispatch<SetStateAction<PanelTab | null>>
}) {
  // A new session is a plain shell in the start folder; the user types what to run in it (ADR
  // 0046). The dialog remains only for the browser preview, which has no daemon to start sessions in.
  function openNewSession() {
    if (!desktop) {
      setNewDialogOpen(true)
      return
    }
    if (daemonDisconnected || conversationCreationPendingRef.current) return
    conversationCreationPendingRef.current = true
    setConversationCreationPending(true)
    const known = new Set(conversations.map((item) => item.id))
    const chosen = settingsPreferences.startFolder
    void (chosen ? Promise.resolve(chosen) : defaultSessionFolder())
      .then((folder) => startShellSession(folder))
      .then(async () => {
        await queryClient.invalidateQueries({ queryKey: ['conversations'] })
        const fresh = await queryClient.query({
          queryKey: ['conversations'],
          queryFn: loadConversations,
          staleTime: 0,
        })
        const created = fresh.find(
          (item) => item.kind === 'terminal' && !known.has(item.id),
        )
        if (created) selectConversation(created.id)
      })
      .catch((error: unknown) => {
        toast.error(errorMessage(error, 'Could not start a shell'))
      })
      .finally(() => {
        conversationCreationPendingRef.current = false
        setConversationCreationPending(false)
      })
  }

  function createConversation() {
    if (daemonDisconnected || conversationCreationPendingRef.current) return
    const typed = newProject.trim()
    if (typed) {
      startConversation(typed)
      return
    }
    // No folder chosen: start in the home folder and let the user move on from there (cd), as in the terminal app.
    void defaultSessionFolder()
      .then((folder) => {
        if (folder) startConversation(folder)
        else toast.error('Choose a project folder first')
      })
      .catch((error: unknown) => {
        toast.error(errorMessage(error, 'Could not find your home folder'))
      })
  }

  function startConversation(project: string) {
    if (daemonDisconnected || conversationCreationPendingRef.current) return
    if (desktop && newKind === 'terminal') {
      conversationCreationPendingRef.current = true
      setConversationCreationPending(true)
      const known = new Set(conversations.map((item) => item.id))
      void startTerminalSession(project, newProvider)
        .then(async () => {
          setNewDialogOpen(false)
          await queryClient.invalidateQueries({ queryKey: ['conversations'] })
          // Open the terminal session that just appeared.
          const fresh = await queryClient.query({
            queryKey: ['conversations'],
            queryFn: loadConversations,
            staleTime: 0,
          })
          const created = fresh.find(
            (item) => item.kind === 'terminal' && !known.has(item.id),
          )
          if (created) selectConversation(created.id)
        })
        .catch((error: unknown) => {
          toast.error(
            errorMessage(error, `Could not start ${newProvider} terminal`),
          )
        })
        .finally(() => {
          conversationCreationPendingRef.current = false
          setConversationCreationPending(false)
        })
      return
    }
    if (desktop) {
      conversationCreationPendingRef.current = true
      setConversationCreationPending(true)
      // The daemon starts the provider before it answers, which can take a few
      // seconds: close the dialog now and show progress in the sidebar instead.
      const startingKey = crypto.randomUUID()
      const folderName =
        project.split(/[\\/]/).filter(Boolean).at(-1) ?? project
      setStartingSessions((current) => [
        ...current,
        {
          key: startingKey,
          startedAt: Date.now(),
          label: `Starting ${newProvider} session in ${folderName}\u2026`,
        },
      ])
      setNewDialogOpen(false)
      void createDaemonConversation(project, newProvider, newPermissions)
        .then(async ({ id }) => {
          rememberConversationPermissions(String(id), newPermissions)
          setStartedWith(readConversationPermissions())
          selectConversation(String(id))
          await queryClient.invalidateQueries({ queryKey: ['conversations'] })
        })
        .catch((error: unknown) => {
          // Reopen the dialog with the same choices so nothing has to be retyped.
          setNewDialogOpen(true)
          toast.error(
            errorMessage(error, `Could not start ${newProvider} conversation`),
          )
        })
        .finally(() => {
          setStartingSessions((current) =>
            current.filter((item) => item.key !== startingKey),
          )
          conversationCreationPendingRef.current = false
          setConversationCreationPending(false)
        })
      return
    }
    const conversation: Conversation = {
      id: crypto.randomUUID(),
      title: `New ${newProvider} conversation`,
      project: project.split(/[\\/]/).filter(Boolean).at(-1) ?? 'project',
      folder: project,
      branch: 'working tree',
      provider: newProvider,
      status: 'idle',
      updated: 'now',
      model: `${newProvider} default`,
      permission: 'Read only · approvals on request',
      context: 0,
      nativePane: 0,
      hasDiff: false,
    }
    setConversations((current) => [conversation, ...current])
    setMessagesById((current) => ({ ...current, [conversation.id]: [] }))
    selectConversation(conversation.id)
    setActivePanel('details')
    setNewDialogOpen(false)
    toast.message('Created in local preview')
  }
  return { openNewSession, createConversation }
}
