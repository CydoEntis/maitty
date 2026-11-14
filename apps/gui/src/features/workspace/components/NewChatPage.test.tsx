import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const sync = vi.hoisted(() => ({ dirty: false, behind: 2, pulled: [] as string[] }))
function syncOf(folder: string) {
  return { branch: folder === '/dev/api' ? 'main' : 'dev', upstream: 'origin/dev', behind: sync.behind, ahead: 0, dirty: sync.dirty, fetched: true }
}
type Saved = { id: string; first_message: string; last_active: number }
const saved = vi.hoisted(() => ({ read: vi.fn<(agent: string, folder: string) => Promise<Saved[] | null>>(() => Promise.resolve([])) }))
vi.mock('@/api/daemon', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/daemon')>()), savedConversations: (agent: string, folder: string) => saved.read(agent, folder), agentsInstalled: () => Promise.resolve(['claude', 'codex', 'gemini']), loadAgentModels: () => Promise.resolve([]), folderSyncState: (folder: string) => Promise.resolve(syncOf(folder)), pullFolder: (folder: string) => { sync.pulled.push(folder); return Promise.resolve() }, folderBranches: () => Promise.resolve({ current: 'dev', branches: [{ name: 'dev', when: 10 }], worktrees: [{ path: '/dev/growth', branch: 'dev', main: true, when: 10 }] }) }))

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
/** Waits until the page has picked this project: the Where pill in the tray names it. */
async function pickProject(name: string) {
  expect(await screen.findByRole('button', { name: new RegExp(`^Where this chat runs: ${name}( ·|,|$)`) })).toBeInTheDocument()
}


/** Opens the Where popover in the message box's tray, unless it is open, so the project, branch and Work in controls can be found. */
/** The menu opened last: the Where popover stays open under a project or branch menu, and both are dialogs. */
async function lastPopup() {
  const open = await screen.findAllByRole('dialog')
  const last = open.at(-1)
  if (last === undefined) throw new Error('no menu is open')
  return last
}

async function inWhere() {
  const pill = await screen.findByRole('button', { name: /^Where this chat runs/ })
  if (pill.getAttribute('aria-expanded') !== 'true') await userEvent.click(pill)
}
describe('the new chat page', () => {
  it('opens ready to type on the most recent project, with no folder step', async () => {
    page()
    expect(screen.getByRole('heading', { name: 'What should we work on?' })).toBeInTheDocument()
    await pickProject('growth')
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
    await inWhere()
    expect(await screen.findByRole('button', { name: 'Project: new-app' })).toBeInTheDocument()
  })

  it('starts the agent ahead only when it works in the checkout, and says none is wanted when the page goes', async () => {
    const onWarm = vi.fn()
    const { unmount } = render(
      <div className="mx-app">
        <NewChatPage machine={0} machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={() => Promise.resolve()} onWarm={onWarm} onOtherMachine={() => undefined} onCancel={() => undefined} />
      </div>,
    )
    // New worktree is chosen: a worktree is made when the chat starts, so there is no checkout to start the agent in ahead.
    await inWhere()
    await screen.findByRole('radio', { name: 'This folder' })
    await inWhere()
    await userEvent.click(screen.getByRole('radio', { name: 'This folder' }))
    await waitFor(() => { expect(onWarm).toHaveBeenLastCalledWith({ folder: '/dev/growth', agent: 'claude', model: 'default', reasoning: 'medium', mode: 'default' }) })

    unmount()
    expect(onWarm).toHaveBeenLastCalledWith(null)
  })

  it('puts the project, its branch and where to work in the Where popover, with the tasks on the page', async () => {
    page()
    await inWhere()
    expect(await screen.findByRole('button', { name: 'Project: growth' })).toBeInTheDocument()
    await inWhere()
    expect(await screen.findByRole('button', { name: /^Branch: [a-z]+-[a-z]+$/ })).toBeInTheDocument()
    await inWhere()
    expect(screen.getByRole('radio', { name: 'New worktree' })).toBeChecked()
    expect(within(screen.getByRole('list', { name: 'Suggested tasks' })).getAllByRole('button')).toHaveLength(3)
  })

  it('swaps the project from the dropdown under its button', async () => {
    page()
    await inWhere()
    await userEvent.click(await screen.findByRole('button', { name: 'Project: growth' }))
    const menu = await lastPopup()
    expect(within(menu).getByRole('radio', { name: /growth/ })).toHaveTextContent('6')
    await userEvent.click(within(menu).getByRole('radio', { name: /api/ }))
    await inWhere()
    expect(await screen.findByRole('button', { name: 'Project: api' })).toBeInTheDocument()
  })

  it('starts in a worktree of its own on a new branch by default, from where the checkout is', async () => {
    const onStart = vi.fn<(request: { message: string; branch?: unknown }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await inWhere()
    await screen.findByText(/Starts from dev · ↓2 behind origin\/dev/)
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'hello')
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    expect(onStart.mock.calls[0]?.[0].branch).toEqual({ name: 'hello', create: true, worktree: true })
  })

  it('names the new branch after the message as it is typed, and two words while it has none', async () => {
    page()
    await inWhere()
    expect(await screen.findByRole('button', { name: /^Branch: [a-z]+-[a-z]+$/ })).toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'Fix the login bug')
    await inWhere()
    expect(screen.getByRole('button', { name: 'Branch: fix-the-login-bug' })).toBeInTheDocument()
    await userEvent.clear(screen.getByRole('textbox', { name: 'First message' }))
    await inWhere()
    expect(screen.getByRole('button', { name: /^Branch: [a-z]+-[a-z]+$/ })).toBeInTheDocument()
  })

  it('names the branch after a suggested task when that is what is sent', async () => {
    const onStart = vi.fn<(request: NewChatRequest) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await inWhere()
    await screen.findByRole('button', { name: /^Branch: [a-z]+-[a-z]+$/ })
    await userEvent.click(screen.getByRole('button', { name: /Review my uncommitted changes/ }))
    await waitFor(() => { expect(onStart.mock.calls[0]?.[0].branch?.name).toBe('review-my-uncommitted-changes') })
  })

  it('keeps the two-word name for a page opened with a message, such as a chat’s summary', async () => {
    render(
      <div className="mx-app">
        <NewChatPage machine={0} machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={() => Promise.resolve()} onOtherMachine={() => undefined} onCancel={() => undefined}
          seed={{ folder: '/dev/growth', message: 'This continues an earlier chat whose context was full.' }} />
      </div>,
    )
    await inWhere()
    expect(await screen.findByRole('button', { name: /^Branch: [a-z]+-[a-z]+$/ })).toBeInTheDocument()
  })

  it('stops following the message once a branch is named', async () => {
    const onStart = vi.fn<(request: NewChatRequest) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await inWhere()
    await userEvent.click(await screen.findByRole('button', { name: /^Branch: / }))
    await userEvent.type(screen.getByRole('textbox', { name: 'Search or create a branch' }), 'my-own')
    await userEvent.click(screen.getByRole('radio', { name: 'Create branch my-own' }))
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'Fix the login bug')
    await inWhere()
    expect(screen.getByRole('button', { name: 'Branch: my-own' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart.mock.calls[0]?.[0].branch?.name).toBe('my-own') })
  })

  it('edits the checkout on its own branch when This folder is chosen', async () => {
    const onStart = vi.fn<(request: { message: string; branch?: unknown }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await inWhere()
    await userEvent.click(await screen.findByRole('radio', { name: 'This folder' }))
    await inWhere()
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
    await inWhere()
    await userEvent.click(await screen.findByRole('radio', { name: 'This folder' }))
    expect(await screen.findByText('↓2 behind origin/dev')).toBeInTheDocument()
    await inWhere()
    await userEvent.click(screen.getByRole('button', { name: '↓ Pull' }))
    await waitFor(() => { expect(sync.pulled).toEqual(['/dev/growth']) })
  })

  it('will not pull over uncommitted changes, and says why', async () => {
    sync.dirty = true
    sync.behind = 1
    page()
    await inWhere()
    await userEvent.click(await screen.findByRole('radio', { name: 'This folder' }))
    await inWhere()
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
    await inWhere()
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

  it('starts the agent with the picked conversation in the folder it was saved in, without a new worktree', async () => {
    saved.read.mockResolvedValue([{ id: ID, first_message: 'Fix the login redirect', last_active: HOURS_AGO_2 }])
    const onStart = vi.fn<(request: { resume: string | null; branch: unknown; folder: string }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await screen.findByRole('list', { name: 'Resume' })
    await inWhere()
    expect(screen.getByRole('radio', { name: 'New worktree' })).toBeChecked()
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
    await inWhere()
    expect(await screen.findByRole('button', { name: 'Project: api' })).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'First message' })).toHaveValue('This continues an earlier chat')
  })
})

