import { useId, useState } from 'react'
import { Button } from '@/components/ui/button'
import { toast } from '@/lib/toast'
import { errorMessage } from '@/lib/error-message'
import type { PermissionAsk } from '@/data/preview'
import type { ApprovalDecision, ApprovalEvent } from '../chatEvents'
import { ANSWER_KEY, type AskChoice } from '../permissionAsk'
import type { AskedPlan } from '../planApproval'
import { PLAN_TOOL } from '../planApproval'
import { keysToPick, type ScreenMenu } from '../screenMenu'
import type { ApprovalActions } from './ApprovalCard'
import { FileBadge } from './cardParts'
import { Markdown } from './Markdown'
import { UserInputForm } from './UserInputForm'
import { TerminalPrompt } from './TerminalPrompt'
import { WaitingCard } from './WaitingCard'

/** The tool Claude asks a multiple-choice question with; it is a question to the person, not a request to act. */
const QUESTION_TOOL = 'AskUserQuestion'

const TITLE: Record<ApprovalEvent['approvalKind'], string> = {
  Command: 'Wants to run a command',
  FileChange: 'Wants to change files',
  Permission: 'Is requesting additional permissions',
  UserInput: 'Needs your input',
}

type Answer = Exclude<ApprovalDecision, 'Cancel'>

const ANSWER_LABEL: Record<Answer, string> = { Accept: 'Yes once', AcceptForSession: 'This session', Decline: 'No' }

/** One answer: the main one ("Yes once") in the needs colour, the others soft, each with the key that also gives it. */
function AnswerButton({ decision, busy, onAnswer }: { decision: Answer; busy: boolean; onAnswer: () => void }) {
  return (
    <Button variant={decision === 'Accept' ? 'needs' : 'soft'} keyHint={ANSWER_KEY[decision]} disabled={busy} aria-keyshortcuts={ANSWER_KEY[decision]} onClick={onAnswer}>
      {ANSWER_LABEL[decision]}
    </Button>
  )
}

const RUN = (send: () => Promise<void>, setBusy: (busy: boolean) => void) => {
  setBusy(true)
  send()
    .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not send your answer')) })
    .finally(() => { setBusy(false) })
}

/**
 * A question the agent asks on its screen that its hooks do not report, such as trusting a new
 * folder: its text and one button per choice, each moving the terminal's cursor to the choice and
 * pressing Enter.
 */
export function ScreenMenuTray({ agent, menu, send }: { agent: string; menu: ScreenMenu; send: (keys: string[]) => Promise<void> }) {
  const [busy, setBusy] = useState(false)
  const pick = (index: number) => { RUN(() => send(keysToPick(menu, index)), setBusy) }
  return (
    <WaitingCard
      label={`${agent} is asking`}
      title={`${agent} is asking`}
      foot={menu.options.map((option, index) => (
        <Button key={`${String(index)}-${option}`} variant={index === 0 ? 'needs' : 'soft'} disabled={busy} onClick={() => { pick(index) }}>{option}</Button>
      ))}
    >
      {menu.question !== '' && <p className="mx-waiting-text is-pre">{menu.question}</p>}
    </WaitingCard>
  )
}

/** What a request would run or touch: the command on a well, or one badge per file. */
function RequestDetail({ event }: { event: ApprovalEvent }) {
  if (event.command !== null && event.command !== '') {
    return <pre className="mx-waiting-command"><span className="mx-step-prompt" aria-hidden="true">$ </span>{event.command}</pre>
  }
  if (event.changes.length === 0) return null
  return <div className="mx-waiting-files">{event.changes.map((change) => <FileBadge key={change.path} path={change.path} />)}</div>
}

