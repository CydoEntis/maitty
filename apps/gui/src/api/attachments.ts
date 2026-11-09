import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'

/**
 * Chat attachments (ticket 20). The host copies a file the person picked or dropped into a
 * private folder of its cache, after checking it is a regular file within the caps, and answers with
 * the copy's path and cleaned name. A pasted image is kept by `api/imagePaste`. The message names the
 * copy by path; nothing here reads the file's bytes into the window.
 */

/** The most files one message may carry. */
export const MOST_ATTACHMENTS = 10

const AttachedSchema = z.object({
  path: z.string().min(1),
  name: z.string().min(1),
  size: z.number().int().nonnegative(),
  kind: z.enum(['image', 'archive', 'file']),
})
export type Attached = z.infer<typeof AttachedSchema>
export type AttachmentKind = Attached['kind']

/** Keep a copy of the file at `path` for a message. Rejects with the host's words when it is refused (a folder, empty, too big). */
export async function attachLocalFile(path: string): Promise<Attached> {
  if (!isTauri()) throw new Error('Files can only be attached in the desktop app')
  return AttachedSchema.parse(await invoke('attach_local_file', { path }))
}

/** For each of `paths`, whether the kept file is still there. Anything outside the app's folders reads as missing. */
export async function attachmentsExist(paths: readonly string[]): Promise<boolean[]> {
  if (!isTauri() || paths.length === 0) return paths.map(() => false)
  return z.array(z.boolean()).parse(await invoke('attachments_exist', { paths }))
}

/** The system's file picker, any file, several at once. Null when it is cancelled. */
export async function chooseFilesToAttach(): Promise<string[] | null> {
  if (!isTauri()) throw new Error('Files can only be attached in the desktop app')
  const { open } = await import('@tauri-apps/plugin-dialog')
  const chosen = await open({ multiple: true, directory: false })
  if (chosen === null) return null
  return Array.isArray(chosen) ? chosen : [chosen]
}

/** The bytes of a kept image, for showing it. The host reads only images it kept itself; anything else rejects. */
export async function attachmentImage(path: string): Promise<Blob> {
  if (!isTauri()) throw new Error('Images can only be shown in the desktop app')
  return new Blob([await invoke<ArrayBuffer>('attachment_image', { path })])
}
