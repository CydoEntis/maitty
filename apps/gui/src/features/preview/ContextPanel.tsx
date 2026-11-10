import type { KeyboardEvent, ReactNode } from 'react'
import { Activity, GitBranch, ShieldCheck } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { StatusMark } from '@/components/StatusMark'
import { useReviewStat } from '@/features/review/useReviewStat'
import { FilesView } from '@/features/review/FilesView'
import { ScrollArea } from '@/components/ui/scroll-area'
import { TerminalView as DaemonTerminalView } from '../../TerminalView'
import type { Conversation } from '@/data/preview'
import { type PanelTab, statusText } from './shared'
import { DiffView } from './DiffView'
import { AgentAvatar } from './AgentAvatar'

const PANEL_TABS: { id: PanelTab; label: string }[] = [
  { id: 'diff', label: 'Changes' },
  { id: 'files', label: 'Files' },
  { id: 'details', label: 'Context' },
]

export function ContextPanel({
  tab,
  conversation,
  desktop,
  onTab,
  onClose,
}: {
  tab: PanelTab
  conversation: Conversation
  desktop: boolean
  onTab: (tab: PanelTab) => void
  onClose: () => void
}) {
  const reviewStat = useReviewStat(conversation)

  function handleTabKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
    const tabs = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]'),
    )
    const currentIndex = tabs.findIndex(
      (item) => item === document.activeElement,
    )
    const selectedIndex = PANEL_TABS.findIndex((item) => item.id === tab)
    const targetIndex =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? tabs.length - 1
          : (Math.max(currentIndex, selectedIndex) +
              (event.key === 'ArrowRight' ? 1 : -1) +
              tabs.length) %
            tabs.length
    const target = tabs[targetIndex]
    if (!target) return
    event.preventDefault()
    const targetTab = PANEL_TABS[targetIndex]
    if (targetTab) onTab(targetTab.id)
    target.focus()
  }

  return (
    <aside
      className={`context-panel is-${tab}`}
      aria-label="Conversation details"
    >
      <div className="context-panel-heading">
        <div
          className="panel-tabs"
          role="tablist"
          aria-label="Conversation panels"
          onKeyDown={handleTabKeyDown}
        >
          {PANEL_TABS.map((item) => (
            <button
              key={item.id}
              className={tab === item.id ? 'active' : ''}
              type="button"
              role="tab"
              aria-selected={tab === item.id}
              tabIndex={tab === item.id ? 0 : -1}
              onClick={() => {
                onTab(item.id)
              }}
            >
              {item.label}
              {item.id === 'diff' && reviewStat && (
                <span className="panel-tab-count">{reviewStat.files}</span>
              )}
            </button>
          ))}
        </div>
        <Button
          className="panel-close"
          variant="ghost"
          size="sm"
          aria-label="Close details panel"
          onClick={onClose}
        >
          Close <kbd>esc</kbd>
        </Button>
      </div>
      {tab === 'files' ? (
        <div className="context-panel-fill">
          <FilesView conversation={conversation} desktop={desktop} />
        </div>
      ) : (
        <ScrollArea className="context-panel-scroll">
          {tab === 'details' && <DetailsView conversation={conversation} />}
          {tab === 'diff' && (
            <DiffView conversation={conversation} desktop={desktop} />
          )}
        </ScrollArea>
      )}
      {tab === 'diff' && (
        <p className="review-footer">
          Read-only. Ask the agent for edits, or open the file in your editor.
        </p>
      )}
    </aside>
  )
}

function DetailsView({ conversation }: { conversation: Conversation }) {
  return (
    <div className="details-view">
      <div className="detail-status-block">
        <div className="detail-status-head">
          <StatusMark status={conversation.status} />
          <strong>{statusText[conversation.status]}</strong>
          {conversation.updated && (
            <span>Updated {conversation.updated} ago</span>
          )}
        </div>
        <p>
          {conversation.status === 'unknown'
            ? `Last known status: ${conversation.lastKnownStatus ? statusText[conversation.lastKnownStatus] : 'Unavailable'}. Reconnect to confirm.`
            : conversation.status === 'needs-you'
              ? 'The agent is waiting for your decision.'
              : conversation.status === 'working'
                ? 'The agent is making progress on this task.'
                : 'The conversation is saved and ready to continue.'}
        </p>
      </div>
      <DetailSection title="Conversation">
        <DetailRow
          label="Provider"
          value={
            <span className="detail-agent">
              <AgentAvatar
                provider={conversation.provider}
                status={conversation.status}
                small
              />
              {conversation.provider}
            </span>
          }
        />
        <DetailRow label="Model" value={conversation.model} />
        <DetailRow
          label="Native session"
          value={
            conversation.nativePane === null
              ? 'Not linked'
              : `Pane ${String(conversation.nativePane)}`
          }
        />
      </DetailSection>
      <DetailSection title="Project">
        <DetailRow
          label="Folder"
          value={
            <span className="path-value" title={conversation.folder}>
              {conversation.folder}
            </span>
          }
        />
        <DetailRow
          label="Branch"
          value={
            <span className="detail-agent">
              <GitBranch size={13} />
              {conversation.branch}
            </span>
          }
        />
      </DetailSection>
      <DetailSection title="Permissions">
        <div className="permission-summary">
          <ShieldCheck size={15} />
          <span>{conversation.permission}</span>
        </div>
        <p className="permission-note">
          Maitty keeps the provider’s permission settings. Approval requests
          appear in this conversation.
        </p>
      </DetailSection>
      <DetailSection title="Context">
        <div className="context-meter-heading">
          <span>Conversation context</span>
          <span>
            {conversation.context === null
              ? 'Unavailable'
              : `${String(conversation.context)}%`}
          </span>
        </div>
        {conversation.context !== null && (
          <div className="context-meter">
            <span style={{ width: `${String(conversation.context)}%` }} />
          </div>
        )}
      </DetailSection>
      <div className="daemon-note">
        <span className="connection-dot" />
        <div>
          <strong>Owned by Maitty daemon</strong>
          <p>Closing the window will not stop this session.</p>
        </div>
      </div>
    </div>
  )
}

function DetailSection({
  title,
  children,
}: {
  title: string
  children: ReactNode
}) {
  return (
    <section className="detail-section">
      <h3>{title}</h3>
      {children}
    </section>
  )
}

function DetailRow({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <div>{value}</div>
    </div>
  )
}

export function TerminalView({
  conversation,
  desktop,
  fontSize,
}: {
  conversation: Conversation
  desktop: boolean
  fontSize: number
}) {
  if (desktop)
    return <DaemonTerminalView conversation={conversation} desktop={desktop} fontSize={fontSize} />
  return (
    <div className="terminal-view">
      <div className="terminal-heading">
        <span className="terminal-live-dot" />
        Native session · pane {conversation.nativePane || 'pending'}
      </div>
      <pre className="terminal-output">
        <span className="terminal-dim">~/maitty · {conversation.branch}</span>
        {'\n'}
        <span className="terminal-green">$ cargo test branch_view_tests</span>
        {'\n'}running 8 tests{'\n'}
        <span className="terminal-ok">
          test branch_view::renders_status ... ok
        </span>
        {'\n'}test branch_view::opens_switcher ... ok{'\n'}test result: ok. 8
        passed{'\n'}
        <span className="terminal-prompt">❯</span>{' '}
        <span className="terminal-cursor" />
      </pre>
      <div className="terminal-footnote">
        <Activity size={13} />
        Preview terminal output · interactive PTY bridge not connected yet
      </div>
    </div>
  )
}
