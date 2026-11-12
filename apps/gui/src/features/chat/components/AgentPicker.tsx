import { useState } from 'react'
import { RadioGroup, RadioGroupItem, Segmented, SegmentedItem } from '@/components/ui/radio-group'
import { Check } from 'lucide-react'
import { Popover } from 'radix-ui'
import { PopupContent } from '@/components/ui/popup'
import { CHAT_AGENTS, type ChatAgent } from '@/features/workspace/model/chatView'
import { AppIcon, Icon } from '@/features/workspace/components/glyphs'
import { AGENT_LABEL, modelName } from '../modelName'
import {
  choiceFor, DEFAULT_PICK, effortAllowed, leadingModels, menuModels, pickedModel, REASONING_LABEL, REASONING_LEVELS, useAgentModels, useDiscoveredModels,
  type AgentChoice, type ModelOption, type Reasoning, type ReasoningPick,
} from '../agentChoice'

/** The level a chat drops to when its new model does not take the one it had. */
const FALLBACK_LEVEL: Reasoning = 'medium'

/** How many of the four bars a level fills; none while the agent's own level is kept. */
function filledBars(level: ReasoningPick): number {
  return level === DEFAULT_PICK ? 0 : REASONING_LEVELS.indexOf(level) + 1
}

/** The small strength meter on the button: four rising bars, filled up to the picked level. */
function MiniMeter({ level }: { level: ReasoningPick }) {
  const filled = filledBars(level)
  return (
    <span aria-hidden="true" className="mx-mm-mini">
      {REASONING_LEVELS.map((name, index) => <span key={name} className={index < filled ? 'is-on' : undefined} />)}
    </span>
  )
}

type PickerProps = {
  agent: ChatAgent
  choice: AgentChoice
  onChange: (change: Partial<AgentChoice>) => void
  /** The model the agent's own file says it runs; null when it has not said. */
  currentModel?: string | null
  /** Given where the agent is still to be chosen (a chat that has not started): the menu has the agent switch. */
  onAgent?: (agent: ChatAgent) => void
  /** Given for a chat that follows the agent's own terminal: a pick is sent to it now, as its own `/model` and `/effort` commands. */
  live?: (command: string) => void
  /** Why the choice cannot be changed right now. The button stays where it is, greyed out, so the message box never rearranges. */
  locked?: string | undefined
  /** Only the agent is a choice: no model list or effort, because they are read from this computer and mean nothing elsewhere. */
  agentOnly?: boolean
}

/**
 * The message box's model button and its menu: one column with the models, then the effort as four
 * bars. The earlier models wait under "More models", so the newest lead. Picking leaves the menu open, so both can be set in one go; Esc or a click outside closes it.
 * The agent switch is there only before a chat starts: after that its agent is already running.
 * What is picked goes to the agent with the next message; the account's default model and a level
 * that was never picked send nothing, so the chat keeps what it started with.
 */
