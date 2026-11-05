import type { TouchedFile } from './panelModel'
import { splitPath } from './patchLines'

type Props = {
  files: readonly TouchedFile[]
  /** A file's path as the panel shows it (relative to the project). */
  displayPath: (file: string) => string
  /** Whether the worktree still has a diff for the file. */
  hasDiff: (file: string) => boolean
  onOpenDiff: (file: string) => void
  onOpenInFiles: (file: string) => void
}

const KIND_WORD = { edited: 'Edited', created: 'Created', read: 'Read' } as const
const KIND_TONE = {
  edited: 'text-[var(--warn)]',
  created: 'text-[var(--ok)]',
  read: 'text-dim',
} as const

/**
 * The files this session touched, from what the agent's tool calls named (v18 C.2). An edited or
 * created file opens its diff; one that has no diff now (it was committed, or reverted) and a file
 * that was only read open in the Files drawer.
 */
export function FilesSegment({ files, displayPath, hasDiff, onOpenDiff, onOpenInFiles }: Props) {
  if (files.length === 0) {
    return <p className="m-0 px-1 py-4 text-center text-[12.5px] text-dim">The agent has not read or changed a file in this session yet.</p>
  }
  return (
    <ul className="m-0 flex list-none flex-col gap-0.5 p-0" aria-label="Files this session touched">
      {files.map((file) => {
        const path = displayPath(file.file)
        const { dir, name } = splitPath(path)
        const diff = file.kind !== 'read' && hasDiff(path)
        return (
          <li key={file.file}>
            <button type="button" className="flex min-h-7 w-full items-center gap-2 rounded-lg px-2 text-left text-[12.5px] hover:bg-hover" aria-label={`${KIND_WORD[file.kind]} ${path}. ${diff ? 'Open its diff' : 'Open it in the Files drawer'}`} onClick={() => { if (diff) onOpenDiff(file.file); else onOpenInFiles(file.file) }}>
              <span className={`w-[52px] shrink-0 text-[11px] ${KIND_TONE[file.kind]}`}>{KIND_WORD[file.kind]}</span>
              <span className="min-w-0 flex-1 truncate" title={path}><span className="text-dim">{dir}</span><span className="text-fg">{name}</span></span>
              {file.added > 0 && <span className="shrink-0 font-mono text-[11.5px] text-[var(--ok)]">+{file.added}</span>}
              {file.removed > 0 && <span className="shrink-0 font-mono text-[11.5px] text-[var(--danger)]">−{file.removed}</span>}
            </button>
          </li>
        )
      })}
    </ul>
  )
}
