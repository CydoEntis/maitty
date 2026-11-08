import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { useState, type KeyboardEvent } from 'react'
import { Dialog as DialogPrimitive } from 'radix-ui'
import { LOCAL_MACHINE } from '@/api/machines'
import { errorMessage } from '@/lib/error-message'
import { CHAT_AGENTS, type ChatAgent } from '../model/chatView'
import type { NewSessionProject } from '../model/newSession'
import { AppIcon, Icon } from './glyphs'

/** What the dialog asks the window to start: a shell, or an agent typed into one, with its first message. */
export type NewSessionRequest = { machine: number; machineName: string; folder: string; startAs: 'terminal' | ChatAgent; message: string }

/** The dialog itself takes the focus as it opens, so no control shows a focus ring nobody asked for. */
const focusDialog = (event: Event) => {
  event.preventDefault()
  if (event.currentTarget instanceof HTMLElement) event.currentTarget.focus()
}

type Kind = 'agent' | 'shell'

type Props = {
  machine: number
  /** The machine's name, said in the title when it is not this computer. */
  machineName: string | null
  /** The projects the machine has sessions in, the focused one first. */
  projects: readonly NewSessionProject[]
  defaultAgent: ChatAgent
  /** The system's folder picker; null when it is cancelled. Only this computer has one. */
  browse: (() => Promise<string | null>) | null
  onStart: (request: NewSessionRequest) => Promise<void>
  onCancel: () => void
  /** Where the dialog mounts, so it reads the theme's colours. */
  portalContainer: HTMLElement | null
}

const AGENT_TITLE: Readonly<Record<ChatAgent, string>> = { claude: 'Claude Code', codex: 'Codex', gemini: 'Gemini CLI' }

/** A row of options in the sunken segmented tray the dialog uses for its choices. */
function Segments<T extends string>({ label, value, options, onPick }: { label: string; value: T; options: ReadonlyArray<{ value: T; label: string; disabled?: string }>; onPick: (value: T) => void }) {
  return (
    <RadioGroup aria-label={label} orientation="horizontal" value={value} onValueChange={(next) => { const picked = options.find((option) => option.value === next); if (picked) onPick(picked.value) }} className="mx-ns-segments">
      {options.map((option) => (
        <RadioGroupItem key={option.value} value={option.value} disabled={option.disabled !== undefined} title={option.disabled} className="mx-ns-segment">{option.label}</RadioGroupItem>
      ))}
    </RadioGroup>
  )
}

/**
 * New session (README: New session dialog): an agent chat or a shell, which agent, which project and
 * where it works, and an optional first message held until the agent can take it. The window does the
 * starting; this only asks. Ctrl+Enter starts, Esc cancels.
 */
