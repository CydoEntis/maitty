import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'

const HitSchema = z.object({ term: z.number().int().nonnegative(), line: z.number().int().positive(), text: z.string() })
export type SearchHit = z.infer<typeof HitSchema>

/** Where a word appears in what every session of this app printed recently. */
export async function searchPanes(query: string): Promise<SearchHit[]> {
  if (!isTauri()) throw new Error('Searching every session is only available in the desktop app')
  return z.array(HitSchema).parse(await invoke<unknown>('search_panes', { query }))
}
