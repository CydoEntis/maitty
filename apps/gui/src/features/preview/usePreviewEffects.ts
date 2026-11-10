import { type Dispatch, type RefObject, type SetStateAction, useEffect, useRef } from 'react'
import type { QueryClient, UseQueryResult } from '@tanstack/react-query'
import { planPreferenceMigration, type SidebarPreferences, writeSidebarPreferences } from '@/data/sidebarPreferences'
import { type ThemePreference, writeThemePreference } from '@/data/themePreference'
import { toast } from '@/lib/toast'
import { showAttentionToast, showFinishedToast } from '@/features/notifications/backgroundToasts'
import { setSpinnerStyle } from '@/lib/spinner-clock'
import { archiveConversationOnDaemon, renameConversationOnDaemon } from '@/api/daemon'
import type { Conversation, SessionStatus } from '@/data/preview'
import { writeConversationDrafts } from '@/data/conversationDrafts'
import { findConversationNotifications, findConversationStatusAnnouncements } from '@/data/conversationNotifications'
import { type SettingsPreferences, writeSettingsPreferences } from '@/features/settings/preferences'
import { playNotificationSound } from '@/data/notificationSound'
import { writeSelectedConversation } from '@/data/selectedConversation'
import { writeOpenConversationTabs } from '@/data/openConversationTabs'
import { APPROVAL_COMPACT_HEIGHT_PX, COMPACT_SIDEBAR_MAX_WIDTH_PX, type PanelTab } from './shared'

