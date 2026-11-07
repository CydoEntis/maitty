import { markConversationsDisconnected } from '@/data/connection'
import type { Conversation } from '@/data/preview'
import type { MachineState } from '@/api/machines'
import { LOCAL_MACHINE, machineNameOf } from '@/api/machines'
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
  for (const tab of layout.arrangements) {
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
export type MachineHead = {
  key: string
  /** The number the host gave the machine this run; what a command for it takes. */
  machine: number
  name: string | null
  label: string
  chip: MachineChip | null
  why: string | null
  /** True once a line that was wanted is not up (not while it is still the first dial). */
  down: boolean
  /** Since when, in ms since the epoch; null when the host did not say. */
  downSince: number | null
  /** What the machine is allowed to do to this computer. */
  grants: { clipboard: boolean; notices: boolean }
  /** The command that brings the app on a machine that needs it up to date, to copy; null when none is needed. */
  updateCommand: string | null
}

const UPDATE_COMMAND = /`(maitty machine update [^`]+)`/

/** The command a needs-update fix names, as the host wrote it (the machine's name is in it). */
export function updateCommandOf(state: MachineState): string | null {
  if (state.state !== 'needs-update') return null
  return UPDATE_COMMAND.exec(state.why ?? '')?.[1] ?? null
}

/** Whether a machine's sessions are shown as last seen: its line is not up and not on its first dial. */
export function isDown(state: MachineState): boolean {
  return state.state !== 'connected' && state.state !== 'connecting'
}

export const THIS_COMPUTER = 'this computer'

/** This computer's header, which the sidebar shows even when there is no other machine. */
export const LOCAL_HEAD: MachineHead = { key: 'machine-0', machine: LOCAL_MACHINE, name: null, label: THIS_COMPUTER, chip: null, why: null, down: false, downSince: null, grants: { clipboard: false, notices: false }, updateCommand: null }

/** The name a machine heading shows. */
export function machineTitle(head: MachineHead): string {
  return head.name ?? 'This computer'
}

/**
 * The headers the sidebar groups its folders under: this computer first, then the others in the
 * order of their ids, which only changes when the machine list does (the sidebar never reorders
 * itself). Nothing while there is no other machine, so one machine looks as it always did.
 */
export function machineHeads(machines: ReadonlyMap<number, RemoteMachine>): MachineHead[] {
  if (machines.size === 0) return []
  const others = [...machines.entries()]
    .sort(([a], [b]) => a - b)
    .map(([id, machine]): MachineHead => ({ key: `machine-${String(id)}`, machine: id, name: machine.state.name, label: machine.state.name, chip: machineChip(machine.state), why: machine.state.why, down: isDown(machine.state), downSince: machine.state.since ?? null, grants: { clipboard: machine.state.clipboard === true, notices: machine.state.notices === true }, updateCommand: updateCommandOf(machine.state) }))
  return [LOCAL_HEAD, ...others]
}

/**
 * Where a new session goes: the machine of the focused session when it is not this computer, else
 * nothing (this computer, its start folder and its folder picker).
 */
export function remoteTargetOf(focused: { machine: number; machineName: string | null } | undefined): { machine: number; name: string } | null {
  if (focused === undefined || focused.machine === LOCAL_MACHINE || focused.machineName === null) return null
  return { machine: focused.machine, name: focused.machineName }
}

/**
 * What a screen reader is told when a machine's line changes: that it went down and why in a word,
 * or that it is back. Nothing for its first connection, for a change that is still down, or for a
 * machine that is only connecting: the sidebar says those, and a reconnect storm would be noise.
 */
export function announcementFor(before: MachineState | undefined, after: MachineState): string | null {
  const wasDown = before !== undefined && isDown(before)
  if (isDown(after)) return wasDown ? null : `${after.name} disconnected: ${machineChip(after).word}`
  if (after.state === 'connected' && wasDown) return `${after.name} is back`
  return null
}

/**
 * Why a shell cannot be started from a pane or project of another machine, or null when it can. A
 * shell of this computer starts in a folder of this computer; the folder another machine reports is
 * only a string it chose, and not a place to start a process here.
 */
export function remoteStartRefusal(remote: boolean, machineName: string | null, what: string): string | null {
  return remote ? `${what} is not available on ${machineName ?? 'another machine'} yet` : null
}
