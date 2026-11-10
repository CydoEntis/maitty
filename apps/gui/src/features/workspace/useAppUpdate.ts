import { useCallback, useEffect, useRef, useState } from 'react'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { appVersion } from '@/features/updates/appVersion'
import { markRestarting, skippedUpdate, takeRestarted } from '@/features/updates/updateMemory'
import { useIdleUpdate } from '@/features/updates/useIdleUpdate'
import { useUpdater } from '@/features/updates/useUpdater'
import type { SessionView } from './model/session'

/** How long "Restarting Maitty" shows before the window relaunches. */
const RESTART_SCREEN_MS = 1200

/** Fetching and installing an app update: by hand, as soon as one is found, or when the app is idle. */
export function useAppUpdate({ updater, serviceNewer, preferences, sessions, online }: {
  updater: ReturnType<typeof useUpdater>
  serviceNewer: boolean
  preferences: SettingsPreferences
  sessions: SessionView[]
  online: boolean
}) {
  const [runningVersion, setRunningVersion] = useState<string | null>(null)
  const [skippedVersion, setSkippedVersion] = useState(skippedUpdate)
  const [restarting, setRestarting] = useState(false)
  useEffect(() => {
    let active = true
    void appVersion().then((version) => { if (active) setRunningVersion(version) }).catch(() => undefined)
    return () => { active = false }
  }, [])

  const restartInto = useCallback((automatic: boolean) => {
    if (updater.state.status !== 'ready') return
    markRestarting({ version: updater.state.version, previous: runningVersion, notes: updater.releaseNotes(), automatic })
    setRestarting(true)
    // If the install fails, the screen must not stay up: it goes, and so does the "kept running" note for the next launch.
    window.setTimeout(() => { updater.restart(() => { setRestarting(false); takeRestarted() }) }, RESTART_SCREEN_MS)
  }, [updater, runningVersion])
  const restartNow = useCallback(() => { restartInto(false) }, [restartInto])
  const installWhenIdle = useCallback(() => { restartInto(true) }, [restartInto])

  // The app launched before the release that matches the service may have been published, so look again.
  const checkForUpdateRef = useRef(updater.check)
  useEffect(() => { checkForUpdateRef.current = updater.check })
  useEffect(() => { if (serviceNewer) checkForUpdateRef.current() }, [serviceNewer])

  // With the setting on, or while the app is behind the service, a new release is fetched as soon as it is found. Installing still waits for a restart.
  const updateStatus = updater.state.status
  const updateOffered = updater.state.status === 'available' ? updater.state.version : null
  const downloadUpdate = updater.download
  useEffect(() => {
    if (!(preferences.autoDownloadUpdates || serviceNewer) || updateStatus !== 'available' || updateOffered === skippedVersion) return
    const timer = window.setTimeout(downloadUpdate, 0)
    return () => { window.clearTimeout(timer) }
  }, [preferences.autoDownloadUpdates, serviceNewer, updateStatus, updateOffered, skippedVersion, downloadUpdate])

  // With the setting on, an idle app fetches and installs an update by itself (docs/stack/tauri-updater.md).
  const busyCount = sessions.filter((session) => session.status === 'working' || session.status === 'needs').length
  useIdleUpdate({
    enabled: preferences.restartWhenIdle && (online || serviceNewer) && !restarting,
    agentBusy: busyCount > 0,
    status: updateStatus,
    offeredIsSkipped: updateOffered !== null && updateOffered === skippedVersion,
    onDownload: downloadUpdate,
    onInstall: installWhenIdle,
  })
  return { runningVersion, skippedVersion, restartNow, setSkippedVersion, restarting }
}
