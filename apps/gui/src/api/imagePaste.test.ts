import { beforeEach, describe, expect, it, vi } from 'vitest'
import { MAX_IMAGE_BYTES, oversizeMessage, saveImageLocally, sendImageToMachine } from './imagePaste'

const shell = vi.hoisted(() => ({
  isTauri: true,
  invoke: vi.fn<(command: string, args?: unknown, options?: unknown) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => shell.isTauri, invoke: shell.invoke }))

beforeEach(() => {
  shell.isTauri = true
  shell.invoke.mockReset()
})

describe('image paste commands', () => {
  it('hands the host the bytes themselves, not an encoding of them', async () => {
    shell.invoke.mockResolvedValue('/home/me/.cache/maitty/pastes/paste-1-a.png')
    const bytes = new Uint8Array([1, 2, 3])
    await expect(saveImageLocally(bytes)).resolves.toBe('/home/me/.cache/maitty/pastes/paste-1-a.png')
    expect(shell.invoke).toHaveBeenCalledWith('save_pasted_image', bytes)
  })

  it('names the machine and the send in headers beside the raw bytes', async () => {
    shell.invoke.mockResolvedValue(undefined)
    const bytes = new Uint8Array([1, 2, 3])
    await sendImageToMachine('build', bytes, 'send-1')
    expect(shell.invoke).toHaveBeenCalledWith('send_pasted_image', bytes, { headers: { 'x-maitty-machine': 'build', 'x-maitty-transfer': 'send-1' } })
  })

  it('passes on the words the host refuses with', async () => {
    shell.invoke.mockRejectedValue('build is not connected; reconnect before sending an image to it')
    await expect(sendImageToMachine('build', new Uint8Array([1]), 'send-1')).rejects.toBe('build is not connected; reconnect before sending an image to it')
  })

  it('cannot paste from a plain browser', async () => {
    shell.isTauri = false
    await expect(saveImageLocally(new Uint8Array([1]))).rejects.toThrow('desktop app')
    await expect(sendImageToMachine('build', new Uint8Array([1]), 'send-1')).rejects.toThrow('desktop app')
    expect(shell.invoke).not.toHaveBeenCalled()
  })
})

describe('the size limit', () => {
  it('lets an image of up to 25 MB through and rounds a bigger one up in the words', () => {
    expect(oversizeMessage(MAX_IMAGE_BYTES)).toBeNull()
    expect(oversizeMessage(MAX_IMAGE_BYTES + 1)).toBe('The image is 26 MB, more than the 25 MB that can be pasted')
  })
})
