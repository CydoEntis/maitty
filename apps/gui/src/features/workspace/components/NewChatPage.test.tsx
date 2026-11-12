import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

const sync = vi.hoisted(() => ({ dirty: false, behind: 2, pulled: [] as string[] }))
function syncOf(folder: string) {
  return { branch: folder === '/dev/api' ? 'main' : 'dev', upstream: 'origin/dev', behind: sync.behind, ahead: 0, dirty: sync.dirty, fetched: true }
}
vi.mock('@/api/daemon', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/daemon')>()), loadAgentModels: () => Promise.resolve([]), folderSyncState: (folder: string) => Promise.resolve(syncOf(folder)), pullFolder: (folder: string) => { sync.pulled.push(folder); return Promise.resolve() }, folderBranches: () => Promise.resolve({ current: 'dev', branches: [{ name: 'dev', when: 10 }], worktrees: [{ path: '/dev/growth', branch: 'dev', main: true, when: 10 }] }) }))

const attach = vi.hoisted(() => ({ attachLocalFile: vi.fn(), chooseFilesToAttach: vi.fn() }))
vi.mock('@/api/attachments', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/attachments')>()), ...attach }))

import { NewChatPage } from './NewChatPage'

const PROJECTS = [{ folder: '/dev/growth', name: 'growth', color: 'teal', sessions: 6 }, { folder: '/dev/api', name: 'api', color: 'blue', sessions: 2 }]

function page(onStart: (request: { message: string }) => Promise<void> = () => Promise.resolve()) {
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
    await waitFor(() => { expect(onWarm).toHaveBeenLastCalledWith({ folder: '/dev/growth', agent: 'claude', model: 'default', reasoning: 'default', mode: 'default' }) })

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

  it('starts in a worktree of its own on a new branch by default, from the latest of its upstream', async () => {
    const onStart = vi.fn<(request: { message: string; branch?: unknown }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await screen.findByText(/Starts from latest origin\/dev · fetched just now/)
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'hello')
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    expect(onStart.mock.calls[0]?.[0].branch).toEqual({ name: expect.stringMatching(/^[a-z]+-[a-z]+$/) as unknown, create: true, worktree: true, base: 'origin/dev' })
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
