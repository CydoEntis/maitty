import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { GitConfigWrite, GitRuleChange } from '@/api/gitIdentities'
import { GitRuleDialog } from './GitRuleDialog'

const git = vi.hoisted(() => ({ changeGitRule: vi.fn<(machine: number, change: GitRuleChange) => Promise<GitConfigWrite[]>>() }))
vi.mock('@/api/gitIdentities', () => git)

const LINES: GitConfigWrite[] = [{ file: 'C:/Users/dev/.gitconfig', lines: ['includeIf.gitdir/i:C:/work/.path = C:/data/git-identities/ab.gitconfig'] }]
const IDENTITY = { name: 'Casey', email: 'casey@work.invalid', login: 'work-me' }

afterEach(cleanup)
beforeEach(() => { git.changeGitRule.mockReset() })

function open(props: { chooseFolder?: () => Promise<string | null> } = {}) {
  const onWritten = vi.fn()
  render(
    <GitRuleDialog machine={0} where="This computer" logins={['me', 'work-me']} chooseFolder={props.chooseFolder ?? (() => Promise.resolve('C:/work'))} onWritten={onWritten} onCancel={vi.fn()} />,
  )
  return { onWritten }
}

async function fill(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Choose…' }))
  await user.selectOptions(screen.getByRole('combobox', { name: 'GitHub account' }), 'work-me')
  await user.type(screen.getByRole('textbox', { name: 'Commit name' }), IDENTITY.name)
  await user.type(screen.getByRole('textbox', { name: 'Commit email' }), IDENTITY.email)
}

describe('adding a folder rule', () => {
  it('asks for the lines first and sends nothing to write until Write these lines', async () => {
    const user = userEvent.setup()
    git.changeGitRule.mockResolvedValue(LINES)
    const { onWritten } = open()
    await fill(user)
    expect(screen.getByRole('textbox', { name: 'Folder on This computer' })).toHaveValue('C:/work')
    await user.click(screen.getByRole('button', { name: 'Show the lines' }))
    expect(await screen.findByText(LINES[0]?.lines[0] ?? '')).toBeInTheDocument()
    expect(git.changeGitRule).toHaveBeenCalledExactlyOnceWith(0, { kind: 'preview', folder: 'C:/work', identity: IDENTITY })
    expect(onWritten).not.toHaveBeenCalled()
    await user.click(screen.getByRole('button', { name: 'Write these lines' }))
    expect(git.changeGitRule).toHaveBeenLastCalledWith(0, { kind: 'apply', folder: 'C:/work', identity: IDENTITY })
    expect(onWritten).toHaveBeenCalledOnce()
  })

  it('takes the preview away when a field changes, so only what was shown is written', async () => {
    const user = userEvent.setup()
    git.changeGitRule.mockResolvedValue(LINES)
    open()
    await fill(user)
    await user.click(screen.getByRole('button', { name: 'Show the lines' }))
    await screen.findByRole('button', { name: 'Write these lines' })
    await user.type(screen.getByRole('textbox', { name: 'Commit email' }), 'x')
    expect(screen.queryByRole('button', { name: 'Write these lines' })).toBeNull()
    expect(screen.queryByRole('group', { name: 'Lines to be written' })).toBeNull()
  })

  it('shows the machine\'s reason and writes nothing when it refuses the folder', async () => {
    const user = userEvent.setup()
    git.changeGitRule.mockRejectedValue(new Error('a folder with * cannot have a rule'))
    const { onWritten } = open()
    await fill(user)
    await user.click(screen.getByRole('button', { name: 'Show the lines' }))
    expect((await screen.findByRole('alert')).textContent).toBe('a folder with * cannot have a rule')
    expect(screen.queryByRole('button', { name: 'Write these lines' })).toBeNull()
    expect(onWritten).not.toHaveBeenCalled()
  })

  it('cannot ask for the lines until every field is filled', async () => {
    const user = userEvent.setup()
    open()
    await user.type(screen.getByRole('textbox', { name: 'Commit name' }), 'Casey')
    expect(screen.getByRole('button', { name: 'Show the lines' })).toBeDisabled()
  })
})
