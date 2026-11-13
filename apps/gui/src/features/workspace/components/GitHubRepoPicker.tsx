import { useEffect, useMemo, useState, type KeyboardEvent } from 'react'
import { ResultsList, ResultsOption } from '@/components/ui/results-list'
import type { GitHubList, GitHubRepo, GitHubRepoList } from '@/api/githubRepos'
import { errorMessage } from '@/lib/error-message'

/** How long after the last keystroke the machine is asked again. */
const SEARCH_DELAY_MS = 300
/** Longest search the daemon takes. */
const MOST_SEARCH_CHARS = 100
/** Where GitHub gives out its command line. */
const GH_SITE = 'cli.github.com'

/** Where the picker's lists come from: one machine's own GitHub command line. */
export type GitHubRepoSource = {
  loadOwners: () => Promise<GitHubList<string[]>>
  loadRepos: (owner: string | null, search: string) => Promise<GitHubList<GitHubRepoList>>
  /** Opens Settings on the GitHub page, where the machine can be signed in. */
  onOpenSettings: () => void
}

type Props = {
  /** The machine as the dialog names it: "this computer" or its name. */
  where: string
  source: GitHubRepoSource
  disabled: boolean
  onPick: (repo: GitHubRepo) => void
}

type Asked = { owner: string | null; search: string }
type Answered = { asked: Asked; result: GitHubList<GitHubRepoList> }

const sameAsked = (a: Asked, b: Asked) => a.owner === b.owner && a.search === b.search

/** Newest pushed first; a repository with no known time goes last. GitHub's times sort as text. */
function newestFirst(repos: readonly GitHubRepo[]): GitHubRepo[] {
  return [...repos].sort((a, b) => b.pushedAt.localeCompare(a.pushedAt))
}

/**
 * The repositories the machine's GitHub account can clone, to pick one instead of pasting its
 * address. Every name shown came from that machine's own `gh`, through its daemon. A machine that
 * is not signed in, or has no `gh`, says what to do about it; pasting an address still works.
 */
export function GitHubRepoPicker({ where, source, disabled, onPick }: Props) {
  const { loadOwners, loadRepos, onOpenSettings } = source
  const [owners, setOwners] = useState<readonly string[]>([])
  const [owner, setOwner] = useState<string | null>(null)
  const [typed, setTyped] = useState('')
  const [search, setSearch] = useState('')
  // Each answer is kept with what was asked, so one for a search the person has left is never shown.
  const [answered, setAnswered] = useState<Answered | null>(null)
  const [selected, setSelected] = useState(0)
  const asked = useMemo<Asked>(() => ({ owner, search }), [owner, search])
  const result = answered !== null && sameAsked(answered.asked, asked) ? answered.result : null

  useEffect(() => {
    let live = true
    loadOwners().then(
      (answer) => { if (live && answer.state === 'listed') setOwners(answer.listed) },
      // Without the owners there is no owner menu; the repositories' own answer says what is wrong.
      () => undefined,
    )
    return () => { live = false }
  }, [loadOwners])

  useEffect(() => {
    const timer = window.setTimeout(() => { setSearch(typed.trim()) }, SEARCH_DELAY_MS)
    return () => { window.clearTimeout(timer) }
  }, [typed])

  useEffect(() => {
    let live = true
    loadRepos(asked.owner, asked.search).then(
      (answer) => { if (live) setAnswered({ asked, result: answer }) },
      (error: unknown) => { if (live) setAnswered({ asked, result: { state: 'failed', why: errorMessage(error, 'The repositories could not be listed') } }) },
    )
    return () => { live = false }
  }, [loadRepos, asked])

  const list = result?.state === 'listed' ? result.listed : null
  const rows = useMemo(() => (list === null ? [] : newestFirst(list.repos)), [list])

  const onSearchKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setSelected((at) => Math.min(rows.length - 1, at + 1))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setSelected((at) => Math.max(0, at - 1))
    } else if (event.key === 'Enter') {
      // Enter here picks the highlighted repository; it does not start the clone.
      event.preventDefault()
      const row = rows[selected]
      if (row !== undefined) onPick(row)
    }
  }

  if (result?.state === 'not-signed-in') {
    return (
      <p>
        Sign in to GitHub on {where} to pick from your repositories.{' '}
        <button type="button" className="mx-text-button" disabled={disabled} onClick={onOpenSettings}>Open GitHub settings</button>
      </p>
    )
  }
  if (result?.state === 'not-installed') {
    return <p>To pick from your repositories, install the GitHub command line (gh) on {where}. Get it from {GH_SITE}</p>
  }

  return (
    <>
      <div className="mx-clone-into">
        <input
          aria-label="Search your repositories"
          placeholder="Search your repositories on GitHub"
          value={typed}
          maxLength={MOST_SEARCH_CHARS}
          spellCheck={false}
          disabled={disabled}
          onChange={(event) => { setTyped(event.target.value); setSelected(0) }}
          onKeyDown={onSearchKey}
        />
        {owners.length > 1 && (
          <select aria-label="Owner" value={owner ?? owners[0]} disabled={disabled} onChange={(event) => { setOwner(event.target.value); setSelected(0) }}>
            {owners.map((name) => <option key={name} value={name}>{name}</option>)}
          </select>
        )}
      </div>
      <ResultsList aria-label={`Your repositories on GitHub, from ${where}`} className="mx-getfile-list">
        {result === null && <p role="status">Loading your repositories…</p>}
        {result?.state === 'failed' && <p role="alert">{result.why}. You can still paste an address.</p>}
        {rows.map((repo, at) => (
          <ResultsOption
            key={repo.name}
            selected={at === selected}
            aria-label={repo.private ? `${repo.name}, private repository` : `${repo.name}, repository`}
            aria-disabled={disabled}
            title={repo.description === '' ? repo.name : `${repo.name}: ${repo.description}`}
            className={`mx-getfile-row${at === selected ? ' is-selected' : ''}`}
            onClick={() => { if (!disabled) onPick(repo) }}
            onMouseEnter={() => { setSelected(at) }}
          >
            <span aria-hidden="true">{repo.name}{repo.private ? '  · private' : ''}{repo.description === '' ? '' : `  · ${repo.description}`}</span>
          </ResultsOption>
        ))}
        {list !== null && rows.length === 0 && <p role="status">{asked.search === '' ? 'No repositories here yet.' : 'No repository matches.'}</p>}
      </ResultsList>
      {list?.more === true && <p role="status">Showing the first {rows.length} repositories. Search to find the others.</p>}
    </>
  )
}
