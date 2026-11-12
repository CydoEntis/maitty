import type { KeyboardEvent } from 'react'
import { TriangleAlert } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { ApprovalDecision, ApprovalEvent, ItemStatus } from '../chatEvents'
import type { StepRef } from '../chatRows'
import { StepGlyph } from './cardParts'
import { UserInputForm } from './UserInputForm'

/** What answering an approval needs from the chat; `null` while the answer cannot be sent from here. */
export type ApprovalActions = {
  answer: (requestId: number | string, decision: ApprovalDecision) => void
  /** Answers a user-input request's questions, one list of answers per question id. */
  answerInput: (requestId: number | string, answers: Record<string, string[]>) => void
  /** Approves a command request and has the daemon remember it for the project; absent where it cannot. */
  remember?: (requestId: number | string) => void
  /** An answer is on its way. */
  busy: boolean
}

const CHOICE_LABEL: Record<ApprovalDecision, string> = {
  Accept: 'Yes, once (y)',
  AcceptForSession: 'This session (a)',
  Decline: 'No (n)',
  Cancel: 'Cancel',
}

/** How an answered request reads once it has folded into one line, and the mark it carries. */
const ANSWERED: Record<ApprovalDecision | 'unknown', { word: string; status: ItemStatus }> = {
  Accept: { word: 'Allowed', status: 'completed' },
  AcceptForSession: { word: 'Allowed for this session', status: 'completed' },
  Decline: { word: 'Denied', status: 'failed' },
  Cancel: { word: 'Cancelled', status: 'interrupted' },
  unknown: { word: 'Answered', status: 'interrupted' },
}

/** The keys that answer the focused card, the fallback to typing the same letter in the terminal. */
const KEY_DECISION: Readonly<Record<string, ApprovalDecision>> = { y: 'Accept', a: 'AcceptForSession', n: 'Decline' }

const TITLE: Record<ApprovalEvent['approvalKind'], string> = {
  Command: 'Wants to run a command',
  FileChange: 'Wants to change files',
  Permission: 'Is requesting additional permissions',
  UserInput: 'Needs your input',
}

const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })

type Props = {
  event: ApprovalEvent
  actions: ApprovalActions | null
  /** The tool step this request is about, when the provider named it. */
  step: StepRef | null
  onGoToStep: ((key: string) => void) | null
}

/**
 * A request the agent is waiting on. While it is pending the composer is off. Once answered, here
 * or anywhere else (the daemon keeps the choice), it folds into one line that says what was chosen.
 * While it is pending and focused, y, a and n answer it.
 */
export function ApprovalCard({ event, actions, step, onGoToStep }: Props) {
  const pending = event.resolution.status === 'pending'
  const detail = event.command ?? event.changes.map((change) => change.path).join('\n')
  const canAnswer = pending && actions !== null && event.approvalKind !== 'UserInput'

  if (event.resolution.status === 'answered') {
    // A question's answer is not a permission, so it is never said to be allowed or denied.
    const answered = event.approvalKind === 'UserInput' ? ANSWERED.unknown : ANSWERED[event.resolution.answer]
    return (
      <div role="group" aria-label={`Approval: ${TITLE[event.approvalKind]}`} className="mx-answered">
        <StepGlyph status={answered.status} />
        <strong>{answered.word}</strong>
        {detail !== '' && <span className="mx-answered-detail">{detail.split(/\r?\n/)[0]}</span>}
        <span className="mx-spacer" />
        {step !== null && onGoToStep !== null && <button type="button" className="mx-answered-step" aria-label={`Go to the step this is about: ${step.label}`} onClick={() => { onGoToStep(step.key) }}>Go to step ↑</button>}
        {event.resolution.answeredAtMs != null && <span className="mx-step-meta is-dim">{clock(event.resolution.answeredAtMs)}</span>}
      </div>
    )
  }

  const onKeyDown = (key: KeyboardEvent<HTMLDivElement>) => {
    if (!canAnswer || actions.busy || key.ctrlKey || key.metaKey || key.altKey) return
    const decision = KEY_DECISION[key.key.toLowerCase()]
    if (decision === undefined || !event.choices.includes(decision)) return
    key.preventDefault()
    actions.answer(event.requestId, decision)
  }

  return (
    <div
      role="group"
      aria-label={`Approval: ${TITLE[event.approvalKind]}`}
      // Focusable only while it can be answered, so Tab reaches a pending request and passes an answered one.
      tabIndex={canAnswer ? 0 : undefined}
      aria-keyshortcuts={canAnswer ? 'y a n' : undefined}
      className="rounded-[10px] border border-[var(--warn-line)] bg-[var(--warn-soft)] px-3 py-2.5 outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent)]"
      onKeyDown={onKeyDown}
    >
      <div className="flex items-center gap-2 text-[13px] font-semibold text-fg">
        <TriangleAlert className="size-4 shrink-0 text-[var(--warn)]" aria-hidden="true" />
        {TITLE[event.approvalKind]}
      </div>
      {step !== null && onGoToStep !== null && (
        <button type="button" className="mt-1 max-w-full truncate text-left font-mono text-[11.5px] text-dim underline-offset-2 hover:text-fg hover:underline" aria-label={`Go to the step this is about: ${step.label}`} onClick={() => { onGoToStep(step.key) }}>
          For: {step.label}
        </button>
      )}
      {event.reason !== null && event.reason !== '' && <p className="mt-1.5 mb-0 text-[12.5px] text-fg2">{event.reason}</p>}
      {detail !== '' && <pre className="mt-2 mb-0 max-h-40 overflow-auto rounded-lg border border-line2 bg-term p-2.5 font-mono text-[12px] leading-[1.5] whitespace-pre-wrap text-fg [overflow-wrap:anywhere]">{detail}</pre>}
      {event.approvalKind === 'UserInput' && actions !== null && event.questions.length > 0 ? (
        <UserInputForm questions={event.questions} busy={actions.busy} onSubmit={(answers) => { actions.answerInput(event.requestId, answers) }} />
      ) : event.approvalKind === 'UserInput' || actions === null ? (
        <p className="mt-2 mb-0 text-[12.5px] text-fg2">Waiting for your answer. Answer it in the terminal.</p>
      ) : (
        <div className="mt-2.5 flex flex-wrap items-center gap-2">
          {event.choices.map((choice, index) => (
            <Button
              key={choice}
              type="button"
              size="sm"
              variant={index === 0 ? 'default' : choice === 'AcceptForSession' ? 'secondary' : 'ghost'}
              className={index === 0 ? 'bg-[var(--warn)] text-[var(--on-warn)] hover:bg-[var(--warn)]/85' : ''}
              disabled={actions.busy || !pending}
              onClick={() => { actions.answer(event.requestId, choice) }}
            >
              {CHOICE_LABEL[choice]}
            </Button>
          ))}
          {event.canRemember && actions.remember !== undefined && (
            <Button type="button" size="sm" variant="ghost" disabled={actions.busy || !pending} title="Allows this exact command in this project from now on, without asking" onClick={() => { actions.remember?.(event.requestId) }}>
              Always in this project
            </Button>
          )}
        </div>
      )}
    </div>
  )
}
