import { renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { MAX_IMAGE_BYTES } from '@/api/imagePaste'
import { useImagePaste } from './useImagePaste'

const host = vi.hoisted(() => ({ save: vi.fn<(bytes: Uint8Array) => Promise<string>>() }))
vi.mock('@/api/imagePaste', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/imagePaste')>()),
  saveImageLocally: host.save,
}))

const toasts = vi.hoisted(() => ({ loading: vi.fn(), success: vi.fn(), error: vi.fn() }))
vi.mock('sonner', () => ({ toast: toasts }))

const flush = () => new Promise((resolve) => setTimeout(resolve, 0))
const image = new Blob([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], { type: 'image/png' })

function setup(overrides: Partial<Parameters<typeof useImagePaste>[0]> = {}) {
  const sendImage = vi.fn()
  const typeInto = vi.fn<(sessionId: string, text: string) => boolean>(() => true)
  const options = { sendImage, typeInto, isConnected: () => true, online: true, windows: false, ...overrides }
  const { result } = renderHook(() => useImagePaste(options))
  return { paste: result.current, sendImage, typeInto }
}

beforeEach(() => {
  vi.clearAllMocks()
  host.save.mockResolvedValue('/home/me/.cache/maitty/pastes/paste-1-0000000a.png')
})

describe('pasting an image into a pane of this computer', () => {
  it('has the host keep the image and types its path, quoted, into the pane', async () => {
    const { paste, sendImage, typeInto } = setup()
    paste('t3', image)
    await flush()
    expect(host.save).toHaveBeenCalledWith(new Uint8Array([0x89, 0x50, 0x4e, 0x47]))
    expect(typeInto).toHaveBeenCalledWith('t3', '/home/me/.cache/maitty/pastes/paste-1-0000000a.png ')
    expect(toasts.success).toHaveBeenCalledWith('Pasted the image', expect.objectContaining({ id: 'image-paste' }))
    expect(sendImage).not.toHaveBeenCalled()
  })

  it('quotes a path with a space the way the shell reads it', async () => {
    host.save.mockResolvedValue("/home/my files/it's.png")
    const { paste, typeInto } = setup()
    paste('t3', image)
    await flush()
    expect(typeInto).toHaveBeenCalledWith('t3', "/home/my\\ files/it\\'s.png ")
  })

  it('shows what the host says when it refuses, and types nothing', async () => {
    host.save.mockRejectedValue('the clipboard holds something that is not a PNG, JPEG, GIF or WebP image')
    const { paste, typeInto } = setup()
    paste('t3', image)
    await flush()
    expect(toasts.error).toHaveBeenCalledWith('the clipboard holds something that is not a PNG, JPEG, GIF or WebP image', expect.objectContaining({ id: 'image-paste' }))
    expect(typeInto).not.toHaveBeenCalled()
  })

  it('says the pane is gone when it closed meanwhile', async () => {
    const { paste } = setup({ typeInto: () => false })
    paste('t3', image)
    await flush()
    expect(toasts.error).toHaveBeenCalledWith('The pane is closed; the image was not pasted', expect.anything())
  })

  it('does not reach the host while the service is down', async () => {
    const { paste } = setup({ online: false })
    paste('t3', image)
    await flush()
    expect(host.save).not.toHaveBeenCalled()
    expect(toasts.error).toHaveBeenCalledWith('Reconnect to the Maitty service before pasting an image')
  })
})

describe('pasting an image into a pane of another machine', () => {
  it('sends the bytes to that machine and keeps nothing here', async () => {
    const { paste, sendImage } = setup()
    paste('build/t7', image)
    await flush()
    expect(sendImage).toHaveBeenCalledWith('build/t7', 'build', new Uint8Array([0x89, 0x50, 0x4e, 0x47]))
    expect(host.save).not.toHaveBeenCalled()
  })

  it('refuses a machine that is not connected, in words, and sends nothing', async () => {
    const { paste, sendImage } = setup({ isConnected: () => false })
    paste('build/t7', image)
    await flush()
    expect(sendImage).not.toHaveBeenCalled()
    expect(toasts.error).toHaveBeenCalledWith('build is not connected; reconnect before sending an image to it')
  })

  it('asks about the machine by its name', async () => {
    const isConnected = vi.fn(() => true)
    const { paste } = setup({ isConnected })
    paste('kingdom/t2', image)
    await flush()
    expect(isConnected).toHaveBeenCalledWith('kingdom')
  })
})

describe('an image that is too big', () => {
  it('is refused before it is read, for either kind of pane', async () => {
    const read = vi.fn()
    const big = { size: MAX_IMAGE_BYTES + 1, arrayBuffer: read } as unknown as Blob
    const { paste, sendImage } = setup()
    paste('t3', big)
    paste('build/t7', big)
    await flush()
    expect(toasts.error).toHaveBeenCalledTimes(2)
    expect(toasts.error).toHaveBeenCalledWith('The image is 26 MB, more than the 25 MB that can be pasted')
    expect(host.save).not.toHaveBeenCalled()
    expect(sendImage).not.toHaveBeenCalled()
    expect(read).not.toHaveBeenCalled()
  })
})
