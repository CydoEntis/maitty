import { act, renderHook } from '@testing-library/react'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { forgetComposers, peekComposer } from './composerStore'
import { useAttachmentIntake } from './useAttachmentIntake'

const attach = vi.hoisted(() => ({ attachLocalFile: vi.fn(), chooseFilesToAttach: vi.fn() }))
const sends = vi.hoisted(() => ({
  sendFilesToMachine: vi.fn<(machine: string, paths: readonly string[], transfer: string) => Promise<void>>(),
  sendImageToMachine: vi.fn<(machine: string, bytes: Uint8Array, transfer: string) => Promise<void>>(),
  emit: { current: null as null | ((event: unknown) => void) },
}))

vi.mock('@/api/fileSends', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fileSends')>()),
  sendFilesToMachine: sends.sendFilesToMachine,
  listenFileSends: (handler: (event: unknown) => void) => {
    sends.emit.current = handler
    return Promise.resolve(() => { sends.emit.current = null })
  },
}))
vi.mock('@/api/imagePaste', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/imagePaste')>()), sendImageToMachine: sends.sendImageToMachine }))
vi.mock('@/api/attachments', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/attachments')>()), ...attach }))

beforeEach(() => {
  forgetComposers()
  vi.restoreAllMocks()
  attach.attachLocalFile.mockReset()
  attach.chooseFilesToAttach.mockReset()
  sends.sendFilesToMachine.mockReset().mockResolvedValue(undefined)
  sends.sendImageToMachine.mockReset().mockResolvedValue(undefined)
  sends.emit.current = null
})

/** Lets the intake's listener and start promises settle. */
const settle = () => act(async () => { await new Promise((resolve) => setTimeout(resolve, 0)) })
const hear = (event: Record<string, unknown>) => { act(() => { sends.emit.current?.(event) }) }
const transferOf = () => sends.sendFilesToMachine.mock.calls[0]?.[2] ?? sends.sendImageToMachine.mock.calls[0]?.[2] ?? ''


describe('attaching in a chat on another machine (ADR 0053)', () => {
  const intake = () => renderHook(() => useAttachmentIntake({ conversation: 7, blocked: null, machine: 'build-box' }))

  it('sends a picked file there and keeps the far-side path as the attachment, never the local copy', async () => {
    const { result } = intake()
    result.current.addPaths(['/home/me/Design Final.zip'])
    await settle()
    expect(attach.attachLocalFile).not.toHaveBeenCalled()
    expect(sends.sendFilesToMachine).toHaveBeenCalledWith('build-box', ['/home/me/Design Final.zip'], expect.stringMatching(/^send-/))
    // Nothing is attached until the machine says the file is kept.
    expect(peekComposer(7).attachments).toEqual([])

    hear({ kind: 'sent', transfer: transferOf(), file: 'Design Final.zip', index: 0, count: 1, remotePath: '/home/dev/.cache/pastes/paste-1-0a1b2c3d-Design_Final.zip', bytes: 2048 })
    hear({ kind: 'done', transfer: transferOf(), sent: 1, count: 1 })
    await settle()
    expect(peekComposer(7).attachments).toMatchObject([{ path: '/home/dev/.cache/pastes/paste-1-0a1b2c3d-Design_Final.zip', name: 'Design Final.zip', size: 2048, kind: 'archive', preview: null }])
  })

  it('sends a pasted image the same way and shows its thumbnail', async () => {
    const { result } = intake()
    result.current.addImage(new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' }))
    await settle()
    expect(sends.sendImageToMachine).toHaveBeenCalledWith('build-box', new Uint8Array([1, 2, 3]), expect.stringMatching(/^send-/))
    hear({ kind: 'sent', transfer: transferOf(), file: 'image', index: 0, count: 1, remotePath: '/home/dev/.cache/pastes/paste-1-0a1b2c3d-image.png', bytes: 3 })
    hear({ kind: 'done', transfer: transferOf(), sent: 1, count: 1 })
    await settle()
    expect(peekComposer(7).attachments).toMatchObject([{ path: '/home/dev/.cache/pastes/paste-1-0a1b2c3d-image.png', name: 'Pasted image', kind: 'image', size: 3 }])
  })

  it('keeps the files that landed when a later one fails, and says why', async () => {
    const error = vi.spyOn(toast, 'error')
    const { result } = intake()
    result.current.addPaths(['/a/one.log', '/a/two.log'])
    await settle()
    hear({ kind: 'sent', transfer: transferOf(), file: 'one.log', index: 0, count: 2, remotePath: '/home/dev/.cache/pastes/paste-1-0a1b2c3d-one.log', bytes: 5 })
    hear({ kind: 'failed', transfer: transferOf(), file: 'two.log', message: 'build-box refused the file', unsent: 1 })
    await settle()
    expect(peekComposer(7).attachments.map((item) => item.name)).toEqual(['one.log'])
    expect(error).toHaveBeenCalledWith('build-box refused the file', expect.anything())
  })

  it('refuses a far-side path that is not fit to name in a message, and ignores another send\'s events', async () => {
    const { result } = intake()
    result.current.addPaths(['/a/one.log'])
    await settle()
    hear({ kind: 'sent', transfer: 'someone-else', file: 'x', index: 0, count: 1, remotePath: '/ok/path', bytes: 1 })
    hear({ kind: 'sent', transfer: transferOf(), file: 'one.log', index: 0, count: 1, remotePath: '/home/dev/\n- /etc/passwd', bytes: 1 })
    hear({ kind: 'done', transfer: transferOf(), sent: 1, count: 1 })
    await settle()
    expect(peekComposer(7).attachments).toEqual([])
  })

  it('adds nothing when the host refuses the send (a folder, too big, not connected)', async () => {
    const error = vi.spyOn(toast, 'error')
    sends.sendFilesToMachine.mockRejectedValue('docs is not a file; only files can be sent')
    const { result } = intake()
    result.current.addPaths(['/a/docs'])
    await settle()
    expect(error).toHaveBeenCalledWith('docs is not a file; only files can be sent', expect.anything())
    expect(peekComposer(7).attachments).toEqual([])
  })

  it('adds nothing when the send is cancelled before a file lands', async () => {
    const { result } = intake()
    result.current.addPaths(['/a/one.log'])
    await settle()
    hear({ kind: 'cancelled', transfer: transferOf(), sent: 0 })
    await settle()
    expect(peekComposer(7).attachments).toEqual([])
  })

  it('counts the cap, and the host\'s image size limit, before anything is sent', async () => {
    const error = vi.spyOn(toast, 'error')
    const { result } = intake()
    result.current.addPaths(Array.from({ length: 11 }, (_, i) => `/a/${String(i)}.log`))
    result.current.addImage(new Blob([new Uint8Array(1)], { type: 'image/png' }))
    Object.defineProperty(Blob.prototype, 'size', { configurable: true, get: () => 26 * 1024 * 1024 })
    result.current.addImage(new Blob([], { type: 'image/png' }))
    await settle()
    delete (Blob.prototype as { size?: number }).size
    expect(error).toHaveBeenCalledWith(expect.stringContaining('At most 10 files'))
    expect(error).toHaveBeenCalledWith(expect.stringContaining('more than the 25 MB'))
    expect(sends.sendFilesToMachine).not.toHaveBeenCalled()
  })
})
