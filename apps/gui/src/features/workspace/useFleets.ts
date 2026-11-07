import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { Fleet } from '@/api/fleets'
import { fleetKeyOf, withHomeFleets } from './model/fleets'
import { readSelectedFleet, writeSelectedFleet } from './model/fleetStorage'
import type { Layout } from './model/layout'
import type { SessionView } from './model/session'
import { tabsOf } from './model/tabs'

type Args = {
  /** The fleets the daemons list, this computer's first. */
  daemonFleets: readonly Fleet[]
  sessions: readonly SessionView[]
  /** The sessions the sidebar lists, in creation order. */
  sidebarSessions: readonly SessionView[]
  layout: Layout
  focusedId: string | null
}

/**
 * Which fleet is selected and what it holds: the fleets (the daemon's, plus a `Main` for sessions it
 * lists in none), the selected fleet's ships and their tabs. The choice follows the focused ship,
 * so jumping to a ship in another fleet selects that fleet.
 */
export function useFleets({ daemonFleets, sessions, sidebarSessions, layout, focusedId }: Args) {
  const fleets = useMemo(() => withHomeFleets(daemonFleets, sessions), [daemonFleets, sessions])
  // The fleet each ship is shown in, by ship id.
  const fleetOfShip = useMemo(() => new Map(sessions.map((session) => [session.id, fleetKeyOf(session.machine, session.fleet, fleets)] as const)), [sessions, fleets])
  const focusedFleet = (focusedId === null ? null : fleetOfShip.get(focusedId)) ?? null
  const [picked, setPicked] = useState(readSelectedFleet)
  const [seenFocused, setSeenFocused] = useState(focusedFleet)
  if (seenFocused !== focusedFleet) {
    setSeenFocused(focusedFleet)
    if (focusedFleet !== null) setPicked(focusedFleet)
  }
  const selected = fleets.some((fleet) => fleet.key === picked) ? picked : (focusedFleet ?? fleets[0]?.key ?? null)
  const selectedInfo = fleets.find((fleet) => fleet.key === selected)
  useEffect(() => { writeSelectedFleet(selected) }, [selected])
  const shipsIn = useCallback((key: string) => sidebarSessions.filter((ship) => fleetOfShip.get(ship.id) === key), [sidebarSessions, fleetOfShip])
  const ships = useMemo(() => (selected === null ? [] : shipsIn(selected)), [selected, shipsIn])
  const tabs = useMemo(() => tabsOf(layout, ships), [layout, ships])
  // The ship each fleet last showed, so choosing the fleet again comes back to it.
  const lastShip = useRef(new Map<string, string>())
  useEffect(() => { if (focusedFleet !== null && focusedId !== null) lastShip.current.set(focusedFleet, focusedId) }, [focusedFleet, focusedId])
  const rememberedShip = useCallback((key: string) => lastShip.current.get(key), [])
  return { fleets, fleetOfShip, selected, selectedMachine: selectedInfo?.machine, selectedName: selectedInfo?.name, pick: setPicked, shipsIn, tabs, rememberedShip }
}
