import { useCallback, useMemo } from 'react'
import { chooseProjectFolder } from '@/api/folderPicker'
import { listFolders } from '@/api/folders'
import { LOCAL_MACHINE } from '@/api/machines'
import type { ChatAgent, PaneView } from '../model/chatView'
import type { MachineHead } from '../model/machines'
import type { SessionView } from '../model/session'
import { modalMachines, recentFolders } from '../model/newSession'
import { NewSessionModal, type NewSessionRequest } from './NewSessionModal'

/** The New session modal wired to the window's data. The starting itself is `onStart`'s. */
export function NewSessionHost({ machine, heads, sessions, focused, startFolder, defaultAgent, defaultView, onStart, onCancel }: {
  /** The machine the modal opens on. */
  machine: number
  heads: readonly MachineHead[]
  sessions: readonly SessionView[]
  focused: SessionView | undefined
  /** Where a session starts when no session is focused: the Settings start folder, resolved. */
  startFolder: string
  defaultAgent: ChatAgent
  defaultView: PaneView
  onStart: (request: NewSessionRequest) => Promise<void>
  onCancel: () => void
}) {
  const machines = useMemo(() => modalMachines(heads), [heads])
  const projectsOf = useCallback((id: number) => recentFolders(sessions, id), [sessions])
  const initialFolder = focused?.machine === machine && focused.cwd !== '' ? focused.cwd : machine === LOCAL_MACHINE && startFolder !== '' ? startFolder : null
  return (
    <NewSessionModal
      machines={machines}
      initialMachine={machine}
      initialFolder={initialFolder}
      projectsOf={projectsOf}
      list={listFolders}
      browse={chooseProjectFolder}
      defaultAgent={defaultAgent}
      defaultView={defaultView}
      onStart={onStart}
      onCancel={onCancel}
    />
  )
}
