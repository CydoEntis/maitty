import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { ClosedSession } from '@/api/daemon'
import { RecentSessionsDialog } from './RecentSessionsDialog'

const answer = vi.hoisted(() => ({ next: (): Promise<ClosedSession[] | null> => Promise.resolve([]) }))
vi.mock('@/api/daemon', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/daemon')>()), closedSessions: () => answer.next() }))

const NOW = Math.floor(Date.now() / 1000)
const parser: ClosedSession = { agent: 'claude', session: 'abc-123', cwd: '/home/me/dev/app', branch: 'fix/parser', title: 'Fix the parser', at: NOW - 3 * 3600, folder_present: true }
const gone: ClosedSession = { agent: 'codex', session: 'def-456', cwd: '/home/me/dev/old', branch: null, title: null, at: NOW - 86_400, folder_present: false }

function open(list: ClosedSession[] | null | Error) {
  answer.next = () => (list instanceof Error ? Promise.reject(list) : Promise.resolve(list))
  const onPick = vi.fn()
  render(<RecentSessionsDialog home="/home/me" onPick={onPick} onCancel={vi.fn()} />)
  return { onPick }
}

describe('the Recent sessions list', () => {
  it("shows each session's title, agent, folder, branch and when it closed, and picking one reopens it", async () => {
    const { onPick } = open([parser])
    const row = await screen.findByRole('button', { name: /Fix the parser/ })
    expect(row.textContent).toContain('Claude · ~/dev/app · fix/parser')
    expect(row.textContent).toContain('closed 3h ago')
    fireEvent.click(row)
    expect(onPick).toHaveBeenCalledWith(parser)
  })

  it('marks a session whose folder is gone and does not let it be picked', async () => {
    const { onPick } = open([gone])
    const row = await screen.findByRole('button', { name: /Codex session/ })
    expect(row.textContent).toContain('folder removed')
    expect(row.getAttribute('aria-disabled')).toBe('true')
    expect(row.hasAttribute('disabled')).toBe(false)
    row.focus()
    expect(document.activeElement).toBe(row)
    fireEvent.click(row)
    expect(onPick).not.toHaveBeenCalled()
  })

  it('says when nothing has closed yet', async () => {
    open([])
    expect(await screen.findByText(/No closed sessions yet/)).toBeTruthy()
  })

  it('asks for a service restart when the service is older than the list', async () => {
    open(null)
    expect(await screen.findByText('Restart the Maitty service to see closed sessions.')).toBeTruthy()
  })

  it('says a failed read in plain words and tries again', async () => {
    open(new Error('pipe closed'))
    const retry = await screen.findByRole('button', { name: 'Try again' })
    answer.next = () => Promise.resolve([parser])
    fireEvent.click(retry)
    expect(await screen.findByRole('button', { name: /Fix the parser/ })).toBeTruthy()
  })
})
