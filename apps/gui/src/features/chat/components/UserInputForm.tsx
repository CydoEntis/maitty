import { useId, useState } from 'react'
import { Button } from '@/components/ui/button'
import type { UserInputQuestion } from '../chatEvents'

/** The prefix the daemon accepts for an answer the person wrote beside the offered options. */
const NOTE_PREFIX = 'user_note: '

type Props = {
  questions: readonly UserInputQuestion[]
  /** An answer is on its way. */
  busy: boolean
  /** One answer list per question id, as the daemon takes them. */
  onSubmit: (answers: Record<string, string[]>) => void
}

/** What a question's current choice sends: an offered option as it is, a written one as a note. */
function answerOf(question: UserInputQuestion, choice: string | undefined, note: string): string | null {
  const written = note.trim()
  if (choice === undefined || choice === '') return written === '' ? null : question.options.length === 0 ? written : `${NOTE_PREFIX}${written}`
  return choice
}

/**
 * The questions of a user-input request (Codex's `requestUserInput`), answered in the chat by the
 * request's id. Each needs exactly one answer: an offered option, or, where the provider allows it,
 * the person's own words. A secret is typed hidden, is sent once and is never shown again.
 */
export function UserInputForm({ questions, busy, onSubmit }: Props) {
  const name = useId()
  const [choices, setChoices] = useState<Readonly<Record<string, string>>>({})
  const [notes, setNotes] = useState<Readonly<Record<string, string>>>({})
  const answers = questions.map((question) => [question.id, answerOf(question, choices[question.id], notes[question.id] ?? '')] as const)
  const ready = answers.every(([, answer]) => answer !== null)

  return (
    <form
      className="mx-waiting-form"
      onSubmit={(event) => {
        event.preventDefault()
        if (!ready || busy) return
        onSubmit(Object.fromEntries(answers.map(([id, answer]) => [id, [answer ?? '']])))
      }}
    >
      {questions.map((question) => {
        const choice = choices[question.id]
        const writes = question.options.length === 0 || question.isOther
        return (
          <fieldset key={question.id} className="mx-waiting-question">
            <legend>{question.header !== '' ? `${question.header}: ` : ''}{question.question}</legend>
            {question.options.map((option) => (
              <label key={option.label} className={`mx-option${choice === option.label ? ' is-picked' : ''}`}>
                <input
                  type="radio"
                  className="sr-only"
                  name={`${name}-${question.id}`}
                  checked={choice === option.label}
                  disabled={busy}
                  onChange={() => { setChoices((current) => ({ ...current, [question.id]: option.label })) }}
                />
                <span className="mx-option-mark" aria-hidden="true" />
                <span className="mx-option-text">
                  <span>{option.label}</span>
                  {option.description !== '' && <span className="mx-option-about"> &mdash; {option.description}</span>}
                </span>
              </label>
            ))}
            {writes && (
              <input
                type={question.isSecret ? 'password' : 'text'}
                autoComplete="off"
                spellCheck={false}
                aria-label={question.options.length === 0 ? question.question : `${question.question} (your own answer)`}
                placeholder={question.options.length === 0 ? 'Your answer' : 'Or write your own'}
                value={notes[question.id] ?? ''}
                disabled={busy}
                className="mx-waiting-input"
                onChange={(event) => {
                  setNotes((current) => ({ ...current, [question.id]: event.target.value }))
                  // Writing an answer replaces an option that was picked.
                  setChoices((current) => ({ ...current, [question.id]: '' }))
                }}
              />
            )}
          </fieldset>
        )
      })}
      <div className="mx-waiting-foot is-form">
        <span className="mx-spacer" />
        <Button type="submit" variant="needs" keyHint="⏎" disabled={!ready || busy}>Send answer</Button>
      </div>
    </form>
  )
}
