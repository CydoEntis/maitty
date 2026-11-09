import { useState, type CSSProperties } from 'react'
import { Popover } from 'radix-ui'
import { PopupContent } from '@/components/ui/popup'
import { Icon } from '@/features/workspace/components/glyphs'
import { noteLines } from './noteLines'
import type { UpdaterState } from './useUpdater'

const PERCENT = 100
const CARD_GAP_PX = 8
const CARD_EDGE_PX = 12

type Props = {
  variant: 'footer' | 'rail'
  state: UpdaterState
  /** The running version. */
  version: string | null
  /** A release the user chose to skip: it gets no pill. */
  skipped: string | null
  portalContainer: HTMLElement | null
  onInstall: () => void
  onRestartNow: () => void
  onCheck: () => void
  onSkip: (version: string) => void
}

type Offer = Exclude<UpdaterState, { status: 'idle' | 'checking' | 'current' }>

function offerOf(state: UpdaterState, skipped: string | null): Offer | null {
  if (state.status === 'available') return state.version === skipped ? null : state
  // A failed check is not something you started, so it gets no pill; Settings still says so.
  if (state.status === 'failed') return state.during === 'update' ? state : null
  if (state.status === 'downloading' || state.status === 'ready') return state
  return null
}

/** The sidebar footer's left slot: the version, or what an update needs from you, with a card that opens upward. */
export function UpdateSlot({ variant, state, version, skipped, portalContainer, onInstall, onRestartNow, onCheck, onSkip }: Props) {
  const [open, setOpen] = useState(false)
  const offer = offerOf(state, skipped)

  if (!offer) {
    if (variant === 'rail' || version === null) return null
    return <span className="mx-version">Version <span className="mono">{version}</span></span>
  }

  const percent = offer.status === 'downloading' && offer.progress !== null ? Math.round(offer.progress * PERCENT) : null
  const label = offer.status === 'available' ? `Update to ${offer.version}`
    : offer.status === 'downloading' ? (percent === null ? 'Downloading…' : `${String(percent)}%`)
    : offer.status === 'ready' ? 'Restart to update'
    : 'Update failed'
  // An update that is ready to install is the glass button with the falling arrow; a download and a failure keep their own pills.
  const glass = variant !== 'rail' && (offer.status === 'available' || offer.status === 'ready')
  const cardBody = (() => {
    switch (offer.status) {
      case 'available': {
        const lines = noteLines(offer.notes)
        return (
          <>
            <h3>Maitty {offer.version} is available</h3>
            {version && <p className="mx-upd-sub">You have {version}</p>}
            {lines.length > 0 && <ul className="mx-upd-notes">{lines.map((line) => <li key={line}>{line}</li>)}</ul>}
            <div className="mx-upd-actions">
              <button type="button" className="mx-upd-primary" onClick={() => { onInstall() }}>Download</button>
              <button type="button" className="mx-upd-ghost" onClick={() => { onSkip(offer.version); setOpen(false) }}>Skip this version</button>
              <button type="button" className="mx-upd-ghost" onClick={() => { setOpen(false) }}>Later</button>
            </div>
          </>
        )
      }
      case 'downloading':
        return (
          <>
            <h3>Downloading Maitty {offer.version}</h3>
            <div className="mx-upd-bar" role="progressbar" aria-valuemin={0} aria-valuemax={PERCENT} aria-valuenow={percent ?? undefined}><span style={{ width: `${String(percent ?? 0)}%` }} /></div>
            <p className="mx-upd-sub">{percent === null ? 'Working out the size…' : `${String(percent)}%`}</p>
            <div className="mx-upd-actions"><button type="button" className="mx-upd-ghost" onClick={() => { setOpen(false) }}>Hide</button></div>
          </>
        )
      case 'ready':
        return (
          <>
            <h3>Maitty {offer.version} is ready</h3>
            <ul className="mx-upd-notes is-safe">
              <li>Your sessions keep running in maittyd.</li>
              <li>Your tabs and splits reopen.</li>
              <li>Agents in the middle of a turn are not interrupted.</li>
            </ul>
            <div className="mx-upd-actions">
              <button type="button" className="mx-upd-primary" onClick={() => { setOpen(false); onRestartNow() }}>Restart now</button>
              <button type="button" className="mx-upd-ghost" onClick={() => { setOpen(false) }}>Later</button>
            </div>
          </>
        )
      case 'failed':
        return (
          <>
            <h3>Update failed</h3>
            <p className="mx-upd-error">{offer.message}</p>
            <div className="mx-upd-actions">
              <button type="button" className="mx-upd-primary" onClick={() => { onCheck() }}>Retry</button>
              <button type="button" className="mx-upd-ghost" onClick={() => { void navigator.clipboard.writeText(offer.message) }}>Copy details</button>
            </div>
          </>
        )
    }
  })()

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger asChild>
      <button
        type="button"
        className={`mx-upd-pill is-${offer.status} is-${variant}`}
        aria-label={variant === 'rail' ? `${label}. Open update details` : glass ? label : undefined}
        style={percent === null ? undefined : { '--p': `${String(percent)}%` } as CSSProperties}
      >
        {variant === 'rail' ? <Icon name="download" /> : (
          <>
            {glass && <span className="mx-upd-shimmer" aria-hidden="true" />}
            {glass && (
              <span className="mx-upd-tile" aria-hidden="true">
                <span className="mx-upd-arrow">
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round"><path d="M12 4v12M6.5 10.5 12 16l5.5-5.5" /></svg>
                  <span className="mx-upd-arrow-bar" />
                </span>
              </span>
            )}
            {offer.status === 'downloading' && <span className="mx-upd-ring" aria-hidden="true" />}
            {offer.status === 'available' ? <><span className="mx-upd-word">Update</span><span className="mx-upd-version">{offer.version}</span></> : glass ? <span className="mx-upd-word">Restart</span> : label}
          </>
        )}
      </button>
      </Popover.Trigger>
      <PopupContent side="top" align={portalContainer?.dataset.side === 'right' ? 'end' : 'start'} sideOffset={CARD_GAP_PX} collisionPadding={CARD_EDGE_PX} aria-label="Update" className="mx-upd-card" onEscapeKeyDown={(event) => { event.stopPropagation() }}>{cardBody}</PopupContent>
    </Popover.Root>
  )
}
