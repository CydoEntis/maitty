import { beforeEach, describe, expect, it, vi } from 'vitest'
import { updateMachine } from '@/api/machines'
import { runMachineUpdate } from './machineUpdate'

const toasts = vi.hoisted(() => ({ loading: vi.fn(() => 'toast-1'), success: vi.fn(), error: vi.fn() }))
vi.mock('sonner', () => ({ toast: toasts }))
vi.mock('@/api/machines', async (importOriginal) => ({ ...await importOriginal<typeof import('@/api/machines')>(), updateMachine: vi.fn() }))

beforeEach(() => { vi.clearAllMocks() })

describe('installing or updating a machine', () => {
  it('passes --install for a machine with none, says it is running, then how it ended, and dials it again', async () => {
    vi.mocked(updateMachine).mockResolvedValue('box: maitty 0.67.0 installed to ~/.local/bin/maitty')
    const onDone = vi.fn()
    await runMachineUpdate('box', 'install', false, onDone)
    expect(updateMachine).toHaveBeenCalledWith('box', true, false)
    expect(toasts.loading).toHaveBeenCalledWith('Installing Maitty on box…')
    expect(toasts.success).toHaveBeenCalledWith('box: maitty 0.67.0 installed to ~/.local/bin/maitty', { id: 'toast-1' })
    expect(onDone).toHaveBeenCalledTimes(1)
  })

  it('replaces another version without --install, and keeps the daemon note it gave', async () => {
    vi.mocked(updateMachine).mockResolvedValue('box: maitty 0.67.0 is already up to date\nnote: could not ask the daemon (x); running sessions are untouched')
    await runMachineUpdate('box', 'update', true, vi.fn())
    expect(updateMachine).toHaveBeenCalledWith('box', false, true)
    expect(toasts.success).toHaveBeenCalledWith('box: maitty 0.67.0 is already up to date', { id: 'toast-1', description: 'note: could not ask the daemon (x); running sessions are untouched' })
  })

  it.each([
    ['a newer version there', 'maitty: box has a newer maitty than this one; update this computer, or run `maitty machine update box --force` to downgrade it', 'box has a newer maitty than this one; update this computer, or run `maitty machine update box --force` to downgrade it'],
    ['ssh that cannot get in', 'could not reach box: Permission denied; make plain `ssh` to that machine work in a terminal, then retry', 'could not reach box: Permission denied; make plain `ssh` to that machine work in a terminal, then retry'],
    ['a Windows machine', 'maitty: Windows machines are not supported as a remote in v1', 'Windows machines are not supported as a remote in v1'],
  ])('says why it did not, for %s, and does not dial it again', async (_case, said, shown) => {
    vi.mocked(updateMachine).mockRejectedValue(said)
    const onDone = vi.fn()
    await runMachineUpdate('box', 'update', false, onDone)
    expect(toasts.error).toHaveBeenCalledWith('Could not update Maitty on box', { id: 'toast-1', description: shown })
    expect(onDone).not.toHaveBeenCalled()
  })
})
