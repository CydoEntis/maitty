import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { MachineActivity } from '@/api/machines'
import type { UpdateAction } from '../model/machines'
import { MachineUpdateDialog } from './MachineUpdateDialog'

function ask(action: UpdateAction, activity: Promise<MachineActivity | null> = Promise.resolve(null)) {
  const onUpdate = vi.fn()
  const onCancel = vi.fn()
  const checkActivity = vi.fn(() => activity)
  render(<MachineUpdateDialog name="box" action={action} checkActivity={checkActivity} onUpdate={onUpdate} onCancel={onCancel} />)
  return { onUpdate, onCancel, checkActivity }
}

describe('the question before the sidebar updates a machine', () => {
  it('says what is open there before an update, and updates now or when idle', async () => {
    const user = userEvent.setup()
    const { onUpdate } = ask('update', Promise.resolve({ sessions: 3, busy: 1, current: false }))
    expect(screen.getByRole('dialog', { name: 'Update Maitty on box' })).toHaveTextContent('box runs another version of Maitty.')
    expect(await screen.findByText(/box has 3 sessions open, 1 busy\./)).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Update when idle' }))
    expect(onUpdate).toHaveBeenCalledWith(true)
    await user.click(screen.getByRole('button', { name: 'Update now' }))
    expect(onUpdate).toHaveBeenLastCalledWith(false)
  })

  it('says restart, not update, when the app there is already this version', async () => {
    const user = userEvent.setup()
    const { onUpdate } = ask('update', Promise.resolve({ sessions: 1, busy: 0, current: true }))
    expect(await screen.findByRole('dialog', { name: 'Restart Maitty on box' })).toHaveTextContent('box already has this version; its daemon is still the old one.')
    expect(screen.queryByRole('button', { name: 'Update now' })).toBeNull()
    await user.click(screen.getByRole('button', { name: 'Restart now' }))
    expect(onUpdate).toHaveBeenCalledWith(false)
    expect(screen.getByRole('button', { name: 'Restart when idle' })).toBeInTheDocument()
  })

  it('cannot update until it knows what is open, and still asks when the machine cannot say', async () => {
    const { onUpdate } = ask('update', Promise.reject(new Error('no route')))
    expect(screen.getByRole('button', { name: 'Update now' })).toBeDisabled()
    expect(await screen.findByText(/box could not say what is open on it\./)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Update now' })).toBeEnabled()
    expect(onUpdate).not.toHaveBeenCalled()
  })

  it('installs on a machine with none without asking what is open there', async () => {
    const user = userEvent.setup()
    const { onUpdate, checkActivity } = ask('install')
    expect(screen.getByRole('dialog', { name: 'Install Maitty on box' })).toHaveTextContent('Maitty is not installed on box.')
    expect(screen.queryByRole('button', { name: 'Update when idle' })).toBeNull()
    await user.click(screen.getByRole('button', { name: 'Install now' }))
    expect(onUpdate).toHaveBeenCalledWith(false)
    expect(checkActivity).not.toHaveBeenCalled()
  })

  it('changes nothing when cancelled, by the button or Esc', async () => {
    const user = userEvent.setup()
    const { onUpdate, onCancel } = ask('install')
    await user.keyboard('{Escape}')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalledTimes(2)
    expect(onUpdate).not.toHaveBeenCalled()
  })
})
