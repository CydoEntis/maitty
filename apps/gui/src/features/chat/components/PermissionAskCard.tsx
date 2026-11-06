import { useState } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { errorMessage } from '@/lib/error-message'
import type { PermissionAsk } from '@/data/preview'
import type { AskChoice } from '../permissionAsk'

/** The tool Claude asks a multiple-choice question with; it is a question to the person, not a request to act. */
const QUESTION_TOOL = 'AskUserQuestion'

type Props = {
  agent: string
  ask: PermissionAsk | null
  /** The daemon is connected, so an answer can be sent. */
  online: boolean
  /** Sends the answer to the agent's prompt; absent where the window cannot answer. */
  answer: ((choice: AskChoice) => Promise<void>) | undefined
  /** Picks the numbered choice of a question the agent asks; absent where the window cannot type for the agent. */
  pick?: ((index: number) => Promise<void>) | undefined
}

/**
 * What a stopped agent is asking, shown in the chat (ADR 0003). The question comes from the agent's
 * own hook, never the screen. The buttons exist only where the daemon knows the key that answers
 * this agent; otherwise the card says to answer in the terminal rather than guess.
 */
export function PermissionAskCard({ agent, ask, online, answer, pick }: Props) {
  const [busy, setBusy] = useState(false)
  const canAnswer = ask !== null && ask.answerable && online && answer !== undefined
  const choices = ask?.choices ?? []
  const canPick = choices.length > 0 && online && pick !== undefined
  const run = (action: Promise<void>) => {
    setBusy(true)
    action
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not send your answer')) })
      .finally(() => { setBusy(false) })
  }
  const send = (choice: AskChoice) => {
    if (answer === undefined || busy) return
    run(answer(choice))
  }
  const choose = (index: number) => {
    if (pick === undefined || busy) return
    run(pick(index))
  }
  const isQuestion = ask?.tool === QUESTION_TOOL
  return (
    <section aria-label={`${agent} needs your permission`} className="mx-ask">
      <header className="mx-ask-head">
        <span className="mx-ask-dot" aria-hidden="true" />
        <strong>{ask === null ? `${agent} is waiting for you` : isQuestion ? `${agent} has a question for you` : `${agent} wants to use ${ask.tool}`}</strong>
      </header>
      {ask !== null && ask.detail !== '' && (isQuestion
        ? <p className="mx-ask-question">{ask.detail}</p>
        : <pre className="mx-ask-detail">{ask.detail}</pre>)}
      {canPick ? (
        <>
          <ul className="mx-ask-choices" aria-label="Choices">
            {choices.map((choice, index) => (
              <li key={`${String(index)}-${choice}`}>
                <button type="button" className="mx-ask-choice" disabled={busy} onClick={() => { choose(index) }}>
                  <kbd>{index + 1}</kbd>
                  <span>{choice}</span>
                </button>
              </li>
            ))}
          </ul>
          <p className="mx-ask-foot">Or type your own answer in the terminal.</p>
        </>
      ) : canAnswer ? (
        <div className="mx-ask-actions">
          <Button size="sm" disabled={busy} onClick={() => { send('approve') }}>Approve once</Button>
          <Button size="sm" variant="outline" disabled={busy} onClick={() => { send('deny') }}>Deny</Button>
        </div>
      ) : (
        <p className="mx-ask-foot">Answer in the terminal.</p>
      )}
    </section>
  )
}
