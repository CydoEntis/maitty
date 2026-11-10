import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, type UseQueryResult } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { ServiceBehindNotice } from './ServiceBehindNotice'

const daemon = vi.hoisted(() => ({
  loadDaemonUpdateStatus: vi.fn(),
  restartStaleDaemon: vi.fn<(version: string) => Promise<void>>(),
}))
vi.mock('@/api/daemon', () => daemon)

const toasts = vi.hoisted(() => ({
  warning: vi.fn<(title: string, options?: { action?: { label: string; onClick: () => void } }) => void>(),
  error: vi.fn(),
  dismiss: vi.fn(),
}))
vi.mock('@/lib/toast', () => ({ toast: toasts }))

const behind = { daemonVersion: '0.71.0', behind: true, canRestart: true, sessionCount: 11, activeTurnCount: 1 }

function renderNotice() {
  const refetch = vi.fn(() => Promise.resolve())
  const sessionsQuery = { isSuccess: true, refetch } as unknown as UseQueryResult<Conversation[]>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <ServiceBehindNotice sessionsQuery={sessionsQuery} />
    </QueryClientProvider>,
  )
  return { refetch }
}

beforeEach(() => {
  vi.clearAllMocks()
  daemon.loadDaemonUpdateStatus.mockResolvedValue(behind)
  daemon.restartStaleDaemon.mockResolvedValue()
})

describe('the notice that the service is a release behind', () => {
  it('says nothing when the service is current', async () => {
    daemon.loadDaemonUpdateStatus.mockResolvedValue({ daemonVersion: null, behind: false, canRestart: false, sessionCount: 3, activeTurnCount: 0 })
    renderNotice()

    await waitFor(() => { expect(daemon.loadDaemonUpdateStatus).toHaveBeenCalled() })
    expect(toasts.warning).not.toHaveBeenCalled()
  })

  it('names the version the service is on and offers the restart', async () => {
    renderNotice()

    await waitFor(() => { expect(toasts.warning).toHaveBeenCalledTimes(1) })
    expect(toasts.warning.mock.calls[0]?.[0]).toBe('The Maitty service is still on 0.71.0')
    expect(toasts.warning.mock.calls[0]?.[1]?.action?.label).toBe('Restart…')
  })

  it('offers no restart when the sessions are not saved', async () => {
    daemon.loadDaemonUpdateStatus.mockResolvedValue({ ...behind, canRestart: false })
    renderNotice()

    await waitFor(() => { expect(toasts.warning).toHaveBeenCalledTimes(1) })
    expect(toasts.warning.mock.calls[0]?.[1]?.action).toBeUndefined()
  })

  it('says what the restart costs before it restarts anything', async () => {
    renderNotice()
    await waitFor(() => { expect(toasts.warning).toHaveBeenCalled() })

    toasts.warning.mock.calls[0]?.[1]?.action?.onClick()

    expect(await screen.findByText(/11 sessions are saved and come back\. 1 agent turn in progress will stop/)).toBeTruthy()
    expect(daemon.restartStaleDaemon).not.toHaveBeenCalled()
  })

  it('restarts the version it was shown, then reconnects', async () => {
    const { refetch } = renderNotice()
    await waitFor(() => { expect(toasts.warning).toHaveBeenCalled() })
    toasts.warning.mock.calls[0]?.[1]?.action?.onClick()

    await userEvent.click(await screen.findByRole('button', { name: 'Restart service' }))

    await waitFor(() => { expect(refetch).toHaveBeenCalled() })
    expect(daemon.restartStaleDaemon).toHaveBeenCalledWith('0.71.0')
  })

  it('keeps the dialog open and says why when the restart is refused', async () => {
    daemon.restartStaleDaemon.mockRejectedValue(new Error('the local Maitty service changed since the restart prompt opened'))
    renderNotice()
    await waitFor(() => { expect(toasts.warning).toHaveBeenCalled() })
    toasts.warning.mock.calls[0]?.[1]?.action?.onClick()

    await userEvent.click(await screen.findByRole('button', { name: 'Restart service' }))

    await waitFor(() => { expect(toasts.error).toHaveBeenCalledWith('the local Maitty service changed since the restart prompt opened') })
    expect(screen.getByRole('button', { name: 'Restart service' })).toBeTruthy()
  })
})
