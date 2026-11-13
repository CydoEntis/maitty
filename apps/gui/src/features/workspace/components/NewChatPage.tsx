import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type CSSProperties, type KeyboardEvent } from 'react'
import { useWarmAgent, type WarmAgent, type WarmSpec } from '../useWarmAgent'
import { toast } from '@/lib/toast'
import { pasteFromHost, pickFromClipboard, unsupportedMessage } from '@/features/terminal/clipboardImage'
import { buildAttachmentMessage } from '@/features/chat/attachmentMessage'
import { registerComposerDrop } from '@/features/chat/composerDrop'
import { updateComposer, useComposerState } from '@/features/chat/composerStore'
import { revokePreviews, useAttachmentIntake } from '@/features/chat/useAttachmentIntake'
import { DraftAttachments } from '@/features/chat/components/Attachments'
import { errorMessage } from '@/lib/error-message'
import { DEFAULT_PICK, modelsFor, START_REASONING, type AgentChoice } from '@/features/chat/agentChoice'
import { AGENT_LABEL } from '@/features/chat/modelName'
import { CODEX_FULL_MODE, CODEX_START_MODES, DANGER_MODES, modeLabel, PICKABLE_MODES, START_MODE } from '@/features/chat/chatMeta'
import { AgentPicker } from '@/features/chat/components/AgentPicker'
import { PermissionMenu } from '@/features/chat/components/PermissionMenu'
import { openAgentDocs } from '@/api/daemon'
import { LOCAL_MACHINE } from '@/api/machines'
import { chatAgentOf, type ChatAgent } from '../model/chatView'
import { ageOf, nowSeconds, splitRecent, useFolderBranches, useGlances, useInstalledAgents, useSyncState } from '../model/newChatHost'
import { Switch } from '@/components/ui/switch'
import type { NewSessionProject } from '../model/newSession'
import { suggestBranchName } from '../model/worktreeName'
import { Icon } from './glyphs'
import { ChipMenu, type ChipItem } from './NewChatMenus'

/** The branch a chat starts on when it is not the folder's current one: switch to it, or work in a new worktree of it. */
export type BranchChoice = { name: string; create: boolean; worktree: boolean }

/** What the page asks the window to start: a chat with this agent, model and effort, in this folder, and its first message. */
export type NewChatRequest = { folder: string; agent: ChatAgent; model: string; reasoning: AgentChoice['reasoning']; mode: string; message: string; branch: BranchChoice | null; machine: number; machineName: string }

/** The key the first message's files are kept under: no conversation exists yet, and a real one is never negative. */
const NEW_CHAT_DRAFT = -1

/** A computer a chat could start on. */
export type NewChatMachine = { machine: number; label: string; local: boolean }

type Props = {
  machines: readonly NewChatMachine[]
  /** The computer the chat starts on. */
  machine: number
  /** The projects that computer has sessions in, the focused one first. */
  projects: readonly NewSessionProject[]
  defaultAgent: ChatAgent
  /** Yolo mode is on for Claude, so a Claude chat may start in Bypass permissions. */
  bypassAllowed?: boolean
  /** Yolo mode is on for Codex, so every Codex chat starts with full access whatever is picked here. */
  codexYolo?: boolean
  /** A folder picker whose answer is null when it is cancelled: the system's for this computer, the window's own for another machine. Null when there is none. */
  browse: (() => Promise<string | null>) | null
  /** Clones a repository onto that computer and answers with its new folder, or null when it is cancelled. Without it the page has no such button. */
  clone?: (() => Promise<string | null>) | null
  onStart: (request: NewChatRequest) => Promise<void>
  /** Starts the chat's agent while the first message is written (`useWarmAgent`); without it the agent starts on send. */
  onWarm?: WarmAgent
  onOtherMachine: (machine: number) => void
  /** Opens the dialog that adds a machine; without it the list has no such row. */
  onAddMachine?: () => void
  onCancel: () => void
  /** A folder and first message to open with (a chat continued from a full one). */
  seed?: { folder: string; message: string } | null
}

/** How many other recent projects the page offers to switch to. */
const SWITCH_CHIPS = 4

/** What the page offers to start with once a folder is picked; a click starts the chat with it. */
const SUGGESTED_TASKS = ['Explain how this project is laid out', 'Review my uncommitted changes', 'Find and fix a failing test'] as const

