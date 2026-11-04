import { markConversationsDisconnected } from '@/data/connection'
import type { Conversation } from '@/data/preview'
import type { MachineState } from '@/api/machines'
import { machineNameOf } from '@/api/machines'
import type { Layout } from './layout'

/** What the window holds of one other machine: how its line is, and the sessions it last listed. */
export type RemoteMachine = {
  state: MachineState
  /** The terminal sessions of its last snapshot, with ids that carry the machine's name. */
  conversations: readonly Conversation[]
  /** Its first snapshot has come: before that, a layout's panes for it are not known to be gone. */
  heard: boolean
}

/**
 * The sessions of the machines other than this computer, in the order of their names. A machine
 * whose line is not up keeps its last sessions, shown as unknown the way a lost daemon's are,
 * so a row does not vanish and its last status is still told.
 */
export function remoteTerminals(machines: ReadonlyMap<number, RemoteMachine>): Conversation[] {
  return [...machines.values()]
    .sort((a, b) => a.state.name.localeCompare(b.state.name, undefined, { sensitivity: 'base' }))
    .flatMap((machine) => {
      const terminals = machine.conversations.filter((item) => item.kind === 'terminal')
      return machine.state.state === 'connected' ? terminals : markConversationsDisconnected([...terminals])
    })
}

/**
 * The sessions of a layout's panes that belong to a machine that has not listed its sessions yet.
 * A layout read at launch is not emptied of them: they are only unknown until the machine answers.
 */
export function idsOfMachinesNotHeard(layout: Layout, machines: ReadonlyMap<number, RemoteMachine>): string[] {
  const heard = new Set([...machines.values()].filter((machine) => machine.heard).map((machine) => machine.state.name.toLowerCase()))
  const ids = new Set<string>()
  for (const tab of layout.tabs) {
    for (const pane of tab.panes) {
      const name = machineNameOf(pane.sessionId)
      if (name !== null && !heard.has(name.toLowerCase())) ids.add(pane.sessionId)
    }
  }
  return [...ids]
}

/** How a chip is coloured: up, on its way, or needing a person. Amber stays for agents that need you. */
export type ChipTone = 'ok' | 'quiet' | 'bad'

/** A machine's state as a header shows it: a shape and a word, so colour is never the only signal. */
export type MachineChip = { glyph: string; word: string; tone: ChipTone }

export function machineChip(state: MachineState): MachineChip {
  switch (state.state) {
    case 'connected':
      return { glyph: '●', word: 'connected', tone: 'ok' }
    case 'connecting':
      return { glyph: '○', word: 'connecting', tone: 'quiet' }
    case 'reconnecting':
      return { glyph: '◐', word: state.attempt === null ? 'reconnecting' : `reconnecting #${String(state.attempt)}`, tone: 'quiet' }
    case 'needs-login':
      return { glyph: '▲', word: 'needs login', tone: 'bad' }
    case 'needs-update':
      return { glyph: '▲', word: 'needs update', tone: 'bad' }
    case 'unreachable':
      return { glyph: '✕', word: 'unreachable', tone: 'bad' }
    case 'refused':
      return { glyph: '✕', word: 'refused', tone: 'bad' }
  }
}

/** One machine's header in the sidebar. `name` is null for this computer. */
export type MachineHead = { key: string; name: string | null; label: string; chip: MachineChip | null; why: string | null }

export const THIS_COMPUTER = 'this computer'

/**
 * The headers the sidebar groups its folders under: this computer first, then the others in the
 * order of their ids, which only changes when the machine list does (the sidebar never reorders
 * itself). Nothing while there is no other machine, so one machine looks as it always did.
 */
export function machineHeads(machines: ReadonlyMap<number, RemoteMachine>): MachineHead[] {
  if (machines.size === 0) return []
  const others = [...machines.entries()]
    .sort(([a], [b]) => a - b)
    .map(([id, machine]): MachineHead => ({ key: `machine-${String(id)}`, name: machine.state.name, label: machine.state.name, chip: machineChip(machine.state), why: machine.state.why }))
  return [{ key: 'machine-0', name: null, label: THIS_COMPUTER, chip: null, why: null }, ...others]
}
