import { open } from '@tauri-apps/plugin-dialog'

/** The system's folder picker, opening at `defaultPath` when it is given. Null when it is cancelled. */
export async function chooseProjectFolder(defaultPath?: string): Promise<string | null> {
  const folder = await open({ directory: true, multiple: false, ...(defaultPath ? { defaultPath } : {}) })
  return typeof folder === 'string' ? folder : null
}
