import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'

const Count = z.number().int().nonnegative()

const ChangeKindSchema = z.enum(['Added', 'Modified', 'Deleted', 'Renamed', 'Untracked', 'Conflicted'])
const StatusSchema = z.object({
  branch: z.string().nullable(),
  ahead: Count,
  behind: Count,
  files: z.array(z.object({ path: z.string(), kind: ChangeKindSchema })),
})
const EntrySchema = z.object({ name: z.string(), is_directory: z.boolean() })
const DiffSchema = z.object({
  files: z.array(z.object({ path: z.string(), additions: Count, deletions: Count, patch: z.string(), binary: z.boolean() })),
  additions: Count,
  deletions: Count,
  total_files: Count,
  truncated: z.boolean(),
})
const BranchSchema = z.object({ name: z.string(), current: z.boolean() })
const CommitSchema = z.object({ hash: z.string(), subject: z.string(), author: z.string(), time: z.number().int() })
const WorktreeSchema = z.object({ path: z.string(), branch: z.string(), main: z.boolean(), repo: z.string() })
const SnapshotSchema = z.object({ commit: z.string(), at: Count, why: z.string() })
const ReferenceSchema = z.object({ name: z.string(), kind: z.string(), bytes: Count, modified: Count })

export type RepoStatus = z.infer<typeof StatusSchema>
export type RepoChangeKind = z.infer<typeof ChangeKindSchema>
export type RepoEntry = z.infer<typeof EntrySchema>
export type RepoDiff = z.infer<typeof DiffSchema>
export type RepoBranch = z.infer<typeof BranchSchema>
export type RepoCommit = z.infer<typeof CommitSchema>
export type RepoWorktree = z.infer<typeof WorktreeSchema>
export type RepoSnapshot = z.infer<typeof SnapshotSchema>
export type RepoReference = z.infer<typeof ReferenceSchema>

const LOG_LIMIT = 60

/** Ask the daemon about the repository `term` is in. It answers `{ <Kind>: payload }`; a different kind is a bug. */
async function ask<T>(term: number, question: unknown, kind: string, payload: z.ZodType<T>): Promise<T> {
  if (!isTauri()) throw new Error('Repository details are only available in the desktop app')
  const reply = z.record(z.string(), z.unknown()).parse(await invoke<unknown>('repo_query', { term, question }))
  if (!(kind in reply)) throw new Error('The daemon answered a different question')
  return payload.parse(reply[kind])
}

export const repoStatus = (term: number) => ask(term, 'Status', 'Status', StatusSchema)
export const repoTree = (term: number, path: string | null) => ask(term, { Tree: { path } }, 'Tree', z.array(EntrySchema))
export const repoLocate = (term: number, path: string) => ask(term, { Locate: { path } }, 'Located', z.string())
export const repoFile = (term: number, path: string) => ask(term, { File: { path } }, 'File', z.string())
export const repoDiff = (term: number, path: string | null = null) => ask(term, { Diff: { path } }, 'Diff', DiffSchema)
export const repoDiffRange = (term: number, from: string, to: string, path: string | null = null) =>
  ask(term, { DiffRange: { from, to, path } }, 'Diff', DiffSchema)
export const repoBranches = (term: number) => ask(term, 'Branches', 'Branches', z.array(BranchSchema))
export const repoWorktrees = (term: number) => ask(term, 'Worktrees', 'Worktrees', z.array(WorktreeSchema))
export const repoLog = (term: number) => ask(term, { Log: { limit: LOG_LIMIT } }, 'Log', z.array(CommitSchema))
export const repoSnapshots = (term: number) => ask(term, 'Snapshots', 'Snapshots', z.array(SnapshotSchema))
export const repoReferences = (term: number) => ask(term, 'References', 'References', z.array(ReferenceSchema))

/** Put the pane's checkout back as it was in one of the daemon's snapshots; resolves to how many files changed. */
export async function restoreSnapshot(term: number, commit: string): Promise<number> {
  if (!isTauri()) throw new Error('Restoring is only available in the desktop app')
  return Count.parse(await invoke<unknown>('restore_snapshot', { term, commit }))
}
