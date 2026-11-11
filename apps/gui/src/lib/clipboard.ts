import { isTauri } from '@tauri-apps/api/core'

/**
 * Puts text on the system clipboard. In the desktop app the host writes it (`clipboard-manager:allow-write-text`):
 * the Linux web view refuses its own clipboard call at times, with no reason the page can act on, and the host's
 * write does not depend on it. Outside the desktop app the browser's clipboard is the only one there is.
 */
export async function copyText(text: string): Promise<void> {
  if (!isTauri()) {
    await navigator.clipboard.writeText(text)
    return
  }
  const { writeText } = await import('@tauri-apps/plugin-clipboard-manager')
  await writeText(text)
}
