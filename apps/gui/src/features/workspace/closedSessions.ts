/** Sessions this window closed on purpose, so the sidebar does not hold their rows as if they had been lost (see `trackMissing`). */
const closed = new Set<string>()

export function markClosed(ids: Iterable<string>): void {
  for (const id of ids) closed.add(id)
}

/** A closed session that Undo brings back is listed again. */
export function unmarkClosed(ids: Iterable<string>): void {
  for (const id of ids) closed.delete(id)
}

export const wasClosed = (id: string): boolean => closed.has(id)
