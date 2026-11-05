import type { KeyboardEvent } from 'react'
import { ArrowUp, Square, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import type { ChatComposer } from '../useChatComposer'

type Props = { agent: ChatAgent; composer: ChatComposer }

/**
 * The composer (v18 B.3, C.1). Enter sends, Shift+Enter adds a line, Esc or Stop interrupts the
 * running turn. While the agent works, sending queues the message; while a request waits for an
 * answer, or the daemon is offline, the composer is off and says why.
 */
export function Composer({ agent, composer }: Props) {
  const { draft, setDraft, queue, removeQueued, submit, stop, running, sending, disabledReason, error } = composer
  const off = disabledReason !== null
  const placeholder = disabledReason ?? (running ? `Queue a message for when ${agent} finishes` : `Message ${agent}`)

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    // Enter while an input method is composing text confirms the text; it is not a send.
    if (event.nativeEvent.isComposing) return
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      submit()
    } else if (event.key === 'Escape' && running) {
      event.preventDefault()
      stop()
    }
  }

  return (
    <div className="shrink-0 border-t border-line2 px-4 pt-2.5 pb-3">
      <div className="mx-auto w-full max-w-[760px]">
        {queue.length > 0 && (
          <ul className="m-0 mb-2 flex list-none flex-col items-end gap-1.5 p-0" aria-label="Queued messages">
            {queue.map((message) => (
              <li key={message.id} className="flex max-w-[85%] flex-col gap-0.5 rounded-[16px_16px_5px_16px] border border-dashed border-line2 px-3.5 py-2 text-[14px] leading-[1.5] text-fg2">
                <span className="whitespace-pre-wrap [overflow-wrap:anywhere]">{message.text}</span>
                <span className="flex items-center gap-1.5 text-[11.5px] text-dim">
                  Queued · sent when {agent} finishes this turn, even if you close the window
                  <button type="button" className="inline-flex items-center gap-0.5 text-dim underline-offset-2 hover:text-fg hover:underline" onClick={() => { removeQueued(message.id) }}>
                    <X className="size-3" aria-hidden="true" />Remove
                  </button>
                </span>
              </li>
            ))}
          </ul>
        )}
        <div className={`flex items-end gap-2 rounded-[14px] border border-line2 bg-raise px-3 py-2 focus-within:border-[var(--accent-line)] focus-within:ring-2 focus-within:ring-[var(--accent-soft)] ${off ? 'opacity-70' : ''}`}>
          <textarea
            aria-label={`Message ${agent}`}
            rows={1}
            value={draft}
            disabled={off}
            placeholder={placeholder}
            className="max-h-40 min-h-[24px] flex-1 resize-none bg-transparent py-0.5 text-[14px] leading-[1.5] text-fg outline-none [field-sizing:content] placeholder:text-dim disabled:cursor-not-allowed"
            onChange={(event) => { setDraft(event.target.value) }}
            onKeyDown={onKeyDown}
          />
          {running && (
            <Button type="button" variant="outline" size="icon" className="size-8 shrink-0 rounded-lg" aria-label={`Stop ${agent}`} onClick={stop}>
              <Square className="size-3.5 fill-current" aria-hidden="true" />
            </Button>
          )}
          <Button
            type="button"
            size="icon"
            className="size-8 shrink-0 rounded-lg bg-brand text-[var(--on-accent)] hover:bg-brand/85"
            aria-label={running ? 'Queue message' : 'Send message'}
            disabled={off || sending || draft.trim() === ''}
            onClick={submit}
          >
            <ArrowUp className="size-4" aria-hidden="true" />
          </Button>
        </div>
        <p className="m-0 mt-1.5 text-[11.5px] text-dim">
          {running ? `Enter queues it · Esc interrupts ${agent}` : `Same session as the terminal · switching views never restarts ${agent}`}
        </p>
        {error !== null && <p role="alert" className="m-0 mt-1 text-[12px] text-[var(--danger)]">{error}. Your message is kept; send it again to retry.</p>}
      </div>
    </div>
  )
}
