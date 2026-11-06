import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { ChatRow } from '../chatRows'
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

  it('shows no half-typed table while it is still arriving', () => {
    const { container } = render(reply('running', 'Here:\n\n| a | b |\n|---'))

    expect(container).not.toHaveTextContent('|')
    expect(container).toHaveTextContent('Here:')
  })
})
