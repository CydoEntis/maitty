import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { toSessionView } from '../model/session'
import { GetFileDialog } from './GetFileDialog'

const session = toSessionView(
  { id: 't2', generation: 0, kind: 'terminal', title: '', project: '', folder: '/srv/app', cwd: '/srv/app', repo: null, branch: 'working tree', provider: 'Codex', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: 2, hasDiff: false, agent: null, machine: 4, machineName: 'devbox' },
  '/home/me',
  { fleet: '4:1', worktree: false },
)

function open() {
  const onGet = vi.fn()
  const onCancel = vi.fn()
  render(<GetFileDialog session={session} onGet={onGet} onCancel={onCancel} />)
  return { onGet, onCancel, box: screen.getByRole<HTMLInputElement>('textbox', { name: 'Path on devbox' }) }
}

describe('the Get a file dialog', () => {
  it('names the machine and starts with the session folder, so only the file name is left to type', () => {
    const { box } = open()
    expect(screen.getByRole('dialog', { name: 'Get a file from devbox' })).toBeTruthy()
    expect(box.value).toBe('/srv/app/')
    expect(document.activeElement).toBe(box)
  })

  it('cannot get until a file is named', () => {
    open()
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Get file' }).disabled).toBe(true)
  })

  it('gets the path that was typed, trimmed', async () => {
    const user = userEvent.setup()
    const { onGet, box } = open()
    await user.type(box, 'out.log  ')
    await user.click(screen.getByRole('button', { name: 'Get file' }))
    expect(onGet).toHaveBeenCalledWith('/srv/app/out.log')
  })

  it('cancels with the button or Escape, and gets nothing', async () => {
    const user = userEvent.setup()
    const { onGet, onCancel } = open()
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    await user.keyboard('{Escape}')
    expect(onCancel).toHaveBeenCalledTimes(2)
    expect(onGet).not.toHaveBeenCalled()
  })
})
