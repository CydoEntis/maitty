import { useCallback, useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { Folder, Laptop, MessageSquare, Server, SquareTerminal } from 'lucide-react'
import type { FolderListing } from '@/api/folders'
import { LOCAL_MACHINE } from '@/api/machines'
import { errorMessage } from '@/lib/error-message'
import { usePlatform } from '@/features/shortcuts/platform'
import type { ChatAgent, PaneView } from '../model/chatView'
import { DEFAULT_REMOTE_FOLDER, splitTyped, subfolderSuggestions, withoutTrailingSlash } from '../model/folderPath'
import {
  NEW_SESSION_STEPS, completionFor, effectiveStartView, folderName, isPathLike, matchingProjects, viewLockReason,
  type NewSessionMachine, type NewSessionStep, type StartAs,
} from '../model/newSession'
import './newSession.css'

/** How long typing pauses before the machine is asked about the folder. */
const ASK_AFTER_MS = 150
/** The most folders listed at once. */
const MOST_ROWS = 40

/** What the modal asks the window to start. */
export type NewSessionRequest = { machine: number; machineName: string; folder: string; startAs: StartAs; view: PaneView }

type Props = {
  machines: readonly NewSessionMachine[]
  /** The machine chosen when the modal opens. */
  initialMachine: number
  /** The folder chosen when the modal opens (the focused session's), or null. */
  initialFolder: string | null
  /** Folders sessions of a machine are in, most recent first. */
  projectsOf: (machine: number) => readonly string[]
  /** The folders inside a path on a machine. */
  list: (machine: number, path: string) => Promise<FolderListing>
  /** The system's folder picker; null when it is cancelled. Only this computer has one. */
  browse: (startAt?: string) => Promise<string | null>
  defaultAgent: ChatAgent
  defaultView: PaneView
  onStart: (request: NewSessionRequest) => Promise<void>
  onCancel: () => void
}

type Row = { kind: 'browse' } | { kind: 'folder'; path: string }
type Check = 'checking' | 'found' | 'missing' | 'unknown'

const AGENT_ROWS: ReadonlyArray<{ value: StartAs | 'crew'; label: string; glyph: string; hint: string }> = [
  { value: 'terminal', label: 'Terminal', glyph: '>_', hint: 'A plain shell' },
  { value: 'claude', label: 'Claude', glyph: '✻', hint: 'Starts claude in the shell' },
  { value: 'codex', label: 'Codex', glyph: '✻', hint: 'Starts codex in the shell' },
  { value: 'crew', label: 'Crew (soon)', glyph: '◇', hint: 'Several agents on one task' },
]

const STEP_LABEL: Record<NewSessionStep, string> = { project: 'Project', machine: 'Machine', agent: 'Start as' }

/**
 * The one way to start a session (⌘T): "Start in <project> on <machine> as <agent> in <view>". Each
 * part of the sentence is a pill that jumps to its step. The box keeps one height whatever the step,
 * so the list never makes the dialog jump. The window does the starting; this only asks.
 */
export function NewSessionModal(props: Props) {
  const { machines, projectsOf, list, browse, defaultAgent, defaultView, onStart, onCancel } = props
  const platform = usePlatform()
  const id = useId()
  const [step, setStep] = useState<NewSessionStep>('project')
  const [machine, setMachine] = useState(props.initialMachine)
  const [folder, setFolder] = useState(props.initialFolder ?? (props.initialMachine === LOCAL_MACHINE ? '' : DEFAULT_REMOTE_FOLDER))
  const [typed, setTyped] = useState('')
  // The row an arrow key chose; until one does, the step's own default row is the highlighted one.
  const [picked, setPicked] = useState<number | null>(null)
  const [startAs, setStartAs] = useState<StartAs>(defaultAgent)
  const [viewChoice, setViewChoice] = useState<PaneView>(defaultView)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [listings, setListings] = useState<ReadonlyMap<string, FolderListing | null>>(new Map())
  const root = useRef<HTMLDivElement>(null)
  const input = useRef<HTMLInputElement>(null)
  const listbox = useRef<HTMLDivElement>(null)
  const asked = useRef(new Set<string>())

  const isLocal = machine === LOCAL_MACHINE
  const machineName = machines.find((candidate) => candidate.id === machine)?.name ?? 'this computer'
  const projects = useMemo(() => {
    const known = [...projectsOf(machine)]
    if (props.initialFolder !== null && machine === props.initialMachine && !known.includes(props.initialFolder)) known.unshift(props.initialFolder)
    return known
  }, [projectsOf, machine, props.initialFolder, props.initialMachine])

  const key = (path: string) => `${String(machine)}:${path}`
  const typedPath = withoutTrailingSlash(typed)
  const pathMode = isPathLike(typed)
  const parentPath = withoutTrailingSlash(splitTyped(typed).folder)
  const parentListing = pathMode ? (listings.get(key(parentPath)) ?? null) : null
  const typedListing = pathMode ? listings.get(key(typedPath)) : undefined
  const check: Check = !pathMode ? 'found' : typedListing === undefined ? 'checking' : typedListing === null ? 'unknown' : typedListing.exists ? 'found' : 'missing'
  const missing = pathMode && check === 'missing'

  // Asks the chosen machine about the folder being typed and the one it is inside; an answer is kept per machine and path.
  useEffect(() => {
    if (!pathMode || step !== 'project') return undefined
    let current = true
    const timer = window.setTimeout(() => {
      for (const path of new Set([typedPath, parentPath])) {
        const where = `${String(machine)}:${path}`
        if (asked.current.has(where)) continue
        asked.current.add(where)
        list(machine, path).then(
          (listing) => { if (current) setListings((before) => new Map(before).set(where, listing)) },
          () => {
            asked.current.delete(where)
            if (current) setListings((before) => new Map(before).set(where, null))
          },
        )
      }
    }, ASK_AFTER_MS)
    return () => { current = false; window.clearTimeout(timer) }
  }, [pathMode, step, typedPath, parentPath, machine, list])

  const completion = step === 'project' ? completionFor(typed, projects, parentListing) : null

  const rows: Row[] = (() => {
    const folders = pathMode
      ? subfolderSuggestions(parentListing, splitTyped(typed).folder, splitTyped(typed).part).map((suggestion) => suggestion.folder)
      : matchingProjects(typed, projects)
    return [{ kind: 'browse' }, ...folders.slice(0, MOST_ROWS).map((path): Row => ({ kind: 'folder', path }))]
  })()

  // On opening, the current project is the highlighted row, so Enter takes it at once.
  const currentRow = typed === '' ? rows.findIndex((row) => row.kind === 'folder' && row.path === folder) : -1
  const defaultRow = step === 'project' ? (currentRow >= 0 ? currentRow : rows.length > 1 ? 1 : 0)
    : step === 'machine' ? Math.max(0, machines.findIndex((candidate) => candidate.id === machine))
      : Math.max(0, AGENT_ROWS.findIndex((row) => row.value === startAs))
  const highlight = picked ?? defaultRow
  const touched = picked !== null

  const previous = useRef<Element | null>(document.activeElement)
  useEffect(() => {
    const opener = previous.current
    return () => { if (opener instanceof HTMLElement) opener.focus() }
  }, [])
  useEffect(() => {
    if (step === 'project') input.current?.focus()
    else listbox.current?.focus()
  }, [step])

  const goTo = useCallback((next: NewSessionStep) => {
    setStep(next)
    setError(null)
    setPicked(null)
  }, [])

  const chooseFolder = (path: string) => {
    setFolder(path)
    setTyped('')
    goTo('machine')
  }
  const runBrowse = useCallback(() => {
    if (!isLocal) return
    void browse(folder === '' ? undefined : folder).then((picked) => { if (picked !== null) chooseFolder(picked) }, (failure: unknown) => { setError(errorMessage(failure, 'Could not open the folder picker')) })
    // chooseFolder only sets state and moves a step
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isLocal, browse, folder])

  const pickMachine = (target: NewSessionMachine) => {
    if (!target.online) return
    if (target.id !== machine) {
      setMachine(target.id)
      setFolder(projectsOf(target.id)[0] ?? (target.id === LOCAL_MACHINE ? '' : DEFAULT_REMOTE_FOLDER))
      setTyped('')
    }
    goTo('agent')
  }

  const start = (as: StartAs) => {
    if (busy || folder.trim() === '') return
    setBusy(true)
    setError(null)
    onStart({ machine, machineName, folder, startAs: as, view: effectiveStartView(viewChoice, as, machine) })
      .catch((failure: unknown) => { setError(errorMessage(failure, `Could not start a session on ${machineName}`)) })
      .finally(() => { setBusy(false) })
  }

  const enter = () => {
    if (step === 'project') {
      const row = rows[highlight]
      if (pathMode && !touched) {
        if (check === 'found' && typedPath !== '') chooseFolder(typedPath)
        return
      }
      if (row?.kind === 'browse') runBrowse()
      else if (row?.kind === 'folder') chooseFolder(row.path)
      else if (pathMode && check === 'found' && typedPath !== '') chooseFolder(typedPath)
    } else if (step === 'machine') {
      const target = machines[highlight]
      if (target) pickMachine(target)
    } else {
      const row = AGENT_ROWS[highlight]
      if (row && row.value !== 'crew') start(row.value)
    }
  }

  const count = step === 'project' ? rows.length : step === 'machine' ? machines.length : AGENT_ROWS.length
  const move = (delta: number) => {
    const target = Math.min(count - 1, Math.max(0, highlight + delta))
    setPicked(target)
    if (step === 'agent') {
      const next = AGENT_ROWS[target]
      if (next && next.value !== 'crew') setStartAs(next.value)
    }
  }

  const acceptCompletion = () => {
    if (!completion) return false
    setTyped(completion.folder)
    setPicked(null)
    return true
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const modifier = event.metaKey || event.ctrlKey
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      if (step === 'project') onCancel()
      else goTo(step === 'agent' ? 'machine' : 'project')
    } else if (modifier && event.key.toLowerCase() === 'o') {
      event.preventDefault()
      if (step === 'project') runBrowse()
    } else if (event.key === 'ArrowDown') {
      event.preventDefault()
      move(1)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      move(-1)
    } else if (event.key === 'Enter') {
      event.preventDefault()
      enter()
    } else if (event.key === 'ArrowRight' && step === 'project' && event.target === input.current && input.current.selectionStart === typed.length) {
      if (acceptCompletion()) event.preventDefault()
    } else if (event.key === 'Tab') {
      if (step === 'project' && !event.shiftKey && acceptCompletion()) {
        event.preventDefault()
        return
      }
      // The dialog traps focus: Tab cycles through its own controls.
      const focusable = Array.from(root.current?.querySelectorAll<HTMLElement>('button:not([disabled]), input, [tabindex="0"]') ?? [])
      if (focusable.length === 0) return
      const index = focusable.indexOf(document.activeElement as HTMLElement)
      const next = event.shiftKey ? (index <= 0 ? focusable.length - 1 : index - 1) : (index + 1) % focusable.length
      event.preventDefault()
      focusable[next]?.focus()
    }
  }

  const lock = viewLockReason(startAs, machine)
  const shownView = effectiveStartView(viewChoice, startAs, machine)
  const startAsLabel = AGENT_ROWS.find((row) => row.value === startAs)?.label ?? 'Terminal'
  const MachineIcon = isLocal ? Laptop : Server
  const optionId = (index: number) => `${id}-option-${String(index)}`

  return (
    <div className="mx-ns-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) onCancel() }}>
      <div
        ref={root}
        role="dialog"
        aria-modal="true"
        aria-label="New session"
        className="mx-ns"
        onKeyDown={onKeyDown}
      >
        <div className="mx-ns-sentence">
          <span>Start in</span>
          <button type="button" className={`mx-ns-pill${step === 'project' ? ' is-on' : ''}`} onClick={() => { goTo('project') }} aria-label={`Project: ${folder === '' ? 'none chosen' : folderName(folder)}. Change`}>
            <Folder aria-hidden="true" size={13} />
            <span className="mx-ns-pill-text">{folder === '' ? 'choose a project' : folderName(folder)}</span>
          </button>
          <span>on</span>
          <button type="button" className={`mx-ns-pill${step === 'machine' ? ' is-on' : ''}`} onClick={() => { goTo('machine') }} aria-label={`Machine: ${isLocal ? 'This computer' : machineName}. Change`}>
            <MachineIcon aria-hidden="true" size={13} />
            <span className="mx-ns-pill-text">{isLocal ? 'This computer' : machineName}</span>
          </button>
          <span>as</span>
          <button type="button" className={`mx-ns-pill${step === 'agent' ? ' is-on' : ''}`} onClick={() => { goTo('agent') }} aria-label={`Start as: ${startAsLabel}. Change`}>
            <span aria-hidden="true">{AGENT_ROWS.find((row) => row.value === startAs)?.glyph}</span>
            <span className="mx-ns-pill-text">{startAsLabel}</span>
          </button>
          <span>in</span>
          <div role="radiogroup" aria-label="View" className="mx-ns-toggle">
            <button type="button" role="radio" aria-checked={shownView === 'terminal'} aria-label="Terminal view" title={lock ?? 'Terminal view'} onClick={() => { setViewChoice('terminal') }}>
              <SquareTerminal aria-hidden="true" size={14} />
            </button>
            <button type="button" role="radio" aria-checked={shownView === 'chat'} aria-disabled={lock !== null} aria-label="Chat view" title={lock ?? 'Chat view'} className={lock !== null ? 'is-locked' : undefined} onClick={() => { if (lock === null) setViewChoice('chat') }}>
              <MessageSquare aria-hidden="true" size={14} />
            </button>
          </div>
        </div>

        <ol className="mx-ns-steps" aria-label="Steps">
          {NEW_SESSION_STEPS.map((name, index) => (
            <li key={name} aria-current={name === step ? 'step' : undefined} className={name === step ? 'is-on' : undefined}>{String(index + 1)}. {STEP_LABEL[name]}</li>
          ))}
        </ol>

        {step === 'project' && (
          <div className={`mx-ns-search${missing ? ' is-missing' : ''}`}>
            <div className="mx-ns-field">
              <span className="mx-ns-ghost" aria-hidden="true"><span className="mx-ns-typed">{typed}</span>{completion?.rest}</span>
              <input
                ref={input}
                role="combobox"
                aria-expanded="true"
                aria-controls={`${id}-list`}
                aria-activedescendant={rows[highlight] ? optionId(highlight) : undefined}
                aria-label="Find a project or type a path"
                aria-invalid={missing}
                aria-describedby={`${id}-note`}
                placeholder="Find a project or type a path…"
                value={typed}
                spellCheck={false}
                autoComplete="off"
                onChange={(event) => { setTyped(event.target.value); setPicked(null); setError(null) }}
              />
              {completion && <kbd className="mx-ns-hint">Tab complete</kbd>}
            </div>
            <p id={`${id}-note`} className="mx-ns-note" role="status">
              {missing ? <span className="mx-ns-bad">No such folder on {isLocal ? 'This computer' : machineName}</span> : pathMode && check === 'unknown' ? `${machineName} did not answer` : ''}
            </p>
          </div>
        )}

        <div
          ref={listbox}
          id={`${id}-list`}
          role="listbox"
          aria-label={STEP_LABEL[step]}
          aria-activedescendant={step !== 'project' ? optionId(highlight) : undefined}
          tabIndex={step === 'project' ? -1 : 0}
          className="mx-ns-list"
        >
          {step === 'project' && rows.map((row, index) => (
            row.kind === 'browse' ? (
              <div key="browse">
                <div
                  id={optionId(index)}
                  role="option"
                  aria-selected={index === highlight}
                  aria-disabled={!isLocal}
                  title={isLocal ? undefined : 'The folder picker shows this computer; type a path on this machine'}
                  className={`mx-ns-row${index === highlight ? ' is-on' : ''}${isLocal ? '' : ' is-off'}`}
                  onMouseDown={(event) => { event.preventDefault(); runBrowse() }}
                >
                  <Folder aria-hidden="true" size={15} />
                  <span>Browse…</span>
                  <kbd>{platform === 'mac' ? '⌘O' : 'Ctrl+O'}</kbd>
                </div>
                <hr className="mx-ns-divider" />
              </div>
            ) : (
              <div
                key={row.path}
                id={optionId(index)}
                role="option"
                aria-selected={index === highlight}
                className={`mx-ns-row${index === highlight ? ' is-on' : ''}`}
                title={row.path}
                onMouseDown={(event) => { event.preventDefault(); chooseFolder(row.path) }}
              >
                <Folder aria-hidden="true" size={15} />
                <span>{folderName(row.path)}</span>
                <small>{row.path}</small>
              </div>
            )
          ))}
          {step === 'project' && rows.length === 1 && !pathMode && typed.trim() !== '' && <p className="mx-ns-empty">No project matches. Type a path starting with ~ or /.</p>}
          {step === 'machine' && machines.map((target, index) => (
            <div
              key={target.id}
              id={optionId(index)}
              role="option"
              aria-selected={index === highlight}
              aria-disabled={!target.online}
              className={`mx-ns-row${index === highlight ? ' is-on' : ''}${target.online ? '' : ' is-off'}`}
              onMouseDown={(event) => { event.preventDefault(); pickMachine(target) }}
            >
              {target.id === LOCAL_MACHINE ? <Laptop aria-hidden="true" size={15} /> : <Server aria-hidden="true" size={15} />}
              <span>{target.id === LOCAL_MACHINE ? 'This computer' : target.name}</span>
              <small>{target.online ? target.note : 'Reconnecting…'}</small>
            </div>
          ))}
          {step === 'agent' && AGENT_ROWS.map((row, index) => {
            const off = row.value === 'crew'
            return (
              <div
                key={row.value}
                id={optionId(index)}
                role="option"
                aria-selected={index === highlight}
                aria-disabled={off}
                className={`mx-ns-row${index === highlight ? ' is-on' : ''}${off ? ' is-off' : ''}`}
                onMouseDown={(event) => { event.preventDefault(); if (!off) { setStartAs(row.value as StartAs); start(row.value as StartAs) } }}
              >
                <span aria-hidden="true" className="mx-ns-glyph">{row.glyph}</span>
                <span>{row.label}</span>
                <small>{row.value === defaultAgent ? 'Default' : row.hint}</small>
              </div>
            )
          })}
        </div>

        {error !== null && <p role="alert" className="mx-ns-bad mx-ns-error">{error}</p>}
        <div className="mx-ns-foot">
          <span>↑↓ move</span><span>↵ {step === 'agent' ? 'start' : 'next'}</span>
          <span>Esc {step === 'project' ? 'cancel' : 'back'}</span>
          <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
        </div>
      </div>
    </div>
  )
}
