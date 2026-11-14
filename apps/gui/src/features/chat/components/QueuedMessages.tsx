import { useState } from 'react'
import { Icon } from '@/features/workspace/components/glyphs'
import { parseAttachmentMessage } from '../attachmentMessage'
import type { QueuedItem } from '../useChatComposer'
import { SentAttachments } from './Attachments'

type Props = {
  agent: string
  /** Oldest first: the one that is sent next leads. */
  queue: readonly QueuedItem[]
  onRemove: (id: number) => void
  /** Takes the message out of the queue and puts its words and files back in the message box. */
  onEdit: (id: number) => void
  /** Sends a message into the turn that is running; absent while the agent asks something, or where the chat cannot send. */
  onSendNow?: ((id: number) => void) | undefined
}

function fileCount(count: number): string {
  return `${String(count)} attached ${count === 1 ? 'file' : 'files'}`
}

/**
 * The messages waiting for the agent's turn to end. Two or more sit as a stack with the next one to be sent in front, so the
 * box stays close to the conversation; its header says its place (`1 of 3`), so the stack is not taken for one message,
 * and a click anywhere on the card spreads them out, to edit or remove any one of them. Each card's header holds Edit
 * and Remove. A message's files show as the chips a sent message has, never as
 * the lines that name them to the agent.
 */
export function QueuedMessages({ agent, queue, onRemove, onEdit, onSendNow }: Props) {
  const [spread, setSpread] = useState(false)
  const many = queue.length > 1
  // A stack that is down to one message starts closed the next time it grows.
  if (spread && !many) setSpread(false)
  const stacked = many && !spread
  if (queue.length === 0) return null
  // The agent's daemon holds every one of them, so they go in whether or not this window stays open.
  const sentWhen = `Sent when ${agent} finishes this turn, even if you close the window`
  const messages = queue.map((message) => {
    const parsed = parseAttachmentMessage(message.text)
    const files = parsed?.files ?? []
    const words = parsed?.text ?? message.text
    return { id: message.id, files, words, label: words === '' ? fileCount(files.length) : words }
  })
  return (
    <div className="mx-queue-row">
      <ul className={`mx-queue${stacked ? ' is-stacked' : ''}`} aria-label="Queued messages">
        {messages.map((message, index) => {
          const behind = stacked && index > 0
          return (
            <li key={message.id} className={`mx-queued${many ? ' is-many' : ''}${behind ? ' is-behind' : ''}`} aria-hidden={behind || undefined}>
              <span className="mx-queued-head">
                <span className="mx-queued-place" title={sentWhen}>Queued{many && <span className="mx-queue-place">{`${String(index + 1)} of ${String(queue.length)}`}</span>}</span>
                <span className="mx-spacer" />
                {onSendNow !== undefined && (
                  <button type="button" className="mx-queued-tool is-send" aria-label="Send now" title={`Send it to ${agent} now, without waiting for the turn to end`} onClick={() => { onSendNow(message.id) }}><Icon name="arrowUp" size={12} /></button>
                )}
                <button type="button" className="mx-queued-tool" aria-label="Edit" title="Take it back into the message box to change it" onClick={() => { onEdit(message.id) }}><Icon name="pencil" size={11} /></button>
                <button type="button" className="mx-queued-tool" aria-label="Remove queued message" title="Remove" onClick={() => { onRemove(message.id) }}><Icon name="close" size={10} /></button>
              </span>
              {many
                // The button's own area is stretched over the whole card (CSS), so a click anywhere on it opens the stack.
                ? <button type="button" className="mx-queued-text" aria-expanded={spread} title={spread ? 'Stack them again' : `Show all ${String(queue.length)} queued messages`} onClick={() => { setSpread((open) => !open) }}>{message.label}</button>
                : message.words !== '' && <span className="mx-queued-text">{message.words}</span>}
              {message.files.length > 0 && <SentAttachments files={message.files} />}
            </li>
          )
        })}
      </ul>
    </div>
  )
}
