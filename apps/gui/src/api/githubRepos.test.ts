import { beforeEach, describe, expect, it, vi } from 'vitest'
import { loadGitHubOwners, loadGitHubRepos } from './githubRepos'

const shell = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args: { tag: number }) => Promise<unknown>>(),
  handlers: new Map<string, (event: { payload: unknown }) => void>(),
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: (name: string, handler: (event: { payload: unknown }) => void) => {
    shell.handlers.set(name, handler)
    return Promise.resolve(() => undefined)
  },
}))

/** Waits for the question to be sent, then answers it on `event` with the tag it was given. */
async function answer(event: string, payload: Record<string, unknown>) {
  await vi.waitFor(() => { expect(shell.invoke).toHaveBeenCalledOnce() })
  const tag = shell.invoke.mock.lastCall?.[1].tag
  shell.handlers.get(event)?.({ payload: { machine: 0, tag, why: null, listed: null, ...payload } })
}

beforeEach(() => {
  shell.invoke.mockReset()
  shell.invoke.mockResolvedValue(undefined)
})

describe('a machine\'s GitHub repositories', () => {
  it('asks this computer for its own, and reads each repository', async () => {
    const asked = loadGitHubRepos(0, null, '  ')
    await answer('github-repos', { state: 'listed', listed: { more: true, repos: [{ name: 'me/widgets', description: 'Parts', private: true, pushed_at: '2026-01-02T03:04:05Z', clone_url: 'https://github.com/me/widgets.git' }] } })
    await expect(asked).resolves.toEqual({ state: 'listed', listed: { more: true, repos: [{ name: 'me/widgets', description: 'Parts', private: true, pushedAt: '2026-01-02T03:04:05Z', cloneUrl: 'https://github.com/me/widgets.git' }] } })
    expect(shell.invoke).toHaveBeenCalledWith('github_repos', { owner: null, search: null, tag: expect.any(Number) as number })
  })

  it('names another machine, an owner and a search', async () => {
    const asked = loadGitHubRepos(3, 'my-org', ' wid ')
    await answer('github-repos', { state: 'listed', listed: { more: false, repos: [] } })
    await asked
    expect(shell.invoke).toHaveBeenCalledWith('github_repos', { machine: 3, owner: 'my-org', search: 'wid', tag: expect.any(Number) as number })
  })

  it.each([
    [{ state: 'not-installed' }, { state: 'not-installed' }],
    [{ state: 'not-signed-in' }, { state: 'not-signed-in' }],
    [{ state: 'failed', why: 'rate limited' }, { state: 'failed', why: 'rate limited' }],
    [{ state: 'listed', listed: null }, { state: 'failed', why: 'The machine sent no list' }],
  ])('reads %j as %j', async (said, read) => {
    const asked = loadGitHubRepos(0, null, '')
    await answer('github-repos', said)
    await expect(asked).resolves.toEqual(read)
  })

  it('rejects with the host\'s words when the machine could not be asked', async () => {
    shell.invoke.mockRejectedValue('kingdom is not connected')
    await expect(loadGitHubRepos(3, null, '')).rejects.toThrow('kingdom is not connected')
  })
})

describe('whose repositories a machine can list', () => {
  it('is the logins the machine gives', async () => {
    const asked = loadGitHubOwners(0)
    await answer('github-owners', { state: 'listed', listed: ['me', 'my-org'] })
    await expect(asked).resolves.toEqual({ state: 'listed', listed: ['me', 'my-org'] })
    expect(shell.invoke).toHaveBeenCalledWith('github_owners', { tag: expect.any(Number) as number })
  })
})
