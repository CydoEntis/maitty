import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { FolderListing } from '@/api/folders'
import { parentOf, splitTyped, withoutTrailingSlash } from '../model/folderPath'
import { RemoteSessionDialog } from './RemoteSessionDialog'

const entry = (name: string) => ({ name, git: null, has_children: false })
const listing = (path: string, names: string[]): FolderListing => ({ path, exists: true, entries: names.map(entry), truncated: false })
const missing = (path: string): FolderListing => ({ path, exists: false, entries: [], truncated: false })

/** A machine with these folders: a path maps to the names inside it; anything else is not there. */
function machineWith(folders: Record<string, string[]>) {
  return vi.fn((path: string) => Promise.resolve(folders[path] ? listing(path, folders[path]) : missing(path)))
}

const HOME_FOLDERS = { '~': ['code', 'notes', '.cache'], '~/code': ['maitty', 'shop'], '/srv/app': [] }

function open(overrides: Partial<Parameters<typeof RemoteSessionDialog>[0]> = {}) {
  const onStart = vi.fn().mockResolvedValue(undefined)
  const onCancel = vi.fn()
  const list = machineWith(HOME_FOLDERS)
  render(<RemoteSessionDialog machineName="build" list={list} recent={[]} onStart={onStart} onCancel={onCancel} {...overrides} />)
  return { onStart, onCancel, list }
}

describe('the folder arithmetic', () => {
  it('goes up a level, and stops at the home folder and the root', () => {
    expect(parentOf('~/code/maitty')).toBe('~/code')
    expect(parentOf('~/code/')).toBe('~')
    expect(parentOf('~')).toBe('~')
    expect(parentOf('/srv/app')).toBe('/srv')
    expect(parentOf('/srv')).toBe('/')
    expect(parentOf('/')).toBe('/')
  })

  it('splits what is typed into the folder it is in and the part after it', () => {
    expect(splitTyped('~/code/ma')).toEqual({ folder: '~/code/', part: 'ma' })
    expect(splitTyped('~/code/')).toEqual({ folder: '~/code/', part: '' })
    expect(splitTyped('~')).toEqual({ folder: '~/', part: '' })
    expect(withoutTrailingSlash('~/code/')).toBe('~/code')
    expect(withoutTrailingSlash('/')).toBe('/')
  })
})

describe('a new terminal on another machine', () => {
  it('starts at ~, checks the folder on the machine, and starts in the one typed', async () => {
    const user = userEvent.setup()
    const { onStart, list } = open()
    const folder = screen.getByRole('textbox', { name: 'Folder on build' })
    expect(folder).toHaveValue('~')
    await waitFor(() => { expect(screen.getByRole('status')).toHaveTextContent('3 items') })
    expect(list).toHaveBeenCalledWith('~')

    await user.clear(folder)
    await user.type(folder, '/srv/app')
    await waitFor(() => { expect(screen.getByRole('button', { name: 'Open terminal ↵' })).toBeEnabled() })
    await user.keyboard('{Enter}')
    expect(onStart).toHaveBeenCalledWith('/srv/app')
  })

  it('will not start in a folder that is not there, and says so', async () => {
    const user = userEvent.setup()
    const { onStart } = open()
    const folder = screen.getByRole('textbox', { name: 'Folder on build' })
    await user.clear(folder)
    await user.type(folder, '~/nope')
    expect(await screen.findByText('Folder not found on build')).toBeInTheDocument()
    expect(folder).toHaveAttribute('aria-invalid', 'true')
    const open_ = screen.getByRole('button', { name: 'Open terminal ↵' })
    expect(open_).toBeDisabled()
    await user.keyboard('{Enter}')
    expect(onStart).not.toHaveBeenCalled()
  })

  it('suggests the folders that continue what is typed, hides dot folders, and completes one with Tab', async () => {
    const user = userEvent.setup()
    open()
    const folder = screen.getByRole('textbox', { name: 'Folder on build' })
    await user.clear(folder)
    await user.type(folder, '~/c')
    const options = await screen.findAllByRole('option')
    expect(options.map((o) => o.textContent)).toEqual(['▸code'])

    await user.keyboard('{Tab}')
    expect(folder).toHaveValue('~/code/')
    await waitFor(() => { expect(screen.getAllByRole('option').map((o) => o.textContent)).toEqual(['▸maitty', '▸shop']) })
    await user.type(folder, '{Backspace}')
    expect(screen.queryByText('.cache')).toBeNull()
  })

  it('offers the machine\'s recent folders, and ↓ then Enter picks one', async () => {
    const user = userEvent.setup()
    open({ recent: ['/srv/app', '~/code/shop'] })
    const folder = screen.getByRole('textbox', { name: 'Folder on build' })
    await user.clear(folder)
    await user.type(folder, '/s')
    const option = await screen.findByRole('option', { name: /\/srv\/app/ })
    expect(option).toHaveTextContent('recent')
    await user.keyboard('{ArrowDown}{Enter}')
    expect(folder).toHaveValue('/srv/app')
  })

  it('goes Up a level and Home from the buttons', async () => {
    const user = userEvent.setup()
    open()
    const folder = screen.getByRole('textbox', { name: 'Folder on build' })
    await user.clear(folder)
    await user.type(folder, '~/code/maitty')
    await user.click(screen.getByRole('button', { name: 'Up' }))
    expect(folder).toHaveValue('~/code')
    await user.click(screen.getByRole('button', { name: 'Home' }))
    expect(folder).toHaveValue('~')
  })

  it('says when the machine did not answer, and does not start', async () => {
    const list = vi.fn().mockRejectedValue(new Error('build is not connected'))
    open({ list })
    expect(await screen.findByText('build is not connected')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Open terminal ↵' })).toBeDisabled()
  })

  it('shows why a machine could not start it and stays open', async () => {
    const user = userEvent.setup()
    const { onStart } = open()
    onStart.mockRejectedValue(new Error('build is not connected'))
    await waitFor(() => { expect(screen.getByRole('button', { name: 'Open terminal ↵' })).toBeEnabled() })
    await user.click(screen.getByRole('button', { name: 'Open terminal ↵' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('build is not connected')
    expect(screen.getByRole('dialog', { name: 'New session on build' })).toBeInTheDocument()
  })

  it('cancels with Esc and with Cancel', async () => {
    const user = userEvent.setup()
    const { onCancel, onStart } = open()
    await user.keyboard('{Escape}')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalledTimes(2)
    expect(onStart).not.toHaveBeenCalled()
  })
})
