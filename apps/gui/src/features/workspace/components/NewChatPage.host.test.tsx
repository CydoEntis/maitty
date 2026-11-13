import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const host = vi.hoisted(() => ({
  installed: ['claude', 'opencode'] as string[],
  branches: null as null | { current: string | null; branches: { name: string; current: boolean; when: number | null }[]; worktrees: { path: string; branch: string | null; main: boolean; when: number | null }[] },
  openAgentDocs: vi.fn(() => Promise.resolve()),
}))

vi.mock('@/api/daemon', () => ({
  loadAgentModels: () => Promise.resolve([]),
  agentsInstalled: () => Promise.resolve(host.installed),
  folderBranches: () => Promise.resolve(host.branches),
  folderSyncState: () => Promise.resolve(null),
  pullFolder: () => Promise.resolve(),
  savedConversations: () => Promise.resolve([]),
  openAgentDocs: host.openAgentDocs,
}))

import { NewChatPage, type NewChatRequest } from './NewChatPage'

const PROJECTS = ['a', 'b', 'c', 'd', 'e'].map((name) => ({ folder: `/dev/${name}`, name, color: 'teal', sessions: 1 }))
const NOW = Math.floor(Date.now() / 1000)
const GIT = {
  current: 'main',
  branches: [
    { name: 'main', current: true, when: NOW - 3600 },
    { name: 'dev', current: false, when: NOW - 120 },
    { name: 'agent/grow-028', current: false, when: NOW - 240 },
  ],
  worktrees: [
    { path: '/dev/a', branch: 'main', main: true, when: NOW - 3600 },
    { path: '/dev/a-agent-grow-028', branch: 'agent/grow-028', main: false, when: NOW - 240 },
  ],
}

function page(onStart: (request: NewChatRequest) => Promise<void> = () => Promise.resolve(), onAddMachine?: () => void) {
  return render(
    <div className="mx-app">
      <NewChatPage machine={0} machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)}
        onStart={onStart} onOtherMachine={() => undefined} {...(onAddMachine ? { onAddMachine } : {})} onCancel={() => undefined} />
    </div>,
  )
}

beforeEach(() => {
  host.installed = ['claude', 'opencode']
  host.branches = null
  host.openAgentDocs.mockClear()
})

/** The page opens on the most recent project, the first of the list. */
async function pickProject(name: string) {
  expect(await screen.findByRole('button', { name: `Project: ${name}` })).toBeInTheDocument()
}

/** Own copy is on by default; the branch menu below speaks of the checkout, so these turn it off first. */
async function editCheckout() {
  await userEvent.click(await screen.findByRole('switch', { name: 'Own copy' }))
}

describe('the new chat page, against this computer', () => {
  it('shows no branch button for a folder that is not a git repository', async () => {
    page()
    await pickProject('a')
    expect(await screen.findByRole('button', { name: 'Project: a' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Branch/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('switch', { name: 'Own copy' })).not.toBeInTheDocument()
  })

  it('lists the real branches and worktrees with their ages, the current branch ticked', async () => {
    host.branches = GIT
    page()
    await pickProject('a')
    await editCheckout()
    await userEvent.click(await screen.findByRole('button', { name: 'Branch: main' }))
    const menu = await screen.findByRole('dialog')
    expect(within(menu).getByRole('radio', { name: /^dev/ })).toHaveTextContent('2m')
    expect(within(menu).getByRole('radio', { name: /^main/ })).toBeChecked()
    expect(within(menu).getByRole('group', { name: 'Worktrees' })).toHaveTextContent('agent/grow-028')
  })

  it('starts the chat on a branch that was picked, or on a new one that was typed', async () => {
    host.branches = GIT
    const started: NewChatRequest[] = []
    page((request) => { started.push(request); return Promise.resolve() })
    await pickProject('a')
    await editCheckout()
    await userEvent.click(await screen.findByRole('button', { name: 'Branch: main' }))
    await userEvent.click(within(await screen.findByRole('dialog')).getByRole('radio', { name: /^dev/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Start chat' }))
    expect(started[0]?.branch).toEqual({ name: 'dev', create: false, worktree: false })

    await userEvent.click(screen.getByRole('switch', { name: 'Own copy' }))
    await userEvent.click(await screen.findByRole('button', { name: /^Branch: / }))
    await userEvent.type(screen.getByRole('textbox', { name: 'Search or create a branch' }), 'agent/grow-030')
    await userEvent.click(screen.getByRole('radio', { name: 'Create branch agent/grow-030' }))
    await userEvent.click(screen.getByRole('button', { name: 'Start chat' }))
    expect(started[1]?.branch).toEqual({ name: 'agent/grow-030', create: true, worktree: true })
  })

  it('splits the projects in the dropdown into the recent few and the rest', async () => {
    page()
    await pickProject('a')
    await userEvent.click(await screen.findByRole('button', { name: 'Project: a' }))
    const menu = await screen.findByRole('dialog')
    expect(within(within(menu).getByRole('group', { name: 'Recent projects' })).getAllByRole('radio')).toHaveLength(3)
    expect(within(within(menu).getByRole('group', { name: 'All on this computer' })).getAllByRole('radio')).toHaveLength(2)
  })

  it('switches the agent from the model button, and sends one that is not installed to its setup page', async () => {
    page()
    await pickProject('a')
    await userEvent.click(screen.getByRole('button', { name: /^Agent, model and reasoning: Claude/ }))
    await userEvent.click(within(screen.getByRole('radiogroup', { name: 'Agent' })).getByRole('radio', { name: 'Codex' }))
    await userEvent.keyboard('{Escape}')
    expect(await screen.findByText(/Codex is not installed on this computer/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Start chat' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'set it up' }))
    expect(host.openAgentDocs).toHaveBeenCalledWith('codex')
  })

  it('will not start with an agent that is not installed', async () => {
    host.installed = ['codex']
    page()
    await pickProject('a')
    expect(await screen.findByText(/Claude is not installed on this computer/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Start chat' })).toBeDisabled()
  })

  it('offers adding a machine under the projects when the window can', async () => {
    const add = vi.fn()
    page(undefined, add)
    await userEvent.click(await screen.findByRole('button', { name: 'Project: a' }))
    await userEvent.click(await screen.findByRole('button', { name: /Add a machine/ }))
    expect(add).toHaveBeenCalledOnce()
  })
})
