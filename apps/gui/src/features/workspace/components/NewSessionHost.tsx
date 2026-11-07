import { useMemo } from 'react'
import { chooseProjectFolder } from '@/api/folderPicker'
import { LOCAL_MACHINE } from '@/api/machines'
import type { ChatAgent } from '../model/chatView'
import type { MachineHead } from '../model/machines'
import type { SessionView } from '../model/session'
import { machineProjects } from '../model/newSession'
import { NewSessionModal, type NewSessionRequest } from './NewSessionModal'

/** The New session dialog wired to the window's data. The starting itself is `onStart`'s. */
export function NewSessionHost({ machine, heads, sessions, focused, startFolder, defaultAgent, portalContainer, onStart, onCancel }: {
  /** The machine the dialog opens on. */
  machine: number
  heads: readonly MachineHead[]
  sessions: readonly SessionView[]
  focused: SessionView | undefined
  /** Where a session starts when the machine has no project yet: the Settings start folder, resolved, or `~` on another machine. */
  startFolder: string
  defaultAgent: ChatAgent
  portalContainer: HTMLElement | null
  onStart: (request: NewSessionRequest) => Promise<void>
  onCancel: () => void
}) {
  const local = machine === LOCAL_MACHINE
  const projects = useMemo(() => machineProjects(sessions, machine, focused, local ? startFolder : '~'), [sessions, machine, focused, local, startFolder])
  return (
    <NewSessionModal
      machine={machine}
      machineName={heads.find((head) => head.machine === machine)?.name ?? null}
      projects={projects}
      defaultAgent={defaultAgent}
      browse={local ? () => chooseProjectFolder() : null}
      portalContainer={portalContainer}
      onStart={onStart}
      onCancel={onCancel}
    />
  )
}
