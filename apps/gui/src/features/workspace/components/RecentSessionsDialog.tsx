import { useRef } from 'react'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import type { ClosedSession } from '@/api/daemon'
import { agentName } from '../model/agentMessage'
import { ageOf, nowSeconds } from '../model/newChatHost'
import { reopenRefusal, useClosedSessions, type RecentRead } from '../model/recentSessions'
import { tildePath } from '../model/session'
import { AppIcon } from './glyphs'

type Props = {
  /** The home folder, so a folder under it reads as `~/…`. */
  home: string
  onPick: (closed: ClosedSession) => void
  onCancel: () => void
}

function Row({ closed, home, now, onPick }: { closed: ClosedSession; home: string; now: number; onPick: Props['onPick'] }) {
  const refusal = reopenRefusal(closed)
  const agent = agentName(closed.agent)
  const where = [agent, tildePath(closed.cwd, home), closed.branch].filter((part): part is string => part !== null && part !== '').join(' · ')
  return (
    <li>
      <button type="button" className="mx-recent-row" disabled={refusal !== null} title={closed.cwd} onClick={() => { onPick(closed) }}>
        <AppIcon process={closed.agent} size={20} />
        <span className="mx-recent-text">
          <span className="mx-recent-title">{closed.title ?? `${agent} session`}</span>
          <span className="mx-recent-where">{where}</span>
        </span>
        <span className="mx-recent-when">{refusal ?? `closed ${ageOf(closed.at, now)} ago`}</span>
      </button>
    </li>
  )
}

function Body({ read, home, onPick, onRetry }: { read: RecentRead; home: string; onPick: Props['onPick']; onRetry: () => void }) {
  if (read.kind === 'loading') return <p role="status">Looking for closed sessions…</p>
  if (read.kind === 'older') return <p role="status">Restart the Maitty service to see closed sessions.</p>
  if (read.kind === 'failed') {
    return (
      <p role="status">
        Could not read the closed sessions.{' '}
        <button type="button" className="mx-text-button" onClick={onRetry}>Try again</button>
      </p>
    )
  }
  if (read.list.length === 0) return <p role="status">No closed sessions yet. A chat you close shows here once its Undo has passed.</p>
  const now = nowSeconds()
  return (
    <ul className="mx-recent-list" aria-label="Closed sessions">
      {read.list.map((closed) => <Row key={`${closed.agent}:${closed.session}:${String(closed.at)}`} closed={closed} home={home} now={now} onPick={onPick} />)}
    </ul>
  )
}

/** The agent sessions closed on this computer, newest first; picking one starts its agent in its folder with its conversation. */
export function RecentSessionsDialog({ home, onPick, onCancel }: Props) {
  const { read, retry } = useClosedSessions()
  const dialog = useRef<HTMLDivElement>(null)
  return (
    <Dialog open onOpenChange={(open) => { if (!open) onCancel() }}>
      <DialogContent unstyled asChild aria-describedby={undefined} onOpenAutoFocus={(event) => { event.preventDefault(); dialog.current?.focus() }}>
        <div
          ref={dialog}
          tabIndex={-1}
          aria-label="Recent sessions"
          className="mx-rename mx-recent"
          onKeyDown={(event) => {
            // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
            if (event.key === 'Escape') event.stopPropagation()
          }}
        >
          <DialogTitle asChild><h2>Recent sessions</h2></DialogTitle>
          <p>Agent sessions closed on this computer, newest first. Picking one starts its agent in that folder with its conversation.</p>
          <Body read={read} home={home} onPick={onPick} onRetry={retry} />
          <div className="mx-rename-actions">
            <button type="button" className="mx-text-button" onClick={onCancel}>Close</button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