describe('starting a chat and going on writing', () => {
  type Start = (request: NewChatRequest, options?: { background?: boolean }) => Promise<void>
  const box = () => screen.getByRole('textbox', { name: 'First message' })
  async function ready() {
    await inWhere()
    await screen.findByRole('button', { name: /^Branch: [a-z]+-[a-z]+$/ })
  }

  it('starts the chat in the background on Ctrl+Enter, and empties the box with every other choice kept', async () => {
    const onStart = vi.fn<Start>().mockResolvedValue(undefined)
    page(onStart)
    await ready()
    await userEvent.click(screen.getByRole('button', { name: /^Starting permission mode/ }))
    await userEvent.click(screen.getByRole('menuitemradio', { name: /^Plan mode/ }))
    await userEvent.type(box(), 'first task')
    await userEvent.keyboard('{Control>}{Enter}{/Control}')

    expect(onStart).toHaveBeenCalledWith(expect.objectContaining({ folder: '/dev/growth', message: 'first task', mode: 'plan' }), { background: true })
    expect(box()).toHaveValue('')
    expect(screen.getByRole('heading', { name: 'What should we work on?' })).toBeInTheDocument()
    await inWhere()
    expect(screen.getByRole('button', { name: 'Project: growth' })).toBeInTheDocument()
    await inWhere()
    expect(screen.getByRole('radio', { name: 'New worktree' })).toBeChecked()
    expect(screen.getByRole('button', { name: /^Starting permission mode: Plan mode/ })).toBeInTheDocument()
  })

  it('starts a second while the first is still starting, each in a worktree of its own', async () => {
    const onStart = vi.fn<Start>(() => new Promise(() => undefined))
    page(onStart)
    await ready()
    await userEvent.type(box(), 'first task')
    await userEvent.keyboard('{Control>}{Enter}{/Control}')
    await userEvent.type(box(), 'second task')
    await userEvent.keyboard('{Meta>}{Enter}{/Meta}')

    expect(onStart.mock.calls.map(([request]) => request.message)).toEqual(['first task', 'second task'])
    const [first, second] = onStart.mock.calls.map(([request]) => request.branch)
    expect(first).toMatchObject({ create: true, worktree: true })
    expect(second).toMatchObject({ create: true, worktree: true })
    expect(second?.name).not.toBe(first?.name)
  })

  it('names a start that failed and puts its message back after what was typed since', async () => {
    let fail: (reason: Error) => void = () => undefined
    const onStart = vi.fn<Start>(() => new Promise((_, reject) => { fail = reject }))
    page(onStart)
    await ready()
    await userEvent.type(box(), 'fix the login')
    await userEvent.keyboard('{Control>}{Enter}{/Control}')
    await userEvent.type(box(), 'a new draft')
    fail(new Error('The worktree could not be made'))

    expect(await screen.findByText(/Could not start “fix the login”: The worktree could not be made/)).toBeInTheDocument()
    expect(box()).toHaveValue('a new draft')
    await userEvent.click(screen.getByRole('button', { name: 'Put back “fix the login”' }))
    expect(box()).toHaveValue('a new draft\n\nfix the login')
    expect(screen.queryByText(/Could not start/)).not.toBeInTheDocument()
  })

  it('starts the same message with each agent ticked, each on a new branch in its own worktree and on its own defaults', async () => {
    const onStart = vi.fn<Start>(() => new Promise(() => undefined))
    page(onStart)
    await ready()
    await userEvent.click(screen.getByRole('button', { name: /^Starting permission mode/ }))
    await userEvent.click(screen.getByRole('menuitemradio', { name: /^Plan mode/ }))
    await userEvent.click(within(screen.getByRole('group', { name: 'Also start with' })).getByRole('checkbox', { name: 'Codex' }))
    await userEvent.type(box(), 'fix the login{Enter}')

    expect(onStart).toHaveBeenCalledTimes(2)
    const [main, extra] = onStart.mock.calls
    expect(main).toEqual([expect.objectContaining({ agent: 'claude', mode: 'plan', message: 'fix the login' })])
    expect(extra).toEqual([expect.objectContaining({ agent: 'codex', model: 'default', reasoning: 'medium', mode: 'codexAuto', message: 'fix the login', resume: null }), { background: true }])
    expect(extra?.[0].branch).toMatchObject({ create: true, worktree: true })
    expect(extra?.[0].branch?.name).not.toBe(main?.[0].branch?.name)
  })

  it('names each ticked agent’s branch after the message too, each with its own number', async () => {
    const onStart = vi.fn<Start>(() => new Promise(() => undefined))
    page(onStart)
    await ready()
    const also = within(screen.getByRole('group', { name: 'Also start with' }))
    await userEvent.click(also.getByRole('checkbox', { name: 'Codex' }))
    await userEvent.click(also.getByRole('checkbox', { name: 'Gemini' }))
    await userEvent.type(box(), 'fix the login{Enter}')

    expect(onStart.mock.calls.map(([request]) => request.branch?.name)).toEqual(['fix-the-login', 'fix-the-login-2', 'fix-the-login-3'])
  })

  it('names each ticked agent’s branch after a suggested task when that is what is sent', async () => {
    const onStart = vi.fn<Start>(() => new Promise(() => undefined))
    page(onStart)
    await ready()
    await userEvent.click(within(screen.getByRole('group', { name: 'Also start with' })).getByRole('checkbox', { name: 'Codex' }))
    await userEvent.click(screen.getByRole('button', { name: /Review my uncommitted changes/ }))

    expect(onStart.mock.calls.map(([request]) => request.branch?.name)).toEqual(['review-my-uncommitted-changes', 'review-my-uncommitted-changes-2'])
  })

  it('names a failed main start by its agent when others went with it, and a retry starts only the main agent', async () => {
    let fail: (reason: Error) => void = () => undefined
    const onStart = vi.fn<Start>((request) => new Promise((_, reject) => { if (request.agent === 'claude') fail = reject }))
    page(onStart)
    await ready()
    await userEvent.click(within(screen.getByRole('group', { name: 'Also start with' })).getByRole('checkbox', { name: 'Codex' }))
    await userEvent.type(box(), 'fix the login{Enter}')
    fail(new Error('boom'))

    expect(await screen.findByText('Could not start the chat with Claude: boom')).toBeInTheDocument()
    await userEvent.type(box(), '{Enter}')
    await waitFor(() => { expect(onStart).toHaveBeenCalledTimes(3) })
    expect(onStart.mock.calls.map(([request]) => request.agent)).toEqual(['claude', 'codex', 'claude'])
  })

  it('drops an agent from the others once it is the one the chat starts with', async () => {
    page()
    await ready()
    await userEvent.click(within(screen.getByRole('group', { name: 'Also start with' })).getByRole('checkbox', { name: 'Codex' }))
    await userEvent.click(screen.getByRole('button', { name: /^Agent, model and reasoning/ }))
    await userEvent.click(within(screen.getByRole('radiogroup', { name: 'Agent' })).getByRole('radio', { name: 'Codex' }))
    await userEvent.click(within(screen.getByRole('radiogroup', { name: 'Agent' })).getByRole('radio', { name: 'Claude' }))
    await userEvent.keyboard('{Escape}')

    expect(within(screen.getByRole('group', { name: 'Also start with' })).getByRole('checkbox', { name: 'Codex' })).not.toBeChecked()
  })

  it('will not tick another agent in This folder, and says why', async () => {
    const onStart = vi.fn<Start>(() => new Promise(() => undefined))
    page(onStart)
    await ready()
    const codex = within(screen.getByRole('group', { name: 'Also start with' })).getByRole('checkbox', { name: 'Codex' })
    await userEvent.click(codex)
    await inWhere()
    await userEvent.click(screen.getByRole('radio', { name: 'This folder' }))

    expect(codex).toHaveAttribute('aria-disabled', 'true')
    expect(codex).not.toBeChecked()
    expect(codex).toHaveAccessibleDescription('Each agent needs a copy of its own. Choose New worktree to start several.')
    await userEvent.click(codex)
    expect(codex).not.toBeChecked()
    await userEvent.type(box(), 'fix the login{Enter}')
    expect(onStart).toHaveBeenCalledTimes(1)
  })

  it('names each failed start by its agent, lets the others go on, and puts the message back once', async () => {
    const onStart = vi.fn<Start>((request) => (request.agent === 'claude' ? new Promise(() => undefined) : Promise.reject(new Error('no worktree'))))
    page(onStart)
    await ready()
    const also = within(screen.getByRole('group', { name: 'Also start with' }))
    await userEvent.click(also.getByRole('checkbox', { name: 'Codex' }))
    await userEvent.click(also.getByRole('checkbox', { name: 'Gemini' }))
    attach.chooseFilesToAttach.mockResolvedValue(['/home/me/errors.log'])
    attach.attachLocalFile.mockResolvedValue({ path: '/data/attachments/attach-1-errors.log', name: 'errors.log', size: 120, kind: 'file' })
    await userEvent.click(screen.getByRole('button', { name: 'Attach files' }))
    expect(await screen.findByText('errors.log')).toBeInTheDocument()
    await userEvent.type(box(), 'fix the login')
    await userEvent.keyboard('{Control>}{Enter}{/Control}')

    expect(onStart.mock.calls.map(([request]) => request.agent)).toEqual(['claude', 'codex', 'gemini'])
    for (const [request] of onStart.mock.calls) expect(request.message).toContain('/data/attachments/attach-1-errors.log')
    expect(new Set(onStart.mock.calls.map(([request]) => request.branch?.name)).size).toBe(3)
    expect(await screen.findByText(/Could not start “fix the login” with Codex: no worktree/)).toBeInTheDocument()
    expect(await screen.findByText(/Could not start “fix the login” with Gemini: no worktree/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Put back “fix the login” with Codex' }))
    expect(box()).toHaveValue('fix the login')
    expect(screen.queryByRole('button', { name: /^Put back/ })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Dismiss “fix the login” with Gemini' })).toBeInTheDocument()
  })

  it('starts and opens the chat on plain Enter, as before', async () => {
    const onStart = vi.fn<Start>(() => new Promise(() => undefined))
    page(onStart)
    await ready()
    await userEvent.type(box(), 'open it{Enter}')

    expect(onStart).toHaveBeenCalledTimes(1)
    expect(onStart.mock.calls[0]).toHaveLength(1)
    expect(box()).toHaveValue('open it')
    expect(screen.getByRole('button', { name: 'Send message' })).toBeDisabled()
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
    await inWhere()
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
    await inWhere()
    await userEvent.click(await screen.findByRole('button', { name: 'Project: app' }))
    expect(screen.queryByRole('button', { name: /Browse for a folder/ })).not.toBeInTheDocument()
    await userEvent.click(await screen.findByRole('button', { name: /Start on This computer/ }))
    expect(onOtherMachine).toHaveBeenCalledWith(0)
  })
})
