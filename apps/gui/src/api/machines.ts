import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'

/**
 * Machines, as the window knows them (ADR 0048): a name, and a number the host gave it for this
 * run. The number is what the host's commands take; the name is what the window shows and what a
 * saved layout keeps, because the number changes whenever the machine list does. This computer is
 * machine 0 and has no name in a session's id.
 */
export const LOCAL_MACHINE = 0

const idsByName = new Map<string, number>()

/** Remember which number the host gave a machine this run. Names are compared in any case. */
export function registerMachine(name: string, machine: number): void {
  idsByName.set(name.toLowerCase(), machine)
}

/** Forget every machine. For tests: the registry belongs to the window's lifetime. */
export function forgetMachines(): void {
  idsByName.clear()
}

/** Where a terminal session lives: its machine, and its pane's number on that machine. */
export type SessionAddress = { machine: number; machineName: string | null; term: number }

const SESSION_ID = /^(?:([A-Za-z0-9][A-Za-z0-9._-]*)\/)?t(\d+)$/

/**
 * A terminal session's id. A pane of this computer is `t12`; a pane of another machine is
 * `build/t12`, so two machines' pane 12 never meet and a saved layout survives a restart.
 */
export function terminalSessionId(term: number, machineName: string | null = null): string {
  return machineName === null ? `t${String(term)}` : `${machineName}/t${String(term)}`
}

/** The pane number of a terminal session id of any machine; null for anything else (a chat's number). */
export function terminalOf(sessionId: string): number | null {
  const match = SESSION_ID.exec(sessionId)
  return match?.[2] === undefined ? null : Number(match[2])
}

/** The name of the machine a session id belongs to; null for this computer or a chat. */
export function machineNameOf(sessionId: string): string | null {
  return SESSION_ID.exec(sessionId)?.[1] ?? null
}

/**
 * Where a terminal session id points. Null for anything that is not one, and for a machine the
 * host has not told the window about (this run): its commands cannot be sent anywhere.
 */
export function parseSessionId(sessionId: string): SessionAddress | null {
  const match = SESSION_ID.exec(sessionId)
  if (match?.[2] === undefined) return null
  const term = Number(match[2])
  const name = match[1]
  if (name === undefined) return { machine: LOCAL_MACHINE, machineName: null, term }
  const machine = idsByName.get(name.toLowerCase())
  return machine === undefined ? null : { machine, machineName: name, term }
}

/**
 * The arguments that name a session's machine to the host. Nothing for this computer, so every
 * call about one of its own panes is exactly what it was before machines.
 */
export function machineArgs(address: SessionAddress): { machine?: number } {
  return address.machine === LOCAL_MACHINE ? {} : { machine: address.machine }
}

/** The address of a terminal session, or why a command for it cannot be sent. */
export function addressOf(sessionId: string): SessionAddress | null {
  const address = parseSessionId(sessionId)
  if (address !== null) return address
  const name = machineNameOf(sessionId)
  if (name !== null) throw new Error(`${name} is not one of this computer's machines`)
  return null
}

export const MACHINE_KINDS = ['connecting', 'connected', 'reconnecting', 'needs-login', 'needs-update', 'unreachable'] as const
export type MachineKind = (typeof MACHINE_KINDS)[number]

const MachineStateSchema = z.object({
  machine: z.number().int().nonnegative(),
  name: z.string(),
  state: z.enum(MACHINE_KINDS),
  /** What a person has to do, when one has to. */
  why: z.string().nullable(),
  /** Which dial a reconnect is on. */
  attempt: z.number().nullable(),
  /** On connecting: how many keystrokes the machine could not take while it was down. */
  unsent: z.number().nullable().optional(),
})
export type MachineState = z.infer<typeof MachineStateSchema>

const MachineOverviewSchema = MachineStateSchema.extend({ snapshot: z.unknown().nullable() })
export type MachineOverview = z.infer<typeof MachineOverviewSchema>

/** Every machine but this computer: its state, and the snapshot it last gave. For a window that started late. */
export async function loadMachinesOverview(): Promise<MachineOverview[]> {
  if (!isTauri()) return []
  const overview = z.array(MachineOverviewSchema).parse(await invoke<unknown>('list_machines_overview'))
  for (const machine of overview) registerMachine(machine.name, machine.machine)
  return overview
}

/** Calls back each time a machine's connection changes state (this computer's too, as machine 0). */
export function listenMachineState(onState: (state: MachineState) => void): Promise<() => void> {
  return listen<unknown>('machine-state', (event) => {
    const state = MachineStateSchema.safeParse(event.payload)
    if (!state.success) {
      console.warn('The host sent a machine state this window cannot read', state.error.issues)
      return
    }
    registerMachine(state.data.name, state.data.machine)
    onState(state.data)
  })
}

const MachineSessionsSchema = z.object({ machine: z.number().int().positive(), snapshot: z.unknown().nullable() })

/** Calls back with another machine's whole snapshot each time it changes. */
export function listenMachineSessions(onChange: (machine: number, snapshot: unknown) => void): Promise<() => void> {
  return listen<unknown>('machine-sessions-changed', (event) => {
    const sessions = MachineSessionsSchema.safeParse(event.payload)
    if (!sessions.success) {
      console.warn('The host sent a machine snapshot this window cannot read', sessions.error.issues)
      return
    }
    onChange(sessions.data.machine, sessions.data.snapshot)
  })
}

const MachineErrorSchema = z.object({ machine: z.number().int().positive(), text: z.string() })

/** Calls back with what another machine's daemon said went wrong with a request (already cleaned, led by its name). */
export function listenMachineError(onError: (text: string) => void): Promise<() => void> {
  return listen<unknown>('machine-error', (event) => {
    const error = MachineErrorSchema.safeParse(event.payload)
    if (error.success) onError(error.data.text)
  })
}

