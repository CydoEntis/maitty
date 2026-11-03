import { act, renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { checkForUpdate, restartApp, type UpdateHandle } from './updater'
import { updaterSummary, useUpdater } from './useUpdater'

vi.mock('./updater', () => ({ checkForUpdate: vi.fn(), restartApp: vi.fn() }))

const check = vi.mocked(checkForUpdate)

function newer(install: UpdateHandle['install'] = () => Promise.resolve()): UpdateHandle {
  return { info: { version: '0.59.0', notes: '- New: a thing' }, install }
}

beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(restartApp).mockResolvedValue(undefined)
})

describe('updater', () => {
  it('reports the app is current when no newer release exists', async () => {
    check.mockResolvedValue(null)
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('current') })
  })

  it('offers a newer release without installing it', async () => {
    const install = vi.fn(() => Promise.resolve())
    check.mockResolvedValue(newer(install))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state).toEqual({ status: 'available', version: '0.59.0', notes: '- New: a thing' }) })
    expect(install).not.toHaveBeenCalled()
  })

  it('downloads with progress, then waits for a restart', async () => {
    let report: (fraction: number | null) => void = () => {}
    let finish: () => void = () => {}
    check.mockResolvedValue(newer((onProgress) => { report = onProgress; return new Promise<void>((resolve) => { finish = resolve }) }))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('available') })

    act(() => { result.current.install() })
    expect(result.current.state).toEqual({ status: 'downloading', version: '0.59.0', progress: null })
    act(() => { report(0.5) })
    expect(result.current.state).toEqual({ status: 'downloading', version: '0.59.0', progress: 0.5 })
    act(() => { finish() })
    await waitFor(() => { expect(result.current.state).toEqual({ status: 'ready', version: '0.59.0' }) })

    act(() => { result.current.restart() })
    expect(vi.mocked(restartApp)).toHaveBeenCalledOnce()
  })

  it('keeps a failed check quiet in the banner and reports it for Settings, and a retry can succeed', async () => {
    check.mockRejectedValueOnce(new Error('offline'))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state).toEqual({ status: 'failed', message: 'offline' }) })
    expect(updaterSummary(result.current.state)).toBe('Could not check for updates: offline')

    check.mockResolvedValue(newer())
    act(() => { result.current.check() })
    expect(result.current.state.status).toBe('checking')
    await waitFor(() => { expect(result.current.state.status).toBe('available') })
  })

  it('reports a failed install and does not claim it is installed', async () => {
    check.mockResolvedValue(newer(() => Promise.reject(new Error('signature mismatch'))))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('available') })
    act(() => { result.current.install() })
    await waitFor(() => { expect(result.current.state).toEqual({ status: 'failed', message: 'signature mismatch' }) })
  })

  it('brings the offer back after Later when you check again', async () => {
    check.mockResolvedValue(newer())
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('available') })
    act(() => { result.current.dismiss() })
    expect(result.current.dismissed).toBe(true)
    act(() => { result.current.check() })
    await waitFor(() => { expect(result.current.state.status).toBe('available') })
    expect(result.current.dismissed).toBe(false)
  })
})