/** A request the daemon holds for a chat it runs: answered by its id, never by typing. */
export function ApprovalTray({ event, actions, agent }: { event: ApprovalEvent; actions: ApprovalActions | null; agent?: string }) {
  const answerable = event.choices.filter((choice): choice is Answer => choice !== 'Cancel')
  const reason = event.reason !== null && event.reason !== '' ? <p className="mx-waiting-text">{event.reason}</p> : null
  const label = `Approval: ${TITLE[event.approvalKind]}`
  if (event.approvalKind === 'UserInput') {
    return (
      <WaitingCard label={label} title={agent === undefined ? TITLE.UserInput : `${agent} needs your input`}>
        {reason}
        {actions !== null && event.questions.length > 0
          ? <UserInputForm questions={event.questions} busy={actions.busy} onSubmit={(answers) => { actions.answerInput(event.requestId, answers) }} />
          : <p className="mx-waiting-note">Answer it in the terminal.</p>}
      </WaitingCard>
    )
  }
  const main = actions !== null && answerable.includes('Accept') ? () => { actions.answer(event.requestId, 'Accept') } : undefined
  const hasCommand = event.command !== null && event.command !== ''
  return (
    <WaitingCard
      label={label}
      title={event.approvalKind === 'FileChange' && event.changes.length > 0 ? `Wants to change ${String(event.changes.length)} ${event.changes.length === 1 ? 'file' : 'files'}` : TITLE[event.approvalKind]}
      aside={agent}
      flush={hasCommand && reason === null}
      onEnter={main}
      foot={actions === null ? <span className="mx-waiting-note">Answer it in the terminal.</span> : (
        <>
          {answerable.map((choice) => <AnswerButton key={choice} decision={choice} busy={actions.busy} onAnswer={() => { actions.answer(event.requestId, choice) }} />)}
          <span className="mx-spacer" />
          {event.canRemember && actions.remember !== undefined && (
            <Button variant="ghost" size="sm" disabled={actions.busy} title="Allows this exact command in this project from now on, without asking" onClick={() => { actions.remember?.(event.requestId) }}>
              Always in this project
            </Button>
          )}
        </>
      )}
    >
      {reason}
      <RequestDetail event={event} />
    </WaitingCard>
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
  /** Types a key into the agent's terminal, for a menu the chat has no buttons for; absent where the window cannot type for the agent. */
  sendKeys?: ((keys: string) => Promise<void>) | undefined
  /** Reads the agent's terminal screen, to show that menu; absent where the screen cannot be read. */
  readScreen?: (() => Promise<string>) | undefined
  /** The plan a plan-approval prompt is about, from the conversation's history. */
  plan?: AskedPlan | null
}

type AskedQuestion = NonNullable<PermissionAsk['questions']>[number]

/** One choice of a question: a box with its number key, lit in the needs colour once picked. */
function Choice({ label, index, picked, busy, onPick, radio }: { label: string; index: number; picked: boolean; busy: boolean; onPick: () => void; radio?: string }) {
  const body = (
    <>
      <span className="mx-option-mark" aria-hidden="true" />
      <span className="mx-option-text">{label}</span>
      <kbd className="mx-option-key" aria-hidden="true">{index + 1}</kbd>
    </>
  )
  if (radio === undefined) {
    return <button type="button" className="mx-option" aria-keyshortcuts={String(index + 1)} disabled={busy} onClick={onPick}>{body}</button>
  }
  return (
    <label className={`mx-option${picked ? ' is-picked' : ''}`}>
      <input type="radio" className="sr-only" name={radio} checked={picked} disabled={busy} onChange={onPick} />
      {body}
    </label>
  )
}

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
      className="mx-waiting-form"
      onSubmit={(event) => {
        event.preventDefault()
        if (left === 0 && !busy) onSend(answers)
      }}
    >
      {questions.map((asked, at) => (
        <fieldset key={`${String(at)}-${asked.question}`} className="mx-waiting-question">
          <legend>{asked.question}</legend>
          {asked.choices.map((choice, index) => (
            <Choice key={`${String(index)}-${choice}`} label={choice} index={index} picked={picked[at] === index} busy={busy} radio={`${name}-${String(at)}`} onPick={() => { setPicked((now) => now.map((was, place) => (place === at ? index : was))) }} />
          ))}
        </fieldset>
      ))}
      <div className="mx-waiting-foot is-form">
        <span className="mx-waiting-note">{left === 0 ? 'Ready to send.' : left === questions.length ? 'Pick an answer for each question.' : `${String(left)} more to answer`}</span>
        <span className="mx-spacer" />
        <Button type="submit" variant="needs" keyHint="⏎" disabled={busy || left > 0}>{busy ? 'Sending…' : 'Send answers'}</Button>
      </div>
    </form>
  )
}

/** Claude's plan, formatted as a reply would be, for the person to approve or send back. */
function PlanBody({ plan }: { plan: AskedPlan | null }) {
  if (plan === null) return <p className="mx-waiting-note">The plan is in the terminal.</p>
  return (
    <>
      <Markdown className="mx-waiting-plan" text={plan.text} />
      {plan.cut && <p className="mx-waiting-note">The history kept only the start of this plan. Read the rest in the terminal.</p>}
    </>
  )
}

