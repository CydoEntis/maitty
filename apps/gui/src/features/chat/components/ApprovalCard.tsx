import { TriangleAlert } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { ApprovalDecision, ApprovalEvent } from '../chatEvents'

/** What answering an approval needs from the chat; `null` while the answer cannot be sent from here. */
export type ApprovalActions = {
  answer: (requestId: number | string, decision: ApprovalDecision) => void
  /** An answer is on its way. */
  busy: boolean
}

const CHOICE_LABEL: Record<ApprovalDecision, string> = {
  Accept: 'Yes, once (y)',
  AcceptForSession: 'This session (a)',
  Decline: 'No (n)',
  Cancel: 'Cancel',
}

const ANSWER_WORD: Record<ApprovalDecision, string> = {
  Accept: 'Yes, once',
  AcceptForSession: 'This session',
  Decline: 'No',
  Cancel: 'Cancel',
}

const TITLE: Record<ApprovalEvent['approvalKind'], string> = {
  Command: 'Wants to run a command',
  FileChange: 'Wants to change files',
  Permission: 'Is requesting additional permissions',
  UserInput: 'Needs your input',
}

/**
 * A request the agent is waiting on (v18 C.1). While it is pending the composer is off. Once
 * answered it says what was chosen and the buttons are gone, in this view and everywhere else.
 */
export function ApprovalCard({ event, actions }: { event: ApprovalEvent; actions: ApprovalActions | null }) {
  const pending = event.resolution.status === 'pending'
  const detail = event.command ?? event.changes.map((change) => change.path).join('\n')
  return (
    <div role="group" aria-label={`Approval: ${TITLE[event.approvalKind]}`} className="rounded-[10px] border border-[var(--warn-line)] bg-[var(--warn-soft)] px-3 py-2.5">
      <div className="flex items-center gap-2 text-[13px] font-semibold text-fg">
        <TriangleAlert className="size-4 shrink-0 text-[var(--warn)]" aria-hidden="true" />
        {TITLE[event.approvalKind]}
      </div>
      {event.reason !== null && event.reason !== '' && <p className="mt-1.5 mb-0 text-[12.5px] text-fg2">{event.reason}</p>}
      {detail !== '' && <pre className="mt-2 mb-0 max-h-40 overflow-auto rounded-lg border border-line2 bg-term p-2.5 font-mono text-[12px] leading-[1.5] whitespace-pre-wrap text-fg [overflow-wrap:anywhere]">{detail}</pre>}
      {event.resolution.status === 'answered' ? (
        <p className="mt-2 mb-0 text-[12.5px] text-fg2">
          {event.resolution.answer === 'unknown' ? 'Answered.' : `You answered: ${ANSWER_WORD[event.resolution.answer]}`}
        </p>
      ) : event.approvalKind === 'UserInput' || actions === null ? (
        <p className="mt-2 mb-0 text-[12.5px] text-fg2">Waiting for your answer. Answer it in the terminal.</p>
      ) : (
        <div className="mt-2.5 flex flex-wrap gap-2">
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
        </div>
      )}
    </div>
  )
}
