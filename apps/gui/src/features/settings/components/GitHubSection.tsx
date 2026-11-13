import { useEffect, useState } from 'react'
import { loadGitHubAccounts, type GitHubReport } from '@/api/githubAccounts'
import { LOCAL_MACHINE } from '@/api/machines'
import { errorMessage } from '@/lib/error-message'
import type { MachineHead } from '@/features/workspace/model/machines'
import { BRANCH_PATH } from '@/features/workspace/components/paths'
import { Card, Row } from './DesktopSettings'
import { GitHubDetails } from './GitHubDetails'

/** How the page starts a sign-in, and what tells it one has ended. */
export type GitHubSignIn = {
  /** Opens GitHub's own sign-in in a terminal on that machine. */
  onSignIn: (machine: number, label: string) => void
  /** Goes up each time such a terminal ends: the page then asks every machine again. */
  signInsEnded: number
}

type SignInState = { status: 'loading' } | { status: 'ready'; report: GitHubReport } | { status: 'failed'; message: string }

const THIS_COMPUTER = 'This computer'
const LOADING: SignInState = { status: 'loading' }
/** Where GitHub gives out its command line. */
const GH_SITE = 'cli.github.com'

/** What a machine's row says about its GitHub sign-in. */
function signInWords(state: SignInState): string {
  if (state.status === 'loading') return 'Checking…'
  if (state.status === 'failed') return `Could not check: ${state.message}`
  const { installed, accounts, problem } = state.report
  if (!installed) return `The GitHub command line (gh) is not installed. Get it from ${GH_SITE}`
  if (problem !== null) return `Could not check: ${problem}`
  const [first, ...others] = accounts
  if (first === undefined) return 'Not signed in'
  const also = others.length === 0 ? '' : ` (also ${others.map((account) => account.login).join(', ')})`
  return `Signed in as ${first.login}${also}`
}

/** Whether the machine has `gh`, could be read, and has nobody signed in: the one case a sign-in is offered for. */
function canSignIn(state: SignInState): boolean {
  return state.status === 'ready' && state.report.installed && state.report.problem === null && state.report.accounts.length === 0
}

/** Whether a machine's row can be opened: it has `gh`, could be read, and somebody is signed in. */
function hasDetails(state: SignInState): state is { status: 'ready'; report: GitHubReport } {
  return state.status === 'ready' && state.report.installed && state.report.problem === null && state.report.accounts.length > 0
}

/** The machines that can be asked now: this computer, then every other machine whose line is up. */
function askable(machines: readonly MachineHead[]): Array<{ machine: number; label: string }> {
  const others = machines.filter((head) => head.machine !== LOCAL_MACHINE && head.chip?.tone === 'ok').map((head) => ({ machine: head.machine, label: head.label }))
  return [{ machine: LOCAL_MACHINE, label: THIS_COMPUTER }, ...others]
}

/**
 * Who is signed in to GitHub on this computer and on each connected machine, as each machine's own
 * `gh` says. A row with somebody signed in opens to every account there, the name and email its
 * commits carry and its folder rules. The window keeps no credentials and asks for none: it shows
 * logins only.
 */
export function GitHubSection({ machines, signIn }: { machines: readonly MachineHead[]; signIn?: GitHubSignIn | undefined }) {
  const rows = askable(machines)
  const asking = rows.map((row) => String(row.machine)).join(',')
  const [states, setStates] = useState<Readonly<Record<number, SignInState>>>({})
  const [asked, setAsked] = useState(0)
  const [open, setOpen] = useState<ReadonlySet<number>>(() => new Set())
  const toggle = (machine: number) => { setOpen((now) => { const next = new Set(now); if (!next.delete(machine)) next.add(machine); return next }) }
  const signInsEnded = signIn?.signInsEnded ?? 0
  useEffect(() => {
    let current = true
    for (const machine of asking.split(',').map(Number)) {
      loadGitHubAccounts(machine).then(
        (report) => { if (current) setStates((now) => ({ ...now, [machine]: { status: 'ready', report } })) },
        (error: unknown) => { if (current) setStates((now) => ({ ...now, [machine]: { status: 'failed', message: errorMessage(error, 'the machine did not answer') } })) },
      )
    }
    return () => { current = false }
  }, [asking, asked, signInsEnded])
  return (
    <Card title="Signed in" color="var(--ansi-blue)" iconPath={<path d={BRANCH_PATH} />}
      meta={<button type="button" className="mx-text-button" onClick={() => { setAsked((count) => count + 1) }}>Check again</button>}
      foot="Sign in opens GitHub's own sign-in in a terminal on that machine: finish it there and in your browser, then close the terminal. Maitty reads who is signed in and keeps no passwords or tokens."
    >
      {rows.map((row) => {
        const state = states[row.machine] ?? LOADING
        const shown = hasDetails(state) && open.has(row.machine)
        return (
          <div key={row.machine}>
            <Row label={row.label} detail={signInWords(state)}>
              {signIn !== undefined && canSignIn(state) && <button type="button" className="mx-scard-button" aria-label={`Sign in to GitHub on ${row.label}`} onClick={() => { signIn.onSignIn(row.machine, row.label) }}>Sign in</button>}
              {hasDetails(state) && <button type="button" className="mx-scard-button" aria-expanded={shown} aria-label={`Accounts and folder rules on ${row.label}`} onClick={() => { toggle(row.machine) }}>{shown ? 'Hide' : 'Details'}</button>}
            </Row>
            {shown && <GitHubDetails machine={row.machine} label={row.label} accounts={state.report.accounts} asked={asked + signInsEnded} onAddAccount={signIn === undefined ? undefined : () => { signIn.onSignIn(row.machine, row.label) }} />}
          </div>
        )
      })}
    </Card>
  )
}
