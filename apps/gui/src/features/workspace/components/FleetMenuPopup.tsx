import type { Fleet } from '@/api/fleets'
import { fleetColorOf, FLEET_COLORS } from '../model/fleets'
import type { FleetMenuActions } from './FleetMenu'
import { Icon } from './glyphs'
import { MenuPopup } from './MenuPopup'

type Props = { fleet: Fleet; actions: FleetMenuActions; at: { x: number; y: number }; onClose: () => void }

export function FleetMenuPopup({ fleet, actions, at, onClose }: Props) {
  const current = fleetColorOf(fleet.color)
  return (
    <MenuPopup label={`Menu for ${fleet.name}`} at={at} portalContainer={actions.portalContainer} onClose={onClose}>
      <button type="button" role="menuitem" className="mx-menu-item" onClick={() => { onClose(); actions.onRename() }}><Icon name="pencil" size={13} />Rename<span className="mx-menu-hint" aria-hidden="true">F2</span></button>
      <div className="mx-menu-sep" />
      <div className="mx-menu-label" id="mx-fleet-colour-label">Colour</div>
      <div className="mx-menu-swatches" role="group" aria-labelledby="mx-fleet-colour-label">
        {FLEET_COLORS.map((color) => (
          <button
            key={color}
            type="button"
            role="menuitemradio"
            aria-checked={color === current}
            aria-label={color}
            title={color}
            className={`mx-swatch${color === current ? ' is-on' : ''}`}
            style={{ background: `var(--fleet-${color})` }}
            onClick={() => { onClose(); if (color !== current) actions.onRecolor(color) }}
          />
        ))}
      </div>
    </MenuPopup>
  )
}
