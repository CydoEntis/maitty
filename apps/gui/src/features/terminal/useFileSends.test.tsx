import { renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { SendEvent } from '@/api/fileSends'
import { useFileSends } from './useFileSends'

const host = vi.hoisted(() => ({
  send: vi.fn<(machine: string, paths: readonly string[], transfer: string) => Promise<void>>(),
  cancel: vi.fn<(transfer: string) => Promise<void>>(),
  emit: { current: null as null | ((event: SendEvent) => void) },
}))

vi.mock('@/api/fileSends', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fileSends')>()),
  sendFilesToMachine: host.send,
  cancelFileSend: host.cancel,
  listenFileSends: (onEvent: (event: SendEvent) => void) => {
    host.emit.current = onEvent
    return Promise.resolve(() => { host.emit.current = null })
  },
}))

const toasts = vi.hoisted(() => ({
  loading: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  message: vi.fn(),
}))
vi.mock('sonner', () => ({ toast: toasts }))

const flush = () => new Promise((resolve) => setTimeout(resolve, 0))

async function start(typeInto: (sessionId: string, text: string) => boolean, paths: string[] = ['/home/me/a.log']) {
  const { result } = renderHook(() => useFileSends({ typeInto }))
  await flush()
  result.current('build/t7', 'build', paths)
  const transfer = host.send.mock.calls[0]?.[2]
  if (transfer === undefined) throw new Error('nothing was sent')
  return transfer
}

const emit = (event: SendEvent) => { host.emit.current?.(event) }

beforeEach(() => {
  vi.clearAllMocks()
  host.send.mockResolvedValue(undefined)
  host.cancel.mockResolvedValue(undefined)
})

describe('files sent to the machine of a pane', () => {
  it('asks the host with an id made here and shows a progress toast with Cancel at once', async () => {
    const transfer = await start(() => true)
    expect(host.send).toHaveBeenCalledWith('build', ['/home/me/a.log'], expect.stringMatching(/^send-[0-9a-f-]+$/))
    const [title, options] = toasts.loading.mock.calls[0] as [string, { id: string; action: { label: string }; dismissible: boolean; closeButton: boolean }]
    expect(title).toBe('Sending a.log to build')
    expect(options).toMatchObject({ id: transfer, dismissible: false, closeButton: false, action: { label: 'Cancel' } })
  })

  it('stops the send from Cancel and keeps the toast until the host says it has stopped', async () => {
    const transfer = await start(() => true)
    const options = toasts.loading.mock.calls[0]?.[1] as { action: { onClick: (event: { preventDefault: () => void }) => void } }
    const event = { preventDefault: vi.fn() }
    options.action.onClick(event)
    expect(event.preventDefault).toHaveBeenCalled()
    expect(host.cancel).toHaveBeenCalledWith(transfer)
    emit({ kind: 'cancelled', transfer, sent: 0 })
    expect(toasts.message).toHaveBeenCalledWith('Stopped sending to build', expect.objectContaining({ id: transfer, closeButton: true }))
  })

  it('shows the percentage as the host reports it, with the file and its place in the drop', async () => {
    const transfer = await start(() => true, ['/a/one.log', '/a/two.log'])
    emit({ kind: 'progress', transfer, file: 'two.log', index: 1, count: 2, percent: 55 })
    const [title] = toasts.loading.mock.calls.at(-1) as [string]
    expect(title).toBe('Sending two.log to build (2 of 2)')
    const options = toasts.loading.mock.calls.at(-1)?.[1] as { description: { props: { percent: number; file: string } } }
    expect(options.description.props).toEqual({ file: 'two.log', percent: 55 })
  })

  it('types each path the machine kept a file at into the pane the drop was for, as it lands', async () => {
    const typed: Array<[string, string]> = []
    const transfer = await start((session, text) => { typed.push([session, text]); return true }, ['/a/one.log', '/a/two.log'])
    emit({ kind: 'sent', transfer, file: 'one.log', index: 0, count: 2, remotePath: '/home/me/.cache/p-one.log' })
    expect(typed).toEqual([['build/t7', '/home/me/.cache/p-one.log ']])
    emit({ kind: 'sent', transfer, file: 'two.log', index: 1, count: 2, remotePath: '/home/me/.cache/p-two.log' })
    emit({ kind: 'done', transfer, sent: 2, count: 2 })
    expect(typed.map(([, text]) => text)).toEqual(['/home/me/.cache/p-one.log ', '/home/me/.cache/p-two.log '])
    expect(toasts.success).toHaveBeenCalledWith('Sent 2 files to build', expect.objectContaining({ id: transfer, action: undefined, description: undefined }))
  })

  it('never types a path the machine made up', async () => {
    const typed: string[] = []
    const transfer = await start((_, text) => { typed.push(text); return true })
    emit({ kind: 'sent', transfer, file: 'a.log', index: 0, count: 1, remotePath: '/home/me/\u001b[2Jp.log' })
    emit({ kind: 'done', transfer, sent: 1, count: 1 })
    expect(typed).toEqual([])
    expect(toasts.success).toHaveBeenCalledWith('Sent 1 file to build', expect.objectContaining({ description: 'Not typed into the pane: a.log' }))
  })

  it('says where the file is when the pane is gone', async () => {
    const transfer = await start(() => false)
    emit({ kind: 'sent', transfer, file: 'a.log', index: 0, count: 1, remotePath: '/home/me/.cache/p-a.log' })
    emit({ kind: 'done', transfer, sent: 1, count: 1 })
    expect(toasts.success).toHaveBeenCalledWith('Sent 1 file to build', expect.objectContaining({ description: 'Not typed into the pane: /home/me/.cache/p-a.log' }))
  })

  it('shows a failure as an error toast and says how many files were not tried', async () => {
    const transfer = await start(() => true, ['/a/one.log', '/a/two.log', '/a/three.log'])
    emit({ kind: 'failed', transfer, file: 'one.log', message: 'could not send the file to build: Permission denied (publickey).', unsent: 3 })
    expect(toasts.error).toHaveBeenCalledWith('could not send the file to build: Permission denied (publickey). (2 more not sent)', expect.objectContaining({ id: transfer }))
  })

  it('shows the host refusing the drop (a folder, a machine without ssh) as an error toast', async () => {
    host.send.mockRejectedValue('docs is not a file; only files can be sent')
    const transfer = await start(() => true)
    await flush()
    expect(toasts.error).toHaveBeenCalledWith('docs is not a file; only files can be sent', expect.objectContaining({ id: transfer }))
  })

  it('ignores what it did not start', async () => {
    await start(() => true)
    toasts.success.mockClear()
    emit({ kind: 'done', transfer: 'send-someone-else', sent: 1, count: 1 })
    expect(toasts.success).not.toHaveBeenCalled()
  })

  it('stops listening when the window closes', async () => {
    const { unmount } = renderHook(() => useFileSends({ typeInto: () => true }))
    await flush()
    expect(host.emit.current).not.toBeNull()
    unmount()
    expect(host.emit.current).toBeNull()
  })
})
