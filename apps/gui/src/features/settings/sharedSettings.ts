import { useEffect, useRef } from 'react'
import { toast } from '@/lib/toast'
import {
  listenSharedSettings, requestSharedSettings, setSharedSetting, type SharedSettingKey, type SharedSettings,
} from '@/api/sharedSettings'
import { errorMessage } from '@/lib/error-message'
import { framesOfTuiSpinner, isWebSpinner, tuiSpinnerOfFrames } from '@/lib/spinner-clock'
import { themeIdOf, themeNameOf } from '@/features/workspace/theme/themes'
import type { SettingsPreferences } from './preferences'

type Update = (change: (current: SettingsPreferences) => SettingsPreferences) => void

/**
 * The shared settings as this window's own preferences would write them. A key is left out when this
 * window holds something the shared config cannot (a web spinner, the System theme mode, an agent
 * the shared "last used" stands for), so it never overwrites the other app's choice with a stand-in.
 */
export function sharedFrom(preferences: SettingsPreferences): Partial<SharedSettings> {
  const shared: Partial<SharedSettings> = {
    default_agent: preferences.defaultAgent,
    new_session_view: preferences.newSessionView,
    sidebar_side: preferences.sidebarSide,
    theme_dark: themeIdOf(preferences.darkTheme),
    theme_light: themeIdOf(preferences.lightTheme),
    start_dir: preferences.startFolder,
    reduce_motion: preferences.reduceMotion,
  }
  if (preferences.themeMode !== 'system') shared.appearance = preferences.themeMode
  if (!isWebSpinner(preferences.spinner)) shared.spinner = [...framesOfTuiSpinner(preferences.spinner)]
  return shared
}

/** These preferences, with what the daemon says laid over them. Values this window cannot show are kept as they are. */
export function applyShared(preferences: SettingsPreferences, shared: SharedSettings): SettingsPreferences {
  const next: SettingsPreferences = {
    ...preferences,
    newSessionView: shared.new_session_view,
    sidebarSide: shared.sidebar_side,
    startFolder: shared.start_dir,
    reduceMotion: shared.reduce_motion,
  }
  if (shared.default_agent === 'claude' || shared.default_agent === 'codex') next.defaultAgent = shared.default_agent
  const dark = themeNameOf(shared.theme_dark)
  const light = themeNameOf(shared.theme_light)
  if (dark !== null) next.darkTheme = dark
  if (light !== null) next.lightTheme = light
  // The shared config has two modes; this window also has System, which stays unless the other app moved off it.
  if (preferences.themeMode !== 'system') next.themeMode = shared.appearance
  const spinner = tuiSpinnerOfFrames(shared.spinner)
  if (spinner !== null) next.spinner = spinner
  return next
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)

/**
 * The settings to send after the preferences went from `before` to `after`: the keys this window's
 * user changed that the daemon does not already have. Keys that did not change are never sent, so
 * a stand-in this window holds (its own agent for the shared "last used") cannot overwrite the other app's choice.
 */
export function changesToSend(before: SettingsPreferences, after: SettingsPreferences, shared: SharedSettings): Array<[SharedSettingKey, SharedSettings[SharedSettingKey]]> {
  const was = sharedFrom(before)
  const now = sharedFrom(after)
  const changes: Array<[SharedSettingKey, SharedSettings[SharedSettingKey]]> = []
  for (const key of Object.keys(now) as SharedSettingKey[]) {
    const value = now[key]
    if (value !== undefined && !same(value, was[key]) && !same(value, shared[key])) changes.push([key, value])
  }
  return changes
}

/**
 * Keeps the shared keys of the preferences in step with the daemon's config: what the daemon says
 * (at start, and when the other app changes one) is laid over the preferences, and a change made
 * here is sent to the daemon. The daemon is the source of truth while it is reachable; the
 * browser-storage copy of the preferences is only what shows before it answers.
 */
export function useSharedSettings(preferences: SettingsPreferences, update: Update, connected: boolean) {
  const known = useRef<SharedSettings | null>(null)
  const before = useRef(preferences)

  useEffect(() => {
    if (!connected) return undefined
    let stop: (() => void) | undefined
    let stopped = false
    void listenSharedSettings((shared) => {
      known.current = shared
      update((current) => applyShared(current, shared))
    }).then((unlisten) => {
      if (stopped) unlisten()
      else stop = unlisten
      return requestSharedSettings()
    }).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not read the shared settings')) })
    return () => {
      stopped = true
      stop?.()
      known.current = null
    }
  }, [connected, update])

  useEffect(() => {
    const was = before.current
    before.current = preferences
    const shared = known.current
    if (shared === null) return
    for (const [key, value] of changesToSend(was, preferences, shared)) {
      // The daemon's answer lands in `known`, so each change is sent once.
      void setSharedSetting(key, value).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not save a shared setting')) })
    }
  }, [preferences])
}
