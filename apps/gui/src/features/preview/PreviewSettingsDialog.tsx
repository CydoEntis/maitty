import type { Dispatch, SetStateAction } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import type { ThemePreference } from '@/data/themePreference'
import { toast } from '@/lib/toast'
import { showNotice } from '@/features/notifications/notify'
import type { Conversation } from '@/data/preview'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { SettingsDialog } from '@/features/settings/components/SettingsDialog'
import { enableNotificationSound, playNotificationSound } from '@/data/notificationSound'

/** The preview app's settings, with what it knows about the service and the agents installed. */
export function PreviewSettingsDialog({ settingsDialogOpen, setSettingsDialogOpen, theme, setTheme, settingsPreferences, updateSettingsPreferences, desktop, daemonDisconnected, sessionsQuery, codexExecutableQuery, claudeExecutableQuery, primaryModifier }: {
  settingsDialogOpen: boolean
  setSettingsDialogOpen: Dispatch<SetStateAction<boolean>>
  theme: ThemePreference
  setTheme: Dispatch<SetStateAction<ThemePreference>>
  settingsPreferences: SettingsPreferences
  updateSettingsPreferences: (preferences: SettingsPreferences) => void
  desktop: boolean
  daemonDisconnected: boolean
  sessionsQuery: UseQueryResult<Conversation[]>
  codexExecutableQuery: UseQueryResult<string>
  claudeExecutableQuery: UseQueryResult<string>
  primaryModifier: '⌘' | 'Ctrl'
}) {
  return (
    <SettingsDialog
      open={settingsDialogOpen}
      onClose={() => {
        setSettingsDialogOpen(false)
      }}
      theme={theme}
      onThemeChange={setTheme}
      preferences={settingsPreferences}
      onPreferencesChange={updateSettingsPreferences}
      onNotificationSoundChange={(enabled) => {
        if (!enabled || enableNotificationSound()) return true
        toast.error('Could not enable notification sound')
        return false
      }}
      onTestNotification={() => {
        showNotice({
          kind: 'info',
          title: 'Test notification',
          body: 'Maitty notifications are working.',
        })
        if (settingsPreferences.notificationSound) playNotificationSound()
      }}
      daemonConnectionState={
        !desktop
          ? 'preview'
          : daemonDisconnected
            ? 'disconnected'
            : sessionsQuery.isPending
              ? 'connecting'
              : 'connected'
      }
      onReconnect={() => {
        void sessionsQuery.refetch()
      }}
      codexExecutableState={
        !desktop
          ? 'preview'
          : codexExecutableQuery.isPending
            ? 'checking'
            : codexExecutableQuery.isError
              ? 'unavailable'
              : 'available'
      }
      codexExecutablePath={codexExecutableQuery.data}
      onCodexRecheck={() => {
        void codexExecutableQuery.refetch()
      }}
      claudeExecutableState={
        !desktop
          ? 'preview'
          : claudeExecutableQuery.isPending
            ? 'checking'
            : claudeExecutableQuery.isError
              ? 'unavailable'
              : 'available'
      }
      claudeExecutablePath={claudeExecutableQuery.data}
      onClaudeRecheck={() => {
        void claudeExecutableQuery.refetch()
      }}
      primaryModifier={primaryModifier}
    />
  )
}
