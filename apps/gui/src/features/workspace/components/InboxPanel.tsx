import { displayName, STATUS_WORD, truncatePrompt, type SessionView } from '../model/session'
import { formatAgo, type SessionTrack } from '../model/tracking'
import { AppIcon, Icon } from './glyphs'

export type InboxEntry = { session: SessionView; projectName: string; projectColor: string }

/** The keystrokes the provider's own prompt takes: allow once, allow for this session, deny. */
export type Answer = 'y' | 'a' | 'n'

type InboxProps = {
  entries: readonly InboxEntry[]
  tracks: ReadonlyMap<string, SessionTrack>
  now: number
  online: boolean
  onAnswer: (sessionId: string, answer: Answer) => void
  onClear: (sessionId: string) => void
  onClearDone: () => void
  onOpen: (sessionId: string) => void
  onClose: () => void
}

/** ⌘J: every session that needs you, failed or finished, across all projects. The GUI only types your answer into the session. */
export function InboxPanel({ entries, tracks, now, online, onAnswer, onClear, onClearDone, onOpen, onClose }: InboxProps) {
  const settled = entries.some((entry) => entry.session.status !== 'needs')
  return (
    <div className="mx-inbox-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}>
      <aside role="dialog" aria-label="Inbox" className="mx-inbox" onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); onClose() } }}>
        <div className="mx-inbox-head">
          <strong>Inbox</strong>
          <span className="mx-inbox-sub">{entries.length} across all projects</span>
          <span className="mx-spacer" />
          <button type="button" className="mx-text-button" disabled={!settled} onClick={onClearDone}>Clear done</button>
          <button type="button" className="mx-icon-button mx-inbox-close" aria-label="Close" title="Close (esc)" onClick={onClose}><Icon name="close" size={12} /></button>
        </div>
        <div className="mx-inbox-list">
          {entries.map(({ session, projectName, projectColor }) => {
            const track = tracks.get(session.id)
            const needs = session.status === 'needs'
            const since = track?.changedAt ? formatAgo(now - track.changedAt) : ''
            return (
              <article key={session.id} className={`mx-inbox-card is-${session.status}`} aria-label={`${displayName(session)}, ${STATUS_WORD[session.status]}`}>
                <div className="mx-inbox-card-head">
                  <AppIcon process={session.process} size={22} />
                  <span className="mx-inbox-dot" style={{ background: projectColor }} aria-hidden="true" />
                  <span className="mx-inbox-text">
                    <span className="mx-inbox-title"><strong>{projectName}</strong> {displayName(session)}</span>
                    <span className="mx-inbox-meta">{session.process}{since ? ` · ${since}` : ''}</span>
                  </span>
                  <span className={`mx-inbox-word is-${session.status}`}>{needs ? 'Needs you' : session.status === 'error' ? 'Exited' : 'Done'}</span>
                </div>
                {session.lastPrompt && <div className="mx-inbox-well">{truncatePrompt(session.lastPrompt)}</div>}
                <div className="mx-inbox-actions">
                  {needs ? (
                    <>
                      <button type="button" className="mx-inbox-yes" disabled={!online} onClick={() => { onAnswer(session.id, 'y') }}>Yes, once <kbd>y</kbd></button>
                      <button type="button" disabled={!online} onClick={() => { onAnswer(session.id, 'a') }}>This session <kbd>a</kbd></button>
                      <button type="button" disabled={!online} onClick={() => { onAnswer(session.id, 'n') }}>No <kbd>n</kbd></button>
                    </>
                  ) : (
                    <button type="button" onClick={() => { onClear(session.id) }}>Clear</button>
                  )}
                  <span className="mx-spacer" />
                  <button type="button" onClick={() => { onOpen(session.id) }}>Open</button>
                </div>
              </article>
            )
          })}
          {entries.length === 0 && (
            <div className="mx-inbox-empty">
              <span className="mx-inbox-check"><Icon name="check" size={18} /></span>
              <strong>You&apos;re all caught up</strong>
              <span>Nothing needs you, failed or just finished.</span>
            </div>
          )}
        </div>
      </aside>
    </div>
  )
}
