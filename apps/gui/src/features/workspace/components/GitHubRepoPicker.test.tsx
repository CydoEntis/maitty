import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { GitHubList, GitHubRepo, GitHubRepoList } from '@/api/githubRepos'
import { CloneRepoDialog } from './CloneRepoDialog'
import type { GitHubRepoSource } from './GitHubRepoPicker'

afterEach(cleanup)

const repo = (name: string, pushedAt: string, more: Partial<GitHubRepo> = {}): GitHubRepo => ({ name, description: '', private: false, pushedAt, cloneUrl: `https://github.com/${name}.git`, ...more })
const listed = (repos: GitHubRepo[], more = false): GitHubList<GitHubRepoList> => ({ state: 'listed', listed: { repos, more } })

const OLD = repo('me/old', '2024-01-01T00:00:00Z')
const NEW = repo('me/new', '2026-01-01T00:00:00Z', { private: true, description: 'The new one' })
const MID = repo('me/mid', '2025-01-01T00:00:00Z')

type Repos = GitHubRepoSource['loadRepos']

/** The clone dialog for kingdom with a GitHub account that has `repos`, or that answers as `loadRepos` does. */
function open(loadRepos: Repos = vi.fn<Repos>(() => Promise.resolve(listed([OLD, NEW, MID]))), owners: GitHubList<string[]> = { state: 'listed', listed: ['me', 'my-org'] }) {
  const source = { loadOwners: vi.fn(() => Promise.resolve(owners)), loadRepos, onOpenSettings: vi.fn() }
  const clone = vi.fn(() => Promise.resolve('/home/dev/code/new'))
  render(<CloneRepoDialog where="kingdom" startIn="~/code" chooseFolder={null} clone={clone} github={source} onCloned={vi.fn()} onCancel={vi.fn()} />)
  return { ...source, clone }
}

const addressBox = () => screen.getByRole<HTMLInputElement>('textbox', { name: 'Repository address' })
const searchBox = () => screen.getByRole('textbox', { name: 'Search your repositories' })
const names = () => within(screen.getByRole('listbox')).getAllByRole('option').map((option) => option.getAttribute('aria-label'))

describe('picking a GitHub repository in the clone dialog', () => {
  it('lists the machine\'s repositories newest pushed first and marks a private one', async () => {
    const { loadRepos } = open()
    await screen.findByRole('option', { name: 'me/new, private repository' })
    expect(names()).toEqual(['me/new, private repository', 'me/mid, repository', 'me/old, repository'])
    expect(loadRepos).toHaveBeenCalledExactlyOnceWith(null, '')
  })

  it('fills the address with the repository clicked, which then clones', async () => {
    const user = userEvent.setup()
    const { clone } = open()
    await user.click(await screen.findByRole('option', { name: 'me/mid, repository' }))
    expect(addressBox().value).toBe('https://github.com/me/mid.git')
    await user.click(screen.getByRole('button', { name: 'Clone' }))
    expect(clone).toHaveBeenCalledWith('https://github.com/me/mid.git', '~/code')
  })

  it('moves with Up and Down and picks with Enter, without starting the clone', async () => {
    const user = userEvent.setup()
    const { clone } = open()
    await screen.findByRole('option', { name: 'me/new, private repository' })
    await user.click(searchBox())
    await user.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}{ArrowUp}')
    expect(screen.getByRole('option', { name: 'me/mid, repository' })).toHaveAttribute('aria-selected', 'true')
    await user.keyboard('{Enter}')
    expect(addressBox().value).toBe('https://github.com/me/mid.git')
    expect(clone).not.toHaveBeenCalled()
  })

  it('asks again once typing has stopped, with what was typed', async () => {
    const user = userEvent.setup()
    const loadRepos = vi.fn<Repos>((_owner, search) => Promise.resolve(listed(search === 'wid' ? [repo('me/widgets', '2026-02-02T00:00:00Z')] : [OLD])))
    open(loadRepos)
    await screen.findByRole('option', { name: 'me/old, repository' })
    await user.type(searchBox(), 'wid')
    expect(await screen.findByRole('option', { name: 'me/widgets, repository' })).toBeInTheDocument()
    expect(loadRepos.mock.calls).toEqual([[null, ''], [null, 'wid']])
  })

  it('lists another owner\'s repositories when it is chosen from the owner menu', async () => {
    const user = userEvent.setup()
    const loadRepos = vi.fn<Repos>((owner) => Promise.resolve(listed(owner === 'my-org' ? [repo('my-org/site', '2026-03-03T00:00:00Z')] : [OLD])))
    open(loadRepos)
    await user.selectOptions(await screen.findByRole('combobox', { name: 'Owner' }), 'my-org')
    expect(await screen.findByRole('option', { name: 'my-org/site, repository' })).toBeInTheDocument()
    expect(loadRepos).toHaveBeenLastCalledWith('my-org', '')
  })

  it('says to sign in, with a way to Settings, and still takes a pasted address', async () => {
    const user = userEvent.setup()
    const { onOpenSettings, clone } = open(vi.fn<Repos>(() => Promise.resolve({ state: 'not-signed-in' })))
    expect(await screen.findByText(/Sign in to GitHub on kingdom to pick from your repositories/)).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Open GitHub settings' }))
    expect(onOpenSettings).toHaveBeenCalledOnce()
    await user.type(addressBox(), 'https://example.com/o/r.git')
    await user.click(screen.getByRole('button', { name: 'Clone' }))
    expect(clone).toHaveBeenCalledWith('https://example.com/o/r.git', '~/code')
  })

  it('says where to get gh on a machine without it, and still takes a pasted address', async () => {
    const user = userEvent.setup()
    open(vi.fn<Repos>(() => Promise.resolve({ state: 'not-installed' })))
    expect(await screen.findByText(/install the GitHub command line \(gh\) on kingdom\. Get it from cli\.github\.com/)).toBeInTheDocument()
    await user.type(addressBox(), 'https://example.com/o/r.git')
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Clone' }).disabled).toBe(false)
  })

  it('says a list was cut and how to find the rest', async () => {
    open(vi.fn<Repos>(() => Promise.resolve(listed([OLD, NEW], true))))
    expect(await screen.findByText('Showing the first 2 repositories. Search to find the others.')).toBeInTheDocument()
  })

  it('says why a list could not be loaded and leaves the address box usable', async () => {
    const user = userEvent.setup()
    open(vi.fn<Repos>(() => Promise.reject(new Error('The machine did not answer in time'))))
    expect((await screen.findByRole('alert')).textContent).toBe('The machine did not answer in time. You can still paste an address.')
    await user.type(addressBox(), 'https://example.com/o/r.git')
    expect(addressBox().value).toBe('https://example.com/o/r.git')
  })

  it('says so when the account has no repositories, and when none matches', async () => {
    const user = userEvent.setup()
    open(vi.fn<Repos>(() => Promise.resolve(listed([]))), { state: 'listed', listed: ['me'] })
    expect(await screen.findByText('No repositories here yet.')).toBeInTheDocument()
    expect(screen.queryByRole('combobox', { name: 'Owner' })).toBeNull()
    await user.type(searchBox(), 'zzz')
    expect(await screen.findByText('No repository matches.')).toBeInTheDocument()
  })
})
