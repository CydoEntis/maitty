import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'
import { LOCAL_MACHINE } from './machines'
import { taggedAnswers } from './taggedAnswers'

const RepoSchema = z.object({
  name: z.string(),
  description: z.string(),
  private: z.boolean(),
  pushed_at: z.string(),
  clone_url: z.string(),
})
const RepoListSchema = z.object({ repos: z.array(RepoSchema), more: z.boolean() })

/** What the host says of a GitHub list: one of four states, with `why` for a failure and `listed` for a list. */
function answerSchema<T extends z.ZodType>(listed: T) {
  return z.object({
    machine: z.number().int(),
    tag: z.number().int(),
    state: z.enum(['listed', 'not-installed', 'not-signed-in', 'failed']),
    why: z.string().nullable(),
    listed: listed.nullable(),
  })
}

/** A repository the machine's GitHub account can clone. */
export type GitHubRepo = {
  /** `owner/name`. */
  name: string
  /** One line; empty when it has none. */
  description: string
  private: boolean
  /** When it was last pushed to, as GitHub writes a time; empty when unknown. */
  pushedAt: string
  /** The https address it is cloned from. */
  cloneUrl: string
}
export type GitHubRepoList = { repos: GitHubRepo[]; /** There are more than were given: a search narrows them. */ more: boolean }

/**
 * What a machine said when asked for a list from GitHub. Having no GitHub command line there, and
 * nobody signed in, are answers with a way out, not errors.
 */
export type GitHubList<T> = { state: 'listed'; listed: T } | { state: 'not-installed' } | { state: 'not-signed-in' } | { state: 'failed'; why: string }

/**
 * How long a machine has to answer. Its daemon gives `gh` fifteen seconds to say who is signed in
 * and thirty to list, and the host waits for that long and a little more.
 */
export const GITHUB_LIST_ANSWER_MS = 60_000

const NOT_ANSWERED = 'The machine did not answer in time'
const askRepos = taggedAnswers('github-repos', answerSchema(RepoListSchema))
const askOwners = taggedAnswers('github-owners', answerSchema(z.array(z.string())))

function listOf<T, U>(answer: { state: GitHubList<T>['state']; why: string | null; listed: T | null }, shape: (listed: T) => U): GitHubList<U> {
  switch (answer.state) {
    case 'listed':
      return answer.listed === null ? { state: 'failed', why: 'The machine sent no list' } : { state: 'listed', listed: shape(answer.listed) }
    case 'failed':
      return { state: 'failed', why: answer.why ?? 'The list could not be read' }
    case 'not-installed':
    case 'not-signed-in':
      return { state: answer.state }
  }
}

/**
 * The repositories a machine's GitHub account can clone (this computer is machine 0): `owner`'s, or
 * its own when `owner` is null, narrowed to those matching `search` when it is not empty.
 */
export async function loadGitHubRepos(machine: number, owner: string | null, search: string): Promise<GitHubList<GitHubRepoList>> {
  const words = { owner, search: search.trim() === '' ? null : search.trim() }
  const send = (tag: number) => invoke('github_repos', machine === LOCAL_MACHINE ? { ...words, tag } : { machine, ...words, tag })
  return listOf(await askRepos(send, GITHUB_LIST_ANSWER_MS, NOT_ANSWERED), (listed) => ({
    more: listed.more,
    repos: listed.repos.map((repo) => ({ name: repo.name, description: repo.description, private: repo.private, pushedAt: repo.pushed_at, cloneUrl: repo.clone_url })),
  }))
}

/** Whose repositories a machine can list: its signed-in account first, then that account's organisations. */
export async function loadGitHubOwners(machine: number): Promise<GitHubList<string[]>> {
  const send = (tag: number) => invoke('github_owners', machine === LOCAL_MACHINE ? { tag } : { machine, tag })
  return listOf(await askOwners(send, GITHUB_LIST_ANSWER_MS, NOT_ANSWERED), (owners) => owners)
}
