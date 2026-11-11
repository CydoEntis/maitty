import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

vi.mock('@/api/daemon', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/daemon')>()), loadAgentModels: () => Promise.resolve([]), folderBranches: () => Promise.resolve({ current: 'dev', branches: [{ name: 'dev', when: 10 }], worktrees: [{ path: '/dev/growth', branch: 'dev', main: true, when: 10 }] }) }))

const attach = vi.hoisted(() => ({ attachLocalFile: vi.fn(), chooseFilesToAttach: vi.fn() }))
vi.mock('@/api/attachments', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/attachments')>()), ...attach }))

import { NewChatPage } from './NewChatPage'

const PROJECTS = [{ folder: '/dev/growth', name: 'growth', color: 'teal', sessions: 6 }, { folder: '/dev/api', name: 'api', color: 'blue', sessions: 2 }]

function page(onStart: (request: { message: string }) => Promise<void> = () => Promise.resolve()) {
  return render(
    <div className="mx-app">
      <NewChatPage machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={onStart} onOtherMachine={() => undefined} onCancel={() => undefined} />
    </div>,
  )
}

function codexPage(onStart: () => Promise<void>, codexYolo = false) {
  return render(
    <div className="mx-app">
      <NewChatPage machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="codex" codexYolo={codexYolo} browse={() => Promise.resolve(null)} onStart={onStart} onOtherMachine={() => undefined} onCancel={() => undefined} />
    </div>,
  )
}

/** Step 1: the folder the chat works in. */
async function pickProject(name: string) {
  await userEvent.click(await screen.findByRole('button', { name: new RegExp(`^${name}`) }))
}

describe('the new chat page', () => {
  it('asks for a folder first: the projects, a search over them, and a pasted path that opens as a folder', async () => {
    page()
    expect(screen.getByRole('heading', { name: 'What should we work on?' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Start chat' })).toBeDisabled()
    const list = screen.getByRole('list', { name: 'Recent projects' })
    expect(within(list).getAllByRole('button')).toHaveLength(2)
    expect(within(list).getByRole('button', { name: /^growth/ })).toHaveTextContent('6 sessions')

    const search = screen.getByRole('textbox', { name: 'Search projects or paste a path' })
    await userEvent.type(search, 'api')
    expect(within(screen.getByRole('list', { name: 'Matching projects' })).getAllByRole('button')).toHaveLength(1)

    await userEvent.clear(search)
    await userEvent.type(search, 'zzz')
    expect(screen.getByText('No projects match. Paste a full path, or use Browse…')).toBeInTheDocument()

    await userEvent.clear(search)
    await userEvent.type(search, '/srv/new-app')
    await userEvent.click(within(screen.getByRole('list', { name: 'Open folder' })).getByRole('button', { name: /^Open new-app/ }))
    expect(await screen.findByRole('button', { name: 'Project: new-app' })).toBeInTheDocument()
  })

  it('starts the agent ahead once a folder is picked, and says none is wanted when the page goes', async () => {
    const onWarm = vi.fn()
    const { unmount } = render(
      <div className="mx-app">
        <NewChatPage machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={() => Promise.resolve()} onWarm={onWarm} onOtherMachine={() => undefined} onCancel={() => undefined} />
      </div>,
    )
    // No folder yet: nothing to start in.
    expect(onWarm).toHaveBeenLastCalledWith(null)

    await pickProject('growth')
    await waitFor(() => { expect(onWarm).toHaveBeenLastCalledWith({ folder: '/dev/growth', agent: 'claude', model: 'default', reasoning: 'default', mode: 'default' }) })

    unmount()
    expect(onWarm).toHaveBeenLastCalledWith(null)
  })

  it('shows the project, its branch and where the chat works as three buttons once a folder is picked', async () => {
    page()
    await pickProject('growth')
    expect(await screen.findByRole('button', { name: 'Project: growth' })).toBeInTheDocument()
    expect(await screen.findByRole('button', { name: 'Branch: dev' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Work in: This folder' })).toBeInTheDocument()
    expect(within(screen.getByRole('list', { name: 'Suggested tasks' })).getAllByRole('button')).toHaveLength(3)
    expect(screen.queryByRole('textbox', { name: 'Search projects or paste a path' })).not.toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'First message' })).toHaveFocus()
  })

  it('swaps the project from the dropdown under its button', async () => {
    page()
    await pickProject('growth')
    await userEvent.click(await screen.findByRole('button', { name: 'Project: growth' }))
    const menu = await screen.findByRole('dialog')
    expect(within(menu).getByRole('radio', { name: /growth/ })).toHaveTextContent('6')
    await userEvent.click(within(menu).getByRole('radio', { name: /api/ }))
    expect(await screen.findByRole('button', { name: 'Project: api' })).toBeInTheDocument()
  })

  it('starts in a new worktree on a new branch as soon as New worktree is chosen, and says so on the page', async () => {
    const onStart = vi.fn<(request: { message: string; branch?: unknown }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await pickProject('growth')
    await userEvent.click(await screen.findByRole('button', { name: 'Work in: This folder' }))
    await userEvent.click(within(await screen.findByRole('dialog')).getByRole('radio', { name: /^New worktree/ }))

    expect(await screen.findByRole('button', { name: 'Work in: New worktree' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^Branch: [a-z]+-[a-z]+$/ })).toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'hello')
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    expect(onStart.mock.calls[0]?.[0].branch).toEqual({ name: expect.stringMatching(/^[a-z]+-[a-z]+$/) as unknown, create: true, worktree: true })
  })

  it('goes back to this folder when that is chosen again', async () => {
    page()
    await pickProject('growth')
    await userEvent.click(await screen.findByRole('button', { name: 'Work in: This folder' }))
    await userEvent.click(within(await screen.findByRole('dialog')).getByRole('radio', { name: /^New worktree/ }))
    await userEvent.click(await screen.findByRole('button', { name: 'Work in: New worktree' }))
    await userEvent.click(within(await screen.findByRole('dialog')).getByRole('radio', { name: /^This folder/ }))
    expect(await screen.findByRole('button', { name: 'Branch: dev' })).toBeInTheDocument()
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
