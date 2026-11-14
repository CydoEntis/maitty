import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { ChatRow } from '../chatRows'
import { messageEvent } from '../eventFixtures'
import { KnownRowsContext } from '../knownRows'
import { ChatItem } from './ChatItems'

const context = { agent: 'claude' as const, workingSince: null, approval: null, root: null, openDiff: null, goToStep: null, waitingStep: null }

function reply(status: 'running' | 'completed', text: string) {
  const row: ChatRow = { kind: 'agent', key: 'm1', turnId: 't1', paragraphs: [text], status }
  return <ChatItem row={row} {...context} />
}

describe('a message the person sent', () => {
  const said = (timing: { startedAtMs: number | null; completedAtMs: number | null }) => {
    const row: ChatRow = { kind: 'user', key: 'u1', event: { ...messageEvent('u1', 'user', 'Fix it'), timing } }
    return <ChatItem row={row} {...context} />
  }

  it('carries the time the daemon gave it', () => {
    const sentAt = Date.UTC(2026, 10, 4, 10, 11)
    const { container } = render(said({ startedAtMs: sentAt, completedAtMs: null }))

    expect(container.querySelector('time')).toHaveAttribute('datetime', new Date(sentAt).toISOString())
  })

  it('shows no time when the history kept none', () => {
    const { container } = render(said({ startedAtMs: null, completedAtMs: null }))

    expect(container.querySelector('time')).toBeNull()
  })
})

describe('a reply from the agent', () => {
  it('is read in its finished form while it is still arriving', () => {
    render(reply('running', 'Done. Now the **bol'))

    expect(screen.getByText('bol').tagName).toBe('STRONG')
    expect(screen.queryByText(/\*\*/)).toBeNull()
  })

  it('is read as written once it has finished', () => {
    const { container } = render(reply('completed', 'Done. Now the **bol'))

    expect(container.querySelector('strong')).toBeNull()
    expect(container).toHaveTextContent('Now the **bol')
  })

  it('is typed in once: drawn again, a reply that arrived live shows whole', () => {
    const known = new Set<string>()
    const live = () => <KnownRowsContext.Provider value={known}>{reply('completed', 'The endpoint is in.')}</KnownRowsContext.Provider>

    const first = render(live())
    expect(first.container).not.toHaveTextContent('The endpoint is in.')
    first.unmount()

    // What the list does when the row scrolls out of sight and back.
    const again = render(live())
    expect(again.container).toHaveTextContent('The endpoint is in.')
  })

  it('shows no half-typed table while it is still arriving', () => {
    const { container } = render(reply('running', 'Here:\n\n| a | b |\n|---'))

    expect(container).not.toHaveTextContent('|')
    expect(container).toHaveTextContent('Here:')
  })
})

describe('a thought', () => {
  const thought = (texts: string[]) => {
    const row: ChatRow = { kind: 'thought', key: 'r1', turnId: 't1', texts }
    return <ChatItem row={row} {...context} />
  }

  it('is one folded row labelled Thought until it is clicked', async () => {
    render(thought(['Weighing two fixes']))

    const head = screen.getByRole('button', { name: /Thought/ })
    expect(head).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByText('Weighing two fixes')).toBeNull()
    await userEvent.click(head)
    expect(head).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByText('Weighing two fixes')).toBeInTheDocument()
  })

  it('names the number of parts when a row holds several, and links the button to its panel', async () => {
    render(thought(['One', 'Two', 'Three']))

    const head = screen.getByRole('button', { name: 'Thought, 3 parts' })
    await userEvent.click(head)
    expect(document.getElementById(head.getAttribute('aria-controls') ?? '')).toHaveTextContent('Three')
  })

  it('opens from the keyboard', async () => {
    render(thought(['Weighing two fixes']))

    await userEvent.tab()
    await userEvent.keyboard('{Enter}')
    expect(screen.getByText('Weighing two fixes')).toBeInTheDocument()
  })

  it('shows the text as plain text, never as Markdown or HTML', async () => {
    const { container } = render(thought(['**bold** <b>tag</b>', 'Second paragraph']))

    await userEvent.click(screen.getByRole('button', { name: /Thought/ }))
    expect(screen.getByText('**bold** <b>tag</b>')).toBeInTheDocument()
    expect(container.querySelector('strong, b')).toBeNull()
    expect(screen.getByText('Second paragraph')).toBeInTheDocument()
  })
})
