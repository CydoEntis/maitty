import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'
import { LOCAL_MACHINE } from './machines'
import { taggedAnswers } from './taggedAnswers'

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

const ask = taggedAnswers('folder-listing', AnswerSchema)

/**
 * The folders inside `path` on a machine (this computer is machine 0). A path that is not there, or
 * cannot be read, is a listing with `exists` false, not an error; an error is a machine that could
 * not be asked or did not answer.
 */
export async function listFolders(machine: number, path: string): Promise<FolderListing> {
  const send = (tag: number) => invoke('list_folders', machine === LOCAL_MACHINE ? { path, tag } : { machine, path, tag })
  return (await ask(send, FOLDER_ANSWER_MS, 'The machine did not answer in time')).listing
}
