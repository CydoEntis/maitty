import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { FindBar } from './FindBar'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }))

function renderBar(overrides: Partial<Parameters<typeof FindBar>[0]> = {}) {
  const props = {
    findInPane: vi.fn(() => true),
    clearPane: vi.fn(),
    nameOf: (term: number) => (term === 7 ? 'api server' : null),
    onOpenSession: vi.fn(),
    onClose: vi.fn(),
    online: true,
    ...overrides,
  }
  render(<FindBar {...props} />)
  return props
}

beforeEach(() => { vi.mocked(isTauri).mockReturnValue(true) })

describe('Find bar', () => {
  it('searches the focused pane: Enter goes forward, Shift+Enter goes back', () => {
    const props = renderBar()
    const input = screen.getByLabelText('Search text')
    fireEvent.change(input, { target: { value: 'error' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    fireEvent.keyDown(input, { key: 'Enter', shiftKey: true })
    expect(props.findInPane).toHaveBeenNthCalledWith(1, 'error', 'next')
    expect(props.findInPane).toHaveBeenNthCalledWith(2, 'error', 'previous')
  })

  it('says when the pane has no match', () => {
    renderBar({ findInPane: vi.fn(() => false) })
    const input = screen.getByLabelText('Search text')
    fireEvent.change(input, { target: { value: 'zzz' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    expect(screen.getByRole('status').textContent).toBe('No match in this pane')
  })

  it('lists matches from every session and opens the one you pick', async () => {
    vi.mocked(invoke).mockReset().mockResolvedValue([{ term: 7, line: 12, text: 'build failed: error E0432' }])
    const props = renderBar()
    fireEvent.click(screen.getByRole('button', { name: 'All sessions' }))
    const input = screen.getByLabelText('Search text')
    fireEvent.change(input, { target: { value: 'error' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    fireEvent.click(await screen.findByRole('button', { name: /api server/ }))
    expect(vi.mocked(invoke)).toHaveBeenCalledWith('search_panes', { query: 'error' })
    // A hit names its machine as well as the pane: every machine numbers its own panes. Only this computer's are searched.
    expect(props.onOpenSession).toHaveBeenCalledWith(7, 0)
    expect(props.onClose).toHaveBeenCalled()
  })

  it('shows why a search failed', async () => {
    vi.mocked(invoke).mockReset().mockRejectedValue(new Error('A search is 1 to 200 characters'))
    renderBar()
    fireEvent.click(screen.getByRole('button', { name: 'All sessions' }))
    const input = screen.getByLabelText('Search text')
    fireEvent.change(input, { target: { value: 'x' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    await waitFor(() => { expect(screen.getByRole('alert').textContent).toContain('1 to 200') })
  })

  it('closes on Escape and clears the pane’s highlight', () => {
    const props = renderBar()
    fireEvent.keyDown(screen.getByLabelText('Search text'), { key: 'Escape' })
    expect(props.clearPane).toHaveBeenCalled()
    expect(props.onClose).toHaveBeenCalled()
  })
})
