import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const sync = vi.hoisted(() => ({ dirty: false, behind: 2, pulled: [] as string[] }))
function syncOf(folder: string) {
  return { branch: folder === '/dev/api' ? 'main' : 'dev', upstream: 'origin/dev', behind: sync.behind, ahead: 0, dirty: sync.dirty, fetched: true }
}
type Saved = { id: string; first_message: string; last_active: number }
const saved = vi.hoisted(() => ({ read: vi.fn<(agent: string, folder: string) => Promise<Saved[] | null>>(() => Promise.resolve([])) }))
vi.mock('@/api/daemon', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/daemon')>()), savedConversations: (agent: string, folder: string) => saved.read(agent, folder), loadAgentModels: () => Promise.resolve([]), folderSyncState: (folder: string) => Promise.resolve(syncOf(folder)), pullFolder: (folder: string) => { sync.pulled.push(folder); return Promise.resolve() }, folderBranches: () => Promise.resolve({ current: 'dev', branches: [{ name: 'dev', when: 10 }], worktrees: [{ path: '/dev/growth', branch: 'dev', main: true, when: 10 }] }) }))

const attach = vi.hoisted(() => ({ attachLocalFile: vi.fn(), chooseFilesToAttach: vi.fn() }))
vi.mock('@/api/attachments', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/attachments')>()), ...attach }))

import { NewChatPage, type NewChatRequest } from './NewChatPage'

const PROJECTS = [{ folder: '/dev/growth', name: 'growth', color: 'teal', sessions: 6 }, { folder: '/dev/api', name: 'api', color: 'blue', sessions: 2 }]

function page(onStart: (request: NewChatRequest) => Promise<void> = () => Promise.resolve()) {
  return render(
    <div className="mx-app">
      <NewChatPage machine={0} machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={onStart} onOtherMachine={() => undefined} onCancel={() => undefined} />
    </div>,
  )
}

function codexPage(onStart: () => Promise<void>, codexYolo = false) {
  return render(
    <div className="mx-app">
      <NewChatPage machine={0} machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="codex" codexYolo={codexYolo} browse={() => Promise.resolve(null)} onStart={onStart} onOtherMachine={() => undefined} onCancel={() => undefined} />
    </div>,
  )
}

/** The page opens on the most recent project: it is the first of the list. */
async function pickProject(name: string) {
  expect(await screen.findByRole('button', { name: `Project: ${name}` })).toBeInTheDocument()
}

describe('the new chat page', () => {
  it('opens ready to type on the most recent project, with no folder step', async () => {
    page()
    expect(screen.getByRole('heading', { name: 'What should we work on?' })).toBeInTheDocument()
    expect(await screen.findByRole('button', { name: 'Project: growth' })).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: 'Search projects or paste a path' })).not.toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'First message' })).toHaveFocus()
  })

  it('asks for a folder first only when there is no project: a search, and a pasted path that opens as a folder', async () => {
    render(
      <div className="mx-app">
        <NewChatPage machine={0} machines={[{ machine: 0, label: 'This computer', local: true }]} projects={[]} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={() => Promise.resolve()} onOtherMachine={() => undefined} onCancel={() => undefined} />
      </div>,
    )
    expect(screen.getByRole('button', { name: 'Start chat' })).toBeDisabled()
    expect(screen.getByText('No projects match. Paste a full path, or use Browse…')).toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: 'Search projects or paste a path' }), '/srv/new-app')
    await userEvent.click(within(screen.getByRole('list', { name: 'Open folder' })).getByRole('button', { name: /^Open new-app/ }))
    expect(await screen.findByRole('button', { name: 'Project: new-app' })).toBeInTheDocument()
  })

  it('starts the agent ahead only when it works in the checkout, and says none is wanted when the page goes', async () => {
    const onWarm = vi.fn()
    const { unmount } = render(
      <div className="mx-app">
        <NewChatPage machine={0} machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={() => Promise.resolve()} onWarm={onWarm} onOtherMachine={() => undefined} onCancel={() => undefined} />
      </div>,
    )
    // Own copy is on: a worktree is made when the chat starts, so there is no checkout to start the agent in ahead.
    await screen.findByRole('switch', { name: 'Own copy' })
    await userEvent.click(screen.getByRole('switch', { name: 'Own copy' }))
    await waitFor(() => { expect(onWarm).toHaveBeenLastCalledWith({ folder: '/dev/growth', agent: 'claude', model: 'default', reasoning: 'medium', mode: 'default' }) })

    unmount()
    expect(onWarm).toHaveBeenLastCalledWith(null)
  })

  it('shows the project, its branch and Own copy as three controls, with the tasks under them', async () => {
    page()
    expect(await screen.findByRole('button', { name: 'Project: growth' })).toBeInTheDocument()
    expect(await screen.findByRole('button', { name: /^Branch: [a-z]+-[a-z]+$/ })).toBeInTheDocument()
    expect(screen.getByRole('switch', { name: 'Own copy' })).toBeChecked()
    expect(within(screen.getByRole('list', { name: 'Suggested tasks' })).getAllByRole('button')).toHaveLength(3)
  })

  it('swaps the project from the dropdown under its button', async () => {
    page()
    await userEvent.click(await screen.findByRole('button', { name: 'Project: growth' }))
    const menu = await screen.findByRole('dialog')
    expect(within(menu).getByRole('radio', { name: /growth/ })).toHaveTextContent('6')
    await userEvent.click(within(menu).getByRole('radio', { name: /api/ }))
    expect(await screen.findByRole('button', { name: 'Project: api' })).toBeInTheDocument()
  })

  it('starts in a worktree of its own on a new branch by default, from where the checkout is', async () => {
    const onStart = vi.fn<(request: { message: string; branch?: unknown }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await screen.findByText(/Starts from dev · ↓2 behind origin\/dev/)
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'hello')
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    expect(onStart.mock.calls[0]?.[0].branch).toEqual({ name: expect.stringMatching(/^[a-z]+-[a-z]+$/) as unknown, create: true, worktree: true })
  })

  it('edits the checkout on its own branch when Own copy is turned off', async () => {
    const onStart = vi.fn<(request: { message: string; branch?: unknown }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await userEvent.click(await screen.findByRole('switch', { name: 'Own copy' }))
    expect(await screen.findByRole('button', { name: 'Branch: dev' })).toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'hello')
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    expect(onStart.mock.calls[0]?.[0].branch).toBeNull()
  })

  it('says how far behind the checkout is, and pulls it when it is clean', async () => {
    sync.dirty = false
    sync.behind = 2
    sync.pulled = []
    page()
    await userEvent.click(await screen.findByRole('switch', { name: 'Own copy' }))
    expect(await screen.findByText('↓2 behind origin/dev')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: '↓ Pull' }))
    await waitFor(() => { expect(sync.pulled).toEqual(['/dev/growth']) })
  })

  it('will not pull over uncommitted changes, and says why', async () => {
    sync.dirty = true
    sync.behind = 1
    page()
    await userEvent.click(await screen.findByRole('switch', { name: 'Own copy' }))
    const pull = await screen.findByRole('button', { name: '↓ Pull' })
    expect(pull).toBeDisabled()
    expect(pull).toHaveAttribute('title', 'You have uncommitted changes. Commit or stash first.')
    sync.dirty = false
  })

  it('offers the other recent projects as chips with their branch and how far behind they are', async () => {
    sync.behind = 3
    page()
    const chips = await screen.findByRole('group', { name: 'Switch to' })
    const api = await within(chips).findByRole('button', { name: /api/ })
    await waitFor(() => { expect(api).toHaveTextContent('main') })
    expect(api).toHaveTextContent('↓3')
    expect(api).toHaveAttribute('title', '/dev/api')
    await userEvent.click(api)
    expect(await screen.findByRole('button', { name: 'Project: api' })).toBeInTheDocument()
  })

  it('starts the chat with a suggested task when one is clicked', async () => {
    const onStart = vi.fn<(request: { message: string }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await pickProject('growth')
    await userEvent.click(await screen.findByRole('button', { name: /Review my uncommitted changes/ }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    expect(onStart.mock.calls[0]?.[0].message).toBe('Review my uncommitted changes')
  })

  it('attaches a picked file to the first message, shows it, and names it in the message', async () => {
    attach.chooseFilesToAttach.mockResolvedValue(['/home/me/errors.log'])
    attach.attachLocalFile.mockResolvedValue({ path: '/data/attachments/attach-1-errors.log', name: 'errors.log', size: 120, kind: 'file' })
    const onStart = vi.fn<(request: { message: string }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await pickProject('growth')
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'why does this fail?')
    await userEvent.click(screen.getByRole('button', { name: 'Attach files' }))
    expect(await screen.findByText('errors.log')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    const message = onStart.mock.calls[0]?.[0].message ?? ''
    expect(message).toContain('why does this fail?')
    expect(message).toContain('/data/attachments/attach-1-errors.log')
  })

  it('starts on medium effort and says so on the button and in the menu', async () => {
    page()
    expect(screen.getByRole('button', { name: /^Agent, model and reasoning: Claude · .* · Medium/ })).toHaveTextContent('Medium')
    await userEvent.click(screen.getByRole('button', { name: /^Agent, model and reasoning/ }))
    expect(screen.getByRole('radio', { name: 'Medium' })).toBeChecked()
  })

  it('has the agent, model and effort on one button under the message box, and the permission mode beside it', async () => {
    page()
    await userEvent.click(screen.getByRole('button', { name: /^Agent, model and reasoning: Claude/ }))
    expect(within(screen.getByRole('radiogroup', { name: 'Agent' })).getByRole('radio', { name: 'Claude' })).toBeChecked()
    expect(screen.getByRole('radiogroup', { name: 'Model' })).toBeInTheDocument()
    expect(screen.getByRole('radiogroup', { name: 'Effort' })).toBeInTheDocument()
    await userEvent.keyboard('{Escape}')
    expect(screen.getByRole('button', { name: /^Starting permission mode: Ask before edits/ })).toBeInTheDocument()
  })

  it('offers the three Codex modes for a Codex chat and starts it in the picked one', async () => {
    const onStart = vi.fn(() => Promise.resolve())
    codexPage(onStart)
    await pickProject('growth')
    await userEvent.click(screen.getByRole('button', { name: /^Starting permission mode: Auto/ }))
    expect(await screen.findAllByRole('menuitemradio')).toHaveLength(3)
    expect(screen.getByRole('menuitemradio', { name: /^Full access/ })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('menuitemradio', { name: /^Read only/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Start chat' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ agent: 'codex', mode: 'codexReadOnly' })) })
  })

  it('says a Codex chat starts with full access while its Yolo setting is on, and sends no mode of its own', async () => {
    const onStart = vi.fn(() => Promise.resolve())
    codexPage(onStart, true)
    expect(screen.getByText('Full access')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Starting permission mode/ })).toBeNull()
    await pickProject('growth')
    await userEvent.click(screen.getByRole('button', { name: 'Start chat' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ agent: 'codex', mode: 'default' })) })
  })
})

describe('resuming a saved conversation from the new chat page', () => {
  const ID = '0d0f5c1e-1111-4222-8333-444455556666'
  const HOURS_AGO_2 = Math.floor(Date.now() / 1000) - 7200
  beforeEach(() => { saved.read.mockReset(); saved.read.mockResolvedValue([]) })

  it('lists each conversation saved for the folder with its first message and how long ago it was active', async () => {
    saved.read.mockResolvedValue([{ id: ID, first_message: 'Fix the login redirect', last_active: HOURS_AGO_2 }])
    page()
    const list = await screen.findByRole('list', { name: 'Resume' })
    const row = within(list).getByRole('button', { name: /Fix the login redirect/ })
    expect(row).toHaveTextContent('2h')
    expect(saved.read).toHaveBeenLastCalledWith('claude', '/dev/growth')
  })

  it('starts the agent with the picked conversation in the folder it was saved in, without Own copy', async () => {
    saved.read.mockResolvedValue([{ id: ID, first_message: 'Fix the login redirect', last_active: HOURS_AGO_2 }])
    const onStart = vi.fn<(request: { resume: string | null; branch: unknown; folder: string }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await screen.findByRole('list', { name: 'Resume' })
    expect(screen.getByRole('switch', { name: 'Own copy' })).toBeChecked()
    expect(screen.getByText('A resumed conversation continues in growth, the folder it was saved in, not in a copy of its own.')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Fix the login redirect/ }))
    await waitFor(() => { expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ folder: '/dev/growth', resume: ID, branch: null })) })
  })

  it('says a typed message goes to the conversation picked, and sends it there', async () => {
    saved.read.mockResolvedValue([{ id: ID, first_message: 'Fix the login redirect', last_active: HOURS_AGO_2 }])
    const onStart = vi.fn<(request: NewChatRequest) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await screen.findByRole('list', { name: 'Resume' })
    const note = 'Your message is sent to the conversation you pick.'
    expect(screen.queryByText(note)).not.toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'now add a test')
    expect(screen.getByText(note)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Fix the login redirect/ }))
    await waitFor(() => { expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ resume: ID, message: 'now add a test' })) })
  })

  it('asks for the agent picked: Codex\'s own conversations for a Codex chat', async () => {
    codexPage(() => Promise.resolve())
    await waitFor(() => { expect(saved.read).toHaveBeenLastCalledWith('codex', '/dev/growth') })
  })

  it('shows no list when the folder has no saved conversations, and a new chat starts as ever', async () => {
    const onStart = vi.fn<(request: { resume: string | null }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await waitFor(() => { expect(saved.read).toHaveBeenCalled() })
    expect(screen.queryByRole('list', { name: 'Resume' })).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Start chat' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ resume: null })) })
  })

  it('says in one quiet line that they could not be read, and a new chat still starts', async () => {
    saved.read.mockRejectedValue(new Error('the local daemon did not answer'))
    const onStart = vi.fn(() => Promise.resolve())
    page(onStart)
    expect(await screen.findByText('Could not read the saved conversations.')).toBeInTheDocument()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Start chat' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
  })

  it('says in one quiet line when the service running is too old to list them', async () => {
    saved.read.mockResolvedValue(null)
    page()
    expect(await screen.findByText('Restart the Maitty service to see saved conversations.')).toBeInTheDocument()
  })

  it('never lets a late answer for the folder left behind replace the one picked', async () => {
    let answerGrowth: (found: Saved[]) => void = () => undefined
    saved.read.mockImplementation((_agent, folder) => (folder === '/dev/growth' ? new Promise((resolve) => { answerGrowth = resolve }) : Promise.resolve([])))
    page()
    await waitFor(() => { expect(saved.read).toHaveBeenCalledWith('claude', '/dev/growth') })
    await userEvent.click(within(await screen.findByRole('group', { name: 'Switch to' })).getByRole('button', { name: /api/ }))
    await waitFor(() => { expect(saved.read).toHaveBeenLastCalledWith('claude', '/dev/api') })
    answerGrowth([{ id: ID, first_message: 'Work in growth', last_active: HOURS_AGO_2 }])
    await new Promise((resolve) => { setTimeout(resolve, 20) })
    expect(screen.queryByRole('list', { name: 'Resume' })).not.toBeInTheDocument()
  })

  it('is not offered on another machine, whose saved conversations this computer cannot read', async () => {
    render(
      <div className="mx-app">
        <NewChatPage machine={2} machines={[{ machine: 0, label: 'This computer', local: true }, { machine: 2, label: 'Shipyard', local: false }]} projects={[{ folder: '/srv/app', name: 'app', color: 'teal', sessions: 3 }]} defaultAgent="claude" browse={null} onStart={() => Promise.resolve()} onOtherMachine={() => undefined} onCancel={() => undefined} />
      </div>,
    )
    await pickProject('app')
    expect(saved.read).not.toHaveBeenCalled()
  })
})

