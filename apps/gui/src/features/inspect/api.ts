import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'
import { LOCAL_MACHINE } from '@/api/machines'

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

const HighlightSchema = z.array(z.array(z.object({ text: z.string(), kind: z.enum(['plain', 'comment', 'string', 'number', 'keyword', 'variable']) }))).nullable()
export type HighlightedLines = z.infer<typeof HighlightSchema>

/** The file's lines split into colour roles by the shared highlighter, or null when its type has no grammar. */
export async function highlightFile(path: string, text: string): Promise<HighlightedLines> {
  return HighlightSchema.parse(await invoke('highlight_file', { path, text }))
}

const LOG_LIMIT = 60

/**
 * Ask the daemon that owns pane `term` about the repository it is in: this computer's, or machine
 * `machine`'s. It answers `{ <Kind>: payload }`; a different kind is a bug.
 */
async function ask<T>(term: number, question: unknown, kind: string, payload: z.ZodType<T>, machine: number = LOCAL_MACHINE): Promise<T> {
  if (!isTauri()) throw new Error('Repository details are only available in the desktop app')
  const reply = z.record(z.string(), z.unknown()).parse(await invoke<unknown>('repo_query', { ...(machine === LOCAL_MACHINE ? {} : { machine }), term, question }))
  if (!(kind in reply)) throw new Error('The daemon answered a different question')
  return payload.parse(reply[kind])
}

// What changed and its diff are the two questions another machine's daemon is asked; the rest are this computer's.
export const repoStatus = (term: number, machine: number = LOCAL_MACHINE) => ask(term, 'Status', 'Status', StatusSchema, machine)
export const repoTree = (term: number, path: string | null) => ask(term, { Tree: { path } }, 'Tree', z.array(EntrySchema))
export const repoLocate = (term: number, path: string) => ask(term, { Locate: { path } }, 'Located', z.string())
export const repoFile = (term: number, path: string) => ask(term, { File: { path } }, 'File', z.string())
export const repoDiff = (term: number, path: string | null = null, machine: number = LOCAL_MACHINE) => ask(term, { Diff: { path } }, 'Diff', DiffSchema, machine)
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

/** Stages and commits every change in the checkout at `folder` on this computer; resolves to the commit's short hash. */
export async function commitAll(folder: string, message: string, machine: number, paths?: readonly string[]): Promise<string> {
  if (!isTauri()) throw new Error('Committing is only available in the desktop app')
  return z.string().parse(await invoke<unknown>('git_commit_all', { machine, folder, message, paths: paths ?? null }))
}

/** Saves the text of an existing file of the checkout `term` is in; the daemon refuses anything but a plain file inside it. */
export async function writeProjectFile(term: number, machine: number, path: string, contents: string): Promise<void> {
  if (!isTauri()) throw new Error('Editing is only available in the desktop app')
  await invoke('write_project_file', { machine, term, path, contents })
}

/** What the remote button can do for a checkout. */
export type SyncAction = 'fetch' | 'pull' | 'push'

/** Fetches, pulls (fast-forward only) or pushes the checkout at `folder` on this computer; resolves to a line to show. */
export async function syncRemote(folder: string, action: SyncAction, machine: number): Promise<string> {
  if (!isTauri()) throw new Error('Syncing is only available in the desktop app')
  return z.string().parse(await invoke<unknown>('git_sync', { machine, folder, action }))
}

/** Pushes the checkout's branch and opens a pull request for it; resolves to the pull request's address. */
export async function createPullRequest(folder: string, machine: number): Promise<string> {
  if (!isTauri()) throw new Error('Pull requests are only available in the desktop app')
  return z.string().parse(await invoke<unknown>('git_create_pull_request', { machine, folder }))
}
