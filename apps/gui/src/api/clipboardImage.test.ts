import { isTauri } from '@tauri-apps/api/core'
import { readImage } from '@tauri-apps/plugin-clipboard-manager'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { readClipboardImage } from './clipboardImage'

vi.mock('@tauri-apps/api/core', () => ({ isTauri: vi.fn() }))
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ readImage: vi.fn() }))

const LINUX = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15'
const WINDOWS = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'

beforeEach(() => {
  vi.mocked(isTauri).mockReturnValue(true)
  vi.mocked(readImage).mockReset()
})

describe('the picture on the system clipboard', () => {
  it('is not asked for off Linux, where the paste event already carries it', async () => {
    expect(await readClipboardImage(WINDOWS)).toBeNull()
    expect(readImage).not.toHaveBeenCalled()
  })

  it('is not asked for in the browser preview, which has no host', async () => {
    vi.mocked(isTauri).mockReturnValue(false)
    expect(await readClipboardImage(LINUX)).toBeNull()
    expect(readImage).not.toHaveBeenCalled()
  })

  it('is nothing when the host says the clipboard holds no picture', async () => {
    vi.mocked(readImage).mockRejectedValue('The clipboard contents were not available in the requested format')
    expect(await readClipboardImage(LINUX)).toBeNull()
  })

  it('is read from the host on Linux, and the host’s copy is let go afterwards', async () => {
    const close = vi.fn().mockResolvedValue(undefined)
    const image = { rgba: vi.fn().mockResolvedValue(new Uint8Array(4)), size: vi.fn().mockResolvedValue({ width: 1, height: 1 }), close }
    vi.mocked(readImage).mockResolvedValue(image as unknown as Awaited<ReturnType<typeof readImage>>)
    // jsdom draws nothing, so there is no PNG to hand back here; the host's copy is still released.
    await readClipboardImage(LINUX)
    expect(image.rgba).toHaveBeenCalled()
    expect(close).toHaveBeenCalled()
  })
})
