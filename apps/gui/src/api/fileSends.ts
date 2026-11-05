import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'

/**
 * Sending files dropped on a pane of another machine (ADR 0055). The host checks the paths and the
 * machine, sends the files one after another over the machine's own ssh line, and tells the window
 * how it goes. `transfer` is an id this window makes up, so the events of a send are known before
 * the host has answered.
 */

/** The most files one drop may send; the host refuses more. */
export const MOST_SENT_FILES = 10

const PERCENT_MAX = 100

const Shared = { transfer: z.string() }

const SendEventSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('progress'), ...Shared, file: z.string(), index: z.number().int().nonnegative(), count: z.number().int().positive(), percent: z.number().int().min(0).max(PERCENT_MAX) }),
  z.object({ kind: z.literal('sent'), ...Shared, file: z.string(), index: z.number().int().nonnegative(), count: z.number().int().positive(), remotePath: z.string(), bytes: z.number().int().nonnegative() }),
  z.object({ kind: z.literal('failed'), ...Shared, file: z.string(), message: z.string(), unsent: z.number().int().nonnegative() }),
  z.object({ kind: z.literal('cancelled'), ...Shared, sent: z.number().int().nonnegative() }),
  z.object({ kind: z.literal('done'), ...Shared, sent: z.number().int().nonnegative(), count: z.number().int().positive() }),
])
export type SendEvent = z.infer<typeof SendEventSchema>

/** A fresh id for one send: letters, digits and dashes only, which is all the host accepts. */
export function newTransferId(): string {
  return `send-${crypto.randomUUID()}`
}

/** Start sending `paths` to the machine called `machine`. Rejects with the host's words when a path or the machine is refused; how the send goes comes as events. */
export async function sendFilesToMachine(machine: string, paths: readonly string[], transfer: string): Promise<void> {
  if (!isTauri()) throw new Error('Files can only be sent from the desktop app')
  await invoke('send_files_to_machine', { machine, paths, transfer })
}

/** Stop the send: the file now going is cut off and the rest are not tried. */
export async function cancelFileSend(transfer: string): Promise<void> {
  if (isTauri()) await invoke('cancel_file_send', { transfer })
}

/** Calls back with each thing the host says about a send. */
export function listenFileSends(onEvent: (event: SendEvent) => void): Promise<() => void> {
  return listen<unknown>('file-send', (event) => {
    const parsed = SendEventSchema.safeParse(event.payload)
    if (!parsed.success) {
      console.warn('The host sent a file-send event this window cannot read', parsed.error.issues)
      return
    }
    onEvent(parsed.data)
  })
}
