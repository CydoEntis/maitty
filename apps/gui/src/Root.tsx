import { lazy, Suspense } from 'react'
import { isDesktop } from '@/api/daemon'
import { DesktopApp } from '@/features/workspace/DesktopApp'

// The browser preview (the older chat screens in App.tsx) is its own chunk: the desktop window
// never downloads or parses it.
const PreviewApp = lazy(() => import('./App.tsx'))

/** The desktop window, or the browser preview outside the desktop app. */
export function Root() {
  if (isDesktop()) return <DesktopApp />
  return (
    <Suspense fallback={null}>
      <PreviewApp />
    </Suspense>
  )
}
