import { MOST_SENT_FILES } from '@/api/fileSends'
import { machineNameOf } from '@/api/machines'
import { formatDroppedPaths } from './droppedPaths'

/** What a drop on a pane comes to. */
export type DropRoute =
  | { kind: 'paste'; text: string }
  | { kind: 'send'; machine: string; paths: string[] }
  | { kind: 'refuse'; message: string }

/**
 * Where files dropped on a pane go. On this computer their paths are typed into the pane. On
 * another machine a path here names nothing there, so the files are sent to it and the paths they
 * land at are typed instead (ADR 0050); more than the limit are refused whole, never cut.
 */
export function routeDrop(sessionId: string, paths: readonly string[], windows: boolean): DropRoute | null {
  const named = paths.filter((path) => path !== '')
  if (named.length === 0) return null
  const machine = machineNameOf(sessionId)
  if (machine === null) return { kind: 'paste', text: formatDroppedPaths(named, windows) }
  if (named.length > MOST_SENT_FILES) {
    return { kind: 'refuse', message: `At most ${String(MOST_SENT_FILES)} files can be sent at a time; ${String(named.length)} were dropped.` }
  }
  return { kind: 'send', machine, paths: named }
}


/** The text typed into a pane for a file kept on another machine: quoted for the shell there, with a space after. */
export function remotePathText(path: string): string | null {
  const safe = typablePath(path)
  return safe === null ? null : formatDroppedPaths([safe], false)
}
const MOST_PATH = 4096

/**
 * A path the far side named, if it is fit to type into a pane: absolute, of reasonable length and with
 * nothing in it that is not text. The host checks this too; the other machine is not trusted by either.
 * Refused: control characters (Cc), format characters (Cf: zero-width, direction overrides, the
 * byte-order mark) and line or paragraph separators.
 */
export function typablePath(path: string): string | null {
  if (!path.startsWith('/') || path.length > MOST_PATH || /[\p{Cc}\p{Cf}\p{Zl}\p{Zp}]/u.test(path)) return null
  return path
}
