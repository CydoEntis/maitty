import type { ChatRow } from './chatRows'

const TASK_CHARS = 700
const ASK_CHARS = 280
const REPLY_CHARS = 900
/** How many of the person's latest requests the summary repeats, besides the first one that set the task. */
const LATEST_ASKS = 4

const clip = (text: string, limit: number): string => {
  const flat = text.trim()
  return flat.length <= limit ? flat : `${flat.slice(0, limit).trimEnd()}…`
}

/**
 * A summary of a chat to open a fresh one with, written from the chat itself and not by a model: the task it began
 * with, the latest things the person asked, and the agent's last reply. It is plain text the person can edit before
 * it is sent, on the new chat page.
 */
export function summaryOf(rows: readonly ChatRow[]): string {
  const asks = rows.flatMap((row) => (row.kind === 'user' && row.event.text.trim() !== '' ? [row.event.text] : []))
  const reply = rows.findLast((row) => row.kind === 'agent')
  const first = asks[0]
  const latest = asks.slice(1).slice(-LATEST_ASKS)
  const parts = ['This continues an earlier chat whose context was nearly full. What it covered:']
  if (first !== undefined) parts.push(`The task it began with:\n${clip(first, TASK_CHARS)}`)
  if (latest.length > 0) parts.push(`What I asked after that:\n${latest.map((ask) => `- ${clip(ask, ASK_CHARS)}`).join('\n')}`)
  if (reply?.kind === 'agent') parts.push(`Where it stood (your last reply there):\n${clip(reply.paragraphs.join('\n\n'), REPLY_CHARS)}`)
  parts.push('Look at the project as it is now, then pick up from here.')
  return parts.join('\n\n')
}