/** What the preview app keeps in step as its state changes: saved preferences, drafts and open tabs, status announcements, the theme and the window width. */
export function usePreviewEffects({ desktop, sessionsQuery, sidebarPreferences, setSidebarPreferences, queryClient, drafts, selectedConversationId, openTabIds, daemonDisconnected, previousConversationStatuses, activeConversations, setStatusAnnouncement, selected, selectConversation, settingsPreferences, setActivePanel, theme, setApprovalCollapsed, setIsCompactSidebar, setSidebarDrawerOpen }: {
  desktop: boolean
  sessionsQuery: UseQueryResult<Conversation[]>
  sidebarPreferences: SidebarPreferences
  setSidebarPreferences: Dispatch<SetStateAction<SidebarPreferences>>
  queryClient: QueryClient
  drafts: Record<string, string>
  selectedConversationId: string | undefined
  openTabIds: string[]
  daemonDisconnected: boolean
  previousConversationStatuses: RefObject<Map<string, SessionStatus>>
  activeConversations: Conversation[]
  setStatusAnnouncement: Dispatch<SetStateAction<string>>
  selected: Conversation | undefined
  selectConversation: (conversationId: string) => void
  settingsPreferences: SettingsPreferences
  setActivePanel: Dispatch<SetStateAction<PanelTab | null>>
  theme: ThemePreference
  setApprovalCollapsed: Dispatch<SetStateAction<boolean>>
  setIsCompactSidebar: Dispatch<SetStateAction<boolean>>
  setSidebarDrawerOpen: Dispatch<SetStateAction<boolean>>
}) {


  // Once per start: hand the names and closed sessions this browser used to keep to the daemon,
  // so every client shows them. A failure keeps them here and tries again next start.
  const preferencesMigrated = useRef(false)
  useEffect(() => {
    if (!desktop || preferencesMigrated.current || !sessionsQuery.isSuccess)
      return
    preferencesMigrated.current = true
    const plan = planPreferenceMigration(sidebarPreferences, sessionsQuery.data)
    const hasLocal =
      sidebarPreferences.archivedIds.length > 0 ||
      Object.keys(sidebarPreferences.renamedTitles).length > 0
    if (!hasLocal) return
    void Promise.all([
      ...plan.archive.map((id) => archiveConversationOnDaemon({ id }, true)),
      ...plan.rename.map(([id, title]) =>
        renameConversationOnDaemon({ id }, title),
      ),
    ])
      .then(() => {
        setSidebarPreferences(plan.remaining)
        return queryClient.invalidateQueries({ queryKey: ['conversations'] })
      })
      .catch((error: unknown) => {
        console.warn('Could not move saved names to the daemon', error)
      })
  }, [desktop, queryClient, sessionsQuery.data, sessionsQuery.isSuccess, setSidebarPreferences, sidebarPreferences])

  useEffect(() => {
    try {
      writeSidebarPreferences(sidebarPreferences)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save sidebar preferences: ${error.message}`
          : 'Could not save sidebar preferences',
      )
    }
  }, [sidebarPreferences])

  useEffect(() => {
    try {
      writeConversationDrafts(drafts)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save conversation drafts: ${error.message}`
          : 'Could not save conversation drafts',
      )
    }
  }, [drafts])

  useEffect(() => {
    if (!selectedConversationId) return
    try {
      writeSelectedConversation(selectedConversationId)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save selected conversation: ${error.message}`
          : 'Could not save selected conversation',
      )
    }
  }, [selectedConversationId])

  useEffect(() => {
    try {
      writeOpenConversationTabs(openTabIds)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save open conversation tabs: ${error.message}`
          : 'Could not save open conversation tabs',
      )
    }
  }, [openTabIds])

  useEffect(() => {
    if (!desktop) return
    if (daemonDisconnected) {
      previousConversationStatuses.current.clear()
      return
    }
    if (!sessionsQuery.data) return

    const statuses = previousConversationStatuses.current
    const announcements =
      statuses.size === 0
        ? []
        : findConversationStatusAnnouncements(statuses, activeConversations)
    if (announcements.length > 0) {
      setStatusAnnouncement(
        announcements.map(({ title, kind }) => `${kind}: ${title}`).join('. '),
      )
    }
    const notifications =
      statuses.size === 0
        ? []
        : findConversationNotifications(
            statuses,
            activeConversations,
            selected?.id,
          )

    for (const notification of notifications) {
      const conversation = activeConversations.find(
        (item) => item.id === notification.conversationId,
      )
      const open = () => {
        selectConversation(notification.conversationId)
      }
      if (notification.kind === 'attention') {
        if (!settingsPreferences.notifyNeedsYou) continue
        if (conversation) {
          void showAttentionToast(conversation, {
            open,
            answered: () => {
              void queryClient.invalidateQueries({
                queryKey: ['conversations'],
              })
            },
          })
        } else {
          toast.warning(`${notification.title} needs your attention`, {
            action: { label: 'Open', onClick: open },
          })
        }
      } else {
        if (!settingsPreferences.notifyFinished) continue
        if (conversation) {
          void showFinishedToast(conversation, {
            open,
            review: () => {
              open()
              setActivePanel('diff')
            },
          })
        } else {
          toast.success(`${notification.title} is done`, {
            action: { label: 'Open', onClick: open },
          })
        }
      }
      if (settingsPreferences.notificationSound) playNotificationSound()
    }

    statuses.clear()
    for (const conversation of activeConversations)
      statuses.set(conversation.id, conversation.status)
  }, [activeConversations, daemonDisconnected, desktop, selected?.id, sessionsQuery.data, settingsPreferences.notificationSound, settingsPreferences.notifyFinished, settingsPreferences.notifyNeedsYou, selectConversation, queryClient, previousConversationStatuses, setStatusAnnouncement, setActivePanel])

  useEffect(() => {
    try {
      writeSettingsPreferences(settingsPreferences)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save notification preferences: ${error.message}`
          : 'Could not save notification preferences',
      )
    }
  }, [settingsPreferences])

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    document.documentElement.classList.toggle('dark', theme !== 'day')
    try {
      writeThemePreference(theme)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Could not save theme preference: ${error.message}`
          : 'Could not save theme preference',
      )
    }
  }, [theme])

  useEffect(() => {
    document.documentElement.dataset.accent = settingsPreferences.accent
    document.documentElement.dataset.reducedMotion = String(
      settingsPreferences.reduceMotion,
    )
    document.documentElement.dataset.sidebarActivity =
      settingsPreferences.animateSidebarActivity ? 'on' : 'off'
    setSpinnerStyle(settingsPreferences.spinner)
  }, [
    settingsPreferences.spinner,
    settingsPreferences.accent,
    settingsPreferences.animateSidebarActivity,
    settingsPreferences.reduceMotion,
  ])

  useEffect(() => {
    const updateApprovalLayout = () => {
      setApprovalCollapsed(window.innerHeight < APPROVAL_COMPACT_HEIGHT_PX)
    }
    updateApprovalLayout()
    window.addEventListener('resize', updateApprovalLayout)
    return () => {
      window.removeEventListener('resize', updateApprovalLayout)
    }
  }, [setApprovalCollapsed])

  useEffect(() => {
    const updateSidebarMode = () => {
      const compact = window.innerWidth <= COMPACT_SIDEBAR_MAX_WIDTH_PX
      setIsCompactSidebar(compact)
      if (!compact) setSidebarDrawerOpen(false)
    }
    updateSidebarMode()
    window.addEventListener('resize', updateSidebarMode)
    return () => {
      window.removeEventListener('resize', updateSidebarMode)
    }
  }, [setIsCompactSidebar, setSidebarDrawerOpen])
}
