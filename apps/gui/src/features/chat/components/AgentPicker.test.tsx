import { render, screen, within } from '@testing-library/react'
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

describe('the model button and its menu', () => {
  it('shows the current choice on its button and opens one column: the models, then the effort', async () => {
    const user = userEvent.setup()
    render(<Harness />)
    const button = screen.getByRole('button', { name: /Agent, model and reasoning: Claude · Default · Default/ })
    await user.click(button)
    expect(screen.getByRole('radiogroup', { name: 'Model' })).toBeInTheDocument()
    expect(within(screen.getByRole('radiogroup', { name: 'Effort' })).getAllByRole('radio').map((bar) => bar.getAttribute('aria-label'))).toEqual(['Low', 'Medium', 'High', 'Max'])
    // Nothing was picked, so no level is claimed.
    expect(screen.getByText("Agent's own")).toBeInTheDocument()
  })

  it('has no agent switch in a chat that has started, and one where the agent is still to be chosen', async () => {
    const user = userEvent.setup()
    const { unmount } = render(<Harness />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    expect(screen.queryByRole('radiogroup', { name: 'Agent' })).toBeNull()
    unmount()

    const onAgent = vi.fn()
    render(<AgentPicker agent="claude" choice={{ model: 'default', reasoning: 'default' }} onChange={() => undefined} onAgent={onAgent} />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    await user.click(within(screen.getByRole('radiogroup', { name: 'Agent' })).getByRole('radio', { name: 'Codex' }))
    expect(onAgent).toHaveBeenCalledWith('codex')
  })

  it('ticks a model and an effort level without closing, and remembers them for the chat', async () => {
    const user = userEvent.setup()
    render(<Harness />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    await user.click(within(screen.getByRole('radiogroup', { name: 'Model' })).getByRole('radio', { name: /Opus/ }))
    await user.click(within(screen.getByRole('radiogroup', { name: 'Effort' })).getByRole('radio', { name: 'Max' }))
    // Still open, both ticked.
    expect(within(screen.getByRole('radiogroup', { name: 'Model' })).getByRole('radio', { name: /Opus/ })).toBeChecked()
    expect(within(screen.getByRole('radiogroup', { name: 'Effort' })).getByRole('radio', { name: 'Max' })).toBeChecked()
    expect(readChoice(7, 'claude')).toEqual({ model: 'opus', reasoning: 'max' })
    expect(readChoice(8, 'claude')).toEqual({ model: 'default', reasoning: 'default' })
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('radiogroup', { name: 'Model' })).toBeNull()
    expect(screen.getByRole('button', { name: /Claude · Opus · Max/ })).toBeInTheDocument()
  })

  it('lets go of a level when it is pressed again, back to the agent\'s own', async () => {
    const user = userEvent.setup()
    render(<Harness />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    const high = () => within(screen.getByRole('radiogroup', { name: 'Effort' })).getByRole('radio', { name: 'High' })
    await user.click(high())
    expect(readChoice(7, 'claude').reasoning).toBe('high')
    await user.click(high())
    expect(readChoice(7, 'claude').reasoning).toBe('default')
  })

  it('offers only the efforts the agent listed for a model, and drops to Medium when the picked one is not among them', async () => {
    const user = userEvent.setup()
    setDiscoveredModels('claude', [
      { id: 'opus', label: 'Opus 5.5', blurb: '', efforts: ['low', 'medium', 'high', 'max'] },
      { id: 'claude-sonnet-4-6', label: 'Sonnet 4.6', blurb: '', efforts: ['low', 'medium'] },
      { id: 'claude-haiku-4-5', label: 'Haiku 4.5', blurb: '', efforts: [] },
    ])
    writeChoiceFor(7, { model: 'opus', reasoning: 'max' })
    render(<Harness />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    const models = within(screen.getByRole('radiogroup', { name: 'Model' }))
    // The agent's own name for the alias, not the fixed word.
    expect(models.getByRole('radio', { name: /Opus 5\.5/ })).toBeChecked()
    await user.click(models.getByRole('radio', { name: /Sonnet 4\.6/ }))
    expect(readChoice(7, 'claude')).toEqual({ model: 'claude-sonnet-4-6', reasoning: 'medium' })
    const max = within(screen.getByRole('radiogroup', { name: 'Effort' })).getByRole('radio', { name: 'Max' })
    expect(max).toBeDisabled()
    expect(max).toHaveAttribute('title', 'Max: not on this model')
    // A model that takes no effort at all keeps none.
    await user.click(models.getByRole('radio', { name: /Haiku 4\.5/ }))
    expect(readChoice(7, 'claude')).toEqual({ model: 'claude-haiku-4-5', reasoning: 'default' })
  })

  it('keeps the earlier models under More models, and shows the picked one wherever it is', async () => {
    const user = userEvent.setup()
    setDiscoveredModels('claude', [
      { id: 'opus', label: 'Opus 5.5', blurb: '' },
      { id: 'claude-opus-4-8', label: 'Opus 4.8', blurb: '', more: true },
      { id: 'claude-sonnet-4-6', label: 'Sonnet 4.6', blurb: '', more: true },
    ])
    render(<Harness />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    const models = within(screen.getByRole('radiogroup', { name: 'Model' }))
    expect(models.queryByRole('radio', { name: /Opus 4\.8/ })).toBeNull()
    await user.click(screen.getByRole('button', { name: 'More models' }))
    await user.click(models.getByRole('radio', { name: /Opus 4\.8/ }))
    await user.click(screen.getByRole('button', { name: 'Fewer models' }))
    expect(models.getByRole('radio', { name: /Opus 4\.8/ })).toBeChecked()
    expect(models.queryByRole('radio', { name: /Sonnet 4\.6/ })).toBeNull()
  })

  it('has no Max for Codex until its own list says the model takes it', async () => {
    const user = userEvent.setup()
    setDiscoveredModels('codex', [{ id: 'gpt-x', label: 'GPT-X', blurb: '', efforts: ['low', 'medium', 'high', 'max'] }])
    render(<Harness agent="codex" />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    // Default: which model it is cannot be read, so neither can its levels.
    expect(within(screen.getByRole('radiogroup', { name: 'Effort' })).getByRole('radio', { name: 'Max' })).toBeDisabled()
    expect(within(screen.getByRole('radiogroup', { name: 'Effort' })).getByRole('radio', { name: 'High' })).toBeEnabled()
    await user.click(within(screen.getByRole('radiogroup', { name: 'Model' })).getByRole('radio', { name: /GPT-X/ }))
    expect(within(screen.getByRole('radiogroup', { name: 'Effort' })).getByRole('radio', { name: 'Max' })).toBeEnabled()
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

  it('adds the extra models Claude reports to its fixed aliases, once each', async () => {
    const user = userEvent.setup()
    setDiscoveredModels('claude', [{ id: 'claude-fable-5-1[1m]', label: 'Fable 5.1', blurb: 'Most capable' }, { id: 'opus', label: 'Opus', blurb: 'x' }])
    render(<Harness agent="claude" />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    const models = within(screen.getByRole('radiogroup', { name: 'Model' })).getAllByRole('radio')
    expect(models.map((model) => model.textContent)).toEqual(['Default', 'Fable 5.1', 'Opus', 'Sonnet', 'Haiku'])
  })

  it('names a model read by its id, and lets the account default stand in for the Default row', async () => {
    const user = userEvent.setup()
    setDiscoveredModels('claude', [{ id: 'claude-sonnet-5-5', label: 'claude-sonnet-5-5', blurb: '' }, { id: 'claude-fable-5-2', label: 'claude-fable-5-2', blurb: '', isDefault: true }])
    render(<Harness agent="claude" />)
    await user.click(screen.getByRole('button', { name: /Agent, model and reasoning/ }))
    const models = within(screen.getByRole('radiogroup', { name: 'Model' })).getAllByRole('radio')
    expect(models.map((model) => model.textContent)).toEqual(['Sonnet 5.5', 'Fable 5.2default', 'Opus', 'Sonnet', 'Haiku'])
    // Nothing is picked, so the default's row is the ticked one, and picking it sends no model.
    expect(models[1]).toBeChecked()
    await user.click(models[0] as HTMLElement)
    expect(wireChoice(7, 'claude')).toEqual({ model: 'claude-sonnet-5-5' })
    await user.click(within(screen.getByRole('radiogroup', { name: 'Model' })).getByRole('radio', { name: /Fable 5\.2/ }))
    expect(wireChoice(7, 'claude')).toBeNull()
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
