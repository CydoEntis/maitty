import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'
import { LOCAL_MACHINE } from './machines'
import { taggedAnswers } from './taggedAnswers'

// One of `folder` and `refused` is set: where the clone is, or why there is none.
const AnswerSchema = z.object({
  machine: z.number().int(),
  tag: z.number().int(),
  folder: z.string().nullable(),
  refused: z.string().nullable(),
})

/**
 * How long a machine has to say how a clone ended. The daemon stops a clone after ten minutes
 * (`CLONE_LIMIT_SECS`) and the host waits half a minute more for the answer to cross: eleven
 * minutes here covers both.
 */
export const CLONE_ANSWER_MS = 660_000

const ask = taggedAnswers('repository-cloned', AnswerSchema)

/**
 * Clones the repository at `address` into a new folder inside `parent` on a machine (this computer
 * is machine 0) and gives the new folder's path. It rejects with the machine's own reason when
 * nothing was cloned: an address it will not clone, a folder already there, a failed sign-in.
 */
export async function cloneRepository(machine: number, address: string, parent: string): Promise<string> {
  const send = (tag: number) => invoke('clone_repository', machine === LOCAL_MACHINE ? { address, parent, tag } : { machine, address, parent, tag })
  const answer = await ask(send, CLONE_ANSWER_MS, 'The machine did not say how the clone ended')
  if (answer.folder !== null) return answer.folder
  throw new Error(answer.refused ?? 'The repository was not cloned')
}
