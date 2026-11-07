import { fleetKey, type Fleet } from '@/api/fleets'
import { LOCAL_MACHINE } from '@/api/machines'
import { THIS_COMPUTER, type MachineHead } from './machines'
import { matchesFilter, SHELL_PROCESS, type SessionStatus, type SessionView } from './session'

/**
 * Fleets and ships. A fleet is a daemon workspace (a named, coloured group on one machine); a ship is
 * one session in it, an agent or a shell. This file only arranges what the daemon says: it never
 * decides which fleet a session is in, and it never derives a status.
 */

/** The colours a fleet may have. Never amber or red: those mean needs you and an error. */
export const FLEET_COLORS = ['teal', 'violet', 'blue', 'green', 'pink', 'orange'] as const
export type FleetColor = (typeof FLEET_COLORS)[number]

/** The daemon keeps a palette index per workspace; it wraps onto the six fleet colours. */
export function fleetColorOf(index: number): FleetColor {
  return FLEET_COLORS[index % FLEET_COLORS.length] ?? 'teal'
}

const INITIALS_LENGTH = 2

/** Two letters for a fleet's tile: the first letters of two words, else the first two letters. */
export function initialsOf(name: string): string {
  const words = name.trim().split(/[\s_-]+/).filter(Boolean)
  const [first, second] = words
  const letters = first && second ? `${first.charAt(0)}${second.charAt(0)}` : (first ?? '?').slice(0, INITIALS_LENGTH)
  return letters.toUpperCase()
}

/** The heading for this computer, shown even while it is the only machine. */
export const LOCAL_HEAD: MachineHead = { key: 'machine-0', machine: LOCAL_MACHINE, name: null, label: THIS_COMPUTER, chip: null, why: null, down: false, downSince: null, grants: { clipboard: false, notices: false }, updateCommand: null }

/** The machines the FLEETS section lists: this computer always, then the others the host knows. */
export function sidebarMachines(heads: readonly MachineHead[]): MachineHead[] {
  return heads.length === 0 ? [LOCAL_HEAD] : [...heads]
}

/** The name a machine heading shows. */
export function machineTitle(head: MachineHead): string {
  return head.name ?? 'This computer'
}

/** The id of a `Main` fleet this window adds for sessions the daemon holds in no fleet; no command can name it. */
export const NO_FLEET_ID = -1

/** Whether the daemon has this fleet, so a command may name it. */
export function isDaemonFleet(fleet: Pick<Fleet, 'id'>): boolean {
  return fleet.id !== NO_FLEET_ID
}

/**
 * The daemon's fleets, plus a `Main` fleet for a machine that has sessions the daemon lists in no
 * fleet (an older daemon, a chat session) so none of them is left without a place.
 */
export function withHomeFleets(fleets: readonly Fleet[], sessions: ReadonlyArray<Pick<SessionView, 'machine' | 'fleet'>>): Fleet[] {
  const known = new Set(fleets.map((fleet) => fleet.key))
  const homeless = new Set(sessions.filter((session) => session.fleet === null || !known.has(session.fleet)).map((session) => session.machine))
  const added = [...homeless]
    .filter((machine) => !fleets.some((fleet) => fleet.machine === machine))
    .sort((a, b) => a - b)
    .map((machine): Fleet => ({ key: fleetKey(machine, NO_FLEET_ID), machine, id: NO_FLEET_ID, name: 'Main', color: 0, worktree: false, sessionIds: [] }))
  return [...fleets, ...added]
}

/** The fleet a session is shown in: the one that holds it, else its machine's first fleet. */
export function fleetKeyOf(machine: number, held: string | null, fleets: readonly Fleet[]): string | null {
  if (held !== null && fleets.some((fleet) => fleet.key === held)) return held
  return fleets.find((fleet) => fleet.machine === machine)?.key ?? null
}

/** The fleets of one machine, in the daemon's order. */
export function fleetsOfMachine(fleets: readonly Fleet[], machine: number): Fleet[] {
  return fleets.filter((fleet) => fleet.machine === machine)
}

/** Whether a ship is an agent (Claude, Codex, ...) rather than a plain shell. */
export function isAgent(session: Pick<SessionView, 'process'>): boolean {
  return session.process !== SHELL_PROCESS
}

