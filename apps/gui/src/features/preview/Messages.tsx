import type { ReactNode } from 'react'
import { StatusMark } from '@/components/StatusMark'
import { Elapsed } from '@/components/Elapsed'
import { DiffStat } from '@/components/DiffStat'
import { useReviewStat } from '@/features/review/useReviewStat'
import { HighlightedCode } from '@/components/HighlightedCode'
import { useWorkingSince } from '@/lib/working-since'
import type { Conversation, Message } from '@/data/preview'
import { CODE_FENCE_MARKER, statusText } from './shared'
import { ActivityGroup } from './Activity'
import { ApprovalMarker } from './Approvals'

export function ConversationHeader({ conversation }: { conversation: Conversation }) {
  const since = useWorkingSince(conversation.id)
  const sandbox = conversation.permission.split(' · ')[0]
  const lastSeen =
    conversation.status === 'unknown' && conversation.lastKnownStatus
      ? ` · last seen ${statusText[conversation.lastKnownStatus].toLowerCase()}`
      : ''
  return (
    <div className="conversation-heading">
      <h1>{conversation.title}</h1>
      <span className="provider-pill">
        <span aria-hidden="true">›_</span>
        {conversation.provider} · {conversation.model}
      </span>
      <span className="conversation-branch">{conversation.branch}</span>
      <span className="conversation-sandbox">{sandbox}</span>
      <span
        className={`conversation-status status-${conversation.status}`}
        role="status"
        aria-label={`Status: ${statusText[conversation.status]}`}
      >
        <StatusMark status={conversation.status} />
        {conversation.status === 'unknown'
          ? 'Unknown'
          : statusText[conversation.status]}
        {conversation.status === 'working' && since !== undefined && (
          <>
            {' · '}
            <Elapsed since={since} />
          </>
        )}
        {lastSeen}
      </span>
    </div>
  )
}

export function MessageRow({
  message,
  provider,
  showLabel,
}: {
  message: Message
  provider: Conversation['provider']
  showLabel: boolean
}) {
  if (message.role === 'activity') {
    return <ActivityGroup messages={[message]} provider={provider} />
  }
  if (message.role === 'approval') return <ApprovalMarker message={message} />
  if (message.role === 'user')
    return (
      <div className="message-row user-message">
        <div className="user-bubble">{message.content}</div>
      </div>
    )
  return (
    <div className="message-row assistant-message">
      {showLabel && <span className="agent-label">{provider}</span>}
      <AssistantContent content={message.content} />
    </div>
  )
}

type AssistantBlock =
  | { kind: 'text'; content: string }
  | { kind: 'code'; language: string; content: string }

function AssistantContent({ content }: { content: string }) {
  const blocks = splitAssistantCode(content)
  return (
    <div className="assistant-copy">
      {blocks.map((block, index) =>
        block.kind === 'code' ? (
          <section
            className="assistant-code-block"
            key={`code-${String(index)}`}
          >
            <div className="assistant-code-language">
              {block.language || 'Code'}
            </div>
            <pre>
              <code>
                <HighlightedCode
                  code={block.content}
                  language={block.language}
                />
              </code>
            </pre>
          </section>
        ) : (
          <p key={`text-${String(index)}`}>{renderInlineCode(block.content)}</p>
        ),
      )}
    </div>
  )
}

function splitAssistantCode(content: string): AssistantBlock[] {
  const blocks: AssistantBlock[] = []
  const textLines: string[] = []
  let codeLines: string[] = []
  let language = ''
  let inCode = false

  for (const line of content.split('\n')) {
    if (line.startsWith(CODE_FENCE_MARKER)) {
      if (inCode) {
        blocks.push({ kind: 'code', language, content: codeLines.join('\n') })
        codeLines = []
        language = ''
        inCode = false
      } else {
        if (textLines.length > 0) {
          blocks.push({ kind: 'text', content: textLines.join('\n') })
          textLines.length = 0
        }
        language = line.slice(CODE_FENCE_MARKER.length).trim()
        inCode = true
      }
      continue
    }

    if (inCode) codeLines.push(line)
    else textLines.push(line)
  }

  if (inCode) {
    textLines.push(`\`\`\`${language}`, ...codeLines)
  }
  if (textLines.length > 0) {
    blocks.push({ kind: 'text', content: textLines.join('\n') })
  }
  return blocks
}

function renderInlineCode(content: string): ReactNode[] {
  return content.split(/(`[^`\n]+`)/g).map((part, index) => {
    if (part.startsWith('`') && part.endsWith('`')) {
      return <code key={String(index)}>{part.slice(1, -1)}</code>
    }
    return part
  })
}

export function FilesChangedButton({
  conversation,
  onReview,
}: {
  conversation: Conversation
  onReview: () => void
}) {
  const stat = useReviewStat(conversation)
  if (!stat || conversation.status === 'working') return null
  return (
    <button type="button" className="files-changed-button" onClick={onReview}>
      {stat.files} {stat.files === 1 ? 'file' : 'files'} changed
      <DiffStat additions={stat.additions} deletions={stat.deletions} />
      <span className="files-changed-review">Review →</span>
    </button>
  )
}

export function StreamingRow({ provider }: { provider: Conversation['provider'] }) {
  return (
    <div className="streaming-row" role="status">
      <StatusMark status="working" />
      <span>{provider} is working</span>
      <span className="stream-caret" aria-hidden="true" />
    </div>
  )
}

export function EmptyConversation({ conversation }: { conversation: Conversation }) {
  return (
    <div className="empty-conversation">
      <h2>
        New {conversation.provider} session in {conversation.project}
      </h2>
      <p>
        <span className="conversation-branch">{conversation.branch}</span> ·{' '}
        {conversation.permission.split(' · ')[0]}. Send a message to start. It
        keeps running in maittyd if you close this window.
      </p>
    </div>
  )
}
