import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { agoWords, contextFull, exitWords } from '../problem'
import { ProblemBanner } from './ProblemBanner'

describe('the problem banner', () => {
  it('says a stopped agent exited with its code, and offers the log and a restart', async () => {
    const onViewLog = vi.fn()
    const onRestart = vi.fn()
    render(<ProblemBanner problem={{ kind: 'crashed', agent: 'Claude', code: 137, onViewLog, onRestart }} />)
    expect(screen.getByText('Claude stopped unexpectedly')).toBeInTheDocument()
    expect(screen.getByText(/code 137, out of memory or killed/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'View log' }))
    await userEvent.click(screen.getByRole('button', { name: 'Restart' }))
    expect(onViewLog).toHaveBeenCalled()
    expect(onRestart).toHaveBeenCalled()
  })

  it('has no restart for a session that cannot be restarted', () => {
    render(<ProblemBanner problem={{ kind: 'crashed', agent: 'Claude', code: null, onViewLog: () => undefined, onRestart: null }} />)
    expect(screen.queryByRole('button', { name: 'Restart' })).not.toBeInTheDocument()
  })

  it('says a machine is offline and for how long, with a retry', async () => {
    const onRetry = vi.fn()
    render(<ProblemBanner problem={{ kind: 'offline', machine: 'devbox', since: Date.now() - 40_000, onRetry }} />)
    expect(screen.getByText('devbox is offline')).toBeInTheDocument()
    expect(screen.getByText(/Lost the connection 4\ds ago/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Retry now' }))
    expect(onRetry).toHaveBeenCalled()
  })

  it('warns when the context is nearly full and offers to compact', async () => {
    const onCompact = vi.fn()
    const onNewWithSummary = vi.fn()
    render(<ProblemBanner problem={{ kind: 'context', agent: 'Claude', percent: 96, onCompact, onNewWithSummary }} summarize={() => 'the summary'} />)
    expect(screen.getByText('Context is almost full · 96%')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Compact' }))
    expect(onCompact).toHaveBeenCalled()
    await userEvent.click(screen.getByRole('button', { name: 'New chat with summary' }))
    expect(onNewWithSummary).toHaveBeenCalledWith('the summary')
  })
})

describe('what makes a problem', () => {
  it('counts a context as full from 95%, and not before or when unreported', () => {
    expect(contextFull({ context: 94.4, limits: [] })).toBeNull()
    expect(contextFull({ context: 95, limits: [] })).toBe(95)
    expect(contextFull({ context: null, limits: [] })).toBeNull()
    expect(contextFull(undefined)).toBeNull()
  })
  it('tells a time and an exit in words', () => {
    expect(agoWords(null, 5)).toBeNull()
    expect(agoWords(1000, 41_000)).toBe('40s')
    expect(agoWords(0, 180_000)).toBe('3m')
    expect(exitWords(1)).toBe('The process exited (code 1).')
    expect(exitWords(null)).toBe('The process exited.')
  })
})
