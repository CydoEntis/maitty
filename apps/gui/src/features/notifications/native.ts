import { isTauri } from '@tauri-apps/api/core'

let permission: Promise<boolean> | null = null

async function allowed(): Promise<boolean> {
  const { isPermissionGranted, requestPermission } = await import('@tauri-apps/plugin-notification')
  if (await isPermissionGranted()) return true
  return (await requestPermission()) === 'granted'
}

/**
 * A notification from the operating system, if the user lets the app send them. Asked once per run:
 * a refusal is remembered, so a denied app does not nag. Does nothing outside the desktop app.
 */
export async function sendNativeNotice(title: string, body: string): Promise<void> {
  if (!isTauri()) return
  permission ??= allowed().catch(() => false)
  if (!(await permission)) return
  const { sendNotification } = await import('@tauri-apps/plugin-notification')
  sendNotification({ title, body })
}
