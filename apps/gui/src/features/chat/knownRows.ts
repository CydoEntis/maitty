import { createContext } from 'react'

/**
 * The rows the chat has already shown: those its history held when it loaded, and each reply since, once it
 * is on screen. A reply that is not among them arrived live and is typed in from its first letter; one that is
 * shows whole, so a row the list drew again (scrolled back to, or after the window came back) is never retyped.
 */
export const KnownRowsContext = createContext<Set<string> | null>(null)
