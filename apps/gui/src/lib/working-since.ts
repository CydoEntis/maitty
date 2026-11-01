import { createContext, useContext } from 'react'

const MS_PER_SECOND = 1000
const SECONDS_PER_MINUTE = 60
const SECOND_DIGITS = 2

/**
 * When the GUI watched a conversation start working. A conversation that was
 * already working when the GUI first saw it has no entry: the daemon does not
 * report a start time, so showing a timer would misreport it.
 */
export const WorkingSinceContext = createContext<ReadonlyMap<string, number>>(
  new Map(),
)

export function useWorkingSince(conversationId: string): number | undefined {
  return useContext(WorkingSinceContext).get(conversationId)
}

export function formatElapsed(milliseconds: number): string {
  const total = Math.max(0, Math.floor(milliseconds / MS_PER_SECOND))
  const minutes = Math.floor(total / SECONDS_PER_MINUTE)
  const seconds = String(total % SECONDS_PER_MINUTE).padStart(
    SECOND_DIGITS,
    '0',
  )
  return `${String(minutes)}:${seconds}`
}
