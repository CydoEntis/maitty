import { useId, useState, type ReactNode } from 'react'
import { toast } from '@/lib/toast'
import { errorMessage } from '@/lib/error-message'
import type { PermissionAsk } from '@/data/preview'
import type { ApprovalDecision, ApprovalEvent } from '../chatEvents'
import { ANSWER_KEY, type AskChoice } from '../permissionAsk'
import { keysToPick, type ScreenMenu } from '../screenMenu'
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

/** The tray's frame: "Needs you" and the title in amber for a request, the question alone in the accent when the agent asks. */
function Tray({ label, title, asking = false, children }: { label: string; title: string; asking?: boolean; children: ReactNode }) {
  return (
    <section className={`mx-question${asking ? ' is-asking' : ''}`} aria-label={label}>
      <span className="mx-question-title">{!asking && <span className="mx-question-needs">Needs you</span>}<span>{title}</span></span>
      {children}
    </section>
  )
}

/**
 * A question the agent asks on its screen that its hooks do not report, such as trusting a new
 * folder: its text and one button per choice, each moving the terminal's cursor to the choice and
 * pressing Enter.
 */
export function ScreenMenuTray({ agent, menu, send }: { agent: string; menu: ScreenMenu; send: (keys: string[]) => Promise<void> }) {
  const [busy, setBusy] = useState(false)
  const pick = (index: number) => {
    setBusy(true)
    send(keysToPick(menu, index))
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not send your answer')) })
      .finally(() => { setBusy(false) })
  }
  return (
    <Tray label={`${agent} is asking`} title={`${agent} is asking`} asking>
      {menu.question !== '' && <p className="mx-question-reason" style={{ whiteSpace: 'pre-wrap' }}>{menu.question}</p>}
      <div className="mx-question-answers is-choices">
        {menu.options.map((option, index) => (
          <button key={`${String(index)}-${option}`} type="button" className="mx-answer" disabled={busy} onClick={() => { pick(index) }}>{option}</button>
        ))}
      </div>
    </Tray>
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
    <Tray label={`Approval: ${TITLE[event.approvalKind]}`} title={TITLE[event.approvalKind]} asking={event.approvalKind === 'UserInput'}>
      {event.reason !== null && event.reason !== '' && <p className="mx-question-reason">{event.reason}</p>}
      {detail !== '' && <code className="mx-question-code">{detail}</code>}
      {event.approvalKind === 'UserInput' && actions !== null && event.questions.length > 0 ? (
        <UserInputForm questions={event.questions} busy={actions.busy} onSubmit={(answers) => { actions.answerInput(event.requestId, answers) }} />
      ) : event.approvalKind === 'UserInput' || actions === null ? (
        <p className="mx-question-note">Answer it in the terminal.</p>
      ) : (
        <div className="mx-question-answers">
          {answerable.map((choice) => <AnswerButton key={choice} decision={choice} busy={actions.busy} onAnswer={() => { actions.answer(event.requestId, choice) }} />)}
          {event.canRemember && actions.remember !== undefined && (
            <button type="button" className="mx-answer" disabled={actions.busy} title="Allows this exact command in this project from now on, without asking" onClick={() => { actions.remember?.(event.requestId) }}>
              Always in this project
            </button>
          )}
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
  /** Answers a prompt of several questions: the choice picked for each, in the prompt's order. */
  pickAll?: ((picked: readonly number[]) => Promise<void>) | undefined
}

type AskedQuestion = NonNullable<PermissionAsk['questions']>[number]

/**
 * A prompt that asks several questions: each with its choices to pick from, and one button that sends them all once
 * every question has an answer. Nothing is typed into the agent until then, so an answer can be changed first.
 */
function QuestionsForm({ questions, busy, onSend }: { questions: readonly AskedQuestion[]; busy: boolean; onSend: (picked: readonly number[]) => void }) {
  const name = useId()
  const [picked, setPicked] = useState<readonly (number | null)[]>(() => questions.map(() => null))
  const answers = picked.filter((choice): choice is number => choice !== null)
  const left = questions.length - answers.length
  return (
    <form
      className="mx-question-form"
      onSubmit={(event) => {
        event.preventDefault()
        if (left === 0 && !busy) onSend(answers)
      }}
    >
      {questions.map((asked, at) => (
        <fieldset key={`${String(at)}-${asked.question}`} className="mx-question-group">
          <legend>{asked.question}</legend>
          {asked.choices.map((choice, index) => (
            <label key={`${String(index)}-${choice}`} className="mx-choice">
              <input
                type="radio"
                name={`${name}-${String(at)}`}
                checked={picked[at] === index}
                disabled={busy}
                onChange={() => { setPicked((now) => now.map((was, place) => (place === at ? index : was))) }}
              />
              <span>{choice}</span>
            </label>
          ))}
        </fieldset>
      ))}
      <div className="mx-question-answers">
        <button type="submit" className="mx-answer is-y" disabled={busy || left > 0}>{busy ? 'Sending…' : 'Send answers'}</button>
        {left > 0 && <span className="mx-question-note">{left === questions.length ? 'Pick an answer for each question.' : `${String(left)} more to answer.`}</span>}
      </div>
    </form>
  )
}

/**
 * What an agent in a terminal is stopped on, from its own hook and never the screen (ADR 0003). The
 * buttons exist only where the daemon knows the keys that answer this agent; otherwise the tray says
 * to answer in the terminal rather than guess.
 */
export function PromptTray({ agent, ask, online, answer, pick, pickAll }: PromptProps) {
  const [busy, setBusy] = useState(false)
  const run = (action: Promise<void>) => {
    setBusy(true)
    action
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not send your answer')) })
      .finally(() => { setBusy(false) })
  }
  const choices = ask?.choices ?? []
  const picker = choices.length > 0 && online ? pick : undefined
  const questions = ask?.questions ?? []
  const sendAll = questions.length > 1 && online ? pickAll : undefined
  const canAnswer = ask !== null && ask.answerable && online && answer !== undefined
  const isQuestion = ask?.tool === QUESTION_TOOL
  const title = ask === null ? `${agent} is waiting for you` : isQuestion ? `${agent} has a question for you` : ask.tool === 'Bash' ? 'Wants to run a command' : `Wants to use ${ask.tool}`
  return (
    <Tray label={`${agent} needs your permission`} title={title} asking={isQuestion}>
      {/* Several questions are each shown with their choices; the one line that runs them together is for where they cannot be. */}
      {ask !== null && ask.detail !== '' && sendAll === undefined && (isQuestion ? <p className="mx-question-reason">{ask.detail}</p> : <code className="mx-question-code">{ask.detail}</code>)}
      {sendAll !== undefined ? (
        <QuestionsForm questions={questions} busy={busy} onSend={(picked) => { run(sendAll(picked)) }} />
      ) : picker !== undefined ? (
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
