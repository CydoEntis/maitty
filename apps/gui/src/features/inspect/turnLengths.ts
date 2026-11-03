import type { TerminalDetails } from '@/data/preview'

/** How long each turn ran, by the index of its Working event: from there to the next event that is not working. */
export function turnLengths(events: TerminalDetails['timeline']): Map<number, number> {
  const lengths = new Map<number, number>()
  events.forEach((event, index) => {
    if (event.kind !== 'Working') return
    const end = events.slice(index + 1).find((later) => later.kind === 'Done' || later.kind === 'NeedsYou' || later.kind === 'Idle')
    if (end) lengths.set(index, end.at - event.at)
  })
  return lengths
}
