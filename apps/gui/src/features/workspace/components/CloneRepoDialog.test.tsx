import { cleanup, render, renderHook, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { lastCloneDestination, rememberCloneDestination } from '../model/cloneDestination'
import { CloneRepoDialog } from './CloneRepoDialog'
import { useCloneRepo } from './useCloneRepo'

const api = vi.hoisted(() => ({ cloneRepository: vi.fn<(machine: number, address: string, parent: string) => Promise<string>>() }))
vi.mock('@/api/cloneRepository', () => api)

const ADDRESS = 'https://example.com/o/widgets.git'

afterEach(cleanup)
beforeEach(() => {
  window.localStorage.clear()
  api.cloneRepository.mockReset()
})

type Clone = (address: string, parent: string) => Promise<string>

function open(clone: Clone = vi.fn<Clone>(() => Promise.resolve('/home/dev/code/widgets')), chooseFolder: (() => Promise<string | null>) | null = null) {
  const onCloned = vi.fn()
  const onCancel = vi.fn()
  render(<CloneRepoDialog where="kingdom" startIn="~/code" chooseFolder={chooseFolder} clone={clone} onCloned={onCloned} onCancel={onCancel} />)
  return { clone, onCloned, onCancel }
}

const addressBox = () => screen.getByRole('textbox', { name: 'Repository address' })
const cloneButton = () => screen.getByRole<HTMLButtonElement>('button', { name: 'Clone' })

describe('the Clone a repository dialog', () => {
  it('names the machine and cannot clone until it has an address and a folder', async () => {
    const user = userEvent.setup()
    open()
    expect(screen.getByRole('dialog', { name: 'Clone a repository to kingdom' })).toBeInTheDocument()
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Folder on kingdom to clone into' }).value).toBe('~/code')
    expect(cloneButton().disabled).toBe(true)
    await user.type(addressBox(), ADDRESS)
    expect(cloneButton().disabled).toBe(false)
    await user.clear(screen.getByRole('textbox', { name: 'Folder on kingdom to clone into' }))
    expect(cloneButton().disabled).toBe(true)
  })

  it('clones into the folder shown and hands back the new folder', async () => {
    const user = userEvent.setup()
    const { clone, onCloned } = open()
    await user.type(addressBox(), `  ${ADDRESS} `)
    await user.click(cloneButton())
    expect(clone).toHaveBeenCalledWith(ADDRESS, '~/code')
    expect(onCloned).toHaveBeenCalledWith('/home/dev/code/widgets', '~/code')
  })

  it('takes the folder from the machine\'s folder picker, and keeps the one shown when the picker is cancelled', async () => {
    const user = userEvent.setup()
    const chooseFolder = vi.fn<() => Promise<string | null>>().mockResolvedValueOnce('/srv/projects').mockResolvedValueOnce(null)
    open(undefined, chooseFolder)
    const folder = screen.getByRole<HTMLInputElement>('textbox', { name: 'Folder on kingdom to clone into' })
    await user.click(screen.getByRole('button', { name: 'Choose…' }))
    expect(folder.value).toBe('/srv/projects')
    await user.click(screen.getByRole('button', { name: 'Choose…' }))
    expect(folder.value).toBe('/srv/projects')
  })

  it('has no Choose button where there is no folder picker', () => {
    open()
    expect(screen.queryByRole('button', { name: 'Choose…' })).toBeNull()
  })

  it('says it is cloning and cannot be closed or sent twice until the clone has ended', async () => {
    const user = userEvent.setup()
    let finish: (folder: string) => void = () => undefined
    const { clone, onCancel, onCloned } = open(vi.fn<Clone>(() => new Promise<string>((resolve) => { finish = resolve })))
    await user.type(addressBox(), `${ADDRESS}{Enter}`)
    expect(screen.getByRole('status').textContent).toContain('Cloning…')
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Cloning…' }).disabled).toBe(true)
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Cancel' }).disabled).toBe(true)
    await user.keyboard('{Escape}')
    expect(onCancel).not.toHaveBeenCalled()
    expect(clone).toHaveBeenCalledOnce()
    act(() => { finish('/home/dev/code/widgets') })
    await vi.waitFor(() => { expect(onCloned).toHaveBeenCalledOnce() })
  })

  it('shows the machine\'s own reason when nothing was cloned and keeps what was typed', async () => {
    const user = userEvent.setup()
    const { onCloned } = open(vi.fn<Clone>(() => Promise.reject(new Error('/home/dev/code/widgets is already there'))))
    await user.type(addressBox(), ADDRESS)
    await user.click(cloneButton())
    expect((await screen.findByRole('alert')).textContent).toBe('/home/dev/code/widgets is already there')
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Repository address' }).value).toBe(ADDRESS)
    expect(cloneButton().disabled).toBe(false)
    expect(onCloned).not.toHaveBeenCalled()
  })

  it('closes on Cancel', async () => {
    const user = userEvent.setup()
    const { onCancel } = open()
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalledOnce()
  })
})

describe('the folder last cloned into', () => {
  it('is remembered for each machine by itself', () => {
    expect(lastCloneDestination('kingdom')).toBeNull()
    rememberCloneDestination('kingdom', '/srv/projects')
    expect(lastCloneDestination('Kingdom')).toBe('/srv/projects')
    expect(lastCloneDestination('this computer')).toBeNull()
  })

  it('is where the next clone on that machine starts, and the clone answers with the new folder', async () => {
    const user = userEvent.setup()
    api.cloneRepository.mockResolvedValue('/srv/projects/widgets')
    rememberCloneDestination('kingdom', '/srv/projects')
    const hook = renderHook(() => useCloneRepo({ machine: 3, where: 'kingdom', startIn: '~', chooseFolder: null }))
    let answered: Promise<string | null> = Promise.resolve(null)
    act(() => { answered = hook.result.current.open() })
    const view = render(<>{hook.result.current.dialog}</>)
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Folder on kingdom to clone into' }).value).toBe('/srv/projects')

    await user.clear(screen.getByRole('textbox', { name: 'Folder on kingdom to clone into' }))
    await user.type(screen.getByRole('textbox', { name: 'Folder on kingdom to clone into' }), '/srv/other')
    await user.type(addressBox(), ADDRESS)
    await user.click(cloneButton())

    await expect(answered).resolves.toBe('/srv/projects/widgets')
    expect(api.cloneRepository).toHaveBeenCalledWith(3, ADDRESS, '/srv/other')
    expect(lastCloneDestination('kingdom')).toBe('/srv/other')
    view.unmount()
  })
})
