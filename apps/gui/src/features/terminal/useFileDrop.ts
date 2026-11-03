import { isTauri } from '@tauri-apps/api/core'
import { useEffect, useRef } from 'react'

export type DropPoint = { x: number; y: number }

/**
 * Calls `onDrop` with the files dropped on the window and where, in CSS pixels. The desktop shell takes
 * file drops for itself, so the page's own drop events never fire for them; this listens to the shell's.
 * Does nothing in a plain browser.
 */
export function useFileDrop(onDrop: (paths: string[], at: DropPoint) => void) {
  const latest = useRef(onDrop)
  useEffect(() => { latest.current = onDrop })

  useEffect(() => {
    if (!isTauri()) return
    let cancelled = false
    let stop: (() => void) | undefined
    import('@tauri-apps/api/webview')
      .then(({ getCurrentWebview }) =>
        getCurrentWebview().onDragDropEvent((event) => {
          if (event.payload.type !== 'drop') return
          const { paths, position } = event.payload
          latest.current(paths, { x: position.x / window.devicePixelRatio, y: position.y / window.devicePixelRatio })
        }),
      )
      .then((unlisten) => {
        if (cancelled) unlisten()
        else stop = unlisten
      })
      .catch((error: unknown) => { console.warn('Dropping files onto a pane is unavailable', error) })
    return () => {
      cancelled = true
      stop?.()
    }
  }, [])
}
