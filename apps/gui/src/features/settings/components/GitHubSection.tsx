import { useEffect, useState } from 'react'
import { loadGitHubAccounts, type GitHubReport } from '@/api/githubAccounts'
import { LOCAL_MACHINE } from '@/api/machines'
import { errorMessage } from '@/lib/error-message'
import type { MachineHead } from '@/features/workspace/model/machines'
import { BRANCH_PATH } from '@/features/workspace/components/paths'
import { Card, Row } from './DesktopSettings'

type SignInState = { status: 'loading' } | { status: 'ready'; report: GitHubReport } | { status: 'failed'; message: string }

const THIS_COMPUTER = 'This computer'
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

/** The machines that can be asked now: this computer, then every other machine whose line is up. */
function askable(machines: readonly MachineHead[]): Array<{ machine: number; label: string }> {
  const others = machines.filter((head) => head.machine !== LOCAL_MACHINE && head.chip?.tone === 'ok').map((head) => ({ machine: head.machine, label: head.label }))
  return [{ machine: LOCAL_MACHINE, label: THIS_COMPUTER }, ...others]
}

/**
 * Who is signed in to GitHub on this computer and on each connected machine, as each machine's own
 * `gh` says. The window keeps no credentials and asks for none: it shows logins only.
 */
export function GitHubSection({ machines }: { machines: readonly MachineHead[] }) {
  const rows = askable(machines)
  const asking = rows.map((row) => String(row.machine)).join(',')
  const [states, setStates] = useState<Readonly<Record<number, SignInState>>>({})
  const [asked, setAsked] = useState(0)
  useEffect(() => {
    let current = true
    for (const machine of asking.split(',').map(Number)) {
      loadGitHubAccounts(machine).then(
        (report) => { if (current) setStates((now) => ({ ...now, [machine]: { status: 'ready', report } })) },
        (error: unknown) => { if (current) setStates((now) => ({ ...now, [machine]: { status: 'failed', message: errorMessage(error, 'the machine did not answer') } })) },
      )
    }
    return () => { current = false }
  }, [asking, asked])
  return (
    <Card title="Signed in" color="var(--ansi-blue)" iconPath={<path d={BRANCH_PATH} />}
      meta={<button type="button" className="mx-text-button" onClick={() => { setAsked((count) => count + 1) }}>Check again</button>}
      foot="Each machine signs in to GitHub by itself, with its own GitHub command line. Maitty reads who is signed in and keeps no passwords or tokens."
    >
      {rows.map((row) => <Row key={row.machine} label={row.label} detail={signInWords(states[row.machine] ?? { status: 'loading' })}>{null}</Row>)}
    </Card>
  )
}
