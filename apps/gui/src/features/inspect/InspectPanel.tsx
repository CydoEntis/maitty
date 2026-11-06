import { useCallback, useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react'
import { BranchesTab } from './BranchesTab'
import { ChangesTab } from './ChangesTab'
import { FilesTab, type LookAt } from './FilesTab'
import { HistoryTab } from './HistoryTab'
import { ReferencesTab } from './ReferencesTab'
import { TimelineTab } from './TimelineTab'
import type { TerminalDetails } from '@/data/preview'
import './inspect.css'

export type InspectView = 'files' | 'changes' | 'branches' | 'history' | 'timeline' | 'references'

const VIEWS: readonly { id: InspectView; label: string }[] = [
  { id: 'files', label: 'Files' },
  { id: 'changes', label: 'Changes' },
  { id: 'branches', label: 'Branches' },
  { id: 'history', label: 'History' },
  { id: 'timeline', label: 'Timeline' },
  { id: 'references', label: 'References' },
]

/** The drawer opens at this width on every tab, so tabs never jump; the edge can be dragged and the width is remembered. */
const DEFAULT_WIDTH_PX = 680
const MIN_WIDTH_PX = 420
/** Room left for the sessions and panes beside the drawer. */
const MIN_BESIDE_PX = 300
const SNAP_PX = 10
const KEY_STEP_PX = 24
const WIDTH_KEY = 'maitty.inspect.width'
/** The exit animation's length: the drawer is unmounted when it ends, not at once. */
const EXIT_MS = 210

function readWidth(): number {
  try {
    const stored = Number(window.localStorage.getItem(WIDTH_KEY))
    return Number.isFinite(stored) && stored >= MIN_WIDTH_PX ? stored : DEFAULT_WIDTH_PX
  } catch {
    return DEFAULT_WIDTH_PX
  }
}

function clampWidth(width: number): number {
  const widest = Math.max(MIN_WIDTH_PX, window.innerWidth - MIN_BESIDE_PX)
  const clamped = Math.min(Math.max(width, MIN_WIDTH_PX), widest)
  return Math.abs(clamped - DEFAULT_WIDTH_PX) <= SNAP_PX ? DEFAULT_WIDTH_PX : clamped
}

type Props = {
  /** The daemon's pane number: every question is about that pane's own repository. */
  term: number
  /** The project's name, then its branch when it has one, in the header. */
  project: string
  branch: string | null
  /** How many files the repository says changed, shown beside Changes. */
  changedFiles: number
  view: InspectView
  agentWorking: boolean
  /** What the pane's agent and shell reported lately, for the Timeline view. */
  timeline: TerminalDetails['timeline']
  lookAt: LookAt | null
  onView: (view: InspectView) => void
  onClose: () => void
  onNotice: (text: string) => void
}

/**
 * The repository drawer (v19 §9). It slides over the panes from beside the activity bar, over a scrim,
 * and never pushes them. A read-only look at the focused pane's repository: anything that changes it
 * happens in the pane's terminal.
 */
export function InspectPanel({ term, project, branch, changedFiles, view, agentWorking, timeline, lookAt, onView, onClose, onNotice }: Props) {
  const [width, setWidth] = useState(readWidth)
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
  const save = useCallback((next: number) => {
    setWidth(next)
    try { window.localStorage.setItem(WIDTH_KEY, String(next)) } catch { /* the width is only remembered, never needed */ }
  }, [])
  const startDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault()
    const move = (at: PointerEvent) => { setWidth(clampWidth(widthAtPointer(at.clientX))) }
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
        aria-label="Repository"
        onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); close() } }}
      >
        <div className="mx-inspect-grip" role="separator" aria-orientation="vertical" aria-label="Resize the repository drawer" tabIndex={0} onPointerDown={startDrag} onKeyDown={nudge} onDoubleClick={() => { save(DEFAULT_WIDTH_PX) }} />
        <header className="mx-inspect-head">
          <strong>{project}</strong>
          {branch !== null && <span className="mx-inspect-branch">{branch}</span>}
          <span className="mx-spacer" />
          <button type="button" className="mx-icon-button" aria-label="Close" title="Close (esc)" onClick={close}>×</button>
        </header>
        <div role="tablist" aria-label="Repository views" className={`mx-inspect-tabs is-${towards}`}>
          {VIEWS.map((item) => (
            <button key={item.id} type="button" role="tab" aria-selected={view === item.id} className={view === item.id ? 'is-current' : undefined} onClick={() => { onView(item.id) }}>
              {item.label}
              {item.id === 'changes' && changedFiles > 0 && <span className="mx-inspect-count">{changedFiles}</span>}
            </button>
          ))}
        </div>
        <div className="mx-inspect-scroll" role="tabpanel">
          {view === 'files' && <FilesTab term={term} lookAt={lookAt} />}
          {view === 'changes' && <ChangesTab term={term} />}
          {view === 'branches' && <BranchesTab term={term} />}
          {view === 'history' && <HistoryTab term={term} agentWorking={agentWorking} onNotice={onNotice} />}
          {view === 'timeline' && <TimelineTab events={timeline} />}
          {view === 'references' && <ReferencesTab term={term} />}
        </div>
      </aside>
    </>
  )
}

/** The drawer's width if its inner edge were at this x: it is anchored beside the activity bar, so the gap is the bar and the margins around it. */
function widthAtPointer(x: number): number {
  const bar = document.querySelector('.mx-activity')
  const gap = (bar?.getBoundingClientRect().width ?? 0) + DRAWER_MARGIN_PX
  const onRight = document.querySelector('.mx-app')?.getAttribute('data-side') === 'right'
  return onRight ? x - gap : window.innerWidth - x - gap
}
/** The margins between the window's edge, the bar and the drawer (v19 §9). */
const DRAWER_MARGIN_PX = 22
