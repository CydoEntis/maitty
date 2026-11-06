import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'
import { LOCAL_MACHINE } from './machines'

const FolderEntrySchema = z.object({ name: z.string(), git: z.string().nullable(), has_children: z.boolean() })
const FolderListingSchema = z.object({
  path: z.string(),
  exists: z.boolean(),
  entries: z.array(FolderEntrySchema),
  truncated: z.boolean(),
})
const AnswerSchema = z.object({ machine: z.number().int(), tag: z.number().int(), listing: FolderListingSchema })

/** The folders directly inside a folder of a machine. Only folder names, never files. */
export type FolderListing = z.infer<typeof FolderListingSchema>
export type FolderEntry = z.infer<typeof FolderEntrySchema>

/** How long a machine has to answer before the question is given up. */
export const FOLDER_ANSWER_MS = 8000

const waiting = new Map<number, (listing: FolderListing) => void>()
let nextTag = 1
let listening: Promise<void> | null = null

function listenForAnswers(): Promise<void> {
  listening ??= listen<unknown>('folder-listing', (event) => {
    const answer = AnswerSchema.safeParse(event.payload)
    if (!answer.success) return
    const resolve = waiting.get(answer.data.tag)
    waiting.delete(answer.data.tag)
    resolve?.(answer.data.listing)
  }).then(() => undefined)
  listening.catch(() => { listening = null })
  return listening
}

/**
 * The folders inside `path` on a machine (this computer is machine 0). A path that is not there, or
 * cannot be read, is a listing with `exists` false, not an error; an error is a machine that could
 * not be asked or did not answer.
 */
export async function listFolders(machine: number, path: string): Promise<FolderListing> {
  await listenForAnswers()
  const tag = nextTag
  nextTag += 1
  return new Promise<FolderListing>((resolve, reject) => {
    const timer = window.setTimeout(() => {
      waiting.delete(tag)
      reject(new Error('The machine did not answer in time'))
    }, FOLDER_ANSWER_MS)
    waiting.set(tag, (listing) => {
      window.clearTimeout(timer)
      resolve(listing)
    })
    const args = machine === LOCAL_MACHINE ? { path, tag } : { machine, path, tag }
    invoke('list_folders', args).catch((error: unknown) => {
      window.clearTimeout(timer)
      waiting.delete(tag)
      reject(error instanceof Error ? error : new Error(String(error)))
    })
  })
}
