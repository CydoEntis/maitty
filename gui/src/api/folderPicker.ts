import { open } from '@tauri-apps/plugin-dialog'

export async function chooseProjectFolder(): Promise<string | null> {
  const folder = await open({ directory: true, multiple: false })
  return typeof folder === 'string' ? folder : null
}
