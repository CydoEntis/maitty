import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { forgetChoices, forgetDiscoveredModels, setDiscoveredModels, readChoice, useAgentChoice, wireChoice, writeChoice } from '../agentChoice'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import { AgentPicker } from './AgentPicker'

const writeChoiceFor = writeChoice

vi.mock('@/api/daemon', () => ({ loadAgentModels: () => Promise.resolve([]) }))

function Harness({ agent = 'claude' as ChatAgent, conversation = 7 }) {
  const [choice, change] = useAgentChoice(conversation, agent)
  return <AgentPicker agent={agent} choice={choice} onChange={change} />
}

beforeEach(() => {
  window.localStorage.clear()
  forgetChoices()
  forgetDiscoveredModels()
})

describe('the agent, model and reasoning picker', () => {
  it('shows the current choice on its button and opens three radio columns', async () => {
    const user = userEvent.setup()
    render(<Harness />)
    const button = screen.getByRole('button', { name: /Agent, model and reasoning: Claude · Default · Default/ })
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
    expect(other).toBeDisabled()
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
    expect(readChoice(8, 'claude')).toEqual({ model: 'default', reasoning: 'default' })
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('radiogroup', { name: 'Model' })).toBeNull()
    expect(screen.getByRole('button', { name: /Claude · Opus · Max/ })).toBeInTheDocument()
  })

  it('moves a radio with the arrow keys', async () => {
    const user = userEvent.setup()
    render(<Harness agent="claude" />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    within(screen.getByRole('radiogroup', { name: 'Reasoning' })).getByRole('radio', { name: /Default/ }).focus()
    // Radix picks on the frame after focus arrives, while the key is still down, as a real press is.
    for (let press = 0; press < 2; press += 1) {
      await user.keyboard('{ArrowDown>}')
      await new Promise((resolve) => { setTimeout(resolve, 40) })
      await user.keyboard('{/ArrowDown}')
    }
    await waitFor(() => { expect(within(screen.getByRole('radiogroup', { name: 'Reasoning' })).getByRole('radio', { name: /Medium/ })).toBeChecked() })
  })

  it('lists the models Codex itself offers after Default, and a stored one is kept', async () => {
    const user = userEvent.setup()
    setDiscoveredModels('codex', [{ id: 'gpt-x', label: 'GPT-X', blurb: 'The newest' }])
    writeChoiceFor(7, { model: 'gpt-x' })
    expect(readChoice(7, 'codex').model).toBe('gpt-x')
    render(<Harness agent="codex" />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    const models = within(screen.getByRole('radiogroup', { name: 'Model' })).getAllByRole('radio')
    expect(models.map((model) => model.textContent)).toEqual(['Default', 'GPT-X'])
    expect(models[1]).toBeChecked()
    expect(models[1]).toHaveAttribute('title', 'The newest')
  })
})

describe('what the picker sends with a message', () => {
  it('sends nothing until something is picked, then only what was picked', () => {
    expect(wireChoice(7, 'claude')).toBeNull()
    writeChoiceFor(7, { reasoning: 'high' })
    expect(wireChoice(7, 'claude')).toEqual({ effort: 'high' })
    writeChoiceFor(7, { model: 'opus' })
    expect(wireChoice(7, 'claude')).toEqual({ model: 'opus', effort: 'high' })
    writeChoiceFor(7, { model: 'default', reasoning: 'default' })
    expect(wireChoice(7, 'claude')).toBeNull()
  })

  it('ignores a stored model the agent does not offer', () => {
    writeChoiceFor(9, { model: 'opus' })
    expect(wireChoice(9, 'codex')).toBeNull()
  })
})
