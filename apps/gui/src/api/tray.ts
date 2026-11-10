import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'

/** A session as the tray menu lists it: its id, and the name the sidebar shows for it. */
export type TraySession = { id: string; label: string }

/**
 * Tells the host which sessions its tray (or menu-bar) menu lists: the ones that need you, then the ones that
 * finished. The host only lists them. Does nothing outside the desktop app.
 */
export async function setTraySessions(needs: readonly TraySession[], finished: readonly TraySession[]): Promise<void> {
  if (!isTauri()) return
  await invoke('set_tray_sessions', { needs, finished })
}

/** Calls back with a session's id when it is chosen in the tray menu. Never calls back outside the desktop app. */
export function listenTrayOpenSession(onOpen: (sessionId: string) => void): Promise<() => void> {
  if (!isTauri()) return Promise.resolve(() => undefined)
  return listen<unknown>('tray-open-session', (event) => {
    const chosen = z.string().min(1).safeParse(event.payload)
    if (chosen.success) onOpen(chosen.data)
  })
}
