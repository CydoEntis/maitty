import { useEffect, useMemo, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { TitleBarView } from './TitleBarView'
import { titleBarPlatform, type TitleBarPlatform } from './title-bar-platform'

/**
 * The window's own title bar, drawn in the app's colors. The native bar is turned
 * off in tauri.conf.json, so this one carries the logo, the drag area and the
 * window buttons, drawn the way the platform draws them. It renders nothing
 * outside the desktop app. `platform` overrides detection, for tests and previews.
 */
export function TitleBar({ platform = titleBarPlatform(navigator.userAgent) }: { platform?: TitleBarPlatform }) {
  if (!isTauri() || !hasWindowMetadata()) return null
  return <DesktopTitleBar platform={platform} />
}

// getCurrentWindow() needs the host's window metadata, which a real Tauri
// webview always has.
function hasWindowMetadata(): boolean {
  const internals = (
    window as unknown as { __TAURI_INTERNALS__?: { metadata?: unknown } }
  ).__TAURI_INTERNALS__
  return internals?.metadata !== undefined
}

function DesktopTitleBar({ platform }: { platform: TitleBarPlatform }) {
  // One handle for the window's life: a new one each render re-subscribed to its events every second.
  const appWindow = useMemo(() => getCurrentWindow(), [])
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let active = true
    const sync = () => {
      void appWindow.isMaximized().then((value) => {
        if (active) setMaximized(value)
      })
    }
    sync()
    void appWindow.onResized(sync).then((stop) => {
      if (active) unlisten = stop
      else stop()
    })
    return () => {
      active = false
      unlisten?.()
    }
  }, [appWindow])

  return (
    <TitleBarView
      platform={platform}
      actions={{
        maximized,
        onMinimize: () => {
          void appWindow.minimize()
        },
        onToggleMaximize: () => {
          void appWindow.toggleMaximize()
        },
        onClose: () => {
          void appWindow.close()
        },
      }}
    />
  )
}
