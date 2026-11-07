import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'
import { addressOf } from './machines'

/**
 * Fleet commands. The host answers `create_fleet`, `rename_fleet`, `recolor_fleet`,
 * `move_session_to_fleet` and `start_shell_in_fleet`; `machine` is the number the host gave the
 * machine (0 for this computer) and `fleet` the daemon's workspace id.
 */
const CreatedFleetSchema = z.number().int().nonnegative().nullable()

/** Makes a fleet; resolves to its id when the host reports it. */
export async function createFleet(machine: number, name: string): Promise<number | null> {
  if (!isTauri()) return null
  const created = await invoke<unknown>('create_fleet', { machine, name })
  const parsed = CreatedFleetSchema.safeParse(created)
  return parsed.success ? parsed.data : null
}

export async function renameFleet(machine: number, fleet: number, name: string): Promise<void> {
  if (isTauri()) await invoke('rename_fleet', { machine, fleet, name })
}

export async function recolorFleet(machine: number, fleet: number, color: number): Promise<void> {
  if (isTauri()) await invoke('recolor_fleet', { machine, fleet, color })
}

/** Moves a session to another fleet of the same machine. */
export async function moveSessionToFleet(machine: number, sessionId: string, fleet: number): Promise<void> {
  const address = addressOf(sessionId)
  if (isTauri() && address) await invoke('move_session_to_fleet', { machine, session: address.term, fleet })
}

/** Starts a plain shell in a fleet, in `folder` or the machine's start folder. */
export async function startShellInFleet(machine: number, fleet: number, folder?: string): Promise<void> {
  if (isTauri()) await invoke('start_shell_in_fleet', { machine, fleet, ...(folder === undefined ? {} : { folder }) })
}
