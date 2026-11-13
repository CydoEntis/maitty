import { cleanup, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { GitHubReport } from '@/api/githubAccounts'
import type { GitConfigWrite, GitIdentityReport, GitRuleChange } from '@/api/gitIdentities'
import { LOCAL_HEAD, type MachineHead } from '@/features/workspace/model/machines'
import { GitHubSection } from './GitHubSection'

const api = vi.hoisted(() => ({ loadGitHubAccounts: vi.fn<(machine: number) => Promise<GitHubReport>>() }))
vi.mock('@/api/githubAccounts', () => api)
const git = vi.hoisted(() => ({
  loadGitIdentities: vi.fn<(machine: number) => Promise<GitIdentityReport>>(),
  changeGitRule: vi.fn<(machine: number, change: GitRuleChange) => Promise<GitConfigWrite[]>>(),
}))
vi.mock('@/api/gitIdentities', () => git)

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
beforeEach(() => {
  api.loadGitHubAccounts.mockReset()
  git.loadGitIdentities.mockReset()
  git.changeGitRule.mockReset()
})

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

  const WHO: GitIdentityReport = { installed: true, name: 'Casey Example', email: 'casey@example.invalid', rules: [], problem: null }
  const openRow = async (user: ReturnType<typeof userEvent.setup>, label: string) => {
    await user.click(await screen.findByRole('button', { name: `Accounts and folder rules on ${label}` }))
    return within(await screen.findByRole('group', { name: `GitHub details for ${label}` }))
  }

  it('opens a machine\'s row to its accounts, who commits there and its folder rules', async () => {
    const user = userEvent.setup()
    answers({ 0: signedIn('me'), 3: signedIn('kingdom-me', 'work-me') })
    git.loadGitIdentities.mockResolvedValue({
      ...WHO,
      rules: [
        { folder: '/home/dev/work', identity: { name: 'Casey At Work', email: 'casey@work.invalid', login: 'work-me' } },
        { folder: '~/oss', identity: null },
      ],
    })
    render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM]} />)
    const details = await openRow(user, 'kingdom')
    expect(git.loadGitIdentities).toHaveBeenCalledExactlyOnceWith(3)
    const accounts = within(details.getByRole('list', { name: 'Accounts' })).getAllByRole('listitem').map((item) => item.textContent)
    expect(accounts).toEqual(['kingdom-meactive', 'work-me'])
    expect(await details.findByText('Commits as Casey Example <casey@example.invalid>')).toBeInTheDocument()
    const rules = within(details.getByRole('list', { name: 'Folder rules' })).getAllByRole('listitem').map((item) => item.textContent)
    expect(rules).toEqual(['/home/dev/workwork-me · Casey At Work <casey@work.invalid>ChangeRemove', '~/ossset outside Maitty'])
    expect(screen.getByRole('button', { name: 'Accounts and folder rules on kingdom' })).toHaveAttribute('aria-expanded', 'true')
  })

  it('says every folder uses the active account when there are no rules', async () => {
    const user = userEvent.setup()
    answers({ 0: signedIn('me', 'work-me') })
    git.loadGitIdentities.mockResolvedValue({ ...WHO, name: null, email: null })
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    const details = await openRow(user, 'This computer')
    expect(await details.findByText('Every folder uses me')).toBeInTheDocument()
    expect(details.getByText('No name or email is set for commits')).toBeInTheDocument()
    expect(details.queryByRole('list', { name: 'Folder rules' })).toBeNull()
  })

  it('starts the same sign-in from Add another account', async () => {
    const user = userEvent.setup()
    const onSignIn = vi.fn()
    answers({ 0: signedIn('me'), 3: signedIn('kingdom-me') })
    git.loadGitIdentities.mockResolvedValue(WHO)
    render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM]} signIn={{ onSignIn, signInsEnded: 0 }} />)
    const details = await openRow(user, 'kingdom')
    await user.click(details.getByRole('button', { name: 'Add another account' }))
    expect(onSignIn).toHaveBeenCalledExactlyOnceWith(3, 'kingdom')
  })

  it('says why who commits could not be read, and still lists the accounts', async () => {
    const user = userEvent.setup()
    answers({ 0: signedIn('me') })
    git.loadGitIdentities.mockRejectedValue(new Error('kingdom did not answer within 60 seconds'))
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    const details = await openRow(user, 'This computer')
    expect((await details.findByRole('alert')).textContent).toBe('Could not read git’s settings: kingdom did not answer within 60 seconds')
    expect(details.getByRole('list', { name: 'Accounts' })).toBeInTheDocument()
    expect(details.queryByRole('button', { name: 'Add another account' })).toBeNull()
  })

  it('offers no details where nobody is signed in, and shows a rule\'s text as plain text', async () => {
    const user = userEvent.setup()
    answers({ 0: NOBODY, 3: signedIn('kingdom-me'), 5: NO_GH })
    git.loadGitIdentities.mockResolvedValue({ ...WHO, rules: [{ folder: '<img src=x onerror=alert(1)>', identity: null }] })
    const { container } = render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM, BARE]} />)
    const details = await openRow(user, 'kingdom')
    expect(await details.findByText('<img src=x onerror=alert(1)>')).toBeInTheDocument()
    expect(container.querySelector('img')).toBeNull()
    expect(screen.getAllByRole('button', { name: /^Accounts and folder rules on/ })).toHaveLength(1)
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

  const WORK = { name: 'Casey At Work', email: 'casey@work.invalid', login: 'work-me' }
  const WITH_RULE: GitIdentityReport = { ...WHO, rules: [{ folder: '/home/dev/work', identity: WORK }, { folder: '~/oss', identity: null }] }
  const LINES: GitConfigWrite[] = [
    { file: '/home/dev/.gitconfig', lines: ['includeIf.gitdir:/home/dev/work/.path = /data/git-identities/ab.gitconfig'] },
    { file: '/data/git-identities/ab.gitconfig', lines: ['user.name = Casey At Work', 'user.email = casey@work.invalid'] },
  ]

  it('removes a rule only after asking once, and reads the rules again', async () => {
    const user = userEvent.setup()
    answers({ 0: signedIn('me', 'work-me') })
    git.loadGitIdentities.mockResolvedValue(WITH_RULE)
    git.changeGitRule.mockResolvedValue([])
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    const details = await openRow(user, 'This computer')
    await user.click(await details.findByRole('button', { name: 'Remove the rule for /home/dev/work' }))
    expect(git.changeGitRule).not.toHaveBeenCalled()
    git.loadGitIdentities.mockResolvedValue({ ...WHO, rules: [{ folder: '~/oss', identity: null }] })
    await user.click(details.getByRole('button', { name: 'Remove this rule' }))
    expect(git.changeGitRule).toHaveBeenCalledExactlyOnceWith(0, { kind: 'remove', folder: '/home/dev/work' })
    await waitFor(() => { expect(details.queryByText('/home/dev/work')).toBeNull() })
  })

  it('keeps the rule on screen and says why when the machine will not remove it', async () => {
    const user = userEvent.setup()
    answers({ 0: signedIn('me', 'work-me') })
    git.loadGitIdentities.mockResolvedValue(WITH_RULE)
    git.changeGitRule.mockRejectedValue(new Error('the global git config is locked'))
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    const details = await openRow(user, 'This computer')
    await user.click(await details.findByRole('button', { name: 'Remove the rule for /home/dev/work' }))
    await user.click(details.getByRole('button', { name: 'Remove this rule' }))
    expect((await details.findByRole('alert')).textContent).toBe('the global git config is locked')
    expect(details.getByText('/home/dev/work')).toBeInTheDocument()
    expect(git.loadGitIdentities).toHaveBeenCalledOnce()
  })

  it('offers no change or removal for a rule set outside Maitty', async () => {
    const user = userEvent.setup()
    answers({ 0: signedIn('me') })
    git.loadGitIdentities.mockResolvedValue(WITH_RULE)
    render(<GitHubSection machines={[LOCAL_HEAD]} />)
    const details = await openRow(user, 'This computer')
    await details.findByText('~/oss')
    expect(details.queryByRole('button', { name: 'Remove the rule for ~/oss' })).toBeNull()
    expect(details.queryByRole('button', { name: 'Change the rule for ~/oss' })).toBeNull()
  })

  it('changes a rule on another machine: shows the lines first and writes only on Write these lines', async () => {
    const user = userEvent.setup()
    answers({ 0: signedIn('me'), 3: signedIn('kingdom-me', 'work-me') })
    git.loadGitIdentities.mockResolvedValue(WITH_RULE)
    git.changeGitRule.mockResolvedValue(LINES)
    render(<GitHubSection machines={[LOCAL_HEAD, KINGDOM]} />)
    const details = await openRow(user, 'kingdom')
    await user.click(await details.findByRole('button', { name: 'Change the rule for /home/dev/work' }))
    const dialog = await screen.findByRole('dialog', { name: 'Change the folder rule on kingdom' })
    const form = within(dialog)
    expect(form.getByRole('textbox', { name: 'Folder on kingdom' })).toHaveValue('/home/dev/work')
    expect(form.getByRole('combobox', { name: 'GitHub account' })).toHaveValue('work-me')
    await user.clear(form.getByRole('textbox', { name: 'Commit name' }))
    await user.type(form.getByRole('textbox', { name: 'Commit name' }), 'Casey W')
    await user.click(form.getByRole('button', { name: 'Show the lines' }))
    const changed = { ...WORK, name: 'Casey W' }
    expect(git.changeGitRule).toHaveBeenCalledExactlyOnceWith(3, { kind: 'preview', folder: '/home/dev/work', identity: changed })
    expect(await form.findByText('/data/git-identities/ab.gitconfig')).toBeInTheDocument()
    expect(git.changeGitRule.mock.calls.some(([, change]) => change.kind === 'apply')).toBe(false)
    await user.click(form.getByRole('button', { name: 'Write these lines' }))
    expect(git.changeGitRule).toHaveBeenLastCalledWith(3, { kind: 'apply', folder: '/home/dev/work', identity: changed })
    await waitFor(() => { expect(screen.queryByRole('dialog')).toBeNull() })
  })
})
