import { beforeEach, describe, expect, it, vi } from 'vitest'
import { isTauri } from '@tauri-apps/api/core'
import { writeText as hostWriteText } from '@tauri-apps/plugin-clipboard-manager'
import { copyText } from './clipboard'

vi.mock('@tauri-apps/api/core', () => ({ isTauri: vi.fn() }))
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ writeText: vi.fn(() => Promise.resolve()) }))

const pageWriteText = vi.fn(() => Promise.resolve())

beforeEach(() => {
  vi.clearAllMocks()
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: pageWriteText } })
})

describe('copying text', () => {
  it('is the host\'s to do in the desktop app, so the web view cannot refuse it', async () => {
    vi.mocked(isTauri).mockReturnValue(true)
    await copyText('the text')
    expect(hostWriteText).toHaveBeenCalledExactlyOnceWith('the text')
    expect(pageWriteText).not.toHaveBeenCalled()
  })

  it('uses the browser\'s clipboard outside the desktop app', async () => {
    vi.mocked(isTauri).mockReturnValue(false)
    await copyText('the text')
    expect(pageWriteText).toHaveBeenCalledExactlyOnceWith('the text')
    expect(hostWriteText).not.toHaveBeenCalled()
  })

  it('fails when the write does, for the caller to say so', async () => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(hostWriteText).mockRejectedValueOnce(new Error('no clipboard'))
    await expect(copyText('the text')).rejects.toThrow('no clipboard')
  })
})