/** The sessions aboard a fleet, in creation order. */
export function aboard(sessions: readonly SessionView[], fleets: readonly Fleet[], key: string): SessionView[] {
  return sessions.filter((session) => fleetKeyOf(session.machine, session.fleet, fleets) === key)
}

/** How many ships of a fleet need you; the badge on its tile. */
export function needsCount(ships: readonly Pick<SessionView, 'status'>[]): number {
  return ships.filter((ship) => ship.status === 'needs').length
}

/** The icons stacked on a fleet row, Claude first, then Codex, other agents, then shells. */
const STACK_MAX = 3

const RANK = { claude: 0, codex: 1, otherAgent: 2, shell: 3 } as const

function stackRank(session: Pick<SessionView, 'process'>): number {
  if (session.process === 'claude') return RANK.claude
  if (session.process === 'codex') return RANK.codex
  return isAgent(session) ? RANK.otherAgent : RANK.shell
}

export type IconStack = { processes: string[]; more: number }

export function iconStack(ships: readonly Pick<SessionView, 'process'>[]): IconStack {
  const ordered = [...ships].sort((a, b) => stackRank(a) - stackRank(b))
  return { processes: ordered.slice(0, STACK_MAX).map((ship) => ship.process), more: Math.max(0, ordered.length - STACK_MAX) }
}

export type Ships = { agents: SessionView[]; shells: SessionView[] }

/** A fleet's ships in the two subgroups, each in creation order. */
export function shipsOf(sessions: readonly SessionView[]): Ships {
  return { agents: sessions.filter(isAgent), shells: sessions.filter((session) => !isAgent(session)) }
}

/** The Priority view's groups, most urgent first. */
export const PRIORITY_GROUPS = [
  { id: 'needs', title: 'Needs you' },
  { id: 'error', title: 'Exited' },
  { id: 'working', title: 'Working' },
  { id: 'done', title: 'Done' },
  { id: 'idle', title: 'Idle' },
] as const
export type PriorityGroupId = (typeof PRIORITY_GROUPS)[number]['id']

function priorityOf(status: SessionStatus): PriorityGroupId {
  // A session whose daemon is gone has no state to rank; it waits with the idle ones, dimmed.
  return status === 'unknown' ? 'idle' : status
}

export type PriorityGroup = { id: PriorityGroupId; title: string; ships: SessionView[] }

/** Every agent across all fleets by how much it needs you. Empty groups are left out. */
export function priorityGroups(sessions: readonly SessionView[]): PriorityGroup[] {
  const agents = sessions.filter(isAgent)
  return PRIORITY_GROUPS
    .map((group) => ({ id: group.id, title: group.title, ships: agents.filter((session) => priorityOf(session.status) === group.id) }))
    .filter((group) => group.ships.length > 0)
}

export function matchesFleet(fleet: Pick<Fleet, 'name'>, query: string): boolean {
  const needle = query.trim().toLowerCase()
  return needle === '' || fleet.name.toLowerCase().includes(needle)
}

/**
 * The fleets the FLEETS section lists while the filter is on: one whose name matches, or that has a
 * ship that does. The selected fleet stays out of this when nothing in it matches; the SHIPS section says so.
 */
export function visibleFleets(fleets: readonly Fleet[], sessions: readonly SessionView[], query: string): Fleet[] {
  if (query.trim() === '') return [...fleets]
  return fleets.filter((fleet) => matchesFleet(fleet, query) || aboard(sessions, fleets, fleet.key).some((ship) => matchesFilter(ship, query)))
}

/** What the SHIPS section says when nothing is listed. */
export function emptyShipsNote(args: { fleetName: string; query: string; hasShips: boolean; otherMatch: string | null }): string {
  const query = args.query.trim()
  if (query === '' || !args.hasShips) return 'Nothing aboard yet. Use + to start a session.'
  const hint = args.otherMatch === null ? '' : ` Try ${args.otherMatch} above.`
  return `Nothing in ${args.fleetName} matches “${query}”.${hint}`
}

/** A fleet name another fleet on the same machine does not already use, case aside. */
export function isFreeFleetName(fleets: readonly Fleet[], machine: number, name: string): boolean {
  const wanted = name.trim().toLowerCase()
  return wanted !== '' && !fleetsOfMachine(fleets, machine).some((fleet) => fleet.name.toLowerCase() === wanted)
}
