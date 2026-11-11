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
}

function fileCount(count: number): string {
  return `${String(count)} attached ${count === 1 ? 'file' : 'files'}`
}

/**
 * The messages waiting for the agent's turn to end. Two or more sit as a stack with the next one to be sent in front, so the
 * box stays close to the conversation; a click on a message spreads them out, to edit or remove any one of them. A message's
 * files show as the chips a sent message has, never as the lines that name them to the agent.
 */
export function QueuedMessages({ agent, queue, onRemove, onEdit }: Props) {
  const [spread, setSpread] = useState(false)
  const many = queue.length > 1
  // A stack that is down to one message starts closed the next time it grows.
  if (spread && !many) setSpread(false)
  const stacked = many && !spread
  if (queue.length === 0) return null
  return (
    <ul className={`mx-queue${stacked ? ' is-stacked' : ''}`} aria-label="Queued messages">
      {queue.map((message, index) => {
        const behind = stacked && index > 0
        const parsed = parseAttachmentMessage(message.text)
        const words = parsed?.text ?? message.text
        const files = parsed?.files ?? []
        return (
          <li key={message.id} className={`mx-queued${behind ? ' is-behind' : ''}`} aria-hidden={behind || undefined}>
            {many
              ? <button type="button" className="mx-queued-text" aria-expanded={spread} title={spread ? 'Stack them again' : `Show all ${String(queue.length)} queued messages`} onClick={() => { setSpread((open) => !open) }}>{words === '' ? fileCount(files.length) : words}</button>
              : words !== '' && <span className="mx-queued-text">{words}</span>}
            {files.length > 0 && <SentAttachments files={files} />}
            <span className="mx-queued-note">
              <span>{`Queued · ${stacked ? `first of ${String(queue.length)} · ` : ''}sent when ${agent} finishes this turn, even if you close the window`}</span>
              <button type="button" className="mx-queued-edit" title="Take it back into the message box to change it" onClick={() => { onEdit(message.id) }}><Icon name="pencil" size={10} />Edit</button>
            </span>
            <button type="button" className="mx-queued-remove" aria-label="Remove queued message" title="Remove" onClick={() => { onRemove(message.id) }}><Icon name="close" size={10} /></button>
          </li>
        )
      })}
    </ul>
  )
}
