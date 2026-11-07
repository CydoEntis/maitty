import { useRef, useState, type KeyboardEvent } from 'react'
import type { Tab } from '../model/tabs'
import { displayName, MAX_SESSION_NAME_LENGTH, rowName, STATUS_WORD, type SessionView } from '../model/session'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { InlineName } from './InlineName'
import { runsProgram, useArmed } from './useArmed'

type TabProps = {
  tab: Tab
  head: SessionView
  /** Closing the tab ends every session in it, so a running program asks for a second click. */
  running: boolean
  focusable: boolean
  onSelect: () => void
  onClose: () => void
  onSetName: (name: string) => void
}

function TabItem({ tab, head, running, focusable, onSelect, onClose, onSetName }: TabProps) {
  const [editing, setEditing] = useState(false)
  const closing = useArmed(running)
  const closeLabel = closing.armed ? `Click again to close ${displayName(head)} and stop what runs in it` : `Close ${displayName(head)}`
  return (
    <div className={`mx-tab${tab.active ? ' is-active' : ''}`} role="presentation">
      {editing
        ? (
          <div className="mx-tab-main">
            <InlineName value={head.name ?? ''} placeholder={head.process} label={`Name for ${displayName(head)}`} maxLength={MAX_SESSION_NAME_LENGTH} onCommit={(name) => { setEditing(false); onSetName(name) }} onCancel={() => { setEditing(false) }} />
          </div>
        )
        : (
          <button
            type="button"
            role="tab"
            className="mx-tab-main"
            aria-selected={tab.active}
            aria-label={`${displayName(head)}, ${head.process}, ${STATUS_WORD[head.status]}${tab.paneCount > 1 ? `, ${String(tab.paneCount)} panes` : ''}`}
            tabIndex={focusable ? 0 : -1}
            onClick={onSelect}
            onDoubleClick={() => { setEditing(true) }}
            onKeyDown={(event) => { if (event.key === 'F2') { event.preventDefault(); setEditing(true) } }}
          >
            <span className="mx-tab-icon">
              <AppIcon process={head.process} size={18} dimmed={head.status === 'unknown'} />
              <StatusBadge status={head.status} rail />
            </span>
            <span className="mx-tab-name">{rowName(head)}</span>
            {tab.paneCount > 1 && <span className="mx-tab-panes" title={`${String(tab.paneCount)} panes`}>{tab.paneCount}</span>}
          </button>
        )}
      <button type="button" className={`mx-tab-close${closing.armed ? ' is-armed' : ''}`} aria-label={closeLabel} title={closeLabel} tabIndex={-1} onBlur={closing.disarm} onClick={() => { if (closing.press()) onClose() }}>
        <Icon name="close" size={11} />
      </button>
    </div>
  )
}

type Props = {
  tabs: readonly Tab[]
  sessions: ReadonlyMap<string, SessionView>
  /** The selected fleet's name, for the tray's label. */
  fleetName: string
  onSelect: (tab: Tab) => void
  onClose: (tab: Tab) => void
  onSetName: (sessionId: string, name: string) => void
}

/** The tray of the selected fleet's tabs: one per ship, the active one raised. Arrow keys move between tabs. */
export function TabBar({ tabs, sessions, fleetName, onSelect, onClose, onSetName }: Props) {
  const tray = useRef<HTMLDivElement>(null)
  const active = tabs.findIndex((tab) => tab.active)
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft' && event.key !== 'Home' && event.key !== 'End') return
    const buttons = Array.from(tray.current?.querySelectorAll<HTMLButtonElement>('[role="tab"]') ?? [])
    if (buttons.length === 0) return
    event.preventDefault()
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement)
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (at + (event.key === 'ArrowRight' ? 1 : -1) + buttons.length) % buttons.length
    buttons[next]?.focus()
  }
  return (
    <div className="mx-tabbar" ref={tray}>
      <div className="mx-tabtray" role="tablist" aria-label={`Tabs of ${fleetName}`} onKeyDown={onKeyDown}>
        {tabs.map((tab, index) => {
          const head = sessions.get(tab.sessionId)
          if (!head) return null
          const running = tab.sessionIds.some((id) => { const member = sessions.get(id); return member !== undefined && runsProgram(member) })
          return (
            <TabItem
              key={tab.arrangementId ?? tab.sessionId}
              tab={tab}
              head={head}
              running={running}
              focusable={active === -1 ? index === 0 : index === active}
              onSelect={() => { onSelect(tab) }}
              onClose={() => { onClose(tab) }}
              onSetName={(name) => { onSetName(head.id, name) }}
            />
          )
        })}
      </div>
    </div>
  )
}
