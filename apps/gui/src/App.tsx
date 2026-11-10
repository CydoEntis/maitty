import { useEffect } from 'react'
import { DesktopApp } from '@/features/workspace/DesktopApp'
import { installUndoRedo } from '@/features/editing/undoRedo'
import { isDesktop } from '@/api/daemon'
import { PreviewApp } from './features/preview/PreviewApp'
import './App.css'
import './material.css'

/** The desktop app is the terminal multiplexer; the browser preview keeps the older chat screens. */
function App() {
  // Here and not in either screen, so the desktop window and the preview both have it.
  useEffect(() => installUndoRedo(document, navigator.userAgent), [])
  return isDesktop() ? <DesktopApp /> : <PreviewApp />
}

export default App
