import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { FolderListing } from '@/api/folders'
import type { NewSessionMachine } from '../model/newSession'
import { NewSessionModal } from './NewSessionModal'

const entry = (name: string) => ({ name, git: null, has_children: false })
const listing = (path: string, names: string[]): FolderListing => ({ path, exists: true, entries: names.map(entry), truncated: false })
const missing = (path: string): FolderListing => ({ path, exists: false, entries: [], truncated: false })

const MACHINES: NewSessionMachine[] = [
  { id: 0, name: 'This computer', online: true, note: '' },
  { id: 1, name: 'devbox', online: true, note: 'connected' },
  { id: 2, name: 'gpu-01', online: false, note: 'unreachable' },
]
const FOLDERS: Record<string, string[]> = { '~': ['code', 'notes'], '~/code': ['maitty', 'shop'] }

function open(overrides: Partial<Parameters<typeof NewSessionModal>[0]> = {}) {
  const onStart = vi.fn().mockResolvedValue(undefined)
  const onCancel = vi.fn()
  const browse = vi.fn().mockResolvedValue(null)
  const list = vi.fn((_machine: number, path: string) => Promise.resolve(FOLDERS[path] ? listing(path, FOLDERS[path]) : missing(path)))
  render(
    <NewSessionModal
      machines={MACHINES}
      initialMachine={0}
      initialFolder="/home/u/pacman"
      projectsOf={() => ['/home/u/pacman', '/home/u/AeVoxGrowth']}
      list={list}
      browse={browse}
      defaultAgent="claude"
      defaultView="chat"
      onStart={onStart}
      onCancel={onCancel}
      {...overrides}
    />,
  )
  return { onStart, onCancel, browse, list }
}

describe('the New session modal', () => {
  it('reads as a sentence of pills and opens on the current project, so Enter takes it', async () => {
    const user = userEvent.setup()
    const { onStart } = open()
    expect(screen.getByRole('dialog', { name: 'New session' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Project: pacman/ })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: /Browse/ })).toBeInTheDocument()
    await user.keyboard('{Enter}{Enter}')
    // Project, then machine, then the default agent is highlighted: Enter starts it.
    await user.keyboard('{Enter}')
    expect(onStart).toHaveBeenCalledWith({ machine: 0, machineName: 'This computer', folder: '/home/u/pacman', startAs: 'claude', view: 'chat' })
  })

  it('lists Browse first and opens the folder picker with Ctrl+O', async () => {
    const user = userEvent.setup()
    const { browse } = open()
    const options = screen.getAllByRole('option')
    expect(options[0]).toHaveTextContent('Browse')
    await user.keyboard('{Control>}o{/Control}')
    expect(browse).toHaveBeenCalledWith('/home/u/pacman')
  })

  it('completes a typed name with Tab and shows the hint', async () => {
    const user = userEvent.setup()
    open()
    await user.type(screen.getByRole('combobox'), 'Ae')
    expect(screen.getByText('Tab complete')).toBeInTheDocument()
    await user.keyboard('{Tab}')
    expect(screen.getByRole('combobox')).toHaveValue('/home/u/AeVoxGrowth')
  })

  it('says a typed folder is missing and blocks Enter', async () => {
    const user = userEvent.setup()
    const { onStart } = open()
    await user.type(screen.getByRole('combobox'), '~/no-such')
    await waitFor(() => { expect(screen.getByRole('status')).toHaveTextContent('No such folder on This computer') })
    await user.keyboard('{Enter}')
    expect(screen.getByRole('button', { name: /Project: pacman/ })).toBeInTheDocument()
    expect(onStart).not.toHaveBeenCalled()
  })

  it('takes a typed folder that exists on the machine', async () => {
    const user = userEvent.setup()
    open()
    await user.type(screen.getByRole('combobox'), '~/code')
    await waitFor(() => { expect(screen.queryByText(/No such folder/)).toBeNull() })
    await waitFor(() => { expect(screen.getByRole('combobox')).toHaveValue('~/code') })
    // The check is asked after a pause; Enter moves on once the machine has said the folder is there.
    await waitFor(async () => {
      await user.keyboard('{Enter}')
      expect(screen.getByRole('button', { name: /Project: code/ })).toBeInTheDocument()
    })
  })

  it('greys out an offline machine and cannot pick it', async () => {
    const user = userEvent.setup()
    open()
    await user.click(screen.getByRole('button', { name: /Machine:/ }))
    const offline = screen.getByRole('option', { name: /gpu-01/ })
    expect(offline).toHaveAttribute('aria-disabled', 'true')
    expect(offline).toHaveTextContent('Reconnecting…')
    await user.click(offline)
    expect(screen.getByRole('button', { name: /Machine: This computer/ })).toBeInTheDocument()
  })

  it('locks the view to the terminal for a shell and for another machine', async () => {
    const user = userEvent.setup()
    open()
    const chat = screen.getByRole('radio', { name: 'Chat view' })
    expect(chat).toHaveAttribute('aria-checked', 'true')
    await user.click(screen.getByRole('button', { name: /Start as:/ }))
    await user.keyboard('{ArrowUp}')
    expect(screen.getByRole('radio', { name: 'Chat view' })).toHaveAttribute('aria-disabled', 'true')
    expect(screen.getByRole('radio', { name: 'Chat view' })).toHaveAttribute('title', 'Shells always open in Terminal')
    await user.click(screen.getByRole('button', { name: /Machine:/ }))
    await user.click(screen.getByRole('option', { name: /devbox/ }))
    await user.keyboard('{ArrowDown}')
    expect(screen.getByRole('radio', { name: 'Chat view' })).toHaveAttribute('title', 'Chat runs on this computer')
  })

  it('disables Crew, and goes back with Esc before it cancels', async () => {
    const user = userEvent.setup()
    const { onCancel, onStart } = open()
    await user.click(screen.getByRole('button', { name: /Start as:/ }))
    expect(screen.getByRole('option', { name: /Crew/ })).toHaveAttribute('aria-disabled', 'true')
    await user.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}{Enter}')
    expect(onStart).not.toHaveBeenCalled()
    await user.keyboard('{Escape}')
    expect(screen.getByRole('listbox', { name: 'Machine' })).toBeInTheDocument()
    await user.keyboard('{Escape}{Escape}')
    expect(onCancel).toHaveBeenCalledTimes(1)
  })

  it('starts on another machine as a terminal view', async () => {
    const user = userEvent.setup()
    const { onStart } = open({ initialMachine: 1, initialFolder: '~/code' })
    await user.keyboard('{Enter}{Enter}{Enter}')
    expect(onStart).toHaveBeenCalledWith({ machine: 1, machineName: 'devbox', folder: '~/code', startAs: 'claude', view: 'terminal' })
  })
})
