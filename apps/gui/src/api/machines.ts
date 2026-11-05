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

/** Forget one machine's number: its sessions can no longer be addressed. */
export function unregisterMachine(name: string): void {
  idsByName.delete(name.toLowerCase())
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

export const MACHINE_KINDS = ['connecting', 'connected', 'reconnecting', 'needs-login', 'needs-update', 'unreachable', 'refused'] as const
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
  /** Since when the machine has not been connected, in ms since the epoch. */
  since: z.number().nullable().optional(),
  /** What the machine may do to this computer, from its entry in config.local.toml; off unless it says. */
  clipboard: z.boolean().optional(),
  notices: z.boolean().optional(),
  sounds: z.boolean().optional(),
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


/** Dial a machine now; a daemon that was refused is looked at again. */
export async function retryMachine(name: string): Promise<void> {
  if (isTauri()) await invoke('retry_machine', { name })
}

/**
 * Take a machine out of the list. Its sessions keep running on it and the layout saved under its
 * name is kept. Rejects with the host's words when the entry could not be removed.
 */
export async function removeMachine(name: string): Promise<void> {
  if (isTauri()) await invoke('remove_machine', { name })
}

/** Calls back with the number of a machine that was removed from the list. */
export function listenMachineRemoved(onRemoved: (machine: number) => void): Promise<() => void> {
  return listen<unknown>('machine-removed', (event) => {
    const machine = z.number().int().positive().safeParse(event.payload)
    if (machine.success) onRemoved(machine.data)
  })
}

const HostChoiceSchema = z.object({ alias: z.string(), detail: z.string() })
export type HostChoice = z.infer<typeof HostChoiceSchema>

/** The hosts of ~/.ssh/config that are not machines yet. */
export async function listSshHosts(): Promise<HostChoice[]> {
  if (!isTauri()) return []
  return z.array(HostChoiceSchema).parse(await invoke<unknown>('list_ssh_hosts'))
}

const AddedSchema = z.discriminatedUnion('outcome', [
  z.object({ outcome: z.literal('added'), name: z.string(), version: z.string().nullable() }),
  z.object({ outcome: z.literal('login'), name: z.string() }),
  z.object({ outcome: z.literal('login-manual'), name: z.string(), target: z.string(), why: z.string() }),
  z.object({ outcome: z.literal('needs-name') }),
  z.object({ outcome: z.literal('cancelled') }),
])
/** What adding a machine came to. A machine that could not be added rejects with the words to show. */
export type Added = z.infer<typeof AddedSchema>

/**
 * Look at `target` over ssh without asking anyone anything and, when it holds up, add it and start
 * dialling it. `name` is only needed when none can be made from the target.
 */
export async function addMachine(target: string, name?: string): Promise<Added> {
  if (!isTauri()) throw new Error('Machines can only be added in the desktop app')
  return AddedSchema.parse(await invoke<unknown>('add_machine', { target, name: name ?? null }))
}

/** End the look that is running. */
export async function cancelAddMachine(): Promise<void> {
  if (isTauri()) await invoke('cancel_add_machine')
}

/** Start a terminal session in a folder of another machine; the host says why it could not. */
export async function startSessionOnMachine(machine: number, folder: string): Promise<void> {
  if (isTauri()) await invoke('start_session_on_machine', { machine, folder })
}

/** Calls back with what a machine that is allowed to raise notices wants shown (already cleaned, led by its name). */
export function listenMachineNotice(onNotice: (text: string) => void): Promise<() => void> {
  return listen<unknown>('machine-notice', (event) => {
    const notice = MachineErrorSchema.safeParse(event.payload)
    if (notice.success) onNotice(notice.data.text)
  })
}

/** What a machine may be allowed to do to this computer, that the window can act on. */
export type Grant = 'clipboard' | 'notices'

/** Allow or stop one of a machine's powers; written to config.local.toml. Rejects with the host's words. */
export async function setMachineGrant(name: string, grant: Grant, on: boolean): Promise<void> {
  if (isTauri()) await invoke('set_machine_grant', { name, grant, on })
}

/** Calls back with text a machine that was allowed to asks to be copied (cut to a size the host allows). */
export function listenMachineClipboard(onCopy: (machine: number, text: string) => void): Promise<() => void> {
  return listen<unknown>('machine-clipboard', (event) => {
    const copy = MachineErrorSchema.safeParse(event.payload)
    if (copy.success) onCopy(copy.data.machine, copy.data.text)
  })
}