const lastPart = (path: string): string => path.split(/[\\/]/).filter(Boolean).at(-1) ?? path
const isPath = (text: string): boolean => /^(~|\/|[A-Za-z]:[\\/])/.test(text)

/**
 * A new agent chat (layout 2f). First a folder is picked from the projects, a pasted path or the
 * system's folder picker; then three buttons say where the chat works (project, branch, a new
 * worktree or this folder) over a few tasks to start from. The message box is under both, with the
 * agent, model and effort on its model button and the permission mode beside it. The chat it starts
 * is the agent in a terminal the daemon owns, shown as its chat (ADR 0003). Enter sends, Esc goes back.
 */
export function NewChatPage({ machines, machine, projects, defaultAgent, bypassAllowed = false, codexYolo = false, browse, clone = null, onStart, onWarm, onOtherMachine, onAddMachine, onCancel, seed = null }: Props) {
  // On another machine the chat is the agent started in a shell there: this computer's models, branches, files and installed agents say nothing of it.
  const remote = machine !== LOCAL_MACHINE
  const target = machines.find((candidate) => candidate.machine === machine)
  const targetLabel = target?.label ?? 'This computer'
  const [agent, setAgent] = useState<ChatAgent>(defaultAgent)
  const [mode, setMode] = useState(START_MODE[defaultAgent])
  const [choice, setChoice] = useState<AgentChoice>({ model: DEFAULT_PICK, reasoning: START_REASONING })
  const [picked, setFolder] = useState<string | null>(seed?.folder ?? null)
  const [extra, setExtra] = useState<NewSessionProject | null>(seed === null ? null : { folder: seed.folder, name: lastPart(seed.folder), color: 'var(--dim)' })
  const [message, setMessage] = useState(seed?.message ?? '')
  const [branchPick, setBranchPick] = useState<{ folder: string; pick: BranchChoice } | null>(null)
  // Own copy: the agent works in a worktree of its own, so the checkout is untouched. On unless it is turned off.
  const [treeMode, setTreeMode] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const text = useRef<HTMLTextAreaElement>(null)
  const box = useRef<HTMLDivElement>(null)
  const { attachments } = useComposerState(NEW_CHAT_DRAFT)
  // A new chat runs on this computer, so its files are copied here and named by path in the first message.
  const attach = useAttachmentIntake({ conversation: NEW_CHAT_DRAFT, blocked: remote ? `Files stay on this computer, not on ${targetLabel}` : busy ? 'The chat is starting' : null })
  const { addPaths } = attach
  useEffect(() => {
    const element = box.current
    return element === null || remote ? undefined : registerComposerDrop(element, addPaths)
  }, [addPaths, remote])
  // Files meant for a chat that was never started are not kept for the next one.
  useEffect(() => () => {
    updateComposer(NEW_CHAT_DRAFT, (state) => { revokePreviews(state.attachments); return { ...state, attachments: [] } })
  }, [])
  // Text first, as in the terminal: a clipboard with any text pastes the text; only a bare image is attached.
  const onPaste = (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const pick = pickFromClipboard(event.clipboardData)
    if (pick.kind === 'none') pasteFromHost(attach.addImage)
    if (pick.kind === 'text' || pick.kind === 'none') return
    event.preventDefault()
    if (pick.kind === 'image') attach.addImage(pick.image)
    else toast.error(unsupportedMessage(pick.type))
  }
  const [query, setQuery] = useState('')
  // It opens ready to type, on the most recent project; the list of projects comes first only when there is none.
  const folder = picked ?? projects[0]?.folder ?? ''
  const shown = extra === null || projects.some((project) => project.folder === extra.folder) ? projects : [...projects, extra]
  const current = shown.find((project) => project.folder === folder)
  const folderName = current?.name ?? (folder === '' ? 'Choose a folder' : folder.split(/[\\/]/).filter(Boolean).at(-1) ?? folder)
  const installed = useInstalledAgents()
  const [branches, reloadBranches] = useFolderBranches(remote ? '' : folder)
  // A branch chosen for one folder means nothing in another.
  const explicit = branchPick?.folder === folder ? branchPick.pick : null
  const { state: sync, busy: pulling, pull } = useSyncState(remote ? '' : folder, true)
  // With Own copy on and no branch chosen, the chat gets a new branch under a suggested name, in a worktree.
  const suggested = useMemo(() => (treeMode && branches !== null ? suggestBranchName(new Set(branches.branches.map((entry) => entry.name))) : null), [treeMode, branches])
  const chosen = useMemo(() => explicit ?? (treeMode && suggested !== null ? { name: suggested, create: true, worktree: true } : null), [explicit, treeMode, suggested])
  const branch = chosen
  const isInstalled = (id: string) => (installed === null ? chatAgentOf(id) !== null : installed.includes(id))
  const agentMissing = !remote && !isInstalled(agent)

  // The message box takes the keys once there is a folder; before that the search field has them.
  useEffect(() => { if (folder !== '') text.current?.focus() }, [folder])

  const pickAgent = (next: ChatAgent) => {
    setAgent(next)
    // A mode is the agent's own too.
    setMode(START_MODE[next])
    // A model is the agent's own; the other agent's name means nothing to it.
    if (!modelsFor(next).some((option) => option.id === choice.model)) setChoice((now) => ({ ...now, model: DEFAULT_PICK }))
  }
  const modes = agent === 'claude' ? PICKABLE_MODES : agent === 'codex' ? CODEX_START_MODES : []
  // The Yolo setting already starts Codex with full access, and its flag cannot be combined with a mode's.
  const forcedFull = agent === 'codex' && codexYolo
  const startMode = !forcedFull && modes.includes(mode) ? mode : 'default'
  // A branch or a worktree is made when the chat starts, so only a chat on the folder as it is can have its agent started ahead.
  const warmSpec = useMemo<WarmSpec | null>(
    () => (remote || folder === '' || agentMissing || branch !== null ? null : { folder, agent, model: choice.model, reasoning: choice.reasoning, mode: startMode }),
    [remote, folder, agentMissing, branch, agent, choice.model, choice.reasoning, startMode],
  )
  useWarmAgent(onWarm, warmSpec, busy)
  const start = (task: string = message) => {
    if (busy || folder === '' || agentMissing) return
    setBusy(true)
    setError(null)
    onStart({ folder, agent, model: choice.model, reasoning: choice.reasoning, mode: startMode, message: buildAttachmentMessage(task.trim(), attachments), branch: remote ? null : branch, machine, machineName: targetLabel })
      .catch((failure: unknown) => { setError(errorMessage(failure, 'Could not start the chat')) })
      .finally(() => { setBusy(false); reloadBranches() })
  }
  const openFolder = (path: string) => {
    setExtra({ folder: path, name: lastPart(path), color: 'var(--dim)' })
    setFolder(path)
    setQuery('')
  }
  const pickOther = () => {
    if (browse === null) return
    void browse().then((chosen) => {
      if (chosen === null) return
      openFolder(chosen)
    })
  }
  const cloneOne = () => {
    if (clone === null) return
    void clone().then((cloned) => {
      if (cloned === null) return
      openFolder(cloned)
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

  const empty = message.trim() === '' && attachments.length === 0

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
  const folderSections = [{ heading: 'Recent projects', items: recent.map(folderItem) }, ...(rest.length > 0 ? [{ heading: `All on ${remote ? targetLabel : 'this computer'}`, items: rest.map(folderItem) }] : [])]
  const fromPath = (typed: string): ChipItem | null => {
    if (!isPath(typed)) return null
    return { key: 'path', label: `Use ${typed}`, icon: <Icon name="folder" size={12} />, checked: false, onPick: () => { openFolder(typed) } }
  }
  const elsewhere = machines.filter((candidate) => candidate.machine !== machine)
  const [now] = useState(nowSeconds)
  const [pulledNow, setPulledNow] = useState(false)
  // The other recent projects, a click away; each shows its branch and how far behind it is.
  const switchTo = shown.filter((project) => project.folder !== folder).slice(0, SWITCH_CHIPS)
  const glances = useGlances(remote ? [] : switchTo.map((project) => project.folder))
  const here = branch !== null && !branch.worktree ? branch.name : branches?.current ?? ''
  const inWorktree = new Set((branches?.worktrees ?? []).flatMap((tree) => (tree.branch === null ? [] : [tree.branch])))
  const branchLabel = branch?.name ?? branches?.current ?? 'detached'
  const pickBranch = (pick: BranchChoice) => { setBranchPick({ folder, pick }) }
  // Own copy off drops a worktree pick: the agent edits the checkout, on the branch the folder is on.
  const setOwnCopy = (on: boolean) => {
    setTreeMode(on)
    if (!on && branch?.worktree === true) setBranchPick(null)
    if (on) setBranchPick(null)
  }
  const newBranchRows: ChipItem[] = treeMode && branch?.worktree === true && branch.create
    ? [{ key: 'tree-new-branch', label: branch.name, sub: `A new branch from ${branches?.current ?? 'here'}, in its own worktree`, icon: <Icon name="plus" size={12} />, checked: true, onPick: () => undefined }]
    : []
  const worktreeRows: ChipItem[] = (branches?.worktrees ?? []).filter((tree) => !tree.main).map((tree) => {
    const name = tree.branch ?? lastPart(tree.path)
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
    checked: treeMode ? branch?.worktree === true && !branch.create && branch.name === entry.name : entry.name === here,
    onPick: () => { pickBranch({ name: entry.name, create: false, worktree: treeMode }) },
  }))
  const branchSections = [
    ...(newBranchRows.length > 0 ? [{ heading: 'New branch', items: newBranchRows }] : []),
    ...(worktreeRows.length > 0 ? [{ heading: 'Worktrees', items: worktreeRows }] : []),
    { heading: 'Branches', items: branchRows },
  ]
  const fromBranchName = (typed: string): ChipItem | null => {
    if (!/^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(typed) || (branches?.branches ?? []).some((entry) => entry.name === typed)) return null
    return { key: 'new-branch', label: `Create branch ${typed}`, icon: <Icon name="plus" size={12} />, checked: false, onPick: () => { pickBranch({ name: typed, create: true, worktree: treeMode }) } }
  }

  // Step 1: what the search field shows under it.
  const needle = query.trim().toLowerCase()
  const matching = shown.filter((project) => needle === '' || `${project.name} ${project.folder}`.toLowerCase().includes(needle))
  const typedPath = isPath(query.trim()) && !shown.some((project) => project.folder === query.trim()) ? query.trim() : null
  const listTitle = needle === '' ? 'Recent projects' : typedPath !== null && matching.length === 0 ? 'Open folder' : 'Matching projects'
  const pickFirst = () => {
    const first = matching[0]
    if (first !== undefined) setFolder(first.folder)
    else if (typedPath !== null) openFolder(typedPath)
  }
  const otherMachines = elsewhere.map((candidate) => (
    <button key={candidate.machine} type="button" className="mx-chip-action" onClick={() => { onOtherMachine(candidate.machine) }}><span className="mx-chip-dot" aria-hidden="true" />Start on {candidate.label}…</button>
  ))
  const addMachine = onAddMachine && <button type="button" className="mx-chip-action" onClick={onAddMachine}><Icon name="plus" size={12} />Add a machine…</button>

  return (
    <section className="mx-newchat" aria-label="New agent chat" onKeyDown={onKeyDown}>
      <button type="button" className="mx-icon-button mx-newchat-close" aria-label="Close" title="Close (Esc)" onClick={onCancel}><Icon name="close" size={13} /></button>
      <div className="mx-nc-top">
        <h2 className="mx-nc-title">What should we work on?</h2>
        {folder === '' ? (
          <div className="mx-nc-find">
            <label className="mx-nc-search">
              <Icon name="search" size={15} />
              <input autoFocus aria-label="Search projects or paste a path" placeholder="Search projects or paste a path…" value={query} onChange={(event) => { setQuery(event.target.value) }}
                onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); pickFirst() } }} />
              {browse !== null && <button type="button" className="mx-nc-browse" onClick={pickOther}>Browse…</button>}
              {clone !== null && <button type="button" className="mx-nc-browse" title="Bring a git repository onto this machine" onClick={cloneOne}>Clone a repository…</button>}
            </label>
            <h3 className="mx-nc-heading">{listTitle}</h3>
            <ul className="mx-nc-list" aria-label={listTitle}>
              {matching.map((project) => (
                <li key={project.folder}>
                  <button type="button" className="mx-nc-project" onClick={() => { setFolder(project.folder) }}>
                    <span className="mx-nc-folder" style={{ '--tone': project.color } as CSSProperties} aria-hidden="true"><Icon name="folder" size={15} /></span>
                    <span className="mx-nc-project-text">
                      <span className="mx-nc-project-name"><strong>{project.name}</strong><span>{project.folder}</span></span>
                      <span className="mx-nc-project-sub">{targetLabel}{project.sessions === undefined ? '' : ` · ${String(project.sessions)} ${project.sessions === 1 ? 'session' : 'sessions'}`}</span>
                    </span>
                  </button>
                </li>
              ))}
              {typedPath !== null && (
                <li><button type="button" className="mx-nc-project" onClick={() => { openFolder(typedPath) }}><span className="mx-nc-folder" aria-hidden="true"><Icon name="folder" size={15} /></span><span className="mx-nc-project-text"><span className="mx-nc-project-name"><strong>Open {lastPart(typedPath)}</strong><span>{typedPath}</span></span></span></button></li>
              )}
            </ul>
            {matching.length === 0 && typedPath === null && <p className="mx-nc-none">No projects match. {browse === null ? 'Paste a full path.' : 'Paste a full path, or use Browse…'}</p>}
            {(elsewhere.length > 0 || onAddMachine) && <div className="mx-nc-elsewhere">{otherMachines}{addMachine}</div>}
          </div>
        ) : (
          <>
            <div className="mx-nc-picks">
              <ChipMenu title="Project" side="bottom" label={<strong>{folderName}</strong>} summary={folderName} icon={<span className="mx-ns-square" style={{ background: current?.color ?? 'var(--dim)' }} aria-hidden="true" />} search="Search projects" width={300}
                sections={folderSections} fromQuery={fromPath}
                footer={<>{browse !== null && <button type="button" className="mx-chip-action" onClick={pickOther}><Icon name="folder" size={12} />Browse for a folder…</button>}{clone !== null && <button type="button" className="mx-chip-action" onClick={cloneOne}><Icon name="folder" size={12} />Clone a repository…</button>}{otherMachines}{addMachine}</>} />
              {branches !== null && (
                <>
                  <ChipMenu title="Branch" side="bottom" summary={branchLabel} label={<span className="mx-nc-branch">{branchLabel}</span>} icon={<span className="mx-nc-branch"><Icon name="branch" size={12} /></span>}
                    search="Search or create a branch" width={320} sections={branchSections} fromQuery={fromBranchName} />
                </>
              )}
              {branches !== null && (
                <label className="mx-own-copy" title="The agent works in a worktree of its own, so your checkout stays as it is.">
                  <Icon name="worktree" size={13} />
                  <span>Own copy</span>
                  <Switch checked={treeMode} onCheckedChange={setOwnCopy} aria-label="Own copy" />
                </label>
              )}
            </div>
            {sync !== null && !remote && (
              <p className="mx-nc-sync" aria-live="polite">
                {treeMode ? (
                  <>
                    <span className="mx-nc-dot" aria-hidden="true" />
                    {`Starts from ${sync.branch ?? 'here'}`}{sync.upstream !== null && sync.behind > 0 ? ` · ↓${String(sync.behind)} behind ${sync.upstream}` : ''}
                  </>
                ) : sync.upstream !== null && sync.behind > 0 ? (
                  <>
                    <span className="mx-nc-behind">↓{sync.behind} behind {sync.upstream}</span>
                    <button type="button" className="mx-nc-pull" disabled={sync.dirty || pulling} title={sync.dirty ? 'You have uncommitted changes. Commit or stash first.' : `Run git pull --ff-only`} onClick={() => { void pull().then(() => { setPulledNow(true) }).catch((failure: unknown) => { toast.error(errorMessage(failure, 'Could not pull')) }) }}>↓ Pull</button>
                  </>
                ) : sync.upstream !== null ? (
                  <><span className="mx-nc-dot" aria-hidden="true" />{pulledNow ? 'Pulled · up to date with' : 'Up to date with'} {sync.upstream}</>
                ) : null}
              </p>
            )}
            {switchTo.length > 0 && (
              <div className="mx-nc-switch" role="group" aria-label="Switch to">
                <span className="mx-nc-switch-label">Switch to</span>
                {switchTo.map((project) => {
                  const glance = glances.get(project.folder)
                  return (
                    <button key={project.folder} type="button" className="mx-nc-switch-chip" title={`${project.folder}${remote ? ` · ${targetLabel}` : ''}`} onClick={() => { setFolder(project.folder) }}>
                      <span className="mx-ns-square" style={{ background: project.color }} aria-hidden="true" />
                      <span>{project.name}</span>
                      {remote && <span className="mx-nc-switch-machine">· {targetLabel}</span>}
                      {glance?.branch != null && <span className="mx-nc-branch">{glance.branch}</span>}
                      {glance !== undefined && glance.behind > 0 && <span className="mx-nc-behind">↓{glance.behind}</span>}
                    </button>
                  )
                })}
              </div>
            )}
            <ul className="mx-nc-tasks" aria-label="Suggested tasks">
              {SUGGESTED_TASKS.map((task) => (
                <li key={task}><button type="button" disabled={busy || agentMissing} onClick={() => { start(task) }}><span aria-hidden="true">→</span>{task}</button></li>
              ))}
            </ul>
          </>
        )}
      </div>
      <div ref={box} className="mx-composer">
        <div className="mx-composer-column">
          <div className="mx-newchat-stack mx-field">
            <div className="mx-box">
              <DraftAttachments attachments={attachments} onRemove={attach.remove} />
              <textarea ref={text} onPaste={onPaste} aria-label="First message" rows={2} value={message} placeholder={folder === '' ? 'Pick a folder above, then describe a task…' : 'Describe a task, or pick one above…'} className="mx-box-text" onChange={(event) => { setMessage(event.target.value) }} />
              <div className="mx-box-row">
                {!remote && <button type="button" className="mx-box-attach" aria-label="Attach files" title="Attach files" onClick={attach.pick}><Icon name="plus" size={16} /></button>}
                <span className="mx-spacer" />
                <button type="button" className={`mx-send${busy || folder === '' || agentMissing ? ' is-off' : ''}`} aria-label={empty ? 'Start chat' : 'Send message'} title={folder === '' ? 'Pick a folder first' : empty ? 'Start an empty chat' : 'Send (Enter)'} disabled={busy || folder === '' || agentMissing} onClick={() => { start() }}>
                  <Icon name="arrowUp" size={15} strokeWidth={2.2} />
                </button>
              </div>
            </div>
            <div className="mx-ctxtray">
              <AgentPicker agentOnly={remote} agent={agent} choice={choice} onChange={(change) => { setChoice((was) => ({ ...was, ...change })) }} onAgent={pickAgent} />
              {forcedFull ? (
                <span className="mx-tray-fact is-danger" title="Yolo mode is on for Codex (Settings, General), so it starts with full access. Turn the setting off to pick another mode.">{modeLabel(CODEX_FULL_MODE)}</span>
              ) : modes.length > 0 ? (
                <PermissionMenu agent={AGENT_LABEL[agent]} label="Starting permission mode" mode={mode} modes={modes} onPick={setMode}
                  bypassOff={bypassAllowed ? undefined : { note: 'needs Yolo mode', why: 'Turn on Yolo mode for Claude in Settings, General, to start a chat in this mode.' }}>
                  <button type="button" className={`mx-tray-fact is-button${DANGER_MODES.includes(mode) ? ' is-danger' : ''}`} title="The permission mode this chat starts in" aria-label={`Starting permission mode: ${modeLabel(mode) ?? mode}. Pick another.`}>
                    {modeLabel(mode)}
                    <Icon name="chevronDown" size={10} />
                  </button>
                </PermissionMenu>
              ) : (
                <span className="mx-tray-fact" title="This chat asks before it runs a tool">Asks before running</span>
              )}
              <span className="mx-spacer" />
              {remote && <span className="mx-tray-fact" title={`Model and effort are the agent's own on ${targetLabel}`}>On {targetLabel}</span>}
              {folder !== '' && <span className="mx-tray-fact" title={folder}>{folder}</span>}
            </div>
          </div>
          {error !== null && <p role="alert" className="mx-composer-error">{error}</p>}
          {agentMissing && (
            <p role="alert" className="mx-composer-error">
              {`${AGENT_LABEL[agent]} is not installed on this computer. Pick another agent from the model button, or `}
              <button type="button" className="mx-text-button" onClick={() => { void openAgentDocs(agent).catch((failure: unknown) => { setError(errorMessage(failure, 'Could not open the setup page')) }) }}>set it up</button>.
            </p>
          )}
        </div>
      </div>
    </section>
  )
}
