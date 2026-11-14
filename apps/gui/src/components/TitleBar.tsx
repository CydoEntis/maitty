import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { TitleBarView } from './TitleBarView'
import { titleBarPlatform, type TitleBarPlatform } from './title-bar-platform'

type Props = {
  platform?: TitleBarPlatform
  /** The app's menu at the bar's left. */
  start?: ReactNode
  /** The view toggles before the window buttons. */
  end?: ReactNode
}

/**
 * The window's own title bar, drawn in the app's colors. The native bar is turned off in tauri.conf.json, so this one
 * carries the app's menu, the view toggles, the drag area and the window buttons, drawn the way the platform draws
 * them. Outside the desktop app it draws the menu and toggles alone, and nothing without them. `platform` overrides
 * detection, for tests and previews.
 */
export function TitleBar({ platform = titleBarPlatform(navigator.userAgent), start, end }: Props) {
  if (isTauri() && hasWindowMetadata()) return <DesktopTitleBar platform={platform} start={start} end={end} />
  if (start === undefined && end === undefined) return null
  return <TitleBarView platform={platform} actions={null} start={start} end={end} />
}

// getCurrentWindow() needs the host's window metadata, which a real Tauri
// webview always has.
function hasWindowMetadata(): boolean {
  const internals = (
    window as unknown as { __TAURI_INTERNALS__?: { metadata?: unknown } }
  ).__TAURI_INTERNALS__
  return internals?.metadata !== undefined
}

function DesktopTitleBar({ platform, start, end }: { platform: TitleBarPlatform; start: ReactNode; end: ReactNode }) {
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
      start={start}
      end={end}
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
