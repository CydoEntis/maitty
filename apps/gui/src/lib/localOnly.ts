import { machineNameOf } from '@/api/machines'

/**
 * Features that read or change a pane's files, its repository or this computer's folders. Files
 * are where the pane runs, so for a pane of another machine they are not here to read: each says
 * so, in the terminal app's words, naming the machine. The host refuses the same questions too.
 */
export const LOCAL_FEATURES = {
  files: 'Files',
  changes: 'Changes',
  find: 'Find',
  branches: 'Branches',
  references: 'References',
  'paste-image': 'Pasting an image',
  'agent-status': 'Connecting an agent',
  worktrees: 'Worktrees',
  inspect: 'Inspect',
  'review-stat': 'The change count',
  'path-links': 'Opening a file from the output',
  'screen-text': 'The screen text',
  replay: 'A fresh replay',
} as const

export type LocalFeature = keyof typeof LOCAL_FEATURES

/** Where a pane runs: this computer is machine 0 and has no name. */
export type PaneWhere = { machine: number; machineName: string | null }

export function localOnlyMessage(feature: LocalFeature, machineName: string | null): string {
  return `${LOCAL_FEATURES[feature]} is local-only in v1: the files are on ${machineName ?? 'another machine'}, not on this computer.`
}

/** The words to show when `feature` cannot be used on the pane, or null when it can. Nothing is a pane of this computer. */
export function guardLocal(pane: PaneWhere | null | undefined, feature: LocalFeature): string | null {
  if (pane === null || pane === undefined || pane.machine === 0) return null
  return localOnlyMessage(feature, pane.machineName)
}

/** The same, for a session id: `build/t12` is on build, `t12` is here. */
export function guardLocalSession(sessionId: string, feature: LocalFeature): string | null {
  const name = machineNameOf(sessionId)
  return name === null ? null : localOnlyMessage(feature, name)
}

/** Throws the words when `feature` cannot be used on the pane. */
export function assertLocal(pane: PaneWhere | null | undefined, feature: LocalFeature): void {
  const refusal = guardLocal(pane, feature)
  if (refusal !== null) throw new Error(refusal)
}
