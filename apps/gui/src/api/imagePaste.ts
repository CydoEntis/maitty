import { invoke, isTauri } from '@tauri-apps/api/core'

/**
 * Pasting an image into a pane (ticket 19, ADRs 0049 and 0055). The image travels as the raw body of
 * a command, never as base64 in an event; the machine and the send's id ride in headers. The host
 * checks the bytes (a PNG, JPEG, GIF or WebP signature, at most 25 MB), keeps them in its private
 * cache and either returns the path on this computer or sends them to the machine as files are sent,
 * reporting through the `file-send` events (`api/fileSends`). A chat composer's attachment calls the
 * same two functions.
 */

const BYTES_PER_KB = 1024
const BYTES_PER_MB = BYTES_PER_KB * BYTES_PER_KB
const MAX_IMAGE_MB = 25
/** The most an image may weigh; the host refuses more and so does the window, before reading it. */
export const MAX_IMAGE_BYTES = MAX_IMAGE_MB * BYTES_PER_MB

/** The image types the host accepts (it checks the bytes, not this list). */
export const IMAGE_TYPES: readonly string[] = ['image/png', 'image/jpeg', 'image/gif', 'image/webp']


/** The words for an image over the limit, or null when it fits. */
export function oversizeMessage(bytes: number): string | null {
  if (bytes <= MAX_IMAGE_BYTES) return null
  return `The image is ${String(Math.ceil(bytes / BYTES_PER_MB))} MB, more than the ${String(MAX_IMAGE_MB)} MB that can be pasted`
}

/** Keep the image on this computer and resolve to its path, for a pane of this computer. Rejects with the host's words. */
export async function saveImageLocally(bytes: Uint8Array): Promise<string> {
  if (!isTauri()) throw new Error('Images can only be pasted in the desktop app')
  return invoke<string>('save_pasted_image', bytes)
}

/** Start sending the image to the machine called `machine`; how it goes comes as `file-send` events named by `transfer`. Rejects with the host's words when it refuses. */
export async function sendImageToMachine(machine: string, bytes: Uint8Array, transfer: string): Promise<void> {
  if (!isTauri()) throw new Error('Images can only be pasted in the desktop app')
  await invoke('send_pasted_image', bytes, { headers: { 'x-maitty-machine': machine, 'x-maitty-transfer': transfer } })
}
