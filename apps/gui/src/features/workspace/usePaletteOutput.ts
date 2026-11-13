import { useEffect, useRef, useState } from 'react'
import { searchPanes, type SearchHit } from '@/features/find/api'
import { OUTPUT_MIN_CHARS } from './model/palette'

/** Each search copies every pane's recent output on the service, so it waits for a pause in typing. */
const SEARCH_DELAY_MS = 300

type Answer = { query: string; hits: SearchHit[] }

/**
 * What the sessions printed that matches the palette's query; empty until the service answers for this exact query.
 * One search runs at a time: a sent search cannot be called off, so while one runs only the newest query is kept,
 * and it is sent when the running one returns.
 */
export function usePaletteOutput(query: string): SearchHit[] {
  const text = query.trim()
  const [answer, setAnswer] = useState<Answer>({ query: '', hits: [] })
  const running = useRef(false)
  const waiting = useRef<string | null>(null)
  const open = useRef(true)

  useEffect(() => {
    open.current = true
    return () => { open.current = false }
  }, [])

  useEffect(() => {
    waiting.current = null
    if (text.length < OUTPUT_MIN_CHARS) return
    const send = (asked: string) => {
      running.current = true
      const settle = (hits: SearchHit[]) => {
        running.current = false
        if (!open.current) return
        setAnswer({ query: asked, hits })
        const next = waiting.current
        waiting.current = null
        if (next !== null && next !== asked) send(next)
      }
      // The palette's own results still stand, so a failed search adds nothing rather than an error per keystroke.
      searchPanes(asked).then(settle, () => { settle([]) })
    }
    const timer = setTimeout(() => {
      if (running.current) waiting.current = text
      else send(text)
    }, SEARCH_DELAY_MS)
    return () => { clearTimeout(timer) }
  }, [text])

  return answer.query === text ? answer.hits : []
}
