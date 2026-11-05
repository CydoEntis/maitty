import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'
import { LOCAL_MACHINE } from '@/api/machines'

const HitSchema = z.object({ term: z.number().int().nonnegative(), line: z.number().int().positive(), text: z.string() })
/** Where a word appears: the machine and the pane on it, the line, and its text. Only this computer's panes are searched. */
export type SearchHit = z.infer<typeof HitSchema> & { machine: number }

/** Where a word appears in what every session of this app printed recently. */
export async function searchPanes(query: string): Promise<SearchHit[]> {
  if (!isTauri()) throw new Error('Searching every session is only available in the desktop app')
  return z.array(HitSchema).parse(await invoke<unknown>('search_panes', { query })).map((hit) => ({ ...hit, machine: LOCAL_MACHINE }))
}
