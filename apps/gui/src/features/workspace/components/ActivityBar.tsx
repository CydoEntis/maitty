import { Icon, type IconName } from './glyphs'
import type { InspectView } from '@/features/inspect/InspectPanel'

/** The repository views the bar opens, top to bottom: what was the Inspect panel's tab tray. */
const REPO_VIEWS: readonly { view: InspectView; label: string; icon: IconName; shortcut?: string }[] = [
  { view: 'files', label: 'Files', icon: 'files', shortcut: 'I' },
  { view: 'changes', label: 'Changes', icon: 'changes' },
  { view: 'branches', label: 'Branches', icon: 'repo' },
  { view: 'history', label: 'History', icon: 'history' },
  { view: 'references', label: 'References', icon: 'references' },
]

type Props = {
  /** The view the open Inspect drawer shows, or null while it is closed. */
  openView: InspectView | null
  inboxOpen: boolean
  inboxCount: number
  inboxNeeds: boolean
  /** How many files the repository says changed, shown on Changes. */
  changedFiles: number
  /** Why the repository cannot be read for the focused pane (another machine, a chat session), or undefined. */
  refusal: string | undefined
  onInbox: () => void
  onView: (view: InspectView) => void
}

/**
 * The bar of icons beside the panes (v19 §9): the Inbox, then one icon per repository view. Each opens
 * its drawer over the panes; pressing the open one again closes it. It sits on the side opposite the sessions sidebar.
 */
export function ActivityBar({ openView, inboxOpen, inboxCount, inboxNeeds, changedFiles, refusal, onInbox, onView }: Props) {
  return (
    <div className="mx-activity" role="toolbar" aria-orientation="vertical" aria-label="Panels">
      <button type="button" className="mx-activity-button" aria-pressed={inboxOpen} aria-label={`Inbox, ${String(inboxCount)}${inboxNeeds ? ', something needs you' : ''}`} title="Inbox" onClick={onInbox}>
        <Icon name="inbox" size={18} />
        {inboxCount > 0 && <span className={`mx-activity-badge${inboxNeeds ? ' is-needs' : ''}`}>{inboxCount}</span>}
      </button>
      <span className="mx-activity-rule" aria-hidden="true" />
      {REPO_VIEWS.map((item) => (
        <button
          key={item.view}
          type="button"
          className="mx-activity-button"
          aria-pressed={openView === item.view}
          aria-disabled={refusal !== undefined}
          aria-label={item.label}
          title={refusal ?? (item.shortcut === undefined ? item.label : `${item.label} (Ctrl+${item.shortcut})`)}
          onClick={() => { onView(item.view) }}
        >
          <Icon name={item.icon} size={18} />
          {item.view === 'changes' && changedFiles > 0 && <span className="mx-activity-badge">{changedFiles}</span>}
        </button>
      ))}
    </div>
  )
}
