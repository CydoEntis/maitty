import { useEffect, useRef } from 'react'
import { ResultsList, ResultsOption } from '@/components/ui/results-list'
import type { SlashCommand } from '../slashCommands'

/** The most rows drawn: past this a name is typed further, not scrolled to. */
export const MAX_SHOWN = 50

type Props = {
  agent: string
  /** The commands that match what is typed, closest first. */
  matches: readonly SlashCommand[]
  /** Which of them the keys are on. */
  active: number
  loading: boolean
  error: string | null
  onPick: (command: SlashCommand) => void
  onHover: (index: number) => void
}

/**
 * The agent's commands, over the message box while a slash and a name are being typed: each with what it
 * does and what it takes. The keys stay in the box (the composer moves `active`); a click picks one.
 */
export function SlashPicker({ agent, matches, active, loading, error, onPick, onHover }: Props) {
  const list = useRef<HTMLDivElement>(null)
  // The row the keys are on stays in view as they move through a list longer than the panel.
  useEffect(() => {
    const row = list.current?.children.item(active)
    if (row instanceof HTMLElement && typeof row.scrollIntoView === 'function') row.scrollIntoView({ block: 'nearest' })
  }, [active])
  const shown = matches.slice(0, MAX_SHOWN)
  const note = matches.length > 0 ? null
    : loading ? `Asking ${agent} for its commands…`
    : error !== null ? `Could not list ${agent}'s commands: ${error}`
    : 'No command by that name. Enter sends it as typed.'
  return (
    <div className="mx-slash" role="presentation" onMouseDown={(event) => { event.preventDefault() }}>
      {note !== null && <p className="mx-slash-note" role="status">{note}</p>}
      {shown.length > 0 && (
        <ResultsList ref={list} id="mx-slash-list" className="mx-slash-list" aria-label={`${agent} commands`}>
          {shown.map((command, index) => (
            <ResultsOption
              key={command.name}
              id={`mx-slash-${String(index)}`}
              selected={index === active}
              className={`mx-slash-row${index === active ? ' is-active' : ''}`}
              onMouseMove={() => { if (index !== active) onHover(index) }}
              onClick={() => { onPick(command) }}
            >
              <span className="mx-slash-name">/{command.name}{command.hint !== '' && <span className="mx-slash-hint"> {command.hint}</span>}</span>
              <span className="mx-slash-what">{command.terminal === true && <span className="mx-slash-tag" title="Opens in the terminal">terminal</span>}{command.description}</span>
            </ResultsOption>
          ))}
        </ResultsList>
      )}
      {matches.length > MAX_SHOWN && <p className="mx-slash-note">{matches.length - MAX_SHOWN} more: keep typing to narrow them down.</p>}
      {shown.length > 0 && <p className="mx-slash-keys"><kbd>↑</kbd><kbd>↓</kbd> move · <kbd>Tab</kbd> complete · <kbd>Enter</kbd> run · <kbd>Esc</kbd> close</p>}
    </div>
  )
}
