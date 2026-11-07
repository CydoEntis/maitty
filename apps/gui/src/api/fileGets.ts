import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'

/**
 * Pulling a file from a session's machine to this computer (ADR 0007). The host checks the machine
 * and the path, pulls the file over the machine's own ssh line into Downloads/maitty/<machine>/,
 * and tells the window how it goes. `transfer` is an id this window makes up, so the events of a
 * pull are known before the host has answered. Stopping one uses `cancelFileSend`: the host keeps
 * one list of running transfers.
 */

const PERCENT_MAX = 100

const Shared = { transfer: z.string(), file: z.string() }

const GetEventSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('progress'), ...Shared, percent: z.number().int().min(0).max(PERCENT_MAX) }),
  z.object({ kind: z.literal('done'), ...Shared, path: z.string(), folder: z.string() }),
  z.object({ kind: z.literal('failed'), ...Shared, message: z.string() }),
  z.object({ kind: z.literal('cancelled'), ...Shared }),
])
export type GetEvent = z.infer<typeof GetEventSchema>

/** A fresh id for one pull: letters, digits and dashes only, which is all the host accepts. */
export function newGetId(): string {
  return `get-${crypto.randomUUID()}`
}

/** Start pulling `path` from the machine called `machine`. Rejects with the host's words when the machine or the path is refused; how the pull goes comes as events. */
export async function getFileFromMachine(machine: string, path: string, transfer: string): Promise<void> {
  if (!isTauri()) throw new Error('Files can only be pulled from the desktop app')
  await invoke('get_file_from_machine', { machine, path, transfer })
}

const ListingSchema = z.object({
  folder: z.string(),
  dirs: z.array(z.string()),
  files: z.array(z.string()),
  truncated: z.boolean(),
  hidden: z.number().int().nonnegative(),
})
const IndexSchema = z.object({
  folder: z.string(),
  files: z.array(z.string()),
  truncated: z.boolean(),
  hidden: z.number().int().nonnegative(),
})
/** What is in one folder of a machine, as the host cleaned and capped it. */
export type RemoteListing = z.infer<typeof ListingSchema>
/** The files below a folder of a machine, as paths below `folder`. */
export type RemoteIndex = z.infer<typeof IndexSchema>

/** What is in the folder `path` on `machine`; an empty path is its home folder. Rejects with the host's words. */
export async function listRemoteFolder(machine: string, path: string): Promise<RemoteListing> {
  if (!isTauri()) throw new Error('Folders can only be listed from the desktop app')
  return ListingSchema.parse(await invoke('list_remote_folder', { machine, path }))
}

/** The files below `folder` on `machine`, for a search. Rejects with the host's words. */
export async function indexRemoteFiles(machine: string, folder: string): Promise<RemoteIndex> {
  if (!isTauri()) throw new Error('Folders can only be searched from the desktop app')
  return IndexSchema.parse(await invoke('index_remote_files', { machine, folder }))
}

/** Show a folder pulled files were saved in. The host opens only folders it saved into. */
export async function openPulledFolder(folder: string): Promise<void> {
  if (isTauri()) await invoke('open_pulled_folder', { folder })
}

/** Calls back with each thing the host says about a pull. */
export function listenFileGets(onEvent: (event: GetEvent) => void): Promise<() => void> {
  return listen<unknown>('file-get', (event) => {
    const parsed = GetEventSchema.safeParse(event.payload)
    if (!parsed.success) {
      console.warn('The host sent a file-get event this window cannot read', parsed.error.issues)
      return
    }
    onEvent(parsed.data)
  })
}
