import type { ReactNode } from 'react'
import { fileTag } from '@/features/inspect/fileTag'
import { Icon, Spinner } from '@/features/workspace/components/glyphs'
import type { FileChangeKind, ItemStatus } from '../chatEvents'

/** Small pieces the chat's steps and cards share. Colours are theme tokens. */

const STATUS_WORD: Record<ItemStatus, string> = { running: 'Running', completed: 'Done', failed: 'Failed', interrupted: 'Stopped' }

const GLYPH_CHECK_SIZE = 13
const GLYPH_CHECK_STROKE = 2.8

/**
 * The mark every step starts with: the spinner while it runs, a green check when done, a red × tile
 * when it failed, a grey square when it was stopped. One waiting on the person shows the amber tile.
 */
export function StepGlyph({ status, waiting = false }: { status: ItemStatus; waiting?: boolean }) {
  return (
    <span className="mx-step-glyph" aria-hidden="true">
      {waiting ? <span className="mx-step-bang">!</span>
        : status === 'running' ? <Spinner />
        : status === 'completed' ? <Icon name="check" size={GLYPH_CHECK_SIZE} strokeWidth={GLYPH_CHECK_STROKE} />
        : status === 'failed' ? <span className="mx-step-x">×</span>
        : <span className="mx-step-stop" />}
    </span>
  )
}

export type MetaTone = 'dim' | 'danger' | 'working' | 'warn'

type StepRowProps = {
  status: ItemStatus
  verb: ReactNode
  target?: ReactNode
  meta?: ReactNode
  metaTone?: MetaTone
  /** Whether the step's detail shows; `null` for a step with nothing to open. */
  open: boolean | null
  onToggle?: () => void
  /** A request about this step is waiting for the person. */
  waiting?: boolean
}

/** One step as a 30px line: its mark, a verb, what it acted on, and how it ended at the right. */
export function StepRow({ status, verb, target, meta, metaTone = 'dim', open, onToggle, waiting = false }: StepRowProps) {
  const body = (
    <>
      <StepGlyph status={status} waiting={waiting} />
      <span className="mx-step-verb">{verb}</span>
      {target !== undefined && <span className="mx-step-target">{target}</span>}
      <span className="sr-only">{STATUS_WORD[status]}</span>
      <span className="mx-spacer" />
      {waiting ? <span className="mx-step-meta is-warn">waiting for you ↓</span> : meta !== undefined && meta !== null && <span className={`mx-step-meta is-${metaTone}`}>{meta}</span>}
      {open !== null && <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>}
    </>
  )
  const className = `mx-step-row${waiting ? ' is-waiting' : ''}`
  return open === null
    ? <div className={className}>{body}</div>
    : <button type="button" className={className} aria-expanded={open} onClick={onToggle}>{body}</button>
}

const TAG_TONE: Readonly<Record<string, string>> = { TS: 'is-blue', TSX: 'is-blue', JS: 'is-green', JSX: 'is-green', IMG: 'is-green', SVG: 'is-green', '{ }': 'is-mag', TOML: 'is-mag', YML: 'is-mag' }

/** The short type tag before a file's name; nothing for a type that has none. */
export function FileTag({ name }: { name: string }) {
  const tag = fileTag(name)
  return tag === null ? null : <span className={`mx-ftag ${TAG_TONE[tag] ?? ''}`}>{tag}</span>
}

const FOLDER_ICON_SIZE = 14

/** A path split where its last name starts: a folder keeps its closing slash with the name. */
function splitPath(path: string): { dir: string; name: string } {
  const slashed = path.replace(/\\/g, '/')
  const cut = slashed.replace(/\/$/, '').lastIndexOf('/')
  return { dir: slashed.slice(0, cut + 1), name: slashed.slice(cut + 1) }
}

type FileBadgeProps = {
  path: string
  line?: number | null
  /** Opens the file; without it the badge is text, as inside a row that is itself a button. */
  onOpen?: (() => void) | undefined
}

/** A file's name as a badge: its type tag, the folder dimmed, the name bright, the line in the accent. */
export function FileBadge({ path, line = null, onOpen }: FileBadgeProps) {
  const { dir, name } = splitPath(path)
  const body = (
    <>
      {name.endsWith('/') ? <span className="mx-fbadge-folder"><Icon name="files" size={FOLDER_ICON_SIZE} /></span> : <FileTag name={name} />}
      {dir !== '' && <span className="mx-fbadge-dir">{dir}</span>}
      <span className="mx-fbadge-name">{name}</span>
      {line !== null && <span className="mx-fbadge-line">:{line}</span>}
    </>
  )
  return onOpen === undefined
    ? <span className="mx-fbadge" title={path}>{body}</span>
    : <button type="button" className="mx-fbadge is-link" title={path} onClick={onOpen}>{body}</button>
}

/** A small coloured word in a pill: Added, Modified, Deleted, or an exit code. */
export function Pill({ tone, children }: { tone: 'ok' | 'danger' | 'blue'; children: ReactNode }) {
  return <span className={`mx-pill is-${tone}`}>{children}</span>
}

const CHANGE_PILL = { add: { word: 'Added', tone: 'ok' }, update: { word: 'Modified', tone: 'blue' }, delete: { word: 'Deleted', tone: 'danger' } } as const

/** What happened to a file, where the history says: a Write does not say whether the file was new. */
export function ChangePill({ change }: { change: FileChangeKind }) {
  if (change === 'unknown') return null
  return <Pill tone={CHANGE_PILL[change].tone}>{CHANGE_PILL[change].word}</Pill>
}
