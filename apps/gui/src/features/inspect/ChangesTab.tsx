import { useState } from 'react'
import { Icon } from '@/features/workspace/components/glyphs'
import { repoDiff, repoDiffRange, repoSnapshots, repoStatus, type RepoDiff } from './api'
import { changeKind } from './changeKind'
import { CommitBar, type ChangeActions } from './CommitBar'
import { PatchView, type PatchComments, type SentComment } from './PatchView'
import { RepoError } from './RepoError'
import { turnRange, type MissingSnapshot, type SnapshotRange, type TurnSpan } from './turnRange'
import { messageOf, useRepoQuery } from './useRepoQuery'

export const CHANGES_REFRESH_MS = 4000

type ChangedFile = RepoDiff['files'][number]
/** A path as its folder (with the trailing slash) and its name. */
function splitPath(path: string): { folder: string; name: string } {
  const cut = path.lastIndexOf('/') + 1
  return { folder: path.slice(0, cut), name: path.slice(cut) }
}

function commentHint(count: number): string {
  if (count === 0) return 'Click a line to comment'
  return count === 1 ? '1 comment' : `${String(count)} comments`
}

/** A turn whose changes are asked for instead of the whole working tree; `turn` is null when its times are unknown. */
export type TurnScope = { turn: TurnSpan | null; onAllChanges: () => void }

/**
 * The diff to list: the working tree against HEAD, or, for a turn, the diff between the snapshots around it. A turn
 * whose snapshots cannot be picked falls back to the working tree and says why.
 */
function useScopedDiff(term: number, scope: TurnScope | undefined) {
  const turn = scope?.turn ?? null
  // Keyed by the turn: a list read before a later turn ended would not hold that turn's end snapshot.
  const snapshots = useRepoQuery(term, ['snapshots', 'turn', turn?.startedAtMs, turn?.endedAtMs], () => repoSnapshots(term), false, turn !== null)
  const picked = turn !== null && snapshots.data ? turnRange(snapshots.data, turn) : null
  const range = picked !== null && 'from' in picked ? picked : null
  const query = useRepoQuery(
    term,
    range ? ['range', range.from, range.to] : ['diff'],
    () => (range ? repoDiffRange(term, range.from, range.to) : repoDiff(term)),
    range ? false : CHANGES_REFRESH_MS,
  )
  const waiting = turn !== null && (snapshots.isPending || snapshots.isFetching)
  if (scope === undefined || waiting || range !== null) return { query, waiting, shown: range === null ? 'tree' as const : 'turn' as const, why: null }
  return { query, waiting, shown: 'tree' as const, why: fallbackReason(turn, snapshots.isError ? snapshots.error : null, picked) }
}

/** Why a turn's changes row shows the whole working tree instead. */
function fallbackReason(turn: TurnSpan | null, error: unknown, picked: MissingSnapshot | SnapshotRange | null): string {
  if (turn === null) return 'this turn has no times to find its snapshots by'
  if (error !== null) return `couldn't read the snapshots: ${messageOf(error)}`
  return picked !== null && 'missing' in picked && picked.missing === 'after' ? 'no snapshot from after this turn is kept' : 'no snapshot from before this turn is kept'
}

function ScopeLine({ shown, why, onAllChanges }: { shown: 'tree' | 'turn'; why: string | null; onAllChanges: () => void }) {
  if (shown === 'tree') return <p className="mx-changes-scope">Whole working tree · {why}</p>
  return (
    <div className="mx-changes-scope">
      <h3>This turn's changes</h3>
      <button type="button" className="mx-inspect-action" onClick={onAllChanges}>All changes</button>
    </div>
  )
}

type Props = {
  term: number
  /** The branch the checkout is on, shown above the files. */
  branch?: string
  comments?: Omit<PatchComments, 'path' | 'sent' | 'onSent'>
  /** Commit and Create PR, for a checkout on this computer; absent elsewhere. */
  actions?: ChangeActions
  /** A turn's changes row opened the tab: show what that turn changed. */
  scope?: TurnScope
  onNotice: (text: string) => void
}

function FileRow({ file, current, ticked, onTick, onPick }: { file: ChangedFile; current: boolean; ticked: boolean | null; onTick: () => void; onPick: () => void }) {
  const kind = changeKind(file.patch)
  const { folder, name } = splitPath(file.path)
  return (
    <div className="mx-changes-line">
      {ticked !== null && <input type="checkbox" className="mx-changes-tick" aria-label={`Commit ${file.path}`} checked={ticked} onChange={onTick} />}
    <button type="button" className={`mx-changes-row${current ? ' is-current' : ''}`} aria-current={current} title={file.path} onClick={onPick}>
      <span className={`mx-changes-letter is-${kind.tone}`} title={kind.word}>{kind.letter}</span>
      <span className="mx-changes-name">
        <b>{name}</b>
        {folder !== '' && <small>{folder.slice(0, -1)}</small>}
      </span>
      <span className="mx-changes-count">
        {file.additions > 0 && <span className="is-add">+{file.additions}</span>}
        {file.deletions > 0 && <span className="is-remove">−{file.deletions}</span>}
      </span>
    </button>
    </div>
  )
}

