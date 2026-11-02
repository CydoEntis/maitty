import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Copy, Minus, Square, X } from 'lucide-react'

/**
 * The window's own title bar, drawn in the app's colors. The native bar is turned
 * off in tauri.conf.json, so this one carries the logo, the drag area and the
 * minimize, maximize and close buttons. It renders nothing outside the desktop app.
 */
export function TitleBar() {
  if (!isTauri() || !hasWindowMetadata()) return null
  return <DesktopTitleBar />
}

// getCurrentWindow() needs the host's window metadata, which a real Tauri
// webview always has.
function hasWindowMetadata(): boolean {
  const internals = (
    window as unknown as { __TAURI_INTERNALS__?: { metadata?: unknown } }
  ).__TAURI_INTERNALS__
  return internals?.metadata !== undefined
}

function DesktopTitleBar() {
  const appWindow = getCurrentWindow()
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
    <header className="title-bar" data-tauri-drag-region>
      <img className="title-bar-logo" src="/maitty-logo.png" alt="" />
      <span className="title-bar-name">Maitty</span>
      <div className="title-bar-controls">
        <button
          type="button"
          aria-label="Minimize"
          onClick={() => {
            void appWindow.minimize()
          }}
        >
          <Minus size={14} />
        </button>
        <button
          type="button"
          aria-label={maximized ? 'Restore' : 'Maximize'}
          onClick={() => {
            void appWindow.toggleMaximize()
          }}
        >
          {maximized ? <Copy size={12} /> : <Square size={12} />}
        </button>
        <button
          type="button"
          className="is-close"
          aria-label="Close window"
          onClick={() => {
            void appWindow.close()
          }}
        >
          <X size={15} />
        </button>
      </div>
    </header>
  )
}
