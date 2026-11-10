import type { RefObject } from 'react'
import type { RemoteMachine } from './model/machines'
import { toast } from '@/lib/toast'
import { routeDrop } from '@/features/terminal/dropRoute'
import { dropOnComposer } from '@/features/chat/composerDrop'
import { useFileDrop } from '@/features/terminal/useFileDrop'
import { useFileGets } from '@/features/terminal/useFileGets'
import { useFileSends } from '@/features/terminal/useFileSends'
import { useImagePaste } from '@/features/terminal/useImagePaste'
import type { TerminalHandle } from '@/features/terminal/handle'

/** Files and images dropped or pasted on a pane: their paths are typed into it, or they are sent to its machine first. */
export function usePaneFiles({ terminalHandles, machinesRef, online, focusedId }: {
  terminalHandles: RefObject<Map<string, TerminalHandle>>
  machinesRef: RefObject<ReadonlyMap<number, RemoteMachine>>
  online: boolean
  focusedId: string | null
}) {
  // A path the machine kept a sent file at is typed into the pane the drop was for, if that pane is still open.
  const typeIntoPane = (sessionId: string, text: string) => {
    const handle = terminalHandles.current.get(sessionId)
    if (!handle) return false
    handle.paste(text)
    return true
  }
  const { sendFiles, sendImage } = useFileSends({ typeInto: typeIntoPane })
  const { getFile } = useFileGets()
  // A pasted image goes the same two ways (ticket 19): saved here and its path typed, or sent to the pane's machine.
  const pasteImage = useImagePaste({
    sendImage,
    typeInto: typeIntoPane,
    isConnected: (name) => [...machinesRef.current.values()].some((candidate) => candidate.state.name.toLowerCase() === name.toLowerCase() && candidate.state.state === 'connected'),
    online,
    windows: /Windows/i.test(navigator.userAgent),
  })
  // Files dropped on a pane type their paths into it; the pane under the pointer, else the focused one.
  // On another machine's pane a path here names nothing there, so the files are sent to it and the
  // paths they land at are typed instead (ADR 0007).
  useFileDrop((paths, at) => {
    const under = document.elementFromPoint(at.x, at.y)
    // A drop on a chat composer attaches the files to its message; it is not a drop on the pane around it (ticket 20).
    if (dropOnComposer(paths, under)) return
    const target = under?.closest<HTMLElement>('[data-session-id]')?.dataset.sessionId ?? focusedId
    const handle = target ? terminalHandles.current.get(target) : undefined
    if (!target || !handle) return
    const route = routeDrop(target, paths, /Windows/i.test(navigator.userAgent))
    if (route === null) return
    if (route.kind === 'refuse') {
      toast.error(route.message)
      return
    }
    if (route.kind === 'send') {
      const machine = [...machinesRef.current.values()].find((candidate) => candidate.state.name.toLowerCase() === route.machine.toLowerCase())
      if (machine === undefined || machine.state.state !== 'connected') {
        toast.error(`${route.machine} is not connected; reconnect before sending files to it`)
        return
      }
      sendFiles(target, route.machine, route.paths)
      handle.focus()
      return
    }
    if (!online) {
      toast.error('Reconnect to the Maitty service before dropping files on a pane')
      return
    }
    handle.paste(route.text)
    handle.focus()
  })
  return { getFile, pasteImage }
}
