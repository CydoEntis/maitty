import { useState } from 'react'
import { Icon } from '@/features/workspace/components/glyphs'
import { LOCAL_MACHINE } from '@/api/machines'
import { repoDiff, repoStatus, type RepoDiff } from './api'
import { changeKind } from './changeKind'
import { CommitBar, type ChangeActions } from './CommitBar'
import { PatchView, type PatchComments, type SentComment } from './PatchView'
import { RepoError } from './RepoError'
import { useRepoQuery } from './useRepoQuery'

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

type Props = {
  term: number
  /** The machine the pane is on (0: this computer), whose daemon is asked. */
  machine?: number
  /** The branch the checkout is on, shown above the files. */
  branch?: string
  comments?: Omit<PatchComments, 'path' | 'sent' | 'onSent'>
  /** The commit bar: Commit, the remote button and Create PR on this computer, the remote button alone on another machine. */
  actions?: ChangeActions
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
export function ChangesTab({ term, machine = LOCAL_MACHINE, branch, comments, actions, onNotice }: Props) {
  // Each machine numbers its own panes, so another machine's answers are kept under its number too.
  const elsewhere = machine === LOCAL_MACHINE ? [] : [machine]
  const query = useRepoQuery(term, ['diff', ...elsewhere], () => repoDiff(term, null, machine), CHANGES_REFRESH_MS)
  // How far the branch is from its upstream, for the remote button; without it (an error, no upstream) the branch reads as level.
  const status = useRepoQuery(term, ['status', ...elsewhere], () => repoStatus(term, machine), CHANGES_REFRESH_MS)
  const [picked, setPicked] = useState<string | null>(null)
  // Files unticked, so left out of the commit; every changed file starts ticked.
  const [skipped, setSkipped] = useState<ReadonlySet<string>>(() => new Set())
  // Comments sent in this look at the changes, by file: a comment stays pinned on its line while other files are read.
  const [sent, setSent] = useState<ReadonlyMap<string, readonly SentComment[]>>(() => new Map())
  if (query.isPending) return <p className="mx-inspect-note">Loading…</p>
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
        <ul className="mx-changes-list" aria-label="Changed files">
          {files.map((file) => (
            <li key={file.path}><FileRow file={file} current={file.path === selected?.path} ticked={actions && actions.machine === LOCAL_MACHINE ? !skipped.has(file.path) : null} onTick={() => { tick(file.path) }} onPick={() => { setPicked(file.path) }} /></li>
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
          <p className="mx-changes-none">No changes. The working tree matches the last commit.</p>
        )}
      </div>
    </div>
  )
}
