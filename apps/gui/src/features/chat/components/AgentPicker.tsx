import { useState, type KeyboardEvent, type ReactNode } from 'react'
import { Check, ChevronUp } from 'lucide-react'
import { Popover } from 'radix-ui'
import { CHAT_AGENTS, type ChatAgent } from '@/features/workspace/model/chatView'
import {
  PLACEHOLDER_MODELS, REASONING_LABEL, REASONING_LEVELS, type AgentChoice, type Reasoning,
} from '../agentChoice'

/** The shortest bar of the meter, and how much taller each next one is, in px. */
const BAR_BASE_PX = 4
const BAR_STEP_PX = 3

const AGENT_LABEL: Readonly<Record<ChatAgent, string>> = { claude: 'Claude', codex: 'Codex' }

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
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === 'ArrowDown' || event.key === 'ArrowRight' ? 1 : event.key === 'ArrowUp' || event.key === 'ArrowLeft' ? -1 : 0
    if (step === 0) return
    event.preventDefault()
    const enabled = options.filter((option) => option.disabledReason === undefined)
    const at = enabled.findIndex((option) => option.value === value)
    const next = enabled[(at + step + enabled.length) % enabled.length]
    if (next) {
      onPick(next.value)
      const radios = event.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]')
      radios[options.indexOf(next)]?.focus()
    }
  }
  return (
    <div role="radiogroup" aria-label={title} onKeyDown={onKeyDown} className="flex min-w-0 flex-1 flex-col gap-1">
      <h3 className="m-0 px-2 pb-1 text-[11px] font-semibold uppercase tracking-wide text-dim">{title}</h3>
      {options.map((option) => {
        const checked = option.value === value
        const off = option.disabledReason !== undefined
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={checked}
            aria-disabled={off}
            tabIndex={checked ? 0 : -1}
            title={option.disabledReason}
            className={`flex items-start gap-2 rounded-lg px-2 py-1.5 text-left text-[13px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent)] ${checked ? 'bg-hover text-fg' : 'text-fg2 hover:bg-hover'} ${off ? 'cursor-not-allowed opacity-45' : ''}`}
            onClick={() => { if (!off) onPick(option.value) }}
          >
            <Check aria-hidden="true" className={`mt-0.5 size-3.5 shrink-0 ${checked ? 'text-[var(--accent)]' : 'opacity-0'}`} />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="font-medium">{option.label}</span>
              {option.blurb !== undefined && <span className="text-[11.5px] text-dim">{option.blurb}</span>}
            </span>
            {option.extra}
          </button>
        )
      })}
    </div>
  )
}

/**
 * The composer's Agent, model and reasoning picker: one popover with three radio columns. Picking
 * ticks the choice and leaves the popover open, so all three can be set in one go; Esc or a click
 * outside closes it. The agent is fixed for a chat (its agent is already running in the session), so
 * the other one is greyed out with the reason; model and reasoning apply from the next message.
 */
export function AgentPicker({ agent, choice, onChange }: { agent: ChatAgent; choice: AgentChoice; onChange: (change: Partial<AgentChoice>) => void }) {
  const [open, setOpen] = useState(false)
  const models = PLACEHOLDER_MODELS[agent]
  const model = models.find((option) => option.id === choice.model) ?? models[0]
  const agentOptions: Option[] = CHAT_AGENTS.map((name) => ({
    value: name,
    label: AGENT_LABEL[name],
    ...(name === agent ? {} : { disabledReason: `Fixed for this chat · start a new chat to use ${AGENT_LABEL[name]}` }),
  }))
  const summary = `${AGENT_LABEL[agent]} · ${model?.label ?? ''} · ${REASONING_LABEL[choice.reasoning]}`
  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger asChild>
        <button
          type="button"
          aria-label={`Agent, model and reasoning: ${summary}`}
          title="Agent, model and reasoning"
          className="inline-flex h-7 max-w-full items-center gap-1.5 rounded-lg px-2 text-[12px] text-fg2 outline-none hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-[var(--accent)]"
        >
          <span className="truncate">{summary}</span>
          <ChevronUp aria-hidden="true" className="size-3.5 shrink-0" />
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          side="top"
          align="start"
          sideOffset={8}
          aria-label="Agent, model and reasoning"
          className="z-50 w-[min(640px,calc(100vw-32px))] rounded-[14px] border border-line2 bg-[var(--pane)] p-2 shadow-[var(--sh-pop)]"
        >
          <div className="flex gap-2">
            <Column title="Agent" options={agentOptions} value={agent} onPick={() => undefined} />
            <Column
              title="Model"
              options={models.map((option) => ({ value: option.id, label: option.label, blurb: option.blurb }))}
              value={model?.id ?? ''}
              onPick={(value) => { onChange({ model: value }) }}
            />
            <Column
              title="Reasoning"
              options={REASONING_LEVELS.map((level) => ({ value: level, label: REASONING_LABEL[level], extra: <Meter level={level} /> }))}
              value={choice.reasoning}
              onPick={(value) => { onChange({ reasoning: value as Reasoning }) }}
            />
          </div>
          <p className="m-0 mt-2 flex justify-between border-t border-line2 px-2 pt-2 text-[11.5px] text-dim">
            <span>Changes apply from your next message</span>
            <span>Esc</span>
          </p>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  )
}
