import { useEffect, useState } from 'react'
import { readHelper } from '@/api/daemon'
import { buildRows, type ChatRow } from './chatRows'
import type { ChatProvider } from './chatEvents'
import { applyHistoryEntries, createChatProjection, selectChatEvents } from './projection'

/** How often a working helper's record is read again. A finished one is read once. */
export const HELPER_POLL_MS = 1500

/** A helper's own steps, or why they could not be read (never its text). */
export type HelperRecordView =
  | { readonly kind: 'rows'; readonly rows: readonly ChatRow[]; readonly startMissing: boolean }
  | { readonly kind: 'unreadable'; readonly reason: string }

/** A helper's record entries as the rows a chat draws. */
export function helperRows(provider: ChatProvider, entries: readonly unknown[], working: boolean, nowMs: number): ChatRow[] {
  const projection = applyHistoryEntries(createChatProjection(provider), entries, nowMs)
  return buildRows(selectChatEvents(projection), working)
}

/**
 * Reads what a helper of one of this computer's panes did, again every so often while it works. Null
 * until the first read, and always for a pane on another machine (`term` null).
 */
export function useHelperRecord(term: number | null, helper: string, provider: ChatProvider, working: boolean): HelperRecordView | null {
  const key = `${String(term)}:${helper}`
  const [state, setState] = useState<{ key: string; view: HelperRecordView } | null>(null)
  useEffect(() => {
    if (term === null) return
    const life = { stopped: false }
    let timer = 0
    const read = async () => {
      try {
        const record = await readHelper(term, helper)
        if (life.stopped) return
        setState({ key, view: { kind: 'rows', rows: helperRows(provider, record.entries, working, Date.now()), startMissing: record.start_missing } })
        if (working) timer = window.setTimeout(() => { void read() }, HELPER_POLL_MS)
      } catch (error) {
        if (life.stopped) return
        // An older daemon, or an agent that keeps no record: the view keeps what the runtime reported.
        setState({ key, view: { kind: 'unreadable', reason: error instanceof Error ? error.message : String(error) } })
      }
    }
    void read()
    return () => {
      life.stopped = true
      window.clearTimeout(timer)
    }
  }, [key, term, helper, provider, working])
  return state?.key === key ? state.view : null
}
