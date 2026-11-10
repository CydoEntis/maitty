import { readClipboardImage } from '@/api/clipboardImage'
import { IMAGE_TYPES } from '@/api/imagePaste'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'

/** What a paste event carries, as far as a pane cares. */
export type ClipboardPick =
  | { kind: 'text' }
  | { kind: 'image'; image: File }
  | { kind: 'unsupported'; type: string }
  | { kind: 'none' }

/**
 * What to do with a paste. Text comes first, as in the terminal app: when the clipboard holds any
 * text (a copied cell range holds both text and a picture) the terminal pastes the text as it always
 * has. Only a clipboard with an image and no text becomes an image paste.
 */
export function pickFromClipboard(data: DataTransfer | null): ClipboardPick {
  if (data === null) return { kind: 'none' }
  if (data.getData('text/plain') !== '') return { kind: 'text' }
  const images = Array.from(data.items).filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
  const usable = images.filter((item) => IMAGE_TYPES.includes(item.type))
  for (const item of usable) {
    const image = item.getAsFile()
    if (image !== null) return { kind: 'image', image }
  }
  // A usable type whose file the webview would not hand over is nothing to paste, not a format to blame.
  const other = images.find((item) => !IMAGE_TYPES.includes(item.type))
  return usable.length > 0 || other === undefined ? { kind: 'none' } : { kind: 'unsupported', type: other.type }
}

/**
 * A paste that carried nothing may still be a picture: the Linux web view does not hand one to the
 * page, so the host reads the clipboard (`api/clipboardImage`). Elsewhere this finds nothing.
 */
export function pasteFromHost(onImage: (image: File) => void): void {
  readClipboardImage().then(
    (image) => { if (image !== null) onImage(image) },
    (error: unknown) => { toast.error(errorMessage(error, 'Could not read the picture on the clipboard')) },
  )
}

/** The words for an image format the host will not take. */
export function unsupportedMessage(type: string): string {
  return `A ${type.replace('image/', '').toUpperCase()} image cannot be pasted; copy it as PNG, JPEG, GIF or WebP`
}

/**
 * Listens for pastes anywhere inside `host` (the terminal's own paste listener is on its textarea,
 * below it, so this sees the event first). A paste with text is left alone for the terminal. A paste
 * of an image is taken for `onImage` and kept from the terminal, which would otherwise paste nothing
 * for it; a format the host cannot use is reported to `onRefuse`. A paste with nothing in it is asked
 * of the host (`pasteFromHost`).
 */
export function listenForImagePaste(host: HTMLElement, onImage: (image: File) => void, onRefuse: (message: string) => void): () => void {
  const onPaste = (event: ClipboardEvent) => {
    const pick = pickFromClipboard(event.clipboardData)
    if (pick.kind === 'none') pasteFromHost(onImage)
    if (pick.kind === 'text' || pick.kind === 'none') return
    event.preventDefault()
    event.stopPropagation()
    if (pick.kind === 'image') onImage(pick.image)
    else onRefuse(unsupportedMessage(pick.type))
  }
  host.addEventListener('paste', onPaste, true)
  return () => { host.removeEventListener('paste', onPaste, true) }
}

/**
 * Whether a key press is Ctrl+Shift+V outside macOS. Plain Ctrl+V is never taken: on Windows and
 * Linux a plain Ctrl+letter belongs to the program in the pane (a raw ^V, which Claude Code reads
 * for its own clipboard handling and vim for a literal insert). Ctrl+Shift+V is the terminal paste
 * chord; the pane leaves it to the browser, whose paste event is how an image on the clipboard is
 * found. (Cmd+V on macOS already pastes and needs no help.)
 */
export function isPasteChord(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>, userAgent: string): boolean {
  if (/Macintosh|Mac OS X/i.test(userAgent)) return false
  return event.key.toLowerCase() === 'v' && event.ctrlKey && event.shiftKey && !event.altKey && !event.metaKey
}
