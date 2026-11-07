import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'
import { terminalSessionId } from './machines'

/**
 * A fleet is a daemon workspace: a named, coloured group of work on one machine. The window reads
 * them from the same snapshot as the sessions (`list_sessions`, `sessions-changed`,
 * `machine-sessions-changed`) and never invents one; the daemon decides what is in which fleet.
 */
type LayoutNode = { Leaf: number } | { Split: { a: LayoutNode; b: LayoutNode } }
const NodeSchema: z.ZodType<LayoutNode> = z.lazy(() => z.union([
  z.object({ Leaf: z.number().int().nonnegative() }),
  z.object({ Split: z.object({ a: NodeSchema, b: NodeSchema }) }),
]))

const FleetSchema = z.object({
  id: z.number().int().nonnegative(),
  name: z.string(),
  color: z.number().int().nonnegative().default(0),
  worktree: z.boolean().default(false),
  tabs: z.array(z.object({ layout: NodeSchema })).default([]),
})
const FleetsSnapshotSchema = z.object({ workspaces: z.array(z.unknown()).default([]) })

export type Fleet = {
  /** `machine:id`, unique across machines. */
  key: string
  /** The number the host gave the machine, 0 for this computer. */
  machine: number
  /** The daemon's workspace id on that machine; what a fleet command takes. */
  id: number
  name: string
  /** The daemon's palette index; the window maps it onto the fleet colours. */
  color: number
  /** A worktree the app made; its sessions are in a worktree. */
  worktree: boolean
  /** The ids of the sessions the fleet holds, in the order their panes were created. */
  sessionIds: string[]
}

export function fleetKey(machine: number, id: number): string {
  return `${String(machine)}:${String(id)}`
}

function termsOf(node: LayoutNode): number[] {
  return 'Leaf' in node ? [node.Leaf] : [...termsOf(node.Split.a), ...termsOf(node.Split.b)]
}

/** A machine whose snapshot this is: its number for this run, and its name. This computer when left out. */
export type FleetMachine = { id: number; name: string }

/** The fleets in a daemon snapshot, in the daemon's order. A workspace this window cannot read is left out. */
export function fleetsFromSnapshot(raw: unknown, machine?: FleetMachine): Fleet[] {
  const from = machine?.id ?? 0
  const { workspaces } = FleetsSnapshotSchema.parse(raw)
  return workspaces.flatMap((item) => {
    const parsed = FleetSchema.safeParse(item)
    if (!parsed.success) return []
    const { id, name, color, worktree, tabs } = parsed.data
    const terms = [...new Set(tabs.flatMap((tab) => termsOf(tab.layout)))].sort((a, b) => a - b)
    return [{ key: fleetKey(from, id), machine: from, id, name, color, worktree, sessionIds: terms.map((term) => terminalSessionId(term, machine?.name ?? null)) }]
  })
}

/** This computer's fleets, asked for directly (the first list after launch, and when a pushed snapshot did not read). */
export async function loadFleets(): Promise<Fleet[]> {
  if (!isTauri()) return []
  return fleetsFromSnapshot(await invoke<unknown>('list_sessions'))
}
