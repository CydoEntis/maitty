import { AppIcon, StatusBadge } from '@/features/workspace/components/glyphs'
import type { CrewStatus, Participant } from '../model/crew'

type AvatarSize = 20 | 22 | 26 | 28 | 32

/** An agent's mark or a person's round letter, with the status badge on its corner when it has one. */
export function CrewAvatar({ who, size, status }: { who: Participant; size: AvatarSize; status?: CrewStatus | undefined }) {
  return (
    <span className="mx-crew-av" aria-hidden="true">
      {who.agent === undefined
        ? <span className={`mx-crew-person size-${String(size)}`} style={{ background: who.color }}>{who.name.charAt(0)}</span>
        : <AppIcon process={who.agent} size={size} />}
      {status !== undefined && <StatusBadge status={status} />}
    </span>
  )
}
