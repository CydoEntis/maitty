import { invoke, isTauri } from '@tauri-apps/api/core'

/**
 * Saves text as a file in `folder` (Downloads when it is empty) and resolves with where it went. The host
 * checks the name, the type and the size, and never replaces a file. Rejects with the host's words.
 */
export async function saveExport(folder: string, name: string, text: string): Promise<string> {
  if (!isTauri()) throw new Error('Saving a file needs the desktop app')
  return invoke<string>('save_export', { folder, name, text })
}
