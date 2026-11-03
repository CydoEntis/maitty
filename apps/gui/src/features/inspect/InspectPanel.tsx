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

type Props = {
  /** The daemon's pane number: every question is about that pane's own repository. */
  term: number
  title: string
  view: InspectView
  agentWorking: boolean
  /** What the pane's agent and shell reported lately, for the Timeline view. */
  timeline: TerminalDetails['timeline']
  lookAt: LookAt | null
  onView: (view: InspectView) => void
  onClose: () => void
  onNotice: (text: string) => void
}

/** ⌘I: a read-only look at the focused pane's repository. Anything that changes it happens in the pane's terminal. */
export function InspectPanel({ term, title, view, agentWorking, timeline, lookAt, onView, onClose, onNotice }: Props) {
  return (
    <aside className="mx-inspect" aria-label="Inspect" onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); onClose() } }}>
      <header className="mx-inspect-head">
        <strong>Inspect</strong>
        <span className="mx-inspect-sub">{title}</span>
        <span className="mx-spacer" />
        <button type="button" className="mx-icon-button" aria-label="Close" title="Close (esc)" onClick={onClose}>×</button>
      </header>
      <div role="tablist" aria-label="Inspect views" className="mx-inspect-tabs">
        {VIEWS.map((item) => (
          <button key={item.id} type="button" role="tab" aria-selected={view === item.id} className={view === item.id ? 'is-current' : undefined} onClick={() => { onView(item.id) }}>{item.label}</button>
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
  )
}
