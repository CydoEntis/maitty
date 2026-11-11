import { createContext } from 'react'

/** Opens a file an agent's reply names, at a line when the reply gave one; `null` where the chat has no pane to open it in. */
export type OpenFile = (path: string, line: number | null) => void

export const OpenFileContext = createContext<OpenFile | null>(null)
