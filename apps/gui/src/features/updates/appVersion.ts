import { isTauri } from '@tauri-apps/api/core'

/** The version of the running app, or null outside the desktop app. */
export async function appVersion(): Promise<string | null> {
  if (!isTauri()) return null
  const { getVersion } = await import('@tauri-apps/api/app')
  return getVersion()
}
