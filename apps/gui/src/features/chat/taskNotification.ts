/**
 * Claude Code tells the agent that a helper finished with a `<task-notification>` block, and the daemon's
 * history keeps it as a user message. It is the harness talking to the agent, not the person typing, so the
 * chat shows what it says in one quiet line and never the markup.
 */
const BLOCK = /<task-notification>([\s\S]*?)<\/task-notification>/g
const MS_PER_SECOND = 1000
const TOKENS_PER_K = 1000

const tag = (body: string, name: string): string | null => new RegExp(`<${name}>([\\s\\S]*?)</${name}>`).exec(body)?.[1]?.trim() ?? null

/** One line for each notification in the message, or `null` when the message is not made of them. */
export function parseTaskNotifications(text: string): string[] | null {
  const trimmed = text.trim()
  if (!trimmed.startsWith('<task-notification>')) return null
  const lines: string[] = []
  for (const block of trimmed.matchAll(BLOCK)) {
    const body = block[1] ?? ''
    const status = tag(body, 'status')
    const summary = tag(body, 'summary') ?? `Agent ${status ?? 'reported'}`
    const duration = Number(tag(body, 'duration_ms'))
    const tokens = Number(tag(body, 'subagent_tokens'))
    const facts = [
      Number.isFinite(duration) && duration > 0 ? `${String(Math.round(duration / MS_PER_SECOND))}s` : null,
      Number.isFinite(tokens) && tokens > 0 ? `${(tokens / TOKENS_PER_K).toFixed(1)}k tokens` : null,
    ].filter((fact): fact is string => fact !== null)
    lines.push([summary, ...facts].join(' · '))
  }
  return lines.length > 0 ? lines : null
}
