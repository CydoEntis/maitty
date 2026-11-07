import { useState, type ReactNode } from 'react'
import { toast } from 'sonner'
import { errorMessage } from '@/lib/error-message'
import type { PermissionAsk } from '@/data/preview'
import type { ApprovalDecision, ApprovalEvent } from '../chatEvents'
import { ANSWER_KEY, type AskChoice } from '../permissionAsk'
import type { ApprovalActions } from './ApprovalCard'
import { UserInputForm } from './UserInputForm'

/** The tool Claude asks a multiple-choice question with; it is a question to the person, not a request to act. */
const QUESTION_TOOL = 'AskUserQuestion'

const TITLE: Record<ApprovalEvent['approvalKind'], string> = {
  Command: 'Wants to run a command',
  FileChange: 'Wants to change files',
  Permission: 'Is requesting additional permissions',
  UserInput: 'Needs your input',
}

const ANSWER_LABEL: Record<Exclude<ApprovalDecision, 'Cancel'>, string> = { Accept: 'Yes, once', AcceptForSession: 'This session', Decline: 'No' }

/** The amber tray's frame: the pause tile and the title, then whatever the request holds. */
function Tray({ label, title, children }: { label: string; title: string; children: ReactNode }) {
  return (
    <section className="mx-question" aria-label={label}>
      <span className="mx-question-title"><span className="mx-question-tile" aria-hidden="true"><i /><i /></span>{title}</span>
      {children}
    </section>
  )
}

function AnswerButton({ decision, busy, onAnswer }: { decision: Exclude<ApprovalDecision, 'Cancel'>; busy: boolean; onAnswer: () => void }) {
  return (
    <button type="button" className={`mx-answer is-${ANSWER_KEY[decision]}`} disabled={busy} aria-keyshortcuts={ANSWER_KEY[decision]} onClick={onAnswer}>
      {ANSWER_LABEL[decision]} <kbd>{ANSWER_KEY[decision]}</kbd>
    </button>
  )
}

/** A request the daemon holds for a chat it runs: answered by its id, never by typing. */
export function ApprovalTray({ event, actions }: { event: ApprovalEvent; actions: ApprovalActions | null }) {
  const detail = event.command ?? event.changes.map((change) => change.path).join('\n')
  const answerable = event.choices.filter((choice): choice is Exclude<ApprovalDecision, 'Cancel'> => choice !== 'Cancel')
  return (
    <Tray label={`Approval: ${TITLE[event.approvalKind]}`} title={TITLE[event.approvalKind]}>
      {event.reason !== null && event.reason !== '' && <p className="mx-question-reason">{event.reason}</p>}
      {detail !== '' && <code className="mx-question-code">{detail}</code>}
      {event.approvalKind === 'UserInput' && actions !== null && event.questions.length > 0 ? (
        <UserInputForm questions={event.questions} busy={actions.busy} onSubmit={(answers) => { actions.answerInput(event.requestId, answers) }} />
      ) : event.approvalKind === 'UserInput' || actions === null ? (
        <p className="mx-question-note">Answer it in the terminal.</p>
      ) : (
        <div className="mx-question-answers">
          {answerable.map((choice) => <AnswerButton key={choice} decision={choice} busy={actions.busy} onAnswer={() => { actions.answer(event.requestId, choice) }} />)}
        </div>
      )}
    </Tray>
  )
}

type PromptProps = {
  agent: string
  ask: PermissionAsk | null
  online: boolean
  /** Sends the answer to the agent's prompt; absent where the window cannot answer. */
  answer: ((choice: AskChoice) => Promise<void>) | undefined
  /** Picks the numbered choice of a question the agent asks; absent where the window cannot type for the agent. */
  pick?: ((index: number) => Promise<void>) | undefined
}

/**
 * What an agent in a terminal is stopped on, from its own hook and never the screen (ADR 0003). The
 * buttons exist only where the daemon knows the keys that answer this agent; otherwise the tray says
 * to answer in the terminal rather than guess.
 */
export function PromptTray({ agent, ask, online, answer, pick }: PromptProps) {
  const [busy, setBusy] = useState(false)
  const run = (action: Promise<void>) => {
    setBusy(true)
    action
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not send your answer')) })
      .finally(() => { setBusy(false) })
  }
  const choices = ask?.choices ?? []
  const picker = choices.length > 0 && online ? pick : undefined
  const canAnswer = ask !== null && ask.answerable && online && answer !== undefined
  const isQuestion = ask?.tool === QUESTION_TOOL
  const title = ask === null ? `${agent} is waiting for you` : isQuestion ? `${agent} has a question for you` : ask.tool === 'Bash' ? 'Wants to run a command' : `Wants to use ${ask.tool}`
  return (
    <Tray label={`${agent} needs your permission`} title={title}>
      {ask !== null && ask.detail !== '' && (isQuestion ? <p className="mx-question-reason">{ask.detail}</p> : <code className="mx-question-code">{ask.detail}</code>)}
      {picker !== undefined ? (
        <div className="mx-question-answers is-choices">
          {choices.map((choice, index) => (
            <button key={`${String(index)}-${choice}`} type="button" className="mx-answer" disabled={busy} onClick={() => { run(picker(index)) }}>
              {choice} <kbd>{index + 1}</kbd>
            </button>
          ))}
        </div>
      ) : canAnswer ? (
        <div className="mx-question-answers">
          <AnswerButton decision="Accept" busy={busy} onAnswer={() => { run(answer('approve')) }} />
          <AnswerButton decision="Decline" busy={busy} onAnswer={() => { run(answer('deny')) }} />
        </div>
      ) : (
        <p className="mx-question-note">Answer in the terminal.</p>
      )}
    </Tray>
  )
}
