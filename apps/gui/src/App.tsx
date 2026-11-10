import { DesktopApp } from '@/features/workspace/DesktopApp'
import { isDesktop } from '@/api/daemon'
import { PreviewApp } from './features/preview/PreviewApp'
import './App.css'
import './material.css'

/** The desktop app is the terminal multiplexer; the browser preview keeps the older chat screens. */
function App() {
  return isDesktop() ? <DesktopApp /> : <PreviewApp />
}

export default App
