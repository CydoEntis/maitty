import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { FolderListing } from '@/api/folders'
import { PickFolderDialog } from './PickFolderDialog'

const HOME = '/home/dev'

function folder(path: string, names: string[]): FolderListing {
  return { path, exists: true, truncated: false, entries: names.map((name) => ({ name, git: name === 'maitty' ? 'dev' : null, has_children: true })) }
}

/** A machine whose home holds two folders, one of them a repo with a worktree folder inside. */
function machine() {
  return vi.fn((path: string): Promise<FolderListing> => {
    if (path === '~' || path === HOME) return Promise.resolve(folder(HOME, ['maitty', 'notes']))
    if (path === `${HOME}/maitty`) return Promise.resolve(folder(`${HOME}/maitty`, ['src']))
    if (path === '/home') return Promise.resolve(folder('/home', ['dev']))
    return Promise.resolve({ path, exists: false, truncated: false, entries: [] })
  })
}

function open(list = machine()) {
  const onPick = vi.fn()
  const onCancel = vi.fn()
  render(<PickFolderDialog machine="kingdom" list={list} onPick={onPick} onCancel={onCancel} />)
  return { onPick, onCancel, list }
}

describe('the Choose a folder dialog', () => {
  it('starts in the home folder of the machine it names', async () => {
    open()
    expect(screen.getByRole('dialog', { name: 'Choose a folder on kingdom' })).toBeTruthy()
    expect(await screen.findByRole('option', { name: 'maitty, folder on dev' })).toBeTruthy()
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Path on kingdom' }).value).toBe(HOME)
  })

  it('uses the folder a person clicked into', async () => {
    const user = userEvent.setup()
    const { onPick } = open()
    await user.click(await screen.findByRole('option', { name: 'maitty, folder on dev' }))
    await screen.findByRole('option', { name: 'src, folder' })
    await user.click(screen.getByRole('button', { name: 'Use this folder' }))
    expect(onPick).toHaveBeenCalledWith(`${HOME}/maitty`)
  })

  it('goes up one folder', async () => {
    const user = userEvent.setup()
    open()
    await screen.findByRole('option', { name: 'notes, folder' })
    await user.click(screen.getByRole('button', { name: 'Go up one folder' }))
    expect(await screen.findByRole('option', { name: 'dev, folder' })).toBeTruthy()
  })

  it('narrows the folders by the letters typed and opens the first with Enter', async () => {
    const user = userEvent.setup()
    open()
    await screen.findByRole('option', { name: 'notes, folder' })
    await user.type(screen.getByRole('textbox', { name: 'Narrow the folders' }), 'mai{Enter}')
    expect(await screen.findByRole('option', { name: 'src, folder' })).toBeTruthy()
  })

  it('says so when a typed path is not a folder there, and cannot use it', async () => {
    const user = userEvent.setup()
    open()
    const box = await screen.findByRole<HTMLInputElement>('textbox', { name: 'Path on kingdom' })
    await screen.findByRole('option', { name: 'notes, folder' })
    await user.clear(box)
    await user.type(box, '/nowhere{Enter}')
    expect((await screen.findByRole('alert')).textContent).toContain('There is no folder /nowhere on kingdom')
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Use this folder' }).disabled).toBe(true)
  })

  it('says why when the machine cannot be asked', async () => {
    open(vi.fn(() => Promise.reject(new Error('The machine did not answer in time'))))
    expect((await screen.findByRole('alert')).textContent).toContain('The machine did not answer in time')
  })
})
