import { displayName, type SessionView } from '../model/session'
import { fleetColorOf } from '../model/fleets'
import type { SessionMenuActions } from './SessionMenu'
import { Icon } from './glyphs'
import { MenuPopup } from './MenuPopup'
import { runsAgent, useArmed } from './useArmed'

type Props = { session: SessionView; actions: SessionMenuActions; at: { x: number; y: number }; onClose: () => void }

export function SessionMenuPopup({ session, actions, at, onClose }: Props) {
  const closing = useArmed(runsAgent(session.process))
  const run = (action: () => void) => () => { onClose(); action() }
  return (
    <MenuPopup label={`Menu for ${displayName(session)}`} at={at} portalContainer={actions.portalContainer} onClose={onClose}>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(actions.onRename)}><Icon name="pencil" size={13} />Rename<span className="mx-menu-hint" aria-hidden="true">F2</span></button>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(() => { actions.onPin(!session.pinned) })}>
        <Icon name="pin" size={13} />{session.pinned ? 'Unpin' : 'Pin'}
      </button>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(() => { actions.onSplit('right') })}><Icon name="splitRight" size={13} />Open in split right</button>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(() => { actions.onSplit('down') })}><Icon name="splitDown" size={13} />Open in split down</button>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={run(actions.onCopyPath)}><span className="mx-sm-gap" />Copy path</button>
      {actions.onGetFile !== undefined && (
        <button type="button" role="menuitem" className="mx-menu-item" disabled={!actions.online} onClick={run(actions.onGetFile)}><span className="mx-sm-gap" />Get a file…</button>
      )}
      {actions.moveTargets.length > 0 && (
        <>
          <div className="mx-menu-sep" />
          <div className="mx-menu-label" id="mx-move-label">Move to fleet</div>
          <div role="group" aria-labelledby="mx-move-label">
            {actions.moveTargets.map((fleet) => (
              <button key={fleet.key} type="button" role="menuitem" className="mx-menu-item" onClick={run(() => { actions.onMove(fleet) })}>
                <span className="mx-menu-swatch" style={{ background: `var(--fleet-${fleetColorOf(fleet.color)})` }} aria-hidden="true" />{fleet.name}
              </button>
            ))}
          </div>
        </>
      )}
      <div className="mx-menu-sep" />
      <button
        type="button"
        role="menuitem"
        className={`mx-menu-item is-danger${closing.armed ? ' is-armed' : ''}`}
        disabled={!actions.online}
        onClick={() => { if (closing.press()) run(actions.onEnd)() }}
      >
        <span className="mx-sm-gap" />{closing.armed ? `Close and stop ${session.process}? Click again` : 'Close session'}<span className="mx-menu-hint" aria-hidden="true">{closing.armed ? '' : 'ends it'}</span>
      </button>
    </MenuPopup>
  )
}
