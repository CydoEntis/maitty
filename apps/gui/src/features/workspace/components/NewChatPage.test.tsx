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

describe('the new chat page', () => {
  it('starts in a new worktree on a new branch as soon as New worktree is chosen, and says so on the page', async () => {
    const onStart = vi.fn<(request: { message: string; branch?: unknown }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await userEvent.click(await screen.findByRole('button', { name: 'Branch: dev' }))
    await userEvent.click(screen.getByRole('tab', { name: 'New worktree' }))
    await userEvent.keyboard('{Escape}')

    expect(await screen.findByRole('button', { name: /^Branch: new worktree · [a-z]+-[a-z]+$/ })).toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'hello')
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    expect(onStart.mock.calls[0]?.[0].branch).toEqual({ name: expect.stringMatching(/^[a-z]+-[a-z]+$/) as unknown, create: true, worktree: true })
  })

  it('goes back to this checkout when that tab is chosen again', async () => {
    page()
    await userEvent.click(await screen.findByRole('button', { name: 'Branch: dev' }))
    await userEvent.click(screen.getByRole('tab', { name: 'New worktree' }))
    await userEvent.click(screen.getByRole('tab', { name: 'This checkout' }))
    await userEvent.keyboard('{Escape}')
    expect(await screen.findByRole('button', { name: 'Branch: dev' })).toBeInTheDocument()
  })

  it('attaches a picked file to the first message, shows it, and names it in the message', async () => {
    attach.chooseFilesToAttach.mockResolvedValue(['/home/me/errors.log'])
    attach.attachLocalFile.mockResolvedValue({ path: '/data/attachments/attach-1-errors.log', name: 'errors.log', size: 120, kind: 'file' })
    const onStart = vi.fn<(request: { message: string }) => Promise<void>>().mockResolvedValue(undefined)
    page(onStart)
    await userEvent.type(screen.getByRole('textbox', { name: 'First message' }), 'why does this fail?')
    await userEvent.click(screen.getByRole('button', { name: 'Attach files' }))
    expect(await screen.findByText('errors.log')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Send message' }))
    await waitFor(() => { expect(onStart).toHaveBeenCalled() })
    const message = onStart.mock.calls[0]?.[0].message ?? ''
    expect(message).toContain('why does this fail?')
    expect(message).toContain('/data/attachments/attach-1-errors.log')
  })

  it('reads as a sentence above the message box: computer, folder, agent, model', () => {
    page()
    const row = screen.getByRole('button', { name: /^Computer/ }).parentElement as HTMLElement
    expect(within(row).getAllByRole('button').map((chip) => chip.getAttribute('aria-label')?.split(':')[0])).toEqual(['Computer', 'Folder', 'Agent', 'Model and reasoning'])
    expect(screen.getByRole('textbox', { name: 'First message' })).toHaveAttribute('placeholder', 'What should Claude work on?')
  })

  it('lists projects with their session counts and turns a pasted path into a row', async () => {
    page()
    await userEvent.click(screen.getByRole('button', { name: /^Folder/ }))
    const menu = await screen.findByRole('dialog')
    expect(within(menu).getByRole('radio', { name: /growth/ })).toHaveTextContent('6')
    await userEvent.type(screen.getByRole('textbox', { name: 'Search projects or paste a path' }), '/srv/new-app')
    await userEvent.click(within(menu).getByRole('radio', { name: 'Use /srv/new-app' }))
    expect(screen.getByRole('button', { name: /^Folder: new-app/ })).toBeInTheDocument()
  })
})
