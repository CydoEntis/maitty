import { useCallback, useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react'
import { BranchesTab } from './BranchesTab'
import { ChangesTab } from './ChangesTab'
import { ChecksTab } from './ChecksTab'
import { checksNeedingYou, type Check } from './checks'
import type { ChangeActions } from './CommitBar'
import type { PatchComments } from './PatchView'
import { FilesTab, type LookAt } from './FilesTab'
import { HistoryTab } from './HistoryTab'
import { ReferencesTab } from './ReferencesTab'
import { TimelineTab } from './TimelineTab'
import { LOCAL_MACHINE } from '@/api/machines'
import type { TerminalDetails } from '@/data/preview'
import { localOnlyMessage, type LocalFeature } from '@/lib/localOnly'
import { Icon, type IconName } from '@/features/workspace/components/glyphs'
import './inspect.css'

export type InspectView = 'changes' | 'files' | 'terminal' | 'checks' | 'branches' | 'history' | 'timeline' | 'references'

const VIEWS: readonly { id: InspectView; label: string; icon?: IconName }[] = [
  { id: 'changes', label: 'Changes', icon: 'changes' },
  { id: 'files', label: 'Files', icon: 'files' },
  { id: 'terminal', label: 'Terminal', icon: 'terminal' },
  { id: 'checks', label: 'Checks' },
  { id: 'branches', label: 'Branches' },
  { id: 'history', label: 'History' },
  { id: 'timeline', label: 'Timeline' },
  { id: 'references', label: 'References' },
]

/** The panel opens at this share of the window; the edge can be dragged and the width is remembered. */
const DEFAULT_SHARE = 0.5
const MIN_WIDTH_PX = 420
/** The strip of the chat that stays in sight beside the panel, to click it away. */
const STRIP_PX = 56
/** The gap between the window's edge and the panel. */
const EDGE_PX = 24
const KEY_STEP_PX = 24
// A new key when the opening width changes, so a width kept from the wider panel does not undo it.
const WIDTH_KEY = 'maitty.inspect.width.v3'
/** The exit animation's length: the panel is unmounted when it ends, not at once. */
const EXIT_MS = 190

/** The widest the panel may be: it never covers the sidebar, and leaves a strip of the chat. */
function roomPx(): number {
  const sidebar = document.querySelector('.mx-sidebar-expanded')?.getBoundingClientRect().width ?? 0
  return Math.max(MIN_WIDTH_PX, window.innerWidth - sidebar - STRIP_PX - EDGE_PX)
}

function clampWidth(width: number): number {
  return Math.round(Math.min(Math.max(width, MIN_WIDTH_PX), roomPx()))
}

/** The width someone dragged it to, or null while it follows the window. */
function readWidth(): number | null {
  try {
    const stored = Number(window.localStorage.getItem(WIDTH_KEY))
    return Number.isFinite(stored) && stored >= MIN_WIDTH_PX ? stored : null
  } catch {
    return null
  }
}

function writeWidth(width: number | null): void {
  try {
    if (width === null) window.localStorage.removeItem(WIDTH_KEY)
    else window.localStorage.setItem(WIDTH_KEY, String(width))
  } catch {
    // The width is only remembered, never needed.
  }
}

type Props = {
  /** The daemon's pane number every repository question is about; null when there is none to ask, and Changes and Files are left out. */
  term: number | null
  /** The session's own terminals, for the Terminal tab; null when it has none here. */
  terminals: ReactNode | null
  /** How many files the repository says changed, shown beside Changes. */
  changedFiles: number
  /** The branch the session's checkout is on, shown above the changed files. */
  branch?: string
  view: InspectView
  agentWorking: boolean
  /** The session's checks, for the Checks view. */
  checks?: readonly Check[]
  /** The machine the session is on (0: this computer): its daemon answers Changes and Checks, and the views that read files say they cannot. */
  machine?: number
  /** That machine's name, for those words; null on this computer. */
  machineName?: string | null
  /** What the pane's agent and shell reported lately, for the Timeline view. */
  timeline: TerminalDetails['timeline']
  lookAt: LookAt | null
  /** The agent a comment on a diff line goes to; absent for a shell, which has nobody to tell. */
  comments?: Omit<PatchComments, 'path' | 'sent' | 'onSent'>
  /** The Changes tab's commit bar; on another machine it holds the remote button alone. */
  changeActions?: ChangeActions
  onView: (view: InspectView) => void
  onClose: () => void
  onNotice: (text: string) => void
}

/** The views that read the checkout's files, which are not on this computer for a session of another machine. */
const NEEDS_FILES = { files: 'files', branches: 'branches', history: 'history', references: 'references' } as const satisfies Partial<Record<InspectView, LocalFeature>>

function needsFiles(view: InspectView): view is keyof typeof NEEDS_FILES {
  return view in NEEDS_FILES
}

/** The asked view when the panel can show it, or else the first one it can. */
function shownView(view: InspectView, hasRepo: boolean, hasTerminals: boolean): InspectView {
  const can = (id: InspectView) => (id === 'terminal' ? hasTerminals : hasRepo)
  return can(view) ? view : (VIEWS.map((item) => item.id).find(can) ?? view)
}

/**
 * The side panel (layout 2e). It slides over the panes from the window's edge, over a scrim, and
 * never pushes them or covers the sidebar: the focused session's changes, its files and its own
 * terminals.
 */
export function InspectPanel({ term, terminals, changedFiles, branch, view: asked, agentWorking, timeline, checks = [], machine = 0, machineName = null, lookAt, comments, changeActions, onView, onClose, onNotice }: Props) {
  const elsewhere = machine !== LOCAL_MACHINE
  const view = shownView(asked, term !== null, terminals !== null)
  const [dragged, setDragged] = useState(readWidth)
  const width = clampWidth(dragged ?? window.innerWidth * DEFAULT_SHARE)
  const [leaving, setLeaving] = useState(false)
  const previous = useRef(view)
  const [towards, setTowards] = useState<'forward' | 'back'>('forward')
  useEffect(() => {
    const from = VIEWS.findIndex((item) => item.id === previous.current)
    const to = VIEWS.findIndex((item) => item.id === view)
    setTowards(to >= from ? 'forward' : 'back')
    previous.current = view
  }, [view])
  const close = useCallback(() => {
    setLeaving(true)
    window.setTimeout(onClose, EXIT_MS)
  }, [onClose])
  const save = useCallback((next: number | null) => {
    setDragged(next)
    writeWidth(next)
  }, [])
  const startDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault()
    const move = (at: PointerEvent) => { setDragged(clampWidth(widthAtPointer(at.clientX))) }
    const stop = (at: PointerEvent) => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', stop)
      save(clampWidth(widthAtPointer(at.clientX)))
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', stop)
  }
  const nudge = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'ArrowLeft') save(clampWidth(width + KEY_STEP_PX))
    else if (event.key === 'ArrowRight') save(clampWidth(width - KEY_STEP_PX))
  }
  return (
    <>
      <div className={`mx-inspect-scrim${leaving ? ' is-leaving' : ''}`} aria-hidden="true" onMouseDown={close} />
      <aside
        className={`mx-inspect${leaving ? ' is-leaving' : ''}`}
        style={{ width }}
        aria-label="Side panel"
        onKeyDown={(event) => {
          // Esc closes the panel, except in a terminal, where it belongs to the program running there.
          if (event.key !== 'Escape' || (event.target instanceof Element && event.target.closest('.mx-terms-screen'))) return
          event.stopPropagation()
          close()
        }}
      >
        <div className="mx-inspect-grip" role="separator" aria-orientation="vertical" aria-label="Resize the side panel" tabIndex={0} onPointerDown={startDrag} onKeyDown={nudge} onDoubleClick={() => { save(null) }} />
        <header className="mx-inspect-head">
          <div role="tablist" aria-label="Side panel views" className={`mx-inspect-tabs is-${towards}`}>
            {VIEWS.filter((item) => shownView(item.id, term !== null, terminals !== null) === item.id).map((item) => (
              <button key={item.id} type="button" role="tab" aria-selected={view === item.id} className={view === item.id ? 'is-current' : undefined} onClick={() => { onView(item.id) }}>
                {item.icon !== undefined && <Icon name={item.icon} size={13} />}
                {item.label}
                {item.id === 'changes' && changedFiles > 0 && <span className="mx-inspect-count">{changedFiles}</span>}
                {item.id === 'checks' && checksNeedingYou(checks) > 0 && <span className="mx-inspect-count">{checksNeedingYou(checks)}</span>}
              </button>
            ))}
          </div>
          <button type="button" className="mx-icon-button" aria-label="Close" title="Close (esc)" onClick={close}>×</button>
        </header>
        {view === 'terminal' && <div className="mx-inspect-terminals" role="tabpanel">{terminals}</div>}
        {view !== 'terminal' && term !== null && (
          <div className="mx-inspect-scroll" role="tabpanel">
            {elsewhere && needsFiles(view) && <p className="mx-inspect-note">{localOnlyMessage(NEEDS_FILES[view], machineName)}</p>}
            {view === 'files' && !elsewhere && <FilesTab term={term} machine={machine} lookAt={lookAt} />}
            {view === 'changes' && <ChangesTab term={term} machine={machine} {...(branch !== undefined ? { branch } : {})} {...(comments ? { comments } : {})} {...(changeActions ? { actions: changeActions } : {})} onNotice={onNotice} />}
            {view === 'checks' && <ChecksTab term={term} machine={machine} checks={checks} onNotice={onNotice} />}
            {view === 'branches' && !elsewhere && <BranchesTab term={term} />}
            {view === 'history' && !elsewhere && <HistoryTab term={term} agentWorking={agentWorking} onNotice={onNotice} />}
            {view === 'timeline' && <TimelineTab events={timeline} />}
            {view === 'references' && !elsewhere && <ReferencesTab term={term} />}
          </div>
        )}
      </aside>
    </>
  )
}

/** The panel's width if its inner edge were at this x: it is anchored a gap in from the window's edge. */
function widthAtPointer(x: number): number {
  const onLeft = document.querySelector('.mx-app')?.getAttribute('data-side') === 'right'
  return onLeft ? x - EDGE_PX : window.innerWidth - x - EDGE_PX
}
