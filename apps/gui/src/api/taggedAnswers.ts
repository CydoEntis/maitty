import { listen } from '@tauri-apps/api/event'
import type { z } from 'zod'

/** Sends a question with the tag it is given, then resolves with the answer that carries that tag. */
type Ask<A> = (send: (tag: number) => Promise<unknown>, withinMs: number, lateWords: string) => Promise<A>

/**
 * Questions to the host whose answers come back later, as `event`s that carry the tag their
 * question was sent with. Each question gets a tag of its own and only the answer with that tag,
 * read through `schema`; anything else on the event is ignored. A question rejects with the host's
 * own words when it could not be sent, and with `lateWords` when nothing answers within `withinMs`.
 */
export function taggedAnswers<A extends { tag: number }>(event: string, schema: z.ZodType<A>): Ask<A> {
  const waiting = new Map<number, (answer: A) => void>()
  let nextTag = 1
  let listening: Promise<void> | null = null

  const listenForAnswers = (): Promise<void> => {
    listening ??= listen<unknown>(event, (told) => {
      const answer = schema.safeParse(told.payload)
      if (!answer.success) return
      const resolve = waiting.get(answer.data.tag)
      waiting.delete(answer.data.tag)
      resolve?.(answer.data)
    }).then(() => undefined)
    listening.catch(() => { listening = null })
    return listening
  }

  return async (send, withinMs, lateWords) => {
    await listenForAnswers()
    const tag = nextTag
    nextTag += 1
    return new Promise<A>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        waiting.delete(tag)
        reject(new Error(lateWords))
      }, withinMs)
      waiting.set(tag, (answer) => {
        window.clearTimeout(timer)
        resolve(answer)
      })
      send(tag).catch((error: unknown) => {
        window.clearTimeout(timer)
        waiting.delete(tag)
        reject(error instanceof Error ? error : new Error(String(error)))
      })
    })
  }
}
