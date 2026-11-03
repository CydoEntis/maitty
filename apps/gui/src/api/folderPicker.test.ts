import { beforeEach, describe, expect, it, vi } from 'vitest'
import { open } from '@tauri-apps/plugin-dialog'
import { chooseProjectFolder } from './folderPicker'

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))

describe('chooseProjectFolder', () => {
  beforeEach(() => {
    vi.mocked(open).mockReset()
  })

  it('opens a single-directory picker and returns the selected path', async () => {
    vi.mocked(open).mockResolvedValueOnce('/home/casey/projects/maitty')

    await expect(chooseProjectFolder()).resolves.toBe('/home/casey/projects/maitty')
    expect(open).toHaveBeenCalledWith({ directory: true, multiple: false })
  })

  it('opens at the folder it is given', async () => {
    vi.mocked(open).mockResolvedValueOnce('/home/casey/dev')

    await chooseProjectFolder('/home/casey/dev')

    expect(open).toHaveBeenCalledWith({ directory: true, multiple: false, defaultPath: '/home/casey/dev' })
  })

  it('returns no path when the picker is canceled', async () => {
    vi.mocked(open).mockResolvedValueOnce(null)

    await expect(chooseProjectFolder()).resolves.toBeNull()
  })
})
