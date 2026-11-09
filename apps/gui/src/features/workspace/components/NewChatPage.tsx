import { useEffect, useRef, useState, type KeyboardEvent } from 'react'
import { errorMessage } from '@/lib/error-message'
import { DEFAULT_PICK, REASONING_LABEL, REASONING_PICKS, modelsFor, useAgentModels, useDiscoveredModels, type AgentChoice, type ReasoningPick } from '@/features/chat/agentChoice'
import { AGENT_LABEL, modelName } from '@/features/chat/modelName'
import { BYPASS_MODE, modeLabel, PICKABLE_MODES } from '@/features/chat/chatMeta'
import { DropdownMenu, DropdownMenuContent, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { openAgentDocs } from '@/api/daemon'
import { chatAgentOf, type ChatAgent } from '../model/chatView'
import { AGENT_CATALOG, ageOf, nowSeconds, splitRecent, useFolderBranches, useInstalledAgents } from '../model/newChatHost'
import type { NewSessionProject } from '../model/newSession'
import { AppIcon, Icon } from './glyphs'
import { ChipMenu, ReasoningMeter, SegmentTabs, type ChipItem } from './NewChatMenus'

/** The branch a chat starts on when it is not the folder's current one: switch to it, or work in a new worktree of it. */
export type BranchChoice = { name: string; create: boolean; worktree: boolean }

/** What the page asks the window to start: a chat with this agent, model and effort, in this folder, and its first message. */
export type NewChatRequest = { folder: string; agent: ChatAgent; model: string; reasoning: AgentChoice['reasoning']; mode: string; message: string; branch: BranchChoice | null }

/** A computer a chat could start on. Chats run on this computer; another one hands over to the dialog that starts a session there. */
export type NewChatMachine = { machine: number; label: string; local: boolean }

type Props = {
  machines: readonly NewChatMachine[]
  /** The projects this computer has sessions in, the focused one first. */
  projects: readonly NewSessionProject[]
  defaultAgent: ChatAgent
  /** Yolo mode is on for Claude, so a Claude chat may start in Bypass permissions. */
  bypassAllowed?: boolean
  /** The system's folder picker; null when it is cancelled. */
  browse: () => Promise<string | null>
  onStart: (request: NewChatRequest) => Promise<void>
  onOtherMachine: (machine: number) => void
  /** Opens the dialog that adds a machine; without it the list has no such row. */
  onAddMachine?: () => void
  onCancel: () => void
}

/**
 * A new agent chat: the chat page itself, empty. The box takes the first message; under it sit the
 * choices a chat starts from (the computer, the folder, the agent, its model and effort, and the
 * branch). The chat it starts is the agent in a terminal the daemon owns, shown as its chat (ADR 0003).
 * Enter sends, Esc goes back.
 */
export function NewChatPage({ machines, projects, defaultAgent, bypassAllowed = false, browse, onStart, onOtherMachine, onAddMachine, onCancel }: Props) {
  const [agent, setAgent] = useState<ChatAgent>(defaultAgent)
  const [mode, setMode] = useState('default')
  const [choice, setChoice] = useState<AgentChoice>({ model: DEFAULT_PICK, reasoning: DEFAULT_PICK })
  const [picked, setFolder] = useState<string | null>(null)
  const [extra, setExtra] = useState<NewSessionProject | null>(null)
  const [message, setMessage] = useState('')
  const [branchPick, setBranchPick] = useState<{ folder: string; pick: BranchChoice } | null>(null)
  const [treeMode, setTreeMode] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const text = useRef<HTMLTextAreaElement>(null)
  // Until one is picked, the first project is the choice, even when the list arrives after the page opened.
  const folder = picked ?? projects[0]?.folder ?? ''
  const shown = extra === null || projects.some((project) => project.folder === extra.folder) ? projects : [...projects, extra]
  const current = shown.find((project) => project.folder === folder)
  const folderName = current?.name ?? (folder === '' ? 'Choose a folder' : folder.split(/[\\/]/).filter(Boolean).at(-1) ?? folder)
  const local = machines.find((machine) => machine.local)
  const installed = useInstalledAgents()
  const [branches, reloadBranches] = useFolderBranches(folder)
  // A branch chosen for one folder means nothing in another.
  const branch = branchPick?.folder === folder ? branchPick.pick : null
  const isInstalled = (id: string) => (installed === null ? chatAgentOf(id) !== null : installed.includes(id))
  const agentMissing = !isInstalled(agent)

  useEffect(() => { text.current?.focus() }, [])

  const pickAgent = (next: ChatAgent) => {
    setAgent(next)
    // A model is the agent's own; the other agent's name means nothing to it.
    if (!modelsFor(next).some((option) => option.id === choice.model)) setChoice((now) => ({ ...now, model: DEFAULT_PICK }))
  }
  const start = () => {
    if (busy || folder === '' || agentMissing) return
    setBusy(true)
    setError(null)
    onStart({ folder, agent, model: choice.model, reasoning: choice.reasoning, mode: agent === 'claude' ? mode : 'default', message: message.trim(), branch })
      .catch((failure: unknown) => { setError(errorMessage(failure, 'Could not start the chat')) })
      .finally(() => { setBusy(false); reloadBranches() })
  }
  const pickOther = () => {
    void browse().then((chosen) => {
      if (chosen === null) return
      const name = chosen.split(/[\\/]/).filter(Boolean).at(-1) ?? chosen
      setExtra({ folder: chosen, name, color: 'var(--dim)' })
      setFolder(chosen)
    })
  }
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.nativeEvent.isComposing) return
    if (event.key === 'Enter' && !event.shiftKey && event.target === text.current) {
      event.preventDefault()
      start()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      onCancel()
    }
  }

  const models = useAgentModels(agent)
  const model = models.find((option) => option.id === choice.model) ?? models[0]
  const empty = message.trim() === ''
  useDiscoveredModels(agent)

  const machineItems: ChipItem[] = machines.map((machine) => ({
    key: String(machine.machine),
    label: machine.label,
    sub: machine.local ? 'This computer' : 'Remote · starts a session there',
    icon: <span className="mx-chip-dot" aria-hidden="true" />,
    checked: machine.local,
    onPick: () => { if (!machine.local) onOtherMachine(machine.machine) },
  }))
  const folderItem = (project: NewSessionProject): ChipItem => ({
    key: project.folder,
    label: project.name,
    sub: project.folder,
    icon: <span className="mx-ns-square" style={{ background: project.color }} aria-hidden="true" />,
    right: project.sessions === undefined ? undefined : <span className="mx-chip-count">{project.sessions}</span>,
    checked: project.folder === folder,
    onPick: () => { setFolder(project.folder) },
  })
  const { recent, rest } = splitRecent(shown)
  const folderSections = [{ heading: 'Recent', items: recent.map(folderItem) }, ...(rest.length > 0 ? [{ heading: 'All on this computer', items: rest.map(folderItem) }] : [])]
  const fromPath = (query: string): ChipItem | null => {
    if (!/^(~|\/|[A-Za-z]:[\\/])/.test(query)) return null
    const name = query.split(/[\\/]/).filter(Boolean).at(-1) ?? query
    return { key: 'path', label: `Use ${query}`, icon: <Icon name="folder" size={12} />, checked: false, onPick: () => { setExtra({ folder: query, name, color: 'var(--dim)' }); setFolder(query) } }
  }
  const agentRow = (entry: { id: string; label: string }): ChipItem => {
    const chat = chatAgentOf(entry.id)
    const ready = isInstalled(entry.id)
    return {
      key: entry.id,
      label: entry.label,
      ...(ready && chat === null ? { sub: 'No chat view yet' } : {}),
      icon: <AppIcon process={entry.id} size={18} />,
      ...(!ready ? { right: <span className="mx-chip-setup">Set up</span> } : chat === null ? {} : { right: <span className="mx-chip-count">{`${String(modelsFor(chat).length - 1)} models`}</span> }),
      checked: chat === agent && ready,
      disabled: ready && chat === null,
      onPick: () => {
        if (!ready) void openAgentDocs(entry.id).catch((failure: unknown) => { setError(errorMessage(failure, 'Could not open the setup page')) })
        else if (chat !== null) pickAgent(chat)
      },
    }
  }
  const agentSections = [
    { heading: 'Installed', items: AGENT_CATALOG.filter((entry) => isInstalled(entry.id)).map(agentRow) },
    { heading: 'Not set up', items: AGENT_CATALOG.filter((entry) => !isInstalled(entry.id)).map(agentRow) },
  ]
  const modelItems: ChipItem[] = models.map((option) => ({
    key: option.id,
    label: option.label,
    sub: option.blurb,
    checked: option.id === (model?.id ?? ''),
    onPick: () => { setChoice((now) => ({ ...now, model: option.id })) },
  }))
  const [now] = useState(nowSeconds)
  const here = branch !== null && !branch.worktree ? branch.name : branches?.current ?? ''
  const inWorktree = new Set((branches?.worktrees ?? []).flatMap((tree) => (tree.branch === null ? [] : [tree.branch])))
  const branchLabel = branch === null ? branches?.current ?? 'detached' : `${branch.name}${branch.worktree ? ' (worktree)' : ''}`
  const pickBranch = (pick: BranchChoice) => { setBranchPick({ folder, pick }) }
  const worktreeRows: ChipItem[] = (branches?.worktrees ?? []).filter((tree) => !tree.main).map((tree) => {
    const name = tree.branch ?? tree.path.split(/[\\/]/).filter(Boolean).at(-1) ?? tree.path
    return {
      key: `tree:${tree.path}`,
      label: name,
      sub: tree.path,
      icon: <Icon name="folder" size={13} />,
      right: <span className="mx-chip-count">{ageOf(tree.when, now)}</span>,
      checked: tree.path === folder,
      onPick: () => { setExtra({ folder: tree.path, name, color: 'var(--dim)' }); setFolder(tree.path); setBranchPick(null) },
    }
  })
  const branchRows: ChipItem[] = (branches?.branches ?? []).filter((entry) => !treeMode || !inWorktree.has(entry.name)).map((entry) => ({
    key: `branch:${entry.name}`,
    label: entry.name,
    icon: <Icon name="branch" size={13} />,
    right: <span className="mx-chip-count">{ageOf(entry.when, now)}</span>,
    checked: !treeMode && entry.name === here,
    onPick: () => { pickBranch({ name: entry.name, create: false, worktree: treeMode }) },
  }))
  const branchSections = [
    ...(worktreeRows.length > 0 ? [{ heading: 'Worktrees', items: worktreeRows }] : []),
    { heading: 'Branches', items: branchRows },
  ]
  const fromBranchName = (query: string): ChipItem | null => {
    if (!/^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(query) || (branches?.branches ?? []).some((entry) => entry.name === query)) return null
    return { key: 'new-branch', label: `Create branch ${query}`, icon: <Icon name="plus" size={12} />, checked: false, onPick: () => { pickBranch({ name: query, create: true, worktree: treeMode }) } }
  }
  const effortLabels: Readonly<Record<ReasoningPick, string>> = { ...REASONING_LABEL, [DEFAULT_PICK]: 'Auto' }

  return (
    <section className="mx-newchat" aria-label="New agent chat" onKeyDown={onKeyDown}>
      <button type="button" className="mx-icon-button mx-newchat-close" aria-label="Close" title="Close (Esc)" onClick={onCancel}><Icon name="close" size={13} /></button>
      <p className="mx-chat-empty">{`What should ${AGENT_LABEL[agent]} work on?`}</p>
      <div className="mx-composer">
        <div className="mx-composer-column">
          <div className="mx-newchat-stack mx-field">
            <div className="mx-ctxrow">
              <span>on</span>
              <ChipMenu title="Computer" label={local?.label ?? 'This computer'} icon={<span className="mx-chip-dot" aria-hidden="true" />} sections={[{ heading: 'Connected', items: machineItems }]} search="Search machines"
                footer={onAddMachine && <button type="button" className="mx-chip-action" onClick={onAddMachine}><Icon name="plus" size={12} />Add a machine…</button>} />
              <span>in</span>
              <ChipMenu title="Folder" label={folderName} icon={<span className="mx-ns-square" style={{ background: current?.color ?? 'var(--dim)' }} aria-hidden="true" />} search="Search projects or paste a path" width={360}
                sections={folderSections} fromQuery={fromPath}
                footer={<><button type="button" className="mx-chip-action" onClick={pickOther}><Icon name="folder" size={12} />Open folder…</button><p>Changing folder changes where this chat starts.</p></>} />
              <span>using</span>
              <ChipMenu title="Agent" label={AGENT_LABEL[agent]} icon={<AppIcon process={agent} size={14} />} sections={agentSections} search="Search agents"
                footer={<p>Agents are the coding programs Maitty finds on this computer.</p>} />
              <ChipMenu title="Model and reasoning" summary={`${modelName(agent, model, null)} ${REASONING_LABEL[choice.reasoning]}`} label={<><strong>{modelName(agent, model, null)}</strong><span className="mx-chip-muted">{REASONING_LABEL[choice.reasoning]}</span></>} search="Search models" width={360}
                sections={[{ heading: 'Models', items: modelItems }]}
                below={<ReasoningMeter levels={REASONING_PICKS} value={choice.reasoning} labels={effortLabels} onPick={(level) => { setChoice((now) => ({ ...now, reasoning: level })) }} />}
                footer={<p>Used by this chat from its first message.</p>} />
              {branches !== null && (
                <>
                  <span>on</span>
                  <ChipMenu title="Branch" summary={branchLabel} label={<span className="mx-chip-mono">{branchLabel}</span>} icon={<Icon name="branch" size={12} />}
                    search="Search or create a branch" width={380} sections={branchSections} fromQuery={fromBranchName}
                    extra={<SegmentTabs value={treeMode ? 'worktree' : 'checkout'} options={[{ value: 'checkout', label: 'This checkout' }, { value: 'worktree', label: 'New worktree' }]} onPick={(value) => { setTreeMode(value === 'worktree') }} />}
                    footer={<p>{treeMode ? 'Makes a worktree of the branch beside the project, and the chat works there.' : `Switches the branch in ${folder}. Other sessions in this folder see the switch too.`}</p>} />
                </>
              )}
            </div>
            <div className="mx-box">
              <textarea ref={text} aria-label="First message" rows={2} value={message} placeholder={`What should ${AGENT_LABEL[agent]} work on?`} className="mx-box-text" onChange={(event) => { setMessage(event.target.value) }} />
              <div className="mx-box-row">
                <span className="mx-spacer" />
                <button type="button" className={`mx-send${busy || folder === '' || agentMissing ? ' is-off' : ''}`} aria-label={empty ? 'Start chat' : 'Send message'} title={empty ? 'Start an empty chat' : 'Send (Enter)'} disabled={busy || folder === '' || agentMissing} onClick={start}>
                  <Icon name="arrowUp" size={15} strokeWidth={2.2} />
                </button>
              </div>
            </div>
            <div className="mx-ctxtray">
              {agent === 'claude' ? (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <button type="button" className={`mx-tray-fact is-button${mode === BYPASS_MODE ? ' is-danger' : ''}`} title="The permission mode this chat starts in" aria-label={`Starting permission mode: ${modeLabel(mode) ?? mode}. Pick another.`}>
                      {modeLabel(mode)}
                      <Icon name="chevronDown" size={10} />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent side="top" align="start" aria-label="Starting permission mode" className="w-[240px] text-fg">
                    <DropdownMenuRadioGroup value={mode} onValueChange={setMode}>
                      {PICKABLE_MODES.map((key) => (
                        <DropdownMenuRadioItem
                          key={key}
                          value={key}
                          disabled={key === BYPASS_MODE && !bypassAllowed}
                          title={key === BYPASS_MODE && !bypassAllowed ? 'Turn on Yolo mode for Claude in Settings, General, to start a chat in this mode.' : undefined}
                          className="text-[12.5px] text-fg focus:bg-hover focus:text-fg"
                        >
                          {modeLabel(key)}
                          {key === BYPASS_MODE && <span className="ml-auto pl-2 text-[11px] text-dim">{bypassAllowed ? 'asks nothing' : 'needs Yolo mode'}</span>}
                        </DropdownMenuRadioItem>
                      ))}
                    </DropdownMenuRadioGroup>
                  </DropdownMenuContent>
                </DropdownMenu>
              ) : (
                <span className="mx-tray-fact" title="This chat asks before it runs a tool">Ask before running</span>
              )}
              <span className="mx-spacer" />
              {folder !== '' && <span className="mx-tray-fact" title={folder}>{folder}</span>}
            </div>
          </div>
          {error !== null && <p role="alert" className="mx-composer-error">{error}</p>}
          {agentMissing && <p role="alert" className="mx-composer-error">{`${AGENT_LABEL[agent]} is not installed on this computer. Pick another agent, or set it up.`}</p>}
        </div>
      </div>
    </section>
  )
}
