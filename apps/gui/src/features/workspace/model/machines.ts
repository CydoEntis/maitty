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

/**
 * Why a shell cannot be started from a pane or project of another machine, or null when it can. A
 * shell of this computer starts in a folder of this computer; the folder another machine reports is
 * only a string it chose, and not a place to start a process here.
 */
export function remoteStartRefusal(remote: boolean, machineName: string | null, what: string): string | null {
  return remote ? `${what} is not available on ${machineName ?? 'another machine'} yet` : null
}
