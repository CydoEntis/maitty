import { useCallback, useMemo, type ReactNode } from 'react'
import type { ChatAgent, ChatLink } from '@/features/workspace/model/chatView'
import { hasPendingApproval, runningTurnStartedAt, type ChatFeed } from '../chatFeed'
import { buildRows, type ChatRow } from '../chatRows'
import { useChatFeed } from '../useChatFeed'
import { ChatItem } from './ChatItems'
import { VirtualList } from './VirtualList'

/** A row not yet measured is drawn as if it were this tall, which a message or a card usually is. */
const ESTIMATED_ROW_PX = 72

const STATUS_WORD: Record<NonNullable<ChatFeed['status']>, string> = {
  None: 'Not started',
  Idle: 'Idle',
  Working: 'Working',
  Blocked: 'Needs you',
  Done: 'Done',
}

const INCOMPLETE_WORD: Record<string, string> = {
  Disconnected: 'the connection to the provider was lost while it ran',
  UnsupportedItem: 'it holds an item this app cannot show',
  InvalidEvent: 'the provider sent an event this app could not read',
  MissingPages: 'some pages could not be read',
  RevisionExhausted: 'it changed more often than the daemon can follow',
}

function incompleteReason(completeness: ChatFeed['completeness']): string | null {
  if (completeness === null || typeof completeness === 'string') return null
  return INCOMPLETE_WORD[completeness.Incomplete] ?? 'part of it could not be read'
}

/** What a screen reader hears when the conversation changes state: one short line per change, never the text. */
function announcement(feed: ChatFeed, agent: ChatAgent): string {
  if (hasPendingApproval(feed)) return `${agent} is waiting for your answer`
  if (feed.status === 'Working') return `${agent} is working`
  if (feed.status === 'Done') return `${agent} finished`
  return ''
}

const rowKey = (row: ChatRow) => row.key

type Props = {
  link: ChatLink
  agent: ChatAgent
  /** The composer, drawn under the list. */
  composer?: (state: { feed: ChatFeed; refresh: () => void }) => ReactNode
}

/** The chat of one session: its history as a virtualised list, the daemon's status, and the composer under it. */
export function ChatSession({ link, agent, composer }: Props) {
  const { feed, events, offline, refresh } = useChatFeed({ conversation: link.conversation, generation: link.generation, provider: agent })
  const working = feed.status === 'Working'
  const rows = useMemo(() => buildRows(events, working), [events, working])
  const workingSince = working ? runningTurnStartedAt(feed) : null
  const incomplete = incompleteReason(feed.completeness)
  const renderRow = useCallback(
    (row: ChatRow) => <ChatItem row={row} agent={agent} workingSince={workingSince} approval={null} />,
    [agent, workingSince],
  )

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-8 shrink-0 items-center gap-2 border-b border-line2 px-4 text-[12px] text-fg2">
        <span className="font-semibold text-fg">{feed.status === null ? 'Reading…' : STATUS_WORD[feed.status]}</span>
        {offline && <span role="status" className="text-[var(--warn)]">Reconnecting to the daemon. What is shown may be out of date.</span>}
        {!offline && incomplete !== null && <span className="text-dim">History is incomplete: {incomplete}.</span>}
        {feed.error !== null && <span className="text-[var(--danger)]">{feed.error}</span>}
      </div>
      {/* Politely announces a change of state only; the log itself is not a live region, so streaming text never floods a screen reader. */}
      <div role="status" aria-live="polite" className="sr-only">{announcement(feed, agent)}</div>
      {feed.loaded && rows.length === 0 ? (
        <p className="m-0 flex flex-1 items-center justify-center px-6 text-center text-[13px] text-dim">No messages yet. Send one below, or type in the terminal.</p>
      ) : feed.loaded ? (
        <VirtualList items={rows} getKey={rowKey} estimate={ESTIMATED_ROW_PX} renderItem={renderRow} label={`Conversation with ${agent}`} />
      ) : (
        <p className="m-0 flex flex-1 items-center justify-center px-6 text-center text-[13px] text-dim">Reading the conversation…</p>
      )}
      {composer?.({ feed, refresh })}
    </div>
  )
}
