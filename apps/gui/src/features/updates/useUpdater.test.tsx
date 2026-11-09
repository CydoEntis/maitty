import { act, renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { checkForUpdate, restartApp, type UpdateHandle } from './updater'
import { updaterSummary, useUpdater } from './useUpdater'

vi.mock('./updater', () => ({ checkForUpdate: vi.fn(), restartApp: vi.fn() }))

const check = vi.mocked(checkForUpdate)

function newer(download: UpdateHandle['download'] = () => Promise.resolve(), install: UpdateHandle['install'] = () => Promise.resolve()): UpdateHandle {
  return { info: { version: '0.59.0', notes: '- New: a thing' }, download, install }
}

beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(restartApp).mockResolvedValue(undefined)
})

describe('updater', () => {
  it('finds a release published after launch by checking again every hour', async () => {
    vi.useFakeTimers()
    try {
      check.mockResolvedValueOnce(null).mockResolvedValue(newer())
      const { result } = renderHook(() => useUpdater())
      await act(async () => { await Promise.resolve() })
      expect(result.current.state.status).toBe('current')

      await act(async () => { await vi.advanceTimersByTimeAsync(60 * 60 * 1000) })
      expect(result.current.state.status).toBe('available')
    } finally {
      vi.useRealTimers()
    }
  })

  it('does not replace a downloaded update when the hour passes', async () => {
    vi.useFakeTimers()
    try {
      check.mockResolvedValue(newer())
      const { result } = renderHook(() => useUpdater())
      await act(async () => { await Promise.resolve() })
      act(() => { result.current.download() })
      await act(async () => { await Promise.resolve() })
      expect(result.current.state.status).toBe('ready')

      await act(async () => { await vi.advanceTimersByTimeAsync(60 * 60 * 1000) })
      expect(check).toHaveBeenCalledOnce()
      expect(result.current.state.status).toBe('ready')
    } finally {
      vi.useRealTimers()
    }
  })

  it('reports the app is current when no newer release exists', async () => {
    check.mockResolvedValue(null)
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('current') })
  })

  it('offers a newer release without downloading or installing it', async () => {
    const download = vi.fn(() => Promise.resolve())
    const install = vi.fn(() => Promise.resolve())
    check.mockResolvedValue(newer(download, install))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state).toEqual({ status: 'available', version: '0.59.0', notes: '- New: a thing' }) })
    expect(download).not.toHaveBeenCalled()
    expect(install).not.toHaveBeenCalled()
  })

  it('downloads with progress, then waits for a restart', async () => {
    let report: (fraction: number | null) => void = () => {}
    let finish: () => void = () => {}
    const install = vi.fn(() => Promise.resolve())
    check.mockResolvedValue(newer((onProgress) => { report = onProgress; return new Promise<void>((resolve) => { finish = resolve }) }, install))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('available') })

    act(() => { result.current.download() })
    expect(result.current.state).toEqual({ status: 'downloading', version: '0.59.0', progress: null })
    act(() => { report(0.5) })
    expect(result.current.state).toEqual({ status: 'downloading', version: '0.59.0', progress: 0.5 })
    act(() => { finish() })
    await waitFor(() => { expect(result.current.state).toEqual({ status: 'ready', version: '0.59.0' }) })

    expect(install).not.toHaveBeenCalled()
    act(() => { result.current.restart() })
    await waitFor(() => { expect(vi.mocked(restartApp)).toHaveBeenCalledOnce() })
    expect(install).toHaveBeenCalledOnce()
  })

  it('keeps a failed check quiet in the banner and reports it for Settings, and a retry can succeed', async () => {
    check.mockRejectedValueOnce(new Error('offline'))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state).toEqual({ status: 'failed', message: 'offline', during: 'check' }) })
    expect(updaterSummary(result.current.state)).toBe('Could not check for updates: offline')

    check.mockResolvedValue(newer())
    act(() => { result.current.check() })
    expect(result.current.state.status).toBe('checking')
    await waitFor(() => { expect(result.current.state.status).toBe('available') })
  })

  it('reports a failed download and does not claim it is ready', async () => {
    check.mockResolvedValue(newer(() => Promise.reject(new Error('signature mismatch'))))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('available') })
    act(() => { result.current.download() })
    await waitFor(() => { expect(result.current.state).toEqual({ status: 'failed', message: 'signature mismatch', during: 'update' }) })
  })

  it('does not replace a downloaded update when you check again', async () => {
    const install = vi.fn(() => Promise.resolve())
    check.mockResolvedValue(newer(() => Promise.resolve(), install))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('available') })
    act(() => { result.current.download() })
    await waitFor(() => { expect(result.current.state.status).toBe('ready') })
    check.mockClear()

    act(() => { result.current.check() })

    expect(check).not.toHaveBeenCalled()
    expect(result.current.state.status).toBe('ready')
  })

  it('tells the caller when the restart fails, and says it was the update that failed', async () => {
    check.mockResolvedValue(newer(() => Promise.resolve(), () => Promise.reject(new Error('no space left'))))
    const { result } = renderHook(() => useUpdater())
    await waitFor(() => { expect(result.current.state.status).toBe('available') })
    act(() => { result.current.download() })
    await waitFor(() => { expect(result.current.state.status).toBe('ready') })
    const onFailed = vi.fn()

    act(() => { result.current.restart(onFailed) })

    await waitFor(() => { expect(onFailed).toHaveBeenCalledTimes(1) })
    expect(result.current.state).toEqual({ status: 'failed', message: 'no space left', during: 'update' })
    expect(updaterSummary(result.current.state)).toBe('The update failed: no space left')
  })
})
