import type { Conversation, SessionStatus } from '@/data/preview'

export function AgentAvatar({
  provider,
  status,
  small = false,
}: {
  provider: Conversation['provider']
  status: SessionStatus
  small?: boolean
}) {
  const glyph = provider === 'Codex' ? '◆' : '✳'
  return (
    <span
      className={`agent-avatar ${provider === 'Claude' ? 'is-claude' : 'is-codex'} ${small ? 'is-small' : ''} status-${status}`}
      aria-label={provider}
    >
      {glyph}
    </span>
  )
}
