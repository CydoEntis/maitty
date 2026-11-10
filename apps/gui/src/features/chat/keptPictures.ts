import { attachmentImage } from '@/api/attachments'

/** The picture of each kept file already read, by path: a chat that scrolls back asks the host once. */
const pictures = new Map<string, string>()

/** The address an `<img>` can show for a kept image, or null while the window does not hold it. */
export function keptPicture(path: string): string | null {
  return pictures.get(path) ?? null
}

/**
 * Holds a picture the window already has under the path it was kept at, so its chip shows the
 * thumbnail at once, in the message box and again in the sent message, with no moment of bare name.
 */
export function keepPicture(path: string, image: Blob): void {
  if (!pictures.has(path)) pictures.set(path, URL.createObjectURL(image))
}

/** Reads a kept image now, so a chip added for it starts with its thumbnail. A file that cannot be read just has none. */
export async function warmPicture(path: string): Promise<void> {
  if (pictures.has(path)) return
  try {
    keepPicture(path, await attachmentImage(path))
  } catch (error) {
    console.warn('Could not show the attached image', error)
  }
}
