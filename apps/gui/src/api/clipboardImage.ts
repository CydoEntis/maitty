import { isTauri } from '@tauri-apps/api/core'
import { readImage } from '@tauri-apps/plugin-clipboard-manager'
import { detectPlatform } from '@/features/workspace/model/shortcuts'

const PASTED_NAME = 'pasted.png'
const PNG = 'image/png'

/**
 * The picture on the system clipboard, read by the host. WebKitGTK does not hand a pasted picture
 * to the page (WebKit bug 218519): its paste event arrives empty, so on Linux the host is asked
 * instead. Null when the clipboard holds no picture, and always off Linux, where the web view's
 * paste event already carries it.
 */
export async function readClipboardImage(userAgent: string = navigator.userAgent): Promise<File | null> {
  if (!isTauri() || detectPlatform(userAgent) !== 'linux') return null
  let image: Awaited<ReturnType<typeof readImage>>
  try {
    image = await readImage()
  } catch {
    // The host answers a clipboard with no picture on it with an error: there is nothing to paste.
    return null
  }
  try {
    const [pixels, size] = await Promise.all([image.rgba(), image.size()])
    const png = await pngOf(pixels, size.width, size.height)
    return png === null ? null : new File([png], PASTED_NAME, { type: PNG })
  } finally {
    await image.close()
  }
}

/** The host hands over raw pixels; the app keeps and sends pictures as files, so they become a PNG. */
function pngOf(pixels: Uint8Array, width: number, height: number): Promise<Blob | null> {
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const context = canvas.getContext('2d')
  if (context === null) return Promise.resolve(null)
  context.putImageData(new ImageData(new Uint8ClampedArray(pixels), width, height), 0, 0)
  return new Promise((resolve) => { canvas.toBlob(resolve, PNG) })
}
