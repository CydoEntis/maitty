import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

vi.mock('@/api/daemon', () => ({ loadAgentModels: () => Promise.resolve([]) }))

import { NewChatPage } from './NewChatPage'

const PROJECTS = [{ folder: '/dev/growth', name: 'growth', color: 'teal', sessions: 6 }, { folder: '/dev/api', name: 'api', color: 'blue', sessions: 2 }]

function page() {
  return render(
    <div className="mx-app">
      <NewChatPage machines={[{ machine: 0, label: 'This computer', local: true }]} projects={PROJECTS} defaultAgent="claude" browse={() => Promise.resolve(null)} onStart={() => Promise.resolve()} onOtherMachine={() => undefined} onCancel={() => undefined} />
    </div>,
  )
}

describe('the new chat page', () => {
  it('reads as a sentence above the message box: computer, folder, agent, model', () => {
    page()
    const row = screen.getByRole('button', { name: /^Computer/ }).parentElement as HTMLElement
    expect(within(row).getAllByRole('button').map((chip) => chip.getAttribute('aria-label')?.split(':')[0])).toEqual(['Computer', 'Folder', 'Agent', 'Model and reasoning'])
    expect(screen.getByRole('textbox', { name: 'First message' })).toHaveAttribute('placeholder', 'What should Claude work on?')
  })

  it('lists projects with their session counts and turns a pasted path into a row', async () => {
    page()
    await userEvent.click(screen.getByRole('button', { name: /^Folder/ }))
    const menu = await screen.findByRole('menu')
    expect(within(menu).getByRole('menuitemradio', { name: /growth/ })).toHaveTextContent('6')
    await userEvent.type(screen.getByRole('textbox', { name: 'Search projects or paste a path' }), '/srv/new-app')
    await userEvent.click(within(menu).getByRole('menuitemradio', { name: 'Use /srv/new-app' }))
    expect(screen.getByRole('button', { name: /^Folder: new-app/ })).toBeInTheDocument()
  })
})
