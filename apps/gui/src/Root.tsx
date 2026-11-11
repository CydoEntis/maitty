import { lazy, Suspense, useEffect } from 'react'
import { isDesktop } from '@/api/daemon'
import { installUndoRedo } from '@/features/editing/undoRedo'
import { DesktopApp } from '@/features/workspace/DesktopApp'

// The browser preview (the older chat screens in App.tsx) is its own chunk: the desktop window
// never downloads or parses it.
const PreviewApp = lazy(() => import('./App.tsx'))

/** The desktop window, or the browser preview outside the desktop app. */
export function Root() {
  // Here and not in either screen: this is the one component both the desktop window and the preview render.
  useEffect(() => installUndoRedo(document, navigator.userAgent), [])
  if (isDesktop()) return <DesktopApp />
  return (
    <Suspense fallback={null}>
      <PreviewApp />
    </Suspense>
  )
}
