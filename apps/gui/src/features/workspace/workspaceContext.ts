import { createContext, useContext } from 'react'
import type { useWorkspaceState } from './useWorkspaceState'

/** Everything the window's parts share: the sessions, the layout, the preferences, what is open, and what each action does. */
type Workspace = ReturnType<typeof useWorkspaceState>

export const WorkspaceContext = createContext<Workspace | null>(null)

/** The window's shared state, for a part of the window rendered inside `DesktopApp`. */
export function useWorkspace(): Workspace {
  const workspace = useContext(WorkspaceContext)
  if (workspace === null) throw new Error('useWorkspace is only for the parts of the window inside DesktopApp')
  return workspace
}
