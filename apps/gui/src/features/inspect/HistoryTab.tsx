import { useMemo, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { repoDiffRange, repoLog, repoSnapshots, restoreSnapshot } from './api'
import { ago, MS_PER_SECOND } from './format'
import { PatchView } from './PatchView'
import { messageOf, useRepoQuery } from './useRepoQuery'

type Entry = { id: string; kind: 'commit' | 'snapshot'; title: string; detail: string; time: number }

const SHORT_HASH = 7
const COMPARE_LIMIT = 2

type Props = { term: number; agentWorking: boolean; onNotice: (text: string) => void }

/** Commits and the daemon's before-each-turn snapshots, newest first. A snapshot can be restored; any two can be compared. */
export function HistoryTab({ term, agentWorking, onNotice }: Props) {
  const queryClient = useQueryClient()
  const log = useRepoQuery(term, ['log'], () => repoLog(term))
  const snapshots = useRepoQuery(term, ['snapshots'], () => repoSnapshots(term))
  const [compare, setCompare] = useState(false)
  const [picked, setPicked] = useState<string[]>([])
  const [confirming, setConfirming] = useState<string | null>(null)
  const [restoring, setRestoring] = useState(false)

  const entries = useMemo<Entry[]>(() => {
    const commits = (log.data ?? []).map((commit): Entry => ({ id: commit.hash, kind: 'commit', title: commit.subject, detail: `${commit.hash.slice(0, SHORT_HASH)} · ${commit.author}`, time: commit.time }))
    const saved = (snapshots.data ?? []).map((snapshot): Entry => ({ id: snapshot.commit, kind: 'snapshot', title: snapshot.why, detail: 'snapshot', time: snapshot.at }))
    return [...commits, ...saved].sort((a, b) => b.time - a.time)
  }, [log.data, snapshots.data])

  const [newer, older] = picked
    .map((id) => entries.find((entry) => entry.id === id))
    .filter((entry): entry is Entry => entry !== undefined)
    .sort((a, b) => b.time - a.time)
  const now = Math.max(log.dataUpdatedAt, snapshots.dataUpdatedAt) / MS_PER_SECOND
  const comparing = compare && older !== undefined && newer !== undefined
  const range = useRepoQuery(term, ['range', older?.id, newer?.id], () => repoDiffRange(term, older?.id ?? '', newer?.id ?? ''), false, comparing)

  const toggle = (id: string) => {
    setPicked((current) => (current.includes(id) ? current.filter((item) => item !== id) : [...current, id].slice(-COMPARE_LIMIT)))
  }

  const restore = (entry: Entry) => {
    setRestoring(true)
    void restoreSnapshot(term, entry.id)
      .then(async (changed) => {
        onNotice(changed === 0 ? 'The files are already as they were then' : `Restored ${String(changed)} file${changed === 1 ? '' : 's'} · the newest snapshot takes it back`)
        setConfirming(null)
        await queryClient.invalidateQueries({ queryKey: ['repo', term] })
      })
      .catch((error: unknown) => { onNotice(messageOf(error)) })
      .finally(() => { setRestoring(false) })
  }

  if (log.isPending || snapshots.isPending) return <p className="mx-inspect-note">Loading…</p>
  if (log.isError) return <p className="mx-inspect-note is-error" role="alert">{messageOf(log.error)}</p>

  return (
    <div className="mx-inspect-body">
      <label className="mx-inspect-toggle">
        <input type="checkbox" checked={compare} onChange={(event) => { setCompare(event.target.checked); setPicked([]) }} />
        Compare <span className="mx-inspect-meta">{compare ? `pick two (${String(picked.length)}/${String(COMPARE_LIMIT)})` : ''}</span>
      </label>
      {comparing && (
        <section aria-label="Comparison">
          <p className="mx-inspect-summary">{older.title} → {newer.title}</p>
          {range.isPending && <p className="mx-inspect-note">Loading…</p>}
          {range.isError && <p className="mx-inspect-note is-error" role="alert">{messageOf(range.error)}</p>}
          {range.data && (range.data.files.length === 0
            ? <p className="mx-inspect-note">No differences between these two.</p>
            : range.data.files.map((file) => (
              <div key={file.path}>
                <h3 className="mx-inspect-file">{file.path} <span className="is-add">+{file.additions}</span> <span className="is-remove">−{file.deletions}</span></h3>
                {file.binary ? <p className="mx-inspect-note">Binary file; no diff to show.</p> : <PatchView patch={file.patch} />}
              </div>
            )))}
        </section>
      )}
      <ul className="mx-inspect-list" aria-label="History">
        {entries.map((entry) => {
          const isPicked = picked.includes(entry.id)
          return (
            <li key={entry.id} className={`mx-inspect-row is-${entry.kind}${isPicked ? ' is-current' : ''}`}>
              {compare ? (
                <button type="button" className="mx-inspect-pick" aria-pressed={isPicked} onClick={() => { toggle(entry.id) }}>
                  <span className="mx-inspect-main">{entry.title}</span>
                  <span className="mx-inspect-meta">{entry.detail} · {ago(entry.time, now)}</span>
                </button>
              ) : (
                <>
                  <span className="mx-inspect-main">{entry.title}</span>
                  <span className="mx-inspect-meta">{entry.detail} · {ago(entry.time, now)}</span>
                  {entry.kind === 'snapshot' && confirming !== entry.id && (
                    <button type="button" className="mx-inspect-action" disabled={agentWorking} title={agentWorking ? 'Wait for the agent to finish' : undefined} onClick={() => { setConfirming(entry.id) }}>Restore</button>
                  )}
                </>
              )}
              {!compare && confirming === entry.id && (
                <div className="mx-inspect-confirm" role="alertdialog" aria-label="Confirm restore">
                  <span>Put the files back as they were then? What is there now is saved first.</span>
                  <button type="button" disabled={restoring} onClick={() => { setConfirming(null) }}>Cancel</button>
                  <button type="button" className="is-danger" disabled={restoring || agentWorking} onClick={() => { restore(entry) }}>{restoring ? 'Restoring…' : 'Restore files'}</button>
                </div>
              )}
            </li>
          )
        })}
        {entries.length === 0 && <li className="mx-inspect-note">No commits or snapshots yet.</li>}
      </ul>
    </div>
  )
}
