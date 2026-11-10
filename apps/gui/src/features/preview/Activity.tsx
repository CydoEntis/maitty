import { useState } from 'react'
import { Check, ChevronRight, X } from 'lucide-react'
import { StatusMark } from '@/components/StatusMark'
import { DiffText } from '@/components/DiffText'
import type { Conversation, Message } from '@/data/preview'
import { ACTIVITY_PREVIEW_LINE_COUNT } from './shared'

const TOOL_LABELS: Record<string, { done: string; running: string }> = {
  commandExecution: { done: 'Ran a command', running: 'Running a command' },
  fileChange: { done: 'Edited files', running: 'Editing files' },
  mcpToolCall: { done: 'Used a tool', running: 'Using a tool' },
  webSearch: { done: 'Searched the web', running: 'Searching the web' },
}

/** The daemon reports a tool by its internal name; show it in plain words. */
function toolLabel(kind: string, running: boolean): string {
  const known = TOOL_LABELS[kind]
  if (known) return running ? known.running : known.done
  if (!/^[a-z]+(?:[A-Z][a-z0-9]*)+$/.test(kind)) return kind
  const words = kind.replace(/([A-Z])/g, ' $1').toLowerCase()
  return words.charAt(0).toUpperCase() + words.slice(1)
}

export function ActivityGroup({
  messages,
  provider,
}: {
  messages: Message[]
  provider: Conversation['provider']
}) {
  const [expanded, setExpanded] = useState(false)
  const statePriority = [
    'Completed',
    'Interrupted',
    'Failed',
    'Running',
  ] as const
  const activityState = statePriority.reduce<(typeof statePriority)[number]>(
    (current, state) =>
      messages.some(
        (message) => (message.activityState ?? 'Completed') === state,
      )
        ? state
        : current,
    'Completed',
  )
  const latestActivity = messages.at(-1)
  const runningActivity = messages.findLast(
    (message) => message.activityState === 'Running',
  )
  const summary =
    activityState === 'Running'
      ? toolLabel(
          runningActivity?.content ?? latestActivity?.content ?? 'Tool',
          true,
        )
      : messages.length === 1
        ? toolLabel(latestActivity?.content ?? 'Tool activity', false)
        : `${String(messages.length)} actions ${activityState.toLowerCase()}`
  const preview = (runningActivity?.detail ?? '')
    .split(/\r?\n/)
    .filter(Boolean)
    .slice(-ACTIVITY_PREVIEW_LINE_COUNT)
  const stateIcon =
    activityState === 'Running' ? (
      <StatusMark status="working" />
    ) : activityState === 'Failed' || activityState === 'Interrupted' ? (
      <X size={13} />
    ) : (
      <Check size={13} />
    )
  return (
    <section
      className={`activity-row activity-${activityState.toLowerCase()} ${expanded ? 'is-expanded' : ''}`}
      aria-label={`${provider} tool activity`}
    >
      <button
        className="activity-trigger"
        type="button"
        onClick={() => {
          setExpanded(!expanded)
        }}
        aria-expanded={expanded}
        aria-label={`${provider} · ${summary} · ${activityState}`}
      >
        <span className="activity-marker" aria-label={activityState}>
          {stateIcon}
        </span>
        <span className="activity-label">{summary}</span>
        <span className="activity-count">
          {messages.length} {messages.length === 1 ? 'step' : 'steps'}
        </span>
        {activityState !== 'Completed' && (
          <span className="activity-state">{activityState}</span>
        )}
        <ChevronRight size={12} />
      </button>
      {!expanded && activityState === 'Running' && preview.length > 0 && (
        <pre className="activity-preview">{preview.join('\n')}</pre>
      )}
      {expanded && (
        <div className="activity-expanded">
          {messages.map((message) => (
            <article className="activity-step" key={message.id}>
              <div className="activity-step-heading">
                <strong>
                  {toolLabel(
                    message.content,
                    message.activityState === 'Running',
                  )}
                </strong>
                <span
                  className={`activity-state activity-state-${(message.activityState ?? 'Completed').toLowerCase()}`}
                >
                  {message.activityState ?? 'Completed'}
                </span>
              </div>
              {message.detail ? (
                message.content === 'fileChange' ? (
                  <DiffText text={message.detail} />
                ) : (
                  <pre>{message.detail}</pre>
                )
              ) : (
                <span className="activity-empty">No output captured.</span>
              )}
            </article>
          ))}
        </div>
      )}
    </section>
  )
}
