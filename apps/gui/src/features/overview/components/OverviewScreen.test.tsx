import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { SessionStatus, SessionView } from '@/features/workspace/model/session'
import { OverviewScreen } from './OverviewScreen'

function session(pane: number, status: SessionStatus, extra: Partial<SessionView> = {}): SessionView {
  return { id: `s${String(pane)}`, pane, machine: 0, machineName: null, name: `Agent ${String(pane)}`, title: `t${String(pane)}`, status, process: 'claude', cwdLabel: '~/app', branch: null, asking: '', lastKnownStatus: null, ...extra } as SessionView
}

function show(sessions: SessionView[], handlers: { onOpen?: (id: string) => void; onJump?: () => void } = {}) {
  const projectOf = new Map(sessions.map((item) => [item.id, { name: 'app' }]))
  render(<OverviewScreen sessions={sessions} tracks={new Map()} projectOf={projectOf} jumped={null} onOpen={handlers.onOpen ?? vi.fn()} onJump={handlers.onJump ?? vi.fn()} onClose={vi.fn()} />)
}

describe('the Overview screen', () => {
  it('always shows Needs you, with a hint when nothing needs you, and hides the other empty sections', () => {
    show([session(1, 'working')])
    expect(screen.getByRole('region', { name: 'Needs you' })).toHaveTextContent('Nothing needs you. Agents are working or done.')
    expect(screen.getByRole('region', { name: 'Working' })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Done' })).toBeNull()
    expect(screen.queryByRole('region', { name: 'Idle' })).toBeNull()
  })

  it('puts the sections in order and cards in creation order, with exited sessions under Done', () => {
    show([session(3, 'idle'), session(2, 'error'), session(1, 'done'), session(5, 'needs'), session(4, 'working')])
    const headings = screen.getAllByRole('heading', { level: 3 }).map((heading) => heading.textContent)
    expect(headings).toEqual(['Needs you', 'Working', 'Done', 'Idle'])
    const done = within(screen.getByRole('region', { name: 'Done' })).getAllByRole('button')
    expect(done.map((card) => card.getAttribute('data-status'))).toEqual(['done', 'error'])
  })

  it('says the counts, and names the machine only on a remote card', () => {
    show([session(1, 'idle'), session(2, 'idle', { machine: 1, machineName: 'devbox' })])
    expect(screen.getByText('2 sessions on 2 machines')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Agent 2, idle, app, on devbox/ })).toHaveTextContent('devbox')
    expect(screen.getByRole('button', { name: 'Agent 1, idle, app' })).not.toHaveTextContent('devbox')
  })

  it('shows what a stopped agent is asking, else the pane title', () => {
    show([session(1, 'needs', { asking: '\nRun rm -rf build?\n[y/n]' }), session(2, 'idle')])
    expect(screen.getByText('Run rm -rf build?')).toBeInTheDocument()
    expect(screen.getByText('claude · ~/app')).toBeInTheDocument()
  })

  it('opens a session from its card and jumps from the header', async () => {
    const onOpen = vi.fn()
    const onJump = vi.fn()
    show([session(1, 'needs')], { onOpen, onJump })
    await userEvent.click(screen.getByRole('button', { name: /Agent 1, needs you/ }))
    expect(onOpen).toHaveBeenCalledWith('s1')
    await userEvent.click(screen.getByRole('button', { name: 'Jump to next session that needs you' }))
    expect(onJump).toHaveBeenCalled()
  })
})
