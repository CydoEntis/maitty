import { TitleBar } from '@/components/TitleBar'
import { Toaster } from '@/components/ui/sonner'
import { ReducedMotionContext } from './components/motion'
import { useWorkspaceState } from './useWorkspaceState'
import { WorkspaceContext } from './workspaceContext'
import { WorkspaceSidebars } from './components/WorkspaceSidebars'
import { WorkspaceMain } from './components/WorkspaceMain'
import { WorkspaceDialogs } from './components/WorkspaceDialogs'
import './workspace.css'
import './components/sidebar.css'
import './components/pane.css'
import './components/menu.css'
import './components/dialogs.css'
import '@/features/chat/chat.css'
import '@/features/work/work.css'

export function DesktopApp() {
  const workspace = useWorkspaceState()
  const { reducedMotion, setRoot, narrow, rootStyle, theme, preferences, machineSaid } = workspace
  return (
    <WorkspaceContext.Provider value={workspace}>
    <ReducedMotionContext.Provider value={reducedMotion}>
        <div ref={setRoot} className={`mx-app${narrow ? ' is-narrow' : ''}`} style={rootStyle} data-motion={reducedMotion ? 'reduced' : 'full'} data-theme-kind={theme.light ? 'light' : 'dark'} data-side={preferences.sidebarSide}>
          <TitleBar />
          <div className="mx-window" role="application" aria-label="Maitty">
            <WorkspaceSidebars />
            <WorkspaceMain />
            <WorkspaceDialogs />
          </div>
          <Toaster position={preferences.toastPosition} theme={theme.light ? 'light' : 'dark'} />
          <div className="sr-only" role="status" aria-live="polite" aria-atomic="true">{machineSaid}</div>
        </div>
    </ReducedMotionContext.Provider>
    </WorkspaceContext.Provider>
  )
}
