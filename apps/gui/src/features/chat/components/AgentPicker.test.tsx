import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'
import { forgetChoices, readChoice, useAgentChoice } from '../agentChoice'
import { AgentPicker } from './AgentPicker'

function Harness({ agent = 'claude' as const, conversation = 7 }) {
  const [choice, change] = useAgentChoice(conversation, agent)
  return <AgentPicker agent={agent} choice={choice} onChange={change} />
}

beforeEach(() => {
  window.localStorage.clear()
  forgetChoices()
})

describe('the agent, model and reasoning picker', () => {
  it('shows the current choice on its button and opens three radio columns', async () => {
    const user = userEvent.setup()
    render(<Harness />)
    const button = screen.getByRole('button', { name: /Agent, model and reasoning: Claude · Sonnet · Medium/ })
    await user.click(button)
    expect(screen.getByRole('radiogroup', { name: 'Agent' })).toBeInTheDocument()
    expect(screen.getByRole('radiogroup', { name: 'Model' })).toBeInTheDocument()
    expect(screen.getByRole('radiogroup', { name: 'Reasoning' })).toBeInTheDocument()
    expect(screen.getByText('Changes apply from your next message')).toBeInTheDocument()
  })

  it('fixes the agent with the stated tooltip', async () => {
    const user = userEvent.setup()
    render(<Harness />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    const other = within(screen.getByRole('radiogroup', { name: 'Agent' })).getByRole('radio', { name: /Codex/ })
    expect(other).toHaveAttribute('aria-disabled', 'true')
    expect(other).toHaveAttribute('title', 'Fixed for this chat · start a new chat to use Codex')
    await user.click(other)
    expect(within(screen.getByRole('radiogroup', { name: 'Agent' })).getByRole('radio', { name: /Claude/ })).toBeChecked()
  })

  it('ticks a model and a reasoning level without closing, and remembers them for the chat', async () => {
    const user = userEvent.setup()
    render(<Harness />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    await user.click(within(screen.getByRole('radiogroup', { name: 'Model' })).getByRole('radio', { name: /Opus/ }))
    await user.click(within(screen.getByRole('radiogroup', { name: 'Reasoning' })).getByRole('radio', { name: /Max/ }))
    // Still open, both ticked.
    expect(within(screen.getByRole('radiogroup', { name: 'Model' })).getByRole('radio', { name: /Opus/ })).toBeChecked()
    expect(within(screen.getByRole('radiogroup', { name: 'Reasoning' })).getByRole('radio', { name: /Max/ })).toBeChecked()
    expect(readChoice(7, 'claude')).toEqual({ model: 'opus', reasoning: 'max' })
    expect(readChoice(8, 'claude')).toEqual({ model: 'sonnet', reasoning: 'medium' })
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('radiogroup', { name: 'Model' })).toBeNull()
    expect(screen.getByRole('button', { name: /Claude · Opus · Max/ })).toBeInTheDocument()
  })

  it('moves a radio with the arrow keys', async () => {
    const user = userEvent.setup()
    render(<Harness agent="claude" />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    within(screen.getByRole('radiogroup', { name: 'Reasoning' })).getByRole('radio', { name: /Medium/ }).focus()
    await user.keyboard('{ArrowDown}')
    expect(within(screen.getByRole('radiogroup', { name: 'Reasoning' })).getByRole('radio', { name: /High/ })).toBeChecked()
  })
})
