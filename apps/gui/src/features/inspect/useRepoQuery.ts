import { useQuery } from '@tanstack/react-query'

/** A read-only question about a pane's repository, kept fresh while its panel is open. */
export function useRepoQuery<T>(term: number, key: readonly unknown[], load: () => Promise<T>, refetchMs: number | false = false, enabled = true) {
  return useQuery({ queryKey: ['repo', term, ...key], queryFn: load, enabled, retry: false, refetchInterval: refetchMs, refetchOnWindowFocus: false })
}

export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : 'Something went wrong'
}
