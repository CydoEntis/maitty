import { isTauri } from '@tauri-apps/api/core'

export type AvailableUpdate = { version: string; notes: string }

export type UpdateHandle = {
  info: AvailableUpdate
  /** Downloads it and leaves it ready, reporting the bytes that arrived and the whole size (null when the server did not say). */
  download: (onProgress: (received: number, total: number | null) => void) => Promise<void>
  /** Installs what was downloaded. On Windows the installer quits the app; elsewhere restart afterwards. */
  install: () => Promise<void>
}

/**
 * The newest release when it is newer than this app, else null. Null outside the desktop app too,
 * where there is nothing to update. A failed check throws.
 */
export async function checkForUpdate(): Promise<UpdateHandle | null> {
  if (!isTauri()) return null
  const { check } = await import('@tauri-apps/plugin-updater')
  const update = await check()
  if (!update) return null
  return {
    info: { version: update.version, notes: update.body ?? '' },
    install: () => update.install(),
    download: async (onProgress) => {
      let total = 0
      let received = 0
      await update.download((event) => {
        if (event.event === 'Started') total = event.data.contentLength ?? 0
        if (event.event === 'Progress') {
          received += event.data.chunkLength
          onProgress(received, total > 0 ? total : null)
        }
      })
    },
  }
}

/** Restarts the window only. The daemon and every session in it keep running. */
export async function restartApp(): Promise<void> {
  const { relaunch } = await import('@tauri-apps/plugin-process')
  await relaunch()
}
