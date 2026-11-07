import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'

/**
 * The settings the terminal app and this app share, as the daemon serves them (the host's
 * `get_shared_settings` and `set_shared_setting`). The daemon owns them: this window asks, shows
 * what it is told, and asks for a change; it never writes config.toml itself.
 */
export const SharedSettingsSchema = z.object({
  default_agent: z.string(),
  new_session_view: z.enum(['chat', 'terminal']),
  sidebar_side: z.enum(['left', 'right']),
  theme_dark: z.string(),
  theme_light: z.string(),
  appearance: z.enum(['dark', 'light']),
  spinner: z.array(z.string()),
  start_dir: z.string(),
  reduce_motion: z.boolean(),
  window_style: z.enum(['flat', 'glass']),
})
export type SharedSettings = z.infer<typeof SharedSettingsSchema>

/** The names a setting is changed by. */
export type SharedSettingKey = keyof SharedSettings

/** Asks for the settings; they arrive through `listenSharedSettings`. Does nothing outside the desktop app. */
export async function requestSharedSettings(): Promise<void> {
  if (!isTauri()) return
  await invoke('get_shared_settings')
}

/** Asks the daemon to change one setting. The new value arrives through `listenSharedSettings` like any change. */
export async function setSharedSetting(key: SharedSettingKey, value: SharedSettings[SharedSettingKey]): Promise<void> {
  if (!isTauri()) return
  await invoke('set_shared_setting', { key, value })
}

/** Calls back with the settings each time the daemon says them: the answer to a question, or a change from either app. */
export function listenSharedSettings(onSettings: (settings: SharedSettings) => void): Promise<() => void> {
  return listen<unknown>('shared-settings', (event) => {
    const settings = SharedSettingsSchema.safeParse(event.payload)
    if (!settings.success) {
      console.warn('The host sent settings this window cannot read', settings.error.issues)
      return
    }
    onSettings(settings.data)
  })
}
