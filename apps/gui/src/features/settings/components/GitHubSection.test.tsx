import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { GitHubReport } from '@/api/githubAccounts'
import { LOCAL_HEAD, type MachineHead } from '@/features/workspace/model/machines'
import { GitHubSection } from './GitHubSection'

const api = vi.hoisted(() => ({ loadGitHubAccounts: vi.fn<(machine: number) => Promise<GitHubReport>>() }))
vi.mock('@/api/githubAccounts', () => api)

const KINGDOM: MachineHead = { ...LOCAL_HEAD, key: 'machine-3', machine: 3, name: 'kingdom', label: 'kingdom', chip: { glyph: '●', word: 'connected', tone: 'ok' } }
const BARE: MachineHead = { ...KINGDOM, key: 'machine-5', machine: 5, name: 'bare', label: 'bare' }
const AWAY: MachineHead = { ...LOCAL_HEAD, key: 'machine-4', machine: 4, name: 'away', label: 'away', chip: { glyph: '✕', word: 'unreachable', tone: 'bad' }, down: true }

const signedIn = (...logins: string[]): GitHubReport => ({ installed: true, accounts: logins.map((login, at) => ({ login, active: at === 0 })), problem: null })
const NOBODY: GitHubReport = { installed: true, accounts: [], problem: null }
const NO_GH: GitHubReport = { installed: false, accounts: [], problem: null }

/** Each machine's answer by its number; a machine with none never answers. */
function answers(by: Record<number, GitHubReport | Error>) {
  api.loadGitHubAccounts.mockImplementation((machine) => {
    const answer = by[machine]
    if (answer === undefined) return new Promise<GitHubReport>(() => undefined)
    return answer instanceof Error ? Promise.reject(answer) : Promise.resolve(answer)
  })
}

afterEach(cleanup)
beforeEach(() => { api.loadGitHubAccounts.mockReset() })

describe('the GitHub page of Settings', () => {
  it('says who this computer and each connected machine is signed in as', async () => {
    answers({ 0: signedIn('me', 'work-me'), 3: signedIn('kingdom-me') })
    render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM, AWAY]} />)
    expect(await screen.findByText('Signed in as me (also work-me)')).toBeInTheDocument()
    expect(await screen.findByText('Signed in as kingdom-me')).toBeInTheDocument()
    expect(screen.getByText('This computer')).toBeInTheDocument()
    expect(screen.getByText('kingdom')).toBeInTheDocument()
  })

  it('leaves out a machine that is not connected, and does not ask it', async () => {
    answers({ 0: NOBODY, 3: NOBODY })
    render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM, AWAY]} />)
    await screen.findAllByText('Not signed in')
    expect(screen.queryByText('away')).toBeNull()
    expect(api.loadGitHubAccounts.mock.calls.map(([machine]) => machine)).toEqual([0, 3])
  })

  it('says a machine is not signed in, and that one has no gh with where to get it', async () => {
    answers({ 0: NOBODY, 3: NO_GH })
    render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM]} />)
    expect(await screen.findByText('Not signed in')).toBeInTheDocument()
    expect(await screen.findByText('The GitHub command line (gh) is not installed. Get it from cli.github.com')).toBeInTheDocument()
  })

  it('says it is checking until a machine answers', () => {
    answers({})
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    expect(screen.getByText('Checking…')).toBeInTheDocument()
  })

  it('shows why a machine could not be read while the other rows still show', async () => {
    answers({ 0: signedIn('me'), 3: new Error('The machine did not answer in time') })
    render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM]} />)
    expect(await screen.findByText('Could not check: The machine did not answer in time')).toBeInTheDocument()
    expect(screen.getByText('Signed in as me')).toBeInTheDocument()
  })

  it('shows the line gh gave when it could not say who is signed in', async () => {
    answers({ 0: { installed: true, accounts: [], problem: 'unknown flag: --json' } })
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    expect(await screen.findByText('Could not check: unknown flag: --json')).toBeInTheDocument()
  })

  it('shows a login from another machine as plain text, never as markup', async () => {
    answers({ 0: NOBODY, 3: signedIn('<img src=x onerror=alert(1)>') })
    const { container } = render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM]} />)
    expect(await screen.findByText('Signed in as <img src=x onerror=alert(1)>')).toBeInTheDocument()
    expect(container.querySelector('img')).toBeNull()
  })

  it('offers Sign in only where gh is installed and nobody is signed in, and starts it on that machine', async () => {
    const user = userEvent.setup()
    const onSignIn = vi.fn()
    answers({ 0: signedIn('me'), 3: NOBODY, 5: NO_GH })
    render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM, BARE]} signIn={{ onSignIn, signInsEnded: 0 }} />)
    await user.click(await screen.findByRole('button', { name: 'Sign in to GitHub on kingdom' }))
    expect(onSignIn).toHaveBeenCalledExactlyOnceWith(3, 'kingdom')
    expect(screen.getAllByRole('button', { name: /^Sign in to GitHub/ })).toHaveLength(1)
  })

  it('offers no Sign in where the window cannot start one', async () => {
    answers({ 0: NOBODY })
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    await screen.findByText('Not signed in')
    expect(screen.queryByRole('button', { name: /^Sign in to GitHub/ })).toBeNull()
  })

  it('asks again when a sign-in terminal has ended, so the row changes by itself', async () => {
    const onSignIn = vi.fn()
    answers({ 0: NOBODY })
    const view = render(<GitHubSection machines={[LOCAL_HEAD]} signIn={{ onSignIn, signInsEnded: 0 }} />)
    await screen.findByText('Not signed in')
    answers({ 0: signedIn('me') })
    view.rerender(<GitHubSection machines={[LOCAL_HEAD]} signIn={{ onSignIn, signInsEnded: 1 }} />)
    expect(await screen.findByText('Signed in as me')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Sign in to GitHub/ })).toBeNull()
  })

  it('asks every machine again on Check again', async () => {
    const user = userEvent.setup()
    answers({ 0: NOBODY })
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    await screen.findByText('Not signed in')
    answers({ 0: signedIn('me') })
    await user.click(screen.getByRole('button', { name: 'Check again' }))
    expect(await screen.findByText('Signed in as me')).toBeInTheDocument()
  })
})
