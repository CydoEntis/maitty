import { AppIcon, StatusBadge } from '@/features/workspace/components/glyphs'

/**
 * The mark before "is thinking" in a chat: the agent's own tile with the working spinner at its bottom right corner,
 * the way a session's row marks a working agent, so the line says who is at work and not only that something is.
 */
export function WorkingMark({ agent }: { agent: string }) {
  return (
    <span className="mx-row-icon mx-working-mark" aria-hidden="true">
      <AppIcon process={agent} size={18} />
      <StatusBadge status="working" />
    </span>
  )
}
