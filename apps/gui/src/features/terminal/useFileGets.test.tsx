import { renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { GetEvent } from '@/api/fileGets'
import { useFileGets } from './useFileGets'

const host = vi.hoisted(() => ({
  get: vi.fn<(machine: string, path: string, transfer: string) => Promise<void>>(),
  open: vi.fn<(folder: string) => Promise<void>>(),
  cancel: vi.fn<(transfer: string) => Promise<void>>(),
  emit: { current: null as null | ((event: GetEvent) => void) },
}))

vi.mock('@/api/fileGets', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fileGets')>()),
  getFileFromMachine: host.get,
  openPulledFolder: host.open,
  listenFileGets: (onEvent: (event: GetEvent) => void) => {
    host.emit.current = onEvent
    return Promise.resolve(() => { host.emit.current = null })
  },
}))
vi.mock('@/api/fileSends', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fileSends')>()),
  cancelFileSend: host.cancel,
}))

const toasts = vi.hoisted(() => ({ loading: vi.fn(), success: vi.fn(), error: vi.fn(), message: vi.fn() }))
vi.mock('sonner', () => ({ toast: toasts }))

const flush = () => new Promise((resolve) => setTimeout(resolve, 0))
const emit = (event: GetEvent) => { host.emit.current?.(event) }

async function start() {
  const { result } = renderHook(() => useFileGets())
  await flush()
  result.current.getFile('devbox', '/srv/app/out.log')
  const transfer = host.get.mock.calls[0]?.[2]
  if (transfer === undefined) throw new Error('nothing was asked of the host')
  return transfer
}

beforeEach(() => {
  vi.clearAllMocks()
  host.get.mockResolvedValue(undefined)
  host.open.mockResolvedValue(undefined)
  host.cancel.mockResolvedValue(undefined)
})

describe('pulling a file from a machine', () => {
  it('shows a progress toast with Cancel at once, named by the transfer', async () => {
    const transfer = await start()
    expect(host.get).toHaveBeenCalledWith('devbox', '/srv/app/out.log', transfer)
    const [title, options] = toasts.loading.mock.calls[0] as [string, { id: string; action: { label: string; onClick: (event: { preventDefault: () => void }) => void } }]
    expect(title).toBe('Getting out.log from devbox')
    expect(options.id).toBe(transfer)
    options.action.onClick({ preventDefault: vi.fn() })
    expect(host.cancel).toHaveBeenCalledWith(transfer)
  })

  it('says where the file was saved and opens that folder from the toast', async () => {
    const transfer = await start()
    emit({ kind: 'done', transfer, file: 'out.log', path: '/d/maitty/devbox/out.log', folder: '/d/maitty/devbox' })
    const [title, options] = toasts.success.mock.calls[0] as [string, { description: string; action: { label: string; onClick: () => void } }]
    expect(title).toBe('Saved out.log from devbox')
    expect(options.description).toBe('/d/maitty/devbox')
    expect(options.action.label).toBe('Open folder')
    options.action.onClick()
    expect(host.open).toHaveBeenCalledWith('/d/maitty/devbox')
  })

  it("shows the host's own words when the pull fails, and when the host refuses it", async () => {
    const transfer = await start()
    emit({ kind: 'failed', transfer, file: 'out.log', message: '/srv/app/out.log does not exist on devbox' })
    expect(toasts.error).toHaveBeenCalledWith('/srv/app/out.log does not exist on devbox', expect.objectContaining({ id: transfer }))
    host.get.mockRejectedValueOnce(new Error('files are already being sent; wait for them or cancel'))
    const { result } = renderHook(() => useFileGets())
    await flush()
    result.current.getFile('devbox', '/x')
    await flush()
    expect(toasts.error).toHaveBeenCalledWith('files are already being sent; wait for them or cancel', expect.anything())
  })

  it('says a stopped pull was stopped, and ignores an event it did not start', async () => {
    const transfer = await start()
    emit({ kind: 'cancelled', transfer, file: 'out.log' })
    expect(toasts.message).toHaveBeenCalledWith('Stopped getting out.log from devbox', expect.objectContaining({ id: transfer }))
    emit({ kind: 'done', transfer: 'get-someone-elses', file: 'x', path: '/x', folder: '/' })
    expect(toasts.success).not.toHaveBeenCalled()
  })
})