/**
 * What an agent in a terminal is stopped on, from its own hook and never the screen (ADR 0003). The
 * buttons exist only where the daemon knows the keys that answer this agent; otherwise the card says
 * to answer in the terminal rather than guess.
 */
export function PromptTray({ agent, ask, online, answer, pick, pickAll, sendKeys, readScreen, plan = null }: PromptProps) {
  const [busy, setBusy] = useState(false)
  const run = (action: () => Promise<void>) => { RUN(action, setBusy) }
  const choices = ask?.choices ?? []
  const picker = choices.length > 0 && online ? pick : undefined
  const questions = ask?.questions ?? []
  const sendAll = questions.length > 1 && online ? pickAll : undefined
  const canAnswer = ask !== null && ask.answerable && online && answer !== undefined
  const isQuestion = ask?.tool === QUESTION_TOOL
  const label = `${agent} needs your permission`
  const mirror = sendKeys !== undefined && online ? sendKeys : undefined

  if (ask?.tool === PLAN_TOOL) {
    return (
      <WaitingCard
        label={label}
        title="Approve this plan?"
        aside="Plan mode"
        onEnter={canAnswer ? () => { run(() => answer('approve')) } : undefined}
        foot={canAnswer ? (
          <>
            <Button variant="needs" keyHint="y" aria-keyshortcuts="y" disabled={busy} title="Picks the first choice in the agent's plan menu" onClick={() => { run(() => answer('approve')) }}>Approve</Button>
            <Button variant="soft" keyHint="n" aria-keyshortcuts="n" disabled={busy} onClick={() => { run(() => answer('deny')) }}>Keep planning</Button>
          </>
        ) : mirror === undefined ? <span className="mx-waiting-note">Answer in the terminal.</span> : null}
      >
        <PlanBody plan={plan} />
        {!canAnswer && mirror !== undefined && <TerminalPrompt readScreen={readScreen} sendKeys={mirror} />}
      </WaitingCard>
    )
  }

  if (sendAll !== undefined) {
    return (
      <WaitingCard label={label} title={`${agent} needs your input`} aside={`${String(questions.length)} questions`}>
        <QuestionsForm questions={questions} busy={busy} onSend={(picked) => { run(() => sendAll(picked)) }} />
      </WaitingCard>
    )
  }

  if (picker !== undefined) {
    return (
      <WaitingCard label={label} title={isQuestion ? `${agent} has a question for you` : `${agent} is asking`}>
        {ask !== null && ask.detail !== '' && <p className="mx-waiting-question-text">{ask.detail}</p>}
        <div className="mx-waiting-options">
          {choices.map((choice, index) => <Choice key={`${String(index)}-${choice}`} label={choice} index={index} picked={false} busy={busy} onPick={() => { run(() => picker(index)) }} />)}
        </div>
      </WaitingCard>
    )
  }

  const title = ask === null ? `${agent} is waiting for you` : isQuestion ? `${agent} has a question for you` : ask.tool === 'Bash' ? 'Wants to run a command' : `Wants to use ${ask.tool}`
  const detail = ask !== null && ask.detail !== '' ? ask.detail : null
  const shownDetail = detail === null ? null : isQuestion ? <p className="mx-waiting-text">{detail}</p> : <pre className="mx-waiting-command">{detail}</pre>

  if (!canAnswer && mirror !== undefined) {
    return (
      <WaitingCard label={label} title={title} aside={agent} flush>
        {shownDetail}
        <TerminalPrompt readScreen={readScreen} sendKeys={mirror} />
      </WaitingCard>
    )
  }

  return (
    <WaitingCard
      label={label}
      title={title}
      aside={agent}
      flush={detail !== null && !isQuestion}
      onEnter={canAnswer ? () => { run(() => answer('approve')) } : undefined}
      foot={canAnswer ? (
        <>
          <AnswerButton decision="Accept" busy={busy} onAnswer={() => { run(() => answer('approve')) }} />
          <AnswerButton decision="Decline" busy={busy} onAnswer={() => { run(() => answer('deny')) }} />
        </>
      ) : <span className="mx-waiting-note">Answer in the terminal.</span>}
    >
      {shownDetail}
    </WaitingCard>
  )
}