describe('the new chat page opened from a full chat', () => {
  it('opens on that chat\'s folder with the summary as the first message, ready to edit', async () => {
    render(
      <div className="mx-app">
        <NewChatPage machine={0} machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={() => Promise.resolve()} onOtherMachine={() => undefined} onCancel={() => undefined} seed={{ folder: '/dev/api', message: 'This continues an earlier chat' }} />
      </div>,
    )
    expect(await screen.findByRole('button', { name: 'Project: api' })).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'First message' })).toHaveValue('This continues an earlier chat')
  })
})

describe('the new chat page on another machine', () => {
  const MACHINES = [{ machine: 0, label: 'This computer', local: true }, { machine: 2, label: 'Shipyard', local: false }]
  const folders = [{ folder: '/srv/app', name: 'app', color: 'teal', sessions: 3 }]
  function remotePage(onStart: (request: { machine: number; machineName: string; branch: unknown; folder: string; message: string }) => Promise<void>, onOtherMachine = vi.fn()) {
    return render(
      <div className="mx-app">
        <NewChatPage machine={2} machines={MACHINES} projects={folders} defaultAgent="claude" browse={null} onStart={onStart} onOtherMachine={onOtherMachine} onCancel={() => undefined} />
      </div>,
    )
  }

  it('lists that machine\'s projects, has no folder picker, and starts there with no branch', async () => {
    const onStart = vi.fn(() => Promise.resolve())
    remotePage(onStart)
    await pickProject('app')
    expect(screen.queryByRole('button', { name: /^Branch/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Attach files' })).not.toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'hello')
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ machine: 2, machineName: 'Shipyard', folder: '/srv/app', message: 'hello', branch: null }))
  })

  it('offers the agent only on the model button: no model list or effort that this computer would have to guess', async () => {
    remotePage(() => Promise.resolve())
    await pickProject('app')
    await userEvent.click(screen.getByRole('button', { name: /^Agent, / }))
    expect(screen.getByRole('radiogroup', { name: 'Agent' })).toBeInTheDocument()
    expect(screen.queryByRole('radiogroup', { name: 'Model' })).not.toBeInTheDocument()
    expect(screen.queryByRole('radiogroup', { name: 'Effort' })).not.toBeInTheDocument()
  })

  it('offers this computer as another place to start', async () => {
    const onOtherMachine = vi.fn()
    remotePage(() => Promise.resolve(), onOtherMachine)
    await userEvent.click(await screen.findByRole('button', { name: 'Project: app' }))
    expect(screen.queryByRole('button', { name: /Browse for a folder/ })).not.toBeInTheDocument()
    await userEvent.click(await screen.findByRole('button', { name: /Start on This computer/ }))
    expect(onOtherMachine).toHaveBeenCalledWith(0)
  })
})
