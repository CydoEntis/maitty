import { useEffect, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { TerminalView } from '@/TerminalView'
import { useWorkspace } from '../workspaceContext'
import type { SessionView } from '../model/session'
import { clampDrawerHeight, DRAWER_HEAD_PX, readDrawerHeight, writeDrawerHeight } from '../model/terminalDrawer'
import { Icon } from './glyphs'

type Props = {
  /** The agent the terminals belong to: they open in its folder. */
  agent: SessionView
  /**
   * Where they are shown. In the side panel they fill its Terminal tab; under the chat they are a
   * drawer of the agent's pane, with a handle to size it and a header that folds it away.
   */
  dock: 'panel' | 'under'
  /** The keyboard goes to the terminal that shows, not to the chat. */
  focused: boolean
  /** Under the chat: the drawer is down to its header, and the pane says when that changes. */
  folded?: boolean
  onFold?: (folded: boolean) => void
}

/**
 * An agent's own terminals (layout 2e): a tab each, + for another and × to close one. They are shells
 * the daemon owns like any session; that they belong to this agent is the window's knowledge
 * (`data/agentTerminals.ts`), which is why they are tabs here and not rows of the sidebar.
 */
export function AgentTerminals({ agent, dock, focused, folded: asked = false, onFold }: Props) {
  const { agentTerminals, activeTerminals, setActiveTerminals, addAgentTerminal, closeAgentTerminal, sessionMap, preferences, setPreferences, look, online, canStart, modalOpen, openPath, pasteImage, terminalHandles } = useWorkspace()
  const ids = agentTerminals.get(agent.id) ?? []
  const picked = activeTerminals.get(agent.id)
  const activeId = picked !== undefined && ids.includes(picked) ? picked : ids.at(-1)
  const active = activeId === undefined ? undefined : sessionMap.get(activeId)
  const [height, setHeight] = useState(readDrawerHeight)
  // With no terminal yet the drawer is only its header: there is nothing to give the room to.
  const folded = asked || ids.length === 0
  // A terminal that was just added or picked is what the person asked to see, so a folded drawer opens for it.
  useEffect(() => { if (picked !== undefined) onFold?.(false) }, [picked, onFold])
  const pick = (id: string) => { setActiveTerminals((current) => new Map(current).set(agent.id, id)) }
  const moveTo = (terminalDock: 'panel' | 'under') => { setPreferences((current) => ({ ...current, terminalDock })) }

  const startResize = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault()
    const pane = event.currentTarget.closest<HTMLElement>('.mx-pane')
    const room = pane?.getBoundingClientRect().height ?? window.innerHeight
    const from = event.clientY
    const begin = height
    const at = (now: PointerEvent) => clampDrawerHeight(begin + (from - now.clientY), room)
    const move = (now: PointerEvent) => { setHeight(at(now)) }
    const stop = (now: PointerEvent) => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', stop)
      writeDrawerHeight(at(now))
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', stop)
  }

  const tabs = (
    <div className="mx-terms-tabs" role="tablist" aria-label={`Terminals of ${agent.cwdLabel}`}>
      {ids.map((id, index) => (
        <span key={id} className={`mx-terms-tab${id === activeId ? ' is-current' : ''}`}>
          <button type="button" role="tab" aria-selected={id === activeId} className="mx-terms-pick" onClick={() => { pick(id) }}>
            <span className="mono" aria-hidden="true">&gt;_</span>Terminal {index + 1}
          </button>
          <button type="button" className="mx-terms-close" aria-label={`Close terminal ${String(index + 1)}`} title="Close this terminal" disabled={!online} onClick={() => { closeAgentTerminal(id) }}><Icon name="close" size={10} /></button>
        </span>
      ))}
      <button type="button" className="mx-terms-add" aria-label="New terminal" title={`New terminal in ${agent.cwdLabel}`} disabled={!canStart} onClick={() => { addAgentTerminal(agent.id) }}><Icon name="plus" size={12} /></button>
    </div>
  )

  const body = active === undefined ? (
    <div className="mx-terms-empty">
      <strong>No terminal yet</strong>
      <span>A terminal opens in {agent.cwdLabel}, beside the agent.</span>
      <button type="button" className="mx-terms-start" disabled={!canStart} onClick={() => { addAgentTerminal(agent.id) }}>New terminal</button>
    </div>
  ) : (
    <div className="mx-terms-screen">
      <TerminalView
        key={active.id}
        conversation={active.conversation}
        desktop
        fontSize={preferences.terminalFontSize}
        look={look}
        label={`Terminal in ${active.cwdLabel}`}
        inputDisabled={!online}
        ended={active.ended}
        focused={focused && !modalOpen && !folded}
        paused={dock === 'under' && folded}
        onOpenPath={(path, line) => { openPath(active, path, line) }}
        onPasteImage={(image) => { pasteImage(active.id, image) }}
        onHandle={(handle) => {
          if (handle) terminalHandles.current.set(active.id, handle)
          else terminalHandles.current.delete(active.id)
        }}
      />
    </div>
  )

  if (dock === 'panel') {
    return (
      <div className="mx-terms is-panel">
        <div className="mx-terms-head">
          {tabs}
          <span className="mx-spacer" />
          <button type="button" className="mx-terms-tool" aria-label="Pin terminals under the chat" title="Pin terminals under the chat" onClick={() => { moveTo('under') }}><Icon name="pin" size={13} /></button>
        </div>
        {body}
      </div>
    )
  }
  return (
    <div className={`mx-terms is-under${folded ? ' is-folded' : ''}`} style={{ height: folded ? DRAWER_HEAD_PX : height }} role="group" aria-label="Terminals">
      {!folded && <div className="mx-terms-grip" role="separator" aria-orientation="horizontal" aria-label="Resize the terminals" onPointerDown={startResize} />}
      <div className="mx-terms-head">
        <button type="button" className="mx-terms-fold" aria-expanded={!folded} disabled={ids.length === 0} onClick={() => { onFold?.(!folded) }}>
          <span className={`mx-caret-wrap${folded ? '' : ' is-open'}`}><Icon name="caret" /></span>Terminals
        </button>
        {tabs}
        <span className="mx-spacer" />
        <span className="mx-terms-where" title={agent.cwd}>{agent.cwdLabel}</span>
        <button type="button" className="mx-terms-tool" aria-label="Move terminals to the side panel" title="Move terminals to the side panel" onClick={() => { moveTo('panel') }}><Icon name="expand" size={13} /></button>
      </div>
      {!folded && body}
    </div>
  )
}
