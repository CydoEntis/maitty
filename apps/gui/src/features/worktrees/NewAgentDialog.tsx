import { useState, type KeyboardEvent } from 'react'
import { RadioGroup } from 'radix-ui'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Textarea } from '@/components/ui/textarea'
import { errorMessage } from '@/lib/error-message'
import { Icon } from '@/features/workspace/components/glyphs'
import { Keycaps } from '@/features/shortcuts/Keycaps'
import { usePlatform } from '@/features/shortcuts/platform'
import { agentBranchProblem, BRANCH_AGENTS, defaultProject, promptProblem, suggestedBranch, worktreeFolder, type AgentProject, type BranchAgent } from './newAgent'

export type NewAgentRequest = { project: AgentProject; agent: BranchAgent; prompt: string; branch: string }

type Props = {
  projects: readonly AgentProject[]
  /** The sidebar group the dialog opens on (the focused session's, or the one right-clicked). */
  initialKey: string | null
  home: string
  /** The themed window root, so the dialog reads the theme's tokens. */
  container: HTMLElement | null
  onStart: (request: NewAgentRequest) => Promise<void>
  onCancel: () => void
}

const chipClass = 'inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-[var(--line2)] bg-transparent px-2.5 py-1 text-[12.5px] text-[var(--fg2)] outline-none hover:text-[var(--fg)] focus-visible:ring-2 focus-visible:ring-[var(--accent)] data-[state=checked]:border-[var(--accent)] data-[state=checked]:bg-[var(--accent-soft)] data-[state=checked]:text-[var(--fg)]'
const fieldClass = 'w-full rounded-lg border border-[var(--line2)] bg-[var(--well)] px-2.5 py-2 text-[13px] text-[var(--fg)] outline-none placeholder:text-[var(--dim)] focus-visible:border-[var(--accent)] aria-[invalid=true]:border-[var(--danger)] aria-[invalid=true]:ring-2 aria-[invalid=true]:ring-[var(--danger-line)]'
const labelClass = 'text-[11px] font-semibold tracking-wide text-[var(--muted-text)] uppercase'

/**
 * New agent on a branch (v18 C.4): a project, an agent and a first prompt become a branch in its own
 * worktree with the agent started in it. The branch follows the prompt until it is edited by hand.
 */
export function NewAgentDialog({ projects, initialKey, home, container, onStart, onCancel }: Props) {
  const platform = usePlatform()
  const [projectKey, setProjectKey] = useState(() => defaultProject(projects, initialKey)?.key ?? '')
  const [agent, setAgent] = useState<BranchAgent>('claude')
  const [prompt, setPrompt] = useState('')
  // Null while the branch is the one the prompt suggests; the user's own text once they edit it.
  const [typedBranch, setTypedBranch] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const project = projects.find((candidate) => candidate.key === projectKey)
  const branch = (typedBranch ?? suggestedBranch(prompt)).trim()
  const branchProblem = branch === '' ? null : agentBranchProblem(branch)
  const promptIssue = promptProblem(prompt)
  const ready = project !== undefined && branch !== '' && branchProblem === null && promptIssue === null && !busy

  const start = () => {
    if (!ready) return
    setBusy(true)
    setError(null)
    onStart({ project, agent, prompt: prompt.trim(), branch })
      .catch((failure: unknown) => { setError(errorMessage(failure, 'Could not start the agent')) })
      .finally(() => { setBusy(false) })
  }

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== 'Enter' || !(event.ctrlKey || event.metaKey)) return
    event.preventDefault()
    start()
  }
  const startChord = platform === 'mac' ? ['⌘', '↵'] : ['Ctrl', 'Enter']

  return (
    <Dialog open onOpenChange={(open) => { if (!open) onCancel() }}>
      <DialogContent container={container} showCloseButton={false} className="sm:max-w-[500px] gap-3.5 bg-[var(--raise)] text-[var(--fg)]" onKeyDown={onKeyDown}>
        <DialogHeader>
          <DialogTitle>New agent on a branch</DialogTitle>
          <DialogDescription className="text-[var(--muted-text)]">A branch in its own folder, with the agent started in it. The other sessions keep working undisturbed.</DialogDescription>
        </DialogHeader>
        <form className="grid gap-3.5" onSubmit={(event) => { event.preventDefault(); start() }}>
          <div className="grid gap-1.5">
            <span id="new-agent-project" className={labelClass}>Project</span>
            <RadioGroup.Root aria-labelledby="new-agent-project" value={projectKey} onValueChange={setProjectKey} className="flex flex-wrap gap-1.5">
              {projects.map((candidate) => (
                <RadioGroup.Item key={candidate.key} value={candidate.key} className={chipClass}>
                  <Icon name="folder" size={13} />{candidate.name}
                </RadioGroup.Item>
              ))}
            </RadioGroup.Root>
          </div>
          <label className="grid gap-1.5">
            <span className={labelClass}>First prompt <span className="font-normal normal-case">(optional)</span></span>
            <Textarea
              aria-label="First prompt"
              autoFocus
              rows={3}
              value={prompt}
              placeholder="What should the agent do?"
              aria-invalid={promptIssue !== null}
              className={`${fieldClass} min-h-[84px] resize-y`}
              onChange={(event) => { setPrompt(event.target.value); setError(null) }}
            />
            {promptIssue !== null && <span role="alert" className="text-[12px] text-[var(--danger)]">{promptIssue}</span>}
          </label>
          <label className="grid gap-1.5">
            <span className={labelClass}>Branch</span>
            <input
              aria-label="Branch"
              value={typedBranch ?? suggestedBranch(prompt)}
              placeholder="agent/<slug>"
              spellCheck={false}
              autoCapitalize="off"
              autoCorrect="off"
              aria-invalid={branchProblem !== null}
              className={`${fieldClass} font-mono text-[12.5px]`}
              onChange={(event) => { setTypedBranch(event.target.value); setError(null) }}
            />
            {branchProblem !== null && <span role="alert" className="text-[12px] text-[var(--danger)]">{branchProblem}</span>}
          </label>
          <div className="grid gap-1.5">
            <span id="new-agent-agent" className={labelClass}>Agent</span>
            <RadioGroup.Root aria-labelledby="new-agent-agent" value={agent} onValueChange={(value) => { setAgent(value as BranchAgent) }} className="flex gap-1.5">
              {BRANCH_AGENTS.map((choice) => (
                <RadioGroup.Item key={choice.id} value={choice.id} className={chipClass}>{choice.label}</RadioGroup.Item>
              ))}
            </RadioGroup.Root>
          </div>
          <p className="m-0 text-[12px] text-[var(--muted-text)]" aria-live="polite">
            Folder <span className="font-mono text-[var(--fg2)]">{project && branch !== '' ? worktreeFolder(project.path, branch, home) : '—'}</span>
          </p>
          {error !== null && <p role="alert" className="m-0 text-[12.5px] text-[var(--danger)]">{error}</p>}
          <div className="flex items-center justify-end gap-2">
            <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
            <button type="submit" className="mx-text-button is-primary inline-flex items-center gap-2" disabled={!ready}>
              {busy ? 'Starting…' : 'Start'}
              {!busy && <Keycaps chord={startChord} />}
            </button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
