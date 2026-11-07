import { useCallback, useEffect, useRef, useState } from 'react'
import { addCrew, addMate, addToCrew, initialCrew, receive, select, send, type CrewAgent, type CrewState } from './model/crew'

/** How long a crewmate "types" before its answer arrives. */
const REPLY_DELAY_MS = 1300

/** The crew state with the few things the window does to it. The state is sample data kept in memory (see model/crew.ts). */
export function useCrew() {
  const [state, setState] = useState<CrewState>(initialCrew)
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => () => { if (timer.current !== null) clearTimeout(timer.current) }, [])

  const setMode = useCallback((mode: CrewState['mode']) => { setState((current) => ({ ...current, mode })) }, [])
  const open = useCallback((id: string) => { setState((current) => select(current, id)) }, [])

  const say = useCallback((chat: string, text: string) => {
    setState((current) => send(current, chat, text, new Date()))
    if (timer.current !== null) clearTimeout(timer.current)
    timer.current = setTimeout(() => { setState((current) => receive(current, new Date())) }, REPLY_DELAY_MS)
  }, [])

  const createMate = useCallback((input: { name: string; role: string; agent: CrewAgent }) => { setState((current) => addMate(current, input)) }, [])
  const createCrew = useCallback((name: string) => { setState((current) => addCrew(current, name)) }, [])
  const bringAboard = useCallback((crewId: string, mateId: string) => { setState((current) => addToCrew(current, crewId, mateId)) }, [])

  return { state, setMode, open, say, createMate, createCrew, bringAboard }
}

export type CrewApi = ReturnType<typeof useCrew>
