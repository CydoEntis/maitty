import { useEffect, useState } from 'react'
import { searchPanes, type SearchHit } from '@/features/find/api'
import { OUTPUT_MIN_CHARS } from './model/palette'

/** Each search copies every pane's recent output on the service, so it waits for a pause in typing. */
const SEARCH_DELAY_MS = 250

type Answer = { query: string; hits: SearchHit[] }

/** What the sessions printed that matches the palette's query; empty until the service answers for this exact query. */
export function usePaletteOutput(query: string): SearchHit[] {
  const text = query.trim()
  const [answer, setAnswer] = useState<Answer>({ query: '', hits: [] })
  useEffect(() => {
    if (text.length < OUTPUT_MIN_CHARS) return
    let current = true
    const timer = setTimeout(() => {
      searchPanes(text).then(
        (hits) => { if (current) setAnswer({ query: text, hits }) },
        // The palette's own results still stand, so a failed search adds nothing rather than an error per keystroke.
        () => { if (current) setAnswer({ query: text, hits: [] }) },
      )
    }, SEARCH_DELAY_MS)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [text])
  return answer.query === text ? answer.hits : []
}
