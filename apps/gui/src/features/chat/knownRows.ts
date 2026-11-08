import { createContext } from 'react'

/** The rows the chat already showed when its history had loaded; a reply that is not among them arrived live and is typed in from its first letter. */
export const KnownRowsContext = createContext<ReadonlySet<string> | null>(null)
