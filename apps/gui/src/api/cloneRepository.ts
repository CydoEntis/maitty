import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'
import { LOCAL_MACHINE } from './machines'

// One of `folder` and `refused` is set: where the clone is, or why there is none.
const AnswerSchema = z.object({
  machine: z.number().int(),
  tag: z.number().int(),
  folder: z.string().nullable(),
  refused: z.string().nullable(),
})
type Answer = z.infer<typeof AnswerSchema>

/**
 * How long a machine has to say how a clone ended. The daemon stops a clone after ten minutes
 * (`CLONE_LIMIT_SECS`) and the host waits half a minute more for the answer to cross: eleven
 * minutes here covers both.
 */
export const CLONE_ANSWER_MS = 660_000

const waiting = new Map<number, (answer: Answer) => void>()
let nextTag = 1
let listening: Promise<void> | null = null

function listenForAnswers(): Promise<void> {
  listening ??= listen<unknown>('repository-cloned', (event) => {
    const answer = AnswerSchema.safeParse(event.payload)
    if (!answer.success) return
    const resolve = waiting.get(answer.data.tag)
    waiting.delete(answer.data.tag)
    resolve?.(answer.data)
  }).then(() => undefined)
  listening.catch(() => { listening = null })
  return listening
}

/**
 * Clones the repository at `address` into a new folder inside `parent` on a machine (this computer
 * is machine 0) and gives the new folder's path. It rejects with the machine's own reason when
 * nothing was cloned: an address it will not clone, a folder already there, a failed sign-in.
 */
export async function cloneRepository(machine: number, address: string, parent: string): Promise<string> {
  await listenForAnswers()
  const tag = nextTag
  nextTag += 1
  return new Promise<string>((resolve, reject) => {
    const timer = window.setTimeout(() => {
      waiting.delete(tag)
      reject(new Error('The machine did not say how the clone ended'))
    }, CLONE_ANSWER_MS)
    waiting.set(tag, (answer) => {
      window.clearTimeout(timer)
      if (answer.folder !== null) resolve(answer.folder)
      else reject(new Error(answer.refused ?? 'The repository was not cloned'))
    })
    const args = machine === LOCAL_MACHINE ? { address, parent, tag } : { machine, address, parent, tag }
    invoke('clone_repository', args).catch((error: unknown) => {
      window.clearTimeout(timer)
      waiting.delete(tag)
      reject(error instanceof Error ? error : new Error(String(error)))
    })
  })
}