export function NewSessionModal({ machine, machineName, projects, defaultAgent, browse, onStart, onCancel, portalContainer }: Props) {
  const [kind, setKind] = useState<Kind>('agent')
  const [agent, setAgent] = useState<ChatAgent>(defaultAgent)
  const [picked, setFolder] = useState<string | null>(null)
  // Until one is picked, the first project is the choice, even when the list arrives after the dialog opened.
  const folder = picked ?? projects[0]?.folder ?? ''
  const [extra, setExtra] = useState<NewSessionProject | null>(null)
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const shown = extra === null || projects.some((project) => project.folder === extra.folder) ? projects : [...projects, extra]

  const start = () => {
    if (busy || folder === '') return
    setBusy(true)
    setError(null)
    onStart({ machine, machineName: machineName ?? 'this computer', folder, startAs: kind === 'shell' ? 'terminal' : agent, message: kind === 'agent' ? message.trim() : '' })
      .catch((failure: unknown) => { setError(errorMessage(failure, 'Could not start the session')) })
      .finally(() => { setBusy(false) })
  }
  const pickOther = () => {
    if (browse === null) return
    void browse().then((chosen) => {
      if (chosen === null) return
      const name = chosen.split(/[\\/]/).filter(Boolean).at(-1) ?? chosen
      setExtra({ folder: chosen, name, color: 'var(--dim)' })
      setFolder(chosen)
    })
  }
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault()
      start()
    }
  }

  return (
    <DialogPrimitive.Root open onOpenChange={(open) => { if (!open) onCancel() }}>
      <DialogPrimitive.Portal container={portalContainer}>
        <DialogPrimitive.Overlay className="mx-dialog-scrim" />
        <DialogPrimitive.Content className="mx-ns-dialog" aria-describedby={undefined} tabIndex={-1} onOpenAutoFocus={focusDialog} onKeyDown={onKeyDown}>
          <header className="mx-dialog-head">
            <DialogPrimitive.Title className="mx-dialog-title">{machine === LOCAL_MACHINE || machineName === null ? 'New session' : `New session on ${machineName}`}</DialogPrimitive.Title>
            <DialogPrimitive.Close className="mx-dialog-close" aria-label="Close"><Icon name="close" size={13} /></DialogPrimitive.Close>
          </header>
          <div className="mx-ns-body">
            <Segments label="Kind" value={kind} options={[{ value: 'agent', label: 'Agent chat' }, { value: 'shell', label: 'Shell' }]} onPick={setKind} />
            {kind === 'agent' && (
              <div className="mx-ns-field">
                <span className="mx-ns-label">AGENT</span>
                <RadioGroup aria-label="Agent" value={agent} onValueChange={(next) => { const picked = CHAT_AGENTS.find((candidate) => candidate === next); if (picked) setAgent(picked) }} className="mx-ns-agents">
                  {CHAT_AGENTS.map((candidate) => (
                    <RadioGroupItem key={candidate} value={candidate} className="mx-ns-agent">
                      <AppIcon process={candidate} size={26} />
                      <span className="mx-ns-agent-text"><strong>{AGENT_TITLE[candidate]}</strong><small>{candidate === defaultAgent ? 'Your default agent' : 'Its own default model'}</small></span>
                    </RadioGroupItem>
                  ))}
                </RadioGroup>
              </div>
            )}
            <div className="mx-ns-field">
              <span className="mx-ns-label">PROJECT</span>
              <RadioGroup aria-label="Project" value={folder} onValueChange={setFolder} className="mx-ns-projects">
                {shown.map((project) => (
                  <RadioGroupItem key={project.folder} value={project.folder} title={project.folder} className="mx-ns-project">
                    <span className="mx-ns-square" style={{ background: project.color }} aria-hidden="true" />{project.name}
                  </RadioGroupItem>
                ))}
                {browse !== null && <button type="button" className="mx-ns-project is-other" onClick={pickOther}><Icon name="folder" size={12} />Other folder…</button>}
              </RadioGroup>
            </div>
            {kind === 'agent' && (
              <>
                <div className="mx-ns-field">
                  <span className="mx-ns-label">WHERE IT WORKS</span>
                  <Segments
                    label="Where it works"
                    value="worktree"
                    options={[
                      { value: 'worktree', label: 'New worktree' },
                      { value: 'current', label: 'Current branch', disabled: 'Maitty starts every agent typed in a project in a worktree of its own (worktree.per_agent in config.toml).' },
                    ]}
                    onPick={() => undefined}
                  />
                  <span className="mx-ns-note">A new branch and folder, so it can’t touch your other work.</span>
                </div>
                <label className="mx-ns-field">
                  <span className="mx-ns-label">FIRST MESSAGE</span>
                  <textarea aria-label="First message" rows={3} value={message} placeholder="What should it do? You can also start empty." className="mx-ns-message" onChange={(event) => { setMessage(event.target.value) }} />
                </label>
              </>
            )}
            {error !== null && <p role="alert" className="mx-ns-error">{error}</p>}
          </div>
          <footer className="mx-dialog-foot">
            <DialogPrimitive.Close className="mx-ns-cancel">Cancel</DialogPrimitive.Close>
            <button type="button" className="mx-ns-start" disabled={busy || folder === ''} onClick={start}>{kind === 'agent' ? 'Start agent' : 'Open shell'}<kbd>Ctrl ↵</kbd></button>
          </footer>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}
