import { useState, type ReactNode } from 'react'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Check } from 'lucide-react'
import { Popover } from 'radix-ui'
import { PopupContent } from '@/components/ui/popup'
import { CHAT_AGENTS, type ChatAgent } from '@/features/workspace/model/chatView'
import { AppIcon, Icon } from '@/features/workspace/components/glyphs'
import { AGENT_LABEL, modelName } from '../modelName'
import {
  DEFAULT_PICK, REASONING_LABEL, useAgentModels, useDiscoveredModels, REASONING_LEVELS, REASONING_PICKS, type AgentChoice, type Reasoning, type ReasoningPick,
} from '../agentChoice'

/** The shortest bar of the meter, and how much taller each next one is, in px. */
const BAR_BASE_PX = 4
const BAR_STEP_PX = 3


/** The bars of a reasoning level's strength meter: Low fills one of four, Max all four. */
function Meter({ level }: { level: Reasoning }) {
  const filled = REASONING_LEVELS.indexOf(level) + 1
  return (
    <span aria-hidden="true" className="ml-auto flex h-3 items-end gap-[2px]">
      {REASONING_LEVELS.map((name, index) => (
        <span key={name} style={{ height: `${String(BAR_BASE_PX + index * BAR_STEP_PX)}px` }} className={`w-[3px] rounded-[1px] ${index < filled ? 'bg-[var(--accent)]' : 'bg-line2'}`} />
      ))}
    </span>
  )
}

type Option = { value: string; label: string; blurb?: string; disabledReason?: string; extra?: ReactNode }

/** One radio column. Arrow keys move and pick, as a radio group does; a disabled option is skipped. */
function Column({ title, options, value, onPick }: { title: string; options: readonly Option[]; value: string; onPick: (value: string) => void }) {
  return (
    <RadioGroup aria-label={title} value={value} onValueChange={onPick} className="flex max-h-[280px] min-w-0 flex-1 flex-col gap-1 overflow-y-auto">
      <h3 className="m-0 px-2 pb-1 text-[11px] font-semibold uppercase tracking-wide text-dim">{title}</h3>
      {options.map((option) => {
        const checked = option.value === value
        const off = option.disabledReason !== undefined
        return (
          <RadioGroupItem
            key={option.value}
            value={option.value}
            disabled={off}
            title={option.disabledReason ?? option.blurb}
            className={`flex items-start gap-2 rounded-lg px-2 py-1.5 text-left text-[13px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent)] ${checked ? 'bg-hover text-fg' : 'text-fg2 hover:bg-hover'} ${off ? 'cursor-not-allowed opacity-45' : ''}`}
          >
            <Check aria-hidden="true" className={`mt-0.5 size-3.5 shrink-0 ${checked ? 'text-[var(--accent-text)]' : 'opacity-0'}`} />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="font-medium">{option.label}</span>
            </span>
            {option.extra}
          </RadioGroupItem>
        )
      })}
    </RadioGroup>
  )
}

/**
 * The composer's Agent, model and reasoning picker: one popover with three radio columns. Picking
 * ticks the choice and leaves the popover open, so all three can be set in one go; Esc or a click
 * outside closes it. The agent is fixed for a chat (its agent is already running in the session), so
 * the other one is greyed out with the reason. A model and a reasoning level that are picked go to the
 * agent with each next message; "Default" sends nothing, so the chat keeps what it started with.
 */
type PickerProps = {
  agent: ChatAgent
  choice: AgentChoice
  onChange: (change: Partial<AgentChoice>) => void
  /** The model the agent's own file says it runs; null when it has not said. */
  currentModel?: string | null
  /** Given where the agent is still to be chosen (a chat that has not started): the other agent can be picked. */
  onAgent?: (agent: ChatAgent) => void
  /** Given for a chat that follows the agent's own terminal: a pick is sent to it now, as its own `/model` and `/effort` commands. */
  live?: (command: string) => void
}

export function AgentPicker({ agent, choice, onChange, currentModel = null, onAgent, live }: PickerProps) {
  const [open, setOpen] = useState(false)
  useDiscoveredModels(agent)
  const models = useAgentModels(agent)
  const model = models.find((option) => option.id === choice.model) ?? models[0]
  const agentOptions: Option[] = CHAT_AGENTS.map((name) => ({
    value: name,
    label: AGENT_LABEL[name],
    ...(name === agent || onAgent !== undefined ? {} : { disabledReason: `Fixed for this chat · start a new chat to use ${AGENT_LABEL[name]}` }),
  }))
  const summary = `${AGENT_LABEL[agent]} · ${model?.label ?? ''} · ${REASONING_LABEL[choice.reasoning]}`
  const shownModel = modelName(agent, model, currentModel)
  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger asChild>
        <button type="button" aria-label={`Agent, model and reasoning: ${summary}`} title="Agent, model and reasoning" className="mx-model-button">
          <span className="mx-model-icon"><AppIcon process={agent} size={14} /></span>
          <span className="mx-model-name">{shownModel}</span>
          <span className="mx-model-effort">{REASONING_LABEL[choice.reasoning]}</span>
          <Icon name="chevronDown" />
        </button>
      </Popover.Trigger>
      <PopupContent side="top" align="end" aria-label="Agent, model and reasoning" className="w-[min(640px,calc(100vw-32px))]">
          <div className="flex gap-2">
            {live === undefined && <Column title="Agent" options={agentOptions} value={agent} onPick={(value) => { onAgent?.(value as ChatAgent) }} />}
            <Column
              title="Model"
              options={models.map((option) => ({ value: option.id, label: option.label, blurb: option.blurb }))}
              value={model?.id ?? ''}
              onPick={(value) => { onChange({ model: value }); live?.(`/model ${value}\r`) }}
            />
            <Column
              title="Reasoning"
              options={REASONING_PICKS.map((level) => ({ value: level, label: REASONING_LABEL[level], ...(level === DEFAULT_PICK ? {} : { extra: <Meter level={level} /> }) }))}
              value={choice.reasoning}
              onPick={(value) => { onChange({ reasoning: value as ReasoningPick }); if (live !== undefined && value !== DEFAULT_PICK) live(`/effort ${value}\r`) }}
            />
          </div>
          <p className="m-0 mt-2 flex justify-between border-t border-line2 px-2 pt-2 text-[11.5px] text-dim">
            <span>{live === undefined ? 'Changes apply from your next message' : 'Sent to the agent now, as /model and /effort'}</span>
            <span>Esc</span>
          </p>
      </PopupContent>
    </Popover.Root>
  )
}
