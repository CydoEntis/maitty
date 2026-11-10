import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { ChatRow } from '../chatRows'
import { KnownRowsContext } from '../knownRows'
import { ChatItem } from './ChatItems'

const context = { agent: 'claude' as const, workingSince: null, approval: null, root: null, openDiff: null, goToStep: null }

function reply(status: 'running' | 'completed', text: string) {
  const row: ChatRow = { kind: 'agent', key: 'm1', turnId: 't1', paragraphs: [text], status }
  return <ChatItem row={row} {...context} />
}

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
