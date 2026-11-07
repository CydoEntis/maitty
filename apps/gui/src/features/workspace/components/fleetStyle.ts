import type { CSSProperties } from 'react'
import type { Fleet } from '@/api/fleets'
import { fleetColorOf } from '../model/fleets'

/** The longest name a fleet may be given. */
export const MAX_FLEET_NAME_LENGTH = 40

/** Sets `--fleet` to the fleet's colour for the rules in `fleets.css`. */
export function fleetStyle(fleet: Pick<Fleet, 'color'>): CSSProperties {
  return { '--fleet': `var(--fleet-${fleetColorOf(fleet.color)})` } as CSSProperties
}
