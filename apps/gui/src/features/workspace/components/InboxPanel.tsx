import { Panel } from '@/components/ui/panel'
import { displayName, STATUS_WORD, truncatePrompt, type SessionView } from '../model/session'
import type { SessionTrack } from '../model/tracking'
import { Ago } from './times'
import { AppIcon, Icon } from './glyphs'

export type InboxEntry = { session: SessionView; projectName: string }

type InboxProps = {
  entries: readonly InboxEntry[]
  tracks: ReadonlyMap<string, SessionTrack>
  onClear: (sessionId: string) => void
  onClearDone: () => void
  onOpen: (sessionId: string) => void
  onClose: () => void
}

/** The one line that says why a session is here: what it is asking, else that it finished or exited. */
function reason(session: SessionView): string {
  if (session.status === 'needs') return session.asking.split('\n').find((line) => line.trim() !== '')?.trim() ?? (session.lastPrompt ? truncatePrompt(session.lastPrompt) : 'Needs you')
  return session.status === 'error' ? 'Exited' : 'Finished'
}

/**
 * ⌘⇧J: every session that needs you, failed or finished, across all projects, as a plain list. A row opens its session (where
 * the answer is given); a session that has finished or exited can also be cleared from the list.
 */
export function InboxPanel({ entries, tracks, onClear, onClearDone, onOpen, onClose }: InboxProps) {
  const settled = entries.some((entry) => entry.session.status !== 'needs')
  return (
    <Panel label="Inbox" onClose={onClose} scrimClassName="mx-inbox-scrim" className="mx-inbox">
      <>
        <div className="mx-inbox-head">
          <strong>Inbox</strong>
          <span className="mx-inbox-sub">{entries.length} across all projects</span>
          <span className="mx-spacer" />
          <button type="button" className="mx-text-button" disabled={!settled} onClick={onClearDone}>Clear done</button>
          <button type="button" className="mx-icon-button mx-inbox-close" aria-label="Close" title="Close (esc)" onClick={onClose}><Icon name="close" size={12} /></button>
        </div>
        <ul className="mx-inbox-list" aria-label="Needs you">
          {entries.map(({ session, projectName }) => {
            const track = tracks.get(session.id)
            const needs = session.status === 'needs'
            return (
              <li key={session.id} className={`mx-inbox-item is-${session.status}`}>
                <button type="button" className="mx-inbox-open" aria-label={`${displayName(session)}, ${STATUS_WORD[session.status]}`} onClick={() => { onOpen(session.id) }}>
                  <AppIcon process={session.process} size={22} />
                  <span className="mx-inbox-text">
                    <span className="mx-inbox-title"><strong>{projectName}</strong> {displayName(session)}</span>
                    <span className="mx-inbox-reason">{reason(session)}{session.machineName ? ` · on ${session.machineName}` : ''}</span>
                  </span>
                  <span className="mx-inbox-aside">
                    <span className={`mx-inbox-word is-${session.status}`}>{needs ? 'Needs you' : session.status === 'error' ? 'Exited' : 'Done'}</span>
                    {track?.changedAt ? <span className="mx-inbox-when"><Ago at={track.changedAt} /></span> : null}
                  </span>
                </button>
                {!needs && (
                  <button type="button" className="mx-inbox-clear" aria-label={`Clear ${displayName(session)}`} title="Clear" onClick={() => { onClear(session.id) }}><Icon name="close" size={12} /></button>
                )}
              </li>
            )
          })}
          {entries.length === 0 && (
            <li className="mx-inbox-empty">
              <span className="mx-inbox-check"><Icon name="check" size={18} /></span>
              <strong>You&apos;re all caught up</strong>
              <span>Nothing needs you, failed or just finished.</span>
            </li>
          )}
        </ul>
      </>
    </Panel>
  )
}
