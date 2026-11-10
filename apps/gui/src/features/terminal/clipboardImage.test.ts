import { describe, expect, it, vi } from 'vitest'
import { isPasteChord, listenForImagePaste, pickFromClipboard, unsupportedMessage } from './clipboardImage'
import { readClipboardImage } from '@/api/clipboardImage'

vi.mock('@/api/clipboardImage', () => ({ readClipboardImage: vi.fn(() => Promise.resolve(null)) }))

type Item = { kind: 'file' | 'string'; type: string; file?: File | null }

/** The part of a paste's clipboard these functions read; jsdom has no DataTransfer to build one from. */
function clipboard(text: string, items: Item[]): DataTransfer {
  return {
    getData: (type: string) => (type === 'text/plain' ? text : ''),
    items: items.map((item) => ({ kind: item.kind, type: item.type, getAsFile: () => item.file ?? null })),
  } as unknown as DataTransfer
}

const png = new File([new Uint8Array([0x89, 0x50])], 'image.png', { type: 'image/png' })
const file = (type: string, image: File | null = png): Item => ({ kind: 'file', type, file: image })

describe('what a paste carries', () => {
  it('is text when there is text, even when an image comes with it', () => {
    expect(pickFromClipboard(clipboard('hello', []))).toEqual({ kind: 'text' })
    expect(pickFromClipboard(clipboard('=SUM(A1:A3)', [file('image/png')]))).toEqual({ kind: 'text' })
  })

  it('is the image when there is an image and no text', () => {
    expect(pickFromClipboard(clipboard('', [file('image/png')]))).toEqual({ kind: 'image', image: png })
    for (const type of ['image/jpeg', 'image/gif', 'image/webp']) {
      expect(pickFromClipboard(clipboard('', [file(type)])).kind, type).toBe('image')
    }
  })

  it('takes a usable image over one the host cannot use', () => {
    expect(pickFromClipboard(clipboard('', [file('image/bmp'), file('image/png')])).kind).toBe('image')
  })

  it('names a format the host cannot use when it is all there is', () => {
    expect(pickFromClipboard(clipboard('', [file('image/bmp')]))).toEqual({ kind: 'unsupported', type: 'image/bmp' })
    expect(unsupportedMessage('image/bmp')).toBe('A BMP image cannot be pasted; copy it as PNG, JPEG, GIF or WebP')
  })

  it('is nothing for an empty clipboard, a file that is not an image, or no clipboard', () => {
    expect(pickFromClipboard(clipboard('', []))).toEqual({ kind: 'none' })
    expect(pickFromClipboard(clipboard('', [file('application/pdf')]))).toEqual({ kind: 'none' })
    expect(pickFromClipboard(clipboard('', [{ kind: 'string', type: 'image/png' }]))).toEqual({ kind: 'none' })
    expect(pickFromClipboard(null)).toEqual({ kind: 'none' })
  })

  it('skips an item whose file the webview will not hand over', () => {
    expect(pickFromClipboard(clipboard('', [file('image/png', null)]))).toEqual({ kind: 'none' })
  })
})

function pasteOn(host: HTMLElement, data: DataTransfer | null) {
  const event = new Event('paste', { bubbles: true, cancelable: true })
  Object.defineProperty(event, 'clipboardData', { value: data })
  const reachedTerminal = vi.fn()
  const textarea = document.createElement('textarea')
  host.append(textarea)
  textarea.addEventListener('paste', reachedTerminal)
  textarea.dispatchEvent(event)
  return { event, reachedTerminal }
}