/** The session's uncommitted changes: the files down the left with the commit under them, the picked file's diff beside. */
export function ChangesTab({ term, branch, comments, actions: treeActions, scope, onNotice }: Props) {
  const { query, waiting, shown, why } = useScopedDiff(term, scope)
  // A commit takes the working tree, so it is not offered beside one turn's changes.
  const actions = shown === 'turn' ? undefined : treeActions
  // How far the branch is from its upstream, for the remote button; without it (an error, no upstream) the branch reads as level.
  const status = useRepoQuery(term, ['status'], () => repoStatus(term), CHANGES_REFRESH_MS)
  const [picked, setPicked] = useState<string | null>(null)
  // Files unticked, so left out of the commit; every changed file starts ticked.
  const [skipped, setSkipped] = useState<ReadonlySet<string>>(() => new Set())
  // Comments sent in this look at the changes, by file: a comment stays pinned on its line while other files are read.
  const [sent, setSent] = useState<ReadonlyMap<string, readonly SentComment[]>>(() => new Map())
  if (waiting || query.isPending) return <p className="mx-inspect-note">Loading…</p>
  if (query.isError) return <RepoError error={query.error} tab="changes" onRetry={() => { void query.refetch() }} retrying={query.isFetching} />
  const { files } = query.data
  const chosen = files.filter((file) => !skipped.has(file.path))
  const everything = chosen.length === files.length
  const tick = (path: string) => { setSkipped((current) => { const next = new Set(current); if (!next.delete(path)) next.add(path); return next }) }
  const selected = files.find((file) => file.path === picked) ?? files[0]
  const kind = selected ? changeKind(selected.patch) : null
  const where = selected ? splitPath(selected.path) : null
  return (
    <div className="mx-changes">
      <div className="mx-changes-side">
        {branch !== undefined && branch !== '' && <p className="mx-changes-branch" title={`Branch ${branch}`}><Icon name="branch" size={11} /><span>{branch}</span></p>}
        {scope && <ScopeLine shown={shown} why={why} onAllChanges={scope.onAllChanges} />}
        <ul className="mx-changes-list" aria-label="Changed files">
          {files.map((file) => (
            <li key={file.path}><FileRow file={file} current={file.path === selected?.path} ticked={actions ? !skipped.has(file.path) : null} onTick={() => { tick(file.path) }} onPick={() => { setPicked(file.path) }} /></li>
          ))}
          {query.data.truncated && <li className="mx-inspect-note">Showing {files.length} of {query.data.total_files} files.</li>}
        </ul>
        {actions && <CommitBar {...actions} files={files.length} chosen={chosen} everything={everything} ahead={status.data?.ahead ?? 0} behind={status.data?.behind ?? 0} onChanged={() => { void query.refetch(); void status.refetch() }} onNotice={onNotice} />}
      </div>
      <div className="mx-changes-main">
        {selected && kind && where ? (
          <>
            <header className="mx-changes-file">
              <span className="mx-changes-path" title={selected.path}>{where.folder}<b>{where.name}</b></span>
              <span className={`mx-changes-chip is-${kind.tone}`}>{kind.word}</span>
              <span className="mx-changes-count">
                {selected.additions > 0 && <span className="is-add">+{selected.additions}</span>}
                {selected.deletions > 0 && <span className="is-remove">−{selected.deletions}</span>}
              </span>
              {comments && !selected.binary && <span className="mx-changes-hint">{commentHint(sent.get(selected.path)?.length ?? 0)}</span>}
            </header>
            {selected.binary ? <p className="mx-inspect-note">Binary file; no diff to show.</p> : <PatchView key={selected.path} patch={selected.patch} {...(comments ? { comments: { ...comments, path: selected.path, sent: sent.get(selected.path) ?? [], onSent: (comment) => { setSent((current) => new Map(current).set(selected.path, [...(current.get(selected.path) ?? []), comment])) } } } : {})} />}
          </>
        ) : (
          <p className="mx-changes-none">{shown === 'turn' ? 'This turn left the files in the checkout as they were.' : 'No changes. The working tree matches the last commit.'}</p>
        )}
      </div>
    </div>
  )
}