export function AgentPicker({ agent, choice, onChange, currentModel = null, onAgent, live, locked, agentOnly = false }: PickerProps) {
  const [open, setOpen] = useState(false)
  const [allModels, setAllModels] = useState(false)
  useDiscoveredModels(agent)
  const models = useAgentModels(agent)
  const rows = menuModels(models)
  const model = pickedModel(models, choice.model)
  const modelId = model?.id ?? DEFAULT_PICK
  const leading = leadingModels(rows, modelId)
  const shown = allModels ? rows : leading
  const summary = `${AGENT_LABEL[agent]} · ${model?.label ?? ''} · ${REASONING_LABEL[choice.reasoning]}`
  const pickModel = (option: ModelOption) => {
    onChange({ model: choiceFor(option) })
    live?.(`/model ${choiceFor(option)}\r`)
    if (choice.reasoning !== DEFAULT_PICK && !effortAllowed(agent, option, choice.reasoning)) pickLevel(effortAllowed(agent, option, FALLBACK_LEVEL) ? FALLBACK_LEVEL : DEFAULT_PICK)
  }
  const pickLevel = (level: ReasoningPick) => {
    onChange({ reasoning: level })
    if (level !== DEFAULT_PICK) live?.(`/effort ${level}\r`)
  }
  return (
    // aria-disabled, not disabled: a disabled button gets no tooltip, and the reason is the point.
    <Popover.Root open={open && locked === undefined} onOpenChange={(next) => { if (locked === undefined) setOpen(next) }}>
      <Popover.Trigger asChild>
        <button type="button" aria-label={`Agent, model and reasoning: ${summary}`} title={locked ?? 'Model and effort'} aria-disabled={locked !== undefined} className={`mx-model-button${locked === undefined ? '' : ' is-locked'}`}>
          <span className="mx-model-icon"><AppIcon process={agent} size={14} /></span>
          <span className="mx-model-name">{/* With nothing picked, what the agent says it runs is the better name than the row that stands for it. */}
            {agentOnly ? AGENT_LABEL[agent] : modelName(agent, choice.model === DEFAULT_PICK && currentModel !== null ? undefined : model, currentModel)}</span>
          {!agentOnly && choice.reasoning !== DEFAULT_PICK && <span className="mx-model-level">{REASONING_LABEL[choice.reasoning]}</span>}
          {!agentOnly && <MiniMeter level={choice.reasoning} />}
          <Icon name="chevronDown" />
        </button>
      </Popover.Trigger>
      <PopupContent side="top" align="start" aria-label="Model and effort" className="mx-mm">
        {onAgent !== undefined && (
          <Segmented aria-label="Agent" value={agent} onValueChange={(value) => { onAgent(value as ChatAgent) }} className="mx-mm-agents">
            {CHAT_AGENTS.map((name) => (
              <SegmentedItem key={name} value={name} className="mx-mm-agent"><AppIcon process={name} size={14} />{AGENT_LABEL[name]}</SegmentedItem>
            ))}
          </Segmented>
        )}
        {!agentOnly && (<>
        <RadioGroup aria-label="Model" value={modelId} onValueChange={(value) => { const option = rows.find((row) => row.id === value); if (option) pickModel(option) }} className="mx-mm-models">
          {shown.map((option) => (
            <RadioGroupItem key={option.id} value={option.id} title={option.blurb === '' ? undefined : option.blurb} className="mx-mm-model">
              <span>{option.label}</span>
              {option.isDefault === true && <small>default</small>}
              <span className="mx-spacer" />
              {option.id === modelId && <Check aria-hidden="true" className="mx-mm-check" />}
            </RadioGroupItem>
          ))}
        </RadioGroup>
        {leading.length < rows.length && (
          <button type="button" aria-expanded={allModels} className="mx-mm-more" onClick={() => { setAllModels((now) => !now) }}>
            <span>{allModels ? 'Fewer models' : 'More models'}</span>
            <span className="mx-spacer" />
            <Icon name="chevronDown" />
          </button>
        )}
        <div className="mx-mm-rule" role="presentation" />
        <div className="mx-mm-effort">
          <span className="mx-mm-effort-word">Effort</span>
          <RadioGroup aria-label="Effort" orientation="horizontal" value={choice.reasoning === DEFAULT_PICK ? '' : choice.reasoning} onValueChange={(value) => { pickLevel(value as Reasoning) }} className="mx-mm-bars">
            {REASONING_LEVELS.map((level, index) => {
              const allowed = effortAllowed(agent, model, level)
              return (
                <RadioGroupItem
                  key={level}
                  value={level}
                  disabled={!allowed}
                  aria-label={REASONING_LABEL[level]}
                  title={allowed ? REASONING_LABEL[level] : `${REASONING_LABEL[level]}: not on this model`}
                  className={`mx-mm-bar${index < filledBars(choice.reasoning) ? ' is-on' : ''}`}
                  // A second press on the picked level lets go of it: the chat keeps the agent's own again.
                  onClick={() => { if (level === choice.reasoning) pickLevel(DEFAULT_PICK) }}
                />
              )
            })}
          </RadioGroup>
          <span className="mx-spacer" />
          <span className="mx-mm-level">{choice.reasoning === DEFAULT_PICK ? "Agent's own" : REASONING_LABEL[choice.reasoning]}</span>
        </div>
        </>)}
      </PopupContent>
    </Popover.Root>
  )
}
