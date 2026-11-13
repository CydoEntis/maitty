import { useCallback, useState, type ReactNode } from 'react'
import { listFolders } from '@/api/folders'
import { PickFolderDialog } from './PickFolderDialog'

/**
 * The new chat page's Browse for another machine: `browse` opens the dialog and answers with the
 * folder chosen, or null when it is cancelled, as the system's folder picker does for this computer.
 */
export function useRemoteFolderPick(machine: number, label: string): { browse: () => Promise<string | null>; dialog: ReactNode } {
  const [answer, setAnswer] = useState<((folder: string | null) => void) | null>(null)
  const browse = useCallback(() => new Promise<string | null>((resolve) => { setAnswer(() => resolve) }), [])
  const list = useCallback((path: string) => listFolders(machine, path), [machine])
  const settle = (folder: string | null) => {
    answer?.(folder)
    setAnswer(null)
  }
  const dialog = answer === null ? null : <PickFolderDialog machine={label} list={list} onPick={settle} onCancel={() => { settle(null) }} />
  return { browse, dialog }
}
