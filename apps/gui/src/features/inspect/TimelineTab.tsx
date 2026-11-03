import type { TerminalDetails, TurnKind } from '@/data/preview'
import { turnLengths } from './turnLengths'

type Events = TerminalDetails['timeline']

const LABELS: Record<TurnKind, string> = {
  Working: 'Started working',
  NeedsYou: 'Stopped for you',
  Done: 'Finished the turn',
  Idle: 'Idle',
  Prompt: 'Shell prompt',
  CommandStarted: 'Command started',
  CommandFinished: 'Command finished',
}

const SECONDS_PER_MINUTE = 60
const MS_PER_SECOND = 1000

const clock = (unixSeconds: number): string => new Date(unixSeconds * MS_PER_SECOND).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })

function duration(seconds: number): string {
  if (seconds < SECONDS_PER_MINUTE) return `${String(seconds)}s`
  return `${String(Math.floor(seconds / SECONDS_PER_MINUTE))}m ${String(seconds % SECONDS_PER_MINUTE)}s`
}

function describe(event: Events[number]): string {
  if (event.kind === 'CommandFinished') return event.detail === '' ? LABELS.CommandFinished : `${LABELS.CommandFinished} (exit ${event.detail})`
  return LABELS[event.kind]
}

/** What the agent's own reports and the shell's prompt marks say happened in the pane, newest first. */
export function TimelineTab({ events }: { events: Events }) {
  if (events.length === 0) {
    return <p className="mx-inspect-note">Nothing yet. Turns show here as the agent reports them, and commands too in a shell that marks its prompts.</p>
  }
  const lengths = turnLengths(events)
  return (
    <ol className="mx-inspect-list mx-timeline" aria-label="Turn timeline">
      {events.map((event, index) => ({ event, index })).reverse().map(({ event, index }) => (
        <li key={`${String(event.at)}-${String(index)}`} className={`mx-inspect-row mx-timeline-${event.kind}`}>
          <span className="mx-inspect-meta">{clock(event.at)}</span>
          <span className="mx-inspect-main">{describe(event)}</span>
          {lengths.has(index) && <span className="mx-inspect-meta">ran {duration(lengths.get(index) ?? 0)}</span>}
          {event.detail !== '' && event.kind !== 'CommandFinished' && <span className="mx-inspect-meta">{event.detail}</span>}
        </li>
      ))}
    </ol>
  )
}