describe('listening for a pasted image', () => {
  function setup() {
    const host = document.createElement('div')
    document.body.append(host)
    const onImage = vi.fn()
    const onRefuse = vi.fn()
    const stop = listenForImagePaste(host, onImage, onRefuse)
    return { host, onImage, onRefuse, stop }
  }

  it('asks the host for the picture when the paste arrives empty, as it does on Linux', async () => {
    vi.mocked(readClipboardImage).mockResolvedValueOnce(png)
    const { host, onImage } = setup()
    const { event } = pasteOn(host, clipboard('', []))
    await vi.waitFor(() => { expect(onImage).toHaveBeenCalledWith(png) })
    expect(event.defaultPrevented).toBe(false)
  })

  it('leaves a paste of text to the terminal without asking the host', () => {
    vi.mocked(readClipboardImage).mockClear()
    const { host, onImage } = setup()
    pasteOn(host, clipboard('hello', []))
    expect(readClipboardImage).not.toHaveBeenCalled()
    expect(onImage).not.toHaveBeenCalled()
  })

  it('takes an image paste for the window and keeps it from the terminal', () => {
    const { host, onImage, onRefuse } = setup()
    const { event, reachedTerminal } = pasteOn(host, clipboard('', [file('image/png')]))
    expect(onImage).toHaveBeenCalledWith(png)
    expect(onRefuse).not.toHaveBeenCalled()
    expect(event.defaultPrevented).toBe(true)
    expect(reachedTerminal).not.toHaveBeenCalled()
  })

  it('leaves a paste of text to the terminal, untouched', () => {
    const { host, onImage, onRefuse } = setup()
    const { event, reachedTerminal } = pasteOn(host, clipboard('ls -la', []))
    expect(onImage).not.toHaveBeenCalled()
    expect(onRefuse).not.toHaveBeenCalled()
    expect(event.defaultPrevented).toBe(false)
    expect(reachedTerminal).toHaveBeenCalled()
  })

  it('leaves a text paste alone even when an image is on the clipboard too', () => {
    const { host, onImage, reachedTerminal } = (() => {
      const made = setup()
      return { ...made, ...pasteOn(made.host, clipboard('a cell', [file('image/png')])) }
    })()
    expect(onImage).not.toHaveBeenCalled()
    expect(reachedTerminal).toHaveBeenCalled()
    expect(host).toBeDefined()
  })

  it('says so when the image is a format the host cannot use, and sends nothing', () => {
    const { host, onImage, onRefuse } = setup()
    pasteOn(host, clipboard('', [file('image/tiff')]))
    expect(onImage).not.toHaveBeenCalled()
    expect(onRefuse).toHaveBeenCalledWith('A TIFF image cannot be pasted; copy it as PNG, JPEG, GIF or WebP')
  })

  it('ignores an empty clipboard and stops listening when asked', () => {
    const { host, onImage, stop } = setup()
    pasteOn(host, clipboard('', []))
    expect(onImage).not.toHaveBeenCalled()
    stop()
    pasteOn(host, clipboard('', [file('image/png')]))
    expect(onImage).not.toHaveBeenCalled()
  })
})

describe('the paste chord', () => {
  const key = (init: Partial<Parameters<typeof isPasteChord>[0]>) => ({ key: 'v', ctrlKey: true, shiftKey: false, altKey: false, metaKey: false, ...init })
  const linux = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15'
  const windows = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebView2'
  const mac = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15'

  // true: the pane's key handler returns false and the browser pastes; false: the key goes on to the program as before.
  it.each([
    ['Linux Ctrl+Shift+V pastes', linux, key({ shiftKey: true }), true],
    ['Windows Ctrl+Shift+V pastes', windows, key({ key: 'V', shiftKey: true }), true],
    ['Linux plain Ctrl+V is the program\'s ^V', linux, key({}), false],
    ['Windows plain Ctrl+V is the program\'s ^V', windows, key({}), false],
    ['macOS Cmd+V already pastes, untouched', mac, key({ ctrlKey: false, metaKey: true }), false],
    ['macOS Cmd+Shift+V already pastes, untouched', mac, key({ ctrlKey: false, metaKey: true, shiftKey: true }), false],
    ['macOS Ctrl+V is the program\'s', mac, key({}), false],
    ['Ctrl+Shift+C is another key', linux, key({ key: 'c', shiftKey: true }), false],
    ['Ctrl+Alt+Shift+V is not the chord', linux, key({ shiftKey: true, altKey: true }), false],
    ['Shift+V without Ctrl is typing', linux, key({ ctrlKey: false, shiftKey: true }), false],
  ])('%s', (_name, agent, event, expected) => {
    expect(isPasteChord(event, agent)).toBe(expected)
  })
})
