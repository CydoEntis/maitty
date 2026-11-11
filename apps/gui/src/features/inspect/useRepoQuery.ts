import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { errorMessage } from '@/lib/error-message'

/** A read-only question about a pane's repository, kept fresh while its panel is open. */
export function useRepoQuery<T>(term: number, key: readonly unknown[], load: () => Promise<T>, refetchMs: number | false = false, enabled = true) {
  return useQuery({ queryKey: ['repo', term, ...key], queryFn: load, enabled, retry: false, refetchInterval: refetchMs, refetchOnWindowFocus: false })
}

/**
 * Reads the top folder of a pane's repository before its panel is opened, so Files has its list
 * the moment it shows. A failed read is kept for the tab to say when it opens.
 */
export function usePreloadFiles(term: number | null, loadTop: (term: number) => Promise<unknown>): void {
  const client = useQueryClient()
  useEffect(() => {
    if (term === null) return
    // Nothing to do with a failure here: the query keeps it, and the Files tab is where it is said.
    client.query({ queryKey: ['repo', term, 'tree', ''], queryFn: () => loadTop(term), retry: false }).catch(() => undefined)
  }, [client, term, loadTop])
}

/** The desktop shell rejects with plain strings, so read those too: the message is the daemon's own words. */
export function messageOf(error: unknown): string {
  return errorMessage(error, 'Something went wrong')
}

/** A folder outside any repository is an expected state, not a failure. Git's wording is matched: the daemon runs it in English. */
export function isNotARepository(error: unknown): boolean {
  return messageOf(error).toLowerCase().includes('not a git repository')
}
