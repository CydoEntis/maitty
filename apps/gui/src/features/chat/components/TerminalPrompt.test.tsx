import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { KEY_DOWN, KEY_ENTER, KEY_ESCAPE, promptOfScreen } from '../terminalKeys'
import { TerminalPrompt } from './TerminalPrompt'

const SCREEN = [
  'earlier conversation',
  '────────────────────────────',
  'We recommend compacting earlier',
  '',
  'Want to autocompact at 400k tokens instead?',
  '❯ Autocompact at ‹ 400k › tokens',
  '  Keep auto (1m tokens)',
  '',
].join('\n')

describe('the agent\'s menu in its terminal, answered from the chat', () => {
  it('shows what follows the last rule of the screen, not the conversation above it', () => {
    const shown = promptOfScreen(SCREEN)
    expect(shown).toContain('We recommend compacting earlier')
    expect(shown).toContain('Keep auto (1m tokens)')
    expect(shown).not.toContain('earlier conversation')
  })

  it('shows the question and types the keys a person would press into the agent\'s terminal', async () => {
    const sendKeys = vi.fn<(keys: string) => Promise<void>>(() => Promise.resolve())
    const user = userEvent.setup()
    render(<TerminalPrompt readScreen={() => Promise.resolve(SCREEN)} sendKeys={sendKeys} />)
    expect(await screen.findByLabelText('What the agent shows in its terminal')).toHaveTextContent('Autocompact at ‹ 400k › tokens')

    await user.click(screen.getByRole('button', { name: 'Down' }))
    await user.click(screen.getByRole('button', { name: 'Enter' }))
    await user.click(screen.getByRole('button', { name: 'Escape' }))
    expect(sendKeys.mock.calls.map(([keys]) => keys)).toEqual([KEY_DOWN, KEY_ENTER, KEY_ESCAPE])
  })

  it('still offers the keys, and says where the question is, when the screen cannot be read', async () => {
    const sendKeys = vi.fn<(keys: string) => Promise<void>>(() => Promise.resolve())
    render(<TerminalPrompt readScreen={() => Promise.reject(new Error('local-only in v1'))} sendKeys={sendKeys} />)
    await waitFor(() => { expect(screen.getByText('The question is in the terminal.')).toBeInTheDocument() })
    expect(screen.getByRole('button', { name: 'Up' })).toBeEnabled()
  })
})
